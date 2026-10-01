# two-face: bundled syntax definitions and themes

`crates/highlight` tokenises code with the syntax set of the
[two-face](https://codeberg.org/CosmicHarper/two-face) crate, version `0.5.2+bat-0.26.1`
(`two_face::syntax::extra_newlines()`, the `fancy-regex` dump). The crate itself is
MIT OR Apache-2.0 and is checked by `tools/neohugo/licence-check.sh`; the dump inside it bundles
Sublime Text syntax definitions (and themes) curated by the [bat](https://github.com/sharkdp/bat)
project, each under its own licence, which cargo cannot see.

- `ACKNOWLEDGEMENTS.md` is two-face's own listing of those licences
  (`two_face::acknowledgement::listing().to_md()`, CRLF line ends written as LF). It
  covers every bundled syntax and theme whose licence requires acknowledgement: MIT, BSD-2/3-Clause, Apache-2.0 and
  similar permissive licences; no copyleft licence appears. The complete list (including
  licences that need no acknowledgement) is at `two_face::acknowledgement::url()`.
- The themes are not used: colours come from Chroma's own style files
  (`crates/highlight/src/styles/`, MIT, see `PROVENANCE.md`), so the theme dump is not linked
  into the binary.
- The Go template syntaxes in `crates/highlight/src/syntaxes/` are original (MIT) and are
  added to the set at run time.

Regenerate `ACKNOWLEDGEMENTS.md` after a two-face upgrade:

```sh
NEOHUGO_HL_ACK=$PWD/THIRD_PARTY/two-face/ACKNOWLEDGEMENTS.md \
  cargo test --offline -p neohugo-highlight --test it write_acknowledgements
```
