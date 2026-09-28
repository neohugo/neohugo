//! Port of Go's `regexp/syntax/parse.go` (go1.27.1): the RE2 parser.
//!
//! Go keeps the parser's size and height caches in maps keyed by `*Regexp` and recycles nodes
//! through a free list; a recycled node keeps its (stale) size entry. The port stores both
//! caches in the node and passes a recycled node's size entry on to the next node the free
//! list hands out, so the `expression too large` / `expression nests too deeply` decisions are
//! Go's.

use go_unicode::tables::{CATEGORIES, CATEGORY_ALIASES, FOLD_CATEGORY, FOLD_SCRIPT, SCRIPTS};
use go_unicode::utf8::{self, RUNE_ERROR, RUNE_SELF};
use go_unicode::{MAX_RUNE, Range16, Range32, RangeTable, Rune, lookup, simple_fold};

use super::perl_groups::{CharGroup, perl_group, posix_group};
use super::regexp::{
    CLASS_NL, DOT_NL, FOLD_CASE, Flags, LITERAL, NON_GREEDY, ONE_LINE, OP_PSEUDO, Op, PERL_X,
    Regexp, UNICODE_GROUPS, WAS_DOLLAR,
};

/// An Error describes a failure to parse a regular expression and gives the offending
/// expression.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: ErrorCode,
    /// The offending expression (a Go string: arbitrary bytes).
    pub expr: Vec<u8>,
}

impl Error {
    fn new(code: ErrorCode, expr: &[u8]) -> Error {
        Error {
            code,
            expr: expr.to_vec(),
        }
    }

    /// Go: `e.Error()`: "error parsing regexp: " + code + ": `" + expr + "`" (bytes).
    // Go: regexp/syntax/parse.go:Error
    pub fn error(&self) -> Vec<u8> {
        let mut b = b"error parsing regexp: ".to_vec();
        b.extend_from_slice(self.code.string().as_bytes());
        b.extend_from_slice(b": `");
        b.extend_from_slice(&self.expr);
        b.push(b'`');
        b
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.error()))
    }
}

impl std::error::Error for Error {}

/// An ErrorCode describes a failure to parse a regular expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    /// Unexpected error
    InternalError,
    InvalidCharClass,
    InvalidCharRange,
    InvalidEscape,
    InvalidNamedCapture,
    InvalidPerlOp,
    InvalidRepeatOp,
    InvalidRepeatSize,
    InvalidUTF8,
    MissingBracket,
    MissingParen,
    MissingRepeatArgument,
    TrailingBackslash,
    UnexpectedParen,
    NestingDepth,
    Large,
}

impl ErrorCode {
    /// Go: `ErrorCode.String()`.
    pub fn string(self) -> &'static str {
        match self {
            ErrorCode::InternalError => "regexp/syntax: internal error",
            ErrorCode::InvalidCharClass => "invalid character class",
            ErrorCode::InvalidCharRange => "invalid character class range",
            ErrorCode::InvalidEscape => "invalid escape sequence",
            ErrorCode::InvalidNamedCapture => "invalid named capture",
            ErrorCode::InvalidPerlOp => "invalid or unsupported Perl syntax",
            ErrorCode::InvalidRepeatOp => "invalid nested repetition operator",
            ErrorCode::InvalidRepeatSize => "invalid repeat count",
            ErrorCode::InvalidUTF8 => "invalid UTF-8",
            ErrorCode::MissingBracket => "missing closing ]",
            ErrorCode::MissingParen => "missing closing )",
            ErrorCode::MissingRepeatArgument => "missing argument to repetition operator",
            ErrorCode::TrailingBackslash => "trailing backslash at end of expression",
            ErrorCode::UnexpectedParen => "unexpected )",
            ErrorCode::NestingDepth => "expression nests too deeply",
            ErrorCode::Large => "expression too large",
        }
    }
}

type PResult<T> = std::result::Result<T, Error>;

/// maxHeight is the maximum height of a regexp parse tree.
const MAX_HEIGHT: i64 = 1000;

/// maxSize is the maximum size of a compiled regexp in Insts.
const MAX_SIZE: i64 = (128 << 20) / INST_SIZE;
const INST_SIZE: i64 = 5 * 8; // byte, 2 uint32, slice is 5 64-bit words

/// maxRunes is the maximum number of runes allowed in a regexp tree counting the runes in all
/// the nodes.
const MAX_RUNES: i64 = (128 << 20) / RUNE_SIZE;
const RUNE_SIZE: i64 = 4; // rune is int32

struct Parser<'a> {
    /// parse mode flags
    flags: Flags,
    /// stack of parsed expressions
    stack: Vec<Regexp>,
    /// Go's free list `p.free`: the size-cache entry each recycled node still has in `p.size`
    /// (LIFO, like the list).
    free: Vec<Option<i64>>,
    /// number of capturing groups seen
    num_cap: i64,
    whole_regexp: &'a [u8],
    /// temporary char class work space
    tmp_class: Vec<Rune>,
    /// number of regexps allocated
    num_regexp: i64,
    /// number of runes in char classes
    num_runes: i64,
    /// product of all repetitions seen
    repeats: i64,
    /// `p.height != nil`
    height_on: bool,
    /// `p.size != nil`
    size_on: bool,
}

impl<'a> Parser<'a> {
    // Go: regexp/syntax/parse.go:newRegexp
    fn new_regexp(&mut self, op: Op) -> Regexp {
        let mut re = Regexp::with_op(op);
        if let Some(stale) = self.free.pop() {
            // A recycled node: `*re = Regexp{}` but its p.size entry stays.
            re.size_cache = stale;
        } else {
            self.num_regexp += 1;
        }
        re
    }

    // Go: regexp/syntax/parse.go:reuse
    fn reuse(&mut self, re: Regexp) {
        // Go deletes the p.height entry; the p.size entry stays with the pointer.
        self.free.push(re.size_cache);
    }

    /// checkLimits of `re`, which is not on the stack (the stack entries are checked by
    /// [`Parser::check_limits_top`]).
    // Go: regexp/syntax/parse.go:checkLimits
    fn check_limits(&mut self, re: &mut Regexp) -> PResult<()> {
        if self.num_runes > MAX_RUNES {
            return Err(Error::new(ErrorCode::Large, self.whole_regexp));
        }
        self.check_size(re)?;
        self.check_height(re)
    }

    /// checkLimits of the top of the stack. Go's belated cache population walks the whole
    /// stack, `re` included; computing `re` last instead gives the same caches and errors.
    fn check_limits_top(&mut self) -> PResult<()> {
        let mut re = self.stack.pop().expect("non-empty stack");
        let r = self.check_limits(&mut re);
        self.stack.push(re);
        r
    }

    // Go: regexp/syntax/parse.go:checkSize
    fn check_size(&mut self, re: &mut Regexp) -> PResult<()> {
        if !self.size_on {
            // We haven't started tracking size yet. Do a relatively cheap check to see if we
            // need to start. Maintain the product of all the repeats we've seen and don't
            // track if the total number of regexp nodes we've seen times the repeat product is
            // in budget.
            if self.repeats == 0 {
                self.repeats = 1;
            }
            if re.op == Op::Repeat {
                let mut n = re.max;
                if n == -1 {
                    n = re.min;
                }
                if n <= 0 {
                    n = 1;
                }
                if n > MAX_SIZE / self.repeats {
                    self.repeats = MAX_SIZE;
                } else {
                    self.repeats *= n;
                }
            }
            if self.num_regexp < MAX_SIZE / self.repeats {
                return Ok(());
            }

            // We need to start tracking size. Make the map and belatedly populate it with
            // info about everything we've constructed so far.
            self.size_on = true;
            let mut stack = std::mem::take(&mut self.stack);
            let r = stack.iter_mut().try_for_each(|s| self.check_size(s));
            self.stack = stack;
            r?;
        }

        if self.calc_size(re, true) > MAX_SIZE {
            return Err(Error::new(ErrorCode::Large, self.whole_regexp));
        }
        Ok(())
    }

    // Go: regexp/syntax/parse.go:calcSize
    fn calc_size(&mut self, re: &mut Regexp, force: bool) -> i64 {
        if !force && let Some(size) = re.size_cache {
            return size;
        }

        let mut size: i64 = 0;
        match re.op {
            Op::Literal => size = re.rune.len() as i64,
            Op::Capture | Op::Star => {
                // star can be 1+ or 2+; assume 2 pessimistically
                size = 2 + self.calc_size(&mut re.sub[0], false);
            }
            Op::Plus | Op::Quest => size = 1 + self.calc_size(&mut re.sub[0], false),
            Op::Concat => {
                for sub in &mut re.sub {
                    size += self.calc_size(sub, false);
                }
            }
            Op::Alternate => {
                for sub in &mut re.sub {
                    size += self.calc_size(sub, false);
                }
                if re.sub.len() > 1 {
                    size += re.sub.len() as i64 - 1;
                }
            }
            Op::Repeat => {
                let sub = self.calc_size(&mut re.sub[0], false);
                if re.max == -1 {
                    if re.min == 0 {
                        size = 2 + sub; // x*
                    } else {
                        size = 1 + re.min * sub; // xxx+
                    }
                } else {
                    // x{2,5} = xx(x(x(x)?)?)?
                    size = re.max * sub + (re.max - re.min);
                }
            }
            _ => {}
        }

        size = size.max(1);
        re.size_cache = Some(size);
        size
    }

