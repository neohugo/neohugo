//! Port of go1.27.1 `compress/zlib`.
//!
//! Package zlib implements reading and writing of zlib format compressed data,
//! as specified in RFC 1950.

mod reader;
mod writer;

pub use reader::{Reader, new_reader, new_reader_dict};
pub use writer::{Writer, new_writer, new_writer_level, new_writer_level_dict};

use crate::flate;

// These constants are copied from the flate package, so that code that imports
// compress/zlib does not also have to import compress/flate.
pub const NO_COMPRESSION: i32 = flate::NO_COMPRESSION;
pub const BEST_SPEED: i32 = flate::BEST_SPEED;
pub const BEST_COMPRESSION: i32 = flate::BEST_COMPRESSION;
pub const DEFAULT_COMPRESSION: i32 = flate::DEFAULT_COMPRESSION;
pub const HUFFMAN_ONLY: i32 = flate::HUFFMAN_ONLY;
