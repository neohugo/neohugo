# neohugo to Rust: overall architecture and build flow (spec `architecture-core`)

> **Historical (old port).** Paths under `crates/` refer to the byte-parity port that T00 of
> [`REWRITE_PLAN.md`](../REWRITE_PLAN.md) deleted; it is recoverable with
> `git show go-parity-final:crates/<path>` (commit `be02933a`, local tag). Byte-parity sections are obsolete.

Author: agent `architecture-core`. Scope: the end-to-end `neohugo --minify` build as used by the
seeksnack site, measured against the Go source at `/Users/blackb1rd/git/github/org/neohugo`
(module `github.com/neohugo/neohugo`, HEAD d5930ba1f, version string `v0.149.0-DEV`, built with
go1.27.1). The spec proposes the Rust workspace layout. Other agents' specs cover the subsystems in
depth (content-model, output-publishing, resources-pipeline, images, markdown, minify,
template-engine, templates-inventory, i18n-lang-misc). This document is the glue: the build
lifecycle, what "correct" means for the golden output, which Go code is exercised, the cross-cutting
abstractions, and the crate boundaries.

Paths used below:

- `$REPO` = `/Users/blackb1rd/git/github/org/neohugo`
- `$SP` = `/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad`
- `$W` = `$SP/work/architecture-core`
- `$DATA` = `$SP/specs/architecture-core-data` (machine-readable artifacts produced for this spec, listed in §1)

---

## 0. Executive summary (read this first)

1. **The golden output is a cold-cache build.** If `resources/_gen` already holds images from an
   earlier build, the Go binary produces about 4,690 different files. Every watermarked content
   image gets a different `_hu_<hash>` name, so every HTML page that references one changes too.
   Root cause (verified): `genericResource.Key()` (`$REPO/resources/resource.go:447-465`) appends
   `"_"+decimal(xxhash64(original source bytes))` only when `sourceFilenameIsHash == false`. The
   file-cache *read* path sets it to true (`resources/image_cache.go:59-76`, line 67 `img.setSourceFilenameIsHash(true)`; `create` closure at line 78),
   and the *create* path does not. The overlay filter hashes `src.Key()`
   (`resources/images/filters.go:49-54` → `resources/image.go:265`, `confMain.Key = hashing.HashString(gfilters)`),
   so the output file names depend on cache state. **Rust rule: always behave like the create
   (cold) path.** Details in §2.1.
2. **26 output files are nondeterministic even in Go.** 13 taxonomy terms map to the same URL
   (for example `Lays`/`Lay's` → `/brands/lays/`). With parallel rendering the last writer wins.
   `--printPathWarnings` lists them all (§2.2). The acceptance test must accept either variant for
   those `index.html`/`index.xml` pairs. A single-worker Go build (`HUGO_NUMWORKERMULTIPLIER=1`)
   reproduces every other file byte-for-byte, so **sequential rendering in content-tree walk order
   is a valid reference model** for Rust.
3. **`resources.GetRemote` (YouTube API, 87 pages) must be served from Hugo's file cache.** The
   cache lives at `~/Library/Caches/hugo_cache/<basename(workingDir)>/filecache/getresource/<key>`.
   `key = hashing.HashString(uri, map[string]any(nil))`, a decimal uint64 (verified against the
   cache: `iIsZs0m-BVU → 5844198154546968338`). Responses are raw `httputil.DumpResponse` dumps.
   View counts in the output (`interactionCount`) change over time, so parity requires reading
   this cache. Never run the acceptance build with `--ignoreCache`: it sets `MaxAge=0` on every
   file cache (`config/allconfig/alldecoders.go:86-99`). That bypasses the getresource cache, so
   GetRemote refetches live data from the network.
4. **The output depends on the current date.** `now.Format "2006"` appears in the footer and the
   RSS copyright (`Copyright 2019 -2026`). Rust must implement `--clock`. Regenerate the golden
   with a fixed `--clock` before 2027.
5. **Build shape** (all verified in code): the static copy runs in parallel with the site build
   (`commands/hugobuilder.go:518-574`). Then the lifecycle is process (parallel file collection)
   → assemble (per-site) → render (sites × output formats in sequence, pages in parallel) → write
   `hugo_stats.json` **into the site's working dir** → postProcess. postProcess replaces
   `__h_pp_l1_<id>_Content__e=` placeholders in about 3,400 HTML files with the
   postcss/purgecss-processed CSS. purgecss reads that same `hugo_stats.json`, which is collected
   from the *minified* HTML stream.
6. **Measured scope.** I ran a coverage-instrumented Go binary on the real build. It executes 2,239
   of 4,908 neohugo functions (41.6k of 74.6k function-body lines, 113k total non-test lines), plus
   about 95k executed function lines in dependencies. About 52k of those are locale data tables and
   about 20k are Go stdlib (image/jpeg, png, flate, time, fmt). Packages that are **not needed**:
   deploy, livereload, watcher, server, create/new, releaser, codegen, docshelper, metrics,
   identity (1%), pagesfromdata, markup/{asciidoc,pandoc,rst,org,blackfriday}, dartsass, babel,
   tailwind, warpc, the go-modules/git/npm parts of `modules`, chroma highlighting (no code blocks
   in the content), openapi, data (getJSON), diagrams.
7. **Parity-critical ports.** Byte identity is impossible without line-by-line ports of these:
   Go text/template + html/template (the in-repo fork); goldmark v1.7.12 and its extensions;
   tdewolff minify v2.23.8 and parse v2.8.1; Go image/jpeg + image/png + compress/flate;
   disintegration/gift; gohugoio/hashstructure; Go fmt/strconv/time-format/net/url/encoding/json
   (encode) subsets; x/text/collate (CLDR 23 tables, used by the template `sort` function);
   yaml.v2 resolution rules; spf13/cast. libwebp and libsass stay C/C++ via `-sys` crates built
   from the exact vendored sources. esbuild stays the Go-built esbuild v0.25.6 binary, driven over
   its service protocol.

---

## 1. Method and evidence

- **Coverage run.** `go build -cover -coverpkg=<neohugo/...,goldmark,minify,parse,gift,image/...,compress/flate,...>`
  produced `$W/neohugo-cover`, which is kept for reuse by any agent. I ran it on a copy of the site
  at `$W/sites/seeksnack` with an empty `resources/`, using `GOCOVERDIR=$W/cov1 neohugo-cover --minify -d $W/out-fresh`.
  The output matched the golden except for the collision files. Artifacts:
  - `$DATA/neohugo-pkg-table.tsv`: per package directory, non-test lines, executed function lines, statements, covered statements, %.
  - `$DATA/neohugo-pkg-exec.tsv`, `$DATA/neohugo-file-exec.tsv`: per package and per file function counts and executed function lines.
  - `$DATA/neohugo-executed-funcs.txt`: every executed neohugo function (`file:start-end name pct`). Use it as the porting checklist.
  - `$DATA/deps-pkg-exec.tsv`, `$DATA/dep-executed-funcs.txt`: the same for goldmark, minify, parse, gift, stdlib image/flate/fmt/time/url/json, yaml, toml, cast, i18n, hashstructure and others.
  - `$DATA/cov1-func.txt`: full `go tool cover -func` output. Raw textfmt profile: `$W/cov1.txt`.
- **Config oracle.** `$DATA/config-en.json`, `config-th.json` and `config-en-printzero.json` are
  the resolved config per language from `neohugo-go config --format json [--lang th]`.
  `$DATA/config-mounts.json` is the effective mounts.
- **Experiments.** Each was run with the golden Go binary on `$W/sites/seeksnack`:
  - cold vs warm `resources/_gen`
  - `HUGO_NUMWORKERMULTIPLIER=1`
  - `--printPathWarnings`
  - a minimal watermark reproduction in `$W/wmtest`
  - GetRemote cache-key computation in `$W/hashexp` (a Go program using a `replace` directive to the repo)
- **Retained reference outputs:**
  - `$W/out-fresh`: cold cache, parallel. Equals golden except 8 collision files.
  - `$W/out-w1cold`: cold cache, `HUGO_NUMWORKERMULTIPLIER=1`. This is the variant a sequential
    Rust port should produce for the collision files.
  - `$W/out-warm`: warm cache. Demonstrates the §2.1 quirk, with 4,688 diffs.
  - `$W/sites/seeksnack`: a site copy with a cold cache. Its basename is `seeksnack`, so it shares
    the GetRemote cache.

---

## 2. What "correct" means for the golden output (applies to every agent)

### 2.1 Cold resource cache (critical)

Measurements, all with `neohugo-go --minify -d <out>` in `$W/sites/seeksnack`:

| resources/_gen state | diff vs `golden/run1` (files) |
|---|---|
| deleted (cold) | 8, all term-collision files |
| populated by the previous build (warm) | 4,688 |
| warm, but `--ignoreCache` | same as cold. Tested on the minimal site; the full single-worker run also matched golden. Caveat: this refetches GetRemote over the network |

Minimal reproduction (`$W/wmtest`, same render-image hook as seeksnack):

```
cold:  WM.Key=/images/watermark_hu_24e7af27b4ce3fa8.png_6519743917224815147   F=/p1/a_hu_9439791329542c49.jpg
warm:  WM.Key=/images/watermark_hu_24e7af27b4ce3fa8.png                       F=/p1/a_hu_c99c50ff39a62.jpg
```

`6519743917224815147` is `hashing.XXHashFromReader(watermark.png)`, verified. Here is the
mechanism, with code references:

- `resources/resource_spec.go:195` sets `includeHashInKey: isImage` for every image resource.
- `resources/resource.go:447-465` `Key()` = RelPermalink (minus base path), plus
  `fmt.Sprintf("_%d", l.hash())` if `includeHashInKey && !sourceFilenameIsHash`.
- `resources/image_cache.go:36-113` `getOrCreate`:
  - The `read` closure (cache hit) calls `img.setSourceFilenameIsHash(true)`.
  - The `create` closure does not.
  - `parent.relTargetPathFromConfig(...)` (image.go:459) calls `parent.hash()` first. Clones share
    the `*resourceHash` pointer (`genericResource.clone()` copies the struct by value,
    resource.go:638), so a processed image's `hash()` returns **the original source's hash**.
- `resources/image.go:475` names processed images `p1 + "_hu_" + HashStringHex(incomingID, i.hash(), conf.Key, imagingConfigSourceHash) + ext`.
  `incomingID` is the previous `_hu_` id when an image is re-processed.

**Rust rule:** a processed image created in this build has `Key() = relPermalink + "_" + xxh64(original source bytes)`.
Do not read `resources/_gen` for images or assets at all in phase 1. The golden never used it.
Writing the cache is optional. Warm-cache emulation is a later nicety and must reproduce this quirk.

### 2.2 Nondeterministic files (term URL collisions)

`--printPathWarnings` reports these duplicate target paths, 13 terms × {`index.html`, `index.xml`},
plus the identical `page/1/index.html` alias:

```
/brands/lays/  /tags/lays/
/ingredients/{disodium-5-guanylate,disodium-5-inosinate,disodium-5-ribonucleotide,disodium-5-ribonucleotides,ins-322i,ins-500ii}/
/th/ingredients/{ins-322i,ins-500ii,ไดโซเดียม-5-กัวไนเลต,ไดโซเดียม-5-ไรโบนิวคลีโอไทด์,ไดโซเดียม-5-ไอโนซิเนต}/
```

- Observed variants: run1 vs run2 differ on 8 of them. The fresh cold build differs from run1 on
  8. The single-worker cold build differs from run1 on 8 and from run2 on 2.
- **Acceptance rule:** exclude these 26 files from the byte comparison, or accept any of the
  observed variants. Better still, compute both candidate renderings.
