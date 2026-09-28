//! Go's `regexp` package (go1.27.1), ported function by function with its `regexp/syntax`
//! parser, simplifier and compiler and all three matching engines (one-pass, backtracker,
//! NFA). Matches, submatches, `FindAll`, `ReplaceAll`, `Split` and the compile errors are
//! Go's, on arbitrary bytes (Go strings may hold invalid UTF-8: every invalid byte is one
//! U+FFFD rune of width 1).
//!
//! Go has a `string` and a `[]byte` form of most functions; both run the same code on the same
//! bytes, so the port has one byte form (named after the `[]byte` function) and keeps the
//! `String` names for `&str` convenience where callers want Rust strings. A `&str` input
//! yields matches on character boundaries (Go steps by runes), so the `&str` results slice
//! safely. The `io.RuneReader` forms are not ported (Hugo does not use them).
//!
//! Go keeps pools of matching machines; the port allocates them per call (no observable
//! difference).

pub mod backtrack;
pub mod exec;
pub mod onepass;
pub mod syntax;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use go_unicode::Rune;
use go_unicode::utf8::{self, RUNE_ERROR, RUNE_SELF};

use self::onepass::{OnePassProg, compile_one_pass, one_pass_prefix};
use self::syntax::{EmptyOp, Prog};

/// The error of [`Regexp::compile`]: Go's `*syntax.Error`. Its text
/// (`error parsing regexp: <code>: `<expr>``) is kept as bytes (`as_bytes`) and printed
/// lossily by `Display`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub code: syntax::ErrorCode,
    /// The offending expression (bytes).
    pub expr: Vec<u8>,
    text: Vec<u8>,
}

impl Error {
    fn from_syntax(e: syntax::Error) -> Error {
        let text = e.error();
        Error {
            code: e.code,
            expr: e.expr,
            text,
        }
    }

    /// Go: `err.Error()` as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.text
    }

    /// Go: `err.Error()` (lossy for an expression that is not valid UTF-8).
    pub fn error(&self) -> String {
        String::from_utf8_lossy(&self.text).into_owned()
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.text))
    }
}

impl std::error::Error for Error {}

impl From<Error> for String {
    fn from(e: Error) -> String {
        e.error()
    }
}

/// Regexp is the representation of a compiled regular expression. A Regexp is safe for
/// concurrent use by multiple goroutines, except for configuration methods, such as
/// [`Regexp::longest`].
#[derive(Clone, Debug)]
pub struct Regexp {
    inner: Arc<Inner>,
}

#[derive(Clone, Debug)]
pub(crate) struct Inner {
    /// as passed to Compile
    expr: Vec<u8>,
    /// compiled program
    pub(crate) prog: Prog,
    /// onepass program or nil
    pub(crate) onepass: Option<OnePassProg>,
    num_subexp: usize,
    max_bit_state_len: usize,
    subexp_names: Vec<String>,
    /// required prefix in unanchored matches
    pub(crate) prefix: Vec<u8>,
    /// first rune in prefix
    pub(crate) prefix_rune: Rune,
    /// pc for last rune in prefix
    pub(crate) prefix_end: u32,
    /// size of recorded match lengths
    matchcap: usize,
    /// prefix is the entire regexp
    prefix_complete: bool,
    /// empty-width conditions required at start of match
    pub(crate) cond: EmptyOp,
    /// minimum length of the input in bytes
    min_input_len: usize,

    // This field can be modified by the Longest method, but it is otherwise read-only.
    /// whether regexp prefers leftmost-longest match
    pub(crate) longest: bool,
}

impl PartialEq for Regexp {
    fn eq(&self, other: &Self) -> bool {
        self.inner.expr == other.inner.expr && self.inner.longest == other.inner.longest
    }
}

/// A match: Go's `[]int` of submatch index pairs (`-1` for a group that did not participate).
pub type Match = Vec<isize>;

impl Regexp {
    /// String returns the source text used to compile the regular expression (lossy for a
    /// pattern that is not valid UTF-8, which never compiles).
    // Go: regexp/regexp.go:String
    pub fn string(&self) -> &str {
        std::str::from_utf8(&self.inner.expr).unwrap_or("")
    }

    /// The source text as bytes.
    pub fn expr(&self) -> &[u8] {
        &self.inner.expr
    }

