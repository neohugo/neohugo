//! Port of `helpers/url.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).
//!
//! Go `helpers/url.go`: `urlize`, `absURL`/`absLangURL`, `relURL`/`relLangURL` (canonifyURLs=true
//! skips AddContextRoot in RelURL — see specs/i18n-lang-misc.md §3.2).

use go_path::{filepath, path};
use nh_common::Result;
use nh_common::herrors::Error;

use crate::pathspec::PathSpec;

impl PathSpec {
    /// Go: `URLize(uri)` = `URLEscape(MakePathSanitized(uri))`.
    // Go: helpers/url.go:URLize
    pub fn urlize(&self, uri: &str) -> String {
        self.url_escape(&self.make_path_sanitized(uri))
    }

    /// Go: `URLizeFilename(filename)` — escapes and turns separators into slashes.
    // Go: helpers/url.go:URLizeFilename
    pub fn urlize_filename(&self, filename: &str) -> String {
        self.url_escape(filepath::to_slash(filename))
    }

    /// Go: `URLEscape(uri)` = `url.Parse(uri).String()` (panics on a parse error in Go, and here;
    /// see [`PathSpec::try_url_escape`]).
    // Go: helpers/url.go:URLEscape
    pub fn url_escape(&self, uri: &str) -> String {
        match self.try_url_escape(uri) {
            Ok(s) => s,
            Err(e) => panic!("{e}"),
        }
    }

    /// [`PathSpec::url_escape`] with Go's panic value as an error.
    // Go: helpers/url.go:URLEscape
    pub fn try_url_escape(&self, uri: &str) -> Result<String> {
        // escape unicode letters
        let parsed = go_url::parse(uri).map_err(|e| Error::new(e.to_string()))?;
        Ok(String::from_utf8(parsed.string()).expect("URL.String of a parsed str is UTF-8"))
    }

    /// Go: `AbsURL(in, addLanguage)` — parse error -> `in` unchanged (why `%!s(<nil>)` survives).
    // Go: helpers/url.go:AbsURL
    pub fn abs_url(&self, input: &str, add_language: bool) -> String {
        let is_abs = match self.try_is_abs_url(input) {
            Ok(b) => b,
            Err(_) => return input.to_string(),
        };
        if is_abs || input.starts_with("//") {
            // It  is already  absolute, return it as is.
            return input.to_string();
        }

        let base_url = self.get_base_url_root(input);

        let mut input = input.to_string();
        if add_language {
            let prefix = self.get_language_prefix();
            if !prefix.is_empty() {
                // avoid adding language prefix if already present
                let in2 = input.strip_prefix('/').unwrap_or(&input);
                let has_prefix = if in2 == prefix {
                    true
                } else {
                    in2.starts_with(&format!("{prefix}/"))
                };

                if !has_prefix {
                    let add_slash = input.is_empty() || input.ends_with('/');
                    input = path::join(&[prefix.as_str(), input.as_str()]);

                    if add_slash {
                        input.push('/');
                    }
                }
            }
        }

        nh_common::paths::url::make_permalink(&base_url, &input)
    }

    // Go: helpers/url.go:getBaseURLRoot
    fn get_base_url_root(&self, path: &str) -> String {
        if path.starts_with('/') {
            // Treat it as relative to the server root.
            self.cfg.base_url().without_path
        } else {
            // Treat it as relative to the baseURL.
            self.cfg.base_url().with_path
        }
    }

    /// Go: `IsAbsURL(in)` — a `url.Parse` error is `false` here (see [`PathSpec::try_is_abs_url`]).
    // Go: helpers/url.go:IsAbsURL
    pub fn is_abs_url(&self, input: &str) -> bool {
        self.try_is_abs_url(input).unwrap_or(false)
    }

    /// Go: `IsAbsURL(in) (bool, error)`.
    // Go: helpers/url.go:IsAbsURL
    pub fn try_is_abs_url(&self, input: &str) -> Result<bool> {
        // Fast path.
        if input.starts_with("http://") || input.starts_with("https://") {
            return Ok(true);
        }
        let u = go_url::parse(input).map_err(|e| Error::new(e.to_string()))?;
        Ok(u.is_abs())
    }

    // Go: helpers/url.go:RelURL
    pub fn rel_url(&self, input: &str, add_language: bool) -> String {
        let is_abs = match self.try_is_abs_url(input) {
            Ok(b) => b,
            Err(_) => return input.to_string(),
        };
        let base_url = self.get_base_url_root(input);
        let canonify_urls = self.cfg.canonify_urls();

        if (!input.starts_with(base_url.as_str()) && is_abs) || input.starts_with("//") {
            return input.to_string();
        }

        let mut u = input.to_string();

        if input.starts_with(base_url.as_str()) {
            u = u[base_url.len()..].to_string();
        }

        if add_language {
            let prefix = self.get_language_prefix();
            if !prefix.is_empty() {
                // avoid adding language prefix if already present
                let in2 = input.strip_prefix('/').unwrap_or(input);
                let has_prefix = if in2 == prefix {
                    true
                } else {
                    in2.starts_with(&format!("{prefix}/"))
                };

                if !has_prefix {
                    let had_slash = u.ends_with('/');

                    u = path::join(&[prefix.as_str(), u.as_str()]);

                    if had_slash {
                        u.push('/');
                    }
                }
            }
        }

        if !canonify_urls {
            u = nh_common::paths::url::add_context_root(&base_url, &u);
        }

        if input.is_empty() && !u.ends_with('/') && base_url.ends_with('/') {
            u.push('/');
        }

        if !u.starts_with('/') {
            u.insert(0, '/');
        }

        u
    }

    /// Go: `PrependBasePath(rel, isAbs)` — prepends any baseURL sub-folder to the given resource.
    // Go: helpers/url.go:PrependBasePath
    pub fn prepend_base_path(&self, rel: &str, is_abs: bool) -> String {
        let base_path = self.get_base_path(!is_abs);
        let mut rel = rel.to_string();
        if !base_path.is_empty() {
            rel = filepath::to_slash(&rel).to_string();
            // Need to prepend any path from the baseURL
            let had_slash = rel.ends_with('/');
            rel = path::join(&[base_path.as_str(), rel.as_str()]);
            if had_slash {
                rel.push('/');
            }
        }
        rel
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/url.go (189 lines; 7/8 funcs executed)
// OK L30-32: (p *PathSpec) URLize(uri string) string
// OK L36-38: (p *PathSpec) URLizeFilename(filename string) string
// OK L41-50: (p *PathSpec) URLEscape(uri string) string
// OK L53-92: (p *PathSpec) AbsURL(in string, addLanguage bool) string
// OK L94-102: (p *PathSpec) getBaseURLRoot(path string) string
// OK L104-114: (p *PathSpec) IsAbsURL(in string) (bool, error)
// OK L116-174: (p *PathSpec) RelURL(in string, addLanguage bool) string
// OK L177-189: (p *PathSpec) PrependBasePath(rel string, isAbs bool) string
// ---------------------------------------------------------------------------
