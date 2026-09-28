//! Port of `markup/blackfriday/anchors.go`.
//!
//! Owner: Wave B task T06 (markup).

use go_unicode::utf8;

/// SanitizedAnchorName is how Blackfriday sanitizes anchor names (the `blackfriday` auto ID
/// type). Go ranges over the string: an invalid byte is U+FFFD (not a letter or number).
// Go: markup/blackfriday/anchors.go:SanitizedAnchorName
pub fn sanitized_anchor_name(text: &[u8]) -> String {
    let mut anchor_name: Vec<u8> = Vec::new();
    let mut future_dash = false;
    let mut i = 0;
    while i < text.len() {
        let (r, size) = utf8::decode_rune(&text[i..]);
        if go_unicode::is_letter(r) || go_unicode::is_number(r) {
            if future_dash && !anchor_name.is_empty() {
                anchor_name.push(b'-');
            }
            future_dash = false;
            utf8::append_rune(&mut anchor_name, go_unicode::to_lower(r));
        } else {
            future_dash = true;
        }
        i += size;
    }
    String::from_utf8(anchor_name).expect("runes are re-encoded as UTF-8")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: markup/blackfriday/anchors.go (39 lines; 0/1 funcs executed)
// OK L23-39: SanitizedAnchorName(text string) string
// ---------------------------------------------------------------------------