    /// Compile parses a regular expression and returns, if successful, a Regexp object that
    /// can be used to match against text.
    ///
    /// When matching against text, the regexp returns a match that begins as early as
    /// possible in the input (leftmost), and among those it chooses the one that a
    /// backtracking implementation would have chosen (leftmost-first).
    // Go: regexp/regexp.go:Compile
    pub fn compile(expr: &str) -> Result<Regexp, Error> {
        compile(expr.as_bytes(), syntax::PERL, false)
    }

    /// Compile of a pattern held as bytes (a Go string need not be valid UTF-8; the parser
    /// rejects invalid UTF-8 with Go's error).
    // Go: regexp/regexp.go:Compile
    pub fn compile_bytes(expr: &[u8]) -> Result<Regexp, Error> {
        compile(expr, syntax::PERL, false)
    }

    /// CompilePOSIX is like Compile but restricts the regular expression to POSIX ERE
    /// (egrep) syntax and changes the match semantics to leftmost-longest.
    // Go: regexp/regexp.go:CompilePOSIX
    pub fn compile_posix(expr: &str) -> Result<Regexp, Error> {
        compile(expr.as_bytes(), syntax::POSIX, true)
    }

    /// CompilePOSIX of a pattern held as bytes.
    // Go: regexp/regexp.go:CompilePOSIX
    pub fn compile_posix_bytes(expr: &[u8]) -> Result<Regexp, Error> {
        compile(expr, syntax::POSIX, true)
    }

    /// MustCompile is like Compile but panics if the expression cannot be parsed.
    // Go: regexp/regexp.go:MustCompile
    pub fn must_compile(s: &str) -> Regexp {
        let s = s.as_bytes();
        match Regexp::compile_bytes(s) {
            Ok(re) => re,
            Err(e) => panic!("regexp: Compile({}): {}", quote(s), e),
        }
    }

    /// Longest makes future searches prefer leftmost-longest matches.
    // Go: regexp/regexp.go:Longest
    pub fn longest(&mut self) {
        Arc::make_mut(&mut self.inner).longest = true;
    }

    /// NumSubexp returns the number of parenthesized subexpressions in this Regexp.
    // Go: regexp/regexp.go:NumSubexp
    pub fn num_subexp(&self) -> usize {
        self.inner.num_subexp
    }

    /// SubexpNames returns the names of the parenthesized subexpressions in this Regexp. The
    /// name for the first sub-expression is names[1]. names[0] is always the empty string.
    // Go: regexp/regexp.go:SubexpNames
    pub fn subexp_names(&self) -> &[String] {
        &self.inner.subexp_names
    }

    /// SubexpIndex returns the index of the first subexpression with the given name, or -1 if
    /// there is no subexpression with that name.
    // Go: regexp/regexp.go:SubexpIndex
    pub fn subexp_index(&self, name: &str) -> isize {
        if !name.is_empty() {
            for (i, s) in self.inner.subexp_names.iter().enumerate() {
                if name == s {
                    return i as isize;
                }
            }
        }
        -1
    }

    /// LiteralPrefix returns a literal string that must begin any match of the regular
    /// expression re. It returns the boolean true if the literal string comprises the entire
    /// regular expression.
    // Go: regexp/regexp.go:LiteralPrefix
    pub fn literal_prefix(&self) -> (&[u8], bool) {
        (&self.inner.prefix, self.inner.prefix_complete)
    }

    /// MatchString reports whether the string s contains any match of the regular expression
    /// re (Go's `MatchString` and `Match`).
    // Go: regexp/regexp.go:MatchString
    pub fn match_string(&self, s: impl AsRef<[u8]>) -> bool {
        self.inner.do_match(s.as_ref())
    }

    /// Go: `re.Match(b)` (same as [`Regexp::match_string`]).
    // Go: regexp/regexp.go:Match
    pub fn is_match(&self, b: &[u8]) -> bool {
        self.inner.do_match(b)
    }

