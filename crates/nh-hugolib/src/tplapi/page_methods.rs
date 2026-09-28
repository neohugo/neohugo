//! Module `tplapi::page_methods`.
//!
//! NEW: template-visible method set of page.Page (replaces reflection on *pageState)
//!
//! Owner: Wave B task T23 (hugolib-site).

//! The template-visible method set of a page (Go reflection on `*pageState` / `pageWithWeight0` /
//! `*pageState` via `page.Page`). `nh_resource::resourcetypes::Resource` and `nh_page::page::Page`
//! are implemented for `crate::page::PageHandle` here, with ONE dispatch table
//! ([`call_page_method`]) and one method-name list per Go type.
//!
//! Members seeksnack uses (templates-inventory §3.1 / A.2, exact case): AllTranslations,
//! AlternativeOutputFormats, Content (ctx), Data, Date, Description, File, IsHome, IsNode, IsPage,
//! IsSection, IsTranslated, Kind, Lang, Language, Lastmod, OutputFormats, Page, Pages, Paginate,
//! Paginator, Params, Parent, Permalink, Plain (ctx), PublishDate, RegularPages, RelPermalink,
//! Resources, Scratch, Section, Site, Sitemap, Title, Translations, Type (+ via render hooks:
//! GetPage, Resources on `PageInner`; via shortcodes: Ref/RelRef). The whole Go method set is
//! here (the lists below come from Go's reflect and `tests/site.rs` checks them against the
//! `site` oracle).
//! Go type names for `%T`: `*hugolib.pageState`, `hugolib.pageWithWeight0`, `*hugolib.pageWithOrdinal`,
//! `*hugolib.pageForShortcode`, `*hugolib.pageForRenderHooks` (see `crate::page::PageWrapper`),
//! `*hugolib.pageHeadingsFiltered` ([`PageHeadingsFiltered`]).
//!
//! Method-set rules for the wrappers (HUGO_LAYER.md §5.1):
//! * A Go wrapper struct exposes the methods of its EMBEDDED INTERFACES plus its own methods,
//!   not the full `*pageState` method set. `pageForShortcode` / `pageForRenderHooks` =
//!   `page.PageWithoutContent` (delegating to the page) + `TableOfContentsProvider` (the page;
//!   `pageForShortcode` overrides `TableOfContents` with the TOC placeholder) + `MarkupProvider`
//!   and `ContentProvider` from `page.NopPage` (`.Content`, `.Plain`, `.Summary`, ... are the
//!   nop results) + `Unwrapv`/`String`. `pageWithWeight0` / `pageWithOrdinal` embed `page.Page`
//!   (+ `Weight0()` / `Ordinal()`). Build the table per wrapper, not by filtering `*pageState`.
//! * A struct field shadows a promoted method of the same name at a deeper level (Go selector
//!   depth rule), e.g. `hugolib.aliasPage{Permalink string; page.Page}`: `.Permalink` is the field.
//!
//! Nil results (HUGO_LAYER.md §4.7): `Parent` (and other methods declared to return the
//! `page.Page` interface) return `TypedNil("page.Page")` when Go returns a nil interface — it
//! prints `<nil>` in text templates and becomes `Invalid` when passed to a func or stored
//! (Scratch/dict), so `jsonLd.html:389` (`.Parent` stored in a Scratch, then `.Parent` on the
//! `Get` result) yields nothing, as in Go. `GetPage` of a missing ref returns the NopPage OBJECT
//! (Go `page.NilPage` = `(*nopPage)(nil)`, a non-nil interface holding a nil pointer: methods
//! work, `IsZero` is true, prints `<nil>`), never `Invalid`. `Sitemap` returns
//! `Value::object(SitemapConfig)` (nh-config implements `Object` for it). Page lists built by
//! appending (the collections, `.Translations`, `.GetTerms`, `.Resources`, ...) are Go's nil
//! slice when empty.
//!
//! Arguments: Go counts the injected `context.Context` in its argument errors
//! (`tplapi::values::arity`).

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Map, MapType, SliceType, Time, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::object::{GoResult, args};
use nh_common::paths::pathparser::Path;
use nh_config::common_config::SitemapConfig;
use nh_helpers::source::file_info::{File, FileObject};
use nh_langs::language::{Language, LanguageObject};
use nh_markup::tableofcontents::Headings;
use nh_media::media::media_type::MediaType;
use nh_page::page::{Page, PageRef, Pages, pages_to_value};
use nh_page::page_outputformat::OutputFormats;
use nh_page::related::{IndexConfig, Keyword};
use nh_page::site::SiteRef;
use nh_resource::resourcetypes::{Resource, Resources};

use super::objects::{fragments_value, headings_value};
use super::values::{
    arity, arity_variadic, bad_results, nil_of, page_or_nil, pages_nil_if_empty, pages_opt_value,
    strings_nil_if_empty, strings_opt_value, unsupported,
};
use crate::page::{PageHandle, PageWrapper};
use crate::page__per_output::PageContentOutput;

/// The exported method set of `*hugolib.pageState` (Go reflect, sorted). The same set is
/// `*hugolib.pageHeadingsFiltered`'s; `hugolib.pageWithWeight0` adds `Unwrapv` and `Weight0`,
/// `*hugolib.pageWithOrdinal` adds `Ordinal`.
pub const PAGE_STATE_METHODS: &[&str] = &[
    "Aliases",
    "AllTranslations",
    "AlternativeOutputFormats",
    "Ancestors",
    "ApplyFilterToHeadings",
    "BundleType",
    "CodeOwners",
    "Content",
    "ContentWithoutSummary",
    "CurrentSection",
    "Data",
    "Date",
    "Description",
    "Draft",
    "Eq",
    "ExpiryDate",
    "File",
    "FirstSection",
    "ForEeachIdentity",
    "Fragments",
    "FuzzyWordCount",
    "GetDependencyManager",
    "GetDependencyManagerForScope",
    "GetDependencyManagerForScopesAll",
    "GetIdentity",
    "GetInternalRelatedDocsHandler",
    "GetInternalTemplateBasePathAndDescriptor",
    "GetPage",
    "GetTerms",
    "GitInfo",
    "Group",
    "HasMenuCurrent",
    "HasShortcode",
    "HeadingsFiltered",
    "IdentifierBase",
    "InSection",
    "IsAncestor",
    "IsDescendant",
    "IsHome",
    "IsMenuCurrent",
    "IsNode",
    "IsPage",
    "IsSection",
    "IsTranslated",
    "Key",
    "Keywords",
    "Kind",
    "Lang",
    "Language",
    "Lastmod",
    "Layout",
    "Len",
    "LinkTitle",
    "MarkStale",
    "Markup",
    "MarshalJSON",
    "MediaType",
    "Menus",
    "Name",
    "Next",
    "NextInSection",
    "NextPage",
    "OutputFormats",
    "Page",
    "Pages",
    "PagesRecursive",
    "Paginate",
    "Paginator",
    "Param",
    "Params",
    "Parent",
    "Path",
    "PathInfo",
    "Permalink",
    "Plain",
    "PlainWords",
    "Prev",
    "PrevInSection",
    "PrevPage",
    "PublishDate",
    "RawContent",
    "ReadingTime",
    "Ref",
    "RefFrom",
    "RegularPages",
    "RegularPagesRecursive",
    "RelPermalink",
    "RelRef",
    "RelRefFrom",
    "RelatedKeywords",
    "Render",
    "RenderShortcodes",
    "RenderString",
    "ResourceType",
    "Resources",
    "Scratch",
    "Section",
    "Sections",
    "SectionsEntries",
    "SectionsPath",
    "Site",
    "Sitemap",
    "Sites",
    "Slice",
    "Slug",
    "StaleVersion",
    "Store",
    "String",
    "Summary",
    "TableOfContents",
    "Title",
    "TranslationKey",
    "Translations",
    "Truncated",
    "Type",
    "Weight",
    "WordCount",
];

