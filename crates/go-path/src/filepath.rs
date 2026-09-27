//! Port of Go's `path/filepath` for unix (go1.27.1: `src/path/filepath/path.go`,
//! `path_unix.go`, `match.go` and `src/internal/filepathlite/{path.go,path_unix.go,
//! path_nonwindows.go}`), `Separator = '/'`, `ListSeparator = ':'`.
//!
//! Filesystem-touching functions (`Abs`, `EvalSymlinks`, `Walk`, `WalkDir`,
//! `Glob`) are not ported.

use crate::BadPattern;
use crate::utf8;

/// OS-specific path separator (unix).
pub const SEPARATOR: u8 = b'/';
/// OS-specific path list separator (unix).
pub const LIST_SEPARATOR: u8 = b':';

// Go: internal/filepathlite/path_unix.go:IsPathSeparator
pub fn is_path_separator(c: u8) -> bool {
    SEPARATOR == c
}

// Go: internal/filepathlite/path_unix.go:volumeNameLen
fn volume_name_len(_path: &[u8]) -> usize {
    0
}

// Go: internal/filepathlite/path.go:lazybuf
struct LazyBuf<'a> {
    path: &'a [u8],
    buf: Option<Vec<u8>>,
    w: usize,
    vol_and_path: &'a [u8],
    vol_len: usize,
}

impl LazyBuf<'_> {
    // Go: internal/filepathlite/path.go:lazybuf.index
    fn index(&self, i: usize) -> u8 {
        match &self.buf {
            Some(buf) => buf[i],
            None => self.path[i],
        }
    }

    // Go: internal/filepathlite/path.go:lazybuf.append
    fn append(&mut self, c: u8) {
        if self.buf.is_none() {
            if self.w < self.path.len() && self.path[self.w] == c {
                self.w += 1;
                return;
            }
            let mut buf = vec![0u8; self.path.len()];
            buf[..self.w].copy_from_slice(&self.path[..self.w]);
            self.buf = Some(buf);
        }
        let w = self.w;
        self.buf.as_mut().expect("buf allocated above")[w] = c;
        self.w += 1;
    }

    // Go: internal/filepathlite/path.go:lazybuf.string
    fn string(&self) -> Vec<u8> {
        match &self.buf {
            None => self.vol_and_path[..self.vol_len + self.w].to_vec(),
            Some(buf) => {
                let mut v = self.vol_and_path[..self.vol_len].to_vec();
                v.extend_from_slice(&buf[..self.w]);
                v
            }
        }
    }
}

