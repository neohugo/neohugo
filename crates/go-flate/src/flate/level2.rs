//! Port of go1.27.1 `compress/flate/level2.go`.

use super::deflatefast::{
    BASE_MATCH_OFFSET, BUFFER_RESET, FastGen, MAX_MATCH_OFFSET, TableEntry, hash_len, load_le32,
    load_le64, match_len_long,
};
use super::token::{Token, Tokens, emit_literals};

const L2_TABLE_BITS: u8 = 17; // Bits used in level 2 table
const L2_TABLE_SIZE: usize = 1 << L2_TABLE_BITS; // Size of the level 2 table

/// Level 2 uses a similar algorithm to level 1, but with a larger table.
pub(crate) struct FastEncL2 {
    pub(crate) fg: FastGen,
    table: Vec<TableEntry>, // [l2TableSize]tableEntry
}

impl FastEncL2 {
    pub(crate) fn new(fg: FastGen) -> FastEncL2 {
        FastEncL2 {
            fg,
            table: vec![0; L2_TABLE_SIZE],
        }
    }

    // Go: compress/flate/level2.go:(*fastEncL2).encode
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
            // When should we start skipping if we haven't found matches in a long while.
            const SKIP_LOG: i32 = 5;
            const DO_EVERY: i32 = 2;

            let mut next_s = s;
            let mut candidate: TableEntry;
            loop {
                let mut next_hash = hash_len(cv, L2_TABLE_BITS, HASH_BYTES) as usize;
                s = next_s;
                next_s = s + DO_EVERY + ((s - next_emit) >> SKIP_LOG);
                if next_s > s_limit {
                    break 'outer; // goto emitRemainder
                }
                candidate = table[next_hash];
                let mut now = load_le64(src, next_s);
                table[next_hash] = s + cur;
                next_hash = hash_len(now, L2_TABLE_BITS, HASH_BYTES) as usize;

                let mut offset = s - (candidate - cur);
                if offset < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, candidate - cur) {
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

                offset = s - (candidate - cur);
                if offset < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, candidate - cur) {
                    break;
                }
                cv = now;
            }

            // A 4-byte match has been found. We'll later see if more than 4 bytes match.
            loop {
                // Extend the 4-byte match as long as possible.
                let mut t = candidate - cur;
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
                    if ((s + l + 8) as i64) < src.len() as i64 {
                        let cv = load_le64(src, s);
                        table[hash_len(cv, L2_TABLE_BITS, HASH_BYTES) as usize] = s + cur;
                    }
                    break 'outer; // goto emitRemainder
                }

                // Store every second hash in-between, but offset by 1.
                let mut i = s - l + 2;
                while i < s - 5 {
                    let mut x = load_le64(src, i);
                    let mut next_hash = hash_len(x, L2_TABLE_BITS, HASH_BYTES) as usize;
                    table[next_hash] = cur + i;
                    // Skip one
                    x >>= 16;
                    next_hash = hash_len(x, L2_TABLE_BITS, HASH_BYTES) as usize;
                    table[next_hash] = cur + i + 2;
                    // Skip one
                    x >>= 16;
                    next_hash = hash_len(x, L2_TABLE_BITS, HASH_BYTES) as usize;
                    table[next_hash] = cur + i + 4;
                    i += 7;
                }

                // We could immediately start working at s now, but to improve
                // compression we first update the hash table at s-2 to s. If
                // another emitCopy is not our next move, also calculate nextHash
                // at s+1.
                let x = load_le64(src, s - 2);
                let o = cur + s - 2;
                let prev_hash = hash_len(x, L2_TABLE_BITS, HASH_BYTES) as usize;
                let prev_hash2 = hash_len(x >> 8, L2_TABLE_BITS, HASH_BYTES) as usize;
                table[prev_hash] = o;
                table[prev_hash2] = o + 1;
                let curr_hash = hash_len(x >> 16, L2_TABLE_BITS, HASH_BYTES) as usize;
                candidate = table[curr_hash];
                table[curr_hash] = o + 2;

                let offset = s - (candidate - cur);
                if offset > MAX_MATCH_OFFSET || (x >> 16) as u32 != load_le32(src, candidate - cur)
                {
                    cv = x >> 24;
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
