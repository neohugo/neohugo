//! Go: github.com/tdewolff/parse/v2/buffer — buffer and wrapper types for
//! byte slices.

pub mod lexer;
pub mod reader;
pub mod streamlexer;
pub mod writer;

pub use lexer::Lexer;
pub use reader::Reader;
pub use streamlexer::StreamLexer;
pub use writer::Writer;

/// Go: buffer/buffer.go:defaultBufSize — the default initial length of
/// internal buffers.
pub const DEFAULT_BUF_SIZE: usize = 4096;

/// Go: buffer/buffer.go:MinBuf
pub const MIN_BUF: usize = DEFAULT_BUF_SIZE;
