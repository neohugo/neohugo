//! Port of `internal/js/esbuild/sourcemap.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Go `sourcemap.go`: the `sources` of esbuild's external source maps are resolved to file URLs
//! (`fixOutputFile`). The map is re-marshalled from Go's `sourceMap` struct, so only its five
//! fields survive, in struct order.

use go_json::{JsonField, JsonStruct};
use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;

use crate::build::OutputFile;

/// Go: `sourceMap`.
#[derive(Clone, Debug, Default)]
struct SourceMap {
    version: i64,
    /// `None` = Go's nil slice (`null`).
    sources: Option<Vec<String>>,
    sources_content: Option<Vec<String>>,
    mappings: String,
    names: Option<Vec<String>>,
}

impl SourceMap {
    // encoding/json.Unmarshal into the struct (case-insensitive field names, unknown fields
    // ignored, wrong types an error).
    fn unmarshal(b: &[u8]) -> Result<SourceMap> {
        let v = go_json::unmarshal(b).map_err(|e| Error::new(e.to_string()))?;
        let mut sm = SourceMap::default();
        let Value::Map(m) = &v else {
            if v.is_invalid() {
                return Ok(sm);
            }
            return Err(Error::new(format!(
                "json: cannot unmarshal {} into Go value of type esbuild.sourceMap",
                json_kind(&v)
            )));
        };
        let strings = |x: &Value, field: &str| -> Result<Option<Vec<String>>> {
            match x {
                Value::Invalid => Ok(None),
                Value::List(l) => {
                    let mut out = Vec::new();
                    for e in l.items.iter() {
                        match e {
                            Value::String(s) => out.push(s.to_str_lossy().into_owned()),
                            Value::Invalid => out.push(String::new()),
                            other => {
                                return Err(Error::new(format!(
                                    "json: cannot unmarshal {} into Go struct field sourceMap.{field} of type string",
                                    json_kind(other)
                                )));
                            }
                        }
                    }
                    Ok(Some(out))
                }
                other => Err(Error::new(format!(
                    "json: cannot unmarshal {} into Go struct field sourceMap.{field} of type []string",
                    json_kind(other)
                ))),
            }
        };
        for (k, x) in m.entries.iter() {
            let k = k.to_str_lossy();
            match go_unicode::strings::to_lower_str(&k).as_ref() {
                "version" => match x {
                    Value::Float(f, _) => sm.version = *f as i64,
                    Value::Invalid => {}
                    other => {
                        return Err(Error::new(format!(
                            "json: cannot unmarshal {} into Go struct field sourceMap.version of type int",
                            json_kind(other)
                        )));
                    }
                },
                "sources" => sm.sources = strings(x, "sources")?,
                "sourcescontent" => sm.sources_content = strings(x, "sourcesContent")?,
                "mappings" => match x {
                    Value::String(s) => sm.mappings = s.to_str_lossy().into_owned(),
                    Value::Invalid => {}
                    other => {
                        return Err(Error::new(format!(
                            "json: cannot unmarshal {} into Go struct field sourceMap.mappings of type string",
                            json_kind(other)
                        )));
                    }
                },
                "names" => sm.names = strings(x, "names")?,
                _ => {}
            }
        }
        Ok(sm)
    }

    // encoding/json.Marshal of the struct.
    fn marshal(&self) -> Result<Vec<u8>> {
        let list = |v: &Option<Vec<String>>| match v {
            None => Value::TypedNil(std::sync::Arc::from("[]string")),
            Some(v) => Value::string_list(v.iter().map(String::as_str)),
        };
        let s = JsonStruct::new(
            "esbuild.sourceMap",
            vec![
                JsonField::new("version", Value::int(self.version)),
                JsonField::new("sources", list(&self.sources)),
                JsonField::new("sourcesContent", list(&self.sources_content)),
                JsonField::new("mappings", Value::string(self.mappings.as_str())),
                JsonField::new("names", list(&self.names)),
            ],
        );
        go_json::marshal(&Value::object(s)).map_err(|e| Error::new(e.to_string()))
    }
}

/// The JSON kind names of encoding/json's `UnmarshalTypeError`.
fn json_kind(v: &Value) -> &'static str {
    match v {
        Value::Bool(_) => "bool",
        Value::String(_) => "string",
        Value::List(_) => "array",
        Value::Map(_) => "object",
        _ => "number",
    }
}

// Go: internal/js/esbuild/sourcemap.go:fixOutputFile
pub(crate) fn fix_output_file(o: &mut OutputFile, resolve: &dyn Fn(&str) -> String) -> Result<()> {
    if o.path.ends_with(".map") {
        let b = fix_source_map(&o.contents, resolve)?;
        o.contents = b;
    }
    Ok(())
}

// Go: internal/js/esbuild/sourcemap.go:fixSourceMap
fn fix_source_map(s: &[u8], resolve: &dyn Fn(&str) -> String) -> Result<Vec<u8>> {
    let mut sm = SourceMap::unmarshal(s)?;

    sm.sources = fix_source_map_sources(sm.sources.as_deref().unwrap_or_default(), resolve);

    sm.marshal()
}

// Go: internal/js/esbuild/sourcemap.go:fixSourceMapSources
fn fix_source_map_sources(s: &[String], resolve: &dyn Fn(&str) -> String) -> Option<Vec<String>> {
    let mut result: Option<Vec<String>> = None;
    for src in s {
        let s = resolve(src);
        if !s.is_empty() {
            // Absolute filenames works fine on U*ix (tested in Chrome on MacOs), but works very poorly on Windows (again Chrome).
            // So, convert it to a URL.
            if let Ok(u) = nh_common::paths::url::url_from_filename(&s) {
                result
                    .get_or_insert_with(Vec::new)
                    .push(String::from_utf8_lossy(&u.string()).into_owned());
            }
        }
    }
    result
}

/// Go: `SourcesFromSourceMap(s)` (used in tests).
// Go: internal/js/esbuild/sourcemap.go:SourcesFromSourceMap
pub fn sources_from_source_map(s: &str) -> Vec<String> {
    match SourceMap::unmarshal(s.as_bytes()) {
        Ok(sm) => sm.sources.unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: internal/js/esbuild/sourcemap.go (80 lines; 1/4 funcs executed)
//   types: sourceMap
// OK L32-41: fixOutputFile(o *api.OutputFile, resolve func(string) string) error
// OK L43-57: fixSourceMap(s []byte, resolve func(string) string) ([]byte, error)
// OK L59-71: fixSourceMapSources(s []string, resolve func(string) string) []string
// OK L74-80: SourcesFromSourceMap(s string) []string
// ---------------------------------------------------------------------------
