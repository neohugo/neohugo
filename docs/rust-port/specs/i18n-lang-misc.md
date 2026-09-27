# Spec: config, languages, i18n, string/time helpers, collation, front matter decoding (agent: i18n-lang-misc)

Scope: everything needed so that a Rust port of neohugo reproduces, byte-for-byte, the parts of the seeksnack output that come from
(1) configuration loading and language setup, (2) `i18n`/`T`, (3) the string/URL/time/compare/collection helpers the seeksnack
templates call, (4) locale collation used by sorting, (5) YAML/TOML/JSON decoding of front matter, config, i18n and data files.

All Go line numbers refer to `/Users/blackb1rd/git/github/org/neohugo` (HEAD d5930ba1f, binary reports v0.149.0-DEV, built with go1.27.1)
and to the module cache `~/go/pkg/mod` (versions from go.mod). "golden" = `scratchpad/golden/run1`.

Work artifacts (all under `scratchpad/work/i18n-lang-misc/`):

| file | what |
|---|---|
| `config-dump.toml`, `config-dump-th.toml`, `config-dump-zero.toml`, `config-dump.json` | `neohugo-go config` (en, `--lang th`, `--printZero`, json) run in the site dir |
| `fm.json`, `fm-types.txt` | every content file's front matter decoded with neohugo's own `pageparser.ParseFrontMatterAndContent`, plus Go dynamic types per key |
| `fixtures-strings.json` | 1447 inputs (all titles/taxonomy values/section names + edge cases) -> Go outputs for `paths.Sanitize`, `urlize`, `urlize|lower`, `humanize`, `lower`, AP `CreateTitle`, `flect.Pluralize`, `CreateTitle(Pluralize)` |
| `fixtures-dates.json` | en/th x 7 layouts x 4 inputs -> Go `dateFormat` (localized) + `time.Time.String()` |
| `fixtures-i18n.json` | every i18n key used by the site x en/th x 6 template-data shapes -> exact go-i18n output |
| `coll/xtext-und-data{en,th}.txt` | x/text/collate order of all 1575 (en) / 1187 (th) strings the site can sort; `coll/icu4x-*.txt` ICU4X orders; `coll/bytes-*.txt` inputs |
| `goexp/cmd/{extract,coll,collcmp,fixtures,i18nfix,numstr}` | Go programs (module with `replace github.com/neohugo/neohugo => repo`) that produced the above |
| `rcoll/` | Rust program comparing icu_collator 2.3.1 against x/text |
| `build-cold/`, `res-empty/` | a cold-cache Go rebuild (HUGO_RESOURCEDIR=res-empty) which reproduces golden (only the 7-8 known RSS collision files differ) |

---------------------------------------------------------------------------------------------------------------------------

## 0. Key findings (read this first)

1. **Collation matters and ICU4X "th" is WRONG.** Template `sort` on strings/pages (`sort . `, `sort .Paginator.Pages "Title"`) and
   `DefaultPageSort` tie-breaks use `golang.org/x/text/collate` (UCA 6.2 / CLDR 23). For the site's strings x/text `th` == `en` == `und`
   (0 differences). ICU4X `icu_collator` with locale `und` or `en` reproduces x/text for **all 2762 sortable strings** (0 diffs);
   with locale `th` it differs in 1187/1187 positions (CLDR>=24 Thai tailoring has `[reorder Thai]` + `alternate=shifted`;
   golden shows Latin ingredients sorted *before* Thai ones, i.e. x/text behaviour). Only known x/text vs ICU4X root divergence
   observed: U+2019 vs U+0027 (in descriptions, which are never sorted). Rust: use root collation for every language, or port x/text.
2. **`dateFormat` is localized; `.Date.Format` is not.** `dateFormat "Jan 2, 2006"` on the th site prints `ส.ค. 14, 2020`
   (gohugoio/locales th month abbreviations substituted by string replace). `.Date.Format`/`.Lastmod.Format`/`now.Format` are Go's
   `time.Time.Format` (English). `{{ .PublishDate }}` prints Go `time.Time.String()` (`2020-08-14 14:59:12.921 +0000 UTC`).
3. **`humanize` mangles Thai** (flect drops combining marks: `ญี่ปุ่น` -> `ญ ป น`, visible in golden) and **ordinalizes integers**
   (`"3"` -> `3rd`). `urlize` keeps Thai, percent-encodes with UPPERCASE hex; the site's `| lower` then lowercases the hex
   (golden: `/th/tags/%e0%b8%84...` vs `/th/countries/%E0%B8%9B...`).
4. **i18n**: all 70 keys the live layouts use exist in both `en.toml` and `th.toml`; plural count is always nil (dict args
   without `Count`) -> form `other`; messages with `{{ .Context }}` are executed by Go `text/template` (floats print `0.5`,
   nil prints `<no value>`); missing key in th falls back to en; missing everywhere -> `""`.
5. **Config**: keys are lower-cased recursively (maps only, not inside arrays of tables); `rssLimit=10` -> `services.rss.limit=10`;
   `pygments*` -> `markup.highlight` (style monokai, codeFences, guessSyntax, `linenos=table` -> lineNos+lineNumbersInTable);
   `footnoteReturnLinkContents` is ignored (no reference anywhere in neohugo); `canonifyURLs=true` changes relURL and turns on the
   absolute-URL output transformer. Per-language `LanguageCode` overrides root `languageCode` (`en-us` -> `en`).
6. **Go-isms that leak into bytes**: `printf "%s" nil` -> `%!s(<nil>)` (1678 golden files contain `og:image content="%!s(<nil>)"`),
   Go float formatting (`%v`: shortest digits, exponent iff exp<-4 || exp>=6, e.g. `1e+06`; jsonify: exponent only
   below 1e-6 / from 1e21 — see 3.7), Go simple case mapping
   (`İ`->`i`, `Σ`->`σ` never `ς`), Go `sort.Stable` with non-strict comparators (a `nil` tag stays last), yaml.v2 YAML-1.1 scalar
   resolution, numeric-string comparison in `ge/lt/sort` (`ge "2019" "2026"` compares as floats).
7. **Golden reproducibility (cross-cutting, not my subsystem)**: a Go rebuild *with the current `resources/_gen` cache* differs from
   golden in ~4690 paths (image `_hu_<hash>` names change because the hash chains over the bytes of cached intermediate images).
   A rebuild with an empty resource dir (`HUGO_RESOURCEDIR=<empty dir>`) reproduces golden exactly (only RSS collision files differ).
   The acceptance harness must build golden and Rust output from a cold resource cache. `now` (copyright year 2026) and the
   YouTube `resources.GetRemote` JSON-LD data (served from `~/Library/Caches/hugo_cache/seeksnack`) are also environment inputs.

---------------------------------------------------------------------------------------------------------------------------

## 1. Configuration loading

### 1.1 Pipeline (config/allconfig/load.go `LoadConfig` L43-97, `loadConfigMain` L294-414)

1. `d.Environ = os.Environ()` (L44-46). Flags (from CLI) are applied first (L307-318) to find `workingDir`/`themesDir`.
2. Config file discovery (`loadConfig` L479-526): names `config.DefaultConfigNames = ["hugo","config"]`
   (config/configLoader.go L35), extensions in order `toml, yaml, yml, json` (L39); first existing wins (`break` L338).
   seeksnack: `hugo.toml`. No `config/` dir (ConfigDir default "config" -> `LoadConfigFromDir` returns nothing).
3. `config.FromFileToMap` -> `metadecoders.Default.UnmarshalFileToMap` (TOML via pelletier/go-toml/v2) -> `RenameKeys`
   (configLoader.go L214-231: key renamer `{menu,languages/*/menu}` -> `menus`). seeksnack: `Languages.en.menu.main` -> `menus`.
4. `l.cfg.Set("", m)` (defaultConfigProvider.go L116-131): `maps.ToParamsAndPrepare` -> **all map keys lower-cased recursively**
   (common/maps/params.go `PrepareParams` L310-346). `PrepareParams` recurses only into `map[string]any`, `map[any]any`,
   `map[string]string`; it does **not** descend into `[]any`, so array-of-tables entries (`[[params.footer.socialmedia]]`,
   `[[...column1.links]]`, `[[related.indices]]`, `[[module.mounts]]`, `[[Languages.en.menu.main]]`) keep original key case
   (in seeksnack those keys are already lower case). Merge-strategy key `_merge` is special (L316).
5. `normalizeCfg` (L172-180): `minify = true` (bool) -> `minify.minifyOutput`. `cleanExternalConfig`: drops `internal`.
6. `applyDefaultConfig` (L160-170): `themesDir="themes"`, `configDir="config"` only if unset.
7. `applyOsEnvOverrides` (L196-270), applied twice (L375, L405): every env var starting `HUGO` whose next char is the delimiter
   (usually `_`): key = rest with delimiter -> nested path, lower-cased; value parsed with
   `metadecoders.UnmarshalStringTo(value, existingValue)` (type of the existing value: string/int/int64/float64/bool/map/slice;
   decoder.go L102-125); `disablekinds`/`disablelanguages` split on `,` or whitespace (L281-292). Verified: `HUGO_RESOURCEDIR=...`
   works. The Rust port must implement this (the harness may pass `HUGO_ENVIRONMENT`).
