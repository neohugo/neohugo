//! Port of `resources/images/exif/exif.go`.
//!
//! MINIMAL (excludeFields='.*' => nothing decoded)
//!
//! Owner: Wave B task T10 (images).


/// Go: `exif.ExifInfo` (MINIMAL: seeksnack excludes all fields; nothing is decoded).
#[derive(Clone, Debug, Default)]
pub struct ExifInfo {
    pub lat: f64,
    pub long: f64,
    pub date: Option<go_value::Time>,
    pub tags: std::collections::BTreeMap<String, go_value::Value>,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/exif/exif.go (366 lines; 8/13 funcs executed)
//   types: ExifInfo, Decoder, Tags
//    L52-54: (d *Decoder) shouldInclude(s string) bool
//    L56-58: (d *Decoder) shouldExclude(s string) bool
// EX L60-69: IncludeFields(expression string) func(*Decoder) error
// EX L71-80: ExcludeFields(expression string) func(*Decoder) error
// EX L82-87: WithLatLongDisabled(disabled bool) func(*Decoder) error
// EX L89-94: WithDateDisabled(disabled bool) func(*Decoder) error
// EX L96-101: WithWarnLogger(warnl logg.LevelLogger) func(*Decoder) error
// EX L103-114: compileRegexp(expression string) (*regexp.Regexp, error)
// EX L116-125: NewDecoder(options ...func(*Decoder) error) (*Decoder, error)
//    L137-226: (d *Decoder) Decode(filename string, format imagemeta.ImageFormat, r io.Reader) (ex *ExifInfo, err error)
// EX L230-346: init()
//    L352-361: (v *Tags) UnmarshalJSON(b []byte) error
//    L364-366: (v Tags) MarshalJSON() ([]byte, error)
// ---------------------------------------------------------------------------
