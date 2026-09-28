//! Port of `resources/images/exif/exif.go` and of the EXIF part of
//! `github.com/bep/imagemeta@v0.12.0` (the only source Hugo requests: `Sources: EXIF`).
//!
//! imagemeta files ported here: `imagemeta.go` (Decode, Tags, GetDateTime, GetLatLong),
//! `io.go` (streamReader), `helpers.go` (rat, value converters, printableString, errors),
//! `metadecoder_exif.go`, `imagedecoder_{jpg,png,webp,tif}.go` (EXIF paths); the tag table
//! `metadecoder_exif_fields.go` is `crate::exif_fields`. The IPTC and XMP decoders are not
//! ported: Hugo never enables those sources, so they are never reached.
//!
//! imagemeta's control flow uses panics: a failed read `panic(errStop)`s (the first plain EOF
//! is swallowed and the read returns the stale buffer), the tag limit `panic(ErrStopWalking)`s,
//! and Go runtime panics (a type assertion or slice bound in a converter) are recovered by
//! `Decode` as errors. The port models them with [`Fail`].
//!
//! Owner: Wave B task T10 (images).

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;

use go_value::{HostCtx, IntKind, Object, SliceType, UintKind, Value};
use nh_common::{Error, Result};
use nh_config::goregexp::Regexp;

use crate::exif_fields::exif_field_name;

/// A reader Hugo hands to the EXIF decoder (Go `io.ReadSeeker`).
pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek + ?Sized> ReadSeek for T {}

/// Go: `imagemeta.ImageFormat` (`-1` = Hugo's "no EXIF for this format").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Unknown = -1,
    Auto = 0,
    Jpeg = 1,
    Tiff = 2,
    Png = 3,
    WebP = 4,
}

/// Go: `exif.ExifInfo` — the decoded Exif data for an Image.
#[derive(Clone, Debug)]
pub struct ExifInfo {
    /// GPS latitude in degrees.
    pub lat: f64,
    /// GPS longitude in degrees.
    pub long: f64,
    /// Image creation date/time (Go's zero `time.Time` when absent).
    pub date: go_value::Time,
    /// A collection of the available Exif tags for this Image (Go `Tags map[string]any`).
    pub tags: BTreeMap<String, Value>,
}

impl Default for ExifInfo {
    fn default() -> Self {
        ExifInfo {
            lat: 0.0,
            long: 0.0,
            date: go_value::Time::zero(),
            tags: BTreeMap::new(),
        }
    }
}

/// Go: the functional options of `exif.NewDecoder` (`WithWarnLogger` is not modelled: the
/// port does not log imagemeta warnings, which never reach the output).
#[derive(Clone, Debug)]
pub enum DecoderOption {
    IncludeFields(String),
    ExcludeFields(String),
    WithLatLongDisabled(bool),
    WithDateDisabled(bool),
}

/// Go: `exif.Decoder`.
#[derive(Clone, Debug, Default)]
pub struct Decoder {
    include_fields_re: Option<Regexp>,
    exclude_fieldsr_re: Option<Regexp>,
    no_date: bool,
    no_lat_long: bool,
}

// Go: resources/images/exif/exif.go:compileRegexp
fn compile_regexp(expression: &str) -> Result<Option<Regexp>> {
    let expression = go_unicode::strings::trim_space_str(expression);
    if expression.is_empty() {
        return Ok(None);
    }
    let expression = if !expression.starts_with('(') {
        // Make it case insensitive
        format!("(?i){expression}")
    } else {
        expression.to_string()
    };

    Regexp::compile(&expression).map(Some).map_err(Error::new)
}

impl Decoder {
    // Go: resources/images/exif/exif.go:NewDecoder
    pub fn new(options: &[DecoderOption]) -> Result<Decoder> {
        let mut d = Decoder::default();
        for opt in options {
            match opt {
                // Go: resources/images/exif/exif.go:IncludeFields
                DecoderOption::IncludeFields(e) => d.include_fields_re = compile_regexp(e)?,
                // Go: resources/images/exif/exif.go:ExcludeFields
                DecoderOption::ExcludeFields(e) => d.exclude_fieldsr_re = compile_regexp(e)?,
                // Go: resources/images/exif/exif.go:WithLatLongDisabled
                DecoderOption::WithLatLongDisabled(b) => d.no_lat_long = *b,
                // Go: resources/images/exif/exif.go:WithDateDisabled
                DecoderOption::WithDateDisabled(b) => d.no_date = *b,
            }
        }
        Ok(d)
    }

    // Go: resources/images/exif/exif.go:shouldInclude
    fn should_include(&self, s: &str) -> bool {
        match &self.include_fields_re {
            None => true,
            Some(re) => re.match_string(s),
        }
    }

    // Go: resources/images/exif/exif.go:shouldExclude
    fn should_exclude(&self, s: &str) -> bool {
        match &self.exclude_fieldsr_re {
            None => false,
            Some(re) => re.match_string(s),
        }
    }

    /// Filename is only used for logging. Go returns the (partial) `ExifInfo` together with a
    /// non-nil error; Hugo discards it then, so the port returns only the error.
    // Go: resources/images/exif/exif.go:Decode
    pub fn decode(
        &self,
        _filename: &str,
        format: ImageFormat,
        r: &mut dyn ReadSeek,
    ) -> Result<ExifInfo> {
        let mut tag_infos = Tags::default();

        let err = imagemeta_decode(self, format, r, &mut tag_infos);

        // (Go continues after an imagemeta error and returns both; a panic below is recovered
        // as "exif failed: %v".)
        let finish = || -> std::result::Result<ExifInfo, String> {
            let mut tm = go_value::Time::zero();
            let (mut lat, mut long) = (0.0, 0.0);

            if !self.no_date {
                tm = tag_infos.get_date_time()?;
            }

            if !self.no_lat_long {
                (lat, long) = tag_infos.get_lat_long()?;
            }

            let mut tags = BTreeMap::new();
            for (k, v) in tag_infos.all() {
                if self.should_exclude(k) {
                    continue;
                }
                if !self.should_include(k) {
                    continue;
                }
                tags.insert(k.clone(), v.value.to_value());
            }

            Ok(ExifInfo {
                lat,
                long,
                date: tm,
                tags,
            })
        };

        match finish() {
            // The deferred recover replaces any imagemeta error.
            Err(p) => Err(Error::new(format!("exif failed: {p}"))),
            Ok(ex) => match err {
                Some(e) => Err(Error::new(e)),
                None => Ok(ex),
            },
        }
    }

    /// Hugo's `shouldInclude` closure (`ShouldHandleTag`).
    fn should_handle_tag(&self, ti: &TagInfo) -> bool {
        if ti.source == SOURCE_EXIF {
            if !self.no_date {
                // We need the time tags to calculate the date.
                if is_time_tag(&ti.tag) {
                    return true;
                }
            }
            if !self.no_lat_long {
                // We need to GPS tags to calculate the lat/long.
                if is_gps_tag(&ti.tag) {
                    return true;
                }
            }

            if !ti.namespace.starts_with("IFD0") {
                // Drop thumbnail tags.
                return false;
            }
        }

        if self.should_exclude(&ti.tag) {
            return false;
        }

        self.should_include(&ti.tag)
    }
}

// Go: resources/images/exif/exif.go:isTimeTag
fn is_time_tag(s: &str) -> bool {
    s.contains("Time")
}

// Go: resources/images/exif/exif.go:isGPSTag
fn is_gps_tag(s: &str) -> bool {
    s.starts_with("GPS")
}

// ---------------------------------------------------------------------------
// imagemeta: values

/// A tag value with its Go dynamic type (imagemeta `TagInfo.Value any`).
#[derive(Clone, Debug, PartialEq)]
pub enum TagValue {
    /// nil
    Nil,
    /// `uint8` (`byte`)
    U8(u8),
    U16(u16),
    U32(u32),
    I32(i32),
    /// `int`
    Int(i64),
    F32(f32),
    F64(f64),
    /// `string` (Go bytes)
    Str(Vec<u8>),
    /// `[]byte`
    Bytes(Vec<u8>),
    /// `[]any`
    Any(Vec<TagValue>),
    /// `*imagemeta.rat[uint32]`
    RatU(u32, u32),
    /// `*imagemeta.rat[int32]`
    RatI(i32, i32),
    /// `time.Time{}` (convertToTimestampString's odd return)
    ZeroTime,
}

impl TagValue {
    fn str(s: &str) -> TagValue {
        TagValue::Str(s.as_bytes().to_vec())
    }

