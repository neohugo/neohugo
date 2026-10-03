//! `build_stats.json`: the tags, classes and ids used by the HTML outputs (`[build.buildStats]`),
//! read by external CSS tools (Tailwind's `@source`).
//!
//! Every start tag of an HTML output is recorded with its `class` and `id` values. The content
//! of `pre`, `textarea`, `script` and `style` elements is skipped (highlighted code and inline
//! scripts do not count), as are comments and doctypes. Besides `class`, attributes whose name
//! contains `transition` (Alpine.js `x-transition:enter="…"`) hold classes, and Vue/Alpine
//! class bindings (`:class`, `x-bind:class`, `v-bind:class`) contribute their object keys
//! (`{ 'active': on }`) and every single-quoted word.

use std::collections::BTreeSet;
use std::sync::Mutex;

use html5gum::{State, Token, Tokenizer};
use serde::Serialize;
use ssg_config::global::BuildStats;

/// The tags, classes and ids of some HTML.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HtmlElements {
    pub tags: BTreeSet<String>,
    pub classes: BTreeSet<String>,
    pub ids: BTreeSet<String>,
}

impl HtmlElements {
    /// The elements of an HTML document or fragment.
    #[must_use]
    pub fn collect(html: &str) -> Self {
        let mut found = Self::default();
        found.add_html(html);
        found
    }

    /// Adds the elements of `html`.
    pub fn add_html(&mut self, html: &str) {
        let mut tokenizer = Tokenizer::new(html);
        // The element whose content is skipped (`pre`); script-like elements are skipped by
        // switching the tokenizer to their text state.
        let mut skipping: Option<Vec<u8>> = None;
        while let Some(Ok(token)) = tokenizer.next() {
            match token {
                Token::StartTag(tag) if skipping.is_none() => {
                    // The tokenizer lower-cases ASCII only; `DİV` is `div`.
                    let name: String = lossy(&tag.name)
                        .chars()
                        .map(|c| c.to_lowercase().next().unwrap_or(c))
                        .collect();
                    for (key, value) in &tag.attributes {
                        self.add_attribute(&lossy(key), &lossy(&value.value));
                    }
                    if !tag.self_closing {
                        let text_state = match name.as_str() {
                            "script" => Some(State::ScriptData),
                            "style" => Some(State::RawText),
                            "textarea" => Some(State::RcData),
                            "pre" => {
                                skipping = Some(tag.name.to_vec());
                                None
                            }
                            _ => None,
                        };
                        // The text ends at the end tag of the tokenizer's own last start tag
                        // (its name, lower-cased ASCII only), so the state switches only when
                        // that is `name`: `<SCRİPT>` is recorded as `script`, but its content is
                        // not skipped, as with Go's `(?i)` match.
                        if let Some(state) = text_state.filter(|_| tag.name[..] == *name.as_bytes())
                        {
                            tokenizer.set_state(state);
                        }
                    }
                    self.tags.insert(name);
                }
                Token::EndTag(tag) if skipping.as_deref() == Some(&tag.name[..]) => {
                    skipping = None;
                }
                _ => {}
            }
        }
    }

    fn add_attribute(&mut self, key: &str, value: &str) {
        if key == "id" {
            self.ids.insert(value.to_owned());
        } else if key == "class" || key.contains("transition") {
            self.classes
                .extend(value.split_whitespace().map(str::to_owned));
        } else if key.contains(":class") {
            self.classes.extend(class_binding(value));
        }
    }

    /// Adds everything of `other`.
    pub fn merge(&mut self, other: Self) {
        self.tags.extend(other.tags);
        self.classes.extend(other.classes);
        self.ids.extend(other.ids);
    }

    /// Whether nothing was found.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tags.is_empty() && self.classes.is_empty() && self.ids.is_empty()
    }
}

fn lossy(b: &[u8]) -> String {
    String::from_utf8_lossy(b).into_owned()
}

