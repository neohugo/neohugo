//! Port of `langs/i18n/translationProvider.go`.
//!
//! Owner: Wave B task T17 (i18n).

//! Go `langs/i18n/translationProvider.go`: loads every file of the i18n component (Walkway,
//! sorted by name) into a go-i18n Bundle (default language tag first), then sets `Deps.translate`
//! for each site.

use std::sync::{Arc, Mutex};

use nh_common::Result;
use nh_common::herrors::Error;
use nh_deps::deps::Deps;
use nh_helpers::source::file_info::File;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_hugofs::walk::{Walkway, WalkwayConfig};
use xtext_collate::language as xl;

use crate::goi18n::bundle::Bundle;
use crate::i18n::Translator;

/// Go: `i18n.TranslationProvider` — translation handling, i.e. loading of bundles etc.
#[derive(Default)]
pub struct TranslationProvider {
    pub(crate) t: std::sync::OnceLock<Arc<crate::i18n::Translator>>,
}

impl TranslationProvider {
    /// Go: `NewTranslationProvider()`.
    // Go: langs/i18n/translationProvider.go:NewTranslationProvider
    pub fn new() -> Self {
        Self::default()
    }

    /// The translator built by [`TranslationProvider::new_resource`] (Go `tp.t`).
    pub fn translator(&self) -> Option<&Arc<Translator>> {
        self.t.get()
    }

    /// Go: `NewResource(dst)` — load i18n files and set `dst.Translate`.
    // Go: langs/i18n/translationProvider.go:NewResource
    pub fn new_resource(&self, dst: &Deps) -> Result<()> {
        let default_lang_tag = match xl::parse(&dst.conf.default_content_language()) {
            Ok(t) => t,
            Err(_) => xl::english(),
        };
        let bundle = Mutex::new(Bundle::new(default_lang_tag));

        let i18n_fs = dst.path_spec().base_fs.source_filesystems.i18n.fs.clone();
        // Go: `IgnoreFile: dst.SourceSpec.IgnoreFile` (tests may build Deps without a
        // SourceSpec; nothing is ignored then).
        let source_spec = dst.source_spec.clone();
        let ignore_file = move |filename: &str| match &source_spec {
            Some(ss) => ss.ignore_file(filename),
            None => false,
        };
        let mut walk_fn = |_path: &str, info: &FileMetaInfo| -> Result<()> {
            if info.is_dir() {
                return Ok(());
            }
            add_translation_file(&mut bundle.lock().unwrap(), &File::new(info.clone()))
        };
        let mut cfg = WalkwayConfig::new(i18n_fs, &mut walk_fn);
        cfg.ignore_file = Some(&ignore_file);
        cfg.path_parser = Some(dst.conf.path_parser());
        let mut w = Walkway::new(cfg);

        w.walk()?;

        let bundle = Arc::new(bundle.into_inner().unwrap());
        let t = Arc::new(Translator::new(bundle, &*dst.conf, dst.log.clone()));
        let f = t.func(&dst.conf.language().lang);
        // Go reassigns tp.t and dst.Translate; both are set once per build here.
        let _ = self.t.set(t);
        let _ = dst.translate.set(f);

        Ok(())
    }

    /// Go: `CloneResource(dst, src)` — other sites reuse the translator.
    // Go: langs/i18n/translationProvider.go:CloneResource
    pub fn clone_resource(&self, dst: &Deps, _src: &Deps) -> Result<()> {
        let t = self
            .t
            .get()
            .ok_or_else(|| Error::new("i18n: CloneResource called before NewResource"))?;
        let _ = dst.translate.set(t.func(&dst.conf.language().lang));
        Ok(())
    }
}

pub(crate) const ARTIFICIAL_LANG_TAG_PREFIX: &str = "art-x-";

// Go: langs/i18n/translationProvider.go:addTranslationFile
fn add_translation_file(bundle: &mut Bundle, r: &File) -> Result<()> {
    let mut f = r.file_info().meta().open().map_err(|err| {
        Error::new(format!(
            "failed to open translations file {}:: {}",
            go_strconv::quote(r.logical_name()),
            err.message()
        ))
    })?;

    let b = nh_helpers::general::reader_to_bytes(Some(&mut f));
    drop(f);

    let mut name = r.logical_name();
    let lang = nh_common::paths::path::filename(&name);
    let tag = xl::make(&lang);
    if tag == xl::und() {
        let try_ = format!("{ARTIFICIAL_LANG_TAG_PREFIX}{lang}");
        if let Err(err) = xl::parse(&try_) {
            return Err(Error::new(format!("{}: {}", go_strconv::quote(&try_), err)));
        }
        name = format!("{ARTIFICIAL_LANG_TAG_PREFIX}{name}");
    }

    if let Err(err) = bundle.parse_message_file_bytes(&b, &name) {
        let mut err = err;
        if err.message.contains("no plural rule") {
            // https://github.com/gohugoio/hugo/issues/7798
            name = format!("{ARTIFICIAL_LANG_TAG_PREFIX}{name}");
            match bundle.parse_message_file_bytes(&b, &name) {
                Ok(_) => return Ok(()),
                Err(e) => err = e,
            }
        }
        return Err(err_with_file_context(
            Error::new(format!("failed to load translations: {err}")),
            err.toml_position,
            r,
        ));
    }

    Ok(())
}

// Go: langs/i18n/translationProvider.go:errWithFileContext
/// The error as a file error of the translation file (Go also adds the file's content lines
/// around the position for display; the message is the same). `toml_position` is the position
/// of a wrapped `*toml.DecodeError`, which Go's `herrors.extractFileTypePos` prefers to the
/// positions in the message.
fn err_with_file_context(inerr: Error, toml_position: Option<(i64, i64)>, r: &File) -> Error {
    let meta = r.file_info().meta();
    let real_filename = meta.filename.clone();
    if meta.open().is_err() {
        return inerr;
    }
    match toml_position {
        Some((line, column)) => nh_common::herrors::new_file_error_from_pos(
            inerr,
            nh_common::herrors::FilePos {
                filename: real_filename,
                line,
                column,
            },
        ),
        None => nh_common::herrors::new_file_error_from_name(inerr, &real_filename),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: langs/i18n/translationProvider.go (139 lines; 4/5 funcs executed)
//   types: TranslationProvider
// OK L43-45: NewTranslationProvider() *TranslationProvider
// OK L48-82: (tp *TranslationProvider) NewResource(dst *deps.Deps) error
// OK L86-121: addTranslationFile(bundle *i18n.Bundle, r *source.File) error
// OK L124-127: (tp *TranslationProvider) CloneResource(dst, src *deps.Deps) error
// OK L129-139: errWithFileContext(inerr error, r *source.File) error
// ---------------------------------------------------------------------------