/// The exported method set of `*hugolib.pageForShortcode` and `*hugolib.pageForRenderHooks`
/// (Go reflect, sorted): `page.PageWithoutContent` + the TOC, markup and content providers +
/// `Unwrapv`/`String`.
pub const PAGE_FOR_SHORTCODE_METHODS: &[&str] = &[
    "Aliases",
    "AllTranslations",
    "AlternativeOutputFormats",
    "Ancestors",
    "BundleType",
    "CodeOwners",
    "Content",
    "ContentWithoutSummary",
    "CurrentSection",
    "Data",
    "Date",
    "Description",
    "Draft",
    "Eq",
    "ExpiryDate",
    "File",
    "FirstSection",
    "Fragments",
    "FuzzyWordCount",
    "GetPage",
    "GetTerms",
    "GitInfo",
    "HasMenuCurrent",
    "HasShortcode",
    "HeadingsFiltered",
    "InSection",
    "IsAncestor",
    "IsDescendant",
    "IsHome",
    "IsMenuCurrent",
    "IsNode",
    "IsPage",
    "IsSection",
    "IsTranslated",
    "Keywords",
    "Kind",
    "Lang",
    "Language",
    "Lastmod",
    "Layout",
    "Len",
    "LinkTitle",
    "Markup",
    "MediaType",
    "Menus",
    "Name",
    "Next",
    "NextInSection",
    "NextPage",
    "OutputFormats",
    "Page",
    "Pages",
    "Paginate",
    "Paginator",
    "Param",
    "Params",
    "Parent",
    "Path",
    "PathInfo",
    "Permalink",
    "Plain",
    "PlainWords",
    "Prev",
    "PrevInSection",
    "PrevPage",
    "PublishDate",
    "RawContent",
    "ReadingTime",
    "Ref",
    "RefFrom",
    "RegularPages",
    "RegularPagesRecursive",
    "RelPermalink",
    "RelRef",
    "RelRefFrom",
    "RelatedKeywords",
    "Render",
    "RenderShortcodes",
    "RenderString",
    "ResourceType",
    "Resources",
    "Scratch",
    "Section",
    "Sections",
    "SectionsEntries",
    "SectionsPath",
    "Site",
    "Sitemap",
    "Sites",
    "Slug",
    "Store",
    "String",
    "Summary",
    "TableOfContents",
    "Title",
    "TranslationKey",
    "Translations",
    "Truncated",
    "Type",
    "Unwrapv",
    "Weight",
    "WordCount",
];

/// Go: `reflect.Type.MethodByName(name)` for the Go type of a page handle.
pub fn page_has_method(wrapper: PageWrapper, name: &str) -> bool {
    match wrapper {
        PageWrapper::None => PAGE_STATE_METHODS.binary_search(&name).is_ok(),
        PageWrapper::Weight0(_) => {
            matches!(name, "Unwrapv" | "Weight0") || PAGE_STATE_METHODS.binary_search(&name).is_ok()
        }
        PageWrapper::Ordinal(_) => {
            name == "Ordinal" || PAGE_STATE_METHODS.binary_search(&name).is_ok()
        }
        PageWrapper::ForShortcode | PageWrapper::ForRenderHooks => {
            PAGE_FOR_SHORTCODE_METHODS.binary_search(&name).is_ok()
        }
    }
}

/// The `*pageState` value of a (possibly wrapped) handle.
fn unwrapped(p: &PageHandle) -> PageHandle {
    PageHandle {
        h: p.h.clone(),
        id: p.id,
        wrapper: PageWrapper::None,
    }
}

/// The page's current content provider (Go's embedded `ContentProvider` of the current page
/// output: the `pco`, a lazy provider's, or `None` for `page.NopPage`).
fn content_provider(p: &PageHandle) -> Option<Arc<PageContentOutput>> {
    // `None` before the first shift too (Go: `nopPageOutput`'s `page.NopPage`).
    p.state().current_output_opt()?.content_renderer()
}

/// Whether the content-provider methods of this handle are `page.NopPage`'s (the shortcode and
/// render-hook page wrappers).
fn nop_content(p: &PageHandle) -> bool {
    matches!(
        p.wrapper,
        PageWrapper::ForShortcode | PageWrapper::ForRenderHooks
    )
}

fn html(s: impl Into<GoString>) -> Value {
    Value::html(s.into())
}

/// A `map[string]any` parameter (Go `validateType`: a map whose type is assignable, e.g.
/// `maps.Params` or a dict; nil is the zero map).
fn map_arg(a: &[Value], i: usize) -> GoResult<Map> {
    match a.get(i) {
        Some(Value::Map(m)) if !matches!(m.ty, MapType::StringString) => Ok((**m).clone()),
        Some(Value::Invalid) | Some(Value::TypedNil(_)) => Ok(Map::new(MapType::StringAny)),
        Some(v) => Err(args::wrong_type("map[string]interface {}", v)),
        None => Err(go_value::Error::new(format!("missing argument {i}"))),
    }
}

/// A `*navigation.MenuEntry` parameter.
fn menu_entry_arg(
    a: &[Value],
    i: usize,
) -> GoResult<Option<Arc<nh_page::navigation::menu::MenuEntry>>> {
    match a.get(i) {
        Some(Value::Invalid) => Ok(None),
        Some(v) => match v.downcast::<nh_page::navigation::menu::MenuEntryRef>() {
            Some(m) => Ok(Some(m.0.clone())),
            None if matches!(v, Value::TypedNil(t) if &**t == nh_page::navigation::menu::MENU_ENTRY_TYPE) => {
                Ok(None)
            }
            None => Err(args::wrong_type(
                nh_page::navigation::menu::MENU_ENTRY_TYPE,
                v,
            )),
        },
        None => Err(go_value::Error::new(format!("missing argument {i}"))),
    }
}

