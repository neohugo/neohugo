# nh-hugofs — porting notes

neohugo hugofs/*, hugolib/filesystems (BaseFs), hugolib/paths, modules (project module mounts), plus afero / bep/overlayfs / spf13/fsync semantics.

Crate lead: T05 (hugofs-vfs). `modules::*` belongs to T09 (allconfig-modules).

## Go file → Rust module

| Rust module | Go source(s) | Owner | Status / note |
|---|---|---|---|
| `afero` | `github.com/spf13/afero@v1.14.0` subset: `afero.go` (Fs, File), `os.go` (OsFs), `basepath.go` (BasePathFs, BasePathFile), `readonlyfs.go`, `memmap.go` + `mem/*` (MemMapFs, tests), `util.go`/`ioutil.go` (ReadFile, WriteFile, Exists, IsDir, ReadDir) | T05 | ported (deviations 4, 5) |
| `overlayfs` | `github.com/bep/overlayfs@v0.10.0` (`overlayfs.go`, `readops.go`, `writeops.go`) | T05 | ported in full (deviation 6) |
| `fs` | `hugofs/fs.go`, `hugofs/noop_fs.go` | T05 | ported; `MakeReadableAndRemoveAllModulePkgDir` STUB (module cache cleanup) |
| `fileinfo` | `hugofs/fileinfo.go` | T05 | ported (deviation 1) |
| `rootmapping_fs` | `hugofs/rootmapping_fs.go` (+ the `armon/go-radix` operations it uses) | T05 | ported |
| `component_fs` | `hugofs/component_fs.go` | T05 | ported |
| `decorators` | `hugofs/decorators.go` | T05 | ported |
| `dirsmerger` | `hugofs/dirsmerger.go` | T05 | ported |
| `walk` | `hugofs/walk.go` | T05 | ported (deviation 3) |
| `hasbytes_fs` | `hugofs/hasbytes_fs.go` | T05 | ported |
| `filename_filter_fs` | `hugofs/filename_filter_fs.go` | T05 | ported |
| `glob` | `hugofs/glob.go` | T05 | ported |
| `filesystems::basefs` | `hugolib/filesystems/basefs.go` (+ `rogpeppe/go-internal/lockedfile.MutexAt`) | T05 | ported (`printFs` is a debugging helper, not ported) |
| `paths` | `hugolib/paths/paths.go` | T05 | ported |
| `oserror` | Go `os.PathError`/`syscall.Errno` texts (`zerrors_linux_arm64.go`, `zerrors_darwin_arm64.go`) | T05 | NEW |
| `nfc` | `golang.org/x/text@v0.26.0/unicode/norm` `NFC.String` (Unicode 15.0.0) | T05 | re-export of `nh_common::text::norm::nfc_string` (deviation 7) |
| `modules::client` | `modules/client.go` | T09 | only the project-module path (no go/npm) |
| `modules::collect` | `modules/collect.go` | T09 | |
| `modules::config` | `modules/config.go` | T09 | |
| `modules::module` | `modules/module.go` | T09 | |

Every GO PORTING CHECKLIST entry of the T05 modules is `OK`.

## Dependencies

- nh-*: nh-common, nh-config, nh-media (`default_path_parser` for Walkway), nh-parser (skeleton).
- Wave A: go-value, go-path, go-sort (`sort.Slice` = pdqsort, for componentFs, sortDirEntries and
  Walkway's `SortDirEntries`), go-strconv (`%q` in error texts), go-unicode (`strings.ToLower`
  in Glob).
- crates.io: none. The darwin NFC normalization is nh-common's port of x/text's `norm`
  (`nh_common::text::norm`) instead of `unicode-normalization`, which carries a newer Unicode
  version than x/text v0.26.0 (15.0.0) and is not in the offline registry cache.
- dev: `serde_json`, `flate2` (`rust_backend`, gunzip of fixtures), nh-langs (the test
  `AllProvider` names `Language`).

## The modules seam (T05 ↔ T09)

`BaseFs` needs what `modules.Module` gives basefs.go: `Path()`, `Dir()`, `Mounts()` (the
effective list after `normalizeMounts`, `mountCommonJSConfig` and `filterUnwantedMounts`),
`Owner()` (`nil` for the project) and `Watch()`, in module order (the ordinal is the index). The
input type is the existing `modules::module::Module` data struct (fields `path`, `dir`,
`mounts: Vec<modules::config::Mount>`, `owner: Option<Arc<Module>>`, `watch`; the other fields
are not read) and `Modules = Vec<Arc<Module>>`. BaseFs reads it the way Go does:
`Paths::all_modules()` downcasts `cfg.get_config_section("allModules")` to `Modules`.

- `Mount.include_files` / `exclude_files` are Go's `any` after `types.ToStringSlicePreserveString`
  (a string is a one-element list). An empty `Vec` stands for Go's nil; Go's explicitly empty
  list gives a filter that matches everything, which behaves the same.
- T09 must register the `allModules` section as `Arc<Modules>` and fill those fields; nothing
  else of `Module`/`Mount` is used here (`Mount::component` stays T09's).
- Tests build the modules from the Go oracle's recording of `confs.Modules` (`tests/support`:
  `build_modules`, `TestCfg`), so T05's acceptance does not depend on T09.

## Go behaviour reproduced on purpose

- Mount weights `(10+ordinal)*(len(mounts)-i)`, `Weight++` for a language in the file name,
  first-wins overlays, `LanguageDirsMerger` (name + lang), `AppendDirsMerger` (data, i18n,
  `AssetsWithDuplicatesPreserved`), the union-dir merger of `newUnionFile` (a file named like a
  directory of an earlier mount is dropped), single-file mounts as renaming directory mounts.
- componentFs: symlinks dropped (their `IsDir` is false and their lstat mode has
  `ModeSymlink`), disabled languages dropped, the ReadDir sort (dirs first, module ordinal —
  reversed for i18n — bundles first for content, extension descending, base, weight, name),
  `applyMeta` returning the unmodified entry for `Stat` of a disabled path, and the panic
  `no language found for <lang>`.
- `includeFiles` filters matched against `strings.TrimPrefix(Filename, SourceRoot)` (so
  `**/*.md` does not match a file at the mount root), `statRoot` matching with `isDir = true`.
- Byte-prefix radix semantics (`LongestPrefix`, `WalkPrefix`): a mount `/site/assets` is a
  prefix of `/site/assetsX/...` in `ReverseLookup`; `collectDirEntries` opens nested virtual
  directories with the prefix-relative path.
- Walkway: `walk: stat: %s` and `walk: open: path: %q filename: %q: %s` are not wrapped (not
  `IsNotExist`); `walk: Readdir: %w` is; a missing root is not an error unless
  `FailOnNotExist`; the root dir info of a component view has `Filename` = the component name.
- Error texts: `LStat <name>: file does not exist`, `file does not exist`,
  `stat <abs>: no such file or directory`, `readdirent <abs>: not a directory`,
  `stat ../outside: file does not exist` (BasePathFs), `could not determine content directory
  for "<name>"`, `decorate: ...`.
- hasBytesFs: the file name reported is the BasePathFile name (`/sub/page.html`), files opened
  read-only are not scanned, and nh-common's `HasBytesWriter` keeps Go's quirks.

## Deliberate deviations

1. **`*FileMeta` aliasing.** Go mutates `*FileMeta` in place (`Merge`, `applyMeta`, the
   walker's `PathInfo`, `Rename`d names). A `FileMetaInfo` here owns an `Arc<FileMeta>` updated
   copy-on-write (`FileMetaInfo::meta_mut`). Go only mutates metas created for the entry at hand
   (decorators, `collectDirEntries`, `newDirNameOnlyFileInfo` copy), so no Go-visible sharing is
   lost; the one exception would be a caller passing the same `Arc<FileMeta>` to several
   `RootMapping`s (Go would mutate the shared meta for each), which basefs never does.
   `RootMapping`'s identity in `getRoots` (`seen[rm]`) is the meta pointer.
2. **Plain file infos carry a meta.** Go's `os.FileInfo`s are not `FileMetaInfo`s; here every
   file info has an (empty) meta, which `Merge` treats like Go's missing one. Go's lazy
   `DirEntry.Info()` (an lstat) is done eagerly by `OsFile::read_dir`; a failure is kept and
   returned by `FileMetaInfo::info()` as in Go.
3. **Sentinel errors.** nh-common's error model has no identity, so Go's `filepath.SkipDir`,
   `errIsDir`, Glob's `done` and `io.EOF` are recognised by kind (`Generic`) and exact text
   (`walk::skip_dir`/`is_skip_dir`, `"isDir"`, `"done"` plus a flag, `"EOF"`). A user error with
   the text `skip this directory` would also skip (see "Requests").
4. **OsFs.** File names that are not UTF-8 are converted lossily (`String` paths, as in
   nh-common). `Chown` returns an explicit unsupported error. `RemoveAll` and `Rename` errors use
   the errno text with a simplified op. The OS directory order is the kernel's (`read_dir`),
   the same order Go's `File.ReadDir` gets on the same directory.
5. **MemMapFs** (tests only): parents are created on demand, no uid/gid, directory listings
   are snapshotted at `Open`, `Rename` moves descendants without Go's `findDescendants` order.
6. **overlayfs.** `Dir` has no `sync.Pool`; `Read`/`Write`/`Seek` on a directory return an
   error where Go panics (`noOpRegularFileOps`, `Dir.notSupported`); a partial `ReadDir(n)` past
   the end stops at the end where Go reslices into stale capacity (hugofs always reads -1).
7. **NFC.** `nfc::nfc_string` re-exports `nh_common::text::norm::nfc_string`, a
   function-by-function port of x/text's `unicode/norm` (NFC/NFD, stream-safe CGJ insertion,
   `Transform`) over x/text's own trie. It moved to nh-common with its data and oracle
   (`tools/go-oracle/nh-common/norm`, which checks it on every code point and 77,726 fixture
   cases; see nh-common's PORTING.md). Only darwin builds call it.
8. **Go panics as errors** (rule 9): `NewRootMappingFs` with a too short `To` or an empty
   component (`invalid root mapping; from/to: ...`, ` rm.FromBase is empty`), `noOpFs.Create`/
   `Rename`/`Chmod`/`Chtimes`/`Chown`. Kept as panics (invariants): `NewComponentFs` without a
   component, `applyMeta`'s `no language found for`, `newFs` with an empty publishDir or a too
   short workingDir, a Walkway walked twice, `MakePathRelative`/`ResolvePaths` on a lookup error.
9. **Build lock.** `LockBuild` uses `std::fs::File::lock` (flock) on `.hugo_build.lock`, like
   `lockedfile.MutexAt`; the in-process mutex is used when `noBuildLock` is set (Go also when
   running as a test).
10. **Map order.** `MakeStaticPathRelative` and `IsStatic` iterate the static filesystems in
    byte order of the language key (Go: random map order; one entry unless multihost).
11. `AddFileInfoToError` sets the file position only (no source excerpt; nh-common deviation 9).
12. `BaseFs` derefs to `SourceFilesystems` (Go embeds `*SourceFilesystems`); `WithBaseFs` is
    `BaseFs::new_with_base`. `SourceFilesystem::make_path_relative` returns `Option` (Go's
    `(string, bool)`), `SourceFilesystems::stat_resource` returns `(Result, fs)`.
13. `Readdir(count)` (the `os.FileInfo` form) panics in Go on every hugofs directory and is not
    ported; `read_dir`/`readdirnames` are.

## Known gaps

- `MakeReadableAndRemoveAllModulePkgDir` (Go module cache cleanup) and `printFs` are not ported.
- The fsync static copy (`spf13/fsync`) lives in nh-commands (T25).

## Requests to other crates

- **nh-common (T01):** an `ErrorKind::SkipDir` (Go's `filepath.SkipDir` identity) would replace
  the text match of deviation 3; walkers in other crates must return `nh_hugofs::walk::skip_dir()`
  meanwhile.
- **T09:** fill `Module.{path, dir, mounts, owner, watch}` and register `allModules` as
  `Arc<Modules>` (see the seam above).
- **T20 (deps):** wrap `Fs.publish_dir` with `hasbytes_fs::new_has_bytes_receiver(publish_dir,
  should_check, callback, vec![b"__hdeferred/", b"__h_pp_l1"])` where `should_check(name)` is
  `media_types.is_text_suffix(filepath.Ext(name) without the dot)`; the order of the patterns
  is Go's.

## Verification

Go oracles in `tools/go-oracle/nh-hugofs/` write `tests/fixtures/<topic>/`; `cargo test` needs
neither Go nor the network. The seeksnack site is private; the inputs are this repository's
`docs` site and `hugolib/testsite` plus synthetic trees, recreated in a temporary directory by
both the oracle and the test (file names only; configuration files keep their content).

| topic | inputs | Rust test | checks |
|---|---|---|---|
| `walk/*.json.gz` | 6 sites: `docs` (1,367 tree entries, 10 mounts incl. a single-file mount and the auto `_jsconfig` mounts), `testsite` (en + nn with `contentDir`), `multilang` (en/th/fr with `contentDir` per language, fr disabled, `foo.th.md`, bundles, NFC and NFD `café`, Thai, hidden files, `#`/`~` names, file/dir/dangling symlinks, empty dirs), `mounts` (a theme shadowing the project, overlapping content mounts with `includeFiles`/`excludeFiles`, `node_modules` → `assets/vendor`, a single-file mount, static from 5 mounts incl. a symlinked source and a `th` mount, nested `layouts/partials/extra`, data/i18n duplicates, a missing source), `multihost` (static per language), `empty`; config loaded by `allconfig.LoadConfig`, modules and PathParser answers recorded | `tests/walk.rs` | 48,974 cases: the Walkway walk of every component view (content, data, i18n, layouts, archetypes, assets, assets with duplicates, static per key) with path, Name, Filename, PathInfo.Path(), Lang, LangIndex, Weight, ModuleOrdinal, IsDir, Component, Module, IsProject, Watch, BaseDir, SourceRoot, meta Name, Type; Stat and Open+ReadDir of every walked path and 9 odd/missing names on every view, every RootMappingFs (mount roots, their children, missing paths) and the source/work fss; `Mounts` per component; `MakePathRelative` (check on/off) and `Contains` for every tree file on every view; `ResolvePaths`, `IsContent`, `IsStatic`, `MakeStaticPathRelative`, `RealDirs`, `ResolveJSConfigFile`, `AbsProjectContentDir`, `WatchFilenames`, `StatResource`, `Glob` (17 patterns, early stop, missing root, handle error); every PathParser callback replayed (a call Go never made fails) |
| `hasbytes/hasbytes.json.gz` | 1,500 seeded writes of 30 file names (text and binary suffixes, dots, Thai) through `Create`, `OpenFile` (write, read-write, read-only), content from pattern fragments split at random chunk boundaries | `tests/hasbytes.rs` | the callbacks (name, pattern) in order, the bytes on disk, `shouldCheck` against nh-media's `IsTextSuffix` |

Go's tests are ported in `tests/go_tables.rs`: `TestIsOsFs`, `TestNewDefault`, `TestFileMeta`,
`TestWalk`, `TestWalkRootMappingFs` (incl. the parallel run), `TestGlob`,
`TestFilenameFilterFs`, `TestLanguageRootMapping`, `TestRootMappingFsDirnames`,
`TestRootMappingFsFilename`, `TestRootMappingFsMount`, `TestRootMappingFsMountOverlap`,
`TestRootMappingFsOs`, `TestRootMappingFsOsBase`, `TestRootMappingFileFilter`
(basefs_test.go is covered by the walk oracle, which builds BaseFs from real configurations).

Results: 0 differences. Directory order: the component views sort their entries, so their walks
are compared in order. The static filesystem, `RootMappingFs.ReadDir` and the source filesystem
return the OS directory order of the temporary tree, which differs between filesystems; those
results are recorded and compared as sorted lists (`"sorted": true`).

Nothing here depends on the CPU architecture (the errno table differs only in entry 133). The
fixtures regenerate byte for byte on linux/amd64:

```sh
export GOTOOLCHAIN=go1.27.1
go run ./tools/go-oracle/nh-hugofs/walk -root . -out crates/nh-hugofs/tests/fixtures/walk
go run ./tools/go-oracle/nh-hugofs/hasbytes -out crates/nh-hugofs/tests/fixtures/hasbytes
```

The `docs` fixture records this repository's `docs/` tree (without `docs/rust-port`), so it
changes when that tree does.
