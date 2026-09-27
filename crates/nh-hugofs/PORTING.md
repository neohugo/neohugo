# nh-hugofs — porting notes

neohugo hugofs/*, hugolib/filesystems (BaseFs), hugolib/paths, modules (project module mounts), plus afero / bep/overlayfs / spf13/fsync semantics.

## Go file → Rust module

| Rust module | Go source(s) | Owner | Note |
|---|---|---|---|
| `afero` | — | T05 hugofs-vfs | NEW: spf13/afero subset: Fs trait, OsFs, BasePathFs, ReadOnlyFs, MemMapFs (tests) |
| `overlayfs` | — | T05 hugofs-vfs | NEW: bep/overlayfs subset (first-wins Stat/Open, merged ReadDir with DirsMerger) |
| `fs` | `hugofs/fs.go` | T05 hugofs-vfs |  |
| `fileinfo` | `hugofs/fileinfo.go` | T05 hugofs-vfs |  |
| `rootmapping_fs` | `hugofs/rootmapping_fs.go` | T05 hugofs-vfs |  |
| `component_fs` | `hugofs/component_fs.go` | T05 hugofs-vfs |  |
| `decorators` | `hugofs/decorators.go` | T05 hugofs-vfs |  |
| `dirsmerger` | `hugofs/dirsmerger.go` | T05 hugofs-vfs |  |
| `walk` | `hugofs/walk.go` | T05 hugofs-vfs |  |
| `hasbytes_fs` | `hugofs/hasbytes_fs.go` | T05 hugofs-vfs |  |
| `filename_filter_fs` | `hugofs/filename_filter_fs.go` | T05 hugofs-vfs |  |
| `glob` | `hugofs/glob.go` | T05 hugofs-vfs |  |
| `filesystems::basefs` | `hugolib/filesystems/basefs.go` | T05 hugofs-vfs |  |
| `paths` | `hugolib/paths/paths.go` | T05 hugofs-vfs |  |
| `modules::client` | `modules/client.go` | T09 allconfig-modules | only the project-module path (no go/npm) |
| `modules::collect` | `modules/collect.go` | T09 allconfig-modules |  |
| `modules::config` | `modules/config.go` | T09 allconfig-modules |  |
| `modules::module` | `modules/module.go` | T09 allconfig-modules |  |

## Dependencies

- nh-*: nh-common, nh-config, nh-media, nh-parser
- Wave A (to add when available): go-path
- crates.io (justify each): unicode-normalization (NFC of darwin file names)

## Deliberate deviations

_Wave B: list every deviation from the Go code here (README rule 1)._

## Known gaps

_Wave B: list unported / stubbed functionality here._
