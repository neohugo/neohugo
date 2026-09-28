//! Port of Go's `regexp/syntax/compile.go` (go1.27.1).

use go_unicode::{MAX_RUNE, Rune, simple_fold};

use super::prog::{
    EMPTY_BEGIN_LINE, EMPTY_BEGIN_TEXT, EMPTY_END_LINE, EMPTY_END_TEXT, EMPTY_NO_WORD_BOUNDARY,
    EMPTY_WORD_BOUNDARY, EmptyOp, Inst, InstOp, Prog,
};
use super::regexp::{FOLD_CASE, Flags, NON_GREEDY, Op, Regexp};

/// A patchList is a list of instruction pointers that need to be filled in (patched). Because
/// the pointers haven't been filled in yet, we can reuse their storage to hold the list. It's
/// kind of sleazy, but works well in practice. See
/// https://swtch.com/~rsc/regexp/regexp1.html for inspiration.
///
/// These aren't really pointers: they're integers, so we can reinterpret them this way
/// without using package unsafe. A value l.head denotes p.inst[l.head>>1].out (l.head&1==0)
/// or .arg (l.head&1==1). head == 0 denotes the empty list, okay because we start every
/// program with a fail instruction, so we'll never want to point at its output link.
#[derive(Clone, Copy, Default)]
struct PatchList {
    head: u32,
    tail: u32,
}

// Go: regexp/syntax/compile.go:makePatchList
fn make_patch_list(n: u32) -> PatchList {
    PatchList { head: n, tail: n }
}

impl PatchList {
    // Go: regexp/syntax/compile.go:patch
    fn patch(self, p: &mut Prog, val: u32) {
        let mut head = self.head;
        while head != 0 {
            let i = &mut p.inst[(head >> 1) as usize];
            if head & 1 == 0 {
                head = i.out;
                i.out = val;
            } else {
                head = i.arg;
                i.arg = val;
            }
        }
    }

    // Go: regexp/syntax/compile.go:append
    fn append(self, p: &mut Prog, l2: PatchList) -> PatchList {
        let l1 = self;
        if l1.head == 0 {
            return l2;
        }
        if l2.head == 0 {
            return l1;
        }

        let i = &mut p.inst[(l1.tail >> 1) as usize];
        if l1.tail & 1 == 0 {
            i.out = l2.head;
        } else {
            i.arg = l2.head;
        }
        PatchList {
            head: l1.head,
            tail: l2.tail,
        }
    }
}

/// A frag represents a compiled program fragment.
#[derive(Clone, Copy, Default)]
struct Frag {
    /// index of first instruction
    i: u32,
    /// where to record end instruction
    out: PatchList,
    /// whether fragment can match empty string
    nullable: bool,
}

struct Compiler {
    p: Prog,
}

/// Compile compiles the regexp into a program to be executed. The regexp should have been
/// simplified already (returned from re.Simplify).
// Go: regexp/syntax/compile.go:Compile
pub fn compile(re: &Regexp) -> Prog {
    let mut c = Compiler { p: Prog::default() };
    c.init();
    let f = c.compile(re);
    let m = c.inst(InstOp::Match).i;
    f.out.patch(&mut c.p, m);
    c.p.start = f.i as usize;
    c.p
}

static ANY_RUNE_NOT_NL: [Rune; 4] = [0, '\n' as Rune - 1, '\n' as Rune + 1, MAX_RUNE];
static ANY_RUNE: [Rune; 2] = [0, MAX_RUNE];

impl Compiler {
    // Go: regexp/syntax/compile.go:init
    fn init(&mut self) {
        self.p.num_cap = 2; // implicit ( and ) for whole match $0
        self.inst(InstOp::Fail);
    }

