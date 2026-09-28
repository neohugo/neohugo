//! NEW (a copy of nh-resources' private `mime` module, T14, plus `ExtensionsByType`): the
//! parts of Go's `mime` package (go1.27.1) that `create/remote.go` reaches:
//! `mime.ParseMediaType` (the `Content-Disposition` file name) and `mime.ExtensionsByType`
//! (the extension hints of a response whose content type is not accepted), with the built-in
//! table, the unix system databases (`type_unix.go`) and `FormatMediaType` (`mediatype.go`,
//! `grammar.go`). nh-resources keeps `TypeByExtension`; I04 can merge the two copies.
//!
//! Like Go, the result depends on the machine: the FreeDesktop `globs2` database, or else the
//! `mime.types` files, are read once (`sync.Once`) from the OS file system. Go strings are bytes:
//! everything here works on `&[u8]`.

use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use go_unicode::strings;

/// Go: `mimeTypes`, `mimeTypesLower`, `extensions`.
struct Tables {
    mime_types: HashMap<Vec<u8>, Vec<u8>>,
    mime_types_lower: HashMap<Vec<u8>, Vec<u8>>,
    /// Go `extensions`: media type (without parameters) -> lower-case extensions, in insertion
    /// order (Go ranges over the built-in map: the order is random, `ExtensionsByType` sorts).
    extensions: HashMap<Vec<u8>, Vec<Vec<u8>>>,
}

