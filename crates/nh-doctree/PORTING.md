# nh-doctree — porting notes

neohugo hugolib/doctree (radix-tree walk order == BTreeMap byte order; language-dimension shifting).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `dimensions` | `hugolib/doctree/dimensions.go` | T27 doctree |  |
| `nodeshifttree` | `hugolib/doctree/nodeshifttree.go` | T27 doctree |  |
| `simpletree` | `hugolib/doctree/simpletree.go` | T27 doctree |  |
| `support` | `hugolib/doctree/support.go` | T27 doctree |  |
| `treeshifttree` | `hugolib/doctree/treeshifttree.go` | T27 doctree |  |

## Dependencies

- nh-*: nh-common (Go imports `resources/resource` only for `resource.MarkStale` in the rebuild-only
  Delete paths, which are not ported, so the dependency was dropped)
- Wave A (to add when available): none
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
