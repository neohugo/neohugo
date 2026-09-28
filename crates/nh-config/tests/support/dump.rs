//! The dump format of `tools/go-oracle/nh-config/cval` for Rust values (shared by the T04
//! nh-config tests).

use std::collections::BTreeMap;

use go_value::{Map, Value};
use nh_config::common_config::{
    BuildConfig, Headers, PageConfig, Pagination, Redirect, Server, SitemapConfig,
};
use nh_config::decode::{AnyValue, Int64, Uint64};
use nh_config::security::whitelist::Whitelist;
use nh_config::{privacy, security, services};
use serde_json::{Value as J, json};

use super::{encode, str_enc};

// ---------------------------------------------------------------------------
// Dumps (the format of tools/go-oracle/nh-config/cval)

pub trait Dump {
    fn dump(&self) -> J;
}

impl Dump for String {
    fn dump(&self) -> J {
        str_enc(self.as_bytes())
    }
}
impl Dump for bool {
    fn dump(&self) -> J {
        json!(self)
    }
}
macro_rules! dump_num {
    ($($t:ty),*) => {$(impl Dump for $t { fn dump(&self) -> J { json!(self) } })*};
}
dump_num!(i64, i32, i16, i8, u64, u32, u16, u8);
impl Dump for Int64 {
    fn dump(&self) -> J {
        json!(self.0)
    }
}
impl Dump for Uint64 {
    fn dump(&self) -> J {
        json!(self.0)
    }
}
impl Dump for go_time::Duration {
    fn dump(&self) -> J {
        json!(self.0)
    }
}
impl Dump for f64 {
    fn dump(&self) -> J {
        json!({"f": format!("{:016x}", self.to_bits())})
    }
}
impl Dump for f32 {
    fn dump(&self) -> J {
        json!({"f": format!("{:016x}", f64::from(*self).to_bits())})
    }
}
impl Dump for Value {
    fn dump(&self) -> J {
        encode(self)
    }
}
impl Dump for AnyValue {
    fn dump(&self) -> J {
        encode(&self.0)
    }
}
impl<T: Dump> Dump for Vec<T> {
    fn dump(&self) -> J {
        J::Array(self.iter().map(Dump::dump).collect())
    }
}
impl<T: Dump, const N: usize> Dump for [T; N] {
    fn dump(&self) -> J {
        J::Array(self.iter().map(Dump::dump).collect())
    }
}
impl<T: Dump> Dump for Option<Box<T>> {
    fn dump(&self) -> J {
        match self {
            Some(b) => b.dump(),
            None => J::Null,
        }
    }
}
pub fn dump_map<T: Dump>(go_type: &str, m: &BTreeMap<String, T>) -> J {
    json!({
        "@map": go_type,
        "entries": m.iter().map(|(k, v)| json!([str_enc(k.as_bytes()), v.dump()])).collect::<Vec<_>>(),
    })
}
impl Dump for Map {
    fn dump(&self) -> J {
        json!({
            "@map": self.ty.go_name(),
            "entries": self.entries.iter().map(|(k, v)| json!([str_enc(k), encode(v)])).collect::<Vec<_>>(),
        })
    }
}

macro_rules! dump_struct {
    ($ty:ty, $go:expr, |$s:ident| { $($name:expr => $val:expr),* $(,)? }) => {
        impl Dump for $ty {
            fn dump(&self) -> J {
                let $s = self;
                let mut o = serde_json::Map::new();
                o.insert("@".to_string(), J::String($go.to_string()));
                $(o.insert($name.to_string(), $val);)*
                J::Object(o)
            }
        }
    };
}

#[allow(unused_imports)]
pub(crate) use dump_struct;

// ---------------------------------------------------------------------------
// nh-config's structs

