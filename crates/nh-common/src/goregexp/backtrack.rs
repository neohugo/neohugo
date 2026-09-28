//! Port of Go's `regexp/backtrack.go` (go1.27.1): the bit-state backtracker for small
//! programs on short inputs.
//!
//! backtrack is a regular expression search with submatch tracking for small regular
//! expressions and texts. It allocates a bit vector with (length of input) * (length of
//! prog) bits, to make sure it never explores the same (character position, instruction)
//! state multiple times. This limits the search to run in time linear in the length of the
//! test.

use super::syntax::{EMPTY_BEGIN_TEXT, EmptyOp, InstOp, Prog};
use super::{END_OF_TEXT, Inner, Input, Match};
use go_unicode::Rune;

/// A job is an entry on the backtracker's job stack. It holds the instruction pc and the
/// position in the input.
#[derive(Clone, Copy)]
struct Job {
    pc: u32,
    arg: bool,
    pos: usize,
}

const VISITED_BITS: usize = 32;
/// len(prog.Inst) <= max
const MAX_BACKTRACK_PROG: usize = 500;
/// bit vector size <= max (bits)
const MAX_BACKTRACK_VECTOR: usize = 256 * 1024;

/// bitState holds state for the backtracker.
struct BitState {
    end: usize,
    cap: Vec<isize>,
    matchcap: Vec<isize>,
    jobs: Vec<Job>,
    visited: Vec<u32>,
}

/// maxBitStateLen returns the maximum length of a string to search with the backtracker using
/// prog.
// Go: regexp/backtrack.go:maxBitStateLen
pub(crate) fn max_bit_state_len(prog: &Prog) -> usize {
    if !should_backtrack(prog) {
        return 0;
    }
    MAX_BACKTRACK_VECTOR / prog.inst.len()
}

/// shouldBacktrack reports whether the program is too long for the backtracker to run.
// Go: regexp/backtrack.go:shouldBacktrack
fn should_backtrack(prog: &Prog) -> bool {
    prog.inst.len() <= MAX_BACKTRACK_PROG
}

impl BitState {
    /// reset resets the state of the backtracker. end is the end position in the input. ncap
    /// is the number of captures.
    // Go: regexp/backtrack.go:reset
    fn new(prog: &Prog, end: usize, ncap: usize) -> BitState {
        let visited_size = (prog.inst.len() * (end + 1)).div_ceil(VISITED_BITS);
        BitState {
            end,
            jobs: Vec::with_capacity(256),
            visited: vec![0; visited_size],
            cap: vec![-1; ncap],
            matchcap: vec![-1; ncap],
        }
    }

    /// shouldVisit reports whether the combination of (pc, pos) has not been visited yet.
    // Go: regexp/backtrack.go:shouldVisit
    fn should_visit(&mut self, pc: u32, pos: usize) -> bool {
        let n = pc as usize * (self.end + 1) + pos;
        if self.visited[n / VISITED_BITS] & (1 << (n & (VISITED_BITS - 1))) != 0 {
            return false;
        }
        self.visited[n / VISITED_BITS] |= 1 << (n & (VISITED_BITS - 1));
        true
    }

    /// push pushes (pc, pos, arg) onto the job stack if it should be visited.
    // Go: regexp/backtrack.go:push
    fn push(&mut self, re: &Inner, pc: u32, pos: usize, arg: bool) {
        // Only check shouldVisit when arg is false. When arg is true, we are continuing a
        // previous visit.
        if re.prog.inst[pc as usize].op != InstOp::Fail && (arg || self.should_visit(pc, pos)) {
            self.jobs.push(Job { pc, arg, pos });
        }
    }
}

