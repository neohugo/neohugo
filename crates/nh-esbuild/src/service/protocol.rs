//! Module `service::protocol`.
//!
//! NEW: esbuild stdio service packet codec (evanw/esbuild@v0.25.6 cmd/esbuild/service.go + internal/helpers stdio_protocol.go)
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! esbuild's stdin/stdout service protocol (evanw/esbuild@v0.25.6 `cmd/esbuild/stdio_protocol.go`):
//! little-endian length-prefixed packets `{id: u32 (bit 0 = response), value}` where values are a
//! tagged encoding (null/bool/int/string/bytes/array/map). This is the same code on both ends of
//! the pipe (esbuild's JavaScript host has an identical copy), ported function by function.
//! Byte-exact outputs depend only on esbuild itself; this codec just has to be correct.

/// A protocol value (Go: `interface{}` holding nil, bool, int, string, []byte, []interface{} or
/// map[string]interface{}).
#[derive(Clone, Debug, PartialEq)]
pub enum PacketValue {
    Null,
    Bool(bool),
    /// Go writes `uint32(v)` and reads `int(uint32)`; the host side keeps the 32 bits as `i32`
    /// (esbuild's `-1` "undefined" detail round-trips as `-1`).
    Int(i32),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<PacketValue>),
    /// Keys in the order they were decoded (Go encodes them sorted, see [`encode_packet`]).
    Map(Vec<(String, PacketValue)>),
}

