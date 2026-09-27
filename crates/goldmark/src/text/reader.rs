// Go: github.com/yuin/goldmark@v1.7.12/text/reader.go

use std::borrow::Cow;

use go_unicode::Rune;
use go_unicode::utf8;

use super::segment::{Segment, Segments, new_segment, new_segments};
use crate::util;

const INVALID_VALUE: i64 = -1;

/// EOF indicates the end of file.
pub const EOF: u8 = 0xff;

/// FindClosureOptions is options for Reader.FindClosure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FindClosureOptions {
    /// CodeSpan is a flag for the FindClosure. If this is set to true,
    /// FindClosure ignores closers in codespans.
    pub code_span: bool,
    /// Nesting is a flag for the FindClosure. If this is set to true,
    /// FindClosure allows nesting.
    pub nesting: bool,
    /// Newline is a flag for the FindClosure. If this is set to true,
    /// FindClosure searches for a closer over multiple lines.
    pub newline: bool,
    /// Advance is a flag for the FindClosure. If this is set to true,
    /// FindClosure advances pointers when closer is found.
    pub advance: bool,
}

/// A Reader interface provides abstracted method for reading text.
///
/// `'a` is the lifetime of the source bytes: lines returned by
/// [`Reader::peek_line`] borrow the source, not the reader, so a line can be
/// held while the reader is advanced (as Go code does).
pub trait Reader<'a> {
    /// Source returns a source of the reader.
    fn source(&self) -> &'a [u8];

    /// ResetPosition resets positions.
    fn reset_position(&mut self);

    /// Peek returns a byte at current position without advancing the internal pointer.
    fn peek(&self) -> u8;

    /// PeekLine returns the current line without advancing the internal pointer.
    /// `None` is Go's nil line (end of input).
    fn peek_line(&mut self) -> (Option<Cow<'a, [u8]>>, Segment);

    /// PrecendingCharacter returns a character just before current internal pointer.
    fn precending_character(&self) -> Rune;

    /// Value returns a value of the given segment.
    fn value(&self, seg: Segment) -> Cow<'a, [u8]>;

    /// LineOffset returns a distance from the line head to current position.
    fn line_offset(&mut self) -> i64;

    /// Position returns current line number and position.
    fn position(&self) -> (i64, Segment);

    /// SetPosition sets current line number and position.
    fn set_position(&mut self, line: i64, pos: Segment);

    /// SetPadding sets padding to the reader.
    fn set_padding(&mut self, v: i64);

    /// Advance advances the internal pointer.
    fn advance(&mut self, n: i64);

    /// AdvanceAndSetPadding advances the internal pointer and add padding to the
    /// reader.
    fn advance_and_set_padding(&mut self, n: i64, padding: i64);

    /// AdvanceToEOL advances the internal pointer to the end of line.
    /// If the line ends with a newline, it will be included in the segment.
    /// If the line ends with EOF, it will not be included in the segment.
    fn advance_to_eol(&mut self);

    /// AdvanceLine advances the internal pointer to the next line head.
    fn advance_line(&mut self);

    /// SkipSpaces skips space characters and returns a non-blank line.
    /// If it reaches EOF, returns false.
    fn skip_spaces(&mut self) -> (Segment, i64, bool) {
        skip_spaces_reader(self)
    }

    /// SkipSpaces skips blank lines and returns a non-blank line.
    /// If it reaches EOF, returns false.
    fn skip_blank_lines(&mut self) -> (Segment, i64, bool) {
        skip_blank_lines_reader(self)
    }

    /// io.RuneReader: `None` is io.EOF.
    fn read_rune(&mut self) -> Option<(Rune, i64)> {
        read_rune_reader(self)
    }

    /// FindClosure finds corresponding closure.
    fn find_closure(
        &mut self,
        opener: u8,
        closer: u8,
        options: FindClosureOptions,
    ) -> Option<Segments> {
        find_closure_reader(self, opener, closer, options)
    }

    /// Match performs a match of an anchored pattern against the text from
    /// the current position (Go: `Match(reg *regexp.Regexp)`).
    ///
    /// Go runs a `regexp.Regexp` over the reader as an `io.RuneReader`; the
    /// Rust port takes a matcher over a [`RuneStream`] that returns the match
    /// length in bytes (see PORTING.md, "regular expressions").
    fn match_with(&mut self, matcher: &mut dyn FnMut(&mut RuneStream<'_>) -> Option<i64>) -> bool {
        match_reader(self, matcher)
    }

    /// Reset resets current state and sets new segments to the reader
    /// (BlockReader only; Go's `BlockReader` interface).
    fn reset(&mut self, _segments: &Segments) {
        panic!("Reset called on a non-block reader");
    }
}

/// The runes a Go regexp would read from a Reader via io.RuneReader, buffered
/// lazily so a matcher can look ahead (see [`Reader::match_with`]).
pub struct RuneStream<'s> {
    next: &'s mut dyn FnMut() -> Option<Rune>,
    buf: Vec<u8>,
    eof: bool,
}