    // Go: regexp/syntax/parse.go:checkHeight
    fn check_height(&mut self, re: &mut Regexp) -> PResult<()> {
        if self.num_regexp < MAX_HEIGHT {
            return Ok(());
        }
        if !self.height_on {
            self.height_on = true;
            let mut stack = std::mem::take(&mut self.stack);
            let r = stack.iter_mut().try_for_each(|s| self.check_height(s));
            self.stack = stack;
            r?;
        }
        if calc_height(re, true) > MAX_HEIGHT {
            return Err(Error::new(ErrorCode::NestingDepth, self.whole_regexp));
        }
        Ok(())
    }

    // Parse stack manipulation.

    /// push pushes the regexp re onto the parse stack (Go returns it, or nil when it was merged
    /// into the literal below it).
    // Go: regexp/syntax/parse.go:push
    fn push(&mut self, mut re: Regexp) -> PResult<()> {
        self.num_runes += re.rune.len() as i64;
        if re.op == Op::CharClass && re.rune.len() == 2 && re.rune[0] == re.rune[1] {
            // Single rune.
            if self.maybe_concat(re.rune[0], self.flags & !FOLD_CASE) {
                return Ok(());
            }
            re.op = Op::Literal;
            re.rune.truncate(1);
            re.flags = self.flags & !FOLD_CASE;
        } else if re.op == Op::CharClass
            && re.rune.len() == 4
            && re.rune[0] == re.rune[1]
            && re.rune[2] == re.rune[3]
            && simple_fold(re.rune[0]) == re.rune[2]
            && simple_fold(re.rune[2]) == re.rune[0]
            || re.op == Op::CharClass
                && re.rune.len() == 2
                && re.rune[0] + 1 == re.rune[1]
                && simple_fold(re.rune[0]) == re.rune[1]
                && simple_fold(re.rune[1]) == re.rune[0]
        {
            // Case-insensitive rune like [Aa] or [Δδ].
            if self.maybe_concat(re.rune[0], self.flags | FOLD_CASE) {
                return Ok(());
            }

            // Rewrite as (case-insensitive) literal.
            re.op = Op::Literal;
            re.rune.truncate(1);
            re.flags = self.flags | FOLD_CASE;
        } else {
            // Incremental concatenation.
            self.maybe_concat(-1, 0);
        }

        self.stack.push(re);
        self.check_limits_top()
    }

    /// maybeConcat implements incremental concatenation of literal runes into string nodes.
    /// If r >= 0 and there's a node left over, maybeConcat uses it to push r with the given
    /// flags. maybeConcat reports whether r was pushed.
    // Go: regexp/syntax/parse.go:maybeConcat
    fn maybe_concat(&mut self, r: Rune, flags: Flags) -> bool {
        let n = self.stack.len();
        if n < 2 {
            return false;
        }

        {
            let re1 = &self.stack[n - 1];
            let re2 = &self.stack[n - 2];
            if re1.op != Op::Literal
                || re2.op != Op::Literal
                || re1.flags & FOLD_CASE != re2.flags & FOLD_CASE
            {
                return false;
            }
        }

        // Push re1 into re2.
        let (lo, hi) = self.stack.split_at_mut(n - 1);
        let re2 = &mut lo[n - 2];
        let re1 = &mut hi[0];
        re2.rune.extend_from_slice(&re1.rune);

        // Reuse re1 if possible.
        if r >= 0 {
            re1.rune.clear();
            re1.rune.push(r);
            re1.flags = flags;
            return true;
        }

        let re1 = self.stack.pop().expect("n >= 2");
        self.reuse(re1);
        false // did not push r
    }

    /// literal pushes a literal regexp for the rune r on the stack.
    // Go: regexp/syntax/parse.go:literal
    fn literal(&mut self, mut r: Rune) -> PResult<()> {
        let mut re = self.new_regexp(Op::Literal);
        re.flags = self.flags;
        if self.flags & FOLD_CASE != 0 {
            r = min_fold_rune(r);
        }
        re.rune = vec![r];
        self.push(re)
    }

    /// op pushes a regexp with the given op onto the stack (Go returns it for the caller to
    /// set `Cap` or `Flags`; `set` does that before the push, which does not look at them).
    // Go: regexp/syntax/parse.go:op
    fn op_with(&mut self, op: Op, set: impl FnOnce(&mut Regexp)) -> PResult<()> {
        let mut re = self.new_regexp(op);
        re.flags = self.flags;
        set(&mut re);
        self.push(re)
    }

    fn op(&mut self, op: Op) -> PResult<()> {
        self.op_with(op, |_| {})
    }

    /// repeat replaces the top stack element with itself repeated according to op, min, max.
    /// before is the regexp suffix starting at the repetition operator. after is the regexp
    /// suffix following after the repetition operator. repeat returns an updated 'after' and an
    /// error, if any.
    // Go: regexp/syntax/parse.go:repeat
    fn repeat(
        &mut self,
        op: Op,
        min: i64,
        max: i64,
        before: &'a [u8],
        mut after: &'a [u8],
        last_repeat: &'a [u8],
    ) -> PResult<&'a [u8]> {
        let mut flags = self.flags;
        if self.flags & PERL_X != 0 {
            if !after.is_empty() && after[0] == b'?' {
                after = &after[1..];
                flags ^= NON_GREEDY;
            }
            if !last_repeat.is_empty() {
                // In Perl it is not allowed to stack repetition operators: a** is a syntax
                // error, not a doubled star, and a++ means something else entirely, which we
                // don't support!
                return Err(Error::new(
                    ErrorCode::InvalidRepeatOp,
                    &last_repeat[..last_repeat.len() - after.len()],
                ));
            }
        }
        let n = self.stack.len();
        if n == 0 {
            return Err(Error::new(
                ErrorCode::MissingRepeatArgument,
                &before[..before.len() - after.len()],
            ));
        }
        if self.stack[n - 1].op >= OP_PSEUDO {
            return Err(Error::new(
                ErrorCode::MissingRepeatArgument,
                &before[..before.len() - after.len()],
            ));
        }
        let sub = self.stack.pop().expect("n > 0");

        let mut re = self.new_regexp(op);
        re.min = min;
        re.max = max;
        re.flags = flags;
        re.sub = vec![sub];
        self.stack.push(re);
        self.check_limits_top()?;

        if op == Op::Repeat
            && (min >= 2 || max >= 2)
            && !repeat_is_valid(self.stack.last().expect("pushed"), 1000)
        {
            return Err(Error::new(
                ErrorCode::InvalidRepeatSize,
                &before[..before.len() - after.len()],
            ));
        }

