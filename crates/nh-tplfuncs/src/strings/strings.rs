//! Port of `tpl/strings/strings.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::{Arc, LazyLock};

use go_unicode::strings as gostrings;
use go_unicode::utf8;
use go_value::{GoString, HostCtx, Object, SafeKind, SliceType, Value};
use nh_common::cast::caste;
use nh_common::goregexp::Regexp;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

// Parity notes: Go `strings` semantics with go-unicode (simple case mapping); `substr`/`truncate` rune based.

/// Go: `strings.Namespace` (template value `*strings.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
}

pub(crate) fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

/// `cast.ToStringE(v)` with the error as a template error.
pub(crate) fn to_string_e(v: &Value) -> GoResult<GoString> {
    Ok(caste::to_string_e(v)?)
}

/// `cast.ToIntE(v)` with the error as a template error.
pub(crate) fn to_int_e(v: &Value) -> GoResult<i64> {
    Ok(caste::to_int_e(v)?)
}

/// A Go `string` result.
pub(crate) fn sv(b: impl Into<GoString>) -> Value {
    Value::String(b.into())
}

/// A Go `[]string` result (`nil` = a nil slice).
pub(crate) fn string_slice(items: Option<Vec<Vec<u8>>>) -> Value {
    match items {
        None => Value::TypedNil(Arc::from("[]string")),
        Some(items) => Value::list(
            SliceType::String,
            items
                .into_iter()
                .map(|s| Value::String(GoString::from(s)))
                .collect(),
        ),
    }
}

/// The runes of a Go string (`[]rune(s)`: an invalid byte is `U+FFFD`).
pub(crate) fn runes(s: &[u8]) -> Vec<go_unicode::Rune> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let (r, n) = utf8::decode_rune(&s[i..]);
        out.push(r);
        i += n;
    }
    out
}

/// `string(runes)`.
pub(crate) fn runes_to_string(rs: &[go_unicode::Rune]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rs.len());
    for &r in rs {
        utf8::append_rune(&mut out, r);
    }
    out
}

/// `fmt.Errorf("<prefix>: %w", err)`.
fn wrap(prefix: &str, e: nh_common::herrors::Error) -> go_value::Error {
    gerr(format!("{prefix}: {}", e.message()))
}

static CJK_RE: LazyLock<Regexp> =
    LazyLock::new(|| Regexp::must_compile(r"\p{Han}|\p{Hangul}|\p{Hiragana}|\p{Katakana}"));

