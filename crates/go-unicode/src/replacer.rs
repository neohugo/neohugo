//! Go's `strings.Replacer` (src/strings/replace.go, go1.27.1) over `&[u8]`.
//!
//! The four Go algorithms (generic trie, single string, byte, byte-to-string)
//! are ported with the same selection rules, so priorities between
//! overlapping old strings, the handling of an empty old string and of
//! repeated old bytes are Go's. Go builds the replacer lazily on first use;
//! here it is built in [`Replacer::new`] (no observable difference).

use std::borrow::Cow;

/// Replacer replaces a list of strings with replacements.
/// It is safe for concurrent use.
#[derive(Debug, Clone)]
pub struct Replacer {
    r: ReplacerImpl,
}

#[derive(Debug, Clone)]
enum ReplacerImpl {
    Generic(Box<GenericReplacer>),
    SingleString(SingleStringReplacer),
    Byte(Box<[u8; 256]>),
    ByteString(Box<ByteStringReplacer>),
}

impl Replacer {
    // Go: src/strings/replace.go:NewReplacer
    /// NewReplacer returns a new Replacer from a list of old, new string
    /// pairs. Replacements are performed in the order they appear in the
    /// target string, without overlapping matches. The old string
    /// comparisons are done in argument order.
    ///
    /// Panics if given an odd number of arguments (as Go does).
    pub fn new<T: AsRef<[u8]>>(oldnew: &[T]) -> Replacer {
        if oldnew.len() % 2 == 1 {
            panic!("strings.NewReplacer: odd argument count");
        }
        let oldnew: Vec<&[u8]> = oldnew.iter().map(|s| s.as_ref()).collect();
        Replacer { r: build(&oldnew) }
    }

    // Go: src/strings/replace.go:Replacer.Replace
    /// Replace returns a copy of s with all replacements performed.
    pub fn replace<'a>(&self, s: &'a [u8]) -> Cow<'a, [u8]> {
        match &self.r {
            ReplacerImpl::Generic(r) => Cow::Owned(r.replace(s)),
            ReplacerImpl::SingleString(r) => r.replace(s),
            ReplacerImpl::Byte(r) => byte_replacer_replace(r, s),
            ReplacerImpl::ByteString(r) => r.replace(s),
        }
    }

    // Go: src/strings/replace.go:Replacer.WriteString
    /// WriteString writes s to w with all replacements performed and returns
    /// the number of bytes written.
    pub fn write_string(&self, w: &mut Vec<u8>, s: &[u8]) -> usize {
        match &self.r {
            ReplacerImpl::Generic(r) => r.write_string(w, s),
            ReplacerImpl::SingleString(r) => r.write_string(w, s),
            ReplacerImpl::Byte(r) => byte_replacer_write_string(r, w, s),
            ReplacerImpl::ByteString(r) => r.write_string(w, s),
        }
    }

    // Go: src/strings/export_test.go:Replacer.Replacer (formatted with %T)
    /// The Go type name of the algorithm `NewReplacer` picked
    /// (`"*strings.genericReplacer"`, ...). Test support for Go's
    /// `TestPickAlgorithm`.
    #[doc(hidden)]
    pub fn algorithm(&self) -> &'static str {
        match &self.r {
            ReplacerImpl::Generic(_) => "*strings.genericReplacer",
            ReplacerImpl::SingleString(_) => "*strings.singleStringReplacer",
            ReplacerImpl::Byte(_) => "*strings.byteReplacer",
            ReplacerImpl::ByteString(_) => "*strings.byteStringReplacer",
        }
    }

    // Go: src/strings/export_test.go:Replacer.PrintTrie
    /// The trie of a generic replacer, one node per line (Go's `PrintTrie`,
    /// test support for `TestGenericTrieBuilding`); `None` for the other
    /// algorithms (Go panics on its type assertion there).
    #[doc(hidden)]
    pub fn print_trie(&self) -> Option<Vec<u8>> {
        match &self.r {
            ReplacerImpl::Generic(r) => {
                let mut s = Vec::new();
                r.print_node(&mut s, ROOT, 0);
                Some(s)
            }
            _ => None,
        }
    }
}

