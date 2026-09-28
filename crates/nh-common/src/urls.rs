//! Port of `common/urls/baseURL.go`, `common/urls/ref.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

//! Go `common/urls`: `BaseURL` (compiled once from `baseURL`, trailing `/` forced on the path).

use crate::herrors::Result;

/// Go: `urls.BaseURL`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaseURL {
    /// The parsed URL (Go `url *url.URL`); stored as its `String()` form, re-parse with go-url.
    pub url: String,
    /// e.g. `https://seeksnack.com/`
    pub with_path: String,
    /// e.g. `https://seeksnack.com`
    pub with_path_no_trailing_slash: String,
    /// e.g. `https://seeksnack.com`
    pub without_path: String,
    /// e.g. `/`
    pub base_path: String,
    /// e.g. ``
    pub base_path_no_trailing_slash: String,
}

impl BaseURL {
    /// Go: `BaseURL.String()` = `WithPath`.
    pub fn string(&self) -> &str {
        &self.with_path
    }

    /// Go: `BaseURL.Path()`.
    pub fn path(&self) -> &str {
        todo!()
    }

    /// Go: `BaseURL.HostURL()`.
    pub fn host_url(&self) -> String {
        todo!()
    }

    /// Go: `BaseURL.WithProtocol(protocol)`.
    pub fn with_protocol(&self, protocol: &str) -> Result<BaseURL> {
        todo!()
    }
}

/// Go: `urls.NewBaseURLFromString(b string) (BaseURL, error)`.
// Go: common/urls/baseURL.go:NewBaseURLFromString
pub fn new_base_url_from_string(b: &str) -> Result<BaseURL> {
    todo!()
}

/// Go: `urls.RefLinker` interface (`Ref(args map[string]any, source any) (string, error)`).
pub trait RefLinker {
    fn ref_(&self, args: &go_value::Map, source: &go_value::Value) -> Result<String>;
    fn rel_ref(&self, args: &go_value::Map, source: &go_value::Value) -> Result<String>;
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/urls/baseURL.go (112 lines; 4/9 funcs executed)
//   types: BaseURL
// EX L34-36: (b BaseURL) String() string
//    L38-40: (b BaseURL) Path() string
//    L42-45: (b BaseURL) Port() int
//    L48-50: (b BaseURL) HostURL() string
//    L54-76: (b BaseURL) WithProtocol(protocol string) (BaseURL, error)
//    L78-82: (b BaseURL) WithPort(port int) (BaseURL, error)
// EX L86-89: (b BaseURL) URL() *url.URL
// EX L91-97: NewBaseURLFromString(b string) (BaseURL, error)
// EX L99-112: newBaseURLFromURL(u *url.URL) (BaseURL, error)
// Source: common/urls/ref.go (22 lines; 0/0 funcs executed)
//   types: RefLinker
// ---------------------------------------------------------------------------