- **Rust ordering model:** render pages in doctree (sorted key) order, last write wins. That
  equals Go with `HUGO_NUMWORKERMULTIPLIER=1`, which matched golden on every non-colliding file,
  verified with a cold build: only 8 collision files differed.

### 2.3 GetRemote file cache (YouTube data)

- Template: `layouts/partials/marketing/jsonLd.html:200-204`, `resources.GetRemote $ytUrl | transform.Unmarshal`.
- Client: `resources/resource_factories/create/create.go:64-120`.
  - The HTTP client uses `httpcache.Transport` with `Cache: fileCache.AsHTTPCache()`.
  - `AlwaysUseCachedResponse = !httpCacheConfig.For(url)`. With the default config
    (`cache/httpcache/httpcache.go:27-40`, `Cache.For.Excludes ["**"]`) that is always true, so a
    cached response is never revalidated.
  - `MarkCachedResponses`, `EnableETagPair`.
- Key: `remote.go:322-333` `remoteResourceKeys`. With no options map,
  `optionsKey = userKey = hashing.HashString(uri, optionsm)` where `optionsm` is a **nil**
  `map[string]any` (the clone of nil).
- Location: `filecache_config.go:33,45-66,236-247`.
  `getresource.Dir = ":cacheDir/:project"`, where `:project = filepath.Base(workingDir)`, and
  files sit under `filecache/getresource/`. cacheDir comes from `helpers/path.go:328-365`
  `GetCacheDir`: `cacheDir` config, then the `HUGO_CACHEDIR` env var, then
  `os.UserCacheDir()/hugo_cache`, which on macOS is `~/Library/Caches/hugo_cache`.
  `MaxAge = -1` (never expires).
- Format: the file is the full HTTP response dump, `HTTP/2.0 200 OK\r\n` + headers + `\r\n` + body,
  not chunked. There are 51 entries for 87 pages.
- **Consequence:** the site directory used for the acceptance build must have basename `seeksnack`.
  The Rust port must implement: the hashstructure key, cache lookup, the Go
  `http.ReadResponse`-compatible parse, and the media-type detection path (`remote.go:222-275`,
  mime from `Content-Type: application/json; charset=UTF-8` → `application/json`). Then
  `transform.Unmarshal` → JSON decoded to Go types (numbers as float64).

### 2.4 Clock and environment

- `now` (`tpl/time`, backed by `htime.Now()`) is used in `layouts/_default/rss.xml` and
  `layouts/partials/footer.html` (year only). `shouldBuild` (`hugolib/site.go:1556-1578`) also
  compares publish/expiry dates against `htime.Now()`.
  - The `--clock` flag sets `htime.Clock = clocks.Start(t)` (`commands/commandeer.go:203-206, 309-312`).
    Rust must support it.
  - Language time zone: `time.LoadLocation("")` = UTC (`langs/language.go:116`). Machine TZ only
    affects `now`.
- Environment:
  - `HUGO_ENVIRONMENT`/`HUGO_ENV` choose the environment. The default for a build is `production`
    (`commands/hugobuilder.go:1033-1047`).
  - `HUGO_*` variables override config keys (`config/allconfig/load.go:196-270`,
    `applyOsEnvOverrides`). None were set for the golden build.
  - `HUGO_NUMWORKERMULTIPLIER` sets the worker count (`config/env.go:33-40`, default `runtime.NumCPU()`).
  - postcss is found at `<workingDir>/node_modules/.bin/postcss` (`common/hexec/exec.go:177-230`).
    Its environment comes from `common/neohugo/neohugo.go:201-230` `GetExecEnviron`:
    `NODE_PATH`, `PWD`, `HUGO_ENVIRONMENT`, `HUGO_ENV`, `HUGO_PUBLISHDIR`, plus
    `HUGO_FILE_<NAME>` for each `assets/_jsconfig` mount.
- The build writes into the site directory: `.hugo_build.lock`, `hugo_stats.json`
  (`hugolib/hugo_sites_build.go:719-775`), `resources/_gen/**` (Go only), and possibly
  `assets/jsconfig.json` (`hugo_sites_build.go:604-641`). For seeksnack the last one is skipped
  because `JSConfigBuilder.Build` returns nil.

### 2.5 Recommended acceptance procedure

```
cp -R <pristine site> <tmp>/seeksnack     # basename must be "seeksnack" (GetRemote cache)
rm -rf <tmp>/seeksnack/resources          # cold cache == golden
cd <tmp>/seeksnack && <bin> --minify [--clock 2026-09-27T10:25:00Z] -d <out>
diff -r <golden> <out>  excluding the 26 collision files (or accepting any known variant)
```

Also regenerate the golden with the same `--clock` value so it stays valid in 2027 and later.

---

## 3. End-to-end build flow (Go), with citations

### 3.1 CLI bootstrap

1. `main.go:23-28` `main()` → `commands.Execute(os.Args[1:])`.
2. `commands/commandeer.go:60-88` `Execute`:
   - `maxprocs.Set()`, then `newExec()` (`commands/commands.go:23-43`). This builds the
     simplecobra tree: root, build, version, env, server, deploy, config, new, convert, import,
     list, mod, gen, release.
   - `mapLegacyArgs` (`commandeer.go:673-679`), then `x.Execute`.
3. `rootCommand.PreRun` (`commandeer.go:424-466`): stdout/stderr, logger (level warn by default,
   `createLogger` 468-499), `loggers.SetGlobalLogger`. It also creates lazycaches for configs and
   HugoSites, which exist only for server mode.
4. `rootCommand.Run` (`commandeer.go:367-422`): `newHugoBuilder`, `b.loadConfig(cd,false)`,
   `b.build()`. Watch mode is not needed.
5. `hugoBuilder.loadConfig` (`commands/hugobuilder.go:1029-1078`):
   - `cfg.Set("renderToMemory")`
   - resolve the environment (flag, then `HUGO_ENVIRONMENT`, then `HUGO_ENV`, then `"production"`)
   - `cfg.Set("internal", {running:false, watch:false, verbose, fastRenderMode:false})`
   - `ConfigFromProvider(key, flagsToCfg(cd, cfg))`
   - fail if no config file was found.
6. `flagsToCfgWithAdditionalConfigBase` (`commands/helpers.go:73-114`) copies every *changed* flag
   into config. Renames: `minify→minifyOutput`, `destination→publishDir`, `editor→newContentEditor`.
   Keys `quiet, verbose, watch, liveReloadPort, renderToMemory, clock` go under `internal.*`.
7. `rootCommand.ConfigFromProvider` (`commandeer.go:219-331`):
   - `workingDir` = `--source` or cwd
   - `allconfig.LoadConfig(...)` (§3.2)
   - `cfg.Set("publishDir"/"publishDirStatic"/"publishDirDynamic", base.PublishDir)`
   - `hugofs.NewFromSourceAndDestination(OsFs, OsFs, cfg)` (`hugofs/fs.go`)
8. `hugoBuilder.build` (`hugobuilder.go:385-413`) → `fullBuild` → print the stats table. The table
   goes to stdout and is not part of parity.
9. `hugoBuilder.fullBuild` (`hugobuilder.go:518-589`) runs `copyStatic` and `buildSites` **in
   parallel** in an errgroup, unless `cleanDestinationDir` is set. `buildSites` →
   `c.hugo()` → `HugFromConfig` → `hugolib.NewHugoSites(depsCfg)` (§3.4) → `h.Build(BuildCfg{})` (§3.6).

### 3.2 Config loading (`config/allconfig/load.go:43-96` `LoadConfig`)

1. `loadConfigMain` (`load.go:294-414`):
   1. `normalizeCfg` on the flags. `minifyOutput=true` becomes `minify.minifyOutput=true` (`load.go:172-180`).
   2. Apply flag overrides, then load `hugo.toml`. The search order is `config.DefaultConfigNames`,
      and the first match wins.
   3. `config/` dir via `config.LoadConfigFromDir` (environment subdirs). seeksnack has no config dir.
   4. `applyDefaultConfig` (`themesDir`, `configDir`).
   5. OS env overrides (applied twice), `GetCacheDir`, `SetDefaultMergeStrategy`.
   6. Flags again, then env again, then `applyConfigAliases` (`indexes→taxonomies`, …).
2. `fromLoadConfigResult` (`allconfig.go:1025-1153`):
   1. `newDefaultConfig()` (`allconfig.go:996-1022`). Defaults:
      - taxonomies tag/category
      - sitemap priority -1, filename `sitemap.xml`
      - environment production, titleCaseStyle AP, pluralize/capitalize list titles
      - staticDir `["static"]`, summaryLength 70, timeout 60s
      - dirs: archetypes/content/resources/public/themes/assets/layouts/i18n/data
   2. `decodeConfigFromParams` runs every decoder in `alldecoders.go` (build, caches, httpcache,
      imaging, markup, mediaTypes, outputFormats, outputs, minify, related, security, …).
      `CompileConfig` (`allconfig.go:254-505`) derives `C.*`: BaseURL, KindOutputFormats,
      DisabledKinds, IsUglyURLSection, SegmentFilter, …
   3. Per language: a merged clone when language keys differ from the root. There is a special
      default for multilingual single-host sites: `RenderHooks.Image/Link.UseEmbedded = fallback`
      (`allconfig.go:1119-1128`). seeksnack is multilingual and has no `render-link` hook, so the
      **embedded link render hook** is active. The templates agent should cover this.
3. `loadModules` (`load.go:416-477`) runs `modules.NewClient(...).Collect()` (§3.3). If modules add
   config files, the config is re-read. seeksnack has none.
4. `Configs.Init` (`allconfig.go:839-977`):
   - sorts languages by weight, then lang; filters disabled ones (`disableLanguages`), leaving `en`, `th`
   - `LanguagesDefaultFirst`
   - builds `ContentPathParser` (`common/paths/pathparser.go`) with the language index, content extensions and output formats
   - builds `configLangs`
   - `modules.ApplyProjectConfigDefaults` (`modules/config.go:59-...`) adds default mounts for
     components with no configured mount. seeksnack configures all of them.

### 3.3 Modules and mounts (only the project module is needed)

- `modules/collect.go:51-72` `Collect` → `collect()` (`collect.go:508-530`). `initModules` runs
  `go list -m` only when a `go.mod` with imports exists (`client.go:432`), which seeksnack lacks.
  `createProjectModule` names the module `"project"`. `addAndRecurse` (`collect.go:334-360`) then
  calls `applyMounts`, which does:
  - `normalizeMounts` (`collect.go:639-702`): Clean, existence check (a missing source is
    skipped silently), and a target-component check against `files.ComponentFolders`.
  - `mountCommonJSConfig` (`collect.go:600-637`): any `package.hugo.json`, `package.json` or
    `(babel|postcss|tailwind).config.js` at the project root is auto-mounted to `assets/_jsconfig/<name>`.
- Effective mounts for seeksnack (`$DATA/config-mounts.json`), in order: content, static, layouts,
  data, assets, i18n, archetypes, `node_modules → assets/vendor`,
  `package.json → assets/_jsconfig/package.json`, `postcss.config.js → assets/_jsconfig/postcss.config.js`.
- `filterUnwantedMounts` (`collect.go:145-157`) de-duplicates by `mount.key()`.
- Not needed: Go modules, vendoring, themes dir, `hugo mod npm pack` (`modules/npm`), workspace files.

### 3.4 HugoSites and Deps creation (`hugolib/site.go:143-335` `NewHugoSites`)