    /// Go's `%T`.
    pub fn go_type(&self) -> &'static str {
        match self {
            TagValue::Nil => "<nil>",
            TagValue::U8(_) => "uint8",
            TagValue::U16(_) => "uint16",
            TagValue::U32(_) => "uint32",
            TagValue::I32(_) => "int32",
            TagValue::Int(_) => "int",
            TagValue::F32(_) => "float32",
            TagValue::F64(_) => "float64",
            TagValue::Str(_) => "string",
            TagValue::Bytes(_) => "[]uint8",
            TagValue::Any(_) => "[]interface {}",
            TagValue::RatU(..) => "*imagemeta.rat[uint32]",
            TagValue::RatI(..) => "*imagemeta.rat[int32]",
            TagValue::ZeroTime => "time.Time",
        }
    }

    /// The template value (Go `Tags` map values).
    pub fn to_value(&self) -> Value {
        match self {
            TagValue::Nil => Value::Invalid,
            TagValue::U8(v) => Value::Uint(*v as u64, UintKind::Uint8),
            TagValue::U16(v) => Value::Uint(*v as u64, UintKind::Uint16),
            TagValue::U32(v) => Value::Uint(*v as u64, UintKind::Uint32),
            TagValue::I32(v) => Value::Int(*v as i64, IntKind::Int32),
            TagValue::Int(v) => Value::int(*v),
            TagValue::F32(v) => Value::Float(*v as f64, go_value::FloatKind::F32),
            TagValue::F64(v) => Value::float64(*v),
            TagValue::Str(s) => Value::string(s.clone()),
            TagValue::Bytes(b) => Value::list(
                SliceType::Uint8,
                b.iter()
                    .map(|&x| Value::Uint(x as u64, UintKind::Uint8))
                    .collect(),
            ),
            TagValue::Any(items) => {
                Value::list(SliceType::Any, items.iter().map(|v| v.to_value()).collect())
            }
            TagValue::RatU(n, d) => Value::object(Rat::Uint32(*n, *d)),
            TagValue::RatI(n, d) => Value::object(Rat::Int32(*n, *d)),
            TagValue::ZeroTime => Value::Time(go_value::Time::zero()),
        }
    }

    /// Go's `float64Provider` (the rats).
    fn float64_provider(&self) -> Option<f64> {
        match self {
            TagValue::RatU(n, d) => Some(*n as f64 / *d as f64),
            TagValue::RatI(n, d) => Some(*n as f64 / *d as f64),
            _ => None,
        }
    }
}

/// Go: `imagemeta.rat[T]` (a pointer in the tag values): `String()` is `num` when the
/// denominator is 1, else `num/den`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rat {
    Uint32(u32, u32),
    Int32(i32, i32),
}

impl Rat {
    // Go: helpers.go:rat.String
    pub fn string(&self) -> String {
        match *self {
            Rat::Uint32(n, 1) => n.to_string(),
            Rat::Uint32(n, d) => format!("{n}/{d}"),
            Rat::Int32(n, 1) => n.to_string(),
            Rat::Int32(n, d) => format!("{n}/{d}"),
        }
    }

    // Go: helpers.go:rat.Float64
    pub fn float64(&self) -> f64 {
        match *self {
            Rat::Uint32(n, d) => n as f64 / d as f64,
            Rat::Int32(n, d) => n as f64 / d as f64,
        }
    }
}

impl Object for Rat {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(match self {
            Rat::Uint32(..) => "*imagemeta.rat[uint32]",
            Rat::Int32(..) => "*imagemeta.rat[int32]",
        })
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "Num" | "Den" | "Float64" | "String" | "MarshalText")
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if !self.has_method(name) {
            return None;
        }
        if !args.is_empty() {
            return Some(Err(go_value::Error::new(format!(
                "wrong number of args for {name}: want 0 got {}",
                args.len()
            ))));
        }
        Some(Ok(match (name, *self) {
            ("Num", Rat::Uint32(n, _)) => Value::Uint(n as u64, UintKind::Uint32),
            ("Den", Rat::Uint32(_, d)) => Value::Uint(d as u64, UintKind::Uint32),
            ("Num", Rat::Int32(n, _)) => Value::Int(n as i64, IntKind::Int32),
            ("Den", Rat::Int32(_, d)) => Value::Int(d as i64, IntKind::Int32),
            ("Float64", _) => Value::float64(self.float64()),
            ("MarshalText", _) => Value::list(
                SliceType::Uint8,
                self.string()
                    .bytes()
                    .map(|b| Value::Uint(b as u64, UintKind::Uint8))
                    .collect(),
            ),
            _ => Value::string(self.string()),
        }))
    }
    fn go_string(&self) -> Option<go_value::GoString> {
        Some(self.string().into())
    }
    fn marshal_text(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Ok(self.string().into_bytes()))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `imagemeta.NewRat[uint32](num, den)`.
// Go: helpers.go:NewRat
fn new_rat_u32(num: u32, den: u32) -> std::result::Result<TagValue, String> {
    if den == 0 {
        return Err("denominator must be non-zero".into());
    }
    // Remove the greatest common divisor.
    let gcd = |mut a: u32, mut b: u32| {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    };
    let d = gcd(num, den);
    let (mut num, mut den) = (num, den);
    if d != 1 {
        (num, den) = (num / d, den / d);
    }
    Ok(TagValue::RatU(num, den))
}

/// Go: `imagemeta.NewRat[int32](num, den)` (Go's wrapping division and remainder).
fn new_rat_i32(num: i32, den: i32) -> std::result::Result<TagValue, String> {
    if den == 0 {
        return Err("denominator must be non-zero".into());
    }
    let gcd = |mut a: i32, mut b: i32| {
        while b != 0 {
            (a, b) = (b, a.wrapping_rem(b));
        }
        a
    };
    let d = gcd(num, den);
    let (mut num, mut den) = (num, den);
    if d != 1 {
        (num, den) = (num.wrapping_div(d), den.wrapping_div(d));
    }
    // Denominator must be positive.
    if den < 0 {
        (num, den) = (num.wrapping_neg(), den.wrapping_neg());
    }
    Ok(TagValue::RatI(num, den))
}

/// Go: `imagemeta.TagInfo`.
#[derive(Clone, Debug)]
pub struct TagInfo {
    pub source: u32,
    pub tag: String,
    pub namespace: String,
    pub value: TagValue,
}

const SOURCE_EXIF: u32 = 1;

/// Go: `imagemeta.Tags` (EXIF only).
#[derive(Default, Debug)]
pub struct Tags {
    exif: BTreeMap<String, TagInfo>,
}

impl Tags {
    // Go: imagemeta.go:(*Tags).Add
    fn add(&mut self, tag: TagInfo) {
        self.exif.insert(tag.tag.clone(), tag);
    }

    /// The decoded tags (for tests).
    pub fn exif(&self) -> &BTreeMap<String, TagInfo> {
        &self.exif
    }

    // Go: imagemeta.go:Tags.All
    fn all(&self) -> &BTreeMap<String, TagInfo> {
        &self.exif
    }

    /// Go: `GetDateTime` (`time.ParseInLocation("2006:01:02 15:04:05", s, time.Local)`; Hugo
    /// ignores the parse error, which gives the zero time). `Err` is a Go panic (a
    /// non-string date tag).
    // Go: imagemeta.go:Tags.GetDateTime
    fn get_date_time(&self) -> std::result::Result<go_value::Time, String> {
        let date_str = self.date_time()?;
        if date_str.is_empty() {
            return Ok(go_value::Time::zero());
        }

        // (location(): "Canon.TimeInfo" is never an EXIF tag name, so loc is time.Local.)
        let loc = go_time::local();

        const LAYOUT: &str = "2006:01:02 15:04:05";

        Ok(go_time::parse_in_location(LAYOUT, &date_str, &loc)
            .unwrap_or_else(|_| go_value::Time::zero()))
    }

    /// Go: `GetLatLong`. `Err` is a Go panic (a failed type assertion).
    // Go: imagemeta.go:Tags.GetLatLong
    fn get_lat_long(&self) -> std::result::Result<(f64, f64), String> {
        let mut ns: &[u8] = b"";
        let mut ew: &[u8] = b"";

        let exif = &self.exif;

        let Some(long_tag) = exif.get("GPSLongitude") else {
            return Ok((0.0, 0.0));
        };
        if let Some(ew_tag) = exif.get("GPSLongitudeRef") {
            ew = assert_string(&ew_tag.value)?;
        }
        let Some(lat_tag) = exif.get("GPSLatitude") else {
            return Ok((0.0, 0.0));
        };
        if let Some(ns_tag) = exif.get("GPSLatitudeRef") {
            ns = assert_string(&ns_tag.value)?;
        }

        let mut lat = assert_float64(&lat_tag.value)?;
        let mut long = assert_float64(&long_tag.value)?;

        if ns == b"S" {
            lat = -lat;
        }

        if ew == b"W" {
            long = -long;
        }

        if lat.is_nan() {
            lat = 0.0;
        }
        if long.is_nan() {
            long = 0.0;
        }

        Ok((lat, long))
    }

    // Go: imagemeta.go:Tags.dateTime
    fn date_time(&self) -> std::result::Result<Vec<u8>, String> {
        if let Some(ti) = self.exif.get("DateTimeOriginal") {
            return Ok(assert_string(&ti.value)?.to_vec());
        }
        if let Some(ti) = self.exif.get("DateTime") {
            return Ok(assert_string(&ti.value)?.to_vec());
        }
        Ok(Vec::new())
    }
}

/// Go's `v.(string)` (the panic message of a failed assertion).
fn assert_string(v: &TagValue) -> std::result::Result<&[u8], String> {
    match v {
        TagValue::Str(s) => Ok(s),
        other => Err(type_assertion_panic(other, "string")),
    }
}

/// Go's `v.(float64)`.
fn assert_float64(v: &TagValue) -> std::result::Result<f64, String> {
    match v {
        TagValue::F64(f) => Ok(*f),
        other => Err(type_assertion_panic(other, "float64")),
    }
}

