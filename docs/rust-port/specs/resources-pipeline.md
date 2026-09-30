# Spec: RESOURCE PIPELINE (non-image) — neohugo → Rust port

> **Byte-parity sections obsolete.** This spec is research for the old byte-for-byte port
> (the `crates/` tree deleted in T00 of [`REWRITE_PLAN.md`](../REWRITE_PLAN.md); recoverable with
> `git show go-parity-final:crates/<path>`, commit `be02933a`, local tag). The Rust rewrite in
> `rust/` compares structurally (REWRITE_PLAN.md §7), so every byte-parity target, golden-byte
> count and "reproduce Go's bytes" rule below is obsolete. The Hugo semantics it documents (Go
> file and line references, parity traps) remain a reference; scratch paths (`/Users/…`,
> `/private/tmp/…`, `$W`, `$SP`, `golden/run1`) no longer exist. Current state:
> [`HANDOFF.md`](../HANDOFF.md).

Agent: `resources-pipeline`. Scope: `resources.Get`, publishing and links (`Permalink`/`RelPermalink`/`.Content`), the transformation chain engine, `resources.ExecuteAsTemplate`, `resources.Concat`, `js.Build` (esbuild), `toCSS` (libsass), `postCSS`, `minify` (as a resource transform), `fingerprint`, `resources.PostProcess`, build stats (`hugo_stats.json`), and `resources/_gen` caching.

All Go paths are relative to `/Users/blackb1rd/git/github/org/neohugo` unless prefixed with `$GOMODCACHE` (`/Users/blackb1rd/go/pkg/mod`).
`$SP` = `/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad`, `$W` = `$SP/work/resources-pipeline`.

---

## 0. TL;DR: verified results

Every non-image pipeline artifact in the golden output was reproduced byte for byte **without neohugo**:

| Golden artifact | Size | Reproduced with | Result |
|---|---|---|---|
| `js/set-theme.5307ae76…aedd.js` | 179 | esbuild 0.25.6 native CLI | MATCH |
| `js/website.77dd0c33…9f0d.js` (list-type pages; with jQuery) | 161160 | esbuild CLI ×2 levels + concat `"\n;\n"` + sha256 | MATCH |
| `js/website.8c83f39f…6a8f.js` (single + term pages; md5 + comment) | 100316 | same | MATCH |
| inline `<style>` CSS in 1940 HTML pages (sha256 `186be38d…81e4`) | 51857 | libsass 3.6.6 compiled with clang `-std=c++0x -O2` (C++ driver) → `node_modules/.bin/postcss --config …` → tdewolff CSS minify (with SVG minifier registered) | MATCH |
| `hugo_stats.json` (site root) | 19674 | Python re-implementation of `publisher/htmlElementsCollector.go` run over the 3422 published (minified) HTML files | MATCH |

The end-to-end script is `$W/repro_all.sh <siteDir> <outDir>`. It uses the native esbuild binary, the libsass driver `$W/libsass/driver`, the node postcss, and `$W/gomin/gomin` (a Go tdewolff helper; the Rust tdewolff port replaces it). The stats reproducer is `$W/stats_repro.py`, and the esbuild JS-API reproducer with Hugo-plugin emulation is `$W/esbuild/repro.js`.

Key facts:
- **The two `website.<hash>.js` variants do not come from EN vs TH or from the `api` param.** They come from two different `resources.Concat` targets:
  - `"js/website.js"` = jquery + bootstrap + search + themeswitch. It is used by `index.html`, `_default/list.html`, `taxonomy/list.html`, `404.html` and `_default/simple.html` (239 pages).
  - `"js/all.js"` = bootstrap + search + themeswitch + md5 + comment. It is used by `_default/single.html` and `term/term.html` (1701 pages).
  - Both are then built with `js.Build (dict "targetPath" "js/website.js" "minify" true)`, so both are named `js/website.<sha256>.js`.
- **Race:** `term/term.html` concatenates `"js/all.js"` in a DIFFERENT order (bootstrap, search, md5, comment, themeswitch). Concat is cached by target path only (`bundler.go:84-85`), so the first page that executes it wins for the whole build, across both languages. The golden output has the `single.html` order. The term order would give `website.cda9b0b0e83a7e1405811ba9f3a7e953aef0d3fcd0f5dc0d54a6e0a6aded3c77.js`, which I reproduced as a counter-check.
- **Only resources whose `.Permalink`/`.RelPermalink` is called are written to disk.** The SCSS chain is only inlined through `resources.PostProcess` + `.Content`, so no `scss/*.css` is published. `css/seeksnack.css` in the output is a plain copy of `static/css/seeksnack.css`.
- **postCSS/purgecss reads `./hugo_stats.json`, which is written by the same build** after all pages render and before post-processing (`hugolib/hugo_sites_build.go:168-195`). The whole CSS chain (toCSS → postCSS → minify → fingerprint) runs lazily inside the post-process phase, triggered by the `__h_pp_l1_1_Content__e=` placeholder.
- The placeholder is replaced after HTML minification and absURL canonicalization, so the CSS is inserted verbatim.

---

## 1. Inventory: what seeksnack uses (production branch; `hugo.Environment == "production"`)

### 1.1 CSS (`layouts/partials/head.html:275-281`)
```
{{ $cssOpts := (dict "enableSourceMap" false "includePaths" (slice "node_modules" "assets/scss") "outputStyle" "compressed") }}
{{ with $styles := resources.Get "scss/website.scss" | toCSS $cssOpts | postCSS | minify | fingerprint | resources.PostProcess }}
  <style>
    {{ .Content | safeCSS }}
  </style>
{{ end }}
```
The development branch (`head.html:270-274`, with ExecuteAsTemplate on scss + `.RelPermalink`) is NOT executed.

### 1.2 JS (`afterbody` and `script` blocks, rendered by `_default/baseof.html:22,28`)
- `set-theme`, in every template with an `afterbody` block (index, 404, list, single, simple, taxonomy/list, term):
  `resources.Get "ts/themeset.ts" | js.Build (dict "targetPath" "js/set-theme.js" "minify" true) | fingerprint` → `<script src="{{ .Permalink }}">`
- List-type templates (`index.html:216-229`, `_default/list.html:100-113`, `taxonomy/list.html:112-125`, `404.html:62-75`, `_default/simple.html:47-60`):
  ```
  $jsJquery    := resources.Get "/vendor/jquery/dist/jquery.slim.min.js"
  $jsBootstrap := resources.Get "/vendor/bootstrap/dist/js/bootstrap.bundle.min.js"
  $jsWebsiteTheme  := resources.Get "ts/themeswitch.ts" | js.Build (dict "target" "es2015")
  $jsWebsiteSearch := resources.Get "ts/search.ts" | resources.ExecuteAsTemplate "ts/search.ts" (dict "api" .Site.Params.Comment.Apipro) | js.Build (dict "target" "es2015")
  $script := slice $jsJquery $jsBootstrap $jsWebsiteSearch $jsWebsiteTheme | resources.Concat "js/website.js" | js.Build (dict "targetPath" "js/website.js" "minify" true) | fingerprint
  <script src="{{ $script.Permalink }}">
  ```
- `_default/single.html:202-224`: `slice $jsBootstrap $jsWebsiteSearch $jsWebsiteTheme $jsMd5 $jsComment | resources.Concat "js/all.js" | js.Build $opts | fingerprint`
- `term/term.html:204-223`: `slice $jsBootstrap $jsWebsiteSearch $jsMd5 $jsComment $jsWebsiteTheme | resources.Concat "js/all.js" | …` (**different order, loses the race**)
- `$jsComment = resources.Get "ts/comment.ts" | resources.ExecuteAsTemplate "ts/comment.ts" (dict "api" .Site.Params.Comment.Apipro) | js.Build (dict "target" "es2015")`
- `$jsMd5 = resources.Get "/vendor/md5/dist/md5.min.js"`
- `.Site.Params.Typescript.Compiler.Target` = `"es2015"`. `.Site.Params.Comment.Apipro` = `"https://api.seeksnack.com"`. Param lookups on `maps.Params` are case-insensitive. This is a template-engine concern, verified through the matching hashes.
- `/vendor/...` resolves through the module mount `node_modules/ → assets/vendor` (`hugo.toml [[module.mounts]]`).