        Ok(after)
    }

    /// concat replaces the top of the stack (above the topmost '|' or '(') with its
    /// concatenation.
    // Go: regexp/syntax/parse.go:concat
    fn concat(&mut self) -> PResult<()> {
        self.maybe_concat(-1, 0);

        // Scan down to find pseudo-operator | or (.
        let mut i = self.stack.len();
        while i > 0 && self.stack[i - 1].op < OP_PSEUDO {
            i -= 1;
        }
        let subs = self.stack.split_off(i);

        // Empty concatenation is special case.
        if subs.is_empty() {
            let re = self.new_regexp(Op::EmptyMatch);
            return self.push(re);
        }

        let re = self.collapse(subs, Op::Concat)?;
        self.push(re)
    }

    /// alternate replaces the top of the stack (above the topmost '(') with its alternation.
    // Go: regexp/syntax/parse.go:alternate
    fn alternate(&mut self) -> PResult<()> {
        // Scan down to find pseudo-operator (. There are no | above (.
        let mut i = self.stack.len();
        while i > 0 && self.stack[i - 1].op < OP_PSEUDO {
            i -= 1;
        }
        let mut subs = self.stack.split_off(i);

        // Make sure top class is clean. All the others already are (see swapVerticalBar).
        if let Some(last) = subs.last_mut() {
            clean_alt(last);
        }

        // Empty alternate is special case (shouldn't happen but easy to handle).
        if subs.is_empty() {
            let re = self.new_regexp(Op::NoMatch);
            return self.push(re);
        }

        let re = self.collapse(subs, Op::Alternate)?;
        self.push(re)
    }

    /// collapse returns the result of applying op to sub. If sub contains op nodes, they all
    /// get hoisted up so that there is never a concat of a concat or an alternate of an
    /// alternate.
    // Go: regexp/syntax/parse.go:collapse
    fn collapse(&mut self, mut subs: Vec<Regexp>, op: Op) -> PResult<Regexp> {
        if subs.len() == 1 {
            return Ok(subs.pop().expect("one"));
        }
        let mut re = self.new_regexp(op);
        re.sub = Vec::new();
        for mut sub in subs {
            if sub.op == op {
                re.sub.append(&mut sub.sub);
                self.reuse(sub);
            } else {
                re.sub.push(sub);
            }
        }
        if op == Op::Alternate {
            let sub = std::mem::take(&mut re.sub);
            re.sub = self.factor(sub)?;
            if re.sub.len() == 1 {
                let only = re.sub.pop().expect("one");
                self.reuse(re);
                re = only;
            }
        }
        Ok(re)
    }

    /// factor factors common prefixes from the alternation list sub. It returns a replacement
    /// list and frees (passes to p.reuse) any removed *Regexps.
    ///
    /// For example, `ABC|ABD|AEF|BCX|BCY` simplifies by literal prefix extraction to
    /// `A(B(C|D)|EF)|BC(X|Y)` which simplifies by character class introduction to
    /// `A(B[CD]|EF)|BC[XY]`.
    // Go: regexp/syntax/parse.go:factor
    fn factor(&mut self, sub: Vec<Regexp>) -> PResult<Vec<Regexp>> {
        if sub.len() < 2 {
            return Ok(sub);
        }
        let mut sub: Vec<Option<Regexp>> = sub.into_iter().map(Some).collect();

        // Round 1: Factor out common literal prefixes.
        let mut str: Vec<Rune> = Vec::new();
        let mut strflags: Flags = 0;
        let mut start = 0;
        let mut out: Vec<Regexp> = Vec::new();
        let mut i = 0;
        while i <= sub.len() {
            // Invariant: sub[start:i] consists of regexps that all begin with str as modified
            // by strflags.
            let mut istr: Vec<Rune> = Vec::new();
            let mut iflags: Flags = 0;
            if i < sub.len() {
                let (s, f) = leading_string(sub[i].as_ref().expect("live"));
                istr = s.to_vec();
                iflags = f;
                if iflags == strflags {
                    let mut same = 0;
                    while same < str.len() && same < istr.len() && str[same] == istr[same] {
                        same += 1;
                    }
                    if same > 0 {
                        // Matches at least one rune in current range. Keep going around.
                        str.truncate(same);
                        i += 1;
                        continue;
                    }
                }
            }

            // Found end of a run with common leading literal string: sub[start:i] all begin
            // with str[:len(str)], but sub[i] does not even begin with str[0].
            //
            // Factor out common string and append factored expression to out.
            if i == start {
                // Nothing to do - run of length 0.
            } else if i == start + 1 {
                // Just one: don't bother factoring.
                out.push(sub[start].take().expect("live"));
            } else {
                // Construct factored form: prefix(suffix1|suffix2|...)
                let mut prefix = self.new_regexp(Op::Literal);
                prefix.flags = strflags;
                prefix.rune = str.clone();

                let mut run = Vec::with_capacity(i - start);
                for slot in sub.iter_mut().take(i).skip(start) {
                    let mut s = self.remove_leading_string(slot.take().expect("live"), str.len());
                    self.check_limits(&mut s)?;
                    run.push(s);
                }
                let suffix = self.collapse(run, Op::Alternate)?; // recurse

                let mut re = self.new_regexp(Op::Concat);
                re.sub = vec![prefix, suffix];
                out.push(re);
            }

            // Prepare for next iteration.
            start = i;
            str = istr;
            strflags = iflags;
            i += 1;
        }
        let mut sub: Vec<Option<Regexp>> = out.into_iter().map(Some).collect();

        // Round 2: Factor out common simple prefixes, just the first piece of each
        // concatenation. This will be good enough a lot of the time.
        //
        // Complex subexpressions (e.g. involving quantifiers) are not safe to factor because
        // that collapses their distinct paths through the automaton, which affects
        // correctness in some cases.
        start = 0;
        let mut out: Vec<Regexp> = Vec::new();
        // `first` is the leading regexp of sub[start] (Go keeps a pointer into it).
        let mut first_live = false;
        let mut i = 0;
        while i <= sub.len() {
            // Invariant: sub[start:i] consists of regexps that all begin with ifirst.
            let mut ifirst_live = false;
            if i < sub.len() {
                let ifirst = leading_regexp(sub[i].as_ref().expect("live"));
                ifirst_live = ifirst.is_some();
                if first_live && i != start {
                    let first =
                        leading_regexp(sub[start].as_ref().expect("live")).expect("first_live");
                    if let Some(ifirst) = ifirst
                        && first.equal(ifirst)
                        // first must be a character class OR a fixed repeat of a character
                        // class.
                        && (is_char_class(first)
                            || (first.op == Op::Repeat
                                && first.min == first.max
                                && is_char_class(&first.sub[0])))
                    {
                        i += 1;
                        continue;
                    }
                }
            }

            // Found end of a run with common leading regexp: sub[start:i] all begin with first
            // but sub[i] does not.
            //
            // Factor out common regexp and append factored expression to out.
            if i == start {
                // Nothing to do - run of length 0.
            } else if i == start + 1 {
                // Just one: don't bother factoring.
                out.push(sub[start].take().expect("live"));
            } else {
                // Construct factored form: prefix(suffix1|suffix2|...)
                let mut prefix: Option<Regexp> = None;
                let mut run = Vec::with_capacity(i - start);
                for (j, slot) in sub.iter_mut().enumerate().take(i).skip(start) {
                    let reuse = j != start; // prefix came from sub[start]
                    let (mut s, lead) =
                        self.remove_leading_regexp(slot.take().expect("live"), reuse);
                    if j == start {
                        prefix = lead;
                    }
                    self.check_limits(&mut s)?;
                    run.push(s);
                }
                let suffix = self.collapse(run, Op::Alternate)?; // recurse

                let mut re = self.new_regexp(Op::Concat);
                re.sub = vec![prefix.expect("leading regexp"), suffix];
                out.push(re);
            }

            // Prepare for next iteration.
            start = i;
            first_live = ifirst_live;
            i += 1;
        }
        let mut sub: Vec<Regexp> = out;

        // Round 3: Collapse runs of single literals into character classes.
        start = 0;
        let mut out: Vec<Regexp> = Vec::new();
        let mut taken: Vec<Option<Regexp>> = sub.drain(..).map(Some).collect();
        let mut i = 0;
        while i <= taken.len() {
            // Invariant: sub[start:i] consists of regexps that are either literal runes or
            // character classes.
            if i < taken.len() && is_char_class(taken[i].as_ref().expect("live")) {
                i += 1;
                continue;
            }

            // sub[i] is not a char or char class; emit char class for sub[start:i]...
            if i == start {
                // Nothing to do - run of length 0.
            } else if i == start + 1 {
                out.push(taken[start].take().expect("live"));
            } else {
                // Make new char class. Start with most complex regexp in sub[start].
                let mut max = start;
                for j in start + 1..i {
                    let (m, s) = (
                        taken[max].as_ref().expect("live"),
                        taken[j].as_ref().expect("live"),
                    );
                    if m.op < s.op || m.op == s.op && m.rune.len() < s.rune.len() {
                        max = j;
                    }
                }
                taken.swap(start, max);

                let mut dst = taken[start].take().expect("live");
                for slot in taken.iter_mut().take(i).skip(start + 1) {
                    let src = slot.take().expect("live");
                    merge_char_class(&mut dst, &src);
                    self.reuse(src);
                }
                clean_alt(&mut dst);
                out.push(dst);
            }

            // ... and then emit sub[i].
            if i < taken.len() {
                out.push(taken[i].take().expect("live"));
            }
            start = i + 1;
            i += 1;
        }
        let sub = out;

        // Round 4: Collapse runs of empty matches into a single empty match.
        let mut out: Vec<Regexp> = Vec::with_capacity(sub.len());
        let n = sub.len();
        let ops: Vec<Op> = sub.iter().map(|r| r.op).collect();
        for (i, re) in sub.into_iter().enumerate() {
            if i + 1 < n && ops[i] == Op::EmptyMatch && ops[i + 1] == Op::EmptyMatch {
                // Go drops the node without recycling it.
                continue;
            }
            out.push(re);
        }

        Ok(out)
    }

    /// removeLeadingString removes the first n leading runes from the beginning of re. It
    /// returns the replacement for re.
    // Go: regexp/syntax/parse.go:removeLeadingString
    fn remove_leading_string(&mut self, mut re: Regexp, n: usize) -> Regexp {
        if re.op == Op::Concat && !re.sub.is_empty() {
            // Removing a leading string in a concatenation might simplify the concatenation.
            let sub = std::mem::take(&mut re.sub[0]);
            let sub = self.remove_leading_string(sub, n);
            if sub.op == Op::EmptyMatch {
                self.reuse(sub);
                match re.sub.len() {
                    0 | 1 => {
                        // Impossible but handle.
                        re.op = Op::EmptyMatch;
                        re.sub = Vec::new();
                    }
                    2 => {
                        let second = re.sub.pop().expect("two");
                        re.sub.clear();
                        self.reuse(re);
                        re = second;
                    }
                    _ => {
                        re.sub.remove(0);
                    }
                }
            } else {
                re.sub[0] = sub;
            }
            return re;
        }

        if re.op == Op::Literal {
            re.rune.drain(..n.min(re.rune.len()));
            if re.rune.is_empty() {
                re.op = Op::EmptyMatch;
            }
        }
        re
    }

    /// removeLeadingRegexp removes the leading regexp in re. It returns the replacement for
    /// re, and the removed leading regexp when `reuse` is false (Go's caller keeps a pointer
    /// to it as the factored prefix).
    // Go: regexp/syntax/parse.go:removeLeadingRegexp
    fn remove_leading_regexp(&mut self, mut re: Regexp, reuse: bool) -> (Regexp, Option<Regexp>) {
        if re.op == Op::Concat && !re.sub.is_empty() {
            let lead = re.sub.remove(0);
            let kept = if reuse {
                self.reuse(lead);
                None
            } else {
                Some(lead)
            };
            match re.sub.len() {
                0 => {
                    re.op = Op::EmptyMatch;
                    re.sub = Vec::new();
                }
                1 => {
                    let only = re.sub.pop().expect("one");
                    self.reuse(re);
                    re = only;
                }
                _ => {}
            }
            return (re, kept);
        }
        let kept = if reuse {
            self.reuse(re);
            None
        } else {
            Some(re)
        };
        (self.new_regexp(Op::EmptyMatch), kept)
    }

    /// parseRepeat parses {min} (max=min) or {min,} (max=-1) or {min,max}. If s is not of that
    /// form, it returns ok == false. If s has the right form but the values are too big, it
    /// returns min == -1, ok == true.
    // Go: regexp/syntax/parse.go:parseRepeat
    fn parse_repeat(&self, s: &'a [u8]) -> Option<(i64, i64, &'a [u8])> {
        if s.is_empty() || s[0] != b'{' {
            return None;
        }
        let s = &s[1..];
        let (mut min, s) = parse_int(s)?;
        if s.is_empty() {
            return None;
        }
        let max;
        let mut s = s;
        if s[0] != b',' {
            max = min;
        } else {
            s = &s[1..];
            if s.is_empty() {
                return None;
            }
            if s[0] == b'}' {
                max = -1;
            } else {
                let (m, rest) = parse_int(s)?;
                max = m;
                s = rest;
                if max < 0 {
                    // parseInt found too big a number
                    min = -1;
                }
            }
        }
        if s.is_empty() || s[0] != b'}' {
            return None;
        }
        Some((min, max, &s[1..]))
    }

    /// parsePerlFlags parses a Perl flag setting or non-capturing group or both, like (?i) or
    /// (?: or (?i:. It removes the prefix from s and updates the parse state. The caller must
    /// have ensured that s begins with "(?".
    // Go: regexp/syntax/parse.go:parsePerlFlags
    fn parse_perl_flags(&mut self, s: &'a [u8]) -> PResult<&'a [u8]> {
        let mut t = s;

        // Check for named captures, first introduced in Python's regexp library.
        let starts_with_p = t.len() > 4 && t[2] == b'P' && t[3] == b'<';
        let starts_with_name = t.len() > 3 && t[2] == b'<';

        if starts_with_p || starts_with_name {
            // position of expr start
            let expr_start_pos = if starts_with_name { 3 } else { 4 };

            // Pull out name.
            let Some(end) = t.iter().position(|&c| c == b'>') else {
                check_utf8(t)?;
                return Err(Error::new(ErrorCode::InvalidNamedCapture, s));
            };

            let capture = &t[..end + 1]; // "(?P<name>" or "(?<name>"
            let name = &t[expr_start_pos..end]; // "name"
            check_utf8(name)?;
            if !is_valid_capture_name(name) {
                return Err(Error::new(ErrorCode::InvalidNamedCapture, capture));
            }

            // Like ordinary capture, but named.
            self.num_cap += 1;
            let cap = self.num_cap;
            let name = String::from_utf8(name.to_vec()).expect("checked ASCII");
            self.op_with(Op::LeftParen, |re| {
                re.cap = cap;
                re.name = name;
            })?;
            return Ok(&t[end + 1..]);
        }

        // Non-capturing group. Might also twiddle Perl flags.
        t = &t[2..]; // skip (?
        let mut flags = self.flags;
        let mut sign = 1;
        let mut saw_flag = false;
        while !t.is_empty() {
            let (c, rest) = next_rune(t)?;
            t = rest;
            match c {
                // Flags.
                0x69 /* i */ => {
                    flags |= FOLD_CASE;
                    saw_flag = true;
                }
                0x6d /* m */ => {
                    flags &= !ONE_LINE;
                    saw_flag = true;
                }
                0x73 /* s */ => {
                    flags |= DOT_NL;
                    saw_flag = true;
                }
                0x55 /* U */ => {
                    flags |= NON_GREEDY;
                    saw_flag = true;
                }

                // Switch to negation.
                0x2d /* - */ => {
                    if sign < 0 {
                        break;
                    }
                    sign = -1;
                    // Invert flags so that | above turn into &^ and vice versa. We'll invert
                    // flags again before using it below.
                    flags = !flags;
                    saw_flag = false;
                }

                // End of flags, starting group or not.
                0x3a /* : */ | 0x29 /* ) */ => {
                    if sign < 0 {
                        if !saw_flag {
                            break;
                        }
                        flags = !flags;
                    }
                    if c == ':' as Rune {
                        // Open new group
                        self.op(Op::LeftParen)?;
                    }
                    self.flags = flags;
                    return Ok(t);
                }
                _ => break,
            }
        }

        Err(Error::new(
            ErrorCode::InvalidPerlOp,
            &s[..s.len() - t.len()],
        ))
    }

    /// parseVerticalBar handles a | in the input.
    // Go: regexp/syntax/parse.go:parseVerticalBar
    fn parse_vertical_bar(&mut self) -> PResult<()> {
        self.concat()?;

        // The concatenation we just parsed is on top of the stack. If it sits above an
        // opVerticalBar, swap it below (things below an opVerticalBar become an alternation).
        // Otherwise, push a new vertical bar.
        if !self.swap_vertical_bar() {
            self.op(Op::VerticalBar)?;
        }
        Ok(())
    }

    /// If the top of the stack is an element followed by an opVerticalBar swapVerticalBar
    /// swaps the two and returns true. Otherwise it returns false.
    // Go: regexp/syntax/parse.go:swapVerticalBar
    fn swap_vertical_bar(&mut self) -> bool {
        // If above and below vertical bar are literal or char class, can merge into a single
        // char class.
        let n = self.stack.len();
        if n >= 3
            && self.stack[n - 2].op == Op::VerticalBar
            && is_char_class(&self.stack[n - 1])
            && is_char_class(&self.stack[n - 3])
        {
            // Make re3 the more complex of the two.
            if self.stack[n - 1].op > self.stack[n - 3].op {
                self.stack.swap(n - 1, n - 3);
            }
            let re1 = self.stack.pop().expect("n >= 3");
            merge_char_class(&mut self.stack[n - 3], &re1);
            self.reuse(re1);
            return true;
        }

        if n >= 2 && self.stack[n - 2].op == Op::VerticalBar {
            if n >= 3 {
                // Now out of reach. Clean opportunistically.
                clean_alt(&mut self.stack[n - 3]);
            }
            self.stack.swap(n - 2, n - 1);
            return true;
        }
        false
    }

    /// parseRightParen handles a ) in the input.
    // Go: regexp/syntax/parse.go:parseRightParen
    fn parse_right_paren(&mut self) -> PResult<()> {
        self.concat()?;
        if self.swap_vertical_bar() {
            // pop vertical bar
            self.stack.pop();
        }
        self.alternate()?;

        let n = self.stack.len();
        if n < 2 {
            return Err(Error::new(ErrorCode::UnexpectedParen, self.whole_regexp));
        }
        let re1 = self.stack.pop().expect("n >= 2");
        let mut re2 = self.stack.pop().expect("n >= 2");
        if re2.op != Op::LeftParen {
            return Err(Error::new(ErrorCode::UnexpectedParen, self.whole_regexp));
        }
        // Restore flags at time of paren.
        self.flags = re2.flags;
        if re2.cap == 0 {
            // Just for grouping.
            self.push(re1)
        } else {
            re2.op = Op::Capture;
            re2.sub = vec![re1];
            self.push(re2)
        }
    }

    /// parseEscape parses an escape sequence at the beginning of s and returns the rune.
    // Go: regexp/syntax/parse.go:parseEscape
    fn parse_escape(&self, s: &'a [u8]) -> PResult<(Rune, &'a [u8])> {
        let t = &s[1..];
        if t.is_empty() {
            return Err(Error::new(ErrorCode::TrailingBackslash, b""));
        }
        let (c, mut t) = next_rune(t)?;

        'sw: {
            match c {
                // Octal escapes.
                0x30..=0x37 => {
                    // Single non-zero digit is a backreference; not supported
                    if c != '0' as Rune && (t.is_empty() || t[0] < b'0' || t[0] > b'7') {
                        break 'sw;
                    }
                    // Consume up to three octal digits; already have one.
                    let mut r = c - '0' as Rune;
                    for _ in 1..3 {
                        if t.is_empty() || t[0] < b'0' || t[0] > b'7' {
                            break;
                        }
                        r = r * 8 + Rune::from(t[0]) - '0' as Rune;
                        t = &t[1..];
                    }
                    return Ok((r, t));
                }

                // Hexadecimal escapes.
                0x78 /* x */ => {
                    if t.is_empty() {
                        break 'sw;
                    }
                    let (c, rest) = next_rune(t)?;
                    t = rest;
                    if c == '{' as Rune {
                        // Any number of digits in braces. Perl accepts any text at all; it
                        // ignores all text after the first non-hex digit. We require only hex
                        // digits, and at least one.
                        let mut nhex = 0;
                        let mut r: Rune = 0;
                        loop {
                            if t.is_empty() {
                                break 'sw;
                            }
                            let (c, rest) = next_rune(t)?;
                            t = rest;
                            if c == '}' as Rune {
                                break;
                            }
                            let v = unhex(c);
                            if v < 0 {
                                break 'sw;
                            }
                            r = r * 16 + v;
                            if r > MAX_RUNE {
                                break 'sw;
                            }
                            nhex += 1;
                        }
                        if nhex == 0 {
                            break 'sw;
                        }
                        return Ok((r, t));
                    }

                    // Easy case: two hex digits.
                    let x = unhex(c);
                    let (c, rest) = next_rune(t)?;
                    t = rest;
                    let y = unhex(c);
                    if x < 0 || y < 0 {
                        break 'sw;
                    }
                    return Ok((x * 16 + y, t));
                }

                // C escapes. There is no case 'b', to avoid misparsing the Perl word-boundary
                // \b as the C backspace \b when in POSIX mode.
                0x61 /* a */ => return Ok((0x07, t)),
                0x66 /* f */ => return Ok((0x0c, t)),
                0x6e /* n */ => return Ok((0x0a, t)),
                0x72 /* r */ => return Ok((0x0d, t)),
                0x74 /* t */ => return Ok((0x09, t)),
                0x76 /* v */ => return Ok((0x0b, t)),
                _ => {
                    if c < RUNE_SELF && !is_alnum(c) {
                        // Escaped non-word characters are always themselves.
                        return Ok((c, t));
                    }
                }
            }
        }
        Err(Error::new(
            ErrorCode::InvalidEscape,
            &s[..s.len() - t.len()],
        ))
    }

    /// parseClassChar parses a character class character at the beginning of s and returns
    /// it.
    // Go: regexp/syntax/parse.go:parseClassChar
    fn parse_class_char(&self, s: &'a [u8], whole_class: &'a [u8]) -> PResult<(Rune, &'a [u8])> {
        if s.is_empty() {
            return Err(Error::new(ErrorCode::MissingBracket, whole_class));
        }

        // Allow regular escape sequences even though many need not be escaped in this context.
        if s[0] == b'\\' {
            return self.parse_escape(s);
        }

        next_rune(s)
    }

    /// parsePerlClassEscape parses a leading Perl character class escape like \d from the
    /// beginning of s. If one is present, it appends the characters to r and returns the new
    /// slice r and the remainder of the string.
    // Go: regexp/syntax/parse.go:parsePerlClassEscape
    fn parse_perl_class_escape(&mut self, s: &'a [u8], r: &mut Vec<Rune>) -> Option<&'a [u8]> {
        if self.flags & PERL_X == 0 || s.len() < 2 || s[0] != b'\\' {
            return None;
        }
        let g = perl_group(&s[0..2])?;
        self.append_group(r, g);
        Some(&s[2..])
    }

    /// parseNamedClass parses a leading POSIX named character class like [:alnum:] from the
    /// beginning of s. If one is present, it appends the characters to r and returns the
    /// remainder of the string.
    // Go: regexp/syntax/parse.go:parseNamedClass
    fn parse_named_class(&mut self, s: &'a [u8], r: &mut Vec<Rune>) -> PResult<Option<&'a [u8]>> {
        if s.len() < 2 || s[0] != b'[' || s[1] != b':' {
            return Ok(None);
        }

        let Some(i) = find(&s[2..], b":]") else {
            return Ok(None);
        };
        let i = i + 2;
        let (name, s) = (&s[0..i + 2], &s[i + 2..]);
        let Some(g) = posix_group(name) else {
            return Err(Error::new(ErrorCode::InvalidCharRange, name));
        };
        self.append_group(r, g);
        Ok(Some(s))
    }

    // Go: regexp/syntax/parse.go:appendGroup
    fn append_group(&mut self, r: &mut Vec<Rune>, g: CharGroup) {
        if self.flags & FOLD_CASE == 0 {
            if g.sign < 0 {
                append_negated_class(r, g.class);
            } else {
                append_class(r, g.class);
            }
        } else {
            self.tmp_class.clear();
            append_folded_class(&mut self.tmp_class, g.class);
            clean_class(&mut self.tmp_class);
            if g.sign < 0 {
                append_negated_class(r, &self.tmp_class);
            } else {
                append_class(r, &self.tmp_class);
            }
        }
    }

    /// parseUnicodeClass parses a leading Unicode character class like \p{Han} from the
    /// beginning of s. If one is present, it appends the characters to r and returns the
    /// remainder of the string (`None`: no class).
    // Go: regexp/syntax/parse.go:parseUnicodeClass
    fn parse_unicode_class(&mut self, s: &'a [u8], r: &mut Vec<Rune>) -> PResult<Option<&'a [u8]>> {
        if self.flags & UNICODE_GROUPS == 0
            || s.len() < 2
            || s[0] != b'\\'
            || s[1] != b'p' && s[1] != b'P'
        {
            return Ok(None);
        }

        // Committed to parse or return error.
        let mut sign = 1;
        if s[1] == b'P' {
            sign = -1;
        }
        let t = &s[2..];
        // Go: `c, t, err := nextRune(t)` assigns the named result `err`.
        let (c, t) = next_rune(t)?;
        let (seq, mut name, t) = if c != '{' as Rune {
            // Single-letter name.
            let seq = &s[..s.len() - t.len()];
            (seq, &seq[2..], t)
        } else {
            // Name is in braces.
            let Some(end) = s.iter().position(|&c| c == b'}') else {
                check_utf8(s)?;
                return Err(Error::new(ErrorCode::InvalidCharRange, s));
            };
            let name = &s[3..end];
            check_utf8(name)?;
            (&s[..end + 1], name, &s[end + 1..])
        };

        // Group can have leading negation too.  \p{^Han} == \P{Han}, \P{^Han} == \p{Han}.
        if !name.is_empty() && name[0] == b'^' {
            sign = -sign;
            name = &name[1..];
        }

        let Some((tab, fold, tsign)) = unicode_table(name) else {
            return Err(Error::new(ErrorCode::InvalidCharRange, seq));
        };
        if tsign < 0 {
            sign = -sign;
        }

        let fold = if self.flags & FOLD_CASE == 0 {
            None
        } else {
            fold
        };
        if let Some(fold) = fold {
            // Merge and clean tab and fold in a temporary buffer. This is necessary for the
            // negative case and just tidy for the positive case.
            self.tmp_class.clear();
            append_table(&mut self.tmp_class, tab);
            append_table(&mut self.tmp_class, fold);
            clean_class(&mut self.tmp_class);
            if sign > 0 {
                append_class(r, &self.tmp_class);
            } else {
                append_negated_class(r, &self.tmp_class);
            }
        } else if sign > 0 {
            append_table(r, tab);
        } else {
            append_negated_table(r, tab);
        }
        Ok(Some(t))
    }

    /// parseClass parses a character class at the beginning of s and pushes it onto the parse
    /// stack.
    // Go: regexp/syntax/parse.go:parseClass
    fn parse_class(&mut self, s: &'a [u8]) -> PResult<&'a [u8]> {
        let mut t = &s[1..]; // chop [
        let mut re = self.new_regexp(Op::CharClass);
        re.flags = self.flags;
        let mut class: Vec<Rune> = Vec::new();

        let mut sign = 1;
        if !t.is_empty() && t[0] == b'^' {
            sign = -1;
            t = &t[1..];

            // If character class does not match \n, add it here, so that negation later will
            // do the right thing.
            if self.flags & CLASS_NL == 0 {
                class.push('\n' as Rune);
                class.push('\n' as Rune);
            }
        }

        let mut first = true; // ] and - are okay as first char in class
        while t.is_empty() || t[0] != b']' || first {
            // POSIX: - is only okay unescaped as first or last in class.
            // Perl: - is okay anywhere.
            if !t.is_empty()
                && t[0] == b'-'
                && self.flags & PERL_X == 0
                && !first
                && (t.len() == 1 || t[1] != b']')
            {
                let (_, size) = utf8::decode_rune_in_string(&t[1..]);
                return Err(Error::new(ErrorCode::InvalidCharRange, &t[..1 + size]));
            }
            first = false;

            // Look for POSIX [:alnum:] etc.
            if t.len() > 2
                && t[0] == b'['
                && t[1] == b':'
                && let Some(nt) = self.parse_named_class(t, &mut class)?
            {
                t = nt;
                continue;
            }

            // Look for Unicode character group like \p{Han}.
            if let Some(nt) = self.parse_unicode_class(t, &mut class)? {
                t = nt;
                continue;
            }

            // Look for Perl character class symbols (extension).
            if let Some(nt) = self.parse_perl_class_escape(t, &mut class) {
                t = nt;
                continue;
            }

            // Single character or simple range.
            let rng = t;
            let (lo, rest) = self.parse_class_char(t, s)?;
            t = rest;
            let mut hi = lo;
            // [a-] means (a|-) so check for final ].
            if t.len() >= 2 && t[0] == b'-' && t[1] != b']' {
                t = &t[1..];
                let (h, rest) = self.parse_class_char(t, s)?;
                hi = h;
                t = rest;
                if hi < lo {
                    let rng = &rng[..rng.len() - t.len()];
                    return Err(Error::new(ErrorCode::InvalidCharRange, rng));
                }
            }
            if self.flags & FOLD_CASE == 0 {
                append_range(&mut class, lo, hi);
            } else {
                append_folded_range(&mut class, lo, hi);
            }
        }
        t = &t[1..]; // chop ]

        clean_class(&mut class);
        if sign < 0 {
            negate_class(&mut class);
        }
        re.rune = class;
        self.push(re)?;
        Ok(t)
    }
}

// Go: regexp/syntax/parse.go:calcHeight
fn calc_height(re: &mut Regexp, force: bool) -> i64 {
    if !force && let Some(h) = re.height_cache {
        return h;
    }
    let mut h = 1;
    for sub in &mut re.sub {
        let hsub = calc_height(sub, false);
        if h < 1 + hsub {
            h = 1 + hsub;
        }
    }
    re.height_cache = Some(h);
    h
}

/// minFoldRune returns the minimum rune fold-equivalent to r.
// Go: regexp/syntax/parse.go:minFoldRune
fn min_fold_rune(r: Rune) -> Rune {
    if !(MIN_FOLD..=MAX_FOLD).contains(&r) {
        return r;
    }
    let mut m = r;
    let r0 = r;
    let mut r = simple_fold(r);
    while r != r0 {
        m = m.min(r);
        r = simple_fold(r);
    }
    m
}

/// repeatIsValid reports whether the repetition re is valid. Valid means that the combination
/// of the top-level repetition and any inner repetitions does not exceed n copies of the
/// innermost thing.
// Go: regexp/syntax/parse.go:repeatIsValid
fn repeat_is_valid(re: &Regexp, mut n: i64) -> bool {
    if re.op == Op::Repeat {
        let mut m = re.max;
        if m == 0 {
            return true;
        }
        if m < 0 {
            m = re.min;
        }
        if m > n {
            return false;
        }
        if m > 0 {
            n /= m;
        }
    }
    for sub in &re.sub {
        if !repeat_is_valid(sub, n) {
            return false;
        }
    }
    true
}

/// cleanAlt cleans re for eventual inclusion in an alternation.
// Go: regexp/syntax/parse.go:cleanAlt
fn clean_alt(re: &mut Regexp) {
    if re.op == Op::CharClass {
        clean_class(&mut re.rune);
        if re.rune.len() == 2 && re.rune[0] == 0 && re.rune[1] == MAX_RUNE {
            re.rune = Vec::new();
            re.op = Op::AnyChar;
            return;
        }
        if re.rune.len() == 4
            && re.rune[0] == 0
            && re.rune[1] == '\n' as Rune - 1
            && re.rune[2] == '\n' as Rune + 1
            && re.rune[3] == MAX_RUNE
        {
            re.rune = Vec::new();
            re.op = Op::AnyCharNotNL;
        }
    }
}

/// leadingString returns the leading literal string that re begins with.
// Go: regexp/syntax/parse.go:leadingString
fn leading_string(mut re: &Regexp) -> (&[Rune], Flags) {
    if re.op == Op::Concat && !re.sub.is_empty() {
        re = &re.sub[0];
    }
    if re.op != Op::Literal {
        return (&[], 0);
    }
    (&re.rune, re.flags & FOLD_CASE)
}

/// leadingRegexp returns the leading regexp that re begins with.
// Go: regexp/syntax/parse.go:leadingRegexp
fn leading_regexp(re: &Regexp) -> Option<&Regexp> {
    if re.op == Op::EmptyMatch {
        return None;
    }
    if re.op == Op::Concat && !re.sub.is_empty() {
        let sub = &re.sub[0];
        if sub.op == Op::EmptyMatch {
            return None;
        }
        return Some(sub);
    }
    Some(re)
}

// Go: regexp/syntax/parse.go:literalRegexp
fn literal_regexp(s: &[u8], flags: Flags) -> Regexp {
    let mut re = Regexp::with_op(Op::Literal);
    re.flags = flags;
    re.rune = utf8::to_runes(s);
    re
}

/// Parse parses a regular expression string s, controlled by the specified Flags, and returns
/// a regular expression parse tree.
// Go: regexp/syntax/parse.go:Parse
pub fn parse(s: &[u8], flags: Flags) -> PResult<Regexp> {
    if flags & LITERAL != 0 {
        // Trivial parser for literal string.
        check_utf8(s)?;
        return Ok(literal_regexp(s, flags));
    }

    // Otherwise, must do real work.
    let mut p = Parser {
        flags,
        stack: Vec::new(),
        free: Vec::new(),
        num_cap: 0,
        whole_regexp: s,
        tmp_class: Vec::new(),
        num_regexp: 0,
        num_runes: 0,
        repeats: 0,
        height_on: false,
        size_on: false,
    };
    let mut last_repeat: &[u8] = b"";
    let mut t = s;
    while !t.is_empty() {
        let mut repeat: &[u8] = b"";
        'big_switch: {
            match t[0] {
                b'(' => {
                    if p.flags & PERL_X != 0 && t.len() >= 2 && t[1] == b'?' {
                        // Flag changes and non-capturing groups.
                        t = p.parse_perl_flags(t)?;
                        break 'big_switch;
                    }
                    p.num_cap += 1;
                    let cap = p.num_cap;
                    p.op_with(Op::LeftParen, |re| re.cap = cap)?;
                    t = &t[1..];
                }
                b'|' => {
                    p.parse_vertical_bar()?;
                    t = &t[1..];
                }
                b')' => {
                    p.parse_right_paren()?;
                    t = &t[1..];
                }
                b'^' => {
                    if p.flags & ONE_LINE != 0 {
                        p.op(Op::BeginText)?;
                    } else {
                        p.op(Op::BeginLine)?;
                    }
                    t = &t[1..];
                }
                b'$' => {
                    if p.flags & ONE_LINE != 0 {
                        p.op_with(Op::EndText, |re| re.flags |= WAS_DOLLAR)?;
                    } else {
                        p.op(Op::EndLine)?;
                    }
                    t = &t[1..];
                }
                b'.' => {
                    if p.flags & DOT_NL != 0 {
                        p.op(Op::AnyChar)?;
                    } else {
                        p.op(Op::AnyCharNotNL)?;
                    }
                    t = &t[1..];
                }
                b'[' => {
                    t = p.parse_class(t)?;
                }
                b'*' | b'+' | b'?' => {
                    let before = t;
                    let op = match t[0] {
                        b'*' => Op::Star,
                        b'+' => Op::Plus,
                        _ => Op::Quest,
                    };
                    let after = &t[1..];
                    let after = p.repeat(op, 0, 0, before, after, last_repeat)?;
                    repeat = before;
                    t = after;
                }
                b'{' => {
                    let op = Op::Repeat;
                    let before = t;
                    let Some((min, max, after)) = p.parse_repeat(t) else {
                        // If the repeat cannot be parsed, { is a literal.
                        p.literal('{' as Rune)?;
                        t = &t[1..];
                        break 'big_switch;
                    };
                    if !(0..=1000).contains(&min) || max > 1000 || max >= 0 && min > max {
                        // Numbers were too big, or max is present and min > max.
                        return Err(Error::new(
                            ErrorCode::InvalidRepeatSize,
                            &before[..before.len() - after.len()],
                        ));
                    }
                    let after = p.repeat(op, min, max, before, after, last_repeat)?;
                    repeat = before;
                    t = after;
                }
                b'\\' => {
                    if p.flags & PERL_X != 0 && t.len() >= 2 {
                        match t[1] {
                            b'A' => {
                                p.op(Op::BeginText)?;
                                t = &t[2..];
                                break 'big_switch;
                            }
                            b'b' => {
                                p.op(Op::WordBoundary)?;
                                t = &t[2..];
                                break 'big_switch;
                            }
                            b'B' => {
                                p.op(Op::NoWordBoundary)?;
                                t = &t[2..];
                                break 'big_switch;
                            }
                            b'C' => {
                                // any byte; not supported
                                return Err(Error::new(ErrorCode::InvalidEscape, &t[..2]));
                            }
                            b'Q' => {
                                // \Q ... \E: the ... is always literals
                                let rest = &t[2..];
                                let (mut lit, after) = match find(rest, br"\E") {
                                    Some(i) => (&rest[..i], &rest[i + 2..]),
                                    None => (rest, &b""[..]),
                                };
                                t = after;
                                while !lit.is_empty() {
                                    let (c, rest) = next_rune(lit)?;
                                    p.literal(c)?;
                                    lit = rest;
                                }
                                break 'big_switch;
                            }
                            b'z' => {
                                p.op(Op::EndText)?;
                                t = &t[2..];
                                break 'big_switch;
                            }
                            _ => {}
                        }
                    }

                    let mut re = p.new_regexp(Op::CharClass);
                    re.flags = p.flags;

                    // Look for Unicode character group like \p{Han}
                    if t.len() >= 2 && (t[1] == b'p' || t[1] == b'P') {
                        let mut r = Vec::new();
                        if let Some(rest) = p.parse_unicode_class(t, &mut r)? {
                            re.rune = r;
                            t = rest;
                            p.push(re)?;
                            break 'big_switch;
                        }
                    }

                    // Perl character class escape.
                    let mut r = Vec::new();
                    if let Some(rest) = p.parse_perl_class_escape(t, &mut r) {
                        re.rune = r;
                        t = rest;
                        p.push(re)?;
                        break 'big_switch;
                    }
                    p.reuse(re);

                    // Ordinary single-character escape.
                    let (c, rest) = p.parse_escape(t)?;
                    t = rest;
                    p.literal(c)?;
                }
                _ => {
                    let (c, rest) = next_rune(t)?;
                    t = rest;
                    p.literal(c)?;
                }
            }
        }
        last_repeat = repeat;
    }

    p.concat()?;
    if p.swap_vertical_bar() {
        // pop vertical bar
        p.stack.pop();
    }
    p.alternate()?;

    let n = p.stack.len();
    if n != 1 {
        return Err(Error::new(ErrorCode::MissingParen, s));
    }
    Ok(p.stack.pop().expect("n == 1"))
}

/// isValidCaptureName reports whether name is a valid capture name: [A-Za-z0-9_]+.
// Go: regexp/syntax/parse.go:isValidCaptureName
fn is_valid_capture_name(name: &[u8]) -> bool {
    if name.is_empty() {
        return false;
    }
    // Go ranges over runes; a multi-byte rune is never '_' or alphanumeric.
    name.iter().all(|&c| c == b'_' || is_alnum(Rune::from(c)))
}

/// parseInt parses a decimal integer.
// Go: regexp/syntax/parse.go:parseInt
fn parse_int(s: &[u8]) -> Option<(i64, &[u8])> {
    if s.is_empty() || s[0] < b'0' || b'9' < s[0] {
        return None;
    }
    // Disallow leading zeros.
    if s.len() >= 2 && s[0] == b'0' && s[1].is_ascii_digit() {
        return None;
    }
    let digits = s.iter().take_while(|c| c.is_ascii_digit()).count();
    let (t, rest) = s.split_at(digits);
    // Have digits, compute value.
    let mut n: i64 = 0;
    for &d in t {
        // Avoid overflow.
        if n >= 100_000_000 {
            n = -1;
            break;
        }
        n = n * 10 + i64::from(d) - i64::from(b'0');
    }
    Some((n, rest))
}

/// can this be represented as a character class? single-rune literal string, char class, .,
/// and .|\n.
// Go: regexp/syntax/parse.go:isCharClass
fn is_char_class(re: &Regexp) -> bool {
    re.op == Op::Literal && re.rune.len() == 1
        || re.op == Op::CharClass
        || re.op == Op::AnyCharNotNL
        || re.op == Op::AnyChar
}

/// does re match r?
// Go: regexp/syntax/parse.go:matchRune
fn match_rune(re: &Regexp, r: Rune) -> bool {
    match re.op {
        Op::Literal => re.rune.len() == 1 && re.rune[0] == r,
        Op::CharClass => {
            let mut i = 0;
            while i + 1 < re.rune.len() {
                if re.rune[i] <= r && r <= re.rune[i + 1] {
                    return true;
                }
                i += 2;
            }
            false
        }
        Op::AnyCharNotNL => r != '\n' as Rune,
        Op::AnyChar => true,
        _ => false,
    }
}

/// mergeCharClass makes dst = dst|src. The caller must ensure that dst.Op >= src.Op, to reduce
/// the amount of copying.
// Go: regexp/syntax/parse.go:mergeCharClass
fn merge_char_class(dst: &mut Regexp, src: &Regexp) {
    match dst.op {
        Op::AnyChar => {
            // src doesn't add anything.
        }
        Op::AnyCharNotNL => {
            // src might add \n
            if match_rune(src, '\n' as Rune) {
                dst.op = Op::AnyChar;
            }
        }
        Op::CharClass => {
            // src is simpler, so either literal or char class
            if src.op == Op::Literal {
                append_literal(&mut dst.rune, src.rune[0], src.flags);
            } else {
                append_class(&mut dst.rune, &src.rune);
            }
        }
        Op::Literal => {
            // both literal
            if src.rune[0] == dst.rune[0] && src.flags == dst.flags {
                return;
            }
            dst.op = Op::CharClass;
            let d0 = dst.rune[0];
            dst.rune.clear();
            append_literal(&mut dst.rune, d0, dst.flags);
            append_literal(&mut dst.rune, src.rune[0], src.flags);
        }
        _ => {}
    }
}

/// A Go `unicode.RangeTable` value built at run time (`anyTable`, `asciiTable`,
/// `asciiFoldTable`).
static ANY_TABLE: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0,
        hi: 0xFFFF,
        stride: 1,
    }],
    r32: &[Range32 {
        lo: 0x10000,
        hi: MAX_RUNE as u32,
        stride: 1,
    }],
    latin_offset: 0,
};