8. `SetDefaultMergeStrategy` (defaultConfigProvider.go L319-336; strategy rules L263-317: `params` deep, root
   `outputformats`/`mediatypes` shallow, root or `languages/*/menus` shallow, else none). The strategies are stored as `_merge`
   keys inside the Params maps; they are stripped again by `deleteMergeStrategies` (deferred in `LoadConfig`, load.go L56,
   L528-533) and by `maps.CleanConfigStringMap` (params.go L268-293) in the `params` decoder, so templates never see `_merge`.
9. `applyConfigAliases` (L142-158): `indexes`->`taxonomies`, `logI18nWarnings`->`printI18nWarnings`,
   `logPathWarnings`->`printPathWarnings`, `ignoreErrors`->`ignoreLogs`.
10. `fromLoadConfigResult` (allconfig.go L1025-1153) -> `Configs.Init` (L839-977).

Case-insensitivity: every `Get/Set/IsSet` lower-cases the key and splits on `.` (defaultConfigProvider.go L52-65, L338-366).
Struct decoding uses `mapstructure.WeakDecode` whose default field matching is case-insensitive, and "weak" conversions
(e.g. TOML float `weight = 1.0` -> `int`, string "true" -> bool, single value -> slice).

### 1.2 Decoding into `Config` (config/allconfig/alldecoders.go)

`decodeConfigFromParams` (allconfig.go L1155-1189) runs the decoders of `allDecoderSetups` (alldecoders.go L61-452) sorted by
`(weight, key)`: key `""` (weight -100, RootConfig via WeakDecode, then `DefaultContentLanguage` lower-cased L64-74) first,
`contenttypes` and `related` (weight 100) last. Relevant decoders for seeksnack:

| key | Go | seeksnack effect |
|---|---|---|
| `params` | L213-229 `maps.CleanConfigStringMap(GetStringMap("params"))` | lower-cased `maps.Params`; `mainsections` absent |
| `taxonomies` | L253-261 `CleanConfigStringMapString` | 6 taxonomies (default would be tag/category) |
| `languages` | L275-311 -> `langs.DecodeConfig` (langs/config.go L47-58, WeakDecode into `LanguageConfig{LanguageName,LanguageCode,Title,LanguageDirection,Weight,Disabled}`) | 13 languages; validation that `defaultContentLanguage` exists |
| `services` | config/services/servicesConfig.go `DecodeConfig` L88-110 | `services.rss.limit` unset -> `cfg.GetInt("rssLimit")` = 10; if 0 -> -1 (L102-107) |
| `pagination` | L352-366 defaults `{PagerSize:10, Path:"page"}` then WeakDecode | `pagerSize=12` |
| `page` | L336-351 defaults NextPrev*SortOrder `desc` | |
| `sitemap` | L243-252 `config.DecodeSitemap` from default `{Priority:-1, Filename:"sitemap.xml"}` | `changefreq=weekly` |
| `related` | L262-275 | threshold 80, includeNewer false, toLower false, indices categories/brands/companies weights 100/80/60 type basic |
| `markup` | `markup_config.Decode` + `highlight.ApplyLegacyConfig` (markup/highlight/config.go L163-190) | see 1.5 |
| `minify` | `minifiers.DecodeConfig` | tdewolff html: keepDefaultAttrVals, keepDocumentTags, keepEndTags, keepSpecialComments(=legacy keepConditionalComments) true; keepQuotes/keepWhitespace/keepComments false |
| `outputs` | L189-204: lower-cases format names, fills defaults from `createDefaultOutputFormats` (L1191-1217) | home `[html json rss]`, page `[html]`, section/taxonomy/term `[html rss]`, `rss:[rss]` |
| `permalinks` | `page.DecodePermalinksConfig` | legacy flat `posts = "/posts/:year/:month/:title"` shows up under both `page` and `term` (no posts section exists) |
| `build` | `config.DecodeBuildConfig` | legacy `writeStats=true` -> `build.buildStats.enable=true` |
| `imaging` | `images.DecodeConfig` | quality 75, resampleFilter box, hint photo, bgColor #ffffff, exif excludeFields `.*` |

`CompileConfig` (allconfig.go L254-505): timeout "60s"; `disableLanguages` -> `Languages[x].Disabled=true` (L313-326; error if the
default language is disabled); `BaseURL` compiled via `urls.NewBaseURLFromString` (common/urls/baseURL.go: forces trailing `/` on
path; `WithPath="https://seeksnack.com/"`, `WithoutPath="https://seeksnack.com"`, `BasePath="/"`); `CreateTitle =
helpers.GetTitleFunc(TitleCaseStyle="AP")` (L495); `Clock` from `--clock` (L370-377; commands/commandeer.go L205/L311 install
`htime.Clock = clocks.Start(clock)`).

Defaults (`newDefaultConfig`, allconfig.go L996-1022): `Environment="production"`, `TitleCaseStyle="AP"`,
`PluralizeListTitles=true`, `CapitalizeListTitles=true`, `StaticDir=["static"]`, `SummaryLength=70`, `Timeout="60s"`, dirs.

### 1.3 Languages (allconfig.go L1045-1133, L839-977; langs/language.go)

* For each `languages.<key>` (keys lower-cased: `zh-cn`, `zh-tw`) a per-language config is produced (L1050-1124):
  if `params` missing it is added with deep merge (L1055-1060); each language key `kk` is `Set` into `mergedConfig`; if the root
  has a *different* value: maps (`params`, `menus`, `taxonomies`) are cloned from the language value and the root value is merged
  under it (`maps.MergeParams` only adds missing keys => **language wins**), scalars trigger re-decoding of the root struct from
  the merged language map (key `""`). If nothing differs, the language shares the root config (`langConfigMap[k] = all`).
  Multilingual single-host: `markup.goldmark.renderHooks.{image,link}.useEmbedded` `auto` -> `fallback` (L1108-1115).
* `Configs.Init`: languages sorted by `(Weight, key)` (L852-864) -> en(1), th(2), fr(3), pl(4), pt(5), de(6), es(7), zh-cn(8),
  zh-tw(9), ja(10), nl(11); `langs.NewLanguage(k, defaultContentLanguage, TimeZone, conf)` for each (L879); **disabled
  languages filtered out** (L887-895) => `Languages = [en, th]`; `LanguagesDefaultFirst = [en, th]`.
* `IsMultilingual() = len(Languages) > 1` (configlanguage.go L84-86) => true.
* `LanguagePrefix()` (configlanguage.go L50-59): `""` for the default language (en) because `defaultContentLanguageInSubdir=false`,
  `"th"` for th. This drives `/th/...` URLs and `relLangURL/absLangURL`.
* `langs.NewLanguage` (language.go L55-93): translator `localescompressed.GetTranslator(lang)` (fallback default language, then
  "en"); collators `collate.New(language.Parse(lang))` (fallback English); location `time.LoadLocation(timeZone)` where timeZone is
  `""` => **UTC**. `Language.LanguageCode()` returns config LanguageCode or Lang (L108-113). `Language.Params()` returns the site
  params (set with `SetParams`).
* Effective per-language values (from `config-dump*.toml`):

| | en | th |
|---|---|---|
| title (`.Site.Title`) | SeekSnack | SeekSnack |
| languageCode (`.Site.LanguageCode`, `<html lang>`) | `en` (root `en-us` overridden) | `th` |
| languageName | `🇺🇸 English` | `🇹🇭 ไทย` |
| params.hometitle | Snack Food Diary | สมุดไดอารีขนมขบเคี้ยว |
| params.description | The diary of seeking the snack food | ไดอารี่ของการแสวงหาขนมขบเคี้ยว |
| params.languagecode / languagecodeopengraph | en / en_US | th / th_TH |
| menus.main (unused by layouts) | 5 English entries, weights 1..5 | Thai names, weights 1,3,3,2,4 |
| taxonomies | same 6 | same 6 (identical, DeepEqual -> no override) |
| URL prefix | `` | `th` |

### 1.4 Effective root config (non-default values; full dumps in `config-dump*.toml`)

`baseurl='https://seeksnack.com/'`, `canonifyurls=true`, `defaultcontentlanguage='en'`, `defaultcontentlanguageinsubdir=false`,
`disablelanguages=[de es fr ja nl pl pt zh-cn zh-tw]`, `disablepathtolower=false`, `removepathaccents=false`, `enablerobotstxt=true`,
`enablemissingtranslationplaceholders=false`, `printi18nwarnings=false`, `hascjklanguage=false`, `timezone=''`, `titlecasestyle='AP'`,
`pluralizelisttitles=true`, `capitalizelisttitles=true`, `summarylength=70`, `environment='production'`, `relativeurls=false`,
`enableemoji=false`, `enablegitinfo=false` (so the `:git` lastmod handler is inert), `frontmatter.date=[date publishdate pubdate
published lastmod modified]`, `frontmatter.lastmod=[:git lastmod modified date publishdate pubdate published]`,
`frontmatter.publishdate=[publishdate pubdate published date]`, `pagination.pagersize=12`, `services.rss.limit=10`,
`sitemap={changefreq weekly, filename sitemap.xml, priority -1}`, module mounts: 8 configured + 2 defaults added by
`modules.ApplyProjectConfigDefaults` (`package.json` -> `assets/_jsconfig/package.json`, `postcss.config.js` ->
`assets/_jsconfig/postcss.config.js`). `params` values keep their TOML types: `yearcreate="2019"` (string),
`carousel.totalshow=5` (int64), `sitemap.priority.page=1` (int64, prints `1`), `.term=0.5`/`.section=0.3`/`.taxonomy=0.1`
(float64, print `0.5`...), `comment.enabled=true`, `hidecopyright=false`, `dynamiccontent=false`.