fn type_assertion_panic(v: &TagValue, want: &str) -> String {
    match v {
        TagValue::Nil => format!("interface conversion: interface {{}} is nil, not {want}"),
        _ => format!(
            "interface conversion: interface {{}} is {}, not {want}",
            v.go_type()
        ),
    }
}

// ---------------------------------------------------------------------------
// imagemeta: control flow and errors

/// A returned imagemeta error.
#[derive(Clone, Debug, PartialEq)]
enum DErr {
    /// `io.EOF`
    Eof,
    /// `io.ErrUnexpectedEOF`
    UnexpectedEof,
    /// `*InvalidFormatError`
    InvalidFormat(String),
    /// Any other error (its text).
    Msg(String),
}

impl DErr {
    fn message(&self) -> String {
        match self {
            DErr::Eof => "EOF".into(),
            DErr::UnexpectedEof => "unexpected EOF".into(),
            DErr::InvalidFormat(m) => format!("invalid format: {m}"),
            DErr::Msg(m) => m.clone(),
        }
    }
}

/// How an imagemeta call ends when it does not return normally.
#[derive(Clone, Debug)]
enum Fail {
    /// `panic(errStop)` from `streamReader.stop`.
    Stop,
    /// `panic(ErrStopWalking)` (the tag limit).
    StopWalking,
    /// A Go runtime panic (its message).
    Panic(String),
    /// A returned error.
    Err(DErr),
}

type R<T> = std::result::Result<T, Fail>;

fn invalid_format(msg: impl Into<String>) -> Fail {
    Fail::Err(DErr::InvalidFormat(msg.into()))
}

/// Go: `errInvalidFormat`.
fn err_invalid_format() -> Fail {
    invalid_format("invalid format")
}

// Go: helpers.go:isInvalidFormatErrorCandidate
fn is_invalid_format_error_candidate(msg: &str) -> bool {
    msg.contains("unexpected EOF")
}

/// Go: `imagemeta.Decode`'s two deferred functions (`errFromRecover`, then `errFinal`): the
/// final error text, or `None`.
fn decode_result(res: R<()>) -> Option<String> {
    // errFromRecover: a recovered panic becomes the error.
    let err: Option<String> = match res {
        Ok(()) => None,
        // errStop and ErrStopWalking are errors; errFinal maps both to nil.
        Err(Fail::Stop) | Err(Fail::StopWalking) => return None,
        Err(Fail::Panic(msg)) => Some(if is_invalid_format_error_candidate(&msg) {
            format!("invalid format: {msg}")
        } else {
            msg
        }),
        Err(Fail::Err(DErr::Eof)) => return None,
        Err(Fail::Err(e)) => Some(e.message()),
    };
    // errFinal.
    let e = err?;
    if is_invalid_format_error_candidate(&e) {
        return Some(format!("invalid format: {e}"));
    }
    Some(e)
}

// ---------------------------------------------------------------------------
// imagemeta: io.go

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ByteOrder {
    Big,
    Little,
}

impl ByteOrder {
    fn uint16(self, b: &[u8]) -> u16 {
        match self {
            ByteOrder::Big => u16::from_be_bytes([b[0], b[1]]),
            ByteOrder::Little => u16::from_le_bytes([b[0], b[1]]),
        }
    }
    fn uint32(self, b: &[u8]) -> u32 {
        let a = [b[0], b[1], b[2], b[3]];
        match self {
            ByteOrder::Big => u32::from_be_bytes(a),
            ByteOrder::Little => u32::from_le_bytes(a),
        }
    }
    fn uint64(self, b: &[u8]) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[..8]);
        match self {
            ByteOrder::Big => u64::from_be_bytes(a),
            ByteOrder::Little => u64::from_le_bytes(a),
        }
    }
    /// `binary.ByteOrder.Uint16` of a slice that may be too short (Go panics).
    fn checked_uint16(self, b: &[u8]) -> R<u16> {
        if b.len() < 2 {
            return Err(Fail::Panic(format!(
                "runtime error: index out of range [1] with length {}",
                b.len()
            )));
        }
        Ok(self.uint16(b))
    }
}

/// Go `io.ReadFull` over a Rust reader.
fn read_full(r: &mut dyn Read, buf: &mut [u8]) -> std::result::Result<(), DErr> {
    let mut n = 0;
    while n < buf.len() {
        match r.read(&mut buf[n..]) {
            Ok(0) => {
                return Err(if n == 0 {
                    DErr::Eof
                } else {
                    DErr::UnexpectedEof
                });
            }
            Ok(m) => n += m,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(DErr::Msg(e.to_string())),
        }
    }
    Ok(())
}

/// The mutable state of a Go `streamReader` other than its reader.
struct BufState {
    byte_order: ByteOrder,
    /// Go `buf`: `buf.len()` is Go's `cap(buf)` (reads use the prefix).
    buf: Vec<u8>,
    is_eof: bool,
}

impl BufState {
    fn new(byte_order: ByteOrder) -> BufState {
        BufState {
            byte_order,
            buf: Vec::new(),
            is_eof: false,
        }
    }

    // Go: io.go:allocateBuf
    fn allocate_buf(&mut self, length: usize) {
        if length > self.buf.len() {
            self.buf = vec![0; length];
        }
    }

    // Go: io.go:readNFromRIntoBufE
    fn read_n_from_r_into_buf_e(
        &mut self,
        n: usize,
        r: &mut dyn Read,
    ) -> std::result::Result<(), DErr> {
        self.allocate_buf(n);
        read_full(r, &mut self.buf[..n])
    }

    // Go: io.go:readNFromRIntoBuf
    fn read_n_from_r_into_buf(&mut self, n: usize, r: &mut dyn Read) -> R<()> {
        if let Err(err) = self.read_n_from_r_into_buf_e(n, r) {
            self.stop(err)?;
        }
        Ok(())
    }

    // Go: io.go:stop
    fn stop(&mut self, err: DErr) -> R<()> {
        // Alow one silent EOF.
        // This allows the client to not having to check for EOF on every read.
        if err == DErr::Eof && !self.is_eof {
            self.is_eof = true;
            return Ok(());
        }
        Err(Fail::Stop)
    }

    // Go: io.go:read1r
    fn read1r(&mut self, r: &mut dyn Read) -> R<u8> {
        self.read_n_from_r_into_buf(1, r)?;
        Ok(self.buf[0])
    }

    // Go: io.go:read2r
    fn read2r(&mut self, r: &mut dyn Read) -> R<u16> {
        self.read_n_from_r_into_buf(2, r)?;
        Ok(self.byte_order.uint16(&self.buf[..2]))
    }

    // Go: io.go:read4r
    fn read4r(&mut self, r: &mut dyn Read) -> R<u32> {
        self.read_n_from_r_into_buf(4, r)?;
        Ok(self.byte_order.uint32(&self.buf[..4]))
    }

    // Go: io.go:read4sr
    fn read4sr(&mut self, r: &mut dyn Read) -> R<i32> {
        self.read_n_from_r_into_buf(4, r)?;
        Ok(self.byte_order.uint32(&self.buf[..4]) as i32)
    }

    // Go: io.go:read8r
    fn read8r(&mut self, r: &mut dyn Read) -> R<u64> {
        self.read_n_from_r_into_buf(8, r)?;
        Ok(self.byte_order.uint64(&self.buf[..8]))
    }

    // Go: io.go:readBytesFromRVolatile
    fn read_bytes_from_r_volatile(&mut self, n: usize, r: &mut dyn Read) -> R<&[u8]> {
        self.read_n_from_r_into_buf(n, r)?;
        Ok(&self.buf[..n])
    }
}

/// Go: `imagemeta.streamReader`.
struct StreamReader<'a> {
    r: Box<dyn ReadSeek + 'a>,
    st: BufState,
    reader_offset: i64,
}

/// Go `bytes.Reader`.
type BytesReader = std::io::Cursor<Vec<u8>>;

// 10 MB should be plenty for image metadata.
const MAX_BUF_SIZE: i64 = 10 * 1024 * 1024;