    /// ReplaceAll returns a copy of src, replacing matches of the Regexp with the replacement
    /// text repl. Inside repl, $ signs are interpreted as in Expand. (Go's `ReplaceAll` and
    /// `ReplaceAllString` run the same code.)
    // Go: regexp/regexp.go:ReplaceAll
    pub fn replace_all(&self, src: &[u8], repl: &[u8]) -> Vec<u8> {
        let re = &self.inner;
        let mut n = 2;
        if repl.contains(&b'$') {
            n = 2 * (re.num_subexp + 1);
        }
        re.replace_all(src, n, |dst, m| re.expand(dst, repl, src, m))
    }

    /// Go: `re.ReplaceAllString(src, repl)`.
    // Go: regexp/regexp.go:ReplaceAllString
    pub fn replace_all_string(&self, src: &str, repl: &str) -> String {
        let b = self.replace_all(src.as_bytes(), repl.as_bytes());
        // Matches of valid UTF-8 text are on character boundaries.
        String::from_utf8(b).expect("valid UTF-8")
    }

    /// ReplaceAllLiteral returns a copy of src, replacing matches of the Regexp with the
    /// replacement bytes repl. The replacement repl is substituted directly, without using
    /// Expand. (Go's `ReplaceAllLiteral` and `ReplaceAllLiteralString`.)
    // Go: regexp/regexp.go:ReplaceAllLiteral
    pub fn replace_all_literal(&self, src: &[u8], repl: &[u8]) -> Vec<u8> {
        self.inner
            .replace_all(src, 2, |dst, _| dst.extend_from_slice(repl))
    }

    /// Go: `re.ReplaceAllLiteralString(src, repl)`.
    // Go: regexp/regexp.go:ReplaceAllLiteralString
    pub fn replace_all_literal_string(&self, src: &str, repl: &str) -> String {
        let b = self.replace_all_literal(src.as_bytes(), repl.as_bytes());
        String::from_utf8(b).expect("valid UTF-8")
    }

    /// ReplaceAllFunc returns a copy of src in which all matches of the Regexp have been
    /// replaced by the return value of function repl applied to the matched byte slice. The
    /// replacement returned by repl is substituted directly, without using Expand.
    // Go: regexp/regexp.go:ReplaceAllFunc
    pub fn replace_all_func(&self, src: &[u8], mut repl: impl FnMut(&[u8]) -> Vec<u8>) -> Vec<u8> {
        self.inner.replace_all(src, 2, |dst, m| {
            dst.extend_from_slice(&repl(&src[m[0] as usize..m[1] as usize]))
        })
    }