// Go: internal/filepathlite/path_nonwindows.go:postClean
fn post_clean(_out: &mut LazyBuf<'_>) {}

// Go: internal/filepathlite/path.go:Clean (via path/filepath/path.go:Clean)
/// Returns the shortest path name equivalent to `path` by purely lexical
/// processing (Go `filepath.Clean`).
pub fn clean_bytes(path: &[u8]) -> Vec<u8> {
    let original_path = path;
    let vol_len = volume_name_len(path);
    let path = &path[vol_len..];
    if path.is_empty() {
        if vol_len > 1 && is_path_separator(original_path[0]) && is_path_separator(original_path[1])
        {
            // should be UNC
            return from_slash_bytes(original_path);
        }
        let mut v = original_path.to_vec();
        v.push(b'.');
        return v;
    }
    let rooted = is_path_separator(path[0]);

    // Invariants:
    //	reading from path; r is index of next byte to process.
    //	writing to buf; w is index of next byte to write.
    //	dotdot is index in buf where .. must stop, either because
    //		it is the leading slash or it is a leading ../../.. prefix.
    let n = path.len();
    let mut out = LazyBuf {
        path,
        buf: None,
        w: 0,
        vol_and_path: original_path,
        vol_len,
    };
    let (mut r, mut dotdot) = (0usize, 0usize);
    if rooted {
        out.append(SEPARATOR);
        r = 1;
        dotdot = 1;
    }

    while r < n {
        if is_path_separator(path[r]) {
            // empty path element
            r += 1;
        } else if path[r] == b'.' && (r + 1 == n || is_path_separator(path[r + 1])) {
            // . element
            r += 1;
        } else if path[r] == b'.'
            && path[r + 1] == b'.'
            && (r + 2 == n || is_path_separator(path[r + 2]))
        {
            // .. element: remove to last separator
            r += 2;
            if out.w > dotdot {
                // can backtrack
                out.w -= 1;
                while out.w > dotdot && !is_path_separator(out.index(out.w)) {
                    out.w -= 1;
                }
            } else if !rooted {
                // cannot backtrack, but not rooted, so append .. element.
                if out.w > 0 {
                    out.append(SEPARATOR);
                }
                out.append(b'.');
                out.append(b'.');
                dotdot = out.w;
            }
        } else {
            // real path element.
            // add slash if needed
            if rooted && out.w != 1 || !rooted && out.w != 0 {
                out.append(SEPARATOR);
            }
            // copy element
            while r < n && !is_path_separator(path[r]) {
                out.append(path[r]);
                r += 1;
            }
        }
    }

    // Turn empty string into "."
    if out.w == 0 {
        out.append(b'.');
    }

    post_clean(&mut out); // avoid creating absolute paths on Windows
    from_slash_bytes(&out.string())
}

/// `&str` form of [`clean_bytes`].
pub fn clean(path: &str) -> String {
    String::from_utf8(clean_bytes(path.as_bytes())).expect("Clean splits only at ASCII bytes")
}

// Go: internal/filepathlite/path.go:unixIsLocal (via IsLocal/isLocal)
/// Reports whether `path` is local (Go `filepath.IsLocal`, unix).
pub fn is_local_bytes(path: &[u8]) -> bool {
    if is_abs_bytes(path) || path.is_empty() {
        return false;
    }
    let mut has_dots = false;
    let mut p = path;
    while !p.is_empty() {
        let (part, rest) = match p.iter().position(|&b| b == b'/') {
            Some(i) => (&p[..i], &p[i + 1..]),
            None => (p, &b""[..]),
        };
        p = rest;
        if part == b"." || part == b".." {
            has_dots = true;
            break;
        }
    }
    let cleaned;
    let mut path = path;
    if has_dots {
        cleaned = clean_bytes(path);
        path = &cleaned;
    }
    if path == b".." || path.starts_with(b"../") {
        return false;
    }
    true
}

/// `&str` form of [`is_local_bytes`].
pub fn is_local(path: &str) -> bool {
    is_local_bytes(path.as_bytes())
}

// Go: internal/filepathlite/path.go:ToSlash
/// Replaces each separator with a slash; the identity on unix.
pub fn to_slash_bytes(path: &[u8]) -> Vec<u8> {
    // Separator == '/'
    path.to_vec()
}

/// `&str` form of [`to_slash_bytes`].
pub fn to_slash(path: &str) -> &str {
    path
}

// Go: internal/filepathlite/path.go:FromSlash
/// Replaces each slash with a separator; the identity on unix.
pub fn from_slash_bytes(path: &[u8]) -> Vec<u8> {
    // Separator == '/'
    path.to_vec()
}

/// `&str` form of [`from_slash_bytes`].
pub fn from_slash(path: &str) -> &str {
    path
}

// Go: path/filepath/path_unix.go:splitList (via SplitList)
/// Splits a list of paths joined by [`LIST_SEPARATOR`]; an empty string
/// yields an empty list (Go `filepath.SplitList`).
pub fn split_list_bytes(path: &[u8]) -> Vec<&[u8]> {
    if path.is_empty() {
        return Vec::new();
    }
    path.split(|&b| b == LIST_SEPARATOR).collect()
}

/// `&str` form of [`split_list_bytes`].
pub fn split_list(path: &str) -> Vec<&str> {
    if path.is_empty() {
        return Vec::new();
    }
    path.split(LIST_SEPARATOR as char).collect()
}

// Go: internal/filepathlite/path.go:Split
/// Splits `path` immediately following the final separator
/// (Go `filepath.Split`).
pub fn split_bytes(path: &[u8]) -> (&[u8], &[u8]) {
    let vol = volume_name_len(path) as isize;
    let mut i = path.len() as isize - 1;
    while i >= vol && !is_path_separator(path[i as usize]) {
        i -= 1;
    }
    let at = (i + 1) as usize;
    (&path[..at], &path[at..])
}

/// `&str` form of [`split_bytes`].
pub fn split(path: &str) -> (&str, &str) {
    let (d, _) = split_bytes(path.as_bytes());
    path.split_at(d.len())
}

// Go: path/filepath/path_unix.go:join (via Join)
/// Joins path elements with the separator and cleans the result
/// (Go `filepath.Join`). Returns an empty string if all elements are empty.
pub fn join_bytes<S: AsRef<[u8]>>(elem: &[S]) -> Vec<u8> {
    // If there's a bug here, fix the logic in ./path_plan9.go too.
    for (i, e) in elem.iter().enumerate() {
        if !e.as_ref().is_empty() {
            let mut joined = Vec::new();
            for (k, e2) in elem[i..].iter().enumerate() {
                if k > 0 {
                    joined.push(SEPARATOR);
                }
                joined.extend_from_slice(e2.as_ref());
            }
            return clean_bytes(&joined);
        }
    }
    Vec::new()
}

/// `&str` form of [`join_bytes`].
pub fn join<S: AsRef<str>>(elem: &[S]) -> String {
    let v: Vec<&[u8]> = elem.iter().map(|e| e.as_ref().as_bytes()).collect();
    String::from_utf8(join_bytes(&v)).expect("Join splits only at ASCII bytes")
}

// Go: internal/filepathlite/path.go:Ext
/// Returns the file name extension (Go `filepath.Ext`).
pub fn ext_bytes(path: &[u8]) -> &[u8] {
    let mut i = path.len() as isize - 1;
    while i >= 0 && !is_path_separator(path[i as usize]) {
        if path[i as usize] == b'.' {
            return &path[i as usize..];
        }
        i -= 1;
    }
    b""
}

/// `&str` form of [`ext_bytes`].
pub fn ext(path: &str) -> &str {
    let e = ext_bytes(path.as_bytes());
    &path[path.len() - e.len()..]
}

// Go: internal/filepathlite/path_unix.go:IsAbs
/// Reports whether the path is absolute (Go `filepath.IsAbs`, unix).
pub fn is_abs_bytes(path: &[u8]) -> bool {
    path.starts_with(b"/")
}

/// `&str` form of [`is_abs_bytes`].
pub fn is_abs(path: &str) -> bool {
    is_abs_bytes(path.as_bytes())
}

// Go: path/filepath/path_unix.go:sameWord
fn same_word(a: &[u8], b: &[u8]) -> bool {
    a == b
}

/// Error returned by [`rel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelError {
    /// The Go error string: `Rel: can't make <targ> relative to <base>`.
    pub msg: Vec<u8>,
}

