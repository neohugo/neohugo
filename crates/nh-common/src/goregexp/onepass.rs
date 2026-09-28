//! Port of Go's `regexp/onepass.go` (go1.27.1): one-pass (unambiguous, anchored) programs.

use go_unicode::utf8::{self, RUNE_ERROR};
use go_unicode::{MAX_RUNE, Rune, simple_fold};

use super::syntax::{
    EMPTY_BEGIN_TEXT, EMPTY_END_TEXT, EmptyOp, FOLD_CASE, Flags, Inst, InstOp, Prog,
};

/// "One-pass" regexp execution. Some regexps can be analyzed to determine that they never
/// need backtracking: they are guaranteed to run in one pass over the string without
/// bothering to save all the usual NFA state. Detect those and execute them more quickly.
///
/// A onePassProg is a compiled one-pass regular expression program. It is the same as
/// syntax.Prog except for the use of onePassInst.
#[derive(Clone, Debug)]
pub(crate) struct OnePassProg {
    pub(crate) inst: Vec<OnePassInst>,
    /// index of start instruction
    pub(crate) start: usize,
    /// number of InstCapture insts in re
    #[allow(dead_code)]
    pub(crate) num_cap: usize,
}

/// A onePassInst is a single instruction in a one-pass regular expression program. It is the
/// same as syntax.Inst except for the new 'Next' field.
#[derive(Clone, Debug, Default)]
pub(crate) struct OnePassInst {
    pub(crate) inst: Inst,
    pub(crate) next: Vec<u32>,
}

/// onePassPrefix returns a literal string that all matches for the regexp must start with.
/// Complete is true if the prefix is the entire match. Pc is the index of the last rune
/// instruction in the string. The onePassPrefix skips over the mandatory EmptyBeginText.
// Go: regexp/onepass.go:onePassPrefix
pub(crate) fn one_pass_prefix(p: &Prog) -> (Vec<u8>, bool, u32) {
    let mut i = &p.inst[p.start];
    if i.op != InstOp::EmptyWidth || (i.arg as EmptyOp) & EMPTY_BEGIN_TEXT == 0 {
        return (Vec::new(), i.op == InstOp::Match, p.start as u32);
    }
    let mut pc = i.out;
    i = &p.inst[pc as usize];
    while i.op == InstOp::Nop {
        pc = i.out;
        i = &p.inst[pc as usize];
    }
    // Avoid allocation of buffer if prefix is empty.
    if iop(i) != InstOp::Rune || i.rune.len() != 1 {
        return (Vec::new(), i.op == InstOp::Match, p.start as u32);
    }

    // Have prefix; gather characters.
    let mut buf = Vec::new();
    while iop(i) == InstOp::Rune
        && i.rune.len() == 1
        && (i.arg as Flags) & FOLD_CASE == 0
        && i.rune[0] != RUNE_ERROR
    {
        utf8::append_rune(&mut buf, i.rune[0]);
        pc = i.out;
        i = &p.inst[i.out as usize];
    }
    let complete = i.op == InstOp::EmptyWidth
        && (i.arg as EmptyOp) & EMPTY_END_TEXT != 0
        && p.inst[i.out as usize].op == InstOp::Match;
    (buf, complete, pc)
}

/// onePassNext selects the next actionable state of the prog, based on the input character.
/// It should only be called when i.Op == InstAlt or InstAltMatch, and from the one-pass
/// machine. One of the alternates may ultimately lead without input to end of line. If the
/// instruction is InstAltMatch the path to the InstMatch is in i.Out, the normal node in
/// i.Next.
// Go: regexp/onepass.go:onePassNext
pub(crate) fn one_pass_next(i: &OnePassInst, r: Rune) -> u32 {
    let next = i.inst.match_rune_pos(r);
    if next >= 0 {
        return i.next[next as usize];
    }
    if i.inst.op == InstOp::AltMatch {
        return i.inst.out;
    }
    0
}

// Go: regexp/onepass.go:iop
fn iop(i: &Inst) -> InstOp {
    match i.op {
        InstOp::Rune1 | InstOp::RuneAny | InstOp::RuneAnyNotNL => InstOp::Rune,
        op => op,
    }
}

/// Sparse Array implementation is used as a queueOnePass.
struct QueueOnePass {
    sparse: Vec<u32>,
    dense: Vec<u32>,
    size: u32,
    next_index: u32,
}