    /// Find returns a slice holding the text of the leftmost match in b of the regular
    /// expression. A return value of `None` indicates no match.
    // Go: regexp/regexp.go:Find
    pub fn find<'s>(&self, b: &'s [u8]) -> Option<&'s [u8]> {
        let a = self.inner.find(b, 0, 2)?;
        Some(&b[a[0] as usize..a[1] as usize])
    }

    /// FindString returns a string holding the text of the leftmost match in s of the regular
    /// expression. If there is no match, the return value is an empty string, but it will
    /// also be empty if the regular expression successfully matches an empty string.
    // Go: regexp/regexp.go:FindString
    pub fn find_string<'s>(&self, s: &'s str) -> &'s str {
        match self.inner.find(s.as_bytes(), 0, 2) {
            None => "",
            Some(a) => &s[a[0] as usize..a[1] as usize],
        }
    }

    /// FindIndex returns a two-element slice of integers defining the location of the
    /// leftmost match in b of the regular expression. The match itself is at b[loc[0]:loc[1]].
    /// A return value of `None` indicates no match.
    // Go: regexp/regexp.go:FindIndex
    pub fn find_index(&self, b: &[u8]) -> Option<[usize; 2]> {
        let m = self.inner.find(b, 0, 2)?;
        Some([m[0] as usize, m[1] as usize])
    }

    /// FindSubmatch returns a slice of slices holding the text of the leftmost match of the
    /// regular expression in b and the matches, if any, of its subexpressions. A return value
    /// of `None` indicates no match; a group that did not participate is `None`.
    // Go: regexp/regexp.go:FindSubmatch
    pub fn find_submatch<'s>(&self, b: &'s [u8]) -> Option<Vec<Option<&'s [u8]>>> {
        let re = &self.inner;
        let m = re.find(b, 0, re.prog.num_cap)?;
        let mut sub = vec![None; 1 + re.num_subexp];
        for (i, s) in sub.iter_mut().enumerate() {
            if 2 * i < m.len() && m[2 * i] >= 0 {
                *s = Some(&b[m[2 * i] as usize..m[2 * i + 1] as usize]);
            }
        }
        Some(sub)
    }

    /// FindStringSubmatch returns a slice of strings holding the text of the leftmost match of
    /// the regular expression in s and the matches, if any, of its subexpressions. A group
    /// that did not participate is "". A return value of `None` indicates no match.
    // Go: regexp/regexp.go:FindStringSubmatch
    pub fn find_string_submatch(&self, s: &str) -> Option<Vec<String>> {
        let sub = self.find_submatch(s.as_bytes())?;
        Some(
            sub.into_iter()
                .map(|m| {
                    // Matches of valid UTF-8 text are on character boundaries.
                    m.map(|b| String::from_utf8(b.to_vec()).expect("valid UTF-8"))
                        .unwrap_or_default()
                })
                .collect(),
        )
    }

    /// FindSubmatchIndex returns a slice holding the index pairs identifying the leftmost
    /// match of the regular expression in b and the matches, if any, of its subexpressions.
    /// A return value of `None` indicates no match.
    // Go: regexp/regexp.go:FindSubmatchIndex
    pub fn find_submatch_index(&self, b: &[u8]) -> Option<Match> {
        let re = &self.inner;
        re.find(b, 0, re.prog.num_cap).map(|a| re.pad(a))
    }

    /// FindAll is the 'All' version of Find; it returns a slice of all successive matches of
    /// the expression. If n >= 0, the function returns at most n matches/submatches;
    /// otherwise, it returns all of them.
    // Go: regexp/regexp.go:FindAll
    pub fn find_all<'s>(&self, b: &'s [u8], n: isize) -> Vec<&'s [u8]> {
        let mut out = Vec::new();
        self.inner.matches(b, n, 2, |m| {
            out.push(&b[m[0] as usize..m[1] as usize]);
            true
        });
        out
    }

    /// Go: `re.FindAllString(s, n)`.
    // Go: regexp/regexp.go:FindAllString
    pub fn find_all_string<'s>(&self, s: &'s str, n: isize) -> Vec<&'s str> {
        let mut out = Vec::new();
        self.inner.matches(s.as_bytes(), n, 2, |m| {
            out.push(&s[m[0] as usize..m[1] as usize]);
            true
        });
        out
    }

    /// FindAllIndex is the 'All' version of FindIndex.
    // Go: regexp/regexp.go:FindAllIndex
    pub fn find_all_index(&self, b: &[u8], n: isize) -> Vec<[usize; 2]> {
        let mut out = Vec::new();
        self.inner.matches(b, n, 2, |m| {
            out.push([m[0] as usize, m[1] as usize]);
            true
        });
        out
    }

    /// FindAllSubmatch is the 'All' version of FindSubmatch.
    // Go: regexp/regexp.go:FindAllSubmatch
    pub fn find_all_submatch<'s>(&self, b: &'s [u8], n: isize) -> Vec<Vec<Option<&'s [u8]>>> {
        let mut out = Vec::new();
        self.inner.matches(b, n, self.inner.prog.num_cap, |m| {
            let mut sub = vec![None; m.len() / 2];
            for (i, s) in sub.iter_mut().enumerate() {
                if m[2 * i] >= 0 {
                    *s = Some(&b[m[2 * i] as usize..m[2 * i + 1] as usize]);
                }
            }
            out.push(sub);
            true
        });
        out
    }

    /// Go: `re.FindAllStringSubmatch(s, n)` (a group that did not participate is "").
    // Go: regexp/regexp.go:FindAllStringSubmatch
    pub fn find_all_string_submatch<'s>(&self, s: &'s str, n: isize) -> Vec<Vec<&'s str>> {
        let mut out = Vec::new();
        self.inner
            .matches(s.as_bytes(), n, self.inner.prog.num_cap, |m| {
                let mut sub = vec![""; m.len() / 2];
                for (i, x) in sub.iter_mut().enumerate() {
                    if m[2 * i] >= 0 {
                        *x = &s[m[2 * i] as usize..m[2 * i + 1] as usize];
                    }
                }
                out.push(sub);
                true
            });
        out
    }

    /// FindAllSubmatchIndex is the 'All' version of FindSubmatchIndex.
    // Go: regexp/regexp.go:FindAllSubmatchIndex
    pub fn find_all_submatch_index(&self, b: &[u8], n: isize) -> Vec<Match> {
        let mut out = Vec::new();
        self.inner.matches(b, n, self.inner.prog.num_cap, |m| {
            out.push(m.to_vec());
            true
        });
        out
    }

    /// Expand appends template to dst and returns the result; during the append, Expand
    /// replaces variables in the template with corresponding matches drawn from src. The match
    /// slice should have been returned by FindSubmatchIndex.
    ///
    /// In the template, a variable is denoted by a substring of the form $name or ${name},
    /// where name is a non-empty sequence of letters, digits, and underscores. A purely
    /// numeric name like $1 refers to the submatch with the corresponding index; other names
    /// refer to capturing parentheses named with the (?P<name>...) syntax. A reference to an
    /// out of range or unmatched index or a name that is not present in the regular expression
    /// is replaced with an empty slice.
    ///
    /// In the $name form, name is taken to be as long as possible: $1x is equivalent to
    /// ${1x}, not ${1}x, and, $10 is equivalent to ${10}, not ${1}0.
    ///
    /// To insert a literal $ in the output, use $$ in the template.
    // Go: regexp/regexp.go:Expand
    pub fn expand(&self, dst: &mut Vec<u8>, template: &[u8], src: &[u8], m: &[isize]) {
        self.inner.expand(dst, template, src, m)
    }

    /// Split slices s into substrings separated by the expression and returns a slice of the
    /// substrings between those expression matches.
    ///
    /// The count determines the number of substrings to return: n > 0: at most n substrings;
    /// the last substring will be the unsplit remainder; n == 0: the result is nil (zero
    /// substrings); n < 0: all substrings.
    // Go: regexp/regexp.go:Split
    pub fn split<'s>(&self, s: &'s [u8], n: isize) -> Vec<&'s [u8]> {
        if n == 0 {
            return Vec::new();
        }

        if !self.inner.expr.is_empty() && s.is_empty() {
            return vec![&s[..0]];
        }

        let matches = self.find_all_index(s, n);
        let mut strings: Vec<&[u8]> = Vec::with_capacity(matches.len());

        let mut beg = 0;
        let mut end = 0;
        for m in &matches {
            if n > 0 && strings.len() as isize >= n - 1 {
                break;
            }

            end = m[0];
            if m[1] != 0 {
                strings.push(&s[beg..end]);
            }
            beg = m[1];
        }

        if end != s.len() {
            strings.push(&s[beg..]);
        }

        strings
    }

    /// Go: `re.Split(s, n)` over a `&str`.
    pub fn split_string<'s>(&self, s: &'s str, n: isize) -> Vec<&'s str> {
        self.split(s.as_bytes(), n)
            .into_iter()
            .map(|b| {
                let start = b.as_ptr() as usize - s.as_ptr() as usize;
                &s[start..start + b.len()]
            })
            .collect()
    }
}

