//! Port of `encoding/json/internal/jsonflags/flags.go` (go1.27.1).
//!
//! Every boolean option is one bit of a `u64`; bit 0 carries the boolean
//! value in the `Bools` form (`jsonflags.X | 1` sets X to true).
//!
//! Every Go flag is listed, including the ones the v1 API never reaches.

#![allow(dead_code)]

/// Go: `jsonflags.Bools`.
pub(crate) type Bools = u64;

/// reserved for the boolean value itself
pub(crate) const INIT_FLAG: Bools = 1 << 0;

// Coder flags (Go: `initFlag Bools = 1 << iota` ...).
/// encode or decode
pub(crate) const ALLOW_DUPLICATE_NAMES: Bools = 1 << 1;
/// encode or decode
pub(crate) const ALLOW_INVALID_UTF8: Bools = 1 << 2;
/// encode or decode; for internal use by json.Marshal and json.Unmarshal
pub(crate) const WITHIN_ARSHAL_CALL: Bools = 1 << 3;
/// encode only; for internal use by json.Marshal and json.MarshalWrite
pub(crate) const OMIT_TOP_LEVEL_NEWLINE: Bools = 1 << 4;
/// encode only
pub(crate) const PRESERVE_RAW_STRINGS: Bools = 1 << 5;
/// encode only
pub(crate) const CANONICALIZE_RAW_INTS: Bools = 1 << 6;
/// encode only
pub(crate) const CANONICALIZE_RAW_FLOATS: Bools = 1 << 7;
/// encode only
pub(crate) const REORDER_RAW_OBJECTS: Bools = 1 << 8;
/// encode only
pub(crate) const ESCAPE_FOR_HTML: Bools = 1 << 9;
/// encode only
pub(crate) const ESCAPE_FOR_JS: Bools = 1 << 10;
/// encode only
pub(crate) const MULTILINE: Bools = 1 << 11;
/// encode only
pub(crate) const SPACE_AFTER_COLON: Bools = 1 << 12;
/// encode only
pub(crate) const SPACE_AFTER_COMMA: Bools = 1 << 13;
/// encode only; non-boolean flag
pub(crate) const INDENT: Bools = 1 << 14;
/// encode only; non-boolean flag
pub(crate) const INDENT_PREFIX: Bools = 1 << 15;
/// encode or decode; non-boolean flag
pub(crate) const BYTE_LIMIT: Bools = 1 << 16;
/// encode or decode; non-boolean flag
pub(crate) const DEPTH_LIMIT: Bools = 1 << 17;
const MAX_CODER_FLAG: Bools = 1 << 18;

// Marshal/unmarshal v2 flags (Go: `_ Bools = (maxCoderFlag >> 1) << iota`).
/// marshal or unmarshal
pub(crate) const STRINGIFY_NUMBERS: Bools = (MAX_CODER_FLAG >> 1) << 1;
/// marshal only
pub(crate) const DETERMINISTIC: Bools = (MAX_CODER_FLAG >> 1) << 2;
/// marshal only
pub(crate) const FORMAT_NIL_MAP_AS_NULL: Bools = (MAX_CODER_FLAG >> 1) << 3;
/// marshal only
pub(crate) const FORMAT_NIL_SLICE_AS_NULL: Bools = (MAX_CODER_FLAG >> 1) << 4;
/// marshal only
pub(crate) const OMIT_ZERO_STRUCT_FIELDS: Bools = (MAX_CODER_FLAG >> 1) << 5;
/// marshal or unmarshal
pub(crate) const MATCH_CASE_INSENSITIVE_NAMES: Bools = (MAX_CODER_FLAG >> 1) << 6;
/// unmarshal only
pub(crate) const REJECT_UNKNOWN_MEMBERS: Bools = (MAX_CODER_FLAG >> 1) << 7;
/// marshal only; non-boolean flag
pub(crate) const MARSHALERS: Bools = (MAX_CODER_FLAG >> 1) << 8;
/// unmarshal only; non-boolean flag
pub(crate) const UNMARSHALERS: Bools = (MAX_CODER_FLAG >> 1) << 9;
/// marshal or unmarshal
pub(crate) const STRING_TAG: Bools = (MAX_CODER_FLAG >> 1) << 10;
/// marshal or unmarshal; non-boolean flag
pub(crate) const FORMAT_TAG: Bools = (MAX_CODER_FLAG >> 1) << 11;
/// marshal or unmarshal
pub(crate) const FORMAT_TAG_SUPPORTED: Bools = (MAX_CODER_FLAG >> 1) << 12;
const MAX_ARSHAL_V2_FLAG: Bools = (MAX_CODER_FLAG >> 1) << 13;

