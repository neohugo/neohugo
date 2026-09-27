//! Port of Go's `path` package (go1.27.1: `src/path/path.go`, `src/path/match.go`).
//!
//! Every function has a byte-slice form (`*_bytes`, Go `string` semantics for
//! arbitrary bytes) and a `&str` convenience form. All functions only split
//! and join at ASCII bytes (`/`, `.`), so valid UTF-8 input always yields
//! valid UTF-8 output and the `&str` forms are exact.

use crate::BadPattern;
use crate::utf8;

// Go: path/path.go:lazybuf
/// A lazily constructed path buffer. It does not allocate a buffer to hold
/// the output until that output diverges from `s`.
struct LazyBuf<'a> {
    s: &'a [u8],
    buf: Option<Vec<u8>>,
    w: usize,
}

impl LazyBuf<'_> {
    // Go: path/path.go:lazybuf.index
    fn index(&self, i: usize) -> u8 {
        match &self.buf {
            Some(buf) => buf[i],
            None => self.s[i],
        }
    }

    // Go: path/path.go:lazybuf.append
    fn append(&mut self, c: u8) {
        if self.buf.is_none() {
            if self.w < self.s.len() && self.s[self.w] == c {
                self.w += 1;
                return;
            }
            let mut buf = vec![0u8; self.s.len()];
            buf[..self.w].copy_from_slice(&self.s[..self.w]);
            self.buf = Some(buf);
        }
        let w = self.w;
        self.buf.as_mut().expect("buf allocated above")[w] = c;
        self.w += 1;
    }

    // Go: path/path.go:lazybuf.string
    fn string(&self) -> Vec<u8> {
        match &self.buf {
            None => self.s[..self.w].to_vec(),
            Some(buf) => buf[..self.w].to_vec(),
        }
    }
}

// Go: path/path.go:Clean
/// Returns the shortest path name equivalent to `path` by purely lexical
/// processing (Go `path.Clean`).
pub fn clean_bytes(path: &[u8]) -> Vec<u8> {
    if path.is_empty() {
        return b".".to_vec();
    }

    let rooted = path[0] == b'/';
    let n = path.len();

    // Invariants:
    //	reading from path; r is index of next byte to process.
    //	writing to buf; w is index of next byte to write.
    //	dotdot is index in buf where .. must stop, either because
    //		it is the leading slash or it is a leading ../../.. prefix.
    let mut out = LazyBuf {
        s: path,
        buf: None,
        w: 0,
    };
    let (mut r, mut dotdot) = (0usize, 0usize);
    if rooted {
        out.append(b'/');
        r = 1;
        dotdot = 1;
    }

    while r < n {
        if path[r] == b'/' {
            // empty path element
            r += 1;
        } else if path[r] == b'.' && (r + 1 == n || path[r + 1] == b'/') {
            // . element
            r += 1;
        } else if path[r] == b'.' && path[r + 1] == b'.' && (r + 2 == n || path[r + 2] == b'/') {
            // .. element: remove to last /
            r += 2;
            if out.w > dotdot {
                // can backtrack
                out.w -= 1;
                while out.w > dotdot && out.index(out.w) != b'/' {
                    out.w -= 1;
                }
            } else if !rooted {
                // cannot backtrack, but not rooted, so append .. element.
                if out.w > 0 {
                    out.append(b'/');
                }
                out.append(b'.');
                out.append(b'.');
                dotdot = out.w;
            }
        } else {
            // real path element.
            // add slash if needed
            if rooted && out.w != 1 || !rooted && out.w != 0 {
                out.append(b'/');
            }
            // copy element
            while r < n && path[r] != b'/' {
                out.append(path[r]);
                r += 1;
            }
        }
    }

    // Turn empty string into "."
    if out.w == 0 {
        return b".".to_vec();
    }

    out.string()
}

/// `&str` form of [`clean_bytes`].
pub fn clean(path: &str) -> String {
    String::from_utf8(clean_bytes(path.as_bytes())).expect("Clean splits only at ASCII bytes")
}

fn last_index_byte(s: &[u8], c: u8) -> isize {
    match s.iter().rposition(|&b| b == c) {
        Some(i) => i as isize,
        None => -1,
    }
}

// Go: path/path.go:Split
/// Splits `path` immediately following the final slash (Go `path.Split`).
pub fn split_bytes(path: &[u8]) -> (&[u8], &[u8]) {
    let i = last_index_byte(path, b'/');
    let at = (i + 1) as usize;
    (&path[..at], &path[at..])
}