/// MatchString reports whether the string s contains any match of the regular expression
/// pattern.
// Go: regexp/regexp.go:MatchString
pub fn match_string(pattern: &str, s: impl AsRef<[u8]>) -> Result<bool, Error> {
    let re = Regexp::compile(pattern)?;
    Ok(re.match_string(s))
}

/// Patterns longer than this are compiled on a thread with [`DEEP_STACK`] bytes of stack: the
/// syntax tree can be about 1,000 levels deep (2,000 after `Simplify`), and its recursive walks
/// (Go grows goroutine stacks) need more than a default 2 MiB thread in debug builds.
const DEEP_PATTERN_LEN: usize = 1024;
const DEEP_STACK: usize = 256 << 20;

// Go: regexp/regexp.go:compile
fn compile(expr: &[u8], mode: syntax::Flags, longest: bool) -> Result<Regexp, Error> {
    if expr.len() > DEEP_PATTERN_LEN {
        let owned = expr.to_vec();
        let spawned = std::thread::Builder::new()
            .name("goregexp-compile".into())
            .stack_size(DEEP_STACK)
            .spawn(move || compile_inner(&owned, mode, longest));
        if let Ok(handle) = spawned {
            match handle.join() {
                Ok(r) => return r,
                Err(panic) => std::panic::resume_unwind(panic),
            }
        }
    }
    compile_inner(expr, mode, longest)
}