impl Namespace {
    /// New returns a new instance of the strings-namespaced template functions.
    // Go: tpl/strings/strings.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        Namespace { d }
    }

    /// Chomp returns a copy of s with all trailing newline characters removed.
    // Go: tpl/strings/strings.go:Chomp
    pub fn chomp(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Chomp")?;
        let ss = to_string_e(&a[0])?;
        let res = nh_common::text::chomp_bytes(ss.as_bytes()).to_vec();
        match &a[0] {
            Value::Safe(SafeKind::Html, _) => Ok(Value::Safe(SafeKind::Html, res.into())),
            _ => Ok(sv(res)),
        }
    }

    /// Contains reports whether substr is in s.
    // Go: tpl/strings/strings.go:Contains
    pub fn contains(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Contains")?;
        let ss = to_string_e(&a[0])?;
        let su = to_string_e(&a[1])?;
        Ok(Value::Bool(gostrings::contains(
            ss.as_bytes(),
            su.as_bytes(),
        )))
    }

    /// ContainsAny reports whether any Unicode code points in chars are within s.
    // Go: tpl/strings/strings.go:ContainsAny
    pub fn contains_any(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "ContainsAny")?;
        let ss = to_string_e(&a[0])?;
        let sc = to_string_e(&a[1])?;
        Ok(Value::Bool(gostrings::contains_any(
            ss.as_bytes(),
            sc.as_bytes(),
        )))
    }

    /// ContainsNonSpace reports whether s contains any non-space characters as defined by
    /// Unicode's White Space property.
    // Go: tpl/strings/strings.go:ContainsNonSpace
    pub fn contains_non_space(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ContainsNonSpace")?;
        let ss = to_string_e(&a[0])?;
        for r in runes(ss.as_bytes()) {
            if !go_unicode::is_space(r) {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    /// Count counts the number of non-overlapping instances of substr in s. If substr is an
    /// empty string, Count returns 1 + the number of Unicode code points in s.
    // Go: tpl/strings/strings.go:Count
    pub fn count(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Count")?;
        let substrs =
            caste::to_string_e(&a[0]).map_err(|e| wrap("failed to convert substr to string", e))?;
        let ss = caste::to_string_e(&a[1]).map_err(|e| wrap("failed to convert s to string", e))?;
        Ok(Value::int(
            gostrings::count(ss.as_bytes(), substrs.as_bytes()) as i64,
        ))
    }

    /// CountRunes returns the number of runes in s, excluding whitespace.
    // Go: tpl/strings/strings.go:CountRunes
    pub fn count_runes(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "CountRunes")?;
        let ss = caste::to_string_e(&a[0])
            .map_err(|e| wrap("failed to convert content to string", e))?;
        let mut counter = 0i64;
        for r in runes(&nh_tpl::template::strip_html(ss.as_bytes())) {
            // Go: helpers.IsWhitespace.
            if !(r == ' ' as i32 || r == '\t' as i32 || r == '\n' as i32 || r == '\r' as i32) {
                counter += 1;
            }
        }
        Ok(Value::int(counter))
    }

    /// CountWords returns the approximate word count in s.
    // Go: tpl/strings/strings.go:CountWords
    pub fn count_words(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "CountWords")?;
        let ss = caste::to_string_e(&a[0])
            .map_err(|e| wrap("failed to convert content to string", e))?;

        let is_cjk_language = CJK_RE.match_string(ss.as_bytes());

        let stripped = nh_tpl::template::strip_html(ss.as_bytes());
        if !is_cjk_language {
            return Ok(Value::int(gostrings::fields(&stripped).len() as i64));
        }

        let mut counter = 0i64;
        for word in gostrings::fields(&stripped) {
            let rune_count = utf8::rune_count_in_string(word);
            if word.len() == rune_count {
                counter += 1;
            } else {
                counter += rune_count as i64;
            }
        }
        Ok(Value::int(counter))
    }

    /// Diff returns an anchored diff of the two texts old and new in the "unified diff"
    /// format. If old and new are identical, Diff returns an empty string.
    // Go: tpl/strings/strings.go:Diff
    pub fn diff(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 4, "Diff")?;
        let oldname = args::string(a, 0)?;
        let newname = args::string(a, 2)?;
        let olds = to_string_e(&a[1])?;
        let news = to_string_e(&a[3])?;
        Ok(sv(super::diff::diff(
            oldname.as_bytes(),
            olds.as_bytes(),
            newname.as_bytes(),
            news.as_bytes(),
        )))
    }

    /// FirstUpper converts s making the first character upper case.
    // Go: tpl/strings/strings.go:FirstUpper
    pub fn first_upper(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "FirstUpper")?;
        let ss = to_string_e(&a[0])?;
        // Go: helpers.FirstUpper (over the string's bytes).
        let s = ss.as_bytes();
        if s.is_empty() {
            return Ok(sv(""));
        }
        let (r, n) = utf8::decode_rune(s);
        let mut out = utf8::rune_to_string(go_unicode::to_upper(r));
        out.extend_from_slice(&s[n..]);
        Ok(sv(out))
    }

    /// HasPrefix tests whether the input s begins with prefix.
    // Go: tpl/strings/strings.go:HasPrefix
    pub fn has_prefix(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "HasPrefix")?;
        let ss = to_string_e(&a[0])?;
        let sx = to_string_e(&a[1])?;
        Ok(Value::Bool(gostrings::has_prefix(
            ss.as_bytes(),
            sx.as_bytes(),
        )))
    }

    /// HasSuffix tests whether the input s begins with suffix.
    // Go: tpl/strings/strings.go:HasSuffix
    pub fn has_suffix(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "HasSuffix")?;
        let ss = to_string_e(&a[0])?;
        let sx = to_string_e(&a[1])?;
        Ok(Value::Bool(gostrings::has_suffix(
            ss.as_bytes(),
            sx.as_bytes(),
        )))
    }

    /// Repeat returns a new string consisting of n copies of the string s.
    ///
    /// Go's `strings.Repeat` panics when `len(s) * n` overflows and when the result cannot be
    /// allocated (`bytealg.MakeNoZero` beyond `maxAlloc`, 1<<48 on 64-bit linux and darwin);
    /// text/template reports the panic value as the call's error, and so does the port.
    // Go: tpl/strings/strings.go:Repeat
    pub fn repeat(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Repeat")?;
        let ss = to_string_e(&a[1])?;
        let sn = to_int_e(&a[0])?;

        if sn < 0 {
            return Err(gerr("strings: negative Repeat count"));
        }

        // Go: strings.Repeat.
        match sn {
            0 => return Ok(sv("")),
            1 => return Ok(Value::String(ss)),
            _ => {}
        }
        let Some(n) = (ss.len() as u128)
            .checked_mul(sn as u128)
            .filter(|&n| n <= i64::MAX as u128)
        else {
            return Err(gerr("strings: Repeat output length overflow"));
        };
        if ss.is_empty() {
            return Ok(sv(""));
        }
        const MAX_ALLOC: u128 = 1 << 48;
        if n > MAX_ALLOC {
            return Err(gerr("runtime error: makeslice: len out of range"));
        }
        Ok(sv(gostrings::repeat(ss.as_bytes(), sn as isize)))
    }

    /// Replace returns a copy of the string s with all occurrences of old replaced with new.
    /// The number of replacements can be limited with an optional fourth parameter.
    // Go: tpl/strings/strings.go:Replace
    pub fn replace(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 3, "Replace")?;
        let ss = to_string_e(&a[0])?;
        let so = to_string_e(&a[1])?;
        let sn = to_string_e(&a[2])?;

        if a.len() == 3 {
            return Ok(sv(gostrings::replace_all(
                ss.as_bytes(),
                so.as_bytes(),
                sn.as_bytes(),
            )
            .into_owned()));
        }

        let lim = to_int_e(&a[3])?;

        Ok(sv(gostrings::replace(
            ss.as_bytes(),
            so.as_bytes(),
            sn.as_bytes(),
            lim as isize,
        )
        .into_owned()))
    }

    /// RuneCount returns the number of runes in s.
    // Go: tpl/strings/strings.go:RuneCount
    pub fn rune_count(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "RuneCount")?;
        let ss = caste::to_string_e(&a[0])
            .map_err(|e| wrap("failed to convert content to string", e))?;
        Ok(Value::int(utf8::rune_count_in_string(ss.as_bytes()) as i64))
    }

    /// SliceString slices a string by specifying a half-open range with two indices, start and
    /// end. 1 and 4 creates a slice including elements 1 through 3. The end index can be
    /// omitted, it defaults to the string's length.
    // Go: tpl/strings/strings.go:SliceString
    pub fn slice_string(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "SliceString")?;
        let a_str = to_string_e(&a[0])?;
        let start_end = &a[1..];

        let mut arg_start: i64 = 0;
        let mut arg_end: i64 = 0;

        let arg_num = start_end.len();

        if arg_num > 0 {
            arg_start = caste::to_int_e(&start_end[0])
                .map_err(|_| gerr("start argument must be integer"))?;
        }
        if arg_num > 1 {
            arg_end =
                caste::to_int_e(&start_end[1]).map_err(|_| gerr("end argument must be integer"))?;
        }

        if arg_num > 2 {
            return Err(gerr("too many arguments"));
        }

        let as_runes = runes(a_str.as_bytes());
        let n = as_runes.len() as i64;

        if arg_num > 0 && (arg_start < 0 || arg_start >= n) {
            return Err(gerr("slice bounds out of range"));
        }

        match arg_num {
            2 => {
                if arg_end < 0 || arg_end > n {
                    return Err(gerr("slice bounds out of range"));
                }
                if arg_start > arg_end {
                    // Go: `asRunes[argStart:argEnd]` panics.
                    return Err(gerr(format!(
                        "runtime error: slice bounds out of range [{arg_start}:{arg_end}]"
                    )));
                }
                Ok(sv(runes_to_string(
                    &as_runes[arg_start as usize..arg_end as usize],
                )))
            }
            1 => Ok(sv(runes_to_string(&as_runes[arg_start as usize..]))),
            _ => Ok(sv(runes_to_string(&as_runes))),
        }
    }

    /// Split slices an input string into all substrings separated by delimiter.
    // Go: tpl/strings/strings.go:Split
    pub fn split(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Split")?;
        let delimiter = args::string(a, 1)?;
        let a_str = to_string_e(&a[0])?;
        Ok(string_slice(Some(
            gostrings::split(a_str.as_bytes(), delimiter.as_bytes())
                .into_iter()
                .map(|s| s.to_vec())
                .collect(),
        )))
    }

    /// Substr extracts parts of a string, beginning at the character at the specified
    /// position, and returns the specified number of characters.
    ///
    /// It normally takes two parameters: start and length. It can also take one parameter:
    /// start, i.e. length is omitted, in which case the substring starting from start until the
    /// end of the string will be returned.
    ///
    /// To extract characters from the end of the string, use a negative start number.
    ///
    /// In addition, borrowing from the extended behavior described at http://php.net/substr,
    /// if length is given and is negative, then that many characters will be omitted from the
    /// end of string.
    // Go: tpl/strings/strings.go:Substr
    pub fn substr(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Substr")?;
        let s = to_string_e(&a[0])?;
        let nums = &a[1..];

        let as_runes = runes(s.as_bytes());
        let rlen = as_runes.len() as i64;

        let mut start: i64;
        let length: i64;

        match nums.len() {
            0 => return Err(gerr("too few arguments")),
            1 => {
                start = caste::to_int_e(&nums[0])
                    .map_err(|_| gerr("start argument must be an integer"))?;
                length = rlen;
            }
            2 => {
                start = caste::to_int_e(&nums[0])
                    .map_err(|_| gerr("start argument must be an integer"))?;
                length = caste::to_int_e(&nums[1])
                    .map_err(|_| gerr("length argument must be an integer"))?;
            }
            _ => return Err(gerr("too many arguments")),
        }

        if rlen == 0 {
            return Ok(sv(""));
        }

        if start < 0 {
            start = start.wrapping_add(rlen);
        }

        // start was originally negative beyond rlen
        if start < 0 {
            start = 0;
        }

        if start > rlen - 1 {
            return Ok(sv(""));
        }

        let mut end = rlen;

        match length.cmp(&0) {
            std::cmp::Ordering::Equal => return Ok(sv("")),
            std::cmp::Ordering::Less => end = end.wrapping_add(length),
            std::cmp::Ordering::Greater => end = start.wrapping_add(length),
        }

        if start >= end {
            return Ok(sv(""));
        }

        if end < 0 {
            return Ok(sv(""));
        }

        if end > rlen {
            end = rlen;
        }

        Ok(sv(runes_to_string(&as_runes[start as usize..end as usize])))
    }

    /// Title returns a copy of the input s with all Unicode letters that begin words mapped to
    /// their title case (the site's `titleCaseStyle`).
    // Go: tpl/strings/strings.go:Title
    pub fn title(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "Title")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(self.d.conf.create_title(&ss.to_str_lossy())))
    }

    /// ToLower returns a copy of the input s with all Unicode letters mapped to their lower
    /// case.
    // Go: tpl/strings/strings.go:ToLower
    pub fn to_lower(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ToLower")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(gostrings::to_lower(ss.as_bytes()).into_owned()))
    }

    /// ToUpper returns a copy of the input s with all Unicode letters mapped to their upper
    /// case.
    // Go: tpl/strings/strings.go:ToUpper
    pub fn to_upper(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "ToUpper")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(gostrings::to_upper(ss.as_bytes()).into_owned()))
    }

    /// Trim returns converts the strings s removing all leading and trailing characters defined
    /// contained.
    // Go: tpl/strings/strings.go:Trim
    pub fn trim(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Trim")?;
        let ss = to_string_e(&a[0])?;
        let sc = to_string_e(&a[1])?;
        Ok(sv(gostrings::trim(ss.as_bytes(), sc.as_bytes()).to_vec()))
    }

    /// TrimLeft returns a slice of the string s with all leading characters contained in cutset
    /// removed.
    // Go: tpl/strings/strings.go:TrimLeft
    pub fn trim_left(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "TrimLeft")?;
        let ss = to_string_e(&a[1])?;
        let sc = to_string_e(&a[0])?;
        Ok(sv(
            gostrings::trim_left(ss.as_bytes(), sc.as_bytes()).to_vec()
        ))
    }

    /// TrimPrefix returns s without the provided leading prefix string. If s doesn't start with
    /// prefix, s is returned unchanged.
    // Go: tpl/strings/strings.go:TrimPrefix
    pub fn trim_prefix(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "TrimPrefix")?;
        let ss = to_string_e(&a[1])?;
        let sx = to_string_e(&a[0])?;
        Ok(sv(
            gostrings::trim_prefix(ss.as_bytes(), sx.as_bytes()).to_vec()
        ))
    }

    /// TrimRight returns a slice of the string s with all trailing characters contained in
    /// cutset removed.
    // Go: tpl/strings/strings.go:TrimRight
    pub fn trim_right(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "TrimRight")?;
        let ss = to_string_e(&a[1])?;
        let sc = to_string_e(&a[0])?;
        Ok(sv(
            gostrings::trim_right(ss.as_bytes(), sc.as_bytes()).to_vec()
        ))
    }

    /// TrimSpace returns the given string, removing leading and trailing whitespace as defined
    /// by Unicode.
    // Go: tpl/strings/strings.go:TrimSpace
    pub fn trim_space(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 1, "TrimSpace")?;
        let ss = to_string_e(&a[0])?;
        Ok(sv(gostrings::trim_space(ss.as_bytes()).to_vec()))
    }

    /// TrimSuffix returns s without the provided trailing suffix string. If s doesn't end with
    /// suffix, s is returned unchanged.
    // Go: tpl/strings/strings.go:TrimSuffix
    pub fn trim_suffix(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "TrimSuffix")?;
        let ss = to_string_e(&a[1])?;
        let sx = to_string_e(&a[0])?;
        Ok(sv(
            gostrings::trim_suffix(ss.as_bytes(), sx.as_bytes()).to_vec()
        ))
    }
}

