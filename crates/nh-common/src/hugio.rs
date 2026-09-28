//! Port of `common/hugio/copy.go`, `common/hugio/hasBytesWriter.go`, `common/hugio/readers.go`, `common/hugio/writers.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

//! Go `common/hugio`: reader/writer helpers. `ReadSeekCloser` providers are how resources open
//! their source bytes lazily.

use std::io::{self, Read, Seek, Write};
use std::sync::Arc;

use go_unicode::strings;

use crate::herrors::{Error, Result};

/// Go: `hugio.ReadSeekCloser` (implemented by `afero.File`; the common type for the content of
/// resources, even for strings). Closing is dropping.
pub trait ReadSeekCloser: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeekCloser for T {}

/// Go: `hugio.OpenReadSeekCloser` — opens the source anew each call.
pub type OpenReadSeekCloser = Arc<dyn Fn() -> Result<Box<dyn ReadSeekCloser>> + Send + Sync>;

/// Go: `hugio.NewReadSeekerNoOpCloserFromBytes`.
// Go: common/hugio/readers.go:NewReadSeekerNoOpCloserFromBytes
pub fn read_seeker_from_bytes(b: Vec<u8>) -> Box<dyn ReadSeekCloser> {
    Box::new(io::Cursor::new(b))
}

/// Go: `hugio.NewReadSeekerNoOpCloserFromString` — a reader that also implements Go's
/// `StringReader` ([`StringReadSeeker::read_string`] returns the whole string whatever has been
/// read).
// Go: common/hugio/readers.go:NewReadSeekerNoOpCloserFromString
pub fn new_read_seeker_no_op_closer_from_string(content: &str) -> StringReadSeeker {
    StringReadSeeker {
        s: content.to_string(),
        r: io::Cursor::new(content.as_bytes().to_vec()),
    }
}

/// Go: `hugio.stringReadSeeker`.
pub struct StringReadSeeker {
    s: String,
    r: io::Cursor<Vec<u8>>,
}

impl StringReadSeeker {
    /// Go: `StringReader.ReadString()`.
    // Go: common/hugio/readers.go:ReadString
    pub fn read_string(&self) -> &str {
        &self.s
    }
}

impl Read for StringReadSeeker {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.r.read(buf)
    }
}

impl Seek for StringReadSeeker {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.r.seek(pos)
    }
}

/// Go: `hugio.NewOpenReadSeekCloser` for byte content: every call opens a reader positioned at
/// the start (Go seeks one shared reader back to the start).
// Go: common/hugio/readers.go:NewOpenReadSeekCloser
pub fn new_open_read_seek_closer_from_bytes(b: Arc<Vec<u8>>) -> OpenReadSeekCloser {
    Arc::new(move || Ok(read_seeker_from_bytes(b.as_ref().clone())))
}

/// Go: `hugio.ReadString(r)` — reads everything from `r`. A [`StringReadSeeker`] should use its
/// own `read_string` (Go's `StringReader` fast path).
// Go: common/hugio/readers.go:ReadString
pub fn read_all(r: &mut dyn Read) -> Result<Vec<u8>> {
    let mut v = Vec::new();
    r.read_to_end(&mut v)?;
    Ok(v)
}

/// Go: `hugio.HasBytesWriter` — records which of `patterns` occur in the written stream (handles
/// matches spanning writes). Used by the publish fs to find files with `__h_pp_l1` placeholders.
///
/// Go's quirks are kept: the whole buffer (unwritten zero bytes and the stale right half after a
/// shift included) is searched, and the rest of a `Write` is skipped as soon as one pattern
/// matches.
#[derive(Clone, Debug, Default)]
pub struct HasBytesWriter {
    pub patterns: Vec<HasBytesPattern>,
    pub(crate) i: usize,
    pub(crate) done: bool,
    pub(crate) buff: Vec<u8>,
}

/// Go: `hugio.HasBytesPattern`.
#[derive(Clone, Debug, Default)]
pub struct HasBytesPattern {
    pub pattern: Vec<u8>,
    /// Go: `Match`.
    pub matched: bool,
}

impl HasBytesWriter {
    /// A writer for the given patterns (none matched yet).
    pub fn new(patterns: Vec<Vec<u8>>) -> Self {
        HasBytesWriter {
            patterns: patterns
                .into_iter()
                .map(|pattern| HasBytesPattern {
                    pattern,
                    matched: false,
                })
                .collect(),
            ..Default::default()
        }
    }

    // Go: common/hugio/hasBytesWriter.go:patternLen
    fn pattern_len(&self) -> usize {
        let mut l = 0;
        for p in &self.patterns {
            l += p.pattern.len();
        }
        l
    }