// Go: regexp/regexp.go:compile
fn compile_inner(expr: &[u8], mode: syntax::Flags, longest: bool) -> Result<Regexp, Error> {
    let re = syntax::parse(expr, mode).map_err(Error::from_syntax)?;
    let max_cap = re.max_cap();
    let cap_names = re.cap_names();

    let re = re.simplify();
    let prog = syntax::compile(&re);
    let matchcap = prog.num_cap.max(2);
    let onepass = compile_one_pass(&prog);
    let mut regexp = Inner {
        expr: expr.to_vec(),
        cond: prog.start_cond(),
        num_subexp: max_cap as usize,
        subexp_names: cap_names,
        longest,
        matchcap,
        min_input_len: min_input_len(&re),
        max_bit_state_len: 0,
        prefix: Vec::new(),
        prefix_rune: 0,
        prefix_end: 0,
        prefix_complete: false,
        onepass: None,
        prog,
    };
    match onepass {
        None => {
            let (prefix, complete) = regexp.prog.prefix();
            regexp.prefix = prefix;
            regexp.prefix_complete = complete;
            regexp.max_bit_state_len = backtrack::max_bit_state_len(&regexp.prog);
        }
        Some(op) => {
            let (prefix, complete, pc) = one_pass_prefix(&regexp.prog);
            regexp.prefix = prefix;
            regexp.prefix_complete = complete;
            regexp.prefix_end = pc;
            regexp.onepass = Some(op);
        }
    }
    if !regexp.prefix.is_empty() {
        // TODO(rsc): Remove this allocation by adding IndexString to package bytes.
        regexp.prefix_rune = utf8::decode_rune_in_string(&regexp.prefix).0;
    }

    Ok(Regexp {
        inner: Arc::new(regexp),
    })
}

/// minInputLen walks the regexp to find the minimum length of any matchable input.
// Go: regexp/regexp.go:minInputLen
fn min_input_len(re: &syntax::Regexp) -> usize {
    use syntax::Op;
    match re.op {
        Op::AnyChar | Op::AnyCharNotNL | Op::CharClass => 1,
        Op::Literal => {
            let mut l = 0;
            for &r in &re.rune {
                if r == RUNE_ERROR {
                    l += 1;
                } else {
                    l += utf8::rune_len(r) as usize;
                }
            }
            l
        }
        Op::Capture | Op::Plus => min_input_len(&re.sub[0]),
        Op::Repeat => re.min as usize * min_input_len(&re.sub[0]),
        Op::Concat => re.sub.iter().map(min_input_len).sum(),
        Op::Alternate => {
            let mut l = min_input_len(&re.sub[0]);
            for sub in &re.sub[1..] {
                let lnext = min_input_len(sub);
                if lnext < l {
                    l = lnext;
                }
            }
            l
        }
        _ => 0,
    }
}

// Go: regexp/regexp.go:quote
fn quote(s: &[u8]) -> String {
    if go_strconv::can_backquote(s) {
        return format!("`{}`", String::from_utf8_lossy(s));
    }
    go_strconv::quote(s)
}

/// endOfText is sent to the input step when there are no more runes.
pub(crate) const END_OF_TEXT: Rune = -1;

/// Go's `inputBytes`/`inputString` (the same code on bytes).
#[derive(Clone, Copy)]
pub(crate) struct Input<'a> {
    pub(crate) s: &'a [u8],
}

