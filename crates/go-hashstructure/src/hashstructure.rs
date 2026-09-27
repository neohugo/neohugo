//! Go: github.com/gohugoio/hashstructure@v0.5.0 hashstructure.go.

use go_time::GoTimeExt;
use go_value::{FloatKind, IntKind, UintKind};

use crate::Error;
use crate::hasher::{Fnv64, Hasher64};
use crate::structtag::struct_tag_get;
use crate::value::{
    GoMap, GoStruct, HashValue, Receiver, UnsupportedKind, from_value, is_zero, natural_stringer,
};

/// Go: `HashOptions`.
#[derive(Default)]
pub struct HashOptions {
    /// The hash function to use. If this isn't set, it will default to FNV
    /// (Go's `hash/fnv.New64`). neohugo uses xxhash ([`crate::XxHash64`]).
    pub hasher: Option<Box<dyn Hasher64>>,
    /// The struct tag to look at when hashing the structure ("hash" if empty).
    pub tag_name: String,
    /// Treat a nil pointer as the zero value of the pointed type.
    pub zero_nil: bool,
    /// Ignore zero value fields.
    pub ignore_zero_value: bool,
    /// Assume that a `set` tag is always present for slices.
    pub slices_as_sets: bool,
    /// Always use `fmt.Stringer` when a field implements it.
    pub use_stringer: bool,
}

// Go: hashstructure.go:Hash
//
// Hash returns the hash value of an arbitrary value. If opts is None, the
// default options are used (FNV-64, tag "hash"). Like Go, missing defaults
// are filled into the given options.
pub fn hash(v: &HashValue, opts: Option<&mut HashOptions>) -> Result<u64, Error> {
    // Create default options
    let mut default_opts = HashOptions::default();
    let opts = match opts {
        Some(o) => o,
        None => &mut default_opts,
    };
    if opts.hasher.is_none() {
        opts.hasher = Some(Box::new(Fnv64::new()));
    }
    if opts.tag_name.is_empty() {
        opts.tag_name = "hash".to_string();
    }

    let HashOptions {
        hasher,
        tag_name,
        zero_nil,
        ignore_zero_value,
        slices_as_sets,
        use_stringer,
    } = opts;
    let h: &mut dyn Hasher64 = hasher.as_deref_mut().expect("hasher set above");

    // Reset the hash
    h.reset();

    // Fast path for strings.
    if let Some(s) = plain_string(v) {
        return Ok(hash_string(h, s));
    }

    // Create our walker and walk the structure
    let mut w = Walker {
        h,
        tag: tag_name,
        zeronil: *zero_nil,
        ignorezerovalue: *ignore_zero_value,
        sets: *slices_as_sets,
        stringer: *use_stringer,
    };
    w.visit(v, false, None, None)
}

/// Go: `if s, ok := v.(string)` — only the plain `string` type.
fn plain_string(v: &HashValue) -> Option<&[u8]> {
    match v {
        HashValue::String(s) => Some(s.as_bytes()),
        HashValue::Value(go_value::Value::String(s)) => Some(s.as_bytes()),
        _ => None,
    }
}

struct Walker<'a> {
    h: &'a mut dyn Hasher64,
    tag: &'a str,
    zeronil: bool,
    ignorezerovalue: bool,
    sets: bool,
    stringer: bool,
}

/// Go: `visitOpts`.
struct VisitOpts<'a> {
    /// Flags are a bitmask of flags to affect behavior of this visit
    flags: u32,
    /// Information about the struct containing this field
    parent: &'a GoStruct,
    struct_field: &'a str,
}

// Go: visitFlag values
const VISIT_FLAG_SET: u32 = 1 << 1;

// Go: hashstructure.go:hashString
fn hash_string(h: &mut dyn Hasher64, s: &[u8]) -> u64 {
    h.reset();
    h.write(s);
    h.sum64()
}

