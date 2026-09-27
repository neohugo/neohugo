// Go: github.com/yuin/goldmark@v1.7.12/text/segment.go

use std::borrow::Cow;

use crate::util;

const SPACE: u8 = b' ';

/// A Segment struct holds information about source positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Segment {
    /// Start is a start position of the segment.
    pub start: i64,
    /// Stop is a stop position of the segment.
    /// This value should be excluded.
    pub stop: i64,
    /// Padding is a padding length of the segment.
    pub padding: i64,
    /// ForceNewline is true if the segment should be ended with a newline.
    /// Some elements(i.e. CodeBlock, FencedCodeBlock) does not trim trailing
    /// newlines. Spec defines that EOF is treated as a newline, so we need to
    /// add a newline to the end of the segment if it is not empty.
    pub force_newline: bool,
}

// Go: text/segment.go:NewSegment
/// NewSegment return a new Segment.
pub fn new_segment(start: i64, stop: i64) -> Segment {
    Segment {
        start,
        stop,
        padding: 0,
        force_newline: false,
    }
}

// Go: text/segment.go:NewSegmentPadding
/// NewSegmentPadding returns a new Segment with the given padding.
pub fn new_segment_padding(start: i64, stop: i64, n: i64) -> Segment {
    Segment {
        start,
        stop,
        padding: n,
        force_newline: false,
    }
}

impl Segment {
    // Go: text/segment.go:Segment.Value
    /// Value returns a value of the segment.
    pub fn value<'a>(&self, buffer: &'a [u8]) -> Cow<'a, [u8]> {
        let mut result: Cow<'a, [u8]> = if self.padding == 0 {
            Cow::Borrowed(&buffer[self.start as usize..self.stop as usize])
        } else {
            let mut r =
                Vec::with_capacity((self.padding + self.stop - self.start + 1).max(0) as usize);
            r.extend(std::iter::repeat_n(SPACE, self.padding as usize));
            r.extend_from_slice(&buffer[self.start as usize..self.stop as usize]);
            Cow::Owned(r)
        };
        if self.force_newline && !result.is_empty() && result[result.len() - 1] != b'\n' {
            result.to_mut().push(b'\n');
        }
        result
    }

    // Go: text/segment.go:Segment.Len
    /// Len returns a length of the segment.
    pub fn len(&self) -> i64 {
        self.stop - self.start + self.padding
    }

    // Go: text/segment.go:Segment.Between
    /// Between returns a segment between this segment and the given segment.
    pub fn between(&self, other: Segment) -> Segment {
        if self.stop != other.stop {
            panic!("invalid state");
        }
        new_segment_padding(self.start, other.start, self.padding - other.padding)
    }

    // Go: text/segment.go:Segment.IsEmpty
    /// IsEmpty returns true if this segment is empty, otherwise false.
    pub fn is_empty(&self) -> bool {
        self.start >= self.stop && self.padding == 0
    }

    // Go: text/segment.go:Segment.TrimRightSpace
    /// TrimRightSpace returns a new segment by slicing off all trailing
    /// space characters.
    pub fn trim_right_space(&self, buffer: &[u8]) -> Segment {
        let v = &buffer[self.start as usize..self.stop as usize];
        let l = util::trim_right_space_length(v);
        if l == v.len() {
            return new_segment(self.start, self.start);
        }
        new_segment_padding(self.start, self.stop - l as i64, self.padding)
    }

    // Go: text/segment.go:Segment.TrimLeftSpace
    /// TrimLeftSpace returns a new segment by slicing off all leading
    /// space characters including padding.
    pub fn trim_left_space(&self, buffer: &[u8]) -> Segment {
        let v = &buffer[self.start as usize..self.stop as usize];
        let l = util::trim_left_space_length(v);
        new_segment(self.start + l as i64, self.stop)
    }

    // Go: text/segment.go:Segment.TrimLeftSpaceWidth
    /// TrimLeftSpaceWidth returns a new segment by slicing off leading space
    /// characters until the given width.
    pub fn trim_left_space_width(&self, mut width: i64, buffer: &[u8]) -> Segment {
        let mut padding = self.padding;
        while width > 0 {
            if padding == 0 {
                break;
            }
            padding -= 1;
            width -= 1;
        }
        if width == 0 {
            return new_segment_padding(self.start, self.stop, padding);
        }
        let text = &buffer[self.start as usize..self.stop as usize];
        let mut start = self.start;
        for &c in text {
            if start >= self.stop - 1 || width <= 0 {
                break;
            }
            if c == b' ' {
                width -= 1;
            } else if c == b'\t' {
                width -= 4;
            } else {
                break;
            }
            start += 1;
        }
        if width < 0 {
            padding = -width;
        }
        new_segment_padding(start, self.stop, padding)
    }

    // Go: text/segment.go:Segment.WithStart
    /// WithStart returns a new Segment with same value except Start.
    pub fn with_start(&self, v: i64) -> Segment {
        new_segment_padding(v, self.stop, self.padding)
    }

    // Go: text/segment.go:Segment.WithStop
    /// WithStop returns a new Segment with same value except Stop.
    pub fn with_stop(&self, v: i64) -> Segment {
        new_segment_padding(self.start, v, self.padding)
    }

    // Go: text/segment.go:Segment.ConcatPadding
    /// ConcatPadding concats the padding to the given slice.
    pub fn concat_padding(&self, v: &mut Vec<u8>) {
        if self.padding > 0 {
            v.extend(std::iter::repeat_n(SPACE, self.padding as usize));
        }
    }
}