static ASCII_TABLE: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0,
        hi: 0x7F,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static ASCII_FOLD_TABLE: RangeTable = RangeTable {
    r16: &[
        Range16 {
            lo: 0,
            hi: 0x7F,
            stride: 1,
        },
        // Old English long s (ſ), folds to S/s.
        Range16 {
            lo: 0x017F,
            hi: 0x017F,
            stride: 1,
        },
        // Kelvin K, folds to K/k.
        Range16 {
            lo: 0x212A,
            hi: 0x212A,
            stride: 1,
        },
    ],
    r32: &[],
    latin_offset: 0,
};

/// canonicalName returns the canonical lookup string for name. The canonical name has a
/// leading uppercase letter and then lowercase letters, and it omits all underscores, spaces,
/// and hyphens.
// Go: regexp/syntax/parse.go:canonicalName
fn canonical_name(name: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(name.len());
    let mut first = true;
    for &c0 in name {
        let mut c = c0;
        if c == b'_' || c == b'-' || c == b' ' {
            continue;
        } else if first {
            c.make_ascii_uppercase();
            first = false;
        } else {
            c.make_ascii_lowercase();
        }
        b.push(c);
    }
    b
}

/// Go's lazily built `aliases.categories` / `aliases.scripts`: the canonical name of a
/// `unicode.CategoryAliases` / `unicode.Scripts` key. Several keys may share a canonical name;
/// Go builds the maps by ranging over the source maps (random order), so the last one wins at
/// random. In go1.27.1 no two keys of either map share a canonical name (checked by a test).
fn alias_category(name: &[u8]) -> Option<&'static str> {
    CATEGORY_ALIASES
        .iter()
        .find(|(k, _)| canonical_name(k.as_bytes()) == name)
        .map(|(_, v)| *v)
}

