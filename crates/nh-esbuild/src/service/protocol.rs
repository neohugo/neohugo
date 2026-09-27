//! Module `service::protocol`.
//!
//! NEW: esbuild stdio service packet codec (evanw/esbuild@v0.25.6 cmd/esbuild/service.go + internal/helpers stdio_protocol.go)
//!
//! Owner: Wave B task T16 (js-css-pipeline).


//! esbuild's stdin/stdout service protocol (evanw/esbuild@v0.25.6 `cmd/esbuild/service.go`,
//! `internal/helpers/stdio_protocol.go`): little-endian length-prefixed packets
//! `{id: u32 (bit 0 = request/response), value}` where values are a tagged encoding
//! (null/bool/int/string/bytes/array/map). Byte-exact outputs depend only on esbuild itself; this
//! codec just has to be correct.

/// A protocol value.
#[derive(Clone, Debug, PartialEq)]
pub enum PacketValue {
    Null,
    Bool(bool),
    Int(i32),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<PacketValue>),
    Map(Vec<(String, PacketValue)>),
}

/// A protocol packet.
#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    pub id: u32,
    pub is_request: bool,
    pub value: PacketValue,
}

pub fn encode_packet(p: &Packet) -> Vec<u8> {
    todo!()
}

pub fn decode_packet(bytes: &[u8]) -> Option<(Packet, usize)> {
    todo!()
}
