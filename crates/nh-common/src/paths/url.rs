//! Port of `common/paths/url.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).
//!
//! The URL functions parse with go-url (`net/url`). Go panics when parsing fails; the plain
//! functions panic the same way and each has a `try_` form that returns Go's panic value as an
//! error. `url.URL.String()` of a parsed `&str` is always valid UTF-8, but a decoded `URL.Path` is
//! not when the input percent-encodes invalid UTF-8: those results are explicit errors
//! (`AddContextRoot`), see PORTING.md.

use go_path::{filepath, path};

use super::path::{FilepathPathBridge, file_and_ext_bridge, prettify_path};
use crate::herrors::{Error, Result};

/// Go: `pathBridge` (the `path` package flavour of `filepathPathBridge`).
pub(crate) struct PathBridge;

impl FilepathPathBridge for PathBridge {
    // Go: common/paths/url.go:(pathBridge).Base
    fn base<'a>(&self, s: &'a str) -> &'a str {
        path::base(s)
    }
    // Go: common/paths/url.go:(pathBridge).Clean
    fn clean(&self, s: &str) -> String {
        path::clean(s)
    }
    // Go: common/paths/url.go:(pathBridge).Dir
    fn dir(&self, s: &str) -> String {
        path::dir(s)
    }
    // Go: common/paths/url.go:(pathBridge).Ext
    fn ext<'a>(&self, s: &'a str) -> &'a str {
        path::ext(s)
    }
    // Go: common/paths/url.go:(pathBridge).Join
    fn join(&self, elem: &[&str]) -> String {
        path::join(elem)
    }
    // Go: common/paths/url.go:(pathBridge).Separator
    fn separator(&self) -> &'static str {
        "/"
    }
}

/// Go: `pb`.
pub(crate) const PB: PathBridge = PathBridge;

fn url_err(e: go_url::Error) -> Error {
    Error::new(e.to_string())
}

/// Go: `paths.MakePermalink(host, plink string) *url.URL` — returns the URL's `String()` form
/// (all callers immediately call `.String()`): `base.Path = path.Join(base.Path, p.Path)`, copy
/// fragment/query, restore a trailing slash if `plink==""` and host ends with `/`, or p.Path ends `/`.
// Go: common/paths/url.go:MakePermalink
pub fn make_permalink(host: &str, plink: &str) -> String {
    match try_make_permalink_url(host, plink) {
        Ok(u) => String::from_utf8(u.string()).expect("URL.String of a parsed str is UTF-8"),
        Err(e) => panic!("{e}"),
    }
}

/// [`make_permalink`] returning the `*url.URL` (Go's panic value as an error).
pub fn try_make_permalink_url(host: &str, plink: &str) -> Result<go_url::Url> {
    let mut base = go_url::parse(host).map_err(url_err)?;

    let p = go_url::parse(plink).map_err(url_err)?;

    if !p.host.is_empty() {
        return Err(Error::new(format!(
            "can't make permalink from absolute link {}",
            go_strconv::quote(plink)
        )));
    }

    base.path = path::join_bytes(&[&base.path, &p.path]);
    base.fragment = p.fragment;
    base.raw_query = p.raw_query;

    // path.Join will strip off the last /, so put it back if it was there.
    let had_trailing_slash = (plink.is_empty() && host.ends_with('/')) || p.path.ends_with(b"/");
    if had_trailing_slash && !base.path.ends_with(b"/") {
        base.path.push(b'/');
    }

    Ok(base)
}

/// Go: `paths.AddContextRoot(baseURL, relativePath)` — adds the context root to an URL if it's
/// not already set. Panics like Go on a bad base URL (see [`try_add_context_root`]).
// Go: common/paths/url.go:AddContextRoot
pub fn add_context_root(base_url: &str, relative_path: &str) -> String {
    let b = match try_add_context_root(base_url, relative_path) {
        Ok(b) => b,
        Err(e) => panic!("{e}"),
    };
    match String::from_utf8(b) {
        Ok(s) => s,
        Err(_) => panic!(
            "neohugo-rs: AddContextRoot({base_url:?}, {relative_path:?}): the decoded base path is not valid UTF-8"
        ),
    }
}

/// [`add_context_root`] with Go's byte result (the decoded base path may hold any bytes) and Go's
/// panic value as an error.
pub fn try_add_context_root(base_url: &str, relative_path: &str) -> Result<Vec<u8>> {
    let url = go_url::parse(base_url).map_err(url_err)?;

    let mut new_path = path::join_bytes(&[&url.path[..], relative_path.as_bytes()]);

    // path strips trailing slash, ignore root path.
    if new_path != b"/" && relative_path.ends_with('/') {
        new_path.push(b'/');
    }
    Ok(new_path)
}

/// Go: `paths.PrettifyURL` — a semantic, clean URL.
// Go: common/paths/url.go:PrettifyURL
pub fn prettify_url(in_: &str) -> String {
    let x = prettify_url_path(in_);

    if path::base(&x) == "index.html" {
        return path::dir(&x);
    }

    if in_.is_empty() {
        return "/".to_string();
    }

    x
}

