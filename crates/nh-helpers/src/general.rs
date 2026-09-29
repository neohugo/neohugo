//! Port of `helpers/general.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).

use std::io::Read;
use std::sync::Arc;

use go_unicode::utf8;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::prose::{TitleConverter, TitleStyle};

/// Go: `helpers.FilePathSeparator` (unix).
pub const FILE_PATH_SEPARATOR: &str = "/";

/// Go: `helpers.TCPListen()` — only used by the server; not supported.
// Go: helpers/general.go:TCPListen
pub fn tcp_listen() -> Result<()> {
    Err(Error::new(
        "neohugo-rs: TCPListen (server) is not supported",
    ))
}

/// Go: `helpers.FirstUpper` — `unicode.ToUpper` of the first rune (an invalid first byte is
/// `U+FFFD`, width 1, like `utf8.DecodeRuneInString`).
// Go: helpers/general.go:FirstUpper
pub fn first_upper(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let (r, n) = utf8::decode_rune_in_string(s.as_bytes());
    let mut out = utf8::rune_to_string(go_unicode::to_upper(r));
    out.extend_from_slice(&s.as_bytes()[n..]);
    String::from_utf8(out).expect("valid UTF-8 in, valid UTF-8 out")
}

/// [`first_upper`] over Go string bytes: an invalid first byte decodes to `U+FFFD` (width 1),
/// which is written as its UTF-8 encoding; the rest is copied as is.
// Go: helpers/general.go:FirstUpper
pub fn first_upper_bytes(s: &[u8]) -> Vec<u8> {
    if s.is_empty() {
        return Vec::new();
    }
    let (r, n) = utf8::decode_rune_in_string(s);
    let mut out = utf8::rune_to_string(go_unicode::to_upper(r));
    out.extend_from_slice(&s[n..]);
    out
}

/// Go: `helpers.UniqueStrings(s)` — a copy with the duplicates removed, order preserved.
// Go: helpers/general.go:UniqueStrings
pub fn unique_strings(s: &[String]) -> Vec<String> {
    let mut unique = Vec::with_capacity(s.len());
    for (i, val) in s.iter().enumerate() {
        let mut seen = false;
        for prev in &s[..i] {
            if prev == val {
                seen = true;
                break;
            }
        }
        if !seen {
            unique.push(val.clone());
        }
    }
    unique
}

/// Go: `helpers.UniqueStringsReuse(s)` (order preserving, in place).
///
/// Go compares each element with the (already compacted) prefix `s[:i]` of the SAME slice, so an
/// element can be compared against a value moved into an earlier slot. The port keeps that.
// Go: helpers/general.go:UniqueStringsReuse
pub fn unique_strings_reuse(mut s: Vec<String>) -> Vec<String> {
    let mut n = 0usize; // len(result)
    for i in 0..s.len() {
        let val = s[i].clone();
        let seen = s[..i].contains(&val);
        if !seen {
            // result = append(result, val) writes into s[n].
            s[n] = val;
            n += 1;
        }
    }
    s.truncate(n);
    s
}

/// Go: `helpers.UniqueStringsSorted(s)` — `sort.Strings` + dedupe; empty -> nil (None).
// Go: helpers/general.go:UniqueStringsSorted
pub fn unique_strings_sorted(mut s: Vec<String>) -> Option<Vec<String>> {
    if s.is_empty() {
        return None;
    }
    // sort.StringSlice.Sort: equal strings are indistinguishable, so any sort gives Go's order.
    s.sort();
    let mut i = 0usize;
    for j in 1..s.len() {
        if s[i] >= s[j] {
            continue;
        }
        i += 1;
        s[i] = s[j].clone();
    }
    s.truncate(i + 1);
    Some(s)
}

/// Go: `helpers.ReaderToBytes(lines)` — `None` is Go's nil reader (an empty slice).
// Go: helpers/general.go:ReaderToBytes
pub fn reader_to_bytes(lines: Option<&mut dyn Read>) -> Vec<u8> {
    let Some(r) = lines else {
        return Vec::new();
    };
    let mut b = Vec::new();
    // b.ReadFrom(lines) // nolint — the error is ignored, the bytes read so far are kept.
    let _ = read_from(r, &mut b);
    b
}

/// Go: `helpers.ReaderToString(lines)`.
// Go: helpers/general.go:ReaderToString
pub fn reader_to_string(lines: Option<&mut dyn Read>) -> Vec<u8> {
    reader_to_bytes(lines)
}

