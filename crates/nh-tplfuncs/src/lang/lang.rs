//! Port of `tpl/lang/lang.go`.
//!
//! Owner: Wave B task T19 (tplfuncs-host).

use std::sync::Arc;

use go_value::{GoString, HostCtx, Object, SliceType, Value};
use nh_common::cast::caste;
use nh_common::locales::Translator;
use nh_common::object::{GoResult, args};
use nh_deps::deps::Deps;

use crate::math::gomath;

// Parity notes: `i18n`/`T` -> deps.translate(ctx, id, data) (go-i18n port in nh-i18n).

/// Go: `lang.Namespace` (template value `*lang.Namespace`).
pub struct Namespace {
    pub d: Arc<Deps>,
    translator: Arc<dyn Translator>,
}

fn gerr(msg: impl Into<String>) -> go_value::Error {
    go_value::Error::new(msg)
}

fn sprintf(format: &str, a: &[Value]) -> String {
    String::from_utf8_lossy(&go_fmt::sprintf(format, a)).into_owned()
}

impl Namespace {
    /// New returns a new instance of the lang-namespaced template functions, with the
    /// translator of the site's language (Go's `init` passes `langs.GetTranslator(...)`).
    // Go: tpl/lang/lang.go:New
    pub fn new(d: Arc<Deps>) -> Namespace {
        let translator = d.conf.language().translator().clone();
        Namespace::new_with(d, translator)
    }

    /// Go: `New(deps, translator)`.
    // Go: tpl/lang/lang.go:New
    pub fn new_with(d: Arc<Deps>, translator: Arc<dyn Translator>) -> Namespace {
        Namespace { d, translator }
    }

