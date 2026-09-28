//! Port of Go's `regexp/exec.go` (go1.27.1): the NFA (Pike VM), the one-pass executor and
//! `find`, which picks the engine.

use go_unicode::Rune;

use super::syntax::{
    EMPTY_BEGIN_LINE, EMPTY_BEGIN_TEXT, EMPTY_END_LINE, EMPTY_END_TEXT, EMPTY_NO_WORD_BOUNDARY,
    EMPTY_WORD_BOUNDARY, EmptyOp, InstOp, Prog, is_word_char,
};
use super::{END_OF_TEXT, Inner, Input, Match};

/// A queue is a 'sparse array' holding pending threads of execution. See
/// https://research.swtch.com/2008/03/using-uninitialized-memory-for-fun-and.html
struct Queue {
    sparse: Vec<u32>,
    dense: Vec<Entry>,
}

impl Queue {
    fn new(n: usize) -> Queue {
        Queue {
            sparse: vec![0; n],
            dense: Vec::with_capacity(n),
        }
    }
}

/// An entry is an entry on a queue. It holds both the instruction pc and the actual thread.
/// Some queue entries are just place holders so that the machine knows it has considered that
/// pc. Such entries have t == nil.
#[derive(Clone, Copy)]
struct Entry {
    pc: u32,
    /// index into `Machine::threads`
    t: Option<usize>,
}

/// A thread is the state of a single path through the machine: an instruction and a
/// corresponding capture array.
struct Thread {
    /// pc of the instruction
    inst: u32,
    cap: Vec<isize>,
}

/// A machine holds all the state during an NFA simulation for p.
struct Machine<'r> {
    /// corresponding Regexp
    re: &'r Inner,
    /// compiled program
    p: &'r Prog,
    /// all threads ever allocated (Go allocates them on the heap)
    threads: Vec<Thread>,
    /// pool of available threads
    pool: Vec<usize>,
    /// whether a match was found
    matched: bool,
    /// capture information for the match
    matchcap: Vec<isize>,
    /// `len(m.matchcap)` (the capture array length of every thread)
    ncap: usize,
}

/// Go: `lazyFlag`, a lazily computed EmptyOp value: the runes before and after the position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LazyFlag {
    r1: Rune,
    r2: Rune,
}

impl LazyFlag {
    // Go: regexp/exec.go:newLazyFlag
    pub(crate) fn new(r1: Rune, r2: Rune) -> LazyFlag {
        LazyFlag { r1, r2 }
    }

    // Go: regexp/exec.go:match
    pub(crate) fn match_(self, mut op: EmptyOp) -> bool {
        if op == 0 {
            return true;
        }
        let r1 = self.r1;
        if op & EMPTY_BEGIN_LINE != 0 {
            if r1 != '\n' as Rune && r1 >= 0 {
                return false;
            }
            op &= !EMPTY_BEGIN_LINE;
        }
        if op & EMPTY_BEGIN_TEXT != 0 {
            if r1 >= 0 {
                return false;
            }
            op &= !EMPTY_BEGIN_TEXT;
        }
        if op == 0 {
            return true;
        }
        let r2 = self.r2;
        if op & EMPTY_END_LINE != 0 {
            if r2 != '\n' as Rune && r2 >= 0 {
                return false;
            }
            op &= !EMPTY_END_LINE;
        }
        if op & EMPTY_END_TEXT != 0 {
            if r2 >= 0 {
                return false;
            }
            op &= !EMPTY_END_TEXT;
        }
        if op == 0 {
            return true;
        }
        if is_word_char(r1) != is_word_char(r2) {
            op &= !EMPTY_WORD_BOUNDARY;
        } else {
            op &= !EMPTY_NO_WORD_BOUNDARY;
        }
        op == 0
    }
}