### 1.3 Plain asset publishing through `.Permalink`/`.RelPermalink`
- `head.html:27-118`: `(resources.Get "images/favicon/<19 files>").Permalink`. These are published verbatim at `/images/favicon/<name>`. The golden output has exactly those 19 files plus 3 `_hu_` processed ones from header/footer.
- `head.html:153`: `(resources.Get site.Params.marketing.twitter.logo).Permalink` → `images/favicon/mstile-150x150.png`.
- `jsonLd.html:4` (`$imageLogo.Permalink`), `header.html`, `footer.html`: image processing belongs to the images agent.
- `jsonLd.html:202`: `resources.GetRemote` of the YouTube API. See §11.2.

### 1.4 Asset sources
`assets/scss/{website.scss,_*.scss}`, `assets/ts/{themeset,themeswitch,search,comment}.ts`. In `node_modules`: bootstrap 5.3.8, jquery 3.7.1, md5 2.3.0, mustache 4.2.0, @fortawesome/fontawesome-free 6.5.2, postcss-cli 11.0.1, postcss 8.5.8, autoprefixer 10.5.2, cssnano 7.1.2, @fullhuman/postcss-purgecss 6.0.0, postcss-fail-on-warn 0.2.1. Also `.browserslistrc`, `postcss.config.js` and `package.json` in the site root.

---

## 2. Resource model (Go: `resources/resource.go`, `resources/resource_factories/create/create.go`, `resources/transform.go`)

### 2.1 `resources.Get(path)` (`tpl/resources/resources.go:93-109` → `create.go:133-160`)
- `pathname = path.Clean(pathname)`. The cache key is `dynacache.CleanKey(pathname) + "__get"`, stored in `ResourceCache.cacheResource` (partition `/res1`).
- It `Stat`s the file in the **assets composite fs**, which has these mounts in order: `assets → <site>/assets`, `node_modules → assets/vendor`, plus `_jsconfig` single-file mounts for `package.json` and `postcss.config.js` (`modules/collect.go:597-637`).
- A missing file returns nil (no error).
- The resource is created with `LazyPublish: true`, so nothing is written until a link method is called.
- An empty string argument returns nil.

### 2.2 Links and publishing
- `resourceAdapter.Permalink()`/`RelPermalink()` (`transform.go:318-342`) call `init(publish=true, setContent=false)`.
- `init` (`transform.go:643-669`):
  - With transformations, it runs the chain once (sync.Once) and writes the result to the publish dir as part of `transform()` (`transform.go:577-583`).
  - Without transformations, `publishOnce` → `genericResource.Publish()` (`resource.go:503-545`) copies the source to every target filename.
- `.Content` (`transform.go:188-194`) calls `init(false, true)`. It **never publishes**.
- `.Data`, `.MediaType`, `.Name` and `.Key` call `init(false,false)`. No publishing.
- `RelPermalink = spec.GetBasePath(false) + paths.PathEscape(paths.TargetLink())` (`resource.go:546-548`). With baseURL `https://seeksnack.com/` the base path is `""`.
- `Permalink = BaseURL().WithPathNoTrailingSlash + paths.PathEscape(paths.TargetPath())` (`resource.go:550-552`) → `https://seeksnack.com/js/website.<hex>.js`.
- **Language independence:** for a single-host multilingual site, resources publish once to the publish-dir root and the Permalink does NOT include `/th/`. Verified: `th/index.html` references `https://seeksnack.com/js/website.77dd….js`. Only multihost adds a language prefix (`resource.go:456-458`).
- Published files for seeksnack (non-image): the 3 `js/*.js` files, 19 `images/favicon/*` copies, and `images/favicon/mstile-150x150.png` (already among the 19). Nothing under `scss/`, `ts/` or `vendor/`.

### 2.3 Resource cache (`resources/resource_cache.go`)
- A single `dynacache` memory cache is shared by **all sites/languages**. Each language gets its own `Spec`, but partitions are fetched by name from the shared `memCache` (`resource_spec.go:117`, `deps/deps.go:249-262`). So `Concat`, `Get` and transformations are global across EN and TH.
- `cleanKey(k) = TrimPrefix(path.Clean(ToLower(ToSlash(k))), "/")` (`resource_cache.go:74-76`).

---

## 3. The transformation-chain engine (`resources/transform.go`, 771 lines — port line by line)

### 3.1 Building chains
- `r.Transform(t)` / `TransformWithContext` (`transform.go:358-376`) returns a **copy** of the adapter with `transformations = append(old, t)`, a fresh `sync.Once` and a fresh `publishOnce`.
- So `resources.Get "a" | toCSS | postCSS | minify | fingerprint` is one adapter over the source resource with 4 queued transformations. Nothing runs until content or links are requested.

### 3.2 Keys (`transform.go:417-423`, `resources/internal/key.go`)
- `TransformationKey = cleanKey(target.Key()) + "_" + MD5hex( concat over t of ("_" + t.Key().Value()) )`.
- `Value() = Name` if there are no elements, else `Name + "_" + hashing.HashString(elements...)`. `HashString` is a gohugoio/hashstructure v0.5.0 xxhash64 printed as decimal (`common/hashing/hashing.go:81-131`).
- `genericResource.Key()` (`resource.go:447-466`) is the RelPermalink without basePath. For `scss/website.scss` it is `/scss/website.scss`.
- Verified with a Go program (`$W/keycalc`): the SCSS chain key string is `_tocss_7149566072694007861_postcss_3803688792395291579_minify_fingerprint_6275448751184701016`, giving `scss/website.scss_ce37005bb9b0d2e87a9f0d33876c2b52`. That is exactly the `resources/_gen/assets/scss/…` filename.
- Transformation key names:
  - `execute-as-template` + targetPath. **The data is NOT part of the key.**
  - `tocss` + `scss.Options` struct.
  - `postcss` + the options map (nil here).
  - `minify` with no elements.
  - `fingerprint` + algo (`"sha256"`).
  - `jsbuild` + the options map.
- Only the file cache names depend on these keys. The in-memory cache also uses them, but any stable key works there. Exact hashstructure parity is **optional**; see §10.

### 3.3 Running (`transform.go:438-637`)
1. Two pooled buffers b1 and b2. `tctx.From` = source ReadSeekCloser and `tctx.To` = b1. `InPath = target.TargetPath()` (leading `/`). `SourcePath = TrimPrefix(InPath,"/")`. `InMediaType = OutMediaType = source media type`.
2. For each transformation i:
   - For i>0, `InMediaType = OutMediaType`.
   - For i>0, if the previous `To` buffer has content: `counter++`, then alternate. Even counter: From=b2, reset b1, To=b1. Odd counter: From=b1, reset b2, To=b2. If the previous step wrote nothing (e.g. fingerprint over a seekable source), From/To are not swapped.
   - `mayBeCachedOnDisk = name ∈ {postcss, tocss, tocss-dart}`. `writeToFileCache` becomes true if any step is one of those.
   - Run `tr.Transform(tctx)`. If the error is `FeatureNotAvailable` (binary not found) and `useResourceCacheWhen == "fallback"` (the default), try the file cache instead: `tryTransformedFileCache(key)` reads `<key>.json` + `<key>.content`. Otherwise it is an error: `"<NAME>: failed to transform %q (%s): …"`.
   - If the step set `OutPath`: `InPath = OutPath; OutPath = ""`.