impl QueueOnePass {
    // Go: regexp/onepass.go:empty
    fn empty(&self) -> bool {
        self.next_index >= self.size
    }

    // Go: regexp/onepass.go:next
    fn next(&mut self) -> u32 {
        let n = self.dense[self.next_index as usize];
        self.next_index += 1;
        n
    }

    // Go: regexp/onepass.go:clear
    fn clear(&mut self) {
        self.size = 0;
        self.next_index = 0;
    }

    // Go: regexp/onepass.go:contains
    fn contains(&self, u: u32) -> bool {
        if u as usize >= self.sparse.len() {
            return false;
        }
        self.sparse[u as usize] < self.size && self.dense[self.sparse[u as usize] as usize] == u
    }

    // Go: regexp/onepass.go:insert
    fn insert(&mut self, u: u32) {
        if !self.contains(u) {
            self.insert_new(u);
        }
    }

    // Go: regexp/onepass.go:insertNew
    fn insert_new(&mut self, u: u32) {
        if u as usize >= self.sparse.len() {
            return;
        }
        self.sparse[u as usize] = self.size;
        self.dense[self.size as usize] = u;
        self.size += 1;
    }
}

// Go: regexp/onepass.go:newQueue
fn new_queue(size: usize) -> QueueOnePass {
    QueueOnePass {
        sparse: vec![0; size],
        dense: vec![0; size],
        size: 0,
        next_index: 0,
    }
}

/// mergeRuneSets merges two non-intersecting runesets, and returns the merged result, and a
/// NextIp array. The idea is that if a rune matches the OnePassRunes at index i, NextIp[i/2]
/// is the target. If the input sets intersect, an empty runeset and a NextIp array with the
/// single element mergeFailed is returned. The code assumes that both inputs contain ordered
/// and non-intersecting rune pairs.
const MERGE_FAILED: u32 = 0xffffffff;

// Go: regexp/onepass.go:mergeRuneSets
// (Go's switch has two cases with the same body.)
#[allow(clippy::if_same_then_else)]
fn merge_rune_sets(
    left_runes: &[Rune],
    right_runes: &[Rune],
    left_pc: u32,
    right_pc: u32,
) -> (Vec<Rune>, Vec<u32>) {
    let left_len = left_runes.len();
    let right_len = right_runes.len();
    if left_len & 0x1 != 0 || right_len & 0x1 != 0 {
        panic!("mergeRuneSets odd length []rune");
    }
    let (mut lx, mut rx) = (0, 0);
    let mut merged: Vec<Rune> = Vec::new();
    let mut next: Vec<u32> = Vec::new();

    let mut ix: isize = -1;
    let mut extend = |new_low: &mut usize, new_array: &[Rune], pc: u32| -> bool {
        if ix > 0 && new_array[*new_low] <= merged[ix as usize] {
            return false;
        }
        merged.push(new_array[*new_low]);
        merged.push(new_array[*new_low + 1]);
        *new_low += 2;
        ix += 2;
        next.push(pc);
        true
    };

    while lx < left_len || rx < right_len {
        let ok = if rx >= right_len {
            extend(&mut lx, left_runes, left_pc)
        } else if lx >= left_len {
            extend(&mut rx, right_runes, right_pc)
        } else if right_runes[rx] < left_runes[lx] {
            extend(&mut rx, right_runes, right_pc)
        } else {
            extend(&mut lx, left_runes, left_pc)
        };
        if !ok {
            return (Vec::new(), vec![MERGE_FAILED]);
        }
    }
    (merged, next)
}

/// cleanupOnePass drops working memory, and restores certain shortcut instructions.
// Go: regexp/onepass.go:cleanupOnePass
fn cleanup_one_pass(prog: &mut OnePassProg, original: &Prog) {
    for (ix, inst_original) in original.inst.iter().enumerate() {
        match inst_original.op {
            InstOp::Alt | InstOp::AltMatch | InstOp::Rune => {}
            InstOp::Capture | InstOp::EmptyWidth | InstOp::Nop | InstOp::Match | InstOp::Fail => {
                prog.inst[ix].next = Vec::new();
            }
            InstOp::Rune1 | InstOp::RuneAny | InstOp::RuneAnyNotNL => {
                prog.inst[ix] = OnePassInst {
                    inst: inst_original.clone(),
                    next: Vec::new(),
                };
            }
        }
    }
}

