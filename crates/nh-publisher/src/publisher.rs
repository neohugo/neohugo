//! Port of `publisher/publisher.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).


//! Go `publisher/publisher.go`: transformer chain (absURL HTML/XML -> [livereload] -> [generator]
//! -> minify), write to the publish fs (Create/truncate, MkdirAll parents), and for HTML formats
//! tee the FINAL bytes into the HTML elements collector (hugo_stats.json).

use std::sync::{Arc, Mutex};

use nh_common::Result;
use nh_hugofs::afero::Fs;
use nh_media::output::output_format::OutputFormat;
use nh_transform::minifiers::minifiers::Client as MinifyClient;

use crate::html_elements_collector::{HtmlElements, HtmlElementsCollector};

/// Go: `publisher.Descriptor`.
pub struct Descriptor<'a> {
    /// The content to publish.
    pub src: &'a [u8],
    /// The OutputFormat of this content.
    pub output_format: OutputFormat,
    /// Where to publish this content (relative to the publish dir).
    pub target_path: String,
    /// Counter for the build summary.
    pub stat_counter: Option<&'a std::sync::atomic::AtomicU64>,
    /// Never set in builds (server only).
    pub live_reload_base_url: Option<String>,
    /// Always false in neohugo builds (inverted flag).
    pub add_hugo_generator_tag: bool,
    /// If set, will replace all relative URLs with this one (canonifyURLs; RSS always).
    pub abs_url_path: String,
    /// Enable to minify the output using the OutputFormat's MediaType.
    pub minify: bool,
}

/// Go: `publisher.PublishStats`.
#[derive(Clone, Debug, Default)]
pub struct PublishStats {
    pub html_elements: HtmlElements,
}

/// Go: `publisher.Publisher`.
pub trait Publisher: Send + Sync {
    fn publish(&self, d: Descriptor<'_>) -> Result<()>;
    fn publish_stats(&self) -> PublishStats;
}

/// Go: `publisher.DestinationPublisher` (one per site; each owns its collector).
pub struct DestinationPublisher {
    pub fs: Arc<dyn Fs>,
    pub min: MinifyClient,
    pub html_elements_collector: Option<Mutex<HtmlElementsCollector>>,
}

impl DestinationPublisher {
    // Go: publisher/publisher.go:NewDestinationPublisher
    pub fn new(fs: Arc<dyn Fs>, min: MinifyClient, build_stats: &nh_config::common_config::BuildStats) -> DestinationPublisher {
        todo!()
    }
}

impl Publisher for DestinationPublisher {
    // Go: publisher/publisher.go:Publish
    fn publish(&self, d: Descriptor<'_>) -> Result<()> {
        todo!()
    }

    // Go: publisher/publisher.go:PublishStats
    fn publish_stats(&self) -> PublishStats {
        todo!()
    }
}

/// Go: `createTransformerChain(f)`.
// Go: publisher/publisher.go:createTransformerChain
pub fn create_transformer_chain(p: &DestinationPublisher, d: &Descriptor<'_>) -> nh_transform::chain::Chain {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: publisher/publisher.go (190 lines; 4/4 funcs executed)
//   types: Descriptor, DestinationPublisher, PublishStats, Publisher
// EX L80-90: NewDestinationPublisher(rs *resources.Spec, outputFormats output.Formats, mediaTypes media.Types) (pub DestinationPublisher, err error)
// EX L94-133: (p DestinationPublisher) Publish(d Descriptor) error
// EX L135-143: (p DestinationPublisher) PublishStats() PublishStats
// EX L156-190: (p DestinationPublisher) createTransformerChain(f Descriptor) transform.Chain
// ---------------------------------------------------------------------------
