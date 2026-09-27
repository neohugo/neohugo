//! Port of `output/config.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use go_value::{Map, Value};
use nh_common::Result;
use nh_config::namespace::ConfigNamespace;

use super::output_format::{Formats, OutputFormat};
use crate::media::media_type::Types;

/// Go: `output.OutputFormatConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputFormatConfig {
    pub media_type: String,
    pub format: OutputFormat,
}

/// Go: `output.DecodeConfig(mediaTypes, in)`.
// Go: output/config.go:DecodeConfig
pub fn decode_config(media_types: &Types, input: &Value) -> Result<ConfigNamespace<Map, Formats>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: output/config.go (144 lines; 1/2 funcs executed)
//   types: OutputFormatConfig
// EX L41-93: DecodeConfig(mediaTypes media.Types, in any) (*config.ConfigNamespace[map[string]OutputFormatConfig, Formats], error)
//    L95-144: decode(mediaTypes media.Types, input any, output *Format) error
// ---------------------------------------------------------------------------