// Go: src/strings/replace.go:Replacer.build
fn build(oldnew: &[&[u8]]) -> ReplacerImpl {
    if oldnew.len() == 2 && oldnew[0].len() > 1 {
        return ReplacerImpl::SingleString(make_single_string_replacer(oldnew[0], oldnew[1]));
    }

    let mut all_new_bytes = true;
    let mut i = 0;
    while i < oldnew.len() {
        if oldnew[i].len() != 1 {
            return ReplacerImpl::Generic(Box::new(make_generic_replacer(oldnew)));
        }
        if oldnew[i + 1].len() != 1 {
            all_new_bytes = false;
        }
        i += 2;
    }

    if all_new_bytes {
        let mut r = Box::new([0u8; 256]);
        for (i, b) in r.iter_mut().enumerate() {
            *b = i as u8;
        }
        // The first occurrence of old->new map takes precedence
        // over the others with the same old string.
        let mut i = oldnew.len() as isize - 2;
        while i >= 0 {
            let o = oldnew[i as usize][0];
            let n = oldnew[i as usize + 1][0];
            r[o as usize] = n;
            i -= 2;
        }
        return ReplacerImpl::Byte(r);
    }

    let mut r = Box::new(ByteStringReplacer {
        replacements: std::array::from_fn(|_| None),
        to_replace: Vec::with_capacity(oldnew.len() / 2),
    });
    // The first occurrence of old->new map takes precedence
    // over the others with the same old string.
    let mut i = oldnew.len() as isize - 2;
    while i >= 0 {
        let o = oldnew[i as usize][0];
        let n = oldnew[i as usize + 1];
        // To avoid counting repetitions multiple times.
        if r.replacements[o as usize].is_none() {
            r.to_replace.push(o);
        }
        r.replacements[o as usize] = Some(n.to_vec());
        i -= 2;
    }
    ReplacerImpl::ByteString(r)
}

/// trieNode is a node in a lookup trie for prioritized key/value pairs. Keys
/// and values may be empty. (Nodes live in `GenericReplacer::nodes`; `next`
/// and `table` hold indices into it.)
#[derive(Debug, Clone, Default)]
struct TrieNode {
    /// value is the value of the trie node's key/value pair. It is empty if
    /// this node is not a complete key.
    value: Vec<u8>,
    /// priority is the priority (higher is more important) of the trie node's
    /// key/value pair; keys are not necessarily matched shortest- or longest-
    /// first. Priority is positive if this node is a complete key, and zero
    /// otherwise.
    priority: usize,
    /// prefix is the difference in keys between this trie node and the next.
    prefix: Vec<u8>,
    next: Option<usize>,
    /// table is a lookup table indexed by the next byte in the key, after
    /// remapping that byte through genericReplacer.mapping to create a dense
    /// index.
    table: Option<Vec<Option<usize>>>,
}

/// genericReplacer is the fully generic algorithm.
/// It's used as a fallback when nothing faster can be used.
#[derive(Debug, Clone)]
struct GenericReplacer {
    nodes: Vec<TrieNode>, // nodes[0] is the root
    /// tableSize is the size of a trie node's lookup table. It is the number
    /// of unique key bytes.
    table_size: usize,
    /// mapping maps from key bytes to a dense index for trieNode.table.
    mapping: [u8; 256],
}

const ROOT: usize = 0;

impl GenericReplacer {
    fn new_node(&mut self, n: TrieNode) -> usize {
        self.nodes.push(n);
        self.nodes.len() - 1
    }

