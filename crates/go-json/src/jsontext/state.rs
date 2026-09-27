//! Port of `encoding/json/jsontext/state.go` (go1.27.1): the token state
//! machine and the object-name stack used for JSON Pointers.
//!
//! The duplicate-name namespaces (`objectNamespaceStack`) are not ported:
//! every v1 entry point sets `AllowDuplicateNames`, which disables them.

use go_unicode::utf8;

use crate::goerr::Err;
use crate::jsonwire;

/// Go: `maxNestingDepth`.
pub(crate) const MAX_NESTING_DEPTH: usize = 10000;

/// Go: `jsontext.Kind` (a byte).
pub(crate) type Kind = u8;

/// Go: `invalidKind`.
pub(crate) const INVALID_KIND: Kind = 0;

// Go: token.go:normKind / Kind.normalize
/// normalize coalesces all possible starting characters of a number as just '0'.
pub(crate) fn normalize(k: u8) -> Kind {
    match k {
        b'n' | b'f' | b't' | b'"' | b'{' | b'}' | b'[' | b']' => k,
        b'-' | b'0'..=b'9' => b'0',
        _ => INVALID_KIND,
    }
}

// Go: token.go:Kind.String
pub(crate) fn kind_string(k: Kind) -> String {
    match k {
        0 => "invalid".to_string(),
        b'n' => "null".to_string(),
        b'f' => "false".to_string(),
        b't' => "true".to_string(),
        b'"' => "string".to_string(),
        b'0' => "number".to_string(),
        b'{' => "{".to_string(),
        b'}' => "}".to_string(),
        b'[' => "[".to_string(),
        b']' => "]".to_string(),
        _ => format!("<invalid jsontext.Kind: {}>", jsonwire::quote_rune(&[k])),
    }
}

// Go: state.go:state
/// state tracks the state of the JSON token stream (and, in Go, the
/// namespace of each object for duplicate-name checks).
#[derive(Clone, Default, Debug)]
pub(crate) struct State {
    /// Tokens validates whether the next token kind is valid.
    pub(crate) tokens: StateMachine,
    /// Names is a stack of object names.
    pub(crate) names: ObjectNameStack,
}

impl State {
    // Go: state.go:state.reset
    pub(crate) fn reset(&mut self) {
        self.tokens.reset();
        self.names.reset();
    }

    // Go: state.go:state.appendStackPointer
    /// appendStackPointer appends a JSON Pointer (RFC 6901) to the current value.
    ///
    ///   - If where is -1, then it points to the previously processed token.
    ///   - If where is 0, then it points to the parent JSON object or array,
    ///     or an object member if in-between an object member key and value.
    ///     This is useful when the position is ambiguous whether
    ///     we are interested in the previous or next token, or
    ///     when we are uncertain whether the next token
    ///     continues or terminates the current object or array.
    ///   - If where is +1, then it points to the next expected value,
    ///     assuming that it continues the current JSON object or array.
    ///     As a special case, if the next token is a JSON object name,
    ///     then it points to the parent JSON object.
    ///
    /// Invariant: Must call s.names.copyQuotedBuffer beforehand.
    pub(crate) fn append_stack_pointer(&mut self, b: &mut Vec<u8>, where_: i32) {
        let mut object_depth = 0usize;
        let depth = self.tokens.depth();
        for i in 1..depth {
            let e = *self.tokens.index(i);
            let mut array_delta: i64 = -1; // by default point to previous array element
            let is_last = i == depth - 1;
            if is_last {
                if where_ < 0 && e.length() == 0
                    || where_ == 0 && !e.need_object_value()
                    || where_ > 0 && e.need_object_name()
                {
                    return;
                } else if where_ > 0 && e.is_array() {
                    array_delta = 0; // point to next array element
                }
            }
            if e.is_object() {
                b.push(b'/');
                let name = self.names.get_unquoted(object_depth).to_vec();
                append_escape_pointer_name(b, &name);
                object_depth += 1;
            } else if e.is_array() {
                b.push(b'/');
                b.extend_from_slice(
                    go_strconv::format_uint((e.length() + array_delta) as u64, 10).as_bytes(),
                );
            }
        }
    }
}

// Go: state.go:appendEscapePointerName
fn append_escape_pointer_name(b: &mut Vec<u8>, name: &[u8]) {
    for (_, r) in utf8::runes(name) {
        match r {
            0x7E => b.extend_from_slice(b"~0"), // '~'
            0x2F => b.extend_from_slice(b"~1"), // '/'
            _ => utf8::append_rune(b, r),
        }
    }
}