impl<'s> RuneStream<'s> {
    /// Wraps a rune source (the reader's ReadRune).
    pub fn new(next: &'s mut dyn FnMut() -> Option<Rune>) -> Self {
        RuneStream {
            next,
            buf: Vec::new(),
            eof: false,
        }
    }

    /// The byte at offset `i` from the match start, reading runes as needed;
    /// `None` once the reader returns EOF (end of input or a U+FFFD/invalid
    /// UTF-8 rune, as `readRuneReader` does).
    pub fn at(&mut self, i: usize) -> Option<u8> {
        while self.buf.len() <= i {
            if self.eof {
                return None;
            }
            match (self.next)() {
                Some(r) => utf8::append_rune(&mut self.buf, r),
                None => {
                    self.eof = true;
                    return None;
                }
            }
        }
        Some(self.buf[i])
    }
}

/// A reader (Go: `text.NewReader`) over a whole source.
pub struct TextReader<'a> {
    source: &'a [u8],
    source_length: i64,
    line: i64,
    peeked_line: Option<Cow<'a, [u8]>>,
    pos: Segment,
    head: i64,
    line_offset: i64,
}

// Go: text/reader.go:NewReader
/// NewReader return a new Reader that can read UTF-8 bytes .
pub fn new_reader(source: &[u8]) -> TextReader<'_> {
    let mut r = TextReader {
        source,
        source_length: source.len() as i64,
        line: 0,
        peeked_line: None,
        pos: Segment::default(),
        head: 0,
        line_offset: 0,
    };
    r.reset_position();
    r
}

