//! Port of the `net/netip.ParseAddr` subset used by `net/url.parseHost`
//! (go1.27.1 `src/net/netip/netip.go`: ParseAddr, parseIPv4Fields,
//! parseIPv4, parseIPv6, parseAddrError).

use std::fmt;

/// The parsed address. `net/url` only needs `Is4`; the bytes are kept for
/// completeness and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Addr {
    V4([u8; 4]),
    V6 { addr: [u8; 16], zone: Vec<u8> },
}

impl Addr {
    // Go: net/netip/netip.go:Addr.Is4
    pub fn is4(&self) -> bool {
        matches!(self, Addr::V4(_))
    }
}

// Go: net/netip/netip.go:parseAddrError
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseAddrError {
    /// the string given to ParseAddr
    pub input: Vec<u8>,
    /// an explanation of the parse failure
    pub msg: &'static str,
    /// optionally, the unparsed portion of in at which the error occurred.
    pub at: Vec<u8>,
}

// Go: net/netip/netip.go:parseAddrError.Error
impl fmt::Display for ParseAddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let q = go_strconv::quote;
        if !self.at.is_empty() {
            return write!(
                f,
                "ParseAddr({}): {} (at {})",
                q(&self.input),
                self.msg,
                q(&self.at)
            );
        }
        write!(f, "ParseAddr({}): {}", q(&self.input), self.msg)
    }
}

fn err(input: &[u8], msg: &'static str, at: &[u8]) -> ParseAddrError {
    ParseAddrError {
        input: input.to_vec(),
        msg,
        at: at.to_vec(),
    }
}

// Go: net/netip/netip.go:ParseAddr
/// Parses s as an IP address (dotted decimal, IPv6, or IPv6 with zone).
pub fn parse_addr(s: &[u8]) -> Result<Addr, ParseAddrError> {
    for &c in s {
        match c {
            b'.' => return parse_ipv4(s),
            b':' => return parse_ipv6(s),
            b'%' => {
                // Assume that this was trying to be an IPv6 address with
                // a zone specifier, but the address is missing.
                return Err(err(s, "missing IPv6 address", b""));
            }
            _ => {}
        }
    }
    Err(err(s, "unable to parse IP", b""))
}

// Go: net/netip/netip.go:parseIPv4Fields
fn parse_ipv4_fields(
    input: &[u8],
    off: usize,
    end: usize,
    fields: &mut [u8],
) -> Result<(), ParseAddrError> {
    let mut val: i64 = 0;
    let mut pos = 0usize;
    let mut dig_len = 0; // number of digits in current octet
    let s = &input[off..end];
    for i in 0..s.len() {
        if s[i].is_ascii_digit() {
            if dig_len == 1 && val == 0 {
                return Err(err(input, "IPv4 field has octet with leading zero", b""));
            }
            val = val * 10 + (s[i] - b'0') as i64;
            dig_len += 1;
            if val > 255 {
                return Err(err(input, "IPv4 field has value >255", b""));
            }
        } else if s[i] == b'.' {
            // .1.2.3
            // 1.2.3.
            // 1..2.3
            if i == 0 || i == s.len() - 1 || s[i - 1] == b'.' {
                return Err(err(
                    input,
                    "IPv4 field must have at least one digit",
                    &s[i..],
                ));
            }
            // 1.2.3.4.5
            if pos == 3 {
                return Err(err(input, "IPv4 address too long", b""));
            }
            fields[pos] = val as u8;
            pos += 1;
            val = 0;
            dig_len = 0;
        } else {
            return Err(err(input, "unexpected character", &s[i..]));
        }
    }
    if pos < 3 {
        return Err(err(input, "IPv4 address too short", b""));
    }
    fields[3] = val as u8;
    Ok(())
}

// Go: net/netip/netip.go:parseIPv4
fn parse_ipv4(s: &[u8]) -> Result<Addr, ParseAddrError> {
    let mut fields = [0u8; 4];
    parse_ipv4_fields(s, 0, s.len(), &mut fields)?;
    Ok(Addr::V4(fields))
}