// Go: state.go:Pointer.Parent
/// Parent strips off the last token and returns the remaining pointer.
pub(crate) fn pointer_parent(p: &[u8]) -> &[u8] {
    let i = p.iter().rposition(|&c| c == b'/').unwrap_or(0);
    &p[..i]
}

// Go: state.go:Pointer.LastToken
/// LastToken returns the last token in the pointer.
pub(crate) fn pointer_last_token(p: &[u8]) -> Vec<u8> {
    let i = p.iter().rposition(|&c| c == b'/').unwrap_or(0);
    let last = &p[i..];
    let last = last.strip_prefix(b"/").unwrap_or(last);
    unescape_pointer_token(last)
}

// Go: state.go:unescapePointerToken
fn unescape_pointer_token(token: &[u8]) -> Vec<u8> {
    if token.contains(&b'~') {
        let t = go_unicode::strings::replace_all(token, b"~1", b"/").into_owned();
        return go_unicode::strings::replace_all(&t, b"~0", b"~").into_owned();
    }
    token.to_vec()
}

// Go: state.go:Pointer.Contains
/// Contains reports whether the JSON value that p points to
/// is equal to or contains the JSON value that pc points to.
pub(crate) fn pointer_contains(p: &[u8], pc: &[u8]) -> bool {
    match pc.strip_prefix(p) {
        Some(suffix) => suffix.is_empty() || suffix[0] == b'/',
        None => false,
    }
}

// Go: state.go:stateMachine
/// stateMachine is a push-down automaton that validates whether
/// a sequence of tokens is valid or not according to the JSON grammar.
/// It is useful for both encoding and decoding.
///
/// It is a stack where each entry represents a nested JSON object or array.
/// The stack has a minimum depth of 1 where the first level is a
/// virtual JSON array to handle a stream of top-level JSON values.
/// The top-level virtual JSON array is special in that it doesn't require commas
/// between each JSON value.
///
/// For performance, most methods are carefully written to be inlinable.
/// The zero value is a valid state machine ready for use.
#[derive(Clone, Default, Debug)]
pub(crate) struct StateMachine {
    pub(crate) stack: Vec<StateEntry>,
    pub(crate) last: StateEntry,
}

impl StateMachine {
    // Go: state.go:stateMachine.reset
    /// reset resets the state machine.
    /// The machine always starts with a minimum depth of 1.
    pub(crate) fn reset(&mut self) {
        self.stack.clear();
        self.last = STATE_TYPE_ARRAY;
    }

    // Go: state.go:stateMachine.Depth
    /// Depth is the current nested depth of JSON objects and arrays.
    /// It is one-indexed (i.e., top-level values have a depth of 1).
    pub(crate) fn depth(&self) -> usize {
        self.stack.len() + 1
    }

    // Go: state.go:stateMachine.index
    /// index returns a reference to the ith entry.
    /// It is only valid until the next push method call.
    pub(crate) fn index(&mut self, i: usize) -> &mut StateEntry {
        if i == self.stack.len() {
            return &mut self.last;
        }
        &mut self.stack[i]
    }

    // Go: state.go:stateMachine.DepthLength
    /// DepthLength reports the current nested depth and
    /// the length of the last JSON object or array.
    pub(crate) fn depth_length(&self) -> (usize, i64) {
        (self.depth(), self.last.length())
    }

    // Go: state.go:stateMachine.appendLiteral
    /// appendLiteral appends a JSON literal as the next token in the sequence.
    /// If an error is returned, the state is not mutated.
    pub(crate) fn append_literal(&mut self) -> Option<Err> {
        if self.last.need_object_name() {
            Some(Err::NonStringName)
        } else if !self.last.is_valid_namespace() {
            Some(Err::InvalidNamespace)
        } else {
            self.last.increment();
            None
        }
    }

    // Go: state.go:stateMachine.appendString
    /// appendString appends a JSON string as the next token in the sequence.
    /// If an error is returned, the state is not mutated.
    pub(crate) fn append_string(&mut self) -> Option<Err> {
        if !self.last.is_valid_namespace() {
            Some(Err::InvalidNamespace)
        } else {
            self.last.increment();
            None
        }
    }

    // Go: state.go:stateMachine.appendNumber
    /// appendNumber appends a JSON number as the next token in the sequence.
    /// If an error is returned, the state is not mutated.
    pub(crate) fn append_number(&mut self) -> Option<Err> {
        self.append_literal()
    }