/// The template call of method `name` on a page handle (the caller checked the method set).
/// `hf` holds the headings of a `*hugolib.pageHeadingsFiltered`.
pub fn call_page_method(
    p: &PageHandle,
    ctx: HostCtx<'_>,
    name: &str,
    a: &[Value],
    hf: Option<&Headings>,
) -> GoResult<Value> {
    let ps = p.state();
    let meta = &ps.meta;
    let recv = p.tpl_type_name().into_owned();
    match name {
        // ---- the wrappers' own methods ----
        "Weight0" => {
            arity(a, 0, false, name)?;
            Ok(Value::int(p.weight0().unwrap_or_default()))
        }
        "Ordinal" => {
            arity(a, 0, false, name)?;
            Ok(Value::int(p.ordinal().unwrap_or_default()))
        }
        "Unwrapv" => {
            arity(a, 0, false, name)?;
            Ok(unwrapped(p).page_ref().to_value())
        }

        // ---- PageMetaProvider (pageMeta) ----
        "Aliases" => {
            arity(a, 0, false, name)?;
            Ok(front_matter_strings(
                meta.params(),
                "aliases",
                meta.aliases(),
            ))
        }
        "BundleType" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.bundle_type()))
        }
        "Date" => {
            arity(a, 0, false, name)?;
            Ok(Value::Time(meta.date()))
        }
        "Lastmod" => {
            arity(a, 0, false, name)?;
            Ok(Value::Time(meta.lastmod()))
        }
        "PublishDate" => {
            arity(a, 0, false, name)?;
            Ok(Value::Time(meta.publish_date()))
        }
        "ExpiryDate" => {
            arity(a, 0, false, name)?;
            Ok(Value::Time(meta.expiry_date()))
        }
        "Description" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.description()))
        }
        "Draft" => {
            arity(a, 0, false, name)?;
            Ok(Value::Bool(meta.draft()))
        }
        "IsHome" => {
            arity(a, 0, false, name)?;
            Ok(Value::Bool(meta.is_home()))
        }
        "IsNode" => {
            arity(a, 0, false, name)?;
            Ok(Value::Bool(meta.is_node()))
        }
        "IsPage" => {
            arity(a, 0, false, name)?;
            Ok(Value::Bool(meta.is_page()))
        }
        "IsSection" => {
            arity(a, 0, false, name)?;
            Ok(Value::Bool(meta.is_section()))
        }
        "Keywords" => {
            arity(a, 0, false, name)?;
            Ok(front_matter_strings(
                meta.params(),
                "keywords",
                meta.keywords(),
            ))
        }
        "Kind" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.kind()))
        }
        "Layout" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.layout()))
        }
        "LinkTitle" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.link_title()))
        }
        "Param" => {
            arity(a, 1, false, name)?;
            Page::param(p, &a[0]).map_err(Into::into)
        }
        "Params" => {
            arity(a, 0, false, name)?;
            Ok(params_value(p))
        }
        "Path" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.path()))
        }
        "PathInfo" => {
            arity(a, 0, false, name)?;
            Err(unsupported(&recv, name))
        }
        "Slug" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.slug()))
        }
        "Lang" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.lang()))
        }
        "Section" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.section()))
        }
        "Sitemap" => {
            arity(a, 0, false, name)?;
            Ok(Value::object(meta.sitemap().clone()))
        }
        "Type" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.page_type()))
        }
        "Weight" => {
            arity(a, 0, false, name)?;
            Ok(Value::int(meta.weight()))
        }
        "Title" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.title()))
        }
        "Name" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(meta.name()))
        }
        "File" => {
            arity(a, 0, false, name)?;
            Ok(Value::object(FileObject(meta.f.clone())))
        }

        // ---- resource.Resource ----
        "ResourceType" => {
            arity(a, 0, false, name)?;
            Ok(Value::string("page"))
        }
        "MediaType" => {
            arity(a, 0, false, name)?;
            Ok(Resource::media_type(p).to_value())
        }
        "Permalink" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(Resource::permalink(p)))
        }
        "RelPermalink" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(Resource::rel_permalink(p)))
        }
        "Data" => {
            arity(a, 0, false, name)?;
            Ok(crate::page__data::data(&unwrapped(p)))
        }
        "Language" => {
            arity(a, 0, false, name)?;
            Ok(Value::object(LanguageObject(
                p.h.sites[ps.site_idx].language.clone(),
            )))
        }
        "TranslationKey" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(ps.translation_key()))
        }
        "Key" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(ps.key()))
        }
        "IdentifierBase" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(ps.identifier_base()))
        }
        "StaleVersion" => {
            arity(a, 0, false, name)?;
            Ok(Value::Uint(0, go_value::UintKind::Uint32))
        }
        "MarkStale" => bad_results(a, 0, false, false, name, 0),

        // ---- page.go ----
        "AllTranslations" => {
            arity(a, 0, false, name)?;
            Ok(pages_opt_value(crate::page::all_translations(p)))
        }
        "Translations" => {
            arity(a, 0, false, name)?;
            Ok(pages_opt_value(crate::page::translations(p)))
        }
        "IsTranslated" => {
            arity(a, 0, false, name)?;
            Ok(Value::Bool(crate::page::is_translated(p)))
        }
        "AlternativeOutputFormats" => {
            arity(a, 0, false, name)?;
            Ok(match crate::page::alternative_output_formats(p) {
                Some(o) => nh_page::page_outputformat::output_formats_to_value(&o),
                None => nh_page::page_outputformat::output_formats_nil_value(),
            })
        }
        "OutputFormats" => {
            arity(a, 0, false, name)?;
            let ofs = crate::page::output_formats(p);
            Ok(if ofs.is_empty() {
                nh_page::page_outputformat::output_formats_nil_value()
            } else {
                nh_page::page_outputformat::output_formats_to_value(&ofs)
            })
        }
        "Eq" => {
            arity(a, 1, false, name)?;
            Ok(Value::Bool(crate::page::eq(p, &a[0])))
        }
        "GetPage" => {
            arity(a, 1, false, name)?;
            let r = args::string(a, 0)?;
            crate::page::page_site_adapter_get_page(p, &r.to_str_lossy())
                .map(|pr| pr.to_value())
                .map_err(Into::into)
        }
        "GetTerms" => {
            arity(a, 1, false, name)?;
            let t = args::string(a, 0)?;
            Ok(pages_nil_if_empty(crate::page::get_terms(
                p,
                &t.to_str_lossy(),
            )))
        }
        "GitInfo" => {
            arity(a, 0, false, name)?;
            Ok(nil_of("*gitmap.GitInfo"))
        }
        "CodeOwners" => {
            arity(a, 0, false, name)?;
            Ok(nil_of("[]string"))
        }
        "HasShortcode" => {
            arity(a, 1, false, name)?;
            let n = args::string(a, 0)?;
            Ok(Value::Bool(ps.has_shortcode(&n.to_str_lossy())))
        }
        "HeadingsFiltered" => {
            arity(a, 1, true, name)?;
            Ok(match hf {
                Some(h) => headings_value(h),
                None => nil_of(super::objects::HEADINGS_TYPE),
            })
        }
        "ApplyFilterToHeadings" => {
            arity(a, 2, true, name)?;
            Err(args::wrong_type(
                "func(*tableofcontents.Heading) bool",
                &a[1],
            ))
        }
        "RelatedKeywords" => {
            arity(a, 1, false, name)?;
            Err(args::wrong_type("related.IndexConfig", &a[0]))
        }
        "MarshalJSON" => {
            arity(a, 0, false, name)?;
            Err(unsupported(&recv, name))
        }
        "ForEeachIdentity" | "GetDependencyManagerForScope" => {
            arity(a, 1, false, name)?;
            Err(unsupported(&recv, name))
        }
        "GetDependencyManager"
        | "GetDependencyManagerForScopesAll"
        | "GetIdentity"
        | "GetInternalRelatedDocsHandler" => {
            arity(a, 0, false, name)?;
            Err(unsupported(&recv, name))
        }
        "GetInternalTemplateBasePathAndDescriptor" => bad_results(a, 0, false, false, name, 2),
        "Group" => {
            arity(a, 2, false, name)?;
            crate::collections::group(p, &a[0], &a[1]).map_err(Into::into)
        }
        "Slice" => {
            arity(a, 1, false, name)?;
            crate::collections::slice(p, &a[0]).map_err(Into::into)
        }
        "Pages" => {
            arity(a, 0, false, name)?;
            Ok(match crate::page::pages_opt(p) {
                Some(ps) => pages_nil_if_empty(ps),
                None => nil_of("page.Pages"),
            })
        }
        "RegularPages" => {
            arity(a, 0, false, name)?;
            Ok(match crate::page::regular_pages_opt(p) {
                Some(ps) => pages_nil_if_empty(ps),
                None => nil_of("page.Pages"),
            })
        }
        "RegularPagesRecursive" => {
            arity(a, 0, false, name)?;
            Ok(pages_nil_if_empty(crate::page::regular_pages_recursive(p)))
        }
        "PagesRecursive" => {
            arity(a, 0, false, name)?;
            Ok(nil_of("page.Pages"))
        }
        "RawContent" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(ps.raw_content()))
        }
        "Resources" => {
            arity(a, 0, false, name)?;
            let rs = crate::page::resources(p);
            Ok(if rs.is_empty() {
                nil_of(nh_resource::resourcetypes::RESOURCES_TYPE)
            } else {
                nh_resource::resourcetypes::resources_to_value(&rs)
            })
        }
        "Site" => {
            arity(a, 0, false, name)?;
            Ok(page_site(p).to_value())
        }
        "Sites" => {
            arity(a, 0, false, name)?;
            if !ps.common_providers_initialised() {
                // Go: `SitesProvider` is a nil interface before `initCommonProviders`.
                return Err(go_value::Error::new(
                    "runtime error: invalid memory address or nil pointer dereference",
                ));
            }
            Ok(crate::site::sites_to_value(&crate::site::site_sites(&p.h)))
        }
        "String" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(ps.string()))
        }
        "Scratch" | "Store" => {
            arity(a, 0, false, name)?;
            Ok(Value::Object(ps.common.store() as Arc<dyn go_value::Object>))
        }

        // ---- tree navigation (page__tree.go) ----
        "Ancestors" => {
            arity(a, 0, false, name)?;
            Ok(pages_nil_if_empty(crate::page__tree::ancestors(
                &unwrapped(p),
            )))
        }
        "CurrentSection" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(
                crate::page__tree::current_section_id(&p.h, p.id)
                    .map(|id| crate::page::handle_of(&p.h, id).page_ref()),
            ))
        }
        "FirstSection" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::page__tree::first_section(&unwrapped(p))))
        }
        "InSection" => {
            arity(a, 1, false, name)?;
            Ok(Value::Bool(crate::page__tree::in_section(
                &unwrapped(p),
                &a[0],
            )))
        }
        "IsAncestor" => {
            arity(a, 1, false, name)?;
            Ok(Value::Bool(crate::page__tree::is_ancestor(
                &unwrapped(p),
                &a[0],
            )))
        }
        "IsDescendant" => {
            arity(a, 1, false, name)?;
            Ok(Value::Bool(crate::page__tree::is_descendant(
                &unwrapped(p),
                &a[0],
            )))
        }
        "Parent" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::page__tree::parent(&unwrapped(p))))
        }
        "Sections" => {
            arity(a, 0, false, name)?;
            Ok(pages_nil_if_empty(crate::page__tree::sections(&unwrapped(
                p,
            ))))
        }
        "Page" => {
            arity(a, 0, false, name)?;
            Ok(crate::page__tree::page(&unwrapped(p)).to_value())
        }
        "SectionsEntries" => {
            arity(a, 0, false, name)?;
            Ok(strings_opt_value(crate::page__tree::sections_entries(
                &unwrapped(p),
            )))
        }
        "SectionsPath" => {
            arity(a, 0, false, name)?;
            Ok(Value::string(crate::page__tree::sections_path(&unwrapped(
                p,
            ))))
        }

        // ---- positions ----
        "Next" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::page__position::page_next(p)))
        }
        "Prev" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::page__position::page_prev(p)))
        }
        "NextPage" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::page__position::page_next_page(p)))
        }
        "PrevPage" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::page__position::page_prev_page(p)))
        }
        "NextInSection" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::page__position::next_in_section(p)))
        }
        "PrevInSection" => {
            arity(a, 0, false, name)?;
            Ok(page_or_nil(crate::page__position::prev_in_section(p)))
        }

        // ---- menus ----
        "Menus" => {
            arity(a, 0, false, name)?;
            Ok(match crate::page__menus::page_menus(&unwrapped(p)) {
                Some(m) => nh_page::navigation::menu::page_menus_to_value(&m),
                None => nil_of(nh_page::navigation::menu::PAGE_MENUS_TYPE),
            })
        }
        "HasMenuCurrent" => {
            arity(a, 2, false, name)?;
            let menu_id = args::string(a, 0)?;
            let me = menu_entry_arg(a, 1)?;
            Ok(Value::Bool(match me {
                Some(me) => crate::page__menus::has_menu_current(p, &menu_id.to_str_lossy(), &me),
                // Go dereferences the nil entry.
                None => {
                    return Err(go_value::Error::new(
                        "runtime error: invalid memory address or nil pointer dereference",
                    ));
                }
            }))
        }
        "IsMenuCurrent" => {
            arity(a, 2, false, name)?;
            let menu_id = args::string(a, 0)?;
            let me = menu_entry_arg(a, 1)?;
            Ok(Value::Bool(match me {
                Some(me) => crate::page__menus::is_menu_current(p, &menu_id.to_str_lossy(), &me),
                None => {
                    return Err(go_value::Error::new(
                        "runtime error: invalid memory address or nil pointer dereference",
                    ));
                }
            }))
        }

        // ---- refs ----
        "Ref" => {
            arity(a, 1, false, name)?;
            let m = map_arg(a, 0)?;
            if !ps.common_providers_initialised() {
                // Go: `page.NopPage`'s `RefProvider` ("", nil) before the lazy page init.
                return Ok(Value::string(""));
            }
            crate::page__ref::new_page_ref(p)
                .ref_(&m)
                .map(Value::string)
                .map_err(Into::into)
        }
        "RelRef" => {
            arity(a, 1, false, name)?;
            let m = map_arg(a, 0)?;
            if !ps.common_providers_initialised() {
                // Go: `page.NopPage`'s `RefProvider` ("", nil) before the lazy page init.
                return Ok(Value::string(""));
            }
            crate::page__ref::new_page_ref(p)
                .rel_ref(&m)
                .map(Value::string)
                .map_err(Into::into)
        }
        "RefFrom" => {
            arity(a, 2, false, name)?;
            let m = map_arg(a, 0)?;
            if !ps.common_providers_initialised() {
                // Go: `page.NopPage`'s `RefProvider` ("", nil) before the lazy page init.
                return Ok(Value::string(""));
            }
            crate::page__ref::new_page_ref(p)
                .ref_from(&m, &a[1])
                .map(Value::string)
                .map_err(Into::into)
        }
        "RelRefFrom" => {
            arity(a, 2, false, name)?;
            let m = map_arg(a, 0)?;
            if !ps.common_providers_initialised() {
                // Go: `page.NopPage`'s `RefProvider` ("", nil) before the lazy page init.
                return Ok(Value::string(""));
            }
            crate::page__ref::new_page_ref(p)
                .rel_ref_from(&m, &a[1])
                .map(Value::string)
                .map_err(Into::into)
        }

        // ---- pagination ----
        "Paginator" => {
            arity_variadic(a, 1, false, name)?;
            paginator(p, None, a)
        }
        "Paginate" => {
            arity_variadic(a, 2, false, name)?;
            paginator(p, Some(&a[0]), &a[1..])
        }

        // ---- content (ContentProvider, MarkupProvider, TableOfContentsProvider,
        //      PageRenderProvider, RenderShortcodesProvider) ----
        "Content" => {
            arity(a, 1, true, name)?;
            match content_provider(p).filter(|_| !nop_content(p)) {
                Some(cp) => cp.content(ctx, p).map_err(Into::into),
                None => Ok(Value::string("")),
            }
        }
        "ContentWithoutSummary" => {
            arity(a, 1, true, name)?;
            match content_provider(p).filter(|_| !nop_content(p)) {
                Some(cp) => cp.content_without_summary(ctx).map_err(Into::into),
                None => Ok(html("")),
            }
        }
        "Plain" => {
            arity(a, 1, true, name)?;
            match content_provider(p).filter(|_| !nop_content(p)) {
                Some(cp) => cp.plain(ctx, p).map_err(Into::into),
                None => Ok(Value::string("")),
            }
        }
        "PlainWords" => {
            arity(a, 1, true, name)?;
            match content_provider(p).filter(|_| !nop_content(p)) {
                Some(cp) => {
                    let w = cp.plain_words(ctx);
                    Ok(if w.is_empty() {
                        nil_of("[]string")
                    } else {
                        Value::list(
                            SliceType::String,
                            w.into_iter().map(Value::String).collect(),
                        )
                    })
                }
                None => Ok(nil_of("[]string")),
            }
        }
        "Summary" => {
            arity(a, 1, true, name)?;
            match content_provider(p).filter(|_| !nop_content(p)) {
                Some(cp) => Ok(cp.summary(ctx)),
                None => Ok(html("")),
            }
        }
        "Truncated" => {
            arity(a, 1, true, name)?;
            match content_provider(p).filter(|_| !nop_content(p)) {
                Some(cp) => Ok(Value::Bool(cp.truncated(ctx))),
                None => Ok(Value::Bool(false)),
            }
        }
        "FuzzyWordCount" | "WordCount" | "ReadingTime" | "Len" => {
            arity(a, 1, true, name)?;
            match content_provider(p).filter(|_| !nop_content(p)) {
                Some(cp) => Ok(Value::int(match name {
                    "FuzzyWordCount" => cp.fuzzy_word_count(ctx),
                    "WordCount" => cp.word_count(ctx),
                    "ReadingTime" => cp.reading_time(ctx),
                    _ => cp.len(ctx),
                })),
                None => Ok(Value::int(0)),
            }
        }
        "Markup" => {
            if nop_content(p) || content_provider(p).is_none() {
                return Ok(Value::object(nh_page::page_nop::NopMarkup));
            }
            Err(unsupported(&recv, name))
        }
        "TableOfContents" => {
            arity(a, 1, true, name)?;
            if p.wrapper == PageWrapper::ForShortcode {
                // We need to replace it after we have rendered it, so provide a
                // temporary placeholder.
                return Ok(html(crate::shortcode_page::toc_shortcode_placeholder()));
            }
            match content_provider(p) {
                Some(cp) => Ok(cp.table_of_contents(ctx)),
                None => Ok(html("")),
            }
        }
        "Fragments" => {
            arity(a, 1, true, name)?;
            match content_provider(p) {
                Some(cp) => Ok(fragments_value(cp.fragments(ctx))),
                None => Ok(nil_of("*tableofcontents.Fragments")),
            }
        }
        "Render" => {
            arity_variadic(a, 2, true, name)?;
            let layouts: Vec<String> = args::strings(a, 0)?
                .into_iter()
                .map(|s| s.to_str_lossy().into_owned())
                .collect();
            match content_provider(p) {
                Some(cp) => cp.render(ctx, &layouts).map_err(Into::into),
                None => Ok(html("")),
            }
        }
        "RenderString" => {
            arity_variadic(a, 2, true, name)?;
            match content_provider(p) {
                Some(cp) => cp.render_string(ctx, p, a).map_err(Into::into),
                None => Ok(html("")),
            }
        }
        "RenderShortcodes" => {
            arity(a, 1, true, name)?;
            // Go's `RenderShortcodesProvider` is the last `pco` set (nil before any).
            match ps.current_output_opt().and_then(|po| po.pco()) {
                Some(cp) => cp.render_shortcodes(ctx).map_err(Into::into),
                None => Err(go_value::Error::new(
                    "runtime error: invalid memory address or nil pointer dereference",
                )),
            }
        }

        _ => Err(unsupported(&recv, name)),
    }
}

