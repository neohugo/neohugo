//! Port of `publisher/publisher.go`.
//!
//! Owner: Wave B task T07 (transform-publisher).

//! Go `publisher/publisher.go`: transformer chain (absURL HTML/XML -> [livereload] -> [generator]
//! -> minify), write to the publish fs (Create/truncate, MkdirAll parents), and for HTML formats
//! tee the FINAL bytes into the HTML elements collector (hugo_stats.json).

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use nh_common::{Error, Result};
use nh_hugofs::afero::Fs;
use nh_media::output::output_format::OutputFormat;
use nh_transform::chain::Chain;
use nh_transform::minifiers::minifiers::Client as MinifyClient;

use crate::html_elements_collector::{
    HtmlElements, HtmlElementsCollector, HtmlElementsCollectorWriter,
};

/// Go: `publisher.Descriptor` — describes the needed publishing chain for an item.
pub struct Descriptor<'a> {
    /// The content to publish.
    pub src: &'a [u8],
    /// The OutputFormat of this content.
    pub output_format: OutputFormat,
    /// Where to publish this content (relative to the publish dir).
    pub target_path: String,
    /// Counter for the build summary.
    pub stat_counter: Option<&'a AtomicU64>,
    /// LiveReload script will be injected if this is set (server only; never set in builds).
    pub live_reload_base_url: Option<String>,
    /// Enable to inject the Hugo generated tag in the header (always false in neohugo builds:
    /// the flag is inverted).
    pub add_hugo_generator_tag: bool,
    /// If set, will replace all relative URLs with this one (canonifyURLs; RSS always).
    pub abs_url_path: String,
    /// Go's `Minify` field; the chain reads the client's `MinifyOutput` instead (as in Go).
    pub minify: bool,
}

/// Go: `publisher.PublishStats`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PublishStats {
    pub html_elements: HtmlElements,
}

/// Go: `publisher.Publisher` — publishes a result file.
pub trait Publisher: Send + Sync {
    fn publish(&self, d: Descriptor<'_>) -> Result<()>;
    fn publish_stats(&self) -> PublishStats;
}

/// Go: `publisher.DestinationPublisher` (one per site; each owns its collector). This publisher
/// prepares and publishes an item to the defined destination, e.g. /public.
pub struct DestinationPublisher {
    pub fs: Arc<dyn Fs>,
    pub min: MinifyClient,
    pub html_elements_collector: Option<Mutex<HtmlElementsCollector>>,
}

impl DestinationPublisher {
    /// Go: `NewDestinationPublisher(rs, outputFormats, mediaTypes)`: the publish fs, a minifier
    /// client (`minifiers.New(mediaTypes, outputFormats, cfg)`, built by the caller) and a
    /// collector when `build.buildStats` is enabled.
    // Go: publisher/publisher.go:NewDestinationPublisher
    pub fn new(
        fs: Arc<dyn Fs>,
        min: MinifyClient,
        build_stats: &nh_config::common_config::BuildStats,
    ) -> DestinationPublisher {
        let mut class_collector = None;
        if build_stats.enabled() {
            class_collector = Some(Mutex::new(HtmlElementsCollector::new(build_stats.clone())));
        }
        DestinationPublisher {
            fs,
            min,
            html_elements_collector: class_collector,
        }
    }

    /// The bytes `Publish` writes for `d`: the transformer chain applied to the source (the
    /// source itself when the chain is empty).
    pub fn transform(&self, d: &Descriptor<'_>) -> Result<Vec<u8>> {
        let transformers = create_transformer_chain(self, d);
        if transformers.is_empty() {
            return Ok(d.src.to_vec());
        }
        transformers.apply(d.src).map_err(|err| {
            Error::new(format!(
                "failed to process {}: {err}",
                go_strconv::quote(&d.target_path)
            ))
        })
    }
}

impl Publisher for DestinationPublisher {
    /// Go: `Publish` — applies any relevant transformations and writes the file to its
    /// destination, e.g. /public.
    // Go: publisher/publisher.go:Publish
    fn publish(&self, d: Descriptor<'_>) -> Result<()> {
        if d.target_path.is_empty() {
            return Err(Error::new("Publish: must provide a TargetPath"));
        }

        // This is now what we write to disk.
        let src = self.transform(&d)?;

        let mut f = nh_helpers::path::open_file_for_writing(self.fs.as_ref(), &d.target_path)?;

        // io.Copy(w, src): a *bytes.Buffer source is written with one Write of all its bytes
        // (none when it is empty); io.MultiWriter writes to the file first, then to the
        // collector.
        let mut res: Result<()> = Ok(());
        if !src.is_empty() {
            res = f.write_all(&src).map_err(|e| Error::new(e.to_string()));
            if res.is_ok()
                && d.output_format.is_html
                && let Some(c) = &self.html_elements_collector
            {
                let mut c = c.lock().unwrap_or_else(|e| e.into_inner());
                let mut w = HtmlElementsCollectorWriter::new(&mut c);
                w.write(&src);
            }
        }
        if res.is_ok()
            && let Some(counter) = d.stat_counter
        {
            counter.fetch_add(1, Ordering::SeqCst);
        }

        // Go: `defer f.Close()` (its error is ignored).
        let _ = f.close();

        res
    }

    // Go: publisher/publisher.go:PublishStats
    fn publish_stats(&self) -> PublishStats {
        let Some(c) = &self.html_elements_collector else {
            return PublishStats::default();
        };

        let c = c.lock().unwrap_or_else(|e| e.into_inner());
        PublishStats {
            html_elements: c.get_html_elements(),
        }
    }
}

/// Go: `createTransformerChain(f)`.
// Go: publisher/publisher.go:createTransformerChain
pub fn create_transformer_chain(p: &DestinationPublisher, d: &Descriptor<'_>) -> Chain {
    let mut transformers = Chain::new_empty();

    let is_html = d.output_format.is_html;

    if !d.abs_url_path.is_empty() {
        if is_html {
            transformers.push(nh_transform::urlreplacers::absurl::new_abs_url_transformer(
                &d.abs_url_path,
            ));
        } else {
            // Assume XML.
            transformers.push(
                nh_transform::urlreplacers::absurl::new_abs_url_in_xml_transformer(&d.abs_url_path),
            );
        }
    }

    if is_html {
        if let Some(base) = &d.live_reload_base_url {
            // Go holds a parsed *url.URL; the port parses the string it was made from.
            match go_url::parse(base.as_bytes()) {
                Ok(u) => transformers.push(nh_transform::livereloadinject::new_transformer(&u)),
                Err(e) => {
                    let msg = e.to_string();
                    transformers.push(Box::new(move |_| Err(Error::new(msg.clone()))));
                }
            }
        }

        // This is only injected on the home page.
        if d.add_hugo_generator_tag {
            transformers.push(nh_transform::metainject::hugo_generator_transformer());
        }
    }

    if p.min.minify_output
        && let Some(minify_transformer) = p.min.transformer(&d.output_format.media_type)
    {
        transformers.push(minify_transformer);
    }

    transformers
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: publisher/publisher.go (190 lines; 4/4 funcs executed)
//   types: Descriptor, DestinationPublisher, PublishStats, Publisher
// OK L80-90: NewDestinationPublisher(rs *resources.Spec, outputFormats output.Formats, mediaTypes media.Types) (pub DestinationPublisher, err error)
// OK L94-133: (p DestinationPublisher) Publish(d Descriptor) error
// OK L135-143: (p DestinationPublisher) PublishStats() PublishStats
// OK L156-190: (p DestinationPublisher) createTransformerChain(f Descriptor) transform.Chain
// ---------------------------------------------------------------------------