    // Go: regexp/syntax/compile.go:compile
    fn compile(&mut self, re: &Regexp) -> Frag {
        match re.op {
            Op::NoMatch => self.fail(),
            Op::EmptyMatch => self.nop(),
            Op::Literal => {
                if re.rune.is_empty() {
                    return self.nop();
                }
                let mut f = Frag::default();
                for j in 0..re.rune.len() {
                    let f1 = self.rune(&re.rune[j..j + 1], re.flags);
                    if j == 0 {
                        f = f1;
                    } else {
                        f = self.cat(f, f1);
                    }
                }
                f
            }
            Op::CharClass => self.rune(&re.rune, re.flags),
            Op::AnyCharNotNL => self.rune(&ANY_RUNE_NOT_NL, 0),
            Op::AnyChar => self.rune(&ANY_RUNE, 0),
            Op::BeginLine => self.empty(EMPTY_BEGIN_LINE),
            Op::EndLine => self.empty(EMPTY_END_LINE),
            Op::BeginText => self.empty(EMPTY_BEGIN_TEXT),
            Op::EndText => self.empty(EMPTY_END_TEXT),
            Op::WordBoundary => self.empty(EMPTY_WORD_BOUNDARY),
            Op::NoWordBoundary => self.empty(EMPTY_NO_WORD_BOUNDARY),
            Op::Capture => {
                let bra = self.cap((re.cap << 1) as u32);
                let sub = self.compile(&re.sub[0]);
                let ket = self.cap((re.cap << 1 | 1) as u32);
                let f = self.cat(bra, sub);
                self.cat(f, ket)
            }
            Op::Star => {
                let f = self.compile(&re.sub[0]);
                self.star(f, re.flags & NON_GREEDY != 0)
            }
            Op::Plus => {
                let f = self.compile(&re.sub[0]);
                self.plus(f, re.flags & NON_GREEDY != 0)
            }
            Op::Quest => {
                let f = self.compile(&re.sub[0]);
                self.quest(f, re.flags & NON_GREEDY != 0)
            }
            Op::Concat => {
                if re.sub.is_empty() {
                    return self.nop();
                }
                let mut f = Frag::default();
                for (i, sub) in re.sub.iter().enumerate() {
                    if i == 0 {
                        f = self.compile(sub);
                    } else {
                        let f2 = self.compile(sub);
                        f = self.cat(f, f2);
                    }
                }
                f
            }
            Op::Alternate => {
                let mut f = Frag::default();
                for sub in &re.sub {
                    let f2 = self.compile(sub);
                    f = self.alt(f, f2);
                }
                f
            }
            _ => panic!("regexp: unhandled case in compile"),
        }
    }

    // Go: regexp/syntax/compile.go:inst
    fn inst(&mut self, op: InstOp) -> Frag {
        // TODO: impose length limit
        let f = Frag {
            i: self.p.inst.len() as u32,
            nullable: true,
            ..Default::default()
        };
        self.p.inst.push(Inst {
            op,
            ..Default::default()
        });
        f
    }

    // Go: regexp/syntax/compile.go:nop
    fn nop(&mut self) -> Frag {
        let mut f = self.inst(InstOp::Nop);
        f.out = make_patch_list(f.i << 1);
        f
    }

    // Go: regexp/syntax/compile.go:fail
    fn fail(&mut self) -> Frag {
        Frag::default()
    }

    // Go: regexp/syntax/compile.go:cap
    fn cap(&mut self, arg: u32) -> Frag {
        let mut f = self.inst(InstOp::Capture);
        f.out = make_patch_list(f.i << 1);
        self.p.inst[f.i as usize].arg = arg;

        if self.p.num_cap < arg as usize + 1 {
            self.p.num_cap = arg as usize + 1;
        }
        f
    }

    // Go: regexp/syntax/compile.go:cat
    fn cat(&mut self, f1: Frag, f2: Frag) -> Frag {
        // concat of failure is failure
        if f1.i == 0 || f2.i == 0 {
            return Frag::default();
        }

        // TODO: elide nop

        f1.out.patch(&mut self.p, f2.i);
        Frag {
            i: f1.i,
            out: f2.out,
            nullable: f1.nullable && f2.nullable,
        }
    }

