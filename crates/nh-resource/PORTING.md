# nh-resource — porting notes

neohugo resources/resource (Resource interfaces, Resources, params, dates) and resources/internal (keys, target paths).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `resourcetypes` | `resources/resource/resourcetypes.go` | T11 page-api-paths |  |
| `resources` | `resources/resource/resources.go` | T11 page-api-paths |  |
| `dates` | `resources/resource/dates.go` | T11 page-api-paths |  |
| `params` | `resources/resource/params.go` | T11 page-api-paths |  |
| `resource_helpers` | `resources/resource/resource_helpers.go` | T11 page-api-paths |  |
| `internal::key` | `resources/internal/key.go` | T11 page-api-paths |  |
| `internal::resourcepaths` | `resources/internal/resourcepaths.go` | T11 page-api-paths |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-langs, nh-helpers
- Wave A (to add when available): none
- crates.io (justify each): none

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