/// onePassCopy creates a copy of the original Prog, as we'll be modifying it.
// Go: regexp/onepass.go:onePassCopy
fn one_pass_copy(prog: &Prog) -> OnePassProg {
    let mut p = OnePassProg {
        start: prog.start,
        num_cap: prog.num_cap,
        inst: prog
            .inst
            .iter()
            .map(|inst| OnePassInst {
                inst: inst.clone(),
                next: Vec::new(),
            })
            .collect(),
    };

    // rewrites one or more common Prog constructs that enable some otherwise non-onepass
    // Progs to be onepass. A:BD (for example) means an InstAlt at A, with its out/arg
    // pointing to B and D.
    for pc in 0..p.inst.len() {
        match p.inst[pc].inst.op {
            InstOp::Alt | InstOp::AltMatch => {
                // A:Bx + B:Ay
                // (the pointers are (inst index, is_arg))
                let mut p_a_other = (pc, false);
                let mut p_a_alt = (pc, true);
                let get = |p: &OnePassProg, (ix, is_arg): (usize, bool)| -> u32 {
                    if is_arg {
                        p.inst[ix].inst.arg
                    } else {
                        p.inst[ix].inst.out
                    }
                };
                let set = |p: &mut OnePassProg, (ix, is_arg): (usize, bool), v: u32| {
                    if is_arg {
                        p.inst[ix].inst.arg = v;
                    } else {
                        p.inst[ix].inst.out = v;
                    }
                };
                // make sure a target is another Alt
                let mut inst_alt = p.inst[get(&p, p_a_alt) as usize].inst.clone();
                if !(inst_alt.op == InstOp::Alt || inst_alt.op == InstOp::AltMatch) {
                    std::mem::swap(&mut p_a_alt, &mut p_a_other);
                    inst_alt = p.inst[get(&p, p_a_alt) as usize].inst.clone();
                    if !(inst_alt.op == InstOp::Alt || inst_alt.op == InstOp::AltMatch) {
                        continue;
                    }
                }
                // simple empty transition loop
                // A:BC + B:DA => A:BC + B:DC
                let inst_other = &p.inst[get(&p, p_a_other) as usize].inst;
                if inst_other.op == InstOp::Alt || inst_other.op == InstOp::AltMatch {
                    // too complicated
                    continue;
                }
                // Alt and Other targets
                let b_ix = get(&p, p_a_alt) as usize;
                let mut p_b_alt = (b_ix, false);
                let mut p_b_other = (b_ix, true);
                let mut patch = false;
                if inst_alt.out == pc as u32 {
                    patch = true;
                } else if inst_alt.arg == pc as u32 {
                    patch = true;
                    std::mem::swap(&mut p_b_alt, &mut p_b_other);
                }
                if patch {
                    let v = get(&p, p_a_other);
                    set(&mut p, p_b_alt, v);
                }

                // empty transition to common target
                // A:BC + B:DC => A:DC + B:DC
                if get(&p, p_a_other) == get(&p, p_b_alt) {
                    let v = get(&p, p_b_other);
                    set(&mut p, p_a_alt, v);
                }
            }
            _ => continue,
        }
    }
    p
}

static ANY_RUNE_NOT_NL: [Rune; 4] = [0, '\n' as Rune - 1, '\n' as Rune + 1, MAX_RUNE];
static ANY_RUNE: [Rune; 2] = [0, MAX_RUNE];

