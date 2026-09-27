//! Port of go1.27.1 `compress/flate/level1.go`.

use super::deflatefast::{
    BASE_MATCH_LENGTH, BASE_MATCH_OFFSET, BUFFER_RESET, FastGen, MAX_MATCH_OFFSET, TABLE_BITS,
    TABLE_SIZE, TableEntry, hash_len, load_le8, load_le32, load_le64, match_len_long,
};
use super::token::{
    LENGTH_CODES1, LENGTH_SHIFT, MATCH_TYPE, Token, Tokens, emit_literals, offset_code,
};

/// Level 1 uses a single small table with 5 byte hashes.
pub(crate) struct FastEncL1 {
    pub(crate) fg: FastGen,
    table: Vec<TableEntry>, // [tableSize]tableEntry
}

impl FastEncL1 {
    pub(crate) fn new(fg: FastGen) -> FastEncL1 {
        FastEncL1 {
            fg,
            table: vec![0; TABLE_SIZE],
        }
    }

    // Go: compress/flate/level1.go:(*fastEncL1).encode
    pub(crate) fn encode(&mut self, dst: &mut Tokens, src: &[u8]) {
        const INPUT_MARGIN: i32 = 12 - 1;
        const MIN_NON_LITERAL_BLOCK_SIZE: usize = 1 + 1 + INPUT_MARGIN as usize;
        const HASH_BYTES: u8 = 5;

        // Protect against e.cur wraparound.
        while self.fg.cur >= BUFFER_RESET {
            if self.fg.hist.is_empty() {
                self.table.fill(0);
                self.fg.cur = MAX_MATCH_OFFSET;
                break;
            }
            // Shift down everything in the table that isn't already too far away.
            let min_off = self.fg.cur + self.fg.hist.len() as i32 - MAX_MATCH_OFFSET;
            for v in self.table.iter_mut() {
                if *v <= min_off {
                    *v = 0;
                } else {
                    *v = *v - self.fg.cur + MAX_MATCH_OFFSET;
                }
            }
            self.fg.cur = MAX_MATCH_OFFSET;
        }

        let mut s = self.fg.add_block(src);

        if src.len() < MIN_NON_LITERAL_BLOCK_SIZE {
            // We do not fill the token table.
            // This will be picked up by caller.
            dst.n = src.len() as u16;
            return;
        }

        // Override src
        let src: &[u8] = &self.fg.hist;
        let cur = self.fg.cur;
        let table = &mut self.table[..];

        // nextEmit is where in src the next emitLiterals should start from.
        let mut next_emit = s;

        // sLimit is when to stop looking for offset/length copies. The inputMargin
        // lets us use a fast path for emitLiterals in the main loop, while we are
        // looking for copies.
        let s_limit = (src.len() as i32).wrapping_sub(INPUT_MARGIN);

        let mut cv = load_le64(src, s);

        'outer: loop {
            const SKIP_LOG: i32 = 5;
            const DO_EVERY: i32 = 2;

            let mut next_s;
            let mut candidate: TableEntry;
            let mut t: i32;
            loop {
                let mut next_hash = hash_len(cv, TABLE_BITS, HASH_BYTES) as usize;
                candidate = table[next_hash];
                next_s = s + DO_EVERY + ((s - next_emit) >> SKIP_LOG);
                if next_s > s_limit {
                    break 'outer; // goto emitRemainder
                }

                let mut now = load_le64(src, next_s);
                table[next_hash] = s + cur;
                next_hash = hash_len(now, TABLE_BITS, HASH_BYTES) as usize;
                t = candidate - cur;
                if s - t < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, t) {
                    table[next_hash] = next_s + cur;
                    break;
                }

                // Do one right away...
                cv = now;
                s = next_s;
                next_s += 1;
                candidate = table[next_hash];
                now >>= 8;
                table[next_hash] = s + cur;

                t = candidate - cur;
                if s - t < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, t) {
                    table[next_hash] = next_s + cur;
                    break;
                }
                cv = now;
                s = next_s;
            }

            // A 4-byte match has been found. We'll later see if more than 4 bytes
            // match. But, prior to the match, src[nextEmit:s] are unmatched. Emit
            // them as literal bytes.
            loop {
                // Invariant: we have a 4-byte match at s, and no need to emit any
                // literal bytes prior to s.

                // Extend the 4-byte match as long as possible.
                let mut l = match_len_long((s + 4) as usize, (t + 4) as usize, src) + 4;

                // Extend backwards
                while t > 0 && s > next_emit && load_le8(src, t - 1) == load_le8(src, s - 1) {
                    s -= 1;
                    t -= 1;
                    l += 1;
                }
                if next_emit < s {
                    for &v in &src[next_emit as usize..s as usize] {
                        dst.tokens[dst.n as usize] = v as Token;
                        dst.lit_hist[v as usize] = dst.lit_hist[v as usize].wrapping_add(1);
                        dst.n = dst.n.wrapping_add(1);
                    }
                }

                // Save the match found. Same as 'dst.AddMatchLong(l, uint32(s-t-baseMatchOffset))'
                let mut x_offset = (s - t - BASE_MATCH_OFFSET) as u32;
                let mut x_length = l;
                let oc = offset_code(x_offset);
                x_offset |= oc << 16;
                while x_length > 0 {
                    let mut xl = x_length;
                    if xl > 258 {
                        if xl > 258 + BASE_MATCH_LENGTH {
                            xl = 258;
                        } else {
                            xl = 258 - BASE_MATCH_LENGTH;
                        }
                    }
                    x_length -= xl;
                    xl -= BASE_MATCH_LENGTH;
                    let lc = LENGTH_CODES1[xl as u8 as usize] as usize;
                    dst.extra_hist[lc] = dst.extra_hist[lc].wrapping_add(1);
                    dst.off_hist[oc as usize] = dst.off_hist[oc as usize].wrapping_add(1);
                    dst.tokens[dst.n as usize] =
                        MATCH_TYPE | (xl as u32) << LENGTH_SHIFT | x_offset;
                    dst.n = dst.n.wrapping_add(1);
                }
                s += l;
                next_emit = s;
                if next_s >= s {
                    s = next_s + 1;
                }
                if s >= s_limit {
                    // Index first pair after match end.
                    if ((s + l + 8) as i64) < src.len() as i64 {
                        let cv = load_le64(src, s);
                        table[hash_len(cv, TABLE_BITS, HASH_BYTES) as usize] = s + cur;
                    }
                    break 'outer; // goto emitRemainder
                }

                // We could immediately start working at s now, but to improve
                // compression we first update the hash table at s-2 and at s. If
                // another emitCopy is not our next move, also calculate nextHash
                // at s+1. At least on GOARCH=amd64, these three hash calculations
                // are faster as one load64 call (with some shifts) instead of
                // three load32 calls.
                let mut x = load_le64(src, s - 2);
                let o = cur + s - 2;
                let prev_hash = hash_len(x, TABLE_BITS, HASH_BYTES) as usize;
                table[prev_hash] = o;
                x >>= 16;
                let curr_hash = hash_len(x, TABLE_BITS, HASH_BYTES) as usize;
                candidate = table[curr_hash];
                table[curr_hash] = o + 2;

                t = candidate - cur;
                if s - t > MAX_MATCH_OFFSET || x as u32 != load_le32(src, t) {
                    cv = x >> 8;
                    s += 1;
                    break;
                }
            }
        }

        // emitRemainder:
        if (next_emit as i64) < src.len() as i64 {
            // If nothing was added, don't encode literals.
            if dst.n == 0 {
                return;
            }
            emit_literals(dst, &src[next_emit as usize..]);
        }
    }
}