/// The classes a Vue/Alpine class binding names: the keys of an object literal
/// (`{ 'a b': x, c: y }` gives `a`, `b`, `c`), or every single-quoted word of an expression
/// (`on ? 'x' : 'y'` gives `x`, `y`).
///
/// The object's entries are its lines and its `, `-separated parts. An entry with a `:`
/// followed by white space is a key and its value; a key whose `:` ends the line has its
/// value on the next line. Other entries are kept as they are.
fn class_binding(value: &str) -> Vec<String> {
    let value = value.trim();
    let mut classes = Vec::new();
    let text = if value.starts_with('{') {
        let body = value.trim_matches(|c| c == '{' || c == '}');
        let entries: Vec<&str> = body
            .split(", ")
            .flat_map(|p| p.split('\n'))
            .map(str::trim)
            .collect();
        let mut keys: Vec<&str> = Vec::with_capacity(entries.len());
        let mut i = 0;
        while i < entries.len() {
            let entry = entries[i];
            i += 1;
            let colon = entry.match_indices(':').map(|(at, _)| at).find(|&at| {
                let after = &entry[at + 1..];
                after.starts_with(char::is_whitespace) || (after.is_empty() && i < entries.len())
            });
            match colon {
                Some(at) => {
                    if at + 1 == entry.len() {
                        // The value is the next line.
                        i += 1;
                    }
                    let key = &entry[..at];
                    let key = key.strip_prefix('\'').unwrap_or(key);
                    keys.push(key.strip_suffix('\'').unwrap_or(key));
                }
                None => keys.push(entry),
            }
        }
        for key in &keys {
            classes.extend(key.split_whitespace().map(str::to_owned));
        }
        keys.join("\n")
    } else {
        value.to_owned()
    };
    let mut quoted = text.split('\'');
    quoted.next();
    while let (Some(inside), Some(_)) = (quoted.next(), quoted.clone().next()) {
        classes.extend(inside.split_whitespace().map(str::to_owned));
        quoted.next();
    }
    classes
}

/// Collects [`HtmlElements`] from the HTML outputs of a build; shared by the render workers.
#[derive(Debug)]
pub struct StatsCollector {
    config: BuildStats,
    found: Mutex<HtmlElements>,
}

impl StatsCollector {
    /// A collector for `[build.buildStats]`; it collects nothing unless `enable` is set.
    #[must_use]
    pub fn new(config: BuildStats) -> Self {
        Self {
            config,
            found: Mutex::new(HtmlElements::default()),
        }
    }

    /// Whether outputs are scanned.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enable
    }

    /// Scans an HTML output (the parsing happens outside the lock).
    pub fn add(&self, html: &str) {
        if self.config.enable {
            let found = HtmlElements::collect(html);
            if !found.is_empty() {
                self.found
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .merge(found);
            }
        }
    }

    /// The collected elements, with the disabled lists left out.
    #[must_use]
    pub fn stats(&self) -> StatsFile {
        let found = self
            .found
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        StatsFile::new(found, &self.config)
    }
}

/// The content of `build_stats.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StatsFile {
    #[serde(rename = "htmlElements")]
    pub html_elements: StatsLists,
}

/// The sorted lists of `build_stats.json`; a disabled or empty list is `null`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct StatsLists {
    pub tags: Option<Vec<String>>,
    pub classes: Option<Vec<String>>,
    pub ids: Option<Vec<String>>,
}

impl StatsFile {
    /// The stats of `found` as configured.
    #[must_use]
    pub fn new(found: HtmlElements, config: &BuildStats) -> Self {
        let list = |set: BTreeSet<String>, disabled: bool| {
            (!disabled && !set.is_empty()).then(|| set.into_iter().collect())
        };
        Self {
            html_elements: StatsLists {
                tags: list(found.tags, config.disable_tags),
                classes: list(found.classes, config.disable_classes),
                ids: list(found.ids, config.disable_ids),
            },
        }
    }

    /// The file's bytes: JSON indented by two spaces, with a final newline.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).unwrap_or_default();
        s.push('\n');
        // JSON allows these raw, but JavaScript string literals did not; Go escapes them.
        s.replace('\u{2028}', "\\u2028")
            .replace('\u{2029}', "\\u2029")
    }

    /// Writes the file unless it already has this content (so file watchers of external
    /// tools do not fire for nothing). Returns whether it wrote.
    ///
    /// # Errors
    /// I/O errors.
    pub fn write_if_changed(&self, path: &std::path::Path) -> std::io::Result<bool> {
        let json = self.to_json();
        if std::fs::read(path).is_ok_and(|old| old == json.as_bytes()) {
            return Ok(false);
        }
        std::fs::write(path, json)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings() {
        assert_eq!(class_binding("{ 'k': 1 }"), ["k"]);
        assert_eq!(class_binding("{ 'a b': x == 'z', c: y }"), ["a", "b", "c"]);
        assert_eq!(class_binding("on ? 'x' : 'y'"), ["x", "y"]);
    }

    #[test]
    fn skips_code() {
        let e = HtmlElements::collect(concat!(
            r#"<div class="a b" id=x><pre class=p><span class="k">x</span></pre>"#,
            r#"<script>"<i class=no>"</script><style>.x{}</style><!-- <b class=c> --></div>"#,
        ));
        assert_eq!(
            e.tags.iter().map(String::as_str).collect::<Vec<_>>(),
            ["div", "pre", "script", "style"]
        );
        assert_eq!(
            e.classes.iter().map(String::as_str).collect::<Vec<_>>(),
            ["a", "b", "p"]
        );
        assert_eq!(e.ids.iter().map(String::as_str).collect::<Vec<_>>(), ["x"]);
    }
}