impl PacketValue {
    /// A map from `(key, value)` pairs.
    pub fn map<K: Into<String>>(
        entries: impl IntoIterator<Item = (K, PacketValue)>,
    ) -> PacketValue {
        PacketValue::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    pub fn str(s: impl Into<String>) -> PacketValue {
        PacketValue::String(s.into())
    }

    /// An array of strings.
    pub fn strings<S: AsRef<str>>(v: impl IntoIterator<Item = S>) -> PacketValue {
        PacketValue::Array(
            v.into_iter()
                .map(|s| PacketValue::str(s.as_ref()))
                .collect(),
        )
    }

    /// Go `request["key"]` on a `map[string]interface{}`: the value of a key (the last one if a
    /// map has duplicates, as a Go map assignment keeps the last).
    pub fn get(&self, key: &str) -> Option<&PacketValue> {
        match self {
            PacketValue::Map(m) => m.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Go `v.(string)`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PacketValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Go `v.(int)`.
    pub fn as_int(&self) -> Option<i32> {
        match self {
            PacketValue::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Go `v.(bool)`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            PacketValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Go `v.([]byte)`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            PacketValue::Bytes(b) => Some(b),
            _ => None,
        }
    }

    /// Go `v.([]interface{})`.
    pub fn as_array(&self) -> Option<&[PacketValue]> {
        match self {
            PacketValue::Array(a) => Some(a),
            _ => None,
        }
    }
}

/// A protocol packet.
#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    pub id: u32,
    pub is_request: bool,
    pub value: PacketValue,
}

// Go: cmd/esbuild/stdio_protocol.go:readUint32
/// Reads a little-endian `u32`; `None` (Go `ok == false`) if fewer than 4 bytes are left.
pub fn read_uint32(bytes: &[u8]) -> Option<(u32, &[u8])> {
    if bytes.len() >= 4 {
        let v = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        return Some((v, &bytes[4..]));
    }
    None
}

// Go: cmd/esbuild/stdio_protocol.go:writeUint32
pub fn write_uint32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

// Go: cmd/esbuild/stdio_protocol.go:readLengthPrefixedSlice
/// A `u32` length followed by that many bytes: `(slice, left over)`.
pub fn read_length_prefixed_slice(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    if let Some((length, after_length)) = read_uint32(bytes)
        && after_length.len() as u64 >= length as u64
    {
        let length = length as usize;
        return Some((&after_length[..length], &after_length[length..]));
    }
    None
}

// Go: cmd/esbuild/stdio_protocol.go:encodePacket
/// Encodes a packet, including its `u32` length prefix. Map keys are written sorted (Go:
/// `sort.Strings(keys)`); a duplicated key keeps its last value, like a Go map.
pub fn encode_packet(p: &Packet) -> Vec<u8> {
    fn visit(bytes: &mut Vec<u8>, value: &PacketValue) {
        match value {
            PacketValue::Null => bytes.push(0),
            PacketValue::Bool(v) => {
                bytes.push(1);
                bytes.push(u8::from(*v));
            }
            PacketValue::Int(v) => {
                bytes.push(2);
                write_uint32(bytes, *v as u32);
            }
            PacketValue::String(v) => {
                bytes.push(3);
                write_uint32(bytes, v.len() as u32);
                bytes.extend_from_slice(v.as_bytes());
            }
            PacketValue::Bytes(v) => {
                bytes.push(4);
                write_uint32(bytes, v.len() as u32);
                bytes.extend_from_slice(v);
            }
            PacketValue::Array(v) => {
                bytes.push(5);
                write_uint32(bytes, v.len() as u32);
                for item in v {
                    visit(bytes, item);
                }
            }
            PacketValue::Map(v) => {
                // Sort keys for determinism (a Go map holds each key once: the last value wins).
                let mut entries: std::collections::BTreeMap<&[u8], &PacketValue> =
                    std::collections::BTreeMap::new();
                for (k, x) in v {
                    entries.insert(k.as_bytes(), x);
                }
                bytes.push(6);
                write_uint32(bytes, entries.len() as u32);
                for (k, x) in entries {
                    write_uint32(bytes, k.len() as u32);
                    bytes.extend_from_slice(k);
                    visit(bytes, x);
                }
            }
        }
    }

    let mut bytes = Vec::new();
    write_uint32(&mut bytes, 0); // Reserve space for the length
    if p.is_request {
        write_uint32(&mut bytes, p.id << 1);
    } else {
        write_uint32(&mut bytes, (p.id << 1) | 1);
    }
    visit(&mut bytes, &p.value);
    let len = (bytes.len() - 4) as u32;
    bytes[..4].copy_from_slice(&len.to_le_bytes()); // Patch the length in
    bytes
}

// Go: cmd/esbuild/stdio_protocol.go:decodePacket
/// Decodes one packet body (without its length prefix; see [`read_length_prefixed_slice`]).
///
/// Returns the packet and the number of bytes consumed, which is always `bytes.len()`: like Go,
/// trailing bytes make the packet invalid. Go panics on an unknown value tag or a truncated bool
/// (`"Invalid packet"`, an index out of range); the port returns `None` for both, as for the
/// truncations Go reports with `ok == false`. Strings that are not valid UTF-8 keep their bytes
/// through [`String::from_utf8_lossy`] only where esbuild never sends such bytes (keys and
/// string values are Go strings built from UTF-8 source text and paths).
pub fn decode_packet(bytes: &[u8]) -> Option<(Packet, usize)> {
    fn visit(bytes: &mut &[u8], depth: usize) -> Option<PacketValue> {
        // Go recurses without a limit (its stack grows to 1 GB); a packet this deep never
        // comes from esbuild, so refuse it instead of overflowing the Rust stack.
        if depth > 10_000 {
            return None;
        }
        let (&kind, rest) = bytes.split_first()?;
        *bytes = rest;
        match kind {
            0 => Some(PacketValue::Null),
            1 => {
                let (&value, rest) = bytes.split_first()?;
                *bytes = rest;
                Some(PacketValue::Bool(value != 0))
            }
            2 => {
                let (value, next) = read_uint32(bytes)?;
                *bytes = next;
                Some(PacketValue::Int(value as i32))
            }
            3 => {
                let (value, next) = read_length_prefixed_slice(bytes)?;
                *bytes = next;
                Some(PacketValue::String(bytes_to_string(value)))
            }
            4 => {
                let (value, next) = read_length_prefixed_slice(bytes)?;
                *bytes = next;
                Some(PacketValue::Bytes(value.to_vec()))
            }
            5 => {
                let (count, next) = read_uint32(bytes)?;
                *bytes = next;
                let mut value = Vec::with_capacity((count as usize).min(bytes.len()));
                for _ in 0..count {
                    value.push(visit(bytes, depth + 1)?);
                }
                Some(PacketValue::Array(value))
            }
            6 => {
                let (count, next) = read_uint32(bytes)?;
                *bytes = next;
                let mut value: Vec<(String, PacketValue)> =
                    Vec::with_capacity((count as usize).min(bytes.len()));
                for _ in 0..count {
                    let (key, next) = read_length_prefixed_slice(bytes)?;
                    *bytes = next;
                    let item = visit(bytes, depth + 1)?;
                    let key = bytes_to_string(key);
                    // value[string(key)] = item: a repeated key replaces the earlier value.
                    if let Some(e) = value.iter_mut().find(|(k, _)| *k == key) {
                        e.1 = item;
                    } else {
                        value.push((key, item));
                    }
                }
                Some(PacketValue::Map(value))
            }
            _ => None,
        }
    }

    let (id, rest) = read_uint32(bytes)?;
    let is_request = (id & 1) == 0;
    let id = id >> 1;
    let mut rest = rest;
    let value = visit(&mut rest, 0)?;
    if !rest.is_empty() {
        return None;
    }
    Some((
        Packet {
            id,
            is_request,
            value,
        },
        bytes.len(),
    ))
}

/// Go `string(b)` for protocol strings (see [`decode_packet`]).
fn bytes_to_string(b: &[u8]) -> String {
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(b).into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(v: PacketValue) {
        let p = Packet {
            id: 7,
            is_request: true,
            value: v,
        };
        let b = encode_packet(&p);
        let (body, rest) = read_length_prefixed_slice(&b).unwrap();
        assert!(rest.is_empty());
        let (q, n) = decode_packet(body).unwrap();
        assert_eq!(n, body.len());
        assert_eq!(q, p);
    }

    #[test]
    fn values_roundtrip() {
        roundtrip(PacketValue::Null);
        roundtrip(PacketValue::Bool(true));
        roundtrip(PacketValue::Int(-1));
        roundtrip(PacketValue::str("héllo"));
        roundtrip(PacketValue::Bytes(vec![0, 255, 3]));
        roundtrip(PacketValue::Array(vec![
            PacketValue::Int(1),
            PacketValue::str("x"),
        ]));
        roundtrip(PacketValue::map([
            ("a", PacketValue::Null),
            ("b", PacketValue::Array(vec![])),
        ]));
    }
}
