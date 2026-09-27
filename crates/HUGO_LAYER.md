# neohugo Hugo layer (`nh-*` crates): design

This is the design of the Hugo layer of the Rust port: the 25 `nh-*` crates that sit on top of the
Wave A crates (Go stdlib clones, goldmark, tdewolff, gift, gotemplate, x/text collate, libwebp,
libsass). The goal is the one in `crates/README.md`: `neohugo --minify --clock 2026-09-27T12:00:00Z`
must build the seeksnack site into output that is byte-identical to the Go build
(`$SCRATCH/golden/canonical`, 6,943 files, plus `hugo_stats.json`).

Companion files:

- `crates/WAVE_B_PLAN.json`: the Wave B work breakdown. Each task owns a disjoint set of modules,
  lists the Go sources it ports, what it depends on, and its oracle and acceptance tests.
- `crates/GOTEMPLATE_CONTRACT.md`: the pinned host contract between the Wave A `gotemplate` crate
  and the Hugo layer (acceptance criteria for gotemplate, reviewed by T13; §6.1).
- `crates/nh-*/`: the compiling skeleton. Every module file states the Go file(s) it ports and the
  owning task, holds the core type declarations, `todo!()` bodies with `// Go: <file>:<Func>`
  markers, and ends with a generated **GO PORTING CHECKLIST**. The checklist lists every Go function
  in the source file with its line range. `EX` marks functions the seeksnack build executes (from a
  coverage-instrumented Go run).
- `crates/nh-*/PORTING.md`: per-crate Go file → Rust module table. Wave B fills in the deviations
  and known gaps.
- Specs this design builds on (in the scratchpad `specs/`): `architecture-core.md`,
  `content-model.md`, `template-engine.md`, `templates-inventory.md`, `markdown.md`, `minify.md`,
  `images.md`, `resources-pipeline.md`, `output-publishing.md`, `i18n-lang-misc.md`. Section numbers
  quoted below (e.g. "AC §3.6") refer to them: AC = architecture-core, CM = content-model,
  TE = template-engine, TI = templates-inventory, MD = markdown, IMG = images, RP =
  resources-pipeline, OP = output-publishing, I18N = i18n-lang-misc.

`$SCRATCH` = `/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad`,
`$W` = `$SCRATCH/work`.

---

## 1. Rules for Wave B

1. `crates/README.md` rules 1–13 apply unchanged. In particular: port function by function,
   `// Go: <path>:<Func>` above every ported function, Go's sorting/unicode/integer semantics,
   and never substitute a crate for Go logic that reaches output bytes.
2. **Ownership is by module, not by crate.** Several crates are shared by more than one task
   (nh-common, nh-page, nh-hugofs, nh-tplfuncs, nh-resource-transformers, nh-hugolib). A task edits
   only the module files it owns (see the `Owner:` line at the top of each file and
   `crates_or_modules_owned` in the plan). `lib.rs`, `Cargo.toml` and `PORTING.md` of a shared crate
   belong to the task listed first for that crate in the plan (the "crate lead"). Other tasks send
   dependency additions to the lead, or add them in their final report for the integrator.
3. **The skeleton's public items are a contract.** You may add items, fill bodies, and add private
   helpers. You may change the signature of a skeleton item only if no other crate uses it
   (`grep -rn <name> crates/nh-*`). Otherwise, add a new item next to it and report the requested
   change. Integration task I01 reconciles the remaining mismatches.
4. **Build and test each crate on its own:**
   `cd crates/<crate> && CARGO_TARGET_DIR=$SCRATCH/targets/<crate> cargo test --offline`. Every
   crate is its own workspace (empty `[workspace]`) and depends on its siblings by path. Wave A
   dependencies are listed in each `Cargo.toml`. The ones whose crate has LANDED (committed to the
   branch) and passes `cargo check` are already uncommented: go-value, go-unicode, go-strconv,
   go-time, go-url, go-path, go-sort, go-html, go-image, go-yaml, go-hashstructure,
   xtext-collate, tdewolff-parse. The others stay commented until their crate lands (a directory
   that exists but is still being written by a Wave A agent does not count); the crate lead then
   uncomments the line. Until then, keep the call site as `todo!("WAVE-A <crate>: <what>")` so
   `grep WAVE-A` finds it.
5. **Scope = seeksnack.** Everything on the executed path (`EX` in the checklists) is ported
   faithfully. Features the build never reaches (asciidoc/pandoc/rst/org, chroma highlighting,
   server/watch, deploy, getJSON, smartcrop, dartsass, babel, tailwind, …) must return an explicit
   error `Error::new("neohugo-rs: <feature> is not supported")`. They must never be silently
   approximated. The checklists mark those modules `STUB`.
6. **Oracles.** Each task writes `tools/go-oracle/<crate>/<topic>/main.go`. It lives inside the
   neohugo Go module so it imports the exact Go code and dependency versions. It writes JSON
   fixtures to `crates/<crate>/tests/fixtures/<topic>/`, and the Rust tests live in
   `crates/<crate>/tests/<topic>.rs`. Reuse the artifacts in `$W/**` (listed per task in the plan).
   `cargo test` must not need Go or the network.
7. **The skeleton was generated once** (from scratchpad scripts). Edit the files in place and
   never regenerate them. Keep the GO PORTING CHECKLIST block at the bottom of each module and
   mark ported entries by changing their `EX`/blank prefix to `OK` (e.g. `// OK L115-117: ...`),
   so the remaining work stays greppable (`grep -rn '^// EX ' crates/nh-*`).
8. **Skeleton-time lint allowances** (`#![allow(unused, dead_code, …)]` in each `lib.rs`) are
   removed by task I04. Do not add new blanket allows.
9. **Test dependencies.** Every nh-* crate has `serde_json` as a `[dev-dependency]` (the reader for
   the Go-oracle JSON fixtures). Any other test-only crate goes through the crate lead like a
   normal dependency.
10. **Never compute while holding a lock** (§4.8). Rendering re-enters templates, other pages and
    the same page's other scopes.
11. **Test seams are contracts too:** `nh_deps::deps::Deps::for_tests` (T15, T17, T18, T19),
    `nh_hugolib::hugo_sites::NewHugoSitesCfg::func_map_factory` (T20),
    `nh_hugolib::template_exec::TemplateExecutor` (T22, T24), and the minimal test FuncMap of
    T13. They exist so that a task's acceptance test needs only the tasks in its `depends_on`
    (§12.1). Keep them working.

---

## 2. Crate map

Go packages and files ported per crate. Layer = longest path to `nh-common` in the dependency
graph. Tasks = the Wave B tasks that own modules in the crate; the first one listed is the crate
lead.