impl std::fmt::Display for RelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&String::from_utf8_lossy(&self.msg))
    }
}

impl std::error::Error for RelError {}

fn rel_error(targ_path: &[u8], base_path: &[u8]) -> RelError {
    let mut msg = b"Rel: can't make ".to_vec();
    msg.extend_from_slice(targ_path);
    msg.extend_from_slice(b" relative to ");
    msg.extend_from_slice(base_path);
    RelError { msg }
}

// Go: path/filepath/path.go:Rel
/// Returns a relative path that is lexically equivalent to `targ_path` when
/// joined to `base_path` (Go `filepath.Rel`).
pub fn rel_bytes(base_path: &[u8], targ_path: &[u8]) -> Result<Vec<u8>, RelError> {
    let base_vol = volume_name_bytes(base_path);
    let targ_vol = volume_name_bytes(targ_path);
    let base_c = clean_bytes(base_path);
    let targ_c = clean_bytes(targ_path);
    if same_word(&targ_c, &base_c) {
        return Ok(b".".to_vec());
    }
    let mut base: &[u8] = &base_c[base_vol.len()..];
    let targ: &[u8] = &targ_c[targ_vol.len()..];
    let sep_buf = [SEPARATOR];
    if base == b"." {
        base = b"";
    } else if base.is_empty() && volume_name_len(&base_vol) > 2
    /* isUNC */
    {
        // Treat any targetpath matching `\\host\share` basePath as absolute path.
        base = &sep_buf;
    }

    // Can't use IsAbs - `\a` and `a` are both relative in Windows.
    let base_slashed = !base.is_empty() && base[0] == SEPARATOR;
    let targ_slashed = !targ.is_empty() && targ[0] == SEPARATOR;
    if base_slashed != targ_slashed || !same_word(&base_vol, &targ_vol) {
        return Err(rel_error(targ_path, base_path));
    }
    // Position base[b0:bi] and targ[t0:ti] at the first differing elements.
    let bl = base.len();
    let tl = targ.len();
    let (mut b0, mut bi, mut t0, mut ti) = (0usize, 0usize, 0usize, 0usize);
    loop {
        while bi < bl && base[bi] != SEPARATOR {
            bi += 1;
        }
        while ti < tl && targ[ti] != SEPARATOR {
            ti += 1;
        }
        if !same_word(&targ[t0..ti], &base[b0..bi]) {
            break;
        }
        if bi < bl {
            bi += 1;
        }
        if ti < tl {
            ti += 1;
        }
        b0 = bi;
        t0 = ti;
    }
    if &base[b0..bi] == b".." {
        return Err(rel_error(targ_path, base_path));
    }
    if b0 != bl {
        // Base elements left. Must go up before going down.
        let seps = base[b0..bl].iter().filter(|&&c| c == SEPARATOR).count();
        let mut size = 2 + seps * 3;
        if tl != t0 {
            size += 1 + tl - t0;
        }
        let mut buf = vec![0u8; size];
        buf[..2].copy_from_slice(b"..");
        let mut n = 2;
        for _ in 0..seps {
            buf[n] = SEPARATOR;
            buf[n + 1..n + 3].copy_from_slice(b"..");
            n += 3;
        }
        if t0 != tl {
            buf[n] = SEPARATOR;
            buf[n + 1..].copy_from_slice(&targ[t0..]);
        }
        return Ok(clean_bytes(&buf));
    }
    Ok(targ[t0..].to_vec())
}