### 1.5 Deprecated / legacy / ignored keys in hugo.toml

| key | handling in neohugo | effect |
|---|---|---|
| `rssLimit = 10` | servicesConfig.go L102-107 fallback | `.Site.Config.Services.RSS.Limit` = 10 (used by rss.xml `first $limit`) |
| `pygmentsstyle = "monokai"` | markup/highlight/config.go `ApplyLegacyConfig` L164-169 (only if style is default) | `markup.highlight.style=monokai` |
| `pygmentscodefences = true` | L175-177 (`IsSet("pygmentsCodeFences")`, case-insensitive) | codeFences true (already default) |
| `pygmentscodefencesguesssyntax = true` | L179-181 | `guessSyntax=true` |
| `pygmentsOptions = "linenos=table"` | L183-187 `applyOptionsFromString` | `lineNos=true`, `lineNumbersInTable=true` |
| `footnoteReturnLinkContents = "↩"` | **no reference anywhere** in neohugo (`grep -r` empty) | ignored; goldmark default backlink applies |
| `canonifyurls = true` | `Cfg.CanonifyURLs()` used by `PathSpec.RelURL` (helpers/url.go L122, L161-163) and the canonify output transformer | relURL returns root-relative `/…` (no context root join); final HTML/XML has `href="/…"` rewritten to `https://seeksnack.com/…` (golden: all hrefs absolute) |
| `languageCode = "en-us"` (root) | overridden by `Languages.en.LanguageCode` | effectively unused |
| `keepConditionalComments` (minify) | minifiers/config.go L111-118 renames it | `keepSpecialComments=true` |
| `[build] writeStats = true` | config/commonConfig.go L188-195 (bool form) | `build.buildStats.enable=true` (hugo_stats.json written) |
| `LanguageName`, `LanguageCode` etc. mixed case | lower-cased keys, WeakDecode | fine |

### 1.6 Access semantics that matter for templates

* `.Site.Params` / `.Language.Params` / `.Params` are `maps.Params` whose keys are lower case; the template engine resolves
  `.Site.Params.HomeTitle` case-insensitively (lower-cases the field name for `maps.Params`). `index $footer "logo"` is a raw map
  index (case-sensitive) and only works because the key is already lower case.
* Elements of arrays-of-tables are plain `map[string]any` (not Params): `.link`, `.text`, `.icon`, `.name`, `.type` are exact
  (case-sensitive) lookups.
* `.Site.BaseURL` = `WithPath` = `https://seeksnack.com/` (hugolib/site.go L513-515). `.Site.Title` = config Title (L462).
  `.Site.LanguageCode` = `Language().LanguageCode()` (L477-479). `site.Languages` = enabled languages sorted.
* No generator meta tag is injected: `transform/metainject` exists but is not wired in hugolib (golden has no `<meta name=generator>`).

---------------------------------------------------------------------------------------------------------------------------

## 2. i18n

### 2.1 Loading (langs/i18n/translationProvider.go)

* `NewResource` L48-82: `defaultLangTag = language.Parse(defaultContentLanguage)` (fallback English);
  `bundle := i18n.NewBundle(defaultLangTag)` (gohugoio/go-i18n/v2 fork `i18n/bundle.go` L83-91: default rules from CLDR plural
  tables, artificial `art` tag rule = English, `addTag(default)` => the default language tag is **first** in `bundle.tags`).
  Unmarshalers registered: `toml` = pelletier/go-toml/v2, `yaml`/`yml` = gopkg.in/yaml.v2, `json` = encoding/json.
* Every file under the `i18n` mount is walked (hugofs Walkway, sorted by name) and passed to `addTranslationFile` (L86-121):
  `lang := paths.Filename(name)` (e.g. `zh-CN`), `language.Make(lang)`; if `Und` -> artificial `art-x-<lang>` name; parse with
  `bundle.ParseMessageFileBytes(bytes, name)`; if the error mentions "no plural rule" retry with the `art-x-` prefix.
  Bundle tag order observed: `en,de,es,fr,id,ja,ko,nl,pl,pt,th,zh-CN,zh-TW`.
* Message extraction (go-i18n `i18n/parse.go` L21-139, `message.go`): the file is unmarshalled into `interface{}`; a map is a
  *message* iff it contains a reserved key (`id description hash leftdelim rightdelim zero one two few many other`, case-sensitive
  lookup in `isMessage`) with a string value; otherwise it is a namespace and children get IDs `parent.child`. A bare string value
  is `{other: value}`. Inside a message, reserved keys are matched with `strings.ToLower(k)`; unknown keys ignored; a nil value is
  skipped, non-string values are an error. **Message IDs are case-sensitive and not lower-cased** (`i18n "404_pagenotfound"`).
  Later messages with the same ID overwrite earlier ones (map assignment, bundle.go L157-159).
* seeksnack i18n files use the `[id]` table + `other = "..."`/`one = "..."` format (TOML). All 13 files parse.

### 2.2 Lookup (go-i18n `i18n/localizer.go`)

`NewLocalizer(bundle, "th")` -> `parseTags` via `language.ParseAcceptLanguage`. `LocalizeWithTag` (L134-179):
1. If `PluralCount != nil`: `operands = plural.NewOperands(count)`; if `TemplateData == nil` it becomes `{"PluralCount": count}`.
2. `getMessageTemplate` (L181-207): `_, i, _ := bundle.matcher.Match(tags...)`, `tag := bundle.tags[i]` (x/text language
   matcher; for exactly-present tags `en`/`th` it returns them). Message in that tag -> use it. Else if the tag is the default
   language -> `MessageNotFoundErr` and no template. Else **fallback to the default-language message** (returned with
   `tag=default` and a `MessageNotFoundErr`). Else not found.
3. Plural form: `Other` when operands is nil, else `bundle.pluralRules.Rule(tag).PluralFormFunc(operands)`.
   `en`: `one` iff `i == 1 && v == 0` else `other` (internal/plural/rule_gen.go L46-55); `th`: always `other` (L9-14).
4. `template.Execute(form, data)`; if that form is missing (`pluralFormNotFoundError`) and form != Other, retry with Other
   (L163-177).

### 2.3 Hugo translate func (langs/i18n/i18n.go)

`Translator.Func(lang)` (L50-63) returns the func for `lang` (keys are `strings.ToLower(tag.String())` with `art-x-` stripped,
L67-73); unknown language -> the default content language's func; no bundle at all -> func returning `""`.
Per call (L73-131):
* `pluralCount := getPluralCount(templateData)` (L146-181): nil -> nil; `map[string]any` -> value of the first key equal-fold to
  `"Count"`; struct -> field `Count` or method `Count()`; otherwise `toPluralCountValue(v)` (L184-205): floats -> string with
  ".0" appended if no dot; numeric strings as is; other strings -> nil; ints (via `cast.ToIntE`) -> int; anything else nil.
  **A dict without `Count` (seeksnack's `(dict "Context" .)`) yields nil => plural form `other`.**
* Int template data is wrapped as `intCount` (has `Count()`); `page.Page` wrapped as `PageWithContext`.
* Result: `err == nil && sameLang` -> translated. `err != nil && sameLang && translated != "" && %T == "i18n.pluralFormNotFoundError"`
  -> translated. Otherwise: warn unless `MessageNotFoundErr`; `printI18nWarnings` (false) -> warn line;
  `enableMissingTranslationPlaceholders` (false) -> would return `"[i18n] " + id`; else **return `translated`** (which is the
  default-language text for a fallback, `""` when missing everywhere, or whatever partial output).
* `tpl/lang/lang.go` `Translate` L48-63: at most one data arg; id via `cast.ToStringE`. Aliases `i18n` and `T`.

### 2.4 Message template execution (go-i18n `internal/template.go` L21-51)

* Fast path: if the source does not contain the left delimiter (`{{` default) the raw string is returned (no parsing at all).
* Otherwise Go **`text/template`** (not html/template), `Delims(left,right)`, no FuncMap (`funcs == nil` -> parsed once, lazily at
  first execution). Standard text/template built-ins exist (`printf`, `len`, `index`, `html`, `js`, `eq`, …).
* Execution with `data`: `{{ .Context }}` on `map[string]any` prints the value with text/template's `printValue` ->
  `fmt.Fprint`: string raw; int/int64 decimal; float64 Go `%v` (`0.5`, `2.5`, `4.5`); nil data or nil value -> `<no value>`;
  a missing map key -> `<no value>`. No HTML escaping here.
* The returned string is then inserted by the page template (html/template) as a *string* => contextually escaped by the page
  template (`&`->`&amp;`, `<`->`&lt;`, `'`->`&#39;`, `"`->`&#34;`, `+`->`&#43;`); Thai is not escaped.

### 2.5 seeksnack usage and verified outputs

* 70 distinct keys used by layouts (`used-keys.txt`); every one exists in both en.toml and th.toml except `readingTime`
  (only in `layouts/posts/single.html`, no posts section => never rendered). So fallback is not exercised by the golden.
* Calls: `i18n "key"` (no data) and `i18n "key" (dict "Context" X)` with X = int (YAML), float64 (e.g. `servings_per_container:
  2.5`, `0.5`) or string (`serving_size: "1/3 (Pack) (30 g)"`, th `กล่อง : 1`).
* Golden checks: `pastry/alices-coconut-mochi` en `0.5 servings per container`, th `จำนวนหน่วยบริโภคต่อ0.5`, th `หนึ่งหน่วยบริโภค : 2`;
  `i18n "categories"` -> `Categories`/`หมวดหมู่`; all verified outputs in `fixtures-i18n.json`.

### 2.6 Rust implementation

Port line-by-line: `langs/i18n/i18n.go` (205), `translationProvider.go` (139), go-i18n `bundle.go` (193), `parse.go` (166),
`message.go` (221), `message_template.go` (65), `localizer.go` (223), `internal/template.go` (51), `internal/plural/{operands.go
120, rule.go 44, rules.go 24, form.go 16}` + the rule_gen entries for the languages that exist (at minimum en and th; generating
the whole 589-line table is cheap). Language matching: exact lower-cased tag match + default-language fallback is sufficient for
this site (x/text `language.NewMatcher` semantics only matter for tags not present in the bundle — document as risk).
Message execution: reuse the port's Go-template engine in *text* mode with no Hugo funcs (must print `<no value>` for nil and
Go-format numbers).