    /// Go: `Write(p)` (never fails; returns `len(p)`). Like Go, panics (index out of range) when
    /// the patterns are all empty.
    // Go: common/hugio/hasBytesWriter.go:Write
    pub fn write(&mut self, p: &[u8]) -> usize {
        if self.done {
            return p.len();
        }

        if self.buff.is_empty() {
            self.buff = vec![0; self.pattern_len() * 2];
        }

        for &c in p {
            self.buff[self.i] = c;
            self.i += 1;
            if self.i == self.buff.len() {
                // Shift left.
                let half = self.buff.len() / 2;
                self.buff.copy_within(half.., 0);
                self.i = half;
            }

            for pp in 0..self.patterns.len() {
                if strings::contains(&self.buff, &self.patterns[pp].pattern) {
                    self.patterns[pp].matched = true;
                    let done = self.patterns.iter().all(|ppp| ppp.matched);
                    if done {
                        self.done = true;
                    }
                    return p.len();
                }
            }
        }

        p.len()
    }
}

impl Write for HasBytesWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(HasBytesWriter::write(self, buf))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Go: `hugio.ToWriteCloser` etc. are not needed; writers are `Box<dyn Write + Send>` and closing
/// is dropping (or [`MultiWriteCloser::close`]).
pub type BoxWriter = Box<dyn Write + Send>;

/// Go: `hugio.multiWriteCloser` — duplicates its writes to all the writers (`io.MultiWriter`:
/// each writer gets the whole buffer in order; the first error stops the write).
pub struct MultiWriteCloser {
    writers: Vec<BoxWriter>,
}

/// Go: `hugio.NewMultiWriteCloser(writeClosers...)`.
// Go: common/hugio/writers.go:NewMultiWriteCloser
pub fn new_multi_write_closer(writers: Vec<BoxWriter>) -> MultiWriteCloser {
    MultiWriteCloser { writers }
}

impl MultiWriteCloser {
    /// Go: `Close()` — closes (flushes) every writer and returns the last error.
    // Go: common/hugio/writers.go:Close
    pub fn close(mut self) -> io::Result<()> {
        let mut err = Ok(());
        for w in &mut self.writers {
            if let Err(e) = w.flush() {
                err = Err(e);
            }
        }
        err
    }
}

impl Write for MultiWriteCloser {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for w in &mut self.writers {
            w.write_all(buf)?;
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        for w in &mut self.writers {
            w.flush()?;
        }
        Ok(())
    }
}

/// Go: `hugio.CopyFile(fs, from, to)` needs an `afero.Fs` (nh-hugofs); not on the seeksnack path.
// Go: common/hugio/copy.go:CopyFile
pub fn copy_file(_from: &str, _to: &str) -> Result<()> {
    Err(Error::new("neohugo-rs: hugio.CopyFile is not supported"))
}

/// Go: `hugio.CopyDir(fs, from, to, shouldCopy)`; see [`copy_file`].
// Go: common/hugio/copy.go:CopyDir
pub fn copy_dir(_from: &str, _to: &str) -> Result<()> {
    Err(Error::new("neohugo-rs: hugio.CopyDir is not supported"))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: common/hugio/copy.go (92 lines; 0/2 funcs executed)
// STUB L26-50: CopyFile(fs afero.Fs, from, to string) error
// STUB L53-92: CopyDir(fs afero.Fs, from, to string, shouldCopy func(filename string) bool) error
// Source: common/hugio/hasBytesWriter.go (80 lines; 2/2 funcs executed)
//   types: HasBytesWriter, HasBytesPattern
// OK L34-40: (h *HasBytesWriter) patternLen() int
// OK L42-80: (h *HasBytesWriter) Write(p []byte) (n int, err error)
// Source: common/hugio/readers.go (106 lines; 4/7 funcs executed)
//   types: ReadSeeker, ReadSeekCloser, ReadSeekCloserProvider, readSeekerNopCloser, stringReadSeeker, StringReader,
//          OpenReadSeekCloser
// OK L46-48: (r readSeekerNopCloser) Close() error (dropping)
// OK L51-53: NewReadSeekerNoOpCloser(r ReadSeeker) ReadSeekCloser (any Read + Seek is one)
// OK L57-59: NewReadSeekerNoOpCloserFromString(content string) ReadSeekCloser
// OK L68-70: (s *stringReadSeeker) ReadString() string
// OK L79-81: NewReadSeekerNoOpCloserFromBytes(content []byte) readSeekerNopCloser
// OK L85-90: NewOpenReadSeekCloser(r ReadSeekCloser) OpenReadSeekCloser (bytes form)
// OK L97-106: ReadString(r io.Reader) (string, error) (read_all)
// Source: common/hugio/writers.go (113 lines; 3/7 funcs executed)
//   types: FlexiWriter, multiWriteCloser, ReadWriteCloser, PipeReadWriteCloser
// OK L33-41: (m multiWriteCloser) Close() error
// OK L45-51: NewMultiWriteCloser(writeClosers ...io.WriteCloser) io.WriteCloser
// OK L55-67: ToWriteCloser(w io.Writer) io.WriteCloser (BoxWriter; closing is dropping)
//    L71-83: ToReadCloser(r io.Reader) io.ReadCloser (not needed: closing is dropping)
//    L98-101: NewPipeReadWriteCloser() PipeReadWriteCloser (not needed)
//    L103-109: (c PipeReadWriteCloser) Close() (err error) (not needed)
//    L111-113: (c PipeReadWriteCloser) WriteString(s string) (int, error) (not needed)
// ---------------------------------------------------------------------------