/// `&str` form of [`split_bytes`].
pub fn split(path: &str) -> (&str, &str) {
    let (d, _) = split_bytes(path.as_bytes());
    path.split_at(d.len())
}

// Go: path/path.go:Join
/// Joins path elements with slashes, ignoring empty elements, and cleans the
/// result (Go `path.Join`). Returns an empty string if all elements are empty.
pub fn join_bytes<S: AsRef<[u8]>>(elem: &[S]) -> Vec<u8> {
    let mut size = 0usize;
    for e in elem {
        size += e.as_ref().len();
    }
    if size == 0 {
        return Vec::new();
    }
    let mut buf: Vec<u8> = Vec::with_capacity(size + elem.len() - 1);
    for e in elem {
        let e = e.as_ref();
        if !buf.is_empty() || !e.is_empty() {
            if !buf.is_empty() {
                buf.push(b'/');
            }
            buf.extend_from_slice(e);
        }
    }
    clean_bytes(&buf)
}

/// `&str` form of [`join_bytes`].
pub fn join<S: AsRef<str>>(elem: &[S]) -> String {
    let v: Vec<&[u8]> = elem.iter().map(|e| e.as_ref().as_bytes()).collect();
    String::from_utf8(join_bytes(&v)).expect("Join splits only at ASCII bytes")
}

// Go: path/path.go:Ext
/// Returns the file name extension (Go `path.Ext`).
pub fn ext_bytes(path: &[u8]) -> &[u8] {
    let mut i = path.len() as isize - 1;
    while i >= 0 && path[i as usize] != b'/' {
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

// Go: path/path.go:Base
/// Returns the last element of `path` (Go `path.Base`).
pub fn base_bytes(path: &[u8]) -> &[u8] {
    if path.is_empty() {
        return b".";
    }
    let mut path = path;
    // Strip trailing slashes.
    while !path.is_empty() && path[path.len() - 1] == b'/' {
        path = &path[0..path.len() - 1];
    }
    // Find the last element
    let i = last_index_byte(path, b'/');
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
    let b = base_bytes(path.as_bytes());
    std::str::from_utf8(b).expect("Base splits only at ASCII bytes")
}

// Go: path/path.go:IsAbs
/// Reports whether the path is absolute (Go `path.IsAbs`).
pub fn is_abs_bytes(path: &[u8]) -> bool {
    !path.is_empty() && path[0] == b'/'
}

/// `&str` form of [`is_abs_bytes`].
pub fn is_abs(path: &str) -> bool {
    is_abs_bytes(path.as_bytes())
}

// Go: path/path.go:Dir
/// Returns all but the last element of `path` (Go `path.Dir`).
pub fn dir_bytes(path: &[u8]) -> Vec<u8> {
    let (dir, _) = split_bytes(path);
    clean_bytes(dir)
}

/// `&str` form of [`dir_bytes`].
pub fn dir(path: &str) -> String {
    String::from_utf8(dir_bytes(path.as_bytes())).expect("Dir splits only at ASCII bytes")
}

// Go: path/match.go:Match
/// Reports whether `name` matches the shell pattern (Go `path.Match`).
/// The only possible error is [`BadPattern`].
pub fn match_bytes(pattern: &[u8], name: &[u8]) -> Result<bool, BadPattern> {
    let mut pattern = pattern;
    let mut name = name;
    'pattern: while !pattern.is_empty() {
        let (star, chunk, rest) = scan_chunk(pattern);
        pattern = rest;
        if star && chunk.is_empty() {
            // Trailing * matches rest of string unless it has a /.
            return Ok(!name.contains(&b'/'));
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
            while i < name.len() && name[i] != b'/' {
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
        // Before returning false with no error,
        // check that the remainder of the pattern is syntactically valid.
        while !pattern.is_empty() {
            let (_, chunk, rest) = scan_chunk(pattern);
            pattern = rest;
            let (_, _, err) = match_chunk(chunk, b"");
            if let Some(err) = err {
                return Err(err);
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

// Go: path/match.go:scanChunk
/// Gets the next segment of pattern, which is a non-star string possibly
/// preceded by a star.
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

// Go: path/match.go:matchChunk
/// Checks whether chunk matches the beginning of s. If so, it returns the
/// remainder of s (after the match).
fn match_chunk<'s>(chunk: &[u8], s: &'s [u8]) -> (&'s [u8], bool, Option<BadPattern>) {
    // failed records whether the match has failed.
    // After the match fails, the loop continues on processing chunk,
    // checking that the pattern is well-formed but no longer reading s.
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
                    failed = s[0] == b'/';
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

// Go: path/match.go:getEsc
/// Gets a possibly-escaped character from chunk, for a character class.
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
