# Third-party material cargo cannot see

`tools/neohugo/licence-check.sh` checks every crate in the dependency graph. The material below
is not a crate licence of its own (data bundled inside a crate, files we embed), so its licence
texts and sources are kept here (REWRITE_PLAN.md §5). Each entry names the task that adds it.

| Directory | Material | Licence | Added by |
|---|---|---|---|
| `hugo/` | Hugo (gohugoio/hugo, forked as neohugo): the embedded templates rewritten in Tera (`crates/layouts/embedded/`) derive from Hugo's `tpl/tplimpl/embedded/templates` | Apache-2.0 (`hugo/LICENSE`) | T00 (licence), T32 (templates) |
| `two-face/` | Syntax definitions and themes bundled in the two-face 0.5.2 dump (bat's assets), each with its own licence; `ACKNOWLEDGEMENTS.md` is two-face's listing | per asset (MIT, Apache-2.0, BSD, …; `two-face/ACKNOWLEDGEMENTS.md`) | T25 |
| `cldr/` | Unicode CLDR 48.2.1 data compiled into the ICU4X 2.3 data crates (collation, plurals, numbers, dates) used by `crates/locale` | Unicode-3.0 (`cldr/LICENSE`) | T12 |
| `emoji/` | Emoji short-code data (comrak's `emojis` crate, from GitHub gemoji / Unicode) | MIT / Unicode-3.0 | T22 |
| `livereload/` | `livereload.js` served by `neohugo-rs server` | MIT | T71 |
| `flect/` | Inflection word lists and rules from gobuffalo/flect v1.0.3, transcribed into `crates/base/src/inflect.rs` | MIT (`flect/LICENSE`) | T10 |
| `prose/` | Title-case word lists and rules from jdkato/prose v1.2.1, transcribed into `crates/base/src/title.rs` | MIT (`prose/LICENSE`) | T10 |