dump_struct!(Pagination, "config.Pagination", |s| {
    "PagerSize" => s.pager_size.dump(), "Path" => s.path.dump(), "DisableAliases" => s.disable_aliases.dump(),
});
dump_struct!(PageConfig, "config.PageConfig", |s| {
    "NextPrevSortOrder" => s.next_prev_sort_order.dump(),
    "NextPrevInSectionSortOrder" => s.next_prev_in_section_sort_order.dump(),
});
dump_struct!(SitemapConfig, "config.SitemapConfig", |s| {
    "ChangeFreq" => s.change_freq.dump(), "Priority" => s.priority.dump(),
    "Filename" => s.filename.dump(), "Disable" => s.disable.dump(),
});
dump_struct!(BuildConfig, "config.BuildConfig", |s| {
    "UseResourceCacheWhen" => s.use_resource_cache_when.dump(),
    "BuildStats" => json!({
        "@": "config.BuildStats", "Enable": s.build_stats.enable, "DisableTags": s.build_stats.disable_tags,
        "DisableClasses": s.build_stats.disable_classes, "DisableIDs": s.build_stats.disable_ids,
    }),
    "NoJSConfigInAssets" => s.no_js_config_in_assets.dump(),
    "CacheBusters" => J::Array(s.cache_busters.iter().map(|c| json!({
        "@": "config.CacheBuster", "Source": c.source.dump(), "Target": c.target.dump(),
    })).collect()),
});
dump_struct!(Headers, "config.Headers", |s| { "For" => s.for_.dump(), "Values" => s.values.dump() });
dump_struct!(Redirect, "config.Redirect", |s| {
    "From" => s.from.dump(), "FromRe" => s.from_re.dump(), "To" => s.to.dump(),
    "FromHeaders" => dump_map("map[string]string", &s.from_headers),
    "Status" => s.status.dump(), "Force" => s.force.dump(),
});
dump_struct!(Server, "config.Server", |s| { "Headers" => s.headers.dump(), "Redirects" => s.redirects.dump() });
dump_struct!(Whitelist, "security.Whitelist", |s| {
    "acceptNone" => json!(s.accept_none()),
    "patterns" => match s.patterns_strings() {
        None => J::Null,
        Some(p) => J::Array(p.iter().map(|x| str_enc(x.as_bytes())).collect()),
    },
});
dump_struct!(security::security_config::Config, "security.Config", |s| {
    "Exec" => json!({"@": "security.Exec", "Allow": s.exec.allow.dump(), "OsEnv": s.exec.os_env.dump()}),
    "Funcs" => json!({"@": "security.Funcs", "Getenv": s.funcs.getenv.dump()}),
    "HTTP" => json!({"@": "security.HTTP", "URLs": s.http.urls.dump(), "Methods": s.http.methods.dump(), "MediaTypes": s.http.media_types.dump()}),
    "EnableInlineShortcodes" => s.enable_inline_shortcodes.dump(),
});
dump_struct!(services::Config, "services.Config", |s| {
    "Disqus" => json!({"@": "services.Disqus", "Shortname": s.disqus.shortname.dump()}),
    "GoogleAnalytics" => json!({"@": "services.GoogleAnalytics", "ID": s.google_analytics.id.dump()}),
    "Instagram" => json!({"@": "services.Instagram", "DisableInlineCSS": s.instagram.disable_inline_css, "AccessToken": s.instagram.access_token.dump()}),
    "Twitter" => json!({"@": "services.Twitter", "DisableInlineCSS": s.twitter.disable_inline_css}),
    "X" => json!({"@": "services.X", "DisableInlineCSS": s.x.disable_inline_css}),
    "RSS" => json!({"@": "services.RSS", "Limit": s.rss.limit}),
});
pub fn service(t: &str, disable: bool, extra: &[(&str, bool)]) -> J {
    let mut o = json!({"@": t, "Service": {"@": "privacy.Service", "Disable": disable}});
    for (k, v) in extra {
        o[*k] = json!(v);
    }
    o
}
dump_struct!(privacy::Config, "privacy.Config", |s| {
    "Disqus" => service("privacy.Disqus", s.disqus.service.disable, &[]),
    "GoogleAnalytics" => service("privacy.GoogleAnalytics", s.google_analytics.service.disable, &[("RespectDoNotTrack", s.google_analytics.respect_do_not_track)]),
    "Instagram" => service("privacy.Instagram", s.instagram.service.disable, &[("Simple", s.instagram.simple)]),
    "Twitter" => service("privacy.Twitter", s.twitter.service.disable, &[("EnableDNT", s.twitter.enable_dnt), ("Simple", s.twitter.simple)]),
    "Vimeo" => service("privacy.Vimeo", s.vimeo.service.disable, &[("EnableDNT", s.vimeo.enable_dnt), ("Simple", s.vimeo.simple)]),
    "YouTube" => service("privacy.YouTube", s.youtube.service.disable, &[("PrivacyEnhanced", s.youtube.privacy_enhanced)]),
    "X" => service("privacy.X", s.x.service.disable, &[("EnableDNT", s.x.enable_dnt), ("Simple", s.x.simple)]),
});