nh_common::go_methods!(Namespace {
    "Chomp" => |n, ctx, a| n.chomp(ctx, a),
    "Contains" => |n, ctx, a| n.contains(ctx, a),
    "ContainsAny" => |n, ctx, a| n.contains_any(ctx, a),
    "ContainsNonSpace" => |n, ctx, a| n.contains_non_space(ctx, a),
    "Count" => |n, ctx, a| n.count(ctx, a),
    "CountRunes" => |n, ctx, a| n.count_runes(ctx, a),
    "CountWords" => |n, ctx, a| n.count_words(ctx, a),
    "Diff" => |n, ctx, a| n.diff(ctx, a),
    "FindRE" => |n, ctx, a| n.find_re(ctx, a),
    "FindRESubmatch" => |n, ctx, a| n.find_re_submatch(ctx, a),
    "FirstUpper" => |n, ctx, a| n.first_upper(ctx, a),
    "HasPrefix" => |n, ctx, a| n.has_prefix(ctx, a),
    "HasSuffix" => |n, ctx, a| n.has_suffix(ctx, a),
    "Repeat" => |n, ctx, a| n.repeat(ctx, a),
    "Replace" => |n, ctx, a| n.replace(ctx, a),
    "ReplaceRE" => |n, ctx, a| n.replace_re(ctx, a),
    "RuneCount" => |n, ctx, a| n.rune_count(ctx, a),
    "SliceString" => |n, ctx, a| n.slice_string(ctx, a),
    "Split" => |n, ctx, a| n.split(ctx, a),
    "Substr" => |n, ctx, a| n.substr(ctx, a),
    "Title" => |n, ctx, a| n.title(ctx, a),
    "ToLower" => |n, ctx, a| n.to_lower(ctx, a),
    "ToUpper" => |n, ctx, a| n.to_upper(ctx, a),
    "Trim" => |n, ctx, a| n.trim(ctx, a),
    "TrimLeft" => |n, ctx, a| n.trim_left(ctx, a),
    "TrimPrefix" => |n, ctx, a| n.trim_prefix(ctx, a),
    "TrimRight" => |n, ctx, a| n.trim_right(ctx, a),
    "TrimSpace" => |n, ctx, a| n.trim_space(ctx, a),
    "TrimSuffix" => |n, ctx, a| n.trim_suffix(ctx, a),
    "Truncate" => |n, ctx, a| n.truncate(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*strings.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/strings/strings.go (546 lines; 5/27 funcs executed)
//   types: Namespace
// OK L36-38: New(d *deps.Deps) *Namespace
// OK L48-62: (ns *Namespace) CountRunes(s any) (int, error)
// OK L65-71: (ns *Namespace) RuneCount(s any) (int, error)
// OK L74-100: (ns *Namespace) CountWords(s any) (int, error)
// OK L104-114: (ns *Namespace) Count(substr, s any) (int, error)
// OK L117-130: (ns *Namespace) Chomp(s any) (any, error)
// OK L133-145: (ns *Namespace) Contains(s, substr any) (bool, error)
// OK L148-160: (ns *Namespace) ContainsAny(s, chars any) (bool, error)
// OK L165-177: (ns *Namespace) ContainsNonSpace(s any) (bool, error)
// OK L181-191: (ns *Namespace) Diff(oldname string, old any, newname string, new any) (string, error)
// OK L194-206: (ns *Namespace) HasPrefix(s, prefix any) (bool, error)
// OK L209-221: (ns *Namespace) HasSuffix(s, suffix any) (bool, error)
// OK L226-252: (ns *Namespace) Replace(s, old, new any, limit ...any) (string, error)
// OK L257-299: (ns *Namespace) SliceString(a any, startEnd ...any) (string, error)
// OK L302-309: (ns *Namespace) Split(a any, delimiter string) ([]string, error)
// OK L323-394: (ns *Namespace) Substr(a any, nums ...any) (string, error)
// OK L398-404: (ns *Namespace) Title(s any) (string, error)
// OK L407-414: (ns *Namespace) FirstUpper(s any) (string, error)
// OK L418-425: (ns *Namespace) ToLower(s any) (string, error)
// OK L429-436: (ns *Namespace) ToUpper(s any) (string, error)
// OK L440-452: (ns *Namespace) Trim(s, cutset any) (string, error)
// OK L456-463: (ns *Namespace) TrimSpace(s any) (string, error)
// OK L467-479: (ns *Namespace) TrimLeft(cutset, s any) (string, error)
// OK L483-495: (ns *Namespace) TrimPrefix(prefix, s any) (string, error)
// OK L499-511: (ns *Namespace) TrimRight(cutset, s any) (string, error)
// OK L515-527: (ns *Namespace) TrimSuffix(suffix, s any) (string, error)
// OK L530-546: (ns *Namespace) Repeat(n, s any) (string, error)
// ---------------------------------------------------------------------------