/// `&str` form of [`rel_bytes`].
pub fn rel(base_path: &str, targ_path: &str) -> Result<String, RelError> {
    rel_bytes(base_path.as_bytes(), targ_path.as_bytes())
        .map(|v| String::from_utf8(v).expect("Rel splits only at ASCII bytes"))
}

// Go: internal/filepathlite/path.go:Base
/// Returns the last element of `path` (Go `filepath.Base`).
pub fn base_bytes(path: &[u8]) -> &[u8] {
    if path.is_empty() {
        return b".";
    }
    let mut path = path;
    // Strip trailing slashes.
    while !path.is_empty() && is_path_separator(path[path.len() - 1]) {
        path = &path[0..path.len() - 1];
    }
    // Throw away volume name
    path = &path[volume_name_len(path)..];
    // Find the last element
    let mut i = path.len() as isize - 1;
    while i >= 0 && !is_path_separator(path[i as usize]) {
        i -= 1;
    }
    if i >= 0 {
        path = &path[(i + 1) as usize..];
    }
    // If empty now, it had only slashes.
    if path.is_empty() {
        return b"/";
    }
    path
}

/// `&str` form of [`base_bytes`].
pub fn base(path: &str) -> &str {
    std::str::from_utf8(base_bytes(path.as_bytes())).expect("Base splits only at ASCII bytes")
}