/// A `[]string` page config field set from front matter with `cast.ToStringSlice` (`Aliases`,
/// `Keywords`): nil unless the key was set (Go stores the same slice in the params; a set but
/// empty list is a non-nil empty slice).
fn front_matter_strings(params: Option<&go_value::Map>, key: &str, v: &[String]) -> Value {
    let set = params
        .and_then(|m| m.get(key.as_bytes()))
        .is_some_and(|x| matches!(x, Value::List(_)));
    if v.is_empty() && !set {
        nil_of("[]string")
    } else {
        strings_opt_value(Some(v.to_vec()))
    }
}

/// `.Paginator`/`.Paginate`: the page output's paginator, or Go's
/// `PaginatorNotSupportedFunc` error.
fn paginator(p: &PageHandle, seq: Option<&Value>, options: &[Value]) -> GoResult<Value> {
    let ps = p.state();
    // Go: `nopPageOutput` (before the first shift): `page.NopPage`'s paginator, (nil, nil).
    let Some(po) = ps.current_output_opt() else {
        return Ok(nil_of("*page.Pager"));
    };
    let Some(pag) = &po.paginator else {
        return Err(go_value::Error::new(format!(
            "pagination not supported for this page: {}",
            ps.get_page_info_for_error()
        )));
    };
    let src = unwrapped(p);
    let r = match seq {
        Some(seq) => pag.paginate(&src, seq, options),
        None => pag.paginator(&src, options),
    };
    match r {
        Ok(Some(pager)) => Ok(Value::object(nh_page::pagination::PagerRef(pager))),
        Ok(None) => Ok(nil_of("*page.Pager")),
        Err(e) => Err(e.into()),
    }
}