fn alias_script(name: &[u8]) -> Option<&'static str> {
    SCRIPTS
        .iter()
        .find(|(k, _)| canonical_name(k.as_bytes()) == name)
        .map(|(k, _)| *k)
}

/// unicodeTable returns the unicode.RangeTable identified by name and the table of additional
/// fold-equivalent code points. If sign < 0, the result should be inverted.
// Go: regexp/syntax/parse.go:unicodeTable
#[allow(clippy::type_complexity)]
fn unicode_table(name: &[u8]) -> Option<(&'static RangeTable, Option<&'static RangeTable>, i32)> {
    let name = canonical_name(name);
    let Ok(name) = std::str::from_utf8(&name) else {
        return None;
    };

    // Special cases: Any, Assigned, and ASCII. Also LC is the only non-canonical Categories
    // key, so handle it here.
    match name {
        "Any" => return Some((&ANY_TABLE, Some(&ANY_TABLE), 1)),
        "Assigned" => {
            // invert Cn (unassigned)
            let cn = lookup(CATEGORIES, "Cn").expect("Cn");
            return Some((cn, Some(cn), -1));
        }
        "Ascii" => return Some((&ASCII_TABLE, Some(&ASCII_FOLD_TABLE), 1)),
        "Lc" => {
            return Some((
                lookup(CATEGORIES, "LC").expect("LC"),
                lookup(FOLD_CATEGORY, "LC"),
                1,
            ));
        }
        _ => {}
    }
    if let Some(t) = lookup(CATEGORIES, name) {
        return Some((t, lookup(FOLD_CATEGORY, name), 1));
    }
    if let Some(t) = lookup(SCRIPTS, name) {
        return Some((t, lookup(FOLD_SCRIPT, name), 1));
    }

    // unicode.CategoryAliases makes liberal use of underscores in its names (they are defined
    // that way by Unicode), but we want to match ignoring the underscores, so make our own map
    // with canonical names.
    if let Some(actual) = alias_category(name.as_bytes()) {
        let t = lookup(CATEGORIES, actual)?;
        return Some((t, lookup(FOLD_CATEGORY, actual), 1));
    }
    if let Some(actual) = alias_script(name.as_bytes()) {
        let t = lookup(SCRIPTS, actual)?;
        return Some((t, lookup(FOLD_SCRIPT, actual), 1));
    }
    None
}