impl<'r> Machine<'r> {
    // Go: regexp/regexp.go:get + regexp/exec.go:init
    fn new(re: &'r Inner, ncap: usize) -> Machine<'r> {
        Machine {
            re,
            p: &re.prog,
            threads: Vec::new(),
            pool: Vec::new(),
            matched: false,
            matchcap: vec![0; ncap],
            ncap,
        }
    }

    /// alloc allocates a new thread with the given instruction. It uses the free pool if
    /// possible.
    // Go: regexp/exec.go:alloc
    fn alloc(&mut self, inst: u32) -> usize {
        let t = match self.pool.pop() {
            Some(t) => t,
            None => {
                self.threads.push(Thread {
                    inst: 0,
                    cap: vec![0; self.ncap],
                });
                self.threads.len() - 1
            }
        };
        self.threads[t].inst = inst;
        t
    }

    /// match runs the machine over the input starting at pos. It reports whether a match was
    /// found. If so, m.matchcap holds the submatch information.
    // Go: regexp/exec.go:match
    fn match_(&mut self, i: Input<'_>, mut pos: usize) -> bool {
        let start_cond = self.re.cond;
        if start_cond == !0 {
            // impossible
            return false;
        }
        self.matched = false;
        for c in self.matchcap.iter_mut() {
            *c = -1;
        }
        let n = self.p.inst.len();
        let mut runq = Queue::new(n);
        let mut nextq = Queue::new(n);
        let (mut r, mut r1) = (END_OF_TEXT, END_OF_TEXT);
        let (mut width, mut width1) = (0, 0);
        (r, width) = i.step(pos);
        if r != END_OF_TEXT {
            (r1, width1) = i.step(pos + width);
        }
        let mut flag = if pos == 0 {
            LazyFlag::new(-1, r)
        } else {
            i.context(pos)
        };
        loop {
            if runq.dense.is_empty() {
                if start_cond & EMPTY_BEGIN_TEXT != 0 && pos != 0 {
                    // Anchored match, past beginning of text.
                    break;
                }
                if self.matched {
                    // Have match; finished exploring alternatives.
                    break;
                }
                if !self.re.prefix.is_empty() && r1 != self.re.prefix_rune {
                    // Match requires literal prefix; fast search for it.
                    let advance = i.index(self.re, pos);
                    if advance < 0 {
                        break;
                    }
                    pos += advance as usize;
                    (r, width) = i.step(pos);
                    (r1, width1) = i.step(pos + width);
                }
            }
            if !self.matched {
                let mut cap = std::mem::take(&mut self.matchcap);
                if !cap.is_empty() {
                    cap[0] = pos as isize;
                }
                self.add(&mut runq, self.p.start as u32, pos, &mut cap, flag, None);
                self.matchcap = cap;
            }
            flag = LazyFlag::new(r, r1);
            self.step(&mut runq, &mut nextq, pos, pos + width, r, flag);
            if width == 0 {
                break;
            }
            if self.matchcap.is_empty() && self.matched {
                // Found a match and not paying attention to where it is, so any match will do.
                break;
            }
            pos += width;
            (r, width) = (r1, width1);
            if r != END_OF_TEXT {
                (r1, width1) = i.step(pos + width);
            }
            std::mem::swap(&mut runq, &mut nextq);
        }
        self.clear(&mut nextq);
        self.matched
    }

    /// clear frees all threads on the thread queue.
    // Go: regexp/exec.go:clear
    fn clear(&mut self, q: &mut Queue) {
        for d in &q.dense {
            if let Some(t) = d.t {
                self.pool.push(t);
            }
        }
        q.dense.clear();
    }

    /// step executes one step of the machine, running each of the threads on runq and
    /// appending new threads to nextq. The step processes the rune c (which may be
    /// endOfText), which starts at position pos and ends at nextPos. nextCond gives the
    /// setting for the empty-width flags after c.
    // Go: regexp/exec.go:step
    fn step(
        &mut self,
        runq: &mut Queue,
        nextq: &mut Queue,
        pos: usize,
        next_pos: usize,
        c: Rune,
        next_cond: LazyFlag,
    ) {
        let longest = self.re.longest;
        let prog = self.p;
        let mut cap: Vec<isize> = Vec::new();
        let mut j = 0;
        while j < runq.dense.len() {
            let d = runq.dense[j];
            j += 1;
            let Some(t) = d.t else {
                continue;
            };
            if longest
                && self.matched
                && !self.threads[t].cap.is_empty()
                && self.matchcap[0] < self.threads[t].cap[0]
            {
                self.pool.push(t);
                continue;
            }
            let pc = self.threads[t].inst;
            let i = &prog.inst[pc as usize];
            let mut add = false;
            match i.op {
                InstOp::Match => {
                    if !self.threads[t].cap.is_empty()
                        && (!longest || !self.matched || self.matchcap[1] < pos as isize)
                    {
                        self.threads[t].cap[1] = pos as isize;
                        let tc = &self.threads[t].cap;
                        self.matchcap.copy_from_slice(tc);
                    }
                    if !longest {
                        // First-match mode: cut off all lower-priority threads.
                        for d in &runq.dense[j..] {
                            if let Some(t) = d.t {
                                self.pool.push(t);
                            }
                        }
                        runq.dense.clear();
                    }
                    self.matched = true;
                }
                InstOp::Rune => add = i.match_rune(c),
                InstOp::Rune1 => add = c == i.rune[0],
                InstOp::RuneAny => add = true,
                InstOp::RuneAnyNotNL => add = c != '\n' as Rune,
                _ => panic!("bad inst"),
            }
            let mut t = Some(t);
            if add {
                // Go passes t.cap itself; the port passes a copy (the capture array is only
                // changed temporarily inside add and restored before it returns).
                let tt = t.expect("live");
                cap.clear();
                cap.extend_from_slice(&self.threads[tt].cap);
                t = self.add(nextq, i.out, next_pos, &mut cap, next_cond, t);
            }
            if let Some(t) = t {
                self.pool.push(t);
            }
        }
        runq.dense.clear();
    }

    /// add adds an entry to q for pc, unless the q already has such an entry. It also
    /// recursively adds an entry for all instructions reachable from pc by following
    /// empty-width conditions satisfied by cond. pos gives the current position in the input.
    // Go: regexp/exec.go:add
    fn add(
        &mut self,
        q: &mut Queue,
        mut pc: u32,
        pos: usize,
        cap: &mut [isize],
        cond: LazyFlag,
        mut t: Option<usize>,
    ) -> Option<usize> {
        let prog = self.p;
        loop {
            if pc == 0 {
                return t;
            }
            let j = q.sparse[pc as usize];
            if (j as usize) < q.dense.len() && q.dense[j as usize].pc == pc {
                return t;
            }

            let j = q.dense.len();
            q.dense.push(Entry { pc, t: None });
            q.sparse[pc as usize] = j as u32;

            let i = &prog.inst[pc as usize];
            match i.op {
                InstOp::Fail => {
                    // nothing
                }
                InstOp::Alt | InstOp::AltMatch => {
                    t = self.add(q, i.out, pos, cap, cond, t);
                    pc = i.arg;
                    continue;
                }
                InstOp::EmptyWidth => {
                    if cond.match_(i.arg as EmptyOp) {
                        pc = i.out;
                        continue;
                    }
                }
                InstOp::Nop => {
                    pc = i.out;
                    continue;
                }
                InstOp::Capture => {
                    if (i.arg as usize) < cap.len() {
                        let opos = cap[i.arg as usize];
                        cap[i.arg as usize] = pos as isize;
                        self.add(q, i.out, pos, cap, cond, None);
                        cap[i.arg as usize] = opos;
                    } else {
                        pc = i.out;
                        continue;
                    }
                }
                InstOp::Match
                | InstOp::Rune
                | InstOp::Rune1
                | InstOp::RuneAny
                | InstOp::RuneAnyNotNL => {
                    let tt = match t {
                        None => self.alloc(pc),
                        Some(tt) => {
                            self.threads[tt].inst = pc;
                            tt
                        }
                    };
                    if !cap.is_empty() {
                        self.threads[tt].cap.copy_from_slice(cap);
                    }
                    q.dense[j].t = Some(tt);
                    t = None;
                }
            }
            return t;
        }
    }
}

impl Inner {
    /// doOnePass implements r.doExecute using the one-pass execution engine.
    // Go: regexp/exec.go:doOnePass
    fn do_one_pass(&self, i: Input<'_>, mut pos: usize, ncap: usize) -> Option<Match> {
        let start_cond = self.cond;
        if start_cond == !0 {
            // impossible
            return None;
        }
        let onepass = self.onepass.as_ref().expect("onepass");

        let mut matchcap: Vec<isize> = vec![-1; ncap];
        let mut matched = false;

        let (mut r, mut r1) = (END_OF_TEXT, END_OF_TEXT);
        let (mut width, mut width1) = (0, 0);
        (r, width) = i.step(pos);
        if r != END_OF_TEXT {
            (r1, width1) = i.step(pos + width);
        }
        let mut flag = if pos == 0 {
            LazyFlag::new(-1, r)
        } else {
            i.context(pos)
        };
        let mut pc = onepass.start;
        let inst = &onepass.inst[pc];
        // If there is a simple literal prefix, skip over it.
        'ret: {
            if pos == 0 && flag.match_(inst.inst.arg as EmptyOp) && !self.prefix.is_empty() {
                // Match requires literal prefix; fast search for it.
                if !i.has_prefix(self) {
                    break 'ret;
                }
                pos += self.prefix.len();
                (r, width) = i.step(pos);
                (r1, width1) = i.step(pos + width);
                flag = i.context(pos);
                pc = self.prefix_end as usize;
            }
            loop {
                let inst = &onepass.inst[pc];
                pc = inst.inst.out as usize;
                match inst.inst.op {
                    InstOp::Match => {
                        matched = true;
                        if !matchcap.is_empty() {
                            matchcap[0] = 0;
                            matchcap[1] = pos as isize;
                        }
                        break 'ret;
                    }
                    InstOp::Rune => {
                        if !inst.inst.match_rune(r) {
                            break 'ret;
                        }
                    }
                    InstOp::Rune1 => {
                        // Special case: single rune.
                        if r != inst.inst.rune[0] {
                            break 'ret;
                        }
                    }
                    InstOp::RuneAny => {
                        // Matches any rune.
                    }
                    InstOp::RuneAnyNotNL => {
                        // Matches any rune except '\n'
                        if r == '\n' as Rune {
                            break 'ret;
                        }
                    }
                    // peek at the input rune to see which branch of the Alt to take
                    InstOp::Alt | InstOp::AltMatch => {
                        pc = super::onepass::one_pass_next(inst, r) as usize;
                        continue;
                    }
                    InstOp::Fail => break 'ret,
                    InstOp::Nop => continue,
                    InstOp::EmptyWidth => {
                        if !flag.match_(inst.inst.arg as EmptyOp) {
                            break 'ret;
                        }
                        continue;
                    }
                    InstOp::Capture => {
                        if (inst.inst.arg as usize) < matchcap.len() {
                            matchcap[inst.inst.arg as usize] = pos as isize;
                        }
                        continue;
                    }
                }
                if width == 0 {
                    break;
                }
                flag = LazyFlag::new(r, r1);
                pos += width;
                (r, width) = (r1, width1);
                if r != END_OF_TEXT {
                    (r1, width1) = i.step(pos + width);
                }
            }
        }

        if !matched {
            return None;
        }
        Some(matchcap)
    }

    /// doMatch reports whether the input matches the regexp.
    // Go: regexp/exec.go:doMatch
    pub(crate) fn do_match(&self, s: &[u8]) -> bool {
        self.find(s, 0, 0).is_some()
    }

    /// find finds the leftmost match in the input, appends the position of its subexpressions
    /// to dstCap and returns dstCap. (Go's `doExecute`.)
    ///
    /// nil is returned if no matches are found and non-nil if matches are found.
    // Go: regexp/exec.go:find
    pub(crate) fn find(&self, s: &[u8], pos: usize, ncap: usize) -> Option<Match> {
        if s.len() < self.min_input_len {
            return None;
        }

        let i = Input { s };
        if self.onepass.is_some() {
            return self.do_one_pass(i, pos, ncap);
        }
        if s.len() < self.max_bit_state_len {
            return self.backtrack(i, pos, ncap);
        }

        let mut m = Machine::new(self, ncap);
        if !m.match_(i, pos) {
            return None;
        }
        Some(m.matchcap)
    }
}