    // Go: state.go:stateMachine.pushObject
    /// pushObject appends a JSON begin object token as next in the sequence.
    /// If an error is returned, the state is not mutated.
    pub(crate) fn push_object(&mut self) -> Option<Err> {
        if self.last.need_object_name() {
            Some(Err::NonStringName)
        } else if !self.last.is_valid_namespace() {
            Some(Err::InvalidNamespace)
        } else if self.stack.len() == MAX_NESTING_DEPTH {
            Some(Err::MaxDepth)
        } else {
            self.last.increment();
            self.stack.push(self.last);
            self.last = STATE_TYPE_OBJECT;
            None
        }
    }

    // Go: state.go:stateMachine.popObject
    /// popObject appends a JSON end object token as next in the sequence.
    /// If an error is returned, the state is not mutated.
    pub(crate) fn pop_object(&mut self) -> Option<Err> {
        if !self.last.is_object() {
            Some(Err::MismatchDelim)
        } else if self.last.need_object_value() {
            Some(Err::MissingValue)
        } else if !self.last.is_valid_namespace() {
            Some(Err::InvalidNamespace)
        } else {
            self.last = self.stack.pop().expect("state stack");
            None
        }
    }

    // Go: state.go:stateMachine.pushArray
    /// pushArray appends a JSON begin array token as next in the sequence.
    /// If an error is returned, the state is not mutated.
    pub(crate) fn push_array(&mut self) -> Option<Err> {
        if self.last.need_object_name() {
            Some(Err::NonStringName)
        } else if !self.last.is_valid_namespace() {
            Some(Err::InvalidNamespace)
        } else if self.stack.len() == MAX_NESTING_DEPTH {
            Some(Err::MaxDepth)
        } else {
            self.last.increment();
            self.stack.push(self.last);
            self.last = STATE_TYPE_ARRAY;
            None
        }
    }

    // Go: state.go:stateMachine.popArray
    /// popArray appends a JSON end array token as next in the sequence.
    /// If an error is returned, the state is not mutated.
    pub(crate) fn pop_array(&mut self) -> Option<Err> {
        if !self.last.is_array() || self.stack.is_empty() {
            // forbid popping top-level virtual JSON array
            Some(Err::MismatchDelim)
        } else if !self.last.is_valid_namespace() {
            Some(Err::InvalidNamespace)
        } else {
            self.last = self.stack.pop().expect("state stack");
            None
        }
    }

    // Go: state.go:stateMachine.NeedIndent
    /// NeedIndent reports whether indent whitespace should be injected.
    /// A zero value means that no whitespace should be injected.
    /// A positive value means '\n', indentPrefix, and (n-1) copies of indentBody
    /// should be appended to the output immediately before the next token.
    pub(crate) fn need_indent(&self, next: Kind) -> usize {
        let will_end = next == b'}' || next == b']';
        if self.depth() == 1 {
            0 // top-level values are never indented
        } else if self.last.length() == 0 && will_end {
            0 // an empty object or array is never indented
        } else if self.last.length() == 0 || self.last.need_implicit_comma(next) {
            self.depth()
        } else if will_end {
            self.depth() - 1
        } else {
            0
        }
    }

    // Go: state.go:stateMachine.MayAppendDelim
    /// MayAppendDelim appends a colon or comma that may precede the next token.
    pub(crate) fn may_append_delim(&self, b: &mut Vec<u8>, next: Kind) {
        if self.last.need_implicit_colon() {
            b.push(b':');
        } else if self.last.need_implicit_comma(next) && !self.stack.is_empty() {
            // comma not needed for top-level values
            b.push(b',');
        }
    }

    // Go: state.go:stateMachine.needDelim
    /// needDelim reports whether a colon or comma token should be implicitly emitted
    /// before the next token of the specified kind.
    /// A zero value means no delimiter should be emitted.
    pub(crate) fn need_delim(&self, next: Kind) -> u8 {
        if self.last.need_implicit_colon() {
            b':'
        } else if self.last.need_implicit_comma(next) && !self.stack.is_empty() {
            // comma not needed for top-level values
            b','
        } else {
            0
        }
    }
}

// Go: state.go:stateEntry
/// stateEntry encodes several artifacts within a single unsigned integer:
///   - whether this represents a JSON object or array,
///   - whether this object should check for duplicate names, and
///   - how many elements are in this JSON object or array.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct StateEntry(u64);

// The type mask (1 bit) records whether this is a JSON object or array.
const STATE_TYPE_MASK: u64 = 0x8000_0000_0000_0000;
pub(crate) const STATE_TYPE_OBJECT: StateEntry = StateEntry(0x8000_0000_0000_0000);
pub(crate) const STATE_TYPE_ARRAY: StateEntry = StateEntry(0x0000_0000_0000_0000);