1. Logger. `dynacache.New` (memory cache). `firstSiteDeps := &deps.Deps{Fs, Log, Conf, BuildState, Counters, MemCache, TranslationProvider: i18n.NewTranslationProvider(), WasmDispatchers}`.
2. `deps.Deps.Init` (`deps/deps.go:133-263`):
   1. `hexec.New`, then wrap `Fs.PublishDir` in `hugofs.NewHasBytesReceiver`. For text-suffix
      files it records filenames containing `__h_pp_l1` (postpub) or the deferred-template prefix.
   2. `helpers.NewPathSpec` (URL/path helpers + BaseFs, §3.5).
   3. `helpers.NewContentSpec` (markup converters), `source.NewSourceSpec`.
   4. `filecache.NewCaches` and `resources.NewSpec` (resource factory, image cache, post-process registry).
3. `esbuild.NewBatcherClient` (js.Batch). It is not used by seeksnack.
4. `confm.Validate`. The page trees are shared by all sites (`pageTrees`, `content_map_page.go:118-259`):
   - `treePages`, `treeResources`: `doctree.NodeShiftTree` over `armon/go-radix`, with a
     `contentNodeShifter{numLanguages}` handling the language dimension
   - `treeTaxonomyEntries`, `treePagesFromTemplateAdapters`
5. One `Site` per non-disabled language:
   - `conf = LanguageConfigMap[lang]`
   - `pagemeta.NewFrontmatterHandler`
   - `langs.SetParams`
   - site 0 owns `firstSiteDeps`; the others get `firstSiteDeps.Clone(s, confp)`
   - `newPageMap(i, …)`, `newPageFinder`, `newSiteRefLinker`
   - **a publisher per site** (`publisher.NewDestinationPublisher`, `publisher/publisher.go:80-91`), each with its own `htmlElementsCollector`
   - `page.NewRelatedDocsHandler`
   - `prepareInits` (lazy prevNext, prevNextInSection, menus, taxonomies: `site.go:697-792`)
6. **Site order** (`site.go:309-325`): the default content language first, then by weight, then
   by lang, giving `[en, th]`.
7. `newHugoSites` (`site.go:337-455`) creates the `HugoSites` struct, whose worker count is
   `config.GetNumWorkerMultiplier()`.
   - The template store is built once: `tplimpl.NewStore(StoreOptions{Fs: s.Layouts.Fs, …}, SiteOptions{Site: s, TemplateFuncs: tplimplinit.CreateFuncMap(s.Deps)})`
     (`tpl/tplimpl/templatestore.go:112`). It parses all layouts plus embedded templates
     (`insertEmbedded` 1024, `insertTemplates` 1240, `init` 1826).
   - Other sites use `prototype.TemplateStore.WithSiteOpts(...)`.
   - `s.Compile(prototype)` loads i18n (`deps.go:265-280` → `langs/i18n/translationProvider.go`).
   - Lazy `h.init.data` (`loadData`, `hugo_sites.go:498-599`) and `h.init.gitInfo`.

### 3.5 Filesystems (`hugolib/filesystems/basefs.go`)

- `NewBase` (`basefs.go:465-513`), then `sourceFilesystemsBuilder.Build` (`basefs.go:541-609`), then `createMainOverlayFs` (611-654), then `createOverlayFs` (664-803).
- Each module's mounts are split three ways: content mounts → `rmfsContent`, static →
  `rmfsStatic`, everything else → `rmfs`. Each part becomes a `hugofs.RootMappingFs`
  (`hugofs/rootmapping_fs.go:44`). Every `RootMapping` has `From=target`, `To=abs source`, and a
  `FileMeta` with the following fields:
  - `Weight = (10+moduleOrdinal)*(len(mounts)-i)`. Earlier mounts win.
  - `Lang`: the mount's lang, or `defaultContentLanguage` for content mounts.
  - `InclusionFilter` (glob include/exclude). Unused by seeksnack.
- Overlays (`bep/overlayfs`):
  - `overlayMounts`: layouts/assets/archetypes/data/i18n, first wins.
  - `overlayMountsContent` and `overlayMountsStatic`, both with `hugofs.LanguageDirsMerger`.
  - `overlayFull`, `overlayResources`.
- Component views: `hugofs.NewComponentFs` (`hugofs/component_fs.go`).
  - **ReadDir order** (`component_fs.go:64-146`):
    1. directories first
    2. lower module ordinal first (i18n uses the reverse)
    3. for content, bundles first (`index.*`/`_index.*`)
    4. **extension descending** (`.md` before `.html`)
    5. path base ascending, then weight descending, then name
  - `applyMeta` (`component_fs.go:148-...`):
    1. **NFC-normalizes names on darwin** (`norm.NFC`)
    2. parses `PathInfo` with the PathParser
    3. filename language (`index.th.md`) → `meta.Lang`, `Weight++`
    4. drops disabled languages (for example `index.fr.md`)
- `Data` and `I18n` use `AppendDirsMerger`, which keeps duplicates. `Static` is a single fs in
  non-multihost mode: `NewBasePathFs(overlayMountsStatic, "static")`.
- **Ignore rules for the content, data and layout walkers** (`source/sourceSpec.go:55-89`): skip
  base names starting with `.` or `#`, or ending with `~`. For seeksnack this skips the 66
  `.gitkeep` files in `content/`. The **static copy copies dotfiles** (`static/.DS_Store` is in
  the golden).
- `hugofs/walk.go:119-225` `Walkway.walk`: `IgnoreFile` filter → `HookPre` → recurse → `HookPost`.

### 3.6 `HugoSites.Build` (`hugolib/hugo_sites_build.go:60-226`)

1. `LockBuild` (the `.hugo_build.lock` file mutex). Start the error collector.
2. `prepare`:
   1. `initSites` → `h.reset`.
   2. **process** (`hugo_sites_build.go:255-272`) → `processFull` (1193) → `processFiles` (1243) →
      `newPagesCollector(...).Collect()` (`hugolib/pages_capture.go:85-219`). It walks the
      content fs sequentially and feeds `rungroup` workers (`NumWorkers = numWorkers`) that call
      `pageMap.AddFi` (`content_map.go:216-317`). That parses the file (front matter via
      `parser/pageparser`) and inserts into `treePages`/`treeResources` keyed by path.
      - Leaf bundles are handled by `handleBundleLeaf` (pages_capture.go:378-425). Everything
        else in the bundle becomes a resource.
      - Duplicate base+lang pairs are dropped at walk time, so the result keeps a consistent ordering.
      - Insert order does not matter: the tree is keyed.
   3. **assemble** (`hugo_sites_build.go:274-349`):
      - Step 1, per site in parallel on `workersSite` (`content_map_page.go:1856-1870`):
        `addMissingTaxonomies` (2103), `addMissingRootSections` (2007), `addStandalonePages`
        (1930, adds `/404`, `/_robots` for the first site only, `/_sitemap`, `/_sitemapindex`
        for multilingual), `applyAggregates` (1385: dates/lastmod roll-up, cascade).
      - Step 2, sequential over sites (1872-1884): `removeShouldNotBuild` (1895),
        `assembleTermsAndTranslations` (1635), `applyAggregatesToTaxonomiesAndTerms` (1546).
      - `initRenderFormats` per site (`site.go:800-837`): the union of the pages' configured
        formats and the per-kind formats, then `sort.Sort(output.Formats)`. HTML comes first
        (weight 10), then the others by name. `h.renderFormats` concatenates all sites' lists;
        page outputs are indexed by that global index (`page__meta.go:884-947`).
      - StepFinal: `assembleResources` (1735).
3. **render** (`hugo_sites_build.go:351-438`): for each site in order, for each `renderFormat`
   (global index `i`), every site calls `preparePagesForRender(s==s2, i)`
   (`hugo_sites.go:481-496`, which uses `pageState.shiftToOutputFormat`, `page.go:675-750`). Then
   `s.render(ctx)` runs (`site.go:1580-1613`):
   1. On the first format of the first build only: `renderAliases()` (`site_render.go:272-333`).
      This must run **before** pages so that real pages overwrite faulty aliases.
   2. `renderPages` (`site_render.go:71-122`): a `NodeShiftTreeWalker` over `treePages` in
      **sorted key order** feeds a buffered channel. `numWorkers` goroutines run `pageRenderer`
      (`site_render.go:125-192`):
      1. skip standalone pages unless `shouldRenderStandalonePage` (`site_render.go:55-68`)
      2. `renderResources()` publishes page-bundle resources (all 531 content images are copied verbatim)
      3. `resolveTemplate()`, then `renderAndWritePage` (`site.go:1440-1496`)
      4. if the page has a paginator, `renderPaginator` (`site_render.go:227-269`): write the
         `page/1` alias, then render pages `2..N` to `<path>/page/N/`
   3. If this is a standalone-rendering context, `renderMainLanguageRedirect`
      (`site_render.go:337-367`). For multilingual sites with the default language not in a
      subdir, it writes `/en/index.html` as an alias to `https://seeksnack.com/`.
4. `writeBuildStats` (`hugo_sites_build.go:719-775`):
   1. merge each site's `publisher.PublishStats().HTMLElements`, then `Sort()`, which uses
      `sort.Strings` on tags, classes and ids (`htmlElementsCollector.go:85-89`)
   2. `json.Encoder` with `SetEscapeHTML(false)` and `SetIndent("", "  ")`
   3. write `<workingDir>/hugo_stats.json`, skipping the write if the bytes are equal
   4. drain the dynacache entries that depend on it
5. `printPathWarningsOnce`; `renderDeferred` (`hugo_sites_build.go:440-469`, a no-op for seeksnack
   since there is no `templates.Defer`); `printUnusedTemplatesOnce`.
6. **postProcess** (`hugo_sites_build.go:600-717`):
   1. Optional `jsconfig.json`.
   2. For every filename recorded by the HasBytesReceiver, in parallel (`para`): read the file,
      scan for `__h_pp_l1`… `__e=` placeholders, and replace each with
      `PostPublishResource.GetFieldString` (`resources/postpub/postpub.go:80-117`). For `Content`
      that calls `delegate.Content()`, which **executes the lazy transformation chain now**:
      `toCSS | postCSS | minify | fingerprint`. So postcss/purgecss runs after `hugo_stats.json`
      is written. Rewrite the file if it changed.
7. Stop the error collector. The build fails if any `ERROR` was logged (`hugo_sites_build.go:212-223`).

### 3.7 Publishing a rendered page (`site.go:1440-1496`, `publisher/publisher.go:94-190`)

1. `renderForTemplate` executes the template into a pooled buffer. Empty output is not written.
2. Build the `publisher.Descriptor`:
   - **RSS**: `AbsURLPath = s.absURLPath(targetPath)`, always, which canonifies URLs.
   - **HTML**: `AbsURLPath` only when `relativeURLs || canonifyURLs`. seeksnack sets `canonifyurls = true`.
     - `AddHugoGeneratorTag = s.conf.DisableHugoGeneratorInject` (sic, `site.go:1482-1484`), so
       with the default `false` no generator tag is injected. Keep this inverted neohugo behaviour.
   - Aliases (`hugolib/alias.go:91-116`) use the page's HTML output format, render the alias
     template (embedded `alias.html` via `LookupPagesLayout` with `OutputFormat=alias`,
     `alias.go:50-85`), and get `AbsURLPath` when canonify is on.
3. `createTransformerChain` (`publisher.go:156-190`), applied in this order:
   1. `urlreplacers.NewAbsURLTransformer` for HTML or `NewAbsURLInXMLTransformer` for other formats (`transform/urlreplacers/absurlreplacer.go`)
   2. livereload: server only, not needed
   3. generator meta: off
   4. **minify** (`minifiers.Client.Transformer(mediaType)`, `minifiers/minifiers.go:41-55`) when `minify.minifyOutput`
