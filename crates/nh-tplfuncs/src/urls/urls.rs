//! Port of `tpl/urls/urls.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{GoString, HostCtx, Kind, Map, MapType, Object, SliceType, Value};
use nh_common::cast::caste;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

// Parity notes: `urlize` = PathSpec.URLize (uppercase %XX); `relLangURL`/`absLangURL` = PathSpec.RelURL/AbsURL(addLanguage) with the canonifyURLs branch; `urls.Parse` returns a `*url.URL` object (String, IsAbs, Path, RawQuery, Fragment) over go-url; `ref` -> page.Ref(argsm).

/// Go: `urls.Namespace` (template value `*urls.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    multihost: bool,
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn sv(b: impl Into<GoString>) -> Value {
    Value::String(b.into())
}

fn to_string_e(v: &Value) -> GoResult<GoString> {
    Ok(caste::to_string_e(v)?)
}

/// Go: `*url.URL` as a template value (the exported fields and methods of `net/url.URL`).
#[derive(Clone, Debug)]
pub struct UrlObject(pub go_url::Url);

fn bytes(b: &[u8]) -> Value {
    Value::String(GoString::from(b.to_vec()))
}

/// Go: `url.Values` (`map[string][]string`).
fn values_to_value(v: &go_url::Values) -> Value {
    let mut m = Map::new(MapType::Named(Arc::from("url.Values")));
    for (k, vs) in &v.0 {
        m.entries.insert(
            GoString::from(k.clone()),
            Value::list(SliceType::String, vs.iter().map(|s| bytes(s)).collect()),
        );
    }
    Value::map(m)
}

/// Go: `*url.Userinfo` as a template value.
#[derive(Clone, Debug)]
pub struct UserinfoObject(pub go_url::Userinfo);

nh_common::go_methods!(UserinfoObject {
    "Password" => |u, _c, a| {
        args::exactly(a, 0, "Password")?;
        // Go returns (string, bool); a template call of a two-result method that is not
        // (value, error) fails in text/template.
        let _ = u;
        Err(go_value::Error::new(
            "can't call method/function \"Password\" with 2 results",
        ))
    },
    "String" => |u, _c, a| { args::exactly(a, 0, "String")?; Ok(bytes(&u.0.string())) },
    "Username" => |u, _c, a| { args::exactly(a, 0, "Username")?; Ok(bytes(u.0.username())) },
});

impl Object for UserinfoObject {
    nh_common::object_basics!("*url.Userinfo");
    fn kind(&self) -> Kind {
        Kind::Ptr
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.string()))
    }
}

fn url_arg(a: &[Value], i: usize) -> GoResult<go_url::Url> {
    match a.get(i) {
        Some(v @ Value::Object(o)) => o
            .as_any()
            .downcast_ref::<UrlObject>()
            .map(|u| u.0.clone())
            .ok_or_else(|| args::wrong_type("*url.URL", v)),
        Some(Value::Invalid) => Err(go_value::Error::new(
            "runtime error: invalid memory address or nil pointer dereference",
        )),
        Some(v) => Err(args::wrong_type("*url.URL", v)),
        None => Err(go_value::Error::new(format!("missing argument {i}"))),
    }
}

fn url_err(e: go_url::Error) -> go_value::Error {
    go_value::Error::new(e.to_string())
}

