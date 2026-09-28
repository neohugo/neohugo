//! Port of `hugolib/hugo_sites.go` (`.Site.Data`: `Data`, `loadData`, `handleDataFile`, `readData`).
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Split from `hugo_sites.go` (construction is T20's): `h.Data()` runs `loadData` once (Go
//! `h.init.data`), which walks the data component (BaseFs.Data, walk order), decodes each file
//! with the metadecoders (JSON/YAML/TOML/CSV; Go types preserved: int vs float64 vs string,
//! `[]interface {}`, `map[string]interface {}`), and nests the results by directory with Go's
//! merge rules for key collisions (`handleDataFile`). The comments partial reads this structure.
//! Oracle: tools/go-oracle/nh-hugolib/data (T23).
//!
//! Go mutates the nested maps in place (maps are references); the port builds the tree with
//! owned maps (`Arc::make_mut`) and freezes it into the `OnceLock`.

use std::io::Read;
use std::sync::Arc;

use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_helpers::source::file_info::File;
use nh_hugofs::walk::{Walkway, WalkwayConfig};

use crate::hugo_sites::HugoSites;

impl HugoSites {
    /// Go: `h.Data()` (lazy `loadData`; the result is cached in `self.data`). A load error is
    /// sent to the error handler and the data is Go's nil map (an empty map here).
    // Go: hugolib/hugo_sites.go:Data
    pub fn data(&self) -> Arc<Map> {
        self.data
            .get_or_init(|| match self.load_data() {
                Ok(m) => m,
                Err(err) => {
                    // Go: the `h.init.data` closure wraps the error once, `Data()` again.
                    self.deps
                        .send_error(err.wrap("failed to load data").wrap("failed to load data"));
                    Arc::new(Map::new(MapType::StringAny))
                }
            })
            .clone()
    }

    /// Go: `loadData()`.
    // Go: hugolib/hugo_sites.go:loadData
    pub fn load_data(&self) -> Result<Arc<Map>> {
        let mut data = Map::new(MapType::StringAny);
        let ps = self.deps.path_spec();
        let source_spec = self.deps.source_spec().clone();
        let ignore_file = move |f: &str| source_spec.ignore_file(f);
        let path_parser = self.deps.conf.path_parser();

        let mut walk_fn = |_path: &str, fi: &nh_hugofs::fileinfo::FileMetaInfo| -> Result<()> {
            if fi.is_dir() {
                return Ok(());
            }
            if fi.meta().path_info.is_none() {
                panic!("no path info");
            }
            let r = File::new(fi.clone());
            self.handle_data_file(&mut data, &r)
        };

        let mut cfg =
            WalkwayConfig::new(ps.base_fs.source_filesystems.data.fs.clone(), &mut walk_fn);
        cfg.ignore_file = Some(&ignore_file);
        cfg.path_parser = Some(path_parser);
        let mut w = Walkway::new(cfg);
        w.walk()?;

        Ok(Arc::new(data))
    }

    /// Go: `handleDataFile(r)` — decodes the file and inserts it below its directory's keys;
    /// a map merges into an existing map (existing keys win), anything else never replaces
    /// existing data.
    // Go: hugolib/hugo_sites.go:handleDataFile
    pub fn handle_data_file(&self, root: &mut Map, r: &File) -> Result<()> {
        // Crawl in data tree to insert data
        let path_info = r
            .file_info()
            .meta()
            .path_info
            .clone()
            .expect("data file path info");
        let dir = path_info.unnormalized().dir().to_string();
        let data_path = if dir.is_empty() { "" } else { &dir[1..] };
        let key_parts: Vec<&str> = data_path.split('/').collect();

        let mut current: &mut Map = root;
        for key in key_parts {
            if key.is_empty() {
                continue;
            }
            if current.get(key.as_bytes()).is_none() {
                current.insert(key, Value::map(Map::new(MapType::StringAny)));
            }
            let entry = current.entries.get_mut(key.as_bytes()).expect("inserted");
            if !matches!(entry, Value::Map(m) if m.ty == MapType::StringAny) {
                // Go: `current[key].(map[string]any)` panics.
                return Err(Error::new(format!(
                    "interface conversion: interface {{}} is {}, not map[string]interface {{}}",
                    entry.go_type_name()
                )));
            }
            let Value::Map(m) = entry else {
                unreachable!("checked")
            };
            current = Arc::make_mut(m);
        }

        let data = self
            .read_data(r)
            .map_err(|err| self.err_with_file_context(err, r))?;

        if data.is_invalid() {
            return Ok(());
        }

        // filepath.Walk walks the files in lexical order, '/' comes before '.'
        let base = r.base_file_name();
        let higher_precedent_data = current.get(base.as_bytes()).cloned();

        match &data {
            Value::Map(dm) if dm.ty == MapType::StringAny => match higher_precedent_data {
                None | Some(Value::Invalid) => {
                    current.insert(base.as_str(), data.clone());
                }
                Some(Value::Map(_)) if matches!(current.get(base.as_bytes()), Some(Value::Map(m)) if m.ty == MapType::StringAny) =>
                {
                    // merge maps: insert entries from data for keys that
                    // don't already exist in higherPrecedentData
                    let Some(Value::Map(hm)) = current.entries.get_mut(base.as_bytes()) else {
                        unreachable!("checked");
                    };
                    let higher_precedent_map = Arc::make_mut(hm);
                    for (key, value) in &dm.entries {
                        if higher_precedent_map.get(key.as_bytes()).is_some() {
                            // this warning could happen if
                            // 1. A theme uses the same key; the main data folder wins
                            // 2. A sub folder uses the same key: the sub folder wins
                            self.deps.log.infof(format!(
                                "Data for key '{}' in path '{}' is overridden by higher precedence data already in the data tree",
                                key.to_str_lossy(),
                                r.path()
                            ));
                        } else {
                            higher_precedent_map.insert(key.clone(), value.clone());
                        }
                    }
                }
                Some(other) => {
                    // can't merge: higherPrecedentData is not a map
                    self.deps.log.warnf(format!(
                        "The {} data from '{}' overridden by higher precedence {} data already in the data tree",
                        data.go_type_name(),
                        r.path(),
                        other.go_type_name()
                    ));
                }
            },
            Value::List(l) if l.ty == go_value::SliceType::Any => match higher_precedent_data {
                None | Some(Value::Invalid) => {
                    current.insert(base.as_str(), data.clone());
                }
                Some(other) => {
                    // we don't merge array data
                    self.deps.log.warnf(format!(
                        "The {} data from '{}' overridden by higher precedence {} data already in the data tree",
                        data.go_type_name(),
                        r.path(),
                        other.go_type_name()
                    ));
                }
            },
            _ => {
                self.deps.log.errorf(format!(
                    "unexpected data type {} in file {}",
                    data.go_type_name(),
                    r.logical_name()
                ));
            }
        }

        Ok(())
    }

    /// Go: `errWithFileContext(err, f)` — a file error at the data file.
    // Go: hugolib/hugo_sites.go:errWithFileContext
    pub fn err_with_file_context(&self, err: Error, f: &File) -> Error {
        let real_filename = f.file_info().meta().filename.clone();
        nh_common::herrors::new_file_error_from_name(err, &real_filename)
    }

    /// Go: `readData(f)` — the file decoded by its extension's format (`metadecoders.Default`).
    // Go: hugolib/hugo_sites.go:readData
    pub fn read_data(&self, f: &File) -> Result<Value> {
        let mut file = f
            .file_info()
            .meta()
            .open()
            .map_err(|err| err.wrap("readData: failed to open data file"))?;
        let mut content = Vec::new();
        // Go `helpers.ReaderToBytes` ignores read errors.
        let _ = file.read_to_end(&mut content);

        let format = nh_parser::metadecoders::format::format_from_string(&f.ext());
        nh_parser::metadecoders::decoder::Decoder::default().unmarshal(&content, format)
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites.go (data part; the rest is in hugo_sites.rs)
// OK L190-196: (h *HugoSites) Data() map[string]any
// OK L498-521: (h *HugoSites) loadData() error
// OK L523-599: (h *HugoSites) handleDataFile(r *source.File) error
// OK L601-604: (h *HugoSites) errWithFileContext(err error, f *source.File) error
// OK L606-616: (h *HugoSites) readData(f *source.File) (any, error)
// ---------------------------------------------------------------------------