4. Minifier registry (`minifiers.go:77-106`, `New`):
   - css, js (plus regex `^(application|text)/(x-)?(java|ecma)script$`), json (plus regex
     `^(application|text)/(x-|(ld|manifest)\+)?json$`), svg, xml, html
   - the HTML minifier is also registered for every `IsHTML` output format's media type
   - media types are selected by suffix (`BySuffix`). `application/rss+xml` has suffix `xml` and
     so gets the XML minifier. `text/plain` (robots.txt) has no minifier and is written verbatim.
5. `helpers.OpenFileForWriting(publishFs, targetPath)`, then `io.Copy` to
   `MultiWriter(file, htmlElementsCollectorWriter)` for HTML formats. **Build stats are collected
   from the transformed (minified) bytes.** Aliases count as HTML, so they contribute
   `html/head/title/link/meta` tags.

### 3.8 Static copy (`commands/hugobuilder.go:429-516`)

`doWithPublishDirs` → for each static fs (one, key `""`) → `copyStaticTo`, which runs
`spf13/fsync.Syncer` with `SrcFs` = the static overlay, `DestFs = fs.PublishDirStatic`,
`NoTimes`/`NoChmod` from config, and `Delete` if `cleanDestinationDir`. It copies all 23 files
verbatim, including `.DS_Store` and the `.bak` files. None of them collide with generated paths;
verified that all 23 are byte-identical in the golden. The only requirements are the same content
and a complete set; mtime and mode are not compared.

### 3.9 Output inventory (golden `run1`, 6,943 files)

| Kind | Count | Notes |
|---|---|---|
| HTML files | 3,427 | 1,481 are alias redirects containing `http-equiv=refresh` (`page/1/`, `404/page/1.html`, front-matter aliases, `/en/`); 1,946 are real pages; 1,702 sit under a `*/page/N` path |
| XML | 1,481 | RSS for home, sections, taxonomies and terms per language; `sitemap.xml` × 3 (index + en + th) |
| JSON | 2 | `index.json`, `th/index.json` (home JSON output) |
| Processed images `*_hu_*` | 1,461 | jpg + webp (+ png) |
| Page bundle originals | 531 | copied verbatim by `renderResources` |
| Assets published | favicon images, 3 fingerprinted JS files | `js/set-theme.<sha256>.js`, `js/website.<sha256>.js` ×2 |
| robots.txt | 1 | `User-agent: *`, no trailing newline |
| Static | 23 | verbatim |

### 3.10 Concurrency and order dependence

| Stage | Go concurrency | Output order-dependent? | Rust recommendation |
|---|---|---|---|
| static copy vs site build | errgroup, parallel | no (disjoint paths) | parallel or sequential |
| content collection | rungroup workers | no (keyed tree; dup filtering at walk time) | sequential walk, parallel parse OK |
| assemble step 1 | parallel per site | no | sequential |
| render: sites × formats | sequential | yes: aliases first, then pages; formats in sorted order | same loop order |
| render: pages | `numWorkers` goroutines fed in tree order | **yes** for duplicate target paths (last write wins) and for **first-wins caches** such as `resources.Concat` keyed by target path only (`bundler.go:85`, `ResourceCache.GetOrCreate(targetPath)`). seeksnack builds `"js/all.js"` from single.html and term.html with **different input orders**; in practice single.html (earlier in walk order) wins | phase 1: sequential in tree order; later: parallel with deterministic first-wins (e.g. resolve by walk index) |
| lazy page content / `.Summary` / related | on demand, mutex/once | no | `OnceCell` per page output |
| image processing | on demand, `imageProcSem` (NumCPU) | no (pure) | rayon OK |
| postProcess placeholder replacement | `para` workers per file | no | parallel OK |
| `hugo_stats.json` | merged + sorted | no | — |
| map iteration | Go randomized | never reaches output (templates sort map keys) | use ordered maps anyway |

---

## 4. Codebase measurement

### 4.1 Top-level packages (non-test lines / executed function lines in the seeksnack build)

Totals: **113,253 non-test lines**; 4,908 functions / 74,567 function-body lines; **2,239 functions /
41,636 lines executed**. Test lines are not counted. Full per-directory table: `$DATA/neohugo-pkg-table.tsv`.

| Top-level | Non-test lines | Executed fn lines | Verdict |
|---|---:|---:|---|
| tpl | 26,567 | ~12,900 | REQUIRED core: go_templates fork 10.7k (4.6k exec), tplimpl 3.5k (2.2k exec), namespaces. Not needed: openapi, data, diagrams, testenv |
| resources | 17,693 | ~5,600 | REQUIRED: resources core, images, page, pagemeta, postpub, create (Get/GetRemote/Concat), cssjs (postcss only), tocss/scss, js, integrity, minifier, templates. Not needed: babel, dartsass, tailwind, imagetesting, page_generate |
| hugolib | 17,642 | ~8,600 | REQUIRED (except pagesfromdata; segments minimal) |
| common | 8,529 | ~3,500 | mostly REQUIRED; tasks/terminal/constants not needed |
| markup | 5,807 | ~1,900 | REQUIRED: goldmark glue, tableofcontents, converter, markup_config, highlight config. Not needed: asciidocext/pandoc/rst/org/blackfriday (stub), passthrough, chroma highlighting (config only) |
| commands | 5,702 | ~2,160* | PARTIAL: root/build/version/config/env. *Executed lines include flag init for every subcommand |
| config | 4,591 | ~2,100 | REQUIRED (allconfig, security, privacy, services); testconfig not needed |
| hugofs | 3,543 | ~1,510 | REQUIRED (rootmapping, component, walk, decorators, hasbytes, glob filter) |
| internal | 3,293 | ~690 | PARTIAL: js/esbuild options+resolve+build (~1k); batch.go (1.4k) and warpc not needed |
| modules | 2,468 | ~755 | PARTIAL: project-module mount collection only; npm not needed |
| parser | 2,337 | ~960 | REQUIRED: pageparser, metadecoders (yaml/toml/json) |
| cache | 1,761 | ~710 | PARTIAL: filecache (getresource required), httpcache; dynacache becomes plain maps |
| helpers | 1,445 | ~550 | REQUIRED subset (path/url/content/general) |
| deploy | 1,089 | 0 | DROP |
| identity | 998 | 8 | DROP (keep a stub `GetIdentity()` if needed for the keyer hash path) |
| output | 867 | ~96 | REQUIRED (formats); `output/layouts` (336) unused |
| media | 847 | ~392 | REQUIRED |
| publisher | 746 | ~487 | REQUIRED (incl. htmlElementsCollector) |
| navigation | 645 | ~67 | REQUIRED (menus from config) |
| related | 632 | ~366 | REQUIRED (`.Site.RegularPages.Related`) |
| langs | 591 | ~325 | REQUIRED (Language, i18n, collators) |
| create | 584 | 0 | DROP (`new`) |
| transform | 561 | ~268 | REQUIRED: chain, urlreplacers. livereloadinject/metainject not needed |
| codegen | 542 | 0 | DROP |
| deps | 498 | 233 | REQUIRED (becomes a Rust context struct) |
| watcher | 475 | 0 | DROP |
| htesting | 353 | 8 | DROP |
| livereload | 336 | 0 | DROP |
| metrics | 292 | 0 | DROP (stub `--templateMetrics`) |
| lazy | 277 | 134 | replace with `OnceCell` |
| minifiers | 265 | 121 | REQUIRED |
| source | 247 | 86 | REQUIRED |
| releaser | 239 | 0 | DROP |
| scripts | 232 | 0 | DROP |
| compare | 180 | 97 | REQUIRED |
| docshelper | 47 | 3 | DROP |
| bufferpool | 38 | 7 | DROP (use `Vec<u8>`) |

### 4.2 Third-party and stdlib code that the build executes (must be ported or wrapped)

Source: `$DATA/deps-pkg-exec.tsv`. Columns: package, total lines, executed function lines.

| Package | Non-test lines | Exec fn lines | Treatment |
|---|---:|---:|---|
| yuin/goldmark v1.7.12 (ast, parser, renderer/html, text, util, extension) | 12,562 | ~5,450 | port line by line (`goldmark` crate) |
| tdewolff/minify v2.23.8 (html, css, js, json, svg, xml, core) | 12,249 | ~6,060 | port line by line |
| tdewolff/parse v2.8.1 (html, css, js, json, xml, strconv, buffer, core) | 12,517 | ~4,900 | port line by line |
| stdlib image/jpeg | 2,843 | 1,881 | port (decode + encode) |
| stdlib image/png | 1,793 | 1,381 | port (decode + encode) |
| stdlib compress/flate + zlib (+ adler32, crc32) | ~6,950 | ~2,400 | port flate **compressor**; checksums may use crates |
| stdlib image, image/color, image/draw | 3,915 | ~615 | port the subset (image types, YCbCr/NRGBA conversions, draw.Draw Src/Over, FloydSteinberg if paletted PNG) |
| disintegration/gift v1.2.1 | 3,294 | 836 | port line by line (resize, float32) |
| gohugoio/hashstructure v0.5.0 | 532 | 358 | port exactly |
| gopkg.in/yaml.v2 v2.4.0 | 9,639 | 3,079 | libyaml-port crate + port yaml.v2 resolve/decode (§7) |
| pelletier/go-toml/v2 v2.2.4 | 5,317 | ~2,800 | `toml` crate + a Go-type mapping layer (verify) |
| spf13/cast v1.9.2 | 1,577 | 541 | port semantics |
| mitchellh/mapstructure | 1,456 | 767 | re-architect (typed config decoding) |
| gohugoio/go-i18n/v2 (i18n + plural) | 1,706 | 1,130 | port |
| gohugoio/localescompressed | 52k data | 52k (data) | generate Rust tables (dates/numbers per locale); only en + th needed now |
| golang.org/x/text collate + colltab (CLDR 23 / Unicode 6.2 tables, 4.9 MB) | 75k (mostly data) | not instrumented | port + table generator (template `sort` uses `LtCollate`, `tpl/compare/compare.go:208-217, 255-380`) |
| gobuffalo/flect | 1,309 | 344 | port (humanize, pluralize, titleize) |
| jdkato/prose/transform | 216 | ~50 | port (AP title case) |
| armon/go-radix | 561 | 294 | replace with `BTreeMap` (same byte-lexicographic in-order walk) |
| bep/overlayfs, spf13/afero, spf13/fsync | — | ~560 | re-implement (VFS) |
| gobwas/glob | ~1.9k | ~970 | minimal port or `globset` (low risk; see §7) |
| evanw/esbuild pkg/api | 3,525 | 1,616 | **do not port**: drive the esbuild 0.25.6 binary (§6.5) |
| bep/golibsass (Go wrapper) | ~270 | 229 | port the wrapper (options, importer callback); C++ via `libsass-sys` |
| bep/gowebp (Go wrapper) | ~106 | 100 | port the wrapper; C via `libwebp-sys` |
| bep/imagemeta | 3,438 | 83 | minimal (EXIF is effectively disabled: `excludeFields='.*'`) |
| alecthomas/chroma v2 | large | 507 (init/config only) | stub (no code blocks rendered) |
| stdlib fmt / strconv / time / net/url / encoding/json / text/template (used by go-i18n) | ~23k | ~5,900 | port subsets into `gostd` |

---

## 5. Cross-cutting core abstractions for Rust

### 5.1 Go dynamic values → `Value` + `Object`

Templates, config, front matter, data files, i18n, `cast`, `compare`, `collections` and
`hashstructure` all rely on Go reflection. Define one shared model in crate `nh-value`:

