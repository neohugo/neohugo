//! NEW: port of Go's `net/http/internal/sniff.go` (go1.27.1): `http.DetectContentType`, used by
//! `media.FromContent` (GetRemote without a Content-Type).
//!
//! Owner: Wave B task T04 (config-base-media).

/// The algorithm uses at most SniffLen bytes to make its decision.
const SNIFF_LEN: usize = 512;

/// Go: `http.DetectContentType(data)`: the algorithm of https://mimesniff.spec.whatwg.org/
/// over at most the first 512 bytes; "application/octet-stream" when nothing more specific is
/// found.
// Go: net/http/internal/sniff.go:DetectContentType
pub fn detect_content_type(data: &[u8]) -> &'static str {
    let data = if data.len() > SNIFF_LEN {
        &data[..SNIFF_LEN]
    } else {
        data
    };

    // Index of the first non-whitespace byte in data.
    let mut first_non_ws = 0;
    while first_non_ws < data.len() && is_ws(data[first_non_ws]) {
        first_non_ws += 1;
    }

    for sig in SNIFF_SIGNATURES {
        let ct = sig.match_(data, first_non_ws);
        if !ct.is_empty() {
            return ct;
        }
    }

    "application/octet-stream" // fallback
}

/// isWS reports whether the provided byte is a whitespace byte (0xWS).
// Go: net/http/internal/sniff.go:isWS
fn is_ws(b: u8) -> bool {
    matches!(b, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

/// isTT reports whether the provided byte is a tag-terminating byte (0xTT).
// Go: net/http/internal/sniff.go:isTT
fn is_tt(b: u8) -> bool {
    matches!(b, b' ' | b'>')
}

enum Sig {
    Html(&'static [u8]),
    Masked {
        mask: &'static [u8],
        pat: &'static [u8],
        skip_ws: bool,
        ct: &'static str,
    },
    Exact(&'static [u8], &'static str),
    Mp4,
    Text,
}

const fn masked(mask: &'static [u8], pat: &'static [u8], ct: &'static str) -> Sig {
    Sig::Masked {
        mask,
        pat,
        skip_ws: false,
        ct,
    }
}

/// Data matching the table in section 6.
static SNIFF_SIGNATURES: &[Sig] = &[
    Sig::Html(b"<!DOCTYPE HTML"),
    Sig::Html(b"<HTML"),
    Sig::Html(b"<HEAD"),
    Sig::Html(b"<SCRIPT"),
    Sig::Html(b"<IFRAME"),
    Sig::Html(b"<H1"),
    Sig::Html(b"<DIV"),
    Sig::Html(b"<FONT"),
    Sig::Html(b"<TABLE"),
    Sig::Html(b"<A"),
    Sig::Html(b"<STYLE"),
    Sig::Html(b"<TITLE"),
    Sig::Html(b"<B"),
    Sig::Html(b"<BODY"),
    Sig::Html(b"<BR"),
    Sig::Html(b"<P"),
    Sig::Html(b"<!--"),
    Sig::Masked {
        mask: b"\xFF\xFF\xFF\xFF\xFF",
        pat: b"<?xml",
        skip_ws: true,
        ct: "text/xml; charset=utf-8",
    },
    Sig::Exact(b"%PDF-", "application/pdf"),
    Sig::Exact(b"%!PS-Adobe-", "application/postscript"),
    // UTF BOMs.
    masked(
        b"\xFF\xFF\x00\x00",
        b"\xFE\xFF\x00\x00",
        "text/plain; charset=utf-16be",
    ),
    masked(
        b"\xFF\xFF\x00\x00",
        b"\xFF\xFE\x00\x00",
        "text/plain; charset=utf-16le",
    ),
    masked(
        b"\xFF\xFF\xFF\x00",
        b"\xEF\xBB\xBF\x00",
        "text/plain; charset=utf-8",
    ),
    // Image types
    Sig::Exact(b"\x00\x00\x01\x00", "image/x-icon"),
    Sig::Exact(b"\x00\x00\x02\x00", "image/x-icon"),
    Sig::Exact(b"BM", "image/bmp"),
    Sig::Exact(b"GIF87a", "image/gif"),
    Sig::Exact(b"GIF89a", "image/gif"),
    masked(
        b"\xFF\xFF\xFF\xFF\x00\x00\x00\x00\xFF\xFF\xFF\xFF\xFF\xFF",
        b"RIFF\x00\x00\x00\x00WEBPVP",
        "image/webp",
    ),
    Sig::Exact(b"\x89PNG\x0D\x0A\x1A\x0A", "image/png"),
    Sig::Exact(b"\xFF\xD8\xFF", "image/jpeg"),
    // Audio and Video types
    masked(
        b"\xFF\xFF\xFF\xFF\x00\x00\x00\x00\xFF\xFF\xFF\xFF",
        b"FORM\x00\x00\x00\x00AIFF",
        "audio/aiff",
    ),
    masked(b"\xFF\xFF\xFF", b"ID3", "audio/mpeg"),
    masked(b"\xFF\xFF\xFF\xFF\xFF", b"OggS\x00", "application/ogg"),
    masked(
        b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF",
        b"MThd\x00\x00\x00\x06",
        "audio/midi",
    ),
    masked(
        b"\xFF\xFF\xFF\xFF\x00\x00\x00\x00\xFF\xFF\xFF\xFF",
        b"RIFF\x00\x00\x00\x00AVI ",
        "video/avi",
    ),
    masked(
        b"\xFF\xFF\xFF\xFF\x00\x00\x00\x00\xFF\xFF\xFF\xFF",
        b"RIFF\x00\x00\x00\x00WAVE",
        "audio/wave",
    ),
    // 6.2.0.2. video/mp4
    Sig::Mp4,
    // 6.2.0.3. video/webm
    Sig::Exact(b"\x1A\x45\xDF\xA3", "video/webm"),
    // Font types
    masked(
        // 34 NULL bytes followed by \xF\xF
        b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\xFF\xFF",
        // 34 NULL bytes followed by the string "LP"
        b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00LP",
        "application/vnd.ms-fontobject",
    ),
    Sig::Exact(b"\x00\x01\x00\x00", "font/ttf"),
    Sig::Exact(b"OTTO", "font/otf"),
    Sig::Exact(b"ttcf", "font/collection"),
    Sig::Exact(b"wOFF", "font/woff"),
    Sig::Exact(b"wOF2", "font/woff2"),
    // Archive types
    Sig::Exact(b"\x1F\x8B\x08", "application/x-gzip"),
    Sig::Exact(b"PK\x03\x04", "application/zip"),
    // RAR's signatures are incorrectly defined by the MIME spec; RAR Labs' definition is
    // used.
    Sig::Exact(b"Rar!\x1A\x07\x00", "application/x-rar-compressed"), // RAR v1.5-v4.0
    Sig::Exact(b"Rar!\x1A\x07\x01\x00", "application/x-rar-compressed"), // RAR v5+
    Sig::Exact(b"\x00\x61\x73\x6D", "application/wasm"),
    Sig::Text, // should be last
];

impl Sig {
    fn match_(&self, data: &[u8], first_non_ws: usize) -> &'static str {
        match self {
            // Go: net/http/internal/sniff.go:(*exactSig).match
            Sig::Exact(sig, ct) => {
                if data.starts_with(sig) {
                    ct
                } else {
                    ""
                }
            }
            // Go: net/http/internal/sniff.go:(*maskedSig).match
            Sig::Masked {
                mask,
                pat,
                skip_ws,
                ct,
            } => {
                let data = if *skip_ws {
                    &data[first_non_ws..]
                } else {
                    data
                };
                if pat.len() != mask.len() {
                    return "";
                }
                if data.len() < pat.len() {
                    return "";
                }
                for (i, pb) in pat.iter().enumerate() {
                    let masked_data = data[i] & mask[i];
                    if masked_data != *pb {
                        return "";
                    }
                }
                ct
            }
            // Go: net/http/internal/sniff.go:(htmlSig).match
            Sig::Html(h) => {
                let data = &data[first_non_ws..];
                if data.len() < h.len() + 1 {
                    return "";
                }
                for (i, b) in h.iter().enumerate() {
                    let mut db = data[i];
                    if b.is_ascii_uppercase() {
                        db &= 0xDF;
                    }
                    if *b != db {
                        return "";
                    }
                }
                // Next byte must be a tag-terminating byte(0xTT).
                if !is_tt(data[h.len()]) {
                    return "";
                }
                "text/html; charset=utf-8"
            }
            // Go: net/http/internal/sniff.go:(mp4Sig).match
            Sig::Mp4 => {
                if data.len() < 12 {
                    return "";
                }
                let box_size = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) as usize;
                if data.len() < box_size || !box_size.is_multiple_of(4) {
                    return "";
                }
                if &data[4..8] != b"ftyp" {
                    return "";
                }
                let mut st = 8;
                while st < box_size {
                    if st != 12 && &data[st..st + 3] == b"mp4" {
                        // (st == 12: the four bytes of the version number of the "major
                        // brand" are ignored.)
                        return "video/mp4";
                    }
                    st += 4;
                }
                ""
            }
            // Go: net/http/internal/sniff.go:(textSig).match
            Sig::Text => {
                for &b in &data[first_non_ws..] {
                    if b <= 0x08
                        || b == 0x0B
                        || (0x0E..=0x1A).contains(&b)
                        || (0x1C..=0x1F).contains(&b)
                    {
                        return "";
                    }
                }
                "text/plain; charset=utf-8"
            }
        }
    }
}