    /// FormatAccounting returns the currency representation of number for the given currency
    /// and precision for the current language in accounting notation.
    ///
    /// The return value is formatted with at least two decimal places.
    // Go: tpl/lang/lang.go:FormatAccounting
    pub fn format_accounting(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 3, "FormatAccounting")?;
        let (p, n) = self.cast_precision_number(&a[0], &a[2])?;
        let code = caste::to_string(&a[1]);
        let code = code.to_str_lossy();
        let c = nh_common::locales::get_currency(&code);
        if c < 0 {
            return Err(gerr(sprintf(
                "unknown currency code: %q",
                std::slice::from_ref(&a[1]),
            )));
        }
        Ok(Value::string(
            self.translator.try_fmt_accounting(n, p, &code)?,
        ))
    }

    /// FormatCurrency returns the currency representation of number for the given currency and
    /// precision for the current language.
    ///
    /// The return value is formatted with at least two decimal places.
    // Go: tpl/lang/lang.go:FormatCurrency
    pub fn format_currency(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 3, "FormatCurrency")?;
        let (p, n) = self.cast_precision_number(&a[0], &a[2])?;
        let code = caste::to_string(&a[1]);
        let code = code.to_str_lossy();
        let c = nh_common::locales::get_currency(&code);
        if c < 0 {
            return Err(gerr(sprintf(
                "unknown currency code: %q",
                std::slice::from_ref(&a[1]),
            )));
        }
        Ok(Value::string(
            self.translator.try_fmt_currency(n, p, &code)?,
        ))
    }

    /// FormatNumber formats number with the given precision for the current language.
    // Go: tpl/lang/lang.go:FormatNumber
    pub fn format_number(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "FormatNumber")?;
        let (p, n) = self.cast_precision_number(&a[0], &a[1])?;
        Ok(Value::string(self.translator.try_fmt_number(n, p)?))
    }

    /// FormatPercent formats number with the given precision for the current language. Note
    /// that the number is assumed to be a percentage.
    // Go: tpl/lang/lang.go:FormatPercent
    pub fn format_percent(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "FormatPercent")?;
        let (p, n) = self.cast_precision_number(&a[0], &a[1])?;
        Ok(Value::string(self.translator.fmt_percent(n, p)))
    }

    // Go: tpl/lang/lang.go:castPrecisionNumber
    fn cast_precision_number(&self, precision: &Value, number: &Value) -> GoResult<(u64, f64)> {
        let p = caste::to_uint64_e(precision)?;

        // Sanity check.
        if p > 20 {
            return Err(gerr(sprintf(
                "invalid precision: %d",
                std::slice::from_ref(precision),
            )));
        }

        let n = caste::to_float64_e(number)?;
        Ok((p, n))
    }

    /// FormatNumberCustom formats a number with the given precision. The first options
    /// parameter is a space-delimited string of characters to represent negativity, the decimal
    /// point, and grouping. The default value is `- . ,`. The second options parameter defines
    /// an alternate delimiting character.
    ///
    /// Note that numbers are rounded up at 5 or greater. So, with precision set to 0, 1.5
    /// becomes `2`, and 1.4 becomes `1`.
    // Go: tpl/lang/lang.go:FormatNumberCustom
    pub fn format_number_custom(&self, _ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 2, "FormatNumberCustom")?;
        let prec = caste::to_int_e(&a[0])?;
        let n = caste::to_float64_e(&a[1])?;
        let options = &a[2..];

        let (mut neg, mut dec, mut grp): (Vec<u8>, Vec<u8>, Vec<u8>) =
            (Vec::new(), Vec::new(), Vec::new());

        if options.is_empty() {
            // defaults
            neg = b"-".to_vec();
            dec = b".".to_vec();
            grp = b",".to_vec();
        } else {
            let mut delim: GoString = GoString::from(" ");

            if options.len() == 2 {
                // custom delimiter
                delim = caste::to_string_e(&options[1])?;
            }

            let s = caste::to_string_e(&options[0])?;

            let rs = go_unicode::strings::split(s.as_bytes(), delim.as_bytes());
            match rs.len() {
                0 => {}
                1 => neg = rs[0].to_vec(),
                2 => {
                    neg = rs[0].to_vec();
                    dec = rs[1].to_vec();
                }
                3 => {
                    neg = rs[0].to_vec();
                    dec = rs[1].to_vec();
                    grp = rs[2].to_vec();
                }
                _ => return Err(gerr("too many fields in options parameter to NumFmt")),
            }
        }

        let exp = gomath::pow(10.0, prec as f64);
        let r = (n * exp).round() / exp;

        // Logic from MIT Licensed github.com/gohugoio/locales/
        // Original Copyright (c) 2016 Go Playground

        let s = go_strconv::format_float(r.abs(), b'f', prec, 64).into_bytes();
        let int_len = s.len() as i64 - 1 - prec;
        // Go: `s[:len(s)-1-prec]` panics out of range.
        if int_len < 0 {
            return Err(gerr(format!(
                "runtime error: slice bounds out of range [:{int_len}]"
            )));
        }
        if int_len > s.len() as i64 {
            return Err(gerr(format!(
                "runtime error: slice bounds out of range [:{int_len}] with length {}",
                s.len()
            )));
        }

        let mut count = 0;
        let mut in_whole = prec == 0;
        let mut b: Vec<u8> = Vec::with_capacity(s.len() + 2 + int_len as usize / 3);

        for i in (0..s.len()).rev() {
            if s[i] == b'.' {
                for j in (0..dec.len()).rev() {
                    b.push(dec[j]);
                }
                in_whole = true;
                continue;
            }

            if in_whole {
                if count == 3 {
                    for j in (0..grp.len()).rev() {
                        b.push(grp[j]);
                    }
                    count = 1;
                } else {
                    count += 1;
                }
            }

            b.push(s[i]);
        }

        if n < 0.0 {
            for j in (0..neg.len()).rev() {
                b.push(neg[j]);
            }
        }

        // reverse
        b.reverse();

        Ok(Value::string(b))
    }

    /// Merge creates a union of pages from two languages.
    // Go: tpl/lang/lang.go:Merge
    pub fn merge(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::exactly(a, 2, "Merge")?;
        let (p2, p1) = (&a[0], &a[1]);
        if !nh_common::hreflect::is_truthful(p1) {
            return Ok(p2.clone());
        }
        if !nh_common::hreflect::is_truthful(p2) {
            return Ok(p1.clone());
        }
        let not_supported = || {
            gerr(format!(
                "language merge not supported for {}",
                p1.go_type_name()
            ))
        };
        let r = match p1 {
            Value::List(l) => match &l.ty {
                SliceType::Named(n) if &**n == nh_page::page::PAGES_TYPE => {
                    (nh_page::pages::PAGES_METHODS.call)(
                        ctx,
                        p1,
                        "MergeByLanguageInterface",
                        std::slice::from_ref(p2),
                    )
                }
                SliceType::Named(n) if &**n == nh_resource::resourcetypes::RESOURCES_TYPE => {
                    (nh_resource::resources::RESOURCES_METHODS.call)(
                        ctx,
                        p1,
                        "MergeByLanguageInterface",
                        std::slice::from_ref(p2),
                    )
                }
                _ => None,
            },
            Value::Object(o) if o.has_method("MergeByLanguageInterface") => {
                o.call_method(ctx, "MergeByLanguageInterface", std::slice::from_ref(p2))
            }
            _ => None,
        };
        r.unwrap_or_else(|| Err(not_supported()))
    }

    /// Translate returns a translated string for id.
    // Go: tpl/lang/lang.go:Translate
    pub fn translate(&self, ctx: HostCtx<'_>, a: &[Value]) -> GoResult<Value> {
        args::at_least(a, 1, "Translate")?;
        let id = &a[0];
        let rest = &a[1..];
        let mut template_data = Value::Invalid;

        if !rest.is_empty() {
            if rest.len() > 1 {
                return Err(gerr(format!(
                    "wrong number of arguments, expecting at most 2, got {}",
                    rest.len() + 1
                )));
            }
            template_data = rest[0].clone();
        }

        let sid = caste::to_string_e(id)?;

        Ok(Value::string(self.d.translate(
            ctx,
            &sid.to_str_lossy(),
            &template_data,
        )?))
    }
}

