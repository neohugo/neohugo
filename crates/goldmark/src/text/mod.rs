//! Go: github.com/yuin/goldmark@v1.7.12/text — text reading utilities.

pub mod reader;
pub mod segment;

pub use reader::{
    BlockReader, EOF, FindClosureOptions, Reader, RuneStream, TextReader, new_block_reader,
    new_reader,
};
pub use segment::{Segment, Segments, new_segment, new_segment_padding, new_segments};