impl Input<'_> {
    // Go: regexp/regexp.go:step
    #[inline]
    pub(crate) fn step(&self, pos: usize) -> (Rune, usize) {
        if pos < self.s.len() {
            let c = self.s[pos];
            if (c as Rune) < RUNE_SELF {
                return (Rune::from(c), 1);
            }
            return utf8::decode_rune(&self.s[pos..]);
        }
        (END_OF_TEXT, 0)
    }

    // Go: regexp/regexp.go:hasPrefix
    pub(crate) fn has_prefix(&self, re: &Inner) -> bool {
        self.s.starts_with(&re.prefix)
    }

    // Go: regexp/regexp.go:index
    pub(crate) fn index(&self, re: &Inner, pos: usize) -> isize {
        go_unicode::bytes::index(&self.s[pos..], &re.prefix)
    }

    // Go: regexp/regexp.go:context
    pub(crate) fn context(&self, pos: usize) -> exec::LazyFlag {
        let (mut r1, mut r2) = (END_OF_TEXT, END_OF_TEXT);
        // 0 < pos && pos <= len(i.str)
        if pos > 0 && pos <= self.s.len() {
            r1 = utf8::decode_last_rune(&self.s[..pos]).0;
        }
        // 0 <= pos && pos < len(i.str)
        if pos < self.s.len() {
            r2 = utf8::decode_rune(&self.s[pos..]).0;
        }
        exec::LazyFlag::new(r1, r2)
    }
}

impl Inner {
    // Go: regexp/regexp.go:replaceAll
    fn replace_all(
        &self,
        src: &[u8],
        mut nmatch: usize,
        mut repl: impl FnMut(&mut Vec<u8>, &[isize]),
    ) -> Vec<u8> {
        let mut last_match_end = 0; // end position of the most recent match
        let mut search_pos = 0; // position where we next look for a match
        let mut buf: Vec<u8> = Vec::new();
        let end_pos = src.len();
        if nmatch > self.prog.num_cap {
            nmatch = self.prog.num_cap;
        }

        while search_pos <= end_pos {
            let Some(a) = self.find(src, search_pos, nmatch) else {
                break; // no more matches
            };
            if a.is_empty() {
                break;
            }

            // Copy the unmatched characters before this match.
            buf.extend_from_slice(&src[last_match_end..a[0] as usize]);

            // Now insert a copy of the replacement string, but not for a match of the empty
            // string immediately after another match. (Otherwise, we get double replacement
            // for patterns that match both empty and nonempty strings.)
            if a[1] as usize > last_match_end || a[0] == 0 {
                repl(&mut buf, &a);
            }
            last_match_end = a[1] as usize;

            // Advance past this match; always advance at least one character.
            let width = if search_pos < src.len() {
                utf8::decode_rune(&src[search_pos..]).1
            } else {
                0
            };
            if search_pos + width > a[1] as usize {
                search_pos += width;
            } else if search_pos + 1 > a[1] as usize {
                // This clause is only needed at the end of the input string. In that case,
                // DecodeRuneInString returns width=0.
                search_pos += 1;
            } else {
                search_pos = a[1] as usize;
            }
        }

        // Copy the unmatched characters after the last match.
        buf.extend_from_slice(&src[last_match_end..]);

        buf
    }

    // Go: regexp/regexp.go:pad
    fn pad(&self, mut a: Match) -> Match {
        let n = (1 + self.num_subexp) * 2;
        while a.len() < n {
            a.push(-1);
        }
        a
    }

    /// Go's `matches` iterator: calls `yield_` with each successive match (padded) until it
    /// returns false.
    // Go: regexp/regexp.go:matches
    fn matches(
        &self,
        b: &[u8],
        mut max: isize,
        ncap: usize,
        mut yield_: impl FnMut(&[isize]) -> bool,
    ) {
        if max == 0 {
            return;
        }
        let end = b.len();
        let mut pos = 0;
        let mut prev_match_end: isize = -1;
        while pos <= end {
            let Some(matches) = self.find(b, pos, ncap) else {
                break;
            };
            if matches.is_empty() {
                break;
            }

            let mut accept = true;
            if matches[1] as usize == pos {
                // We've found an empty match.
                if matches[0] == prev_match_end {
                    // We don't allow an empty match right after a previous match, so ignore
                    // it.
                    accept = false;
                }
                // Move to next rune.
                let width = Input { s: b }.step(pos).1;
                if width > 0 {
                    pos += width;
                } else {
                    pos = end + 1;
                }
            } else {
                pos = matches[1] as usize;
            }
            prev_match_end = matches[1];

            if accept {
                let m = self.pad(matches);
                if !yield_(&m) {
                    return;
                }
                if max > 0 {
                    max -= 1;
                    if max == 0 {
                        return;
                    }
                }
            }
        }
    }

