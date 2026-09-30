# Third-party material cargo cannot see

`tools/neohugo/licence-check.sh` checks every crate in the dependency graph. The material below
is not a crate licence of its own (data bundled inside a crate, files we embed), so its licence
texts and sources are kept here (REWRITE_PLAN.md §5). Each entry names the task that adds it.

| Directory | Material | Licence | Added by |
|---|---|---|---|
| `hugo/` | Hugo (gohugoio/hugo, forked as neohugo): the embedded templates rewritten in Tera (`crates/layouts/embedded/`) derive from Hugo's `tpl/tplimpl/embedded/templates` | Apache-2.0 (`hugo/LICENSE`) | T00 (licence), T32 (templates) |
| `two-face/` | Syntax definitions and themes bundled in the two-face / syntect dumps (bat's assets), each with its own licence | per asset (MIT, Apache-2.0, BSD, …) | T25 |
| `cldr/` | Unicode CLDR data compiled into the ICU4X data crates | Unicode-3.0 | T12 |
| `emoji/` | Emoji short-code data (comrak's `emojis` crate, from GitHub gemoji / Unicode) | MIT / Unicode-3.0 | T22 |
| `livereload/` | `livereload.js` served by `neohugo-rs server` | MIT | T71 |
