//! The packet codec of esbuild's stdio service protocol.
//!
//! esbuild's `--service` mode talks over stdin/stdout in little-endian length-prefixed packets.
//! A packet body is a `u32` id (bit 0 clear for requests, set for responses; the id itself is
//! shifted left by one) followed by one tagged [`Value`]. esbuild's own JavaScript host and its
//! Go service use this same encoding; this module is an independent implementation of it.
//!
//! Only `std` is used here.

use std::collections::BTreeMap;
use std::fmt;

/// Values nested deeper than this are refused (esbuild nests a few levels; this protects the
/// reader thread's stack).
const MAX_DEPTH: usize = 256;

/// A protocol value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    /// A 32-bit integer; esbuild writes `-1` for "no detail".
    Int(i32),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    /// Keys are unique and written in byte order (a repeated key on the wire keeps its last
    /// value).
    Map(BTreeMap<String, Value>),
}

impl Value {
    /// A map from `(key, value)` pairs.
    pub fn map<K: Into<String>>(entries: impl IntoIterator<Item = (K, Value)>) -> Self {
        Self::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    /// A string value.
    pub fn string(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }

    /// An array of strings.
    pub fn strings<S: AsRef<str>>(items: impl IntoIterator<Item = S>) -> Self {
        Self::Array(
            items
                .into_iter()
                .map(|s| Self::string(s.as_ref()))
                .collect(),
        )
    }

    /// The value of `key` when `self` is a map.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Map(m) => m.get(key),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(b) => Some(b),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }
}

/// Whether a packet asks or answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PacketKind {
    Request,
    Response,
}

/// One protocol packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    /// The request id (a response carries the id of the request it answers).
    pub id: u32,
    pub kind: PacketKind,
    pub value: Value,
}

/// A packet body that cannot be decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    /// The body ends inside a value.
    Truncated,
    /// A value tag outside `0..=6`.
    UnknownTag(u8),
    /// Bytes are left after the packet's value.
    TrailingBytes(usize),
    /// Values nested deeper than the codec accepts.
    TooDeep,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => f.write_str("truncated packet"),
            Self::UnknownTag(t) => write!(f, "unknown value tag {t}"),
            Self::TrailingBytes(n) => write!(f, "{n} trailing bytes after the packet value"),
            Self::TooDeep => write!(f, "values nested deeper than {MAX_DEPTH} levels"),
        }
    }
}

impl std::error::Error for ProtocolError {}

const TAG_NULL: u8 = 0;
const TAG_BOOL: u8 = 1;
const TAG_INT: u8 = 2;
const TAG_STRING: u8 = 3;
const TAG_BYTES: u8 = 4;
const TAG_ARRAY: u8 = 5;
const TAG_MAP: u8 = 6;

/// Splits one length-prefixed frame off `bytes`: `(frame, rest)`, or `None` while the frame is
/// incomplete.
#[must_use]
pub fn split_frame(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let (len, rest) = read_u32(bytes)?;
    let len = usize::try_from(len).ok()?;
    (rest.len() >= len).then(|| rest.split_at(len))
}

/// Encodes a packet, including its length prefix.
///
/// # Panics
/// When a string, byte string, array or map is longer than `u32::MAX` (the wire format cannot
/// express it), or the packet id is `2^31` or larger.
#[must_use]
pub fn encode(packet: &Packet) -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    out.extend_from_slice(&[0; 4]); // the length, patched below
    assert!(packet.id < 1 << 31, "packet id out of range");
    let tagged = match packet.kind {
        PacketKind::Request => packet.id << 1,
        PacketKind::Response => (packet.id << 1) | 1,
    };
    write_u32(&mut out, tagged);
    encode_value(&mut out, &packet.value);
    let len = wire_len(out.len() - 4);
    out[..4].copy_from_slice(&len.to_le_bytes());
    out
}

fn wire_len(n: usize) -> u32 {
    u32::try_from(n).expect("protocol values are limited to u32::MAX bytes or items")
}