```rust
pub enum Value {
    Nil,                                  // untyped nil
    Bool(bool),
    Int(i64, IntKind),                    // IntKind: Int, Int8..Int64 (keep Go kind for %T, hashing, compare)
    Uint(u64, UintKind),
    Float(f64, FloatKind),                // Float32 kept for gift/filter args
    String(GoStr),                        // Go strings are BYTES: GoStr = Arc<[u8]> (+ fast &str view)
    Typed(TypedStr),                      // template.HTML/HTMLAttr/JS/JSStr/CSS/URL/Srcset (html/template content types)
    Slice(Arc<Slice>),                    // elem type tag: []any, []string, []int, page.Pages, ...
    Map(Arc<Map>),                        // key kind + ordered storage (BTreeMap for string keys == fmtsort order)
    Params(Arc<Params>),                  // maps.Params: lower-cased keys, case-insensitive lookup
    Time(GoTime),                         // time.Time with Location
    Object(Arc<dyn Object>),              // Page, Site, Resource, Pages, Taxonomy, Pager, OutputFormat, Menu, Scratch, ...
    TypedNil(TypeTag),                    // typed nil pointer/interface (matters for `with`, `if`, eq)
}
pub trait Object: Send + Sync {
    fn type_name(&self) -> &'static str;              // Go type name, for %T, hashstructure, errors
    fn method(&self, ctx: &ExecCtx, name: &str, args: &[Value]) -> Option<Result<Value>>; // Go method set (exact-case names)
    fn field(&self, name: &str) -> Option<Value>;     // exported struct fields
    fn is_truthy(&self) -> bool { true }
    fn as_keyer(&self) -> Option<String> { None }     // common/hashing toHashable: Key()
}
```

Rules to preserve:

- **Truthiness.** Follow `hreflect.IsTruthful`, which uses the `IsZero` method when present. Typed
  nil vs untyped nil matter for `with`, `if` and `default`.
- **Numbers.** YAML ints are Go `int` and TOML ints are `int64`; JSON numbers become `float64`.
  Template arithmetic (`tpl/math`) and `eq` (`tpl/compare`) normalize kinds.
  `printf "%v"` formats `float64` via `strconv 'g' -1`: `1e+06`, `5`, `0.5`.
- **Strings are bytes.** `len`, `index`, `slice` and `substr` (runes in Hugo) must not assume
  UTF-8 boundaries. Keep output buffers `Vec<u8>`.
- **Map iteration.** Go templates `range` over maps with `fmtsort` order
  (`tpl/internal/go_templates/fmtsort/sort.go`: strings by bytes, ints numeric, …). JSON encodes
  map keys sorted. Store string-keyed maps as `BTreeMap<Vec<u8>, Value>`.
- **Case.** Config and Params keys are lower-cased (`common/maps/params.go`). Template access
  `.Site.Params.HomeTitle` works through Hugo's fork of the text/template exec map lookup (the
  template-engine spec covers it). Keep a `Params` variant.

### 5.2 Go stdlib behaviours that leak into bytes (crate `gostd`)

- **fmt**: `print.go` and `format.go` semantics for `%v %s %d %q %x %X %f %e %g %t %T %c %U %%`,
  width/precision/flags, and `Sprint`'s space-insertion rule between operands that are not
  strings. Porting is required.
- **strconv**:
  - `FormatFloat` for `'g','e','f'` with precision -1 or fixed. Shortest digits can come from
    `ryu`, but the layout must follow Go.
  - `Quote`/`Unquote`, `ParseFloat`/`ParseInt` syntax (underscores, `0x`, `inf`).
- **time**:
  - `Format`/`Parse` with Go reference layouts (`time/format.go`), `Duration.String`, `Month`/`Weekday` names.
  - Zones: UTC and Local only; read the tzdata file for Local.
  - `MarshalBinary`, used by hashstructure for `time.Time`.
- **net/url**: `Parse`, `String`, `PathEscape`, `QueryEscape` and the `shouldEscape` tables.
  Needed for `absURL`, `relLangURL`, permalinks, `urlize` and the GetRemote key (the raw string is used).
- **encoding/json**: `Marshal` (`jsonify`, `index.json`, `hugo_stats.json`) with `<>&` → `<`…
  unless `SetEscapeHTML(false)`, ` `/` ` escaping, float format (`'f'` for exponent in
  [-6, 21), else `'e'` with `e-07` cleanup), `MarshalIndent`, sorted map keys, trailing `\n` from
  `Encoder.Encode`. `Unmarshal` to `any` gives float64, `map[string]any` and `[]any`.
- **html**: `html.EscapeString` escapes only `<>&'"` to `&lt; &gt; &amp; &#39; &#34;`.
  `html.UnescapeString` needs the full entity table (goldmark also needs it).
- **unicode**: Go 1.27.1 uses **Unicode 17.0.0** (`$GOROOT/src/unicode/tables.go:6`). Generate
  the IsLetter/IsDigit/IsSpace/IsPunct/IsUpper/ToLower/ToUpper/ToTitle/SimpleFold tables from Go's
  `tables.go` and do not rely on Rust `char` methods. `strings.ToLower`/`Title`/`EqualFold`/`Fields`
  semantics apply.
- **sort**: page sorting uses `sort.Stable`/`SliceStable` (`resources/page/pages_sort.go:76,186`,
  `weighted.go:128`, `taxonomy.go:161`, `tpl/collections/sort.go:187-189`, `navigation/menu.go:193`,
  `related/inverted_index.go:532`), and Rust's stable sort is equivalent. The unstable `sort.Sort`
  and `sort.Slice` calls (`site.go:312,835`, `media/config.go:100,257`, `component_fs.go:106`,
  `pagegroup.go:77-87`) sort by unique keys, so they are also safe. Still provide a port of Go's
  pdqsort (`sort/zsortfunc.go`) for any comparator with ties.
- **path**, **path/filepath**: `Clean`, `Join`, `Ext`, `Base`, `Dir`. These are simple, but port them exactly.
- **regexp**: RE2 syntax. The Rust `regex` crate is compatible for the patterns Hugo uses internally.
- **mime/http.DetectContentType**: used for GetRemote media type when the Content-Type is not
  configured. seeksnack gets `application/json` from the header.

### 5.3 VFS (crate `nh-vfs`)

Model: `Mount { source_abs, target: "content|static|layouts|data|assets|i18n|archetypes[/sub]", lang, weight, module_ordinal, is_project, include/exclude }`.

Views:

- `ComponentFs(component)`: a virtual directory tree built from the mounts of one component.
  - ReadDir merges children across mounts. Directories merge by name. Files dedupe first-wins by
    weight for layouts/assets, keep all for data/i18n, and keep language variants for content.
  - Returns `FileMeta { filename, lang, weight, module_ordinal, path_info: paths::Path }`.
  - Apply the `component_fs.go:106-145` sort. **NFC-normalize names on macOS** and generate the
    same normalization on Linux only if the original filesystem is macOS; parity is measured on
    darwin.
  - Drop entries whose PathInfo is disabled (disabled language).
- `StaticFs`: overlay of static mounts, used by the static copier.
- `PublishFs`: the output dir writer. It records "files containing `__h_pp_l1`" for text suffixes
  (`hugofs/hasbytes_fs.go`) and creates parent dirs.
- `WorkingDirFs`: `hugo_stats.json` and `.hugo_build.lock`.

The `assets` view resolves `resources.Get "/vendor/jquery/dist/jquery.slim.js"` to
`node_modules/jquery/...` through the `assets/vendor` mount. The same view is used by the libsass
importer (`includePaths` "node_modules", "assets/scss") and by the esbuild resolver plugin
(`internal/js/esbuild/resolve.go:160-240`).

### 5.4 Identity and caching for a one-shot build

- **Drop:** `identity` (dependency tracking for server rebuilds), `dynacache` eviction and
  partitions, `BuildState` rebuild signals, `WhatChanged`, `resetBuildState`, the fast-render
  paths, and watcher and livereload.
- **Keep the observable "get-or-create, first wins" semantics:**
  - `ResourceCache.GetOrCreate(key)`: `resources.Get` key `cleanKey(path)+"__get"`, Concat by
    target path, `Copy`, transformations by chain key.
  - `ImageCache` memory key: the target path (`image_cache.go:40-51`).
  - `partialCached`: not used by seeksnack.
  - Publish-once per resource (`genericResource.publishInit` OnceMore).
  - Page `.Content`, `.Summary`, `.TableOfContents` computed once per output format. Content is
    reused across formats when `canReusePageOutputContent` holds (`page.go:121`, used at `page.go:696-712`).
- **File caches:** `getresource` is **required (read)**. `images`/`assets` must **not** be read
  (§2.1). `modules`, `misc`, `getjson`, `getcsv` are not needed.

### 5.5 Hashing (crate `gohashstructure`, exact)

- `gohugoio/hashstructure.Hash(v, &HashOptions{Hasher: xxhash.New()})` (`common/hashing/hashing.go:81-159`).
  `xxhash` is XXH64 with seed 0.
  - `string` → `xxh64(bytes)`.
  - `int` → `binary.Write(LE int64)`. Other int, uint and float kinds are written LE at their
    natural size. `bool` → int8 0/1.
  - `time.Time` → `MarshalBinary` bytes.
  - Slice/array: `h=0; for e: h = xxh64(LE(h)‖LE(hash(e)))`. For sets, XOR, then finish.
  - Map: `h ^= xxh64(LE(hash(k))‖LE(hash(v)))` over entries, then `h = xxh64(LE(h))` (finish). A
    **nil or empty map → `xxh64(8 zero bytes)`**.
  - Struct: start from `h = hash(typeName)`. The field loop (`hashstructure.go:324-393`) works
    field by field:
    - Unexported fields, `hash:"ignore"`/`"-"` fields, zero values when `IgnoreZeroValue` is set,
      and fields excluded by `Includable` all `continue`. That skips both the XOR and the finish.
    - An included field does `h ^= xxh64(LE(hash(fieldName))‖LE(hash(value)))`, and then
      `h = xxh64(LE(h))` (finish). The finish call sits **inside the loop**, at line 392.
    - The `Hashable` and `Includable` interfaces are honoured.
  - Pointers and interfaces are dereferenced.
- `HashString(vs...)` = decimal of `HashUint64`. `HashStringHex` = lowercase hex without padding
  (`strconv.FormatUint(h,16)`; for example `a8d2dfd3d47df12` has 15 digits).
  `HashUint64(v1..vn)` hashes `[]any{toHashable(v)...}` when n>1. `toHashable` maps
  `Key()`-providers to their key string.
- `XXHashFromReader` = XXH64 streaming of the file.
- Where hashing reaches output bytes: `_hu_` image names (`resources/image.go:459-480`), image
  filter keys (`image.go:265`), and the imaging config `SourceHash`. Also cache keys: GetRemote
  (§2.3) and file cache names. Fingerprints use sha256 (`resources/resource_transformers/integrity`)
  and base64 std for the `Data.Integrity` value.

### 5.6 Output formats and media types (exact defaults)

Media types (`media/builtin.go:59-113` `Builtin`, `115-175` `defaultMediaTypesConfig`). `Types` are sorted by the
`Type` string (`media/config.go:257`). `FirstSuffix` is the first listed suffix, delimiter `"."`.