nh_common::go_methods!(UrlObject {
    "AppendBinary" => |_u, _c, _a| Err(go_value::Error::new(
        "neohugo-rs: url.URL.AppendBinary is not supported in templates",
    )),
    "EscapedFragment" => |u, _c, a| { args::exactly(a, 0, "EscapedFragment")?; Ok(bytes(&u.0.escaped_fragment())) },
    "EscapedPath" => |u, _c, a| { args::exactly(a, 0, "EscapedPath")?; Ok(bytes(&u.0.escaped_path())) },
    "Hostname" => |u, _c, a| { args::exactly(a, 0, "Hostname")?; Ok(bytes(u.0.hostname())) },
    "IsAbs" => |u, _c, a| { args::exactly(a, 0, "IsAbs")?; Ok(Value::Bool(u.0.is_abs())) },
    "JoinPath" => |u, _c, a| {
        let mut elems: Vec<GoString> = Vec::new();
        for i in 0..a.len() {
            elems.push(args::string(a, i)?);
        }
        let parts: Vec<&[u8]> = elems.iter().map(|s| s.as_bytes()).collect();
        Ok(Value::object(UrlObject(u.0.join_path(&parts))))
    },
    "MarshalBinary" => |u, _c, a| {
        args::exactly(a, 0, "MarshalBinary")?;
        Ok(Value::list(SliceType::Uint8, u.0.marshal_binary().into_iter().map(|b| Value::Uint(b as u64, go_value::UintKind::Uint8)).collect()))
    },
    "Parse" => |u, _c, a| {
        args::exactly(a, 1, "Parse")?;
        let r = args::string(a, 0)?;
        u.0.parse(r.as_bytes()).map(|x| Value::object(UrlObject(x))).map_err(url_err)
    },
    "Port" => |u, _c, a| { args::exactly(a, 0, "Port")?; Ok(bytes(u.0.port())) },
    "Query" => |u, _c, a| { args::exactly(a, 0, "Query")?; Ok(values_to_value(&u.0.query())) },
    "Redacted" => |u, _c, a| { args::exactly(a, 0, "Redacted")?; Ok(bytes(&u.0.redacted())) },
    "RequestURI" => |u, _c, a| { args::exactly(a, 0, "RequestURI")?; Ok(bytes(&u.0.request_uri())) },
    "ResolveReference" => |u, _c, a| {
        args::exactly(a, 1, "ResolveReference")?;
        let r = url_arg(a, 0)?;
        Ok(Value::object(UrlObject(u.0.resolve_reference(&r))))
    },
    "String" => |u, _c, a| { args::exactly(a, 0, "String")?; Ok(bytes(&u.0.string())) },
    "UnmarshalBinary" => |_u, _c, _a| Err(go_value::Error::new(
        "neohugo-rs: url.URL.UnmarshalBinary is not supported in templates",
    )),
});

impl Object for UrlObject {
    nh_common::object_basics!("*url.URL");
    fn kind(&self) -> Kind {
        Kind::Ptr
    }
    fn field(&self, name: &str) -> Option<Value> {
        let u = &self.0;
        Some(match name {
            "Scheme" => bytes(&u.scheme),
            "Opaque" => bytes(&u.opaque),
            "User" => match &u.user {
                Some(ui) => Value::object(UserinfoObject(ui.clone())),
                None => Value::TypedNil(Arc::from("*url.Userinfo")),
            },
            "Host" => bytes(&u.host),
            "Path" => bytes(&u.path),
            "RawPath" => bytes(&u.raw_path),
            "OmitHost" => Value::Bool(u.omit_host),
            "ForceQuery" => Value::Bool(u.force_query),
            "RawQuery" => bytes(&u.raw_query),
            "Fragment" => bytes(&u.fragment),
            "RawFragment" => bytes(&u.raw_fragment),
            _ => return None,
        })
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.string()))
    }
}

