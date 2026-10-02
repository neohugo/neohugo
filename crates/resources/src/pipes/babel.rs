//! `babel`: the `@babel/cli` binary with the project's Babel configuration. The script goes
//! in on stdin (`--filename` names it); Babel writes the result, and an external source map,
//! to a temporary file.

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::Value as Json;

use super::exec::{self, Tool};
use super::{Output, PipeError, TransformEnv, bad, opt_bool, opt_string, option_entries};
use crate::store::{Resource, ResourceStore};

/// A `babel` switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BabelFlag {
    /// `minified`: `--minified`.
    Minified,
    /// `noComments`: `--no-comments`.
    NoComments,
    /// `verbose`: `--verbose`.
    Verbose,
    /// `noBabelrc`: `--no-babelrc`.
    NoBabelrc,
}

impl BabelFlag {
    const fn arg(self) -> &'static str {
        match self {
            Self::Minified => "--minified",
            Self::NoComments => "--no-comments",
            Self::Verbose => "--verbose",
            Self::NoBabelrc => "--no-babelrc",
        }
    }
}

/// `sourceMap`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BabelSourceMap {
    #[default]
    None,
    /// In the script (`--source-maps=inline`).
    Inline,
    /// Published next to the script as `<target>.map`.
    External,
}

/// The `babel` options.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct BabelOptions {
    /// `config`: the configuration file (default `babel.config.js` when it exists).
    pub config: Option<String>,
    pub flags: BTreeSet<BabelFlag>,
    /// `compact`: `--compact=<bool>` when set.
    pub compact: Option<bool>,
    pub source_map: BabelSourceMap,
}

impl BabelOptions {
    /// Decodes the template's options map (keys case-insensitive; `null`: the defaults).
    ///
    /// # Errors
    /// An option of the wrong type or an unknown `sourceMap`.
    pub fn from_json(v: &Json) -> Result<Self, PipeError> {
        let mut o = Self::default();
        for (key, v) in option_entries(v)? {
            let flag = match key.as_str() {
                "minified" => Some(BabelFlag::Minified),
                "nocomments" => Some(BabelFlag::NoComments),
                "verbose" => Some(BabelFlag::Verbose),
                "nobabelrc" => Some(BabelFlag::NoBabelrc),
                "config" => {
                    let c = opt_string("config", v)?;
                    o.config = (!c.is_empty()).then_some(c);
                    None
                }
                "compact" => {
                    o.compact = Some(opt_bool("compact", v)?);
                    None
                }
                "sourcemap" => {
                    o.source_map = match opt_string("sourceMap", v)?.to_ascii_lowercase().as_str() {
                        "" | "none" => BabelSourceMap::None,
                        "inline" => BabelSourceMap::Inline,
                        "external" => BabelSourceMap::External,
                        other => {
                            return Err(bad("sourceMap", format!("unsupported value {other:?}")));
                        }
                    };
                    None
                }
                _ => None,
            };
            if let Some(f) = flag
                && opt_bool(&key, v)?
            {
                o.flags.insert(f);
            }
        }
        Ok(o)
    }
}

pub(super) fn run(
    store: &ResourceStore,
    env: &TransformEnv,
    src: &Resource,
    this: &Resource,
    o: &BabelOptions,
    input: &[u8],
) -> Result<Output, PipeError> {
    const DEFAULT_CONFIG: &str = "babel.config.js";
    let name = o.config.as_deref().unwrap_or(DEFAULT_CONFIG);
    let config = exec::config_file(store, env, name);
    if config.is_none() && o.config.is_some() {
        return Err(PipeError::ConfigNotFound {
            tool: "babel",
            name: name.to_owned(),
        });
    }
    // One realization per resource at a time: the store, the process and the resource name
    // the file uniquely.
    let out_file: PathBuf = std::env::temp_dir().join(format!(
        "ssg-babel-{}-{:x}-{}.js",
        std::process::id(),
        std::ptr::from_ref(store) as usize,
        this.id.raw()
    ));
    let map_file = PathBuf::from(format!("{}.map", out_file.display()));
    let mut args = Vec::new();
    if let Some(c) = config {
        args.push("--config-file".to_owned());
        args.push(c.display().to_string());
    }
    match o.source_map {
        BabelSourceMap::None => {}
        BabelSourceMap::Inline => args.push("--source-maps=inline".to_owned()),
        BabelSourceMap::External => args.push("--source-maps".to_owned()),
    }
    args.extend(o.flags.iter().map(|f| f.arg().to_owned()));
    if let Some(c) = o.compact {
        args.push(format!("--compact={c}"));
    }
    args.push(format!(
        "--filename={}",
        src.link.as_str().trim_start_matches('/')
    ));
    args.push(format!("--out-file={}", out_file.display()));
    let result = exec::run(store, env, Tool::Babel, &args, input).and_then(|_| {
        std::fs::read(&out_file).map_err(|source| PipeError::Io {
            what: out_file.display().to_string(),
            source,
        })
    });
    let map = std::fs::read(&map_file).ok();
    let _ = std::fs::remove_file(&out_file);
    let _ = std::fs::remove_file(&map_file);
    let mut code = result?;
    if let Some(m) = &map {
        let name = ssg_base::paths::base(this.link.as_str());
        code = relink(&code, &format!("{name}.map"));
        return Ok(Output {
            bytes: code,
            source_map: Some(m.clone()),
        });
    }
    Ok(Output {
        bytes: code,
        source_map: None,
    })
}

/// Points the `//# sourceMappingURL=` comment at `map_name` (appending one if missing).
fn relink(code: &[u8], map_name: &str) -> Vec<u8> {
    const MARK: &[u8] = b"//# sourceMappingURL=";
    let comment = format!("//# sourceMappingURL={map_name}\n");
    let Some(i) = code.windows(MARK.len()).rposition(|w| w == MARK) else {
        let mut out = code.to_vec();
        if !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        out.extend_from_slice(comment.as_bytes());
        return out;
    };
    let end = code[i..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(code.len(), |n| i + n + 1);
    let mut out = code[..i].to_vec();
    out.extend_from_slice(comment.as_bytes());
    out.extend_from_slice(&code[end..]);
    out
}
