//! Port of `langs/i18n/i18n.go`.
//!
//! Owner: Wave B task T17 (i18n).

//! Go `langs/i18n/i18n.go`: the `i18n`/`T` translate func per language (plural count detection
//! from `Count`, default-language fallback, `enableMissingTranslationPlaceholders`, warnings).

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{HostCtx, Kind, MapType, Value};
use nh_common::herrors::Error;
use nh_common::loggers::Logger;
use xtext_collate::language::Tag;

use crate::goi18n::bundle::Bundle;
use crate::goi18n::localizer::{LocalizeConfig, LocalizeError, Localizer};
use crate::translation_provider::ARTIFICIAL_LANG_TAG_PREFIX;

/// A translate func that reports Go's runtime panics as errors: Go panics inside the translate
/// func for a few inputs (`i18n "x" ""`, a `Count` that is an untyped nil, a nil pointer as
/// data), and text/template turns the panic into the error of the `i18n`/`T` call. The error
/// message is Go's panic value.
pub type TranslateFuncE =
    Arc<dyn Fn(HostCtx<'_>, &str, &Value) -> std::result::Result<String, Error> + Send + Sync>;

/// Go: `i18n.Translator` — lang (lower-cased tag) -> translate func.
pub struct Translator {
    pub(crate) translate_funcs: BTreeMap<String, TranslateFuncE>,
    pub default_content_language: String,
    logger: Logger,
}

impl Translator {
    /// Go: `NewTranslator(b, cfg, logger)`.
    // Go: langs/i18n/i18n.go:NewTranslator
    pub fn new(
        b: Arc<Bundle>,
        cfg: &dyn nh_config::config_provider::AllProvider,
        logger: Logger,
    ) -> Translator {
        let mut t = Translator {
            translate_funcs: BTreeMap::new(),
            default_content_language: cfg.default_content_language(),
            logger,
        };
        t.init_funcs(b, cfg);
        t
    }

    /// Go: `Func(lang)` — unknown language -> default content language's func. A Go panic in
    /// the returned func becomes a Rust panic with Go's message; see [`Translator::func_e`].
    // Go: langs/i18n/i18n.go:Func
    pub fn func(&self, lang: &str) -> nh_deps::deps::TranslateFunc {
        let f = self.func_e(lang);
        Arc::new(move |ctx, id, data| match f(ctx, id, data) {
            Ok(s) => s,
            Err(e) => panic!("{}", e.message()),
        })
    }

    /// Go: `Func(lang)`, with Go's runtime panics as errors.
    // Go: langs/i18n/i18n.go:Func
    pub fn func_e(&self, lang: &str) -> TranslateFuncE {
        if let Some(f) = self.translate_funcs.get(lang) {
            return f.clone();
        }
        self.logger.infof(format!(
            "Translation func for language {lang} not found, use default."
        ));
        if let Some(f) = self.translate_funcs.get(&self.default_content_language) {
            return f.clone();
        }

        self.logger.infof("i18n not initialized; if you need string translations, check that you have a bundle in /i18n that matches the site language or the default language.");
        Arc::new(|_ctx, _id, _args| Ok(String::new()))
    }

    /// The languages (lower-cased tags, `art-x-` stripped) that have a translate func.
    pub fn languages(&self) -> Vec<String> {
        self.translate_funcs.keys().cloned().collect()
    }

    // Go: langs/i18n/i18n.go:initFuncs
    fn init_funcs(&mut self, bndl: Arc<Bundle>, cfg: &dyn nh_config::config_provider::AllProvider) {
        let enable_missing_translation_placeholders = cfg.enable_missing_translation_placeholders();
        let print_i18n_warnings = cfg.print_i18n_warnings();
        for lang in bndl.language_tags() {
            let current_lang: Tag = lang.clone();
            let current_lang_str = current_lang.string();
            // This may be pt-BR; make it case insensitive.
            let trimmed = current_lang_str
                .strip_prefix(ARTIFICIAL_LANG_TAG_PREFIX)
                .unwrap_or(&current_lang_str);
            let current_lang_key =
                String::from_utf8_lossy(&go_unicode::strings::to_lower(trimmed.as_bytes()))
                    .into_owned();
            let localizer = Localizer::new(bndl.clone(), &[current_lang_str.as_str()]);
            let logger = self.logger.clone();
            let f: TranslateFuncE = Arc::new(move |ctx, translation_id, template_data| {
                let plural_count = get_plural_count_e(ctx, template_data)?;

                let mut template_data = Cow::Borrowed(template_data);
                if !matches!(*template_data, Value::Invalid) && is_int_kind(&template_data) {
                    // This was how go-i18n worked in v1,
                    // and we keep it like this to avoid breaking
                    // lots of sites in the wild.
                    template_data = Cow::Owned(Value::object(IntCount(
                        nh_common::cast::caste::to_int(&template_data),
                    )));
                }
                // Go wraps a page.Page in page.PageWithContext{Page, Ctx} here (issue 10782);
                // the Rust engine hands the context to every method call, so the page is passed
                // as is (PORTING.md deviation).

                let (translated, translated_lang, err) = localizer.localize_with_tag_ctx(
                    ctx,
                    &LocalizeConfig {
                        message_id: translation_id.to_string(),
                        template_data: template_data.into_owned(),
                        plural_count,
                        default_message: None,
                    },
                );
                if let Some(LocalizeError::Panic(p)) = &err {
                    return Err(Error::new(p.clone()));
                }

                let same_lang = current_lang == translated_lang;

                if err.is_none() && same_lang {
                    return Ok(translated);
                }

                if let Some(e) = &err
                    && same_lang
                    && !translated.is_empty()
                {
                    // See #8492
                    // TODO(bep) this needs to be improved/fixed upstream,
                    // but currently we get an error even if the fallback to
                    // "other" succeeds.
                    if e.is_plural_form_not_found() {
                        return Ok(translated);
                    }
                }

                if !matches!(err, Some(LocalizeError::MessageNotFound { .. })) {
                    let es = match &err {
                        Some(e) => e.to_string(),
                        // Go formats a nil error with %s.
                        None => "%!s(<nil>)".to_string(),
                    };
                    logger.warnf(format!(
                        "Failed to get translated string for language {} and ID {}: {}",
                        go_strconv::quote(&current_lang_str),
                        go_strconv::quote(translation_id),
                        es
                    ));
                }

                if print_i18n_warnings {
                    logger.warnf(format!(
                        "i18n|MISSING_TRANSLATION|{current_lang_str}|{translation_id}"
                    ));
                }

                if enable_missing_translation_placeholders {
                    return Ok(format!("[i18n] {translation_id}"));
                }

                Ok(translated)
            });
            self.translate_funcs.insert(current_lang_key, f);
        }
    }
}

/// Go: `intCount` — wraps the Count method (an int passed as template data).
#[derive(Clone, Copy, Debug)]
pub struct IntCount(pub i64);

impl go_value::Object for IntCount {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("i18n.intCount")
    }

    fn kind(&self) -> Kind {
        Kind::Struct
    }

    fn has_method(&self, name: &str) -> bool {
        name == "Count"
    }

    // Go: langs/i18n/i18n.go:Count
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        if name != "Count" {
            return None;
        }
        if !args.is_empty() {
            return Some(Err(go_value::Error::new(format!(
                "wrong number of args for Count: want 0 got {}",
                args.len()
            ))));
        }
        Some(Ok(Value::int(self.0)))
    }

    fn underlying(&self) -> Option<Value> {
        Some(Value::int(self.0))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

const COUNT_FIELD_NAME: &str = "Count";

/// The reflect kind class of a value, as `getPluralCount`/`toPluralCountValue` test it.
fn basic(v: &Value) -> Cow<'_, Value> {
    if let Value::Object(o) = v
        && let Some(u) = o.underlying()
    {
        return Cow::Owned(u);
    }
    Cow::Borrowed(v)
}

