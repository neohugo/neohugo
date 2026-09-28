//! Port of `resources/jsconfig/jsconfig.go`.
//!
//! Owner: Wave B task T14 (resources-core).

use std::collections::BTreeSet;
use std::sync::Mutex;

/// Go: `jsconfig.Builder` — builds a jsconfig.json file that, currently, is used only to assist
/// IntelliSense in editors. Records Hugo-resolved import roots; if non-empty, postProcess writes
/// `assets/jsconfig.json` (not the case for seeksnack).
#[derive(Default)]
pub struct Builder {
    pub(crate) source_roots: Mutex<BTreeSet<String>>,
}

/// Go: `jsconfig.Config` (`{"compilerOptions": {"baseUrl": ".", "paths": {"*": roots}}}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub base_url: String,
    /// Go `Paths map[string][]string` (only `"*"` is set).
    pub paths_star: Vec<String>,
}

impl Config {
    /// Go: `json.MarshalIndent(jsConfig, "", " ")` (hugolib writes these bytes).
    pub fn marshal_indent(&self) -> Vec<u8> {
        let s = |v: &str| go_json::marshal(&go_value::Value::string(v)).unwrap_or_default();
        let mut b = b"{\n \"compilerOptions\": {\n  \"baseUrl\": ".to_vec();
        b.extend(s(&self.base_url));
        b.extend_from_slice(b",\n  \"paths\": {\n   \"*\": [");
        for (i, p) in self.paths_star.iter().enumerate() {
            if i > 0 {
                b.push(b',');
            }
            b.extend_from_slice(b"\n    ");
            b.extend(s(p));
        }
        if !self.paths_star.is_empty() {
            b.extend_from_slice(b"\n   ");
        }
        b.extend_from_slice(b"]\n  }\n }\n}");
        b
    }
}

impl Builder {
    /// NewBuilder creates a new Builder.
    // Go: resources/jsconfig/jsconfig.go:NewBuilder
    pub fn new() -> Builder {
        Builder::default()
    }

    /// AddSourceRoot adds a new source root. This method is thread safe.
    // Go: resources/jsconfig/jsconfig.go:AddSourceRoot
    pub fn add_source_root(&self, root: &str) {
        self.source_roots
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(root.to_string());
    }

    /// Build builds a new Config with paths relative to dir (`None` when there are no roots),
    /// marshalled like hugolib does (`json.MarshalIndent(jsConfig, "", " ")`). This method is
    /// thread safe.
    // Go: resources/jsconfig/jsconfig.go:Build
    pub fn build(&self, dir: &str) -> Option<Vec<u8>> {
        self.build_config(dir).map(|c| c.marshal_indent())
    }

    /// [`Builder::build`] before marshalling.
    // Go: resources/jsconfig/jsconfig.go:Build
    pub fn build_config(&self, dir: &str) -> Option<Config> {
        let roots_set = self.source_roots.lock().unwrap_or_else(|e| e.into_inner());

        if roots_set.is_empty() {
            return None;
        }
        let mut conf = new_js_config();

        let mut roots = Vec::new();
        for root in roots_set.iter() {
            if let Ok(rel) =
                go_path::filepath::rel(dir, &go_path::filepath::join(&[root.as_str(), "*"]))
            {
                roots.push(rel);
            }
        }
        roots.sort();
        conf.paths_star = roots;

        Some(conf)
    }
}

// Go: resources/jsconfig/jsconfig.go:newJSConfig
fn new_js_config() -> Config {
    Config {
        base_url: ".".to_string(),
        paths_star: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build() {
        let b = Builder::new();
        assert!(b.build("/p").is_none());
        b.add_source_root("/p/assets");
        b.add_source_root("/p/node_modules");
        b.add_source_root("/p/assets");
        let got = String::from_utf8(b.build("/p/assets").unwrap()).unwrap();
        assert_eq!(
            got,
            "{\n \"compilerOptions\": {\n  \"baseUrl\": \".\",\n  \"paths\": {\n   \"*\": [\n    \"*\",\n    \"../node_modules/*\"\n   ]\n  }\n }\n}"
        );
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/jsconfig/jsconfig.go (92 lines; 2/4 funcs executed)
//   types: Builder, CompilerOptions, Config
// OK L30-32: NewBuilder() *Builder
// OK L36-56: (b *Builder) Build(dir string) *Config
// OK L60-72: (b *Builder) AddSourceRoot(root string)
// OK L85-92: newJSConfig() *Config
// ---------------------------------------------------------------------------
