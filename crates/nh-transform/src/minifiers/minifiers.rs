//! Port of `minifiers/minifiers.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).


//! Go `minifiers.Client`: tdewolff `minify.M` with minifiers registered per media type
//! (css; js + regexp; json + regexp; svg; xml by suffix; html + every IsHTML output format).
//! The SVG minifier MUST be registered even for CSS-only use (DataURI re-minification, see
//! specs/resources-pipeline.md §4.6).

use std::sync::Arc;

use nh_common::Result;
use nh_config::config_provider::AllProvider;
use nh_media::media::media_type::{MediaType, Types};
use nh_media::output::output_format::Formats;

use super::config::MinifyConfig;
use crate::chain::Transformer;

/// Go: `minifiers.Client`.
#[derive(Clone)]
pub struct Client {
    /// Wave B: `Arc<tdewolff_minify::M>`.
    pub m: Arc<()>,
    pub minify_output: bool,
}

impl Client {
    /// Go: `minifiers.New(mediaTypes, outputFormats, cfg)`.
    // Go: minifiers/minifiers.go:New
    pub fn new(media_types: &Types, output_formats: &Formats, cfg: &dyn AllProvider) -> Result<Client> {
        todo!()
    }

    /// Go: `Client.Transformer(mediatype)` — `None` when no minifier matches (e.g. text/plain).
    // Go: minifiers/minifiers.go:Transformer
    pub fn transformer(&self, media_type: &MediaType) -> Option<Transformer> {
        todo!()
    }

    /// Go: `Client.Minify(mediatype, dst, src)`.
    // Go: minifiers/minifiers.go:Minify
    pub fn minify(&self, media_type: &MediaType, src: &[u8]) -> Result<Vec<u8>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: minifiers/minifiers.go (131 lines; 5/6 funcs executed)
//   types: Client, noopMinifier
// EX L41-53: (m Client) Transformer(mediatype media.Type) transform.Transformer
// EX L56-58: (m Client) Minify(mediatype media.Type, dst io.Writer, src io.Reader) error
//    L69-72: (m noopMinifier) Minify(_ *minify.M, w io.Writer, r io.Reader, _ map[string]string) error
// EX L77-103: New(mediaTypes media.Types, outputFormats output.Formats, cfg config.AllProvider) (Client, error)
// EX L107-124: getMinifier(c MinifyConfig, s string) minify.Minifier
// EX L126-131: addMinifier(m *minify.M, mt media.Types, suffix string, min minify.Minifier)
// ---------------------------------------------------------------------------