/// cleanClass sorts the ranges (pairs of elements of r), merges them, and eliminates
/// duplicates.
// Go: regexp/syntax/parse.go:cleanClass
pub(crate) fn clean_class(r: &mut Vec<Rune>) {
    // Sort by lo increasing, hi decreasing to break ties. (Equal pairs are identical, so
    // the unstable order of Go's sort.Sort cannot be seen.)
    let mut pairs: Vec<(Rune, Rune)> = r.chunks_exact(2).map(|c| (c[0], c[1])).collect();
    pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    r.clear();
    for (lo, hi) in &pairs {
        r.push(*lo);
        r.push(*hi);
    }

    if r.len() < 2 {
        return;
    }

    // Merge abutting, overlapping.
    let mut w = 2; // write index
    let mut i = 2;
    while i < r.len() {
        let (lo, hi) = (r[i], r[i + 1]);
        if lo <= r[w - 1] + 1 {
            // merge with previous range
            if hi > r[w - 1] {
                r[w - 1] = hi;
            }
            i += 2;
            continue;
        }
        // new disjoint range
        r[w] = lo;
        r[w + 1] = hi;
        w += 2;
        i += 2;
    }

    r.truncate(w);
}

/// inCharClass reports whether r is in the class. It assumes the class has been cleaned by
/// cleanClass.
// Go: regexp/syntax/parse.go:inCharClass
pub(crate) fn in_char_class(r: Rune, class: &[Rune]) -> bool {
    let n = class.len() / 2;
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let m = (lo + hi) / 2;
        let (clo, chi) = (class[2 * m], class[2 * m + 1]);
        if r > chi {
            lo = m + 1;
        } else if r < clo {
            hi = m;
        } else {
            return true;
        }
    }
    false
}