3. `updates.targetPath = final InPath`, `mediaType = OutMediaType`, `data = tctx.Data` (accumulated, e.g. `Integrity`).
4. Writers:
   - If `publish`, the publish file at targetPath.
   - If `writeToFileCache`, the cache `.json` (compact `json.Marshal` of `{"Target","MediaType","Data"}`) plus the `.content` writer.
   - If `setContent || !writeToFileCache`, an in-memory string.
5. Final content = `To` buffer if non-empty, else the original source reader (seeked back to 0).

### 3.4 OutPath helpers (`transform.go:141-172`)
- `ReplaceOutPathExtension(".css")`: `dir + base(without last ext) + newExt`, from **InPath**.
- `AddOutPathIdentifier(id)`: `dir + base + id + ext`, where `ext` is the last extension (`paths.PathAndExt`, `common/paths/path.go:137-200`).

### 3.5 Resulting target paths in seeksnack

| Chain | Final target path | Published? |
|---|---|---|
| `scss/website.scss` → toCSS → `scss/website.css` → postCSS (unchanged) → minify `scss/website.min.css` → fingerprint `scss/website.min.186be38d….css` | `/scss/website.min.186be38d09dc13506b8cedff2d4a7af52dc9c20dce9752e3fc6cd86a865b81e4.css` (see the `resources/_gen/…json` "Target") | **No** (only `.Content` through PostProcess) |
| `ts/themeset.ts` → jsbuild(targetPath `js/set-theme.js`) → fingerprint | `js/set-theme.<sha256>.js` | Yes |
| `ts/themeswitch.ts` → jsbuild → `ts/themeswitch.js` | — | No (read by Concat) |
| `ts/search.ts` → execute-as-template (OutPath `ts/search.ts`) → jsbuild → `ts/search.js` | — | No |
| Concat `js/website.js` / `js/all.js` → jsbuild(targetPath `js/website.js`) → fingerprint | `js/website.<sha256>.js` | Yes |

---

## 4. Transformers

### 4.1 `resources.ExecuteAsTemplate` (`resources/resource_transformers/templates/execute_as_template.go`, 75 lines)
- Args: `(targetPath, data, resource)`. targetPath goes through `paths.ToSlashTrimLeading`.
- Transform: reads the whole input as a string, `TextParse(ctx.InPath, str)` (text/template, not html/template), sets `OutPath = targetPath`, and executes with `data` into `To`. The media type is unchanged (still `text/typescript` for `.ts`).
- **The key ignores `data`** (`execute_as_template.go:53-55`). Two calls with different data and the same target path share the cached result. It is harmless here because production always passes Apipro.
- In seeksnack the only actions are `{{ .api }}` (search.ts:11, comment.ts:47) → `https://api.seeksnack.com`.

