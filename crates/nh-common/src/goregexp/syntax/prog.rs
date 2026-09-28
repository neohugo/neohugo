//! Port of Go's `regexp/syntax/prog.go` (go1.27.1): the compiled program.

use go_unicode::utf8::{self, RUNE_ERROR};
use go_unicode::{Rune, simple_fold};

use super::regexp::{FOLD_CASE, Flags};

/// A Prog is a compiled regular expression program.
#[derive(Clone, Debug, Default)]
pub struct Prog {
    pub inst: Vec<Inst>,
    /// index of start instruction
    pub start: usize,
    /// number of InstCapture insts in re
    pub num_cap: usize,
}

/// An InstOp is an instruction opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum InstOp {
    #[default]
    Alt = 0,
    AltMatch,
    Capture,
    EmptyWidth,
    Match,
    Fail,
    Nop,
    Rune,
    Rune1,
    RuneAny,
    RuneAnyNotNL,
}

impl InstOp {
    // Go: regexp/syntax/prog.go:String
    pub fn string(self) -> &'static str {
        match self {
            InstOp::Alt => "InstAlt",
            InstOp::AltMatch => "InstAltMatch",
            InstOp::Capture => "InstCapture",
            InstOp::EmptyWidth => "InstEmptyWidth",
            InstOp::Match => "InstMatch",
            InstOp::Fail => "InstFail",
            InstOp::Nop => "InstNop",
            InstOp::Rune => "InstRune",
            InstOp::Rune1 => "InstRune1",
            InstOp::RuneAny => "InstRuneAny",
            InstOp::RuneAnyNotNL => "InstRuneAnyNotNL",
        }
    }
}

/// An EmptyOp specifies a kind or mixture of zero-width assertions.
pub type EmptyOp = u8;

pub const EMPTY_BEGIN_LINE: EmptyOp = 1;
pub const EMPTY_END_LINE: EmptyOp = 1 << 1;
pub const EMPTY_BEGIN_TEXT: EmptyOp = 1 << 2;
pub const EMPTY_END_TEXT: EmptyOp = 1 << 3;
pub const EMPTY_WORD_BOUNDARY: EmptyOp = 1 << 4;
pub const EMPTY_NO_WORD_BOUNDARY: EmptyOp = 1 << 5;

/// EmptyOpContext returns the zero-width assertions satisfied at the position between the
/// runes r1 and r2. Passing r1 == -1 indicates that the position is at the beginning of the
/// text. Passing r2 == -1 indicates that the position is at the end of the text.
// Go: regexp/syntax/prog.go:EmptyOpContext
pub fn empty_op_context(r1: Rune, r2: Rune) -> EmptyOp {
    let mut op: EmptyOp = EMPTY_NO_WORD_BOUNDARY;
    let mut boundary: u8 = 0;
    if is_word_char(r1) {
        boundary = 1;
    } else if r1 == '\n' as Rune {
        op |= EMPTY_BEGIN_LINE;
    } else if r1 < 0 {
        op |= EMPTY_BEGIN_TEXT | EMPTY_BEGIN_LINE;
    }
    if is_word_char(r2) {
        boundary ^= 1;
    } else if r2 == '\n' as Rune {
        op |= EMPTY_END_LINE;
    } else if r2 < 0 {
        op |= EMPTY_END_TEXT | EMPTY_END_LINE;
    }
    if boundary != 0 {
        // IsWordChar(r1) != IsWordChar(r2)
        op ^= EMPTY_WORD_BOUNDARY | EMPTY_NO_WORD_BOUNDARY;
    }
    op
}

/// IsWordChar reports whether r is considered a “word character” during the evaluation of the
/// \b and \B zero-width assertions. These assertions are ASCII-only: the word characters are
/// [A-Za-z0-9_].
// Go: regexp/syntax/prog.go:IsWordChar
pub fn is_word_char(r: Rune) -> bool {
    // Test for lowercase letters first, as these occur more frequently than uppercase letters
    // in common cases.
    (0x61..=0x7a).contains(&r)
        || (0x41..=0x5a).contains(&r)
        || (0x30..=0x39).contains(&r)
        || r == '_' as Rune
}

