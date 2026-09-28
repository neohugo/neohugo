//! Port of `output/outputFormat.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

//! Go `output.Format` (+ built-in formats and `Formats` sorting: weight>0 first ascending, then name).

use std::borrow::Cow;
use std::sync::OnceLock;

use go_value::Value;
use nh_common::{Error, Result};
use nh_config::decode::FieldRef;
use nh_config::decode_struct;

use crate::media::builtin::builtin;
use crate::media::media_type::MediaType;

/// Go: `output.Format`: an output representation, usually to a file on disk.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutputFormat {
    /// The Name is used as an identifier (lower-cased, e.g. "html", "rss", "404",
    /// "sitemapindex"). Internal output formats (i.e. html and rss) can be overridden by
    /// providing a new definition for those types.
    pub name: String,
    pub media_type: MediaType,
    /// Must be set to a value when there are two or more conflicting mediatype for the same
    /// resource (e.g. "amp").
    pub path: String,
    /// The base output file name used when not using "ugly URLs", defaults to "index".
    pub base_name: String,
    /// The value to use for rel links (e.g. "alternate", "canonical").
    pub rel: String,
    /// The protocol to use, i.e. "webcal://". Defaults to the protocol of the baseURL.
    pub protocol: String,
    /// IsPlainText decides whether to use text/template or html/template as template parser.
    pub is_plain_text: bool,
    /// IsHTML returns whether this format is int the HTML family. This includes HTML, AMP
    /// etc. This is used to decide when to create alias redirects etc.
    pub is_html: bool,
    /// Enable to ignore the global uglyURLs setting.
    pub no_ugly: bool,
    /// Enable to override the global uglyURLs setting.
    pub ugly: bool,
    /// Enable if it doesn't make sense to include this format in an alternative format
    /// listing, CSS being one good example.
    pub not_alternative: bool,
    /// Eneable if this is a resource which path always starts at the root, e.g. /robots.txt.
    pub root: bool,
    /// Setting this will make this output format control the value of .Permalink and
    /// .RelPermalink for a rendered Page.
    pub permalinkable: bool,
    /// Setting this to a non-zero value will be used as the first sort criteria.
    pub weight: i64,
}

decode_struct!(OutputFormat, "output.Format", |s| vec![
    FieldRef::new("Name", &mut s.name),
    FieldRef::new("MediaType", &mut s.media_type),
    FieldRef::new("Path", &mut s.path),
    FieldRef::new("BaseName", &mut s.base_name),
    FieldRef::new("Rel", &mut s.rel),
    FieldRef::new("Protocol", &mut s.protocol),
    FieldRef::new("IsPlainText", &mut s.is_plain_text),
    FieldRef::new("IsHTML", &mut s.is_html),
    FieldRef::new("NoUgly", &mut s.no_ugly),
    FieldRef::new("Ugly", &mut s.ugly),
    FieldRef::new("NotAlternative", &mut s.not_alternative),
    FieldRef::new("Root", &mut s.root),
    FieldRef::new("Permalinkable", &mut s.permalinkable),
    FieldRef::new("Weight", &mut s.weight),
]);

impl OutputFormat {
    /// IsZero returns true if f represents a zero value.
    // Go: output/outputFormat.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.name.is_empty()
    }

    /// BaseFilename returns the base filename of f including an extension (ie. "index.xml").
    // Go: output/outputFormat.go:BaseFilename
    pub fn base_filename(&self) -> String {
        format!(
            "{}{}",
            self.base_name, self.media_type.first_suffix.full_suffix
        )
    }

    /// MarshalJSON returns the JSON encoding of f (the media type as its string, then the
    /// fields with json tags).
    // Go: output/outputFormat.go:MarshalJSON
    pub fn marshal_json_bytes(&self) -> Result<Vec<u8>> {
        let s = |v: &str| -> Result<Vec<u8>> {
            go_json::marshal(&Value::string(v)).map_err(|e| Error::new(e.to_string()))
        };
        let mut b = Vec::new();
        b.extend_from_slice(b"{\"mediaType\":");
        b.extend(s(self.media_type.string())?);
        b.extend_from_slice(b",\"path\":");
        b.extend(s(&self.path)?);
        b.extend_from_slice(b",\"baseName\":");
        b.extend(s(&self.base_name)?);
        b.extend_from_slice(b",\"rel\":");
        b.extend(s(&self.rel)?);
        b.extend_from_slice(b",\"protocol\":");
        b.extend(s(&self.protocol)?);
        for (k, v) in [
            ("isPlainText", self.is_plain_text),
            ("isHTML", self.is_html),
            ("noUgly", self.no_ugly),
            ("ugly", self.ugly),
            ("notAlternative", self.not_alternative),
            ("root", self.root),
            ("permalinkable", self.permalinkable),
        ] {
            b.extend_from_slice(format!(",\"{k}\":{v}").as_bytes());
        }
        b.extend_from_slice(format!(",\"weight\":{}", self.weight).as_bytes());
        b.push(b'}');
        Ok(b)
    }
}