// The name check mask (2 bit) records whether to update
// the namespaces for the current JSON object and
// whether the namespace is valid.
const STATE_DISABLE_NAMESPACE: u64 = 0x4000_0000_0000_0000;
const STATE_INVALID_NAMESPACE: u64 = 0x2000_0000_0000_0000;

// The count mask (61 bits) records the number of elements.
const STATE_COUNT_MASK: u64 = 0x1fff_ffff_ffff_ffff;
const STATE_COUNT_LSB_MASK: u64 = 0x0000_0000_0000_0001;
const STATE_COUNT_ODD: u64 = 0x0000_0000_0000_0001;
const STATE_COUNT_EVEN: u64 = 0x0000_0000_0000_0000;

impl StateEntry {
    // Go: state.go:stateEntry.Length
    /// Length reports the number of elements in the JSON object or array.
    /// Each name and value in an object entry is treated as a separate element.
    pub(crate) fn length(&self) -> i64 {
        (self.0 & STATE_COUNT_MASK) as i64
    }

    // Go: state.go:stateEntry.isObject
    pub(crate) fn is_object(&self) -> bool {
        self.0 & STATE_TYPE_MASK == STATE_TYPE_OBJECT.0
    }

    // Go: state.go:stateEntry.isArray
    pub(crate) fn is_array(&self) -> bool {
        self.0 & STATE_TYPE_MASK == STATE_TYPE_ARRAY.0
    }

    // Go: state.go:stateEntry.NeedObjectName
    /// NeedObjectName reports whether the next token must be a JSON string,
    /// which is necessary for JSON object names.
    pub(crate) fn need_object_name(&self) -> bool {
        self.0 & (STATE_TYPE_MASK | STATE_COUNT_LSB_MASK) == STATE_TYPE_OBJECT.0 | STATE_COUNT_EVEN
    }

    // Go: state.go:stateEntry.needImplicitColon
    /// needImplicitColon reports whether an colon should occur next,
    /// which always occurs after JSON object names.
    pub(crate) fn need_implicit_colon(&self) -> bool {
        self.need_object_value()
    }

    // Go: state.go:stateEntry.needObjectValue
    /// needObjectValue reports whether the next token must be a JSON value,
    /// which is necessary after every JSON object name.
    pub(crate) fn need_object_value(&self) -> bool {
        self.0 & (STATE_TYPE_MASK | STATE_COUNT_LSB_MASK) == STATE_TYPE_OBJECT.0 | STATE_COUNT_ODD
    }

    // Go: state.go:stateEntry.needImplicitComma
    /// needImplicitComma reports whether an comma should occur next,
    /// which always occurs after a value in a JSON object or array
    /// before the next value (or name).
    pub(crate) fn need_implicit_comma(&self, next: Kind) -> bool {
        !self.need_object_value() && self.length() > 0 && next != b'}' && next != b']'
    }

    // Go: state.go:stateEntry.Increment
    /// Increment increments the number of elements for the current object or array.
    /// This assumes that overflow won't practically be an issue since
    /// 1<<bits.OnesCount(stateCountMask) is sufficiently large.
    pub(crate) fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    // Go: state.go:stateEntry.DisableNamespace
    /// DisableNamespace disables the JSON object namespace such that the
    /// Encoder or Decoder no longer updates the namespace.
    pub(crate) fn disable_namespace(&mut self) {
        self.0 |= STATE_DISABLE_NAMESPACE;
    }

    // Go: state.go:stateEntry.isValidNamespace
    /// isValidNamespace reports whether the JSON object namespace is valid.
    pub(crate) fn is_valid_namespace(&self) -> bool {
        self.0 & STATE_INVALID_NAMESPACE == 0
    }
}

/// Go: `invalidOffset`.
const INVALID_OFFSET: isize = isize::MIN;

// Go: state.go:objectNameStack
/// objectNameStack is a stack of names when descending into a JSON object.
/// In contrast to objectNamespaceStack, this only has to remember a single name
/// per JSON object.
///
/// This data structure may contain offsets to encodeBuffer or decodeBuffer.
/// It violates clean abstraction of layers, but is significantly more efficient.
/// This ensures that popping and pushing in the common case is a trivial
/// push/pop of an offset integer.
///
/// The zero value is an empty names stack ready for use.
#[derive(Clone, Default, Debug)]
pub(crate) struct ObjectNameStack {
    /// offsets is a stack of offsets for each name.
    /// A non-negative offset is the ending offset into the local names buffer.
    /// A negative offset is the bit-wise inverse of a starting offset into
    /// a buffer external to objectNameStack.
    offsets: Vec<isize>,
    /// unquotedNames is a back-to-back concatenation of names.
    unquoted_names: Vec<u8>,
}

