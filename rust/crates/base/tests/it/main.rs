//! Integration tests of `neohugo-base` (the crate's single test binary, REWRITE_PLAN.md §2.2).
//! The oracle tests print their check counts (`cargo test -p neohugo-base -- --nocapture`).

mod glob;
mod inflect;
mod paths;
mod pathspec;
mod support;
mod time;
mod urls;
mod value;