/// Go: `paths.PrettifyURLPath` — `/section/name.html` becomes `/section/name/index.html`,
/// `/section/name/` becomes `/section/name/index.html`.
// Go: common/paths/url.go:PrettifyURLPath
pub fn prettify_url_path(p: &str) -> String {
    prettify_path(p, &PB)
}

/// Go: `paths.Uglify` — the opposite of [`prettify_url_path`].
// Go: common/paths/url.go:Uglify
pub fn uglify(p: &str) -> String {
    if path::ext(p).is_empty() {
        if p.len() < 2 {
            return "/".to_string();
        }
        // /section/name/  -> /section/name.html
        return format!("{}.html", path::clean(p));
    }

    let (name, ext) = file_and_ext_bridge(p, &PB);
    if name == "index" {
        // /section/name/index.html -> /section/name.html
        let d = path::dir(p);
        if d.len() > 1 {
            return format!("{d}{ext}");
        }
        return p.to_string();
    }
    // /.xml -> /index.xml
    if name.is_empty() {
        return format!("{}index{}", path::dir(p), ext);
    }
    // /section/name.html -> /section/name.html
    path::clean(p)
}

/// Go: `paths.URLEscape(uri)` = `url.Parse(uri).String()` (panics on error in Go).
// Go: common/paths/url.go:URLEscape
pub fn url_escape(uri: &str) -> String {
    match try_url_escape(uri.as_bytes()) {
        Ok(v) => String::from_utf8(v).expect("URL.String of a parsed str is UTF-8"),
        Err(e) => panic!("{e}"),
    }
}

/// [`url_escape`] over Go string bytes, returning Go's panic value as an error.
pub fn try_url_escape(uri: &[u8]) -> Result<Vec<u8>> {
    let u = go_url::parse(uri).map_err(url_err)?;
    Ok(u.string())
}

/// Go: `paths.TrimExt` — trims the extension from a path.
// Go: common/paths/url.go:TrimExt
pub fn trim_ext(p: &str) -> String {
    let e = path::ext(p);
    p.strip_suffix(e).unwrap_or(p).to_string()
}

/// Go: `paths.UrlFromFilename` — a `file://` URL from an absolute (unix) filename.
// Go: common/paths/url.go:UrlFromFilename
pub fn url_from_filename(filename: &str) -> Result<go_url::Url> {
    if !filepath::is_abs(filename) {
        return Err(Error::new("filepath must be absolute"));
    }
    // No Windows volume names on unix.

    // /path/to/file
    // becomes
    // file:///path/to/file
    Ok(go_url::Url {
        scheme: b"file".to_vec(),
        path: filepath::to_slash(filename).as_bytes().to_vec(),
        ..Default::default()
    })
}

/// Go: `paths.UrlStringToFilename` — the filename of the URL `s` (unix flavour).
// Go: common/paths/url.go:UrlStringToFilename
pub fn url_string_to_filename(s: &str) -> (Vec<u8>, bool) {
    let u = match go_url::parse_request_uri(s) {
        Ok(u) => u,
        Err(_) => return (filepath::from_slash(s).as_bytes().to_vec(), false),
    };

    let p = u.path;

    if p.is_empty() {
        let p = go_url::query_unescape(&u.opaque).unwrap_or_default();
        return (p, false);
    }

    // runtime.GOOS != "windows"
    (p, true)
}

/// Go: `helpers.(*PathSpec).IsAbsURL` (the skeleton placed it here; there is no `paths.IsAbsURL`):
/// `http://`/`https://` prefixes, else `url.Parse(in).IsAbs()`; a parse error is `false`.
pub fn is_abs_url(s: &str) -> bool {
    try_is_abs_url(s).unwrap_or(false)
}

/// [`is_abs_url`] with the `url.Parse` error.
// Go: helpers/url.go:IsAbsURL
pub fn try_is_abs_url(s: &str) -> Result<bool> {
    // Fast path.
    if s.starts_with("http://") || s.starts_with("https://") {
        return Ok(true);
    }
    let u = go_url::parse(s).map_err(url_err)?;
    Ok(u.is_abs())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/paths/url.go (273 lines; 5/15 funcs executed)
//   types: pathBridge
// OK L27-29: (pathBridge) Base(in string) string
// OK L31-33: (pathBridge) Clean(in string) string
// OK L35-37: (pathBridge) Dir(in string) string
// OK L39-41: (pathBridge) Ext(in string) string
// OK L43-45: (pathBridge) Join(elem ...string) string
// OK L47-49: (pathBridge) Separator() string
// OK L59-85: MakePermalink(host, plink string) *url.URL
// OK L90-103: AddContextRoot(baseURL, relativePath string) string
// OK L108-120: PrettifyURL(in string) string
// OK L128-130: PrettifyURLPath(in string) string
// OK L137-161: Uglify(in string) string
// OK L164-171: URLEscape(uri string) string
// OK L174-176: TrimExt(in string) string
// OK L179-229: UrlFromFilename(filename string) (*url.URL, error) (unix: no volume names)
// OK L233-273: UrlStringToFilename(s string) (string, bool) (unix)
// ---------------------------------------------------------------------------