impl ObjectNameStack {
    // Go: state.go:objectNameStack.reset
    fn reset(&mut self) {
        self.offsets.clear();
        self.unquoted_names.clear();
    }

    // Go: state.go:objectNameStack.getUnquoted
    /// getUnquoted retrieves the ith unquoted name in the stack.
    /// It returns an empty string if the last object is empty.
    ///
    /// Invariant: Must call copyQuotedBuffer beforehand.
    fn get_unquoted(&self, i: usize) -> &[u8] {
        self.ensure_copied_buffer();
        if i == 0 {
            &self.unquoted_names[..self.offsets[0] as usize]
        } else {
            &self.unquoted_names[self.offsets[i - 1] as usize..self.offsets[i] as usize]
        }
    }

    // Go: state.go:objectNameStack.push
    /// push descends into a nested JSON object.
    pub(crate) fn push(&mut self) {
        self.offsets.push(INVALID_OFFSET);
    }

    // Go: state.go:objectNameStack.ReplaceLastQuotedOffset
    /// ReplaceLastQuotedOffset replaces the last name with the starting offset
    /// to the quoted name in some remote buffer. All offsets provided must be
    /// relative to the same buffer until copyQuotedBuffer is called.
    pub(crate) fn replace_last_quoted_offset(&mut self, i: usize) {
        // Use bit-wise inversion instead of naive multiplication by -1 to avoid
        // ambiguity regarding zero (which is a valid offset into the names field).
        // Bit-wise inversion is mathematically equivalent to -i-1,
        // such that 0 becomes -1, 1 becomes -2, and so forth.
        // This ensures that remote offsets are always negative.
        let n = self.offsets.len();
        self.offsets[n - 1] = !(i as isize);
    }

    // Go: state.go:objectNameStack.pop
    /// pop ascends out of a nested JSON object.
    pub(crate) fn pop(&mut self) {
        self.offsets.pop();
    }

    // Go: state.go:objectNameStack.copyQuotedBuffer
    /// copyQuotedBuffer copies names from the remote buffer into the local names
    /// buffer so that there are no more offset references into the remote buffer.
    /// This allows the remote buffer to change contents without affecting
    /// the names that this data structure is trying to remember.
    pub(crate) fn copy_quoted_buffer(&mut self, b: &mut [u8]) {
        // Find the first negative offset.
        let mut i: isize = self.offsets.len() as isize - 1;
        while i >= 0 && self.offsets[i as usize] < 0 {
            i -= 1;
        }

        // Copy each name from the remote buffer into the local buffer.
        let mut i = (i + 1) as usize;
        while i < self.offsets.len() {
            if i == self.offsets.len() - 1 && self.offsets[i] == INVALID_OFFSET {
                if i == 0 {
                    self.offsets[i] = 0;
                } else {
                    self.offsets[i] = self.offsets[i - 1];
                }
                break; // last JSON object had a push without any names
            }

            // As a form of Hyrum proofing, we write an invalid character into the
            // buffer to make misuse of Decoder.ReadToken more obvious.
            // We need to undo that mutation here.
            let q = !self.offsets[i] as usize;
            if b[q] == super::decode::INVALIDATE_BUFFER_BYTE {
                b[q] = b'"';
            }
            let quoted_name = &b[q..];

            // Append the unquoted name to the local buffer.
            let start_offset = if i > 0 {
                self.offsets[i - 1] as usize
            } else {
                0
            };
            self.unquoted_names.truncate(start_offset);
            let n = jsonwire::consume_simple_string(quoted_name);
            if n > 0 {
                self.unquoted_names
                    .extend_from_slice(&quoted_name[1..n - 1]);
            } else {
                let _ = jsonwire::append_unquote(&mut self.unquoted_names, quoted_name);
            }
            self.offsets[i] = self.unquoted_names.len() as isize;
            i += 1;
        }
    }

    // Go: state.go:objectNameStack.ensureCopiedBuffer
    fn ensure_copied_buffer(&self) {
        if let Some(&last) = self.offsets.last() {
            if last < 0 {
                panic!("BUG: copyQuotedBuffer not called beforehand");
            }
        }
    }
}
