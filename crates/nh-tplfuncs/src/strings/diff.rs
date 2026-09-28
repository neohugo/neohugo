//! Port of `github.com/rogpeppe/go-internal/diff` v1.14.1 (`diff.go`), used by `strings.Diff`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).
//!
//! Lines are Go strings (bytes). The unique-line counts are kept in a map keyed by the line
//! bytes; Go's map is only indexed (never ranged), so its order does not matter.

use std::collections::HashMap;

/// A pair is a pair of values tracked for both the x and y side of a diff.
/// It is typically a pair of line indexes.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
struct Pair {
    x: usize,
    y: usize,
}

/// Diff returns an anchored diff of the two texts old and new in the "unified diff" format.
/// If old and new are identical, Diff returns an empty slice (Go: a nil slice).
// Go: github.com/rogpeppe/go-internal/diff/diff.go:Diff
pub fn diff(old_name: &[u8], old: &[u8], new_name: &[u8], new: &[u8]) -> Vec<u8> {
    if old == new {
        return Vec::new();
    }
    let x = lines(old);
    let y = lines(new);

    // Print diff header.
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"diff ");
    out.extend_from_slice(old_name);
    out.push(b' ');
    out.extend_from_slice(new_name);
    out.push(b'\n');
    out.extend_from_slice(b"--- ");
    out.extend_from_slice(old_name);
    out.push(b'\n');
    out.extend_from_slice(b"+++ ");
    out.extend_from_slice(new_name);
    out.push(b'\n');

    // Loop over matches to consider, expanding each match to include surrounding lines, and then
    // printing diff chunks. To avoid setup/teardown cases outside the loop, tgs returns a leading
    // {0,0} and trailing {len(x), len(y)} pair in the sequence of matches.
    let mut done = Pair::default(); // printed up to x[:done.x] and y[:done.y]
    let mut chunk = Pair::default(); // start lines of current chunk
    let mut count = Pair::default(); // number of lines from each side in current chunk
    let mut ctext: Vec<Vec<u8>> = Vec::new(); // lines for current chunk

    for m in tgs(&x, &y) {
        if m.x < done.x {
            // Already handled scanning forward from earlier match.
            continue;
        }

        // Expand matching lines as far possible, establishing that
        // x[start.x:end.x] == y[start.y:end.y]. Note that on the first (or last) iteration we
        // may (or definitely do) have an empty match: start.x==end.x and start.y==end.y.
        let mut start = m;
        while start.x > done.x && start.y > done.y && x[start.x - 1] == y[start.y - 1] {
            start.x -= 1;
            start.y -= 1;
        }
        let mut end = m;
        while end.x < x.len() && end.y < y.len() && x[end.x] == y[end.y] {
            end.x += 1;
            end.y += 1;
        }

        // Emit the mismatched lines before start into this chunk.
        // (No effect on first sentinel iteration, when start = {0,0}.)
        for s in &x[done.x..start.x] {
            ctext.push([&b"-"[..], s].concat());
            count.x += 1;
        }
        for s in &y[done.y..start.y] {
            ctext.push([&b"+"[..], s].concat());
            count.y += 1;
        }

        // If we're not at EOF and have too few common lines, the chunk includes all the common
        // lines and continues.
        const C: usize = 3; // number of context lines
        if (end.x < x.len() || end.y < y.len())
            && (end.x - start.x < C || (!ctext.is_empty() && end.x - start.x < 2 * C))
        {
            for s in &x[start.x..end.x] {
                ctext.push([&b" "[..], s].concat());
                count.x += 1;
                count.y += 1;
            }
            done = end;
            continue;
        }

        // End chunk with common lines for context.
        if !ctext.is_empty() {
            let n = (end.x - start.x).min(C);
            for s in &x[start.x..start.x + n] {
                ctext.push([&b" "[..], s].concat());
                count.x += 1;
                count.y += 1;
            }
            done = Pair {
                x: start.x + n,
                y: start.y + n,
            };

            // Format and emit chunk. Convert line numbers to 1-indexed.
            // Special case: empty file shows up as 0,0 not 1,0.
            if count.x > 0 {
                chunk.x += 1;
            }
            if count.y > 0 {
                chunk.y += 1;
            }
            out.extend_from_slice(
                format!("@@ -{},{} +{},{} @@\n", chunk.x, count.x, chunk.y, count.y).as_bytes(),
            );
            for s in &ctext {
                out.extend_from_slice(s);
            }
            count.x = 0;
            count.y = 0;
            ctext.clear();
        }

        // If we reached EOF, we're done.
        if end.x >= x.len() && end.y >= y.len() {
            break;
        }

        // Otherwise start a new chunk.
        chunk = Pair {
            x: end.x - C,
            y: end.y - C,
        };
        for s in &x[chunk.x..end.x] {
            ctext.push([&b" "[..], s].concat());
            count.x += 1;
            count.y += 1;
        }
        done = end;
    }

    out
}

