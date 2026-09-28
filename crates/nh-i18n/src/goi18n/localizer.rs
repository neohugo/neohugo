//! Module `goi18n::localizer`.
//!
//! PORT i18n/localizer.go
//!
//! Owner: Wave B task T17 (i18n).

use std::sync::{Arc, OnceLock};

use go_value::{HostCtx, MapType, Value};
use xtext_collate::language::Tag;

use super::bundle::Bundle;
use super::message::{ExecuteError, Message, MessageTemplate};
use super::plural::form::Form;
use super::plural::operands::{Operands, OperandsError, new_operands_e};

/// Go: `i18n.LocalizeConfig` (Hugo sets `MessageID`, `TemplateData` and `PluralCount`; `Funcs`
/// is not supported).
pub struct LocalizeConfig {
    /// The id of the message to lookup (ignored if `default_message` is set).
    pub message_id: String,
    /// The data passed when executing the message's template. If it is nil (`Invalid`) and
    /// `plural_count` is set, the template is executed with `{"PluralCount": count}`.
    pub template_data: Value,
    /// Determines which plural form of the message is used (`None` = Go nil).
    pub plural_count: Option<Value>,
    /// Used if the message is not found in any message files.
    pub default_message: Option<Message>,
}

/// Go: `i18n.Localizer` — looks up messages in the bundle according to the language
/// preferences in `tags`.
pub struct Localizer {
    pub bundle: Arc<Bundle>,
    /// The language tags that the Localizer checks in order when localizing a message.
    pub tags: Vec<Tag>,
    /// The bundle tag index `bundle.matcher.Match(tags...)` returns. Go matches on every call;
    /// the bundle is immutable once shared, so the result is computed once.
    matched: OnceLock<usize>,
}

impl Localizer {
    /// Go: `NewLocalizer(bundle, langs...)` (langs may be Accept-Language headers).
    // Go: go-i18n i18n/localizer.go:NewLocalizer
    pub fn new(bundle: Arc<Bundle>, langs: &[&str]) -> Localizer {
        Localizer {
            bundle,
            tags: parse_tags(langs),
            matched: OnceLock::new(),
        }
    }

    /// Go: `Localize(lc)`.
    // Go: go-i18n i18n/localizer.go:Localize
    pub fn localize(
        &self,
        ctx: HostCtx<'_>,
        lc: &LocalizeConfig,
    ) -> (String, Option<LocalizeError>) {
        let (msg, _, err) = self.localize_with_tag_ctx(ctx, lc);
        (msg, err)
    }

    /// Go: `LocalizeWithTag(lc)` -> (translated, tag, err). A missing message in a non-default
    /// language falls back to the default language (with a MessageNotFoundErr).
    // Go: go-i18n i18n/localizer.go:LocalizeWithTag
    pub fn localize_with_tag(&self, lc: &LocalizeConfig) -> (String, Tag, Option<LocalizeError>) {
        self.localize_with_tag_ctx(&(), lc)
    }

