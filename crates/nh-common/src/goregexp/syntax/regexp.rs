//! Port of Go's `regexp/syntax/regexp.go` (go1.27.1): the syntax tree, `Equal`, `String`,
//! `MaxCap` and `CapNames`.

use std::collections::HashMap;

use go_unicode::{MAX_RUNE, Rune, is_print, simple_fold};

use super::parse::{MAX_FOLD, MIN_FOLD, in_char_class};

/// Go: `syntax.Flags`.
pub type Flags = u16;

/// case-insensitive match
pub const FOLD_CASE: Flags = 1;
/// treat pattern as literal string
pub const LITERAL: Flags = 1 << 1;
/// allow character classes like `[^a-z]` and `[[:space:]]` to match newline
pub const CLASS_NL: Flags = 1 << 2;
/// allow `.` to match newline
pub const DOT_NL: Flags = 1 << 3;
/// treat `^` and `$` as only matching at beginning and end of text
pub const ONE_LINE: Flags = 1 << 4;
/// make repetition operators default to non-greedy
pub const NON_GREEDY: Flags = 1 << 5;
/// allow Perl extensions
pub const PERL_X: Flags = 1 << 6;
/// allow `\p{Han}`, `\P{Han}` for Unicode group and negation
pub const UNICODE_GROUPS: Flags = 1 << 7;
/// regexp OpEndText was `$`, not `\z`
pub const WAS_DOLLAR: Flags = 1 << 8;
/// regexp contains no counted repetition
pub const SIMPLE: Flags = 1 << 9;

pub const MATCH_NL: Flags = CLASS_NL | DOT_NL;
/// as close to Perl as possible
pub const PERL: Flags = CLASS_NL | ONE_LINE | PERL_X | UNICODE_GROUPS;
/// POSIX syntax
pub const POSIX: Flags = 0;

/// Go: `syntax.Op`, a single regular expression operator. The discriminants are Go's values;
/// the two parser pseudo-ops (`opLeftParen`, `opVerticalBar`) follow `opPseudo` (128).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(u8)]
pub enum Op {
    /// matches no strings
    #[default]
    NoMatch = 1,
    /// matches empty string
    EmptyMatch,
    /// matches Runes sequence
    Literal,
    /// matches Runes interpreted as range pair list
    CharClass,
    /// matches any character except newline
    AnyCharNotNL,
    /// matches any character
    AnyChar,
    /// matches empty string at beginning of line
    BeginLine,
    /// matches empty string at end of line
    EndLine,
    /// matches empty string at beginning of text
    BeginText,
    /// matches empty string at end of text
    EndText,
    /// matches word boundary `\b`
    WordBoundary,
    /// matches word non-boundary `\B`
    NoWordBoundary,
    /// capturing subexpression with index Cap, optional name Name
    Capture,
    /// matches Sub[0] zero or more times
    Star,
    /// matches Sub[0] one or more times
    Plus,
    /// matches Sub[0] zero or one times
    Quest,
    /// matches Sub[0] at least Min times, at most Max (Max == -1 is no limit)
    Repeat,
    /// matches concatenation of Subs
    Concat,
    /// matches alternation of Subs
    Alternate,
    /// Go: `opLeftParen` (parser pseudo-op).
    LeftParen = 128,
    /// Go: `opVerticalBar` (parser pseudo-op).
    VerticalBar = 129,
}

/// Go: `opPseudo`.
pub(crate) const OP_PSEUDO: Op = Op::LeftParen;

impl Op {
    /// Go: `Op.String()` (the stringer output).
    pub fn string(self) -> String {
        match self {
            Op::NoMatch => "NoMatch".into(),
            Op::EmptyMatch => "EmptyMatch".into(),
            Op::Literal => "Literal".into(),
            Op::CharClass => "CharClass".into(),
            Op::AnyCharNotNL => "AnyCharNotNL".into(),
            Op::AnyChar => "AnyChar".into(),
            Op::BeginLine => "BeginLine".into(),
            Op::EndLine => "EndLine".into(),
            Op::BeginText => "BeginText".into(),
            Op::EndText => "EndText".into(),
            Op::WordBoundary => "WordBoundary".into(),
            Op::NoWordBoundary => "NoWordBoundary".into(),
            Op::Capture => "Capture".into(),
            Op::Star => "Star".into(),
            Op::Plus => "Plus".into(),
            Op::Quest => "Quest".into(),
            Op::Repeat => "Repeat".into(),
            Op::Concat => "Concat".into(),
            Op::Alternate => "Alternate".into(),
            Op::LeftParen => "opPseudo".into(),
            Op::VerticalBar => "Op(129)".into(),
        }
    }
}