| Type | Suffixes |
|---|---|
| text/calendar | ics |
| text/css | css |
| text/x-scss | scss |
| text/x-sass | sass |
| text/csv | csv |
| text/html | html, htm |
| text/javascript | js, jsm, mjs |
| text/typescript | ts |
| text/tsx | tsx |
| text/jsx | jsx |
| text/x-gotmpl | gotmpl |
| application/json | json |
| application/manifest+json | webmanifest |
| application/rss+xml | xml, rss |
| application/xml | xml |
| image/svg+xml | svg |
| text/plain | txt |
| application/toml | toml |
| application/yaml | yaml, yml |
| image/png | png |
| image/jpeg | jpg, jpeg, jpe, jif, jfif |
| image/gif | gif |
| image/tiff | tif, tiff |
| image/bmp | bmp |
| image/webp | webp |
| font/ttf | ttf |
| font/otf | otf |
| application/pdf | pdf |
| text/markdown | md, mdown, markdown |
| text/asciidoc | adoc, asciidoc, ad |
| text/pandoc | pandoc, pdc |
| text/rst | rst |
| text/org | org |
| video/x-msvideo | avi |
| video/mpeg | mpg, mpeg |
| video/mp4 | mp4 |
| video/ogg | ogv |
| video/webm | webm |
| video/3gpp | 3gpp, 3gp |
| application/wasm | wasm |
| application/octet-stream | (none) |

Media type semantics:

- `FromString` splits `main/sub+suffix` (`mediaType.go:137-163`). `String()` returns `Type`,
  which templates print (e.g. `type="application/rss+xml"`).
- `IsText()`: main type `text`, or sub type in {javascript, json, rss, xml, svg, toml, yml, yaml}.
- The suffix `xml` resolves first to `application/rss+xml`, because types are sorted by string.

Output formats (`output/outputFormat.go:84-219`; also `$DATA/config-en.json` → `outputformats`).
Sorting: weight>0 first (ascending), then by name (`outputFormat.go:252-263` `Formats.Less`).

| Name | MediaType | BaseName | Path | Rel | Flags |
|---|---|---|---|---|---|
| html | text/html | index | | canonical | IsHTML, Permalinkable, Weight 10 |
| 404 | text/html | | | | IsHTML, Ugly, NotAlternative, Permalinkable |
| alias | text/html | | | | IsHTML, Ugly |
| amp | text/html | index | amp | amphtml | IsHTML, Permalinkable |
| calendar | text/calendar | index | | alternate | IsPlainText, Protocol webcal:// |
| css | text/css | styles | | stylesheet | IsPlainText, NotAlternative |
| csv | text/csv | index | | alternate | IsPlainText |
| gotmpl | text/x-gotmpl | | | | IsPlainText, NotAlternative |
| json | application/json | index | | alternate | IsPlainText |
| markdown | text/markdown | index | | alternate | IsPlainText |
| robots | text/plain | robots | | alternate | IsPlainText, Root |
| rss | application/rss+xml | index | | alternate | NoUgly |
| sitemap | application/xml | sitemap | | sitemap | Ugly |
| sitemapindex | application/xml | sitemap | | sitemap | Ugly, Root |
| webappmanifest | application/manifest+json | manifest | | manifest | IsPlainText, NotAlternative |

Kind defaults (`allconfig.go:1191-1217`):

- `page:[html]`
- `home`, `section`, `taxonomy`, `term`: `[html, rss]`
- `rss:[rss]`

seeksnack overrides this with `home=[HTML,JSON,RSS]`, `page=[HTML]` and `section/taxonomy/term=[HTML,RSS]`.
The sitemap basename is taken from `sitemap.filename` (`content_map_page.go:1985-2000`).

Page kinds (`resources/kinds/kinds.go:20-42`): page, home, section, taxonomy, term, plus
temporary kinds rss, sitemap, sitemapindex, robotstxt, 404.

`IsPlainText` selects the text/template parser; otherwise html/template is used, which changes
escaping.

### 5.7 Site / HugoSites / Page relationships (Rust shape)

```
HugoSites (1)  ── owns ── sites: Vec<Site> [en, th]  (default lang first, then weight, lang)
   ├─ page_tree: BTreeMap<String /*"/", "/almonds", "/brands/lays", "/404", "/_robots", "/_sitemap"*/, [Option<PageId>; NLANG]>
   ├─ resource_tree: BTreeMap<String, [Option<ResId>; NLANG]>
   ├─ pages: Arena<PageState>   (page id -> lang, kind, meta, outputs[h.render_formats.len()])
   ├─ render_formats: Vec<(site_idx, OutputFormat)>   (concatenation of each site's sorted formats)
   ├─ shared: template store, resource spec + caches, file caches, exec/esbuild/libsass handles, data tree, build stats
Site ── language, config (per-lang clone), publisher (own HTML-elements collector), taxonomies, menus,
        related index, home, lazy inits (prevNext, menus, taxonomies)
PageState ── translations = same tree key, other language slots;  parent/sections = tree prefixes
```

- The **radix tree (`armon/go-radix`) walk equals `BTreeMap` iteration** in byte-lexicographic
  order, with the parent before its children. `WalkPrefix` becomes a `range(prefix..)` that
  `take_while` the key `starts_with(prefix)`.
- Cyclic Go pointers (Page↔Site↔HugoSites) become arena indices plus a `&BuildCtx` passed into
  template execution (Go passes `context.Context` carrying the current page:
  `tpl.Context.Page.Set`, `site.go:1448`).

### 5.8 `deps.Deps` → `SiteDeps` context struct

Fields used during a build:

- `Fs`, `Conf` (per language), `Log`
- `PathSpec` (URL/path helpers + BaseFs + ProcessingStats)
- `ContentSpec` (markup converter provider)
- `SourceSpec`
- `ResourceSpec` (resource factory, image cache, `PostProcessResources` registry, JSConfigBuilder)
- `TemplateStore` (+ per-site func map)
- `TranslationProvider` / translate func
- `ExecHelper` (security-checked exec)
- `BuildState` (postpub filenames)
- `MemCache`

In Rust: `Arc<SharedDeps>` plus a per-site `SiteDeps { lang, conf, path_spec, translate }`.

### 5.9 Errors and logging

- The exit status is non-zero if any `ERROR` is logged (`hugo_sites_build.go:212-223`).
- `warnf`/`erroridf` in templates go to stderr. Stdout (the stats table) is not compared.
- Error message text need not match.

---

## 6. Proposed Rust Cargo workspace

### 6.1 Principles

1. Crate boundaries follow Go package clusters so that ports are traceable. Each crate's README
   lists `Go file → Rust module`.
2. Third-party algorithm ports are separate crates with no Hugo knowledge: `goldmark`,
   `tdewolff-parse`, `tdewolff-minify`, `goimage`, `goflate`, `gift`, `gohashstructure`,
   `xtext-collate`, `goyaml`. They can be built and tested in isolation against Go oracles, in parallel.
3. Hugo layers depend only downward. Cycles in Go (page ↔ template ↔ resources) are broken with
   traits defined low (`nh-api`) and implemented high (`nh-hugolib`).
4. Phase-1 code is single-threaded and deterministic. The concurrency seams (`rayon` for images,
   parse and postProcess; pages later) are designed in from the start.

### 6.2 Crates

