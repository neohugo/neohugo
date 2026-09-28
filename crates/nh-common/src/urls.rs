//! Port of `common/urls/baseURL.go`, `common/urls/ref.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

//! Go `common/urls`: `BaseURL` (compiled once from `baseURL`, trailing `/` forced on the path).
//! A BaseURL in Hugo is normally on the form scheme://path, but the form scheme: is also valid
//! (mailto:hugo@rules.com).

use crate::herrors::{Error, Result};

/// Go: `urls.BaseURL`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BaseURL {
    /// The parsed URL (Go `url *url.URL`).
    pub url: go_url::Url,
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

fn utf8(b: Vec<u8>, what: &str) -> Result<String> {
    String::from_utf8(b).map_err(|_| {
        Error::new(format!(
            "neohugo-rs: a baseURL whose {what} is not valid UTF-8 is not supported"
        ))
    })
}

impl BaseURL {
    /// Go: `BaseURL.String()` = `WithPath`.
    // Go: common/urls/baseURL.go:String
    pub fn string(&self) -> &str {
        &self.with_path
    }

    /// Go: `BaseURL.Path()` (the decoded URL path, checked to be UTF-8 on construction).
    // Go: common/urls/baseURL.go:Path
    pub fn path(&self) -> &str {
        std::str::from_utf8(&self.url.path).expect("BaseURL path is checked to be UTF-8")
    }

    /// Go: `BaseURL.Port()` — `p, _ := strconv.Atoi(url.Port())` (0 on a syntax error, the
    /// clamped value on a range error).
    // Go: common/urls/baseURL.go:Port
    pub fn port(&self) -> i64 {
        go_strconv::internal::atoi(self.url.port()).0
    }

    /// Go: `BaseURL.HostURL()` — the URL to the host root without any path elements.
    // Go: common/urls/baseURL.go:HostURL
    pub fn host_url(&self) -> String {
        let s = self.string();
        s.strip_suffix(self.path()).unwrap_or(s).to_string()
    }

    /// Go: `BaseURL.WithProtocol(protocol)` — the BaseURL prefixed with the given protocol,
    /// normally of the form "scheme://", i.e. "webcal://".
    // Go: common/urls/baseURL.go:WithProtocol
    pub fn with_protocol(&self, protocol: &str) -> Result<BaseURL> {
        let mut u = self.url();

        let mut scheme = protocol;
        let is_full_protocol = scheme.ends_with("://");
        let is_opaque_protocol = scheme.ends_with(':');

        if is_full_protocol {
            scheme = scheme.strip_suffix("://").unwrap_or(scheme);
        } else if is_opaque_protocol {
            scheme = scheme.strip_suffix(':').unwrap_or(scheme);
        }

        u.scheme = scheme.as_bytes().to_vec();

        if is_full_protocol && !u.opaque.is_empty() {
            let mut o = b"//".to_vec();
            o.extend_from_slice(&u.opaque);
            u.opaque = o;
        } else if is_opaque_protocol && u.opaque.is_empty() {
            return Err(Error::new(format!(
                "cannot determine BaseURL for protocol {}",
                go_strconv::quote(protocol)
            )));
        }

        new_base_url_from_url(u)
    }

    /// Go: `BaseURL.WithPort(port)`.
    // Go: common/urls/baseURL.go:WithPort
    pub fn with_port(&self, port: i64) -> Result<BaseURL> {
        let mut u = self.url();
        let mut host = u.hostname().to_vec();
        host.push(b':');
        host.extend_from_slice(go_strconv::itoa(port).as_bytes());
        u.host = host;
        new_base_url_from_url(u)
    }

    /// Go: `BaseURL.URL()` — a copy of the internal URL.
    // Go: common/urls/baseURL.go:URL
    pub fn url(&self) -> go_url::Url {
        self.url.clone()
    }
}

/// Go: `urls.NewBaseURLFromString(b string) (BaseURL, error)`.
// Go: common/urls/baseURL.go:NewBaseURLFromString
pub fn new_base_url_from_string(b: &str) -> Result<BaseURL> {
    let u = go_url::parse(b).map_err(|e| Error::new(e.to_string()))?;
    new_base_url_from_url(u)
}

// Go: common/urls/baseURL.go:newBaseURLFromURL
fn new_base_url_from_url(mut u: go_url::Url) -> Result<BaseURL> {
    // A baseURL should always have a trailing slash, see #11669.
    if !u.path.ends_with(b"/") {
        u.path.push(b'/');
    }
    let path = utf8(u.path.clone(), "decoded path")?;
    let with_path = utf8(u.string(), "URL")?;
    let with_path_no_trailing_slash = with_path
        .strip_suffix('/')
        .unwrap_or(&with_path)
        .to_string();
    let mut base_url_no_path = u.clone();
    base_url_no_path.path = Vec::new();
    let without_path = utf8(base_url_no_path.string(), "URL")?;
    let base_path_no_trailing_slash = path.strip_suffix('/').unwrap_or(&path).to_string();

    Ok(BaseURL {
        url: u,
        with_path,
        with_path_no_trailing_slash,
        without_path,
        base_path: path,
        base_path_no_trailing_slash,
    })
}

/// Go: `urls.RefLinker` interface (`Ref(args map[string]any) (string, error)`) — implemented by
/// those who support reference linking. args must contain a path, but can also point to the
/// target language or output format.
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
// OK L34-36: (b BaseURL) String() string
// OK L38-40: (b BaseURL) Path() string
// OK L42-45: (b BaseURL) Port() int
// OK L48-50: (b BaseURL) HostURL() string
// OK L54-76: (b BaseURL) WithProtocol(protocol string) (BaseURL, error)
// OK L78-82: (b BaseURL) WithPort(port int) (BaseURL, error)
// OK L86-89: (b BaseURL) URL() *url.URL
// OK L91-97: NewBaseURLFromString(b string) (BaseURL, error)
// OK L99-112: newBaseURLFromURL(u *url.URL) (BaseURL, error)
// Source: common/urls/ref.go (22 lines; 0/0 funcs executed)
//   types: RefLinker
// ---------------------------------------------------------------------------
