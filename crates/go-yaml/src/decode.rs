//! Node tree construction from the event stream, and decoding of the node
//! tree into Go-typed targets.
//!
//! Go: gopkg.in/yaml.v2@v2.4.0 decode.go (parser + decoder) and yaml.go
//! (unmarshal, fail/failf, TypeError).
//!
//! Supported decode targets are the ones neohugo uses: `interface{}`
//! (and therefore `map[interface{}]interface{}`, `[]interface{}`), a
//! `map[string]interface{}` top-level value, and `string` (its keys).

use std::collections::{HashMap, HashSet};

use crate::gostd::{base64_std_decode, format_float_g, quote};
use crate::parserc::yaml_parser_parse;
use crate::resolve::{Resolved, resolve, short_tag};
use crate::yamlh::*;
use crate::{Error, IfaceMap, StrMap, Yaml};

// ---------------------------------------------------------------------------
// Nodes

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Document,
    Mapping,
    Sequence,
    Scalar,
    Alias,
}

/// Go: decode.go:node. Nodes live in an arena and refer to each other by
/// index (Go uses pointers; alias identity is by pointer, here by index).
#[derive(Debug)]
pub(crate) struct Node {
    pub kind: NodeKind,
    pub line: i64,
    #[allow(dead_code)]
    pub column: i64,
    pub tag: String,
    /// For an alias node, the resolved alias.
    pub alias: Option<usize>,
    pub value: Vec<u8>,
    pub implicit: bool,
    pub children: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Parser, produces a node tree out of a libyaml event stream.

/// Go: decode.go:parser.
pub(crate) struct NodeParser {
    parser: Parser,
    event: Event,
    anchors: HashMap<Vec<u8>, usize>,
    done_init: bool,
    pub nodes: Vec<Node>,
    /// Maximum nesting depth of the last parsed document (Rust addition,
    /// used to decide whether decoding needs a large stack).
    pub max_depth: usize,
}

/// A fatal decoding error (Go: `panic(yamlError{...})` via fail/failf).
fn failf(msg: impl AsRef<[u8]>) -> Error {
    let mut m = b"yaml: ".to_vec();
    m.extend_from_slice(msg.as_ref());
    Error::fatal(m)
}

/// Concatenate byte pieces (for messages that embed input bytes).
fn cat(parts: &[&[u8]]) -> Vec<u8> {
    parts.concat()
}

impl NodeParser {
    // Go: decode.go:newParser
    pub(crate) fn new(b: &[u8]) -> NodeParser {
        let mut b = b.to_vec();
        if b.is_empty() {
            b = vec![b'\n'];
        }
        Self::new_raw(b)
    }

    // Go: decode.go:newParserFromReader (the input is not padded)
    pub(crate) fn new_from_reader(b: &[u8]) -> NodeParser {
        Self::new_raw(b.to_vec())
    }

    fn new_raw(b: Vec<u8>) -> NodeParser {
        NodeParser {
            parser: Parser::new(b),
            event: Event::default(),
            anchors: HashMap::new(),
            done_init: false,
            nodes: Vec::new(),
            max_depth: 0,
        }
    }

    // Go: decode.go:(*parser).init
    fn init(&mut self) -> Result<(), Error> {
        if self.done_init {
            return Ok(());
        }
        self.expect(EventType::StreamStart)?;
        self.done_init = true;
        Ok(())
    }

    // Go: decode.go:(*parser).expect
    //
    // expect consumes an event from the event stream and
    // checks that it's of the expected type.
    fn expect(&mut self, e: EventType) -> Result<(), Error> {
        if self.event.typ == EventType::NoEvent
            && !yaml_parser_parse(&mut self.parser, &mut self.event)
        {
            return Err(self.fail());
        }
        if self.event.typ == EventType::StreamEnd {
            return Err(failf(
                "attempted to go past the end of stream; corrupted value?",
            ));
        }
        if self.event.typ != e {
            self.parser.problem = format!(
                "expected {} event but got {}",
                e.as_str(),
                self.event.typ.as_str()
            );
            return Err(self.fail());
        }
        self.event = Event::default();
        Ok(())
    }