| # | Crate | Responsibility | Ports (Go sources) | ~Lines to port | Parity-critical |
|---|---|---|---|---:|---|
| 1 | `gostd` | Go stdlib behaviours: fmt, strconv (format/quote/parse), strings/unicode (Go 17.0 tables), utf8, path, filepath, net/url, html (escape + entity table), encoding/json (encode + decode→Value), time (Format/Parse layouts, Location, MarshalBinary), sort (pdqsort port), mime subset | `$GOROOT/src/{fmt,strconv,strings,unicode,path,net/url,html,encoding/json,time,sort}` subsets | 8–10k | yes |
| 2 | `gohashstructure` | hashstructure + XXH64 helpers + `HashString/HashUint64/HashStringHex/XXHashFromReader`; `GoHash` trait | `hashstructure@v0.5.0/hashstructure.go`, `$REPO/common/hashing/*.go` | 0.8k | yes |
| 3 | `goyaml` | YAML 1.1 decoding with yaml.v2 semantics (resolve.go tags: bool yes/no/on/off, ints incl. octal/`0b`, floats, `~`, timestamps; merge keys; `map[any]any` → string keys via Hugo) over a libyaml-port event stream | `yaml.v2@v2.4.0/{resolve,decode,sorter}.go`, or a full port incl. scanner | 1.5–9k | yes |
| 4 | `xtext-collate` | `collate.New(tag).CompareString` for en, th (and more later) with CLDR 23 tables | `x/text@v0.26.0/collate/*`, `internal/colltab/*`, table generator | 3k + generated data | yes (template `sort`) |
| 5 | `goldmark` | CommonMark parser + HTML renderer + extensions (table, strikethrough, linkify, tasklist, definition list, footnote, typographer) | `goldmark@v1.7.12/**` | ~12.5k | yes |
| 6 | `tdewolff-parse` | lexers/parsers for html, css, js, json, xml; strconv; buffer | `parse/v2@v2.8.1/**` | ~12.5k | yes |
| 7 | `tdewolff-minify` | minify html, css, js, json, svg, xml + M registry/params | `minify/v2@v2.23.8/{minify.go,common.go,html,css,js,json,svg,xml}` | ~12.2k | yes |
| 8 | `goflate` | compress/flate (deflate compressor + inflate), zlib, adler32/crc32 via crates | `$GOROOT/src/compress/{flate,zlib}` | ~5.8k | yes (PNG bytes) |
| 9 | `goimage` | Go image model (RGBA, NRGBA, RGBA64, NRGBA64, Gray, Gray16, YCbCr, CMYK, Paletted; color models & conversions), draw (Src/Over, FloydSteinberg), jpeg codec, png codec, gif decode | `$GOROOT/src/image/{.,color,draw,jpeg,png,gif}` | ~9k | yes |
| 10 | `gift` | resampling (resize filters), other filters used by Hugo | `gift@v1.2.1/**` | ~3.3k | yes |
| 11 | `libwebp-sys` | vendored libwebp C sources from `gowebp@v0.3.0/internal/libwebp/*.c` (122 files) + `libwebp_src` headers, built with `cc`, `-O2` (cgo default `-g -O2`) | copy sources | C | yes |
| 12 | `libsass-sys` | vendored libsass C++ from `golibsass@v1.2.0/internal/libsass/*.cpp` (62 files) + `libsass_src/include`, built with `cc`: CFLAGS `-O2 -fPIC`, CXXFLAGS `-std=c++0x -O2 -fPIC` | copy sources | C++ | yes |
| 13 | `nh-common` | common/{maps (Params), paths (PathParser!, path/url helpers), text, urls (BaseURL), htime (clock), hstrings, types subset, loggers (minimal)}, helpers/{path,url,general,content subset}, compare | `$REPO/common/**`, `helpers/*.go`, `compare/*.go` | ~6k | partly (paths/urls: yes) |
| 14 | `nh-value` | `Value`/`Object`/`Params` model, cast semantics, truthiness, Go-kind equality/ordering (tpl/compare internals), fmtsort | `spf13/cast`, `common/hreflect`, `common/maps`, `tpl/compare`, `go_templates/fmtsort` | ~3.5k | yes |
| 15 | `nh-parser` | metadecoders (TOML via `toml` crate + mapping, YAML via `goyaml`, JSON), pageparser (front matter + shortcode lexer), ReplacingJSONMarshaller | `$REPO/parser/**` | ~2.3k | yes |
| 16 | `nh-media-output` | media types, output formats, layout descriptors | `$REPO/media/*.go`, `$REPO/output/*.go` | ~1.4k | yes |
| 17 | `nh-config` | load hugo.toml/yaml/json + config dir + env + flags; defaults; all section decoders; per-language merge; Configs.Init; module mounts (project only); security/privacy/services/sitemap/pagination/permalinks/related/imaging/minify/markup/caches/httpcache configs | `config/**`, `config/allconfig/**`, `modules/{collect,config,module}.go` (mount parts), `langs/config.go`, `minifiers/config.go`, `markup/markup_config`, `goldmark_config`, `resources/images/config.go` (decode), `cache/filecache/filecache_config.go`, `cache/httpcache` | ~6k | yes (defaults) |
| 18 | `nh-vfs` | Mount/RootMapping, overlay, ComponentFs, Walkway, ignore rules, NFC, HasBytes recorder, publish writer, static syncer | `hugofs/**`, `hugolib/filesystems/basefs.go`, `hugolib/paths/paths.go`, `bep/overlayfs`, `spf13/fsync` (semantics) | ~4k | yes (ordering) |
| 19 | `nh-cache` | filecache (getresource read/write; layout `<dir>/filecache/<name>/<key>`), HTTP dump parse/write, memory get-or-create maps | `cache/filecache/*.go`, `cache/httpcache`, `gohugoio/httpcache` (subset) | ~1.2k | yes (GetRemote) |
| 20 | `gotemplate` | text/template (lexer, parser, exec) + html/template (escaper) incl. Hugo fork changes; func-map interface over `nh-value` | `$REPO/tpl/internal/go_templates/{texttemplate,texttemplate/parse,htmltemplate,fmtsort,cfg}` | ~10.7k | yes |
| 21 | `nh-markup` | Hugo goldmark integration: converter, render hooks (link/image/heading/blockquote/table/codeblock), attributes, autoid (github), TOC, hugocontext; asciidoc/pandoc/rst/org stubs; highlight config (+ stub) | `$REPO/markup/**` | ~3.5k | yes |
| 22 | `nh-minifiers` | minifier registry by media type + config → `tdewolff-minify` | `$REPO/minifiers/*.go` | 0.3k | yes |
| 23 | `nh-images` | image spec parsing (`"300x240 webp"`), ImageConfig, processing (resize/fill/fit/crop/filter/overlay), encoders (jpeg quality, png, webp hints via libwebp), exif (minimal), target naming | `$REPO/resources/images/**`, `bep/gowebp` Go wrapper, `bep/imagemeta` (min) | ~3.5k | yes |
| 24 | `nh-exec` | security-checked process runner, npx/node_modules resolution, exec environ | `common/hexec`, `config/security`, `common/neohugo/neohugo.go:GetExecEnviron` | 0.6k | medium |
| 25 | `nh-esbuild` | esbuild 0.25.6 service-protocol client (packet codec, build request, plugins onResolve/onLoad backed by `nh-vfs`), option mapping, source maps (off) | `$REPO/internal/js/esbuild/{options,resolve,build,helpers,sourcemap}.go`, `esbuild/pkg/api` semantics + `cmd/esbuild/service.go` protocol | ~2k | yes |
| 26 | `nh-scss` | libsass wrapper (options, output style, includePaths, importer callback via `nh-vfs`), `toCSS` transformer | `golibsass/libsass/*.go`, `internal/libsass/a__*.go`, `$REPO/resources/resource_transformers/tocss/{scss,sass}` | 0.8k | yes |
| 27 | `nh-resources` | Resource/Spec/genericResource/imageResource/resourceAdapter (transform chains, lazy publish-once, Key semantics incl. §2.1), factories (Get, GetRemote, Concat, FromString, ExecuteAsTemplate), transformers (postCSS, minify, fingerprint/integrity, js.Build, toCSS), postpub | `$REPO/resources/*.go`, `resource/`, `resource_factories/{create,bundler}`, `resource_transformers/{cssjs/postcss,integrity,js,minifier,templates}`, `postpub`, `internal`, `kinds`, `jsconfig` | ~5k | yes |
| 28 | `nh-page` | Page/Site API traits, Pages collection ops (sort, group, by*, related, pagination/Pager), permalinks, target path creation, taxonomy types, WeightedPages, menus (`navigation`), related inverted index, pagemeta (front matter handler, dates, build options, cascade) | `$REPO/resources/page/**`, `navigation/**`, `related/**` | ~6k | yes |
| 29 | `nh-i18n` | Language, i18n bundles (go-i18n), plural rules, translate func, locales (time/number formatting data for en, th), collator hookup | `$REPO/langs/**`, `go-i18n/v2/{i18n,internal/plural}`, `gohugoio/locales` data (generated) | ~2.5k + data | yes |
| 30 | `nh-tpl` | template store (lookup rules, descriptors, baseof/blocks, partials, shortcodes, render hooks, embedded templates), func namespaces used by seeksnack and common Hugo (collections, compare, strings, urls, safe, fmt, math, cast, crypto, hash, encoding, transform, resources, images, js, css, lang, time, path, reflect, hugo, site, partials, templates, os, inflect) | `$REPO/tpl/tplimpl/**`, `tpl/tplimplinit`, `tpl/<ns>/**`, `tpl/internal/resourcehelpers` | ~12k | yes |
| 31 | `nh-publisher` | Descriptor, transformer chain (absurl HTML/XML, minify), HTML elements collector, build stats JSON | `$REPO/publisher/**`, `$REPO/transform/{chain.go,urlreplacers}` | ~1.3k | yes |
| 32 | `nh-hugolib` | HugoSites/Site/pageState; content collection; content map + assembly (sections, taxonomies, terms, translations, aggregates, cascade, standalone pages); page content pipeline (shortcodes, summary, TOC, `.Content` per output); rendering loop, paginator, aliases, language redirect; data loading; postProcess; build stats; static copy orchestration | `$REPO/hugolib/**` (minus pagesfromdata, integration test builder), `source/**`, `deps/deps.go` | ~12k | yes |
| 33 | `nh-cli` (bin `neohugo`) | clap CLI, flag→config mapping, commands build/version/env/config (+mounts), stubs | `$REPO/commands/{commandeer,commands,helpers,hugobuilder(build part),config,env}.go`, `main.go`, `common/neohugo/version*.go` | ~1.5k | no (stdout) |
| — | `tools/go-oracle` (not shipped) | Go programs that emit fixtures (hash, fmt, time, url, json, template, goldmark, minify, jpeg/png/gift, config JSON, page dumps) for differential tests | uses `$REPO` via `replace` | — | test only |

### 6.3 Dependency graph (arrows point to dependencies)

```
L0  gostd   gohashstructure→gostd   goyaml→gostd   xtext-collate   goflate   gift→goimage→goflate
    tdewolff-parse→gostd   tdewolff-minify→tdewolff-parse   goldmark→gostd(html entities, unicode)
    libwebp-sys   libsass-sys
L1  nh-value→{gostd,gohashstructure}        nh-common→{gostd,nh-value}
L2  nh-parser→{nh-value,goyaml,gostd}       nh-media-output→{nh-value}      nh-vfs→{nh-common}
L3  nh-config→{nh-parser,nh-media-output,nh-vfs,nh-common}
    gotemplate→{nh-value,gostd}             nh-cache→{nh-config,nh-vfs,gohashstructure}
    nh-exec→{nh-config}                     nh-minifiers→{tdewolff-minify,nh-config,nh-media-output}
L4  nh-markup→{goldmark,nh-config,nh-common}          (defines hook traits)
    nh-images→{goimage,gift,libwebp-sys,nh-config,gohashstructure}
    nh-scss→{libsass-sys,nh-vfs}            nh-esbuild→{nh-exec,nh-vfs}
    nh-i18n→{nh-config,nh-value,gotemplate,xtext-collate}
L5  nh-resources→{nh-vfs,nh-cache,nh-images,nh-scss,nh-esbuild,nh-exec,nh-minifiers,nh-media-output,gohashstructure}
    nh-publisher→{nh-minifiers,nh-vfs,nh-media-output}
L6  nh-page→{nh-resources,nh-config,nh-value,nh-i18n,nh-markup(traits)}   (defines Page/Site traits = "nh-api")
L7  nh-tpl→{gotemplate,nh-page,nh-resources,nh-i18n,nh-markup,nh-value}
L8  nh-hugolib→{everything above}           (implements Page/Site; wires TemplateExecutor into markup hooks & resources.ExecuteAsTemplate)
L9  nh-cli→{nh-hugolib,nh-config}
```

Cycle breakers:

- Markdown render hooks and shortcodes need template execution. `nh-markup` defines
  `trait HookRenderer`; `nh-hugolib` implements it with `nh-tpl`.
- `resources.ExecuteAsTemplate` needs templates. `nh-resources` defines `trait TemplateExecutor`,
  implemented by `nh-tpl`.
- Template funcs need Page/Site. They reach them through `Value::Object` and the `nh-page` traits.
- Page `.Content` needs markup and templates. It is implemented in `nh-hugolib`.

### 6.4 Key interfaces (to freeze first, so agents can work in parallel)

```rust
// nh-vfs
pub trait ComponentFs { fn read_dir(&self, dir: &str) -> Result<Vec<FileMeta>>; fn open(&self, path: &str) -> Result<Bytes>; fn stat(&self, path:&str) -> Result<FileMeta>; }
// nh-publisher
pub struct Descriptor<'a> { pub src: &'a [u8], pub target_path: String, pub output_format: OutputFormat, pub abs_url_path: Option<String>, pub stat_counter: Counter }
pub trait Publisher { fn publish(&self, d: Descriptor) -> Result<()>; fn stats(&self) -> HtmlElements; }
// gotemplate
pub trait FuncMap { fn call(&self, name: &str, ctx: &mut ExecCtx, args: Vec<Value>) -> Result<Value>; }
pub fn execute(t: &Template, w: &mut Vec<u8>, data: Value, ctx: &mut ExecCtx) -> Result<()>;
// nh-markup
pub trait HookRenderer { fn render_link(&self, ctx: &LinkCtx, w:&mut Vec<u8>) -> Result<bool>; fn render_image(..); fn render_heading(..); ... }
pub fn convert(src: &[u8], cfg: &MarkupConfig, hooks: &dyn HookRenderer, doc: DocumentCtx) -> Result<ConvertResult /*html, toc, heading ids*/>;
// nh-images
pub fn process(src: &SourceImage, cfg: &ImageConfig, filters: &[Filter]) -> Result<EncodedImage>;
// nh-resources
pub trait Resource: Object { fn rel_permalink(&self) -> String; fn permalink(&self) -> String; fn key(&self) -> String; fn content(&self) -> Result<Value>; fn media_type(&self) -> MediaType; ... }
// nh-esbuild
pub fn build(opts: &BuildOptions, resolver: &dyn ImportResolver) -> Result<BuildResult>;
```

### 6.5 Native and external code

- **libwebp-sys.** Copy the `.c` and `.h` files from `$GOMODCACHE/github.com/bep/gowebp@v0.3.0/internal/libwebp`
  plus `libwebp_src/` headers into `crates/libwebp-sys/vendor/`. Compile every `.c` with
  `cc::Build`: `-O2 -g`, include `vendor/libwebp_src`, and the same defines as cgo. The Go
  package compiles all `.c` files in the dir; mirror that list exactly, including the
  `*_neon.c`/`*_sse2.c` variants, since libwebp dispatches on CPU at runtime. Use the same system
  clang, which is also what the golden build used. Port the Go-side encoder config mapping from
  `gowebp/libwebp/encoder.go`: hint, quality, method, and lossless for PNG sources.
- **libsass-sys.** Copy `$GOMODCACHE/github.com/bep/golibsass@v1.2.0/internal/libsass/*.{cpp,c,hpp,h}`
  and `libsass_src/include`. Build with `cc::Build::cpp(true)`, `-std=c++0x -O2 -fPIC`, link
  `c++`. Port the Go wrapper `a__libsass.go`/`a__importer.go`. The importer callback resolves
  `@import` through Hugo's assets fs and `includePaths`.
