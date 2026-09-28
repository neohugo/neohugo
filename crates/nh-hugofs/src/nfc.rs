//! Go `golang.org/x/text/unicode/norm` `NFC.String` (x/text v0.26.0, Unicode 15.0.0).
//!
//! Owner: Wave B task T05 (hugofs-vfs).
//!
//! Hugo NFC-normalizes file names on darwin only (`hugofs/fileinfo.go:normalizeFilename`,
//! `component_fs.go:applyMeta`); the callers are behind `cfg(target_os = "macos")`. The port of
//! x/text's `norm` package lives in nh-common (`nh_common::text::norm`, with its tables and its
//! oracle `tools/go-oracle/nh-common/norm`); this module re-exports it for those callers.

pub use nh_common::text::norm::nfc_string;

#[cfg(test)]
mod tests {
    use super::nfc_string;

    #[test]
    fn nfc_basics() {
        assert_eq!(nfc_string("cafe\u{301}.md"), "café.md");
        assert_eq!(nfc_string("café.md"), "café.md");
        assert_eq!(nfc_string("\u{1100}\u{1161}\u{11a8}"), "\u{ac01}");
        assert_eq!(nfc_string("e\u{327}\u{306}"), "\u{1e1d}");
        assert_eq!(nfc_string("บทความ/ไทย.md"), "บทความ/ไทย.md");
        assert_eq!(nfc_string(""), "");
    }
}
