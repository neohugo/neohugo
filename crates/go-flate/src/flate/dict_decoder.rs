//! Port of go1.27.1 `compress/flate/dict_decoder.go`.

/// dictDecoder implements the LZ77 sliding dictionary as used in decompression.
/// LZ77 decompresses data through sequences of two forms of commands:
///
///   - Literal insertions: Runs of one or more symbols are inserted into the data
///     stream as is. This is accomplished through the writeByte method for a
///     single symbol, or combinations of writeSlice/writeMark for multiple symbols.
///     Any valid stream must start with a literal insertion if no preset dictionary
///     is used.
///
///   - Backward copies: Runs of one or more symbols are copied from previously
///     emitted data. Backward copies come as the tuple (dist, length) where dist
///     determines how far back in the stream to copy from and length determines how
///     many bytes to copy. Note that it is valid for the length to be greater than
///     the distance. Since LZ77 uses forward copies, that situation is used to
///     perform a form of run-length encoding on repeated runs of symbols.
///     The writeCopy and tryWriteCopy are used to implement this command.
///
/// For performance reasons, this implementation performs little to no sanity
/// checks about the arguments. As such, the invariants documented for each
/// method call must be respected.
#[derive(Default)]
pub(crate) struct DictDecoder {
    pub(crate) hist: Vec<u8>, // Sliding window history

    // Invariant: 0 <= rdPos <= wrPos <= len(hist)
    wr_pos: usize, // Current output position in buffer
    rd_pos: usize, // Have emitted hist[:rdPos] already
    full: bool,    // Has a full window length been written yet?
}

