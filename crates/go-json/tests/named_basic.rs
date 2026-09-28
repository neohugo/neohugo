//! Named basic types (Go `type HTML string`, `type Month int`, ...) are
//! hosted as objects whose `Object::underlying` is the basic value; Go
//! marshals them by their Kind, and omitempty/omitzero test that Kind.
//!
//! Expected values: Go 1.27.1 `encoding/json.Marshal` of the same values.

use std::any::Any;
use std::borrow::Cow;

use go_json::{JsonField, JsonStruct};
use go_value::{FloatKind, HostCtx, Kind, Map, MapType, Object, Result, UintKind, Value};

struct Named {
    type_name: &'static str,
    v: Value,
}

impl Object for Named {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.type_name)
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<Result<Value>> {
        None
    }
    fn underlying(&self) -> Option<Value> {
        Some(self.v.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn html(s: &str) -> Value {
    Value::object(Named {
        type_name: "main.HTML",
        v: Value::string(s),
    })
}

fn month(i: i64) -> Value {
    Value::object(Named {
        type_name: "main.Month",
        v: Value::int(i),
    })
}

fn marshal(v: &Value) -> String {
    String::from_utf8(go_json::marshal(v).unwrap()).unwrap()
}

fn s(fields: [(Value, Value, Value, Value, Value); 1]) -> Value {
    let [(a, m, z, n, p)] = fields;
    Value::object(JsonStruct {
        type_name: "main.S".into(),
        fields: vec![
            JsonField::new("a", a).omit_empty(),
            JsonField::new("m", m).omit_empty(),
            JsonField::new("z", z).omit_zero(),
            JsonField::new("n", n).omit_zero(),
            JsonField::new("p", p),
        ],
    })
}

#[test]
fn named_basic_types_marshal_by_kind() {
    let mut m = Map::new(MapType::StringAny);
    m.insert("k", html("v"));
    let cases = [
        (html("<b>&x</b>"), r#""\u003cb\u003e\u0026x\u003c/b\u003e""#),
        (html(""), r#""""#),
        (month(9), "9"),
        (
            Value::object(Named {
                type_name: "main.U",
                v: Value::Uint(200, UintKind::Uint8),
            }),
            "200",
        ),
        (
            Value::object(Named {
                type_name: "main.F",
                v: Value::Float(1.5, FloatKind::F64),
            }),
            "1.5",
        ),
        (
            Value::object(Named {
                type_name: "main.B",
                v: Value::Bool(true),
            }),
            "true",
        ),
        (Value::any_list(vec![html("a"), month(-3)]), r#"["a",-3]"#),
        (Value::map(m), r#"{"k":"v"}"#),
        (
            s([(html(""), month(0), html(""), month(0), html(""))]),
            r#"{"p":""}"#,
        ),
        (
            s([(html("x"), month(2), html("y"), month(3), html("q"))]),
            r#"{"a":"x","m":2,"z":"y","n":3,"p":"q"}"#,
        ),
    ];
    for (v, want) in cases {
        assert_eq!(marshal(&v), want);
    }
}