// Marshal/unmarshal v1 flags (Go: `_ Bools = (maxArshalV2Flag >> 1) << iota`).
/// marshal or unmarshal
pub(crate) const CALL_METHODS_WITH_LEGACY_SEMANTICS: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 1;
/// marshal or unmarshal
pub(crate) const FORMAT_BYTE_ARRAY_AS_ARRAY: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 2;
/// marshal or unmarshal
pub(crate) const FORMAT_BYTES_WITH_LEGACY_SEMANTICS: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 3;
/// marshal or unmarshal
pub(crate) const FORMAT_DURATION_AS_NANO: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 4;
/// marshal or unmarshal
pub(crate) const MATCH_CASE_SENSITIVE_DELIMITER: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 5;
/// unmarshal
pub(crate) const MERGE_WITH_LEGACY_SEMANTICS: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 6;
/// marshal
pub(crate) const OMIT_EMPTY_WITH_LEGACY_SEMANTICS: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 7;
/// unmarshal
pub(crate) const PARSE_BYTES_WITH_LOOSE_RFC4648: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 8;
/// unmarshal
pub(crate) const PARSE_TIME_WITH_LOOSE_RFC3339: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 9;
/// marshal or unmarshal
pub(crate) const REPORT_ERRORS_WITH_LEGACY_SEMANTICS: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 10;
/// marshal or unmarshal
pub(crate) const STRINGIFY_WITH_LEGACY_SEMANTICS: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 11;
/// unmarshal; for internal use by jsonv1.Decoder.UseNumber
pub(crate) const UNMARSHAL_ANY_WITH_RAW_NUMBER: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 12;
/// unmarshal
pub(crate) const UNMARSHAL_ARRAY_FROM_ANY_LENGTH: Bools = (MAX_ARSHAL_V2_FLAG >> 1) << 13;

/// NonBooleanFlags is the set of non-boolean flags,
/// where the value is some other concrete Go type.
/// The value of the flag is stored within jsonopts.Struct.
pub(crate) const NON_BOOLEAN_FLAGS: Bools =
    INDENT | INDENT_PREFIX | BYTE_LIMIT | DEPTH_LIMIT | MARSHALERS | UNMARSHALERS | FORMAT_TAG;

/// DefaultV1Flags is the set of boolean flags that default to true under
/// v1 semantics. None of the non-boolean flags differ between v1 and v2.
pub(crate) const DEFAULT_V1_FLAGS: Bools = ALLOW_DUPLICATE_NAMES
    | ALLOW_INVALID_UTF8
    | ESCAPE_FOR_HTML
    | ESCAPE_FOR_JS
    | PRESERVE_RAW_STRINGS
    | DETERMINISTIC
    | FORMAT_NIL_MAP_AS_NULL
    | FORMAT_NIL_SLICE_AS_NULL
    | MATCH_CASE_INSENSITIVE_NAMES
    | CALL_METHODS_WITH_LEGACY_SEMANTICS
    | FORMAT_BYTE_ARRAY_AS_ARRAY
    | FORMAT_BYTES_WITH_LEGACY_SEMANTICS
    | FORMAT_DURATION_AS_NANO
    | MATCH_CASE_SENSITIVE_DELIMITER
    | MERGE_WITH_LEGACY_SEMANTICS
    | OMIT_EMPTY_WITH_LEGACY_SEMANTICS
    | PARSE_BYTES_WITH_LOOSE_RFC4648
    | PARSE_TIME_WITH_LOOSE_RFC3339
    | REPORT_ERRORS_WITH_LEGACY_SEMANTICS
    | STRINGIFY_WITH_LEGACY_SEMANTICS
    | UNMARSHAL_ARRAY_FROM_ANY_LENGTH;