/// Go: `hreflect.IsInt(reflect.TypeOf(v).Kind())`.
fn is_int_kind(v: &Value) -> bool {
    matches!(*basic(v), Value::Int(..))
}

/// Go: `getPluralCount(templateData)` — nil / map key EqualFold "Count" / struct field or method
/// `Count` / numbers (floats as strings with ".0") -> the plural operand, else nil. Go panics
/// for a few inputs (see [`TranslateFuncE`]); this function then panics too.
// Go: langs/i18n/i18n.go:getPluralCount
pub fn get_plural_count(v: &Value) -> Option<Value> {
    match get_plural_count_e(&(), v) {
        Ok(v) => v,
        Err(e) => panic!("{}", e.message()),
    }
}

// Go: langs/i18n/i18n.go:getPluralCount
/// Gets the plural count as a string (floats) or an integer. If v is nil, nil is returned. A Go
/// runtime panic is returned as an error.
pub(crate) fn get_plural_count_e(
    ctx: HostCtx<'_>,
    v: &Value,
) -> std::result::Result<Option<Value>, Error> {
    if matches!(v, Value::Invalid) {
        // i18n called without any argument, make sure it does not
        // get any plural count.
        return Ok(None);
    }

    match v {
        Value::Map(m) if m.ty == MapType::StringAny => {
            // Go ranges over the map in random order; the first key (in byte order) that
            // EqualFolds "Count" is used here.
            for (k, vv) in &m.entries {
                if go_unicode::strings::equal_fold(k.as_bytes(), COUNT_FIELD_NAME.as_bytes()) {
                    return to_plural_count_value(vv);
                }
            }
        }
        _ => {
            // vv := reflect.Indirect(reflect.ValueOf(v)); tp := vv.Type()
            if let Value::TypedNil(t) = v
                && go_value::typed_nil_kind(t) == go_value::NilKind::Ptr
            {
                return Err(Error::new(
                    "reflect: call of reflect.Value.Type on zero Value",
                ));
            }
            if let Value::Object(o) = v
                && o.underlying().is_none()
                && matches!(o.kind(), Kind::Struct | Kind::Ptr)
            {
                if let Some(f) = o.field(COUNT_FIELD_NAME) {
                    return to_plural_count_value(&f);
                }
                if o.has_method(COUNT_FIELD_NAME)
                    && let Some(Ok(c)) = o.call_method(ctx, COUNT_FIELD_NAME, &[])
                {
                    return to_plural_count_value(&c);
                }
            }
        }
    }

    to_plural_count_value(v)
}