impl Inner {
    /// tryBacktrack runs a backtracking search starting at pos.
    // Go: regexp/backtrack.go:tryBacktrack
    fn try_backtrack(&self, b: &mut BitState, i: Input<'_>, pc: u32, pos: usize) -> bool {
        let longest = self.longest;

        b.push(self, pc, pos, false);
        while let Some(job) = b.jobs.pop() {
            let (mut pc, mut pos, mut arg) = (job.pc, job.pos, job.arg);

            // Optimization: rather than push and pop, code that is going to Push and continue
            // the loop simply updates ip, p, and arg and jumps to CheckAndLoop. We have to do
            // the ShouldVisit check that Push would have, but we avoid the stack manipulation.
            let mut skip = true;
            loop {
                if !skip && !b.should_visit(pc, pos) {
                    break;
                }
                skip = false;

                let inst = &self.prog.inst[pc as usize];

                match inst.op {
                    InstOp::Fail => panic!("unexpected InstFail"),
                    InstOp::Alt => {
                        // Cannot just
                        //   b.push(inst.Out, pos, false)
                        //   b.push(inst.Arg, pos, false)
                        // If during the processing of inst.Out, we encounter inst.Arg via
                        // another path, we want to process it then. Pushing it here will
                        // inhibit that. Instead, re-push inst with arg==true as a reminder to
                        // push inst.Arg out later.
                        if arg {
                            // Finished inst.Out; try inst.Arg.
                            arg = false;
                            pc = inst.arg;
                            continue;
                        } else {
                            b.push(self, pc, pos, true);
                            pc = inst.out;
                            continue;
                        }
                    }
                    InstOp::AltMatch => {
                        // One opcode consumes runes; the other leads to match.
                        match self.prog.inst[inst.out as usize].op {
                            InstOp::Rune
                            | InstOp::Rune1
                            | InstOp::RuneAny
                            | InstOp::RuneAnyNotNL => {
                                // inst.Arg is the match.
                                b.push(self, inst.arg, pos, false);
                                pc = inst.arg;
                                pos = b.end;
                                continue;
                            }
                            _ => {}
                        }
                        // inst.Out is the match - non-greedy
                        b.push(self, inst.out, b.end, false);
                        pc = inst.out;
                        continue;
                    }
                    InstOp::Rune => {
                        let (r, width) = i.step(pos);
                        if !inst.match_rune(r) {
                            break;
                        }
                        pos += width;
                        pc = inst.out;
                        continue;
                    }
                    InstOp::Rune1 => {
                        let (r, width) = i.step(pos);
                        if r != inst.rune[0] {
                            break;
                        }
                        pos += width;
                        pc = inst.out;
                        continue;
                    }
                    InstOp::RuneAnyNotNL => {
                        let (r, width) = i.step(pos);
                        if r == '\n' as Rune || r == END_OF_TEXT {
                            break;
                        }
                        pos += width;
                        pc = inst.out;
                        continue;
                    }
                    InstOp::RuneAny => {
                        let (r, width) = i.step(pos);
                        if r == END_OF_TEXT {
                            break;
                        }
                        pos += width;
                        pc = inst.out;
                        continue;
                    }
                    InstOp::Capture => {
                        if arg {
                            // Finished inst.Out; restore the old value.
                            b.cap[inst.arg as usize] = pos as isize;
                            break;
                        } else {
                            if (inst.arg as usize) < b.cap.len() {
                                // Capture pos to register, but save old value.
                                let old = b.cap[inst.arg as usize];
                                // come back when we're done. (Go pushes the old value as the
                                // job's position; -1 wraps to the maximum int there too.)
                                b.push(self, pc, old as usize, true);
                                b.cap[inst.arg as usize] = pos as isize;
                            }
                            pc = inst.out;
                            continue;
                        }
                    }
                    InstOp::EmptyWidth => {
                        let flag = i.context(pos);
                        if !flag.match_(inst.arg as EmptyOp) {
                            break;
                        }
                        pc = inst.out;
                        continue;
                    }
                    InstOp::Nop => {
                        pc = inst.out;
                        continue;
                    }
                    InstOp::Match => {
                        // We found a match. If the caller doesn't care where the match is, no
                        // point going further.
                        if b.cap.is_empty() {
                            return true;
                        }

                        // Record best match so far. Only need to check end point, because
                        // this entire call is only considering one start position.
                        if b.cap.len() > 1 {
                            b.cap[1] = pos as isize;
                        }
                        let old = b.matchcap[1];
                        if old == -1 || (longest && pos > 0 && pos as isize > old) {
                            let cap = b.cap.clone();
                            b.matchcap.copy_from_slice(&cap);
                        }

                        // If going for first match, we're done.
                        if !longest {
                            return true;
                        }

                        // If we used the entire text, no longer match is possible.
                        if pos == b.end {
                            return true;
                        }

                        // Otherwise, continue on in hope of a longer match.
                        break;
                    }
                }
            }
        }

        longest && b.matchcap.len() > 1 && b.matchcap[1] >= 0
    }

    /// backtrack runs a backtracking search of prog on the input starting at pos.
    // Go: regexp/backtrack.go:backtrack
    pub(crate) fn backtrack(&self, i: Input<'_>, mut pos: usize, ncap: usize) -> Option<Match> {
        let start_cond = self.cond;
        if start_cond == !0 {
            // impossible
            return None;
        }
        if start_cond & EMPTY_BEGIN_TEXT != 0 && pos != 0 {
            // Anchored match, past beginning of text.
            return None;
        }

        let end = i.s.len();
        let mut b = BitState::new(&self.prog, end, ncap);

        // Anchored search must start at the beginning of the input
        if start_cond & EMPTY_BEGIN_TEXT != 0 {
            if !b.cap.is_empty() {
                b.cap[0] = pos as isize;
            }
            if !self.try_backtrack(&mut b, i, self.prog.start as u32, pos) {
                return None;
            }
        } else {
            // Unanchored search, starting from each possible text position. Notice that we
            // have to try the empty string at the end of the text, so the loop condition is
            // pos <= end, not pos < end. This looks like it's quadratic in the size of the
            // text, but we are not clearing visited between calls to TrySearch, so no work is
            // duplicated and it ends up still being linear.
            let mut width: isize = -1;
            let mut found = false;
            while pos <= end && width != 0 {
                if !self.prefix.is_empty() {
                    // Match requires literal prefix; fast search for it.
                    let advance = i.index(self, pos);
                    if advance < 0 {
                        return None;
                    }
                    pos += advance as usize;
                }

                if !b.cap.is_empty() {
                    b.cap[0] = pos as isize;
                }
                if self.try_backtrack(&mut b, i, self.prog.start as u32, pos) {
                    // Match must be leftmost; done.
                    found = true;
                    break;
                }
                width = i.step(pos).1 as isize;
                pos += width as usize;
            }
            if !found {
                return None;
            }
        }

        Some(b.matchcap)
    }
}
