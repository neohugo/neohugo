//! `nh-doctree`: neohugo hugolib/doctree (radix-tree walk order == BTreeMap byte order; language-dimension shifting).
//!
//! Part of the neohugo Rust port (Hugo layer). See `crates/HUGO_LAYER.md` for the design and
//! `crates/WAVE_B_PLAN.json` for module ownership. Every module mirrors one or more Go files; the
//! GO PORTING CHECKLIST at the bottom of each module lists the Go functions it ports. The module
//! map, the deviations and the verification are in `PORTING.md`.
//!
//! - [`nodeshifttree::NodeShiftTree`]: the page and resource trees (`treePages`,
//!   `treeResources`) with their language [`nodeshifttree::Shifter`] and walks that may modify
//!   the walked tree (`walk_mut`).
//! - [`simpletree::SimpleTree`]: the template store trees and `WalkContext.Data()`.
//! - [`treeshifttree::TreeShiftTree`]: one `SimpleTree` per language (`treeTaxonomyEntries`).
//! - [`support`]: walk contexts and events, key validation.
//!
//! All of them sit on `radix`, a port of armon/go-radix: walks visit keys in byte order, and
//! prefix lookups are byte (character) level, not path-segment aware.

pub mod dimensions;
pub mod nodeshifttree;
mod radix;
pub mod simpletree;
pub mod support;
pub mod treeshifttree;