/// An Inst is a single instruction in a regular expression program.
#[derive(Clone, Debug, Default)]
pub struct Inst {
    pub op: InstOp,
    /// all but InstMatch, InstFail
    pub out: u32,
    /// InstAlt, InstAltMatch, InstCapture, InstEmptyWidth
    pub arg: u32,
    pub rune: Vec<Rune>,
}

impl Prog {
    /// Go: `p.String()`.
    // Go: regexp/syntax/prog.go:String
    pub fn string(&self) -> String {
        let mut b = String::new();
        dump_prog(&mut b, self);
        b
    }

    /// skipNop follows any no-op or capturing instructions.
    // Go: regexp/syntax/prog.go:skipNop
    fn skip_nop(&self, pc: u32) -> &Inst {
        let mut i = &self.inst[pc as usize];
        while i.op == InstOp::Nop || i.op == InstOp::Capture {
            i = &self.inst[i.out as usize];
        }
        i
    }

    /// Prefix returns a literal string that all matches for the regexp must start with.
    /// Complete is true if the prefix is the entire match.
    // Go: regexp/syntax/prog.go:Prefix
    pub fn prefix(&self) -> (Vec<u8>, bool) {
        let mut i = self.skip_nop(self.start as u32);

        // Avoid allocation of buffer if prefix is empty.
        if i.op_() != InstOp::Rune || i.rune.len() != 1 {
            return (Vec::new(), i.op == InstOp::Match);
        }

        // Have prefix; gather characters.
        let mut buf = Vec::new();
        while i.op_() == InstOp::Rune
            && i.rune.len() == 1
            && (i.arg as Flags) & FOLD_CASE == 0
            && i.rune[0] != RUNE_ERROR
        {
            utf8::append_rune(&mut buf, i.rune[0]);
            i = self.skip_nop(i.out);
        }
        (buf, i.op == InstOp::Match)
    }

    /// StartCond returns the leading empty-width conditions that must be true in any match. It
    /// returns ^EmptyOp(0) if no matches are possible.
    // Go: regexp/syntax/prog.go:StartCond
    pub fn start_cond(&self) -> EmptyOp {
        let mut flag: EmptyOp = 0;
        let mut pc = self.start;
        loop {
            let i = &self.inst[pc];
            match i.op {
                InstOp::EmptyWidth => flag |= i.arg as EmptyOp,
                InstOp::Fail => return !0,
                InstOp::Capture | InstOp::Nop => {
                    // skip
                }
                _ => break,
            }
            pc = i.out as usize;
        }
        flag
    }
}

const NO_MATCH: isize = -1;

impl Inst {
    /// op returns i.Op but merges all the rune special cases into InstRune
    // Go: regexp/syntax/prog.go:op
    pub(crate) fn op_(&self) -> InstOp {
        match self.op {
            InstOp::Rune1 | InstOp::RuneAny | InstOp::RuneAnyNotNL => InstOp::Rune,
            op => op,
        }
    }

    /// MatchRune reports whether the instruction matches (and consumes) r. It should only be
    /// called when i.Op == InstRune.
    // Go: regexp/syntax/prog.go:MatchRune
    pub fn match_rune(&self, r: Rune) -> bool {
        self.match_rune_pos(r) != NO_MATCH
    }

    /// MatchRunePos checks whether the instruction matches (and consumes) r. If so,
    /// MatchRunePos returns the index of the matching rune pair (or, when len(i.Rune) == 1,
    /// rune singleton). If not, MatchRunePos returns -1.
    // Go: regexp/syntax/prog.go:MatchRunePos
    pub fn match_rune_pos(&self, r: Rune) -> isize {
        let rune = &self.rune;

        match rune.len() {
            0 => return NO_MATCH,
            1 => {
                // Special case: single-rune slice is from literal string, not char class.
                let r0 = rune[0];
                if r == r0 {
                    return 0;
                }
                if (self.arg as Flags) & FOLD_CASE != 0 {
                    let mut r1 = simple_fold(r0);
                    while r1 != r0 {
                        if r == r1 {
                            return 0;
                        }
                        r1 = simple_fold(r1);
                    }
                }
                return NO_MATCH;
            }
            2 => {
                if r >= rune[0] && r <= rune[1] {
                    return 0;
                }
                return NO_MATCH;
            }
            4 | 6 | 8 => {
                // Linear search for a few pairs. Should handle ASCII well.
                let mut j = 0;
                while j < rune.len() {
                    if r < rune[j] {
                        return NO_MATCH;
                    }
                    if r <= rune[j + 1] {
                        return (j / 2) as isize;
                    }
                    j += 2;
                }
                return NO_MATCH;
            }
            _ => {}
        }

        // Otherwise binary search.
        let mut lo = 0;
        let mut hi = rune.len() / 2;
        while lo < hi {
            let m = lo + (hi - lo) / 2;
            let c = rune[2 * m];
            if c <= r {
                if r <= rune[2 * m + 1] {
                    return m as isize;
                }
                lo = m + 1;
            } else {
                hi = m;
            }
        }
        NO_MATCH
    }