/// `bytes.Buffer.ReadFrom`: reads until EOF or the first error, keeping what was read.
fn read_from(r: &mut dyn Read, b: &mut Vec<u8>) -> std::io::Result<()> {
    let mut buf = [0u8; 512];
    loop {
        match r.read(&mut buf) {
            Ok(0) => return Ok(()),
            Ok(n) => b.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
}

/// Go `io.ReadAtLeast(r, buf, min)`: `(n, err)` with Go's `ErrUnexpectedEOF`/`EOF` distinction
/// (`Err(true)` = EOF with nothing read, `Err(false)` = any other error).
fn read_at_least(r: &mut dyn Read, buf: &mut [u8], min: usize) -> (usize, Option<bool>) {
    let mut n = 0usize;
    while n < min {
        match r.read(&mut buf[n..]) {
            Ok(0) => {
                // io.EOF: n==0 -> EOF, else ErrUnexpectedEOF.
                return (n, Some(n == 0));
            }
            Ok(k) => n += k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return (n, Some(false)),
        }
    }
    (n, None)
}

/// Go: `helpers.ReaderContains(r, subslice)` — `None` is Go's nil reader. Go's windowing is kept:
/// the whole buffer (stale half included) is searched after each read.
// Go: helpers/general.go:ReaderContains
pub fn reader_contains(r: Option<&mut dyn Read>, subslice: &[u8]) -> bool {
    let Some(r) = r else {
        return false;
    };
    if subslice.is_empty() {
        return false;
    }

    let bufflen = subslice.len() * 4;
    let halflen = bufflen / 2;
    let mut buff = vec![0u8; bufflen];
    let mut i = 0;

    loop {
        i += 1;
        let (n, err) = if i == 1 {
            read_at_least(r, &mut buff[..halflen], halflen)
        } else {
            if i != 2 {
                // shift left to catch overlapping matches
                buff.copy_within(halflen.., 0);
            }
            read_at_least(r, &mut buff[halflen..], halflen)
        };

        if n > 0 && go_unicode::bytes::contains(&buff, subslice) {
            return true;
        }

        if err.is_some() {
            break;
        }
    }
    false
}

/// A title function (Go `func(s string) string`).
pub type TitleFunc = Arc<dyn Fn(&str) -> String + Send + Sync>;

/// A title func over Go string bytes (invalid UTF-8 included).
pub type TitleBytesFunc = Arc<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>;

/// [`get_title_func`] over Go string bytes: the same styles, with Go's handling of invalid
/// UTF-8 (`strings.Title` maps an invalid byte to `U+FFFD`; the prose converters and
/// `FirstUpper` copy the bytes they do not change).
// Go: helpers/general.go:GetTitleFunc
pub fn get_title_bytes_func(style: &str) -> TitleBytesFunc {
    match go_unicode::strings::to_lower_str(style).as_ref() {
        "go" => Arc::new(|s: &[u8]| go_unicode::strings::title(s).into_owned()),
        "chicago" => {
            let tc = TitleConverter::new(TitleStyle::Chicago);
            Arc::new(move |s: &[u8]| tc.title_bytes(s))
        }
        "none" => Arc::new(|s: &[u8]| s.to_vec()),
        "firstupper" => Arc::new(first_upper_bytes),
        _ => {
            let tc = TitleConverter::new(TitleStyle::Ap);
            Arc::new(move |s: &[u8]| tc.title_bytes(s))
        }
    }
}

/// Go: `helpers.GetTitleFunc(style)` — "ap" (default), "chicago", "go", "firstupper", "none".
// Go: helpers/general.go:GetTitleFunc
pub fn get_title_func(style: &str) -> TitleFunc {
    match go_unicode::strings::to_lower_str(style).as_ref() {
        "go" => Arc::new(|s: &str| go_unicode::strings::title_str(s).into_owned()),
        "chicago" => {
            let tc = TitleConverter::new(TitleStyle::Chicago);
            Arc::new(move |s: &str| tc.title(s))
        }
        "none" => Arc::new(|s: &str| s.to_string()),
        "firstupper" => Arc::new(first_upper),
        _ => {
            let tc = TitleConverter::new(TitleStyle::Ap);
            Arc::new(move |s: &str| tc.title(s))
        }
    }
}

/// Go: `helpers.HasStringsPrefix(s, prefix)`.
// Go: helpers/general.go:HasStringsPrefix
pub fn has_strings_prefix(s: &[String], prefix: &[String]) -> bool {
    s.len() >= prefix.len() && compare_string_slices(Some(&s[..prefix.len()]), Some(prefix))
}

/// Go: `helpers.HasStringsSuffix(s, suffix)`.
// Go: helpers/general.go:HasStringsSuffix
pub fn has_strings_suffix(s: &[String], suffix: &[String]) -> bool {
    s.len() >= suffix.len()
        && compare_string_slices(Some(&s[s.len() - suffix.len()..]), Some(suffix))
}

/// Go: `compareStringSlices(a, b)` (`None` = nil slice).
// Go: helpers/general.go:compareStringSlices
fn compare_string_slices(a: Option<&[String]>, b: Option<&[String]>) -> bool {
    let (a, b) = match (a, b) {
        (None, None) => return true,
        (None, _) | (_, None) => return false,
        (Some(a), Some(b)) => (a, b),
    };
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).all(|(x, y)| x == y)
}

/// Go: `helpers.SliceToLower` (`None` = nil in and out).
// Go: helpers/general.go:SliceToLower
pub fn slice_to_lower(s: Option<&[String]>) -> Option<Vec<String>> {
    let s = s?;
    Some(
        s.iter()
            .map(|v| go_unicode::strings::to_lower_str(v).into_owned())
            .collect(),
    )
}

/// Go: `helpers.StringSliceToList(s, conjunction)`.
// Go: helpers/general.go:StringSliceToList
pub fn string_slice_to_list(s: &[String], c: &str) -> String {
    const DEFAULT_CONJUNCTION: &str = "and";
    let c = if c.is_empty() { DEFAULT_CONJUNCTION } else { c };
    match s.len() {
        0 => String::new(),
        1 => s[0].clone(),
        2 => format!("{} {} {}", s[0], c, s[1]),
        n => format!("{}, {} {}", s[..n - 1].join(", "), c, s[n - 1]),
    }
}

/// Go: `helpers.IsWhitespace` (space, tab, newline, carriage return only).
// Go: helpers/general.go:IsWhitespace
pub fn is_whitespace(r: char) -> bool {
    matches!(r, ' ' | '\t' | '\n' | '\r')
}

/// Go: `helpers.PrintFs` — a debugging helper; not supported.
// Go: helpers/general.go:PrintFs
pub fn print_fs() -> Result<()> {
    Err(Error::new("neohugo-rs: helpers.PrintFs is not supported"))
}

/// Go: `helpers.FormatByteCount(bc)`. Go negates the `uint64` (`-bc` wraps), so every count
/// except 0 and the top 1 GiB of the range prints in GB; reproduced on purpose.
// Go: helpers/general.go:FormatByteCount
pub fn format_byte_count(bc: u64) -> String {
    const GIGABYTE: u64 = 1 << 30;
    const MEGABYTE: u64 = 1 << 20;
    const KILOBYTE: u64 = 1 << 10;
    let neg = bc.wrapping_neg();
    let f = |d: u64, unit: &str| {
        format!(
            "{} {unit}",
            go_strconv::format_float(bc as f64 / d as f64, b'f', 2, 64)
        )
    };
    if bc > GIGABYTE || neg > GIGABYTE {
        return f(GIGABYTE, "GB");
    }
    if bc > MEGABYTE || neg > MEGABYTE {
        return f(MEGABYTE, "MB");
    }
    if bc > KILOBYTE || neg > KILOBYTE {
        return f(KILOBYTE, "KB");
    }
    format!("{bc} B")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/general.go (321 lines; 6/17 funcs executed)
// OK L39-50: TCPListen() (net.Listener, *net.TCPAddr, error) — STUB (server)
// OK L53-59: FirstUpper(s string) string
// OK L62-77: UniqueStrings(s []string) []string
// OK L81-98: UniqueStringsReuse(s []string) []string
// OK L102-118: UniqueStringsSorted(s []string) []string
// OK L122-134: ReaderToBytes(lines io.Reader) []byte
// OK L137-145: ReaderToString(lines io.Reader) string
// OK L148-180: ReaderContains(r io.Reader, subslice []byte) bool
// OK L194-210: GetTitleFunc(style string) func(s string) string
// OK L213-215: HasStringsPrefix(s, prefix []string) bool
// OK L218-220: HasStringsSuffix(s, suffix []string) bool
// OK L222-242: compareStringSlices(a, b []string) bool
// OK L245-256: SliceToLower(s []string) []string
// OK L261-277: StringSliceToList(s []string, c string) string
// OK L280-282: IsWhitespace(r rune) bool
// OK L286-303: PrintFs(fs afero.Fs, path string, w io.Writer) — STUB (debugging)
// OK L306-321: FormatByteCount(bc uint64) string
// ---------------------------------------------------------------------------