// Go: internal/filepathlite/path.go:Dir
/// Returns all but the last element of `path` (Go `filepath.Dir`).
pub fn dir_bytes(path: &[u8]) -> Vec<u8> {
    let vol = volume_name_bytes(path);
    let mut i = path.len() as isize - 1;
    while i >= vol.len() as isize && !is_path_separator(path[i as usize]) {
        i -= 1;
    }
    let dir = clean_bytes(&path[vol.len()..(i + 1) as usize]);
    if dir == b"." && vol.len() > 2 {
        // must be UNC
        return vol;
    }
    let mut v = vol;
    v.extend_from_slice(&dir);
    v
}

/// `&str` form of [`dir_bytes`].
pub fn dir(path: &str) -> String {
    String::from_utf8(dir_bytes(path.as_bytes())).expect("Dir splits only at ASCII bytes")
}

// Go: internal/filepathlite/path.go:VolumeName
/// Returns the leading volume name; always empty on unix.
pub fn volume_name_bytes(path: &[u8]) -> Vec<u8> {
    from_slash_bytes(&path[..volume_name_len(path)])
}

/// `&str` form of [`volume_name_bytes`].
pub fn volume_name(_path: &str) -> &'static str {
    ""
}

// Go: path/filepath/path_unix.go:HasPrefix
/// Deprecated in Go; kept for completeness.
pub fn has_prefix(p: &str, prefix: &str) -> bool {
    p.starts_with(prefix)
}

// Go: path/filepath/match.go:Match
/// Reports whether `name` matches the shell file name pattern
/// (Go `filepath.Match`, unix). The only possible error is [`BadPattern`].
pub fn match_bytes(pattern: &[u8], name: &[u8]) -> Result<bool, BadPattern> {
    let mut pattern = pattern;
    let mut name = name;
    'pattern: while !pattern.is_empty() {
        let (star, chunk, rest) = scan_chunk(pattern);
        pattern = rest;
        if star && chunk.is_empty() {
            // Trailing * matches rest of string unless it has a /.
            return Ok(!name.contains(&SEPARATOR));
        }
        // Look for match at current position.
        let (t, ok, err) = match_chunk(chunk, name);
        // if we're the last chunk, make sure we've exhausted the name
        // otherwise we'll give a false result even if we could still match
        // using the star
        if ok && (t.is_empty() || !pattern.is_empty()) {
            name = t;
            continue;
        }
        if let Some(err) = err {
            return Err(err);
        }
        if star {
            // Look for match skipping i+1 bytes.
            // Cannot skip /.
            let mut i = 0;
            while i < name.len() && name[i] != SEPARATOR {
                let (t, ok, err) = match_chunk(chunk, &name[i + 1..]);
                if ok {
                    // if we're the last chunk, make sure we exhausted the name
                    if pattern.is_empty() && !t.is_empty() {
                        i += 1;
                        continue;
                    }
                    name = t;
                    continue 'pattern;
                }
                if let Some(err) = err {
                    return Err(err);
                }
                i += 1;
            }
        }
        return Ok(false);
    }
    Ok(name.is_empty())
}

/// `&str` form of [`match_bytes`].
pub fn r#match(pattern: &str, name: &str) -> Result<bool, BadPattern> {
    match_bytes(pattern.as_bytes(), name.as_bytes())
}