    /// [`Localizer::localize_with_tag`] with the host context handed to the message template.
    // Go: go-i18n i18n/localizer.go:LocalizeWithTag
    pub fn localize_with_tag_ctx(
        &self,
        ctx: HostCtx<'_>,
        lc: &LocalizeConfig,
    ) -> (String, Tag, Option<LocalizeError>) {
        let mut message_id = lc.message_id.clone();
        if let Some(dm) = &lc.default_message {
            if !message_id.is_empty() && message_id != dm.id {
                return (
                    String::new(),
                    Tag::default(),
                    Some(LocalizeError::MessageIdMismatch {
                        message_id,
                        default_message_id: dm.id.clone(),
                    }),
                );
            }
            message_id = dm.id.clone();
        }

        let mut operands: Option<Operands> = None;
        let mut template_data = lc.template_data.clone();
        if let Some(pc) = &lc.plural_count {
            match new_operands_e(pc) {
                Ok(o) => operands = Some(o),
                Err(OperandsError::Err(err)) => {
                    return (
                        String::new(),
                        Tag::default(),
                        Some(LocalizeError::InvalidPluralCount {
                            message_id,
                            plural_count: pc.clone(),
                            err,
                        }),
                    );
                }
                Err(OperandsError::Panic(p)) => {
                    return (String::new(), Tag::default(), Some(LocalizeError::Panic(p)));
                }
            }
            if matches!(template_data, Value::Invalid) {
                let mut m = go_value::Map::new(MapType::StringAny);
                m.insert("PluralCount", pc.clone());
                template_data = Value::map(m);
            }
        }

        let (tag, template, mut err) =
            self.get_message_template(&message_id, lc.default_message.as_ref());
        let Some(template) = template else {
            return (String::new(), Tag::default(), err);
        };

        let plural_form = self.plural_form(&tag, operands.as_ref());
        let (mut msg, err2) = match template.execute(ctx, plural_form, &template_data) {
            Ok(s) => (s, None),
            Err(e) => (String::new(), Some(e)),
        };
        if let Some(err2) = err2 {
            if err.is_none() {
                err = Some(LocalizeError::Execute(err2));
            }

            // Attempt to fallback to "Other" pluralization in case translations are incomplete.
            if plural_form != Form::Other
                && let Ok(msg2) = template.execute(ctx, Form::Other, &template_data)
            {
                msg = msg2;
            }
        }
        (msg, tag, err)
    }

    // Go: go-i18n i18n/localizer.go:getMessageTemplate
    fn get_message_template(
        &self,
        id: &str,
        default_message: Option<&Message>,
    ) -> (Tag, Option<MessageTemplateRef<'_>>, Option<LocalizeError>) {
        let i = *self
            .matched
            .get_or_init(|| self.bundle.matcher.match_index(&self.tags));
        let tag = self.bundle.tags[i].clone();
        if let Some(mt) = self.bundle.get_message_template(&tag, id) {
            return (tag, Some(MessageTemplateRef::Bundle(mt)), None);
        }

        if tag == self.bundle.default_language {
            let Some(dm) = default_message else {
                return (
                    Tag::default(),
                    None,
                    Some(LocalizeError::MessageNotFound {
                        tag,
                        message_id: id.to_string(),
                    }),
                );
            };
            return (
                tag,
                MessageTemplate::new(dm.clone()).map(|t| MessageTemplateRef::Owned(Box::new(t))),
                None,
            );
        }

        // Fallback to default language in bundle.
        if let Some(mt) = self
            .bundle
            .get_message_template(&self.bundle.default_language, id)
        {
            return (
                self.bundle.default_language.clone(),
                Some(MessageTemplateRef::Bundle(mt)),
                Some(LocalizeError::MessageNotFound {
                    tag,
                    message_id: id.to_string(),
                }),
            );
        }

        // Fallback to default message.
        let Some(dm) = default_message else {
            return (
                Tag::default(),
                None,
                Some(LocalizeError::MessageNotFound {
                    tag,
                    message_id: id.to_string(),
                }),
            );
        };
        (
            self.bundle.default_language.clone(),
            MessageTemplate::new(dm.clone()).map(|t| MessageTemplateRef::Owned(Box::new(t))),
            Some(LocalizeError::MessageNotFound {
                tag,
                message_id: id.to_string(),
            }),
        )
    }

    // Go: go-i18n i18n/localizer.go:pluralForm
    fn plural_form(&self, tag: &Tag, operands: Option<&Operands>) -> Form {
        let Some(operands) = operands else {
            return Form::Other;
        };
        let rule = self
            .bundle
            .plural_rules
            .rule(tag)
            .expect("every bundle tag has a plural rule (AddMessages checks it)");
        (rule.plural_form_func)(operands)
    }
}

