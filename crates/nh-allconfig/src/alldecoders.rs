//! Port of `config/allconfig/alldecoders.go`.
//!
//! Owner: Wave B task T09 (allconfig-modules).

//! Go `alldecoders.go`: one decoder per top-level key, run in (weight, key) order.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use go_value::{Map, MapType, Value};
use nh_common::loggers::Logger;
use nh_common::maps::params::{clean_config_string_map, clean_config_string_map_string};
use nh_common::{Error, Result};
use nh_config::config_provider::Provider;
use nh_config::decode::weak_decode_into;
use nh_hugofs::afero::Fs;

use crate::allconfig::{
    Config, GoSlice, LanguagesMap, UglyUrls, create_default_output_formats, get_string_map_opt,
    get_string_map_string_opt, quote,
};

/// Go: `decodeWeight`.
pub struct DecodeWeight {
    pub key: &'static str,
    pub weight: i64,
    pub internal_or_deprecated: bool,
    pub decode: fn(&DecodeWeight, &mut DecodeConfig<'_>) -> Result<()>,
    /// Go: `getCompiler(c).CompileConfig(logger)`.
    pub compile: Option<fn(&mut Config, &Logger) -> Result<()>>,
}

/// Go: `decodeConfig`.
pub struct DecodeConfig<'a> {
    pub p: &'a dyn Provider,
    pub c: &'a mut Config,
    pub fs: &'a dyn Fs,
    pub bcfg: nh_config::common_config::BaseConfig,
    pub logger: &'a Logger,
}

fn dw(
    key: &'static str,
    weight: i64,
    decode: fn(&DecodeWeight, &mut DecodeConfig<'_>) -> Result<()>,
) -> DecodeWeight {
    DecodeWeight {
        key,
        weight,
        internal_or_deprecated: false,
        decode,
        compile: None,
    }
}

/// `p.p.GetStringMap(key)` (an empty map for Go's nil).
fn string_map(p: &dyn Provider, key: &str) -> Map {
    get_string_map_opt(p, key).unwrap_or_else(|| Map::new(MapType::StringAny))
}

fn s(v: &go_value::GoString) -> String {
    v.to_str_lossy().into_owned()
}

/// Go: `allDecoderSetups` (sorted by (weight, key) at use).
pub fn all_decoder_setups() -> &'static [DecodeWeight] {
    static S: OnceLock<Vec<DecodeWeight>> = OnceLock::new();
    S.get_or_init(build_decoder_setups)
}