/// AnyWhitespace reports whether the encoded output might have any whitespace.
pub(crate) const ANY_WHITESPACE: Bools = MULTILINE | SPACE_AFTER_COLON | SPACE_AFTER_COMMA;

/// AnyEscape is the set of flags related to escaping in a JSON string.
pub(crate) const ANY_ESCAPE: Bools = ESCAPE_FOR_HTML | ESCAPE_FOR_JS;

/// CanonicalizeNumbers is the set of flags related to raw number canonicalization.
pub(crate) const CANONICALIZE_NUMBERS: Bools = CANONICALIZE_RAW_INTS | CANONICALIZE_RAW_FLOATS;

/// TagFlags is the set of flags related to the presence of struct field tags.
/// Tags have non-recursive effects, where the tag only applies to
/// the top-level of the field value itself.
/// Whenever descending into a JSON object or array, these flags are cleared.
pub(crate) const TAG_FLAGS: Bools = STRING_TAG | FORMAT_TAG;

// Go: flags.go:Flags
/// Flags is a set of boolean flags.
/// If the presence bit is zero, then the value bit must also be zero.
/// The least-significant bit of both fields is always zero.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct Flags {
    pub(crate) presence: u64,
    pub(crate) values: u64,
}

impl Flags {
    // Go: flags.go:Flags.Join
    /// Join joins two sets of flags such that the latter takes precedence.
    pub(crate) fn join(&mut self, src: Flags) {
        // Copy over all source presence bits over to the destination (using OR),
        // then invert the source presence bits to clear out source value (using AND-NOT),
        // then copy over source value bits over to the destination (using OR).
        self.presence |= src.presence;
        self.values &= !src.presence;
        self.values |= src.values;
    }

    // Go: flags.go:Flags.Set
    /// Set sets both the presence and value for the provided bool (or set of bools).
    pub(crate) fn set(&mut self, f: Bools) {
        // Select out the bits for the flag identifiers (everything except LSB),
        // then set the presence for all the identifier bits (using OR),
        // then invert the identifier bits to clear out the values (using AND-NOT),
        // then copy over all the identifier bits to the value if LSB is 1.
        let id = f & !1u64;
        self.presence |= id;
        self.values &= !id;
        self.values |= (f & 1) * id;
    }

    // Go: flags.go:Flags.Get
    /// Get reports whether the bool (or any of the bools) is true.
    /// This is generally only used with a singular bool.
    /// The value bit of f (i.e., the LSB) is ignored.
    pub(crate) fn get(&self, f: Bools) -> bool {
        self.values & f > 0
    }

    // Go: flags.go:Flags.Has
    /// Has reports whether the bool (or any of the bools) is set.
    /// The value bit of f (i.e., the LSB) is ignored.
    pub(crate) fn has(&self, f: Bools) -> bool {
        self.presence & f > 0
    }

    // Go: flags.go:Flags.Clear
    /// Clear clears both the presence and value for the provided bool or bools.
    /// The value bit of f (i.e., the LSB) is ignored.
    pub(crate) fn clear(&mut self, f: Bools) {
        // Invert f to produce a mask to clear all bits in f (using AND).
        let mask = !f;
        self.presence &= mask;
        self.values &= mask;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_layout_matches_go() {
        // Go: const bitsUsed = 43 (checked at compile time in flags.go).
        assert_eq!(UNMARSHAL_ARRAY_FROM_ANY_LENGTH, 1 << 42);
        assert_eq!(STRINGIFY_NUMBERS, 1 << 18);
        assert_eq!(CALL_METHODS_WITH_LEGACY_SEMANTICS, 1 << 30);
        let _ = INIT_FLAG;
    }

    #[test]
    fn set_get_clear() {
        let mut f = Flags::default();
        f.set(ESCAPE_FOR_HTML | ESCAPE_FOR_JS | 1);
        assert!(f.get(ESCAPE_FOR_HTML) && f.has(ESCAPE_FOR_JS));
        f.set(ESCAPE_FOR_HTML);
        assert!(!f.get(ESCAPE_FOR_HTML) && f.has(ESCAPE_FOR_HTML) && f.get(ESCAPE_FOR_JS));
        f.clear(ESCAPE_FOR_HTML);
        assert!(!f.has(ESCAPE_FOR_HTML));
    }
}