/// Go: `output.Formats` (sorted by `Formats.Less`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Formats(pub Vec<OutputFormat>);

/// Go: `Formats.Less`: weight > 0 first (ascending), then by name.
// Go: output/outputFormat.go:Less
fn formats_less(fi: &OutputFormat, fj: &OutputFormat) -> bool {
    if fi.weight == fj.weight {
        return fi.name < fj.name;
    }

    if fj.weight == 0 {
        return true;
    }

    fi.weight > 0 && fi.weight < fj.weight
}

impl Formats {
    /// Go: `sort.Sort(formats)` with `Less`: weight > 0 first (ascending), then by name
    /// (Go's pdqsort, so the order is Go's also where `Less` is not a strict ordering, e.g.
    /// with negative weights).
    // Go: output/outputFormat.go:Swap
    pub fn sort(&mut self) {
        go_sort::sort_by(&mut self.0, formats_less);
    }

    /// GetBySuffix gets a output format given as suffix, e.g. "html". It will return `None`
    /// if no format could be found, or if the suffix given is ambiguous. The lookup is case
    /// insensitive.
    pub fn get_by_suffix(&self, suffix: &str) -> Option<OutputFormat> {
        let (f, found) = self.get_by_suffix_found(suffix);
        found.then_some(f)
    }

    /// [`Formats::get_by_suffix`] with Go's results: when the suffix is ambiguous the first
    /// match is returned with `false`.
    // Go: output/outputFormat.go:GetBySuffix
    pub fn get_by_suffix_found(&self, suffix: &str) -> (OutputFormat, bool) {
        let mut f = OutputFormat::default();
        let mut found = false;
        for ff in &self.0 {
            for suffix2 in ff.media_type.suffixes() {
                if go_unicode::strings::equal_fold_str(suffix, &suffix2) {
                    if found {
                        // ambiguous
                        return (f, false);
                    }
                    f = ff.clone();
                    found = true;
                }
            }
        }
        (f, found)
    }

    /// GetByName gets a format by its identifier name (case-insensitive).
    // Go: output/outputFormat.go:GetByName
    pub fn get_by_name(&self, name: &str) -> Option<OutputFormat> {
        self.0
            .iter()
            .find(|ff| go_unicode::strings::equal_fold_str(name, &ff.name))
            .cloned()
    }

    /// GetByNames gets a list of formats given a list of identifiers.
    // Go: output/outputFormat.go:GetByNames
    pub fn get_by_names(&self, names: &[&str]) -> Result<Formats> {
        self.get_by_names_partial(names).map_err(|(_, e)| e)
    }

    /// [`Formats::get_by_names`] returning the formats found before the error, like Go.
    pub fn get_by_names_partial(
        &self,
        names: &[&str],
    ) -> std::result::Result<Formats, (Formats, Error)> {
        let mut types = Vec::new();

        for name in names {
            match self.get_by_name(name) {
                Some(tpe) => types.push(tpe),
                None => {
                    let e = Error::new(format!(
                        "OutputFormat with key {} not found",
                        go_strconv::quote(name)
                    ));
                    return Err((Formats(types), e));
                }
            }
        }
        Ok(Formats(types))
    }

    /// FromFilename gets a Format given a filename (`mytemplate.amp.html`, `mytemplate.html`,
    /// `mytemplate`).
    pub fn from_filename(&self, filename: &str) -> Option<OutputFormat> {
        let (f, found) = self.from_filename_found(filename);
        found.then_some(f)
    }

    /// [`Formats::from_filename`] with Go's results (an ambiguous suffix gives its first
    /// match with `false`).
    // Go: output/outputFormat.go:FromFilename
    pub fn from_filename_found(&self, filename: &str) -> (OutputFormat, bool) {
        let by_name = |name: &str| match self.get_by_name(name) {
            Some(f) => (f, true),
            None => (OutputFormat::default(), false),
        };
        let parts: Vec<&str> = filename.split('.').collect();
        let (mut ext, mut out_format) = ("", "");
        if parts.len() > 2 {
            out_format = parts[1];
            ext = parts[2];
        } else if parts.len() > 1 {
            ext = parts[1];
        }

        if !out_format.is_empty() {
            return by_name(out_format);
        }

        if !ext.is_empty() {
            let (f, found) = self.get_by_suffix_found(ext);
            if !found && parts.len() == 2 {
                // For extensionless output formats (e.g. Netlify's _redirects) we must fall
                // back to using the extension as format lookup.
                return by_name(ext);
            }
            return (f, found);
        }
        (OutputFormat::default(), false)
    }
}

