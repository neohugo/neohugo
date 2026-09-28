//! Module `goi18n::bundle`.
//!
//! PORT gohugoio/go-i18n/v2 i18n/bundle.go
//!
//! Owner: Wave B task T17 (i18n).

//! `gohugoio/go-i18n/v2` `i18n/bundle.go`.

use std::collections::{BTreeMap, HashMap};

use xtext_collate::language::{self as xl, Tag};

use super::message::{Message, MessageTemplate};
use super::parse::{MessageFile, ParseError, parse_message_file_bytes};
use super::plural::rules::Rules;
use crate::xlanguage::Matcher;

/// Go: `i18n.Bundle` — a set of messages and pluralization rules.
pub struct Bundle {
    pub default_language: Tag,
    /// Tags in insertion order (default first).
    pub tags: Vec<Tag>,
    /// tag -> message id -> template (`None` = Go's nil `*MessageTemplate`, stored for a
    /// message without any plural form).
    pub message_templates: HashMap<Tag, BTreeMap<String, Option<MessageTemplate>>>,
    pub plural_rules: Rules,
    pub(crate) matcher: BundleMatcher,
}

/// Go: `language.Matcher` of the bundle: x/text's matcher, or go-i18n's `matcher` wrapper when
/// a tag has the artificial base language `art`.
pub(crate) enum BundleMatcher {
    Default(Matcher),
    /// Go: `matcher` — the matcher in x/text/language does not handle artificial languages
    /// (golang/go#45749); this delegates to it for the other cases.
    Art {
        tags: Vec<Tag>,
        default_matcher: Matcher,
    },
}

/// Go: `artTag`, `artTagBase` — the language tag used for artificial languages.
fn art_tag() -> Tag {
    xl::DEFAULT.must_parse("art")
}

// Go: go-i18n i18n/bundle.go:newMatcher
fn new_matcher(tags: &[Tag]) -> BundleMatcher {
    let art_tag_base = art_tag().base().0;
    let mut has_art = false;
    for tag in tags {
        let (base, _) = tag.base();
        has_art = base == art_tag_base;
        if has_art {
            break;
        }
    }

    if !has_art {
        return BundleMatcher::Default(Matcher::new(tags));
    }

    BundleMatcher::Art {
        tags: tags.to_vec(),
        default_matcher: Matcher::new(tags),
    }
}

impl BundleMatcher {
    /// Go: `Match(t...)` — only the index is used by go-i18n.
    // Go: go-i18n i18n/bundle.go:Match
    pub(crate) fn match_index(&self, t: &[Tag]) -> usize {
        match self {
            BundleMatcher::Default(m) => m.match_tags(t).0,
            BundleMatcher::Art {
                tags,
                default_matcher,
            } => {
                let art_tag_base = art_tag().base().0;
                for candidate in t {
                    let (base, _) = candidate.base();
                    if base != art_tag_base {
                        continue;
                    }

                    for (i, tag) in tags.iter().enumerate() {
                        if tag == candidate {
                            return i;
                        }
                    }
                }

                default_matcher.match_tags(t).0
            }
        }
    }
}

impl Bundle {
    /// Go: `NewBundle(defaultLanguage)` — a bundle with a default language and a default set of
    /// plural rules.
    // Go: go-i18n i18n/bundle.go:NewBundle
    pub fn new(default_language: Tag) -> Bundle {
        let mut plural_rules = Rules::default_rules();
        let en = plural_rules
            .rule(&xl::english())
            .cloned()
            .expect("English has a plural rule");
        plural_rules.rules.insert(art_tag(), en);
        let mut b = Bundle {
            default_language: default_language.clone(),
            tags: Vec::new(),
            message_templates: HashMap::new(),
            plural_rules,
            matcher: BundleMatcher::Default(Matcher::new(&[])),
        };
        b.add_tag(default_language);
        b
    }

    /// Go: `ParseMessageFileBytes(buf, path)` (unmarshal by extension; Hugo registers toml,
    /// yaml, yml and json) — parses the bytes to add translations to the bundle. The error is
    /// Go's `Error()` text.
    // Go: go-i18n i18n/bundle.go:ParseMessageFileBytes
    pub fn parse_message_file_bytes(
        &mut self,
        buf: &[u8],
        path: &str,
    ) -> std::result::Result<MessageFile, ParseError> {
        let message_file = parse_message_file_bytes(buf, path)?;
        self.add_messages(message_file.tag.clone(), message_file.messages.clone())?;
        Ok(message_file)
    }

    /// Go: `AddMessages(tag, messages...)` — adds messages for a language.
    // Go: go-i18n i18n/bundle.go:AddMessages
    pub fn add_messages(
        &mut self,
        tag: Tag,
        messages: Vec<Message>,
    ) -> std::result::Result<(), String> {
        if self.plural_rules.rule(&tag).is_none() {
            return Err(format!("no plural rule registered for {}", tag.string()));
        }
        if !self.message_templates.contains_key(&tag) {
            self.message_templates.insert(tag.clone(), BTreeMap::new());
            self.add_tag(tag.clone());
        }
        let templates = self.message_templates.get_mut(&tag).expect("inserted");
        for m in messages {
            templates.insert(m.id.clone(), MessageTemplate::new(m));
        }
        Ok(())
    }

    // Go: go-i18n i18n/bundle.go:addTag
    fn add_tag(&mut self, tag: Tag) {
        for t in &self.tags {
            if *t == tag {
                // Tag already exists
                return;
            }
        }
        self.tags.push(tag);
        self.matcher = new_matcher(&self.tags);
    }

    /// Go: `LanguageTags()` — the language tags of all the translations loaded into the bundle.
    // Go: go-i18n i18n/bundle.go:LanguageTags
    pub fn language_tags(&self) -> &[Tag] {
        &self.tags
    }

    // Go: go-i18n i18n/bundle.go:getMessageTemplate
    pub(crate) fn get_message_template(&self, tag: &Tag, id: &str) -> Option<&MessageTemplate> {
        let templates = self.message_templates.get(tag)?;
        templates.get(id)?.as_ref()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (go-i18n i18n/bundle.go)
// OK newMatcher
// OK matcher.Match (index only)
// OK NewBundle
//    RegisterUnmarshalFunc (Hugo's toml/yaml/yml/json are built in, see parse.rs)
//    LoadMessageFile, MustLoadMessageFile (file system helpers; Hugo reads through hugofs)
// OK ParseMessageFileBytes
//    MustParseMessageFileBytes (panicking helper; not needed)
// OK AddMessages
//    MustAddMessages (panicking helper; not needed)
// OK addTag
// OK LanguageTags
// OK getMessageTemplate
// ---------------------------------------------------------------------------