fn encode_value(out: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => out.push(TAG_NULL),
        Value::Bool(b) => {
            out.push(TAG_BOOL);
            out.push(u8::from(*b));
        }
        Value::Int(i) => {
            out.push(TAG_INT);
            out.extend_from_slice(&i.to_le_bytes());
        }
        Value::String(s) => {
            out.push(TAG_STRING);
            write_bytes(out, s.as_bytes());
        }
        Value::Bytes(b) => {
            out.push(TAG_BYTES);
            write_bytes(out, b);
        }
        Value::Array(items) => {
            out.push(TAG_ARRAY);
            write_u32(out, wire_len(items.len()));
            for item in items {
                encode_value(out, item);
            }
        }
        Value::Map(entries) => {
            out.push(TAG_MAP);
            write_u32(out, wire_len(entries.len()));
            for (k, v) in entries {
                write_bytes(out, k.as_bytes());
                encode_value(out, v);
            }
        }
    }
}

/// Decodes one packet body (a frame from [`split_frame`], without its length prefix).
///
/// # Errors
/// A truncated body, an unknown tag, trailing bytes or values nested too deeply. Strings that
/// are not UTF-8 (esbuild echoes source text, which may not be) are decoded lossily.
pub fn decode(body: &[u8]) -> Result<Packet, ProtocolError> {
    let (tagged, mut rest) = read_u32(body).ok_or(ProtocolError::Truncated)?;
    let value = decode_value(&mut rest, 0)?;
    if !rest.is_empty() {
        return Err(ProtocolError::TrailingBytes(rest.len()));
    }
    Ok(Packet {
        id: tagged >> 1,
        kind: if tagged & 1 == 0 {
            PacketKind::Request
        } else {
            PacketKind::Response
        },
        value,
    })
}

fn decode_value(bytes: &mut &[u8], depth: usize) -> Result<Value, ProtocolError> {
    if depth > MAX_DEPTH {
        return Err(ProtocolError::TooDeep);
    }
    let tag = take_u8(bytes)?;
    Ok(match tag {
        TAG_NULL => Value::Null,
        TAG_BOOL => Value::Bool(take_u8(bytes)? != 0),
        TAG_INT => Value::Int(i32::from_le_bytes(take_u32(bytes)?.to_le_bytes())),
        TAG_STRING => Value::String(take_string(bytes)?),
        TAG_BYTES => Value::Bytes(take_slice(bytes)?.to_vec()),
        TAG_ARRAY => {
            let count = take_u32(bytes)?;
            // Every item takes at least one byte: never reserve more than the input can hold.
            let mut items = Vec::with_capacity(bounded(count, bytes.len()));
            for _ in 0..count {
                items.push(decode_value(bytes, depth + 1)?);
            }
            Value::Array(items)
        }
        TAG_MAP => {
            let count = take_u32(bytes)?;
            let mut entries = BTreeMap::new();
            for _ in 0..count {
                let key = take_string(bytes)?;
                let item = decode_value(bytes, depth + 1)?;
                entries.insert(key, item);
            }
            Value::Map(entries)
        }
        other => return Err(ProtocolError::UnknownTag(other)),
    })
}

fn bounded(count: u32, available: usize) -> usize {
    usize::try_from(count).map_or(available, |c| c.min(available))
}

fn read_u32(bytes: &[u8]) -> Option<(u32, &[u8])> {
    let (head, rest) = bytes.split_first_chunk::<4>()?;
    Some((u32::from_le_bytes(*head), rest))
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn write_bytes(out: &mut Vec<u8>, b: &[u8]) {
    write_u32(out, wire_len(b.len()));
    out.extend_from_slice(b);
}

fn take_u8(bytes: &mut &[u8]) -> Result<u8, ProtocolError> {
    let (&b, rest) = bytes.split_first().ok_or(ProtocolError::Truncated)?;
    *bytes = rest;
    Ok(b)
}

fn take_u32(bytes: &mut &[u8]) -> Result<u32, ProtocolError> {
    let (v, rest) = read_u32(bytes).ok_or(ProtocolError::Truncated)?;
    *bytes = rest;
    Ok(v)
}

fn take_slice<'a>(bytes: &mut &'a [u8]) -> Result<&'a [u8], ProtocolError> {
    let (slice, rest) = split_frame(bytes).ok_or(ProtocolError::Truncated)?;
    *bytes = rest;
    Ok(slice)
}

fn take_string(bytes: &mut &[u8]) -> Result<String, ProtocolError> {
    Ok(String::from_utf8_lossy(take_slice(bytes)?).into_owned())
}