    // Go: decode.go:(*parser).peek
    //
    // peek peeks at the next event in the event stream,
    // puts the results into p.event and returns the event type.
    fn peek(&mut self) -> Result<EventType, Error> {
        if self.event.typ != EventType::NoEvent {
            return Ok(self.event.typ);
        }
        if !yaml_parser_parse(&mut self.parser, &mut self.event) {
            return Err(self.fail());
        }
        Ok(self.event.typ)
    }

    // Go: decode.go:(*parser).fail
    fn fail(&self) -> Error {
        let mut line: i64 = 0;
        if self.parser.problem_mark.line != 0 {
            line = self.parser.problem_mark.line;
            // Scanner errors don't iterate line before returning error
            if self.parser.error == ErrorType::Scanner {
                line += 1;
            }
        } else if self.parser.context_mark.line != 0 {
            line = self.parser.context_mark.line;
        }
        let mut where_ = String::new();
        if line != 0 {
            where_ = format!("line {line}: ");
        }
        let msg = if !self.parser.problem.is_empty() {
            self.parser.problem.clone()
        } else {
            "unknown problem parsing YAML content".to_string()
        };
        failf(format!("{where_}{msg}"))
    }

    // Go: decode.go:(*parser).anchor
    fn anchor(&mut self, n: usize, anchor: &[u8]) {
        if !anchor.is_empty() {
            self.anchors.insert(anchor.to_vec(), n);
        }
    }

    // Go: decode.go:(*parser).node
    fn node(&mut self, kind: NodeKind) -> usize {
        self.nodes.push(Node {
            kind,
            line: self.event.start_mark.line,
            column: self.event.start_mark.column,
            tag: String::new(),
            alias: None,
            value: Vec::new(),
            implicit: false,
            children: Vec::new(),
        });
        self.nodes.len() - 1
    }