/// appendLiteral returns the result of appending the literal x to the class r.
// Go: regexp/syntax/parse.go:appendLiteral
fn append_literal(r: &mut Vec<Rune>, x: Rune, flags: Flags) {
    if flags & FOLD_CASE != 0 {
        append_folded_range(r, x, x);
    } else {
        append_range(r, x, x);
    }
}

/// appendRange returns the result of appending the range lo-hi to the class r.
// Go: regexp/syntax/parse.go:appendRange
fn append_range(r: &mut Vec<Rune>, lo: Rune, hi: Rune) {
    // Expand last range or next to last range if it overlaps or abuts. Checking two ranges
    // helps when appending case-folded alphabets, so that one range can be expanding A-Z and
    // the other expanding a-z.
    let n = r.len();
    let mut i = 2;
    while i <= 4 {
        // twice, using i=2, i=4
        if n >= i {
            let (rlo, rhi) = (r[n - i], r[n - i + 1]);
            if lo <= rhi + 1 && rlo <= hi + 1 {
                if lo < rlo {
                    r[n - i] = lo;
                }
                if hi > rhi {
                    r[n - i + 1] = hi;
                }
                return;
            }
        }
        i += 2;
    }

    r.push(lo);
    r.push(hi);
}

// minimum and maximum runes involved in folding. checked during test.
pub(crate) const MIN_FOLD: Rune = 0x0041;
pub(crate) const MAX_FOLD: Rune = 0x1e943;

