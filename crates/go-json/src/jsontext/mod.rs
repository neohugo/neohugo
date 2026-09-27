//! Port of `encoding/json/jsontext` (go1.27.1): the syntactic layer
//! (tokens, values, streaming encoder and decoder state machines).

pub(crate) mod decode;
pub(crate) mod encode;
pub(crate) mod errors;
pub(crate) mod state;
pub(crate) mod token;

use crate::goerr::Err;
use crate::jsonflags;
use crate::jsonopts::Opt;

// Go: value.go:AppendFormat
/// AppendFormat formats the JSON value in src and appends it to dst
/// according to the specified options.
/// See [Value.Format] for more details about the formatting behavior.
///
/// The dst and src may overlap.
/// If an error is reported, then the entirety of src is appended to dst.
pub(crate) fn append_format(dst: &mut Vec<u8>, src: &[u8], opts: &[Opt]) -> Option<Err> {
    let mut e = encode::EncoderState::new_buffered(opts);
    e.opts.flags.set(jsonflags::OMIT_TOP_LEVEL_NEWLINE | 1);
    if let Some(err) = e.write_value(src) {
        dst.extend_from_slice(src);
        return Some(err);
    }
    dst.extend_from_slice(&e.buf);
    None
}
