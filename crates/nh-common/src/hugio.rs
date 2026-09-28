//! Port of `common/hugio/copy.go`, `common/hugio/hasBytesWriter.go`, `common/hugio/readers.go`, `common/hugio/writers.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

//! Go `common/hugio`: reader/writer helpers. `ReadSeekCloser` providers are how resources open
//! their source bytes lazily.

use std::io::{Read, Seek, Write};
use std::sync::Arc;

use crate::herrors::Result;

/// Go: `hugio.ReadSeekCloser`.
pub trait ReadSeekCloser: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeekCloser for T {}

/// Go: `hugio.OpenReadSeekCloser` — opens the source anew each call.
pub type OpenReadSeekCloser = Arc<dyn Fn() -> Result<Box<dyn ReadSeekCloser>> + Send + Sync>;

/// Go: `hugio.NewReadSeekerNoOpCloserFromString` / `FromBytes`.
pub fn read_seeker_from_bytes(b: Vec<u8>) -> Box<dyn ReadSeekCloser> {
    Box::new(std::io::Cursor::new(b))
}

/// Go: `hugio.ReadString(r)`.
pub fn read_all(r: &mut dyn Read) -> Result<Vec<u8>> {
    let mut v = Vec::new();
    r.read_to_end(&mut v)?;
    Ok(v)
}

/// Go: `hugio.HasBytesWriter` — records which of `patterns` occur in the written stream (handles
/// matches spanning writes). Used by the publish fs to find files with `__h_pp_l1` placeholders.
pub struct HasBytesWriter {
    pub patterns: Vec<HasBytesPattern>,
    pub(crate) i: usize,
    pub(crate) done: bool,
    pub(crate) buff: Vec<u8>,
}

/// Go: `hugio.HasBytesPattern`.
#[derive(Clone, Debug)]
pub struct HasBytesPattern {
    pub pattern: Vec<u8>,
    pub matched: bool,
}

impl HasBytesWriter {
    // Go: common/hugio/hasBytesWriter.go:Write
    pub fn write(&mut self, p: &[u8]) -> usize {
        todo!()
    }
}

/// Go: `hugio.ToWriteCloser` etc. are not needed; writers are `Box<dyn Write + Send>`.
pub type BoxWriter = Box<dyn Write + Send>;

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hugio/copy.go (92 lines; 0/2 funcs executed)
//    L26-50: CopyFile(fs afero.Fs, from, to string) error
//    L53-92: CopyDir(fs afero.Fs, from, to string, shouldCopy func(filename string) bool) error
// Source: common/hugio/hasBytesWriter.go (80 lines; 2/2 funcs executed)
//   types: HasBytesWriter, HasBytesPattern
// EX L34-40: (h *HasBytesWriter) patternLen() int
// EX L42-80: (h *HasBytesWriter) Write(p []byte) (n int, err error)
// Source: common/hugio/readers.go (106 lines; 4/7 funcs executed)
//   types: ReadSeeker, ReadSeekCloser, ReadSeekCloserProvider, readSeekerNopCloser, stringReadSeeker, StringReader,
//          OpenReadSeekCloser
// EX L46-48: (r readSeekerNopCloser) Close() error
// EX L51-53: NewReadSeekerNoOpCloser(r ReadSeeker) ReadSeekCloser
// EX L57-59: NewReadSeekerNoOpCloserFromString(content string) ReadSeekCloser
//    L68-70: (s *stringReadSeeker) ReadString() string
//    L79-81: NewReadSeekerNoOpCloserFromBytes(content []byte) readSeekerNopCloser
//    L85-90: NewOpenReadSeekCloser(r ReadSeekCloser) OpenReadSeekCloser
// EX L97-106: ReadString(r io.Reader) (string, error)
// Source: common/hugio/writers.go (113 lines; 3/7 funcs executed)
//   types: FlexiWriter, multiWriteCloser, ReadWriteCloser, PipeReadWriteCloser
// EX L33-41: (m multiWriteCloser) Close() error
// EX L45-51: NewMultiWriteCloser(writeClosers ...io.WriteCloser) io.WriteCloser
// EX L55-67: ToWriteCloser(w io.Writer) io.WriteCloser
//    L71-83: ToReadCloser(r io.Reader) io.ReadCloser
//    L98-101: NewPipeReadWriteCloser() PipeReadWriteCloser
//    L103-109: (c PipeReadWriteCloser) Close() (err error)
//    L111-113: (c PipeReadWriteCloser) WriteString(s string) (int, error)
// ---------------------------------------------------------------------------