impl<'a> StreamReader<'a> {
    // Go: io.go:newStreamReader
    fn new(r: Box<dyn ReadSeek + 'a>, byte_order: ByteOrder) -> StreamReader<'a> {
        StreamReader {
            r,
            st: BufState::new(byte_order),
            reader_offset: 0,
        }
    }

    // Go: io.go:otherByteOrder
    fn other_byte_order(&self) -> ByteOrder {
        if self.st.byte_order == ByteOrder::Big {
            ByteOrder::Little
        } else {
            ByteOrder::Big
        }
    }

    /// bufferedReader reads length bytes from the stream and returns a reader over them.
    // Go: io.go:bufferedReader
    fn buffered_reader(&mut self, length: i64) -> R<BytesReader> {
        if length > MAX_BUF_SIZE {
            return Err(invalid_format(format!(
                "length {length} exceeds max {MAX_BUF_SIZE}"
            )));
        }
        if length == 0 {
            return Ok(BytesReader::new(Vec::new()));
        }

        if length < 0 {
            return Err(invalid_format("negative length"));
        }

        let mut b = vec![0u8; length as usize];
        read_full(&mut self.r, &mut b).map_err(Fail::Err)?;

        Ok(BytesReader::new(b))
    }

    // Go: io.go:pos
    fn pos(&mut self) -> i64 {
        self.r.stream_position().map(|p| p as i64).unwrap_or(0)
    }

    // Go: io.go:read1
    fn read1(&mut self) -> R<u8> {
        self.st.read1r(&mut self.r)
    }

    // Go: io.go:read2
    fn read2(&mut self) -> R<u16> {
        self.st.read2r(&mut self.r)
    }

    // Go: io.go:read2E
    fn read2_e(&mut self) -> std::result::Result<u16, DErr> {
        self.st.read_n_from_r_into_buf_e(2, &mut self.r)?;
        Ok(self.st.byte_order.uint16(&self.st.buf[..2]))
    }

    // Go: io.go:read4
    fn read4(&mut self) -> R<u32> {
        self.st.read4r(&mut self.r)
    }

    // Go: io.go:readBytes
    fn read_bytes(&mut self, b: &mut [u8]) -> R<()> {
        if let Err(err) = read_full(&mut self.r, b) {
            self.st.stop(err)?;
        }
        Ok(())
    }

    /// readNullTerminatedBytes reads a slice of bytes from the stream until a null byte is
    /// encountered (max includes the null byte). Returns the bytes and the number read.
    // Go: io.go:readNullTerminatedBytes
    fn read_null_terminated_bytes(&mut self, max: usize) -> R<(Vec<u8>, i64)> {
        let mut b = Vec::new();
        let mut n = 0i64;
        for i in 0..max {
            b.push(self.read1()?);
            n += 1;
            if b[i] == 0 {
                b.truncate(i);
                return Ok((b, n));
            }
        }
        Ok((b, n))
    }

    // Go: io.go:readBytesVolatile
    fn read_bytes_volatile(&mut self, n: usize) -> R<Vec<u8>> {
        self.st.read_n_from_r_into_buf(n, &mut self.r)?;
        Ok(self.st.buf[..n].to_vec())
    }

    // Go: io.go:seek
    fn seek(&mut self, pos: i64) -> R<()> {
        if pos < 0 || self.r.seek(SeekFrom::Start(pos as u64)).is_err() {
            // e.stop(err) with a non-EOF error.
            return Err(Fail::Stop);
        }
        Ok(())
    }

    // Go: io.go:skip (errors ignored)
    fn skip(&mut self, n: i64) {
        let _ = self.r.seek(SeekFrom::Current(n));
    }
}

// ---------------------------------------------------------------------------
// imagemeta: Decode and the image decoders

/// Hugo's `imagemeta.Decode(imagemeta.Options{..., Sources: imagemeta.EXIF})` with its
/// `ShouldHandleTag`/`HandleTag` closures: the error text, or `None`.
// Go: imagemeta.go:Decode
fn imagemeta_decode(
    d: &Decoder,
    format: ImageFormat,
    r: &mut dyn ReadSeek,
    tags: &mut Tags,
) -> Option<String> {
    if format == ImageFormat::Auto {
        return Some("no image format provided; format detection not implemented yet".into());
    }

    const DEFAULT_LIMIT_NUM_TAGS: u32 = 5000;
    const DEFAULT_LIMIT_TAG_SIZE: u32 = 10000;

    // Remove sources not supported by the format; Hugo requests EXIF, which every
    // supported format has.
    match format {
        ImageFormat::Jpeg | ImageFormat::Tiff | ImageFormat::WebP | ImageFormat::Png => {}
        _ => return Some("unsupported image format".into()),
    }

    let mut ctx = Ctx {
        d,
        tags,
        tag_count: 0,
        limit_num_tags: DEFAULT_LIMIT_NUM_TAGS,
        limit_tag_size: DEFAULT_LIMIT_TAG_SIZE,
    };

    let mut base = StreamReader::new(Box::new(r), ByteOrder::Big);

    let res = match format {
        ImageFormat::Jpeg => decode_jpeg(&mut base, &mut ctx),
        ImageFormat::Tiff => decode_tif(&mut base, &mut ctx),
        ImageFormat::WebP => {
            base.st.byte_order = ByteOrder::Little;
            decode_webp(&mut base, &mut ctx)
        }
        _ => decode_png(&mut base, &mut ctx),
    };

    decode_result(res)
}

/// The decode options Hugo passes (`Options.ShouldHandleTag`, `HandleTag`, the limits) and
/// the collected tags.
struct Ctx<'c> {
    d: &'c Decoder,
    tags: &'c mut Tags,
    tag_count: u32,
    limit_num_tags: u32,
    limit_tag_size: u32,
}

impl Ctx<'_> {
    /// The `ShouldHandleTag` wrapper of `Decode` (counts tags; `ErrStopWalking` over the
    /// limit) around Hugo's `shouldInclude`.
    fn should_handle_tag(&mut self, ti: &TagInfo) -> R<bool> {
        self.tag_count += 1;
        if self.tag_count > self.limit_num_tags {
            return Err(Fail::StopWalking);
        }
        Ok(self.d.should_handle_tag(ti))
    }

    /// Hugo's `handleTag`: `tagInfos.Add(ti)`.
    fn handle_tag(&mut self, ti: TagInfo) {
        self.tags.add(ti);
    }
}

const MARKER_SOI: u16 = 0xffd8;
const MARKER_APP1_EXIF: u16 = 0xffe1;
const MARKER_SOS: u16 = 0xffda;
const EXIF_HEADER: u32 = 0x45786966;
const BYTE_ORDER_BIG_ENDIAN: u16 = 0x4d4d;
const BYTE_ORDER_LITTLE_ENDIAN: u16 = 0x4949;
const TAG_NAME_THUMBNAIL_OFFSET: &str = "ThumbnailOffset";
const XMP_MARKER: u16 = 0x02bc;
const IPTC_MARKER: u16 = 0x83bb;

// Go: imagedecoder_jpg.go:(*imageDecoderJPEG).decode
fn decode_jpeg(e: &mut StreamReader<'_>, ctx: &mut Ctx<'_>) -> R<()> {
    // JPEG SOI marker.
    let Ok(soi) = e.read2_e() else {
        return Ok(());
    };

    if soi != MARKER_SOI {
        return Ok(());
    }

    // Hugo requests EXIF only (IPTC and XMP are removed from the source set).
    let mut has_exif = true;

    loop {
        if !has_exif {
            // Done.
            return Ok(());
        }
        let marker = e.read2()?;
        if e.st.is_eof {
            return Ok(());
        }

        if marker == 0 {
            continue;
        }

        if marker == MARKER_SOS {
            // Start of scan. We're done.
            return Ok(());
        }

        // Read the 16-bit length of the segment. The value includes the 2 bytes for the
        // length itself, so we subtract 2 to get the number of remaining bytes.
        let mut length = e.read2()?;
        if length < 2 {
            return Err(err_invalid_format());
        }
        length -= 2;

        if marker == MARKER_APP1_EXIF {
            has_exif = false;
            jpeg_handle_exif(e, ctx, length as i64)?;
            continue;
        }

        e.skip(length as i64);
    }
}

// Go: imagedecoder_jpg.go:(*imageDecoderJPEG).handleEXIF
fn jpeg_handle_exif(e: &mut StreamReader<'_>, ctx: &mut Ctx<'_>, length: i64) -> R<()> {
    let thumbnail_offset = e.pos();
    let r = e.buffered_reader(length)?;
    let mut s = StreamReader::new(Box::new(r), e.st.byte_order);
    let mut exifr = MetaDecoderExif::new(&mut s, thumbnail_offset);

    let header = exifr.s.read4()?;
    if header != EXIF_HEADER {
        // Go returns err, which is nil here.
        return Ok(());
    }
    exifr.s.skip(2);

    exifr.decode(ctx)
}

// Go: imagedecoder_png.go:(*imageDecoderPNG).decode
fn decode_png(e: &mut StreamReader<'_>, ctx: &mut Ctx<'_>) -> R<()> {
    // Skip header.
    e.skip(8);

    let mut has_exif = true;

    loop {
        if !has_exif {
            return Ok(());
        }
        let chunk_length = e.read4()?;
        let tag_id = e.read_bytes_volatile(4)?;
        if has_exif && tag_id == b"eXIf" {
            has_exif = false;
            let r = e.buffered_reader(chunk_length as i64)?;
            let mut s = StreamReader::new(Box::new(r), e.st.byte_order);
            let mut exifr = MetaDecoderExif::new(&mut s, 0);
            exifr.decode(ctx)?;
            e.skip(4); // skip CRC
        } else if tag_id == b"zTXt" {
            // Profile Name is 1-79 bytes, followed by the null character.
            // Note that profileNameLength includes the null character.
            let (_profile_name, profile_name_length) = e.read_null_terminated_bytes(79 + 1)?;
            // "Raw profile type iptc" (IPTC is not requested), "Raw profile type exif" and
            // everything else are skipped.
            e.skip(chunk_length as i64 - profile_name_length);
            e.skip(4); // skip CRC
        } else {
            e.skip(chunk_length as i64);
            e.skip(4); // skip CRC
        }
    }
}