    /// Go: decode.go:(*parser).parse, together with document/alias/scalar/
    /// sequence/mapping. The Go code is recursive; this is the same
    /// traversal with an explicit stack (same event order, same checks in
    /// the same order), so that deeply nested input cannot overflow the
    /// native stack while the tree is built.
    ///
    /// Returns `None` at the end of the stream.
    pub(crate) fn parse(&mut self) -> Result<Option<usize>, Error> {
        self.init()?;

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Frame {
            Document,
            Sequence,
            Mapping,
        }
        // (node index, kind)
        let mut stack: Vec<(usize, Frame)> = Vec::new();
        self.max_depth = 0;

        loop {
            // Close finished collections.
            if let Some(&(idx, frame)) = stack.last() {
                match frame {
                    Frame::Sequence => {
                        if self.peek()? == EventType::SequenceEnd {
                            self.expect(EventType::SequenceEnd)?;
                            stack.pop();
                            if let Some(r) = self.attach(stack.last().map(|x| x.0), idx) {
                                return Ok(Some(r));
                            }
                            continue;
                        }
                    }
                    Frame::Mapping => {
                        // Go: for p.peek() != MAPPING_END { append(p.parse(), p.parse()) }
                        // The value half of a pair is parsed without re-checking.
                        if self.nodes[idx].children.len().is_multiple_of(2)
                            && self.peek()? == EventType::MappingEnd
                        {
                            self.expect(EventType::MappingEnd)?;
                            stack.pop();
                            if let Some(r) = self.attach(stack.last().map(|x| x.0), idx) {
                                return Ok(Some(r));
                            }
                            continue;
                        }
                    }
                    Frame::Document => {
                        if !self.nodes[idx].children.is_empty() {
                            self.expect(EventType::DocumentEnd)?;
                            stack.pop();
                            if let Some(r) = self.attach(stack.last().map(|x| x.0), idx) {
                                return Ok(Some(r));
                            }
                            continue;
                        }
                    }
                }
            }

            // Parse one node at the current position.
            let n = match self.peek()? {
                EventType::Scalar => {
                    // Go: decode.go:(*parser).scalar
                    let n = self.node(NodeKind::Scalar);
                    self.nodes[n].value = std::mem::take(&mut self.event.value);
                    self.nodes[n].tag = String::from_utf8_lossy(&self.event.tag).into_owned();
                    self.nodes[n].implicit = self.event.implicit;
                    let anchor = std::mem::take(&mut self.event.anchor);
                    self.anchor(n, &anchor);
                    self.expect(EventType::Scalar)?;
                    n
                }
                EventType::Alias => {
                    // Go: decode.go:(*parser).alias
                    let n = self.node(NodeKind::Alias);
                    let value = self.event.anchor.clone();
                    self.nodes[n].alias = self.anchors.get(&value).copied();
                    if self.nodes[n].alias.is_none() {
                        return Err(failf(cat(&[b"unknown anchor '", &value, b"' referenced"])));
                    }
                    self.nodes[n].value = value;
                    self.expect(EventType::Alias)?;
                    n
                }
                EventType::MappingStart => {
                    // Go: decode.go:(*parser).mapping
                    let n = self.node(NodeKind::Mapping);
                    let anchor = std::mem::take(&mut self.event.anchor);
                    self.anchor(n, &anchor);
                    self.expect(EventType::MappingStart)?;
                    stack.push((n, Frame::Mapping));
                    self.max_depth = self.max_depth.max(stack.len());
                    continue;
                }
                EventType::SequenceStart => {
                    // Go: decode.go:(*parser).sequence
                    let n = self.node(NodeKind::Sequence);
                    let anchor = std::mem::take(&mut self.event.anchor);
                    self.anchor(n, &anchor);
                    self.expect(EventType::SequenceStart)?;
                    stack.push((n, Frame::Sequence));
                    self.max_depth = self.max_depth.max(stack.len());
                    continue;
                }
                EventType::DocumentStart => {
                    // Go: decode.go:(*parser).document
                    let n = self.node(NodeKind::Document);
                    self.anchors = HashMap::new();
                    self.expect(EventType::DocumentStart)?;
                    stack.push((n, Frame::Document));
                    self.max_depth = self.max_depth.max(stack.len());
                    continue;
                }
                EventType::StreamEnd => {
                    // Happens when attempting to decode an empty buffer.
                    if stack.is_empty() {
                        return Ok(None);
                    }
                    // Go: a nil child would be appended; the next expect
                    // then fails with "attempted to go past the end of stream".
                    return Err(failf(
                        "attempted to go past the end of stream; corrupted value?",
                    ));
                }
                other => {
                    // Go: panic("attempted to parse unknown event: " + ...)
                    return Err(Error::fatal(format!(
                        "attempted to parse unknown event: {}",
                        other.as_str()
                    )));
                }
            };
            if let Some(r) = self.attach(stack.last().map(|x| x.0), n) {
                return Ok(Some(r));
            }
        }
    }