---------------------------------------------------------------------------------------------------------------------------

## 3. Helper functions used by the templates (exact algorithms)

Template-function inventory for seeksnack (from `layouts/**`): `i18n`(111), `printf`(45+), `print`(1, head.html L142),
`dict`, `slice`, `first`, `after`, `index`, `isset`, `seq`, `apply`, `where`, `sort`, `len`, `default`, `eq/ne/ge/gt/le/lt`,
`and/or/not`, `add/sub/mul`, `humanize`(14+), `urlize`(14), `lower`(6), `trim`(2), `relLangURL`(32), `absLangURL`(4),
`absURL`+`urldecode` (development only), `dateFormat`(5), `now`(6), `.Format` on times, `jsonify`(1), `safeHTML/safeCSS/safeJS/safeURL`,
`html` (text/template builtin), `markdownify` (comments), `newScratch`/`.Scratch.{Set,Get,Add,SetInMap,Delete}`,
`reflect.IsSlice`, `path.Ext`, `hugo.Environment`, `hugo.Generator` (dev only), `site.*`, `resources.*`, `js.Build`,
`images.*`, `toCSS`, `postCSS`, `minify`, `fingerprint`, `transform.Unmarshal` (other agents). Not used: `anchorize`,
`plainify`, `htmlEscape/htmlUnescape`, `title`, `truncate`, `substr`, `split`, `replace`, `upper`, `countwords`, `delimit`,
`in`, `intersect`, `union`, `uniq`, `shuffle`, `md5`, `sha256`, `base64*`, `math.*`, `strings.*` namespaced.

### 3.1 `urlize` (tpl/urls/urls.go L75-81 -> helpers/url.go `URLize` L30-32)

```
URLize(s) = URLEscape(MakePathSanitized(s))
MakePathSanitized(s) = disablePathToLower ? MakePath(s) : strings.ToLower(MakePath(s))      (helpers/path.go L59-64)
MakePath(s) = paths.Sanitize(s) [+ text.RemoveAccentsString if removePathAccents]            (helpers/path.go L43-49)
URLEscape(u) = url.Parse(u).String()  (panics on parse error)                                   (helpers/url.go L41-50)
```
`paths.Sanitize` (common/paths/path.go L282-324) + `isAllowedPathCharacter` (L326-337):
* allowed rune r: `r != ' '` and (`unicode.IsLetter` (L*) or `unicode.IsDigit` (Nd) or one of `. / \ _ # + ~ - @` or
  `unicode.IsMark` (M*) or (`r == '%'` and the next two *bytes* are hex digits)). Uses Go 1.27 Unicode tables = **Unicode 17.0.0**.
* Fast path: if every rune is allowed return s unchanged.
* Else build output: for allowed r: `wasHyphen = (r=='-')`; if `prependHyphen` then append `'-'` unless `wasHyphen`, clear
  flag; append r. For a disallowed r: if `len(target) > 0 && !wasHyphen && unicode.IsSpace(r)` set `prependHyphen` (so
  leading spaces vanish, trailing spaces vanish, runs of spaces collapse, other disallowed chars are just dropped and do NOT
  reset `wasHyphen`). `unicode.IsSpace` = the Unicode White_Space property (Latin-1: `\t \n \v \f \r ' ' U+0085 U+00A0`; equals Rust `char::is_whitespace` modulo Unicode version).
* `strings.ToLower`: per-rune Go *simple* case mapping (`unicode.ToLower`); `İ`(U+0130) -> `i`, `Σ` -> `σ` (no final sigma).
  Rust `str::to_lowercase` is NOT equivalent (full mapping + final sigma).
* `url.Parse(x).String()` for these relative strings: `#` starts a fragment (empty fragment is dropped: `C#` -> `c`),
  path is re-escaped with Go `escape(path, encodePath)`: unreserved `A-Za-z0-9-_.~` and `$ & + , / : ; = @` kept, everything
  else (incl. all non-ASCII UTF-8 bytes) `%XX` with **uppercase** hex; valid existing escapes are preserved through `RawPath`.
* Examples (all in `fixtures-strings.json`): `Lay's`->`lays`; `Pepsi-Cola (Thai) Trading Co.,Ltd.`->`pepsi-cola-thai-trading-co.ltd.`;
  `a - b`->`a-b`; `x%20y`->`x%20y`; `x%zz`->`xzz`; `ประเทศไทย`->`%E0%B8%9B%E0%B8%A3...`; `Ünïcödé Straße`->`%C3%BCn...-stra%C3%9Fe`.
* Site pattern `($val | urlize | lower)` lower-cases the hex digits too (golden `/th/tags/%e0%b8%84…/`), whereas brands/companies/
  countries use `urlize` only (golden `/th/countries/%E0%B8%9B…/`).

### 3.2 `relLangURL`, `absLangURL`, `relURL`, `absURL` (tpl/urls/urls.go L43-50, L65-72, L168-187; helpers/url.go L53-174)

`multihost=false` so `addLanguage = true` for the Lang variants. `baseURL := getBaseURLRoot(in)` = `WithoutPath`
(`https://seeksnack.com`) if `in` starts with `/`, else `WithPath` (`https://seeksnack.com/`) (L94-102).

`AbsURL(in, addLanguage)` (L53-92): `IsAbsURL(in)` (L104-114: fast path `http://`/`https://` prefix, else `url.Parse(in).IsAbs()`);
**parse error -> return `in` unchanged** (this is why `absLangURL (printf "%s" nil)` yields `%!s(<nil>)`); absolute or `//`
prefix -> unchanged. If addLanguage and prefix (`th`) non-empty and `in` (minus one leading `/`) is neither `th` nor starts with
`th/`: `addSlash := in == "" || HasSuffix(in,"/")`; `in = path.Join(prefix, in)` (+`/` if addSlash). Return
`paths.MakePermalink(baseURL, in).String()` (common/paths/url.go L59-87: `base.Path = path.Join(base.Path, p.Path)`, copy
fragment/query, restore trailing slash if `plink==""&&host ends '/'` or `p.Path` ends `/`; `String()` percent-escapes the path).

`RelURL(in, addLanguage)` (L116-174): parse error -> `in`; if `(!HasPrefix(in, baseURL) && isAbs) || HasPrefix(in,"//")` -> `in`;
strip `baseURL` prefix; language prefix as above but with `hadSlash := HasSuffix(u,"/")`; **canonifyURLs=true -> no
`AddContextRoot`**; if `in == ""` and u lacks trailing `/` and baseURL has it -> add `/`; ensure leading `/`.

Observed: header `{{ "/" | absLangURL }}` -> en `https://seeksnack.com/`, th `https://seeksnack.com/th/`; `"brands/" | relLangURL`
-> `/brands/` / `/th/brands/` (canonify transformer later makes them absolute); `og:image` = `absLangURL (printf "%s" .Params.image)`
-> `https://seeksnack.com/salted_egg_flavor_preview.jpg` (en), `https://seeksnack.com/th/600x480.jpg` (th), `%!s(<nil>)` when no image.

### 3.3 `humanize` (tpl/inflect/inflect.go L37-54, gobuffalo/flect v1.0.3)