/// Go: `builtinTypesLower` (go1.27.1 `mime/type.go`).
const BUILTIN_TYPES_LOWER: &[(&str, &str)] = &[
    (".ai", "application/postscript"),
    (".apk", "application/vnd.android.package-archive"),
    (".apng", "image/apng"),
    (".avif", "image/avif"),
    (".bin", "application/octet-stream"),
    (".bmp", "image/bmp"),
    (".com", "application/octet-stream"),
    (".css", "text/css; charset=utf-8"),
    (".csv", "text/csv; charset=utf-8"),
    (".doc", "application/msword"),
    (
        ".docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ),
    (".ehtml", "text/html; charset=utf-8"),
    (".eml", "message/rfc822"),
    (".eps", "application/postscript"),
    (".exe", "application/octet-stream"),
    (".flac", "audio/flac"),
    (".gif", "image/gif"),
    (".gz", "application/gzip"),
    (".htm", "text/html; charset=utf-8"),
    (".html", "text/html; charset=utf-8"),
    (".ico", "image/vnd.microsoft.icon"),
    (".ics", "text/calendar; charset=utf-8"),
    (".jfif", "image/jpeg"),
    (".jpeg", "image/jpeg"),
    (".jpg", "image/jpeg"),
    (".js", "text/javascript; charset=utf-8"),
    (".json", "application/json"),
    (".m4a", "audio/mp4"),
    (".mjs", "text/javascript; charset=utf-8"),
    (".mp3", "audio/mpeg"),
    (".mp4", "video/mp4"),
    (".oga", "audio/ogg"),
    (".ogg", "audio/ogg"),
    (".ogv", "video/ogg"),
    (".opus", "audio/ogg"),
    (".pdf", "application/pdf"),
    (".pjp", "image/jpeg"),
    (".pjpeg", "image/jpeg"),
    (".png", "image/png"),
    (".ppt", "application/vnd.ms-powerpoint"),
    (
        ".pptx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    ),
    (".ps", "application/postscript"),
    (".rdf", "application/rdf+xml"),
    (".rtf", "application/rtf"),
    (".shtml", "text/html; charset=utf-8"),
    (".svg", "image/svg+xml"),
    (".text", "text/plain; charset=utf-8"),
    (".tif", "image/tiff"),
    (".tiff", "image/tiff"),
    (".txt", "text/plain; charset=utf-8"),
    (".vtt", "text/vtt; charset=utf-8"),
    (".wasm", "application/wasm"),
    (".wav", "audio/wav"),
    (".weba", "audio/webm"),
    (".webm", "video/webm"),
    (".webp", "image/webp"),
    (".xbl", "text/xml; charset=utf-8"),
    (".xbm", "image/x-xbitmap"),
    (".xht", "application/xhtml+xml"),
    (".xhtml", "application/xhtml+xml"),
    (".xls", "application/vnd.ms-excel"),
    (
        ".xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    ),
    (".xml", "text/xml; charset=utf-8"),
    (".xsl", "text/xml; charset=utf-8"),
    (".zip", "application/zip"),
];

/// Go: `mimeGlobs` (type_unix.go).
const MIME_GLOBS: &[&str] = &["/usr/local/share/mime/globs2", "/usr/share/mime/globs2"];

/// Go: `typeFiles` (type_unix.go).
const TYPE_FILES: &[&str] = &[
    "/etc/mime.types",
    "/etc/apache2/mime.types",
    "/etc/apache/mime.types",
    "/etc/httpd/conf/mime.types",
];

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(init_mime)
}

// Go: mime/type.go:initMime
fn init_mime() -> Tables {
    let mut t = Tables {
        mime_types: HashMap::new(),
        mime_types_lower: HashMap::new(),
        extensions: HashMap::new(),
    };
    set_mime_types(&mut t);
    init_mime_unix(&mut t);
    t
}

// Go: mime/type.go:setMimeTypes (with builtinTypesLower for both maps)
fn set_mime_types(t: &mut Tables) {
    for (k, v) in BUILTIN_TYPES_LOWER {
        t.mime_types_lower
            .insert(k.as_bytes().to_vec(), v.as_bytes().to_vec());
        t.mime_types
            .insert(k.as_bytes().to_vec(), v.as_bytes().to_vec());
    }

    for (k, v) in BUILTIN_TYPES_LOWER {
        let (just_type, _) = parse_media_type(v.as_bytes()).unwrap_or_else(|e| panic!("{e}"));
        t.extensions
            .entry(just_type)
            .or_default()
            .push(k.as_bytes().to_vec());
    }
}

/// ExtensionsByType returns the extensions known to be associated with the MIME type typ. The
/// returned extensions will each begin with a leading dot, as in ".html". When typ has no
/// associated extensions, ExtensionsByType returns `Ok(None)` (Go's nil slice).
// Go: mime/type.go:ExtensionsByType
pub(crate) fn extensions_by_type(typ: &[u8]) -> Result<Option<Vec<Vec<u8>>>, String> {
    let (just_type, _) = parse_media_type(typ)?;

    let Some(s) = tables().extensions.get(&just_type) else {
        return Ok(None);
    };
    let mut ret = s.clone();
    ret.sort();
    Ok(Some(ret))
}

// Go: mime/type_unix.go:initMimeUnix
fn init_mime_unix(t: &mut Tables) {
    for filename in MIME_GLOBS {
        if load_mime_globs_file(t, filename) {
            return; // Stop checking more files if mimetype database is found.
        }
    }

    // Fallback if no system-generated mimetype database exists.
    for filename in TYPE_FILES {
        load_mime_file(t, filename);
    }
}

/// Go's `bufio.Scanner` with `ScanLines`: lines without the `\n` and a trailing `\r`.
fn scan_lines(b: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = b;
    while !rest.is_empty() {
        let (line, next) = match rest.iter().position(|&c| c == b'\n') {
            Some(i) => (&rest[..i], &rest[i + 1..]),
            None => (rest, &rest[rest.len()..]),
        };
        out.push(line.strip_suffix(b"\r").unwrap_or(line));
        rest = next;
    }
    out
}

// Go: mime/type_unix.go:loadMimeGlobsFile (true = Go's nil error)
fn load_mime_globs_file(t: &mut Tables, filename: &str) -> bool {
    let Ok(b) = std::fs::read(filename) else {
        return false;
    };

    for line in scan_lines(&b) {
        // Each line should be of format: weight:mimetype:glob[:morefields...]
        let fields: Vec<&[u8]> = line.split(|&c| c == b':').collect();
        if fields.len() < 3
            || fields[0].is_empty()
            || fields[2].len() < 3
            || fields[0][0] == b'#'
            || fields[2][0] != b'*'
            || fields[2][1] != b'.'
        {
            continue;
        }

        let extension = &fields[2][1..];
        if extension.iter().any(|c| matches!(c, b'?' | b'*' | b'[')) {
            // Not a bare extension, but a glob. Ignore for now.
            continue;
        }
        if t.mime_types.contains_key(extension) {
            // We've already seen this extension.
            // The file is in weight order, so we keep
            // the first entry that we see.
            continue;
        }

        let _ = set_extension_type(t, extension, fields[1]);
    }
    true
}

// Go: mime/type_unix.go:loadMimeFile
fn load_mime_file(t: &mut Tables, filename: &str) {
    let Ok(b) = std::fs::read(filename) else {
        return;
    };

    for line in scan_lines(&b) {
        let fields = strings::fields(line);
        if fields.len() <= 1 || fields[0][0] == b'#' {
            continue;
        }
        let mime_type = fields[0];
        for ext in &fields[1..] {
            if ext[0] == b'#' {
                break;
            }
            let mut e = b".".to_vec();
            e.extend_from_slice(ext);
            let _ = set_extension_type(t, &e, mime_type);
        }
    }
}

// Go: mime/type.go:setExtensionType
fn set_extension_type(t: &mut Tables, extension: &[u8], mime_type: &[u8]) -> Result<(), String> {
    let (just_type, mut param) = parse_media_type(mime_type)?;
    let mut mime_type = mime_type.to_vec();
    if mime_type.starts_with(b"text/")
        && param
            .get(b"charset".as_slice())
            .is_none_or(|v| v.is_empty())
    {
        param.insert(b"charset".to_vec(), b"utf-8".to_vec());
        mime_type = format_media_type(&just_type, &param);
    }
    let ext_lower = strings::to_lower(extension).into_owned();

    t.mime_types.insert(extension.to_vec(), mime_type.clone());
    t.mime_types_lower.insert(ext_lower.clone(), mime_type);

    let exts = t.extensions.entry(just_type).or_default();
    if exts.contains(&ext_lower) {
        return Ok(());
    }
    exts.push(ext_lower);
    Ok(())
}

/// TypeByExtension returns the MIME type associated with the file extension ext. The extension
/// ext should begin with a leading dot, as in ".html". When ext has no associated type,
/// TypeByExtension returns "".
/// (Kept with the tables it reads; remote.go does not call it.)
// Go: mime/type.go:TypeByExtension
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn type_by_extension(ext: &[u8]) -> Vec<u8> {
    let t = tables();

    // Case-sensitive lookup.
    if let Some(v) = t.mime_types.get(ext) {
        return v.clone();
    }

    // Case-insensitive lookup.
    let mut lower = Vec::with_capacity(ext.len());
    for &c in ext {
        if c >= 0x80 {
            // Slow path.
            return t
                .mime_types_lower
                .get(strings::to_lower(ext).as_ref())
                .cloned()
                .unwrap_or_default();
        }
        lower.push(c.to_ascii_lowercase());
    }
    t.mime_types_lower.get(&lower).cloned().unwrap_or_default()
}

// Go: mime/grammar.go:isTSpecial
fn is_tspecial(c: u8) -> bool {
    matches!(
        c,
        b'(' | b')'
            | b'<'
            | b'>'
            | b'@'
            | b','
            | b';'
            | b':'
            | b'\\'
            | b'"'
            | b'/'
            | b'['
            | b']'
            | b'?'
            | b'='
    )
}

// Go: mime/grammar.go:isTokenChar
fn is_token_char(c: u8) -> bool {
    c.is_ascii_alphanumeric()
        || matches!(
            c,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
        )
}

// Go: mime/grammar.go:isToken
fn is_token(s: &[u8]) -> bool {
    !s.is_empty() && s.iter().all(|&c| is_token_char(c))
}

// Go: mime/encodedword.go:needsEncoding (ranging over runes: any byte >= 0x80 is a rune > '~')
fn needs_encoding(s: &[u8]) -> bool {
    s.iter().any(|&b| !(b' '..=b'~').contains(&b) && b != b'\t')
}

const UPPERHEX: &[u8; 16] = b"0123456789ABCDEF";

/// FormatMediaType serializes mediatype t and the parameters param as a media type conforming
/// to RFC 2045 and RFC 2616 ("" when t or an attribute is not a token).
// Go: mime/mediatype.go:FormatMediaType
fn format_media_type(t: &[u8], param: &BTreeMap<Vec<u8>, Vec<u8>>) -> Vec<u8> {
    let mut b = Vec::new();
    match t.iter().position(|&c| c == b'/') {
        None => {
            if !is_token(t) {
                return Vec::new();
            }
            b.extend_from_slice(&strings::to_lower(t));
        }
        Some(i) => {
            let (major, sub) = (&t[..i], &t[i + 1..]);
            if !is_token(major) || !is_token(sub) {
                return Vec::new();
            }
            b.extend_from_slice(&strings::to_lower(major));
            b.push(b'/');
            b.extend_from_slice(&strings::to_lower(sub));
        }
    }

    // BTreeMap iterates in sorted (byte) order, as slices.Sorted(maps.Keys(param)).
    for (attribute, value) in param {
        b.push(b';');
        b.push(b' ');
        if !is_token(attribute) {
            return Vec::new();
        }
        b.extend_from_slice(&strings::to_lower(attribute));

        let need_enc = needs_encoding(value);
        if need_enc {
            b.push(b'*');
        }
        b.push(b'=');

        if need_enc {
            b.extend_from_slice(b"utf-8''");

            let mut offset = 0;
            for (index, &ch) in value.iter().enumerate() {
                if ch <= b' '
                    || ch >= 0x7F
                    || ch == b'*'
                    || ch == b'\''
                    || ch == b'%'
                    || is_tspecial(ch)
                {
                    b.extend_from_slice(&value[offset..index]);
                    offset = index + 1;

                    b.push(b'%');
                    b.push(UPPERHEX[(ch >> 4) as usize]);
                    b.push(UPPERHEX[(ch & 0x0F) as usize]);
                }
            }
            b.extend_from_slice(&value[offset..]);
            continue;
        }

        if is_token(value) {
            b.extend_from_slice(value);
            continue;
        }

        b.push(b'"');
        let mut offset = 0;
        for (index, &character) in value.iter().enumerate() {
            if character == b'"' || character == b'\\' {
                b.extend_from_slice(&value[offset..index]);
                offset = index;
                b.push(b'\\');
            }
        }
        b.extend_from_slice(&value[offset..]);
        b.push(b'"');
    }
    b
}

// Go: mime/mediatype.go:checkMediaTypeDisposition
fn check_media_type_disposition(s: &[u8]) -> Result<(), String> {
    let (typ, rest) = consume_token(s);
    if typ.is_empty() {
        return Err("mime: no media type".into());
    }
    if rest.is_empty() {
        return Ok(());
    }
    let Some(rest) = rest.strip_prefix(b"/") else {
        return Err("mime: expected slash after first token".into());
    };
    let (subtype, rest) = consume_token(rest);
    if subtype.is_empty() {
        return Err("mime: expected token after slash".into());
    }
    if !rest.is_empty() {
        return Err("mime: unexpected content after media subtype".into());
    }
    Ok(())
}

fn trim_left_space(s: &[u8]) -> &[u8] {
    strings::trim_left_func(s, go_unicode::is_space)
}

type Params = BTreeMap<Vec<u8>, Vec<u8>>;

/// ParseMediaType parses a media type value and any optional parameters, per RFC 1521.
// Go: mime/mediatype.go:ParseMediaType
pub(crate) fn parse_media_type(v: &[u8]) -> Result<(Vec<u8>, Params), String> {
    let base = match v.iter().position(|&c| c == b';') {
        Some(i) => &v[..i],
        None => v,
    };
    let mediatype = strings::trim_space(&strings::to_lower(base)).to_vec();

    check_media_type_disposition(&mediatype)?;

    let mut params: Params = BTreeMap::new();

    // Map of base parameter name -> parameter name -> value for parameters containing a '*'
    // character.
    let mut continuation: Option<BTreeMap<Vec<u8>, Params>> = None;

    let mut v = &v[base.len()..];
    while !v.is_empty() {
        v = trim_left_space(v);
        if v.is_empty() {
            break;
        }
        let (key, value, rest) = consume_media_param(v);
        if key.is_empty() {
            if strings::trim_space(rest) == b";" {
                // Ignore trailing semicolons.
                // Not an error.
                break;
            }
            // Parse error.
            return Err("mime: invalid media parameter".into());
        }

        let pmap: &mut Params = match key.iter().position(|&c| c == b'*') {
            Some(i) => {
                let base_name = key[..i].to_vec();
                continuation
                    .get_or_insert_with(BTreeMap::new)
                    .entry(base_name)
                    .or_default()
            }
            None => &mut params,
        };
        if let Some(existing) = pmap.get(&key)
            && *existing != value
        {
            // Duplicate parameter names are incorrect, but we allow them if they are equal.
            return Err("mime: duplicate parameter name".into());
        }
        pmap.insert(key, value);
        v = rest;
    }

    // Stitch together any continuations or things with stars (i.e. RFC 2231 things with
    // stars: "foo*0" or "foo*"). (Go iterates a map here; each key is independent.)
    if let Some(continuation) = continuation {
        for (key, piece_map) in continuation {
            let mut single_part_key = key.clone();
            single_part_key.push(b'*');
            if let Some(v) = piece_map.get(&single_part_key) {
                if let Some(decv) = decode_2231_enc(v) {
                    params.insert(key.clone(), decv);
                }
                continue;
            }

            let mut buf = Vec::new();
            let mut valid = false;
            let mut n = 0;
            loop {
                let mut simple_part = key.clone();
                simple_part.push(b'*');
                simple_part.extend_from_slice(n.to_string().as_bytes());
                if let Some(v) = piece_map.get(&simple_part) {
                    valid = true;
                    buf.extend_from_slice(v);
                    n += 1;
                    continue;
                }
                let mut encoded_part = simple_part;
                encoded_part.push(b'*');
                let Some(v) = piece_map.get(&encoded_part) else {
                    break;
                };
                valid = true;
                if n == 0 {
                    if let Some(decv) = decode_2231_enc(v) {
                        buf.extend_from_slice(&decv);
                    }
                } else if let Some(decv) = percent_hex_unescape(v) {
                    buf.extend_from_slice(&decv);
                }
                n += 1;
            }
            if valid {
                params.insert(key, buf);
            }
        }
    }

    Ok((mediatype, params))
}

// Go: mime/mediatype.go:decode2231Enc
fn decode_2231_enc(v: &[u8]) -> Option<Vec<u8>> {
    let i = v.iter().position(|&c| c == b'\'')?;
    let (charset, v) = (&v[..i], &v[i + 1..]);
    let j = v.iter().position(|&c| c == b'\'')?;
    let ext_other_vals = &v[j + 1..];
    let charset = strings::to_lower(charset);
    match charset.as_ref() {
        b"us-ascii" | b"utf-8" => {}
        _ => return None,
    }
    percent_hex_unescape(ext_other_vals)
}

// Go: mime/mediatype.go:consumeToken
fn consume_token(v: &[u8]) -> (&[u8], &[u8]) {
    for (i, &c) in v.iter().enumerate() {
        if !is_token_char(c) {
            return (&v[..i], &v[i..]);
        }
    }
    (v, &v[v.len()..])
}

// Go: mime/mediatype.go:consumeValue
fn consume_value(v: &[u8]) -> (Vec<u8>, &[u8]) {
    if v.is_empty() {
        return (Vec::new(), v);
    }
    if v[0] != b'"' {
        let (t, r) = consume_token(v);
        return (t.to_vec(), r);
    }

    // parse a quoted-string
    let mut buffer = Vec::new();
    let mut i = 1;
    while i < v.len() {
        let r = v[i];
        if r == b'"' {
            return (buffer, &v[i + 1..]);
        }
        // When MSIE sends a full file path (in "intranet mode"), it does not escape backslashes.
        if r == b'\\' && i + 1 < v.len() && is_tspecial(v[i + 1]) {
            buffer.push(v[i + 1]);
            i += 2;
            continue;
        }
        if r == b'\r' || r == b'\n' {
            return (Vec::new(), v);
        }
        buffer.push(v[i]);
        i += 1;
    }
    // Did not find end quote.
    (Vec::new(), v)
}

// Go: mime/mediatype.go:consumeMediaParam
fn consume_media_param(v: &[u8]) -> (Vec<u8>, Vec<u8>, &[u8]) {
    let rest = trim_left_space(v);
    let Some(rest) = rest.strip_prefix(b";") else {
        return (Vec::new(), Vec::new(), v);
    };

    let rest = trim_left_space(rest);
    let (param, rest) = consume_token(rest);
    let param = strings::to_lower(param).into_owned();
    if param.is_empty() {
        return (Vec::new(), Vec::new(), v);
    }

    let rest = trim_left_space(rest);
    let Some(rest) = rest.strip_prefix(b"=") else {
        return (Vec::new(), Vec::new(), v);
    };
    let rest = trim_left_space(rest);
    let (value, rest2) = consume_value(rest);
    if value.is_empty() && rest2.len() == rest.len() {
        return (Vec::new(), Vec::new(), v);
    }
    (param, value, rest2)
}

// Go: mime/mediatype.go:percentHexUnescape
fn percent_hex_unescape(s: &[u8]) -> Option<Vec<u8>> {
    // Count %, check that they're well-formed.
    let mut percents = 0;
    let mut i = 0;
    while i < s.len() {
        if s[i] != b'%' {
            i += 1;
            continue;
        }
        percents += 1;
        if i + 2 >= s.len() || !s[i + 1].is_ascii_hexdigit() || !s[i + 2].is_ascii_hexdigit() {
            return None;
        }
        i += 3;
    }
    if percents == 0 {
        return Some(s.to_vec());
    }

    let mut t = Vec::with_capacity(s.len() - 2 * percents);
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'%' {
            t.push(unhex(s[i + 1]) << 4 | unhex(s[i + 2]));
            i += 3;
        } else {
            t.push(s[i]);
            i += 1;
        }
    }
    Some(t)
}

// Go: mime/mediatype.go:unhex
fn unhex(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_and_case() {
        assert_eq!(type_by_extension(b".pdf"), b"application/pdf");
        assert_eq!(type_by_extension(b".PDF"), b"application/pdf");
        assert_eq!(type_by_extension(b".nosuchext-xyz"), b"");
    }

    #[test]
    fn parse_and_format() {
        let (t, p) = parse_media_type(b"text/html; charset=\"utf-8\"").unwrap();
        assert_eq!(t, b"text/html");
        assert_eq!(p.get(b"charset".as_slice()).unwrap(), b"utf-8");
        assert_eq!(format_media_type(&t, &p), b"text/html; charset=utf-8");
        assert!(parse_media_type(b"text/").is_err());
    }

    #[test]
    fn extensions() {
        // Built-in entries (a system database may add more; the result is sorted).
        let json = extensions_by_type(b"application/json; charset=UTF-8")
            .unwrap()
            .unwrap();
        assert!(json.contains(&b".json".to_vec()));
        let mut sorted = json.clone();
        sorted.sort();
        assert_eq!(json, sorted);
        assert_eq!(extensions_by_type(b"x-no/such-type").unwrap(), None);
        assert!(extensions_by_type(b"").is_err());
    }
}