/// The page's params (`maps.Params`), the same map value on every call.
fn params_value(p: &PageHandle) -> Value {
    let ps = p.state();
    match ps.meta.params() {
        None => nil_of("maps.Params"),
        Some(_) => Value::Map(page_params_arc(p)),
    }
}

/// The page's params as one `Arc` (created once per page).
fn page_params_arc(p: &PageHandle) -> Arc<Map> {
    let ps = p.state();
    ps.common
        .params
        .get_or_init(|| {
            let mut m = ps
                .meta
                .params()
                .cloned()
                .unwrap_or_else(|| Map::new(MapType::Params));
            m.ty = MapType::Params;
            Arc::new(m)
        })
        .clone()
}

/// The page's `page.Site` (Go `p.sWrapped`: one wrapper per page).
fn page_site(p: &PageHandle) -> SiteRef {
    p.state()
        .common
        .site_wrapped
        .get_or_init(|| crate::page::site(p))
        .clone()
}

impl Resource for PageHandle {
    // Go: hugolib/page.go:ResourceType (pageTypesProvider)
    fn resource_type(&self) -> String {
        "page".to_string()
    }
    // Go: hugolib/page.go:MediaType (pageTypesProvider: application/octet-stream)
    fn media_type(&self) -> MediaType {
        nh_media::media::builtin::builtin().octet_type.clone()
    }
    // Go: hugolib/page__per_output.go:Permalink (the current output's target paths)
    fn permalink(&self) -> String {
        // Go: `nopPageOutput`'s `page.NopPage` ("") before the first shift.
        self.state()
            .current_output_opt()
            .map(|po| po.target_paths.output_format.permalink().to_string())
            .unwrap_or_default()
    }
    // Go: hugolib/page__per_output.go:RelPermalink
    fn rel_permalink(&self) -> String {
        self.state()
            .current_output_opt()
            .map(|po| po.target_paths.output_format.rel_permalink().to_string())
            .unwrap_or_default()
    }
    // Go: hugolib/page__data.go:Data
    fn data(&self) -> Value {
        crate::page__data::data(&unwrapped(self))
    }
    // Go: hugolib/page__meta.go:Name
    fn name(&self) -> String {
        // (T21: `resource.NameNormalizedOrName` in getOrCreateResourcesForPage.)
        self.state().meta.name()
    }
    // Go: hugolib/page__meta.go:Title
    fn title(&self) -> String {
        self.state().meta.title().to_string()
    }
    // Go: hugolib/page__meta.go:Params
    fn params(&self) -> Arc<Map> {
        page_params_arc(self)
    }
    // Go: hugolib/page.go:Key
    fn key(&self) -> String {
        self.state().key()
    }
    // Go: hugolib/page__per_output.go:Content
    fn content(&self, ctx: HostCtx<'_>) -> Option<Result<Value>> {
        Some(match call_page_method(self, ctx, "Content", &[], None) {
            Ok(v) => Ok(v),
            Err(e) => Err(Error::from(e)),
        })
    }
    // Go: hugolib/page.go:Language (the site's language)
    fn language(&self) -> Option<Arc<Language>> {
        Some(self.h.sites[self.state().site_idx].language.clone())
    }
    // Go: hugolib/page.go:TranslationKey
    fn translation_key(&self) -> Option<String> {
        Some(self.state().translation_key())
    }
    fn stale_version(&self) -> Option<u32> {
        Some(0)
    }

    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(match self.wrapper {
            PageWrapper::None => "*hugolib.pageState",
            PageWrapper::Weight0(_) => "hugolib.pageWithWeight0",
            PageWrapper::Ordinal(_) => "*hugolib.pageWithOrdinal",
            PageWrapper::ForShortcode => "*hugolib.pageForShortcode",
            PageWrapper::ForRenderHooks => "*hugolib.pageForRenderHooks",
        })
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        page_has_method(self.wrapper, name)
    }
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        if !page_has_method(self.wrapper, name) {
            return None;
        }
        Some(call_page_method(self, ctx, name, args, None))
    }
    fn tpl_go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.state().string()))
    }
    fn tpl_is_zero(&self) -> Option<bool> {
        None
    }

    fn to_value(self: Arc<Self>) -> Value {
        PageRef(self).to_value()
    }
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Page for PageHandle {
    fn page_id(&self) -> u64 {
        self.state().pid
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        Arc::new(PageHandle {
            h: self.h.clone(),
            id: self.id,
            wrapper: PageWrapper::None,
        })
    }
    fn weight0(&self) -> Option<i64> {
        match self.wrapper {
            PageWrapper::Weight0(w) => Some(w),
            _ => None,
        }
    }
    fn ordinal(&self) -> Option<i64> {
        match self.wrapper {
            PageWrapper::Ordinal(o) => Some(o),
            _ => None,
        }
    }
    fn kind(&self) -> String {
        self.state().meta.kind().to_string()
    }
    fn title(&self) -> String {
        self.state().meta.title().to_string()
    }
    fn link_title(&self) -> String {
        self.state().meta.link_title().to_string()
    }
    fn description(&self) -> String {
        self.state().meta.description().to_string()
    }
    fn weight(&self) -> i64 {
        // (T21: needed by the default page sort of the page collections.)
        self.state().meta.weight()
    }
    fn date(&self) -> Time {
        self.state().meta.date()
    }
    fn lastmod(&self) -> Time {
        self.state().meta.lastmod()
    }
    fn publish_date(&self) -> Time {
        self.state().meta.publish_date()
    }
    fn expiry_date(&self) -> Time {
        self.state().meta.expiry_date()
    }
    fn is_home(&self) -> bool {
        self.state().meta.is_home()
    }
    fn is_node(&self) -> bool {
        self.state().meta.is_node()
    }
    fn is_page(&self) -> bool {
        self.state().meta.is_page()
    }
    fn is_section(&self) -> bool {
        self.state().meta.is_section()
    }
    fn section(&self) -> String {
        self.state().meta.section().to_string()
    }
    fn page_type(&self) -> String {
        self.state().meta.page_type()
    }
    fn layout(&self) -> String {
        self.state().meta.layout().to_string()
    }
    fn lang(&self) -> String {
        self.state().meta.lang().to_string()
    }
    fn path(&self) -> String {
        self.state().meta.path()
    }
    fn path_info(&self) -> Arc<Path> {
        self.state().meta.path_info.clone()
    }
    fn slug(&self) -> String {
        self.state().meta.slug().to_string()
    }
    fn draft(&self) -> bool {
        self.state().meta.draft()
    }
    fn aliases(&self) -> Vec<String> {
        self.state().meta.aliases().to_vec()
    }
    fn keywords(&self) -> Vec<String> {
        self.state().meta.keywords().to_vec()
    }
    fn bundle_type(&self) -> String {
        self.state().meta.bundle_type().to_string()
    }
    fn sitemap(&self) -> SitemapConfig {
        self.state().meta.sitemap().clone()
    }
    // Go: hugolib/page__meta.go:Param (page params, then the site's)
    fn param(&self, key: &Value) -> Result<Value> {
        let site_params = self.h.sites[self.state().site_idx].params();
        nh_resource::params::param(&page_params_arc(self), Some(&site_params), key)
    }
    fn page_params(&self) -> Arc<Map> {
        page_params_arc(self)
    }
    fn site(&self) -> SiteRef {
        // (T21: needed by the default page sort, `Site().Current().Language()`.)
        page_site(self)
    }
    fn file(&self) -> Option<Arc<File>> {
        self.state().meta.f.clone()
    }
    fn parent(&self) -> Option<PageRef> {
        crate::page__tree::parent(self)
    }
    fn pages(&self) -> Pages {
        crate::page::pages(self)
    }
    fn regular_pages(&self) -> Pages {
        crate::page::regular_pages(self)
    }
    fn resources(&self) -> Resources {
        crate::page::resources(self)
    }
    fn output_formats(&self) -> OutputFormats {
        crate::page::output_formats(self)
    }
    fn all_translations(&self) -> Pages {
        crate::page::all_translations(self).unwrap_or_default()
    }
    fn translations(&self) -> Pages {
        crate::page::translations(self).unwrap_or_default()
    }
    fn current_section(&self) -> Option<PageRef> {
        crate::page__tree::current_section_id(&self.h, self.id)
            .map(|id| crate::page::handle_of(&self.h, id).page_ref())
    }
    fn sections_entries(&self) -> Vec<String> {
        crate::page__tree::sections_entries(self).unwrap_or_default()
    }
    fn sections_path(&self) -> String {
        crate::page__tree::sections_path(self)
    }
    fn plain(&self, ctx: HostCtx<'_>) -> Result<GoString> {
        match call_page_method(self, ctx, "Plain", &[], None) {
            Ok(Value::String(s)) => Ok(s),
            Ok(_) => Ok(GoString::empty()),
            Err(e) => Err(e.into()),
        }
    }
    fn content_len(&self, ctx: HostCtx<'_>) -> Result<i64> {
        match call_page_method(self, ctx, "Len", &[], None) {
            Ok(Value::Int(n, _)) => Ok(n),
            Ok(_) => Ok(0),
            Err(e) => Err(e.into()),
        }
    }
    fn render_string(&self, ctx: HostCtx<'_>, args: &[Value]) -> Result<Value> {
        call_page_method(self, ctx, "RenderString", args, None).map_err(Into::into)
    }
    // Go: hugolib/page.go:RelatedKeywords
    fn related_keywords(&self, cfg: &IndexConfig) -> Result<Vec<Keyword>> {
        crate::page::related_keywords(self, cfg)
    }
    // Go: hugolib/page__ref.go:Ref
    fn ref_(&self, args: &Map) -> Result<String> {
        crate::page__ref::new_page_ref(self).ref_(args)
    }
    // Go: hugolib/page__ref.go:RelRef
    fn rel_ref(&self, args: &Map) -> Result<String> {
        crate::page__ref::new_page_ref(self).rel_ref(args)
    }
    // Go: hugolib/page.go:String
    fn page_string(&self) -> String {
        self.state().string()
    }
    // Go: hugolib/site.go:GetInternalRelatedDocsHandler (the page's RelatedDocsHandlerProvider)
    fn related_docs_handler(&self) -> Option<Arc<nh_page::pages_related::RelatedDocsHandler>> {
        if nop_content(self) {
            // `page.PageWithoutContent` has no `RelatedDocsHandlerProvider`.
            return None;
        }
        Some(self.h.sites[self.state().site_idx].get_internal_related_docs_handler())
    }
    // Go: related.FragmentProvider (`Fragments(ctx).Identifiers`)
    fn fragments_identifiers(&self, ctx: HostCtx<'_>) -> Option<Vec<String>> {
        let cp = content_provider(self)?;
        let f = cp.fragments(ctx)?;
        Some(
            f.identifiers
                .iter()
                .map(|s| s.to_str_lossy().into_owned())
                .collect(),
        )
    }
    // Go: hugolib/page.go:ApplyFilterToHeadings
    fn apply_filter_to_headings(
        &self,
        ctx: HostCtx<'_>,
        filter: &dyn Fn(&str) -> bool,
    ) -> Option<PageRef> {
        if nop_content(self) {
            return None;
        }
        // Go: `p.pageOutput.pco.c().Fragments(ctx)` (the pco of the current output).
        let pco = self.state().current_output_opt()?.pco()?;
        let fragments = pco
            .fragments(ctx)
            .unwrap_or_else(nh_markup::tableofcontents::empty);
        let headings = nh_markup::tableofcontents::filter_by(&fragments.headings, &|h| {
            filter(&h.id.to_str_lossy())
        });
        Some(PageRef(Arc::new(PageHeadingsFiltered {
            p: unwrapped(self),
            headings,
        })))
    }
    fn is_ancestor(&self, other: &Value) -> bool {
        crate::page__tree::is_ancestor(&unwrapped(self), other)
    }
}