    // Go: src/strings/replace.go:trieNode.add
    fn add(&mut self, t: usize, key: &[u8], val: &[u8], priority: usize) {
        if key.is_empty() {
            if self.nodes[t].priority == 0 {
                self.nodes[t].value = val.to_vec();
                self.nodes[t].priority = priority;
            }
            return;
        }

        if !self.nodes[t].prefix.is_empty() {
            // Need to split the prefix among multiple nodes.
            let prefix = self.nodes[t].prefix.clone();
            let mut n = 0; // length of the longest common prefix
            while n < prefix.len() && n < key.len() {
                if prefix[n] != key[n] {
                    break;
                }
                n += 1;
            }
            if n == prefix.len() {
                let next = self.nodes[t].next.expect("prefix node has next");
                self.add(next, &key[n..], val, priority);
            } else if n == 0 {
                // First byte differs, start a new lookup table here. Looking up
                // what is currently t.prefix[0] will lead to prefixNode, and
                // looking up key[0] will lead to keyNode.
                let t_next = self.nodes[t].next;
                let prefix_node = if prefix.len() == 1 {
                    t_next.expect("prefix node has next")
                } else {
                    self.new_node(TrieNode {
                        prefix: prefix[1..].to_vec(),
                        next: t_next,
                        ..Default::default()
                    })
                };
                let key_node = self.new_node(TrieNode::default());
                let mut table = vec![None; self.table_size];
                table[self.mapping[prefix[0] as usize] as usize] = Some(prefix_node);
                table[self.mapping[key[0] as usize] as usize] = Some(key_node);
                let node = &mut self.nodes[t];
                node.table = Some(table);
                node.prefix = Vec::new();
                node.next = None;
                self.add(key_node, &key[1..], val, priority);
            } else {
                // Insert new node after the common section of the prefix.
                let t_next = self.nodes[t].next;
                let next = self.new_node(TrieNode {
                    prefix: prefix[n..].to_vec(),
                    next: t_next,
                    ..Default::default()
                });
                let node = &mut self.nodes[t];
                node.prefix.truncate(n);
                node.next = Some(next);
                self.add(next, &key[n..], val, priority);
            }
        } else if self.nodes[t].table.is_some() {
            // Insert into existing table.
            let m = self.mapping[key[0] as usize] as usize;
            let child = match self.nodes[t].table.as_ref().unwrap()[m] {
                Some(c) => c,
                None => {
                    let c = self.new_node(TrieNode::default());
                    self.nodes[t].table.as_mut().unwrap()[m] = Some(c);
                    c
                }
            };
            self.add(child, &key[1..], val, priority);
        } else {
            let next = self.new_node(TrieNode::default());
            let node = &mut self.nodes[t];
            node.prefix = key.to_vec();
            node.next = Some(next);
            self.add(next, &[], val, priority);
        }
    }

    // Go: src/strings/replace.go:genericReplacer.lookup
    fn lookup(&self, mut s: &[u8], ignore_root: bool) -> (&[u8], usize, bool) {
        // Iterate down the trie to the end, and grab the value and keylen with
        // the highest priority.
        let mut best_priority = 0;
        let mut node = Some(ROOT);
        let mut n = 0;
        let mut val: &[u8] = &[];
        let mut keylen = 0;
        let mut found = false;
        while let Some(ni) = node {
            let nd = &self.nodes[ni];
            if nd.priority > best_priority && !(ignore_root && ni == ROOT) {
                best_priority = nd.priority;
                val = &nd.value;
                keylen = n;
                found = true;
            }

            if s.is_empty() {
                break;
            }
            if let Some(table) = &nd.table {
                let index = self.mapping[s[0] as usize];
                if index as usize == self.table_size {
                    break;
                }
                node = table[index as usize];
                s = &s[1..];
                n += 1;
            } else if !nd.prefix.is_empty() && s.starts_with(&nd.prefix) {
                n += nd.prefix.len();
                s = &s[nd.prefix.len()..];
                node = nd.next;
            } else {
                break;
            }
        }
        (val, keylen, found)
    }

    // Go: src/strings/export_test.go:genericReplacer.printNode
    fn print_node(&self, s: &mut Vec<u8>, t: usize, depth: usize) {
        let nd = &self.nodes[t];
        if nd.priority > 0 {
            s.push(b'+');
        } else {
            s.push(b'-');
        }
        s.push(b'\n');

        if !nd.prefix.is_empty() {
            s.extend(std::iter::repeat_n(b'.', depth));
            s.extend_from_slice(&nd.prefix);
            let next = nd.next.expect("prefix node has next");
            self.print_node(s, next, depth + nd.prefix.len());
        } else if let Some(table) = &nd.table {
            for (b, &m) in self.mapping.iter().enumerate() {
                if m as usize != self.table_size
                    && let Some(child) = table[m as usize]
                {
                    s.extend(std::iter::repeat_n(b'.', depth));
                    s.push(b as u8);
                    self.print_node(s, child, depth + 1);
                }
            }
        }
    }

    // Go: src/strings/replace.go:genericReplacer.Replace
    fn replace(&self, s: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(s.len());
        self.write_string(&mut buf, s);
        buf
    }

