# Content & Page Model — seeksnack on neohugo (Go) → Rust port spec

Agent: `content-model`. Scope: content inventory, front matter, page tree/kinds, taxonomies and terms, languages and translations, URLs and target paths, dates, sorting and collation, collections, pagination, related content, content-derived values, menus, Scratch/Store, page resources, render-order nondeterminism.

All Go references are to `/Users/blackb1rd/git/github/org/neohugo` (module `github.com/neohugo/neohugo`) unless a module-cache path is given (`$GOMODCACHE=/Users/blackb1rd/go/pkg/mod`).

Paths used below:

- `SCR` = `/private/tmp/claude-501/-Users-blackb1rd-git-github-org-neohugo/0835ef6d-534f-4ac6-a7eb-50e9240d6eb1/scratchpad`
- `W` = `$SCR/work/content-model`
- `G` = `$SCR/golden/run1`

---

## 0. Executive summary (most important findings)

1. **Reference model dump.** I built a Go tool that loads the site through neohugo's own `hugolib`, runs a full render into memory, and dumps the page model: every page's kind, path, title, dates, permalinks, translations, params with Go types, resources, `RegularPages`, `Pages`, `Related`, and `Site.Taxonomies`, plus `AllPages` order. The output is `W/model_full.json` (tool: `W/modeldump/main.go`). The dumped `s.Pages()` order is **byte-identical** to the golden `en/sitemap.xml` (1023 URLs) and `th/sitemap.xml` (707 URLs). Related results match all 239 golden "See Also" blocks. Paginated membership matches all 1642 golden list pages, and the `index.json` order matches too. Use it as the oracle for Rust unit tests.
2. **Kinds and counts.** EN: home 1, sections 40, taxonomies 6, terms 817, pages 159, for 1023 listable pages. TH: home 1, sections 26, taxonomies 6, terms 594, pages 80, for 707. Plus standalone 404 per language, `robots.txt` (EN only), `sitemap.xml` per language (at `/en/sitemap.xml` and `/th/sitemap.xml`), and a root `sitemap.xml` index.
3. **Aliases come only from the paginator.** No front matter has `aliases`. Every node rendered with a paginator writes a `page/1/` alias: 1478 files (856 EN = 1+40+6+817-8 collided; 622 TH = 1+26+6+594-5 collided). The 404 page also paginates, giving `404/page/1.html` and `th/404/page/1.html` plus `404/page/2..14.html` and `th/404/page/2..7.html`. The last alias is the main-language redirect `/en/index.html`. Total alias files: 858 EN-side and 623 TH.
4. **Paginator quirk.** `.Paginate` and `.Paginator` share one `sync.Once` (`hugolib/page__paginator.go:45-112`). `partials/head.html` calls `.Paginator` first for all nodes (`IsHome` or `IsNode`, and 404 counts as a node). So `index.html`'s `.Paginate (sort … "Date").Reverse` is ignored. The home paginator is always `site.RegularPages` in default sort, sections use `RegularPages()`, and taxonomies and terms use `Pages()`. This is verified against golden.
5. **Term identity and collisions.** A term's tree key is `strings.ToLower("/"+plural+"/"+value)` with spaces replaced by `-`. It is **not** sanitized. The output path is sanitized (for example `'`, `&`, `(`, `)` are dropped). So "Lays" and "Lay's" become two term pages written to the **same** `/brands/lays/`. There are 13 colliding output dirs (8 EN, 5 TH). Which file wins depends on parallel render timing. This is the source of the 8 RSS files that differ between golden run1 and run2, and it is inherently nondeterministic (§7.7).
6. **Term title and `.Data.Term`.** For an auto-created term, `.Data.Term` is the raw value **last seen** while walking pages in byte order of tree key. `.Title` is the AP title-case of that value (`transform.NewTitleConverter(APStyle)`). `.Name` is from the **first** value that created the node. All three are verified against the dump (1400 auto titles, 1408 terms).
7. **Collation is required.** Tie-breaks in the default page sort and all template `sort` calls use `golang.org/x/text/collate` (CLDR 23). There are 28 EN pages with an identical date, and 99 EN and 67 TH groups of terms share a date. I tested `icu_collator` 2.3.1 with locale `en` or `und` and default options: it agrees with x/text on **all 1,128,753 pairs** of the site's 1503 distinct titles and tag values. ICU4X locale **`th` does NOT agree** (1501 of 1503 positions differ). The x/text `th` collator equals `en` on this data, so use the root/en collator for both languages.
8. **Resources are shared across languages.** Bundle images are published once, under the EN page's (sanitized) directory, and TH pages reuse the same resource objects. TH links therefore point at `/section/bundle/img.jpg` with no `/th/` prefix. All 531 content images are copied, because `renderResources` publishes every resource of a rendered page.
9. **Dates.** All dates come from front matter `date`: YAML strings or TOML datetimes, with `Z` offset, in UTC. `lastmod` and `publishDate` fall back to `date`. Node dates (home, sections, taxonomies, auto terms) are the maximum over descendants or members. Content-backed terms keep their own date. Sorting compares `Date().Unix()`, which is **seconds**, not sub-second.
10. **Title defaults.** EN home comes from an empty `_index.md`, so its title is `""`. TH home is auto-created, so its title is `"SeekSnack"`. Auto sections get `AP(flect.Pluralize(dirname))`, for example `biscuit-roll` → `Biscuit-Rolls` and `candy` → `Candies`. Taxonomies get `AP(plural)` with `-` replaced by space. A content-backed branch without a title keeps `""` (TH `companies/frito-lay`).

---

## 1. Verification artifacts (reuse for Rust tests)

| Artifact | Path | What |
|---|---|---|
| Front matter dump | `W/fmdump/main.go` → `W/fmdump/fm.json` | Every `.md` parsed with `parser/pageparser.ParseFrontMatterAndContent`; raw Go types per key |
| Model dump | `W/modeldump/main.go` → `W/model_full.json` (4.8 MB) | Full build (render to memory) then dump of pages, collections, taxonomies, menus, related, AllPages |
| Probe | `W/probe/main.go` | Fresh `SkipRender` build; prints `Pages()` / `RegularPages()` for given paths |
| Collation fixtures | `W/colltest/strings.json`, `W/colltest/sorted_xtext.json`, `W/colltest/pairs.txt` | 1503 site strings, x/text EN sort, pairwise compare signs (`0` <, `1` =, `2` >) |
| ICU check | `W/rcoll/` (Rust, icu_collator 2.3.1) | Proves ICU4X `en`/`und` == x/text on all pairs |
| Term model | `W/py/terms.py` | Python re-implementation of term key/sanitize/collision logic (matches golden dirs) |
| Site copy used | `W/site` (APFS clone of the seeksnack copy) | For in-memory builds without touching the shared copy |

Regenerate the dump with `cd W/modeldump && GOFLAGS=-mod=mod go build -o modeldump . && HUGO_ENVIRONMENT=production ./modeldump W/site W/model_full.json`. It takes about 6 s with the warm `resources/_gen` cache. The go.mod uses `replace github.com/neohugo/neohugo => /Users/blackb1rd/git/github/org/neohugo`.

Caveat on the dump: dynacache entries can be evicted after the build. For terms, `RegularPages()` and `Pages()` share one cache key (§12.3). The dump code calls `RegularPages()` first, so a few terms show `pages: []` in the dump (for example `/tags/le-pan`). The real render calls `Pages()` first, which returns the term page `brands/le-pan`. Use `W/probe` to query in render order when in doubt.

---

## 2. Content inventory

### 2.1 Files

`content/` holds 848 files: 251 `.md`, 526 `.jpg`, 5 `.png`, and 66 `.gitkeep`.

| Class | Count | Language | Notes |
|---|---|---|---|
| `index.en.md` (leaf bundle) | 153 | en | |
| `index.md` (leaf bundle, no lang) | 1 | en (default) | `seafood/squid-seafood-snack-sweet-spicy-flavour-squid-mixed-surimi/index.md` |
| `index.th.md` (leaf bundle) | 80 | th | every TH bundle also has an EN version (no TH-only bundles) |
| root single pages `*.md` | 5 | en | `disclaimer.md`, `latesturl.md`, `privacy.md`, `search.md`, `terms.md` → kind `page` in the root section |
| `_index.md` (root) | 1 | en | **0 bytes**, no front matter → EN home (file-backed, title "") |
| `brands/le-pan/_index.md` | 1 | en | content-backed **term** (brands/le-pan) |
| `companies/*/_index.en.md` | 6 | en | content-backed terms: `Berli-Jucker-Foods-Ltd.Berli-Jucker-PLC`, `classic-foods-inc`, `cpram`, `frito-lay`, `hanami-foods-co-ltd`, `ja-yubari` |
| `companies/*/_index.th.md` | 2 | th | `frito-lay` (no title), `hanami-foods-co-ltd` |
| `ingredients/ins-124/_index.{en,th}.md` | 2 | en, th | content-backed term |
| images in leaf bundles | 524 | shared | resources of the bundle page |
| images in term bundles | 7 | shared | `companies/*/*.png` ×5, `companies/hanami-foods-co-ltd/hanamifoods.jpg`, `ingredients/ins-124/ins124.jpg` |
| `.gitkeep` | 66 | — | **ignored**: basenames starting with `.` or `#`, or ending in `~`, are dropped (`source/sourceSpec.go:55-73` `IgnoreFile`). Dirs that contain only `.gitkeep` produce no section, for example `kaassoufflé`, `apple`, `cookies`… |

There are no nested sections and no `_index.md` in regular section dirs. Sections are all auto-created (§5.4). The only nesting is one term whose value contains `/` (§7.6). There is no `posts` section, so `[permalinks] posts = "/posts/:year/:month/:title"` never applies (§9.8).

Accented and uppercase names: `potato-chips/wise-chili-olé-…` (é is NFC on disk; hugofs NFC-normalizes names on darwin, `hugofs/component_fs.go:174-176`). The only uppercase path is `companies/Berli-Jucker-Foods-Ltd.Berli-Jucker-PLC/`. All image basenames are lowercase ASCII. Five bundle dirs contain `&`.

### 2.2 Front matter formats and decoding

- **YAML: 218 files**, decoded with `gopkg.in/yaml.v2` v2.4.0 (`parser/metadecoders/decoder.go:31-34,194`). YAML timestamps stay **strings** (all 210 YAML `date` values are strings like `2020-05-17T15:05:09.238Z`). Integers become Go `int` and floats become `float64`. `null` becomes `nil`. `map[interface{}]interface{}` is stringified recursively (`decoder.go:336 stringifyMapKeys`).
- **TOML: 32 files** (`+++`), decoded with `pelletier/go-toml/v2` v2.2.4. `date = 2019-12-31T07:06:21.671Z` becomes `time.Time` in UTC (28 files).
- 1 file has no front matter (`_index.md`, empty).
- No JSON or ORG front matter.
- Keys are lower-cased recursively in maps by `maps.PrepareParams` (`common/maps/params.go:310-345`). It does **not** descend into slices of maps: `ingredients_percentage` items keep their original key case.

### 2.3 Front matter keys (counts over 251 md; Go types from `W/fmdump/fm.json`)

| Key (as written) | Files | Go type(s) | Notes |
|---|---|---|---|
| title | 242 | string | missing in 9 files → Title `""` (no default because they have a file): root `_index.md`; `companies/frito-lay/_index.th.md`; TH pages `biscuit/hello-panda-biscuits-with-chocolate-flavoured-filling`, `biscuit-stick/pocky-biscuit-sticks-almond-crush-thailand`, `biscuit-stick/pocky-biscuit-sticks-colourful-limited-edition-thailand`, `cake/shoei-tokyo-sweet-banana-gift-box`, `crepe/thai-crepe`, `seafood/squidy-seasoned-roller-squid-hot-spicy`, `sponge-cake/yubari-melon-steam-cake` |
| image | 241 | string | resource name inside bundle |
| image_preview | 238 | string | resource name inside bundle |
| date | 238 | string (YAML, 210) / time.Time (TOML, 28) | always `YYYY-MM-DDTHH:MM:SS.mmmZ` |
| description | 233 | string | |
| **Description** (capital D) | 2 | string | `biscuit-stick/pocky-biscuit-sticks-white-peach-strawberry/index.en.md`, `seafood/…surimi/index.md`; lower-cased → same as `description` |
| type | 231 | string | `snacks` 229, `seo` 1 (latesturl), `search` 1 |
| layout | 5 | string | `simple` ×3, `latesturl`, `search` |
| output | 5 | []string | NOT `outputs` → plain param, no effect on output formats |
| companies | 226 | string 223 / []string 3 | |
| brands | 203 | string | |
| categories | 225 | []string 220 / **string 3** / **[] 1** / **[string, [string]] 1** | string: `pie/chocky-banana-pie`, `biscuit-stick/pocky-biscuit-sticks-white-peach-strawberry`, `seafood/…surimi`; empty: `ice-cream/vanilla-and-milk-flavoured-ice-cream-in-chewy-mochi`; nested list: `potato-chips/lays-flat-potato-chip-seaweed-gochujang-sauce-flavor` = `["potato-chips", ["potato-chips"]]` |
| tags | 226 | []string 224 / [] 1 (`rice-chips/zeni-zeni`) / **[string…, nil] 1** | nil element: TH `corn-chips/party-crispy-pie-butter-caramel/index.th.md` = `['ปาร์ตี้','คริสปี้พาย ','เนย','คาราเมล', nil]` (note trailing space) |
| ingredients | 196 | []string 195 / [] 1 | |
| **ingrediants** (typo) | 21 | []string | NOT a taxonomy; plain param |
| countries | 158 | []string 157 / [] 1 | |
| author | 217 | string | `SeekSnack` |
| rating | 200 | map | lower-cased keys |
| when_seen | 171 | string | may be `""` |
| ingredients_percentage | 128 | []map | inner keys keep case; values may be `null` |
| nutrition_facts | 126 | map | |
| youtube_video | 89 | string | |
| currency_code | 50 | string 38 / []string 12 | |
| price | 27 | int | |
| image_carousel | 20 | string | |
| currency | 14 | map | |
| low_price | 10 | int 9 / float64 1 | |
| high_price | 8 | int | |
| website, facebook, instagram, twitter, youtube | 7/5/5/5/5 | string | term bundles |
| references | 5 | []string | term bundles |
| smell_review, taste_review | 4/4 | string | |
| Rating_package / Rating_quantity / Rating_sweet / Rating_tastytaste | 2 each | int | lower-cased to `rating_package`… |
| Rating_smell | 2 | float64 | |
| summary | 1 | string | `pocky-biscuit-sticks-colourful-limited-edition-thailand` → `pcfg.Summary` (unused by templates) |
| wikipedia | 1 | string | |
| private | 1 | bool | latesturl |
| sitemap | 1 | map `{priority: 0.1}` | `search.md` → `pcfg.Sitemap` (priority 0.1, changefreq inherits `weekly`) |

**Keys never used:** `draft`, `publishdate`/`pubdate`/`published`, `lastmod`/`modified`, `expirydate`/`unpublishdate`, `weight`, `slug`, `url`, `aliases`, `linktitle`, `translationKey`, `outputs`, `cascade`, `build`, `headless`, `menu(s)`, `resources`, `markup`, `keywords`, `*_weight`, `path`, `lang`, `kind`, `params`. The Rust port may implement these generically, but no byte of the golden output depends on them.

### 2.4 Params normalization (`hugolib/page__meta.go:390-687 setMetaPostParams`)

1. `maps.PrepareParams(frontmatter)` lower-cases all keys, recursively into maps (`page__meta.go:250`).
2. For each key, lower-cased:
   - **Reserved keys** are copied into `PageConfig` and written back into params with their normalized value: `title` (cast.ToString), `linktitle`, `summary`, `description`, `slug` (trimmed of `-`), `url` (error if it has a scheme), `type`, `keywords` ([]string), `outputs`, `draft`, `layout`, `markup`, `weight` (cast.ToInt), `aliases` ([]string, ToSlash), `sitemap` (config.DecodeSitemap merged with site sitemap config), `iscjklanguage`, `translationkey`, `resources`.
   - **Date keys** (`date`, `publishdate`, `pubdate`, `published`, `lastmod`, `modified`, `expirydate`, `unpublishdate`) are skipped here and handled by the date handler (§10).
   - **Everything else** goes to the default branch: a `[]any` whose elements are all `string` becomes `[]string`; a non-empty `[]any` with any non-string (nil, nested list, map) **stays `[]any`**; an empty `[]any` becomes `[]string{}`; other values are stored as-is.
3. Afterwards, `params["draft"] = false` and `params["iscjklanguage"] = false` are always present, even for nodes without a file (see the dump). `hasCJKLanguage` is false (default), so no CJK detection runs.
4. The date handler also sets `params["date"]`, `params["lastmod"]` and `params["publishdate"]` to the parsed `time.Time` when not already set (§10).
5. `pcfg.Init` and `pcfg.Compile` run: content markup comes from the extension (`md` → markdown), and output formats from site config per kind.

**Resulting param types that matter for templates:**

- `categories` for the nested-list page is `[]any{"potato-chips", []any{"potato-chips"}}`.
- `tags` for the TH party page is `[]any{…, nil}`.
- `categories` is a string for 3 pages.
- `brands` is always a string. `companies` is a string (223) or `[]string` (3).

---

## 3. Path parsing and identity (`common/paths/pathparser.go`)

Every file gets a `paths.Path` (component `content`). This is the canonical identity used for tree keys, sections, URLs and translations.

- **Normalization** (`NormalizePathStringBasic`, L50-58): `strings.ToLower` then replace `" "` with `"-"`. That is **all**: no sanitization, and `'`, `&`, `(`, `.` are kept. `Unnormalized()` keeps the original case and spaces (L101-121).
- **Identifiers** (L123-319) are parsed right-to-left from the last path element only. The first identifier is the extension. When the last element has more than one dot, the next identifier is checked against the configured language keys (`en`, `th`). A **disabled** language (`fr`, `de`, …) marks the path `Disabled` and the file is dropped (`component_fs.go:180-183`). Other identifiers are "unknown" and stay part of the name.
- **Bundle type** (L269-293): the name before the extension decides it. `index` → `TypeLeaf`, `_index` → `TypeBranch`, anything else → `TypeContentSingle`. Files inside a leaf bundle other than its root `index.*` are changed to `TypeContentResource` or `TypeFile` (`pages_capture.go:389-396`).
- **Base()** (L634-688) is the tree key:
  - content page: the path without extension or language, and without the `/index` or `/_index` element for bundles. Examples: `/biscuit/koalas-march-chocolate`, `/disclaimer`, `/companies/berli-jucker-foods-ltd.berli-jucker-plc`, `/potato-chips/herrs-salt-&-vinegar-potato-chips`. Home Base is `/`, and its **tree key is `""`** (`cleanTreeKey`, `content_map.go:476-493`; `addMissingRootSections` compares `s == ""`).
  - resource (non-content): the full path with extension kept, for example `/biscuit-roll/collon-cream/collon_cream_package.jpg`.
- **Section()** (L475-480): the first path element (lower-cased), or `""` for root pages.
- **ContainerDir()** (L467-472): for bundles, the dir **above** the bundle dir; otherwise `Dir()`.
- **BaseNameNoIdentifier()** (L522-527): the bundle dir name for bundles, otherwise the file name without identifiers.
- **Language of a file** (`hugofs/component_fs.go:173-215 applyMeta`): the language identifier in the file name if present, otherwise the mount language (none here), otherwise `defaultContentLanguage` = `en`. Files are grouped by `LangIndex` (en=0, th=1).
- **Directory listing order** (`component_fs.go:64-150 ReadDir`): directories before files; bundle files (`index*`/`_index*`) first; then extension descending; then Base ascending; then weight (a lang-suffixed file gets +1); then name. This only affects insertion and ID order, not output (trees are ordered by key).

---

## 4. The content tree (`hugolib/doctree`, `content_map.go`, `content_map_page.go`)

- There are two trees per build, shared by all sites and "shaped" per language: `treePages` (pages keyed by `Base()`) and `treeResources` (resources keyed by `Base()` with extension). There is also `treeTaxonomyEntries`, keyed `termBase + pageKey` (for home, `termBase + "/"`).
- The trees are **radix trees (armon/go-radix)**, so every walk is in **byte-lexicographic order of the key**. For example `"/brands/lay's"` sorts before `"/brands/lays"` because `'` (0x27) is less than `s`. Thai keys order by UTF-8 bytes.
- **Language dimension** (`content_map_page.go:696-911 contentNodeShifter`): a key holds either one node or an array indexed by language.
  - When a site of language L walks the tree, pages are returned only if they exist in L. There is no fallback for pages.
  - For **non-page resources** in a non-exact lookup, the first non-nil language version is returned with match flag 0 (`Shift`, L760-783). This is how TH pages reach EN-owned images.
- `LongestPrefix(s, exact, pred)` (`nodeshifttree.go:224-243`) is a **character-level** radix longest-prefix match (not segment-aware). If the found node does not exist in the current language or fails the predicate, it retries with `path.Dir(s)`. The Rust port needs the same semantics: try `s[..i]` for decreasing `i`, not only at `/` boundaries.
- Insertion: `AddFi` (`content_map.go:216-317`). Content files go to `treePages` via `newPage`. Non-content files go to `treeResources` as `resourceSource{langIndex = file LangIndex}`. Images have no language suffix, so their LangIndex is 0 (EN).

---

## 5. Page creation, kinds and assembly pipeline

### 5.1 `newPage` / `doNewPage` (`hugolib/page__new.go:54-265`)

- The site is chosen from the file's language (L132-150).
- **Kind** (L152-172):
  - `Base()=="/"` → `home`.
  - A branch bundle is `taxonomy` if its Base equals a taxonomy's `pluralTreeKey` (for example `/brands`). It is `term` if Base has a taxonomy prefix (`getTaxonomyConfig` uses `strings.HasPrefix(s, "/brands")`, character-level, `content_map.go:158-165`). Otherwise it is `section`.
  - Otherwise, with a file → `page`.
- For terms, `m.term = TrimLeading(TrimPrefix(Unnormalized().Base(), pluralTreeKey))` (L183), and `m.singular` is set.

### 5.2 Assembly order (`hugolib/hugo_sites_build.go:274-349`)

The steps below run for all sites, in site order en then th:

1. **Step1** (runs in parallel across sites, `content_map_page.go:1856-1870`):
   1. `addMissingTaxonomies` (L2103-2134): creates `/brands`, `/categories`, `/companies`, `/countries`, `/ingredients`, `/tags` for each site if missing. They are all missing (no taxonomy `_index`).
   2. `addMissingRootSections` (L2007-2101): walks the tree. For each `page` or `section` whose `Section()` is non-empty and not yet seen, it creates the section `/<Unnormalized section>/_index.md` if absent. If no node at key `""` exists, it creates the home. EN home exists (`_index.md`). **TH home is created here, with no file.**
   3. `addStandalonePages` (L1930-2005): `/404` (kind `404`, format `404`) for each site. `/_robots` for site 0 only (`enableRobotsTXT`). `/_sitemap` for each site, with basename from `sitemap.filename`. `/_sitemapindex` for site 0 only, because the build is multilingual.
   4. `applyAggregates` (L1385-1544): walks pages in key order. It calls `setMetaPost` (params, dates, defaults, §2.4, §6, §10), except for terms, which are delayed. It propagates cascade (unused) and date aggregation to home and sections (§10.3), and sets resource-page metadata.
2. **Step2** (sequential per site, L1872-1884):
   1. `removeShouldNotBuild` (L1895-1927): drops pages with draft/future/expired. None here.
   2. `assembleTermsAndTranslations` (L1635-1733): creates the term pages and entries (§7).
   3. `applyAggregatesToTaxonomiesAndTerms` (L1546-1633): runs `setMetaPost` for terms (titles are computed **here**, after all terms have their final `m.term`) and does date aggregation for terms and taxonomies.
3. `initRenderFormats` (`site.go:800-837`): per site, sorted by weight then name: `html`, `404`, `json`, `robots`, `rss`, `sitemap`, `sitemapindex`.
4. **Final**: `assembleResources` (L1735-1854), sequential en then th (§18).
5. `Site.Taxonomies()` is built lazily on first use (`site.go:786-791` → `CreateSiteTaxonomies` L2136-2199).

### 5.3 Counts after assembly

| | home | sections | taxonomies | terms | pages | listable total |
|---|---|---|---|---|---|---|
| en | 1 (file) | 40 | 6 | 817 (7 file-backed + 810 auto) | 159 | 1023 |
| th | 1 (auto) | 26 | 6 | 594 (3 file-backed + 591 auto) | 80 | 707 |

**EN sections (40):** almonds, biscuit, biscuit-roll, biscuit-stick, bread-pan, cake, candy, candy-shell, cheese-puffs, cookies, corn-chips, crackers, crepe, croissant, french-fries, fries, fruit, ice-cream, jelly, korean-snacks, marshmallow, nacho-cheese-chips, onion-rings, party-mixes, pastry, peas, pie, popcorn, potato-chips, potato-crisps, pretzels, rice-chips, rice-crackers, seafood, seafood-chips, seaweed, sponge-cake, tortilla-chips, vegetable-chips, wafer.

**TH sections (26):** almonds, biscuit, biscuit-roll, biscuit-stick, cake, candy, candy-shell, cookies, corn-chips, crackers, crepe, croissant, fruit, ice-cream, jelly, marshmallow, peas, popcorn, potato-chips, potato-crisps, seafood, seaweed, sponge-cake, vegetable-chips, wafer, pastry.

A section exists in a language only if that language has at least one page under it. For example there is no TH `bread-pan`.

### 5.4 Standalone and other pages

- `404`: `/404.html` and `/th/404.html`. It is paginated over `site.RegularPages` because the template's `head.html` calls `.Paginator` (§13).
- `robots.txt`: EN only.
- Per-language sitemap: `alwaysInSubDir` means the EN one lands at `/en/sitemap.xml` (`page__paths.go:115,142`; `site.go:1369-1379`).
- `sitemapindex`: `/sitemap.xml`, root format.
- Standalone pages are never listed (`shouldList` returns false, `page__meta.go:691-706`).

---

## 6. Titles and other defaults

`applyDefaultValues` (`page__meta.go:732-786`) sets a default title **only when `Title=="" && p.f == nil`**, that is, only for nodes with no content file:

| Kind | Default title (no file) | Examples |
|---|---|---|
| home | `site.Title` | TH home → `SeekSnack`; EN home has a file, so `""` |
| section | `CreateTitle(flect.Pluralize(Unnormalized().BaseNameNoIdentifier()))` (`pluralizeListTitles=true`, `capitalizeListTitles=true` defaults, `config/allconfig/allconfig.go:1001-1006`) | `biscuit`→`Biscuits`, `biscuit-roll`→`Biscuit-Rolls`, `candy`→`Candies`, `jelly`→`Jellies`, `pastry`→`Pastries`, `popcorn`→`Popcorns`, `seafood`→`Seafoods`, `crepe`→`Crepes`, `almonds`/`peas`/`fries`/`cookies`/`crackers`/`pretzels` unchanged (end in `s`), `corn-chips`→`Corn-Chips` |
| taxonomy | `strings.ReplaceAll(CreateTitle(plural), "-", " ")` | `Brands`, `Categories`, `Companies`, `Countries`, `Ingredients`, `Tags` |
| term | `CreateTitle(m.term)` (final `m.term`, §7.3) | `Lay's`, `Lays`, `No Salt/Low Salt Chips`, `Disodium 5'-Guanylate`, `INS 322(i)`, `CORORO`, `Le-Pan` (from value `le-pan`), ` ขนมเปี๊ยะ` (leading space kept) |
| 404 | `404 Page not found` | |

File-backed nodes with no `title` keep `""`: the EN home and TH `companies/frito-lay`.

- **LinkTitle** = LinkTitle param (none) or Title.
- **Name()** (`page__meta.go:171-179`): for terms, `Unnormalized().BaseNameNoIdentifier()`, that is the value that **first** created the node (for example `lay's`). Otherwise Title.
- **Type()** (L228-238): the `type` param if set, else `Section()`, else `"page"`. The home type is `page`.
- **Description**: the front matter `description` or `Description`, else `""`.

### 6.1 `CreateTitle` = AP style (`helpers/general.go:194-210` → `github.com/jdkato/prose@v1.2.1/transform/title.go`)

This algorithm must be ported exactly, including its bugs:

```
smallWords = a an and as at but by en for if in nor of on or per the to vs vs. via v v.
splitRE    = [\p{N}\p{L}]+[^\s-/]*        (Go RE2: \s is ASCII [\t\n\f\r ])
sanitizer  = “→" ”→" ‘→' ’→' –→- —→- …→...  (applied to a copy t used only for position checks)
Title(s):
  idx=0; t=sanitize(s); end=len(t)   // byte length
  for each match m of splitRE in s (leftmost-first):
     sm=lower(m); pos=Index(t[idx:], m)+idx; prev=charAt(t,pos-1)
     ext=RuneCount(m)        // BUG: rune count, used as byte offset
     idx=pos+ext
     if optionsAP(sm, pos==0 || idx==end) && prev in {' ','-','/'} &&
        charAt(t,pos-2)!=':' && charAt(t,pos-2)!='-' &&
        (charAt(t,pos+ext)!='-' || charAt(t,pos-1)=='-')  -> keep sm (lowercase)
     else -> unicode.ToTitle(first rune) + rest
  charAt(s,i) = s[i] if 0<=i<len(s) else s[0]
  optionsAP(w,bounding) = !bounding && w in smallWords
```

A Python port of this reproduces all 1400 auto term titles in the dump (`W` session, 0 mismatches). In Rust, do **not** use `\s` of the `regex` crate in Unicode mode. Use `[\t\n\f\r ]`. Keep the rune-count-as-byte-offset bug.

### 6.2 `flect.Pluralize` (`$GOMODCACHE/github.com/gobuffalo/flect@v1.0.3`)

This is only used for section titles here. Port `pluralize.go` (72 lines), `plural_rules.go` (417 lines, dictionary plus suffix rules), `ident.go` (122 lines, `New`/`toParts`/`LastPart`/`ReplaceSuffix`), `rule.go`, `acronyms.go`, and `custom_data.go`.

The algorithm:

1. Take the last part of the identifier (split on non-alphanumerics and case changes).
2. Look it up in the exact dictionary; if the lower-cased form is already plural, keep it.
3. Otherwise apply the first matching suffix rule.
4. Otherwise, if it ends in `s`, keep it; else append `s`.

A table-driven shortcut would pass seeksnack (40 names listed in §5.3), but a full port is recommended.

---

## 7. Taxonomies and terms

### 7.1 Configuration

`[taxonomies]` maps company→companies, category→categories, country→countries, ingredient→ingredients, brand→brands, tag→tags. TH repeats the same map.

`taxonomiesConfig.Values()` (`hugolib/site.go:662-680`) sorts views by **plural**: brands, categories, companies, countries, ingredients, tags. `pluralTreeKey = cleanTreeKey(plural)` gives `/brands` and so on.

### 7.2 Value extraction (`content_map_page.go:1635-1733`)

For every node in `treePages` (all kinds, key order, skipping `noLink`), and for every view in plural order:

```
vals = types.ToStringSlicePreserveString(getParam(ps, plural, false))   // common/types/convert.go:54-83
```

- A `string` becomes `[s]`. It is **not** split on whitespace.
- `[]string` is used as-is.
- `[]any` goes through `cast.ToStringSliceE` (spf13/cast v1.9.2 `slice.go:82`, `basic.go:70`): `nil` becomes `""`, numbers and bools are formatted, and **any nested slice or map makes the whole conversion fail**. Then there is a reflect fallback that fails the same way. So `vals=nil` and the page gets **no terms** for that taxonomy. Example: the potato-chips page with `["potato-chips",["potato-chips"]]` is in no category term.
- `""` entries are skipped (`if v == "" continue`). The nil tag in the TH party page is dropped.
- The weight comes from `<plural>_weight`, which is not present here, so it is 0.

For each value `v`:

```
pi   = PathParser.Parse("content", "/"+plural+"/"+v+"/_index.md")
key  = pi.Base()                 // = lower("/"+plural+"/"+v) with ' ' -> '-'
term = treePages.Get(key)
if nil: create term page {term: v, singular, pathInfo: pi, Kind: term}   (Name/Unnormalized from THIS v)
else:   term.m.term = v            // LAST value wins
entries.Insert(key + pageKey (or "/" for home), {weight, n: page, term: &pageWithOrdinal{term, i}})
```

Consequences, all verified against golden output or the dump:

- **Case-insensitive merge**: "Lays", "lays" → one term.
- **Punctuation keeps terms apart**: "Lay's" ≠ "Lays"; "INS 322(i)" ≠ "INS 322i"; "Disodium 5'-Guanylate" ≠ "Disodium 5-Guanylate".
- Leading and trailing spaces become hyphens in the key: `" ขนมเปี๊ยะ"` → `/tags/-ขนมเปี๊ยะ`; `"คริสปี้พาย "` → `/tags/คริสปี้พาย-`; `"Tohato Inc. "` → `/companies/tohato-inc.-`; `"Alice's "` → `/tags/alice's-`.
- A `/` in a value creates a **nested** term key: `"No Salt/Low Salt Chips"` → `/categories/no-salt/low-salt-chips`. Its Parent is `/categories` (LongestPrefix), and it is listed under the taxonomy because taxonomy `Pages()` is recursive.
- Term pages that are content-backed (file `_index.*.md`) are found by exact key. So `companies/frito-lay/_index.en.md` (key `/companies/frito-lay`) merges with value "Frito-Lay". But `companies/Berli-Jucker-Foods-Ltd.Berli-Jucker-PLC/` (key `/companies/berli-jucker-foods-ltd.berli-jucker-plc`) does **not** merge with value "Berli Jucker Foods Ltd. Berli Jucker PLC" (key `…ltd.-berli-jucker-plc`), yet both sanitize to different dirs. The golden output shows both dirs exist.
- **Terms are members of other terms**: content-backed term pages that carry taxonomy params contribute entries. For example `/companies/hanami-foods-co-ltd` (EN) has `countries:[Thailand]` and `tags:[hanami,…]`, so it appears in `/countries/thailand/` (golden page 3) and `/tags/hanami/`. Likewise `/brands/le-pan` is tagged `le-pan`, which creates term `/tags/le-pan` with title `Le-Pan`.
- Content-backed terms with no referencing value still exist and render: EN `/companies/hanami-foods-co-ltd`; TH `/companies/frito-lay`, `/companies/hanami-foods-co-ltd`. They have **no** key in `.Site.Taxonomies`.

### 7.3 Term metadata

| Field | Rule | Source |
|---|---|---|
| `.Data.Term` (`m.term`) | raw value **last** seen in tree-key walk order; for content-backed never-referenced terms: `TrimLeading(TrimPrefix(Unnormalized().Base(), pluralTreeKey))` | `page__new.go:183`, `content_map_page.go:1708-1710`; verified 816 EN + 592 TH |
| `.Title` | front matter title if file-backed (may be `""`), else `CreateTitle(final m.term)` | `page__meta.go:764-773` |
| `.Name` | `Unnormalized().BaseNameNoIdentifier()` of the path from the **first** creating value | `page__meta.go:175-177` |
| `.Data.Singular`, `.Data.Plural`, `.Data.<singular>` | from the view; `.Data.<singular>` = `Site.Taxonomies[plural].Get(TrimPrefix(path, pluralTreeKey))` (note: `Get` lower-cases; leading `/` means it normally does NOT match any key → nil) | `hugolib/page__data.go:39-47` |
| `.Section`, `.Type` | `brands`, …; the Type falls back to the section | |
| Parent | taxonomy page (`LongestPrefix(ContainerDir)`) | `page__tree.go:116-137` |
| CurrentSection | itself (all branch kinds) | `page__tree.go:59-75` |

### 7.4 `.Site.Taxonomies` (`content_map_page.go:2136-2199`)

`map[plural]Taxonomy`. For each term page under `/plural/` that is listable, key = `strings.ToLower(m.term)`. Entries (`WeightedPage{weight, page, owner=term}`) are appended from `treeTaxonomyEntries` under the term key, and each list is then `sort.Stable` by (weight, then `DefaultPageSort`) (`resources/page/weighted.go` `Less`).

- Keys can collide: two term pages with the same lower-cased `m.term` would merge. Here "lay's" and "lays" stay distinct keys.
- Key counts: EN brands 54, categories 63, companies 57, countries 5, ingredients 395, tags 242. TH 39 / 29 / 34 / 5 / 321 / 164.
- The template `range $k, $v := .Site.Taxonomies.brands` iterates keys in **Go byte order** (text/template fmtsort). `index.html` then does `.Site.GetPage "/brands/<key>"`. All keys resolve (§12.4), so the sidebar lists `Lay's` and `Lays` separately, both linking to `/brands/lays/` (verified).
- `Alphabetical()` uses the collator, and `ByCount()` uses count descending then `compare.LessStrings` (`resources/page/taxonomy.go`). Neither is used by the seeksnack templates.

### 7.5 Taxonomy (list) page collections

| Accessor | Result |
|---|---|
| taxonomy `.Pages` / `.Data.Pages` | all listable **term** pages under `/plural/` (recursive), `SortByDefault` (`page.go:333-341`) |
| taxonomy `.RegularPages` | pages directly under the taxonomy (none) |
| taxonomy `.Data.Terms` | `Site.Taxonomies[plural]` (unused by templates) |
| term `.Pages` / `.Data.Pages` | every entry node (any kind) with `ShouldListLocal`, as `pageWithWeight0` wrappers, `SortByDefault` (`content_map_page.go:417-452`) |
| term `.RegularPages` | **shares the cache key with `.Pages`** (`Path+"/"`), so whichever is called first wins (§12.3) |

### 7.6 Output collisions (nondeterministic files)

Term tree keys differ, but after sanitization (`'`, `(`, `)` dropped) they share one output directory:

| Lang | Output dir | Tree keys (title, #members) | golden run1 HTML winner | RSS differs run1 vs run2 |
|---|---|---|---|---|
| en | `/brands/lays/` | `/brands/lay's` ("Lay's", 20) · `/brands/lays` ("Lays", 1) | Lay's (1st in key order) | yes |
| en | `/tags/lays/` | `/tags/lay's` ("Lay's", 9) · `/tags/lays` ("Lays", 6) | Lays (2nd) | yes |
| en | `/ingredients/disodium-5-guanylate/` | `…/disodium-5'-guanylate` · `…/disodium-5-guanylate` | 2nd | yes |
| en | `/ingredients/disodium-5-inosinate/` | `…5'-inosinate` · `…5-inosinate` | 2nd | yes |
| en | `/ingredients/disodium-5-ribonucleotide/` | `…5'-ribonucleotide` · `…5-ribonucleotide` | 2nd | yes |
| en | `/ingredients/disodium-5-ribonucleotides/` | `…5'-ribonucleotides` · `…5-ribonucleotides` | 2nd | yes |
| en | `/ingredients/ins-322i/` | `…/ins-322(i)` · `…/ins-322i` | 2nd | no |
| en | `/ingredients/ins-500ii/` | `…/ins-500(ii)` · `…/ins-500ii` | 2nd | no |
| th | `/th/ingredients/ins-322i/` | same pattern | 2nd | yes |
| th | `/th/ingredients/ins-500ii/` | | 2nd | no |
| th | `/th/ingredients/ไดโซเดียม-5-กัวไนเลต/` | `…5'-…` · `…5-…` | 2nd | yes |
| th | `/th/ingredients/ไดโซเดียม-5-ไรโบนิวคลีโอไทด์/` | | 2nd | no |
| th | `/th/ingredients/ไดโซเดียม-5-ไอโนซิเนต/` | | 2nd | no |

`brands/lays/page/2/index.html` exists only because `Lay's` has 20 members, giving 2 pagers. Both term pages also appear separately in sitemaps, `index.json`, taxonomy lists and the sidebar, with the same permalink.

Recommendation: render sequentially in tree-key order with "last write wins". That matches 12 of 13 golden HTML files. Whitelist the 13 dirs (`index.html`, `index.xml`, `page/*`) in the byte-parity test, because Go itself is not reproducible there.

### 7.7 Why Go is nondeterministic here

`renderPages` (`site_render.go:70-120`) pushes pages in tree order into a channel read by N workers. Two colliding pages are rendered concurrently and the last `WriteFile` wins. Each output format is a separate pass, which is why HTML and RSS winners can differ.

---

## 8. Languages and translations

- `defaultContentLanguage = "en"`, and `defaultContentLanguageInSubdir` is false (default).
- `disableLanguages` = de, es, fr, ja, nl, pl, pt, zh-cn, zh-tw. The enabled languages are **en (weight 1)** and **th (weight 2)**. `IsMultilingual` is true.
- Language prefix (`config/allconfig/configlanguage.go:49-58` `LanguagePrefix`): en → `""`, th → `"th"`. It is used for file paths (`PrefixFilePath`) and links (`PrefixLink`).
- Collators per language: `collate.New(tag)` (`langs/language.go:54-91`). The location is `time.LoadLocation(timeZone="")`, which is **UTC** (L112-120).
- **Translations** (`hugolib/page.go:398-461`):
  - `TranslationKey = Path()`, because there is no `translationKey` front matter.
  - `AllTranslations` is every language's node at the **same tree key** (`ForEeachInDimension`), filtered by `ShouldLink`, and sorted by `lessPageLanguage` (`pages_sort.go:118-140`): language weight (en before th), then date desc, then `compare.Strings(LinkTitle)`, then filename.
  - `Translations` = `AllTranslations` minus self. `IsTranslated` = `len(Translations) > 0`.
  - 358 nodes per language are translated: home 1, taxonomies 6, sections 26, pages 80, terms 245.
  - Terms translate whenever the same key exists in both languages. For example `/brands/alice` is EN and TH, but Thai-script tags only exist in TH.
- Content language comes from the filename identifier (`.en.` / `.th.`). Files with no identifier are EN. A TH file with the same bundle dir is the translation.
- **Per-language taxonomies**: each site builds its own terms from its own pages. TH terms come from TH pages plus TH content-backed terms.
- **Main-language redirect** (`site_render.go:339-367`): multilingual and not in a subdir, so `/en/index.html` is an alias to `https://seeksnack.com/`.
- `site.Languages` = [en, th] by weight.

---

## 9. URLs, target paths and permalinks

### 9.1 Descriptor (`hugolib/page__paths.go:111-181`)

- `Path` = `PathInfo()` (normalized; `disablePathToLower=false`).
- `Section` = `CurrentSection().PathInfo()`, which is the page itself for branch kinds.
- `BaseName` = slug (none), else the standalone format basename (`404`, `sitemap`, `robots`), else `Path.BaseNameNoIdentifier()`.
- `PrefixFilePath` and `PrefixLink` = language prefix; sitemaps force `en`.
- `URL` (none).
- `ExpandedPermalink` (none, §9.8).
- `UglyURLs` = false.

### 9.2 `CreateTargetPaths` (`resources/page/page_paths.go:109-293`)

The element builder applies, in order:

1. If `Type.Path != ""`, add it (none of our formats have one).
2. If the format is 404, Sitemap or Robots, set `noSubResources`.
3. Else if `Kind != page` and `Section.Base() != "/"`, add `Section.Base()` (the whole `"/brands/lay's"` as ONE element) and set `needsBase = false`.
4. Kind `page` (and 404/sitemap/robots): add `ContainerDir` (only if not `/`), then `BaseName`, then `Addends`; then if ugly, concat the suffix onto the last element, else add `BaseName+suffix` (`index.html`, `index.xml`).
5. Branch kinds: add `Addends`; then add `index.<ext>` unless ugly.
6. **Sanitize every element**: `MakePathSanitized(el) = strings.ToLower(paths.Sanitize(el))` (`helpers/path.go:59-64`).
7. `TargetFilename` = `/` + prefixPath + `path.Join(els)`. It is raw UTF-8 (NFC) on disk.
8. Link = `path.Join(els[:len-1])` when the last element is `index.html`, and a trailing `/` is appended. Then `/`+prefixLink is prepended. Then `#` is replaced with `%23`, and `paths.PathEscape` is applied (`common/paths/path.go:388-394` = `url.Parse(link).EscapedPath()`).
9. `SubResourceBaseTarget` / `SubResourceBaseLink` = the dir of the target and link (with prefix; link-dir is *not* escaped).

### 9.3 `paths.Sanitize` (`common/paths/path.go:282-336`) — exact

```
allowed(r) = r != ' ' && (IsLetter(r) || IsDigit(r) || r in "./\\_#+~-@" || IsMark(r)
             || (r=='%' && next two BYTES are hex))
fast path: if every rune is allowed, return s unchanged
out=[]; prependHyphen=false; wasHyphen=false
for r in s:
  if allowed(r): wasHyphen = (r=='-'); if prependHyphen { if !wasHyphen {out+='-'}; prependHyphen=false }; out+=r
  else if len(out)>0 && !wasHyphen && unicode.IsSpace(r): prependHyphen=true
  // any other disallowed rune is silently dropped and does NOT reset wasHyphen
```

Examples:

- `herrs-salt-&-vinegar-potato-chips` → `herrs-salt--vinegar-potato-chips` (double hyphen kept)
- `lay's` → `lays`
- `ins-322(i)` → `ins-322i`
- `คริสปี้พาย-` stays unchanged (Thai vowels and tone marks are `Mn`, which is allowed)
- `ol\u00e9` stays unchanged

Use Go's definitions: `IsLetter` = category L\*, `IsDigit` = Nd, `IsMark` = M\*, `IsSpace` = Go's list (Unicode **17.0**, Go 1.27). **Do not** use Rust `char::is_alphabetic` (it includes Nl and Other_Alphabetic) or `is_numeric` (it includes No/Nl).

### 9.4 Examples (verified against the dump and golden output)

| Page | TargetFilename | RelPermalink / Permalink |
|---|---|---|
| EN home | `/index.html`, `/index.json`, `/index.xml` | `/` · `https://seeksnack.com/` |
| TH home | `/th/index.html` … | `/th/` |
| section | `/biscuit/index.html`, `/biscuit/index.xml` | `/biscuit/` |
| leaf page | `/biscuit/koalas-march-chocolate/index.html` | `/biscuit/koalas-march-chocolate/` |
| root single | `/disclaimer/index.html` | `/disclaimer/` |
| `&` bundle | `/potato-chips/herrs-salt--vinegar-potato-chips/index.html` | same |
| accented | `/potato-chips/wise-chili-olé-chili--spice-flavor-potato-chips/index.html` | `/potato-chips/wise-chili-ol%C3%A9-chili--spice-flavor-potato-chips/` |
| uppercase dir term | `/companies/berli-jucker-foods-ltd.berli-jucker-plc/index.html` | same |
| Thai term | `/th/tags/คริสปี้พาย-/index.html` | `/th/tags/%E0%B8%84…%E0%B8%A2-/` (**uppercase** hex) |
| nested term | `/categories/no-salt/low-salt-chips/index.html` | |
| taxonomy | `/brands/index.html`, `/brands/index.xml` | `/brands/` |
| pager N≥2 | `/biscuit/page/2/index.html` (Addends `/page/2`) | `.URL` = `/biscuit/page/2/` |
| page/1 alias | `/biscuit/page/1/index.html` → redirect to `Permalink()` | |
| 404 | `/404.html`, `/th/404.html` (format Ugly) | `/404.html` |
| 404 pager | `/404/page/2.html`, alias `/404/page/1.html` | |
| sitemap | `/en/sitemap.xml`, `/th/sitemap.xml`; index `/sitemap.xml` | |

**Permalink** = `baseURL + strings.TrimPrefix(link, "/")` (`helpers/pathspec.go:76-78`). The baseURL `https://seeksnack.com/` has no sub-path, so `PrependBasePath` is a no-op.

**canonifyURLs=true** does not change `.Permalink` or `.RelPermalink`. It only affects the HTML post-processing that turns `href="/…"` and `src="/…"` into absolute URLs (output-publishing agent). Golden HTML contains absolute URLs everywhere.

### 9.5 Link escaping variants that reach the output

- **Page and resource permalinks** use Go `url.Parse(x).EscapedPath()`, which gives uppercase `%XX` for non-ASCII and any byte not in `[A-Za-z0-9-_.~$&+,/:;=@]`. Go first tries `RawPath` only when it is validly encoded, and raw UTF-8 is not. So port `net/url` `shouldEscape(encodePath)`, `escape`, `unescape` and `validEncoded` (about 150 lines). Do not use the `percent-encoding` crate unless you reproduce the same set.
- **Template `urlize`** = `url.Parse(MakePathSanitized(s)).String()` (`helpers/url.go:30-50`). Templates then pipe it to `lower`, which gives **lowercase** `%e0%b8…` in tag and category badge links (golden). Note that `urlize` Sanitize drops a trailing space (no hyphen), so the badge link for `"คริสปี้พาย "` points to `/th/tags/คริสปี้พาย/`, which does not exist. The golden output has that broken link.

### 9.6 `relLangURL` / `absLangURL` / `RelURL`

`helpers/url.go:53-174` covers the language prefix insertion and canonify interplay. That is the templates agent's domain.

### 9.7 Resources (bundle images)

Target = `SubResourceBaseTarget` of the **owning page in the site that created it (EN)** + the original-case relative path (`relPathOriginal`) (`content_map_page.go:1779-1825`). All image names are lowercase, so this is effectively lower-cased. TH pages reuse the EN resource object, so there is no `/th/` copy (golden `th/biscuit-roll/collon-cream/` contains only `index.html`).

### 9.8 Permalinks config

A flat `posts = "/posts/:year/:month/:title"` is registered for kinds `page` and `term` under key `posts` (`resources/page/permalinks.go:437-480`). `Expand(p.Section(), p)` finds no `posts` section or taxonomy, so it is a no-op for seeksnack. Implement it as "unsupported / no-op unless a matching section exists", or port it later.

---

## 10. Dates

### 10.1 Handler chains (`resources/page/pagemeta/page_frontmatter.go:405-450, 542-584, 649-689`)

With defaults (no `[frontmatter]` config):

| Field | Chain (first non-empty wins) |
|---|---|
| Date | date → publishdate → pubdate → published → lastmod → modified |
| Lastmod | :git (disabled, `enableGitInfo` false) → lastmod → modified → date → publishdate → pubdate → published |
| PublishDate | publishdate → pubdate → published → date |
| ExpiryDate | expirydate → unpublishdate |

Every page here only has `date`. So `Date = Lastmod = PublishDate = date`, and `ExpiryDate` is zero.

The field handler (L800-826) works as follows:

- A value that is `""` or `nil` is skipped.
- A `time.Time` already in the location is used as-is. Otherwise `htime.ToTimeInDefaultLocationE(v, UTC)` is applied (cast formats; `…Z` gives UTC).
- The parsed time is written back to `params[key]`.
- The setters `setParamIfNotSet` add `params["lastmod"]` and `params["publishdate"]` (and `params["date"]`) as `time.Time`.

### 10.2 Build filters (`hugolib/site.go:1556-1578`)

Drafts and future or expired pages are excluded for a non-server production build. No seeksnack page is affected: the max date is 2023-09-25, and "now" is the build time.

### 10.3 Aggregation for nodes

- **Home and sections** (`content_map_page.go:1456-1487`): if a branch has all dates zero (or is home), it listens to "dates" events from every descendant path (`key + "/"` prefix) and takes the **maximum** of Date and Lastmod. It takes PublishDate only if that is before now (`Dates.UpdateDateAndLastmodAndPublishDateIfAfter`, L62-73). Events are queued during the walk and handled afterwards (`doctree/support.go:32-53, 211-240`). The result is a pure max, so order does not matter.
  - EN home = TH home = 2023-09-25T14:45:00.876Z.
  - TH `/crepe` and `/sponge-cake` have **zero dates**, because their only TH pages lack dates.
- **Taxonomies and auto terms** (L1546-1633): a term with all-zero dates takes the max over its entries' pages. A taxonomy takes the max over all its terms and entries. **Content-backed terms with their own date keep it** (for example `companies/frito-lay` 2020-12-10T12:45:19.622Z, `brands/le-pan` 2020-07-05) and do not aggregate.
- `site.Lastmod` = max Lastmod (home listener).

### 10.4 Timezone and formatting

- The language `timeZone` is unset, so `time.LoadLocation("")` gives UTC. All dates are UTC, so templates print `+0000` / `+00:00`.
- `.Date.Format` is Go's layout formatting (not localized).
- The `dateFormat` template function is localized through `gohugoio/locales` (templates or i18n agent).
- Sorting always uses `Date().Unix()` seconds. A zero time has `Unix() = -62135596800`, so it sorts last in descending order.
- Templates use `now.Format "2006"` (RSS and footer copyright), so output depends on the **build year**. Golden says `2019 -2026`.

---

## 11. Sorting

### 11.1 `DefaultPageSort` (`resources/page/pages_sort.go:83-116`), used by `SortByDefault` (`sort.Stable`)

```
less(p1,p2):
  o1,o2 = ordinals (pageWithOrdinal, only in GetTerms)      -> if both set & differ: o1<o2
  w01,w02 = Weight0 (pageWithWeight0 = taxonomy weight)     -> if both set & differ: w01<w02
  if p1.Weight()==p2.Weight():
     if p1.Date().Unix()==p2.Date().Unix():
        c = collator(currentSite).CompareString(p1.LinkTitle(), p2.LinkTitle())
        if c==0: return compare.LessStrings(p1.PathInfo().Path(), p2.PathInfo().Path())
        return c<0
     return p1.Date().Unix() > p2.Date().Unix()        // newest first
  if p2.Weight()==0: return true; if p1.Weight()==0: return false
  return p1.Weight() < p2.Weight()
```

- `PathInfo().Path()` is the full normalized path with extension and language, for example `/brands/alice/_index.md` or `/pastry/x/index.en.md`.
- `compare.Strings` (`compare/compare_strings.go:22-113`) is an ASCII/simple-fold case-insensitive compare, with a byte compare as tiebreak.
- Weights are all 0 here, so the order is: date desc (seconds), then collated LinkTitle, then path.
- The sort is a strict weak order, so any stable sort (Rust `sort_by`) gives identical results.

### 11.2 Collation

- `collatorStringCompare` (`pages_sort.go:192-200`) uses `langs.GetCollator1(p1.Site().Current().Language())`. `Current()` is the site **being rendered at the time the list is first computed and memoized** (`h.currentSite`, set per site in the render loop, `hugo_sites_build.go:376-386`). For example, the TH site's `Pages()` list is first computed while rendering EN `index.json` (`AllPages`), so it uses the EN collator.
- **For seeksnack this does not matter.** Across all 1503 distinct strings (titles and taxonomy values of both languages), x/text `en`, `th` and `und` give identical comparisons (`W/colltest`, 0 differing pairs out of about 1.13M).
- **x/text uses CLDR 23 / Unicode 6.2** (`$GOMODCACHE/golang.org/x/text@v0.26.0/collate/tables.go:5-9`). Default options: tertiary strength, non-ignorable punctuation (for example `Alice` < `Alice Bakery…` < `alice's` < `Alice's Coconut Mochi`; `INS 1520` < `INS 211`).
- Rust: `icu_collator` 2.3.1 with `locale!("en")` or `und` and `CollatorOptions::default()` matched x/text on **all 1,128,753 pairs** (`W/rcoll`). `locale!("th")` in ICU4X does **not** match. Its CLDR-44+ Thai tailoring (shifted alternate, script reordering) differs from x/text's CLDR-23 `th`. **Use the en/root collator for every language.** Alternatively, port x/text collate for guaranteed parity: `collate/collate.go` 398 lines, `internal/colltab/*` about 1300 lines, `tables.go` 4.9 MB generated, plus `unicode/norm` NFD iteration.
- Byte order differs from collation on 74,340 pairs of these strings. Collation is mandatory.

### 11.3 Other orderings

| Where | Rule | Source |
|---|---|---|
| Translations | lang weight → date desc → `compare.Strings(LinkTitle)` → filename | `pages_sort.go:118-140` |
| Resources of a page | ResourceType string asc (`image` < `page`); pages after non-pages; pages by DefaultPageSort; others by `Name()` byte order; `sort.SliceStable` | `content_map_page.go:594-618` |
| Menu | weight (0 last) → `compare.Strings(Name)` → Identifier | `navigation/menu.go:196-214` |
| Taxonomy views | plural asc (`sort.Slice`, unique keys) | `hugolib/site.go:662-680` |
| Related | Weight desc → PublishDate desc → `Name()` **byte** asc (`sort.Stable` over Go-map iteration order) | `related/inverted_index.go:315-323,532` |
| `Pages.Reverse` | reverse copy | `pages_sort.go:368-380` |
| Template `sort` | stable (`sort.Stable`); keys compared with `compare.LtCollate(collator of rendering site deps)`: numbers numeric, **strings that parse as float compared numerically**, time via Unix seconds, slices/maps by length, other strings by collator | `tpl/collections/sort.go:30-197`, `tpl/compare/compare.go:208-400` |

When the comparator is not a strict weak order, results depend on the exact algorithm. This happens with template `sort .` over the TH `tags` `[]any` containing `nil`, and over the categories list containing a nested slice (length compare against string compare). To be safe, port Go's `sort.Stable` (insertionSort blocks of 20 plus `symMerge`, about 100 lines, `$GOROOT/src/sort/sort.go`) and use it for template `sort`.

---

## 12. Page collections

### 12.1 Predicates (`content_map_page.go:52-86`)

- `ShouldListLocal` / `ShouldListGlobal` / `ShouldListAny` come from `Build.List` (default `always`, so true for all non-standalone pages).
- `ShouldLink` = `Build.Render != never` (true).

### 12.2 Queries

| Accessor | Definition | Order |
|---|---|---|
| `Site.RegularPages` | recursive from `""`, KindPage, ShouldListGlobal | SortByDefault |
| `Site.Pages` | recursive from `""` + home itself, ShouldListGlobal (home, sections, taxonomies, terms, pages; no 404/sitemap/robots) | SortByDefault; **= sitemap `.Data.Pages`**, verified identical to golden sitemap order |
| `Site.AllPages` | `HugoSites.Pages()`: `Site.Pages()` of en ++ th, then SortByDefault (stable, so EN before TH on full ties) | `hugo_sites.go:199-213` |
| `Site.Home` / `.Site.GetPage` | §12.4 | |
| home `.RegularPages` | non-recursive `/`: root single pages only (disclaimer, latesturl, privacy, search, terms) | |
| home `.Pages` | sections + root pages | |
| section `.RegularPages` | direct pages; subtrees of branch nodes skipped (`getPagesInSection` non-recursive, L352-415) | |
| section `.Pages` | pages + direct sub-sections | |
| taxonomy `.Pages` | all term pages below (recursive, KindTerm) | |
| term `.Pages` | entries (any kind), wrapped with Weight0 | |
| `.Parent` | home→nil; page→section (LongestPrefix(ContainerDir) branch); section/taxonomy→home; term→taxonomy; root pages→home | `page__tree.go:116-137` |
| `.CurrentSection` | branch→self; page→LongestPrefix(Dir) branch; root→home | L59-75 |
| `.FirstSection` | | L77-100 |

`.Site.Sections`, `.Sections` and `.Ancestors` are not used by the templates.

### 12.3 Cache keys (`content_map_page.go:267-287`) — replicate the one quirk

- `pageMapQueryPagesInSection.Key() = "gagesInSection/" + Path + "/" + KeyPart + "/" + Recursive + "/" + IncludeSelf`
- `pageMapQueryPagesBelowPath.Key() = Path + "/" + KeyPart`

For **terms**, `RegularPages()` (Include `ShouldListLocal & KindPage`) and `Pages()` (Include nil, meaning `ShouldListLocal`) both use key `"/tags/x/"` in `cachePages1`. Whichever is called first defines both results. Rendering always calls `.Paginator` → `Pages()` first, so term `.RegularPages` returns non-page members too. Seeksnack templates never call `.RegularPages` on a term (the RSS template uses `.Pages` for non-home/non-section nodes), so a clean implementation is observationally equivalent here.

### 12.4 `Site.GetPage` (`hugolib/pagecollections.go:78-256`, `site.go:1406-1420`)

1. `ref` is trimmed of any trailing `/`.
2. Without an extension it becomes `ref+".md"`; `/` becomes `/_index.md`.
3. `PathParser.ParseBaseAndBaseNameNoIdentifier` normalizes (lower-case, spaces become hyphens, language identifiers stripped).
4. The result is looked up with `treePages.Get(base)` in the **current site's language**.
5. With no context and no `/` in the ref, a reverse index by base name is used.

Every `.Site.Taxonomies` key resolves (verified: all 12 sidebar lists have the same number of links as keys). Caveat: a key containing `.en` or `.th` just before the added `.md` would be stripped as a language identifier. That does not occur here.

---

## 13. Pagination

- `pagerSize = 12` (`[pagination]`), path `page`, aliases enabled.
- `Paginate` and `Paginator` share **one** `sync.Once` per page output (`hugolib/page__paginator.go:45-112`). The first call wins, and later `.Paginate(seq)` calls return the already-initialized pager **without error**.
- `partials/head.html` runs before the "content" block (baseof order) and does `{{ if or .IsHome .IsNode }}{{ $pag = .Paginator }}`. So the lists are:
  - home → `s.RegularPages()` (all kinds of page, incl. non-snacks root pages, default sort)
  - term/taxonomy → `.Pages()`
  - section → `.RegularPages()`
  - **404** (kind `404`, IsNode) → default → `s.RegularPages()`

  This was verified: all 1642 paginated HTML files have the expected item sets (items with a non-matching `type` or no image are hidden by templates).
- `splitPages`: chunks of 12 (`resources/page/pagination.go:219-227`). With 0 items there is still 1 pager (L386-404).
- Pager `.URL`: page 1 is the node's RelPermalink; page N is `CreateTargetPaths(desc+Addends "/page/N/").RelPermalink` (L406-417), for example `/biscuit/page/2/` or `/th/page/3/`.
- `renderPaginator` (`hugolib/site_render.go:228-267`): after the page renders, it writes an alias at `CreateTargetPaths(Addends "/page/1").TargetFilename` (HTML formats only; 404 is HTML), then renders pages 2..N, setting `paginator.current` to each pager.
- Pager counts:
  - EN: home 14 (159/12), 404 14, sections ceil(n/12), taxonomies ceil(terms/12), terms ceil(members/12).
  - TH: home 7 (80), 404 7.

---

## 14. Related content

Config: `related.includeNewer=false`, `threshold=80`, `toLower=false`, indices categories(100), brands(80), companies(60), all type `basic`.

- The index is built per page collection. Templates call `.Site.RegularPages.Related .`, which means per language (`resources/page/pages_related.go:55-244`; the handler caches by pages-equality).
- Keywords (`hugolib/page.go:161-172` → `resources/page/page.go:263-318` `NamedPageMetaValue`, default → `resource.Param`) → `IndexConfig.ToKeywords` (`related/inverted_index.go:429-456`):
  - `string` → [kw]
  - `[]string` → each
  - `[]any` → `cast.ToStringSlice` (a nested list gives **none**)
  - `nil` → none

  Keywords are case-sensitive.
- `Add` (L198-239): documents are added in collection order; each keyword maps to a doc list.
- `searchDate(self, upper=self.PublishDate())` (L462-550):
  ```
  for index in [categories, brands, companies]:          // config order
    for kw in doc keywords of that index:
      for d in postings[kw]:
         skip if d == self; skip if !includeNewer && !upper.IsZero() && d.PublishDate().After(upper)
         rank[d]: first hit -> Weight=idx.Weight, Matches=1; else Weight+=idx.Weight, Matches++
  keep r if norm(r.Weight/r.Matches, 0, 100) >= threshold/r.Matches     // integer divisions
  norm(n,min,max)=floor((n-min)/(max-min)*100 + 0.5)
  sort.Stable by (Weight desc, PublishDate desc (After), Name() asc byte)
  ```
  So a single companies-only match (60) is dropped, while one brands (80) or categories (100) match is kept. With two matches the threshold is 40.
- Candidates come from a Go map, so full ties are in random order. No full ties exist in seeksnack (the TH duplicate `(zero date, "")` pages have no keywords).
- Verified: the dump's `Related` first 5 equals the golden "See Also" for all 239 pages.
- `head.html` also computes Related for **nodes** (og:see_also). Nodes have no categories, brands or companies params, so the result is empty.

---

## 15. Content-derived values

| Value | Rule | Used in templates |
|---|---|---|
| `.Content` | rendered markdown (goldmark agent) | pages, simple, term, taxonomy |
| `.Plain` | `tpl.StripHTML(content)` (`tpl/template.go:98-134`): if no `<` or `>`, unchanged; else replace `\n` with space and `</p>`, `<br>`, `<br />` with `___hugonl_`; `htmltemplate.StripTags` (html/template context machine, `tpl/internal/go_templates/htmltemplate/html.go:181-229`); placeholder back to `\n`; collapse runs of `unicode.IsSpace` to the first space char | `index.json` "contents" |
| `.WordCount` | `helpers.TotalWords(plain)` (count of non-space runs; `helpers/content.go:151-162`); CJK variant disabled | — |
| `.ReadingTime` | `(wordCount+212)/213` | only `posts/single.html` (unused) |
| `.Summary` / `.Truncated` | summaryLength 70; front matter `summary` in 1 file | **unused** |
| `.Description` | front matter | everywhere |
| `.Len`, `.FuzzyWordCount` | | unused |

Source: `hugolib/page__content.go:798-853` (contentPlain).

---

## 16. Menus

- Configured per language in `[Languages.*.menu.main]`. The weights are TOML floats `1.0`… and decode to `int`.
- EN order: Snack Companies(1), Snack Brands(2), Snack Countries(3), Snack Categories(4), Snack Ingredients(5).
- TH order: บริษัทขนม(1), ประเภทขนม(2), ขนมของประเทศ(3), ยี่ห้อขนม(3) (the tie is broken by `compare.Strings(Name)`, byte order for Thai), then ส่วนผสมขนม(4).
- URLs are `/companies/` etc., as configured. `.URL()` gives the same.
- **No template references `.Site.Menus`**, so menus do not affect output. The Rust port can load them lazily or skip them.
- Source: `navigation/menu.go`.

---

## 17. Scratch and Store

- `.Scratch` **is** `.Store` (`hugolib/page__common.go:106-113`). There is one `maps.Scratch` per page, shared by all output formats of that page. `newScratch` gives a fresh one.
- `Add` (`common/maps/scratch.go:41-69`):
  - key missing → set the value.
  - existing value is a slice or array → `collections.Append(existing, new)`, so `Add "index" slice` sets `[]any{}` and each later `Add "index" (dict …)` appends one map.
  - otherwise → arithmetic `+`.
- `Set`, `Get` (nil if missing), `Delete` and `SetInMap` behave as expected.
- Uses in seeksnack:
  - `index.json`: home scratch key `index`, per language.
  - `head.html`: `.Scratch.Set "og_image"`.
  - `jsonLd.html` breadcrumb: `newScratch`, `Add` on slices, `range sort ($scratch.Get "pages") "position" "asc"`.

---

## 18. Page resources

- **Ownership** (`content_map_page.go:485-538`): the resources of page P are the resource keys under `P.Path()+"/"`. For branch pages, `path.Dir(resourceKey)` is walked up and the owner is checked by `LongestPrefixAll`. A resource in a sub-bundle belongs to that sub-bundle.
- **Creation** (`assembleResources`, L1735-1854, sequential en then th):
  - For each page in the site's tree, the walk uses `exact = !duplicateResourceFiles`, and goldmark `duplicateResourceFiles` is false by default. So an EN page creates its images: `NameOriginal = relPathOriginal` (original case), `NameNormalized = BaseRel` (lower), `Title = Name`, Target = page's `SubResourceBaseTarget` + original rel path, `LazyPublish = !Build.PublishResources` (false).
  - TH pages find nothing in exact mode and create nothing.
  - `getResourcesForPage` (non-exact) for a TH page then returns the **EN resource objects** (`Shift`, L760-783). So TH pages link to the EN URLs, which is confirmed in golden.
  - A TH-only bundle's images would never be created. None exist, but note it.
- **Order**: §11.3. In practice, images sorted by `Name()` byte order.
- **`.Resources.Get name`** (`resources/resource/resources.go:106-147`): a `./` prefix is relative; otherwise `/` is added to both sides. It compares with `strings.EqualFold` against `Name()` first, then against `NameNormalized()`. It returns the first match or nil. Name `""` or `nil` returns nil.
- **Publishing**: `pageRenderer` calls `p.renderResources()` for every rendered page before rendering its templates (`site_render.go:136-141`, `hugolib/page.go:524-554`). So **every** bundle resource of every rendered page is copied, including unreferenced ones (all 531 images are present in golden). Already-published resources are skipped, so there is no second copy for TH.
- Resource permalinks are escaped (uppercase `%C3%A9` in golden) by the resource spec (images and resources agent).
- `.gitkeep` and other dotfiles are ignored at walk time (§2.1).

---

## 19. Render order (affects memoization and collisions)

`hugo_sites_build.go:376-430`:

- For each site in [en, th], `h.currentSite = s`.
- For each render format in [html, 404, json, robots, rss, sitemap, sitemapindex] (`site.go:800-837`), `renderPages` walks the tree in key order into N parallel workers (`site_render.go:70-191`).
- Standalone pages render once:
  - 404 in outIdx 0.
  - robots and sitemapindex only for language 0.
  - sitemap once per site (`shouldRenderStandalonePage`, L54-67).
- Aliases are rendered separately (`renderAliases`, L270-335). None exist from front matter. The main-language redirect is written separately too.

The Rust port can render sequentially, in this order, deterministically. The only observable consequences are the §7.6 collisions and which collator memoizes a list (irrelevant for this data).

---

## 20. Rust port recommendations

### 20.1 Port line-by-line (Go sources)

| Area | Go files (lines) |
|---|---|
| Path parser and identity | `common/paths/pathparser.go` (787), `common/paths/path.go` Sanitize/PathEscape/Dir (430, about 120 relevant), `common/paths/url.go` MakePermalink/URLEscape (273, about 60) |
| Sanitize and urlize | `helpers/path.go` MakePath/MakePathSanitized (428, about 30), `helpers/url.go` (189), `helpers/pathspec.go` (78) |
| Go `net/url` path escaping | `$GOROOT/src/net/url/url.go` shouldEscape/escape/unescape/validEncoded/setPath/EscapedPath (about 200) |
| Collection and ignore rules | `hugolib/pages_capture.go` (425), `hugofs/component_fs.go` applyMeta/ReadDir (272), `source/sourceSpec.go` IgnoreFile (90) |
| Content map and assembly | `hugolib/content_map.go` (493), `hugolib/content_map_page.go` (2209; the assembly functions §5.2 and queries §12 are about 900 relevant lines), `hugolib/doctree/*` (1110: nodeshifttree, support/WalkContext events, simpletree) |
| Page creation and meta | `hugolib/page__new.go` (265), `hugolib/page__meta.go` (948), `resources/page/pagemeta/pagemeta.go` (103), `resources/page/pagemeta/page_frontmatter.go` (864; the date chain about 350) |
| Paths and URLs | `hugolib/page__paths.go` (181), `resources/page/page_paths.go` (462) |
| Tree navigation, data, translations | `hugolib/page__tree.go` (203), `hugolib/page__data.go` (63), `hugolib/page.go` (785; collections/translations/renderResources about 250) |
| Sorting | `resources/page/pages_sort.go` (431), `compare/compare_strings.go` (113), `resources/page/weighted.go` (138), `resources/page/taxonomy.go` (177) |
| Pagination | `hugolib/page__paginator.go` (112), `resources/page/pagination.go` (417), `hugolib/site_render.go` renderPaginator/aliases (367) |
| Related | `related/inverted_index.go` (632), `resources/page/pages_related.go` (244), `resources/page/page.go` NamedPageMetaValue (about 60) |
| Site collections | `hugolib/site.go` (Pages/RegularPages/Taxonomies/shouldBuild/initRenderFormats/lang prefixes, about 250 of 1613), `hugolib/hugo_sites.go` Pages/RegularPages (about 40), `hugolib/pagecollections.go` (256) |
| Conversions | `common/types/convert.go` (129), spf13/cast v1.9.2 `ToStringE`/`ToStringSliceE`/`ToIntE`/`ToTimeInDefaultLocationE` subsets (about 400) |
| Titles | `jdkato/prose@v1.2.1/transform/title.go` (about 110), `gobuffalo/flect@v1.0.3` pluralize.go + plural_rules.go + ident.go + rule.go + acronyms.go (about 800) |
| Scratch | `common/maps/scratch.go` (160), `common/maps/params.go` PrepareParams (about 40) |
| Resources.Get | `resources/resource/resources.go` (about 50) |
| Menus (optional) | `navigation/menu.go` (318) |
| Stable sort | `$GOROOT/src/sort/sort.go` Stable/insertionSort/symMerge/rotate (about 120) — for template `sort` with non-transitive comparators |
| Plain | `tpl/template.go` StripHTML (40) + html/template `stripTags` and transition funcs (shared with the templates agent) |

### 20.2 Rust crates

**Safe (validated or byte-neutral):**

- `icu_collator` 2.x with the **`en` or `und`** locale and default options. Verified identical to x/text EN on every pair of site strings. Do not use the `th` locale.
- `serde_json`, only for the dump fixtures.
- `toml` / `toml_edit`: TOML datetimes must map to a UTC instant. Keep `time.Time` semantics (location UTC).
- A YAML parser **only if it reproduces yaml.v2 (YAML 1.1) scalar typing**: timestamps kept as strings, ints/floats typed, `null`/`~` → nil, `yes`/`no`/`on`/`off` → bool. `serde_yaml` (YAML 1.2, unmaintained) differs on 1.1 booleans, octals and sexagesimal. No seeksnack value hits these cases (checked: only `null` appears), so it is safe for this site, but it is a general risk.
- `regex` for the AP title split, with `(?-u:\s)` replaced by `[\t\n\f\r ]`. `\p{L}`/`\p{N}` tables must be Unicode 17 to match Go 1.27.
- `unicode-normalization` for NFC of filenames (darwin behavior; names are already NFC).
- A `BTreeMap<String, …>` gives radix-equivalent byte order. Implement longest-prefix by char-level prefix probing.

**Not safe (would break parity):**

- `icu_collator` with `th`, or any CLDR ≥ 24 tailoring: different Thai order.
- Rust `str::to_lowercase`: full case mapping and final sigma. Go uses the simple per-rune mapping: `İ`→`i`, `Σ`→`σ`. Use simple mappings (UnicodeData field 13) for `strings.ToLower`, and `ToTitle` for the AP first rune.
- `char::is_alphabetic` / `is_numeric` / `is_whitespace` in place of Go `IsLetter` / `IsDigit` / `IsMark` / `IsSpace`: the categories differ. Use a general-category crate at Unicode 17, or generated tables.
- `percent-encoding` / `url::Url` for link escaping: different reserved sets and RawPath logic. Port `net/url`.
- `slug` or `deunicode`-style slugifiers: different rules from Sanitize (double hyphens, `.` kept, marks kept).
- Hash maps with random iteration where Go iterates sorted: `.Site.Taxonomies` range is sorted by key.
- `chrono` date parsing: use explicit RFC3339 with milliseconds and keep UTC. Seconds-based `Unix()` comparisons.

### 20.3 Parity risks (ranked)

1. **Term collisions (§7.6)**: 13 output dirs whose content depends on Go worker timing. Golden run1 and run2 already differ on 8 RSS files. Whitelist these paths in acceptance or accept either variant. The deterministic rule "later tree key wins" matches 12/13 golden HTML files.
2. **Paginator initialization order**: `.Paginator` in `head.html` pre-empts `.Paginate` (home list = all RegularPages, not the date-sorted snacks). Getting this wrong changes every home pager page.
3. **Collation**: tie-breaks among 28 same-second EN pages and dozens of same-date term groups decide pager membership and sitemap order. Use the root collator (validated), not byte order and not ICU `th`.
4. **Taxonomy value conversion edge cases**: a nested list means no terms; nil is dropped; a single string is not split; leading and trailing spaces become hyphens in keys but are dropped in `urlize` links.
5. **Key normalization vs output sanitization split**: tree keys keep punctuation; outputs and links sanitize. `.Site.Taxonomies` keys are `ToLower(last raw value)`.
6. **Term metadata ordering**: last value wins for `.Data.Term` and `.Title`; first value for `.Name`; titles are computed only after all terms are assembled.
7. **AP title-case quirks** (rune-count bug, ASCII `\s`, small words) and flect pluralization of section names.
8. **Node date aggregation**: max over descendants; content-backed terms keep their own date; TH `/crepe` and `/sponge-cake` have zero dates (no `lastmod` in sitemap, no `lastBuildDate`).
9. **Home title**: EN file-backed empty `_index.md` gives `""`; TH auto gives `SeekSnack`. RSS and sitemap depend on it.
10. **Resources shared across languages**: TH must reuse EN resource URLs and must not publish duplicates. All resources are published, even unreferenced ones.
11. **Escaping**: uppercase `%XX` for permalinks versus lowercase for template `urlize|lower`; NFC filenames; `#` → `%23`.
12. **404 pagination**: produces `404/page/N.html` files and alias.
13. **Build year**: `now.Format "2006"` appears in the RSS copyright and the footer. The golden output was built in 2026.
14. **Term `.RegularPages` == `.Pages` cache sharing**: not observable with the current templates.
15. **Template `sort` over mixed-type slices**: port Go `sort.Stable` and `LtCollate` exactly.

---

## Appendix A — Section title table (EN; TH identical where present)

pastry=Pastries, seaweed=Seaweeds, cookies=Cookies, cake=Cakes, corn-chips=Corn-Chips, potato-chips=Potato-Chips, crackers=Crackers, wafer=Wafers, biscuit=Biscuits, candy=Candies, marshmallow=Marshmallows, popcorn=Popcorns, ice-cream=Ice-Creams, candy-shell=Candy-Shells, potato-crisps=Potato-Crisps, croissant=Croissants, fruit=Fruits, jelly=Jellies, biscuit-stick=Biscuit-Sticks, vegetable-chips=Vegetable-Chips, almonds=Almonds, peas=Peas, seafood=Seafoods, korean-snacks=Korean-Snacks, tortilla-chips=Tortilla-Chips, cheese-puffs=Cheese-Puffs, rice-crackers=Rice-Crackers, crepe=Crepes, french-fries=French-Fries, biscuit-roll=Biscuit-Rolls, sponge-cake=Sponge-Cakes, pie=Pies, bread-pan=Bread-Pans, fries=Fries, nacho-cheese-chips=Nacho-Cheese-Chips, onion-rings=Onion-Rings, party-mixes=Party-Mixes, pretzels=Pretzels, rice-chips=Rice-Chips, seafood-chips=Seafood-Chips.

## Appendix B — Golden alias and paginator inventory

- `page/1/index.html` aliases: 1478 (EN 856 + TH 622 node dirs, after collision de-dup). Total redirect files = 1478 + 2 (404) + 1 (`/en/`) = 1481.
- `404/page/1.html`, `th/404/page/1.html`.
- `en/index.html`: main-language redirect.
- Each alias body redirects to the node's `Permalink()` (for example `https://seeksnack.com/th/` for TH home `page/1`).

## Appendix C — Quick self-checks the Rust port should pass (from `W/model_full.json`)

1. `site(en).Pages()` mapped to permalinks equals the `<loc>` sequence of `G/en/sitemap.xml`, and likewise for TH.
2. For every node, `ceil(len(list)/12)` equals the number of `page/N` entries in golden, with list = home→`site.RegularPages`, section→`RegularPages`, taxonomy/term→`Pages`. The only exception is the collided `brands/lays`.
3. `Related(page)[:5]` permalinks equal the golden "See Also" `<article><a href>` sequence for all 239 pages.
4. `(sort AllPages "Date").Reverse` filtered by image_preview resource existence equals the `index.json` relpermalinks, for EN and TH (238 each).
5. Every auto term title equals `AP(m.term)`, and every `m.term` equals the last value in key-order walk.