// Go: config/allconfig/alldecoders.go:init
fn build_decoder_setups() -> Vec<DecodeWeight> {
    let mut all = vec![
        dw("", -100, |_d, p| {
            // Always first.
            weak_decode_into(&p.p.get(""), &mut p.c.root)?;

            // This need to match with Lang which is always lower case.
            p.c.root.default_content_language =
                go_unicode::strings::to_lower_str(&p.c.root.default_content_language).into_owned();

            Ok(())
        }),
        dw("imaging", 0, |d, p| {
            let input = string_map(p.p, d.key);
            let ns = nh_images::config::decode_config(&input)?;
            // Go merges the imaging defaults into the config tree's own map (images.DecodeConfig
            // mutates its input); `fromLoadConfigResult` replays that write (see
            // `allconfig::write_back_imaging`).
            p.c.imaging = Some(Arc::new(ns));
            Ok(())
        }),
        dw("caches", 0, |d, p| {
            let r = nh_helpers::cache::filecache::filecache_config::decode_config(
                p.fs,
                &p.bcfg,
                &string_map(p.p, d.key),
            );
            let caches = match r {
                Ok(c) => c,
                Err(e) => return Err(e),
            };
            p.c.caches = caches;
            if p.c.root.ignore_cache {
                // Set MaxAge in all caches to 0.
                for cache in p.c.caches.values_mut() {
                    cache.max_age = go_time::Duration(0);
                }
            }
            Ok(())
        }),
        dw("httpcache", 0, |d, p| {
            p.c.http_cache =
                nh_helpers::cache::httpcache::httpcache::decode_config(&string_map(p.p, d.key))?;
            if p.c.root.ignore_cache {
                p.c.http_cache.cache.for_.excludes = vec!["**".to_string()];
                p.c.http_cache.cache.for_.includes = Vec::new();
            }
            Ok(())
        }),
        DecodeWeight {
            compile: Some(|c, _logger| c.build.compile_config()),
            ..dw("build", 0, |_d, p| {
                p.c.build = nh_config::common_config::decode_build_config(p.p);
                Ok(())
            })
        },
        dw("frontmatter", 0, |_d, p| {
            p.c.frontmatter = nh_page::pagemeta::page_frontmatter::decode_front_matter_config(p.p)?;
            Ok(())
        }),
        dw("markup", 0, |_d, p| {
            p.c.markup = nh_markup::markup_config::decode(p.p)?;
            Ok(())
        }),
        dw("segments", 0, |d, p| {
            let input = match get_string_map_opt(p.p, d.key) {
                Some(m) => Value::map(m),
                None => Value::TypedNil(Arc::from("map[string]interface {}")),
            };
            p.c.segments = Some(Arc::new(crate::segments::decode_segments(&input)?));
            Ok(())
        }),
        DecodeWeight {
            compile: Some(|c, _logger| c.server.compile_config()),
            ..dw("server", 0, |_d, p| {
                p.c.server = nh_config::common_config::decode_server(p.p)?;
                Ok(())
            })
        },
        dw("minify", 0, |d, p| {
            p.c.minify = nh_transform::minifiers::config::decode_config(&p.p.get(d.key))?;
            Ok(())
        }),
        dw("contenttypes", 100, |d, p| {
            // This needs to be decoded after media types.
            let media_types = p.c.media_types.clone().expect("media types decoded");
            p.c.content_types = Some(Arc::new(nh_media::media::config::decode_content_types(
                &string_map(p.p, d.key),
                &media_types.config,
            )?));
            Ok(())
        }),
        dw("mediatypes", 0, |d, p| {
            p.c.media_types = Some(Arc::new(nh_media::media::config::decode_types(
                &string_map(p.p, d.key),
            )?));
            Ok(())
        }),
        dw("outputs", 0, |_d, p| {
            let output_formats = p.c.output_formats.clone().expect("output formats decoded");
            let defaults = create_default_output_formats(&output_formats.config);
            let m = clean_config_string_map(&string_map(p.p, "outputs"));
            let mut outputs: BTreeMap<String, Vec<String>> = BTreeMap::new();
            for (k, v) in m.entries.iter() {
                let s: Vec<String> = nh_common::types::convert::to_string_slice_preserve_string(v)
                    .iter()
                    .map(|v| go_unicode::strings::to_lower_str(&v.to_str_lossy()).into_owned())
                    .collect();
                outputs.insert(s_(k), s);
            }
            // Apply defaults.
            for (k, v) in defaults {
                outputs.entry(k).or_insert(v);
            }
            p.c.outputs = outputs;
            Ok(())
        }),
        dw("outputformats", 0, |d, p| {
            let media_types = p.c.media_types.clone().expect("media types decoded");
            p.c.output_formats = Some(Arc::new(nh_media::output::config::decode_config(
                &media_types.config,
                &p.p.get(d.key),
            )?));
            Ok(())
        }),
        dw("params", 0, |_d, p| {
            let mut params = clean_config_string_map(&string_map(p.p, "params"));
            params.ty = MapType::Params;

            // Before Hugo 0.112.0 this was configured via site Params.
            if let Some(main_sections) = params.get(b"mainsections") {
                let ms: Vec<String> =
                    nh_common::types::convert::to_string_slice_preserve_string(main_sections)
                        .iter()
                        .map(s)
                        .collect();
                p.c.root.main_sections = GoSlice::from_vec(ms);
            }
            p.c.params = Arc::new(params);

            Ok(())
        }),
        dw("module", 0, |_d, p| {
            p.c.module = nh_hugofs::modules::config::decode_config(p.p)?;
            Ok(())
        }),
        dw("permalinks", 0, |d, p| {
            p.c.permalinks =
                nh_page::permalinks::decode_permalinks_config(&string_map(p.p, d.key))?;
            Ok(())
        }),
        dw("sitemap", 0, |d, p| {
            if p.p.is_set(d.key) {
                p.c.sitemap = nh_config::common_config::decode_sitemap(
                    p.c.sitemap.clone(),
                    &string_map(p.p, d.key),
                )?;
            }
            Ok(())
        }),
        dw("taxonomies", 0, |d, p| {
            if p.p.is_set(d.key) {
                let m = get_string_map_string_opt(p.p, d.key)
                    .unwrap_or_else(|| Map::new(MapType::StringString));
                let m = clean_config_string_map_string(&m);
                p.c.taxonomies = m
                    .entries
                    .iter()
                    .map(|(k, v)| (s(k), v.as_go_string().map(s).unwrap_or_default()))
                    .collect();
            }
            Ok(())
        }),
        dw("related", 100, |d, p| {
            // This needs to be decoded after taxonomies.
            if p.p.is_set(d.key) {
                let m = match p.p.get_params(d.key) {
                    Some(m) => Value::map(m),
                    None => Value::Invalid,
                };
                p.c.related = nh_page::related::decode_config(&m)
                    .map_err(|e| Error::new(format!("failed to decode related config: {e}")))?;
            } else {
                p.c.related = nh_page::related::Config::default_config();
                if p.c.taxonomies.contains_key("tag") {
                    p.c.related.add(nh_page::related::IndexConfig {
                        name: "tags".to_string(),
                        weight: 80,
                        type_: nh_page::related::TYPE_BASIC.to_string(),
                        ..Default::default()
                    });
                }
            }
            Ok(())
        }),
        dw("languages", 0, |d, p| {
            let mut m = string_map(p.p, d.key);
            if m.len() == 1 {
                // In v0.112.4 we moved this to the language config, but it's very commmon for
                // mono language sites to have this at the top level.
                let first = m.entries.iter().find_map(|(k, v)| match v {
                    Value::Map(pm) if pm.ty == MapType::Params => Some((k.clone(), (**pm).clone())),
                    _ => None,
                });
                if let Some((k, mut first)) = first
                    && first.get(b"languagecode").is_none()
                {
                    let lc = Value::string(p.p.get_string("languagecode"));
                    first.insert("languagecode", lc.clone());
                    m.insert(k.clone(), Value::map(first));
                    // Go writes into the map of the config tree.
                    p.p.set(&format!("{}.{}.languagecode", d.key, s(&k)), lc);
                }
            }
            let languages = nh_langs::config::decode_config(&m)?;
            p.c.languages = LanguagesMap::new(languages);

            // Validate defaultContentLanguage.
            if !p.c.root.default_content_language.is_empty()
                && !p
                    .c
                    .languages
                    .contains_key(&p.c.root.default_content_language)
            {
                return Err(Error::new(format!(
                    "config value {} for defaultContentLanguage does not match any language definition",
                    quote(&p.c.root.default_content_language)
                )));
            }

            Ok(())
        }),
        dw("cascade", 0, |d, p| {
            p.c.cascade = Some(Arc::new(nh_page::page_matcher::decode_cascade_config(
                true,
                &p.p.get(d.key),
            )?));
            Ok(())
        }),
        dw("menus", 0, |d, p| {
            p.c.menus = Some(Arc::new(
                nh_page::navigation::menu::decode_config_namespace(&p.p.get(d.key))?,
            ));
            Ok(())
        }),
        DecodeWeight {
            compile: Some(|c, _logger| c.page.compile_config()),
            ..dw("page", 0, |d, p| {
                p.c.page = nh_config::common_config::PageConfig {
                    next_prev_sort_order: "desc".to_string(),
                    next_prev_in_section_sort_order: "desc".to_string(),
                };
                if p.p.is_set(d.key) {
                    weak_decode_into(&p.p.get(d.key), &mut p.c.page)?;
                }
                Ok(())
            })
        },
        dw("pagination", 0, |d, p| {
            p.c.pagination = nh_config::common_config::Pagination {
                pager_size: 10,
                path: "page".to_string(),
                ..Default::default()
            };
            if p.p.is_set(d.key) {
                weak_decode_into(&p.p.get(d.key), &mut p.c.pagination)?;
            }
            Ok(())
        }),
        dw("privacy", 0, |_d, p| {
            p.c.privacy = nh_config::privacy::decode_config(p.p)?;
            Ok(())
        }),
        dw("security", 0, |_d, p| {
            p.c.security = nh_config::security::security_config::decode_config(p.p)?;
            Ok(())
        }),
        dw("services", 0, |_d, p| {
            p.c.services = nh_config::services::decode_config(p.p)?;
            Ok(())
        }),
        dw("deployment", 0, |_d, p| {
            p.c.deployment = crate::deployconfig::decode_config(p.p)?;
            Ok(())
        }),
        DecodeWeight {
            internal_or_deprecated: true,
            ..dw("author", 0, |d, p| {
                p.c.author = get_string_map_opt(p.p, d.key).map(|m| clean_config_string_map(&m));
                Ok(())
            })
        },
        DecodeWeight {
            internal_or_deprecated: true,
            ..dw("social", 0, |d, p| {
                p.c.social = get_string_map_string_opt(p.p, d.key)
                    .map(|m| clean_config_string_map_string(&m));
                Ok(())
            })
        },
        DecodeWeight {
            internal_or_deprecated: true,
            ..dw("uglyurls", 0, |d, p| {
                let v = p.p.get(d.key);
                let bools = |m: Map| -> BTreeMap<String, bool> {
                    m.entries
                        .iter()
                        .map(|(k, v)| (s(k), matches!(v, Value::Bool(true))))
                        .collect()
                };
                p.c.ugly_urls = match &v {
                    Value::Bool(vv) => UglyUrls::Bool(*vv),
                    Value::String(vv) => UglyUrls::Bool(vv.as_bytes() == b"true"),
                    Value::Map(vv) if vv.ty == MapType::Params => {
                        UglyUrls::Sections(bools(nh_common::maps::maps::to_string_map_bool(
                            &Value::map(clean_config_string_map(vv)),
                        )))
                    }
                    _ => match nh_common::cast::caste::to_string_map_bool_e(&v) {
                        Ok(m) => UglyUrls::Sections(bools(m)),
                        // A nil map[string]bool.
                        Err(_) => UglyUrls::Nil,
                    },
                };
                Ok(())
            })
        },
        DecodeWeight {
            internal_or_deprecated: true,
            ..dw("internal", 0, |d, p| {
                weak_decode_into(&Value::map(string_map(p.p, d.key)), &mut p.c.internal)
            })
        },
    ];

    // Go: init() verifies the keys are lower case; the vector is the map in key order.
    all.sort_by(|a, b| a.key.cmp(b.key));
    for v in &all {
        debug_assert_eq!(v.key, v.key.to_lowercase());
    }
    all
}

fn s_(k: &go_value::GoString) -> String {
    k.to_str_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/allconfig/alldecoders.go (469 lines; 1/1 funcs executed)
//   types: decodeConfig, decodeWeight
// OK L455-469: init()
// ---------------------------------------------------------------------------
