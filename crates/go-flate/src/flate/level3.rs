//! Port of go1.27.1 `compress/flate/level3.go`.

use super::deflatefast::{
    BASE_MATCH_OFFSET, BUFFER_RESET, FastGen, MAX_MATCH_OFFSET, TableEntry, TableEntryPrev,
    hash_len, load_le32, load_le64, match_len, match_len_long,
};
use super::token::{Token, Tokens, emit_literals};

const L3_TABLE_BITS: u8 = 16; // Bits used in level 3 table
const L3_TABLE_SIZE: usize = 1 << L3_TABLE_BITS; // Size of the level 3 table

/// Level 3 uses a similar algorithm to level 2, with a smaller table,
/// but will check up two candidates for each iteration with more
/// entries added to the table.
pub(crate) struct FastEncL3 {
    pub(crate) fg: FastGen,
    table: Vec<TableEntryPrev>, // [l3TableSize]tableEntryPrev
}

impl FastEncL3 {
    pub(crate) fn new(fg: FastGen) -> FastEncL3 {
        FastEncL3 {
            fg,
            table: vec![TableEntryPrev::default(); L3_TABLE_SIZE],
        }
    }

    // Go: compress/flate/level3.go:(*fastEncL3).encode
    pub(crate) fn encode(&mut self, dst: &mut Tokens, src: &[u8]) {
        const INPUT_MARGIN: i32 = 12 - 1;
        const MIN_NON_LITERAL_BLOCK_SIZE: usize = 1 + 1 + INPUT_MARGIN as usize;
        const HASH_BYTES: u8 = 5;

        // Protect against e.cur wraparound.
        while self.fg.cur >= BUFFER_RESET {
            if self.fg.hist.is_empty() {
                self.table.fill(TableEntryPrev::default());
                self.fg.cur = MAX_MATCH_OFFSET;
                break;
            }
            // Shift down everything in the table that isn't already too far away.
            let min_off = self.fg.cur + self.fg.hist.len() as i32 - MAX_MATCH_OFFSET;
            for v in self.table.iter_mut() {
                if v.cur <= min_off {
                    v.cur = 0;
                } else {
                    v.cur = v.cur - self.fg.cur + MAX_MATCH_OFFSET;
                }
                if v.prev <= min_off {
                    v.prev = 0;
                } else {
                    v.prev = v.prev - self.fg.cur + MAX_MATCH_OFFSET;
                }
            }
            self.fg.cur = MAX_MATCH_OFFSET;
        }

        let mut s = self.fg.add_block(src);

        // Skip if too small.
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
        let mut next_emit = s;

        // sLimit is when to stop looking for offset/length copies. The inputMargin
        // lets us use a fast path for emitLiterals in the main loop, while we are
        // looking for copies.
        let s_limit = (src.len() as i32).wrapping_sub(INPUT_MARGIN);

        // nextEmit is where in src the next emitLiterals should start from.
        let mut cv = load_le64(src, s);
        'outer: loop {
            const SKIP_LOG: i32 = 7;
            let mut next_s = s;
            let mut candidate: TableEntry;
            loop {
                let next_hash = hash_len(cv, L3_TABLE_BITS, HASH_BYTES) as usize;
                s = next_s;
                next_s = s + 1 + ((s - next_emit) >> SKIP_LOG);
                if next_s > s_limit {
                    break 'outer; // goto emitRemainder
                }
                let candidates = table[next_hash];
                let now = load_le64(src, next_s);

                // Safe offset distance until s + 4...
                let min_offset = cur + s - (MAX_MATCH_OFFSET - 4);
                table[next_hash] = TableEntryPrev {
                    prev: candidates.cur,
                    cur: s + cur,
                };

                // Check both candidates
                candidate = candidates.cur;
                if candidate < min_offset {
                    cv = now;
                    // Previous will also be invalid, we have nothing.
                    continue;
                }

                if cv as u32 == load_le32(src, candidate - cur) {
                    if candidates.prev < min_offset
                        || cv as u32 != load_le32(src, candidates.prev - cur)
                    {
                        break;
                    }
                    // Both match and are valid, pick longest.
                    let offset = s - (candidate - cur);
                    let o2 = s - (candidates.prev - cur);
                    let (l1, l2) = (
                        match_len(&src[(s + 4) as usize..], &src[(s - offset + 4) as usize..]),
                        match_len(&src[(s + 4) as usize..], &src[(s - o2 + 4) as usize..]),
                    );
                    if l2 > l1 {
                        candidate = candidates.prev;
                    }
                    break;
                } else {
                    // We only check if value mismatches.
                    // Offset will always be invalid in other cases.
                    candidate = candidates.prev;
                    if candidate > min_offset && cv as u32 == load_le32(src, candidate - cur) {
                        break;
                    }
                }
                cv = now;
            }

            loop {
                // Extend the 4-byte match as long as possible.
                //
                let mut t = candidate - cur;
                let mut l = match_len_long((s + 4) as usize, (t + 4) as usize, src) + 4;

                // Extend backwards
                while t > 0 && s > next_emit && src[(t - 1) as usize] == src[(s - 1) as usize] {
                    s -= 1;
                    t -= 1;
                    l += 1;
                }
                // Emit literals.
                if next_emit < s {
                    for &v in &src[next_emit as usize..s as usize] {
                        dst.tokens[dst.n as usize] = v as Token;
                        dst.lit_hist[v as usize] = dst.lit_hist[v as usize].wrapping_add(1);
                        dst.n = dst.n.wrapping_add(1);
                    }
                }

                // Emit match.
                dst.add_match_long(l, (s - t - BASE_MATCH_OFFSET) as u32);
                s += l;
                next_emit = s;
                if next_s >= s {
                    s = next_s + 1;
                }

                if s >= s_limit {
                    t += l;
                    // Index first pair after match end.
                    if ((t + 8) as i64) < src.len() as i64 && t > 0 {
                        cv = load_le64(src, t);
                        let next_hash = hash_len(cv, L3_TABLE_BITS, HASH_BYTES) as usize;
                        table[next_hash] = TableEntryPrev {
                            prev: table[next_hash].cur,
                            cur: cur + t,
                        };
                    }
                    break 'outer; // goto emitRemainder
                }

                // Store every 5th hash in-between.
                let mut i = s - l + 2;
                while i < s - 5 {
                    let next_hash = hash_len(load_le64(src, i), L3_TABLE_BITS, HASH_BYTES) as usize;
                    table[next_hash] = TableEntryPrev {
                        prev: table[next_hash].cur,
                        cur: cur + i,
                    };
                    i += 6;
                }
                // We could immediately start working at s now, but to improve
                // compression we first update the hash table at s-2 to s.
                let mut x = load_le64(src, s - 2);
                let mut prev_hash = hash_len(x, L3_TABLE_BITS, HASH_BYTES) as usize;

                table[prev_hash] = TableEntryPrev {
                    prev: table[prev_hash].cur,
                    cur: cur + s - 2,
                };
                x >>= 8;
                prev_hash = hash_len(x, L3_TABLE_BITS, HASH_BYTES) as usize;

                table[prev_hash] = TableEntryPrev {
                    prev: table[prev_hash].cur,
                    cur: cur + s - 1,
                };
                x >>= 8;
                let curr_hash = hash_len(x, L3_TABLE_BITS, HASH_BYTES) as usize;
                let candidates = table[curr_hash];
                cv = x;
                table[curr_hash] = TableEntryPrev {
                    prev: candidates.cur,
                    cur: s + cur,
                };

                // Check both candidates
                candidate = candidates.cur;
                let min_offset = cur + s - (MAX_MATCH_OFFSET - 4);

                if candidate > min_offset {
                    if cv as u32 == load_le32(src, candidate - cur) {
                        // Found a match...
                        continue;
                    }
                    candidate = candidates.prev;
                    if candidate > min_offset && cv as u32 == load_le32(src, candidate - cur) {
                        // Match at prev...
                        continue;
                    }
                }
                cv = x >> 8;
                s += 1;
                break;
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