nh_common::go_methods!(Namespace {
    "FormatAccounting" => |n, ctx, a| n.format_accounting(ctx, a),
    "FormatCurrency" => |n, ctx, a| n.format_currency(ctx, a),
    "FormatNumber" => |n, ctx, a| n.format_number(ctx, a),
    "FormatNumberCustom" => |n, ctx, a| n.format_number_custom(ctx, a),
    "FormatPercent" => |n, ctx, a| n.format_percent(ctx, a),
    "Merge" => |n, ctx, a| n.merge(ctx, a),
    "Translate" => |n, ctx, a| n.translate(ctx, a),
});

impl Object for Namespace {
    nh_common::object_basics!("*lang.Namespace");
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: tpl/lang/lang.go (259 lines; 2/9 funcs executed)
//   types: Namespace, pagesLanguageMerger
// OK L34-39: New(deps *deps.Deps, translator locales.Translator) *Namespace
// OK L48-64: (ns *Namespace) Translate(ctx context.Context, id any, args ...any) (string, error)
// OK L67-73: (ns *Namespace) FormatNumber(precision, number any) (string, error)
// OK L77-83: (ns *Namespace) FormatPercent(precision, number any) (string, error)
// OK L89-99: (ns *Namespace) FormatCurrency(precision, currency, number any) (string, error)
// OK L105-115: (ns *Namespace) FormatAccounting(precision, currency, number any) (string, error)
// OK L117-133: (ns *Namespace) castPrecisionNumber(precision, number any) (uint64, float64, error)
// OK L144-240: (ns *Namespace) FormatNumberCustom(precision, number any, options ...any) (string, error)
// OK L247-259: (ns *Namespace) Merge(p2, p1 any) (any, error)
// ---------------------------------------------------------------------------