    /// MatchEmptyWidth reports whether the instruction matches an empty string between the
    /// runes before and after. It should only be called when i.Op == InstEmptyWidth.
    // Go: regexp/syntax/prog.go:MatchEmptyWidth
    pub fn match_empty_width(&self, before: Rune, after: Rune) -> bool {
        match self.arg as EmptyOp {
            EMPTY_BEGIN_LINE => before == '\n' as Rune || before == -1,
            EMPTY_END_LINE => after == '\n' as Rune || after == -1,
            EMPTY_BEGIN_TEXT => before == -1,
            EMPTY_END_TEXT => after == -1,
            EMPTY_WORD_BOUNDARY => is_word_char(before) != is_word_char(after),
            EMPTY_NO_WORD_BOUNDARY => is_word_char(before) == is_word_char(after),
            _ => panic!("unknown empty width arg"),
        }
    }

    /// Go: `i.String()`.
    // Go: regexp/syntax/prog.go:String
    pub fn string(&self) -> String {
        let mut b = String::new();
        dump_inst(&mut b, self);
        b
    }
}

// Go: regexp/syntax/prog.go:dumpProg
fn dump_prog(b: &mut String, p: &Prog) {
    for (j, i) in p.inst.iter().enumerate() {
        let mut pc = j.to_string();
        if pc.len() < 3 {
            b.push_str(&"   "[pc.len()..]);
        }
        if j == p.start {
            pc.push('*');
        }
        b.push_str(&pc);
        b.push('\t');
        dump_inst(b, i);
        b.push('\n');
    }
}

/// `strconv.QuoteToASCII(string(runes))` (Go's `string([]rune)` turns invalid runes into
/// U+FFFD).
fn quote_runes(runes: &[Rune]) -> String {
    go_strconv::quote_to_ascii(utf8::from_runes(runes))
}

// Go: regexp/syntax/prog.go:dumpInst
fn dump_inst(b: &mut String, i: &Inst) {
    match i.op {
        InstOp::Alt => b.push_str(&format!("alt -> {}, {}", i.out, i.arg)),
        InstOp::AltMatch => b.push_str(&format!("altmatch -> {}, {}", i.out, i.arg)),
        InstOp::Capture => b.push_str(&format!("cap {} -> {}", i.arg, i.out)),
        InstOp::EmptyWidth => b.push_str(&format!("empty {} -> {}", i.arg, i.out)),
        InstOp::Match => b.push_str("match"),
        InstOp::Fail => b.push_str("fail"),
        InstOp::Nop => b.push_str(&format!("nop -> {}", i.out)),
        InstOp::Rune => {
            // (Go prints "rune <nil>" first for a nil slice; the port has no nil slices and
            // the compiler never emits an empty one.)
            b.push_str("rune ");
            b.push_str(&quote_runes(&i.rune));
            if (i.arg as Flags) & FOLD_CASE != 0 {
                b.push_str("/i");
            }
            b.push_str(&format!(" -> {}", i.out));
        }
        InstOp::Rune1 => {
            b.push_str("rune1 ");
            b.push_str(&quote_runes(&i.rune));
            b.push_str(&format!(" -> {}", i.out));
        }
        InstOp::RuneAny => b.push_str(&format!("any -> {}", i.out)),
        InstOp::RuneAnyNotNL => b.push_str(&format!("anynotnl -> {}", i.out)),
    }
}