impl<'a> Reader<'a> for TextReader<'a> {
    fn source(&self) -> &'a [u8] {
        self.source
    }

    // Go: text/reader.go:reader.ResetPosition
    fn reset_position(&mut self) {
        self.line = -1;
        self.head = 0;
        self.line_offset = -1;
        self.advance_line();
    }

    // Go: text/reader.go:reader.Value
    fn value(&self, seg: Segment) -> Cow<'a, [u8]> {
        seg.value(self.source)
    }

    // Go: text/reader.go:reader.Peek
    fn peek(&self) -> u8 {
        if self.pos.start >= 0 && self.pos.start < self.source_length {
            if self.pos.padding != 0 {
                return b' ';
            }
            return self.source[self.pos.start as usize];
        }
        EOF
    }

    // Go: text/reader.go:reader.PeekLine
    fn peek_line(&mut self) -> (Option<Cow<'a, [u8]>>, Segment) {
        if self.pos.start >= 0 && self.pos.start < self.source_length {
            if self.peeked_line.is_none() {
                self.peeked_line = Some(self.pos.value(self.source));
            }
            // Note: the cache survives SetPosition, as in Go.
            return (self.peeked_line.clone(), self.pos);
        }
        (None, self.pos)
    }

    // Go: text/reader.go:reader.LineOffset
    fn line_offset(&mut self) -> i64 {
        if self.line_offset < 0 {
            let mut v: i64 = 0;
            let mut i = self.head;
            while i < self.pos.start {
                if self.source[i as usize] == b'\t' {
                    v += util::tab_width(v);
                } else {
                    v += 1;
                }
                i += 1;
            }
            self.line_offset = v - self.pos.padding;
        }
        self.line_offset
    }

    // Go: text/reader.go:reader.PrecendingCharacter
    fn precending_character(&self) -> Rune {
        if self.pos.start <= 0 {
            if self.pos.padding != 0 {
                return ' ' as Rune;
            }
            return '\n' as Rune;
        }
        let mut i = self.pos.start - 1;
        while i >= 0 {
            if utf8::rune_start(self.source[i as usize]) {
                break;
            }
            i -= 1;
        }
        // Go: utf8.DecodeRune(r.source[i:]) panics when i == -1.
        assert!(i >= 0, "slice bounds out of range [-1:]");
        let (rn, _) = utf8::decode_rune(&self.source[i as usize..]);
        rn
    }

    // Go: text/reader.go:reader.Advance
    fn advance(&mut self, mut n: i64) {
        self.line_offset = -1;
        let peeked_len = self.peeked_line.as_ref().map_or(0, |l| l.len() as i64);
        if n < peeked_len && self.pos.padding == 0 {
            self.pos.start += n;
            self.peeked_line = None;
            return;
        }
        self.peeked_line = None;
        let l = self.source_length;
        while n > 0 && self.pos.start < l {
            if self.pos.padding != 0 {
                self.pos.padding -= 1;
                n -= 1;
                continue;
            }
            if self.source[self.pos.start as usize] == b'\n' {
                self.advance_line();
                n -= 1;
                continue;
            }
            self.pos.start += 1;
            n -= 1;
        }
    }

    // Go: text/reader.go:reader.AdvanceAndSetPadding
    fn advance_and_set_padding(&mut self, n: i64, padding: i64) {
        self.advance(n);
        if padding > self.pos.padding {
            self.set_padding(padding);
        }
    }

    // Go: text/reader.go:reader.AdvanceToEOL
    fn advance_to_eol(&mut self) {
        if self.pos.start >= self.source_length {
            return;
        }

        self.line_offset = -1;
        let mut i: i64 = -1;
        if let Some(pl) = &self.peeked_line {
            self.pos.start += pl.len() as i64 - self.pos.padding - 1;
            if self.source[self.pos.start as usize] == b'\n' {
                i = 0;
            }
        }
        if i == -1 {
            i = util::index_byte(&self.source[self.pos.start as usize..], b'\n');
        }
        self.peeked_line = None;
        if i != -1 {
            self.pos.start += i;
        } else {
            self.pos.start = self.source_length;
        }
        self.pos.padding = 0;
    }

    // Go: text/reader.go:reader.AdvanceLine
    fn advance_line(&mut self) {
        self.line_offset = -1;
        self.peeked_line = None;
        self.pos.start = self.pos.stop;
        self.head = self.pos.start;
        if self.pos.start < 0 || self.pos.start >= self.source_length {
            return;
        }
        self.pos.stop = self.source_length;
        let mut i: i64 = 0;
        if self.source[self.pos.start as usize] != b'\n' {
            i = util::index_byte(&self.source[self.pos.start as usize..], b'\n');
        }
        if i != -1 {
            self.pos.stop = self.pos.start + i + 1;
        }
        self.line += 1;
        self.pos.padding = 0;
    }

    // Go: text/reader.go:reader.Position
    fn position(&self) -> (i64, Segment) {
        (self.line, self.pos)
    }

    // Go: text/reader.go:reader.SetPosition
    fn set_position(&mut self, line: i64, pos: Segment) {
        self.line_offset = -1;
        self.line = line;
        self.pos = pos;
    }

    // Go: text/reader.go:reader.SetPadding
    fn set_padding(&mut self, v: i64) {
        self.pos.padding = v;
    }
}

/// A reader (Go: `text.NewBlockReader`) over the lines (segments) of a block.
pub struct BlockReader<'a> {
    source: &'a [u8],
    segments: Segments,
    segments_length: i64,
    line: i64,
    pos: Segment,
    head: i64,
    last: i64,
    line_offset: i64,
}

// Go: text/reader.go:NewBlockReader
/// NewBlockReader returns a new BlockReader.
pub fn new_block_reader<'a>(source: &'a [u8], segments: Option<&Segments>) -> BlockReader<'a> {
    let mut r = BlockReader {
        source,
        segments: new_segments(),
        segments_length: 0,
        line: 0,
        pos: Segment::default(),
        head: 0,
        last: 0,
        line_offset: 0,
    };
    if let Some(segments) = segments {
        r.reset(segments);
    }
    r
}