// Go: path/filepath/match.go:scanChunk (runtime.GOOS != "windows")
fn scan_chunk(pattern: &[u8]) -> (bool, &[u8], &[u8]) {
    let mut pattern = pattern;
    let mut star = false;
    while !pattern.is_empty() && pattern[0] == b'*' {
        pattern = &pattern[1..];
        star = true;
    }
    let mut inrange = false;
    let mut i = 0;
    while i < pattern.len() {
        match pattern[i] {
            b'\\' => {
                // error check handled in matchChunk: bad pattern.
                if i + 1 < pattern.len() {
                    i += 1;
                }
            }
            b'[' => inrange = true,
            b']' => inrange = false,
            b'*' => {
                if !inrange {
                    return (star, &pattern[..i], &pattern[i..]);
                }
            }
            _ => {}
        }
        i += 1;
    }
    (star, pattern, b"")
}

// Go: path/filepath/match.go:matchChunk (runtime.GOOS != "windows")
fn match_chunk<'s>(chunk: &[u8], s: &'s [u8]) -> (&'s [u8], bool, Option<BadPattern>) {
    let mut chunk = chunk;
    let mut s = s;
    let mut failed = false;
    while !chunk.is_empty() {
        failed = failed || s.is_empty();
        match chunk[0] {
            b'[' => {
                // character class
                let mut r: i32 = 0;
                if !failed {
                    let (rr, n) = utf8::decode_rune(s);
                    r = rr;
                    s = &s[n..];
                }
                chunk = &chunk[1..];
                // possibly negated
                let mut negated = false;
                if !chunk.is_empty() && chunk[0] == b'^' {
                    negated = true;
                    chunk = &chunk[1..];
                }
                // parse all ranges
                let mut matched = false;
                let mut nrange = 0;
                loop {
                    if !chunk.is_empty() && chunk[0] == b']' && nrange > 0 {
                        chunk = &chunk[1..];
                        break;
                    }
                    let lo;
                    let mut hi;
                    match get_esc(chunk) {
                        Ok((rr, nchunk)) => {
                            lo = rr;
                            chunk = nchunk;
                        }
                        Err(e) => return (b"", false, Some(e)),
                    }
                    hi = lo;
                    if chunk[0] == b'-' {
                        match get_esc(&chunk[1..]) {
                            Ok((rr, nchunk)) => {
                                hi = rr;
                                chunk = nchunk;
                            }
                            Err(e) => return (b"", false, Some(e)),
                        }
                    }
                    matched = matched || lo <= r && r <= hi;
                    nrange += 1;
                }
                failed = failed || matched == negated;
            }
            b'?' => {
                if !failed {
                    failed = s[0] == SEPARATOR;
                    let (_, n) = utf8::decode_rune(s);
                    s = &s[n..];
                }
                chunk = &chunk[1..];
            }
            c => {
                if c == b'\\' {
                    chunk = &chunk[1..];
                    if chunk.is_empty() {
                        return (b"", false, Some(BadPattern));
                    }
                }
                // fallthrough / default
                if !failed {
                    failed = chunk[0] != s[0];
                    s = &s[1..];
                }
                chunk = &chunk[1..];
            }
        }
    }
    if failed {
        return (b"", false, None);
    }
    (s, true, None)
}

// Go: path/filepath/match.go:getEsc (runtime.GOOS != "windows")
fn get_esc(chunk: &[u8]) -> Result<(i32, &[u8]), BadPattern> {
    if chunk.is_empty() || chunk[0] == b'-' || chunk[0] == b']' {
        return Err(BadPattern);
    }
    let mut chunk = chunk;
    if chunk[0] == b'\\' {
        chunk = &chunk[1..];
        if chunk.is_empty() {
            return Err(BadPattern);
        }
    }
    let (r, n) = utf8::decode_rune(chunk);
    let mut err = false;
    if r == utf8::RUNE_ERROR && n == 1 {
        err = true;
    }
    let nchunk = &chunk[n..];
    if nchunk.is_empty() {
        err = true;
    }
    if err {
        return Err(BadPattern);
    }
    Ok((r, nchunk))
}