/// makeOnePass creates a onepass Prog, if possible. It is possible if at any alt, the match
/// engine can always tell which branch to take. The routine may modify p if it is turned into
/// a onepass Prog. If it isn't possible for this to be a onepass Prog, nil is returned.
/// makeOnePass is recursive to the size of the Prog.
// Go: regexp/onepass.go:makeOnePass
fn make_one_pass(mut p: OnePassProg) -> Option<OnePassProg> {
    // If the machine is very long, it's not worth the time to check if we can use one pass.
    if p.inst.len() >= 1000 {
        return None;
    }

    let mut inst_queue = new_queue(p.inst.len());
    let mut visit_queue = new_queue(p.inst.len());
    let mut one_pass_runes: Vec<Vec<Rune>> = vec![Vec::new(); p.inst.len()];

    struct Ctx<'a> {
        p: &'a mut OnePassProg,
        inst_queue: &'a mut QueueOnePass,
        visit_queue: &'a mut QueueOnePass,
        one_pass_runes: &'a mut Vec<Vec<Rune>>,
    }

    // check that paths from Alt instructions are unambiguous, and rebuild the new program as a
    // onepass program
    fn check(c: &mut Ctx<'_>, pc: u32, m: &mut [bool]) -> bool {
        let mut ok = true;
        if c.visit_queue.contains(pc) {
            return ok;
        }
        c.visit_queue.insert(pc);
        let pcu = pc as usize;
        match c.p.inst[pcu].inst.op {
            InstOp::Alt | InstOp::AltMatch => {
                let (out, arg) = (c.p.inst[pcu].inst.out, c.p.inst[pcu].inst.arg);
                ok = check(c, out, m) && check(c, arg, m);
                // check no-input paths to InstMatch
                let (out, arg) = (c.p.inst[pcu].inst.out, c.p.inst[pcu].inst.arg);
                let mut match_out = m[out as usize];
                let mut match_arg = m[arg as usize];
                if match_out && match_arg {
                    return false;
                }
                // Match on empty goes in inst.Out
                if match_arg {
                    let inst = &mut c.p.inst[pcu].inst;
                    std::mem::swap(&mut inst.out, &mut inst.arg);
                    std::mem::swap(&mut match_out, &mut match_arg);
                }
                if match_out {
                    m[pcu] = true;
                    c.p.inst[pcu].inst.op = InstOp::AltMatch;
                }

                // build a dispatch operator from the two legs
                let (out, arg) = (c.p.inst[pcu].inst.out, c.p.inst[pcu].inst.arg);
                let (runes, next) = merge_rune_sets(
                    &c.one_pass_runes[out as usize],
                    &c.one_pass_runes[arg as usize],
                    out,
                    arg,
                );
                c.one_pass_runes[pcu] = runes;
                c.p.inst[pcu].next = next;
                if !c.p.inst[pcu].next.is_empty() && c.p.inst[pcu].next[0] == MERGE_FAILED {
                    return false;
                }
            }
            InstOp::Capture | InstOp::Nop | InstOp::EmptyWidth => {
                let out = c.p.inst[pcu].inst.out;
                ok = check(c, out, m);
                m[pcu] = m[out as usize];
                c.one_pass_runes[pcu] = c.one_pass_runes[out as usize].clone();
                c.p.inst[pcu].next = vec![out; c.one_pass_runes[pcu].len() / 2 + 1];
            }
            InstOp::Match | InstOp::Fail => {
                m[pcu] = c.p.inst[pcu].inst.op == InstOp::Match;
            }
            InstOp::Rune => {
                m[pcu] = false;
                if !c.p.inst[pcu].next.is_empty() {
                    return ok;
                }
                let inst = &c.p.inst[pcu].inst;
                let out = inst.out;
                c.inst_queue.insert(out);
                if inst.rune.is_empty() {
                    c.one_pass_runes[pcu] = Vec::new();
                    c.p.inst[pcu].next = vec![out];
                    return ok;
                }
                let mut runes: Vec<Rune> = Vec::new();
                if inst.rune.len() == 1 && (inst.arg as Flags) & FOLD_CASE != 0 {
                    let r0 = inst.rune[0];
                    runes.push(r0);
                    runes.push(r0);
                    let mut r1 = simple_fold(r0);
                    while r1 != r0 {
                        runes.push(r1);
                        runes.push(r1);
                        r1 = simple_fold(r1);
                    }
                    runes.sort_unstable();
                } else {
                    runes.extend_from_slice(&inst.rune);
                }
                let n = runes.len() / 2 + 1;
                c.one_pass_runes[pcu] = runes;
                c.p.inst[pcu].next = vec![out; n];
                c.p.inst[pcu].inst.op = InstOp::Rune;
            }
            InstOp::Rune1 => {
                m[pcu] = false;
                if !c.p.inst[pcu].next.is_empty() {
                    return ok;
                }
                let inst = &c.p.inst[pcu].inst;
                let out = inst.out;
                c.inst_queue.insert(out);
                let mut runes: Vec<Rune> = Vec::new();
                // expand for 1 rune
                if (inst.arg as Flags) & FOLD_CASE != 0 {
                    let r0 = inst.rune[0];
                    runes.push(r0);
                    runes.push(r0);
                    let mut r1 = simple_fold(r0);
                    while r1 != r0 {
                        runes.push(r1);
                        runes.push(r1);
                        r1 = simple_fold(r1);
                    }
                    runes.sort_unstable();
                } else {
                    runes.push(inst.rune[0]);
                    runes.push(inst.rune[0]);
                }
                let n = runes.len() / 2 + 1;
                c.one_pass_runes[pcu] = runes;
                c.p.inst[pcu].next = vec![out; n];
                c.p.inst[pcu].inst.op = InstOp::Rune;
            }
            InstOp::RuneAny => {
                m[pcu] = false;
                if !c.p.inst[pcu].next.is_empty() {
                    return ok;
                }
                let out = c.p.inst[pcu].inst.out;
                c.inst_queue.insert(out);
                c.one_pass_runes[pcu] = ANY_RUNE.to_vec();
                c.p.inst[pcu].next = vec![out];
            }
            InstOp::RuneAnyNotNL => {
                m[pcu] = false;
                if !c.p.inst[pcu].next.is_empty() {
                    return ok;
                }
                let out = c.p.inst[pcu].inst.out;
                c.inst_queue.insert(out);
                c.one_pass_runes[pcu] = ANY_RUNE_NOT_NL.to_vec();
                c.p.inst[pcu].next = vec![out; ANY_RUNE_NOT_NL.len() / 2 + 1];
            }
        }
        ok
    }

    inst_queue.clear();
    inst_queue.insert(p.start as u32);
    let mut m = vec![false; p.inst.len()];
    let mut ok = true;
    while !inst_queue.empty() {
        visit_queue.clear();
        let pc = inst_queue.next();
        let mut c = Ctx {
            p: &mut p,
            inst_queue: &mut inst_queue,
            visit_queue: &mut visit_queue,
            one_pass_runes: &mut one_pass_runes,
        };
        if !check(&mut c, pc, &mut m) {
            ok = false;
            break;
        }
    }
    if !ok {
        return None;
    }
    for (inst, runes) in p.inst.iter_mut().zip(one_pass_runes) {
        inst.inst.rune = runes;
    }
    Some(p)
}