    // Go: src/strings/replace.go:genericReplacer.WriteString
    fn write_string(&self, w: &mut Vec<u8>, s: &[u8]) -> usize {
        let mut n = 0;
        let mut last = 0;
        let mut prev_match_empty = false;
        let root = &self.nodes[ROOT];
        let mut i = 0;
        while i <= s.len() {
            // Fast path: s[i] is not a prefix of any pattern.
            if i != s.len() && root.priority == 0 {
                let index = self.mapping[s[i] as usize] as usize;
                if index == self.table_size || root.table.as_ref().unwrap()[index].is_none() {
                    i += 1;
                    continue;
                }
            }

            // Ignore the empty match iff the previous loop found the empty match.
            let (val, keylen, matched) = self.lookup(&s[i..], prev_match_empty);
            prev_match_empty = matched && keylen == 0;
            if matched {
                w.extend_from_slice(&s[last..i]);
                n += i - last;
                w.extend_from_slice(val);
                n += val.len();
                i += keylen;
                last = i;
                continue;
            }
            i += 1;
        }
        if last != s.len() {
            w.extend_from_slice(&s[last..]);
            n += s.len() - last;
        }
        n
    }
}

// Go: src/strings/replace.go:makeGenericReplacer
fn make_generic_replacer(oldnew: &[&[u8]]) -> GenericReplacer {
    let mut r = GenericReplacer {
        nodes: vec![TrieNode::default()],
        table_size: 0,
        mapping: [0u8; 256],
    };
    // Find each byte used, then assign them each an index.
    let mut i = 0;
    while i < oldnew.len() {
        let key = oldnew[i];
        for &b in key {
            r.mapping[b as usize] = 1;
        }
        i += 2;
    }

    for b in r.mapping {
        r.table_size += b as usize;
    }

    let mut index: u8 = 0;
    for i in 0..256 {
        if r.mapping[i] == 0 {
            r.mapping[i] = r.table_size as u8;
        } else {
            r.mapping[i] = index;
            index = index.wrapping_add(1);
        }
    }
    // Ensure root node uses a lookup table (for performance).
    r.nodes[ROOT].table = Some(vec![None; r.table_size]);

    let mut i = 0;
    while i < oldnew.len() {
        r.add(ROOT, oldnew[i], oldnew[i + 1], oldnew.len() - i);
        i += 2;
    }
    r
}

/// singleStringReplacer is the implementation that's used when there is only
/// one string to replace (and that string has more than one byte).
#[derive(Debug, Clone)]
struct SingleStringReplacer {
    pattern: Vec<u8>,
    /// value is the new string that replaces that pattern when it's found.
    value: Vec<u8>,
}

// Go: src/strings/replace.go:makeSingleStringReplacer
fn make_single_string_replacer(pattern: &[u8], value: &[u8]) -> SingleStringReplacer {
    SingleStringReplacer {
        pattern: pattern.to_vec(),
        value: value.to_vec(),
    }
}

impl SingleStringReplacer {
    // Go: src/strings/search.go:stringFinder.next (Boyer-Moore; returns the
    // index of the first occurrence, as memmem does).
    fn next(&self, text: &[u8]) -> isize {
        match memchr::memmem::find(text, &self.pattern) {
            Some(i) => i as isize,
            None => -1,
        }
    }

    // Go: src/strings/replace.go:singleStringReplacer.Replace
    fn replace<'a>(&self, s: &'a [u8]) -> Cow<'a, [u8]> {
        let mut buf: Vec<u8> = Vec::new();
        let (mut i, mut matched) = (0usize, false);
        loop {
            let m = self.next(&s[i..]);
            if m == -1 {
                break;
            }
            let m = m as usize;
            matched = true;
            buf.extend_from_slice(&s[i..i + m]);
            buf.extend_from_slice(&self.value);
            i += m + self.pattern.len();
        }
        if !matched {
            return Cow::Borrowed(s);
        }
        buf.extend_from_slice(&s[i..]);
        Cow::Owned(buf)
    }

    // Go: src/strings/replace.go:singleStringReplacer.WriteString
    fn write_string(&self, w: &mut Vec<u8>, s: &[u8]) -> usize {
        let mut n = 0;
        let mut i = 0;
        loop {
            let m = self.next(&s[i..]);
            if m == -1 {
                break;
            }
            let m = m as usize;
            w.extend_from_slice(&s[i..i + m]);
            n += m;
            w.extend_from_slice(&self.value);
            n += self.value.len();
            i += m + self.pattern.len();
        }
        w.extend_from_slice(&s[i..]);
        n + s.len() - i
    }
}

