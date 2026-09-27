//! Port of `encoding/json/internal/jsonopts/options.go` (go1.27.1), plus
//! the option constructors of `jsontext/options.go`, `v2/options.go` and
//! `json/v2_options.go` that the v1 API uses.
//!
//! Go's `Options` is an interface implemented by `jsonflags.Bools`,
//! `jsonopts.Indent`, `jsonopts.IndentPrefix` and `*jsonopts.Struct`; here it
//! is the [`Opt`] enum. The `Marshalers`/`Unmarshalers` values are not
//! ported (v1 never sets them).

use crate::jsonflags::{self, Bools, Flags};

/// Go: `jsonopts.Options` (the concrete implementations the v1 API uses).
#[derive(Clone, Debug)]
pub(crate) enum Opt {
    /// `jsonflags.Bools` (e.g. `jsonflags.EscapeForHTML | 1`).
    Bools(Bools),
    /// `jsonopts.Indent` (from `jsontext.WithIndent`).
    Indent(Vec<u8>),
    /// `jsonopts.IndentPrefix` (from `jsontext.WithIndentPrefix`).
    IndentPrefix(Vec<u8>),
    /// `*jsonopts.Struct`.
    Struct(Box<Struct>),
}

// Go: options.go:Struct (with CoderValues and ArshalValues inlined)
/// Struct is the combination of all options in struct form.
/// This is efficient to pass down the call stack and to query.
#[derive(Clone, Default, Debug, PartialEq)]
pub(crate) struct Struct {
    pub(crate) flags: Flags,

    // CoderValues
    /// jsonflags.Indent
    pub(crate) indent: Vec<u8>,
    /// jsonflags.IndentPrefix
    pub(crate) indent_prefix: Vec<u8>,
    /// jsonflags.ByteLimit
    pub(crate) byte_limit: i64,
    /// jsonflags.DepthLimit
    pub(crate) depth_limit: i64,

    // ArshalValues
    /// valid if jsonflags.FormatTag is set
    pub(crate) format: String,
}

// Go: options.go:DefaultOptionsV1
/// DefaultOptionsV1 is the set of all options that define default v1 behavior.
pub(crate) fn default_options_v1() -> Struct {
    Struct {
        flags: Flags {
            presence: jsonflags::DEFAULT_V1_FLAGS,
            values: jsonflags::DEFAULT_V1_FLAGS,
        },
        ..Struct::default()
    }
}

impl Struct {
    // Go: options.go:GetOption (the jsonflags.Bools case)
    /// The value of a boolean option and whether it is present.
    pub(crate) fn get_option(&self, opt: Bools) -> (bool, bool) {
        let v = self.flags.get(opt);
        let ok = self.flags.has(opt);
        if !ok && opt == jsonflags::STRINGIFY_NUMBERS && self.flags.get(jsonflags::STRING_TAG) {
            return (true, true); // check also whether the option is specified via a `string` tag
        }
        (v, ok)
    }

    // Go: options.go:Struct.Join
    pub(crate) fn join(&mut self, srcs: &[Opt]) {
        for src in srcs {
            match src {
                Opt::Bools(b) => self.flags.set(*b),
                Opt::Indent(s) => {
                    self.flags.set(jsonflags::MULTILINE | jsonflags::INDENT | 1);
                    self.indent = s.clone();
                }
                Opt::IndentPrefix(s) => {
                    self.flags
                        .set(jsonflags::MULTILINE | jsonflags::INDENT_PREFIX | 1);
                    self.indent_prefix = s.clone();
                }
                Opt::Struct(src) => {
                    self.flags.join(src.flags);
                    if src.flags.has(jsonflags::NON_BOOLEAN_FLAGS) {
                        if src.flags.has(jsonflags::INDENT) {
                            self.indent = src.indent.clone();
                        }
                        if src.flags.has(jsonflags::INDENT_PREFIX) {
                            self.indent_prefix = src.indent_prefix.clone();
                        }
                        if src.flags.has(jsonflags::BYTE_LIMIT) {
                            self.byte_limit = src.byte_limit;
                        }
                        if src.flags.has(jsonflags::DEPTH_LIMIT) {
                            self.depth_limit = src.depth_limit;
                        }
                        if src.flags.has(jsonflags::FORMAT_TAG) {
                            self.format = src.format.clone();
                        }
                    }
                }
            }
        }
    }

    // Go: options.go:Struct.InitializeMultiline
    pub(crate) fn initialize_multiline(&mut self) {
        if !self.flags.has(jsonflags::SPACE_AFTER_COLON) {
            self.flags.set(jsonflags::SPACE_AFTER_COLON | 1);
        }
        if !self.flags.has(jsonflags::SPACE_AFTER_COMMA) {
            self.flags.set(jsonflags::SPACE_AFTER_COMMA);
        }
        if !self.flags.has(jsonflags::INDENT) {
            self.flags.set(jsonflags::INDENT | 1);
            self.indent = b"\t".to_vec();
        }
    }
}

// Go: jsontext/options.go:WithIndent
/// WithIndent specifies that the encoder should emit multiline output
/// where each element in a JSON object or array begins on a new, indented line
/// according to the nesting depth. The indent must only be composed of
/// space or tab characters (Go panics otherwise; the v1 callers replace
/// other characters with spaces first).
pub(crate) fn with_indent(indent: &[u8]) -> Opt {
    debug_assert!(indent.iter().all(|&c| c == b' ' || c == b'\t'));
    Opt::Indent(indent.to_vec())
}

// Go: jsontext/options.go:WithIndentPrefix
pub(crate) fn with_indent_prefix(prefix: &[u8]) -> Opt {
    debug_assert!(prefix.iter().all(|&c| c == b' ' || c == b'\t'));
    Opt::IndentPrefix(prefix.to_vec())
}

/// Go's boolean option constructors (`jsontext.AllowDuplicateNames(v)`,
/// `json.ReportErrorsWithLegacySemantics(v)`, ...): `flag | 1` or `flag | 0`.
pub(crate) fn bool_opt(flag: Bools, v: bool) -> Opt {
    Opt::Bools(if v { flag | 1 } else { flag })
}
