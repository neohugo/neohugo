//! Port of go1.27.1 `compress/flate/level5.go`.

use super::MAX_MATCH_LENGTH;
use super::deflatefast::{
    BASE_MATCH_OFFSET, BUFFER_RESET, FastGen, HASH_LONG_BYTES, MAX_MATCH_OFFSET, TABLE_BITS,
    TABLE_SIZE, TableEntry, TableEntryPrev, hash_len, load_le32, load_le64, match_len_limited,
    match_len_long,
};
use super::token::{Token, Tokens, emit_literals};

/// Level 5 is similar to level 4, but for long matches two candidates are tested.
/// Once a match is found, when it stops it will attempt to find a match that extends further.
pub(crate) struct FastEncL5 {
    pub(crate) fg: FastGen,
    table: Vec<TableEntry>,       // [tableSize]tableEntry
    b_table: Vec<TableEntryPrev>, // [tableSize]tableEntryPrev
}

impl FastEncL5 {
    pub(crate) fn new(fg: FastGen) -> FastEncL5 {
        FastEncL5 {
            fg,
            table: vec![0; TABLE_SIZE],
            b_table: vec![TableEntryPrev::default(); TABLE_SIZE],
        }
    }

    // Go: compress/flate/level5.go:(*fastEncL5).encode
    pub(crate) fn encode(&mut self, dst: &mut Tokens, src: &[u8]) {
        const INPUT_MARGIN: i32 = 12 - 1;
        const MIN_NON_LITERAL_BLOCK_SIZE: usize = 1 + 1 + INPUT_MARGIN as usize;
        const HASH_SHORT_BYTES: u8 = 4;

        // Protect against e.cur wraparound.
        while self.fg.cur >= BUFFER_RESET {
            if self.fg.hist.is_empty() {
                self.table.fill(0);
                self.b_table.fill(TableEntryPrev::default());
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
                if v.cur <= min_off {
                    v.cur = 0;
                    v.prev = 0;
                } else {
                    v.cur = v.cur - self.fg.cur + MAX_MATCH_OFFSET;
                    if v.prev <= min_off {
                        v.prev = 0;
                    } else {
                        v.prev = v.prev - self.fg.cur + MAX_MATCH_OFFSET;
                    }
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

        // nextEmit is where in src the next emitLiterals should start from.
        let mut next_emit = s;

        // sLimit is when to stop looking for offset/length copies. The inputMargin
        // lets us use a fast path for emitLiterals in the main loop, while we are
        // looking for copies.
        let s_limit = (src.len() as i32).wrapping_sub(INPUT_MARGIN);

        let mut cv = load_le64(src, s);
        'outer: loop {
            const SKIP_LOG: i32 = 6;
            const DO_EVERY: i32 = 1;

            let mut next_s = s;
            let mut l: i32 = 0;
            let mut t: i32;
            loop {
                let mut next_hash_s = hash_len(cv, TABLE_BITS, HASH_SHORT_BYTES) as usize;
                let mut next_hash_l = hash_len(cv, TABLE_BITS, HASH_LONG_BYTES) as usize;

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
                {
                    let e_long = &mut b_table[next_hash_l];
                    (e_long.cur, e_long.prev) = (entry, e_long.cur);
                }

                next_hash_s = hash_len(next, TABLE_BITS, HASH_SHORT_BYTES) as usize;
                next_hash_l = hash_len(next, TABLE_BITS, HASH_LONG_BYTES) as usize;

                t = l_candidate.cur - cur;
                if s - t < MAX_MATCH_OFFSET {
                    if cv as u32 == load_le32(src, t) {
                        // Store the next match
                        table[next_hash_s] = next_s + cur;
                        {
                            let e_long = &mut b_table[next_hash_l];
                            (e_long.cur, e_long.prev) = (next_s + cur, e_long.cur);
                        }

                        let t2 = l_candidate.prev - cur;
                        if s - t2 < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, t2) {
                            l = match_len_limited((s + 4) as usize, (t + 4) as usize, src) + 4;
                            let ml1 =
                                match_len_limited((s + 4) as usize, (t2 + 4) as usize, src) + 4;
                            if ml1 > l {
                                t = t2;
                                l = ml1;
                                break;
                            }
                        }
                        break;
                    }
                    t = l_candidate.prev - cur;
                    if s - t < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, t) {
                        // Store the next match
                        table[next_hash_s] = next_s + cur;
                        let e_long = &mut b_table[next_hash_l];
                        (e_long.cur, e_long.prev) = (next_s + cur, e_long.cur);
                        break;
                    }
                }

                t = s_candidate - cur;
                if s - t < MAX_MATCH_OFFSET && cv as u32 == load_le32(src, t) {
                    // Found a 4 match...
                    l = match_len_limited((s + 4) as usize, (t + 4) as usize, src) + 4;
                    l_candidate = b_table[next_hash_l];
                    // Store the next match

                    table[next_hash_s] = next_s + cur;
                    {
                        let e_long = &mut b_table[next_hash_l];
                        (e_long.cur, e_long.prev) = (next_s + cur, e_long.cur);
                    }

                    // If the next long is a candidate, use that...
                    let mut t2 = l_candidate.cur - cur;
                    if next_s - t2 < MAX_MATCH_OFFSET {
                        if load_le32(src, t2) == next as u32 {
                            let ml =
                                match_len_limited((next_s + 4) as usize, (t2 + 4) as usize, src)
                                    + 4;
                            if ml > l {
                                t = t2;
                                s = next_s;
                                l = ml;
                                break;
                            }
                        }
                        // If the previous long is a candidate, use that...
                        t2 = l_candidate.prev - cur;
                        if next_s - t2 < MAX_MATCH_OFFSET && load_le32(src, t2) == next as u32 {
                            let ml =
                                match_len_limited((next_s + 4) as usize, (t2 + 4) as usize, src)
                                    + 4;
                            if ml > l {
                                t = t2;
                                s = next_s;
                                l = ml;
                                break;
                            }
                        }
                    }
                    break;
                }
                cv = next;
            }

            if l == 0 {
                // Extend the 4-byte match as long as possible.
                l = match_len_long((s + 4) as usize, (t + 4) as usize, src) + 4;
            } else if l == MAX_MATCH_LENGTH {
                l += match_len_long((s + l) as usize, (t + l) as usize, src);
            }

            // Try to locate a better match by checking the end of best match...
            let s_at = s + l;
            if l < 30 && s_at < s_limit {
                // Allow some bytes at the beginning to mismatch.
                // Sweet spot is 2/3 bytes depending on input.
                // 3 is only a little better when it is but sometimes a lot worse.
                // The skipped bytes are tested in Extend backwards,
                // and still picked up as part of the match if they do.
                const SKIP_BEGINNING: i32 = 2;
                let e_long = b_table
                    [hash_len(load_le64(src, s_at), TABLE_BITS, HASH_LONG_BYTES) as usize]
                    .cur;
                let t2 = e_long - cur - l + SKIP_BEGINNING;
                let s2 = s + SKIP_BEGINNING;
                let off = s2 - t2;
                if t2 >= 0 && off < MAX_MATCH_OFFSET && off > 0 {
                    let l2 = match_len_long(s2 as usize, t2 as usize, src);
                    if l2 > l {
                        t = t2;
                        l = l2;
                        s = s2;
                    }
                }
            }

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
                break 'outer; // goto emitRemainder
            }

            // Store every 3rd hash in-between.
            const HASH_EVERY: i32 = 3;
            let mut i = s - l + 1;
            if i < s - 1 {
                let mut cv = load_le64(src, i);
                let mut t = i + cur;
                table[hash_len(cv, TABLE_BITS, HASH_SHORT_BYTES) as usize] = t;
                {
                    let e_long = &mut b_table[hash_len(cv, TABLE_BITS, HASH_LONG_BYTES) as usize];
                    (e_long.cur, e_long.prev) = (t, e_long.cur);
                }

                // Do an long at i+1
                cv >>= 8;
                t += 1;
                {
                    let e_long = &mut b_table[hash_len(cv, TABLE_BITS, HASH_LONG_BYTES) as usize];
                    (e_long.cur, e_long.prev) = (t, e_long.cur);
                }

                // We only have enough bits for a short entry at i+2
                cv >>= 8;
                t += 1;
                table[hash_len(cv, TABLE_BITS, HASH_SHORT_BYTES) as usize] = t;

                // Skip one - otherwise we risk hitting 's'
                i += 4;
                while i < s - 1 {
                    let cv = load_le64(src, i);
                    let t = i + cur;
                    let t2 = t + 1;
                    {
                        let e_long =
                            &mut b_table[hash_len(cv, TABLE_BITS, HASH_LONG_BYTES) as usize];
                        (e_long.cur, e_long.prev) = (t, e_long.cur);
                    }
                    table[hash_len(cv >> 8, TABLE_BITS, HASH_SHORT_BYTES) as usize] = t2;
                    i += HASH_EVERY;
                }
            }

            // We could immediately start working at s now, but to improve
            // compression we first update the hash table at s-1 and at s.
            let x = load_le64(src, s - 1);
            let o = cur + s - 1;
            let prev_hash_s = hash_len(x, TABLE_BITS, HASH_SHORT_BYTES) as usize;
            let prev_hash_l = hash_len(x, TABLE_BITS, HASH_LONG_BYTES) as usize;
            table[prev_hash_s] = o;
            {
                let e_long = &mut b_table[prev_hash_l];
                (e_long.cur, e_long.prev) = (o, e_long.cur);
            }
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
