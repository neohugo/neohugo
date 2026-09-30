//! Source maps written as files (`sourceMap: external | linked`): esbuild names sources by
//! module path; the published map names them by `file://` URL so browsers find them.

use std::path::Path;

use serde_json::Value as Json;

use crate::resolve::{NS_HUGO_IMPORT, STDIN, clean};

/// Rewrites the `sources` of an esbuild source map:
/// - `<stdin>` becomes the URL of `stdin_file` (the entry script's real file) when known;
/// - `ns-hugo-imp:<abs path>` (a module resolved as an asset) becomes the URL of that file;
/// - other plain paths are relative to `out_dir` and become the URL of the file they name;
/// - anything else (`ns-hugo-params:…`) is kept, so `sources` stays aligned with
///   `sourcesContent`.
///
/// The map is re-written with its five standard fields (`version`, `sources`,
/// `sourcesContent`, `mappings`, `names`).
///
/// # Errors
/// When the map is not JSON.
pub(crate) fn fix_sources(
    map: &[u8],
    stdin_file: Option<&Path>,
    out_dir: &Path,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut sm: Json = serde_json::from_slice(map)?;
    if let Some(Json::Array(sources)) = sm.get_mut("sources") {
        for s in sources.iter_mut() {
            if let Json::String(src) = s
                && let Some(url) = source_url(src, stdin_file, out_dir)
            {
                *src = url;
            }
        }
    }
    let field = |k: &str| sm.get(k).cloned().unwrap_or(Json::Null);
    let mut out = String::from("{");
    for (i, key) in ["version", "sources", "sourcesContent", "mappings", "names"]
        .into_iter()
        .enumerate()
    {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&serde_json::to_string(key)?);
        out.push(':');
        out.push_str(&serde_json::to_string(&field(key))?);
    }
    out.push('}');
    Ok(out.into_bytes())
}

fn source_url(src: &str, stdin_file: Option<&Path>, out_dir: &Path) -> Option<String> {
    if src == STDIN {
        return stdin_file.map(file_url);
    }
    if let Some(path) = src
        .strip_prefix(NS_HUGO_IMPORT)
        .and_then(|s| s.strip_prefix(':'))
    {
        return Some(file_url(Path::new(path)));
    }
    if src.starts_with("ns-hugo") || src.contains(':') {
        return None;
    }
    let joined = out_dir.join(src);
    Some(file_url(Path::new(&clean(
        &joined.to_string_lossy().replace('\\', "/"),
    ))))
}

/// A `file://` URL of an absolute path.
#[must_use]
pub fn file_url(path: &Path) -> String {
    let p = path.to_string_lossy().replace('\\', "/");
    let mut url = String::from("file://");
    if !p.starts_with('/') {
        url.push('/');
    }
    for b in p.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~/!$&'()*+,;=:@".contains(&b) {
            url.push(char::from(b));
        } else {
            url.push_str(&format!("%{b:02X}"));
        }
    }
    url
}
