//! Host description of Go struct values for the encoder.
//!
//! Go marshals a struct without `MarshalJSON` by reflecting over its
//! fields and `json` tags (`encoding/json/v2/fields.go`). A host type
//! describes the result of that analysis with a [`JsonStruct`]: the fields
//! Go would serialize, in order, with their resolved JSON names and tag
//! options. The encoder then runs Go's `makeStructArshaler` logic on it.

use std::borrow::Cow;

use go_value::{Kind, Object, Value};

/// One struct field: Go's `structField` after `makeStructFields` has applied
/// the `json` tag and the embedding rules.
#[derive(Clone, Debug)]
pub struct JsonField {
    /// The JSON member name: the tag name if the tag has one, else the Go
    /// field name.
    pub name: String,
    pub value: Value,
    /// `omitempty` in the tag (v1 semantics: false, 0, "", nil and empty
    /// collections are omitted).
    pub omit_empty: bool,
    /// `omitzero` in the tag (the type's `IsZero` method, else the Go zero value).
    pub omit_zero: bool,
    /// `string` in the tag: bool, integer, float and string fields are
    /// encoded inside a JSON string.
    pub string: bool,
    /// The field is declared with an interface type (`any`): omitempty and
    /// omitzero only drop a nil interface and `string` is ignored.
    pub interface_typed: bool,
}

impl JsonField {
    /// A field of concrete type without tag options.
    pub fn new(name: impl Into<String>, value: Value) -> JsonField {
        JsonField {
            name: name.into(),
            value,
            omit_empty: false,
            omit_zero: false,
            string: false,
            interface_typed: false,
        }
    }

    pub fn omit_empty(mut self) -> JsonField {
        self.omit_empty = true;
        self
    }

    pub fn omit_zero(mut self) -> JsonField {
        self.omit_zero = true;
        self
    }

    pub fn string(mut self) -> JsonField {
        self.string = true;
        self
    }

    pub fn interface_typed(mut self) -> JsonField {
        self.interface_typed = true;
        self
    }
}

/// A Go struct value without `MarshalJSON`/`MarshalText`, as the list of
/// fields Go would serialize. Wrap it with [`Value::object`].
#[derive(Clone, Debug)]
pub struct JsonStruct {
    /// Go type string, e.g. `"publisher.PublishStats"`.
    pub type_name: String,
    pub fields: Vec<JsonField>,
}

impl JsonStruct {
    pub fn new(type_name: impl Into<String>, fields: Vec<JsonField>) -> JsonStruct {
        JsonStruct {
            type_name: type_name.into(),
            fields,
        }
    }
}

impl Object for JsonStruct {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.type_name)
    }

    fn kind(&self) -> Kind {
        Kind::Struct
    }

    fn has_method(&self, _name: &str) -> bool {
        false
    }

    fn call_method(
        &self,
        _ctx: go_value::HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        None
    }

    /// Looks a field up by its JSON name.
    fn field(&self, name: &str) -> Option<Value> {
        self.fields
            .iter()
            .find(|f| f.name == name)
            .map(|f| f.value.clone())
    }

    /// The fields with their JSON names.
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(
            self.fields
                .iter()
                .map(|f| (Cow::Borrowed(f.name.as_str()), f.value.clone()))
                .collect(),
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