// Go: imagedecoder_webp.go:(*decoderWebP).decode
fn decode_webp(e: &mut StreamReader<'_>, ctx: &mut Ctx<'_>) -> R<()> {
    const FCC_RIFF: [u8; 4] = *b"RIFF";
    const FCC_WEBP: [u8; 4] = *b"WEBP";
    const FCC_VP8X: [u8; 4] = *b"VP8X";
    const FCC_EXIF: [u8; 4] = *b"EXIF";

    // EXIF is the only requested source (XMP is removed from the source set).
    let mut has_exif = true;

    let mut buf = [0u8; 10];

    let mut chunk_id = [0u8; 4];
    // Read the RIFF header.
    e.read_bytes(&mut chunk_id)?;
    if chunk_id != FCC_RIFF {
        return Err(err_invalid_format());
    }

    // File size.
    e.skip(4);

    e.read_bytes(&mut chunk_id)?;
    if chunk_id != FCC_WEBP {
        return Err(err_invalid_format());
    }

    loop {
        if !has_exif {
            return Ok(());
        }

        e.read_bytes(&mut chunk_id)?;
        if e.st.is_eof {
            return Ok(());
        }

        let chunk_len = e.read4()?;

        if chunk_id == FCC_VP8X {
            if chunk_len != 10 {
                return Err(err_invalid_format());
            }

            const EXIF_METADATA_BIT: u8 = 1 << 3;

            e.read_bytes(&mut buf)?;

            let has_exif_bit = buf[0] & EXIF_METADATA_BIT != 0;

            if !has_exif_bit {
                has_exif = false;
            }

            if !has_exif {
                return Ok(());
            }
        } else if chunk_id == FCC_EXIF && has_exif {
            has_exif = false;
            let thumbnail_offset = e.pos();
            let r = e.buffered_reader(chunk_len as i64)?;
            let mut s = StreamReader::new(Box::new(r), e.st.byte_order);
            let mut dec = MetaDecoderExif::new(&mut s, thumbnail_offset);
            dec.decode(ctx)?;
        } else {
            e.skip(chunk_len as i64);
        }
    }
}

// Go: imagedecoder_tif.go:(*imageDecoderTIF).decode
fn decode_tif(e: &mut StreamReader<'_>, ctx: &mut Ctx<'_>) -> R<()> {
    const MEANING_OF_LIFE: u16 = 42;

    let byte_order_tag = e.read2()?;
    match byte_order_tag {
        BYTE_ORDER_BIG_ENDIAN => e.st.byte_order = ByteOrder::Big,
        BYTE_ORDER_LITTLE_ENDIAN => e.st.byte_order = ByteOrder::Little,
        _ => return Err(err_invalid_format()),
    }

    if e.read2()? != MEANING_OF_LIFE {
        return Err(err_invalid_format());
    }

    let ifd_offset = e.read4()?;

    if ifd_offset < 8 {
        return Err(err_invalid_format());
    }

    e.skip((ifd_offset - 8) as i64);

    let mut dec = MetaDecoderExif::new(e, 0);

    dec.decode_tags(ctx, "IFD0")
}

// ---------------------------------------------------------------------------
// imagemeta: metadecoder_exif.go

// exifType represents the basic tiff tag data types.
const EXIF_TYPE_UNSIGNED_BYTE1: u16 = 1;
const EXIF_TYPE_ASCII_STRING1: u16 = 2;
const EXIF_TYPE_UNSIGNED_SHORT2: u16 = 3;
const EXIF_TYPE_UNSIGNED_LONG4: u16 = 4;
const EXIF_TYPE_UNSIGNED_RAT8: u16 = 5;
const EXIF_TYPE_SIGNED_BYTE1: u16 = 6;
const EXIF_TYPE_UNDEF1: u16 = 7;
const EXIF_TYPE_SIGNED_SHORT2: u16 = 8;
const EXIF_TYPE_SIGNED_LONG4: u16 = 9;
const EXIF_TYPE_SIGNED_RAT8: u16 = 10;
const EXIF_TYPE_SIGNED_FLOAT4: u16 = 11;
const EXIF_TYPE_SIGNED_DOUBLE8: u16 = 12;

/// Used for +inf/-inf/nan. This is in line with Exiftool.
const UNDEF: &str = "undef";

/// Go: `exifTypeSize`.
fn exif_type_size(typ: u16) -> Option<u32> {
    Some(match typ {
        EXIF_TYPE_UNSIGNED_BYTE1
        | EXIF_TYPE_ASCII_STRING1
        | EXIF_TYPE_SIGNED_BYTE1
        | EXIF_TYPE_UNDEF1 => 1,
        EXIF_TYPE_UNSIGNED_SHORT2 | EXIF_TYPE_SIGNED_SHORT2 => 2,
        EXIF_TYPE_UNSIGNED_LONG4 | EXIF_TYPE_SIGNED_LONG4 | EXIF_TYPE_SIGNED_FLOAT4 => 4,
        EXIF_TYPE_UNSIGNED_RAT8 | EXIF_TYPE_SIGNED_RAT8 | EXIF_TYPE_SIGNED_DOUBLE8 => 8,
        _ => return None,
    })
}

/// Go: `exifIFDPointers`.
fn exif_ifd_pointer(tag_id: u16) -> Option<&'static str> {
    match tag_id {
        0x8769 => Some("ExifIFDP"),
        0x8825 => Some("GPSInfoIFD"),
        0xa005 => Some("InteroperabilityIFD"),
        _ => None,
    }
}

// Go: helpers.go:isUndefined
fn is_undefined(f: f64) -> bool {
    f.is_nan() || f.is_infinite()
}

/// Go: `metaDecoderEXIF`.
struct MetaDecoderExif<'s, 'a> {
    s: &'s mut StreamReader<'a>,
    thumbnail_offset: i64,
    seen_ifds: std::collections::HashSet<&'static str>,
}

/// Which reader a value is read from: the decoder's stream or a buffered value reader.
enum Src<'r> {
    Main,
    Buf(&'r mut BytesReader),
}

impl<'s, 'a> MetaDecoderExif<'s, 'a> {
    // Go: metadecoder_exif.go:newMetaDecoderEXIFFromStreamReader
    fn new(s: &'s mut StreamReader<'a>, thumbnail_offset: i64) -> Self {
        MetaDecoderExif {
            s,
            thumbnail_offset,
            seen_ifds: std::collections::HashSet::new(),
        }
    }

    fn with_src<T>(
        &mut self,
        src: &mut Src<'_>,
        f: impl FnOnce(&mut BufState, &mut dyn Read) -> R<T>,
    ) -> R<T> {
        match src {
            Src::Main => f(&mut self.s.st, &mut self.s.r),
            Src::Buf(b) => f(&mut self.s.st, &mut **b),
        }
    }

    // Go: metadecoder_exif.go:(*metaDecoderEXIF).convertValue
    fn convert_value(&mut self, typ: u16, src: &mut Src<'_>) -> R<TagValue> {
        let v = self.do_convert_value(typ, src)?;

        match v {
            TagValue::F64(f) if is_undefined(f) => Ok(TagValue::str(UNDEF)),
            TagValue::F32(f) if is_undefined(f as f64) => Ok(TagValue::str(UNDEF)),
            v => Ok(v),
        }
    }

    // Go: metadecoder_exif.go:(*metaDecoderEXIF).doConvertValue
    fn do_convert_value(&mut self, typ: u16, src: &mut Src<'_>) -> R<TagValue> {
        match typ {
            EXIF_TYPE_UNSIGNED_BYTE1
            | EXIF_TYPE_UNDEF1
            | EXIF_TYPE_ASCII_STRING1
            | EXIF_TYPE_SIGNED_BYTE1 => Ok(TagValue::U8(self.with_src(src, |st, r| st.read1r(r))?)),
            EXIF_TYPE_UNSIGNED_SHORT2 | EXIF_TYPE_SIGNED_SHORT2 => {
                Ok(TagValue::U16(self.with_src(src, |st, r| st.read2r(r))?))
            }
            EXIF_TYPE_UNSIGNED_LONG4 => {
                Ok(TagValue::U32(self.with_src(src, |st, r| st.read4r(r))?))
            }
            EXIF_TYPE_UNSIGNED_RAT8 => {
                let n = self.with_src(src, |st, r| st.read4r(r))?;
                let d = self.with_src(src, |st, r| st.read4r(r))?;
                if d == 0 {
                    return Ok(TagValue::str(UNDEF));
                }
                // (NewRat fails only for d == 0; "failed to convert rational" is unreachable.)
                Ok(new_rat_u32(n, d).unwrap_or(TagValue::Int(0)))
            }
            EXIF_TYPE_SIGNED_LONG4 => Ok(TagValue::I32(self.with_src(src, |st, r| st.read4sr(r))?)),
            EXIF_TYPE_SIGNED_RAT8 => {
                let n = self.with_src(src, |st, r| st.read4sr(r))?;
                let d = self.with_src(src, |st, r| st.read4sr(r))?;
                // "failed to convert signed rational" (a warning) -> 0.
                Ok(new_rat_i32(n, d).unwrap_or(TagValue::Int(0)))
            }
            EXIF_TYPE_SIGNED_FLOAT4 => Ok(TagValue::F32(f32::from_bits(
                self.with_src(src, |st, r| st.read4r(r))?,
            ))),
            EXIF_TYPE_SIGNED_DOUBLE8 => Ok(TagValue::F64(f64::from_bits(
                self.with_src(src, |st, r| st.read8r(r))?,
            ))),
            // Unreachable: decodeTag rejects unknown types first.
            _ => Err(invalid_format(format!("unknown EXIF type {typ}"))),
        }
    }

