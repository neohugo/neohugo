//! Port of `markup/asciidocext/convert.go` (+ `asciidocext_config/config.go`).
//!
//! STUB: converting returns `neohugo-rs: asciidoc (asciidoctor) is not supported`.
//!
//! Owner: Wave B task T06 (markup).

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::decode::FieldRef;
use nh_config::decode_struct;

use crate::converter::converter::{
    Converter, DocumentContext, Provider, ProviderConfig, RenderContext, ResultRender,
    new_provider as converter_new_provider,
};

/// Go: `asciidocext_config.Config` (decoded as part of the markup config).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub backend: String,
    pub extensions: Vec<String>,
    pub attributes: BTreeMap<String, String>,
    pub no_header_or_footer: bool,
    pub safe_mode: String,
    pub section_numbers: bool,
    pub verbose: bool,
    pub trace: bool,
    pub failure_level: String,
    pub working_folder_current: bool,
    pub preserve_toc: bool,
}

decode_struct!(Config, "asciidocext_config.Config", |s| vec![
    FieldRef::new("Backend", &mut s.backend),
    FieldRef::new("Extensions", &mut s.extensions),
    FieldRef::new("Attributes", &mut s.attributes),
    FieldRef::new("NoHeaderOrFooter", &mut s.no_header_or_footer),
    FieldRef::new("SafeMode", &mut s.safe_mode),
    FieldRef::new("SectionNumbers", &mut s.section_numbers),
    FieldRef::new("Verbose", &mut s.verbose),
    FieldRef::new("Trace", &mut s.trace),
    FieldRef::new("FailureLevel", &mut s.failure_level),
    FieldRef::new("WorkingFolderCurrent", &mut s.working_folder_current),
    FieldRef::new("PreserveTOC", &mut s.preserve_toc),
]);

/// Go: `asciidocext_config.Default`.
pub fn default_config() -> Config {
    Config {
        backend: "html5".into(),
        extensions: Vec::new(),
        attributes: BTreeMap::new(),
        no_header_or_footer: true,
        safe_mode: "unsafe".into(),
        section_numbers: false,
        verbose: false,
        trace: false,
        failure_level: "fatal".into(),
        working_folder_current: false,
        preserve_toc: false,
    }
}

/// The stub converter (Go: `internal.AsciidocConverter`, which runs `asciidoctor`).
struct AsciidocConverter;

impl Converter for AsciidocConverter {
    fn convert(&self, _ctx: &RenderContext<'_>) -> Result<ResultRender> {
        Err(Error::new(
            "neohugo-rs: asciidoc (asciidoctor) is not supported",
        ))
    }
}

/// Go: `asciidocext.Provider.New(cfg)`: registering the converter works; converting fails.
// Go: markup/asciidocext/convert.go:New
pub fn new_provider(_cfg: &ProviderConfig) -> Result<Arc<dyn Provider>> {
    Ok(converter_new_provider(
        "asciidocext",
        Arc::new(|_ctx: DocumentContext| Ok(Arc::new(AsciidocConverter) as Arc<dyn Converter>)),
    ))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/asciidocext/convert.go (49 lines; 1/2 funcs executed)
//   types: provider
// OK L30-37: (p provider) New(cfg converter.ProviderConfig) (converter.Provider, error)
//    L40-49: Supports() bool STUB (asciidoctor is never used)
// ---------------------------------------------------------------------------