impl DictDecoder {
    // Go: compress/flate/dict_decoder.go:(*dictDecoder).init
    /// init initializes dictDecoder to have a sliding window dictionary of the given
    /// size. If a preset dict is provided, it will initialize the dictionary with
    /// the contents of dict.
    pub(crate) fn init(&mut self, size: usize, mut dict: &[u8]) {
        let hist = std::mem::take(&mut self.hist);
        *self = DictDecoder {
            hist,
            ..Default::default()
        };

        if self.hist.capacity() < size || self.hist.len() < size {
            self.hist = vec![0; size];
        }
        self.hist.truncate(size);

        if dict.len() > self.hist.len() {
            dict = &dict[dict.len() - self.hist.len()..];
        }
        self.hist[..dict.len()].copy_from_slice(dict);
        self.wr_pos = dict.len();
        if self.wr_pos == self.hist.len() {
            self.wr_pos = 0;
            self.full = true;
        }
        self.rd_pos = self.wr_pos;
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).histSize
    /// histSize reports the total amount of historical data in the dictionary.
    pub(crate) fn hist_size(&self) -> usize {
        if self.full {
            return self.hist.len();
        }
        self.wr_pos
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).availRead
    /// availRead reports the number of bytes that can be flushed by readFlush.
    pub(crate) fn avail_read(&self) -> usize {
        self.wr_pos - self.rd_pos
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).availWrite
    /// availWrite reports the available amount of output buffer space.
    pub(crate) fn avail_write(&self) -> usize {
        self.hist.len() - self.wr_pos
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).writeSlice
    /// writeSlice returns a slice of the available buffer to write data to.
    ///
    /// This invariant will be kept: len(s) <= availWrite()
    pub(crate) fn write_slice(&mut self) -> &mut [u8] {
        &mut self.hist[self.wr_pos..]
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).writeMark
    /// writeMark advances the writer pointer by cnt.
    ///
    /// This invariant must be kept: 0 <= cnt <= availWrite()
    pub(crate) fn write_mark(&mut self, cnt: usize) {
        self.wr_pos += cnt;
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).writeByte
    /// writeByte writes a single byte to the dictionary.
    ///
    /// This invariant must be kept: 0 < availWrite()
    pub(crate) fn write_byte(&mut self, c: u8) {
        self.hist[self.wr_pos] = c;
        self.wr_pos += 1;
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).writeCopy
    /// writeCopy copies a string at a given (dist, length) to the output.
    /// This returns the number of bytes copied and may be less than the requested
    /// length if the available space in the output buffer is too small.
    ///
    /// This invariant must be kept: 0 < dist <= histSize()
    pub(crate) fn write_copy(&mut self, dist: usize, length: usize) -> usize {
        let dst_base = self.wr_pos as isize;
        let mut dst_pos = dst_base;
        let mut src_pos = dst_pos - dist as isize;
        let end_pos = std::cmp::min(dst_pos + length as isize, self.hist.len() as isize);

        // Copy non-overlapping section after destination position.
        //
        // This section is non-overlapping in that the copy length for this section
        // is always less than or equal to the backwards distance. This can occur
        // if a distance refers to data that wraps-around in the buffer.
        // Thus, a backwards copy is performed here; that is, the exact bytes in
        // the source prior to the copy is placed in the destination.
        if src_pos < 0 {
            src_pos += self.hist.len() as isize;
            let hist_len = self.hist.len();
            dst_pos += go_copy_within(
                &mut self.hist,
                src_pos as usize,
                hist_len,
                dst_pos as usize,
                end_pos as usize,
            ) as isize;
            src_pos = 0;
        }

        // Copy possibly overlapping section before destination position.
        //
        // This section can overlap if the copy length for this section is larger
        // than the backwards distance. This is allowed by LZ77 so that repeated
        // strings can be succinctly represented using (dist, length) pairs.
        // Thus, a forwards copy is performed here; that is, the bytes copied is
        // possibly dependent on the resulting bytes in the destination as the copy
        // progresses along. This is functionally equivalent to the following:
        //
        //	for i := 0; i < endPos-dstPos; i++ {
        //		dd.hist[dstPos+i] = dd.hist[srcPos+i]
        //	}
        //	dstPos = endPos
        //
        while dst_pos < end_pos {
            dst_pos += go_copy_within(
                &mut self.hist,
                src_pos as usize,
                dst_pos as usize,
                dst_pos as usize,
                end_pos as usize,
            ) as isize;
        }

        self.wr_pos = dst_pos as usize;
        (dst_pos - dst_base) as usize
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).tryWriteCopy
    /// tryWriteCopy tries to copy a string at a given (distance, length) to the
    /// output. This specialized version is optimized for short distances.
    ///
    /// This method is designed to be inlined for performance reasons.
    ///
    /// This invariant must be kept: 0 < dist <= histSize()
    pub(crate) fn try_write_copy(&mut self, dist: usize, length: usize) -> usize {
        let mut dst_pos = self.wr_pos;
        let end_pos = dst_pos + length;
        if dst_pos < dist || end_pos > self.hist.len() {
            return 0;
        }
        let dst_base = dst_pos;
        let src_pos = dst_pos - dist;

        // Copy possibly overlapping section before destination position.
        while dst_pos < end_pos {
            dst_pos += go_copy_within(&mut self.hist, src_pos, dst_pos, dst_pos, end_pos);
        }

        self.wr_pos = dst_pos;
        dst_pos - dst_base
    }

    // Go: compress/flate/dict_decoder.go:(*dictDecoder).readFlush
    /// readFlush returns a slice of the historical buffer that is ready to be
    /// emitted to the user. The data returned by readFlush must be fully consumed
    /// before calling any other dictDecoder methods.
    ///
    /// Returns the `(start, end)` range of `hist` (Go returns the subslice).
    pub(crate) fn read_flush(&mut self) -> (usize, usize) {
        let to_read = (self.rd_pos, self.wr_pos);
        self.rd_pos = self.wr_pos;
        if self.wr_pos == self.hist.len() {
            self.wr_pos = 0;
            self.rd_pos = 0;
            self.full = true;
        }
        to_read
    }
}