// Go: langs/i18n/i18n.go:toPluralCountValue
/// go-i18n expects floats to be represented by string.
fn to_plural_count_value(input: &Value) -> std::result::Result<Option<Value>, Error> {
    if matches!(input, Value::Invalid) {
        // reflect.TypeOf(nil).Kind()
        return Err(Error::new(
            "runtime error: invalid memory address or nil pointer dereference",
        ));
    }
    let b = basic(input);
    match &*b {
        Value::Float(..) => {
            let mut f = nh_common::cast::caste::to_string(input).to_vec();
            if !f.contains(&b'.') {
                f.extend_from_slice(b".0");
            }
            Ok(Some(Value::String(f.into())))
        }
        Value::String(_) | Value::Safe(..) => {
            if nh_common::cast::caste::to_float64_e(input).is_ok() {
                return Ok(Some(input.clone()));
            }
            // A non-numeric value.
            Ok(None)
        }
        _ => match nh_common::cast::caste::to_int_e(input) {
            Ok(i) => Ok(Some(Value::int(i))),
            Err(_) => Ok(None),
        },
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: langs/i18n/i18n.go (205 lines; 5/6 funcs executed)
//   types: translateFunc, Translator, intCount
// OK L42-46: NewTranslator(b *i18n.Bundle, cfg config.AllProvider, logger loggers.Logger) Translator
// OK L50-63: (t Translator) Func(lang string) translateFunc
// OK L65-133: (t Translator) initFuncs(bndl *i18n.Bundle)
// OK L138-140: (c intCount) Count() int
// OK L146-181: getPluralCount(v any) any
// OK L184-205: toPluralCountValue(in any) any
// ---------------------------------------------------------------------------
