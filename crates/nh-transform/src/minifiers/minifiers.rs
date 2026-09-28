//! Port of `minifiers/minifiers.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).

//! Go `minifiers.Client`: tdewolff `minify.M` with minifiers registered per media type
//! (css; js + regexp; json + regexp; svg; xml by suffix; html + every IsHTML output format).
//! The SVG minifier MUST be registered even for CSS-only use (DataURI re-minification, see
//! specs/resources-pipeline.md §4.6).

use std::sync::Arc;

use nh_common::{Error, Result};
use nh_config::config_provider::{AllProvider, config_section};
use nh_media::media::media_type::{MediaType, Types};
use nh_media::output::output_format::Formats;
use tdewolff_minify::{GoBytes, GoError, GoReader, M, Minifier, Params, Regexp, Writer};

use super::config::MinifyConfig;
use crate::chain::Transformer;

/// Go: `minifiers.Client`.
#[derive(Clone)]
pub struct Client {
    /// Go: `m *minify.M`.
    pub m: Arc<M>,
    /// Whether output minification is enabled (HTML in /public).
    pub minify_output: bool,
}

/// Go's `err.Error()` of a minifier error, as an [`Error`].
fn minify_error(err: GoError) -> Error {
    Error::new(String::from_utf8_lossy(&err.error_bytes()).into_owned())
}

impl Client {
    /// Go: `minifiers.New(mediaTypes, outputFormats, cfg)`.
    // Go: minifiers/minifiers.go:New
    pub fn new(
        media_types: &Types,
        output_formats: &Formats,
        cfg: &dyn AllProvider,
    ) -> Result<Client> {
        let conf = config_section::<MinifyConfig>(cfg, "minify");
        Client::from_config(media_types, output_formats, &conf)
    }

    /// Go: `minifiers.New` with the decoded `minify` config section given directly.
    pub fn from_config(
        media_types: &Types,
        output_formats: &Formats,
        conf: &MinifyConfig,
    ) -> Result<Client> {
        let mut m = M::new();

        // We use the Type definition of the media types defined in the site if found.
        add_minifier(&mut m, media_types, "css", get_minifier(conf, "css"));

        add_minifier(&mut m, media_types, "js", get_minifier(conf, "js"));
        m.add_regexp(
            Regexp::must_compile("^(application|text)/(x-)?(java|ecma)script$"),
            get_minifier(conf, "js"),
        );

        add_minifier(&mut m, media_types, "json", get_minifier(conf, "json"));
        m.add_regexp(
            Regexp::must_compile(r"^(application|text)/(x-|(ld|manifest)\+)?json$"),
            get_minifier(conf, "json"),
        );

        add_minifier(&mut m, media_types, "svg", get_minifier(conf, "svg"));

        add_minifier(&mut m, media_types, "xml", get_minifier(conf, "xml"));

        // HTML
        add_minifier(&mut m, media_types, "html", get_minifier(conf, "html"));
        for of in &output_formats.0 {
            if of.is_html {
                m.add(of.media_type.typ.as_bytes(), get_minifier(conf, "html"));
            }
        }

        Ok(Client {
            m: Arc::new(m),
            minify_output: conf.minify_output,
        })
    }

    /// Go: `Client.Transformer(mediatype)` — `None` when no minifier matches (e.g. text/plain).
    // Go: minifiers/minifiers.go:Transformer
    pub fn transformer(&self, media_type: &MediaType) -> Option<Transformer> {
        let (_, params, min) = self.m.match_(media_type.typ.as_bytes());
        // No minifier for this MIME type
        let min = min?;

        let m = self.m.clone();
        Some(Box::new(move |ft| {
            // Note that the source io.Reader will already be buffered, but it implements
            // the Bytes() method, which is recognized by the Minify library.
            let mut r = tdewolff_parse::buffer::Reader::new(GoBytes::from_slice(ft.from));
            min.minify(&m, ft.to, &mut r, params.as_ref())
                .map_err(minify_error)
        }))
    }

