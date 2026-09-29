//! Module `metadecoders::toml`.
//!
//! NEW: a port of `pelletier/go-toml/v2@v2.2.4` `Unmarshal` into `any` / `map[string]any`
//! (the decoder neohugo's `metadecoders` uses for TOML).
//!
//! Owner: Wave B task T03 (parser-langs).
//!
//! The go-toml parser (`unstable/parser.go`, `scanner.go`, `internal/characters`), the scalar
//! decoders (`decode.go`), the seen-key tracker (`internal/tracker/seen.go`), the error
//! wrapping (`errors.go`), the local date/time types (`localtime.go`) and the `any`-target half
//! of `unmarshaler.go` are ported, so values, Go types and error texts match Go (see
//! PORTING.md for why the `toml` crate was not used).
//!
//! Type mapping (Go `Unmarshal` into `interface{}`): tables -> `map[string]interface {}`,
//! arrays -> `[]interface {}`, integers -> `int64`, floats -> `float64`, offset date-times ->
//! `time.Time` (UTC for `Z` and `+00:00`, else `time.FixedZone("", offset)`), local
//! date-time/date/time -> [`TomlLocal`] objects (Go `toml.LocalDateTime/LocalDate/LocalTime`,
//! which implement `AsTime(loc)` and `String()`).

mod characters;
mod decode;
mod errors;
pub mod marshaler;
mod parser;
mod tracker;
mod unmarshaler;

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, Kind, Location, Map, Object, Time, Value};

pub use errors::{DecodeError, Error};

/// Go: `toml.LocalDate` — a calendar day in no specific timezone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalDate {
    pub year: i64,
    pub month: i64,
    pub day: i64,
}

/// Go: `toml.LocalTime` — a time of day of no specific day in no specific timezone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalTime {
    /// Hour of the day: [0; 24[
    pub hour: i64,
    /// Minute of the hour: [0; 60[
    pub minute: i64,
    /// Second of the minute: [0; 60[
    pub second: i64,
    /// Nanoseconds within the second:  [0, 1000000000[
    pub nanosecond: i64,
    /// Number of digits to display for Nanosecond.
    pub precision: i64,
}

/// Go: `toml.LocalDateTime` — a time of a specific day in no specific timezone (embeds
/// `LocalDate` and `LocalTime`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalDateTime {
    pub date: LocalDate,
    pub time: LocalTime,
}

impl LocalDate {
    /// AsTime converts d into a specific time instance at midnight in zone.
    // Go: localtime.go:(LocalDate).AsTime
    pub fn as_time(&self, zone: &Arc<Location>) -> Time {
        go_time::date(
            self.year,
            go_time::Month(self.month),
            self.day,
            0,
            0,
            0,
            0,
            zone,
        )
    }

    /// String returns RFC 3339 representation of d.
    // Go: localtime.go:(LocalDate).String
    pub fn string(&self) -> String {
        format!(
            "{}-{}-{}",
            pad_int(self.year, 4),
            pad_int(self.month, 2),
            pad_int(self.day, 2)
        )
    }
}

impl LocalTime {
    /// String returns RFC 3339 representation of d.
    /// If d.Nanosecond and d.Precision are zero, the time won't have a nanosecond
    /// component. If d.Nanosecond > 0 but d.Precision = 0, then the minimum number
    /// of digits for nanoseconds is provided.
    // Go: localtime.go:(LocalTime).String
    pub fn string(&self) -> String {
        let mut s = format!(
            "{}:{}:{}",
            pad_int(self.hour, 2),
            pad_int(self.minute, 2),
            pad_int(self.second, 2)
        );

        if self.precision > 0 {
            let frac = format!(".{}", pad_int(self.nanosecond, 9));
            s.push_str(&frac[..(self.precision + 1) as usize]);
        } else if self.nanosecond > 0 {
            // Nanoseconds are specified, but precision is not provided. Use the
            // minimum.
            let frac = format!(".{}", pad_int(self.nanosecond, 9));
            s.push_str(frac.trim_matches('0'));
        }

        s
    }
}

impl LocalDateTime {
    /// AsTime converts d into a specific time instance in zone.
    // Go: localtime.go:(LocalDateTime).AsTime
    pub fn as_time(&self, zone: &Arc<Location>) -> Time {
        go_time::date(
            self.date.year,
            go_time::Month(self.date.month),
            self.date.day,
            self.time.hour,
            self.time.minute,
            self.time.second,
            self.time.nanosecond,
            zone,
        )
    }

    /// String returns RFC 3339 representation of d.
    // Go: localtime.go:(LocalDateTime).String
    pub fn string(&self) -> String {
        format!("{}T{}", self.date.string(), self.time.string())
    }
}

/// Go: `fmt.Sprintf("%0Nd", i)`.
fn pad_int(i: i64, width: usize) -> String {
    if i < 0 {
        format!("-{:0>w$}", i.unsigned_abs(), w = width.saturating_sub(1))
    } else {
        format!("{i:0>width$}")
    }
}

/// Go: `toml.LocalDate` / `LocalTime` / `LocalDateTime` values as template values (they have
/// `AsTime(loc)` (not `LocalTime`), `String()` and `MarshalText()`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TomlLocal {
    Date(LocalDate),
    Time(LocalTime),
    DateTime(LocalDateTime),
}