/// Go's `copy(hist[dstStart:dstEnd], hist[srcStart:srcEnd])` on one buffer:
/// copies `min(dstEnd-dstStart, srcEnd-srcStart)` bytes with memmove
/// semantics and returns that count.
#[inline]
fn go_copy_within(
    hist: &mut [u8],
    src_start: usize,
    src_end: usize,
    dst_start: usize,
    dst_end: usize,
) -> usize {
    let n = std::cmp::min(dst_end - dst_start, src_end - src_start);
    hist.copy_within(src_start..src_start + n, dst_start);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    // Go: compress/flate/dict_decoder_test.go:TestDictDecoder
    #[test]
    fn test_dict_decoder() {
        const ABC: &str = "ABC\n";
        const FOX: &str = "The quick brown fox jumped over the lazy dog!\n";
        const POEM: &str = concat!(
            "The Road Not Taken\nRobert Frost\n",
            "\n",
            "Two roads diverged in a yellow wood,\n",
            "And sorry I could not travel both\n",
            "And be one traveler, long I stood\n",
            "And looked down one as far as I could\n",
            "To where it bent in the undergrowth;\n",
            "\n",
            "Then took the other, as just as fair,\n",
            "And having perhaps the better claim,\n",
            "Because it was grassy and wanted wear;\n",
            "Though as for that the passing there\n",
            "Had worn them really about the same,\n",
            "\n",
            "And both that morning equally lay\n",
            "In leaves no step had trodden black.\n",
            "Oh, I kept the first for another day!\n",
            "Yet knowing how way leads on to way,\n",
            "I doubted if I should ever come back.\n",
            "\n",
            "I shall be telling this with a sigh\n",
            "Somewhere ages and ages hence:\n",
            "Two roads diverged in a wood, and I-\n",
            "I took the one less traveled by,\n",
            "And that has made all the difference.\n",
        );
        // (dist, length): backward distance (0 if this is an insertion) and length.
        const POEM_REFS: &[(usize, usize)] = &[
            (0, 38),
            (33, 3),
            (0, 48),
            (79, 3),
            (0, 11),
            (34, 5),
            (0, 6),
            (23, 7),
            (0, 8),
            (50, 3),
            (0, 2),
            (69, 3),
            (34, 5),
            (0, 4),
            (97, 3),
            (0, 4),
            (43, 5),
            (0, 6),
            (7, 4),
            (88, 7),
            (0, 12),
            (80, 3),
            (0, 2),
            (141, 4),
            (0, 1),
            (196, 3),
            (0, 3),
            (157, 3),
            (0, 6),
            (181, 3),
            (0, 2),
            (23, 3),
            (77, 3),
            (28, 5),
            (128, 3),
            (110, 4),
            (70, 3),
            (0, 4),
            (85, 6),
            (0, 2),
            (182, 6),
            (0, 4),
            (133, 3),
            (0, 7),
            (47, 5),
            (0, 20),
            (112, 5),
            (0, 1),
            (58, 3),
            (0, 8),
            (59, 3),
            (0, 4),
            (173, 3),
            (0, 5),
            (114, 3),
            (0, 4),
            (92, 5),
            (0, 2),
            (71, 3),
            (0, 2),
            (76, 5),
            (0, 1),
            (46, 3),
            (96, 4),
            (130, 4),
            (0, 3),
            (360, 3),
            (0, 3),
            (178, 5),
            (0, 7),
            (75, 3),
            (0, 3),
            (45, 6),
            (0, 6),
            (299, 6),
            (180, 3),
            (70, 6),
            (0, 1),
            (48, 3),
            (66, 4),
            (0, 3),
            (47, 5),
            (0, 9),
            (325, 3),
            (0, 1),
            (359, 3),
            (318, 3),
            (0, 2),
            (199, 3),
            (0, 1),
            (344, 3),
            (0, 3),
            (248, 3),
            (0, 10),
            (310, 3),
            (0, 3),
            (93, 6),
            (0, 3),
            (252, 3),
            (157, 4),
            (0, 2),
            (273, 5),
            (0, 14),
            (99, 4),
            (0, 1),
            (464, 4),
            (0, 2),
            (92, 4),
            (495, 3),
            (0, 1),
            (322, 4),
            (16, 4),
            (0, 3),
            (402, 3),
            (0, 2),
            (237, 4),
            (0, 2),
            (432, 4),
            (0, 1),
            (483, 5),
            (0, 2),
            (294, 4),
            (0, 2),
            (306, 3),
            (113, 5),
            (0, 1),
            (26, 4),
            (164, 3),
            (488, 4),
            (0, 1),
            (542, 3),
            (248, 6),
            (0, 5),
            (205, 3),
            (0, 8),
            (48, 3),
            (449, 6),
            (0, 2),
            (192, 3),
            (328, 4),
            (9, 5),
            (433, 3),
            (0, 3),
            (622, 25),
            (615, 5),
            (46, 5),
            (0, 2),
            (104, 3),
            (475, 10),
            (549, 3),
            (0, 4),
            (597, 8),
            (314, 3),
            (0, 1),
            (473, 6),
            (317, 5),
            (0, 1),
            (400, 3),
            (0, 3),
            (109, 3),
            (151, 3),
            (48, 4),
            (0, 4),
            (125, 3),
            (108, 3),
            (0, 2),
        ];

        let mut got: Vec<u8> = Vec::new();
        let mut want: Vec<u8> = Vec::new();
        let mut dd = DictDecoder::default();
        dd.init(1 << 11, &[]);

        let flush = |dd: &mut DictDecoder, got: &mut Vec<u8>| {
            let (a, b) = dd.read_flush();
            got.extend_from_slice(&dd.hist[a..b]);
        };
        let write_copy =
            |dd: &mut DictDecoder, got: &mut Vec<u8>, dist: usize, mut length: usize| {
                while length > 0 {
                    let mut cnt = dd.try_write_copy(dist, length);
                    if cnt == 0 {
                        cnt = dd.write_copy(dist, length);
                    }
                    length -= cnt;
                    if dd.avail_write() == 0 {
                        flush(dd, got);
                    }
                }
            };
        let write_string = |dd: &mut DictDecoder, got: &mut Vec<u8>, mut s: &[u8]| {
            while !s.is_empty() {
                let dst = dd.write_slice();
                let cnt = std::cmp::min(dst.len(), s.len());
                dst[..cnt].copy_from_slice(&s[..cnt]);
                s = &s[cnt..];
                dd.write_mark(cnt);
                if dd.avail_write() == 0 {
                    flush(dd, got);
                }
            }
        };

        write_string(&mut dd, &mut got, b".");
        want.push(b'.');

        let mut s = POEM.as_bytes();
        for &(dist, length) in POEM_REFS {
            if dist == 0 {
                write_string(&mut dd, &mut got, &s[..length]);
            } else {
                write_copy(&mut dd, &mut got, dist, length);
            }
            s = &s[length..];
        }
        want.extend_from_slice(POEM.as_bytes());

        let hs = dd.hist_size();
        write_copy(&mut dd, &mut got, hs, 33);
        let w33 = want[..33].to_vec();
        want.extend_from_slice(&w33);

        write_string(&mut dd, &mut got, ABC.as_bytes());
        write_copy(&mut dd, &mut got, ABC.len(), 59 * ABC.len());
        want.extend_from_slice(ABC.repeat(60).as_bytes());

        write_string(&mut dd, &mut got, FOX.as_bytes());
        write_copy(&mut dd, &mut got, FOX.len(), 9 * FOX.len());
        want.extend_from_slice(FOX.repeat(10).as_bytes());

        write_string(&mut dd, &mut got, b".");
        write_copy(&mut dd, &mut got, 1, 9);
        want.extend_from_slice(".".repeat(10).as_bytes());

        let upper = POEM.to_ascii_uppercase();
        write_string(&mut dd, &mut got, upper.as_bytes());
        write_copy(&mut dd, &mut got, POEM.len(), 7 * POEM.len());
        want.extend_from_slice(upper.repeat(8).as_bytes());

        let hs = dd.hist_size();
        write_copy(&mut dd, &mut got, hs, 10);
        let tail = want[want.len() - hs..][..10].to_vec();
        want.extend_from_slice(&tail);

        flush(&mut dd, &mut got);
        assert_eq!(
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(&want),
            "final string mismatch"
        );
    }
}