/// Built-in output formats (Go: `output.AMPFormat`, `CalendarFormat`, ... `HTTPStatus404HTMLFormat`).
/// See specs/architecture-core.md §5.6 for the table.
pub struct BuiltinFormats {
    pub amp: OutputFormat,
    pub calendar: OutputFormat,
    pub css: OutputFormat,
    pub csv: OutputFormat,
    pub html: OutputFormat,
    pub alias_html: OutputFormat,
    pub markdown: OutputFormat,
    pub json: OutputFormat,
    pub web_app_manifest: OutputFormat,
    pub robots_txt: OutputFormat,
    pub rss: OutputFormat,
    pub sitemap: OutputFormat,
    pub sitemap_index: OutputFormat,
    pub gotmpl: OutputFormat,
    pub http_status_404_html: OutputFormat,
}

fn fmt(name: &str, media_type: &MediaType) -> OutputFormat {
    OutputFormat {
        name: name.to_string(),
        media_type: media_type.clone(),
        ..Default::default()
    }
}

/// Go: the package-level format vars.
pub fn builtin_formats() -> &'static BuiltinFormats {
    static F: OnceLock<BuiltinFormats> = OnceLock::new();
    F.get_or_init(|| {
        let b = builtin();
        BuiltinFormats {
            amp: OutputFormat {
                base_name: "index".into(),
                path: "amp".into(),
                rel: "amphtml".into(),
                is_html: true,
                permalinkable: true,
                // See https://www.ampproject.org/learn/overview/
                ..fmt("amp", &b.html_type)
            },
            calendar: OutputFormat {
                is_plain_text: true,
                protocol: "webcal://".into(),
                base_name: "index".into(),
                rel: "alternate".into(),
                ..fmt("calendar", &b.calendar_type)
            },
            css: OutputFormat {
                base_name: "styles".into(),
                is_plain_text: true,
                rel: "stylesheet".into(),
                not_alternative: true,
                ..fmt("css", &b.css_type)
            },
            csv: OutputFormat {
                base_name: "index".into(),
                is_plain_text: true,
                rel: "alternate".into(),
                ..fmt("csv", &b.csv_type)
            },
            html: OutputFormat {
                base_name: "index".into(),
                rel: "canonical".into(),
                is_html: true,
                permalinkable: true,
                // Weight will be used as first sort criteria. HTML will, by default, be
                // rendered first, but set it to 10 so it's easy to put one above it.
                weight: 10,
                ..fmt("html", &b.html_type)
            },
            // Alias is the output format used for alias redirects.
            alias_html: OutputFormat {
                is_html: true,
                ugly: true,
                permalinkable: false,
                ..fmt("alias", &b.html_type)
            },
            markdown: OutputFormat {
                base_name: "index".into(),
                rel: "alternate".into(),
                is_plain_text: true,
                ..fmt("markdown", &b.markdown_type)
            },
            json: OutputFormat {
                base_name: "index".into(),
                is_plain_text: true,
                rel: "alternate".into(),
                ..fmt("json", &b.json_type)
            },
            web_app_manifest: OutputFormat {
                base_name: "manifest".into(),
                is_plain_text: true,
                not_alternative: true,
                rel: "manifest".into(),
                ..fmt("webappmanifest", &b.web_app_manifest_type)
            },
            robots_txt: OutputFormat {
                base_name: "robots".into(),
                is_plain_text: true,
                root: true,
                rel: "alternate".into(),
                ..fmt("robots", &b.text_type)
            },
            rss: OutputFormat {
                base_name: "index".into(),
                no_ugly: true,
                rel: "alternate".into(),
                ..fmt("rss", &b.rss_type)
            },
            sitemap: OutputFormat {
                base_name: "sitemap".into(),
                ugly: true,
                rel: "sitemap".into(),
                ..fmt("sitemap", &b.xml_type)
            },
            sitemap_index: OutputFormat {
                base_name: "sitemap".into(),
                ugly: true,
                root: true,
                rel: "sitemap".into(),
                ..fmt("sitemapindex", &b.xml_type)
            },
            gotmpl: OutputFormat {
                is_plain_text: true,
                not_alternative: true,
                ..fmt("gotmpl", &b.gotmpl_type)
            },
            // I'm not sure having a 404 format is a good idea, for one, we would want to have
            // multiple formats for this.
            http_status_404_html: OutputFormat {
                not_alternative: true,
                ugly: true,
                is_html: true,
                permalinkable: true,
                ..fmt("404", &b.html_type)
            },
        }
    })
}