/// Go: `syntax.Regexp`, a node in a regular expression syntax tree. Go shares `Sub` nodes
/// between parents after `Simplify`; here every parent owns a copy (the compiled program is the
/// same: Go compiles a shared node once per reference).
#[derive(Clone, Debug, Default)]
pub struct Regexp {
    /// operator
    pub op: Op,
    pub flags: Flags,
    /// subexpressions, if any
    pub sub: Vec<Regexp>,
    /// matched runes, for OpLiteral, OpCharClass
    pub rune: Vec<Rune>,
    /// min, max for OpRepeat
    pub min: i64,
    pub max: i64,
    /// capturing index, for OpCapture
    pub cap: i64,
    /// capturing name, for OpCapture
    pub name: String,
    /// The parser's `p.size[re]` entry (Go keys the map by pointer; see `parse.rs`).
    pub(crate) size_cache: Option<i64>,
    /// The parser's `p.height[re]` entry.
    pub(crate) height_cache: Option<i64>,
}

impl Regexp {
    pub(crate) fn with_op(op: Op) -> Regexp {
        Regexp {
            op,
            ..Default::default()
        }
    }

    /// Equal reports whether x and y have identical structure.
    // Go: regexp/syntax/regexp.go:Equal
    pub fn equal(&self, y: &Regexp) -> bool {
        let x = self;
        if x.op != y.op {
            return false;
        }
        match x.op {
            Op::EndText => {
                // The parse flags remember whether this is \z or \Z.
                if x.flags & WAS_DOLLAR != y.flags & WAS_DOLLAR {
                    return false;
                }
            }
            Op::Literal | Op::CharClass => {
                return x.flags & FOLD_CASE == y.flags & FOLD_CASE && x.rune == y.rune;
            }
            Op::Alternate | Op::Concat => {
                return x.sub.len() == y.sub.len()
                    && x.sub.iter().zip(&y.sub).all(|(a, b)| a.equal(b));
            }
            Op::Star | Op::Plus | Op::Quest => {
                if x.flags & NON_GREEDY != y.flags & NON_GREEDY || !x.sub[0].equal(&y.sub[0]) {
                    return false;
                }
            }
            Op::Repeat => {
                if x.flags & NON_GREEDY != y.flags & NON_GREEDY
                    || x.min != y.min
                    || x.max != y.max
                    || !x.sub[0].equal(&y.sub[0])
                {
                    return false;
                }
            }
            Op::Capture => {
                if x.cap != y.cap || x.name != y.name || !x.sub[0].equal(&y.sub[0]) {
                    return false;
                }
            }
            _ => {}
        }
        true
    }

    /// Go: `re.String()`: the Perl syntax for the regular expression.
    // Go: regexp/syntax/regexp.go:String
    pub fn string(&self) -> String {
        let mut b = String::new();
        let mut flags: Option<FlagMap> = None;
        let (mut must, cant) = calc_flags(self, &mut flags);
        must |= (cant & !FLAG_I) << NEG_SHIFT;
        if must != 0 {
            must |= FLAG_OFF;
        }
        let empty = FlagMap::new();
        write_regexp(&mut b, self, must, flags.as_ref().unwrap_or(&empty));
        b
    }

    /// MaxCap walks the regexp to find the maximum capture index.
    // Go: regexp/syntax/regexp.go:MaxCap
    pub fn max_cap(&self) -> i64 {
        let mut m = 0;
        if self.op == Op::Capture {
            m = self.cap;
        }
        for sub in &self.sub {
            let n = sub.max_cap();
            if m < n {
                m = n;
            }
        }
        m
    }

