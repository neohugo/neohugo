use std::collections::BTreeMap;

use neohugo_esbuild::service::protocol::{
    Packet, PacketKind, ProtocolError, Value, decode, encode, split_frame,
};

fn request(id: u32, value: Value) -> Packet {
    Packet {
        id,
        kind: PacketKind::Request,
        value,
    }
}

fn roundtrip(p: &Packet) {
    let bytes = encode(p);
    let (body, rest) = split_frame(&bytes).expect("a complete frame");
    assert!(rest.is_empty());
    assert_eq!(&decode(body).expect("decodes"), p);
}

#[test]
fn values_roundtrip() {
    for v in [
        Value::Null,
        Value::Bool(true),
        Value::Bool(false),
        Value::Int(-1),
        Value::Int(i32::MAX),
        Value::string("héllo"),
        Value::Bytes(vec![0, 255, 3]),
        Value::Array(vec![
            Value::Int(1),
            Value::string("x"),
            Value::Array(vec![]),
        ]),
        Value::map([("a", Value::Null), ("b", Value::strings(["x", "y"]))]),
    ] {
        roundtrip(&request(7, v.clone()));
        roundtrip(&Packet {
            id: 3,
            kind: PacketKind::Response,
            value: v,
        });
    }
}

#[test]
fn encoding_is_the_wire_format() {
    // {"b": 1, "a": true} as request 5: keys sorted, id shifted, bit 0 clear.
    let p = request(
        5,
        Value::map([("b", Value::Int(1)), ("a", Value::Bool(true))]),
    );
    let mut want = vec![];
    let body: Vec<u8> = [
        &10u32.to_le_bytes()[..], // 5 << 1
        &[6],
        &2u32.to_le_bytes(),
        &1u32.to_le_bytes(),
        b"a",
        &[1, 1],
        &1u32.to_le_bytes(),
        b"b",
        &[2],
        &1u32.to_le_bytes(),
    ]
    .concat();
    want.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
    want.extend_from_slice(&body);
    assert_eq!(encode(&p), want);

    // A response sets bit 0; -1 is 0xFFFFFFFF.
    let r = encode(&Packet {
        id: 5,
        kind: PacketKind::Response,
        value: Value::Int(-1),
    });
    assert_eq!(&r[4..8], &11u32.to_le_bytes());
    assert_eq!(&r[8..], &[2, 0xFF, 0xFF, 0xFF, 0xFF]);
}

#[test]
fn frames_split_only_when_complete() {
    let bytes = encode(&request(1, Value::string("abc")));
    for cut in 0..bytes.len() {
        assert!(split_frame(&bytes[..cut]).is_none(), "cut at {cut}");
    }
    let mut two = bytes.clone();
    two.extend_from_slice(&bytes);
    let (_, rest) = split_frame(&two).unwrap();
    assert_eq!(rest, &bytes[..]);
}

#[test]
fn malformed_bodies_are_errors() {
    let body = |value: &[u8]| [&0u32.to_le_bytes()[..], value].concat();
    assert_eq!(decode(&[1, 0]), Err(ProtocolError::Truncated));
    assert_eq!(decode(&body(&[])), Err(ProtocolError::Truncated));
    assert_eq!(decode(&body(&[1])), Err(ProtocolError::Truncated));
    assert_eq!(
        decode(&body(&[3, 5, 0, 0, 0, b'a'])),
        Err(ProtocolError::Truncated)
    );
    assert_eq!(
        decode(&body(&[5, 2, 0, 0, 0, 0])),
        Err(ProtocolError::Truncated)
    );
    assert_eq!(decode(&body(&[9])), Err(ProtocolError::UnknownTag(9)));
    assert_eq!(
        decode(&body(&[0, 0, 0])),
        Err(ProtocolError::TrailingBytes(2))
    );
    // A huge array count on a short input neither allocates nor panics.
    assert_eq!(
        decode(&body(&[5, 0xFF, 0xFF, 0xFF, 0xFF])),
        Err(ProtocolError::Truncated)
    );
    // Deep nesting is refused.
    let mut deep = [5u8, 1, 0, 0, 0].repeat(300);
    deep.push(0);
    assert_eq!(decode(&body(&deep)), Err(ProtocolError::TooDeep));
}

#[test]
fn a_repeated_key_keeps_its_last_value() {
    let entry = |v: i32| [&1u32.to_le_bytes()[..], b"k", &[2], &v.to_le_bytes()].concat();
    let value = [&[6][..], &2u32.to_le_bytes(), &entry(1), &entry(2)].concat();
    let p = decode(&[&0u32.to_le_bytes()[..], &value].concat()).unwrap();
    assert_eq!(
        p.value,
        Value::Map(BTreeMap::from([("k".to_owned(), Value::Int(2))]))
    );
}

#[test]
fn invalid_utf8_strings_are_decoded_lossily() {
    let value = [&[3][..], &2u32.to_le_bytes(), &[b'a', 0xFF]].concat();
    let p = decode(&[&0u32.to_le_bytes()[..], &value].concat()).unwrap();
    assert_eq!(p.value, Value::string("a\u{FFFD}"));
}