```
word = cast.ToStringE(v); if word == "" -> ""
if v is Go int || strconv.Atoi(word) ok -> flect.Ordinalize(word)          // "1"->"1st","2"->"2nd","3"->"3rd","11"-"13"->"th", else "th"; negatives use abs
str = flect.Humanize(word); return flect.Humanize(strings.ToLower(str))
```
`flect.Humanize` (humanize.go L16-36): `Ident.Humanize`: empty Original -> ""; whitespace-only Original -> returned unchanged;
else `parts[0] = Titleize(parts[0])` (first rune `unicode.ToTitle`, titleize.go L20-37) joined with the remaining parts by a
single space. `toParts` (ident.go L29-96): `TrimSpace`; if `ToUpper(s)` is an acronym -> `[ToUpper(s)]`; iterate runes:
* `isSpace(c)` (flect.go: `_ ' ' : - /` or `unicode.IsSpace`) -> flush current buffer as a part, buffer = c;
* `unicode.IsUpper(c) && !unicode.IsUpper(prev)` -> flush, buffer = c;
* `unicode.IsUpper(c) && baseAcronyms[ToUpper(buffer)]` -> flush, buffer = c;
* `IsLetter || IsDigit || IsPunct || c=='`'` -> append to buffer;
* otherwise (e.g. **Thai combining marks Mn, `+`, symbols**) -> flush and **drop c**;
`xappend` (flect.go L20-35) trims spaces and the 5 space chars from each part, upper-cases it if `ToUpper(part)` is in
`baseAcronyms` (acronyms.go: the upper-case entries: OK UTF8 HTML JSON JWT ID UUID SQL ACK … WWW; the mixed-case entries
`gbps kbps Mbps MoCA WiFi` can never match because lookups use `ToUpper`), and drops empty parts.
`flect` `init` also loads `inflections.json`/`acronyms.json` from the **current working directory** or env
`INFLECT_PATH`/`ACRONYMS_PATH` (custom_data.go L13-50) — none exist for seeksnack, but the port must mimic it.
Examples: `potato-chips`->`Potato chips`; `Pepsi-Cola (Thai) Trading Co.,Ltd.`->`Pepsi cola ( thai) trading co., ltd.`;
`Union Snack Ltd.`->`Union snack ltd.` (golden); `api key`->`API key`; `a+b`->`A b`; `ญี่ปุ่น`->`ญ ป น`, `เกาหลี`->`เกาหล`
(golden th country badges); `3`->`3rd`.

### 3.4 Title creation used for list titles (not a template func here but string-helper parity)

`CreateTitle = helpers.GetTitleFunc("AP")` (helpers/general.go L194-210) -> jdkato/prose `transform.NewTitleConverter(APStyle).Title`
(prose v1.2.1 transform/title.go L41-58): regex `[\p{N}\p{L}]+[^\s-/]*` (Go RE2: `\s` = `[\t\n\f\r ]` ASCII only) over `s`; for
each match `m`: `sm = ToLower(m)`, `pos = strings.Index(t[idx:], m) + idx` where `t = sanitizer.Replace(s)` (“”‘’–—… -> ASCII),
`prev = charAt(t,pos-1)` (byte; `charAt` returns `s[0]` when out of range), `idx = pos + RuneCount(m)` (**byte offset plus rune
count — a bug that must be ported verbatim**); keep lower-case iff `sm` is in smallWords
(`a an and as at but by en for if in nor of on or per the to vs vs. via v v.`) and not first/last (`pos==0 || idx==end`) and
`prev in {' ','-','/'}` and `t[pos-2] not in {':','-'}` and (`t[pos+ext] != '-' || t[pos-1]=='-'`); else upper-case the first
rune with `unicode.ToTitle`. Used by hugolib/page__meta.go L750-780: section title = `CreateTitle(flect.Pluralize(dirName))`,
term title = `CreateTitle(term)`, taxonomy title = `ReplaceAll(CreateTitle(name), "-", " ")`. Golden: `Potato-Chips`, `INS 160a (I)`,
`Lay's`, `เลย์ สแตคส์ ` unchanged. `flect.Pluralize` (pluralize.go L32-72 + plural_rules.go 417 lines of dictionary/suffix
rules, built in `init`, `InsertPluralRule` prepends) must be ported with its tables. Fixtures: `ap_title`, `pluralize`,
`ap_title_pluralize` in `fixtures-strings.json`.

### 3.5 `lower`, `trim` (tpl/strings/strings.go L418-425, L440-452)

`strings.ToLower(cast.ToString(s))` (simple case mapping, see 3.1); `strings.Trim(s, cutset)` removes leading/trailing runes that
are in the cutset (seeksnack: `trim .RelPermalink "/"`).

### 3.6 `printf`, `print` (tpl/fmt/fmt.go L28-35 -> Go `fmt.Sprintf`/`fmt.Sprint`)

Verbs used: `%s`, `%d`, `%v`, `%q`. Needed Go semantics:
* `%s` on string/`[]byte`/Stringer/error; on nil -> `%!s(<nil>)`; on non-string types `%!s(int=5)` (bad-verb format
  `%!verb(type=value)`).
* `%d` on ints (Width/Height are `int`).
* `%v` default formatting (`printf "%v/%v" $parentKey $childKey` on strings).
* `%q` on a Stringer (media type) -> `strconv.Quote(String())` semantics (`"application/rss+xml"`), used by rss.xml L41.
* `fmt.Sprint(a...)`: adds a space between operands **only when neither side is a string** (head.html
  `print $permalink "page/" .PageNumber "/"` -> `https://seeksnack.com/brands/page/2/`).
Recommendation: implement a focused Go-fmt module (print.go 1208 + format.go 595 lines is the reference; only the verbs above plus
`%v` of scalars, slices `[a b]`, maps `map[k:v]` with sorted keys, nil `<nil>` are needed).

### 3.7 Printing of values in templates / i18n (`{{ . }}`) and Go float formatting

text/template `printValue` -> `fmt.Fprint`: `time.Time` uses `String()`; float64 uses `%v` = `strconv` 'g' with shortest
digits: with `d` = shortest round-trip digit string and decimal exponent `exp` (value = 0.d x 10^(exp+1)), Go uses exponent form
iff `exp < -4 || exp >= 6` (internal/strconv/ftoa.go `fmtEFG` L347-373: `if shortest { eprec = 6 }`), exponent written as
`e+06`/`e-05` (sign, at least 2 digits). JSON (encoding/json/encode.go L570-600) instead uses 'f' unless `abs < 1e-6 || abs >= 1e21`
and strips the exponent's leading zero (`1.5e-7`). Verified (`goexp/cmd/floatfmt`):

| value | `{{ . }}` / Sprint / `%v` | jsonify |
|---|---|---|
| 0.5, 2.5, 5, 123456, 999999 | `0.5` `2.5` `5` `123456` `999999` | same |
| 1000000 | `1e+06` | `1000000` |
| 1234567 | `1.234567e+06` | `1234567` |
| 12345678.9 | `1.23456789e+07` | `12345678.9` |
| 1e20 / 1e21 | `1e+20` / `1e+21` | `100000000000000000000` / `1e+21` |
| 0.0001 / 0.00001 / 1.5e-7 | `0.0001` / `1e-05` / `1.5e-07` | `0.0001` / `0.00001` / `1.5e-7` |

Other `fmt` facts verified: `Sprint("a",1,2,"b",3.5,true)` = `a1 2b3.5 true`; `Sprintf("%s", nil)` = `%!s(<nil>)`;
`Sprintf("%s", 5)` = `%!s(int=5)`; `%v` of nil = `<nil>`; `%q` of `a"b` = `"a\"b"`.
Rust: take the shortest round-trip digits (`ryu` or `format!("{:e}", f)`) and re-layout with these rules; never use f64 `Display`.
Integers from YAML are Go `int`, from TOML `int64`, from JSON `float64` (`5.0` prints `5`).

### 3.8 Time: `dateFormat`, `.Format`, `now`, time printing

* `dateFormat layout v` = tpl/time/time.go `Format` L74-81: `t = htime.ToTimeInDefaultLocationE(v, location=UTC)` then
  `timeFormatter.Format(t, layout)` (common/htime/time.go L95-140):
  - empty layout -> `""`; layouts starting with `:` -> `:date_full|:date_long|:date_medium|:date_short|:time_*` -> locales
    translator (`14 สิงหาคม ค.ศ. 2020` for th `:date_long`);
  - else `s := t.Format(layout)` (Go layout); if layout contains `January` replace `longMonthNames[m]` in `s` with
    `ltr.MonthWide(m)`, else if it contains `Jan` replace `shortMonthNames[m]` with `ltr.MonthAbbreviated(m)`; same for `Monday`/`Mon`
    with `WeekdayWide`/`WeekdayAbbreviated` (`strings.ReplaceAll`, so every occurrence of the English word is replaced).
  - th data (localescompressed `locales.autogen.go` L63774+): monthsAbbreviated `ม.ค. ก.พ. มี.ค. เม.ย. พ.ค. มิ.ย. ก.ค. ส.ค. ก.ย. ต.ค. พ.ย. ธ.ค.`,
    monthsWide `มกราคม … ธันวาคม`, daysAbbreviated `อา. จ. อ. พ. พฤ. ศ. ส.`, daysWide `วันอาทิตย์ … วันเสาร์`; en data = English.
  - seeksnack: `dateFormat (default "2006-01-02T15:04:05Z07:00" $siteDateFormat) .context` (datetime attr) and
    `dateFormat (default "Jan 2, 2006" …)` (visible text: en `Aug 14, 2020`, th `ส.ค. 14, 2020`), comments use `.date` strings
    with `+07:00` offsets (`Jun 9, 2021`; offset preserved, not converted).