    // Go: metadecoder_exif.go:(*metaDecoderEXIF).convertValues
    fn convert_values(
        &mut self,
        typ: u16,
        count: usize,
        len: usize,
        src: &mut Src<'_>,
    ) -> R<TagValue> {
        if count == 0 {
            return Ok(TagValue::Nil);
        }

        if typ == EXIF_TYPE_ASCII_STRING1 {
            let b = self.with_src(src, |st, r| {
                st.read_bytes_from_r_volatile(len, r).map(|b| b.to_vec())
            })?;
            return Ok(TagValue::Str(trim_bytes_nulls(&b[..count]).to_vec()));
        }

        if count == 1 {
            return self.convert_value(typ, src);
        }

        let mut values = Vec::with_capacity(count);
        let mut all_bytes = true;
        for _ in 0..count {
            let v = self.convert_value(typ, src)?;
            if all_bytes && !matches!(v, TagValue::U8(_)) {
                all_bytes = false;
            }
            values.push(v);
        }

        if all_bytes {
            let bs = values
                .iter()
                .map(|v| match v {
                    TagValue::U8(b) => *b,
                    _ => 0,
                })
                .collect();
            return Ok(TagValue::Bytes(bs));
        }
        Ok(TagValue::Any(values))
    }

    // Go: metadecoder_exif.go:(*metaDecoderEXIF).decode
    fn decode(&mut self, ctx: &mut Ctx<'_>) -> R<()> {
        self.s.reader_offset = self.s.pos();
        let byte_order_tag = self.s.read2()?;

        match byte_order_tag {
            BYTE_ORDER_BIG_ENDIAN => self.s.st.byte_order = ByteOrder::Big,
            BYTE_ORDER_LITTLE_ENDIAN => self.s.st.byte_order = ByteOrder::Little,
            _ => return Ok(()),
        }

        self.s.skip(2);

        // Main image.
        let ifd0_offset = self.s.read4()?;

        if ifd0_offset < 8 {
            return Ok(());
        }

        self.s.skip((ifd0_offset - 8) as i64);

        self.decode_tags(ctx, "IFD0")?;

        // Thumbnail IFD.
        let ifd1_offset = self.s.read4()?;
        if ifd1_offset == 0 {
            // No more.
            return Ok(());
        }
        self.s.seek(ifd1_offset as i64 + self.s.reader_offset)?;

        self.decode_tags(ctx, "IFD1")
    }

    /// A tag is represented in 12 bytes: 2 bytes for the tag ID, 2 bytes for the data type,
    /// 4 bytes for the number of data values of the specified type, and 4 bytes for the value
    /// itself, if it fits, otherwise for a pointer to another location where the data may be
    /// found; this could be a pointer to the beginning of another IFD.
    // Go: metadecoder_exif.go:(*metaDecoderEXIF).decodeTag
    fn decode_tag(&mut self, ctx: &mut Ctx<'_>, namespace: &str) -> R<()> {
        let tag_id = self.s.read2()?;
        let data_type = self.s.read2()?;
        let count = self.s.read4()?;
        if count > 0x10000 {
            self.s.skip(4);
            return Ok(());
        }

        let mut tag_name = exif_field_name(tag_id).to_string();
        if tag_name.is_empty() {
            tag_name = format!("UnknownTag_0x{tag_id:x}");
        }

        if tag_name.contains(' ') {
            // Space separated, pick first.
            tag_name = tag_name.split(' ').next().unwrap_or("").to_string();
        }

        let ifd = exif_ifd_pointer(tag_id);
        if let Some(ifd) = ifd {
            if self.seen_ifds.contains(ifd) {
                return Ok(());
            }
            self.seen_ifds.insert(ifd);
        }
        let is_ifd_pointer = ifd.is_some();

        let typ = data_type;

        let Some(size) = exif_type_size(typ) else {
            return Err(invalid_format(format!("unknown EXIF type {typ}")));
        };
        let val_len = size * count;

        if tag_id == XMP_MARKER || tag_id == IPTC_MARKER {
            // XMP and IPTC are not requested.
            self.s.skip(4);
            return Ok(());
        }

        // Below is EXIF
        if val_len > ctx.limit_tag_size {
            self.s.skip(4);
            return Ok(());
        }

        let mut tag_info = TagInfo {
            source: SOURCE_EXIF,
            tag: tag_name.clone(),
            namespace: namespace.to_string(),
            value: TagValue::Nil,
        };

        if !is_ifd_pointer && !ctx.should_handle_tag(&tag_info)? {
            self.s.skip(4);
            return Ok(());
        }

        let mut val;
        if val_len > 4 {
            let value_offset = self.s.read4()?;
            let offset = value_offset.wrapping_add(self.s.reader_offset as u32);
            let old_pos = self.s.pos();
            let res = (|| -> R<TagValue> {
                self.s.seek(offset as i64)?;
                let mut rc = self.s.buffered_reader(val_len as i64)?;
                self.convert_values(
                    typ,
                    count as usize,
                    val_len as usize,
                    &mut Src::Buf(&mut rc),
                )
            })();
            // defer e.seek(oldPos)
            let seek_res = self.s.seek(old_pos);
            val = res?;
            seek_res?;
        } else {
            val = self.convert_values(typ, count as usize, val_len as usize, &mut Src::Main)?;
            let padding = 4 - val_len;
            if padding > 0 {
                self.s.skip(padding as i64);
            }
        }

        if let Some(ifd) = ifd {
            let TagValue::U32(offset) = val else {
                return Err(invalid_format("invalid IFD pointer value"));
            };
            let namespace = go_path::path::join(&[namespace, ifd]);
            return self.decode_tags_at(ctx, &namespace, offset as i64);
        }

        if let Some(convert) = exif_value_converter(&tag_name) {
            val = convert(self, val)?;
            if let TagValue::F64(f) = val
                && is_undefined(f)
            {
                val = TagValue::str(UNDEF);
            }
        } else {
            val = to_printable_value(val);
        }

        if val == TagValue::Nil {
            val = TagValue::str("");
        }

        if tag_name == TAG_NAME_THUMBNAIL_OFFSET {
            // When set, thumbnailOffset is set to the offset of the EXIF data in the original
            // file.
            let TagValue::U32(v) = val else {
                return Err(Fail::Panic(type_assertion_panic(&val, "uint32")));
            };
            val = TagValue::U32(
                v.wrapping_add((self.s.reader_offset.wrapping_add(self.thumbnail_offset)) as u32),
            );
        }

        tag_info.value = val;

        ctx.handle_tag(tag_info);

        Ok(())
    }

    // Go: metadecoder_exif.go:(*metaDecoderEXIF).decodeTags
    fn decode_tags(&mut self, ctx: &mut Ctx<'_>, namespace: &str) -> R<()> {
        let num_tags = self.s.read2()?;

        for _ in 0..num_tags {
            self.decode_tag(ctx, namespace)?;
        }

        Ok(())
    }

    // Go: metadecoder_exif.go:(*metaDecoderEXIF).decodeTagsAt
    fn decode_tags_at(&mut self, ctx: &mut Ctx<'_>, namespace: &str, offset: i64) -> R<()> {
        // preservePos
        let pos = self.s.pos();
        let res = (|| -> R<()> {
            self.s.seek(offset + self.s.reader_offset)?;
            self.decode_tags(ctx, namespace)
        })();
        let seek_res = self.s.seek(pos);
        res?;
        seek_res
    }
}

type Converter = fn(&mut MetaDecoderExif<'_, '_>, TagValue) -> R<TagValue>;

/// Go: `exifValueConverterMap`.
fn exif_value_converter(tag_name: &str) -> Option<Converter> {
    Some(match tag_name {
        "ApertureValue" | "MaxApertureValue" => |_, v| Ok(convert_apex_to_f_number(v)),
        "ShutterSpeedValue" => |_, v| Ok(convert_apex_to_seconds(v)),
        "GPSLatitude" | "GPSLongitude" => |_, v| Ok(convert_degrees_to_decimal(v)),
        "GPSMeasureMode"
        | "SubSecTimeDigitized"
        | "SubSecTimeOriginal"
        | "SubSecTime"
        | "GPSSatellites" => |_, v| Ok(convert_string_to_int(v)),
        "GPSTimeStamp" => |_, v| Ok(convert_to_timestamp_string(v)),
        "GPSVersionID" | "ComponentsConfiguration" => {
            |_, v| Ok(convert_bytes_to_string_space_delim(v))
        }
        "SubjectArea" | "BitsPerSample" | "PageNumber" | "StripByteCounts" | "StripOffsets" => {
            |_, v| Ok(convert_numbers_to_space_limited(v))
        }
        "PrimaryChromaticities"
        | "WhitePoint"
        | "ReferenceBlackWhite"
        | "YCbCrCoefficients"
        | "LensInfo" => |_, v| Ok(convert_rats_to_space_limited(v)),
        "Padding" => |_, v| Ok(convert_binary_data(v)),
        "UserComment" => |_, v| Ok(convert_user_comment(v)),
        "CFAPattern" => convert_cfa_pattern,
        _ => return None,
    })
}