    /// CapNames walks the regexp to find the names of capturing groups.
    // Go: regexp/syntax/regexp.go:CapNames
    pub fn cap_names(&self) -> Vec<String> {
        let mut names = vec![String::new(); (self.max_cap() + 1) as usize];
        self.cap_names_into(&mut names);
        names
    }

    // Go: regexp/syntax/regexp.go:capNames
    fn cap_names_into(&self, names: &mut [String]) {
        if self.op == Op::Capture {
            names[self.cap as usize] = self.name.clone();
        }
        for sub in &self.sub {
            sub.cap_names_into(names);
        }
    }
}

/// printFlags is a bit set indicating which flags (including non-capturing parens) to print
/// around a regexp.
type PrintFlags = u8;

const FLAG_I: PrintFlags = 1; // (?i:
const FLAG_M: PrintFlags = 1 << 1; // (?m:
const FLAG_S: PrintFlags = 1 << 2; // (?s:
const FLAG_OFF: PrintFlags = 1 << 3; // )
const FLAG_PREC: PrintFlags = 1 << 4; // (?: )
const NEG_SHIFT: u32 = 5; // flagI<<negShift is (?-i:

/// Go's `map[*Regexp]printFlags` (keyed by node address; the tree is not moved while printing).
type FlagMap = HashMap<*const Regexp, PrintFlags>;

/// addSpan enables the flags f around start..last, by setting flags[start] = f and
/// flags[last] = flagOff.
// Go: regexp/syntax/regexp.go:addSpan
fn add_span(start: &Regexp, last: &Regexp, f: PrintFlags, flags: &mut Option<FlagMap>) {
    let m = flags.get_or_insert_with(FlagMap::new);
    m.insert(start as *const Regexp, f);
    *m.entry(last as *const Regexp).or_insert(0) |= FLAG_OFF; // maybe start==last
}

/// calcFlags calculates the flags to print around each subexpression in re, storing that
/// information in (*flags)[sub] for each affected subexpression. It also calculates the flags
/// that must be active or can't be active around re and returns those flags.
// Go: regexp/syntax/regexp.go:calcFlags
fn calc_flags(re: &Regexp, flags: &mut Option<FlagMap>) -> (PrintFlags, PrintFlags) {
    match re.op {
        Op::Literal => {
            // If literal is fold-sensitive, return (flagI, 0) or (0, flagI) according to
            // whether (?i) is active. If literal is not fold-sensitive, return 0, 0.
            for &r in &re.rune {
                if (MIN_FOLD..=MAX_FOLD).contains(&r) && simple_fold(r) != r {
                    if re.flags & FOLD_CASE != 0 {
                        return (FLAG_I, 0);
                    } else {
                        return (0, FLAG_I);
                    }
                }
            }
            (0, 0)
        }
        Op::CharClass => {
            // If literal is fold-sensitive, return 0, flagI - (?i) has been compiled out.
            // If literal is not fold-sensitive, return 0, 0.
            let mut i = 0;
            while i + 1 < re.rune.len() {
                let lo = MIN_FOLD.max(re.rune[i]);
                let hi = MAX_FOLD.min(re.rune[i + 1]);
                let mut r = lo;
                while r <= hi {
                    let mut f = simple_fold(r);
                    while f != r {
                        if !(lo..=hi).contains(&f) && !in_char_class(f, &re.rune) {
                            return (0, FLAG_I);
                        }
                        f = simple_fold(f);
                    }
                    r += 1;
                }
                i += 2;
            }
            (0, 0)
        }
        Op::AnyCharNotNL => (0, FLAG_S),            // (?-s).
        Op::AnyChar => (FLAG_S, 0),                 // (?s).
        Op::BeginLine | Op::EndLine => (FLAG_M, 0), // (?m)^ (?m)$
        Op::EndText => {
            if re.flags & WAS_DOLLAR != 0 {
                // (?-m)$
                return (0, FLAG_M);
            }
            (0, 0)
        }
        Op::Capture | Op::Star | Op::Plus | Op::Quest | Op::Repeat => calc_flags(&re.sub[0], flags),
        Op::Concat | Op::Alternate => {
            // Gather the must and cant for each subexpression. When we find a conflicting
            // subexpression, insert the necessary flags around the previously identified span
            // and start over.
            let mut must: PrintFlags = 0;
            let mut cant: PrintFlags = 0;
            let mut all_cant: PrintFlags = 0;
            let mut start = 0;
            let mut last = 0;
            let mut did = false;
            for (i, sub) in re.sub.iter().enumerate() {
                let (sub_must, sub_cant) = calc_flags(sub, flags);
                if must & sub_cant != 0 || sub_must & cant != 0 {
                    if must != 0 {
                        add_span(&re.sub[start], &re.sub[last], must, flags);
                    }
                    must = 0;
                    cant = 0;
                    start = i;
                    did = true;
                }
                must |= sub_must;
                cant |= sub_cant;
                all_cant |= sub_cant;
                if sub_must != 0 {
                    last = i;
                }
                if must == 0 && start == i {
                    start += 1;
                }
            }
            if !did {
                // No conflicts: pass the accumulated must and cant upward.
                return (must, cant);
            }
            if must != 0 {
                // Conflicts found; need to finish final span.
                add_span(&re.sub[start], &re.sub[last], must, flags);
            }
            (0, all_cant)
        }
        _ => (0, 0),
    }
}

