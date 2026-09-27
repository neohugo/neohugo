//! Go: $GOROOT/src/reflect/type.go `StructTag.Get` / `StructTag.Lookup`.

// Go: reflect/type.go:StructTag.Get
pub(crate) fn struct_tag_get(tag: &[u8], key: &[u8]) -> Vec<u8> {
    struct_tag_lookup(tag, key).unwrap_or_default()
}

// Go: reflect/type.go:StructTag.Lookup
pub(crate) fn struct_tag_lookup(tag: &[u8], key: &[u8]) -> Option<Vec<u8>> {
    let mut tag = tag;
    while !tag.is_empty() {
        // Skip leading space.
        let mut i = 0;
        while i < tag.len() && tag[i] == b' ' {
            i += 1;
        }
        tag = &tag[i..];
        if tag.is_empty() {
            break;
        }

        // Scan to colon. A space, a quote or a control character is a syntax error.
        i = 0;
        while i < tag.len() && tag[i] > b' ' && tag[i] != b':' && tag[i] != b'"' && tag[i] != 0x7f {
            i += 1;
        }
        if i == 0 || i + 1 >= tag.len() || tag[i] != b':' || tag[i + 1] != b'"' {
            break;
        }
        let name = &tag[..i];
        tag = &tag[i + 1..];

        // Scan quoted string to find value.
        i = 1;
        while i < tag.len() && tag[i] != b'"' {
            if tag[i] == b'\\' {
                i += 1;
            }
            i += 1;
        }
        if i >= tag.len() {
            break;
        }
        let qvalue = &tag[..i + 1];
        tag = &tag[i + 1..];

        if key == name {
            return go_strconv::unquote(qvalue).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup() {
        assert_eq!(struct_tag_get(br#"hash:"ignore""#, b"hash"), b"ignore");
        assert_eq!(struct_tag_get(br#"json:"a" hash:"set""#, b"hash"), b"set");
        assert_eq!(struct_tag_get(br#"json:"a""#, b"hash"), b"");
        assert_eq!(struct_tag_get(br#"hash:"x\"y""#, b"hash"), b"x\"y");
        assert_eq!(struct_tag_get(br#"hash"#, b"hash"), b"");
    }
}