    /// Go: `Client.Minify(mediatype, dst, src)`.
    // Go: minifiers/minifiers.go:Minify
    pub fn minify(&self, media_type: &MediaType, src: &[u8]) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(src.len());
        let mut r = tdewolff_parse::buffer::Reader::new(GoBytes::from_slice(src));
        self.m
            .minify(media_type.typ.as_bytes(), &mut out, &mut r)
            .map_err(minify_error)?;
        Ok(out)
    }
}

/// Go: `noopMinifier` implements minify.Minifier, but doesn't minify content. This means that we
/// can avoid missing minifiers for any MIME types in our minify.M, which causes minify to return
/// errors, while still allowing minification to be disabled for specific types.
struct NoopMinifier;

impl Minifier for NoopMinifier {
    /// Go: `noopMinifier.Minify` — copies r into w without transformation (`io.Copy`).
    // Go: minifiers/minifiers.go:Minify
    fn minify(
        &self,
        _m: &M,
        w: &mut dyn Writer,
        r: &mut dyn GoReader,
        _params: Option<&Params>,
    ) -> std::result::Result<(), GoError> {
        // io.Copy uses the reader's WriteTo when it has one (`*bytes.Buffer`,
        // `*buffer.Reader`): one Write of the remaining bytes.
        if let Some(b) = r.bytes() {
            if !b.is_empty() {
                w.write_go(&b)?;
            }
            return Ok(());
        }
        let mut buf = vec![0u8; 32 * 1024];
        loop {
            let (n, err) = r.read(&mut buf);
            if n > 0 {
                w.write(&buf[..n])?;
            }
            match err {
                None => {}
                Some(GoError::Eof) => return Ok(()),
                Some(e) => return Err(e),
            }
        }
    }
}

/// Go: `getMinifier(c, s)` — the appropriate minifier for the MIME type suffix s, given the
/// config c (a no-op minifier for disabled types).
// Go: minifiers/minifiers.go:getMinifier
fn get_minifier(c: &MinifyConfig, s: &str) -> Arc<dyn Minifier> {
    match s {
        "css" if !c.disable_css => Arc::new(c.tdewolff.css.minifier()),
        "js" if !c.disable_js => Arc::new(c.tdewolff.js.minifier()),
        "json" if !c.disable_json => Arc::new(c.tdewolff.json.minifier()),
        "svg" if !c.disable_svg => Arc::new(c.tdewolff.svg.minifier()),
        "xml" if !c.disable_xml => Arc::new(c.tdewolff.xml.minifier()),
        "html" if !c.disable_html => Arc::new(c.tdewolff.html.minifier()),
        _ => Arc::new(NoopMinifier),
    }
}

// Go: minifiers/minifiers.go:addMinifier
fn add_minifier(m: &mut M, mt: &Types, suffix: &str, min: Arc<dyn Minifier>) {
    let types = mt.by_suffix(suffix);
    for t in types {
        m.add(t.typ.as_bytes(), min.clone());
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: minifiers/minifiers.go (131 lines; 5/6 funcs executed)
//   types: Client, noopMinifier
// OK L41-53: (m Client) Transformer(mediatype media.Type) transform.Transformer
// OK L56-58: (m Client) Minify(mediatype media.Type, dst io.Writer, src io.Reader) error
// OK L69-72: (m noopMinifier) Minify(_ *minify.M, w io.Writer, r io.Reader, _ map[string]string) error
// OK L77-103: New(mediaTypes media.Types, outputFormats output.Formats, cfg config.AllProvider) (Client, error)
// OK L107-124: getMinifier(c MinifyConfig, s string) minify.Minifier
// OK L126-131: addMinifier(m *minify.M, mt media.Types, suffix string, min minify.Minifier)
// ---------------------------------------------------------------------------
