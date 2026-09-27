//! Port of `encoding/json/v2_inject.go` (go1.27.1): the translation of v2
//! errors into the v1 error types.

use crate::error::{Error, MarshalerError, UnmarshalTypeError};
use crate::goerr::Err;
use crate::jsontext::state::kind_string;
use crate::v2_scanner::transform_syntactic_error;

// Go: v2_inject.go:transformMarshalError
pub(crate) fn transform_marshal_error(err: Err) -> Error {
    match err {
        Err::Semantic(err) => match err.err {
            None => Error::UnsupportedType {
                type_name: err.go_type.unwrap_or_default(),
            },
            Some(inner) => {
                let mut err_str = inner.to_string();
                if inner == Err::Cycle {
                    if let Some(t) = &err.go_type {
                        err_str = format!("{} via {}", err_str, t);
                    }
                }
                let err_str = err_str
                    .strip_prefix("unsupported value: ")
                    .map(str::to_string)
                    .unwrap_or(err_str);
                Error::UnsupportedValue { str: err_str }
            }
        },
        Err::Marshaler(m) => Error::Marshaler(MarshalerError {
            type_name: m.type_name,
            err: Box::new(transform_syntactic_error(m.err)),
            source_func: m.source_func,
        }),
        other => transform_syntactic_error(other),
    }
}

/// Go `isNumericKind(t)` for a type string.
fn is_numeric_kind(t: &Option<String>) -> bool {
    matches!(
        t.as_deref(),
        Some(
            "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "uintptr"
                | "float32"
                | "float64"
        )
    )
}

// Go: v2_inject.go:transformUnmarshalError
/// `root_name` is `reflect.TypeOf(root).Elem().Name()` of the target
/// (empty for `*any` and `*map[string]any`).
pub(crate) fn transform_unmarshal_error(root_name: &str, err: Err) -> Error {
    match err {
        Err::Semantic(mut err) => {
            if err.err == Some(Err::NilInterface) {
                err.err = None; // non-descriptive for historical reasons
            }

            // Identify the kind of JSON value.
            let mut value = String::new();
            match err.json_kind {
                b'n' | b'"' | b'0' => value = kind_string(err.json_kind),
                b'f' | b't' => value = "bool".to_string(),
                b'[' | b']' => value = "array".to_string(),
                b'{' | b'}' => value = "object".to_string(),
                _ => {}
            }
            if !err.json_value.is_empty() {
                let is_strconv_error =
                    matches!(err.err, Some(Err::StrconvRange) | Some(Err::StrconvSyntax));
                if is_strconv_error && is_numeric_kind(&err.go_type) {
                    value = "number".to_string();
                    if err.json_kind == b'"' {
                        let mut v = Vec::new();
                        let _ = crate::jsonwire::append_unquote(&mut v, &err.json_value);
                        err.json_value = v;
                    }
                    err.err = None;
                }
                value.push(' ');
                value.push_str(&String::from_utf8_lossy(&err.json_value));
            }

            // Identify the root and field path.
            let root = if !err.json_pointer.is_empty() {
                root_name.to_string()
            } else {
                String::new()
            };
            let field_path = String::from_utf8_lossy(&err.json_pointer).into_owned();
            let field_path = field_path
                .strip_prefix('/')
                .unwrap_or(&field_path)
                .replace('/', ".");

            Error::UnmarshalType(UnmarshalTypeError {
                value,
                type_name: err.go_type.clone().unwrap_or_default(),
                offset: err.byte_offset,
                struct_name: root,
                field: field_path,
                err: err.err.map(|e| Box::new(transform_syntactic_error(e))),
            })
        }
        other => transform_syntactic_error(other),
    }
}