    /// An upper bound of the decoder's recursion depth for the tree rooted
    /// at `root` (Rust addition, used to decide whether decoding needs a
    /// large stack).
    ///
    /// Unlike `max_depth` (the nesting of the parsed tree), this follows
    /// aliases: decoding an alias re-enters the anchored subtree, so a chain
    /// of anchors that each nest ~200 levels and alias the previous one
    /// (`a1: &a1 [[...*a0...]]`, `a2: &a2 [[...*a1...]]`, ...) recurses
    /// `200 × chain length` levels deep although `max_depth` is ~200. Go's
    /// growable goroutine stacks decode such documents; a fixed 2 MB thread
    /// stack would overflow.
    ///
    /// The bound is the longest path through children and alias targets,
    /// where an alias to an ancestor (a cycle, which makes decoding fail
    /// with "anchor '...' value contains itself" at its second visit) counts
    /// as one level; the result is multiplied by the number of such cyclic
    /// aliases + 1, since each can re-enter its anchor once before failing.
    pub(crate) fn decode_depth(&self, root: usize) -> usize {
        const NEW: u8 = 0;
        const ACTIVE: u8 = 1;
        const DONE: u8 = 2;
        let nodes = &self.nodes;
        let mut state = vec![NEW; nodes.len()];
        let mut memo = vec![0usize; nodes.len()];
        let mut cyclic = 0usize;
        let succ = |n: usize, pos: usize| -> Option<usize> {
            let node = &nodes[n];
            if node.kind == NodeKind::Alias {
                if pos == 0 { node.alias } else { None }
            } else {
                node.children.get(pos).copied()
            }
        };
        let mut stack: Vec<(usize, usize)> = vec![(root, 0)];
        state[root] = ACTIVE;
        while let Some(top) = stack.last_mut() {
            let (n, pos) = *top;
            if let Some(c) = succ(n, pos) {
                top.1 += 1;
                match state[c] {
                    NEW => {
                        state[c] = ACTIVE;
                        stack.push((c, 0));
                    }
                    ACTIVE => cyclic += 1,
                    _ => {}
                }
                continue;
            }
            let mut d = 0usize;
            let mut i = 0;
            while let Some(c) = succ(n, i) {
                // A still-active successor is an ancestor (a cycle): one level.
                let cd = if state[c] == DONE { memo[c] } else { 1 };
                d = d.max(cd);
                i += 1;
            }
            memo[n] = d + 1;
            state[n] = DONE;
            stack.pop();
        }
        memo[root].saturating_mul(cyclic.saturating_add(1))
    }