// Go: src/strings/replace.go:byteReplacer.Replace
/// byteReplacer is the implementation that's used when all the "old"
/// and "new" values are single ASCII bytes.
/// The array contains replacement bytes indexed by old byte.
fn byte_replacer_replace<'a>(r: &[u8; 256], s: &'a [u8]) -> Cow<'a, [u8]> {
    let mut buf: Option<Vec<u8>> = None; // lazily allocated
    for i in 0..s.len() {
        let b = s[i];
        if r[b as usize] != b {
            let buf = buf.get_or_insert_with(|| s.to_vec());
            buf[i] = r[b as usize];
        }
    }
    match buf {
        None => Cow::Borrowed(s),
        Some(b) => Cow::Owned(b),
    }
}

// Go: src/strings/replace.go:byteReplacer.WriteString
fn byte_replacer_write_string(r: &[u8; 256], w: &mut Vec<u8>, s: &[u8]) -> usize {
    let start = w.len();
    let mut last = 0;
    for i in 0..s.len() {
        let b = s[i];
        if r[b as usize] == b {
            continue;
        }
        if last != i {
            w.extend_from_slice(&s[last..i]);
        }
        last = i + 1;
        w.push(r[b as usize]);
    }
    if last != s.len() {
        w.extend_from_slice(&s[last..]);
    }
    w.len() - start
}

/// byteStringReplacer is the implementation that's used when all the
/// "old" values are single ASCII bytes but the "new" values vary in size.
#[derive(Debug, Clone)]
struct ByteStringReplacer {
    /// replacements contains replacement byte slices indexed by old byte.
    /// A nil []byte (None) means that the old byte should not be replaced.
    replacements: [Option<Vec<u8>>; 256],
    /// toReplace keeps a list of bytes to replace.
    to_replace: Vec<u8>,
}

/// countCutOff controls the ratio of a string length to a number of replacements
/// at which (*byteStringReplacer).Replace switches algorithms.
const COUNT_CUT_OFF: usize = 8;

impl ByteStringReplacer {
    // Go: src/strings/replace.go:byteStringReplacer.Replace
    fn replace<'a>(&self, s: &'a [u8]) -> Cow<'a, [u8]> {
        let mut new_size = s.len() as isize;
        let mut any_changes = false;
        // Is it faster to use Count?
        if self.to_replace.len() * COUNT_CUT_OFF <= s.len() {
            for &x in &self.to_replace {
                let c = memchr::memchr_iter(x, s).count();
                if c != 0 {
                    // The -1 is because we are replacing 1 byte with len(replacements[b]) bytes.
                    let l = self.replacements[x as usize]
                        .as_ref()
                        .map_or(0, |v| v.len());
                    new_size += c as isize * (l as isize - 1);
                    any_changes = true;
                }
            }
        } else {
            for &b in s {
                if let Some(rep) = &self.replacements[b as usize] {
                    // See above for explanation of -1
                    new_size += rep.len() as isize - 1;
                    any_changes = true;
                }
            }
        }
        if !any_changes {
            return Cow::Borrowed(s);
        }
        let mut buf = Vec::with_capacity(new_size.max(0) as usize);
        for &b in s {
            if let Some(rep) = &self.replacements[b as usize] {
                buf.extend_from_slice(rep);
            } else {
                buf.push(b);
            }
        }
        Cow::Owned(buf)
    }

    // Go: src/strings/replace.go:byteStringReplacer.WriteString
    fn write_string(&self, w: &mut Vec<u8>, s: &[u8]) -> usize {
        let start = w.len();
        let mut last = 0;
        for i in 0..s.len() {
            let b = s[i];
            let Some(rep) = &self.replacements[b as usize] else {
                continue;
            };
            if last != i {
                w.extend_from_slice(&s[last..i]);
            }
            last = i + 1;
            w.extend_from_slice(rep);
        }
        if last != s.len() {
            w.extend_from_slice(&s[last..]);
        }
        w.len() - start
    }
}