// Go: net/netip/netip.go:parseIPv6
fn parse_ipv6(input: &[u8]) -> Result<Addr, ParseAddrError> {
    let mut s = input;

    // Split off the zone right from the start. Yes it's a second scan
    // of the string, but trying to handle it inline makes a bunch of
    // other inner loop conditionals more expensive, and it ends up
    // being slower.
    let mut zone: &[u8] = b"";
    if let Some(i) = s.iter().position(|&c| c == b'%') {
        zone = &s[i + 1..];
        s = &s[..i];
        if zone.is_empty() {
            // Not allowed to have an empty zone if explicitly specified.
            return Err(err(input, "zone must be a non-empty string", b""));
        }
    }

    let mut ip = [0u8; 16];
    let mut ellipsis: isize = -1; // position of ellipsis in ip

    // Might have leading ellipsis
    if s.len() >= 2 && s[0] == b':' && s[1] == b':' {
        ellipsis = 0;
        s = &s[2..];
        // Might be only ellipsis
        if s.is_empty() {
            return Ok(Addr::V6 {
                addr: [0; 16],
                zone: zone.to_vec(),
            });
        }
    }

    // Loop, parsing hex numbers followed by colon.
    let mut i: usize = 0;
    while i < 16 {
        // Hex number. Similar to parseIPv4, inlining the hex number
        // parsing yields a significant performance increase.
        let mut off = 0usize;
        let mut acc: u32 = 0;
        while off < s.len() {
            let c = s[off];
            if c.is_ascii_digit() {
                acc = (acc << 4) + (c - b'0') as u32;
            } else if (b'a'..=b'f').contains(&c) {
                acc = (acc << 4) + (c - b'a' + 10) as u32;
            } else if (b'A'..=b'F').contains(&c) {
                acc = (acc << 4) + (c - b'A' + 10) as u32;
            } else {
                break;
            }
            if off > 3 {
                //more than 4 digits in group, fail.
                return Err(err(input, "each group must have 4 or less digits", s));
            }
            if acc > u16::MAX as u32 {
                // Overflow, fail.
                return Err(err(input, "IPv6 field has value >=2^16", s));
            }
            off += 1;
        }
        if off == 0 {
            // No digits found, fail.
            return Err(err(
                input,
                "each colon-separated field must have at least one digit",
                s,
            ));
        }

        // If followed by dot, might be in trailing IPv4.
        if off < s.len() && s[off] == b'.' {
            if ellipsis < 0 && i != 12 {
                // Not the right place.
                return Err(err(
                    input,
                    "embedded IPv4 address must replace the final 2 fields of the address",
                    s,
                ));
            }
            if i + 4 > 16 {
                // Not enough room.
                return Err(err(
                    input,
                    "too many hex fields to fit an embedded IPv4 at the end of the address",
                    s,
                ));
            }

            let mut end = input.len();
            if !zone.is_empty() {
                end -= zone.len() + 1;
            }
            parse_ipv4_fields(input, end - s.len(), end, &mut ip[i..i + 4])?;
            s = b"";
            i += 4;
            break;
        }

        // Save this 16-bit chunk.
        ip[i] = (acc >> 8) as u8;
        ip[i + 1] = acc as u8;
        i += 2;

        // Stop at end of string.
        s = &s[off..];
        if s.is_empty() {
            break;
        }

        // Otherwise must be followed by colon and more.
        if s[0] != b':' {
            return Err(err(input, "unexpected character, want colon", s));
        } else if s.len() == 1 {
            return Err(err(input, "colon must be followed by more characters", s));
        }
        s = &s[1..];

        // Look for ellipsis.
        if s[0] == b':' {
            if ellipsis >= 0 {
                // already have one
                return Err(err(input, "multiple :: in address", s));
            }
            ellipsis = i as isize;
            s = &s[1..];
            if s.is_empty() {
                // can be at end
                break;
            }
        }
    }

    // Must have used entire string.
    if !s.is_empty() {
        return Err(err(input, "trailing garbage after address", s));
    }

    // If didn't parse enough, expand ellipsis.
    if i < 16 {
        if ellipsis < 0 {
            return Err(err(input, "address string too short", b""));
        }
        let ellipsis = ellipsis as usize;
        let n = 16 - i;
        let mut j = i as isize - 1;
        while j >= ellipsis as isize {
            ip[j as usize + n] = ip[j as usize];
            j -= 1;
        }
        for b in &mut ip[ellipsis..ellipsis + n] {
            *b = 0;
        }
    } else if ellipsis >= 0 {
        // Ellipsis must represent at least one 0 group.
        return Err(err(
            input,
            "the :: must expand to at least one field of zeros",
            b"",
        ));
    }
    Ok(Addr::V6 {
        addr: ip,
        zone: zone.to_vec(),
    })
}
