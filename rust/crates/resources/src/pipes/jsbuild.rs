//! `js_build`: esbuild through `neohugo-esbuild` (T14), imports resolved in the assets view
//! before `node_modules`.

use neohugo_esbuild::{JsBuildOptions, Source};

use super::assets::AssetsView;
use super::{Output, PipeError, TransformEnv};
use crate::store::{Resource, ResourceStore};

pub(super) fn run(
    store: &ResourceStore,
    env: &TransformEnv,
    src: &Resource,
    o: &JsBuildOptions,
    input: &[u8],
) -> Result<Output, PipeError> {
    let builder = env.js_builder()?;
    let media_type = src.media_type_string();
    let source = Source {
        path: src.link.as_str(),
        media_type: &media_type,
        contents: input,
    };
    let out = builder.build(&AssetsView::new(store.cfg.vfs.as_deref()), &source, o)?;
    Ok(Output {
        bytes: out.code,
        source_map: out.source_map,
    })
}
