//! Module `goi18n::plural`.
//!
//! PORT internal/plural/{form,operands,rule,rules,rule_gen}.go
//!
//! Owner: Wave B task T17 (i18n).

//! go-i18n `internal/plural`: CLDR plural forms, operands and generated rules (at least en, th).
//! `rule_gen.rs` is generated from the Go `rule_gen.go` by `tools/go-oracle/nh-i18n/gentables`.

#[rustfmt::skip]
pub mod rule_gen;

pub mod form {
    /// Go: `plural.Form` (a string type; `Invalid` is `""`).
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub enum Form {
        Invalid,
        Other,
        Zero,
        One,
        Two,
        Few,
        Many,
    }

    impl Form {
        /// The Go string value of the form.
        pub fn as_str(self) -> &'static str {
            match self {
                Form::Invalid => "",
                Form::Zero => "zero",
                Form::One => "one",
                Form::Two => "two",
                Form::Few => "few",
                Form::Many => "many",
                Form::Other => "other",
            }
        }
    }
}

pub mod operands {
    use go_value::{IntKind, Value};

    /// Go: `plural.Operands` (n, i, v, w, f, t), see
    /// <http://unicode.org/reports/tr35/tr35-numbers.html#Operands>.
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Operands {
        /// absolute value of the source number (integer and decimals)
        pub n: f64,
        /// integer digits of n
        pub i: i64,
        /// number of visible fraction digits in n, with trailing zeros
        pub v: i64,
        /// number of visible fraction digits in n, without trailing zeros
        pub w: i64,
        /// visible fractional digits in n, with trailing zeros
        pub f: i64,
        /// visible fractional digits in n, without trailing zeros
        pub t: i64,
    }

    impl Operands {
        /// Go: `NEqualsAny` — o represents an integer equal to any of the arguments.
        // Go: internal/plural/operands.go:NEqualsAny
        pub fn n_equals_any(&self, any: &[i64]) -> bool {
            for &i in any {
                if self.i == i && self.t == 0 {
                    return true;
                }
            }
            false
        }

        /// Go: `NModEqualsAny` — o represents an integer equal to any of the arguments modulo
        /// mod.
        // Go: internal/plural/operands.go:NModEqualsAny
        pub fn n_mod_equals_any(&self, m: i64, any: &[i64]) -> bool {
            let mod_i = self.i % m;
            for &i in any {
                if mod_i == i && self.t == 0 {
                    return true;
                }
            }
            false
        }

        /// Go: `NInRange` — o represents an integer in the closed interval [from, to].
        // Go: internal/plural/operands.go:NInRange
        pub fn n_in_range(&self, from: i64, to: i64) -> bool {
            self.t == 0 && from <= self.i && self.i <= to
        }

        /// Go: `NModInRange` — o represents an integer in [from, to] modulo mod.
        // Go: internal/plural/operands.go:NModInRange
        pub fn n_mod_in_range(&self, m: i64, from: i64, to: i64) -> bool {
            let mod_i = self.i % m;
            self.t == 0 && from <= mod_i && mod_i <= to
        }
    }

    /// Why `NewOperands` failed: an error, or a Go runtime panic (`newOperandsString("")`
    /// indexes `s[0]`).
    #[derive(Clone, Debug, PartialEq)]
    pub enum OperandsError {
        Err(String),
        Panic(String),
    }

    /// Go: `plural.NewOperands(v)` — Go `int`, `int8`…`int64` and `string` (exactly those
    /// types; floats must be formatted into a string first).
    // Go: internal/plural/operands.go:NewOperands
    pub fn new_operands_e(number: &Value) -> Result<Operands, OperandsError> {
        match number {
            Value::Int(i, _) => Ok(new_operands_int64(*i)),
            Value::String(s) => new_operands_string(s.as_bytes()),
            Value::Float(..) => Err(OperandsError::Err(
                "floats should be formatted into a string".to_string(),
            )),
            other => Err(OperandsError::Err(format!(
                "invalid type {}; expected integer or string",
                go_type(other)
            ))),
        }
    }

    /// Go: `plural.NewOperands(v)` (a Go panic becomes an error).
    pub fn new_operands(v: &go_value::Value) -> nh_common::Result<Operands> {
        new_operands_e(v).map_err(|e| match e {
            OperandsError::Err(s) | OperandsError::Panic(s) => nh_common::herrors::Error::new(s),
        })
    }

    /// Go `%T` of a value (`<nil>` for an untyped nil).
    pub(crate) fn go_type(v: &Value) -> String {
        match v {
            Value::Invalid => "<nil>".to_string(),
            Value::Int(_, k) if *k == IntKind::Int => "int".to_string(),
            _ => v.go_type_name().into_owned(),
        }
    }

    // Go: internal/plural/operands.go:newOperandsInt64
    fn new_operands_int64(mut i: i64) -> Operands {
        if i < 0 {
            i = i.wrapping_neg();
        }
        Operands {
            n: i as f64,
            i,
            v: 0,
            w: 0,
            f: 0,
            t: 0,
        }
    }