/// Segments is a collection of the Segment.
///
/// Go's `*Segments` is shared by pointer; in the Rust arena a node owns its
/// `Segments` and `SetLines` copies (see PORTING.md, "slice aliasing").
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Segments {
    values: Vec<Segment>,
}

// Go: text/segment.go:NewSegments
/// NewSegments return a new Segments.
pub fn new_segments() -> Segments {
    Segments { values: Vec::new() }
}

impl Segments {
    // Go: text/segment.go:Segments.Append
    /// Append appends the given segment after the tail of the collection.
    pub fn append(&mut self, t: Segment) {
        self.values.push(t);
    }

    // Go: text/segment.go:Segments.AppendAll
    /// AppendAll appends all elements of given segments after the tail of the collection.
    pub fn append_all(&mut self, t: &[Segment]) {
        self.values.extend_from_slice(t);
    }

    // Go: text/segment.go:Segments.Len
    /// Len returns the length of the collection.
    pub fn len(&self) -> i64 {
        self.values.len() as i64
    }

    /// Reports whether the collection has no segments.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    // Go: text/segment.go:Segments.At
    /// At returns a segment at the given index.
    pub fn at(&self, i: i64) -> Segment {
        self.values[i as usize]
    }

    // Go: text/segment.go:Segments.Set
    /// Set sets the given Segment.
    pub fn set(&mut self, i: i64, v: Segment) {
        self.values[i as usize] = v;
    }

    // Go: text/segment.go:Segments.SetSliced
    /// SetSliced replace the collection with a subsliced value.
    pub fn set_sliced(&mut self, lo: i64, hi: i64) {
        let lo = lo as usize;
        let hi = hi as usize;
        assert!(
            lo <= hi && hi <= self.values.len(),
            "slice bounds out of range"
        );
        self.values.truncate(hi);
        self.values.drain(..lo);
    }

    // Go: text/segment.go:Segments.Sliced
    /// Sliced returns a subslice of the collection.
    pub fn sliced(&self, lo: i64, hi: i64) -> Vec<Segment> {
        self.values[lo as usize..hi as usize].to_vec()
    }

    // Go: text/segment.go:Segments.Clear
    /// Clear delete all element of the collection.
    pub fn clear(&mut self) {
        self.values.clear();
    }

    // Go: text/segment.go:Segments.Unshift
    /// Unshift insert the given Segment to head of the collection.
    pub fn unshift(&mut self, v: Segment) {
        // Go: append(s.values[0:1], s.values[0:]...) then s.values[0] = v;
        // s.values[0:1] panics on an empty collection.
        assert!(
            !self.values.is_empty(),
            "slice bounds out of range [:1] with capacity 0"
        );
        self.values.insert(0, v);
    }

    // Go: text/segment.go:Segments.Value
    /// Value returns a string value of the collection.
    pub fn value(&self, buffer: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        for v in &self.values {
            result.extend_from_slice(&v.value(buffer));
        }
        result
    }

    /// The segments as a slice.
    pub fn as_slice(&self) -> &[Segment] {
        &self.values
    }
}
