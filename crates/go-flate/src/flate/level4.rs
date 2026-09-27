//! Port of go1.27.1 `compress/flate/level4.go`.

use super::deflatefast::{
    BASE_MATCH_OFFSET, BUFFER_RESET, FastGen, HASH_LONG_BYTES, MAX_MATCH_OFFSET, TABLE_BITS,
    TABLE_SIZE, TableEntry, hash_len, load_le32, load_le64, match_len, match_len_long,
};
use super::token::{Token, Tokens, emit_literals};

/// Level 4 uses two tables, one for short (4 bytes) and one for long (7 bytes) matches.
pub(crate) struct FastEncL4 {
    pub(crate) fg: FastGen,
    table: Vec<TableEntry>,   // [tableSize]tableEntry
    b_table: Vec<TableEntry>, // [tableSize]tableEntry
}

impl FastEncL4 {
    pub(crate) fn new(fg: FastGen) -> FastEncL4 {
        FastEncL4 {
            fg,
            table: vec![0; TABLE_SIZE],
            b_table: vec![0; TABLE_SIZE],
        }
    }

    // Go: compress/flate/level4.go:(*fastEncL4).encode
    pub(crate) fn encode(&mut self, dst: &mut Tokens, src: &[u8]) {
        const INPUT_MARGIN: i32 = 12 - 1;
        const MIN_NON_LITERAL_BLOCK_SIZE: usize = 1 + 1 + INPUT_MARGIN as usize;
        const HASH_SHORT_BYTES: u8 = 4;
        // Protect against e.cur wraparound.
        while self.fg.cur >= BUFFER_RESET {
            if self.fg.hist.is_empty() {
                self.table.fill(0);
                self.b_table.fill(0);
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
            for v in self.b_table.iter_mut() {
                if *v <= min_off {
                    *v = 0;
                } else {
                    *v = *v - self.fg.cur + MAX_MATCH_OFFSET;
                }
            }
            self.fg.cur = MAX_MATCH_OFFSET;
        }

        let mut s = self.fg.add_block(src);

        // This check isn't in the Snappy implementation, but there, the caller
        // instead of the callee handles this case.
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
        let b_table = &mut self.b_table[..];
        let mut next_emit = s;

        // sLimit is when to stop looking for offset/length copies. The inputMargin
        // lets us use a fast path for emitLiterals in the main loop, while we are
        // looking for copies.
        let s_limit = (src.len() as i32).wrapping_sub(INPUT_MARGIN);

        // nextEmit is where in src the next emitLiterals should start from.
        let mut cv = load_le64(src, s);
        'outer: loop {
            const SKIP_LOG: i32 = 6;
            const DO_EVERY: i32 = 1;

            let mut next_s = s;
            let mut t: i32;
            loop {
                let next_hash_s = hash_len(cv, TABLE_BITS, HASH_SHORT_BYTES) as usize;
                let next_hash_l = hash_len(cv, TABLE_BITS, HASH_LONG_BYTES) as usize;

                s = next_s;
                next_s = s + DO_EVERY + ((s - next_emit) >> SKIP_LOG);
                if next_s > s_limit {
                    break 'outer; // goto emitRemainder
                }
                // Fetch a short+long candidate
                let s_candidate = table[next_hash_s];
                let mut l_candidate = b_table[next_hash_l];
                let next = load_le64(src, next_s);
                let entry = s + cur;
                table[next_hash_s] = entry;
                b_table[next_hash_l] = entry;

                t = l_candidate - cur;
                if s - t < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, t) {
                    // We got a long match. Use that.
                    break;
                }

                t = s_candidate - cur;
                if s - t < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, t) {
                    // Found a 4 match...
                    l_candidate = b_table[hash_len(next, TABLE_BITS, HASH_LONG_BYTES) as usize];

                    // If the next long is a candidate, check if we should use that instead...
                    let l_off = l_candidate - cur;
                    if next_s - l_off < MAX_MATCH_OFFSET && load_le32(src, l_off) == next as u32 {
                        let (l1, l2) = (
                            match_len(&src[(s + 4) as usize..], &src[(t + 4) as usize..]),
                            match_len(
                                &src[(next_s + 4) as usize..],
                                &src[(next_s - l_off + 4) as usize..],
                            ),
                        );
                        if l2 > l1 {
                            s = next_s;
                            t = l_candidate - cur;
                        }
                    }
                    break;
                }
                cv = next;
            }

            // A 4-byte match has been found. We'll later see if more than 4 bytes
            // match. But, prior to the match, src[nextEmit:s] are unmatched. Emit
            // them as literal bytes.

            // Extend the 4-byte match as long as possible.
            let mut l = match_len_long((s + 4) as usize, (t + 4) as usize, src) + 4;

            // Extend backwards
            while t > 0 && s > next_emit && src[(t - 1) as usize] == src[(s - 1) as usize] {
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

            dst.add_match_long(l, (s - t - BASE_MATCH_OFFSET) as u32);
            s += l;
            next_emit = s;
            if next_s >= s {
                s = next_s + 1;
            }

            if s >= s_limit {
                // Index first pair after match end.
                if ((s + 8) as i64) < src.len() as i64 {
                    let cv = load_le64(src, s);
                    table[hash_len(cv, TABLE_BITS, HASH_SHORT_BYTES) as usize] = s + cur;
                    b_table[hash_len(cv, TABLE_BITS, HASH_LONG_BYTES) as usize] = s + cur;
                }
                break 'outer; // goto emitRemainder
            }

            // Store every 3rd hash in-between
            let mut i = next_s;
            if i < s - 1 {
                let cv = load_le64(src, i);
                let t = i + cur;
                let t2 = t + 1;
                b_table[hash_len(cv, TABLE_BITS, HASH_LONG_BYTES) as usize] = t;
                b_table[hash_len(cv >> 8, TABLE_BITS, HASH_LONG_BYTES) as usize] = t2;
                table[hash_len(cv >> 8, TABLE_BITS, HASH_SHORT_BYTES) as usize] = t2;

                i += 3;
                while i < s - 1 {
                    let cv = load_le64(src, i);
                    let t = i + cur;
                    let t2 = t + 1;
                    b_table[hash_len(cv, TABLE_BITS, HASH_LONG_BYTES) as usize] = t;
                    b_table[hash_len(cv >> 8, TABLE_BITS, HASH_LONG_BYTES) as usize] = t2;
                    table[hash_len(cv >> 8, TABLE_BITS, HASH_SHORT_BYTES) as usize] = t2;
                    i += 3;
                }
            }

            // We could immediately start working at s now, but to improve
            // compression we first update the hash table at s-1 and at s.
            let x = load_le64(src, s - 1);
            let o = cur + s - 1;
            let prev_hash_s = hash_len(x, TABLE_BITS, HASH_SHORT_BYTES) as usize;
            let prev_hash_l = hash_len(x, TABLE_BITS, HASH_LONG_BYTES) as usize;
            table[prev_hash_s] = o;
            b_table[prev_hash_l] = o;
            cv = x >> 8;
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