- **esbuild.** Porting its roughly 130k lines of Go is out of scope. Pin esbuild **v0.25.6** from
  `@esbuild/<platform>@0.25.6` on npm (esbuild is published as a Go-compiled binary) or build it
  from `$GOMODCACHE/github.com/evanw/esbuild@v0.25.6/cmd/esbuild`. Talk to it over the
  `--service=0.25.6 --ping` stdin/stdout binary protocol (`internal/js/esbuild`-equivalent
  options; plugin callbacks `on-resolve`/`on-load` implemented in Rust against `nh-vfs`). This
  reproduces Hugo's in-process `api.Build` exactly. The option mapping comes from
  `$REPO/internal/js/esbuild/options.go` and the plugin logic from `resolve.go`.
- **postcss.** Spawn `<workingDir>/node_modules/.bin/postcss --config <assets/_jsconfig/postcss.config.js resolved> ...`,
  with stdin = CSS and stdout = result, using the env from `GetExecEnviron`
  (`resources/resource_transformers/cssjs/postcss.go:143-239`).

### 6.6 Parallel implementation plan

- **Wave A (no Hugo dependencies, independent):** gostd, gohashstructure, goyaml, xtext-collate,
  goflate + goimage + gift, libwebp-sys, libsass-sys, tdewolff-parse + tdewolff-minify, goldmark,
  gotemplate. Each gets a Go oracle under `tools/go-oracle/<crate>`.
  - Corpus for the minify crate: the golden `run1` and `nominify` builds are a large real-world
    corpus. Input = nominify pages before minification; it can be captured by running the Go
    binary with a publisher hook, or by re-minifying the nominify HTML with the Go library.
  - Corpus for the image crates: the source images plus the 1,461 golden `_hu_` outputs.
- **Wave B:** nh-value, nh-common (PathParser), nh-parser, nh-media-output, nh-vfs, nh-config
  (oracle: `$DATA/config-*.json`), nh-cache, nh-exec, nh-minifiers, nh-markup (oracle: Go-rendered
  `.Content` per content file), nh-images, nh-scss, nh-esbuild, nh-i18n.
- **Wave C:** nh-resources, nh-publisher, nh-page, nh-tpl.
- **Wave D:** nh-hugolib + nh-cli, then the end-to-end diff against golden (§2.5).

---

## 7. Rust crates: safe vs not safe for byte parity

**Safe**, because the algorithms are standardized or deterministic or the crate is used only as plumbing:

- `xxhash-rust` (xxh64, seed 0)
- `md-5`, `sha1`, `sha2` (fingerprint, integrity, `md5` template func)
- `base64` (STANDARD, padded, matching Go `StdEncoding`), `hex`
- `crc32fast` (IEEE, PNG chunks), `adler` (zlib trailer)
- `memchr`, `aho-corasick` (placeholder and absURL scanning)
- `unicode-normalization` (NFC of macOS file names; the Unicode stability policy guarantees
  identical NFC for assigned characters)
- `clap`, `anyhow`/`thiserror`, `rayon`, `crossbeam`, `parking_lot`, `once_cell`, `indexmap`, `bstr`
- `serde_json` for **parsing only**, mapped to Go types: numbers → f64, which is correctly rounded
  in both languages
- `toml` for **parsing**, with a mapping layer to go-toml v2 decoded types. Verify dates and times
  and integer overflow against the Go oracle.
- `regex`: RE2-compatible for the patterns used here. Validate any user-facing `replaceRE`/`findRE`.
- `ryu` for shortest float digits only, with Go's layout rules applied on top
- `unsafe-libyaml` (a faithful libyaml port) as the YAML event source under a yaml.v2 resolve/decode port
- `inflate` via `miniz_oxide`/`flate2` for **decompression only**. PNG decoding is lossless, but
  the pixel **types** must still be mapped exactly as Go's png decoder does.

**Not safe**, meaning output would differ:

- **Markdown:** `pulldown-cmark`, `comrak`, `markdown-rs`. Renderer and extensions differ from
  goldmark (typographer entities, linkify, footnotes, attribute handling, raw HTML).
- **Minifiers:** `minify-html`, `lightningcss`, `oxc`, `swc`, `css-minify`. They are different
  algorithms from tdewolff.
- **Templates:** `tera`, `handlebars`, `gtmpl-rust`. None implement html/template contextual
  escaping or Hugo's fork semantics.
- **JPEG decoders** (`jpeg-decoder`, `zune-jpeg`, `image`'s jpeg): IDCT, upsampling and YCbCr
  conversion differ from Go's `image/jpeg`.
- **JPEG encoders** (`jpeg-encoder`, `mozjpeg`, `image`): quantization and Huffman tables, FDCT and
  chroma subsampling differ.
- **Deflate encoders** (`flate2`, `miniz_oxide`, `zopfli`, `libdeflater`), which also rules out
  PNG encoders built on them (`png`, `image`): different compressed bytes.
- **Resize** (`image::imageops`, `fast_image_resize`, `resize`): different filters and float
  accumulation from gift.
- **Sass:** `grass`, `rsass`, `sass-rs` against another libsass version. Use the vendored libsass
  3.x from golibsass v1.2.0 only.
- **WebP:** `libwebp-sys` or `webp` from crates.io link a different libwebp version or build flags;
  vendor gowebp's sources instead.
- **YAML:** `serde_yaml`, `yaml-rust2`, `saphyr` directly. They use YAML 1.2 resolution (`yes`,
  `on`, octal, timestamps).
- **Collation:** `icu_collator` (ICU4X) uses modern CLDR, which differs from x/text's CLDR 23 tables.
- **URL and HTML escaping:** `url` (WHATWG), `percent-encoding`, `html-escape`. They differ from Go
  `net/url` and `html.EscapeString`.
- **Time and JSON serialization:** `chrono` formatting (strftime) and `serde_json` serialization
  (escaping, float formatting, key order) do not match Go.
- **Unicode:** Rust `char::is_alphabetic` and `to_lowercase` may use a different Unicode version
  and multi-char mappings. Go `unicode.ToLower` is 1:1.
- **Globs:** `glob`/`globset` for gobwas semantics. Low risk; not exercised by seeksnack.

---

## 8. CLI surface

**Keep and implement**, from `commands/commandeer.go:515-616` and `helpers.go:73-114`:

- `neohugo [flags]` and `neohugo build [flags]`:
  - `-s/--source`, `-d/--destination` (→`publishDir`), `-e/--environment`, `--config`,
    `--configDir` (default `config`), `-b/--baseURL`, `--minify` (→`minifyOutput` →
    `minify.minifyOutput`), `--cacheDir`, `-c/--contentDir`, `-l/--layoutDir`, `--themesDir`,
    `-t/--theme`
  - `-D/--buildDrafts`, `-F/--buildFuture`, `-E/--buildExpired`, `--disableKinds`
  - `--cleanDestinationDir`, `--ignoreCache`, `--noBuildLock`, `--noTimes`, `--noChmod`
  - `--quiet`, `--logLevel`, `--printPathWarnings` (duplicate-target detection is useful for §2.2),
    `--printI18nWarnings`, `--printUnusedTemplates`, `--panicOnWarning`
  - `--clock` (required, §2.4), `-M/--renderToMemory` (optional), `--renderSegments` (optional)
- `neohugo version`: prints `neohugo v0.149.0-DEV[-<rev>] <GOOS>/<GOARCH> BuildDate=<date>`
  (`common/neohugo/version.go:139-...`). Use Go GOOS/GOARCH names (`darwin/arm64`).
- `neohugo env`: version plus Go-style runtime info (`commands/env.go`).
- `neohugo config [--format toml|yaml|json] [--lang L] [--printZero]` and `neohugo config mounts`
  (`commands/config.go`). The JSON form is the oracle for `nh-config`.
- Environment: `HUGO_ENVIRONMENT`/`HUGO_ENV`, `HUGO_*` config overrides (delimiter = the first
  character after `HUGO`), `HUGO_NUMWORKERMULTIPLIER`, `HUGO_CACHEDIR`.

**Stubs** (print "not supported in the Rust port yet" and exit 1): `server`, `new` (site, theme,
content), `mod` (graph, init, get, tidy, vendor, clean, npm pack, verify), `deploy`, `gen`
(chromastyles, doc, man, docshelper), `convert`, `import`, `list`, `release`, `completion`, and
`-w/--watch`. Also `--gc`, `--enableGitInfo`, `--templateMetrics`, `--poll`, `--forceSyncStatic`,
and the profiling flags.

---

## 9. Parity risks (ranked)

1. **Cache-state-dependent image names** (§2.1). The Rust port must implement create-path `Key()`
   semantics, and testers must run cold. Stale `resources/_gen` in the original site dir
   (`~/git/github/seeksnack-seeksnack/resources`, 2,824 files) would make even Go disagree with
   the golden.
2. **html/template contextual escaping and Hugo's fork changes.** Every page passes through them.
   Only a line-by-line port is viable.
3. **tdewolff minify (HTML/CSS/JS/JSON/SVG/XML).** Every output byte goes through it, including
   inline `<script>`/`<style>` and JSON-LD `application/ld+json`.
4. **goldmark + Hugo hooks.** Embedded link hook fallback for multilingual sites, the custom
   `render-image` (watermark overlay) and `render-heading`, typographer, and github-style autoid on
   Thai text (Go Unicode 17 tables).
5. **Image pipeline bytes.** Go jpeg decode → gift resize (float32) → overlay → encode (Go jpeg q75,
   Go png/flate, libwebp). Plus the hashstructure-derived `_hu_` names.
6. **postcss/purgecss depends on `hugo_stats.json`,** which must match Go byte-for-byte: the element
   collector state machine over minified HTML (`publisher/htmlElementsCollector.go:160-556`), plus
   sort and JSON encoding. The CSS is inlined into every HTML page, so one wrong class changes
   about 3,400 files.
7. **Concat first-wins and duplicate-path races** (§3.10). Keep sequential walk-order semantics.
8. **Collation (x/text CLDR 23)** for template `sort` of strings (tags, Thai terms). Also
   `.ByTitle` and similar sorts where used.
9. **YAML 1.1 and TOML type mapping** in front matter (218 YAML, 32 TOML files): dates, ints vs
   floats, bools.
10. **GetRemote cache** (§2.3): the hashstructure key and HTTP dump parsing. A miss means network
    data, and therefore a diff.
11. **Time:** Go layout formatting, `--clock`, UTC default, RSS/sitemap date formats, Thai locale
    formatting if `time.Format` with a language is used (`gohugoio/locales` data).
12. **esbuild output.** It must be the same 0.25.6 binary and options: target es2015, minify,
    `targetPath`s, plugin resolution through `assets/vendor`.
13. **libsass.** Same sources and flags, and the same importer resolution order for
    `includePaths` ("node_modules", "assets/scss").
14. **Unicode and NFC** differences on file names (darwin-only NFC in `component_fs.go`).
15. **`now`-year drift** after 2026 (§2.4).

---

## 10. Open questions

1. Should the golden be regenerated with an explicit `--clock` now, so the 2027 year rollover does
   not invalidate it?
2. Should the acceptance harness exclude the 26 collision files, or compare against all known
   variants? The Rust sequential order matches the single-worker Go build, which is identical to
   run2 except for `brands/lays`.
3. Is it acceptable to ship the npm/Go-built esbuild 0.25.6 binary alongside the Rust binary? This
   is the only non-Rust, non-C runtime piece; Node for postcss is already required by the site.
   The alternative is a multi-year esbuild port.
4. Is darwin/arm64 the only acceptance platform? libwebp SIMD dispatch and float differences in C
   code could matter on x86_64 or Linux. Go's own image code is pure integer or IEEE float32/64
   and portable.
5. Should the Rust port write `resources/_gen` for speed? If so, warm builds must replicate the Go
   warm-path `Key()` quirk, or the Rust port must ignore the cache on read, which would diverge
   from Go warm builds but match the golden.
6. `neohugo config` output parity (TOML/YAML dump formatting): nice to have, or required?