### 4.2 `resources.Concat` (`resources/resource_factories/bundler/bundler.go`, 171 lines)
- `targetPath = path.Clean(targetPath)`, then `ResourceCache.GetOrCreate(targetPath, …)`. **The key is only the target path. The first caller wins, even if another call site passes a different slice.** This is the race in §0.
- All parts must share `MediaType().Type`, else error. `resolvedm` (the loop variable, ending as the last part's type) is used only for the separator decision below. The composite's own media type comes from its **target path extension** (`ResourceSourceDescriptor.init`, `resource.go:160-185`: `.js` → `text/javascript`). js.Build outputs are `text/javascript` (`media.Builtin.JavascriptType`), and `.js` files from `resources.Get` are too.
- Content (lazy): each part's `ReadSeekCloser()`, which runs each part's transformation chain in memory. **For JavaScript (main/sub type == text/javascript) it inserts `"\n;\n"` between parts** (`bundler.go:139-153`). No trailing separator. Other types are plain concatenation.
- Composite resource: `LazyPublish: true`, TargetPath = targetPath. It is never published in seeksnack.

### 4.3 `js.Build` (esbuild v0.25.6, embedded Go API)
Go files: `tpl/js/js.go:55-78`, `resources/resource_transformers/js/{transform.go 68, build.go 82}`, `internal/js/esbuild/{options.go 411, build.go 236, resolve.go 323, sourcemap.go 80, helpers.go 15}`. `batch.go` (1444 lines) is unused by seeksnack.

**Option decoding** (`options.go:74-91`):
- `mapstructure.WeakDecode` into `ExternalOptions`, with default `SourcesContent: true`.
- `TargetPath` goes through `ToSlashTrimLeading`. `Target` and `Format` are lower-cased.
- Keys are matched case-insensitively (mapstructure).

**Transform** (`js/transform.go:36-68`):
- `OutMediaType = text/javascript`.
- `OutPath = TargetPath`, or else `ReplaceOutPathExtension(".js")`.
- `SourceDir = dir(SourcePath)`, `Contents = input`, `MediaType = InMediaType`, `Stdin = true`.

**BuildClient.Build** (`esbuild/build.go:49-97`):
- `OutDir = AbsPublishDir`
- `ResolveDir = AbsWorkingDir = WorkingDir` (the site root)
- `TsConfig = ResolveJSConfigFile("tsconfig.json")`. Seeksnack has none, so this is `""`.

**Compiled `api.BuildOptions`** (`options.go:220-384`):

| Field | Value (seeksnack) | Notes |
|---|---|---|
| Bundle | true | always |
| Format | IIFE | `""`/`"iife"`→IIFE, `"esm"`, `"cjs"` |
| Platform | browser | default |
| Target | `es2015` for level-1 builds; **ESNext** for `{targetPath, minify}` builds | map at `options.go:34-49`; `""`→ESNext |
| MinifyWhitespace/Identifiers/Syntax | all = `minify` | |
| Sourcemap | None | `""`/`"none"` |
| SourcesContent | Include | irrelevant without maps |
| JSX | Transform | |
| Loader (map) | nil | |
| Stdin | `{Contents, ResolveDir: <site>, Loader}` | Loader by input media subtype: javascript→JS, typescript→TS, tsx→TSX, jsx→JSX (`options.go:245-258`) |
| Outdir | AbsPublishDir | output file = `<outdir>/stdin.js`; content not affected |
| AbsWorkingDir | `<site>` | **affects output**: relative "pretty paths" such as `"node_modules/mustache/mustache.js"` become `__commonJS` keys, and these survive minification in the golden files |
| Tsconfig | "" | esbuild still auto-discovers `tsconfig.json`/`package.json` walking up from the source dirs |
| Define/External/Drop/Inject/JSXFactory… | unset | |
| Write | false (Go API default) | |
| Charset, LegalComments, TreeShaking | esbuild defaults (ASCII, `eof` when bundling) | legal comments from jquery/bootstrap/mustache/is-buffer appear at EOF |
| Plugins | `hugo-import-resolver`, `hugo-params-plugin` (`resolve.go:157-323`) | see below |

**Hugo resolve plugin** (`resolve.go:160-273`):
- `OnResolve{Filter:".*"}` for every import:
  1. Apply shims.
  2. Handle externals.
  3. Compute relDir. For the stdin importer it is `opts.SourceDir` (e.g. `ts`). Otherwise it is `MakePathRelative(importer)` in the assets fs. If the importer is not in any assets mount → return `{}` so esbuild resolves it natively. **Note: node_modules IS in the assets fs (vendor mount), so importers inside node_modules get relDir `vendor/...`.**
  4. If the import starts with `.`, join it with relDir.
  5. `resolveComponent(impPath)` in the assets fs: try `impPath + {.js,.ts,.tsx,.jsx}` (skipping the extension already present), then `index.esm*`, then the path as-is (a directory → `index`/`index.esm`), then strip `.js` (`resolve.go:67-119`).
  6. If found → `{Path: absFilename, Namespace: "ns-hugo-imp"}`, and record `JSConfigBuilder.AddSourceRoot` (this causes `assets/jsconfig.json` to be written in postProcess).
  7. If not found → `{}` (esbuild native).
- `OnLoad{ns-hugo-imp}`: reads the file with `os.ReadFile` and returns `{Contents, ResolveDir: <site>, Loader: by extension (.js/.mjs/.cjs→JS, .jsx, .ts, .tsx, .css, .json, .txt; default JS)}`.
- Params plugin: `^@params(/config)?$` → namespace `ns-hugo-params`, JSON contents `{}` by default.

**Seeksnack import behaviour:** the only import is `require("mustache")` in search.ts:108. The Hugo resolver tries `mustache.js|.ts|.tsx|.jsx` and `mustache` at the assets root, finds nothing, and falls through. esbuild resolves `<site>/node_modules/mustache/mustache.js` (the `exports.require` condition) in the default file namespace. **So the Hugo plugins do not change seeksnack's output.** Proven: `repro.js` with and without plugin emulation gives identical level-1 outputs, and the plain CLI reproduces all golden files.

**Exact CLI reproduction (run with cwd = site root; esbuild 0.25.6 native binary):**
```
# level 1 (js.Build (dict "target" "es2015")):
esbuild --bundle --format=iife --platform=browser --target=es2015 --loader=ts < assets/ts/themeswitch.ts
sed 's#{{ .api }}#https://api.seeksnack.com#g' assets/ts/search.ts  | esbuild --bundle --format=iife --platform=browser --target=es2015 --loader=ts
sed 's#{{ .api }}#https://api.seeksnack.com#g' assets/ts/comment.ts | esbuild … (same)
# concat with "\n;\n" (see 4.2), then level 2 (js.Build (dict "targetPath" "js/website.js" "minify" true)):
esbuild --bundle --format=iife --platform=browser --minify --loader=js < concat.js
# set-theme:
esbuild --bundle --format=iife --platform=browser --minify --loader=ts < assets/ts/themeset.ts
```
Level-1 sizes: themeswitch 1344, search 22704 (it contains the mustache UMD wrapped in `__commonJS({"node_modules/mustache/mustache.js"(exports, module){…}})` and a `// <stdin>` comment), comment 7855. Concat sizes: website 174817, all 118428.

**Why esbuild outputs are location-independent here:** only site-relative paths appear. I built from `$W/site`, a different absolute path, and still matched. **For sites with Hugo-resolved imports** (namespace `ns-hugo-imp:`), non-minified output would contain absolute paths. That is not the case for seeksnack.

### 4.4 `toCSS` → LibSass (`tpl/css/css.go:83-144`; `resources/resource_transformers/tocss/scss/{client.go 93, client_extended.go 56, tocss.go 214}`; `tocss/sass/helpers.go 102`)

**Option decoding:**
- `toCSS $cssOpts` → `css.Sass` with transpiler default `libsass`, then `scss.DecodeOptions(m)` (mapstructure WeakDecode):

  `Options{TargetPath:"", IncludePaths:["node_modules","assets/scss"], OutputStyle:"compressed", Precision:0, EnableSourceMap:false, Vars:nil}`.
- `ToCSS` (`client_extended.go:31-47`):
  - `to.Precision = opts.Precision`. **If it is 0, it becomes 8** (bootstrap-sass requirement).
  - `to.OutputStyle = libsass.ParseOutputStyle("compressed")` = 3 (`nested`=0, `expanded`=1, `compact`=2, `compressed`=3; case-insensitive; unknown values → nested).

**Transform** (`tocss.go:39-181`):
- `OutMediaType = text/css`. `OutPath = TargetPath`, or else `ReplaceOutPathExtension(".css")` → `scss/website.css`.
- `baseDir = path.Dir(SourcePath)` = `scss`.
- Include paths:
  - `IncludePaths = sfs.RealDirs(baseDir)`: for each assets mount **directory**, `join(mountRealDir, "scss")` if it exists. That gives `[<site>/assets/scss]`; `<site>/node_modules/scss` does not exist (`basefs.go:440-452`).
  - Then each user include path is `workFs.Stat(filepath.Clean(ip))` relative to the working dir, and its real filename is appended if it exists.
  - Final list: `[<site>/assets/scss, <site>/node_modules, <site>/assets/scss]`, joined with `:` (`os.PathListSeparator`).
- `SassSyntax` is true only for `.sass` input.
- Source maps are off: no filename/root/output path. golibsass still always calls `SetSourceMapContents(false)`, `SetOmitSourceMapURL(false)`, `SetSourceMapEmbed(false)` and `SetSourceComments(false)` (`$GOMODCACHE/github.com/bep/golibsass@v1.2.0/libsass/transpiler.go:27-91`).
- `replaceRegularImportsIn`/`Out` (`client.go:81-93`) protect `@import "x.css";` lines in the **entry file only**:
  - In: regex `.*(@import "(.*\.css)";).*` → `/* HUGO_IMPORT_START $2 HUGO_IMPORT_END */`.
  - Out: regex `.*(\/\* HUGO_IMPORT_START (.*) HUGO_IMPORT_END \*\/).*` → `@import "$2";`.
  - website.scss has no such lines, but port it anyway.
- Errors: a libsass error on `"stdin"` is remapped to the real filename.

**Import resolver**, called for every `@import` (`tocss.go:70-128`; bridged by golibsass `internal/libsass/a__importer.go` + `a__libsass.go:BridgeImport`):
- The C side calls `SassImport(currPath, importerEntry, compiler)`. It gets `prev = sass_import_get_imp_path(sass_compiler_get_last_import(comp))` and calls the Go resolver with `(url=currPath, prev)`.
- Resolver:
  1. If `url == "hugo:vars"` → return the vars stylesheet (not used here).
  2. `urlDir = filepath.Dir(url)` (`"."` if there is no slash).
  3. `prevDir = baseDir` if `prev == "stdin"`. Otherwise `prevDir = sfs.MakePathRelative(filepath.Dir(prev), checkExists=true)`. `MakePathRelative` is a reverse lookup of a real path into the assets component (`basefs.go:351-385`, `hugofs/rootmapping_fs.go:344-386`). For example, `<site>/node_modules/bootstrap/scss` → `vendor/bootstrap/scss`. If it is not found (e.g. `prev` is a relative URL) → **return unresolved**.
  4. `basePath = join(prevDir, urlDir)`, `name = base(url)`.
  5. Patterns:
     - name contains `.`: `["_%s","%s"]`
     - name starts with `_`: `["_%s.scss","_%s.sass"]`
     - otherwise: `["_%s.scss","%s.scss","_%s.sass","%s.sass","%s/_index.scss","%s/_index.sass","%s/index.scss","%s/index.sass"]`

     Then strip the leading `_` from name.
  6. For each pattern, `assetsFs.Stat(join(basePath, fmt(pattern,name)))`. The first hit returns `(realAbsFilename, body="", true)`.
  7. Otherwise unresolved.
- Resolved (golibsass `BridgeImport`) → `sass_make_import_entry(C.CString(realPath), NULL, NULL)`. libsass `call_loader` → `import_url(imp, abs_path, ctx_path)` (`libsass_src/src/context.cpp:407-482`).
- Unresolved → `sass_make_import_entry(currPath, NULL, NULL)` (the original URL). libsass then resolves it itself relative to the importing file and the include paths.
- Entry context: `sass_make_data_context(C.CString(src))`. The main file's path is `"stdin"`.

**Compiled C/C++:**
- `$GOMODCACHE/github.com/bep/golibsass@v1.2.0` embeds **LibSass v3.6.6** (README badge; `libsass_src` is a git subtree of the upstream tag). `LIBSASS_VERSION` is not defined, so it is `"[NA]"` (`include/sass/version.h`).
- cgo flags (`internal/libsass/a__cgo.go`): `CFLAGS: -O2 -fPIC`; `CPPFLAGS: -I../../libsass_src/include`; `CXXFLAGS: -g -std=c++0x -O2 -fPIC`; `LDFLAGS: -lstdc++ -lm` (+ `-ldl` on darwin/linux). No other defines; `USE_LIBSASS_SRC` is only for the `dev` tag.
- Compilation units: 62 `internal/libsass/*.cpp`, each `#include "../../libsass_src/src/<X>.cpp"`, plus `c99func.c` and `cencode.c`. **`src/memory/allocator.cpp` and `src/memory/shared_ptr.cpp` are NOT compiled.** The allocator is guarded by `SASS_CUSTOM_ALLOCATOR`. The unit list is in `$W/golibsass_units.txt`.
- **Proof:** I compiled exactly those units with Apple clang 21 (`clang++ -g -std=c++0x -O2 -fPIC -I libsass_src/include`; C: `clang -g -O2 -fPIC`) and linked a C++ driver that implements the resolver above (`$W/libsass/driver.cpp`). The output is byte-identical to neohugo's toCSS output (315116 bytes; captured by publishing `toCSS` to `debug/tocss.css` in my site clone).
- For seeksnack, the output was ALSO identical with the custom importer disabled (`driver_noimp`). Seeksnack does not depend on the importer, but port it for generality.

### 4.5 `postCSS` (`tpl/css/css.go:55-66`; `resources/resource_transformers/cssjs/postcss.go`, 239 lines; `common/hexec/exec.go`, 389 lines; `common/neohugo/neohugo.go:201-230`)
- Options: nil map → `PostCSSOptions{}`. **No `--no-map`.** postcss-cli does not emit a source map when writing to stdout; verified, the output ends with `.wa{width:auto}`.
- Config file:
  - `options.Config` or `"postcss.config.js"` → `filepath.Clean`.
  - If it is not absolute, `ResolveJSConfigFile` (`basefs.go:210-224`) looks for `assets/_jsconfig/postcss.config.js` (auto-mounted from the project root, `modules/collect.go:597-637`, regex `(babel|postcss|tailwind)\.config\.js` plus `package.json`/`package.hugo.json`). Its fallback is `workFs.Stat(name)`.
  - The result is `<site>/postcss.config.js`.
- Args: `["--config", "<site>/postcss.config.js"]`, plus `--no-map`, `--use …`, `--parser`, `--stringifier` and `--syntax` only when set (`postcss.go:108-127`).
- **Binary lookup** (`hexec.Npx`, `exec.go:177-232`), cached per name:
  1. `<WorkingDir>/node_modules/.bin/postcss` (exec.LookPath on it).
  2. `npx --no-install postcss …` if `npx` is on PATH.
  3. `postcss` on PATH.

  Nothing found → `NotFoundError` → `FeatureNotAvailableError` → file-cache fallback. The security allowlist (`config/security/securityConfig.go:35-56`) must accept the name; `^postcss$` and `^npx$` are allowed.
- **Env:**
  - Start with os.Environ filtered by the whitelist `(?i)^((HTTPS?|NO)_PROXY|PATH(EXT)?|APPDATA|TE?MP|TERM|GO\w+|(XDG_CONFIG_)?HOME|USERPROFILE|SSH_AUTH_SOCK|DISPLAY|LANG|SYSTEMDRIVE)$`. **NODE_ENV, BROWSERSLIST*, NODE_OPTIONS etc. are DROPPED.** This matters for determinism.
  - Then set or override:
    - `NODE_PATH=<site>/node_modules`, or `<site>:<$NODE_PATH>` if NODE_PATH is set
    - `PWD=<site>`
    - `HUGO_ENVIRONMENT=production`
    - `HUGO_ENV=production`
    - `HUGO_PUBLISHDIR=filepath.Join(<site>, publishDir)`. With `-d /abs/out` this becomes `<site>/abs/out`, a quirk that is unused here.
    - `HUGO_FILE_<NAME_UPPER_DOTS_TO_UNDERSCORE>=<realpath>` for each file in `assets/_jsconfig`: `HUGO_FILE_PACKAGE_JSON` and `HUGO_FILE_POSTCSS_CONFIG_JS`.
- **cwd: NOT set.** It inherits neohugo's process cwd. The golden build runs from the site dir.
  - postcss-cli uses `from = <cwd>/stdin`, and browserslist finds `.browserslistrc` from there.
  - **purgecss `content: ["./hugo_stats.json"]` is relative to process.cwd().** Running from another dir (`-s`) would break parity or fail.
- stdin: a goroutine copies `ctx.From` (the libsass output) to the child's stdin, then closes it. stdout goes to `ctx.To`. stderr goes to the info logger and an error buffer.
- Errors:
  - A non-zero exit becomes `imp.toFileError(stderr)`.
  - A "not found" pattern (`(?s)not found:|could not determine executable`) in npx output becomes a NotFoundError.
  - `postcss-fail-on-warn` in the site config turns any postcss warning into a failure. The `postcss.plugin was deprecated` message is `console.warn`, not a postcss warning.
- `InlineImports` (`cssjs/inline_imports.go`, 247 lines) is off. Porting it is optional.
- Site config effect (`postcss.config.js`): autoprefixer (from `.browserslistrc`) → cssnano default preset with `discardComments.removeAll` → a custom plugin that strips `!important` → **purgecss only when `HUGO_ENVIRONMENT === "production"`**, with `defaultExtractor` = `JSON.parse(content).htmlElements.{tags,classes,ids}`, a safelist and a blocklist.
- Verified: libsass output (315116 B) → postcss (52035 B), run with exactly these args/env and cwd = site, using the golden `hugo_stats.json`.

### 4.6 `minify` as a resource transform (`resources/resource_transformers/minifier/minify.go`, 59 lines; `minifiers/{minifiers.go 131, config.go 134}`)
- `AddOutPathIdentifier(".min")`, then `minifiers.Client.Minify(InMediaType="text/css", To, From)`, which is tdewolff `m.Minify`.
- The `minify.M` is built exactly like HTML output minification (`minifiers.New`, `minifiers.go:56-83`). It registers the css, js (+ regexp), json (+ regexp), **svg**, xml and html minifiers from the site config. Defaults: CSS `{Precision:0, KeepCSS2:true}`; SVG `{KeepComments:false, Precision:0}`; JS `{Version:2022}`. `[minify.tdewolff.html]` in hugo.toml overrides html only.
- **PARITY TRAP (verified):**
  - The CSS minifier minifies `url(data:…)` values with `minify.DataURI` (`$GOMODCACHE/github.com/tdewolff/minify/v2@v2.23.8/common.go:55-95`). That minifies the embedded data with the minifier registered for its mediatype (`image/svg+xml`) and keeps the ORIGINAL if it is shorter.
  - With the SVG minifier registered (as in Hugo), the bootstrap `.sb-links .btn:before{content:url("data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' …")}` stays unchanged, with raw spaces.
  - With only the CSS minifier registered, the output re-encodes spaces as `%20` (51917 B instead of 51857 B, mismatch).
  - **The Rust CSS minifier must support DataURI minification with a registered SVG minifier.**
- Verified: postcss output → `$W/gomin/gomin text/css` (same registrations as Hugo) → 51857 B, identical to golden.

### 4.7 `fingerprint` (`resources/resource_transformers/integrity/integrity.go`, 124 lines)
- Algo `sha256` by default. `md5`, `sha384` and `sha512` are also supported.
- If `From` is an `io.ReadSeeker` (only when fingerprint is the first step over a file source), it only hashes and seeks back. Otherwise it tees `From` → `To`. **Content is unchanged.**
- `Data["Integrity"] = algo + "-" + base64.StdEncoding(digest)` (padded). `AddOutPathIdentifier("." + hex(digest))` (lowercase hex).
- Examples: `js/set-theme.js` → `js/set-theme.5307ae76….js`; `scss/website.min.css` → `scss/website.min.186be38d….css` with Integrity `sha256-GGvjjQncE1BrjO3/LUp69S3Jwg3Ol1Lj/GzYaoZbgeQ=`.
- Seeksnack never uses `.Data.Integrity`.

### 4.8 `resources.PostProcess` (`tpl/resources/resources.go:320-323`; `resources/post_publish.go` 51; `resources/postpub/{postpub.go 182, fields.go 59}`; `hugolib/hugo_sites_build.go:599-716`; `deps/deps.go:196-218,476-494`; `hugofs/hasbytes_fs.go` 102)
- `PostProcess(r)`: `key = r.TransformationKey()`. This does NOT run the chain; it only uses the source key plus the transformation keys. The result is memoized in the global `PostProcessResources[key]`. New entries get `id = BuildState.Incr()` (1 for seeksnack; the only PostProcess call site) and return a `PostPublishResource{prefix: "__h_pp_l1_" + id + "_"}`.
- Field accessors return placeholders `prefix + Field + "__e="`:
  - `.Content` → `__h_pp_l1_1_Content__e=`
  - `.RelPermalink`, `.Permalink`, `.Name`, `.Title`, `.ResourceType`, `.MediaType.X`, `.Data.Integrity`
  - `.Params` panics
- The template therefore emits `<style>\n      __h_pp_l1_1_Content__e=\n    </style>`. The page publisher applies absURL then the tdewolff HTML minifier, which yields `<style>__h_pp_l1_1_Content__e=</style>` (verified with the minifier).
- Detection: the publish fs is wrapped with `hasBytesFs`. For files whose extension is a text media type suffix, it scans all written bytes for `__h_pp_l1` (and `__hdeferred/`). On Close, the filename is recorded in `BuildState.filenamesWithPostPrefix`.
- `postProcess` (after render + writeBuildStats + renderDeferred):
  - For each recorded file (sorted, in a parallel worker pool): read the whole file. Loop: `l = Index(content[k:], "__h_pp_l1")`, `m = Index(content[k+l:], "__e=") + 4`, field = `content[k+l : k+l+m]`. For each PostProcessResource, `GetFieldString(field)` matches by `strings.Index(field, r.prefix)`. Replace the field bytes with the value, then continue from after the inserted value. Write the file back if it changed.
  - `GetFieldString("…Content__e=")` → `delegate.Content()` → runs the whole toCSS → postCSS → minify → fingerprint chain once (sync.Once plus the transformation cache) with `publish=false`. **Nothing is written to /public**; the cache files are written to `resources/_gen`.
  - The inserted CSS is **raw**: no HTML minification and no absURL rewriting.
- Evidence: all 1940 `<style>` blocks in golden (excluding the static `admin/index.html`) equal the 51857-byte final CSS, which equals `resources/_gen/assets/scss/website.scss_ce37005bb9b0d2e87a9f0d33876c2b52.content`.

---

## 5. Build-phase ordering (`hugolib/hugo_sites_build.go:119-196`)
1. `process` + `assemble`
2. **`render`**: all sites (EN then TH), pages rendered concurrently per site. The publisher writes minified HTML, the collector records elements, and resources with `.Permalink` are published (JS, favicons, images).
3. **`writeBuildStats`** (`:719-775`) → `<WorkingDir>/hugo_stats.json`
4. `printPathWarningsOnce`, `renderDeferred` (none in seeksnack), `printUnusedTemplatesOnce`
5. **`postProcess`** (`:599-716`):
   - (a) If `!build.noJSConfigInAssets` and the JSConfigBuilder has source roots, write `<assets>/jsconfig.json`. For seeksnack there are no Hugo-resolved imports, so it is **not written** (verified: `assets/` has only images, scss, ts).
   - (b) Placeholder replacement (§4.8), **which is where postCSS runs and reads the hugo_stats.json from step 3.**

The Rust port must keep 2 → 3 → 5 strictly ordered. The CSS chain must not run before the stats file is complete.

---

## 6. `hugo_stats.json` (`publisher/htmlElementsCollector.go` 556, `publisher/publisher.go` 190, `hugolib/hugo_sites_build.go:719-775`, `helpers/general.go:62-118`)

### 6.1 Enablement and config
`[build] writeStats = true` is a legacy bool, converted to `BuildStats{Enable:true}` (`config/commonConfig.go:185-190`). `Enabled()` = Enable && any of tags/classes/ids is not disabled.

### 6.2 What the collector sees
In `DestinationPublisher.Publish` (`publisher.go:94-133`), the collector is attached with `io.MultiWriter(file, collectorWriter)` **after** the transformer chain (absURL/canonify → [livereload] → [generator tag] → minify). **It sees exactly the bytes written to disk, i.e. post-canonify and post-minify HTML.** The PostProcess placeholders are still in place; they sit inside `<style>`, which the collector skips.
- The source is a `*bytes.Buffer`, so `io.Copy` uses `WriteTo` → a **single `Write` call with the whole document**.
- Only `OutputFormat.IsHTML` outputs are collected: HTML pages, 404, and alias redirect pages (`hugolib/alias.go:90-116`, published with the page's HTML format). Static files are not collected.
- One collector per site (publisher). Merged in `writeBuildStats`: `Merge` appends + `UniqueStringsReuse`, then `Sort()` = `sort.Strings` (byte order).

### 6.3 Scanner state machine (port verbatim)
- `Write(p)`: decode runes (`utf8.DecodeRune`). **Stop processing the rest of this Write at the first `utf8.RuneError`**, which covers invalid UTF-8 AND a literal U+FFFD. The golden has no such bytes (checked).
- `htmlLexStart`: on `<` → backup, reset the buffer → `htmlLexElementStart`.
- `htmlLexElementStart`: append runes until `>` or `unicode.IsSpace`.
  - If the buffer is shorter than 2 bytes or starts with `</` → reset → start.
  - `tagName = buff[1:]`, `selfClosing = last=='/'`.
  - If not self-closing and `(?i)^(pre|textarea|script|style)` matches (**prefix match**, e.g. `<prefix>` would match too): backup, then `lexElementInside` collects the start tag and resolves to `consumeBuffUntil(r=='>' && isClosedByTag(buff, tagName))` → start.
  - Else if `(?i)^!DOCTYPE` matches: reset, and consume (no backup) until `>`.
  - Else: backup → `lexElementInside(start)`.
  - A buffer equal to `<!--` when the current rune is `-` → comment state, until the buffer ends with `-->`.
- `lexElementInside`: append runes. Quote tracking: `'` or `"` toggles only when it equals the open quote or no quote is open. On `>` outside quotes: `s = buff`; skip if already in `elementSet`; else `parseHTMLElement(s)` and append. Reset the buffer.
- `isClosedByTag` scans backwards for `</ name >`, allowing spaces (`' '`, `\t`, `\n`), with a case-insensitive name compare.
- `parseHTMLElement`:
  - `tag = ToLower(parseStartTag(s))`. `parseStartTag` takes the substring after `<` up to the first `unicode.IsSpace` (or `len-1`), minus a trailing `/`.
  - If the tag is in `{thead,tbody,tfoot,td,tr}`, the first occurrence of the tag name in the string is replaced with `div`.
  - Then **`html.Parse` (golang.org/x/net/html, a full HTML5 tree builder) of the element string**. Walk for `ElementNode` with `Data == tagNameToParse`:
    - `id` (EqualFold) → IDs.
    - Key matching `(?i)^class$|transition` → `strings.Fields(val)` → classes.
    - Keys containing `:class` get the Vue/Alpine handling (`htmlJsonFixer`, `jsonAttrRe`, `extractSingleQuotedStrings`); unused in seeksnack.
  - **Tree-builder quirk:** `<th …>`, `<caption>`, `<col>`, `<colgroup>`, `<frame>` start tags are ignored in "in body" mode. Their classes/ids are NOT collected, though the tag name still is. `<image>` becomes `img`, so its attributes are not collected. Seeksnack's 3165 `<th>` carry only `scope`, so there is no impact, but port it faithfully.
- `getHTMLElements`: flatten, then `UniqueStringsSorted` (`sort.Strings` + dedupe; empty → nil).

### 6.4 Output
- `publisher.PublishStats{HTMLElements{Tags,Classes,IDs}}` with JSON keys `htmlElements`/`tags`/`classes`/`ids`, in that order.
- `json.NewEncoder` + `SetEscapeHTML(false)` + `SetIndent("", "  ")`. `Encode` appends `"\n"`.
- A nil slice encodes as `null`.
- Go also escapes U+2028/U+2029, and invalid UTF-8 becomes `�`.
- Written to `filepath.Join(WorkingDir, "hugo_stats.json")` on the OS fs, skipped if the bytes are identical.
- Seeksnack: 53 tags, 241 classes, 260 ids (many Thai ids), 19674 bytes.
- Verified by `$W/stats_repro.py`: re-implementing the state machine over the 3422 publisher-written HTML files of `golden/run1` (static html excluded) reproduces the file **byte-for-byte**. It uses a simple HTML5 attribute tokenizer instead of the tree builder; that is sufficient for seeksnack.

---

## 7. Golden evidence (from `$SP/golden/run1`)

| File | Bytes | Referenced by |
|---|---|---|
| `js/set-theme.5307ae767e2dcacda90028e430626dc651d5ea183b8e79abaf08ff2c4749aedd.js` | 179 | 1940 HTML pages |
| `js/website.77dd0c333828a30bcf7187d8c9e12a8ec50dc8122454c57e36665bfd33a19f0d.js` | 161160 | 239 pages (home EN/TH, sections, taxonomies, 404, simple) |
| `js/website.8c83f39f4e3d8816a1f4d21044ced9536baf9141758ad87ae00ab63a5bf76a8f.js` | 100316 | 1701 pages (single + term, e.g. `brands/pringles/`, `search/`) |
| `css/seeksnack.css` | 27153 | static copy of `static/css/seeksnack.css`, not the pipeline |
| inline `<style>` | 51857 | 1940 pages; sha256 `186be38d…81e4` |

`resources/_gen/assets/scss/website.scss_ce37005bb9b0d2e87a9f0d33876c2b52.json` = `{"Target":"/scss/website.min.186be38d09dc13506b8cedff2d4a7af52dc9c20dce9752e3fc6cd86a865b81e4.css","MediaType":"text/css","Data":{"Integrity":"sha256-GGvjjQncE1BrjO3/LUp69S3Jwg3Ol1Lj/GzYaoZbgeQ="}}`

JS head/tail characteristics:
- Output starts with `(()=>{var …=(K,Oe)=>()=>(Oe||K((Oe={exports:{}}).exports,Oe),Oe.exports);…`. The concatenated UMD code references `module`/`exports`, so esbuild wraps the stdin entry in `__commonJS`.
- Output ends with `…js();})();\n` followed by EOF legal comments: the jQuery `/*! … */`, the Bootstrap `/*!…*/`, `/*! Bundled license information:\n\nmustache/mustache.js:\n  (*!…*)\n*/`, and for all.js the is-buffer `/*!…*/`.

---

## 8. `resources/_gen` caching (`cache/filecache/*`, `resources/resource_cache.go:100-149`)
- Default caches (`filecache_config.go:61-86`):
  - `assets` and `images`: `:resourceDir/_gen` (= `<site>/resources/_gen`), maxAge -1
  - `getresource`: `:cacheDir/:project` (= `~/Library/Caches/hugo_cache/<basename(site)>/filecache/getresource`), maxAge -1
- For chains containing `tocss`/`postcss`/`tocss-dart`, the final content and meta are ALWAYS written to `<site>/resources/_gen/assets/<TransformationKey>.content` / `.json` (e.g. `assets/scss/website.scss_ce37….{content,json}`). They are **read only** when `useResourceCacheWhen` is `always`, or when it is `fallback` (default) and the tool is missing (`FeatureNotAvailable`) (`commonConfig.go:137-147`).
- It does not affect `/public` bytes when node/postcss are present. For parity in environments without node, the Rust port must compute the identical key (hashstructure port) and read these files. This is optional.

---

## 9. Rust port recommendations

### 9.1 Port line by line from Go

| Area | Go files (lines) |
|---|---|
| Chain engine, adapter, lazy publish, links | `resources/transform.go` (771), `resources/resource.go` (733, the publish/links/key parts), `resources/resource_cache.go` (149), `resources/internal/{key.go 42, resourcepaths.go 107}` |
| Factories | `resources/resource_factories/create/create.go` (304: Get, Copy, FromString), `resources/resource_factories/bundler/bundler.go` (171) |
| Template funcs | `tpl/resources/resources.go` (330), `tpl/resources/init.go` (82), `tpl/css/css.go` (192), `tpl/js/js.go` (117), `tpl/internal/resourcehelpers/helpers.go` (69) |
| ExecuteAsTemplate | `resources/resource_transformers/templates/execute_as_template.go` (75) |
| Fingerprint | `resources/resource_transformers/integrity/integrity.go` (124) |
| Minify transform | `resources/resource_transformers/minifier/minify.go` (59) + `minifiers/{minifiers.go 131, config.go 134}` (shared with the minify agent) |
| toCSS | `resources/resource_transformers/tocss/scss/{client.go 93, client_extended.go 56, tocss.go 214}`, `tocss/sass/helpers.go` (102), golibsass `libsass/transpiler.go` (160) + `internal/libsass/{a__importer.go 110, a__libsass.go 206 (BridgeImport)}` |
| postCSS | `resources/resource_transformers/cssjs/postcss.go` (239), optional `inline_imports.go` (247); `common/hexec/exec.go` (389); `common/neohugo/neohugo.go:201-230` (GetExecEnviron); `config/security/securityConfig.go:35-56` (allow/env whitelist) |
| js.Build | `resources/resource_transformers/js/{build.go 82, transform.go 68}`, `internal/js/esbuild/{options.go 411, build.go 236, resolve.go 323, sourcemap.go 80}` |
| PostProcess | `resources/post_publish.go` (51), `resources/postpub/{postpub.go 182, fields.go 59}`, `hugolib/hugo_sites_build.go:599-716`, `deps/deps.go:196-218,476-494`, `hugofs/hasbytes_fs.go` (102) + `common/hugio` HasBytesWriter |
| Build stats | `publisher/htmlElementsCollector.go` (556), `publisher/publisher.go` (190), `hugolib/hugo_sites_build.go:719-775`, `helpers/general.go:62-118` |
| Config | `config/commonConfig.go:98-205` (BuildConfig/BuildStats/UseResourceCache), `cache/filecache/filecache_config.go` (247) + `filecache.go` (499) |
| FS helpers | `hugolib/filesystems/basefs.go` RealDirs/MakePathRelative/ResolveJSConfigFile/RealFilename (`:210-224,351-452`), `hugofs/rootmapping_fs.go:344-386` |
| Optional cache-key parity | `common/hashing/hashing.go` (194) + `$GOMODCACHE/github.com/gohugoio/hashstructure@v0.5.0` |

### 9.2 External engines

| Engine | Recommendation | Why |
|---|---|---|
| **LibSass 3.6.6** | Vendor `golibsass@v1.2.0/libsass_src` and compile the **same 62 .cpp + 2 .c units** with the `cc` crate: `.cpp(true).flag("-std=c++0x").opt_level(2).pic(true)`, include `libsass_src/include`, no extra defines. Write thin FFI to `sass/context.h` plus a C-ABI importer callback implementing §4.4 (use `sass_compiler_get_last_import` + `sass_import_get_imp_path` for `prev`). Strings passed to `sass_make_data_context`/`sass_make_import_entry` must be `malloc`'d (libc::strdup), because libsass frees them. | Proven byte-identical with clang -O2. Avoid `-ffast-math`. Build with the same compiler family: on arm64, fp-contract/FMA could in theory change number formatting between optimization levels, so keep `-O2`. |
| **esbuild 0.25.6** | Run the pinned native esbuild binary. Get it from npm `@esbuild/<platform>@0.25.6`, or build it from `$GOMODCACHE/github.com/evanw/esbuild@v0.25.6/cmd/esbuild`. Set `current_dir = site`, stdin = contents, flags per §4.3 (`--bundle --format=iife --platform=browser [--target=es2015] [--minify] --loader=<ts|js>`). For full Hugo semantics (the assets resolver plugin, `@params`, shims, externals), implement esbuild's `--service=0.25.6` stdio protocol with onResolve/onLoad callbacks, or drive the npm JS API through node (see `$W/esbuild/repro.js`). | The CLI is proven byte-identical for seeksnack. The plugin only matters for imports that resolve inside `/assets`. |
| **PostCSS** | Spawn node postcss exactly as in §4.5: lookup order, args, filtered env, inherited cwd, stdin/stdout. | No Rust equivalent of autoprefixer + cssnano + purgecss gives identical bytes. |
| **tdewolff minify (CSS/SVG/HTML/JS)** | Port it (minify agent). It must include DataURI + the SVG-minifier interplay (§4.6). | |

### 9.3 Crates that are safe for byte-parity
- `sha2` (SHA-256), `md-5` (key MD5), `base64` (`STANDARD`, padded), `hex` (lowercase).
- `cc` (to build libsass). `libc` (strdup/free for the libsass FFI).
- `xxhash-rust` (xxh64), only if porting hashstructure keys.
- `serde_json` for `hugo_stats.json`, with `PrettyFormatter::with_indent(b"  ")` plus a trailing `\n` and a custom formatter that escapes U+2028/U+2029. Go 1.22+ escapes `\b \f \n \r \t` short and other controls as `\u00xx` (lowercase), like serde_json. Emit `null` for empty lists to mirror Go nil slices.
- `std::process::Command` for postcss/esbuild.
- `html5ever` (full tree builder), to emulate `x/net/html.Parse` per element string in the stats collector. It follows the same WHATWG algorithm; validate on the §6.3 quirks. A hand tokenizer is also enough for seeksnack.

### 9.4 Crates that are not safe for byte-parity
- `grass`, `rsass`, `dart-sass-embedded` (Dart Sass semantics): different number precision (10 vs 8), color and number serialization, and compressed formatting.
- `sass-rs`/`sass-sys`: they bundle a different libsass version or build flags; only use them if pinned to 3.6.6 with identical units and flags.
- `swc`, `oxc`, `rolldown`, `esbuild-rs` (old): different minifier renaming, helper names (`__commonJS` shape), and legal-comment placement.
- `lightningcss`/`parcel_css` as a stand-in for postcss/cssnano/purgecss or for tdewolff: different output.
- `minify-html`/`css-minify`: different algorithms.

---

## 10. Parity risks (ranked)
1. **Concat first-writer-wins race** (`bundler.go:84-85`). `"js/all.js"` has two call sites with different orders (single.html vs term.html). The golden uses the single.html order (bundle ends with comment.ts code). A parallel Rust renderer that lets a term page execute first produces `website.cda9b0b0….js` and changes 1701 HTML files. Mitigation: make Concat creation deterministic, e.g. "first page in Go's render order wins" (EN before TH; page-tree path order puts `/almonds/*` single pages before `/brands/*` terms), or serialize the first execution of each Concat key in page order.
2. **hugo_stats.json must be byte- and set-identical, and written before the CSS chain runs.** purgecss output depends on the exact class/id/tag sets. It must be computed on the final minified + canonified HTML of all languages, including aliases and 404, and excluding static files.
3. **CSS DataURI + SVG minifier interplay** in the resource `minify` step (§4.6). The Rust tdewolff port must register SVG and minify data-URI payloads.
4. **The placeholder must be substituted after HTML minify/absURL, with raw content** (§4.8). Do not re-minify the inlined CSS.
5. **postcss process environment:** env filtering, `HUGO_ENVIRONMENT=production` (purgecss on), cwd = site dir (`./hugo_stats.json`, `.browserslistrc`), `--config <abs>/postcss.config.js`, no `--no-map`. The node_modules versions (caniuse-lite, cssnano, etc.) must be the same install.
6. **esbuild version and flags:** exactly 0.25.6; target `es2015` for level-1 builds and **esnext** for the minified builds; IIFE; browser; `cwd`/absWorkingDir = site (the `"node_modules/mustache/mustache.js"` key string is embedded in the final bundles); stdin loader from the media type (ts vs js). Parent-directory `tsconfig.json`/`package.json` files are auto-discovered by esbuild.
7. **LibSass build:** same sources (3.6.6), same units, `-O2`, precision 8, compressed, include-path order `[assets/scss, node_modules, assets/scss]`, and the importer semantics.
8. **ExecuteAsTemplate key ignores data.** Cached by target path only; replicate this.
9. **Publishing semantics:** only `.Permalink`/`.RelPermalink` publish, never `.Content`. Intermediate js.Build outputs and Concat composites are never written. Resources publish once at the root (not under `/th/`).
10. **Fingerprint naming:** `.` + lowercase hex inserted before the LAST extension; `.min` likewise for minify.
11. **UTF-8 stop rule in the collector** (RuneError stops scanning the rest of the file). The tree-builder quirks (§6.3) only matter for other sites.

## 11. Related observations outside this subsystem, for the orchestrator
1. **Processed-image `_hu_` hashes change with the site's absolute path.** Building an APFS clone of the site at `$W/site` (mtimes preserved) produced output identical to golden **except the `_hu_<hash>` names** in 1801 files. After normalizing `_hu_[0-9a-f]+`, `index.html` is byte-identical. The image hash (`resources/image.go:459-480`: `HashStringHex(incomingID, i.hash(), conf.Key, imagingConfigSourceHash)`) evidently includes something path-dependent. For the images agent: the golden must be compared from the same absolute site path, or the path dependency must be understood.
2. **`resources.GetRemote`** (`layouts/partials/marketing/jsonLd.html:202`) calls `https://www.googleapis.com/youtube/v3/videos?key=…&part=snippet,contentDetails,statistics&id=<youtube_video>` for pages with `youtube_video` front matter (87 golden pages contain `VideoObject`).
   - Responses are cached forever in `~/Library/Caches/hugo_cache/seeksnack/filecache/getresource/<HashString(uri, optionsm)>`: 51 entries in raw HTTP-response dump format, fetched during the golden build (`Date: Sun, 27 Sep 2026 10:25:09 GMT`).
   - The file-cache key is `hashing.HashString(uri, nil)` (hashstructure xxhash, decimal; `create/remote.go:322-333`).
   - The project dir name (`basename(workingDir)`) selects the cache folder.
   - Parity requires either reading these cached responses with an identical key, or live fetches returning identical title, description, thumbnails, publishedAt and duration.
   - Code: `resources/resource_factories/create/remote.go` (469) + `gohugoio/httpcache`.

## 12. Reproduction artifacts (all under `$W`)
- `repro_all.sh`: end-to-end non-Go reproduction of the 3 JS files and the inline CSS (MATCH).
- `esbuild/repro.js`: esbuild JS API with emulated Hugo plugins; also produces the term-order variant (`cda9b0b0…`).
- `esbuild/cli/*`: CLI outputs. `esbuild/node_modules/@esbuild/darwin-arm64/bin/esbuild` is the native 0.25.6 binary.
- `libsass/driver.cpp`, `libsass/obj/*.o`, `libsass/driver`: libsass 3.6.6 with the Hugo importer. `driver_noimp` is the same without the importer.
- `gomin/`: Go tdewolff minifier configured like `minifiers.New`. Takes the media type as argv[1].
- `stats_repro.py`: re-implementation of the hugo_stats.json collector (MATCH over golden/run1).
- `keycalc/`: Go program proving the TransformationKey algorithm (`ce37005bb9b0d2e87a9f0d33876c2b52`).
- `site/`: APFS clone of the site. `site/layouts/partials/head.html` has one extra debug line publishing the toCSS output to `debug/tocss.css`. `build1/` is the Go build of that clone.
- `hugo_stats.golden.json`: a copy of the golden-build `hugo_stats.json`.