* `ToTimeInDefaultLocationE` (htime/time.go L142-155): go-toml `LocalDate/LocalDateTime` -> `AsTime(loc)`; `time.Time` ->
  formatted with RFC3339 (drops sub-seconds!) and re-parsed; strings -> `cast.ToTimeInDefaultLocationE` (cast v1.9.2 time.go L26-61):
  `internal.ParseDateWith` tries, in order, the 24 layouts of `internal.TimeFormats` (internal/time.go L32-57:
  `2006-01-02`, RFC3339, `2006-01-02T15:04:05`, RFC1123Z, RFC1123, RFC822Z, RFC822, RFC850,
  `2006-01-02 15:04:05.999999999 -0700 MST`, `2006-01-02T15:04:05-0700`, `2006-01-02 15:04:05Z0700`, `2006-01-02 15:04:05`,
  ANSIC, UnixDate, RubyDate, `2006-01-02 15:04:05Z07:00`, `02 Jan 2006`, `2006-01-02 15:04:05 -07:00`,
  `2006-01-02 15:04:05 -0700`, Kitchen, Stamp, StampMilli, StampMicro, StampNano) with **`time.Parse`** (not ParseInLocation);
  for formats without zone/with only a named zone the wall clock is re-attached to `location` (UTC). With `time.Parse`, a numeric
  offset equal to the *machine's Local zone* offset yields the Local location and its abbreviation (Asia/Bangkok -> `+07`), which is
  visible only through `time.Time.String()` — not used on such values by seeksnack. `Z` -> UTC.
  ints -> `time.Unix`, nil -> zero time.
* Page dates: front-matter date keys are parsed by `pagemeta` `newDateFieldHandler` (resources/page/pagemeta/page_frontmatter.go
  L803-829) with the same function and **the parsed `time.Time` replaces the param** (`.Params.date` is a `time.Time`; also
  `publishdate/lastmod/expirydate` params are set if missing, L691-696). All seeksnack dates are `…Z` (UTC).
* `.Date.Format "Mon, 02 Jan 2006 15:04:05 -0700"` (RSS), `.Lastmod.Format "2006-01-02T15:04:05-07:00"` (sitemap/meta),
  `now.Format "2006"` (copyright): plain Go `time.Time.Format`, **English on the th site too** (golden th RSS `Mon, 25 Sep 2023 …`).
  UTC offsets print `+0000` / `+00:00`.
* `{{ .PublishDate }}` in JSON-LD prints `time.Time.String()` = `Format("2006-01-02 15:04:05.999999999 -0700 MST")`
  (`2020-08-14 14:59:12.921 +0000 UTC`; trailing zero nanos trimmed; `MST` is the zone name, or `+hhmm` when the name is empty),
  then JS-string-escaped by html/template (`+` -> `\u002b`).
* `now` = `htime.Now()` = `Clock.Now()` (system clock in **Local** timezone unless `--clock`). The copyright line
  `if ge .Site.Params.yearCreate (now.Format "2006")` compares `"2019"` vs `"2026"` numerically (see 3.9) -> `2019 -2026`.
  Output changes with the calendar year: the Rust port must implement `--clock` (RFC3339) and the harness should pin it.
* Rust: port Go's `time.Format`/`time.Parse` layout engine (time/format.go, 1731 lines; only the layout elements above are needed)
  rather than chrono's strftime; chrono/time crates can hold the instants. `chrono-tz` is not needed (only UTC and fixed offsets;
  Local only for `now`).

### 3.9 Comparisons: `eq ne ge gt le lt default cond` (tpl/compare/compare.go)

* Template-level namespace: `compare.New(location, caseInsensitive=false)` (tpl/compare/init.go L33).
* `Eq` (L99-157): if either arg implements `compare.Eqer` use it; else normalize: all signed ints -> int64, floats -> float64,
  uints -> int64 if they fit, strings -> string, `AsTimeProvider` -> time; then `reflect.DeepEqual`. So `eq 1 1.0` is false,
  `eq (int 5) (int64 5)` is true.
* `Ge/Gt/Le/Lt` via `compareGetWithCollator` (L255-380): `Comparer` interface first; kinds: slices/maps/arrays -> length;
  ints/uints/floats -> float64; **strings: `strconv.ParseFloat(s,64)`; if it parses (and is not Inf/NaN) the string compares as
  a number, else as a string**; `time.Time` -> Unix seconds; bools 0/1; other kinds -> 0. Two strings: with a collator (or
  caseInsensitive) -> `collator.CompareStrings`, else byte compare. A numeric side vs a string side -> `(left,right)` numbers
  where the string side counts as 0 (inconsistent ordering!). Invalid (nil) values count as 0 and never compare less.
* Seeksnack: `ge "2019" "2026"` (numeric), `ge $ratingnumber $i` (int/float64 vs int; YAML rating values int, a few float64
  `3.5`, and `''` strings -> ParseFloat fails -> string vs number -> treated as 0 on the string side), `gt $i 0`,
  `gt $pag.TotalPages 1`, `le .PageNumber 3`, `lt`.
* `Default` (L48-97): returns `defaultv` when given is absent/invalid, or zero: bool never zero, string/slice/map len 0,
  numbers 0, `time.Time` IsZero, other structs "set", pointers/interfaces nil.
* Go's ParseFloat grammar must be reproduced for the numeric-string rule: optional sign, decimal with optional exponent,
  hex floats `0x1p-2`, `inf`/`infinity`/`nan` (case-insensitive; Inf/NaN results fall back to string), underscores only with a base
  prefix. None of the site's 2762 sortable strings parse as numbers (checked with `goexp/cmd/numstr`).

### 3.10 Collections used

* `sort` (tpl/collections/sort.go L30-141): builds pairs (key = element, or `evaluateSubElem` of the dotted path, with special
  handling of `maps.Params` via `GetNested`); collator = `langs.GetCollator1(site language)`; comparator
  `sortComp.LtCollate(collator, a, b)` with `sortComp = compare.New(loc, caseInsensitive=true)` (collections.go L36-49); invalid keys
  compare against the zero value of the other side's type; **`sort.Stable`** (asc) or `sort.Stable(sort.Reverse(p))` (desc).
  `sort .Site.AllPages "Date"` compares `time.Time` as Unix seconds (sub-second ties keep input order).
* `first N l` (L191-223), `after N l` (L59-91): slices of the input (N via `cast.ToIntE`, clamps).
* `dict` (L154-188): `map[string]any`; `[]string` keys create nested maps.
* `slice` (L515): `[]any` (or typed slice when all elements share a type — see collections.go).
* `seq` (L417+): `seq 5` -> `[1 2 3 4 5]`; args via `cast.ToIntSlice` (int64 from TOML ok).
* `isset` (L326-348): slices/arrays -> `len > key`; maps -> key present when the key type matches exactly; **strings and other kinds
  -> warning + false** (tags.html on a string param).
* `index` (index.go L33-144): map lookup returns nil for a missing key (no error).
* `apply . "humanize" "."` (apply.go L27+): calls the named func for every element, returns `[]any`.
* `where .Site.RegularPages "Params.type" "snacks"` (where.go L29+): default operator `=`, compare via `compare.Eq`-like
  semantics (collections/other agents).
* `len`: Go builtin (strings -> byte length!).
* Scratch (common/maps/scratch.go): `Add` appends when the existing value is a slice (`collections.Append`), else
  `math.DoArithmetic(+)` (int+int -> int64, string+string concat); `SetInMap` creates `map[string]any`.
* `reflect.IsSlice` (tpl/reflect/reflect.go L34): `reflect.ValueOf(v).Kind() == Slice` (both `[]string` and `[]any` true).
* `path.Ext` (tpl/path/path.go L45): Go `path.Ext` on `cast.ToString` of a resource (its name/permalink).

### 3.11 Arithmetic `add sub mul` (tpl/math/math.go L66, L187, L251 -> common/math/math.go `DoArithmetic` L23-133)

int kinds -> int64 result; any float -> float64; uint handling; string+string with `+` concatenates; division by zero error.
Prints as plain integers in seeksnack (`add . 1`, `sub .TotalPages .PageNumber`, `mul $paginator.PageNumber $PagerSize`).

### 3.12 `jsonify` (tpl/encoding/encoding.go L64-110)

`json.NewEncoder` + `SetEscapeHTML(true)` (unless `noHTMLEscape` option) + `SetIndent(prefix, indent)` (none here); trailing newline
trimmed; returns `template.HTML`. Go encoding/json semantics required: map keys sorted by byte order (`categories, contents,
description, image_preview, image_preview_webp, relpermalink, tags, title` in index.json); `<`,`>`,`&` -> `\u003c \u003e \u0026`;
U+2028/U+2029 -> `\u2028 \u2029`; invalid UTF-8 -> `\ufffd`; floats per 3.7 JSON rule; `[]string`/`[]any` identical; nil ->
`null` (golden: `"tags":["ปาร์ตี้","คริสปี้พาย ","เนย","คาราเมล",null]`); non-ASCII emitted raw. `serde_json` is safe only with a
custom formatter that performs the HTML escaping and Go float formatting, and with keys sorted (BTreeMap or sort before write).

### 3.13 Safe types and escaping helpers

`safeHTML/safeCSS/safeJS/safeURL/safeHTMLAttr/safeJSStr` (tpl/safe/safe.go L33-66) just wrap `cast.ToString` in html/template
types. `html` in a pipeline is the text/template builtin (`template.HTMLEscapeString`); `urldecode` = `url.QueryUnescape`
(returns input on error, urls.go L240-254). Escaping itself belongs to the template-engine spec.

### 3.14 Unused-but-registered helpers (for completeness)