// ---------------------------------------------------------------------------
// imagemeta: helpers.go value converters (warnings are not modelled)

// Go: helpers.go:(vc).convertAPEXToFNumber
fn convert_apex_to_f_number(v: TagValue) -> TagValue {
    let Some(f) = v.float64_provider() else {
        return TagValue::Int(0);
    };
    TagValue::F64(gift::gomath::pow(2.0, f / 2.0))
}

// Go: helpers.go:(vc).convertAPEXToSeconds
fn convert_apex_to_seconds(v: TagValue) -> TagValue {
    let Some(f) = v.float64_provider() else {
        return TagValue::Int(0);
    };
    TagValue::F64(1.0 / gift::gomath::pow(2.0, f))
}

/// Go: `typeAssertSlice[byte]`.
fn type_assert_slice_bytes(v: &TagValue) -> Option<Vec<u8>> {
    match v {
        TagValue::Bytes(b) => Some(b.clone()),
        TagValue::U8(b) => Some(vec![*b]),
        _ => None,
    }
}

// Go: helpers.go:(vc).convertBytesToStringDelimBy
fn convert_bytes_to_string_delim_by(v: &TagValue, delim: &str) -> TagValue {
    let Some(bb) = type_assert_slice_bytes(v) else {
        return TagValue::str("");
    };
    let parts: Vec<String> = bb.iter().map(|b| b.to_string()).collect();
    TagValue::Str(parts.join(delim).into_bytes())
}

// Go: helpers.go:(vc).convertBytesToStringSpaceDelim
fn convert_bytes_to_string_space_delim(v: TagValue) -> TagValue {
    convert_bytes_to_string_delim_by(&v, " ")
}

// Go: helpers.go:(vc).convertDegreesToDecimal
fn convert_degrees_to_decimal(v: TagValue) -> TagValue {
    match to_degrees(&v) {
        Ok(d) => TagValue::F64(d),
        // "failed to convert degrees to decimal" (a warning).
        Err(_) => TagValue::F64(0.0),
    }
}

/// Go `fmt.Sprintf("%d", n)` of one tag value (a rat's `Format` prints `String()`).
fn sprint_d(n: &TagValue) -> String {
    match n {
        TagValue::RatU(a, b) => Rat::Uint32(*a, *b).string(),
        TagValue::RatI(a, b) => Rat::Int32(*a, *b).string(),
        other => String::from_utf8_lossy(&go_fmt::sprintf("%d", &[other.to_value()])).into_owned(),
    }
}

// Go: helpers.go:(vc).convertNumbersToSpaceLimited
fn convert_numbers_to_space_limited(v: TagValue) -> TagValue {
    // typeAssertSlice[any]: a []any, or any single non-nil value.
    let nums = match v {
        TagValue::Any(items) => items,
        TagValue::Nil => return TagValue::str(""),
        other => vec![other],
    };

    let parts: Vec<String> = nums.iter().map(sprint_d).collect();
    TagValue::Str(parts.join(" ").into_bytes())
}

// Go: helpers.go:(vc).convertBinaryData
fn convert_binary_data(v: TagValue) -> TagValue {
    match v {
        TagValue::Bytes(b) => {
            TagValue::Str(format!("(Binary data {} bytes)", b.len()).into_bytes())
        }
        _ => TagValue::str(""),
    }
}

// Go: helpers.go:(vc).convertRatsToSpaceLimited
fn convert_rats_to_space_limited(v: TagValue) -> TagValue {
    let TagValue::Any(nums) = v else {
        return TagValue::str("");
    };

    let mut sb = Vec::new();
    for (i, n) in nums.iter().enumerate() {
        if i > 0 {
            sb.push(b' ');
        }
        let mut s: Vec<u8> = Vec::new();
        let mut f = 0.0;
        match n {
            TagValue::Str(x) => s = x.clone(),
            TagValue::F64(x) => f = *x,
            other => {
                if let Some(x) = other.float64_provider() {
                    f = x;
                }
            }
        }

        if s.is_empty() {
            if is_undefined(f) {
                s = UNDEF.as_bytes().to_vec();
            } else {
                s = go_strconv::format_float(f, b'f', -1, 64).into_bytes();
            }
        }

        sb.extend_from_slice(&s);
    }
    TagValue::Str(sb)
}

// Go: helpers.go:(vc).convertStringToInt
fn convert_string_to_int(v: TagValue) -> TagValue {
    let TagValue::Str(s) = v else {
        return TagValue::Int(0);
    };
    let s = printable_string(&s);
    let i = go_strconv::atoi(&s).unwrap_or(0);
    TagValue::Int(i)
}

// Go: helpers.go:(vc).convertUserComment
fn convert_user_comment(v: TagValue) -> TagValue {
    // UserComment tag is identified based on an ID code in a fixed 8-byte area at the start of
    // the tag data area.
    let b = match v {
        TagValue::Bytes(b) => b,
        // Handle plain string user comment (which is against spec; but commonly done).
        TagValue::Str(s) => return TagValue::Str(s),
        _ => return TagValue::str(""),
    };
    if b.len() < 8 {
        return TagValue::str("");
    }
    let id = &b[..8];

    match id {
        b"ASCII\x00\x00\x00" => {
            let s = printable_string(trim_bytes_nulls(&b[8..]));
            if !s.is_ascii() {
                return TagValue::str("");
            }
            TagValue::Str(s)
        }
        b"UNICODE\x00" => TagValue::Str(printable_string(trim_bytes_nulls(&b[8..]))),
        b"\x00\x00\x00\x00\x00\x00\x00\x00" => {
            let s = trim_bytes_nulls(&b[8..]);
            if !go_unicode::utf8::valid(s) {
                return TagValue::str("");
            }
            TagValue::Str(go_unicode::strings::trim_right(s, b" ").to_vec())
        }
        _ => TagValue::str(""),
    }
}

// Go: helpers.go:(vc).ratNum
fn rat_num(v: &TagValue) -> TagValue {
    match v {
        TagValue::RatU(n, _) => TagValue::U32(*n),
        TagValue::RatI(n, _) => TagValue::I32(*n),
        _ => TagValue::Int(0),
    }
}

// Go: helpers.go:(vc).convertToTimestampString
fn convert_to_timestamp_string(v: TagValue) -> TagValue {
    match v {
        TagValue::Any(vv) => {
            if vv.len() != 3 {
                return TagValue::ZeroTime;
            }
            let vals: Vec<Value> = vv.iter().map(|v| rat_num(v).to_value()).collect();
            let mut s = go_fmt::sprintf("%02d:%02d:%02d", &vals);

            if s.len() == 10 {
                // 13:03:4279 => 13:03:42.79
                let mut t = s[..8].to_vec();
                t.push(b'.');
                t.extend_from_slice(&s[8..]);
                s = t;
            }
            TagValue::Str(s)
        }
        TagValue::Str(vv) => {
            // 17,00000,8,00000,29,0000
            let parts = go_unicode::strings::split(&vv, b",");

            if parts.len() != 6 {
                return TagValue::str("");
            }
            let mut vvv = Vec::new();
            for i in (0..6).step_by(2) {
                let v = go_strconv::atoi(parts[i]).unwrap_or(0);
                vvv.push(Value::int(v));
            }
            TagValue::Str(go_fmt::sprintf("%02d:%02d:%02d", &vvv))
        }
        _ => TagValue::str(""),
    }
}

/// The `CFAPattern` converter (a Go closure in `exifValueConverterMap`).
fn convert_cfa_pattern(e: &mut MetaDecoderExif<'_, '_>, v: TagValue) -> R<TagValue> {
    let b = match v {
        TagValue::Bytes(b) => b,
        other => return Err(Fail::Panic(type_assertion_panic(&other, "[]uint8"))),
    };
    let order = e.s.st.byte_order;
    if b.len() < 2 {
        return Err(Fail::Panic(format!(
            "runtime error: slice bounds out of range [:2] with capacity {}",
            b.len()
        )));
    }
    let mut horizontal_repeat = order.checked_uint16(&b[..2])?;
    let mut vertical_repeat = order.checked_uint16(&b[2..])?;
    let repeat_len = horizontal_repeat as i64 * vertical_repeat as i64;
    let mut hi = 4 + repeat_len;
    if hi > b.len() as i64 {
        // See issue 34.
        // There are cameras that writes CFAPattern with a byte order that's not the one
        // specified in the EXIF header.
        let order = e.s.other_byte_order();
        horizontal_repeat = order.uint16(&b[..2]);
        vertical_repeat = order.uint16(&b[2..]);
        let repeat_len = horizontal_repeat as i64 * vertical_repeat as i64;
        hi = 4 + repeat_len;
        if hi > b.len() as i64 {
            // Just return the raw bytes.
            return Ok(TagValue::Bytes(trim_bytes_nulls(&b).to_vec()));
        }
    }

    let val = TagValue::Bytes(b[4..hi as usize].to_vec());
    let TagValue::Str(delim) = convert_bytes_to_string_space_delim(val) else {
        return Ok(TagValue::str(""));
    };
    let mut out = format!("{horizontal_repeat} {vertical_repeat} ").into_bytes();
    out.extend_from_slice(&delim);
    Ok(TagValue::Str(out))
}

