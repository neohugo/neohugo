//! Port of `common/paths/url.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).


/// Go: `paths.MakePermalink(host, plink string) *url.URL` — returns the URL's `String()` form
/// (all callers immediately call `.String()`): `base.Path = path.Join(base.Path, p.Path)`, copy
/// fragment/query, restore a trailing slash if `plink==""` and host ends with `/`, or p.Path ends `/`.
// Go: common/paths/url.go:MakePermalink
pub fn make_permalink(host: &str, plink: &str) -> String {
    todo!("go-url")
}

/// Go: `paths.AddContextRoot(baseURL, relativePath)`.
// Go: common/paths/url.go:AddContextRoot
pub fn add_context_root(base_url: &str, relative_path: &str) -> String { todo!() }

/// Go: `paths.URLEscape(uri)` = `url.Parse(uri).String()` (panics on error in Go).
// Go: common/paths/url.go:URLEscape
pub fn url_escape(uri: &str) -> String { todo!() }

// Go: common/paths/url.go:TrimExt
pub fn trim_ext(p: &str) -> String { todo!() }

/// Go: `paths.PrettifyURLPath` / `Uglify`.
pub fn prettify_url_path(p: &str) -> String { todo!() }
pub fn uglify(p: &str) -> String { todo!() }

/// Go: `paths.IsAbsURL`.
pub fn is_abs_url(s: &str) -> bool { todo!() }

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/paths/url.go (273 lines; 5/15 funcs executed)
//   types: pathBridge
// EX L27-29: (pathBridge) Base(in string) string
//    L31-33: (pathBridge) Clean(in string) string
//    L35-37: (pathBridge) Dir(in string) string
// EX L39-41: (pathBridge) Ext(in string) string
//    L43-45: (pathBridge) Join(elem ...string) string
// EX L47-49: (pathBridge) Separator() string
// EX L59-85: MakePermalink(host, plink string) *url.URL
//    L90-103: AddContextRoot(baseURL, relativePath string) string
//    L108-120: PrettifyURL(in string) string
//    L128-130: PrettifyURLPath(in string) string
//    L137-161: Uglify(in string) string
//    L164-171: URLEscape(uri string) string
// EX L174-176: TrimExt(in string) string
//    L179-229: UrlFromFilename(filename string) (*url.URL, error)
//    L233-273: UrlStringToFilename(s string) (string, bool)
// ---------------------------------------------------------------------------
