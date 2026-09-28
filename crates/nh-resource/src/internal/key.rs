//! Port of `resources/internal/key.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).

use go_value::Value;

/// Go: `internal.ResourceTransformationKey` — `Name` + elements hashed with `hashing.HashString`.
#[derive(Clone, Debug)]
pub struct ResourceTransformationKey {
    pub name: String,
    pub(crate) elements: Vec<Value>,
}

impl ResourceTransformationKey {
    // Go: resources/internal/key.go:NewResourceTransformationKey
    pub fn new(name: &str, elements: Vec<Value>) -> Self {
        ResourceTransformationKey {
            name: name.to_string(),
            elements,
        }
    }

    /// Go: `Value()` = Name, or `Name + "_" + hashing.HashString(elements...)`.
    // Go: resources/internal/key.go:Value
    pub fn value(&self) -> String {
        if self.elements.is_empty() {
            return self.name.clone();
        }
        format!(
            "{}_{}",
            self.name,
            nh_common::hashing::hash_string(&self.elements)
        )
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/internal/key.go (42 lines; 2/2 funcs executed)
//   types: ResourceTransformationKey
// OK L30-32: NewResourceTransformationKey(name string, elements ...any) ResourceTransformationKey
// OK L36-42: (k ResourceTransformationKey) Value() string
// ---------------------------------------------------------------------------