    // Go: internal/plural/operands.go:newOperandsString
    fn new_operands_string(mut s: &[u8]) -> Result<Operands, OperandsError> {
        if s.is_empty() {
            return Err(OperandsError::Panic(
                "runtime error: index out of range [0] with length 0".to_string(),
            ));
        }
        if s[0] == b'-' {
            s = &s[1..];
        }
        let n = go_strconv::parse_float(s, 64).map_err(|e| OperandsError::Err(e.to_string()))?;
        let mut ops = Operands {
            n,
            ..Default::default()
        };
        let (int_part, fraction) = match s.iter().position(|&b| b == b'.') {
            Some(p) => (&s[..p], Some(&s[p + 1..])),
            None => (s, None),
        };
        ops.i = go_strconv::parse_int(int_part, 10, 64)
            .map_err(|e| OperandsError::Err(e.to_string()))?;
        let Some(fraction) = fraction else {
            return Ok(ops);
        };
        ops.v = fraction.len() as i64;
        let mut i = ops.v - 1;
        while i >= 0 {
            if fraction[i as usize] != b'0' {
                ops.w = i + 1;
                break;
            }
            i -= 1;
        }
        if ops.v > 0 {
            let f = go_strconv::parse_int(fraction, 10, 0)
                .map_err(|e| OperandsError::Err(e.to_string()))?;
            ops.f = f;
        }
        if ops.w > 0 {
            let t = go_strconv::parse_int(&fraction[..ops.w as usize], 10, 0)
                .map_err(|e| OperandsError::Err(e.to_string()))?;
            ops.t = t;
        }
        Ok(ops)
    }
}

pub mod rule {
    use super::form::Form;
    use super::operands::Operands;
    use super::rules::Rules;

    /// Go: `plural.Rule` — the CLDR plural rules for a language.
    #[derive(Clone, Debug)]
    pub struct Rule {
        /// Go `PluralForms map[Form]struct{}` (a set; kept in the generated order).
        pub plural_forms: Vec<Form>,
        pub plural_form_func: fn(&Operands) -> Form,
    }

    // Go: internal/plural/rule.go:addPluralRules
    pub(crate) fn add_plural_rules(rules: &mut Rules, ids: &[&str], ps: Rule) {
        for &id in ids {
            if id == "root" {
                continue;
            }
            let tag = xtext_collate::language::DEFAULT.must_parse(id);
            rules.rules.insert(tag, ps.clone());
        }
    }

    // Go: internal/plural/rule.go:newPluralFormSet
    pub(crate) fn new_plural_form_set(plural_forms: &[Form]) -> Vec<Form> {
        let mut set: Vec<Form> = Vec::with_capacity(plural_forms.len());
        for &p in plural_forms {
            if !set.contains(&p) {
                set.push(p);
            }
        }
        set
    }

    // Go: internal/plural/rule.go:intInRange
    pub(crate) fn int_in_range(i: i64, from: i64, to: i64) -> bool {
        from <= i && i <= to
    }

    // Go: internal/plural/rule.go:intEqualsAny
    pub(crate) fn int_equals_any(i: i64, any: &[i64]) -> bool {
        for &a in any {
            if i == a {
                return true;
            }
        }
        false
    }
}

pub mod rules {
    use std::collections::HashMap;

    use xtext_collate::language::Tag;

    pub use super::rule::Rule;

    /// Go: `plural.Rules` (tag -> rule; `Rule(tag)` falls back to the CLDR parents, then to the
    /// base language). Keys are Go `language.Tag`s (Go map; only looked up, never iterated).
    #[derive(Clone, Default)]
    pub struct Rules {
        pub rules: HashMap<Tag, Rule>,
    }

    impl Rules {
        /// Go: `plural.DefaultRules()` (generated from CLDR; en: one iff i==1&&v==0; th: always
        /// other).
        // Go: internal/plural/rule_gen.go:DefaultRules
        pub fn default_rules() -> Rules {
            super::rule_gen::default_rules()
        }

        /// Go: `Rules.Rule(tag)` — the closest matching plural rule for the language tag, or
        /// `None`.
        // Go: internal/plural/rules.go:Rule
        pub fn rule(&self, tag: &Tag) -> Option<&Rule> {
            let mut t = tag.clone();
            loop {
                if let Some(rule) = self.rules.get(&t) {
                    return Some(rule);
                }
                t = t.parent();
                if t.is_root() {
                    break;
                }
            }
            let (base, _) = tag.base();
            // Go: `baseTag, _ := language.Parse(base.String())` (the partial tag on error).
            let base_tag = match xtext_collate::language::DEFAULT.parse(&base.to_string()) {
                Ok(t) => t,
                Err((t, _)) => t,
            };
            self.rules.get(&base_tag)
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (go-i18n internal/plural)
// OK form.go Form constants
// OK operands.go NEqualsAny, NModEqualsAny, NInRange, NModInRange
// OK operands.go NewOperands
// OK operands.go newOperandsInt64
// OK operands.go newOperandsString (the Go panic on "" is returned as OperandsError::Panic)
// OK rule.go addPluralRules
// OK rule.go newPluralFormSet
// OK rule.go intInRange
// OK rule.go intEqualsAny
// OK rules.go Rules.Rule
// OK rule_gen.go DefaultRules (generated: tools/go-oracle/nh-i18n/gentables -> rule_gen.rs)
// ---------------------------------------------------------------------------