/// A message template of the bundle, or one made from the default message.
enum MessageTemplateRef<'a> {
    Bundle(&'a MessageTemplate),
    Owned(Box<MessageTemplate>),
}

impl MessageTemplateRef<'_> {
    fn execute(
        &self,
        ctx: HostCtx<'_>,
        plural_form: Form,
        data: &Value,
    ) -> Result<String, ExecuteError> {
        match self {
            MessageTemplateRef::Bundle(t) => t.execute(ctx, plural_form, data),
            MessageTemplateRef::Owned(t) => t.execute(ctx, plural_form, data),
        }
    }
}

// Go: go-i18n i18n/localizer.go:parseTags
fn parse_tags(langs: &[&str]) -> Vec<Tag> {
    let mut tags = Vec::new();
    for lang in langs {
        let Ok((t, _)) = crate::xlanguage::parse_accept_language(lang) else {
            continue;
        };
        tags.extend(t);
    }
    tags
}

/// go-i18n error kinds that Hugo inspects.
#[derive(Clone, Debug)]
pub enum LocalizeError {
    /// Go: `*MessageNotFoundErr`.
    MessageNotFound { tag: Tag, message_id: String },
    /// Go: `pluralFormNotFoundError` or a message template parse/exec error (from
    /// `MessageTemplate.Execute`).
    Execute(ExecuteError),
    /// Go: `*invalidPluralCountErr`.
    InvalidPluralCount {
        message_id: String,
        plural_count: Value,
        err: String,
    },
    /// Go: `*messageIDMismatchErr`.
    MessageIdMismatch {
        message_id: String,
        default_message_id: String,
    },
    /// A Go runtime panic inside go-i18n (`plural.NewOperands("")` indexes an empty string); in
    /// Go it propagates out of the translate func.
    Panic(String),
}

impl LocalizeError {
    /// Go: `fmt.Sprintf("%T", err) == "i18n.pluralFormNotFoundError"`.
    pub fn is_plural_form_not_found(&self) -> bool {
        matches!(
            self,
            LocalizeError::Execute(ExecuteError::PluralFormNotFound { .. })
        )
    }
}

impl std::fmt::Display for LocalizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Go: go-i18n i18n/localizer.go:(*MessageNotFoundErr).Error
            LocalizeError::MessageNotFound { tag, message_id } => write!(
                f,
                "message {} not found in language {}",
                go_strconv::quote(message_id),
                go_strconv::quote(tag.string())
            ),
            LocalizeError::Execute(e) => write!(f, "{e}"),
            // Go: go-i18n i18n/localizer.go:(*invalidPluralCountErr).Error
            LocalizeError::InvalidPluralCount {
                message_id,
                plural_count,
                err,
            } => write!(
                f,
                "invalid plural count {} for message id {}: {}",
                String::from_utf8_lossy(&go_fmt::sprintf(
                    "%#v",
                    std::slice::from_ref(plural_count)
                )),
                go_strconv::quote(message_id),
                err
            ),
            // Go: go-i18n i18n/localizer.go:(*messageIDMismatchErr).Error
            LocalizeError::MessageIdMismatch {
                message_id,
                default_message_id,
            } => write!(
                f,
                "message id {} does not match default message id {}",
                go_strconv::quote(message_id),
                go_strconv::quote(default_message_id)
            ),
            LocalizeError::Panic(p) => f.write_str(p),
        }
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (go-i18n i18n/localizer.go)
// OK NewLocalizer
// OK parseTags
// OK invalidPluralCountErr.Error
// OK MessageNotFoundErr.Error
//    pluralizeErr.Error (unused by go-i18n itself)
// OK messageIDMismatchErr.Error
// OK Localize
//    LocalizeMessage (DefaultMessage-only helper; not used by Hugo)
// OK LocalizeWithTag
// OK getMessageTemplate
// OK pluralForm
//    MustLocalize (panicking helper; not needed)
// ---------------------------------------------------------------------------