impl<'a> Reader<'a> for BlockReader<'a> {
    fn source(&self) -> &'a [u8] {
        self.source
    }

    // Go: text/reader.go:blockReader.ResetPosition
    fn reset_position(&mut self) {
        self.line = -1;
        self.head = 0;
        self.last = 0;
        self.line_offset = -1;
        self.pos.start = -1;
        self.pos.stop = -1;
        self.pos.padding = 0;
        if self.segments_length > 0 {
            let last = self.segments.at(self.segments_length - 1);
            self.last = last.stop;
        }
        self.advance_line();
    }

    // Go: text/reader.go:blockReader.Reset
    fn reset(&mut self, segments: &Segments) {
        self.segments = segments.clone();
        self.segments_length = segments.len();
        self.reset_position();
    }

    // Go: text/reader.go:blockReader.Value
    fn value(&self, seg: Segment) -> Cow<'a, [u8]> {
        let mut line = self.segments_length - 1;
        let mut ret: Vec<u8> = Vec::with_capacity((seg.stop - seg.start + 1).max(0) as usize);
        while line >= 0 {
            if seg.start >= self.segments.at(line).start {
                break;
            }
            line -= 1;
        }
        let mut i = seg.start;
        while line < self.segments_length {
            // Go: r.segments.At(line) with line == -1 panics.
            let s = self.segments.at(line);
            if i < 0 {
                i = s.start;
            }
            s.concat_padding(&mut ret);
            while i < seg.stop && i < s.stop {
                ret.push(self.source[i as usize]);
                i += 1;
            }
            i = -1;
            if s.stop > seg.stop {
                break;
            }
            line += 1;
        }
        Cow::Owned(ret)
    }

    // Go: text/reader.go:blockReader.PrecendingCharacter
    fn precending_character(&self) -> Rune {
        if self.pos.padding != 0 {
            return ' ' as Rune;
        }
        if self.segments.len() < 1 {
            return '\n' as Rune;
        }
        let first_segment = self.segments.at(0);
        if self.line == 0 && self.pos.start <= first_segment.start {
            return '\n' as Rune;
        }
        let l = self.source.len() as i64;
        let mut i = self.pos.start - 1;
        while i < l && i >= 0 {
            if utf8::rune_start(self.source[i as usize]) {
                break;
            }
            i -= 1;
        }
        if i < 0 || i >= l {
            return '\n' as Rune;
        }
        let (rn, _) = utf8::decode_rune(&self.source[i as usize..]);
        rn
    }

    // Go: text/reader.go:blockReader.LineOffset
    fn line_offset(&mut self) -> i64 {
        if self.line_offset < 0 {
            let mut v: i64 = 0;
            let mut i = self.head;
            while i < self.pos.start {
                if self.source[i as usize] == b'\t' {
                    v += util::tab_width(v);
                } else {
                    v += 1;
                }
                i += 1;
            }
            self.line_offset = v - self.pos.padding;
        }
        self.line_offset
    }

    // Go: text/reader.go:blockReader.Peek
    fn peek(&self) -> u8 {
        if self.line < self.segments_length && self.pos.start >= 0 && self.pos.start < self.last {
            if self.pos.padding != 0 {
                return b' ';
            }
            return self.source[self.pos.start as usize];
        }
        EOF
    }

    // Go: text/reader.go:blockReader.PeekLine
    fn peek_line(&mut self) -> (Option<Cow<'a, [u8]>>, Segment) {
        if self.line < self.segments_length && self.pos.start >= 0 && self.pos.start < self.last {
            return (Some(self.pos.value(self.source)), self.pos);
        }
        (None, self.pos)
    }

    // Go: text/reader.go:blockReader.Advance
    fn advance(&mut self, mut n: i64) {
        self.line_offset = -1;

        if n < self.pos.stop - self.pos.start && self.pos.padding == 0 {
            self.pos.start += n;
            return;
        }

        while n > 0 {
            if self.pos.padding != 0 {
                self.pos.padding -= 1;
                n -= 1;
                continue;
            }
            if self.pos.start >= self.pos.stop - 1 && self.pos.stop < self.last {
                self.advance_line();
                n -= 1;
                continue;
            }
            self.pos.start += 1;
            n -= 1;
        }
    }

    // Go: text/reader.go:blockReader.AdvanceAndSetPadding
    fn advance_and_set_padding(&mut self, n: i64, padding: i64) {
        self.advance(n);
        if padding > self.pos.padding {
            self.set_padding(padding);
        }
    }

    // Go: text/reader.go:blockReader.AdvanceToEOL
    fn advance_to_eol(&mut self) {
        self.line_offset = -1;
        self.pos.padding = 0;
        let c = self.source[(self.pos.stop - 1) as usize];
        if c == b'\n' {
            self.pos.start = self.pos.stop - 1;
        } else {
            self.pos.start = self.pos.stop;
        }
    }

    // Go: text/reader.go:blockReader.AdvanceLine
    fn advance_line(&mut self) {
        self.set_position(self.line + 1, new_segment(INVALID_VALUE, INVALID_VALUE));
        self.head = self.pos.start;
    }

    // Go: text/reader.go:blockReader.Position
    fn position(&self) -> (i64, Segment) {
        (self.line, self.pos)
    }

    // Go: text/reader.go:blockReader.SetPosition
    fn set_position(&mut self, line: i64, pos: Segment) {
        self.line_offset = -1;
        self.line = line;
        if pos.start == INVALID_VALUE {
            if self.line < self.segments_length {
                let s = self.segments.at(line);
                self.head = s.start;
                self.pos = s;
            }
        } else {
            self.pos = pos;
            if self.line < self.segments_length {
                let s = self.segments.at(line);
                self.head = s.start;
            }
        }
    }

    // Go: text/reader.go:blockReader.SetPadding
    fn set_padding(&mut self, v: i64) {
        self.line_offset = -1;
        self.pos.padding = v;
    }
}