/// Go: `output.DefaultFormats` (sorted by the package `init`).
// Go: output/outputFormat.go:init
pub fn default_formats() -> Formats {
    static D: OnceLock<Formats> = OnceLock::new();
    D.get_or_init(|| {
        let b = builtin_formats();
        let mut f = Formats(vec![
            b.amp.clone(),
            b.calendar.clone(),
            b.css.clone(),
            b.csv.clone(),
            b.html.clone(),
            b.gotmpl.clone(),
            b.http_status_404_html.clone(),
            b.alias_html.clone(),
            b.json.clone(),
            b.markdown.clone(),
            b.web_app_manifest.clone(),
            b.robots_txt.clone(),
            b.rss.clone(),
            b.sitemap.clone(),
            b.sitemap_index.clone(),
        ]);
        f.sort();
        f
    })
    .clone()
}

/// Template API of `output.Format` (reached via `page.OutputFormat`'s embedded `Format` field and
/// printed as a struct with `%v`): exported fields; methods BaseFilename, IsZero, MarshalJSON.
impl go_value::Object for OutputFormat {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("output.Format")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "IsZero" | "BaseFilename" | "MarshalJSON")
    }
    fn call_method(
        &self,
        _ctx: go_value::HostCtx<'_>,
        name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "IsZero" => Some(Ok(Value::Bool(self.is_zero()))),
            "BaseFilename" => Some(Ok(Value::string(self.base_filename()))),
            "MarshalJSON" => Some(self.marshal_json_bytes().map_err(Into::into).map(|b| {
                Value::list(
                    go_value::SliceType::Uint8,
                    b.into_iter()
                        .map(|c| Value::Uint(u64::from(c), go_value::UintKind::Uint8))
                        .collect(),
                )
            })),
            _ => None,
        }
    }
    fn field(&self, name: &str) -> Option<Value> {
        Some(match name {
            "Name" => Value::string(self.name.as_str()),
            "MediaType" => self.media_type.to_value(),
            "Path" => Value::string(self.path.as_str()),
            "BaseName" => Value::string(self.base_name.as_str()),
            "Rel" => Value::string(self.rel.as_str()),
            "Protocol" => Value::string(self.protocol.as_str()),
            "IsPlainText" => Value::Bool(self.is_plain_text),
            "IsHTML" => Value::Bool(self.is_html),
            "NoUgly" => Value::Bool(self.no_ugly),
            "Ugly" => Value::Bool(self.ugly),
            "NotAlternative" => Value::Bool(self.not_alternative),
            "Root" => Value::Bool(self.root),
            "Permalinkable" => Value::Bool(self.permalinkable),
            "Weight" => Value::int(self.weight),
            _ => return None,
        })
    }
    fn is_zero(&self) -> Option<bool> {
        Some(self.is_zero())
    }
    fn struct_fields(&self) -> Option<Vec<(std::borrow::Cow<'_, str>, Value)>> {
        let names = [
            "Name",
            "MediaType",
            "Path",
            "BaseName",
            "Rel",
            "Protocol",
            "IsPlainText",
            "IsHTML",
            "NoUgly",
            "Ugly",
            "NotAlternative",
            "Root",
            "Permalinkable",
            "Weight",
        ];
        Some(
            names
                .iter()
                .map(|n| (Cow::Borrowed(*n), self.field(n).expect("field")))
                .collect(),
        )
    }
    fn marshal_json(&self) -> Option<go_value::Result<Vec<u8>>> {
        Some(self.marshal_json_bytes().map_err(Into::into))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: output/outputFormat.go (364 lines; 6/11 funcs executed)
//   types: Format, Formats
// OK L242-244: init()
// OK L250-250: (formats Formats) Len() int
// OK L251-251: (formats Formats) Swap(i, j int)
// OK L252-263: (formats Formats) Less(i, j int) bool
// OK L269-284: (formats Formats) GetBySuffix(suffix string) (f Format, found bool)
// OK L287-296: (formats Formats) GetByName(name string) (f Format, found bool)
// OK L299-310: (formats Formats) GetByNames(names ...string) (Formats, error)
// OK L313-340: (formats Formats) FromFilename(filename string) (f Format, found bool)
// OK L344-346: (f Format) BaseFilename() string
// OK L349-351: (f Format) IsZero() bool
// OK L355-364: (f Format) MarshalJSON() ([]byte, error)
// ---------------------------------------------------------------------------
