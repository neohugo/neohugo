//! Port of `resources/resource_transformers/tocss/scss/client.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! The `scss.Client`, `scss.Options` and `DecodeOptions` are declared in the sibling `tocss`
//! module (the skeleton's public items); this module holds the constructor and the regular
//! `@import "x.css"` protection of the entry file.

use std::sync::{Arc, LazyLock};

use nh_common::Result;
use nh_common::goregexp::Regexp;
use nh_hugofs::filesystems::basefs::SourceFilesystem;
use nh_resources::resource_spec::Spec;

use super::tocss::Client;

pub(crate) const TRANSFORMATION_NAME: &str = "tocss";

impl Client {
    /// Go: `scss.New(fs, rs)` (`workFs` = `rs.Work`).
    // Go: resources/resource_transformers/tocss/scss/client.go:New
    pub fn new(fs: Arc<SourceFilesystem>, rs: Arc<Spec>) -> Result<Client> {
        let work_fs = rs.path_spec.base_fs.work.clone();
        Ok(Client {
            sfs: fs,
            work_fs,
            rs,
        })
    }
}

static REGULAR_CSS_IMPORT_TO: LazyLock<Regexp> =
    LazyLock::new(|| Regexp::must_compile(r#".*(@import "(.*\.css)";).*"#));
static REGULAR_CSS_IMPORT_FROM: LazyLock<Regexp> = LazyLock::new(|| {
    Regexp::must_compile(r".*(\/\* HUGO_IMPORT_START (.*) HUGO_IMPORT_END \*\/).*")
});

// Go: resources/resource_transformers/tocss/scss/client.go:replaceRegularImportsIn
pub(crate) fn replace_regular_imports_in(s: &[u8]) -> (Vec<u8>, bool) {
    let replaced =
        REGULAR_CSS_IMPORT_TO.replace_all(s, b"/* HUGO_IMPORT_START $2 HUGO_IMPORT_END */");
    let changed = s != replaced.as_slice();
    (replaced, changed)
}

// Go: resources/resource_transformers/tocss/scss/client.go:replaceRegularImportsOut
pub(crate) fn replace_regular_imports_out(s: &[u8]) -> Vec<u8> {
    REGULAR_CSS_IMPORT_FROM.replace_all(s, b"@import \"$2\";")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_transformers/tocss/scss/client.go (93 lines; 3/4 funcs executed)
//   types: Client, Options
// OK L35-37: New(fs *filesystems.SourceFilesystem, rs *resources.Spec) (*Client, error)
// OK L68-79: DecodeOptions(m map[string]any) (opts Options, err error) (in `tocss`)
// OK L86-89: replaceRegularImportsIn(s string) (string, bool)
// OK L91-93: replaceRegularImportsOut(s string) string
// ---------------------------------------------------------------------------