impl Namespace {
    /// New returns a new instance of the urls-namespaced template functions.
    // Go: tpl/urls/urls.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let multihost = d.conf.is_multihost();
        Namespace { d, multihost }
    }

    /// AbsLangURL the string s and converts it to an absolute URL according to a page's
    /// position in the project directory structure and the current language.
    // Go: tpl/urls/urls.go:AbsLangURL
    pub fn abs_lang_url(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "AbsLangURL")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(self
            .d
            .path_spec()
            .abs_url(&ss.to_str_lossy(), !self.multihost)))
    }

    /// AbsURL takes the string s and converts it to an absolute URL.
    // Go: tpl/urls/urls.go:AbsURL
    pub fn abs_url(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "AbsURL")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(self.d.path_spec().abs_url(&ss.to_str_lossy(), false)))
    }

    /// Anchorize creates sanitized anchor name version of the string s that is compatible with
    /// how your configured markdown renderer does it.
    // Go: tpl/urls/urls.go:Anchorize
    pub fn anchorize(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Anchorize")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(self
            .d
            .content_spec()
            .sanitize_anchor_name(&ss.to_str_lossy())))
    }

    /// JoinPath joins the provided elements into a URL string and cleans the result of any ./
    /// or ../ elements. If the argument list is empty, JoinPath returns an empty string.
    // Go: tpl/urls/urls.go:JoinPath
    pub fn join_path(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        if a.is_empty() {
            return Ok(sv(""));
        }

        let mut selements: Vec<GoString> = Vec::new();
        for e in a {
            match e {
                Value::List(l) if l.ty == SliceType::String => {
                    for s in &l.items {
                        if let Value::String(s) = s {
                            selements.push(s.clone());
                        }
                    }
                }
                Value::List(l) if l.ty == SliceType::Any => {
                    for s in &l.items {
                        selements.push(to_string_e(s)?);
                    }
                }
                Value::TypedNil(t) if &**t == "[]string" || &**t == "[]interface {}" => {}
                _ => selements.push(to_string_e(e)?),
            }
        }

        if selements.is_empty() {
            // Go panics on `url.JoinPath(selements[0], selements[1:]...)` of an empty slice; the
            // compiled code evaluates the slice expression first.
            return Err(gerr("runtime error: slice bounds out of range [1:0]"));
        }
        let rest: Vec<&[u8]> = selements[1..].iter().map(|s| s.as_bytes()).collect();
        let result = go_url::join_path(selements[0].as_bytes(), &rest).map_err(url_err)?;
        Ok(sv(result))
    }

    /// Parse parses rawurl into a URL structure. The rawurl may be relative or absolute.
    // Go: tpl/urls/urls.go:Parse
    pub fn parse(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Parse")?;
        let s = caste::to_string_e(&a[0])
            .map_err(|e| gerr(format!("error in Parse: {}", e.message())))?;
        go_url::parse(s.as_bytes())
            .map(|u| Value::object(UrlObject(u)))
            .map_err(url_err)
    }

    /// Ref returns the absolute URL path to a given content item from Page p.
    // Go: tpl/urls/urls.go:Ref
    pub fn ref_(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Ref")?;
        self.ref_impl(ctx, &a[0], &a[1], "Ref")
    }

    /// RelRef returns the relative URL path to a given content item from Page p.
    // Go: tpl/urls/urls.go:RelRef
    pub fn rel_ref(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "RelRef")?;
        self.ref_impl(ctx, &a[0], &a[1], "RelRef")
    }

    /// Go's `p.(urls.RefLinker)` check and the `Ref`/`RelRef` call.
    fn ref_impl(&self, ctx: HostCtx<'_>, p: &Value, a: &Value, method: &str) -> GoResult<Value> {
        let page = nh_page::page::page_from_value(p);
        let obj = match p {
            Value::Object(o) if o.has_method("Ref") && o.has_method("RelRef") => Some(o.clone()),
            _ => None,
        };
        if page.is_none() && obj.is_none() {
            return Err(gerr(format!("invalid Page received in {method}")));
        }
        let argsm = self.ref_args_to_map(a)?;
        if let Some(page) = page {
            let r = if method == "Ref" {
                page.0.ref_(&argsm)
            } else {
                page.0.rel_ref(&argsm)
            };
            return r.map(sv).map_err(Into::into);
        }
        let obj = obj.expect("checked above");
        match obj.call_method(ctx, method, &[Value::map(argsm)]) {
            Some(r) => r,
            None => Err(gerr(format!("invalid Page received in {method}"))),
        }
    }

    // Go: tpl/urls/urls.go:refArgsToMap
    fn ref_args_to_map(&self, a: &Value) -> GoResult<Map> {
        let mut s = GoString::empty();
        let mut of = GoString::empty();

        let mut v = a.clone();
        if matches!(&v, Value::List(l) if l.ty == SliceType::Any)
            || matches!(&v, Value::TypedNil(t) if &**t == "[]interface {}")
        {
            v = Value::list(
                SliceType::String,
                caste::to_string_slice(&v)
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            );
        }

        match &v {
            Value::Map(m) if m.ty == MapType::StringAny => return Ok(Map::clone(m)),
            Value::Map(m) if m.ty == MapType::StringString => {
                let mut out = Map::new(MapType::StringAny);
                for (k, vv) in &m.entries {
                    out.entries.insert(k.clone(), vv.clone());
                }
                return Ok(out);
            }
            Value::TypedNil(t) if &**t == "map[string]interface {}" => {
                return Ok(Map::new(MapType::StringAny));
            }
            Value::TypedNil(t) if &**t == "map[string]string" => {
                return Ok(Map::new(MapType::StringAny));
            }
            Value::List(l) if l.ty == SliceType::String => {
                if l.items.is_empty() || l.items.len() > 2 {
                    return Err(gerr("invalid number of arguments to ref"));
                }
                // These were the options before we introduced the map type:
                s = to_string_e(&l.items[0])?;
                if l.items.len() == 2 {
                    of = to_string_e(&l.items[1])?;
                }
            }
            Value::TypedNil(t) if &**t == "[]string" => {
                return Err(gerr("invalid number of arguments to ref"));
            }
            _ => {
                s = to_string_e(a)?;
            }
        }

        let mut m = Map::new(MapType::StringAny);
        m.entries.insert(GoString::from("path"), Value::String(s));
        m.entries
            .insert(GoString::from("outputFormat"), Value::String(of));
        Ok(m)
    }

    /// RelLangURL takes the string s and prepends the relative path according to a page's
    /// position in the project directory structure and the current language.
    // Go: tpl/urls/urls.go:RelLangURL
    pub fn rel_lang_url(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "RelLangURL")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(self
            .d
            .path_spec()
            .rel_url(&ss.to_str_lossy(), !self.multihost)))
    }

    /// RelURL takes the string s and prepends the relative path according to a page's position
    /// in the project directory structure.
    // Go: tpl/urls/urls.go:RelURL
    pub fn rel_url(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "RelURL")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(self.d.path_spec().rel_url(&ss.to_str_lossy(), false)))
    }

    /// URLDecode does the inverse transformation of QueryEscape, converting each 3-byte
    /// encoded substring of the form "%AB" into the hex-decoded byte 0xAB.
    // Go: tpl/urls/urls.go:URLDecode
    pub fn url_decode(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "URLDecode")?;
        let s = to_string_e(&a[0])?;
        match go_url::query_unescape(s.as_bytes()) {
            Ok(d) => Ok(sv(d)),
            // this mean, we cannot urldecode this string then just return as it was
            Err(_) => Ok(Value::String(s)),
        }
    }

    /// URLEncode escapes the string so it can be safely placed inside a URL query.
    // Go: tpl/urls/urls.go:URLEncode
    pub fn url_encode(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "URLEncode")?;
        let s = to_string_e(&a[0])?;
        Ok(Value::html(go_url::query_escape(s.as_bytes())))
    }

    /// URLize returns the strings s formatted as an URL.
    // Go: tpl/urls/urls.go:URLize
    pub fn ur_lize(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "URLize")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(self.d.path_spec().urlize(&ss.to_str_lossy())))
    }
}