| crate | layer | responsibility | Go sources | Wave B tasks |
|---|---|---|---|---|
| `nh-common` | 0 | neohugo common/* + compare + resources/kinds + hugofs/files + hugofs/glob + identity(stub) + cache/dynacache, plus ports of spf13/cast, gobuffalo/flect, jdkato/prose/transform, gohugoio/locales (en, th) | `common/herrors`, `common/constants`, `common/collections`, `common/hashing`, `common/hreflect`, `common/htime`, `common/maps`, `common/math`, `common/predicate`, `common/types`, `common/types/hstring`, `common/types/css`, `compare`, `identity`, `cache/dynacache`, `common/paths`, `common/urls`, `common/text`, `common/hstrings`, `common/hugio`, `common/loggers`, `resources/kinds`, `hugofs/files`, `hugofs/glob`; new/ported-3rd-party modules: `object`, `cast/caste`, `cast/time`, `locales`, `flect`, `prose` | T01, T26, T02 |
| `nh-langs` | 1 | neohugo langs/{language,config}.go: Language, Languages, collators (x/text collate CLDR 23), locale translators | `langs` | T03 |
| `nh-parser` | 1 | neohugo parser/* : metadecoders (YAML via go-yaml, TOML via toml crate + go-toml/v2 type mapping, JSON via go-json), pageparser lexer | `parser/metadecoders`, `parser/pageparser`, `parser`; new/ported-3rd-party modules: `metadecoders/toml` | T03 |
| `nh-config` | 2 | neohugo config (base), config/{security,privacy,services}, common/hexec, common/neohugo | `config`, `config/security`, `config/privacy`, `config/services`, `common/hexec`, `common/neohugo`; new/ported-3rd-party modules: `decode` | T04 |
| `nh-tpl` | 2 | neohugo tpl/template.go: template-execution context (context.Context replacement), CurrentTemplateInfo, StripHTML | `tpl`, `common/hcontext` | T13 |
| `nh-media` | 3 | neohugo media/* and output/* (media types, output formats) | `media`, `output` | T04 |
| `nh-hugofs` | 4 | neohugo hugofs/*, hugolib/filesystems (BaseFs), hugolib/paths, modules (project module mounts), plus afero / bep/overlayfs / spf13/fsync semantics | `hugofs`, `hugolib/filesystems`, `hugolib/paths`, `modules`; new/ported-3rd-party modules: `afero`, `overlayfs` | T05, T09 |
| `nh-markup` | 4 | neohugo markup/**: converter + hooks API, goldmark glue (render hooks, autoid, attributes, tables, blockquotes, hugocontext, images, TOC), highlight config (+ stub highlighter), markup_config; asciidoc/pandoc/rst/org stubs | `markup`, `markup/converter`, `markup/converter/hooks`, `markup/markup_config`, `markup/tableofcontents`, `markup/highlight`, `markup/internal/attributes`, `markup/goldmark`, `markup/goldmark/goldmark_config`, `markup/goldmark/blockquotes`, `markup/goldmark/codeblocks`, `markup/goldmark/hugocontext`, `markup/goldmark/images`, `markup/goldmark/tables`, `markup/goldmark/passthrough`, `markup/goldmark/internal/extensions/attributes`, `markup/goldmark/internal/render`, `markup/asciidocext`, `markup/pandoc`, `markup/rst`, `markup/org`, `markup/blackfriday` | T06 |
| `nh-transform` | 4 | neohugo transform/{chain,urlreplacers} and minifiers/* (tdewolff registry by media type) | `transform`, `transform/urlreplacers`, `transform/livereloadinject`, `transform/metainject`, `minifiers` | T07 |
| `nh-helpers` | 5 | neohugo helpers/*, source/*, cache/filecache, cache/httpcache (+ gohugoio/httpcache + net/http response-dump subset) | `helpers`, `source`, `cache/filecache`, `cache/httpcache`; new/ported-3rd-party modules: `cache/httpcache/transport` | T08 |
| `nh-publisher` | 6 | neohugo publisher/* (DestinationPublisher, transformer chain order, htmlElementsCollector) + x/net/html subset | `publisher`; new/ported-3rd-party modules: `xnethtml` | T07 |
| `nh-resource` | 6 | neohugo resources/resource (Resource interfaces, Resources, params, dates) and resources/internal (keys, target paths) | `resources/resource`, `resources/internal` | T11 |
| `nh-doctree` | 1 | neohugo hugolib/doctree (radix-tree walk order == BTreeMap byte order; language-dimension shifting) | `hugolib/doctree` | T27 |
| `nh-images` | 7 | neohugo resources/images/** (image config, spec parsing, processing, filters, encoders, exif stub) | `resources/images`, `resources/images/exif`, `resources/images/webp` | T10 |
| `nh-page` | 7 | neohugo resources/page/** (Page/Site traits, Pages ops, paths, permalinks, pagination, taxonomies), pagemeta, navigation, related | `resources/page`, `resources/page/pagemeta`, `navigation`, `related` | T11, T12 |
| `nh-allconfig` | 8 | neohugo config/allconfig (load, decode all sections, compile, per-language configs), hugolib/segments, deploy/deployconfig stub | `config/allconfig`, `hugolib/segments`, `deploy/deployconfig` | T09 |
| `nh-tplimpl` | 8 | neohugo tpl/tplimpl/** (template store, layout lookup, baseof, AST transforms, exec helper) + embedded templates; engine.rs is the single adaptation point to the Wave A gotemplate crate | `tpl/tplimpl`; new/ported-3rd-party modules: `engine`, `embedded` | T13 |
| `nh-resources` | 9 | neohugo resources/*.go (Spec, genericResource, resourceAdapter + transformation chain, caches, image resource), resources/postpub, resources/jsconfig | `resources`, `resources/postpub`, `resources/jsconfig` | T14 |
| `nh-deps` | 10 | neohugo deps/deps.go: per-site dependency container (the Rust context struct) | `deps` | T20 |
| `nh-esbuild` | 10 | neohugo internal/js + internal/js/esbuild (options, build client, resolve plugins) over the pinned esbuild 0.25.6 binary (--service protocol) | `internal/js`, `internal/js/esbuild`; new/ported-3rd-party modules: `service/protocol`, `service/client` | T16 |
| `nh-i18n` | 11 | neohugo langs/i18n + gohugoio/go-i18n/v2 fork (bundle, localizer, message templates, CLDR plural rules) | `langs/i18n`; new/ported-3rd-party modules: `goi18n/bundle`, `goi18n/localizer`, `goi18n/message`, `goi18n/parse`, `goi18n/template`, `goi18n/plural` | T17 |
| `nh-resource-transformers` | 11 | neohugo resources/resource_factories/{create,bundler} and resources/resource_transformers/{integrity,minifier,templates,js,cssjs,tocss} | `resources/resource_factories/create`, `resources/resource_factories/bundler`, `resources/resource_transformers/integrity`, `resources/resource_transformers/minifier`, `resources/resource_transformers/templates`, `resources/resource_transformers/js`, `resources/resource_transformers/cssjs`, `resources/resource_transformers/babel`, `resources/resource_transformers/tocss/scss`, `resources/resource_transformers/tocss/sass` | T15, T16 |
| `nh-tplfuncs` | 12 | neohugo tpl/<namespace>/** template functions (all namespaces; seeksnack's set fully, the rest as stubs), registry, tplimplinit | `tpl/internal`, `tpl/internal/resourcehelpers`, `tpl/tplimplinit`, `tpl/cast`, `tpl/collections`, `tpl/compare`, `tpl/crypto`, `tpl/encoding`, `tpl/fmt`, `tpl/hash`, `tpl/math`, `tpl/reflect`, `tpl/safe`, `tpl/css`, `tpl/data`, `tpl/debug`, `tpl/diagrams`, `tpl/hugo`, `tpl/images`, `tpl/inflect`, `tpl/js`, `tpl/lang`, `tpl/openapi/openapi3`, `tpl/os`, `tpl/page`, `tpl/partials`, `tpl/path`, `tpl/resources`, `tpl/site`, `tpl/strings`, `tpl/templates`, `tpl/time`, `tpl/transform`, `tpl/urls` | T18, T19, T15 |
| `nh-hugolib` | 13 | neohugo hugolib/** : HugoSites/Site/pageState, content capture, content map + assembly, content rendering + shortcodes, render loop, aliases, postProcess, build stats | `hugolib`; new/ported-3rd-party modules: `template_exec`, `tplapi/page_methods`, `tplapi/site_methods`, `tplapi/named_types`; several Go files are split by build phase (§12.1) | T20, T21, T22, T23, T24 |
| `nh-commands` | 14 | neohugo commands/* + main.go (bin `neohugo`): flags -> config, build, version/env/config commands, static copy (spf13/fsync) | `commands`; new/ported-3rd-party modules: `fsync` | T25 |

Why some crates differ from Go's package boundaries:

- **nh-common** absorbs the Go leaf packages that import nothing Hugo-specific: `common/*`,
  `compare`, `identity` (stub), `cache/dynacache` (simplified), `resources/kinds`, `hugofs/files`,
  `hugofs/glob`. It also holds the leaf third-party ports: spf13/cast, gobuffalo/flect,
  jdkato/prose `transform`, gohugoio/locales (en, th), bep/clocks, and the gobwas/glob subset.
  This avoids about 20 micro-crates. Module paths still mirror the Go paths
  (`nh_common::maps::params` = `common/maps/params.go`).
- **nh-config** includes `common/hexec` and `common/neohugo`, because they import `config`.
  Deviation: `neohugo::get_exec_environ` takes the list of `assets/_jsconfig` files as a parameter
  instead of importing `hugofs` (Go `GetExecEnviron(workDir, cfg, fs afero.Fs)` walks the fs itself).
- **nh-media** = `media` + `output`. The two packages are always used together.
- **nh-hugofs** = `hugofs` + `hugolib/filesystems` (BaseFs) + `hugolib/paths` + `modules`
  (project module only), plus the afero and bep/overlayfs subsets. `modules/*` is owned by the
  allconfig task, because config loading drives module collection.
- **nh-tpl** is Go's `tpl` package (`tpl/template.go` + `common/hcontext`): the template execution
  context, `CurrentTemplateInfo`, `StripHTML`. It is tiny and sits low in the graph, so
  nh-page/nh-resources/nh-tplfuncs can name the execution context without depending on nh-tplimpl.
- **nh-resource** (`resources/resource` + `resources/internal`) and **nh-resources**
  (`resources/*.go`) are separate for the same reason as in Go. nh-page and nh-images need the
  `Resource` trait, and nh-resources needs nh-page and nh-allconfig
  (`resources/resource_spec.go` imports both).
- **nh-resource-transformers** holds `resources/resource_factories/*` and
  `resources/resource_transformers/*`. They need nh-tplimpl (`ExecuteAsTemplate`) and nh-esbuild
  (`js.Build`), which both sit above nh-resources.
- **nh-esbuild** = `internal/js` + `internal/js/esbuild`, driving the pinned esbuild 0.25.6
  binary over its `--service` stdio protocol (RP §9.2).
- **nh-deps** = `deps/deps.go`, the per-site dependency container. It is the Rust "context
  struct" (§4.2). Owned by T20 because construction is part of the capture phase.
- **nh-doctree** depends only on nh-common. Go's `resources/resource` import is used only by
  `resource.MarkStale` on rebuild-only delete paths, so the dependency was dropped. It is its own
  early task (T27) because both the template store (T13, `SimpleTree`) and capture (T20,
  `NodeShiftTree`) need it.
- **nh-tplfuncs** = every `tpl/<namespace>` + `tpl/internal` + `tpl/tplimplinit`. In Go, the
  namespaces register themselves through `init()`. Rust has an explicit list
  (`tplimplinit::namespaces`).
- **nh-hugolib** = `hugolib/*.go` (without `doctree`, `filesystems`, `paths`, `segments`), plus
  `tplapi/*`, the explicit template method tables that replace reflection on `*pageState` and
  `*Site`.
- **nh-commands** = `commands/{commandeer,commands,helpers,hugobuilder,config,env}.go` + `main.go`
  + a spf13/fsync port. It builds the `neohugo` binary.

---

## 3. Dependency graph

Arrows point to dependencies. All `nh-*` crates also depend on `go-value`.

```
L14  nh-commands
L13  nh-hugolib
L12  nh-tplfuncs
L11  nh-i18n   nh-resource-transformers
L10  nh-deps   nh-esbuild
L9   nh-resources
L8   nh-allconfig   nh-tplimpl
L7   nh-images   nh-page
L6   nh-publisher   nh-resource
L5   nh-helpers
L4   nh-hugofs   nh-markup   nh-transform
L3   nh-media
L2   nh-config   nh-tpl
L1   nh-langs   nh-parser   nh-doctree
L0   nh-common
      (every crate) -> go-value; Wave A crates below all of these
```

Edges (nh-* only):

- `nh-common` → (go-value only)
- `nh-langs` → `nh-common`
- `nh-parser` → `nh-common`
- `nh-config` → `nh-common`, `nh-parser`, `nh-langs`
- `nh-tpl` → `nh-common`, `nh-langs`
- `nh-media` → `nh-common`, `nh-config`
- `nh-hugofs` → `nh-common`, `nh-config`, `nh-media`, `nh-parser`
- `nh-markup` → `nh-common`, `nh-config`, `nh-media`
- `nh-transform` → `nh-common`, `nh-config`, `nh-media`
- `nh-helpers` → `nh-common`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-markup`
- `nh-publisher` → `nh-common`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-helpers`, `nh-transform`
- `nh-resource` → `nh-common`, `nh-config`, `nh-media`, `nh-langs`, `nh-helpers`
- `nh-doctree` → `nh-common`
- `nh-images` → `nh-common`, `nh-config`, `nh-media`, `nh-resource`
- `nh-page` → `nh-common`, `nh-config`, `nh-media`, `nh-langs`, `nh-hugofs`, `nh-helpers`, `nh-markup`, `nh-resource`, `nh-tpl`
- `nh-allconfig` → `nh-common`, `nh-parser`, `nh-langs`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-markup`, `nh-transform`, `nh-helpers`, `nh-images`, `nh-page`
- `nh-tplimpl` → `nh-common`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-markup`, `nh-helpers`, `nh-resource`, `nh-page`, `nh-doctree`, `nh-tpl`, `nh-langs`
- `nh-resources` → `nh-common`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-helpers`, `nh-resource`, `nh-images`, `nh-page`, `nh-allconfig`, `nh-tpl`
- `nh-deps` → `nh-common`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-helpers`, `nh-resource`, `nh-page`, `nh-allconfig`, `nh-tpl`, `nh-tplimpl`, `nh-resources`
- `nh-esbuild` → `nh-common`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-helpers`, `nh-resource`, `nh-resources`
- `nh-i18n` → `nh-common`, `nh-config`, `nh-parser`, `nh-langs`, `nh-hugofs`, `nh-helpers`, `nh-deps`, `nh-tpl`
- `nh-resource-transformers` → `nh-common`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-helpers`, `nh-resource`, `nh-resources`, `nh-transform`, `nh-tplimpl`, `nh-tpl`, `nh-esbuild`
- `nh-tplfuncs` → `nh-common`, `nh-config`, `nh-media`, `nh-langs`, `nh-hugofs`, `nh-helpers`, `nh-markup`, `nh-resource`, `nh-images`, `nh-page`, `nh-allconfig`, `nh-tpl`, `nh-tplimpl`, `nh-resources`, `nh-resource-transformers`, `nh-parser`, `nh-deps`
- `nh-hugolib` → `nh-common`, `nh-parser`, `nh-langs`, `nh-config`, `nh-media`, `nh-hugofs`, `nh-markup`, `nh-transform`, `nh-tpl`, `nh-helpers`, `nh-resource`, `nh-images`, `nh-page`, `nh-doctree`, `nh-allconfig`, `nh-tplimpl`, `nh-resources`, `nh-resource-transformers`, `nh-esbuild`, `nh-publisher`, `nh-deps`, `nh-i18n`, `nh-tplfuncs`
- `nh-commands` → `nh-common`, `nh-config`, `nh-allconfig`, `nh-hugofs`, `nh-helpers`, `nh-hugolib`, `nh-deps`

Wave A usage, per crate: see the commented `# <crate> = { path = ... }` lines in each
`Cargo.toml`. The load-bearing ones:

| Wave A crate | used by | for |
|---|---|---|
| gotemplate | nh-tplimpl (`engine.rs` only), nh-tpl (`strip_tags`), nh-i18n (message text/template) | text/template + html/template (go1.24 fork + Hugo changes); contract: `crates/GOTEMPLATE_CONTRACT.md` |
| goldmark | nh-markup (`goldmark/*`) | Markdown → HTML, AST, extensions |
| tdewolff-minify, tdewolff-minify-js, tdewolff-parse | nh-transform (`minifiers`) | output + resource minification |
| gift, go-image, go-png, go-flate, libwebp-sys | nh-images | resize / overlay / encoders |
| go-hashstructure | nh-common (`hashing`), nh-images, nh-resources, nh-resource-transformers | `hashing.HashString/HashUint64` keys |
| xtext-collate | nh-langs | `langs.Collator`, one pair per language TAG (template `sort`, default page sort tie-breaks) |
| go-fmt, go-strconv, go-time, go-url, go-json, go-html, go-unicode, go-sort, go-path, go-yaml | everywhere | Go stdlib behaviours |
| libsass-sys | nh-resource-transformers (`tocss/scss`) | `toCSS` |

---

## 4. Core data model

### 4.1 Lifecycle: arena, build, freeze, render

Go uses a web of pointers (`*HugoSites` ↔ `*Site` ↔ `*pageState`, `*Deps` → `Site`, template store
→ `Site`, shortcodes → page, hooks → page). The Rust design splits the build into two phases:

1. **Mutable phase** (`&mut HugoSites`): `NewHugoSites` → `process` (collect files, parse front
   matter, insert into the trees) → `assemble` (missing sections/taxonomies/terms, standalone
   pages, aggregates, cascade, `removeShouldNotBuild`, terms and translations, render formats,
   resources). Pages live in the arena `HugoSites.pages: Vec<PageState>` and are addressed by
   `PageId(u32)`. Sites are `HugoSites.sites: Vec<Site>` in language order. Nothing in this phase
   hands out template values.
2. **Frozen phase** (`Arc<HugoSites>`): `hugo_sites_build::build` calls `HugoSites::freeze(self)`
   after assemble. From then on everything is shared and read-only, except **interior-mutable caches
   that mirror Go's lazy init**: `OnceLock` (Go `sync.Once`: lazy page init, lazy site
   collections), dynacache `Partition`s (query caches, rendered content, resources), small
   `Mutex`-guarded slots that Go reassigns (the paginator, a page output's content provider,
   Scratch), atomics (`current_output_idx`, `current_site`, counters). All of them follow §4.8.
   The render loop, templates, content rendering and post-processing all run here. (The page
   outputs are initialised during assembly already — `assembleResources` shifts every page — via
   the same `OnceLock`, called with `&self`.)

Handles are the Rust stand-in for Go pointers:

```rust
pub struct PageHandle { pub h: Arc<HugoSites>, pub id: PageId, pub wrapper: PageWrapper } // *pageState & wrappers
pub struct SiteHandle { pub h: Arc<HugoSites>, pub idx: usize }                            // *Site / *page.siteWrapper
```

`PageHandle` implements `nh_resource::resourcetypes::Resource` + `nh_page::page::Page` (in
`nh_hugolib::tplapi::page_methods`). `SiteHandle` implements `nh_page::site::Site` (in `tplapi::site_methods`). A page becomes a template value
through `nh_page::page::PageRef(Arc<dyn Page>)` (an `Object`). A site becomes one through
`nh_page::site::SiteRef(Arc<dyn Site>)`. Lower crates (nh-page, nh-resources, nh-tplfuncs,
nh-tplimpl) see only these traits. nh-hugolib is the only crate that knows `PageState`.

`PageWrapper` covers the Go wrapper types that templates can see: `None` (`*hugolib.pageState`),
`Weight0(w)` (`hugolib.pageWithWeight0`, taxonomy members), `Ordinal(o)`
(`*hugolib.pageWithOrdinal`), `ForShortcode` (`*hugolib.pageForShortcode`), `ForRenderHooks`
(`*hugolib.pageForRenderHooks`). The wrapper changes the Go type name (`%T`, `printf "%T"`) and, for
the last two, the method set: `Content`, `Plain`, `Summary`, … come from `page.NopPage`
(`hugolib/shortcode_page.go`). Identity (`Page::page_id`, `Object::identity`) is the id of the
wrapped `pageState` in every case.

### 4.2 HugoSites, Site, Deps

| Rust | Go | Owner | Notes |
|---|---|---|---|
| `nh_hugolib::hugo_sites::HugoSites` | `hugolib.HugoSites` | T20 (struct, construction); data loading T23 | `sites`, `configs`, `hugo_info`, `render_formats` (all sites' formats concatenated, the global output index), `current_site` (atomic), `deps` (first site's), `pages` (arena), `page_trees`, `data` (OnceLock), `cache_pages` (Partition), `template_executor` (test seam, `None` in a build), weak `self_ref` |
| `nh_hugolib::site::Site` | `hugolib.Site` | T23 (constructed by T20) | per language: `idx`, `conf` (`Arc<Config>`), `language`, `deps: Arc<Deps>`, `page_map`, `store` (Scratch), `taxonomies`/`menus` (OnceLock, lazy like Go), `home`, `lastmod`, `related_docs_handler`, `publisher` (one per site, with its own HTML elements collector), `frontmatter_handler`, `render_formats`, `template_store` (OnceLock, this site's view) |
| `nh_deps::deps::Deps` | `deps.Deps` | T20 | shared services per site: `log`, `conf: Arc<dyn AllProvider>`, and `Option` services like Go's nil-able pointers (`exec_helper`, `fs`, `path_spec`, `content_spec`, `source_spec`, `resource_spec`, with panicking accessors), `translate: OnceLock<TranslateFunc>`, `site: Arc<OnceLock<SiteRef>>`, `template_store: OnceLock<TemplateStore>`, `build_state` (post-process filenames; THE PostProcess id `Incrementer`, shared by all sites and stored in the shared `resources.SpecCommon`), `counters`. `Deps::for_tests(conf)` + `with_*` builders for component tests |
| `nh_allconfig::allconfig::{Configs, Config}` | `allconfig.Configs/Config` | T09 | `Configs.base`, `language_config_map/slice`, `languages` (sorted: default first, then weight, then lang), `modules`, `content_path_parser` |
| `nh_allconfig::configlanguage::ConfigLanguage` | `allconfig.ConfigLanguage` | T09 | implements `nh_config::config_provider::AllProvider` (typed getters + `get_config_section(name) -> Arc<dyn Any>`, with the `config_section::<T>()` downcast helper) |

Late binding, which replaces Go's mutable pointer fields:

- `Deps.site: Arc<OnceLock<SiteRef>>`. It is set once per site in the bind step right after
  freeze (below), when the `SiteHandle` can exist. The same `Arc` goes into
  `nh_tplimpl::templatestore::SiteOptions.site` and `TemplateExecHelper.site`, so templates reach
  `site` without a cycle.
- `Deps.template_store: OnceLock<TemplateStore>`. The shared store is parsed once
  (`TemplateStore::new`); each site gets a `with_site_opts` view with its own func map and site.
- `Deps.translate: OnceLock<TranslateFunc>`, set by nh-i18n (`TranslationProvider::new_resource` +
  `clone_resource`). It is read by `tpl/lang.Translate`.

Because freeze has to happen before the handles exist, `NewHugoSites` works in two steps:
`HugoSites::new` creates everything that does not need a handle. `hugo_sites_build::build`
freezes, then fills the `OnceLock`s (site refs, template stores with site options, translators),
then renders.

### 4.3 PageState

`nh_hugolib::page::PageState` is the Go `pageState`, split into sub-structs. Each sub-struct (or
function group) is owned by one task, so the build phases can be ported and tested in order (§12.1):

| Field | Type (module) | Go | Owner |
|---|---|---|---|
| `id`, `pid`, `site_idx` | — | `pid`, `p.s` | T20 (set at creation) |
| `meta` | `page__meta::PageMeta` | `pageMeta` + `pageMetaParams` (kind, term/singular, path info, file, decoded front matter `PageConfig`, original dates/params, normalized `params`, standalone output format, bundle info) | T20 (`setMetaPre`, getters) / T21 (`setMetaPost`, `setMetaPostParams`, `applyDefaultValues` in `page__meta_post.rs`; aggregates, term metadata) |
| `common` | `page__common::PageCommon` | `pageCommon` (store/`.Scratch` shared by all outputs, `targetPathDescriptor`, lazy translations, next/prev positions) | T23 |
| `lazy` | `OnceLock<Result<page::PageLazy>>` = `{ paths: PagePaths, outputs: Vec<Arc<PageOutput>> }` | `ps.init` running `initLazyProviders`: `pagePaths` + `pageOutputs` | T21 (`page__init.rs`: `init_page`, using T21's `new_page_paths` and T22's `PageOutput::new` / `PageContentOutput::new`) |
| `current_output_idx` | `AtomicUsize` | `pageOutputIdx` + embedded `*pageOutput`, switched by `shiftToOutputFormat` | T21 (`page__init.rs`) |
| `page_output_template_variations_state` | `AtomicU32` | same (`canReusePageOutputContent` == 1) | T21 |
| `content` | `Option<Arc<CachedContent>>` | `cachedContent` (parse result, content items, shortcodes; scopes) | T20 (`page__content_parse.rs`) / T22 (rendering, `page__content.rs`) |

**Page outputs are shared by format NAME** (Go `created := map[string]*pageOutput`,
page__meta.go:907-916). `outputs` has one slot per GLOBAL render format (`HugoSites.render_formats`,
all sites' formats concatenated), but every slot with the same format name holds the SAME
`Arc<PageOutput>`: en/html (index 0) and th/html (index 7) are one output, so the paginator, the
content provider and the target paths are shared. Standalone pages (404, sitemap, robots) have a
single slot and always use index 0.

`page__output::PageOutput` (T22) holds the per-output state that `shiftToOutputFormat` switches:

- `f`, `render`, `target_paths`;
- `paginator: Option<PagePaginator>` (only when `render && IsNode()`; T23's type, resettable, §7.4);
- `pco: Mutex<Option<Arc<PageContentOutput>>>` (Go `po.pco`);
- `provider: Mutex<ContentProviderSlot>`: the CURRENT content provider (Go's embedded
  `ContentProvider`/`MarkupProvider`/...): `Nop`, `Pco(Arc<PageContentOutput>)`, or
  `Lazy(Arc<nh_page::page_lazy_contentprovider::LazyContentProvider>)` (resettable, T12).

`PageContentOutput` (`page__per_output.go`) resolves the render hooks per output format.
`HookRendererTemplate` implements the nh-markup hook traits by executing templates (§6.2).

### 4.4 Content trees (doctree)

- `nh_doctree::nodeshifttree::NodeShiftTree<T>` stands in for Go's radix tree + dimension
  shifter. Storage is `BTreeMap<String, T>`: Go walks armon/go-radix in **byte-lexicographic key
  order**, and `BTreeMap<String,_>` iterates in the same byte order (Rust `str` `Ord` is bytewise).
  CM §4.
- Keys are Go's: `PathInfo().Base()` for pages (e.g. `/`, `/biscuit`,
  `/biscuit/koalas-march-chocolate`), `Base()` + extension for resources, and for terms
  **`strings.ToLower("/" + plural + "/" + value)` with spaces → `-`, not sanitized**. The target
  paths are sanitized later, which gives the 13 term collisions (CM §7.6). Never key the tree
  by output path.
- The language dimension: a node is `ContentNode` (in `content_map_trees`, T20), either one node
  or a per-language array (`contentNodes`). `ContentNodeShifter` implements
  `nh_doctree::nodeshifttree::Shifter`: `shift(node, dimension, exact)` returns the language
  version for the site that is walking. Non-page resources fall back to the first non-nil language
  (CM §4, the way TH pages reach EN-owned bundle images).
- `LongestPrefix` is character-level (not segment-aware) and retries with `path.Dir`
  (`nodeshifttree.go:224-243`). Port it exactly.
- Trees: `PageTrees { tree_pages, tree_resources, tree_taxonomy_entries }`, shared by all sites and
  owned by `HugoSites`. There is one `PageMap` per site, a view with the site's language index.
  The tree types, the shifter, the `PageMap` struct and `new_page_map` are in
  `content_map_trees.rs` (T20, capture inserts into them). The queries and the assembly steps are
  `impl` blocks in `content_map_page.rs` (T21).
- **Caches keyed by query** (`pageMap.cachePages1/2`, `content_map_page.go:267-287`) reproduce
  Go's key strings, including the `"gagesInSection/"` typo and the term
  `RegularPages()`/`Pages()` shared key (CM §12.3). They are dynacache partitions named like Go's
  (`/pag1/<site>`, `/pag2/<site>`, `/gett/<site>`, `/ress/<site>`). First stored value wins (§4.8).
- **Rendered content is cached in the page's SITE `PageMap`** too, as in Go:
  `cache_content_rendered` / `cache_content_plain` / `cache_content_toc` (`/cont/ren|pla|toc/<site>`),
  keyed `sourceKey + "/" + markupScope + outputFormat.Name` (§7.4).

### 4.5 Resources

- `nh_resource::resourcetypes::Resource` is the Go `resource.Resource` + optional interfaces
  (`ContentProvider`, `ReadSeekCloserProvider`, `Source.Publish`, `LanguageProvider`,
  `TranslationKeyProvider`, `NameNormalizedProvider`) and the template-dispatch trio
  `tpl_type_name`, `tpl_has_method`, `tpl_call_method`. `to_value(self: Arc<Self>)` gives the
  template value: `ResourceRef` for non-pages, `PageRef` for pages.
- `Resources = Vec<Arc<dyn Resource>>`, with template value `SliceType::Named("resource.Resources")`.
  Its methods (`Get`, `GetMatch`, `Match`, `ByType`, …) are registered in the named-type registry
  (§5.3).
- nh-resources: `GenericResource` (`genericResource`: spec, paths, media type, `Arc<ResourceHash>`
  shared by clones, publish-once `OnceLock`, `include_hash_in_key`, `source_filename_is_hash`),
  `ResourceAdapter` (`resourceAdapter`: target + transformation chain + `OnceLock` publish/transform
  state), `ImageResource` (`imageResource`), `ResourceCache` (dynacache partitions), `ImageCache`,
  `PostPublishResource`.
- **Cold-cache rule** (AC §2.1, IMG §3.9): the Rust port never reads `resources/_gen`. Every
  processed image is on the create path, so `Key() = rel_permalink + "_" + xxh64(root source bytes)`
  (`GenericResource::key`). Writing `resources/_gen` is optional and off by default.
- Resources are **global across languages**: one dynacache (`ResourceSpec.mem_cache`) shared by all
  sites. Bundle resources are published once, under the EN page's directory (CM §0.8).
- **Processed images are decoded from ENCODED bytes** (IMG §3.1, §3.5, §9.10). Each `Resize` or
  `Filter` decodes its parent's encoded JPEG/PNG (a round trip through the encoder), and
  `images.Overlay` decodes the watermark's encoded PNG. Go reads these from its file cache. The
  Rust port never reads `resources/_gen`, so `ImageCache.encoded` keeps the encoded bytes of
  every processed image in memory (keyed like Go's file cache) and `ImageResource::decode_image`
  decodes them. Never hand decoded pixels from one step to the next.
- `Slice` is part of every resource's template method table (Go `commonResource.Slice`,
  resources/resource.go:278-297). `slice $r1 $r2 ...` asks the first element for `Slice` and gets
  a `resource.Resources`, which `resources.Concat` accepts; a plain `[]interface {}` is rejected.
- **One PostProcess id counter**: `deps.BuildState` implements
  `nh_common::identity::Incrementer`, is passed to `resources::Spec::new` and stored in the
  shared `SpecCommon` (Go deps.go:255). There is no second counter.

### 4.6 Output formats and render formats

- `nh_media::output::output_format::OutputFormat`/`Formats` are ported with exact defaults
  (AC §5.6). `Site.render_formats` = union of the pages' configured formats and the per-kind
  formats, `sort.Sort(Formats)` (HTML first by weight, then by name). `HugoSites.render_formats` is
  the concatenation over sites, and `PageState.lazy.outputs[i]` is indexed by that **global**
  index, with slots of the same format name sharing one `Arc<PageOutput>` (§4.3).
- `shift_to_output_format(is_rendering_site, idx)` switches `current_output_idx`. Other pages'
  `.Permalink`, `.Content`, `.Plain` then use the **current format of that page**. This is the
  reason `index.json` renders tables through `render-table.json.json` (TI §1.2): when the JSON
  output renders, every page's current output is JSON.

### 4.7 Values that cross into templates

All template data is `go_value::Value`. Host objects are `Value::Object(Arc<dyn Object>)`
(Page, Site, Resource, Pager, Scratch, Language, OutputFormat, MediaType, namespaces, hook contexts,
`*url.URL`, `hstring.HTML`, `config.SitemapConfig`, …). Go named slices and maps are
`Value::List`/`Value::Map` with a `SliceType::Named`/`MapType::Named`/`MapType::Params` tag
(`page.Pages`, `resource.Resources`, `maps.Params`, `page.Taxonomy`, …). Rules:

- `maps.Params` lookups are case-insensitive: the key is lower-cased, and all keys are stored
  lower-case (`prepare_params`). Other maps are exact. Methods shadow map keys (TE §14.3).
- Numbers keep their Go kinds: YAML ints → `int` (`IntKind::Int`), TOML ints → `int64`,
  JSON (`transform.Unmarshal`) → `float64`. The `position` values in TI A.3 show both `int64` and
  `int`.
- Identity survives: a `List`/`Map`/`Object` handed around keeps its `Arc`. `.Site.Params` returns
  the SAME `Arc<Map>` every time (the `mainsections` special case compares pointers, contract C10).

**Go nils → Rust values.** Go has several kinds of nil, and text/template treats them differently
(TE §5.3-5.4, contract C2/C7/C8). The mapping:

| Go value | Rust `Value` | text/template prints | truth | `.X` on it | passed to a func / stored in Scratch/dict |
|---|---|---|---|---|---|
| missing map key; YAML/JSON `null` in Params (`GetMapValue` returns invalid); `reflect.Value{}` | `Invalid` | `<no value>` | false | `Invalid`, no error | `Invalid` (untyped nil) |
| nil `interface{}`/`any` result (e.g. `Scratch.Get` of a nil) | `Invalid` (hosts return it directly; the engine also unwraps a nil empty interface after each pipeline command) | `<no value>` | false | `Invalid` | `Invalid` |
| nil NON-EMPTY interface result: `.Parent` of home (`page.Page`), a nil `resource.Resource`, a nil `error` | `TypedNil("page.Page")` etc. (`typed_nil_kind` = `Interface`: unregistered named type) | `<nil>` | false | error `nil pointer evaluating page.Page.X` | becomes `Invalid` (a nil interface converted to `any` is untyped nil; contract C7) |
| typed nil pointer, e.g. a nil `*source.File` | `TypedNil("*source.File")` | `<nil>` | `IsZero` if the type has it, else false | nil-safe methods through the helper, else `nil pointer evaluating` | stays `TypedNil` |
| nil slice / map of a named or unnamed type | `TypedNil("page.Pages")`, `TypedNil("[]string")`, … | `[]` / `map[]` | false | methods of the named type | stays `TypedNil` |
| `page.NilPage` = `(*nopPage)(nil)` inside a non-nil `page.Page` (`.GetPage` of a missing path) | the NopPage OBJECT (`*page.nopPage`, `is_zero` = true, all methods work) | `<nil>` | false (IsZero) | nop results | the object |

Consequences:

- `%!s(<nil>)` in 1,678 golden files comes from an UNTYPED nil, i.e. `Invalid`: a missing Params
  key passed to `printf "%s"`. A typed nil pointer would print `%!s(*T=<nil>)` instead.
- `jsonLd.html:389` stores `.Parent` in a Scratch. For home that is `TypedNil("page.Page")`, which
  becomes `Invalid` on the way into `Scratch.Set`. The next `if ($scratch.Get "current")` is
  false, so `.Parent` is never called on it. In Go the same thing happens.
- In text templates, `{{ .Parent }}` on home prints `<nil>` (not `<no value>`), and
  `{{ $p := .Parent }}{{ $p.Title }}` is an error, as in Go.

### 4.8 Lazy state and caches: never compute while holding a lock

Rendering re-enters: `.Content` of page A runs hooks and shortcodes, which run templates, which
call `.Plain` of page B, `.Paginator`, `partialCached`, `resources.Concat`, and sometimes
another scope or format of page A itself. So:

- **Maps of cached values** (query caches, rendered content, resources, images, `partialCached`,
  `CachedContent.scopes`, `HugoSites.cache_pages`) use `nh_common::dynacache::Partition`. In
  `get_or_create`, the value is looked up under the lock, then computed WITHOUT the lock, then
  inserted if the key is still absent. The FIRST stored value wins and is returned to every
  caller. With sequential rendering this is exactly Go's first-writer-wins.
- **Reassignable slots** (the paginator, a page output's `pco` and `provider`, a
  `LazyContentProvider`) are `Mutex<…>` that is held only to read or swap an `Arc`. The compute
  step runs between two short critical sections, and the second one stores the result only if
  the slot is still empty.
- **`OnceLock::get_or_init`** is allowed only where Go uses `sync.Once`/`lazy.Init` on a value
  whose initialisation cannot re-enter the same cell. Re-entering a `OnceLock` deadlocks or
  panics, and Go would deadlock too. Examples: `PageState.lazy`, `Site.taxonomies`, `h.data`,
  publish-once.
- A plain `Mutex` for state written only in the mutable phase is unnecessary: use `&mut` (for
  example the shortcode handler, filled during capture and read-only afterwards).

---

## 5. Template API exposure

### 5.1 Mechanism

Go templates find methods and fields by reflection. The port uses explicit tables:

- **Objects.** A host type implements `go_value::Object`. Its exported Go method set is declared
  once with `nh_common::go_methods!(Type { "Name" => |s, ctx, args| ..., ... })` and wired into
  `Object` with `nh_common::object_basics!("<go type string>")`. The go type string must be Go's
  `%T` spelling (e.g. `*page.Pager`, `*maps.Scratch`, `*langs.Language`, `media.Type`,
  `*hugolib.pageState`). Struct fields accessed as `.Field` go through `Object::field`
  (e.g. `langs.Language.Lang`, `media.Type.Type`, `hooks.TableCell.Text`,
  `config.SitemapConfig.ChangeFreq`).
- **Traits with template dispatch** (`Resource`, `Page`, `Site`): the trait carries
  `tpl_has_method`/`tpl_call_method`. The wrapper objects (`ResourceRef`, `PageRef`, `SiteRef`)
  forward to it. `PageHandle` has one `go_methods!` table in `tplapi::page_methods`, and
  `tpl_call_method` delegates to it.
- **Named slice/map types.** Methods on `page.Pages`, `resource.Resources`, `maps.Params`,
  `page.Data`, `page.Taxonomy`, `page.TaxonomyList`, `page.WeightedPages`, `page.OutputFormats`,
  `langs.Languages`, `page.PagesGroup`, … are `NamedMethods { has_method, call }` entries. The
  owning crates export them (`nh_page::pages::PAGES_METHODS`, …), and
  `nh_hugolib::tplapi::named_types::registry()` registers them. The template exec helper consults
  the registry when the receiver is not an `Object` (Go `hreflect.GetMethodByName` on the named
  type).
- **Method arguments** arrive as the engine evaluated them (Go `evalArg`: ideal constants become
  `int`/`float64`). Use `nh_common::object::args::{exactly, get, string, int, rest}` for typed
  parameters. Methods that take `context.Context` first in Go (`Content`, `ContentWithoutSummary`,
  `Plain`, `PlainWords`, `Summary`, `Truncated`, `WordCount`, `FuzzyWordCount`, `ReadingTime`,
  `Len`, `TableOfContents`, `Fragments`, `Render`, `RenderString`, `RenderShortcodes`, …; see
  `resources/page/page.go`) receive the `HostCtx` (the `TplContext`, §6.5). Templates call them
  without the ctx argument (the engine injects it).
- **Interface checks** map as follows. `compare.Eqer` → `has_method("Eq")` + `call_method`.
  `types.Zeroer` → `Object::is_zero`. `fmt.Stringer` → `Object::go_string`.
  `types.Unwrapper`/`Unwrapv` → `Page::unwrap_page`. `resource.Resource` checks →
  `nh_page::page::resource_from_value`. `hashing` `Key()`/`GetIdentity` → `Object::hash_key`.
- **Wrapper structs expose their embedded interfaces, not the wrapped type.** A Go struct that
  embeds interfaces has the method set of those interfaces plus its own methods. For example
  `pageForShortcode` / `pageForRenderHooks` (hugolib/shortcode_page.go) =
  `page.PageWithoutContent` (delegating to the page) + `TableOfContentsProvider` (the page;
  `pageForShortcode` overrides `TableOfContents` with the TOC placeholder) + `MarkupProvider` and
  `ContentProvider` from `page.NopPage` + `Unwrapv`, `String`. So `.Page.Content` inside a hook is
  the nop result, and `*pageState` methods outside those interfaces are not visible. Build one
  table per wrapper (`tplapi/page_methods.rs`); do not filter the `*pageState` table.
- **Shallower fields hide promoted methods.** Go's selector depth rule: a field at depth 0
  shadows a method promoted from an embedded value at depth ≥ 1, so the method is not in the
  method set at all. `hugolib.aliasPage{Permalink string; page.Page}`: `.Permalink` is the field.
  For such types `has_method(name)` must be false and `field(name)` answers.

### 5.2 Member sets per type

The **minimum** set is what seeksnack calls, from the dynamic trace (TI §3, A.2, A.3; TE §13).
Wave B implements the **complete** Go method set of each type. Methods outside the minimum are
still ported (most are small), or they return the explicit unsupported error when they need an
unported subsystem (e.g. `GitInfo`, `CodeOwners`).

| Go type (`%T`) | Rust owner | seeksnack uses (exact case) | complete set = Go source |
|---|---|---|---|
| `*hugolib.pageState` (+ `pageWithWeight0`, `pageWithOrdinal`, `pageForShortcode`, `pageForRenderHooks`) | `nh_hugolib::tplapi::page_methods` (T23), bodies spread over T20–T23 | AllTranslations, AlternativeOutputFormats, Content, Data, Date, Description, File, IsHome, IsNode, IsPage, IsSection, IsTranslated, Kind, Lang, Language, Lastmod, OutputFormats, Page, Pages, Paginate, Paginator, Params, Parent, Permalink, Plain, PublishDate, ReadingTime, RegularPages, RelPermalink, Resources, Scratch, Section, Site, Sitemap, Title, Translations, Type; in hooks: GetPage, Resources (on `PageInner`/`.Page`); in shortcodes: Ref/RelRef, Params | `resources/page/page.go` `Page` interface (all embedded interfaces) as implemented by `*pageState` |
| `*page.siteWrapper` (`.Site`, `site`) | `nh_hugolib::tplapi::site_methods` (T23) | AllPages, BaseURL, Config, Data, GetPage, Home, Language, LanguageCode, Languages, Params, RegularPages, Taxonomies, Title | `resources/page/site.go` `Site` interface |
| `*hugolib.Site` (sitemapindex `range .Sites`) | same | Lastmod, SitemapAbsURL | `hugolib/site.go` exported methods |
| `page.SiteConfig` → `services.Config` → `services.RSS` | nh-page `site::SiteConfig` (T11) | fields Services.RSS.Limit | struct fields |
| `*page.Pager` | `nh_page::pagination::PagerRef` (T12) | First, HasNext, HasPrev, Last, Next, PageNumber, PagerSize, Pagers, Pages, Prev, TotalPages, URL | `resources/page/pagination.go` |
| `page.Pages` | `nh_page::pages::PAGES_METHODS` (T12) | Related, Reverse (+ `where`/`sort`/`first` receive it) | `pages.go`, `pages_sort.go`, `pages_related.go`, `pagegroup.go` (ByX, GroupByX, Len, Limit, Next/Prev …) |
| `page.Data` | `nh_page::page_data::DATA_METHODS` (T11) | Pages; map keys Singular/Plural/Terms/Term … | `page_data.go` |
| `page.Taxonomy`, `page.TaxonomyList`, `page.WeightedPages`, `page.OrderedTaxonomy` | nh-page `taxonomy`/`weighted` (T12) | range, index, `.Count`, `.Pages` | `taxonomy.go`, `weighted.go` |
| `page.OutputFormats`, `*page.OutputFormat` | nh-page `page_outputformat` (T11) | Get; methods MediaType, Permalink; field Rel | `page_outputformat.go` |
| `media.Type` | `nh_media::media::media_type::MediaType` (T04) | field Type (+ MainType, SubType, Suffixes …) | `media/mediaType.go` |
| `*langs.Language` | `nh_langs::language::LanguageObject` (T03) | fields Lang, LanguageName; LanguageCode, Params | `langs/language.go` |
| `*maps.Scratch` | `nh_common::maps::scratch::Scratch` (T01) | Add, Delete, Get, Set, SetInMap | `common/maps/scratch.go` |
| `maps.Params` | `nh_common::maps::params::PARAMS_METHODS` (T01) | case-insensitive keys, IsZero | `common/maps/params.go` |
| `*resources.resourceAdapter` | `nh_resources::transform::ResourceAdapter` (T14) | Height, Permalink, RelPermalink, Resize, Title, Width (+ Content, Data.Integrity, MediaType, Name via chains), `Slice` (through `slice`, §4.5) | `resources/transform.go`, `resources/image.go`, `resources/resource.go` (`commonResource.Slice`) |
| `resource.Resources` | `nh_resource::resources` (T11) | Get | `resources/resource/resources.go` |
| `*postpub.PostPublishResource` | `nh_resources::postpub::postpub` (T14) | Content (placeholder fields) | `resources/postpub/postpub.go` |
| `*source.File` | `nh_helpers::source::file_info::FileObject` (T08) | UniqueID (+ IsZero for `with .File`) | `source/fileInfo.go` |
| `hooks.LinkContext` / `ImageLinkContext` / `HeadingContext` / `TableContext` / `TableCell` | nh-markup `goldmark::render_hooks`, `goldmark::tables`, `converter::hooks` (T06) | Destination, Page, PageInner, Text, Title; Anchor, Level, Text; Attributes, THead, TBody; fields Alignment, Text | `markup/goldmark/render_hooks.go`, `markup/converter/hooks/hooks.go`, `markup/goldmark/tables/tables.go` |
| `hugolib.ShortcodeWithPage` | `nh_hugolib::shortcode_page::ShortcodeWithPage` (T22) | field Params, Page, Get, … | `hugolib/shortcode.go` `ShortcodeWithPage` |
| `hugolib.aliasPage` | `nh_hugolib::alias::AliasPage` (T24) | field Permalink, Page | `hugolib/alias.go` |
| `neohugo.HugoInfo` | `nh_config::neohugo::neohugo::HugoInfo` (T04) | field-like Environment, Generator, IsProduction … | `common/neohugo/neohugo.go` |
| `config.SitemapConfig` (`.Sitemap`, returned BY VALUE) | `nh_config::common_config::SitemapConfig` implements `Object` (T04; `Kind::Struct`, no methods); `Page::sitemap()` is wrapped with `Value::object(..)` in the page table (T23) | fields ChangeFreq, Priority (float64) — 1,730 calls in sitemap.xml; also Filename, Disable | `config/commonConfig.go` |
| `*page.nopPage` (`page.NilPage`, `.GetPage` of a missing path) | `nh_page::page_nop` NopPage object (T11) | IsZero (via `with`/`if`) | `resources/page/page_nop.go` |
| `*url.URL` | `nh_tplfuncs::urls` (T19), over go-url | IsAbs, String; field Path (+ RawQuery, Fragment) | Go `net/url` |
| `time.Time` | `nh_tplimpl::template_funcs::time_call_method` (T13), over go-time | Format, IsZero (+ Year, Unix, …) | Go `time` |
| namespaces `*<ns>.Namespace` | `nh_tplfuncs::<ns>::<ns>::Namespace` (T15/T18/T19) | images.Filter/Overlay, js.Build, path.Ext, reflect.IsSlice, resources.{Concat, ExecuteAsTemplate, Get, GetRemote, PostProcess}, strings.{HasPrefix, TrimPrefix}, transform.Unmarshal, urls.Parse | `tpl/<ns>/<ns>.go` |
| `images.filter` | `nh_images::filters::FilterObject` (T10) | (passed to `images.Filter`) | `resources/images/filters.go` |

Printing host objects (`{{ . }}`, `%v`) uses `Object::printable_value`. Seeksnack never prints a
Page or Site struct (TE Appendix B `T-PRINT-04` shows what Go would dump). Implement
`printable_value` for types that Go prints as values (`OutputFormat`, `media.Type`), and leave the
default for pointers.

### 5.3 Where each piece is resolved

1. `.Method` or `.Field` on `Value::Object`: `Object::has_method` → `call_method`, else
   `Object::field`, else error.
2. `.Method` on `Value::List`/`Value::Map` with a named type: `NamedTypeRegistry::call` (via
   `ExecHelper::call_method`).
3. `.key` on a map: `ExecHelper::get_map_value`. For `maps.Params`, lower-case the key (Go
   `GetMapValue`). The special case `.Site.Params.mainSections` is a METHOD lookup in Go
   (`GetMethod`, template_funcs.go:88-114): `has_method`/`call_method` fire when the receiver map
   `Arc::ptr_eq`s the site params and the name EqualFolds `mainsections`.
4. Functions: `ExecHelper::get_func(name)` → `FuncMap` built by `nh_tplfuncs::tplimplinit`
   (Hugo functions override Go builtins, TE §14.11).
5. Truth tests: `ExecHelper::is_true` (= `hreflect::is_truthful`), used by the engine for
   if/with/and/or/not (contract C5).

---

## 6. Host interfaces (cross-crate call paths)

### 6.1 Template execution from hugolib

```
site_render (T24) → Site::render_and_write_page (T23)
  → PageState::resolve_template → TemplateStore::lookup_pages_layout(TemplateQuery{path, category, desc})
  → template_exec::execute(h, site, &TplContext{page: Some(page_value), ..}, ti, &mut buf, &page_value, ExecCall{Page..})
      → (test seam: HugoSites.template_executor) or TemplateStore::execute_with_context
      → engine::Executer (gotemplate) with TemplateExecHelper (funcs, named types, site, is_true)
  → publisher (absURL → minify → file + htmlElementsCollector)
```

`nh_tplimpl::engine` is the **only** module that talks to the Wave A `gotemplate` crate. The
gotemplate side is pinned by `crates/GOTEMPLATE_CONTRACT.md`. That file lists the semantics that
must live inside the engine:

- truthiness through `ExecHelper::is_true`;
- methods before fields and keys, for every receiver kind;
- `and`/`or` return the deciding operand;
- Go's argument, ideal-constant and nil conversion rules;
- `<no value>` vs `<nil>` printing;
- the escapers unwrap `printable_value`;
- the engine keeps `Arc` identity (for `mainsections`);
- one context, passed unchanged;
- re-entrant nested execution.

`engine.rs` mirrors the contract's `ExecHelper` trait (`init`, `get_func`, `has_method`,
`call_method`, `get_map_value`, `on_called`, `is_true`) and defines `TplFunc`, `FuncMap`,
`Template` (text or html) and `Executer`. The Wave A gotemplate task implements the contract as
acceptance criteria, and T13 reviews it. If the gotemplate API differs, only `engine.rs` changes.

`nh_hugolib::template_exec::execute` is the single dispatch point for every template execution
that nh-hugolib starts: pages, pagers, aliases, render hooks and shortcodes. It passes the caller's
context through unchanged. `HugoSites.template_executor` replaces it in tests (T22 replay, T24
stub).

### 6.2 Markup render hooks calling templates

```
PageState content (T22) → nh_markup::converter::Converter::convert(RenderContext{ctx, src, get_renderer, ..})
  → goldmark (Wave A) with nh_markup::goldmark::render_hooks renderers
  → hooks::LinkRenderer/HeadingRenderer/TableRenderer::render_*(cctx, &mut w, &ctx_value)
  → nh_hugolib::page__per_output::HookRendererTemplate (T22)
      → template_exec::execute(.., cctx UNCHANGED, hook_template, w, ctx_value, ExecCall{Hook(<type>), ordinal})
```

- nh-markup defines the hook traits and the context objects (`LinkContext`, `ImageLinkContext`,
  `HeadingContext`, `TableContext`, `TableCell`). The `Page`/`PageInner` members are
  `Value`s produced by hugolib (`PageHandle` with `PageWrapper::ForRenderHooks`, §5.1).
- `GetRendererFunc(RendererType, id)` resolves the hook template per output format. Seeksnack uses
  a site `render-image.html`, the embedded `render-link.html`, `render-table.html(.html/.json.json)`
  and the default heading renderer (TI §5.4).
- **The hook runs with the caller's template context, unchanged.** Go calls
  `hr.templateHandler.ExecuteWithContext(cctx, hr.templ, w, ctx)` (site.go:1504-1526), where `cctx`
  is the context of the `.Content`/`.Plain`/`RenderString` call that triggered the rendering.
  - No page is set on the context.
  - `is_in_goldmark` is NOT set. It is set only by `prepareShortcode` for `{{% %}}` shortcodes
    (shortcode.go:327-332).
  - If it leaked into hooks, `RenderShortcodes` would wrap its output with `hugocontext.Wrap`
    (page__content.go:1119-1124).
  - The markup scope (`neohugo.Context.MarkupScope`) travels in that same context.

### 6.3 Shortcodes

Parsing (`pageparser`) yields shortcode items. At page creation, `page__content_parse` and
`shortcode_parse` (T20) replace each item with a placeholder
`"HAHAHUGOSHORTCODE" + <page pid> + "s" + <ordinal> + "HBHB"` (Go
`createShortcodePlaceholder("s", pid, ordinal)`; the TOC placeholder is
`createShortcodePlaceholder("TOC", 0, 0)`).

- **pids** only need to be unique per page. Go assigns them with an atomic counter during
  parallel collection, so Go's own order is not reproducible, and it does not matter: the
  placeholders are replaced before output, and the pid is used only for identity and
  `hugocontext`.
- **Template lookup at parse time.** Extraction looks the template up by name
  (`TemplateStore.LookupShortcodeByName`) to know whether it takes inner content, so capture needs
  the template store.
- **Rendering** (T22) runs the templates with `.Page` = `PageHandle{wrapper: ForShortcode}` in a
  `ShortcodeWithPage` object, through `template_exec` (`ExecKind::Shortcode`). For `{{% %}}`
  shortcodes in markdown, the context gets `is_in_goldmark`.
- **Expansion.** The placeholders are expanded after markdown rendering
  (`expand_shortcode_tokens`).

Seeksnack content uses only the embedded `ref` shortcode (6 executions, TI A.3), which calls
`ref . .Params` → `Page::ref_`.

### 6.4 `resources.ExecuteAsTemplate`

`nh_tplfuncs::resources` (T15) → `nh_resource_transformers::resource_transformers::templates`
(T15) → a `ResourceTransformation` whose transform:

1. calls `TemplateStore::text_parse(ctx.in_path, content)`. Go parses a **text/template** named
   by the resource's input path.
2. sets `out_path = target_path` and runs `execute_with_context(ctx, ti, out, data)`.

The transformation key is `("execute-as-template", target_path)`. **Data is not part of the key**
(RP §10.8). The first execution for a target path wins for the rest of the build, across
languages. Seeksnack calls it with `(dict "api" .Site.Params.Comment.Apipro)` from both languages.

### 6.5 Template functions reaching site / page / resources / i18n

- Each namespace object holds `Arc<Deps>` of **its** site (`Namespace { d }`). The func map is
  built per site (`tplimplinit::create_func_map(d)`). Component tests build that `Deps` with
  `Deps::for_tests(conf)` plus the `with_*` services they need, so they do not wait for the
  construction task (T20).
- The current page is `TplContext::from_host(ctx).page` (Go `tpl.Context.Page`), set by the
  renderer and by `partial` (which clones the context with `with_current_template`).
- Site: `d.site()` (the `OnceLock<SiteRef>`). The `site` function returns it, and `hugo`
  returns `HugoInfo`.
- Resources: `d.resource_spec` + `nh_resource_transformers::resource_factories::create::Client`.
- i18n: `d.translate(ctx, id, data)` (the `OnceLock<TranslateFunc>`).
- Partials: `d.get_template_store().lookup_partial(name)` → `execute_with_context`. The
  `partialCached` cache is a dynacache partition keyed by `(name, variants…)`, first execution
  wins.
- Page content from functions: `markdownify` → `d.site().home()` → `Page::render_string`
  (implemented in hugolib, T22), then `TrimShortHTML`. `.Plain`/`.Content` → `Page` trait → hugolib.

### 6.6 Images

`images.Filter`/`images.Overlay` (T19) build `nh_images::filters::FilterObject`s.
`ResourceAdapter.Resize/Filter` (T14) → `ImageResource::process_image(conf)` → `ImageCache::get_or_create(rel_target_path, create)`
(first wins, cold path) → `decode_image()` of the PARENT'S ENCODED BYTES (`ImageCache.encoded`,
§4.5) → `nh_images::image::ImageProcessor::apply_filters/encode` (T10, gift + encoders) → the
new encoded bytes go back into `ImageCache.encoded`. The overlay filter decodes the watermark's
encoded PNG the same way. The file names are
`p1 + "_hu_" + HashStringHex(incomingID, root_hash, conf.key, imaging_source_hash) + ext` (IMG §4.1).

### 6.7 GetRemote and file caches

`resources.GetRemote(uri)` (T15, `create/remote.go`) → `remote_resource_keys` →
the `getresource` entry of `nh_helpers::cache::filecache::filecache::Caches` (T08) → `httpcache::transport` reads the
cached `httputil.DumpResponse` file. `AlwaysUseCachedResponse` is true, so there is never any
network access when the entry exists. Otherwise it returns an error in the acceptance setup. The
cache dir is `cacheDir` config → `$HUGO_CACHEDIR` → `~/Library/Caches/hugo_cache`, then
`/<basename(workingDir)>/filecache/getresource/<HashString(uri, nil)>` (AC §2.3).

### 6.8 External engines

- esbuild: Go links esbuild as a library. Rust runs the pinned **0.25.6 binary** instead
  (deviation, recorded in nh-esbuild PORTING.md). `nh_esbuild::service::client::ServiceClient`
  spawns `esbuild --service=0.25.6 --ping`, encodes build requests with `service::protocol`, and
  answers onResolve/onLoad callbacks with the Hugo resolver plugins (`resolve.rs`). The working dir
  (`AbsWorkingDir`) is the site dir. Binary lookup order: env `NEOHUGO_ESBUILD_BINARY`,
  `<workingDir>/node_modules/@esbuild/<os>-<arch>/bin/esbuild`, `<workingDir>/node_modules/.bin/esbuild`,
  then `esbuild` on `PATH`. The version must print `0.25.6`, otherwise it is an error. seeksnack
  has no esbuild in `node_modules`, and there is none on `PATH`. The scratch copy under
  `$W/resources-pipeline/esbuild` is not durable.
  - **Pinned binary:** `tools/esbuild/build.sh` (created by T16) runs
    `go build -o tools/esbuild/bin/esbuild github.com/evanw/esbuild/cmd/esbuild` in the repo root.
    That is the go.mod-pinned v0.25.6 (the version Go links), built offline from the module
    cache. It was checked: the result prints `0.25.6`, and go.mod/go.sum are unchanged.
    `tools/esbuild/.gitignore` ignores `bin/`.
  - Tests and the acceptance harness set `NEOHUGO_ESBUILD_BINARY=$REPO/tools/esbuild/bin/esbuild`.
- LibSass: `libsass-sys` (Wave A) via `tocss/scss/tocss.rs` with the Hugo importer callback.
- PostCSS: `nh_config::hexec::Exec::npx("postcss", …)` with Go's filtered environment
  (`get_exec_environ`) and the inherited cwd (site dir).

---

## 7. Deterministic render order

### 7.1 Reference model

A single-worker Go build (`HUGO_NUMWORKERMULTIPLIER=1`) reproduces every golden file:
`$SCRATCH/golden/canonical`, `canonical2` and the single-worker `$W/output-publishing/out-seq1` are
byte-identical (`diff -rq`: 0 differences), including the 26 term-collision files. **The Rust port
renders sequentially, in exactly that order:**

```
build():
  NewHugoSites                             # construction: deps, sites, template stores, translators (T20)
  process(&mut h)                          # collect files (sorted walk), parse, insert into trees (T20)
  assemble(&mut h)                         # per site in language order (Go step 1 runs per site in parallel but is order-independent) (T21)
  h = freeze(h); bind site refs / template stores / translators
  render(&h):
    for (site_idx, s) in sites (language order):
      for (i, fmt) in s.render_formats (global index = offset(site_idx) + i):
        for s2 in sites: h.prepare_pages_for_render(s2, s2 == s, global_i)  # shiftToOutputFormat on ALL pages
        nh_page::pages_cache::clear()                                   # Go page.Clear() at the start of EVERY Site.render
        if first format of the first build: s.render_aliases()           # before pages: real pages overwrite aliases
        s.render_pages(): walk tree_pages in key order (NodeShiftTreeWalker, site's language),
            for each page with that output format:
              skip standalone pages unless should_render_standalone_page
              page.render_resources()             # publish all bundle resources (once)
              resolve template; render_and_write_page()
              if page has a paginator: render_paginator()   # page/1 alias, then page/2..N
        if standalone-rendering context: render_main_language_redirect()
  write_build_stats(&h)                    # hugo_stats.json into the working dir
  post_process(&h)                         # placeholders → PostProcess results (postcss runs here)
```

The oracle for the page order is `$W/output-publishing/trace_targets.txt` (3,203 target paths in
publish order from a single-worker traced run). `$W/output-publishing/out-seq1` is the full
single-worker output.

### 7.2 First-writer-wins and other order-dependent state

With sequential rendering these all come out deterministic. Each must be implemented as
"first call wins" (`OnceLock`/dynacache `get_or_create`), not recomputed:

| State | Go | Rust | Effect in golden |
|---|---|---|---|
| `resources.Concat` target | `ResourceCache.GetOrCreate(targetPath)` (`bundler.go:84-85`) | `ResourceCache::get_or_create` keyed by target path | `js/all.js` part order from `single.html` (the first single page `/almonds/...` renders before any term) → one `website.<sha>.js` for 1,701 pages |
| `resources.ExecuteAsTemplate` | cached by target path, ignores data | same | `ts/search.ts`, `ts/comment.ts` keep the data of their first execution (EN) |
| Paginator | `sync.Once` shared by `.Paginator`/`.Paginate` per page output; `reset()` on a rendering-site shift of a built paginator | resettable `PagePaginator { init: Mutex<PagePaginatorInit> }`, first stored wins until reset | `head.html` calls `.Paginator` first, so the home paginates `RegularPages` (TI §6.1) |
| Query caches `cachePages1/2` | dynacache | `Partition` get_or_create (§4.8) | term `Pages()` vs `RegularPages()` share a key (CM §12.3) |
| Sorted-pages cache `spc` | global, keyed by name + input list identity; `page.Clear()` per `Site.render` | `nh_page::pages_cache` (T12), cleared by `render_site` (T24) | hits return the list sorted at first computation |
| `partialCached` | dynacache | partition get_or_create | — |
| Scratch | per page, shared across outputs and pagers | `Arc<Scratch>` in `PageCommon` | `index.json` `.Scratch.Add` (TI §6.2) |
| Rendered content per page/scope/format | dynacache on the site's pageMap | `PageMap.cache_content_*` partitions (§7.4) | content rendered in the **current output format** of the page at first use |
| Image processing | `ImageCache` by `relTargetPath` | same | pure, but publishing happens on first `.RelPermalink` |
| Resource publish | `publishOnce` | `OnceLock<()>` | 531 bundle files + processed images + JS written once |
| Term collisions | last writer wins | last writer wins in tree-key walk order (sequential) | 26 files. NO tolerance: `canonical2` is byte-identical to `canonical`, so the Rust output must equal the single-worker last writer |
| Term `.Data.Term`, `.Title`, `.Name` | `m.term` = the LAST value seen in key order; `.Title` = AP-title-case (`CreateTitle`) of that last `m.term`; `.Name` = the FIRST creating value (`Unnormalized().BaseNameNoIdentifier()`) | same walk order | CM §0.6, §7.3 |
| `BuildState.Incr()` (PostProcess ids) | one atomic counter per build, shared by all sites through `SpecCommon` | `BuildState` as `Incrementer` (§4.5), the only counter | `__h_pp_l1_1_` |

### 7.3 Static copy

Go runs `copyStatic` and `buildSites` concurrently. The port runs `copyStatic` first, then the
build (`nh_commands::hugobuilder`). The paths are disjoint for seeksnack (AC §3.8).

### 7.4 Content scopes and output formats

A page's content is seen through a scope object per (markup scope + output format name) in
`CachedContent.scopes` (`page__content.go:171-180`, first creator wins). The rendered
content/TOC/plain results are cached in the page's SITE `PageMap` partitions
(`cache_content_rendered`/`cache_content_toc`/`cache_content_plain`, Go `/cont/ren|toc|pla/<site>`),
keyed by `sourceKey + "/" + markupScope(ctx) + outputFormat.Name` (`page__content.go:522-530, 680,
800`). They are not per-scope `OnceLock`s. The output format that counts is the **page's current
output** (`current_output_idx`), and `prepare_pages_for_render` shifts every page of every site
before each format renders. So `index.json` calling `.Plain` on page P renders P's content in the
JSON output: the JSON table hook (`render-table.json.json`, text/template) runs, and the result
is cached under the JSON key.

`shiftToOutputFormat` (`page.go:675-747`, `nh_hugolib::page__init`, T21; port exactly). Note
that outputs are shared by format name (§4.3), so shifting an EN page to th/html selects the
same `PageOutput` as en/html:

- `init_page()`. A page with one output always uses index 0. Then set `current_output_idx`.
- On the **rendering site**:
  - If the current output has a BUILT paginator (`current != nil`), `reset()` it (page.go:691-694).
  - Take `cp = po.pco`. If it is empty and `canReusePageOutputContent()`
    (`pageOutputTemplateVariationsState == 1`: no render hook or shortcode template varied by
    output format; `page__per_output.go:234-340`), use the first other output's `pco`. If there is
    still nothing, create `newPageContentOutput(po)`.
  - `po.set_content_provider(cp)` sets both `provider` and `pco`.
- For pages of **other sites**, the page-level `pco` is NOT touched:
  - If the current output's `provider` is `Lazy`, `reset()` it.
  - Otherwise install a new `LazyContentProvider` whose factory creates
    `newPageContentOutput(po)` on first use (page.go:719-743).
  - Because outputs are shared by name, this replaces the provider of the output the page's own
    site used earlier. The next rendering-site shift installs a real `pco` again.
- The lazily created content output reaches the same `CachedContent.scopes` entry (key
  `scope + format name`, first creator wins), and the same `PageMap` content caches.

---

## 8. PostProcess and hugo_stats ordering

1. **Render.** The publisher (`nh_publisher::publisher::DestinationPublisher::publish`, T07)
   applies `urlreplacers` (absURL/canonify; RSS always, HTML because `canonifyURLs = true`) →
   minifier (`minify.minifyOutput`) → writes the file through the `HasBytesReceiver`-wrapped
   publish fs → **and** feeds the same bytes to the site's `HtmlElementsCollector` for HTML
   formats (pages, 404, aliases). Empty output is not written. The collector sees the minified,
   canonified bytes, and the PostProcess placeholders are still in place (inside `<style>`,
   which the collector skips).
2. `HasBytesReceiver` (nh-hugofs `hasbytes_fs`, T05) records every published text file that
   contains `__h_pp_l1` or `__hdeferred/` into `Deps.build_state.filenames_with_post_prefix`.
3. **write_build_stats** (T24): merge all sites' collectors, `sort.Strings` tags, classes and ids,
   encode with Go `json.Encoder` semantics (`SetEscapeHTML(false)`, `SetIndent("", "  ")`,
   trailing `\n`, nil → `null`), and write `<workingDir>/hugo_stats.json` unless identical bytes are
   already there. Oracle: `$SCRATCH/golden/canonical.hugo_stats.json`, and
   `$W/resources-pipeline/stats_repro.py`.
4. `renderDeferred` (no-op for seeksnack), then the unused-template/path warnings (logging only).
5. **post_process** (T24 + T14): `jsconfig.json` only if the JS config builder has roots (not
   for seeksnack). Then, for each recorded filename (sorted), read the file, and for each
   `__h_pp_l1…__e=` field call `PostPublishResource::get_field_string`. For `Content` this runs
   the lazy chain `toCSS → postCSS → minify → fingerprint` **now**. purgecss reads the
   `hugo_stats.json` written in step 3. The raw CSS is inserted with no HTML minify and no absURL,
   and the file is rewritten if it changed.

Steps 1 → 3 → 5 must be strictly ordered. Nothing may evaluate the PostProcess delegate's
`Content` before step 5. `PostProcess` itself only computes the transformation key.

---

## 9. Error handling and logging

- One error type: `nh_common::herrors::Error { msg, kind: ErrorKind, pos: Option<FilePos> }` and
  `nh_common::Result<T>`. `ErrorKind` covers the cases Go code branches on: `NotExist`,
  `FeatureNotAvailable` (postcss missing → cache fallback), `Fatal` (filecache), `ExecNotFound`,
  `Template`. Error **texts** are not part of parity, but keep Go's wording where cheap: it makes
  diffs against Go logs easy.
- Template-facing code returns `go_value::Result` (`GoResult`). `From` conversions go both ways,
  so `?` works across the boundary.
- `nh_common::loggers::Logger`: levels, counters, stderr output. **An `ERROR` log fails the build**
  (Go `hugo_sites_build.go:212-223`). `WARN`s (e.g. `printPathWarnings`) are informational.
- No panics on input (README rule 9). `todo!()` must not remain on the executed path at I01.
  Unported features return the explicit unsupported error.
- Go `panic`s that Hugo recovers from and turns into errors (template exec) become `Err`.

---

## 10. Byte-parity checklist specific to the Hugo layer

These are the cross-cutting facts that break parity if one task gets them wrong. Each is owned by
the task named in brackets.

1. Cold resource cache: never read `resources/_gen`. Image `Key()` has the `_<xxh64>` suffix [T14].
2. `--clock` → `htime::set_clock(StartClock)`, applied in `ConfigFromProvider` [T25, T01]. The
   language location is UTC (`time.LoadLocation("")`) [T03].
3. GetRemote is served from `~/Library/Caches/hugo_cache/seeksnack/filecache/getresource/` with
   decimal hashstructure keys. Never build with `--ignoreCache` [T15, T08].
4. The working directory basename must be `seeksnack` [acceptance harness, I02].
5. Canonify (absURL) before minify. The RSS format always canonifies [T07].
6. `hugo_stats.json` is collected from the published (minified) bytes, including aliases and 404,
   excluding static files [T07, T24].
7. Paginator, Concat, ExecuteAsTemplate, `cachePages`: first call wins [T23, T15, T21].
8. The imaging config `SourceHash` includes the `_merge` keys (`4bf645f71319dd1d`) [T09, T04].
9. html/template is the go1.24 fork semantics [Wave A gotemplate; T13 wires it].
10. x/text collate CLDR 23 with one collator pair PER LANGUAGE TAG (Go `langs.NewLanguage`:
    `collate.New(tag)`, English only if the tag does not parse) [T03]. Page sorts use the collator
    of `Site.Current()` (= `h.current_site`, the site being rendered) at the time the list is first
    computed and cached [T12, T21]; template `sort` uses the namespace's language [T18]. For
    seeksnack's strings `th` == `en`, but the port is per tag and T03 tests both tags.
11. Term tree key vs sanitized output path [T21, T11].
12. `shiftToOutputFormat`: content renders in the page's current output format [T23, T22].
13. Alias pages render before real pages, use the HTML format, and are canonified [T24].
14. `AddHugoGeneratorTag = DisableHugoGeneratorInject` (inverted, so no tag is injected) [T23/T07].
15. robots.txt: `text/plain`, no minifier, no trailing newline added [T24].
16. Static copy is verbatim, including `.DS_Store` and `.bak` [T25].
17. esbuild 0.25.6 with cwd = site dir (the `node_modules/mustache/mustache.js` key survives
    minification). Target es2015 for level-1 builds, esnext for the minified bundle [T16].
18. postcss: filtered env, `HUGO_ENVIRONMENT=production`, cwd = site dir, config path [T16, T04].
19. Only `.Permalink`/`.RelPermalink` publish resources. Intermediate chain results are never
    written [T14].
20. `now` is only used for the year, but `shouldBuild` compares publish/expiry dates with it
    [T21: `build_assemble.rs`, the Go code is `site.go:1556-1578`, called only by the assembly
    walks].
21. Page outputs are shared by format NAME across the global render-format index (§4.3); the
    paginator and content provider follow from that [T21, T22, T23].
22. Processed images decode the parent's ENCODED bytes, never reused pixels (§4.5) [T14].
23. `page.Clear()` (the global sorted-pages cache) at the start of every `Site.render` [T12, T24].
24. Nil values follow the §4.7 table (`<no value>` vs `<nil>`, nil interfaces become untyped nil
    as arguments) [gotemplate contract C7/C8, T23].
25. Render hooks run with the caller's context unchanged; only `{{% %}}` shortcodes set
    `is_in_goldmark` [T22].

---

## 11. Verification

### 11.1 Per task

Each crate has `tools/go-oracle/<crate>/<topic>/` (Go, `package main`, inside the neohugo module,
so it imports the exact Go code). It emits JSON fixtures to `crates/<crate>/tests/fixtures/<topic>/`
that the Rust tests (`crates/<crate>/tests/<topic>.rs`, reading them with the `serde_json`
dev-dependency) replay. The topic level keeps tasks that share a crate out of each other's way.

A task's acceptance test may need only the code of the tasks in its `depends_on`. Where the real
collaborator comes later, the test uses a seam and the end-to-end check moves to I01:

- the names-only func map (T20);
- `Deps::for_tests` (T15, T17, T18, T19);
- hook/shortcode replay through `TemplateExecutor` (T22);
- a stub `TemplateExecutor` (T24);
- a minimal test FuncMap (T13).

Existing artifacts to reuse:

| Task | Crates | Oracle program(s) and fixtures (details: `oracle_and_acceptance_tests` in the plan) | Acceptance |
|---|---|---|---|
| T26 common-thirdparty-ports | nh-common | go-oracle nh-common/{cast,locales,flect,prose} over site strings/dates | all fixture strings and Go kinds equal |
| T01 common-values | nh-common | go-oracle nh-common/values; hashing vectors (IMG §4.5, AC §2.3); Go *_test.go tables | vectors equal; Params/Scratch semantics equal |
| T27 doctree | nh-doctree | go-oracle nh-doctree: tree ops over the site's page/resource/template keys | walk/prefix/shape dumps equal |
| T02 common-paths-text | nh-common | go-oracle nh-common/paths: PathParser dump of every site file; Sanitize/URLize strings | PathParser dump identical for every file |
| T03 parser-langs | nh-langs, nh-parser | $W/content-model/fmdump/fm.json; go-oracle nh-parser/{lex,decode}; collation signs from Go for the `en` AND `th` tags | 251 front matters (value+kind), lexer items, 1.13M collation pairs × 2 tags equal |
| T04 config-base-media | nh-config, nh-media | go-oracle nh-config/{provider,decode}, nh-media; imaging SourceHash; `config.SitemapConfig` fields | config provider/decoder/media/output-format dumps equal |
| T05 hugofs-vfs | nh-hugofs | go-oracle nh-hugofs/walk: BaseFs component walks | walk dumps equal; 23 static files |
| T06 markup | nh-markup | $W/markdown/{mdref,corpus-nohooks,contentsite-out}; go-oracle nh-markup/hooks | 251 files HTML equal; hook contexts + TOC equal |
| T07 transform-publisher | nh-publisher, nh-transform | $W/output-publishing/absurl_ref.py; $W/minify/{recorder,vectors}; stats_repro.py | absURL+minify equal; hugo_stats elements equal |
| T08 helpers-source-cache | nh-helpers | go-oracle nh-helpers/pathspec; source.File; 51 getresource cache entries | fixtures equal; 51 responses parse like Go |
| T09 allconfig-modules | nh-allconfig, nh-hugofs | $W/i18n-lang-misc/config-dump.*; go-oracle nh-allconfig (all sections, mounts) | section dumps and mounts equal |
| T10 images | nh-images | $W/images/{repro,vectors,inventory,*.objdump} | 1,461 processed images byte-identical |
| T11 page-api-paths | nh-page, nh-resource | $W/content-model/model_full.json; go-oracle nh-page/{paths,frontmatter} | target paths, permalinks, dates equal |
| T12 page-collections | nh-page | model_full.json orders/related/pagers; go-oracle nh-page/collections | all orders, Related, pager membership equal |
| T13 tplimpl | nh-tpl, nh-tplimpl | $W/templates-inventory/{store.txt,lines.tsv}; $W/template-engine/{escdump,tsite,expected} with a minimal test FuncMap | store, layout choice, escaped trees equal; probe lines covered by the minimal FuncMap equal |
| T14 resources-core | nh-resources | $W/resources-pipeline/keycalc; images vectors; go-oracle nh-resources (incl. chained Resize→Filter via the API, `Slice`) | keys/links/target paths/bytes equal; no _gen reads |
| T15 resource-factories | nh-resource-transformers, nh-tplfuncs | 51 getresource entries; $W/resources-pipeline/repro_all.sh (Concat, ExecuteAsTemplate) | GetRemote/Concat/fingerprint outputs equal; first-writer keys |
| T16 js-css-pipeline | nh-esbuild, nh-resource-transformers | $W/resources-pipeline/{repro_all.sh,repro_out,esbuild/cli,libsass}; esbuild from tools/esbuild/build.sh | 3 JS bundles + 51,857-byte CSS equal |
| T17 i18n | nh-i18n | $W/i18n-lang-misc/fixtures-i18n.json; go-oracle nh-i18n (plurals) | all i18n call-site outputs equal |
| T18 tplfuncs-data | nh-tplfuncs | go-oracle nh-tplfuncs/data (TI §2.1 arg-type combos); Go *_test.go tables | results equal (value + Go kind) |
| T19 tplfuncs-host | nh-tplfuncs | $W/i18n-lang-misc/fixtures-{strings,dates}.json; go-oracle nh-tplfuncs/host | results equal |
| T20 hugolib-capture | nh-deps, nh-hugolib | model_full.json allPages; go-oracle nh-hugolib/{capture,funcnames} | NewHugoSites works; trees after process and per-page capture dumps equal |
| T21 hugolib-assemble | nh-hugolib | model_full.json after assembly; $W/content-model/{probe,py/terms.py}; page-output lifecycle unit tests | page set, meta, titles, collections, taxonomies, target paths equal |
| T22 hugolib-content | nh-hugolib | $W/markdown/{contentsite-out,contentmap.json}; go-oracle nh-hugolib/{hookrec,content} | Content/Plain/Summary/TOC per page+format equal with replayed hooks/shortcodes |
| T23 hugolib-site | nh-hugolib | model_full.json; $W/templates-inventory/trace*.tsv; go-oracle nh-hugolib/{site,data} | method tables cover TI A.2; site/pager/sitemap/nil/data results equal |
| T24 hugolib-build | nh-hugolib | $W/output-publishing/trace_targets.txt; go-oracle nh-hugolib/pagers; recorded stats inputs | publish order (stub executor) + stats/post-process bytes equal |
| T25 commands-cli | nh-commands | neohugo-go config vs neohugo config; static copy | CLI parses like Go; 23 static files equal |

### 11.2 End-to-end

```
cp -R $SCRATCH/canon/pristine-seeksnack $TMP/seeksnack      # basename must be "seeksnack"
rm -rf $TMP/seeksnack/resources $TMP/seeksnack/hugo_stats.json   # cold cache, like the golden
tools/esbuild/build.sh                                            # once: pinned esbuild 0.25.6 (T16)
cd $TMP/seeksnack && NEOHUGO_ESBUILD_BINARY=$REPO/tools/esbuild/bin/esbuild \
    <neohugo-rs> --minify --clock 2026-09-27T12:00:00Z -d $TMP/out
python3 $SCRATCH/tools/compare.py $TMP/out --golden $SCRATCH/golden/canonical
cmp $TMP/seeksnack/hugo_stats.json $SCRATCH/golden/canonical.hugo_stats.json
```

The run passes when all of the following hold:

- 6,943/6,943 files are identical, with no missing or extra files. There is no alternate golden:
  `canonical2` is byte-identical to `canonical`, so the 26 term-collision files must match exactly.
- `hugo_stats.json` is identical.

To tell template problems from minifier problems, first build without `--minify` and compare
against the single-worker unminified output (`$W/output-publishing/out-nomin-seq`, same clock
caveats as noted there). The Go binary for fresh references is `$SCRATCH/bin/neohugo-go`.

---

## 12. Task overview

| task | owns | Go lines (neohugo total / executed + third-party) | depends on |
|---|---|---|---|
| T26 common-thirdparty-ports | nh-common::cast::*, nh-common::locales, nh-common::flect, nh-common::prose | 0 / 0 + 5290 | — |
| T01 common-values | nh-common::object, nh-common::herrors, nh-common::constants, nh-common::collections::*, nh-common::hashing, nh-common::hreflect, nh-common::htime, nh-common::maps::*, nh-common::math, nh-common::predicate, nh-common::types::*, nh-common::compare, nh-common:... | 5111 / 1563 + 100 | T26 |
| T27 doctree | nh-doctree (whole crate) | 1110 / 410 + 0 | T01 |
| T02 common-paths-text | nh-common::paths::*, nh-common::urls, nh-common::text, nh-common::hstrings, nh-common::hugio, nh-common::loggers, nh-common::kinds, nh-common::files, nh-common::glob::* | 3726 / 1316 + 1000 | T01, T26 |
| T03 parser-langs | nh-parser (whole crate), nh-langs (whole crate) | 2566 / 1059 + 1900 | T01, T02, T26 |
| T04 config-base-media | nh-config (whole crate), nh-media (whole crate) | 4495 / 1567 + 1500 | T01, T02, T03 |
| T05 hugofs-vfs | nh-hugofs::afero, nh-hugofs::overlayfs, nh-hugofs::fs, nh-hugofs::fileinfo, nh-hugofs::rootmapping_fs, nh-hugofs::component_fs, nh-hugofs::decorators, nh-hugofs::dirsmerger, nh-hugofs::walk, nh-hugofs::hasbytes_fs, nh-hugofs::filename_filter_fs, nh-hugofs::... | 3562 / 1850 + 2110 | T01, T02, T04 |
| T06 markup | nh-markup (whole crate) | 5331 / 1916 + 0 | T01, T02, T04 |
| T07 transform-publisher | nh-transform (whole crate), nh-publisher (whole crate) | 1572 / 876 + 4500 | T01, T02, T04, T05, T08 |
| T08 helpers-source-cache | nh-helpers (whole crate) | 2626 / 1028 + 1300 | T01, T02, T04, T05, T06 |
| T09 allconfig-modules | nh-hugofs::modules::*, nh-allconfig (whole crate) | 5149 / 2185 + 0 | T03, T04, T05, T06, T07, T08, T10, T11, T12 |
| T10 images | nh-images (whole crate) | 2879 / 767 + 0 | T01, T04, T11 |
| T11 page-api-paths | nh-resource (whole crate), nh-page::page, nh-page::site, nh-page::page_nop, nh-page::page_outputformat, nh-page::page_paths, nh-page::permalinks, nh-page::page_matcher, nh-page::page_markup, nh-page::page_data, nh-page::page_kinds, nh-page::page_author, nh-... | 5373 / 1481 + 0 | T01, T02, T03, T04, T05, T08 |
| T12 page-collections | nh-page::pages, nh-page::pages_sort, nh-page::pages_sort_search, nh-page::pages_cache, nh-page::pages_language_merge, nh-page::pages_prev_next, nh-page::pages_related, nh-page::pagegroup, nh-page::pagination, nh-page::taxonomy, nh-page::weighted, nh-page::p... | 3824 / 973 + 0 | T01, T03, T11 |
| T13 tplimpl | nh-tpl (whole crate), nh-tplimpl (whole crate) | 3693 / 2254 + 0 | T01, T02, T04, T05, T06, T08, T11, T27 |
| T14 resources-core | nh-resources (whole crate) | 3097 / 1326 + 0 | T08, T09, T10, T11, T13 |
| T15 resource-factories | nh-resource-transformers::resource_factories::*, nh-resource-transformers::resource_transformers::integrity, nh-resource-transformers::resource_transformers::minifier, nh-resource-transformers::resource_transformers::templates, nh-tplfuncs::internal::resour... | 1683 / 853 + 0 | T07, T08, T13, T14 |
| T16 js-css-pipeline | nh-esbuild (whole crate), nh-resource-transformers::resource_transformers::js::build, nh-resource-transformers::resource_transformers::js::transform, nh-resource-transformers::resource_transformers::cssjs::postcss, nh-resource-transformers::resource_transfo... | 2626 / 1130 + 1600 | T04, T05, T14 |
| T17 i18n | nh-i18n (whole crate) | 344 / 224 + 2000 | T03, T05, T09, T13 |
| T18 tplfuncs-data | nh-tplfuncs::cast::*, nh-tplfuncs::collections::*, nh-tplfuncs::compare::*, nh-tplfuncs::crypto::*, nh-tplfuncs::encoding::*, nh-tplfuncs::fmt::*, nh-tplfuncs::hash::*, nh-tplfuncs::math::*, nh-tplfuncs::reflect::*, nh-tplfuncs::safe::*, nh-tplfuncs::intern... | 4808 / 2044 + 0 | T01, T03, T12, T13 |
| T19 tplfuncs-host | nh-tplfuncs::internal::registry, nh-tplfuncs::tplimplinit, nh-tplfuncs::css::*, nh-tplfuncs::data::*, nh-tplfuncs::debug::*, nh-tplfuncs::diagrams::*, nh-tplfuncs::hugo::*, nh-tplfuncs::images::*, nh-tplfuncs::inflect::*, nh-tplfuncs::js::*, nh-tplfuncs::la... | 5871 / 1698 + 0 | T10, T12, T13, T15, T16, T17, T23 |
| T20 hugolib-capture | nh-deps (whole crate), nh-hugolib::hugo_sites, nh-hugolib::build_process, nh-hugolib::template_exec, nh-hugolib::pages_capture, nh-hugolib::content_map, nh-hugolib::content_map_trees, nh-hugolib::page__meta, nh-hugolib::page__new, nh-hugolib::page__content_... | 4263 / 2474 + 0 | T03, T05, T07, T09, T11, T12, T13, T14, T17, T27 |
| T21 hugolib-assemble | nh-hugolib::content_map_page, nh-hugolib::page__meta_post, nh-hugolib::build_assemble, nh-hugolib::page__init, nh-hugolib::page__paths, nh-hugolib::pagecollections, nh-hugolib::collections, nh-hugolib::page__data, nh-hugolib::page__tree, nh-hugolib::site_se... | 3216 / 2269 + 0 | T12, T20, T22 |
| T22 hugolib-content | nh-hugolib::page__content, nh-hugolib::page__output, nh-hugolib::page__per_output, nh-hugolib::shortcode, nh-hugolib::shortcode_page | 2114 / 1213 + 0 | T03, T06, T13, T20 |
| T23 hugolib-site | nh-hugolib::site, nh-hugolib::site_output, nh-hugolib::hugo_sites_data, nh-hugolib::page, nh-hugolib::page__common, nh-hugolib::page__menus, nh-hugolib::page__paginator, nh-hugolib::page__position, nh-hugolib::page__ref, nh-hugolib::page_unwrap, nh-hugolib:... | 2789 / 987 + 0 | T09, T12, T13, T14, T20, T21, T22 |
| T24 hugolib-build | nh-hugolib::hugo_sites_build, nh-hugolib::site_render, nh-hugolib::alias | 1810 / 1012 + 0 | T07, T14, T21, T22, T23 |
| T25 commands-cli | nh-commands (whole crate) | 2340 / 1025 + 370 | T05, T09, T24 |
| I01 hugolib-integration-first-full-build | all nh-* crates (exclusive; starts after T01–T27 have landed) | — | T01, T02, T03, T04, T05, T06, T07, T08, T09, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27 |
| I02 golden-diff-loop-pages | nh-common, nh-parser, nh-langs, nh-config, nh-media, nh-hugofs, nh-markup, nh-transform, nh-tpl, nh-helpers, nh-page, nh-doctree, nh-allconfig, nh-tplimpl, nh-publisher, nh-deps, nh-i18n, nh-hugolib, nh-commands, nh-tplfuncs (all namespaces except resources... | — | I01 |
| I03 golden-diff-loop-assets-postprocess | nh-resource, nh-images, nh-resources, nh-esbuild, nh-resource-transformers, nh-tplfuncs::{resources, images, js, css} | — | I01 |
| I04 hardening-and-workspace | all nh-* crates (exclusive; after I02 and I03), crates/Cargo.toml (new workspace, if the orchestrator approves) | — | I02, I03 |

Tasks may start coding against the skeleton immediately. `depends_on_tasks` lists the tasks whose
code must land before this task's acceptance tests can pass. The critical path is:

T26 → T01 → T02 → T03 → T04 → T05/T06 → T08 → T07/T11 → T12/T13 (T27 alongside) → T09 → T14 →
T20 → T22 → T21 → T23 → T24 → T25 → I01 → I02/I03 → I04.

### 12.1 Build-phase split of nh-hugolib

The hugolib Go files call each other across phases. The capture phase needs the content trees,
content parsing and shortcode extraction. Assembly calls `shiftToOutputFormat`, which creates
page paths and page outputs. So "one Go file = one task" would make every hugolib acceptance test
depend on later tasks. Instead, each task owns one build phase, and the Go files that straddle
phases are split into modules whose checklists hold exactly their line ranges:

| Phase (task) | Owns | Split modules (Go lines) | Acceptance needs |
|---|---|---|---|
| construction + process (T20) | nh-deps, `hugo_sites`, `pages_capture`, `content_map`, `page__meta`, `page__new`, `page_kinds`, `file_info`, `template_exec` | `build_process` (hugo_sites_build.go process*), `content_map_trees` (content_map_page.go 88-259, 636-994), `page__content_parse` (page__content.go 60-224, 266-490), `shortcode_parse` (shortcode.go 193-310, 526-545, 562-736); construction from site.go 143-454, 858-867 in `hugo_sites` | T13 store (shortcode lookup), T14 ResourceSpec, T17 translators, T27 trees; a names-only func map |
| content (T22) | `page__content`, `page__output`, `page__per_output`, `shortcode`, `shortcode_page` | render halves of page__content.go and shortcode.go; site.go hookRendererTemplate | pages from T20; hooks/shortcodes replayed |
| assembly + page outputs (T21) | `content_map_page`, `pagecollections`, `collections`, `page__data`, `page__tree`, `site_sections`, `page__paths` | `build_assemble` (hugo_sites_build.go assemble; site.go initRenderFormats, shouldBuild), `page__meta_post` (page__meta.go setMetaPost*, applyDefaultValues), `page__init` (page__meta.go initLazyProviders; page.go initPage, initCommonProviders, shiftToOutputFormat) | T20, T22 constructors, T12 |
| template API (T23) | `site`, `page`, `page__common`, `page__menus`, `page__paginator`, `page__position`, `page__ref`, `page_unwrap`, `tplapi::*`, … | `hugo_sites_data` (hugo_sites.go Data/loadData/handleDataFile) | T20–T22 |
| render + post-process (T24) | `hugo_sites_build`, `site_render` (+ site.go render), `alias` | — | T21–T23; stub executor |

T21 runs its acceptance after T22 only because `assembleResources` initialises page outputs
through T22's constructors. T22's own tests construct `PageOutput`/`PageContentOutput` directly.

---

## 13. Open issues and risks

1. Wave A crates still missing when this design was revised: gotemplate, gift, go-png,
   tdewolff-minify-js; go-flate, go-fmt, go-json, goldmark, tdewolff-minify, tdewolff-parse-js,
   libsass-sys and libwebp-sys exist but have not landed.
   - Landed crates are wired in the nh-* `Cargo.toml` files. The others stay commented (§1
     rule 4).
   - The gotemplate API is pinned by `crates/GOTEMPLATE_CONTRACT.md`. The orchestrator must hand
     that file to the gotemplate task as acceptance criteria. T13 reviews the result.
2. esbuild is run as the pinned 0.25.6 binary over the --service protocol (Go links esbuild as a
   library). This deviation needs orchestrator sign-off. The binary is built from the go.mod-pinned
   module by `tools/esbuild/build.sh` (T16). T16 must confirm that the 3 bundles are
   byte-identical with it.
3. PostCSS (autoprefixer/cssnano/purgecss) needs node and the site's exact node_modules install;
   the acceptance machine must match the golden machine.
4. Resolved: there is no term-collision tolerance. `canonical`, `canonical2` and the single-worker
   `out-seq1` are byte-identical, so the 26 collision files must be the single-worker last writer
   in tree-key order (T24 checks the publish order against `trace_targets.txt`).
5. Page identity for wrapped pages (pageWithWeight0 / pageForShortcode / pageForRenderHooks) in
   eq/in/uniq: PageRef::identity() uses the wrapped page id; T18 must confirm Go's
   interface-equality result for wrapped-vs-unwrapped comparisons.
6. RP §11.1 observed different _hu_ image names for a site copy at another absolute path (probably
   a warm resources/_gen in that clone); IMG §4.1's formula has no path input. I03 must confirm
   path independence.
7. Static copy runs before buildSites (Go runs them concurrently). Equivalent for seeksnack
   (disjoint paths) but a deviation in general.
8. TOML decoding: the toml crate + go-toml/v2 type mapping (T03) needs a README rule 2
   justification; the alternative is a full go-toml/v2 parser port (~5k lines).
9. golang.org/x/net/html for the stats collector is a large port for little seeksnack impact
   (stats_repro.py reproduces the golden with a simple tokenizer); T07 must still port the
   tree-builder quirks faithfully or document the gap.
10. Rendering is sequential for determinism; build time will be several times Go's. Parallel
    rendering with deterministic first-writer resolution is left for after parity.
11. The shared-crate model relies on the crate lead owning lib.rs/Cargo.toml/parent module files;
    dependency additions by non-lead tasks must be coordinated.
12. Golden regeneration: golden/canonical used --clock 2026-09-27T12:00:00Z; GetRemote data and the
    node_modules versions are frozen in the cache/install. Any refresh of the cache or node_modules
    invalidates the golden.
13. T22's hook recorder (`tools/go-oracle/nh-hugolib/hookrec`) needs a patched copy of the Go code
    (wrapping `hookRendererTemplate.Render*` and `renderShortcodeWithPage`). The patch lives in the
    oracle directory and is applied to a temporary copy. The repository sources are never
    modified.
14. The template-engine behaviours that the host cannot see are covered only by the gotemplate
    crate's own differential tests (contract C14) and by I01. Examples: a nil `any` result in a
    field chain (`.A.B` with `A` returning nil `any` errors in Go, while the Rust hosts return
    `Invalid`). These are error-only paths, so they cannot change the bytes of a successful build.

---

## 14. Design decisions

Responses to the completeness review of this design (critique P0-1 … P2-21). Where the design
differs from the reviewer's suggestion, the reason is given.

1. **gotemplate contract (P0-1).** `crates/GOTEMPLATE_CONTRACT.md` pins the host contract (C1–C14).
   It extends template-engine spec §16, which the orchestrator had already made gotemplate's
   acceptance criteria. It adds the items that were missing there:
   - `ExecHelper::is_true`;
   - the `evalField` order for every receiver kind;
   - ideal-constant and nil conversions (C3, C7);
   - `<no value>` vs `<nil>` (C8);
   - the `printable_value` unwrap;
   - `Arc` identity for `mainsections`;
   - re-entrant nested execution.

   `engine.rs` mirrors C1 as a Rust trait, and T13 is the reviewer. The contract uses
   `has_method` + `call_method` rather than a single `get_method` returning a bound method. Go looks
   the method up before it evaluates the arguments, and a two-step API keeps that order without
   boxing a closure per call.
2. **hugolib ownership (P0-2): re-sliced by build phase, not merged into two tasks.** Merging
   T20–T23 into two tasks would give two ~3,500-executed-line tasks and still leave
   inversions: assembly calls `shiftToOutputFormat`, and capture needs the template store.
   Instead, every Go file that straddles phases is split (§12.1).
   - The moves the reviewer listed are all done: the trees, shifter and `newPageMap` go to T20;
     the parse half of `page__content.go` goes to T20; `initLazyProviders`/`initPage` go to the
     assembly task (T21, not T23).
   - Moves the review did not list were also needed:
     - construction (`NewHugoSites`/`newHugoSites`/`Deps.Init`) goes to T20, with nh-deps and the
       nh-hugolib crate lead;
     - `process` goes to T20, and `assemble`, `initRenderFormats` and `shouldBuild` go to T21;
     - shortcode extraction goes to T20 (called during parsing, and needs the template store);
     - `setMetaPost*`/`applyDefaultValues` go to T21 (only the assembly walks call them);
     - `shiftToOutputFormat` and `newPagePaths` go to T21 (`assembleResources` calls them);
     - data loading goes to T23.
   - T21's acceptance now runs after T22, because T21 uses T22's output constructors.
3. **doctree (P0-3).** It is a separate early task, T27, which depends only on T01. The crate's
   nh-resource dependency was dropped (it was used only by rebuild-only `MarkStale`).
4. **Acceptance dependencies (P0-4).**
   - T09 depends on T12.
   - T15/T17/T18 use `Deps::for_tests` (implemented in the skeleton; nh-deps now has
     `Option` services and `with_*` builders). So T17 no longer depends on the hugolib tasks, and
     T15/T18 never did.
   - T22 replays Go-recorded hook and shortcode outputs through `template_exec::TemplateExecutor`.
     The real hooks are tested in I01.
   - T24 is narrowed to the render order with a stub executor, stats from recorded collector
     inputs, and post-process on fixtures. The full-build checks move to I01, so T15, T16 and T19
     are no longer T24 dependencies.
   - T13 uses a minimal test FuncMap. The rest of the probe site moves to I01.
5. **Outputs shared by name (P1-5).** `PageLazy.outputs: Vec<Arc<PageOutput>>`, with one `Arc` per
   format name.
6. **shiftToOutputFormat state (P1-6).**
   - The paginator is `Mutex<PagePaginatorInit>` with `reset()`. It keeps Go's quirk that only
     the first failing call sees the error.
   - `PageOutput` has `pco: Mutex<Option<..>>` and `provider: Mutex<ContentProviderSlot>`
     (Nop / Pco / Lazy).
   - `LazyContentProvider` (nh-page, T12) is resettable, and its doc no longer says reset is
     unnecessary.
   - The page-level `pco` is not touched when a non-rendering site shifts, as in Go.
7. **`.Sitemap` (P1-7).** nh-config implements `Object` for `config.SitemapConfig` (a struct
   value; fields ChangeFreq, Priority float64, Filename, Disable), and T23 wraps it in the page
   table. Both are in §5.2.
8. **Nil mapping (P1-8): partly different from the review.** The review suggested mapping a nil
   non-empty interface result (`.Parent` of home) straight to `Invalid`. The design instead keeps
   it as `TypedNil("page.Page")`, whose `typed_nil_kind` is `Interface` (go-value already models
   this). The contract (C7) turns it into `Invalid` when it is passed to a func or method, which is
   what Go's conversion to `any` does. This reproduces Go in the jsonLd.html:389 case: `Set`
   receives an untyped nil, and the `if` guard is false. It also reproduces Go in two cases a
   direct `Invalid` would get wrong: text/template prints `<nil>` (not `<no value>`), and
   `$p := .Parent` followed by `$p.Title` is an error. The full table is in §4.7, and the wrong
   `%!s(<nil>)` attribution is fixed there.
9. **Hook context (P1-9).** The doc and the skeleton are fixed: the caller's context is passed
   unchanged, and `is_in_goldmark` is set only for `{{% %}}` shortcodes.
10. **Collation (P1-10).** The collators are built per language tag, T03 tests both the `en` and
    `th` tags, and the "current site at first computation" rule is kept (§10.10).
11. **Image decoding (P1-11).** §4.5 and §6.6 give the rule, `ImageCache.encoded` holds the bytes,
    and T14 gets an API-level Resize → Filter test.
12. **Locks (P1-12).**
    - §4.8 states the rule.
    - `PageMap`, `HugoSites.cache_pages` and `CachedContent.scopes` are now dynacache
      `Partition`s.
    - The shortcode handler has no lock: it is written during capture, which is the mutable phase.
    - The review also exposed that the rendered-content caches live in the site `PageMap` in Go,
      keyed by source key + scope + format, not in per-scope `OnceLock`s. The skeleton has been
      corrected accordingly.
13. **Collision tolerance (P2-13).** It is removed everywhere (§7.1, §7.2, §11.2, §13.4, I02).
14. **Term titles (P2-14).** §7.2 is fixed: `.Title` is the title-cased LAST `m.term`, and `.Name`
    is the first value.
15. **E2E command (P2-15).** `NEOHUGO_ESBUILD_BINARY` is added. The binary is built from the
    go.mod-pinned module by `tools/esbuild/build.sh` (T16), which is durable and offline.
16. **Parent module files (P2-16).** Every parent module file now has an `Owner:` line that
    matches the plan. The ownership rule differs from the review in one point: a parent whose
    children all belong to one task goes to that task. So `cssjs.rs`, `js.rs`, `tocss.rs` and
    `tocss/{sass,scss}.rs` go to T16, whose modules they are. Only parents shared by several tasks
    go to the crate lead: `resource_transformers.rs` to T15, `nh-tplfuncs/src/internal.rs` to T18.
17. **Dev-dependencies (P2-17).**
    - Every nh-* crate has `serde_json` as a dev-dependency. There is no `nh-testutil` crate: it
      would need an owner and would couple every task's tests to one more shared crate. Each task
      writes its small fixture reader.
    - The Wave A dependencies uncommented are only those whose crate has LANDED (committed) and
      passes `cargo check`, which is 13 crates. The directories that merely exist are still being
      written by parallel Wave A agents, and depending on them would make nh-* builds break
      whenever a Wave A agent is mid-edit.
18. **`page.Clear()` (P2-18).** It now belongs to T12 (`pages_cache::clear`). `page::clear`
    delegates to it, and T24's `render_site` calls it first. The page cache is ported faithfully,
    not "recompute".
19. **`.Site.Data` oracle (P2-19).** It is part of T23's acceptance (go-oracle nh-hugolib/data).
    Data loading moved to `hugo_sites_data.rs` (T23).
20. **Wrapper method sets (P2-20).** Both rules are in §5.1, and in the `page.rs` and
    `tplapi/page_methods.rs` docs.
21. **Small items (P2-21).**
    - `shouldBuild` belongs to T21, and §10.20 is fixed.
    - The pid wording in §6.3 is fixed.
    - There is one PostProcess counter: `BuildState` as `nh_common::identity::Incrementer`,
      shared through `SpecCommon`. The duplicate `AtomicU64` in `SpecCommon` is gone.
    - `Slice` is in the resourceAdapter table and has a T14 fixture.