// Go: hashstructure.go:hashUpdateOrdered
fn hash_update_ordered(h: &mut dyn Hasher64, a: u64, b: u64) -> u64 {
    // For ordered updates, use a real hash function
    h.reset();
    h.write(&a.to_le_bytes());
    h.write(&b.to_le_bytes());
    h.sum64()
}

// Go: hashstructure.go:hashUpdateUnordered
fn hash_update_unordered(a: u64, b: u64) -> u64 {
    a ^ b
}

// Go: hashstructure.go:hashFinishUnordered
//
// After mixing a group of unique hashes with hashUpdateUnordered, it's always
// necessary to call hashFinishUnordered.
fn hash_finish_unordered(h: &mut dyn Hasher64, a: u64) -> u64 {
    h.reset();
    h.write(&a.to_le_bytes());
    h.sum64()
}

impl Walker<'_> {
    // Go: hashstructure.go:(*walker).hashDirect — binary.Write(LittleEndian)
    // of a fixed-size value.
    fn hash_direct(&mut self, b: &[u8]) -> u64 {
        self.h.reset();
        self.h.write(b);
        self.h.sum64()
    }

    // Go: hashstructure.go:(*walker).visit — the pointer/interface
    // dereferencing loop. `addr` is whether `v` is addressable (Go:
    // `CanAddr`/`CanSet`), `zero` the zero value of the last pointee type
    // seen with ZeroNil (Go's `t`, `None` = `reflect.TypeOf(0)`).
    fn visit(
        &mut self,
        v: &HashValue,
        addr: bool,
        opts: Option<&VisitOpts<'_>>,
        zero: Option<&HashValue>,
    ) -> Result<u64, Error> {
        match v {
            HashValue::Value(val) => {
                let hv = from_value(val);
                self.visit(&hv, addr, opts, zero)
            }
            // If we have an interface, dereference it.
            HashValue::Interface(inner) => self.visit(inner, false, opts, zero),
            HashValue::Ptr(elem, elem_zero) => {
                let zero = if self.zeronil {
                    elem_zero.as_deref()
                } else {
                    zero
                };
                match elem {
                    Some(e) => self.visit(e, true, opts, zero),
                    // reflect.Indirect(nil) is invalid.
                    None => self.visit_zero(opts, zero),
                }
            }
            // If it is nil, treat it like a zero.
            HashValue::Nil => self.visit_zero(opts, zero),
            _ => self.visit_value(v, addr, opts),
        }
    }

    // Go: `if !v.IsValid() { v = reflect.Zero(t) }`
    fn visit_zero(
        &mut self,
        opts: Option<&VisitOpts<'_>>,
        zero: Option<&HashValue>,
    ) -> Result<u64, Error> {
        match zero {
            None => self.visit_value(&HashValue::Int(0, IntKind::Int), false, opts),
            Some(z) => self.visit_value(z, false, opts),
        }
    }

    // Go: hashstructure.go:(*walker).visit, after dereferencing.
    fn visit_value(
        &mut self,
        v: &HashValue,
        addr: bool,
        opts: Option<&VisitOpts<'_>>,
    ) -> Result<u64, Error> {
        match v {
            HashValue::Int(i, k) => {
                // binary.Write of the fixed-size value (Go `int` is written
                // as int64).
                let h = match k {
                    IntKind::Int | IntKind::Int64 => self.hash_direct(&i.to_le_bytes()),
                    IntKind::Int8 => self.hash_direct(&(*i as i8).to_le_bytes()),
                    IntKind::Int16 => self.hash_direct(&(*i as i16).to_le_bytes()),
                    IntKind::Int32 => self.hash_direct(&(*i as i32).to_le_bytes()),
                };
                Ok(h)
            }
            HashValue::Uint(u, k) => {
                let h = match k {
                    UintKind::Uint | UintKind::Uint64 => self.hash_direct(&u.to_le_bytes()),
                    UintKind::Uint8 => self.hash_direct(&(*u as u8).to_le_bytes()),
                    UintKind::Uint16 => self.hash_direct(&(*u as u16).to_le_bytes()),
                    UintKind::Uint32 => self.hash_direct(&(*u as u32).to_le_bytes()),
                    UintKind::Uintptr => {
                        // binary.Write rejects uintptr (not fixed-size).
                        self.h.reset();
                        return Err(Error::BinaryWrite("uintptr".to_string()));
                    }
                };
                Ok(h)
            }
            HashValue::Float(f, FloatKind::F32) => {
                Ok(self.hash_direct(&(*f as f32).to_bits().to_le_bytes()))
            }
            HashValue::Float(f, FloatKind::F64) => Ok(self.hash_direct(&f.to_bits().to_le_bytes())),
            HashValue::Complex64(re, im) => {
                let mut b = [0u8; 8];
                b[..4].copy_from_slice(&re.to_bits().to_le_bytes());
                b[4..].copy_from_slice(&im.to_bits().to_le_bytes());
                Ok(self.hash_direct(&b))
            }
            HashValue::Complex128(re, im) => {
                let mut b = [0u8; 16];
                b[..8].copy_from_slice(&re.to_bits().to_le_bytes());
                b[8..].copy_from_slice(&im.to_bits().to_le_bytes());
                Ok(self.hash_direct(&b))
            }
            HashValue::Bool(b) => {
                let tmp: i8 = if *b { 1 } else { 0 };
                Ok(self.hash_direct(&tmp.to_le_bytes()))
            }
            HashValue::Time(t) => {
                self.h.reset();
                let b = t.marshal_binary().map_err(|e| Error::Time(e.to_string()))?;
                self.h.write(&b);
                Ok(self.h.sum64())
            }
            HashValue::Array(items) => {
                let mut h: u64 = 0;
                for item in items {
                    let current = self.visit(item, addr, None, None)?;
                    h = hash_update_ordered(self.h, h, current);
                }
                Ok(h)
            }
            HashValue::Map(m) => self.visit_map(m, opts),
            HashValue::Struct(s) => self.visit_struct(s, addr),
            HashValue::Slice(items) => {
                // We have two behaviors here. If it isn't a set, then we just
                // visit all the elements. If it is a set, then we do a deterministic
                // hash code.
                let mut h: u64 = 0;
                let set = opts.is_some_and(|o| o.flags & VISIT_FLAG_SET != 0);
                for item in items.iter().flatten() {
                    let current = self.visit(item, true, None, None)?;
                    if set || self.sets {
                        h = hash_update_unordered(h, current);
                    } else {
                        h = hash_update_ordered(self.h, h, current);
                    }
                }

                if set {
                    // Important: read the docs for hashFinishUnordered
                    h = hash_finish_unordered(self.h, h);
                }

                Ok(h)
            }
            HashValue::String(s) => Ok(hash_string(self.h, s.as_bytes())),
            HashValue::Unsupported { kind, .. } => Err(Error::UnknownKind(kind.go_name())),
            // Zero values of pointer/interface types (ZeroNil only).
            HashValue::Ptr(..) => Err(Error::UnknownKind(UnsupportedKind::Ptr.go_name())),
            HashValue::Interface(_) => {
                Err(Error::UnknownKind(UnsupportedKind::Interface.go_name()))
            }
            HashValue::Nil => self.visit_value(&HashValue::Int(0, IntKind::Int), false, opts),
            HashValue::Value(val) => {
                let hv = from_value(val);
                self.visit_value(&hv, addr, opts)
            }
        }
    }

    // Go: hashstructure.go:(*walker).visit, case reflect.Map
    fn visit_map(&mut self, m: &GoMap, opts: Option<&VisitOpts<'_>>) -> Result<u64, Error> {
        let mut include_map = None;
        let mut field = "";

        if let Some(im) = &m.include_map {
            include_map = Some(im.clone());
        } else if let Some(o) = opts
            && let Some(im) = &o.parent.include_map
        {
            include_map = Some(im.clone());
            field = o.struct_field;
        }

        // Build the hash for the map. We do this by XOR-ing all the key
        // and value hashes. This makes it deterministic despite ordering.
        let mut h: u64 = 0;

        for (k, vv) in m.entries.iter().flatten() {
            if let Some(im) = &include_map {
                let incl = im.hash_include_map(field, k, vv)?;
                if !incl {
                    continue;
                }
            }

            // k and vv are addressable copies (reflect.New(...).Elem()).
            let kh = self.visit(k, true, None, None)?;
            let vh = self.visit(vv, true, None, None)?;

            let field_hash = hash_update_ordered(self.h, kh, vh);
            h = hash_update_unordered(h, field_hash);
        }

        // Important: read the docs for hashFinishUnordered
        h = hash_finish_unordered(self.h, h);

        Ok(h)
    }

    // Go: hashstructure.go:(*walker).visit, case reflect.Struct
    fn visit_struct(&mut self, s: &GoStruct, addr: bool) -> Result<u64, Error> {
        let mut include = None;
        if let Some((Receiver::Value, i)) = &s.includable {
            include = Some(i.clone());
        }

        if let Some((Receiver::Value, hh)) = &s.hashable {
            return hh.hash();
        }

        // If we can address this value, check if the pointer value
        // implements our interfaces and use that if so.
        if addr {
            if let Some((_, i)) = &s.includable {
                include = Some(i.clone());
            }
            if let Some((_, hh)) = &s.hashable {
                return hh.hash();
            }
        }

        let mut h = hash_string(self.h, s.name.as_bytes());

        for f in &s.fields {
            // Go: `if innerV := v.Field(i); v.CanSet() || t.Field(i).Name != "_"`
            if addr || f.name != "_" {
                if !f.exported {
                    // Unexported
                    continue;
                }

                let tag = struct_tag_get(f.tag.as_bytes(), self.tag.as_bytes());
                if tag == b"ignore" || tag == b"-" {
                    // Ignore this field
                    continue;
                }

                if self.ignorezerovalue && is_zero(&f.value) {
                    continue;
                }

                // if string is set, use the string value
                let mut inner_owned: Option<HashValue> = None;
                if tag == b"string" || self.stringer {
                    let natural = if f.stringer.is_none() {
                        natural_stringer(&f.value)
                    } else {
                        None
                    };
                    if let Some(sv) = &f.stringer {
                        inner_owned = Some(HashValue::String(sv.clone()));
                    } else if let Some(r) = natural {
                        inner_owned = Some(HashValue::String(r?));
                    } else if tag == b"string" {
                        // We only show this error if the tag explicitly
                        // requests a stringer.
                        return Err(Error::NotStringer {
                            field: f.name.clone(),
                        });
                    }
                }
                let inner = inner_owned.as_ref().unwrap_or(&f.value);
                let inner_addr = addr && inner_owned.is_none();

                // Check if we implement includable and check it
                if let Some(i) = &include {
                    let incl = i.hash_include(&f.name, inner)?;
                    if !incl {
                        continue;
                    }
                }

                let flags = if tag == b"set" { VISIT_FLAG_SET } else { 0 };

                let kh = hash_string(self.h, f.name.as_bytes());

                let vopts = VisitOpts {
                    flags,
                    parent: s,
                    struct_field: &f.name,
                };
                let vh = self.visit(inner, inner_addr, Some(&vopts), None)?;

                let field_hash = hash_update_ordered(self.h, kh, vh);
                h = hash_update_unordered(h, field_hash);
            }
            // Important: read the docs for hashFinishUnordered
            h = hash_finish_unordered(self.h, h);
        }

        Ok(h)
    }
}
