// Go: github.com/yuin/goldmark@v1.7.12/util/unicode_case_folding.go

use go_unicode::Rune;

use super::unicode_case_folding_gen::{UNICODE_CASE_FOLDING_LENGTH, UNICODE_CASE_FOLDINGS};

/// `unicodeCaseFoldings[r]`: goldmark's generated full case folding map
/// (1530 entries), sorted by code point for a binary search.
pub(crate) fn lookup(r: Rune) -> Option<&'static [Rune]> {
    debug_assert_eq!(UNICODE_CASE_FOLDINGS.len(), UNICODE_CASE_FOLDING_LENGTH);
    UNICODE_CASE_FOLDINGS
        .binary_search_by(|(f, _)| f.cmp(&r))
        .ok()
        .map(|i| UNICODE_CASE_FOLDINGS[i].1)
}