`anchorize` = `SanitizeAnchorName` (goldmark github style), `plainify` = `tpl.StripHTML`, `htmlEscape`/`htmlUnescape` =
`html.EscapeString`/`html.UnescapeString`, `title` = `CreateTitle`, `countwords` (CJK regex, `strings.Fields(StripHTML)`),
`truncate` (tpl/strings/truncate.go), `substr` (rune based), `split`, `replace`, `upper` (`strings.ToUpper`, simple mapping — `ß`
stays `ß`), `delimit`, `in`, `intersect`, `union`, `uniq`, `shuffle` (math/rand — nondeterministic, avoid), `md5`, `sha1`, `sha256`,
`base64Encode/Decode`, `lang.FormatNumber*` (locales). None affects seeksnack output.

---------------------------------------------------------------------------------------------------------------------------

## 4. Collation

### 4.1 Where neohugo collates

| place | Go | collator |
|---|---|---|
| template `sort` on strings/any values | tpl/collections/sort.go L52, L162-182 -> tpl/compare/compare.go `LtCollate` L208-218 / L349-363 | `GetCollator1(site language)` |
| `Pages.ByTitle`, `ByLinkTitle` | resources/page/pages_sort.go L174-189 (`collatorStringSort`, `sort.SliceStable`) | Collator1 |
| `DefaultPageSort` tie-break (same weight and same `Date().Unix()`) on `LinkTitle`, then `compare.LessStrings(path)` | pages_sort.go L81-109, L191-200 | Collator1 |
| `pagesSort` less helper (ByParam etc.) | pages_sort.go L202-214 | Collator2 |
| `Taxonomy.Alphabetical` | resources/page/taxonomy.go L79-95 | Collator1 (not used by seeksnack) |
| `lessPageLanguage` / `ByCount` | pages_sort.go L111-133, taxonomy.go L99-114 | `compare.Strings` (case-fold + byte tiebreak, compare/compare_strings.go L22-113) — not a collator |

`langs.Collator` wraps `*collate.Collator` created with `collate.New(tag)` and default options (tertiary strength, non-ignorable
variables, no numeric). `CompareString` returns -1/0/1.

### 4.2 What seeksnack triggers

* `sort . ` on tags/categories/ingredients/companies/countries (+ `sort $humanized` for brands), `sort . ` on `References`,
  `sort (.Paginator.Pages) "Title"` on every list/taxonomy/term page, `sort ($scratch.Get "pages") "position" "asc"` (ints),
  `sort .Site.AllPages "Date"` (times), default page ordering of `.Pages/.RegularPages/.Data.Pages` (weights 0 everywhere, dates
  mostly distinct; term pages share dates with their newest page so their linkTitle tie-break uses the collator).
* Thai strings on the th site: e.g. golden th `party-crispy-pie-butter-caramel` tags `คริสปี้พาย, คาราเมล, เนย, ปาร์ตี้, <nil>` (byte order
  would put `เนย` after `ปาร์ตี้`); ingredients `INS 160a (i) … INS 941, เกลือไอโอดีน, คาราเมล, แต่ง…, น้ำตาล, …` (Latin before Thai,
  Thai prevowels ignored for ordering).

### 4.3 Experiments (`goexp/cmd/coll`, `goexp/cmd/collcmp`, `rcoll/`)

Input: every title/description/taxonomy value (raw and humanized)/reference per language: en 1575 strings, th 1187 strings.

| comparison | en data | th data |
|---|---|---|
| x/text collation vs byte order | 212 adjacent inversions (differs) | 119 adjacent inversions (differs) |
| x/text `en` vs `th` vs `und` | identical | identical |
| x/text: pairs comparing equal | 0 | 0 |
| ICU4X 2.3.1 `en` / `und` default | 4 positions differ (only `Lay’s …` U+2019 vs `Lay's …` descriptions) | **0** |
| ICU4X `und` alternate=non-ignorable | same as above | 0 |
| ICU4X `th` default | 1520 differ | **1187/1187 differ** (Thai reordered before Latin; spaces/quotes ignorable) |
| ICU4X `th` non-ignorable | 1552 differ | 1176 differ |

No string containing U+2018/2019/201C/201D/00A0 is ever sorted (titles/taxonomies checked), so ICU4X root reproduces every
sort in the current site. x/text data versions: `collate.UnicodeVersion = "6.2.0"`, `CLDRVersion = "23"`
(x/text v0.26.0 collate/tables.go L5-9); it also needs `unicode/norm` NFD iteration (norm tables are Unicode 15).

### 4.4 Recommendation

* Option A (default): `icu_collator` (ICU4X 2.x, compiled data) with **locale `und` for every site language** and default options;
  keep `coll/xtext-und-data*.txt` as a regression fixture (re-generate with the Go program when content changes). Never use the
  `th` locale data. Risk: characters whose UCA/CLDR weights changed between UCA 6.2 and current (typographic quotes/apostrophes,
  symbols, emoji, scripts added after Unicode 6.2 which x/text treats as unassigned/implicit).
* Option B (exact, recommended if future-proof parity is required): port `x/text/collate` (collate.go 403, option.go 239,
  index.go 32, sort.go 81) + `x/text/internal/colltab` (collelem.go 376, colltab.go 105, contract.go 145, iter.go 178, numeric.go 236,
  table.go 275, trie.go 159, weighter.go 31) and generate the 4.95 MB `tables.go` data (73,789 lines) into Rust arrays with a Go
  generator; NFD via `unicode-normalization` (Thai/Latin decompositions are version-stable) or a port of the needed `norm` iterator.
* Either way, reproduce Go's **`sort.Stable`** exactly (sort/zsortinterface.go: `insertionSort` on blocks of 20, then `symMerge`
  with doubling block size, `rotate`, `swapRange`; `sort.SliceStable` uses the same algorithm). The template `sort` comparator is
  not a strict weak order when nil/non-numeric/numeric strings are mixed (e.g. the `nil` tag above is "equal" to everything);
  Rust's `sort_by` produces different results for such inputs on slices > 20.

---------------------------------------------------------------------------------------------------------------------------

## 5. Front matter / config / i18n / data decoding

### 5.1 metadecoders (parser/metadecoders/decoder.go)

`UnmarshalToMap` (L75-84), `Unmarshal` (L129-149; empty input -> empty map), `UnmarshalTo` (L152-235):
* JSON: `encoding/json.Unmarshal` into `interface{}`: objects `map[string]any`, arrays `[]any`, **all numbers float64**, null nil.
* TOML: `pelletier/go-toml/v2` v2.2.4 `toml.Unmarshal` into `interface{}`: tables `map[string]any`, arrays `[]any`, integers
  **int64**, floats float64, bools, offset date-time `time.Time`, local date-time/date/time `toml.LocalDateTime/LocalDate/LocalTime`
  (these implement `AsTime(loc)` and are handled by `htime.ToTimeInDefaultLocationE`).
* YAML: **gopkg.in/yaml.v2 v2.4.0** (YAML 1.1, not goccy, not v3) into `interface{}` then `stringifyMapKeys` (L336-375) converts
  every `map[interface{}]interface{}` (recursively, including inside slices) to `map[string]any` with keys via
  `cast.ToStringE` (fallback `fmt.Sprintf("%v")`). yaml.v2 scalar resolution (resolve.go L90-199) for **plain** scalars only
  (quoted scalars are always strings, decode.go L414-417):
  - map lookup: `y Y yes Yes YES true True TRUE on On ON` -> true; `n N no No NO false False FALSE off Off OFF` -> false;
    `"" ~ null Null NULL` -> nil; `.nan .NaN .NAN`, `[+-].inf …` -> floats; `<<` merge key.
  - first char `.` -> `strconv.ParseFloat`.
  - first char digit or sign: timestamp check (`YYYY-` prefix and one of `2006-1-2T15:4:5.999999999Z07:00`,
    `2006-1-2t15:4:5.999999999Z07:00`, `2006-1-2 15:4:5.999999999`, `2006-1-2`) — **decoded into interface{} it stays the original
    string** (decode.go L474-480); else remove `_` and `strconv.ParseInt(plain, 0, 64)` (base prefixes `0x`, `0o`, `0b`, and a
    leading `0` means **octal**) -> Go `int` if it fits; else `ParseUint`; else YAML-style float regex
    `^[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?$` -> float64 (so `08` -> 8.0); `0b…`/`-0b…` binary.
  - everything else -> string. Duplicate keys: last wins (non-strict decoder, decode.go L685-691).
* CSV/XML/ORG exist but are unused by seeksnack.

### 5.2 What seeksnack actually contains (`fm-types.txt`)

* 218 YAML (`---`) and 32 TOML (`+++`) front matters, 1 empty (`content/_index.md`). No JSON/ORG front matter.
* YAML dates (`date`, `when_seen`) are strings `YYYY-MM-DDTHH:MM:SS.mmmZ` (210 + 39) or `""` (132 when_seen); TOML `date` is
  `time.Time` (28). Numbers: ratings/nutrition `int` (a few `float64`: `3.5`, `4.5`, `0.5`, `2.5`), some `""` strings; `price` int.
  Taxonomies are strings or `[]any` of strings; one th file has a YAML `~`/empty tag -> `nil` element
  (`corn-chips/party-crispy-pie-butter-caramel/index.th.md`). TOML `tags = [""]`, `ingredients = [""]` (empty strings).
  No YAML 1.1 bool-words (`yes/no/on/off/y/n`) or octal-looking values occur in values that are rendered.
  Mixed-case keys `Description`, `Rating_*` exist (no collisions after lower-casing).
