//! Maximum-depth inputs (Go's jsontext limit is 10000 nested values) must
//! not overflow the stack of an ordinary 2 MiB thread in any operation.

use std::sync::Arc;

use go_value::Value;

/// Drops a deeply nested value without recursing (Rust's drop glue would).
fn drop_deep(v: Value) {
    let mut stack = vec![v];
    while let Some(v) = stack.pop() {
        match v {
            Value::List(l) => {
                if let Ok(l) = Arc::try_unwrap(l) {
                    stack.extend(l.items);
                }
            }
            Value::Map(m) => {
                if let Ok(m) = Arc::try_unwrap(m) {
                    stack.extend(m.entries.into_values());
                }
            }
            _ => {}
        }
    }
}

fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn deep_arrays() {
    on_small_stack(|| {
        let depth = 10000;
        let mut src = "[".repeat(depth);
        src.push_str(&"]".repeat(depth));
        assert!(go_json::valid(src.as_bytes()));

        let v = go_json::unmarshal(src.as_bytes()).unwrap();
        let out = go_json::marshal(&v).unwrap();
        assert_eq!(out, src.as_bytes());
        let ind = go_json::marshal_indent(&v, "", " ").unwrap();
        let mut compacted = Vec::new();
        go_json::compact(&mut compacted, &ind).unwrap();
        assert_eq!(compacted, src.as_bytes());
        drop_deep(v);

        // One level deeper is a syntax error, reported without recursing.
        let mut src = "[".repeat(depth + 1);
        src.push_str(&"]".repeat(depth + 1));
        let err = go_json::unmarshal(src.as_bytes()).unwrap_err();
        assert_eq!(err.to_string(), "exceeded max depth");
        assert_eq!(err.offset(), Some(10001));
    });
}

#[test]
fn deep_objects() {
    on_small_stack(|| {
        let depth = 10000;
        let mut src = r#"{"a":"#.repeat(depth - 1);
        src.push_str("{}");
        src.push_str(&"}".repeat(depth - 1));
        let v = go_json::unmarshal(src.as_bytes()).unwrap();
        let out = go_json::marshal(&v).unwrap();
        assert_eq!(out, src.as_bytes());
        let mut dec = go_json::Decoder::new(src.as_bytes());
        let v2 = dec.decode().unwrap();
        drop_deep(v2);
        drop_deep(v);
    });
}