/// compileOnePass returns a new *syntax.Prog suitable for onePass execution if the original
/// Prog can be recharacterized as a one-pass regexp program, or syntax.nil if the Prog cannot
/// be converted. For a one pass prog, the fundamental condition that must be true is: at any
/// InstAlt, there must be no ambiguity about what branch to take.
// Go: regexp/onepass.go:compileOnePass
pub(crate) fn compile_one_pass(prog: &Prog) -> Option<OnePassProg> {
    if prog.start == 0 {
        return None;
    }
    // onepass regexp is anchored
    if prog.inst[prog.start].op != InstOp::EmptyWidth
        || (prog.inst[prog.start].arg as EmptyOp) & EMPTY_BEGIN_TEXT != EMPTY_BEGIN_TEXT
    {
        return None;
    }
    let has_alt = prog
        .inst
        .iter()
        .any(|inst| inst.op == InstOp::Alt || inst.op == InstOp::AltMatch);
    // every instruction leading to InstMatch must be EmptyEndText
    for inst in &prog.inst {
        let op_out = prog.inst[inst.out as usize].op;
        match inst.op {
            InstOp::Alt | InstOp::AltMatch => {
                if op_out == InstOp::Match || prog.inst[inst.arg as usize].op == InstOp::Match {
                    return None;
                }
            }
            InstOp::EmptyWidth => {
                if op_out == InstOp::Match {
                    if (inst.arg as EmptyOp) & EMPTY_END_TEXT == EMPTY_END_TEXT {
                        continue;
                    }
                    return None;
                }
            }
            _ => {
                if op_out == InstOp::Match && has_alt {
                    return None;
                }
            }
        }
    }
    // Creates a slightly optimized copy of the original Prog that cleans up some Prog idioms
    // that block valid onepass programs
    let p = one_pass_copy(prog);

    // checkAmbiguity on InstAlts, build onepass Prog if possible
    let mut p = make_one_pass(p)?;

    cleanup_one_pass(&mut p, prog);
    Some(p)
}