// Go: helpers.go:(vc).parseDegrees
fn parse_degrees(s: &[u8]) -> std::result::Result<f64, String> {
    if s.is_empty() || s == b"0100" {
        return Ok(0.0);
    }
    let (deg, min, sec) = sscanf_3_floats(s)?;
    Ok(deg + min / 60.0 + sec / 3600.0)
}

// Go: helpers.go:(vc).toDegrees
fn to_degrees(v: &TagValue) -> std::result::Result<f64, String> {
    match v {
        TagValue::Any(v) => {
            if v.len() != 3 {
                return Err(format!("expected 3 values, got {}", v.len()));
            }

            let deg = to_float64(&v[0]);
            let min = to_float64(&v[1]);
            let sec = to_float64(&v[2]);

            Ok(deg + min / 60.0 + sec / 3600.0)
        }
        TagValue::F64(v) => Ok(*v),
        TagValue::Str(s) => parse_degrees(s),
        TagValue::Bytes(b) => parse_degrees(b),
        other => Err(format!("unsupported degree type {}", other.go_type())),
    }
}

// Go: helpers.go:toFloat64
fn to_float64(v: &TagValue) -> f64 {
    match v {
        TagValue::F64(f) => *f,
        other => other.float64_provider().unwrap_or(0.0),
    }
}

// Go: helpers.go:printableString
fn printable_string(s: &[u8]) -> Vec<u8> {
    let ss = go_unicode::strings::map(
        |r| {
            if go_unicode::is_graphic(r) { r } else { -1 }
        },
        s,
    );
    go_unicode::strings::trim_space(&ss).to_vec()
}

// Go: helpers.go:toPrintableValue
fn to_printable_value(v: TagValue) -> TagValue {
    match v {
        TagValue::Str(s) => TagValue::Str(printable_string(&s)),
        TagValue::Bytes(b) => TagValue::Str(printable_string(trim_bytes_nulls(&b))),
        other => other,
    }
}

// Go: helpers.go:trimBytesNulls
fn trim_bytes_nulls(b: &[u8]) -> &[u8] {
    let mut lo = 0;
    while lo < b.len() && b[lo] == 0 {
        lo += 1;
    }
    let mut hi = b.len() as isize - 1;
    while hi >= 0 && b[hi as usize] == 0 {
        hi -= 1;
    }
    if lo as isize > hi {
        return &[];
    }
    &b[lo..=hi as usize]
}

/// Go `fmt.Sscanf(s, "%f,%f,%f", &deg, &min, &sec)` (fmt/scan.go: `SkipSpace`, `floatToken`,
/// `convertFloat`, and the literal `,` of the format).
fn sscanf_3_floats(s: &[u8]) -> std::result::Result<(f64, f64, f64), String> {
    let mut sc = Scanner {
        runes: {
            let mut v = Vec::new();
            let mut i = 0;
            while i < s.len() {
                let (r, n) = go_unicode::utf8::decode_rune(&s[i..]);
                v.push(r);
                i += n;
            }
            v
        },
        pos: 0,
        buf: Vec::new(),
    };
    let a = sc.scan_float()?;
    sc.expect_literal(',')?;
    let b = sc.scan_float()?;
    sc.expect_literal(',')?;
    let c = sc.scan_float()?;
    Ok((a, b, c))
}

/// A minimal port of fmt's `ss` for `Sscanf` with `%f` verbs (no newlines allowed).
struct Scanner {
    runes: Vec<i32>,
    pos: usize,
    buf: Vec<u8>,
}

impl Scanner {
    fn peek(&self) -> Option<i32> {
        self.runes.get(self.pos).copied()
    }

    // Go: fmt/scan.go:(*ss).accept (consume)
    fn accept(&mut self, ok: &str) -> bool {
        let Some(r) = self.peek() else {
            return false;
        };
        if ok.chars().any(|c| c as i32 == r) {
            self.pos += 1;
            go_unicode::utf8::append_rune(&mut self.buf, r);
            return true;
        }
        false
    }

    // Go: fmt/scan.go:(*ss).SkipSpace (nlIsSpace = false for Sscanf)
    fn skip_space(&mut self) -> std::result::Result<(), String> {
        while let Some(r) = self.peek() {
            if r == '\r' as i32 && self.runes.get(self.pos + 1) == Some(&('\n' as i32)) {
                self.pos += 1;
                continue;
            }
            if r == '\n' as i32 {
                return Err("unexpected newline".into());
            }
            if !is_scan_space(r) {
                break;
            }
            self.pos += 1;
        }
        Ok(())
    }

    // Go: fmt/scan.go:(*ss).floatToken
    fn float_token(&mut self) -> Vec<u8> {
        self.buf.clear();
        // NaN?
        if self.accept("nN") && self.accept("aA") && self.accept("nN") {
            return self.buf.clone();
        }
        // leading sign?
        self.accept("+-");
        // Inf?
        if self.accept("iI") && self.accept("nN") && self.accept("fF") {
            return self.buf.clone();
        }
        let mut digits = "0123456789_";
        let mut exp = "eEpP";
        if self.accept("0") && self.accept("xX") {
            digits = "0123456789aAbBcCdDeEfF_";
            exp = "pP";
        }
        // digits?
        while self.accept(digits) {}
        // decimal point?
        if self.accept(".") {
            // fraction?
            while self.accept(digits) {}
        }
        // exponent?
        if self.accept(exp) {
            // leading sign?
            self.accept("+-");
            // digits?
            while self.accept("0123456789_") {}
        }
        self.buf.clone()
    }

    // Go: fmt/scan.go:(*ss).convertFloat (n = 64)
    fn convert_float(&self, s: &[u8]) -> std::result::Result<f64, String> {
        if let Some(p) = s.iter().position(|&c| c == b'p') {
            // Atof understands base-2 exponents only for hex mantissas; handle it ourselves.
            let f = go_strconv::parse_float(&s[..p], 64).map_err(|e| e.to_string())?;
            let m = go_strconv::atoi(&s[p + 1..]).map_err(|e| e.to_string())?;
            return Ok(gift::gomath::ldexp(f, m));
        }
        go_strconv::parse_float(s, 64).map_err(|e| e.to_string())
    }

    // Go: fmt/scan.go:(*ss).scanOne, case *float64
    fn scan_float(&mut self) -> std::result::Result<f64, String> {
        self.skip_space()?;
        // notEOF
        if self.peek().is_none() {
            return Err("unexpected EOF".into());
        }
        let tok = self.float_token();
        self.convert_float(&tok)
    }

    /// A literal (non-space) format rune must match the next input rune.
    fn expect_literal(&mut self, c: char) -> std::result::Result<(), String> {
        match self.peek() {
            None => Err("unexpected EOF".into()),
            Some(r) if r == c as i32 => {
                self.pos += 1;
                Ok(())
            }
            Some(_) => Err("input does not match format".into()),
        }
    }
}

/// fmt/scan.go:isSpace (the `space` table).
fn is_scan_space(r: i32) -> bool {
    if r >= 1 << 16 {
        return false;
    }
    const SPACE: [(i32, i32); 11] = [
        (0x0009, 0x000d),
        (0x0020, 0x0020),
        (0x0085, 0x0085),
        (0x00a0, 0x00a0),
        (0x1680, 0x1680),
        (0x2000, 0x200a),
        (0x2028, 0x2029),
        (0x202f, 0x202f),
        (0x205f, 0x205f),
        (0x3000, 0x3000),
        (0xffff, 0xffff),
    ];
    for (lo, hi) in SPACE {
        if r < lo {
            return false;
        }
        if r <= hi {
            return r != 0xffff;
        }
    }
    false
}

/// Shared `Arc` form of an [`ExifInfo`].
pub type SharedExifInfo = Arc<ExifInfo>;

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/exif/exif.go (366 lines; 8/13 funcs executed)
//   types: ExifInfo, Decoder, Tags
// OK L52-54: (d *Decoder) shouldInclude(s string) bool
// OK L56-58: (d *Decoder) shouldExclude(s string) bool
// OK L60-69: IncludeFields(expression string) func(*Decoder) error
// OK L71-80: ExcludeFields(expression string) func(*Decoder) error
// OK L82-87: WithLatLongDisabled(disabled bool) func(*Decoder) error
// OK L89-94: WithDateDisabled(disabled bool) func(*Decoder) error
// OK L96-101: WithWarnLogger(warnl logg.LevelLogger) func(*Decoder) error   (not modelled)
// OK L103-114: compileRegexp(expression string) (*regexp.Regexp, error)
// OK L116-125: NewDecoder(options ...func(*Decoder) error) (*Decoder, error)
// OK L137-226: (d *Decoder) Decode(filename string, format imagemeta.ImageFormat, r io.Reader) (ex *ExifInfo, err error)
// OK L230-346: init()   (the tmc codec: JSON of Tags, see Tags below)
// STUB L352-361: (v *Tags) UnmarshalJSON(b []byte) error   (the warm-cache read path only)
// STUB L364-366: (v Tags) MarshalJSON() ([]byte, error)    (the file cache write, not output)
// ---------------------------------------------------------------------------