/// appendFoldedRange returns the result of appending the range lo-hi and its case
/// folding-equivalent runes to the class r.
// Go: regexp/syntax/parse.go:appendFoldedRange
fn append_folded_range(r: &mut Vec<Rune>, mut lo: Rune, mut hi: Rune) {
    // Optimizations.
    if lo <= MIN_FOLD && hi >= MAX_FOLD {
        // Range is full: folding can't add more.
        append_range(r, lo, hi);
        return;
    }
    if hi < MIN_FOLD || lo > MAX_FOLD {
        // Range is outside folding possibilities.
        append_range(r, lo, hi);
        return;
    }
    if lo < MIN_FOLD {
        // [lo, minFold-1] needs no folding.
        append_range(r, lo, MIN_FOLD - 1);
        lo = MIN_FOLD;
    }
    if hi > MAX_FOLD {
        // [maxFold+1, hi] needs no folding.
        append_range(r, MAX_FOLD + 1, hi);
        hi = MAX_FOLD;
    }

    // Brute force. Depend on appendRange to coalesce ranges on the fly.
    let mut c = lo;
    while c <= hi {
        append_range(r, c, c);
        let mut f = simple_fold(c);
        while f != c {
            append_range(r, f, f);
            f = simple_fold(f);
        }
        c += 1;
    }
}

/// appendClass returns the result of appending the class x to the class r. It assume x is
/// clean.
// Go: regexp/syntax/parse.go:appendClass
fn append_class(r: &mut Vec<Rune>, x: &[Rune]) {
    let mut i = 0;
    while i + 1 < x.len() {
        append_range(r, x[i], x[i + 1]);
        i += 2;
    }
}

/// appendFoldedClass returns the result of appending the case folding of the class x to the
/// class r.
// Go: regexp/syntax/parse.go:appendFoldedClass
fn append_folded_class(r: &mut Vec<Rune>, x: &[Rune]) {
    let mut i = 0;
    while i + 1 < x.len() {
        append_folded_range(r, x[i], x[i + 1]);
        i += 2;
    }
}

/// appendNegatedClass returns the result of appending the negation of the class x to the class
/// r. It assumes x is clean.
// Go: regexp/syntax/parse.go:appendNegatedClass
fn append_negated_class(r: &mut Vec<Rune>, x: &[Rune]) {
    let mut next_lo: Rune = 0;
    let mut i = 0;
    while i + 1 < x.len() {
        let (lo, hi) = (x[i], x[i + 1]);
        if next_lo < lo {
            append_range(r, next_lo, lo - 1);
        }
        next_lo = hi + 1;
        i += 2;
    }
    if next_lo <= MAX_RUNE {
        append_range(r, next_lo, MAX_RUNE);
    }
}

/// appendTable returns the result of appending x to the class r.
// Go: regexp/syntax/parse.go:appendTable
fn append_table(r: &mut Vec<Rune>, x: &RangeTable) {
    let ranges = x
        .r16
        .iter()
        .map(|xr| (Rune::from(xr.lo), Rune::from(xr.hi), Rune::from(xr.stride)))
        .chain(
            x.r32
                .iter()
                .map(|xr| (xr.lo as Rune, xr.hi as Rune, xr.stride as Rune)),
        );
    for (lo, hi, stride) in ranges {
        if stride == 1 {
            append_range(r, lo, hi);
            continue;
        }
        let mut c = lo;
        while c <= hi {
            append_range(r, c, c);
            c += stride;
        }
    }
}

/// appendNegatedTable returns the result of appending the negation of x to the class r.
// Go: regexp/syntax/parse.go:appendNegatedTable
fn append_negated_table(r: &mut Vec<Rune>, x: &RangeTable) {
    let mut next_lo: Rune = 0; // lo end of next class to add
    let ranges = x
        .r16
        .iter()
        .map(|xr| (Rune::from(xr.lo), Rune::from(xr.hi), Rune::from(xr.stride)))
        .chain(
            x.r32
                .iter()
                .map(|xr| (xr.lo as Rune, xr.hi as Rune, xr.stride as Rune)),
        );
    for (lo, hi, stride) in ranges {
        if stride == 1 {
            if next_lo < lo {
                append_range(r, next_lo, lo - 1);
            }
            next_lo = hi + 1;
            continue;
        }
        let mut c = lo;
        while c <= hi {
            if next_lo < c {
                append_range(r, next_lo, c - 1);
            }
            next_lo = c + 1;
            c += stride;
        }
    }
    if next_lo <= MAX_RUNE {
        append_range(r, next_lo, MAX_RUNE);
    }
}

/// negateClass overwrites r and returns r's negation. It assumes the class r is already
/// clean.
// Go: regexp/syntax/parse.go:negateClass
fn negate_class(r: &mut Vec<Rune>) {
    let mut next_lo: Rune = 0; // lo end of next class to add
    let mut w = 0; // write index
    let mut i = 0;
    while i + 1 < r.len() {
        let (lo, hi) = (r[i], r[i + 1]);
        if next_lo < lo {
            r[w] = next_lo;
            r[w + 1] = lo - 1;
            w += 2;
        }
        next_lo = hi + 1;
        i += 2;
    }
    r.truncate(w);
    if next_lo <= MAX_RUNE {
        // It's possible for the negation to have one more range - this one - than the
        // original class, so use append.
        r.push(next_lo);
        r.push(MAX_RUNE);
    }
}

// Go: regexp/syntax/parse.go:checkUTF8
fn check_utf8(mut s: &[u8]) -> PResult<()> {
    while !s.is_empty() {
        let (rune, size) = utf8::decode_rune_in_string(s);
        if rune == RUNE_ERROR && size == 1 {
            return Err(Error::new(ErrorCode::InvalidUTF8, s));
        }
        s = &s[size..];
    }
    Ok(())
}

// Go: regexp/syntax/parse.go:nextRune
fn next_rune(s: &[u8]) -> PResult<(Rune, &[u8])> {
    let (c, size) = utf8::decode_rune_in_string(s);
    if c == RUNE_ERROR && size == 1 {
        return Err(Error::new(ErrorCode::InvalidUTF8, s));
    }
    Ok((c, &s[size..]))
}

// Go: regexp/syntax/parse.go:isalnum
fn is_alnum(c: Rune) -> bool {
    (0x30..=0x39).contains(&c) || (0x41..=0x5a).contains(&c) || (0x61..=0x7a).contains(&c)
}

// Go: regexp/syntax/parse.go:unhex
fn unhex(c: Rune) -> Rune {
    if (0x30..=0x39).contains(&c) {
        return c - 0x30;
    }
    if (0x61..=0x66).contains(&c) {
        return c - 0x61 + 10;
    }
    if (0x41..=0x46).contains(&c) {
        return c - 0x41 + 10;
    }
    -1
}

/// `strings.Index` for the parser (first occurrence of `sep` in `s`).
fn find(s: &[u8], sep: &[u8]) -> Option<usize> {
    let i = go_unicode::strings::index(s, sep);
    (i >= 0).then_some(i as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Go builds the alias maps by ranging over `unicode.CategoryAliases` and
    /// `unicode.Scripts`; if two keys shared a canonical name, the winner would be random.
    #[test]
    fn canonical_alias_names_are_unique() {
        let mut seen = std::collections::HashMap::new();
        for (k, v) in CATEGORY_ALIASES {
            if let Some(old) = seen.insert(canonical_name(k.as_bytes()), *v) {
                assert_eq!(old, *v, "category alias {k}");
            }
        }
        let mut seen = std::collections::HashSet::new();
        for (k, _) in SCRIPTS {
            assert!(seen.insert(canonical_name(k.as_bytes())), "script {k}");
        }
    }

    #[test]
    fn fold_limits() {
        // Go's TestFoldConstants.
        let mut last: Rune = -1;
        for i in 0..=MAX_RUNE {
            if simple_fold(i) == i {
                continue;
            }
            if last == -1 && MIN_FOLD != i {
                panic!("MIN_FOLD={MIN_FOLD:#x} should be {i:#x}");
            }
            last = i;
        }
        assert_eq!(MAX_FOLD, last);
    }
}