    // Go: regexp/syntax/compile.go:alt
    fn alt(&mut self, f1: Frag, f2: Frag) -> Frag {
        // alt of failure is other
        if f1.i == 0 {
            return f2;
        }
        if f2.i == 0 {
            return f1;
        }

        let mut f = self.inst(InstOp::Alt);
        let i = &mut self.p.inst[f.i as usize];
        i.out = f1.i;
        i.arg = f2.i;
        f.out = f1.out.append(&mut self.p, f2.out);
        f.nullable = f1.nullable || f2.nullable;
        f
    }

    // Go: regexp/syntax/compile.go:quest
    fn quest(&mut self, f1: Frag, nongreedy: bool) -> Frag {
        let mut f = self.inst(InstOp::Alt);
        let i = &mut self.p.inst[f.i as usize];
        if nongreedy {
            i.arg = f1.i;
            f.out = make_patch_list(f.i << 1);
        } else {
            i.out = f1.i;
            f.out = make_patch_list(f.i << 1 | 1);
        }
        f.out = f.out.append(&mut self.p, f1.out);
        f
    }

    /// loop returns the fragment for the main loop of a plus or star. For plus, it can be used
    /// after changing the entry to f1.i. For star, it can be used directly when f1 can't match
    /// an empty string. (When f1 can match an empty string, f1* must be implemented as (f1+)?
    /// to get the priority match order correct.)
    // Go: regexp/syntax/compile.go:loop
    fn loop_(&mut self, f1: Frag, nongreedy: bool) -> Frag {
        let mut f = self.inst(InstOp::Alt);
        let i = &mut self.p.inst[f.i as usize];
        if nongreedy {
            i.arg = f1.i;
            f.out = make_patch_list(f.i << 1);
        } else {
            i.out = f1.i;
            f.out = make_patch_list(f.i << 1 | 1);
        }
        f1.out.patch(&mut self.p, f.i);
        f
    }

    // Go: regexp/syntax/compile.go:star
    fn star(&mut self, f1: Frag, nongreedy: bool) -> Frag {
        if f1.nullable {
            // Use (f1+)? to get priority match order correct. See golang.org/issue/46123.
            let p = self.plus(f1, nongreedy);
            return self.quest(p, nongreedy);
        }
        self.loop_(f1, nongreedy)
    }

    // Go: regexp/syntax/compile.go:plus
    fn plus(&mut self, f1: Frag, nongreedy: bool) -> Frag {
        Frag {
            i: f1.i,
            out: self.loop_(f1, nongreedy).out,
            nullable: f1.nullable,
        }
    }

    // Go: regexp/syntax/compile.go:empty
    fn empty(&mut self, op: EmptyOp) -> Frag {
        let mut f = self.inst(InstOp::EmptyWidth);
        self.p.inst[f.i as usize].arg = u32::from(op);
        f.out = make_patch_list(f.i << 1);
        f
    }

    // Go: regexp/syntax/compile.go:rune
    fn rune(&mut self, r: &[Rune], mut flags: Flags) -> Frag {
        let mut f = self.inst(InstOp::Rune);
        f.nullable = false;
        let i = &mut self.p.inst[f.i as usize];
        i.rune = r.to_vec();
        flags &= FOLD_CASE; // only relevant flag is FoldCase
        if r.len() != 1 || simple_fold(r[0]) == r[0] {
            // and sometimes not even that
            flags &= !FOLD_CASE;
        }
        i.arg = u32::from(flags);
        f.out = make_patch_list(f.i << 1);

        // Special cases for exec machine.
        if flags & FOLD_CASE == 0 && (r.len() == 1 || r.len() == 2 && r[0] == r[1]) {
            i.op = InstOp::Rune1;
        } else if r.len() == 2 && r[0] == 0 && r[1] == MAX_RUNE {
            i.op = InstOp::RuneAny;
        } else if r.len() == 4
            && r[0] == 0
            && r[1] == '\n' as Rune - 1
            && r[2] == '\n' as Rune + 1
            && r[3] == MAX_RUNE
        {
            i.op = InstOp::RuneAnyNotNL;
        }

        f
    }
}