    // Go: regexp/regexp.go:expand
    fn expand(&self, dst: &mut Vec<u8>, mut template: &[u8], src: &[u8], m: &[isize]) {
        while !template.is_empty() {
            let Some(i) = template.iter().position(|&c| c == b'$') else {
                break;
            };
            let (before, after) = (&template[..i], &template[i + 1..]);
            dst.extend_from_slice(before);
            template = after;
            if !template.is_empty() && template[0] == b'$' {
                // Treat $$ as $.
                dst.push(b'$');
                template = &template[1..];
                continue;
            }
            let Some((name, num, rest)) = extract(template) else {
                // Malformed; treat $ as raw text.
                dst.push(b'$');
                continue;
            };
            template = rest;
            if num >= 0 {
                let num = num as usize;
                if 2 * num + 1 < m.len() && m[2 * num] >= 0 {
                    dst.extend_from_slice(&src[m[2 * num] as usize..m[2 * num + 1] as usize]);
                }
            } else {
                for (i, namei) in self.subexp_names.iter().enumerate() {
                    if name == namei.as_bytes() && 2 * i + 1 < m.len() && m[2 * i] >= 0 {
                        dst.extend_from_slice(&src[m[2 * i] as usize..m[2 * i + 1] as usize]);
                        break;
                    }
                }
            }
        }
        dst.extend_from_slice(template);
    }
}

/// extract returns the name from a leading "name" or "{name}" in str. (The $ has already been
/// removed by the caller.) If it is a number, idx is the corresponding index; otherwise idx is
/// -1.
// Go: regexp/regexp.go:extract
fn extract(s: &[u8]) -> Option<(&[u8], isize, &[u8])> {
    if s.is_empty() {
        return None;
    }
    let mut str = s;
    let mut brace = false;
    if str[0] == b'{' {
        brace = true;
        str = &str[1..];
    }
    let mut i = 0;
    while i < str.len() {
        let (rune, size) = utf8::decode_rune_in_string(&str[i..]);
        if !go_unicode::is_letter(rune) && !go_unicode::is_digit(rune) && rune != '_' as Rune {
            break;
        }
        i += size;
    }
    if i == 0 {
        // empty name is not okay
        return None;
    }
    let name = &str[..i];
    if brace {
        if i >= str.len() || str[i] != b'}' {
            // missing closing brace
            return None;
        }
        i += 1;
    }

    // Parse number.
    let mut num: isize = 0;
    for &c in name {
        if !c.is_ascii_digit() || num >= 100_000_000 {
            num = -1;
            break;
        }
        num = num * 10 + (c - b'0') as isize;
    }
    // Disallow leading zeros.
    if name[0] == b'0' && name.len() > 1 {
        num = -1;
    }

    let rest = &str[i..];
    Some((name, num, rest))
}

/// Bitmap used by func special to check whether a character needs to be escaped.
// Go: regexp/regexp.go:special
fn special(b: u8) -> bool {
    br"\.+*?()|[]{}^$".contains(&b)
}

/// QuoteMeta returns a string that escapes all regular expression metacharacters inside the
/// argument text; the returned string is a regular expression matching the literal text.
// Go: regexp/regexp.go:QuoteMeta
pub fn quote_meta(s: &[u8]) -> Vec<u8> {
    let mut b = Vec::with_capacity(2 * s.len());
    for &c in s {
        if special(c) {
            b.push(b'\\');
        }
        b.push(c);
    }
    b
}

/// Go: `hstrings`' regexp cache (`regexpCache`): compiled regexps by pattern, shared by all
/// callers (a successful compile is cached; errors are not).
static CACHE: OnceLock<RwLock<HashMap<Vec<u8>, Regexp>>> = OnceLock::new();

/// Go: `hstrings.GetOrCompileRegexp(pattern)`: the cached compiled regexp for the pattern.
// Go: common/hstrings/strings.go:getOrCompileRegexp
pub(crate) fn get_or_compile(pattern: &[u8]) -> Result<Regexp, Error> {
    let cache = CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    if let Some(re) = cache.read().unwrap_or_else(|e| e.into_inner()).get(pattern) {
        return Ok(re.clone());
    }
    let re = Regexp::compile_bytes(pattern)?;
    cache
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(pattern.to_vec(), re.clone());
    Ok(re)
}
