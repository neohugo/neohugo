//! Module `embedded`.
//!
//! NEW: include_bytes! of tpl/tplimpl/embedded/templates/** (read from the Go tree)
//!
//! Owner: Wave B task T13 (tplimpl).


//! The embedded templates, read from the Go tree at build time (single source of truth until parity).

/// (path relative to `tpl/tplimpl/embedded/templates`, content). CRLF must be normalised to LF
/// by `insertEmbedded`.
pub static EMBEDDED_TEMPLATES: &[(&str, &[u8])] = &[
    ("_hugo/build/js/batch-esm-runner.gotmpl", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_hugo/build/js/batch-esm-runner.gotmpl"))),
    ("_markup/render-codeblock-goat.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_markup/render-codeblock-goat.html"))),
    ("_markup/render-image.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_markup/render-image.html"))),
    ("_markup/render-link.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_markup/render-link.html"))),
    ("_markup/render-table.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_markup/render-table.html"))),
    ("_partials/_funcs/get-page-images.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_partials/_funcs/get-page-images.html"))),
    ("_partials/disqus.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_partials/disqus.html"))),
    ("_partials/google_analytics.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_partials/google_analytics.html"))),
    ("_partials/opengraph.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_partials/opengraph.html"))),
    ("_partials/pagination.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_partials/pagination.html"))),
    ("_partials/schema.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_partials/schema.html"))),
    ("_partials/twitter_cards.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_partials/twitter_cards.html"))),
    ("_server/error.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_server/error.html"))),
    ("_shortcodes/1__h_simple_assets.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/1__h_simple_assets.html"))),
    ("_shortcodes/comment.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/comment.html"))),
    ("_shortcodes/details.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/details.html"))),
    ("_shortcodes/figure.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/figure.html"))),
    ("_shortcodes/gist.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/gist.html"))),
    ("_shortcodes/highlight.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/highlight.html"))),
    ("_shortcodes/instagram.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/instagram.html"))),
    ("_shortcodes/instagram_simple.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/instagram_simple.html"))),
    ("_shortcodes/param.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/param.html"))),
    ("_shortcodes/qr.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/qr.html"))),
    ("_shortcodes/ref.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/ref.html"))),
    ("_shortcodes/relref.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/relref.html"))),
    ("_shortcodes/twitter.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/twitter.html"))),
    ("_shortcodes/twitter_simple.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/twitter_simple.html"))),
    ("_shortcodes/vimeo.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/vimeo.html"))),
    ("_shortcodes/vimeo_simple.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/vimeo_simple.html"))),
    ("_shortcodes/x.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/x.html"))),
    ("_shortcodes/x_simple.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/x_simple.html"))),
    ("_shortcodes/youtube.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/_shortcodes/youtube.html"))),
    ("alias.html", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/alias.html"))),
    ("robots.txt", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/robots.txt"))),
    ("rss.xml", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/rss.xml"))),
    ("sitemap.xml", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/sitemap.xml"))),
    ("sitemapindex.xml", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../tpl/tplimpl/embedded/templates/sitemapindex.xml"))),
];