/// lines returns the lines in the file x, including newlines. If the file does not end in a
/// newline, one is supplied along with a warning about the missing newline.
// Go: github.com/rogpeppe/go-internal/diff/diff.go:lines
fn lines(x: &[u8]) -> Vec<Vec<u8>> {
    let mut l: Vec<Vec<u8>> = go_unicode::strings::split_after(x, b"\n")
        .into_iter()
        .map(|s| s.to_vec())
        .collect();
    if l.last().is_some_and(|s| s.is_empty()) {
        l.pop();
    } else if let Some(last) = l.last_mut() {
        // Treat last line as having a message about the missing newline attached, using the
        // same text as BSD/GNU diff (including the leading backslash).
        last.extend_from_slice(b"\n\\ No newline at end of file\n");
    }
    l
}

/// tgs returns the pairs of indexes of the longest common subsequence of unique lines in x and
/// y, where a unique line is one that appears once in x and once in y.
///
/// The longest common subsequence algorithm is as described in Thomas G. Szymanski, "A Special
/// Case of the Maximal Common Subsequence Problem," Princeton TR #170 (January 1975).
// Go: github.com/rogpeppe/go-internal/diff/diff.go:tgs
fn tgs(x: &[Vec<u8>], y: &[Vec<u8>]) -> Vec<Pair> {
    // Count the number of times each string appears in a and b. We only care about 0, 1, many,
    // counted as 0, -1, -2 for the x side and 0, -4, -8 for the y side. Using negative numbers
    // now lets us distinguish positive line numbers later.
    let mut m: HashMap<&[u8], i64> = HashMap::new();
    for s in x {
        let c = *m.get(s.as_slice()).unwrap_or(&0);
        if c > -2 {
            m.insert(s.as_slice(), c - 1);
        }
    }
    for s in y {
        let c = *m.get(s.as_slice()).unwrap_or(&0);
        if c > -8 {
            m.insert(s.as_slice(), c - 4);
        }
    }

    // Now unique strings can be identified by m[s] = -1+-4.
    //
    // Gather the indexes of those strings in x and y, building:
    //	xi[i] = increasing indexes of unique strings in x.
    //	yi[i] = increasing indexes of unique strings in y.
    //	inv[i] = index j such that x[xi[i]] = y[yi[j]].
    let mut xi: Vec<usize> = Vec::new();
    let mut yi: Vec<usize> = Vec::new();
    let mut inv: Vec<usize> = Vec::new();
    for (i, s) in y.iter().enumerate() {
        if m.get(s.as_slice()) == Some(&(-1 + -4)) {
            m.insert(s.as_slice(), yi.len() as i64);
            yi.push(i);
        }
    }
    for (i, s) in x.iter().enumerate() {
        if let Some(&j) = m.get(s.as_slice())
            && j >= 0
        {
            xi.push(i);
            inv.push(j as usize);
        }
    }

    // Apply Algorithm A from Szymanski's paper. In those terms, A = J = inv and B = [0, n). We
    // add sentinel pairs {0,0}, and {len(x),len(y)} to the returned sequence, to help the
    // processing loop.
    let j = &inv;
    let n = xi.len();
    let mut t = vec![n + 1; n];
    let mut l = vec![0usize; n];
    for i in 0..n {
        // sort.Search(n, func(k int) bool { return T[k] >= J[i] })
        let k = t.partition_point(|&tk| tk < j[i]);
        t[k] = j[i];
        l[i] = k + 1;
    }
    let mut k = 0;
    for &v in &l {
        if k < v {
            k = v;
        }
    }
    let mut seq = vec![Pair::default(); 2 + k];
    seq[1 + k] = Pair {
        x: x.len(),
        y: y.len(),
    }; // sentinel at end
    let lastj = n;
    for i in (0..n).rev() {
        if l[i] == k && j[i] < lastj {
            seq[k] = Pair {
                x: xi[i],
                y: yi[j[i]],
            };
            k -= 1;
        }
    }
    seq[0] = Pair { x: 0, y: 0 }; // sentinel at start
    seq
}
