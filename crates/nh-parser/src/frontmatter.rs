//! Port of `parser/frontmatter.go`, `parser/lowercase_camel_json.go`.
//!
//! Owner: Wave B task T03 (parser-langs).


use go_value::Value;

use crate::metadecoders::format::Format;
use nh_common::Result;

/// Go: `parser.InterfaceToConfig(in, format, w)` (config/front matter writer; used by `hugo config`).
// Go: parser/frontmatter.go:InterfaceToConfig
pub fn interface_to_config(v: &Value, format: Format, w: &mut Vec<u8>) -> Result<()> {
    todo!()
}

/// Go: `parser.ReplacingJSONMarshaller` (used by `hugo config --format json`, the nh-allconfig oracle).
pub struct ReplacingJsonMarshaller {
    pub value: Value,
    pub keys_to_lower: bool,
    pub omit_empty: bool,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: parser/frontmatter.go (117 lines; 0/2 funcs executed)
//    L35-78: InterfaceToConfig(in any, format metadecoders.Format, w io.Writer) error
//    L80-117: InterfaceToFrontMatter(in any, format metadecoders.Format, w io.Writer) error
// Source: parser/lowercase_camel_json.go (132 lines; 0/4 funcs executed)
//   types: NullBoolJSONMarshaller, LowerCaseCamelJSONMarshaller, ReplacingJSONMarshaller
//    L36-42: (c NullBoolJSONMarshaller) MarshalJSON() ([]byte, error)
//    L51-53: preserveUpperCaseKey(match []byte) bool
//    L55-79: (c LowerCaseCamelJSONMarshaller) MarshalJSON() ([]byte, error)
//    L88-132: (c ReplacingJSONMarshaller) MarshalJSON() ([]byte, error)
// ---------------------------------------------------------------------------