// Go: text/reader.go:skipBlankLinesReader
fn skip_blank_lines_reader<'a, R: Reader<'a> + ?Sized>(r: &mut R) -> (Segment, i64, bool) {
    let mut lines: i64 = 0;
    loop {
        let (line, seg) = r.peek_line();
        let Some(line) = line else {
            return (seg, lines, false);
        };
        if util::is_blank(&line) {
            lines += 1;
            r.advance_line();
        } else {
            return (seg, lines, true);
        }
    }
}

// Go: text/reader.go:skipSpacesReader
fn skip_spaces_reader<'a, R: Reader<'a> + ?Sized>(r: &mut R) -> (Segment, i64, bool) {
    let mut chars: i64 = 0;
    loop {
        let (line, segment) = r.peek_line();
        let Some(line) = line else {
            return (segment, chars, false);
        };
        for (i, &c) in line.iter().enumerate() {
            if util::is_space(c) {
                chars += 1;
                r.advance(1);
                continue;
            }
            return (
                segment.with_start(segment.start + i as i64 + 1),
                chars,
                true,
            );
        }
    }
}

// Go: text/reader.go:matchReader
fn match_reader<'a, R: Reader<'a> + ?Sized>(
    r: &mut R,
    matcher: &mut dyn FnMut(&mut RuneStream<'_>) -> Option<i64>,
) -> bool {
    let (oldline, oldseg) = r.position();
    let m = {
        let mut next = || r.read_rune().map(|(rn, _)| rn);
        let mut stream = RuneStream::new(&mut next);
        matcher(&mut stream)
    };
    r.set_position(oldline, oldseg);
    let Some(len) = m else {
        return false;
    };
    r.advance(len);
    true
}

// Go: text/reader.go:readRuneReader
fn read_rune_reader<'a, R: Reader<'a> + ?Sized>(r: &mut R) -> Option<(Rune, i64)> {
    let (line, _) = r.peek_line();
    let line = line?;
    let (rn, size) = utf8::decode_rune(&line);
    if rn == utf8::RUNE_ERROR {
        return None;
    }
    r.advance(size as i64);
    Some((rn, size as i64))
}

// Go: text/reader.go:findClosureReader
fn find_closure_reader<'a, R: Reader<'a> + ?Sized>(
    r: &mut R,
    opener: u8,
    closer: u8,
    opts: FindClosureOptions,
) -> Option<Segments> {
    let mut opened = 1;
    let mut code_span_opener = 0;
    let mut closed = false;
    let (orgline, orgpos) = r.position();
    let mut ret: Option<Segments> = None;

    'outer: loop {
        let (bs, seg) = r.peek_line();
        let Some(bs) = bs else {
            break 'outer;
        };
        let mut i: i64 = 0;
        let len = bs.len() as i64;
        while i < len {
            let c = bs[i as usize];
            if opts.code_span && code_span_opener != 0 && c == b'`' {
                let mut code_span_closer = 0;
                while i < len {
                    if bs[i as usize] == b'`' {
                        code_span_closer += 1;
                    } else {
                        i -= 1;
                        break;
                    }
                    i += 1;
                }
                if code_span_closer == code_span_opener {
                    code_span_opener = 0;
                }
            } else if code_span_opener == 0
                && c == b'\\'
                && i < len - 1
                && util::is_punct(bs[(i + 1) as usize])
            {
                i += 2;
                continue;
            } else if opts.code_span && code_span_opener == 0 && c == b'`' {
                while i < len {
                    if bs[i as usize] == b'`' {
                        code_span_opener += 1;
                    } else {
                        i -= 1;
                        break;
                    }
                    i += 1;
                }
            } else if (opts.code_span && code_span_opener == 0) || !opts.code_span {
                if c == closer {
                    opened -= 1;
                    if opened == 0 {
                        let r2 = ret.get_or_insert_with(new_segments);
                        r2.append(seg.with_stop(seg.start + i));
                        r.advance(i + 1);
                        closed = true;
                        break 'outer;
                    }
                } else if c == opener {
                    if !opts.nesting {
                        break 'outer;
                    }
                    opened += 1;
                }
            }
            i += 1;
        }
        if !opts.newline {
            break 'outer;
        }
        r.advance_line();
        ret.get_or_insert_with(new_segments).append(seg);
    }
    if !opts.advance {
        r.set_position(orgline, orgpos);
    }
    if closed {
        return ret;
    }
    None
}