/// Go: `*hugolib.pageHeadingsFiltered` — the page (the `*pageState` method set) whose
/// `HeadingsFiltered` returns the headings the related search kept.
#[derive(Clone)]
pub struct PageHeadingsFiltered {
    pub p: PageHandle,
    pub headings: Headings,
}

impl PageHeadingsFiltered {
    /// Go: `(p *pageHeadingsFiltered) HeadingsFiltered(ctx)`.
    // Go: hugolib/page.go:HeadingsFiltered
    pub fn headings_filtered(&self) -> &Headings {
        &self.headings
    }

    /// Go: `(p *pageHeadingsFiltered) page()`.
    // Go: hugolib/page.go:page
    pub fn page(&self) -> &PageHandle {
        &self.p
    }
}

impl Resource for PageHeadingsFiltered {
    fn resource_type(&self) -> String {
        Resource::resource_type(&self.p)
    }
    fn media_type(&self) -> MediaType {
        Resource::media_type(&self.p)
    }
    fn permalink(&self) -> String {
        Resource::permalink(&self.p)
    }
    fn rel_permalink(&self) -> String {
        Resource::rel_permalink(&self.p)
    }
    fn data(&self) -> Value {
        Resource::data(&self.p)
    }
    fn name(&self) -> String {
        Resource::name(&self.p)
    }
    fn title(&self) -> String {
        Resource::title(&self.p)
    }
    fn params(&self) -> Arc<Map> {
        Resource::params(&self.p)
    }
    fn key(&self) -> String {
        Resource::key(&self.p)
    }
    fn content(&self, ctx: HostCtx<'_>) -> Option<Result<Value>> {
        Resource::content(&self.p, ctx)
    }
    fn language(&self) -> Option<Arc<Language>> {
        Resource::language(&self.p)
    }
    fn translation_key(&self) -> Option<String> {
        Resource::translation_key(&self.p)
    }
    fn stale_version(&self) -> Option<u32> {
        Some(0)
    }
    fn tpl_type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("*hugolib.pageHeadingsFiltered")
    }
    fn tpl_has_method(&self, name: &str) -> bool {
        PAGE_STATE_METHODS.binary_search(&name).is_ok()
    }
    fn tpl_call_method(
        &self,
        ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<GoResult<Value>> {
        if !self.tpl_has_method(name) {
            return None;
        }
        Some(call_page_method(
            &self.p,
            ctx,
            name,
            args,
            Some(&self.headings),
        ))
    }
    fn tpl_go_string(&self) -> Option<GoString> {
        self.p.tpl_go_string()
    }
    fn to_value(self: Arc<Self>) -> Value {
        PageRef(self).to_value()
    }
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Delegates `Page` methods to the embedded `*pageState`.
macro_rules! delegate_page {
    ($($name:ident($($arg:ident: $ty:ty),*) -> $ret:ty;)*) => {
        $(fn $name(&self, $($arg: $ty),*) -> $ret { Page::$name(&self.p, $($arg),*) })*
    };
}

impl Page for PageHeadingsFiltered {
    fn page_id(&self) -> u64 {
        self.p.page_id()
    }
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page> {
        // Go's `unwrapPage` has no case for it: it is a `page.Page`, returned as is.
        self
    }
    delegate_page! {
        kind() -> String;
        title() -> String;
        link_title() -> String;
        description() -> String;
        weight() -> i64;
        date() -> Time;
        lastmod() -> Time;
        publish_date() -> Time;
        expiry_date() -> Time;
        is_home() -> bool;
        is_node() -> bool;
        is_page() -> bool;
        is_section() -> bool;
        section() -> String;
        page_type() -> String;
        layout() -> String;
        lang() -> String;
        path() -> String;
        path_info() -> Arc<Path>;
        slug() -> String;
        draft() -> bool;
        aliases() -> Vec<String>;
        keywords() -> Vec<String>;
        bundle_type() -> String;
        sitemap() -> SitemapConfig;
        param(key: &Value) -> Result<Value>;
        page_params() -> Arc<Map>;
        site() -> SiteRef;
        file() -> Option<Arc<File>>;
        parent() -> Option<PageRef>;
        pages() -> Pages;
        regular_pages() -> Pages;
        resources() -> Resources;
        output_formats() -> OutputFormats;
        all_translations() -> Pages;
        translations() -> Pages;
        current_section() -> Option<PageRef>;
        sections_entries() -> Vec<String>;
        sections_path() -> String;
        related_keywords(cfg: &IndexConfig) -> Result<Vec<Keyword>>;
        ref_(args: &Map) -> Result<String>;
        rel_ref(args: &Map) -> Result<String>;
        page_string() -> String;
        related_docs_handler() -> Option<Arc<nh_page::pages_related::RelatedDocsHandler>>;
        is_ancestor(other: &Value) -> bool;
    }
    fn plain(&self, ctx: HostCtx<'_>) -> Result<GoString> {
        Page::plain(&self.p, ctx)
    }
    fn content_len(&self, ctx: HostCtx<'_>) -> Result<i64> {
        Page::content_len(&self.p, ctx)
    }
    fn render_string(&self, ctx: HostCtx<'_>, args: &[Value]) -> Result<Value> {
        Page::render_string(&self.p, ctx, args)
    }
    fn fragments_identifiers(&self, ctx: HostCtx<'_>) -> Option<Vec<String>> {
        Page::fragments_identifiers(&self.p, ctx)
    }
    fn apply_filter_to_headings(
        &self,
        ctx: HostCtx<'_>,
        filter: &dyn Fn(&str) -> bool,
    ) -> Option<PageRef> {
        Page::apply_filter_to_headings(&self.p, ctx, filter)
    }
}

/// All template-visible page methods of a Go page type, for the coverage test.
pub fn page_type_methods(type_name: &str) -> Option<Vec<&'static str>> {
    let mut v: Vec<&'static str> = match type_name {
        "*hugolib.pageState" | "*hugolib.pageHeadingsFiltered" => PAGE_STATE_METHODS.to_vec(),
        "hugolib.pageWithWeight0" => {
            let mut v = PAGE_STATE_METHODS.to_vec();
            v.extend(["Unwrapv", "Weight0"]);
            v
        }
        "*hugolib.pageWithOrdinal" => {
            let mut v = PAGE_STATE_METHODS.to_vec();
            v.push("Ordinal");
            v
        }
        "*hugolib.pageForShortcode" | "*hugolib.pageForRenderHooks" => {
            PAGE_FOR_SHORTCODE_METHODS.to_vec()
        }
        _ => return None,
    };
    v.sort_unstable();
    Some(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_lists_are_sorted() {
        assert!(PAGE_STATE_METHODS.windows(2).all(|w| w[0] < w[1]));
        assert!(PAGE_FOR_SHORTCODE_METHODS.windows(2).all(|w| w[0] < w[1]));
    }
}
