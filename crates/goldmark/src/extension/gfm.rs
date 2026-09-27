// Go: github.com/yuin/goldmark@v1.7.12/extension/gfm.go

use super::{linkify, strikethrough, table, task_list};
use crate::{Extender, Markdown};

/// Go: `type gfm struct{}`.
pub struct GfmExt;

// Go: extension/gfm.go:GFM
/// GFM is an extension that provides Github Flavored markdown functionalities.
pub fn gfm() -> Box<dyn Extender> {
    Box::new(GfmExt)
}

impl Extender for GfmExt {
    // Go: extension/gfm.go:gfm.Extend
    fn extend(&self, m: &mut Markdown) {
        linkify().extend(m);
        table().extend(m);
        strikethrough().extend(m);
        task_list().extend(m);
    }
}
