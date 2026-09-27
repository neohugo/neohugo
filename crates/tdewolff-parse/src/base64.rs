//! Go: encoding/base64 (go1.27.1), the `StdEncoding` decoder used by
//! `parse.DataURI`. Ported because Go's decoder skips `\r`/`\n`, is not
//! strict about trailing bits and reports `CorruptInputError` offsets that a
//! Rust base64 crate would not reproduce.

use crate::error::GoError;

const STD_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const PAD_CHAR: u8 = b'=';

const fn decode_map() -> [u8; 256] {
    let mut m = [0xFFu8; 256];
    let mut i = 0;
    while i < 64 {
        m[STD_ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    m
}

static DECODE_MAP: [u8; 256] = decode_map();

// Go: encoding/base64/base64.go:Encoding.DecodedLen (padded)
pub fn std_decoded_len(n: usize) -> usize {
    n / 4 * 3
}

// Go: encoding/base64/base64.go:Encoding.decodeQuantum
fn decode_quantum(dst: &mut [u8], src: &[u8], mut si: usize) -> (usize, usize, Option<GoError>) {
    let mut dbuf = [0u8; 4];
    let mut dlen = 4usize;
    let mut err = None;

    let mut j: isize = 0;
    while j < 4 {
        if src.len() == si {
            if j == 0 {
                return (si, 0, None);
            }
            // j == 1 || padChar != NoPadding
            return (
                si,
                0,
                Some(GoError::Base64CorruptInput(si as i64 - j as i64)),
            );
        }
        let inp = src[si];
        si += 1;

        let out = DECODE_MAP[inp as usize];
        if out != 0xFF {
            dbuf[j as usize] = out;
            j += 1;
            continue;
        }

        if inp == b'\n' || inp == b'\r' {
            continue;
        }

        if inp != PAD_CHAR {
            return (si, 0, Some(GoError::Base64CorruptInput(si as i64 - 1)));
        }

        // We've reached the end and there's padding
        match j {
            0 | 1 => {
                // incorrect padding
                return (si, 0, Some(GoError::Base64CorruptInput(si as i64 - 1)));
            }
            2 => {
                // "==" is expected, the first "=" is already consumed.
                // skip over newlines
                while si < src.len() && (src[si] == b'\n' || src[si] == b'\r') {
                    si += 1;
                }
                if si == src.len() {
                    // not enough padding
                    return (si, 0, Some(GoError::Base64CorruptInput(src.len() as i64)));
                }
                if src[si] != PAD_CHAR {
                    // incorrect padding
                    return (si, 0, Some(GoError::Base64CorruptInput(si as i64 - 1)));
                }
                si += 1;
            }
            _ => {}
        }

        // skip over newlines
        while si < src.len() && (src[si] == b'\n' || src[si] == b'\r') {
            si += 1;
        }
        if si < src.len() {
            // trailing garbage
            err = Some(GoError::Base64CorruptInput(si as i64));
        }
        dlen = j as usize;
        break;
    }

    // Convert 4x 6bit source bytes into 3 bytes
    let val: u32 =
        (dbuf[0] as u32) << 18 | (dbuf[1] as u32) << 12 | (dbuf[2] as u32) << 6 | dbuf[3] as u32;
    let (b2, b1, b0) = (val as u8, (val >> 8) as u8, (val >> 16) as u8);
    match dlen {
        4 => {
            dst[2] = b2;
            dst[1] = b1;
            dst[0] = b0;
        }
        3 => {
            dst[1] = b1;
            dst[0] = b0;
        }
        2 => {
            dst[0] = b0;
        }
        _ => {}
    }
    (si, dlen - 1, err)
}

fn assemble32(n1: u8, n2: u8, n3: u8, n4: u8) -> Option<u32> {
    if n1 | n2 | n3 | n4 == 0xFF {
        return None;
    }
    Some((n1 as u32) << 26 | (n2 as u32) << 20 | (n3 as u32) << 14 | (n4 as u32) << 8)
}

#[allow(clippy::too_many_arguments)]
fn assemble64(n1: u8, n2: u8, n3: u8, n4: u8, n5: u8, n6: u8, n7: u8, n8: u8) -> Option<u64> {
    if n1 | n2 | n3 | n4 | n5 | n6 | n7 | n8 == 0xFF {
        return None;
    }
    Some(
        (n1 as u64) << 58
            | (n2 as u64) << 52
            | (n3 as u64) << 46
            | (n4 as u64) << 40
            | (n5 as u64) << 34
            | (n6 as u64) << 28
            | (n7 as u64) << 22
            | (n8 as u64) << 16,
    )
}

// Go: encoding/base64/base64.go:Encoding.Decode (StdEncoding)
/// Decodes `src` into `dst`, returning the number of bytes written and the
/// error (if any), exactly like Go.
pub fn std_decode(dst: &mut [u8], src: &[u8]) -> (usize, Option<GoError>) {
    if src.is_empty() {
        return (0, None);
    }
    let m = &DECODE_MAP;
    let mut n = 0usize;
    let mut si = 0usize;
    let mut err: Option<GoError>;
    while src.len() - si >= 8 && dst.len() - n >= 8 {
        let s = &src[si..si + 8];
        if let Some(dn) = assemble64(
            m[s[0] as usize],
            m[s[1] as usize],
            m[s[2] as usize],
            m[s[3] as usize],
            m[s[4] as usize],
            m[s[5] as usize],
            m[s[6] as usize],
            m[s[7] as usize],
        ) {
            dst[n..n + 8].copy_from_slice(&dn.to_be_bytes());
            n += 6;
            si += 8;
        } else {
            let ninc;
            (si, ninc, err) = decode_quantum(&mut dst[n..], src, si);
            n += ninc;
            if err.is_some() {
                return (n, err);
            }
        }
    }

    while src.len() - si >= 4 && dst.len() - n >= 4 {
        let s = &src[si..si + 4];
        if let Some(dn) = assemble32(
            m[s[0] as usize],
            m[s[1] as usize],
            m[s[2] as usize],
            m[s[3] as usize],
        ) {
            dst[n..n + 4].copy_from_slice(&dn.to_be_bytes());
            n += 3;
            si += 4;
        } else {
            let ninc;
            (si, ninc, err) = decode_quantum(&mut dst[n..], src, si);
            n += ninc;
            if err.is_some() {
                return (n, err);
            }
        }
    }

    err = None;
    while si < src.len() {
        let ninc;
        (si, ninc, err) = decode_quantum(&mut dst[n..], src, si);
        n += ninc;
        if err.is_some() {
            return (n, err);
        }
    }
    (n, err)
}
