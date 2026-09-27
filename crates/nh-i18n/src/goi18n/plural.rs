//! Module `goi18n::plural`.
//!
//! PORT internal/plural/{form,operands,rule,rules,rule_gen}.go
//!
//! Owner: Wave B task T17 (i18n).


//! go-i18n `internal/plural`: CLDR plural forms, operands and generated rules (at least en, th).

pub mod form {
    /// Go: `plural.Form`.
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
}

pub mod operands {
    /// Go: `plural.Operands` (n, i, v, w, f, t).
    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    pub struct Operands {
        pub n: f64,
        pub i: i64,
        pub v: i64,
        pub w: i64,
        pub f: i64,
        pub t: i64,
    }

    /// Go: `plural.NewOperands(v)` (int, string, float formatted...).
    pub fn new_operands(v: &go_value::Value) -> nh_common::Result<Operands> {
        todo!()
    }
}

pub mod rules {
    use std::collections::BTreeMap;

    use super::form::Form;
    use super::operands::Operands;

    /// Go: `plural.Rule`.
    #[derive(Clone)]
    pub struct Rule {
        pub plural_forms: Vec<Form>,
        pub plural_form_func: fn(&Operands) -> Form,
    }

    /// Go: `plural.Rules` (tag -> rule; `Rule(tag)` falls back to the base language).
    #[derive(Clone, Default)]
    pub struct Rules {
        pub rules: BTreeMap<String, Rule>,
    }

    impl Rules {
        /// Go: `plural.DefaultRules()` (generated from CLDR; en: one iff i==1&&v==0; th: always other).
        pub fn default_rules() -> Rules {
            todo!()
        }

        pub fn rule(&self, tag: &str) -> Option<&Rule> {
            todo!()
        }
    }
}