* Data files: `data/comments/**.json` (3 files) -> `.Site.Data.comments.<dir>.<dir>.<file-basename>`; numbers float64
  (`reviewTastyTaste: 5`), dates strings with `Z`/`+07:00`. Used by `partials/comments.html` (matches `parentKey/childKey` ==
  `trim .RelPermalink "/"`; renders on `brands/yubari-melon` and `rice-crackers/dozo-japanese-rice-original-flavoured`; the `th/…`
  entry is 3 levels deep and never matches).
* `transform.Unmarshal` of YouTube API JSON (JSON-LD) -> encoding/json semantics (float64 numbers, strings as given).

### 5.3 Hugo post-processing of front matter (hugolib/page__meta.go L470-645)

Keys lower-cased (`loki`), date keys removed from generic handling and parsed by pagemeta (5.2), `title/linktitle/description/…`
normalized through `cast.ToString`, **`[]any` whose elements are all strings becomes `[]string`** (L615-641; a list containing
`nil` stays `[]any`; an empty list becomes `[]string{}`), other values stored as-is; nested maps become lower-cased
`maps.Params`; maps inside lists keep their keys (e.g. `ingredients_percentage[].name`).

### 5.4 Rust crates and semantic gaps

| format | crate | safe? | gaps to patch |
|---|---|---|---|
| TOML | `toml` (1.x) / `toml_edit` | yes | map `Integer`->i64, `Float`->f64, `Datetime` with offset -> instant+offset, local variants -> "local" kinds converted with the language location; keep keys as written then lower-case like `PrepareParams`; go-toml v2 and toml-rs are both TOML 1.0 |
| YAML | `serde_yaml`/`yaml-rust2` typed loaders | **not safe** (YAML 1.2 core schema: `yes/no/on/off/y/n` stay strings, octal rules differ, timestamps untyped, ints are i64 not "int") | use an event-level parser that exposes scalar style + explicit tags (`saphyr-parser` or `yaml-rust2` `Parser`) and port yaml.v2 `resolve()`/`parseTimestamp` (resolve.go 258 lines) + `stringifyMapKeys`; plain timestamps must remain strings; merge keys `<<` |
| JSON | `serde_json` | yes, with conversion | convert every Number to f64 (Go never yields ints from JSON into `interface{}`); preserve `null`; object key order irrelevant (maps) |
| i18n TOML | `toml` | yes | same as TOML |

---------------------------------------------------------------------------------------------------------------------------

## 6. Rust port plan for this subsystem

### 6.1 Port line-by-line (Go source, lines)

* Config: config/allconfig/{allconfig.go 1217, alldecoders.go 469, configlanguage.go 261, load.go 544}, config/{defaultConfigProvider.go
  366, configLoader.go 231, commonConfig.go 512, env.go 93}, config/services/servicesConfig.go 110, common/maps/{params.go 384,
  maps.go 236, scratch.go 160}, common/urls/baseURL.go 112, markup/highlight/config.go (ApplyLegacyConfig part), langs/{language.go 189,
  config.go 58}. (Only the decoders relevant to the build need full fidelity; `hugo config` output is not part of the acceptance test.)
* i18n: langs/i18n/{i18n.go 205, translationProvider.go 139}, tpl/lang/lang.go (Translate, 16 lines), go-i18n fork files listed in 2.6.
* Paths/URLs: helpers/url.go 189, helpers/path.go (MakePath/MakePathSanitized, ~30), common/paths/path.go (Sanitize,
  isAllowedPathCharacter, ~60), common/paths/url.go (MakePermalink, AddContextRoot ~60), tpl/urls/urls.go 254, plus a Go
  `net/url` subset (Parse, setPath/unescape, EscapedPath, shouldEscape, String — url.go 1353 lines reference).
* Inflection/titles: tpl/inflect/inflect.go 75, flect {ident.go 122, humanize.go 36, titleize.go 38, ordinalize.go 43, flect.go 43,
  acronyms.go 152, pluralize.go 72, plural_rules.go 417, rule.go 17, custom_data.go 88}, helpers/general.go `GetTitleFunc`,
  jdkato/prose transform/title.go 108.
* Time: tpl/time/time.go 150, common/htime/time.go 177, cast time.go 116 + internal/time.go 79, Go time/format.go (Format/Parse,
  1731 reference), `time.Time.String`, locales data for en and th (generate from localescompressed).
* Compare/collections/math/fmt/encoding: tpl/compare/compare.go 388, compare/compare_strings.go 113, tpl/collections/{sort.go 197,
  collections.go 687 (used funcs), apply.go 162, index.go 144, where.go 543}, common/math/math.go 133, tpl/fmt/fmt.go 117 + Go fmt
  subset, tpl/encoding/encoding.go 116 + Go encoding/json encoder subset, Go sort/zsortinterface.go stable part (~150 of 479).
* Decoding: parser/metadecoders/decoder.go 375 (+format.go 114), yaml.v2 resolve.go 258 (+ the interface{} parts of decode.go).
* Collation: see 4.4.

### 6.2 Generated tables (do not hand-write)

Write a small Go generator (pattern in `goexp/`) that emits Rust source for: Go 1.27 `unicode` tables (**Unicode 17.0.0**:
categories L, Lu, Ll, Lt, M, N, Nd, P, Z/White_Space, `CaseRanges` for simple `ToLower/ToUpper/ToTitle`, `SimpleFold`);
gohugoio/locales month/day names for en and th (and the `:date_*` formatters if wanted); go-i18n plural rules; flect dictionaries;
optionally x/text collate tables. This removes Unicode-version drift between Go 1.27 and Rust std/regex/icu data.

### 6.3 Crates: safe vs not safe

Safe (with the noted constraints): `toml` (types mapped as above); `serde_json` (numbers -> f64, custom Go-compatible writer for
jsonify); `icu_collator` **with locale `und` only**; `percent-encoding` as a primitive with a Go-`shouldEscape` AsciiSet;
`regex` only with explicit ASCII `\s` (`(?-u:\s)`) and pinned Unicode classes (or generated tables); `ryu`/`format!("{:e}")` for
shortest float digits (re-laid-out in Go style); `chrono`/`time` as instant containers only; `saphyr-parser`/`yaml-rust2` event API.

NOT safe: `icu_collator` with `th` (or any tailoring newer than CLDR 23); `url` crate (WHATWG: normalizes dot-segments,
lower-cases host/scheme, different percent-encode sets, cannot round-trip relative refs like Go `url.Parse(x).String()`);
`serde_yaml` typed values (YAML 1.2); `str::to_lowercase/to_uppercase` (full case mapping, final sigma), `char::is_alphabetic`
(Alphabetic property != Go `IsLetter`; e.g. Thai vowel signs U+0E31/U+0E34.. are Alphabetic but Mn), `char::is_numeric` (N* != Go
`IsDigit` = Nd), `regex` default `\s`/`\w` (Unicode, while Go RE2 `\s` is ASCII), `Inflector`/`heck`/`cruet` (not flect), chrono `format` (strftime, not Go layouts, no Thai names), Rust
`sort_by` for comparators that are not strict weak orders, Display of f64.

---------------------------------------------------------------------------------------------------------------------------

## 7. Parity risks (ranked)

1. Collation data/tailoring (ICU4X `th`) — would reorder almost every Thai list; use root/und or port x/text.
2. Build-environment inputs: image resource cache state (warm cache changes all `_hu_` names — golden == cold cache), `now`
   year (copyright `2019 -2026`), YouTube `GetRemote` cache/network data in JSON-LD, machine Local timezone for `now`.
3. `humanize` via flect (Thai mark dropping, ordinalize, acronym table, CWD-loaded custom files) — easy to "fix" by accident.
4. `urlize` + `lower` interplay (upper-case hex from Go `url.URL.String`, lower-cased afterwards only where `| lower` is used),
   Go `unicode.IsLetter/IsMark/IsSpace` tables (Unicode 17) and Go simple case mapping.
5. Localized `dateFormat` (Thai month abbreviations) vs non-localized `.Format`; Go layout engine; `time.Time.String()` format.
6. Go formatting of numbers (`%v` floats, int vs int64 vs float64 origins: YAML int, TOML int64, JSON float64) in i18n
   messages, attributes and JSON (`jsonify` HTML escaping `\u0026`, sorted keys).
7. `printf` bad-verb output `%!s(<nil>)` and `absLangURL` returning its input on URL parse errors (1678 golden files).
8. Numeric-string comparisons in `ge/lt/sort` (`ge "2019" "2026"`), nil/zero handling and Go `sort.Stable` behaviour with
   inconsistent comparators (the nil tag).
9. yaml.v2 YAML-1.1 resolution (timestamps as strings, octal, bool words) and `[]any`->`[]string` conversion of all-string lists.
10. Config lower-casing boundaries (arrays of tables keep case) and per-language merges (language params win; LanguageCode `en`).
11. i18n fallback/plural details (irrelevant for current content but part of the contract): default-language fallback,
    `<no value>` for nil data, lazy template parsing, `Count` detection.
12. AP title case (prose) byte/rune index bug and ASCII-only `\s` in its regex (term/section titles like `Potato-Chips`,
    `INS 160a (I)`), `flect.Pluralize` tables for section titles.
