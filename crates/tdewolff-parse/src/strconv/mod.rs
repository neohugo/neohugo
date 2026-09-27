//! Go: github.com/tdewolff/parse/v2/strconv — fast (not correctly rounded)
//! number parsing and formatting.

pub mod decimal;
pub mod float;
pub mod int;
pub mod number;

pub use decimal::{append_decimal, parse_decimal};
pub use float::{append_float, parse_float};
pub use int::{append_int, len_int, len_uint, parse_int, parse_uint};
pub use number::{append_number, parse_number};