nh_common::go_methods!(Namespace {
    "AbsLangURL" => |n, ctx, a| n.abs_lang_url(ctx, a),
    "AbsURL" => |n, ctx, a| n.abs_url(ctx, a),
    "Anchorize" => |n, ctx, a| n.anchorize(ctx, a),
    "JoinPath" => |n, ctx, a| n.join_path(ctx, a),
    "Parse" => |n, ctx, a| n.parse(ctx, a),
    "Ref" => |n, ctx, a| n.ref_(ctx, a),
    "RelLangURL" => |n, ctx, a| n.rel_lang_url(ctx, a),
    "RelRef" => |n, ctx, a| n.rel_ref(ctx, a),
    "RelURL" => |n, ctx, a| n.rel_url(ctx, a),
    "URLDecode" => |n, ctx, a| n.url_decode(ctx, a),
    "URLEncode" => |n, ctx, a| n.url_encode(ctx, a),
    "URLize" => |n, ctx, a| n.ur_lize(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*urls.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/urls/urls.go (254 lines; 7/14 funcs executed)
//   types: Namespace
// OK L29-34: New(deps *deps.Deps) *Namespace
// OK L43-50: (ns *Namespace) AbsURL(s any) (string, error)
// OK L54-61: (ns *Namespace) Parse(rawurl any) (*url.URL, error)
// OK L65-72: (ns *Namespace) RelURL(s any) (string, error)
// OK L75-81: (ns *Namespace) URLize(s any) (string, error)
// OK L85-91: (ns *Namespace) Anchorize(s any) (string, error)
// OK L94-105: (ns *Namespace) Ref(p any, args any) (string, error)
// OK L108-120: (ns *Namespace) RelRef(p any, args any) (string, error)
// OK L122-164: (ns *Namespace) refArgsToMap(args any) (map[string]any, error)
// OK L168-175: (ns *Namespace) RelLangURL(s any) (string, error)
// OK L180-187: (ns *Namespace) AbsLangURL(s any) (string, error)
// OK L192-224: (ns *Namespace) JoinPath(elements ...any) (string, error)
// OK L228-235: (ns *Namespace) URLEncode(rawurl interface{}) (template.HTML, error)
// OK L240-254: (ns *Namespace) URLDecode(rawurl interface{}) (string, error)
// ---------------------------------------------------------------------------
