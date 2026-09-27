//! Module `metadecoders::toml`.
//!
//! NEW: pelletier/go-toml/v2@v2.2.4 Unmarshal-into-any semantics on top of the toml crate
//!
//! Owner: Wave B task T03 (parser-langs).


//! `pelletier/go-toml/v2@v2.2.4` `Unmarshal(data, &any)` semantics on top of the `toml` crate
//! (parser only): tables -> `map[string]interface {}`, arrays -> `[]interface {}`, integers ->
//! `int64`, floats -> `float64`, offset date-times -> `time.Time` (with a fixed zone for non-Z
//! offsets, UTC for Z), local date-time/date/time -> [`TomlLocal`] objects (Go
//! `toml.LocalDateTime/LocalDate/LocalTime`, which implement `AsTime(loc)`).
//! Verify integer overflow, `-0.0`, `inf/nan`, and datetime precision against a Go oracle.

use go_value::Value;

use nh_common::Result;

/// Go: `toml.LocalDate` / `LocalTime` / `LocalDateTime` (have `AsTime(loc)` and `String()`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TomlLocal {
    Date { year: i32, month: u32, day: u32 },
    Time { hour: u32, minute: u32, second: u32, nanosecond: u32, precision: u32 },
    DateTime { year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32, nanosecond: u32, precision: u32 },
}

/// Decodes a TOML document into a Go-typed value tree.
pub fn unmarshal(data: &[u8]) -> Result<Value> {
    todo!()
}