impl TomlLocal {
    /// Go: `String()`.
    pub fn string(&self) -> String {
        match self {
            TomlLocal::Date(d) => d.string(),
            TomlLocal::Time(t) => t.string(),
            TomlLocal::DateTime(dt) => dt.string(),
        }
    }

    /// Go: `AsTime(zone)` (`LocalTime` has no such method).
    pub fn as_time(&self, zone: &Arc<Location>) -> Option<Time> {
        match self {
            TomlLocal::Date(d) => Some(d.as_time(zone)),
            TomlLocal::Time(_) => None,
            TomlLocal::DateTime(dt) => Some(dt.as_time(zone)),
        }
    }

    fn date_fields(d: &LocalDate) -> Vec<(Cow<'static, str>, Value)> {
        vec![
            (Cow::Borrowed("Year"), Value::int(d.year)),
            (Cow::Borrowed("Month"), Value::int(d.month)),
            (Cow::Borrowed("Day"), Value::int(d.day)),
        ]
    }

    fn time_fields(t: &LocalTime) -> Vec<(Cow<'static, str>, Value)> {
        vec![
            (Cow::Borrowed("Hour"), Value::int(t.hour)),
            (Cow::Borrowed("Minute"), Value::int(t.minute)),
            (Cow::Borrowed("Second"), Value::int(t.second)),
            (Cow::Borrowed("Nanosecond"), Value::int(t.nanosecond)),
            (Cow::Borrowed("Precision"), Value::int(t.precision)),
        ]
    }
}

impl Object for TomlLocal {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(match self {
            TomlLocal::Date(_) => "toml.LocalDate",
            TomlLocal::Time(_) => "toml.LocalTime",
            TomlLocal::DateTime(_) => "toml.LocalDateTime",
        })
    }

    fn kind(&self) -> Kind {
        Kind::Struct
    }

    fn has_method(&self, name: &str) -> bool {
        match name {
            "String" | "MarshalText" => true,
            "AsTime" => !matches!(self, TomlLocal::Time(_)),
            _ => false,
        }
    }

    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "String" => Some(
                nh_common::object::args::exactly(args, 0, "String")
                    .map(|_| Value::string(self.string())),
            ),
            "MarshalText" => Some(
                nh_common::object::args::exactly(args, 0, "MarshalText").map(|_| {
                    Value::list(
                        go_value::SliceType::Uint8,
                        self.string()
                            .bytes()
                            .map(|b| Value::Uint(b as u64, go_value::UintKind::Uint8))
                            .collect(),
                    )
                }),
            ),
            "AsTime" if self.has_method("AsTime") => Some((|| {
                nh_common::object::args::exactly(args, 1, "AsTime")?;
                let loc = nh_common::htime::location_arg(args, 0)?;
                Ok(Value::Time(self.as_time(&loc).expect("AsTime")))
            })()),
            _ => None,
        }
    }

    fn field(&self, name: &str) -> Option<Value> {
        match self {
            TomlLocal::Date(d) => Self::date_fields(d)
                .into_iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v),
            TomlLocal::Time(t) => Self::time_fields(t)
                .into_iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v),
            TomlLocal::DateTime(dt) => match name {
                "LocalDate" => Some(Value::object(TomlLocal::Date(dt.date))),
                "LocalTime" => Some(Value::object(TomlLocal::Time(dt.time))),
                _ => Self::date_fields(&dt.date)
                    .into_iter()
                    .chain(Self::time_fields(&dt.time))
                    .find(|(n, _)| n == name)
                    .map(|(_, v)| v),
            },
        }
    }

    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.string()))
    }

    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(match self {
            TomlLocal::Date(d) => Self::date_fields(d),
            TomlLocal::Time(t) => Self::time_fields(t),
            TomlLocal::DateTime(dt) => vec![
                (
                    Cow::Borrowed("LocalDate"),
                    Value::object(TomlLocal::Date(dt.date)),
                ),
                (
                    Cow::Borrowed("LocalTime"),
                    Value::object(TomlLocal::Time(dt.time)),
                ),
            ],
        })
    }

    fn marshal_text(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(Ok(self.string().into_bytes()))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Decodes a TOML document into a Go-typed value tree: Go's `toml.Unmarshal(data, &v)` with
/// `var v any` (a `map[string]interface {}`).
// Go: unmarshaler.go:Unmarshal
pub fn unmarshal(data: &[u8]) -> Result<Value, Error> {
    unmarshal_to_map(data).map(Value::map)
}

/// Go's `toml.Unmarshal(data, &m)` with `m := make(map[string]any)`.
// Go: unmarshaler.go:Unmarshal
pub fn unmarshal_to_map(data: &[u8]) -> Result<Map, Error> {
    unmarshaler::unmarshal(data).map(unmarshaler::tv_map_into_map)
}

/// Go: `fmt.Sprintf("%c", b)` of a byte (the rune `U+00XX`).
pub(crate) fn fmt_byte_c(b: u8) -> Vec<u8> {
    let mut out = Vec::new();
    go_unicode::utf8::append_rune(&mut out, b as i32);
    out
}

/// Go: `fmt.Sprintf("%#U", b)` of a byte: `U+00XX 'c'` (the character only when printable).
pub(crate) fn fmt_byte_sharp_u(b: u8) -> Vec<u8> {
    let mut out = format!("U+{:04X}", b).into_bytes();
    if go_strconv::is_print(b as i32) {
        out.extend_from_slice(b" '");
        go_unicode::utf8::append_rune(&mut out, b as i32);
        out.push(b'\'');
    }
    out
}