/// writeRegexp writes the Perl syntax for the regular expression re to b.
// Go: regexp/syntax/regexp.go:writeRegexp
fn write_regexp(b: &mut String, re: &Regexp, mut f: PrintFlags, flags: &FlagMap) {
    f |= flags.get(&(re as *const Regexp)).copied().unwrap_or(0);
    if f & FLAG_PREC != 0 && f & !(FLAG_OFF | FLAG_PREC) != 0 && f & FLAG_OFF != 0 {
        // flagPrec is redundant with other flags being added and terminated
        f &= !FLAG_PREC;
    }
    if f & !(FLAG_OFF | FLAG_PREC) != 0 {
        b.push_str("(?");
        if f & FLAG_I != 0 {
            b.push('i');
        }
        if f & FLAG_M != 0 {
            b.push('m');
        }
        if f & FLAG_S != 0 {
            b.push('s');
        }
        if f & ((FLAG_M | FLAG_S) << NEG_SHIFT) != 0 {
            b.push('-');
            if f & (FLAG_M << NEG_SHIFT) != 0 {
                b.push('m');
            }
            if f & (FLAG_S << NEG_SHIFT) != 0 {
                b.push('s');
            }
        }
        b.push(':');
    }
    // Go defers `)` for flagOff (outer) and flagPrec (inner): the inner closes first.
    if f & FLAG_PREC != 0 {
        b.push_str("(?:");
    }

    match re.op {
        Op::NoMatch => b.push_str(r"[^\x00-\x{10FFFF}]"),
        Op::EmptyMatch => b.push_str("(?:)"),
        Op::Literal => {
            for &r in &re.rune {
                escape(b, r, false);
            }
        }
        Op::CharClass => {
            if !re.rune.len().is_multiple_of(2) {
                b.push_str("[invalid char class]");
            } else {
                b.push('[');
                if re.rune.is_empty() {
                    b.push_str(r"^\x00-\x{10FFFF}");
                } else if re.rune[0] == 0
                    && re.rune[re.rune.len() - 1] == MAX_RUNE
                    && re.rune.len() > 2
                {
                    // Contains 0 and MaxRune. Probably a negated class. Print the gaps.
                    b.push('^');
                    let mut i = 1;
                    while i < re.rune.len() - 1 {
                        let (lo, hi) = (re.rune[i] + 1, re.rune[i + 1] - 1);
                        escape(b, lo, lo == '-' as Rune);
                        if lo != hi {
                            if hi != lo + 1 {
                                b.push('-');
                            }
                            escape(b, hi, hi == '-' as Rune);
                        }
                        i += 2;
                    }
                } else {
                    let mut i = 0;
                    while i < re.rune.len() {
                        let (lo, hi) = (re.rune[i], re.rune[i + 1]);
                        escape(b, lo, lo == '-' as Rune);
                        if lo != hi {
                            if hi != lo + 1 {
                                b.push('-');
                            }
                            escape(b, hi, hi == '-' as Rune);
                        }
                        i += 2;
                    }
                }
                b.push(']');
            }
        }
        Op::AnyCharNotNL | Op::AnyChar => b.push('.'),
        Op::BeginLine => b.push('^'),
        Op::EndLine => b.push('$'),
        Op::BeginText => b.push_str(r"\A"),
        Op::EndText => {
            if re.flags & WAS_DOLLAR != 0 {
                b.push('$');
            } else {
                b.push_str(r"\z");
            }
        }
        Op::WordBoundary => b.push_str(r"\b"),
        Op::NoWordBoundary => b.push_str(r"\B"),
        Op::Capture => {
            if !re.name.is_empty() {
                b.push_str("(?P<");
                b.push_str(&re.name);
                b.push('>');
            } else {
                b.push('(');
            }
            if re.sub[0].op != Op::EmptyMatch {
                let f0 = flags
                    .get(&(&re.sub[0] as *const Regexp))
                    .copied()
                    .unwrap_or(0);
                write_regexp(b, &re.sub[0], f0, flags);
            }
            b.push(')');
        }
        Op::Star | Op::Plus | Op::Quest | Op::Repeat => {
            let mut p: PrintFlags = 0;
            let sub = &re.sub[0];
            if sub.op > Op::Capture || sub.op == Op::Literal && sub.rune.len() > 1 {
                p = FLAG_PREC;
            }
            write_regexp(b, sub, p, flags);

            match re.op {
                Op::Star => b.push('*'),
                Op::Plus => b.push('+'),
                Op::Quest => b.push('?'),
                Op::Repeat => {
                    b.push('{');
                    b.push_str(&re.min.to_string());
                    if re.max != re.min {
                        b.push(',');
                        if re.max >= 0 {
                            b.push_str(&re.max.to_string());
                        }
                    }
                    b.push('}');
                }
                _ => {}
            }
            if re.flags & NON_GREEDY != 0 {
                b.push('?');
            }
        }
        Op::Concat => {
            for sub in &re.sub {
                let mut p: PrintFlags = 0;
                if sub.op == Op::Alternate {
                    p = FLAG_PREC;
                }
                write_regexp(b, sub, p, flags);
            }
        }
        Op::Alternate => {
            for (i, sub) in re.sub.iter().enumerate() {
                if i > 0 {
                    b.push('|');
                }
                write_regexp(b, sub, 0, flags);
            }
        }
        _ => {
            b.push_str(&format!("<invalid op{}>", re.op as u8));
        }
    }
    if f & FLAG_PREC != 0 {
        b.push(')');
    }
    if f & FLAG_OFF != 0 {
        b.push(')');
    }
}

const META: &str = r"\.+*?()|[]{}^$";

// Go: regexp/syntax/regexp.go:escape
fn escape(b: &mut String, r: Rune, force: bool) {
    if is_print(r) {
        let c = char::from_u32(r as u32).unwrap_or('\u{fffd}');
        if META.contains(c) || force {
            b.push('\\');
        }
        b.push(c);
        return;
    }

    match r {
        0x07 => b.push_str(r"\a"),
        0x0c => b.push_str(r"\f"),
        0x0a => b.push_str(r"\n"),
        0x0d => b.push_str(r"\r"),
        0x09 => b.push_str(r"\t"),
        0x0b => b.push_str(r"\v"),
        _ => {
            if r < 0x100 {
                b.push_str(r"\x");
                let s = go_strconv::format_int(i64::from(r), 16);
                if s.len() == 1 {
                    b.push('0');
                }
                b.push_str(&s);
                return;
            }
            b.push_str(r"\x{");
            b.push_str(&go_strconv::format_int(i64::from(r), 16));
            b.push('}');
        }
    }
}
