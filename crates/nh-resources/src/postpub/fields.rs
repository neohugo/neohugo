//! Port of `resources/postpub/fields.go`.
//!
//! Owner: Wave B task T14 (resources-core).

//! Go `postpub/fields.go`: struct -> map with placeholders (MediaType/Data fields). Go reflects
//! on the struct; the port describes the one struct it is used with (`media.Type`) in a table.

use go_value::{GoString, Map, MapType, Value};

pub const FIELD_NOT_SUPPORTED: &str = "__field_not_supported";

/// The reflect view of a Go struct type that `structToMap` needs: its exported methods of the
/// value method set that take no arguments (`NumIn() == 1`, the receiver) and its exported
/// fields.
pub struct StructInfo {
    pub methods: &'static [&'static str],
    pub fields: &'static [&'static str],
}

/// `reflect.TypeOf(media.Type{})`: the value-receiver methods without arguments
/// (`HasSuffix` takes one; `init` is unexported) and the exported fields (`mimeSuffix` is not).
pub const MEDIA_TYPE_STRUCT: StructInfo = StructInfo {
    methods: &[
        "IsHTML",
        "IsMarkdown",
        "IsText",
        "IsZero",
        "MarshalJSON",
        "String",
        "Suffixes",
    ],
    fields: &[
        "Type",
        "MainType",
        "SubType",
        "Delimiter",
        "FirstSuffix",
        "SuffixesCSV",
    ],
};

// Go: resources/postpub/fields.go:structToMapWithPlaceholders
pub fn struct_to_map_with_placeholders(
    root: &str,
    input: &StructInfo,
    create_placeholder: &dyn Fn(&str) -> String,
) -> Map {
    let mut m = struct_to_map(input);
    insert_field_placeholders(root, &mut m, create_placeholder);
    m
}

// Go: resources/postpub/fields.go:structToMap
pub fn struct_to_map(s: &StructInfo) -> Map {
    let mut m = Map::new(MapType::StringAny);

    for method in s.methods {
        m.entries.insert(GoString::from(*method), Value::string(""));
    }

    for field in s.fields {
        m.entries.insert(GoString::from(*field), Value::string(""));
    }
    m
}

/// insert placeholder for the templates. Do it very shallow for now.
// Go: resources/postpub/fields.go:insertFieldPlaceholders
pub fn insert_field_placeholders(
    root: &str,
    m: &mut Map,
    create_placeholder: &dyn Fn(&str) -> String,
) {
    for (k, v) in m.entries.iter_mut() {
        *v = Value::string(create_placeholder(&format!("{root}.{}", k.to_str_lossy())));
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/postpub/fields.go (59 lines; 0/3 funcs executed)
// OK L24-28: structToMapWithPlaceholders(root string, in any, createPlaceholder func(s string) string) map[string]any
// OK L30-52: structToMap(s any) map[string]any
// OK L55-59: insertFieldPlaceholders(root string, m map[string]any, createPlaceholder func(s string) string)
// ---------------------------------------------------------------------------