    /// Append a finished node to its parent, or return it when it is the
    /// root.
    fn attach(&mut self, parent: Option<usize>, n: usize) -> Option<usize> {
        match parent {
            None => Some(n),
            Some(parent) => {
                self.nodes[parent].children.push(n);
                None
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Decoder, unmarshals a node into a provided value.

/// A decode target (Go: the `reflect.Value` passed to `d.unmarshal`).
pub(crate) enum Out<'a> {
    /// An `interface{}` slot.
    Iface(&'a mut Yaml),
    /// A `string` slot (keys of `map[string]interface{}`).
    Str(&'a mut Vec<u8>),
    /// The addressable top-level `map[string]interface{}` (nil = None).
    TopMap(&'a mut Option<StrMap>),
    /// An existing map being filled (merge targets).
    Map(MapTarget<'a>),
}

pub(crate) enum MapTarget<'a> {
    Iface(&'a mut IfaceMap),
    Str(&'a mut StrMap),
}

impl Out<'_> {
    /// Go: `out.Type().String()`.
    fn type_string(&self) -> &'static str {
        match self {
            Out::Iface(_) => "interface {}",
            Out::Str(_) => "string",
            Out::TopMap(_) => "map[string]interface {}",
            Out::Map(MapTarget::Iface(_)) => "map[interface {}]interface {}",
            Out::Map(MapTarget::Str(_)) => "map[string]interface {}",
        }
    }
}

/// Go: decode.go:decoder.
pub(crate) struct Decoder<'n> {
    nodes: &'n [Node],
    aliases: HashSet<usize>,
    pub terrors: Vec<Vec<u8>>,
    strict: bool,

    decode_count: i64,
    alias_count: i64,
    alias_depth: i64,
}

// 400,000 decode operations is ~500kb of dense object declarations, or
// ~5kb of dense object declarations with 10000% alias expansion
const ALIAS_RATIO_RANGE_LOW: i64 = 400000;

// 4,000,000 decode operations is ~5MB of dense object declarations, or
// ~4.5MB of dense object declarations with 10% alias expansion
const ALIAS_RATIO_RANGE_HIGH: i64 = 4000000;

// alias_ratio_range is the range over which we scale allowed alias ratios
const ALIAS_RATIO_RANGE: f64 = (ALIAS_RATIO_RANGE_HIGH - ALIAS_RATIO_RANGE_LOW) as f64;

// Go: decode.go:allowedAliasRatio
fn allowed_alias_ratio(decode_count: i64) -> f64 {
    if decode_count <= ALIAS_RATIO_RANGE_LOW {
        // allow 99% to come from alias expansion for small-to-medium documents
        0.99
    } else if decode_count >= ALIAS_RATIO_RANGE_HIGH {
        // allow 10% to come from alias expansion for very large documents
        0.10
    } else {
        // scale smoothly from 99% down to 10% over the range.
        // this maps to 396,000 - 400,000 allowed alias-driven decodes over the range.
        // 400,000 decode operations is ~100MB of allocations in worst-case scenarios (single-item maps).
        //
        // FMA: go1.27.1 darwin/arm64 compiles `0.99 - 0.89*x` to FMSUBD
        // (see PORTING.md), i.e. 0.99 - 0.89*x with a single rounding.
        let x = (decode_count - ALIAS_RATIO_RANGE_LOW) as f64 / ALIAS_RATIO_RANGE;
        (-0.89f64).mul_add(x, 0.99)
    }
}

impl<'n> Decoder<'n> {
    // Go: decode.go:newDecoder
    pub(crate) fn new(nodes: &'n [Node], strict: bool) -> Decoder<'n> {
        Decoder {
            nodes,
            aliases: HashSet::new(),
            terrors: Vec::new(),
            strict,
            decode_count: 0,
            alias_count: 0,
            alias_depth: 0,
        }
    }

    // Go: decode.go:(*decoder).terror
    fn terror(&mut self, n: usize, tag: &str, out: &Out<'_>) {
        let node = &self.nodes[n];
        let tag = if !node.tag.is_empty() {
            node.tag.as_str()
        } else {
            tag
        };
        let mut value: Vec<u8> = node.value.clone();
        if tag != SEQ_TAG && tag != MAP_TAG {
            let mut v = b" `".to_vec();
            if value.len() > 10 {
                v.extend_from_slice(&value[..7]);
                v.extend_from_slice(b"...`");
            } else {
                v.extend_from_slice(&value);
                v.push(b'`');
            }
            value = v;
        }
        let head = format!(
            "line {}: cannot unmarshal {}",
            node.line + 1,
            short_tag(tag)
        );
        let tail = format!(" into {}", out.type_string());
        self.terrors
            .push(cat(&[head.as_bytes(), &value, tail.as_bytes()]));
    }

    // Go: decode.go:(*decoder).unmarshal
    pub(crate) fn unmarshal(&mut self, n: usize, out: &mut Out<'_>) -> Result<bool, Error> {
        self.decode_count += 1;
        if self.alias_depth > 0 {
            self.alias_count += 1;
        }
        if self.alias_count > 100
            && self.decode_count > 1000
            && self.alias_count as f64 / self.decode_count as f64
                > allowed_alias_ratio(self.decode_count)
        {
            return Err(failf("document contains excessive aliasing"));
        }
        match self.nodes[n].kind {
            NodeKind::Document => return self.document(n, out),
            NodeKind::Alias => return self.alias(n, out),
            _ => {}
        }
        // Go: d.prepare — a no-op for the supported targets (no pointers,
        // no Unmarshaler implementations).
        match self.nodes[n].kind {
            NodeKind::Scalar => self.scalar(n, out),
            NodeKind::Mapping => self.mapping(n, out),
            NodeKind::Sequence => self.sequence(n, out),
            _ => Err(Error::fatal("internal error: unknown node kind")),
        }
    }

    // Go: decode.go:(*decoder).document
    fn document(&mut self, n: usize, out: &mut Out<'_>) -> Result<bool, Error> {
        if self.nodes[n].children.len() == 1 {
            let child = self.nodes[n].children[0];
            self.unmarshal(child, out)?;
            return Ok(true);
        }
        Ok(false)
    }

    // Go: decode.go:(*decoder).alias
    fn alias(&mut self, n: usize, out: &mut Out<'_>) -> Result<bool, Error> {
        if self.aliases.contains(&n) {
            // TODO this could actually be allowed in some circumstances.
            return Err(failf(cat(&[
                b"anchor '",
                &self.nodes[n].value,
                b"' value contains itself",
            ])));
        }
        self.aliases.insert(n);
        self.alias_depth += 1;
        let target = self.nodes[n].alias.unwrap_or(n);
        let good = self.unmarshal(target, out)?;
        self.alias_depth -= 1;
        self.aliases.remove(&n);
        Ok(good)
    }

    // Go: decode.go:(*decoder).scalar
    fn scalar(&mut self, n: usize, out: &mut Out<'_>) -> Result<bool, Error> {
        let node = &self.nodes[n];
        let tag: String;
        let mut resolved: Resolved;
        if node.tag.is_empty() && !node.implicit {
            tag = STR_TAG.to_string();
            resolved = Resolved::Str(node.value.clone());
        } else {
            let (t, r) = resolve(&node.tag, &node.value).map_err(|e| failf(&e.0))?;
            tag = t;
            resolved = r;
            if tag == BINARY_TAG {
                let s = match &resolved {
                    Resolved::Str(s) => s.clone(),
                    _ => Vec::new(),
                };
                match base64_std_decode(&s) {
                    Some(data) => resolved = Resolved::Str(data),
                    None => return Err(failf("!!binary value contains invalid base64 data")),
                }
            }
        }
        if resolved == Resolved::Nil {
            match out {
                // Go: `out.Kind() == reflect.Map && !out.CanAddr()` → resetMap
                Out::Map(MapTarget::Iface(m)) => m.clear(),
                Out::Map(MapTarget::Str(m)) => m.clear(),
                // Go: out.Set(reflect.Zero(out.Type()))
                Out::TopMap(m) => **m = None,
                Out::Iface(v) => **v = Yaml::Nil,
                Out::Str(s) => s.clear(),
            }
            return Ok(true);
        }
        match out {
            Out::Str(s) => {
                // Go: `out.Type() == resolvedv.Type()` sets the resolved
                // string; otherwise `out.SetString(n.value)`. For !!binary
                // the resolved value is the decoded data.
                match resolved {
                    Resolved::Str(r) => **s = r,
                    _ => **s = self.nodes[n].value.clone(),
                }
                Ok(true)
            }
            Out::Iface(v) => {
                if tag == TIMESTAMP_TAG {
                    // It looks like a timestamp but for backward compatibility
                    // reasons we set it as a string, so that code that unmarshals
                    // timestamp-like values into interface{} will continue to
                    // see a string and not a time.Time.
                    **v = Yaml::String(self.nodes[n].value.clone());
                } else {
                    **v = match resolved {
                        Resolved::Nil => Yaml::Nil,
                        Resolved::Bool(b) => Yaml::Bool(b),
                        Resolved::Int(i) => Yaml::Int(i),
                        Resolved::Uint64(u) => Yaml::Uint64(u),
                        Resolved::Float64(f) => Yaml::Float64(f),
                        Resolved::Str(s) => Yaml::String(s),
                        Resolved::Timestamp => Yaml::String(self.nodes[n].value.clone()),
                    };
                }
                Ok(true)
            }
            Out::TopMap(_) | Out::Map(_) => {
                self.terror(n, &tag, out);
                Ok(false)
            }
        }
    }

    // Go: decode.go:(*decoder).sequence
    fn sequence(&mut self, n: usize, out: &mut Out<'_>) -> Result<bool, Error> {
        match out {
            Out::Iface(v) => {
                // No type hints. Will have to use a generic sequence.
                let children = self.nodes[n].children.clone();
                let mut items: Vec<Yaml> = Vec::with_capacity(children.len());
                for c in children {
                    let mut e = Yaml::Nil;
                    if self.unmarshal(c, &mut Out::Iface(&mut e))? {
                        items.push(e);
                    }
                }
                **v = Yaml::Seq(items);
                Ok(true)
            }
            _ => {
                self.terror(n, SEQ_TAG, out);
                Ok(false)
            }
        }
    }

    // Go: decode.go:(*decoder).mapping
    fn mapping(&mut self, n: usize, out: &mut Out<'_>) -> Result<bool, Error> {
        match out {
            Out::Iface(v) => {
                let mut m = IfaceMap::new();
                self.fill_map(n, &mut MapTarget::Iface(&mut m))?;
                **v = Yaml::Map(m);
                Ok(true)
            }
            Out::TopMap(m) => {
                if m.is_none() {
                    **m = Some(StrMap::new());
                }
                let target = m.as_mut().expect("just set");
                self.fill_map(n, &mut MapTarget::Str(target))?;
                Ok(true)
            }
            Out::Map(MapTarget::Iface(m)) => {
                self.fill_map(n, &mut MapTarget::Iface(m))?;
                Ok(true)
            }
            Out::Map(MapTarget::Str(m)) => {
                self.fill_map(n, &mut MapTarget::Str(m))?;
                Ok(true)
            }
            Out::Str(_) => {
                self.terror(n, MAP_TAG, out);
                Ok(false)
            }
        }
    }

    /// The key/value loop of Go's `(*decoder).mapping` for map targets.
    fn fill_map(&mut self, n: usize, target: &mut MapTarget<'_>) -> Result<(), Error> {
        let children = self.nodes[n].children.clone();
        let l = children.len();
        let mut i = 0;
        while i < l {
            let ki = children[i];
            let vi = children.get(i + 1).copied();
            if self.is_merge(ki) {
                if let Some(vi) = vi {
                    self.merge(vi, target)?;
                }
                i += 2;
                continue;
            }
            match target {
                MapTarget::Iface(m) => {
                    let mut k = Yaml::Nil;
                    if self.unmarshal(ki, &mut Out::Iface(&mut k))? {
                        if matches!(k, Yaml::Map(_) | Yaml::Seq(_)) {
                            return Err(failf(format!("invalid map key: {}", go_sharp_v(&k))));
                        }
                        if let Some(vi) = vi {
                            let mut e = Yaml::Nil;
                            if self.unmarshal(vi, &mut Out::Iface(&mut e))? {
                                self.set_map_index_iface(vi, m, k, e);
                            }
                        }
                    }
                }
                MapTarget::Str(m) => {
                    let mut k: Vec<u8> = Vec::new();
                    if self.unmarshal(ki, &mut Out::Str(&mut k))?
                        && let Some(vi) = vi
                    {
                        let mut e = Yaml::Nil;
                        if self.unmarshal(vi, &mut Out::Iface(&mut e))? {
                            self.set_map_index_str(vi, m, k, e);
                        }
                    }
                }
            }
            i += 2;
        }
        Ok(())
    }

    // Go: decode.go:(*decoder).setMapIndex
    fn set_map_index_iface(&mut self, n: usize, m: &mut IfaceMap, k: Yaml, v: Yaml) {
        if self.strict && m.get(&k).is_some() {
            self.terrors.push(
                format!(
                    "line {}: key {} already set in map",
                    self.nodes[n].line + 1,
                    go_sharp_v(&k)
                )
                .into_bytes(),
            );
            return;
        }
        m.insert(k, v);
    }

    // Go: decode.go:(*decoder).setMapIndex
    fn set_map_index_str(&mut self, n: usize, m: &mut StrMap, k: Vec<u8>, v: Yaml) {
        if self.strict && m.get(&k).is_some() {
            self.terrors.push(
                format!(
                    "line {}: key {} already set in map",
                    self.nodes[n].line + 1,
                    quote(&k)
                )
                .into_bytes(),
            );
            return;
        }
        m.insert(k, v);
    }

    // Go: decode.go:(*decoder).merge
    fn merge(&mut self, n: usize, target: &mut MapTarget<'_>) -> Result<(), Error> {
        let fail_want_map = || failf("map merge requires map or sequence of maps as the value");
        match self.nodes[n].kind {
            NodeKind::Mapping => {
                self.unmarshal(n, &mut Out::Map(reborrow(target)))?;
            }
            NodeKind::Alias => {
                if let Some(a) = self.nodes[n].alias
                    && self.nodes[a].kind != NodeKind::Mapping
                {
                    return Err(fail_want_map());
                }
                self.unmarshal(n, &mut Out::Map(reborrow(target)))?;
            }
            NodeKind::Sequence => {
                // Step backwards as earlier nodes take precedence.
                let children = self.nodes[n].children.clone();
                for &ni in children.iter().rev() {
                    if self.nodes[ni].kind == NodeKind::Alias {
                        if let Some(a) = self.nodes[ni].alias
                            && self.nodes[a].kind != NodeKind::Mapping
                        {
                            return Err(fail_want_map());
                        }
                    } else if self.nodes[ni].kind != NodeKind::Mapping {
                        return Err(fail_want_map());
                    }
                    self.unmarshal(ni, &mut Out::Map(reborrow(target)))?;
                }
            }
            _ => return Err(fail_want_map()),
        }
        Ok(())
    }

    // Go: decode.go:isMerge
    fn is_merge(&self, n: usize) -> bool {
        let n = &self.nodes[n];
        n.kind == NodeKind::Scalar && n.value == b"<<" && (n.implicit || n.tag == MERGE_TAG)
    }
}

fn reborrow<'b>(t: &'b mut MapTarget<'_>) -> MapTarget<'b> {
    match t {
        MapTarget::Iface(m) => MapTarget::Iface(m),
        MapTarget::Str(m) => MapTarget::Str(m),
    }
}

/// Go's `fmt.Sprintf("%#v", v)` for the values yaml.v2 decodes into
/// `interface{}` (used in "invalid map key" messages).
///
/// Map entries are sorted like Go's fmtsort for same-typed keys; for
/// mixed-type keys Go orders by type descriptor address, which is not
/// reproducible (documented gap).
pub(crate) fn go_sharp_v(v: &Yaml) -> String {
    go_sharp_v_iface(v, false)
}

fn go_sharp_v_iface(v: &Yaml, in_iface: bool) -> String {
    match v {
        Yaml::Nil => {
            if in_iface {
                "interface {}(nil)".to_string()
            } else {
                "<nil>".to_string()
            }
        }
        Yaml::Bool(b) => b.to_string(),
        Yaml::Int(i) => i.to_string(),
        Yaml::Uint64(u) => format!("0x{u:x}"),
        Yaml::Float64(f) => format_float_g(*f),
        Yaml::String(s) => quote(s),
        Yaml::Seq(items) => {
            let parts: Vec<String> = items.iter().map(|e| go_sharp_v_iface(e, true)).collect();
            format!("[]interface {{}}{{{}}}", parts.join(", "))
        }
        Yaml::Map(m) => {
            let mut entries: Vec<(&Yaml, &Yaml)> = m.iter().collect();
            entries.sort_by(|a, b| crate::fmtsort_compare(a.0, b.0));
            let parts: Vec<String> = entries
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}:{}",
                        go_sharp_v_iface(k, true),
                        go_sharp_v_iface(v, true)
                    )
                })
                .collect();
            format!("map[interface {{}}]interface {{}}{{{}}}", parts.join(", "))
        }
    }
}
