package main

import (
	"os"
	"path/filepath"
	"sort"
	"strings"
)

// procEnv is the process environment of most cases: the user cache dir is
// $ROOT/xdg (created by load).
var procEnv = map[string]string{"HOME": "$ROOT/home", "XDG_CACHE_HOME": "$ROOT/xdg"}

// environ is the default ConfigSourceDescriptor.Environ (no HUGO_ variables).
var environ = []string{"NEOHUGO_ORACLE=1"}

func readFixture(root, name string) (string, error) {
	b, err := os.ReadFile(filepath.Join(root, "crates/nh-allconfig/tests/fixtures/load", name))
	return string(b), err
}

// fs builds a file map from name/content pairs.
func fs(kv ...string) map[string]string {
	m := map[string]string{}
	for i := 0; i < len(kv); i += 2 {
		m[kv[i]] = kv[i+1]
	}
	return m
}

// with returns a copy of m with the given name/content pairs added.
func with(m map[string]string, kv ...string) map[string]string {
	out := map[string]string{}
	for k, v := range m {
		out[k] = v
	}
	for i := 0; i < len(kv); i += 2 {
		out[kv[i]] = kv[i+1]
	}
	return out
}

// std is a case with the default environment.
func std(name string, files map[string]string) caseSpec {
	return caseSpec{
		Name:        name,
		Files:       files,
		Environment: "production",
		Environ:     environ,
		ProcEnv:     procEnv,
	}
}

// dirs are the component dirs of a typical project.
var dirs = fs("content/", "", "layouts/", "", "static/", "", "assets/", "", "data/", "", "i18n/", "", "archetypes/", "")

// toml returns a hugo.toml site with the component dirs.
func toml(name, cfg string) caseSpec {
	return std(name, with(dirs, "hugo.toml", cfg))
}

// seeksnackCases are the reconstructed seeksnack site (hugo.toml from the
// committed dumps) with the directories and root files that decide its mounts.
func seeksnackCases(root string) ([]caseSpec, error) {
	toml, err := readFixture(root, "seeksnack/hugo.toml")
	if err != nil {
		return nil, err
	}
	files := map[string]string{
		"hugo.toml":         toml,
		"content/":          "",
		"static/":           "",
		"layouts/":          "",
		"data/":             "",
		"assets/":           "",
		"i18n/":             "",
		"archetypes/":       "",
		"node_modules/":     "",
		"package.json":      "{}\n",
		"postcss.config.js": "module.exports = {};\n",
	}
	c1 := std("seeksnack/config", files)
	c1.Site = "seeksnack"
	c2 := std("seeksnack/build", files)
	c2.Site = "seeksnack"
	// The golden build: `neohugo --minify --clock 2026-09-27T12:00:00Z`.
	c2.Flags = map[string]any{"minifyOutput": true, "internal.clock": "2026-09-27T12:00:00Z"}
	c3 := c2
	c3.Name = "seeksnack/build-env-development"
	c3.Environment = "development"
	return []caseSpec{c1, c2, c3}, nil
}

// repoTree returns the top-level entries of dir (files with their content
// when keep says so, else empty; directories as empty directories).
func repoTree(dir string, keep func(name string) bool, sub ...string) (map[string]string, error) {
	files := map[string]string{}
	entries, err := os.ReadDir(dir)
	if err != nil {
		return nil, err
	}
	for _, e := range entries {
		n := e.Name()
		if n == "rust-port" || n == ".DS_Store" {
			continue
		}
		if e.IsDir() {
			files[n+"/"] = ""
			continue
		}
		if keep(n) {
			b, err := os.ReadFile(filepath.Join(dir, n))
			if err != nil {
				return nil, err
			}
			files[n] = string(b)
		} else {
			files[n] = ""
		}
	}
	for _, s := range sub {
		files[s] = ""
	}
	return files, nil
}

func repoCases(root string) ([]caseSpec, error) {
	docs, err := repoTree(filepath.Join(root, "docs"), func(n string) bool {
		return n == "hugo.toml" || n == "go.mod" || n == "hugo.work" || strings.HasPrefix(n, "package")
	}, "content/en/")
	if err != nil {
		return nil, err
	}
	ts, err := repoTree(filepath.Join(root, "hugolib", "testsite"), func(string) bool { return false })
	if err != nil {
		return nil, err
	}
	ts["hugo.toml"] = `baseURL = "https://example.org/"
defaultContentLanguage = "en"
[languages.en]
weight = 1
[languages.nn]
weight = 2
contentDir = "content_nn"
`
	cases := []caseSpec{
		std("repo/docs", docs),
		std("repo/testsite", ts),
	}
	dev := std("repo/docs-development", docs)
	dev.Environment = "development"
	cases = append(cases, dev)
	return cases, nil
}

// basicCases: config file discovery, formats and errors.
func basicCases() []caseSpec {
	var cs []caseSpec
	cs = append(cs, std("basic/minimal", fs("hugo.toml", "baseURL = \"https://example.org/\"\n")))
	cs = append(cs, std("basic/empty-dir", fs("x/", "")))
	cs = append(cs, std("basic/yaml", with(dirs, "hugo.yaml", "baseURL: https://example.org/docs/\ntitle: YAML\nparams:\n  Foo: bar\n  nested:\n    A: 1\n    b: [1, 2]\n")))
	cs = append(cs, std("basic/yml", with(dirs, "hugo.yml", "title: YML\nsummaryLength: 30\n")))
	cs = append(cs, std("basic/json", with(dirs, "hugo.json", `{"baseURL": "https://example.org", "title": "JSON", "params": {"n": 1.5, "i": 3, "list": ["a", "b"]}, "taxonomies": {"tag": "tags", "series": "series"}}`)))
	cs = append(cs, std("basic/legacy-config-toml", with(dirs, "config.toml", "title = \"legacy\"\nbaseURL = \"https://legacy.org/\"\n")))
	cs = append(cs, std("basic/hugo-wins-over-config", with(dirs, "hugo.toml", "title = \"hugo\"\n", "config.toml", "title = \"config\"\n")))
	cs = append(cs, std("basic/toml-before-yaml", with(dirs, "hugo.yaml", "title: yaml\n", "hugo.toml", "title = \"toml\"\n")))
	c := std("basic/filename-list", with(dirs, "a.toml", "title = \"a\"\n[params]\nx = 1\ny = 1\n", "b.yaml", "params:\n  y: 2\n", "missing.toml", ""))
	c.Filename = "$ROOT/site/a.toml,b.yaml"
	cs = append(cs, c)
	c = std("basic/filename-relative", with(dirs, "custom.toml", "title = \"custom\"\n"))
	c.Filename = "custom.toml"
	cs = append(cs, c)
	c = std("basic/filename-noext", with(dirs, "custom.yaml", "title: custom\n"))
	c.Filename = "custom"
	cs = append(cs, c)
	c = std("basic/filename-missing", with(dirs, "hugo.toml", "title = \"x\"\n"))
	c.Filename = "nope.toml"
	cs = append(cs, c)
	cs = append(cs, std("basic/invalid-toml", with(dirs, "hugo.toml", "title = \"x\"\n[params\nfoo = 1\n")))
	cs = append(cs, std("basic/invalid-yaml", with(dirs, "hugo.yaml", "title: [x\n")))
	cs = append(cs, std("basic/invalid-json", with(dirs, "hugo.json", "{\"title\": }")))
	cs = append(cs, toml("basic/base-url-path", "baseURL = \"https://example.org/sub/dir/\"\ntitle = \"T\"\n"))
	cs = append(cs, toml("basic/base-url-invalid", "baseURL = \"http://[::1\"\n"))
	cs = append(cs, toml("basic/internal-in-file", "title = \"x\"\n[internal]\nrunning = true\nclock = \"2020-01-01T00:00:00Z\"\n"))
	cs = append(cs, toml("basic/aliases", "indexes = { tag = \"tags\", cat = \"cats\" }\nlogI18nWarnings = true\nlogPathWarnings = true\nignoreErrors = [\"error-a\", \"ERROR-B\"]\n"))
	cs = append(cs, toml("basic/timeout-number", "timeout = 30\n"))
	cs = append(cs, toml("basic/timeout-string", "timeout = \"2m30s\"\n"))
	cs = append(cs, toml("basic/timeout-invalid", "timeout = \"forever\"\n"))
	cs = append(cs, toml("basic/title-case-go", "titleCaseStyle = \"Go\"\n"))
	cs = append(cs, toml("basic/title-case-chicago", "titleCaseStyle = \"chicago\"\n"))
	cs = append(cs, toml("basic/title-case-none", "titleCaseStyle = \"none\"\n"))
	cs = append(cs, toml("basic/root-flags", `buildDrafts = true
buildFuture = true
buildExpired = true
copyright = "© 2026"
disableAliases = true
disablePathToLower = true
enableEmoji = true
enableGitInfo = false
enableRobotsTXT = true
hasCJKLanguage = true
canonifyURLs = true
relativeURLs = true
removePathAccents = true
printI18nWarnings = true
printPathWarnings = true
printUnusedTemplates = true
refLinksNotFoundURL = "/404"
refLinksErrorLevel = "WARNING"
sectionPagesMenu = "main"
summaryLength = "25"
timeZone = "Europe/Oslo"
newContentEditor = "vim"
noTimes = true
noChmod = true
cleanDestinationDir = true
noBuildLock = true
panicOnWarning = false
templateMetrics = true
templateMetricsHints = true
pluralizeListTitles = false
capitalizeListTitles = false
disableLiveReload = true
disableHugoGeneratorInject = true
disableDefaultLanguageRedirect = true
enableMissingTranslationPlaceholders = true
`))
	cs = append(cs, toml("basic/dirs", `contentDir = "mycontent"
dataDir = "mydata"
layoutDir = "mylayouts"
i18nDir = "myi18n"
archetypeDir = "myarchetypes"
assetDir = "myassets"
publishDir = "out"
resourceDir = "res"
themesDir = "mythemes"
staticDir = ["static1", "static2", "static1"]
staticDir0 = "s0"
staticDir3 = ["s3", "static2"]
staticDir10 = "s10"
`))
	cs = append(cs, toml("basic/ignore-files", "ignoreFiles = [\"\\\\.tmp$\", \"^content/draft/\", \"~$\"]\nignoreLogs = [\"Warning-X\", \"error-Y\"]\n"))
	cs = append(cs, toml("basic/ignore-files-invalid", "ignoreFiles = [\"(unclosed\"]\n"))
	cs = append(cs, toml("basic/disable-kinds", "disableKinds = [\"taxonomy\", \"TERM\", \"taxonomyTerm\", \"nope\", \"RSS\", \"sitemap\", \"robotsTXT\", \"404\"]\n"))
	cs = append(cs, toml("basic/disable-kinds-string", "disableKinds = \"section\"\n"))
	cs = append(cs, toml("basic/main-sections-params", "[params]\nmainSections = [\"blog\", \"docs\"]\n"))
	cs = append(cs, toml("basic/main-sections-params-string", "[params]\nmainSections = \"blog\"\n"))
	cs = append(cs, toml("basic/main-sections-root", "mainSections = [\"news\"]\n"))
	cs = append(cs, toml("basic/main-sections-empty", "mainSections = []\n"))
	cs = append(cs, toml("basic/ugly-urls-bool", "uglyURLs = true\n"))
	cs = append(cs, toml("basic/ugly-urls-string", "uglyURLs = \"true\"\n"))
	cs = append(cs, toml("basic/ugly-urls-map", "[uglyURLs]\nposts = true\nblog = \"false\"\ndocs = 1\n"))
	cs = append(cs, toml("basic/author-social", "[author]\nname = \"Jane\"\nEmail = \"j@example.org\"\n[social]\ntwitter = \"jane\"\nfacebook = \"JaneFB\"\n"))
	cs = append(cs, toml("basic/default-output-format", "defaultOutputFormat = \"JSON\"\n"))
	cs = append(cs, toml("basic/default-output-format-invalid", "defaultOutputFormat = \"nope\"\n"))
	cs = append(cs, toml("basic/environment-in-file", "environment = \"staging\"\n"))
	c = toml("basic/flags", "title = \"file\"\npublishDir = \"public\"\n")
	c.Flags = map[string]any{"title": "flag", "publishDir": "$ROOT/out", "minify": true, "buildDrafts": true, "cacheDir": "$ROOT/cache", "themesDir": "$ROOT/site/themes2", "baseURL": "https://flag.example.org/"}
	cs = append(cs, c)
	c = toml("basic/flags-minify-bool", "title = \"file\"\n")
	c.Flags = map[string]any{"minify": true}
	cs = append(cs, c)
	c = toml("basic/clock-invalid", "title = \"x\"\n")
	c.Flags = map[string]any{"internal.clock": "yesterday"}
	cs = append(cs, c)
	c = toml("basic/cache-dir-config", "cacheDir = \"$ROOT/mycache\"\n")
	cs = append(cs, c)
	c = toml("basic/cache-dir-relative", "cacheDir = \"relcache\"\n")
	cs = append(cs, c)
	c = std("basic/cache-dir-no-xdg", with(dirs, "hugo.toml", "title = \"x\"\n", "../tmp/", ""))
	c.ProcEnv = map[string]string{"HOME": "$ROOT/home", "XDG_CACHE_HOME": "$ROOT/nope/xdg", "USER": "oracle", "TMPDIR": "$ROOT/tmp"}
	cs = append(cs, c)
	c = toml("basic/cache-dir-netlify", "title = \"x\"\n")
	c.ProcEnv = map[string]string{"HOME": "$ROOT/home", "XDG_CACHE_HOME": "$ROOT/xdg", "NETLIFY": "true", "PULL_REQUEST": "false", "DEPLOY_PRIME_URL": "https://x"}
	// Netlify's cache dir is /opt/build/cache/hugo_cache/, which is not writable here.
	_ = c
	return cs
}

// configDirCases: config/_default + config/<env> merges.
func configDirCases() []caseSpec {
	var cs []caseSpec
	base := with(dirs,
		"config/_default/hugo.toml", "baseURL = \"https://default.org/\"\ntitle = \"Default\"\n[params]\ncolor = \"red\"\n[params.nested]\na = 1\nb = 2\n",
		"config/_default/params.toml", "size = \"L\"\n[nested]\nb = 3\nc = 4\n",
		"config/_default/menus.en.toml", "[[main]]\nname = \"Home\"\nurl = \"/\"\nweight = 1\n",
		"config/_default/menu.nn.toml", "[[main]]\nname = \"Heim\"\nurl = \"/\"\n",
		"config/_default/languages.toml", "[en]\nweight = 1\nlanguageName = \"English\"\n[nn]\nweight = 2\nlanguageName = \"Nynorsk\"\n",
		"config/_default/params.nn.toml", "color = \"blå\"\n",
		"config/production/hugo.toml", "baseURL = \"https://prod.org/\"\n[params]\ncolor = \"green\"\n",
		"config/production/params.toml", "env = \"prod\"\n",
		"config/staging/hugo.toml", "baseURL = \"https://staging.org/\"\n",
		"config/staging/nested/deep.toml", "x = 1\n",
		"config/development/hugo.yaml", "baseURL: https://dev.org/\n",
		"config/_default/notes.txt", "not config\n",
	)
	cs = append(cs, std("configdir/production", base))
	c := std("configdir/staging", base)
	c.Environment = "staging"
	cs = append(cs, c)
	c = std("configdir/development", base)
	c.Environment = "development"
	cs = append(cs, c)
	c = std("configdir/unknown-env", base)
	c.Environment = "nope"
	cs = append(cs, c)
	cs = append(cs, std("configdir/with-root-file", with(base, "hugo.toml", "title = \"Root file\"\nbaseURL = \"https://root.org/\"\n[params]\nroot = true\n")))
	c = std("configdir/custom-dir", with(dirs, "myconf/_default/hugo.toml", "title = \"Custom dir\"\n", "myconf/production/params.toml", "p = 1\n"))
	c.ConfigDir = "myconf"
	cs = append(cs, c)
	cs = append(cs, std("configdir/only-env", with(dirs, "config/production/hugo.toml", "title = \"only prod\"\n")))
	cs = append(cs, std("configdir/invalid-file", with(dirs, "config/_default/hugo.toml", "title = \"x\"\n", "config/_default/params.toml", "a = [\n")))
	cs = append(cs, std("configdir/internal-dropped", with(dirs, "config/_default/hugo.toml", "title = \"x\"\n[internal]\nwatch = true\n", "config/_default/minify.toml", "minifyOutput = true\n")))
	cs = append(cs, std("configdir/merge-keys", with(dirs,
		"config/_default/hugo.toml", "title = \"x\"\n[params]\n_merge = \"none\"\na = 1\n",
		"config/production/hugo.toml", "[params]\nb = 2\n[outputFormats.custom]\nmediaType = \"text/plain\"\n",
		"config/_default/outputFormats.toml", "[mine]\nmediaType = \"text/html\"\nbaseName = \"mine\"\n",
	)))
	return cs
}

// languageCases: multilingual configs.
func languageCases() []caseSpec {
	var cs []caseSpec
	cs = append(cs, toml("languages/two", `baseURL = "https://example.org/"
title = "Root"
defaultContentLanguage = "en"
[params]
rootParam = "root"
shared = "root"
[languages.en]
weight = 1
languageName = "English"
title = "English title"
[languages.en.params]
shared = "en"
[languages.th]
weight = 2
languageName = "ไทย"
languageDirection = "ltr"
contentDir = "content_th"
[languages.th.params]
shared = "th"
thOnly = true
`))
	cs = append(cs, toml("languages/per-language-sections", `baseURL = "https://example.org/"
defaultContentLanguage = "fr"
[permalinks]
posts = "/:year/:slug/"
[outputs]
home = ["html"]
[taxonomies]
tag = "tags"
[languages.en]
weight = 2
disableKinds = ["taxonomy"]
[languages.en.permalinks.page]
posts = "/en/:slug/"
[languages.en.outputs]
home = ["html", "rss", "json"]
section = ["html"]
[languages.en.taxonomies]
category = "categories"
[languages.fr]
weight = 1
timeZone = "Europe/Paris"
summaryLength = 10
[languages.fr.menus]
[[languages.fr.menus.main]]
name = "Accueil"
url = "/"
[languages.fr.markup.goldmark.renderer]
unsafe = true
[languages.fr.imaging]
quality = 50
[languages.fr.imaging.exif]
disableDate = true
`))
	cs = append(cs, toml("languages/multihost", `defaultContentLanguage = "en"
[languages.en]
baseURL = "https://en.example.org/"
weight = 1
[languages.de]
baseURL = "https://de.example.org/sub/"
weight = 2
staticDir = ["static_de"]
`))
	cs = append(cs, toml("languages/multihost-one-disabled", `defaultContentLanguage = "en"
disableLanguages = ["de"]
[languages.en]
baseURL = "https://en.example.org/"
weight = 1
[languages.de]
baseURL = "https://de.example.org/"
weight = 2
`))
	cs = append(cs, toml("languages/disabled", `defaultContentLanguage = "en"
disableLanguages = ["sv"]
[languages.en]
weight = 1
[languages.nn]
weight = 2
disabled = true
[languages.sv]
weight = 3
[languages.da]
weight = 4
`))
	cs = append(cs, toml("languages/disable-default", `defaultContentLanguage = "en"
disableLanguages = ["en"]
[languages.en]
weight = 1
[languages.nn]
weight = 2
`))
	cs = append(cs, toml("languages/default-not-defined", `defaultContentLanguage = "fr"
[languages.en]
weight = 1
`))
	cs = append(cs, toml("languages/subdir", `defaultContentLanguage = "nn"
defaultContentLanguageInSubdir = true
[languages.en]
weight = 2
[languages.nn]
weight = 1
`))
	cs = append(cs, toml("languages/no-en", `[languages.nn]
weight = 2
[languages.sv]
weight = 1
`))
	cs = append(cs, toml("languages/no-en-default-case", `defaultContentLanguage = "SV"
[languages.nn]
[languages.sv]
`))
	cs = append(cs, toml("languages/weight-ties", `defaultContentLanguage = "de"
[languages.fr]
weight = 1
[languages.de]
weight = 1
[languages.en]
[languages.it]
weight = -1
`))
	cs = append(cs, toml("languages/single-with-block", `languageCode = "en-GB"
[languages.en]
languageName = "English"
`))
	cs = append(cs, toml("languages/single-no-root-code", `[languages.en]
weight = 1
`))
	cs = append(cs, toml("languages/none", "title = \"no languages\"\ndefaultContentLanguage = \"Nn\"\n"))
	cs = append(cs, toml("languages/none-root-code", "title = \"x\"\nlanguageCode = \"nb-NO\"\n"))
	cs = append(cs, toml("languages/invalid-type", "[languages]\nen = \"english\"\n"))
	cs = append(cs, toml("languages/invalid-weight", "[languages.en]\nweight = \"heavy\"\n"))
	cs = append(cs, toml("languages/invalid-timezone", "[languages.en]\ntimeZone = \"Mars/Olympus\"\n"))
	cs = append(cs, toml("languages/duplicate-resource-files", `[markup.goldmark]
duplicateResourceFiles = true
[languages.en]
weight = 1
title = "x"
[languages.nn]
weight = 2
`))
	cs = append(cs, toml("languages/render-hooks-set", `[markup.goldmark.renderHooks.image]
useEmbedded = "never"
[markup.goldmark.renderHooks.link]
enableDefault = true
[languages.en]
weight = 1
title = "x"
[languages.nn]
weight = 2
`))
	cs = append(cs, toml("languages/render-hooks-invalid", "[markup.goldmark.renderHooks.image]\nuseEmbedded = \"sometimes\"\n"))
	cs = append(cs, toml("languages/lang-overrides-root-scalars", `title = "Root"
paginate = 5
copyright = "root"
[languages.en]
weight = 1
copyright = "en"
[languages.de]
weight = 2
title = "Deutsch"
paginatePath = "seite"
disableLanguages = ["nn"]
[languages.nn]
weight = 3
`))
	cs = append(cs, toml("languages/lang-params-merge", `[params]
a = "root"
[params.deep]
x = 1
y = 1
[languages.en]
weight = 1
[languages.en.params]
b = "en"
[languages.en.params.deep]
y = 2
[languages.nn]
weight = 2
[languages.nn.params]
_merge = "none"
c = "nn"
`))
	cs = append(cs, toml("languages/lang-menus-merge", `[[menus.main]]
name = "Root"
url = "/"
[[menus.footer]]
name = "Foot"
url = "/f/"
[languages.en]
weight = 1
[[languages.en.menus.main]]
name = "EN"
url = "/en/"
[languages.nn]
weight = 2
`))
	return cs
}

// envCases: HUGO_* environment overrides.
func envCases() []caseSpec {
	cfg := `title = "file"
baseURL = "https://example.org/"
[params]
count = 3
ratio = 1.5
flag = false
name = "n"
list = ["a", "b"]
[params.nested]
deep = "d"
[markup.goldmark.renderer]
unsafe = false
[languages.en]
weight = 1
`
	var cs []caseSpec
	add := func(name string, env ...string) {
		c := toml("env/"+name, cfg)
		c.Environ = append([]string{"NEOHUGO_ORACLE=1"}, env...)
		cs = append(cs, c)
	}
	add("title", "HUGO_TITLE=from env")
	add("params-new", "HUGO_PARAMS_NEWKEY=new")
	add("params-typed", "HUGO_PARAMS_COUNT=42", "HUGO_PARAMS_RATIO=2.25", "HUGO_PARAMS_FLAG=true", "HUGO_PARAMS_NAME=env")
	add("params-typed-invalid", "HUGO_PARAMS_COUNT=many", "HUGO_PARAMS_FLAG=maybe")
	add("params-list", "HUGO_PARAMS_LIST=[\"x\", \"y\", \"z\"]")
	add("params-nested", "HUGO_PARAMS_NESTED_DEEP=env deep", "HUGO_PARAMS_NESTED_NEW=added")
	add("params-missing-parent", "HUGO_PARAMS_NOPE_KEY=x")
	add("custom-delim", "HUGOxPARAMSxNESTEDxDEEP=x delim", "HUGO-TITLE=dash")
	add("too-short", "HUGO_=x", "HUGO=y", "HUGOX=z")
	add("bool-root", "HUGO_ENABLEEMOJI=true", "HUGO_BUILDDRAFTS=1", "HUGO_CANONIFYURLS=false")
	add("disable-kinds", "HUGO_DISABLEKINDS=taxonomy,term")
	add("disable-kinds-fields", "HUGO_DISABLEKINDS=taxonomy  term")
	add("disable-languages", "HUGO_DISABLELANGUAGES=fr de")
	add("decoder-map", "HUGO_TAXONOMIES={\"tag\": \"tags\", \"series\": \"series\"}")
	add("decoder-map-toml", "HUGO_TAXONOMIES=tag = \"tags\"\nseries = \"series\"")
	add("decoder-not-map", "HUGO_SITEMAP=weekly")
	add("markup-nested", "HUGO_MARKUP_GOLDMARK_RENDERER_UNSAFE=true", "HUGO_MARKUP_HIGHLIGHT_STYLE=dracula")
	add("cachedir", "HUGO_CACHEDIR=$ROOT/envcache")
	add("cachedir-empty", "HUGO_CACHEDIR=")
	add("environment-key", "HUGO_ENVIRONMENT=staging", "HUGO_ENV=dev")
	add("module-replacements", "HUGO_MODULE_REPLACEMENTS=github.com/a/b -> ../b")
	add("module-workspace-missing", "HUGO_MODULE_WORKSPACE=nope.work")
	add("security", "HUGO_SECURITY_EXEC_ALLOW=['^go$']", "HUGO_SECURITY_FUNCS_GETENV=[\"^HUGO_\", \"^FOO$\"]")
	add("minify", "HUGO_MINIFY_MINIFYOUTPUT=true", "HUGO_MINIFY_TDEWOLFF_HTML_KEEPCOMMENTS=true")
	add("pagination", "HUGO_PAGINATION_PAGERSIZE=7")
	add("languages-en-title", "HUGO_LANGUAGES_EN_TITLE=Env EN")
	add("upper-values", "HUGO_PARAMS_Mixed_Case=MiXeD")
	add("params-whole-map", "HUGO_PARAMS={\"fromEnv\": true}")
	add("services", "HUGO_SERVICES_RSS_LIMIT=5", "HUGO_SERVICES_GOOGLEANALYTICS_ID=G-1")
	return cs
}

// sectionCases: every decoded section with custom and invalid values.
func sectionCases() []caseSpec {
	var cs []caseSpec
	add := func(name, cfg string) {
		cs = append(cs, toml("sections/"+name, cfg))
	}
	add("cascade", `[[cascade]]
[cascade.params]
color = "blue"
[cascade.target]
kind = "page"
path = "/blog/**"
[[cascade]]
[cascade._target]
lang = "en"
[cascade.params]
lang = true
[[cascade]]
title = "legacy top-level"
[cascade.target]
path = "/docs/**.md"
`)
	add("cascade-map", `[cascade]
color = "red"
[cascade._target]
kind = "section"
`)
	add("segments", `renderSegments = ["docs", "missing"]
[segments.docs]
[[segments.docs.includes]]
kind = "{home,section}"
[[segments.docs.includes]]
path = "{/docs,/docs/**}"
lang = "en"
[[segments.docs.excludes]]
output = "rss"
[segments.other]
[[segments.other.excludes]]
lang = "fr"
`)
	add("segments-none-rendered", `[segments.docs]
[[segments.docs.excludes]]
path = "/docs/**"
`)
	add("segments-empty-render", "renderSegments = []\n[segments.docs]\n[[segments.docs.includes]]\nkind = \"page\"\n")
	add("segments-invalid-glob", "[segments.bad]\n[[segments.bad.includes]]\npath = \"[\"\n")
	add("segments-invalid-type", "[segments.bad]\nincludes = \"x\"\n")
	add("deployment", `[deployment]
order = [".jpg$", ".gif$"]
confirm = true
maxDeletes = -1
workers = 0
[[deployment.targets]]
name = "production"
URL = "s3://bucket?region=us-east-1"
cloudFrontDistributionID = "E1"
include = "**.html"
exclude = "**/*.map"
stripIndexHTML = true
[[deployment.matchers]]
pattern = "^.+\\.(js|css|svg|ttf)$"
cacheControl = "max-age=31536000, no-transform, public"
gzip = true
[[deployment.matchers]]
pattern = "^sitemap\\.xml$"
contentType = "application/xml"
force = true
`)
	add("deployment-empty-target", "[[deployment.targets]]\n")
	add("deployment-empty-matcher", "[[deployment.matchers]]\n")
	add("deployment-bad-pattern", "[[deployment.matchers]]\npattern = \"(\"\n")
	add("deployment-bad-order", "[deployment]\norder = [\"[a\"]\n")
	add("deployment-bad-glob", "[[deployment.targets]]\nname = \"x\"\ninclude = \"[\"\n")
	add("related", `[related]
threshold = 60
includeNewer = true
toLower = true
[[related.indices]]
name = "Keywords"
weight = 100
[[related.indices]]
name = "date"
weight = 10
pattern = "2006"
[[related.indices]]
name = "fragmentrefs"
type = "fragments"
applyFilter = true
cardinalityThreshold = 50
`)
	add("related-invalid-threshold", "[related]\nthreshold = 101\n")
	add("related-invalid-type", "[related]\n[[related.indices]]\nname = \"x\"\ntype = \"nope\"\n")
	add("related-empty", "[related]\n")
	add("related-no-tag", "[taxonomies]\ncategory = \"categories\"\n")
	add("sitemap", "[sitemap]\nchangeFreq = \"daily\"\npriority = 0.5\nfilename = \"map.xml\"\ndisable = true\n")
	add("sitemap-invalid", "[sitemap]\npriority = \"high\"\n")
	add("taxonomies", "[taxonomies]\nauthor = \"authors\"\nseries = \"series\"\n_merge = \"none\"\n")
	add("taxonomies-empty", "[taxonomies]\n")
	add("permalinks", `[permalinks]
[permalinks.page]
posts = "/:year/:month/:slug/"
docs = "/:sections/:filename/"
[permalinks.section]
posts = "/articles/"
[permalinks.taxonomy]
tags = "/topics/"
[permalinks.term]
tags = "/topics/:slugorfilename/"
`)
	add("permalinks-legacy", "[permalinks]\nposts = \"/:year/:title/\"\ntags = \"/tag/:slug/\"\n")
	add("permalinks-invalid", "[permalinks.page]\nposts = \"/:nope/\"\n")
	add("outputs", `[outputs]
home = ["HTML", "RSS", "JSON", "calendar"]
page = "html"
section = ["html", "nope"]
taxonomyTerm = ["html"]
unknownkind = ["html"]
`)
	add("outputs-rss-disabled", "disableKinds = [\"rss\"]\n[outputs]\nhome = [\"html\", \"rss\"]\n")
	add("output-formats", `[outputFormats]
[outputFormats.MyFormat]
mediaType = "text/plain"
baseName = "my"
isPlainText = true
protocol = "bep://"
[outputFormats.html]
noUgly = true
[outputs]
home = ["html", "myformat"]
`)
	add("output-formats-unknown-media", "[outputFormats.x]\nmediaType = \"text/nope\"\n")
	add("media-types", `[mediaTypes]
[mediaTypes."text/enriched"]
suffixes = ["enr", "rtf"]
delimiter = "."
[mediaTypes."text/html"]
suffixes = ["htm", "html"]
[outputFormats.enriched]
mediaType = "text/enriched"
`)
	add("content-types", "[contentTypes]\n[contentTypes.'text/markdown']\n[contentTypes.'text/html']\n")
	add("content-types-unknown", "[contentTypes]\n[contentTypes.'text/nope']\n")
	add("minify", `[minify]
disableCSS = true
disableXML = true
minifyOutput = true
[minify.tdewolff.html]
keepComments = true
keepWhitespace = true
templateDelims = ["{{", "}}"]
[minify.tdewolff.css]
precision = 3
keepCSS2 = false
[minify.tdewolff.js]
keepVarNames = true
version = 2019
[minify.tdewolff.json]
keepNumbers = true
[minify.tdewolff.svg]
precision = 2
`)
	add("minify-bool", "minify = true\n")
	add("security", `[security]
enableInlineShortcodes = true
[security.exec]
allow = ["^go$", "^npx$"]
osEnv = "none"
[security.funcs]
getenv = ["^HUGO_"]
[security.http]
methods = ["(?i)GET|POST|PUT"]
urls = ["^https://example\\.org/"]
mediaTypes = ["^application/json$"]
`)
	add("security-invalid", "[security.exec]\nallow = [\"(\"]\n")
	add("privacy", `[privacy]
[privacy.twitter]
disable = true
enableDNT = true
simple = true
[privacy.youtube]
privacyEnhanced = true
[privacy.vimeo]
disable = true
[privacy.googleAnalytics]
respectDoNotTrack = true
[privacy.instagram]
simple = true
[privacy.disqus]
disable = true
`)
	add("services", `[services]
[services.disqus]
shortname = "sn"
[services.googleAnalytics]
id = "G-123"
[services.instagram]
disableInlineCSS = true
accessToken = "tok"
[services.twitter]
disableInlineCSS = true
[services.rss]
limit = 20
`)
	add("pagination", "[pagination]\npagerSize = 5\npath = \"seite\"\ndisableAliases = true\n")
	add("pagination-legacy", "paginate = 7\npaginatePath = \"p\"\n")
	add("page", "[page]\nnextPrevSortOrder = \"ASC\"\nnextPrevInSectionSortOrder = \"asc\"\n")
	add("page-invalid", "[page]\nnextPrevSortOrder = \"sideways\"\n")
	add("server", `[server]
[[server.headers]]
for = "/*"
[server.headers.values]
X-Frame-Options = "DENY"
Referrer-Policy = "strict-origin"
[[server.redirects]]
from = "/old/**"
to = "/new/"
status = 301
force = true
[[server.redirects]]
fromRe = "^/foo/(.*)$"
to = "/bar/$1"
[server.redirects.fromHeaders]
Accept-Language = "nn"
`)
	add("server-invalid-status", "[[server.redirects]]\nfrom = \"/**\"\nto = \"/x\"\nstatus = 200\n")
	add("build", `[build]
useResourceCacheWhen = "always"
noJSConfigInAssets = true
[build.buildStats]
enable = true
disableTags = true
disableIDs = true
[[build.cachebusters]]
source = "assets/.*\\.json"
target = "(js|css)"
`)
	add("build-invalid-cachebuster", "[[build.cachebusters]]\nsource = \"(\"\ntarget = \"x\"\n")
	add("caches", `ignoreCache = false
[caches]
[caches.images]
dir = ":cacheDir/images"
maxAge = "1440h"
[caches.getresource]
dir = ":resourceDir/remote"
maxAge = 3600
[caches.misc]
dir = ":project/misc"
maxAge = -1
`)
	add("caches-invalid-maxage", "[caches.misc]\nmaxAge = \"-1\"\n")
	add("caches-ignore", "ignoreCache = true\n[caches.images]\nmaxAge = \"10h\"\n")
	add("caches-invalid", "[caches]\nimages = \"x\"\n")
	add("httpcache", `[httpcache]
[httpcache.cache.for]
includes = ["https://example.org/**"]
excludes = ["**.json"]
[[httpcache.polls]]
low = "1s"
high = "10s"
[httpcache.polls.for]
includes = ["https://example.org/**"]
[[httpcache.polls]]
disable = true
[httpcache.polls.for]
includes = ["**"]
`)
	add("httpcache-invalid", "[httpcache.cache.for]\nincludes = [\"[\"]\n")
	add("frontmatter", `[frontmatter]
date = ["myDate", ":default"]
lastmod = [":fileModTime", ":default"]
publishDate = [":filename", "pubdate"]
expiryDate = ["expires"]
`)
	add("markup", `[markup]
defaultMarkdownHandler = "goldmark"
[markup.highlight]
style = "dracula"
hl_Lines = "1-3 5"
lineNoStart = 3
tabWidth = 2
noClasses = false
[markup.tableOfContents]
startLevel = 1
endLevel = 6
ordered = true
[markup.goldmark.parser]
autoHeadingIDType = "github-ascii"
[markup.goldmark.parser.attribute]
block = true
[markup.goldmark.extensions]
typographer = false
linkifyProtocol = "http"
[markup.goldmark.extensions.passthrough]
enable = true
[markup.goldmark.extensions.passthrough.delimiters]
inline = [["$", "$"]]
[markup.goldmark.extensions.extras.mark]
enable = true
[markup.goldmark.extensions.cjk]
enable = true
eastAsianLineBreaksStyle = "css3draft"
[markup.asciidocExt]
extensions = ["asciidoctor-html5s"]
[markup.asciidocExt.attributes]
my-attr = "v"
`)
	add("markup-legacy-bools", "[markup.goldmark.parser]\nattribute = false\n[markup.goldmark.extensions]\ntypographer = true\n[markup.highlight]\nhl_Lines = \"2\"\n")
	add("markup-invalid-handler", "[markup]\ndefaultMarkdownHandler = \"nope\"\n")
	add("imaging", `[imaging]
quality = 90
resampleFilter = "Lanczos"
anchor = "TopLeft"
bgColor = "#000"
hint = "picture"
[imaging.exif]
includeFields = "Artist|Copyright"
disableLatLong = true
`)
	add("imaging-invalid", "[imaging]\nquality = 200\n")
	add("imaging-invalid-filter", "[imaging]\nresampleFilter = \"nope\"\n")
	add("menus", `[[menus.main]]
name = "Home"
pageRef = "/"
weight = 1
identifier = "home"
[[menus.main]]
name = "Docs"
url = "/docs/"
parent = "home"
pre = "<i>"
post = "</i>"
title = "Documentation"
[menus.main.params]
class = "docs"
[[menus.footer]]
name = "About"
url = "/about/"
`)
	add("menus-invalid", "[menus]\nmain = \"x\"\n")
	add("module-config", `[module]
noVendor = "github.com/**"
vendorClosest = true
replacements = "github.com/a/b -> ../b, github.com/c/d->/abs/d"
proxy = "https://proxy.example.org"
noProxy = "none"
private = "github.com/private/*"
auth = "netrc"
[module.hugoVersion]
min = "0.100.0"
max = "0.200.0"
extended = true
[module.params]
Author = "me"
`)
	add("module-replacements-invalid", "[module]\nreplacements = [\"github.com/a/b\"]\n")
	add("module-hugo-version-too-new", "[module.hugoVersion]\nmin = \"99.0.0\"\n")
	return cs
}

// mountCases: module mounts of the project and legacy dirs.
func mountCases() []caseSpec {
	var cs []caseSpec
	cs = append(cs, std("mounts/defaults-missing-dirs", fs("hugo.toml", "title = \"no dirs\"\n")))
	cs = append(cs, toml("mounts/configured", `[module]
[[module.mounts]]
source = "content"
target = "content"
[[module.mounts]]
source = "content-extra/"
target = "content/extra"
lang = "en"
includeFiles = "**/*.md"
excludeFiles = ["**/draft/**", "*.tmp"]
[[module.mounts]]
source = "./assets/"
target = "assets"
disableWatch = true
[[module.mounts]]
source = "node_modules/bootstrap/scss"
target = "assets/bootstrap/scss"
[[module.mounts]]
source = "missing"
target = "static"
[[module.mounts]]
source = "public"
target = "static/public"
[[module.mounts]]
source = "hugo_stats.json"
target = "assets/watching/hugo_stats.json"
[[module.mounts]]
source = "content"
target = "content"
`))
	cs[len(cs)-1].Files = with(cs[len(cs)-1].Files, "content-extra/", "", "node_modules/bootstrap/scss/", "")
	cs = append(cs, toml("mounts/abs-source", "[[module.mounts]]\nsource = \"$ROOT/outside\"\ntarget = \"static\"\n"))
	cs[len(cs)-1].Files = with(cs[len(cs)-1].Files, "../outside/", "")
	cs = append(cs, toml("mounts/invalid-target", "[[module.mounts]]\nsource = \"content\"\ntarget = \"contents\"\n"))
	cs = append(cs, toml("mounts/empty-source", "[[module.mounts]]\nsource = \"\"\ntarget = \"content\"\n"))
	cs = append(cs, toml("mounts/only-static", "[[module.mounts]]\nsource = \"files\"\ntarget = \"static\"\n"))
	cs[len(cs)-1].Files = with(cs[len(cs)-1].Files, "files/", "")
	cs = append(cs, toml("mounts/lang-content-dirs", `defaultContentLanguage = "nn"
contentDir = "content"
staticDir = ["static", "static_shared"]
[languages.en]
weight = 1
contentDir = "content_en"
staticDir = "static_en"
[languages.nn]
weight = 2
[languages.sv]
weight = 3
contentDir = "content_sv"
disabled = true
`))
	cs = append(cs, toml("mounts/lang-content-dirs-multihost", `defaultContentLanguage = "en"
[languages.en]
weight = 1
baseURL = "https://en.org/"
contentDir = "content_en"
[languages.nn]
weight = 2
baseURL = "https://nn.org/"
staticDir = ["static_nn", "static"]
`))
	cs = append(cs, toml("mounts/legacy-dirs", `contentDir = "c"
layoutDir = "l"
dataDir = "d"
i18nDir = "i"
archetypeDir = "a"
assetDir = "as"
staticDir = "s"
staticDir1 = ["s1", "s"]
`))
	cs[len(cs)-1].Files = with(cs[len(cs)-1].Files, "c/", "", "l/", "", "d/", "", "i/", "", "a/", "", "as/", "", "s/", "", "s1/", "")
	cs = append(cs, std("mounts/jsconfig-one", with(dirs, "hugo.toml", "title = \"x\"\n", "babel.config.js", "", "notes.config.js", "")))
	cs = append(cs, std("mounts/jsconfig-all", with(dirs, "hugo.toml", "title = \"x\"\n", "package.json", "{}", "package.hugo.json", "{}", "postcss.config.js", "", "tailwind.config.js", "", "xbabel.config.jsx", "")))
	cs = append(cs, std("mounts/jsconfig-user", with(dirs, "hugo.toml", "[[module.mounts]]\nsource = \"postcss.config.js\"\ntarget = \"assets/_jsconfig/pc.js\"\n", "postcss.config.js", "", "package.json", "{}")))
	cs = append(cs, toml("mounts/publish-dir-mount", "publishDir = \"out\"\n[[module.mounts]]\nsource = \"out\"\ntarget = \"static/out\"\n"))
	return cs
}

// themeCases: themes and module imports resolvable without Go.
func themeCases() []caseSpec {
	var cs []caseSpec
	theme := fs(
		"themes/mytheme/hugo.toml", `title = "theme title"
[params]
themeParam = "t"
shared = "theme"
[params.deep]
a = "theme"
b = "theme"
[menus]
[[menus.main]]
name = "Theme menu"
url = "/theme/"
[outputFormats.themeformat]
mediaType = "text/plain"
[outputs]
home = ["html", "rss", "themeformat"]
[mediaTypes."text/theme"]
suffixes = ["thm"]
[imaging]
quality = 60
[languages.en.params]
fromTheme = true
`,
		"themes/mytheme/layouts/", "",
		"themes/mytheme/static/", "",
		"themes/mytheme/i18n/", "",
		"themes/mytheme/package.json", "{}",
	)
	site := with(dirs, "hugo.toml", `theme = "mytheme"
title = "site title"
[params]
shared = "site"
[params.deep]
b = "site"
[languages.en]
weight = 1
`)
	for k, v := range theme {
		site[k] = v
	}
	cs = append(cs, std("themes/one", site))
	cs = append(cs, std("themes/theme-toml", with(dirs,
		"hugo.toml", "theme = [\"legacy\"]\n",
		"themes/legacy/theme.toml", "name = \"Legacy\"\nmin_version = \"0.40\"\nlicense = \"MIT\"\n[author]\nName = \"Me\"\n",
		"themes/legacy/layouts/", "",
	)))
	cs = append(cs, std("themes/theme-toml-invalid", with(dirs,
		"hugo.toml", "theme = \"bad\"\n",
		"themes/bad/theme.toml", "name = [\n",
		"themes/bad/layouts/", "",
	)))
	cs = append(cs, std("themes/config-dir", with(dirs,
		"hugo.toml", "theme = \"cd\"\n",
		"themes/cd/config/_default/params.toml", "p = \"default\"\n",
		"themes/cd/config/production/params.toml", "p = \"prod\"\n",
		"themes/cd/config.yaml", "params:\n  q: yaml\n",
		"themes/cd/assets/", "",
	)))
	cs = append(cs, std("themes/nested-imports", with(dirs,
		"hugo.toml", "theme = [\"a\", \"b\"]\n",
		"themes/a/hugo.toml", "[module]\n[[module.imports]]\npath = \"c\"\n[[module.imports]]\npath = \"b\"\n[params]\nfrom = \"a\"\n",
		"themes/a/layouts/", "",
		"themes/b/hugo.toml", "[params]\nfrom = \"b\"\nbOnly = true\n",
		"themes/b/content/", "",
		"themes/b/data/", "",
		"themes/c/layouts/", "",
		"themes/c/archetypes/", "",
	)))
	cs = append(cs, std("themes/import-options", with(dirs,
		"hugo.toml", `[module]
[[module.imports]]
path = "noconf"
ignoreConfig = true
[[module.imports]]
path = "noimports"
ignoreImports = true
[[module.imports]]
path = "nomounts"
noMounts = true
[[module.imports]]
path = "disabled"
disable = true
[[module.imports]]
path = "withmounts"
[[module.imports.mounts]]
source = "src"
target = "assets/src"
[[module.imports.mounts]]
source = "notthere"
target = "layouts"
`,
		"themes/noconf/hugo.toml", "[params]\nignored = true\n",
		"themes/noconf/layouts/", "",
		"themes/noimports/hugo.toml", "[[module.imports]]\npath = \"deep\"\n",
		"themes/noimports/layouts/", "",
		"themes/deep/layouts/", "",
		"themes/nomounts/layouts/", "",
		"themes/withmounts/src/", "",
		"themes/withmounts/layouts/", "",
	)))
	cs = append(cs, std("themes/missing", with(dirs, "hugo.toml", "theme = \"nothere\"\n")))
	c := std("themes/missing-ignored", with(dirs, "hugo.toml", "theme = [\"nothere\", \"there\"]\n", "themes/there/layouts/", ""))
	c.IgnoreModuleDoesNotExist = true
	cs = append(cs, c)
	cs = append(cs, std("themes/themes-dir", with(dirs, "hugo.toml", "themesDir = \"../shared-themes\"\ntheme = \"t\"\n", "../shared-themes/t/layouts/", "", "../shared-themes/t/hugo.toml", "[params]\nshared = true\n")))
	cs = append(cs, std("themes/abs-import", with(dirs, "hugo.toml", "[[module.imports]]\npath = \"$ROOT/abs-theme\"\n", "../abs-theme/layouts/", "", "../abs-theme/hugo.toml", "title = \"abs\"\n")))
	cs = append(cs, std("themes/relative-outside", with(dirs, "hugo.toml", "[[module.imports]]\npath = \"../outside\"\n", "../outside/layouts/", "")))
	cs = append(cs, std("themes/nested-relative-outside", with(dirs,
		"hugo.toml", "theme = \"a\"\n",
		"themes/a/hugo.toml", "[[module.imports]]\npath = \"../../x\"\n",
		"themes/a/layouts/", "",
	)))
	cs = append(cs, std("themes/replacements", with(dirs,
		"hugo.toml", "[module]\nreplacements = \"github.com/me/theme -> mytheme\"\n[[module.imports]]\npath = \"github.com/me/theme\"\n",
		"themes/mytheme/layouts/", "",
		"themes/mytheme/hugo.toml", "[params]\nreplaced = true\n",
	)))
	cs = append(cs, std("themes/vendored", with(dirs,
		"hugo.toml", "[module]\n[[module.imports]]\npath = \"github.com/me/vtheme\"\n",
		"go.mod", "module example.org/site\n\ngo 1.22\n\nrequire github.com/me/vtheme v1.2.3\n",
		"_vendor/modules.txt", "# github.com/me/vtheme v1.2.3\n# github.com/me/other v0.1.0\n",
		"_vendor/github.com/me/vtheme/hugo.toml", "[params]\nvendored = true\n[[module.imports]]\npath = \"github.com/me/other\"\n",
		"_vendor/github.com/me/vtheme/layouts/", "",
		"_vendor/github.com/me/other/assets/", "",
	)))
	cs = append(cs, std("themes/vendored-no-gomod", with(dirs,
		"hugo.toml", "[module]\n[[module.imports]]\npath = \"github.com/me/vtheme\"\n",
		"_vendor/modules.txt", "# github.com/me/vtheme v1.2.3\n",
		"_vendor/github.com/me/vtheme/layouts/", "",
	)))
	cs = append(cs, std("themes/vendored-invalid", with(dirs,
		"hugo.toml", "[module]\n[[module.imports]]\npath = \"github.com/me/vtheme\"\n",
		"_vendor/modules.txt", "# github.com/me/vtheme\n",
	)))
	cs = append(cs, std("themes/gomod-no-imports", with(dirs,
		"hugo.toml", "theme = \"t\"\n",
		"go.mod", "module example.org/site\n",
		"themes/t/layouts/", "",
	)))
	cs = append(cs, std("themes/theme-lang-mounts", with(dirs,
		"hugo.toml", "theme = \"t\"\ndefaultContentLanguage = \"en\"\n[languages.en]\nweight = 1\n[languages.nn]\nweight = 2\n",
		"themes/t/hugo.toml", "[[module.mounts]]\nsource = \"content/nn\"\ntarget = \"content\"\nlang = \"nn\"\n[[module.mounts]]\nsource = \"layouts\"\ntarget = \"layouts\"\n",
		"themes/t/content/nn/", "",
		"themes/t/layouts/", "",
	)))
	cs = append(cs, std("themes/theme-outputs-transient", with(dirs,
		"hugo.toml", "theme = \"t\"\n[outputs]\nhome = [\"html\", \"themeformat\"]\n",
		"themes/t/hugo.toml", "[outputFormats.themeformat]\nmediaType = \"text/plain\"\n",
		"themes/t/layouts/", "",
	)))
	cs = append(cs, std("themes/unknown-output-format", with(dirs,
		"hugo.toml", "[outputs]\nhome = [\"html\", \"nope\"]\n",
	)))
	return cs
}

// mergeCases: `_merge` strategies with a theme.
func mergeCases() []caseSpec {
	theme := fs(
		"themes/t/layouts/", "",
		"themes/t/hugo.toml", `title = "theme"
copyright = "theme copyright"
[params]
tp = "theme"
[params.deep]
a = "theme"
b = "theme"
[params.deep.deeper]
x = 1
[menus]
[[menus.main]]
name = "Theme"
url = "/t/"
[[menus.footer]]
name = "Theme footer"
url = "/tf/"
[mediaTypes."text/theme"]
suffixes = ["thm"]
[outputFormats.themefmt]
mediaType = "text/theme"
[taxonomies]
theme = "themes"
[markup.goldmark.renderer]
unsafe = true
[languages.en.params]
themeLang = true
[languages.nn]
weight = 9
`,
	)
	site := func(name, cfg string) caseSpec {
		f := with(dirs, "hugo.toml", cfg)
		for k, v := range theme {
			f[k] = v
		}
		return std("merge/"+name, f)
	}
	return []caseSpec{
		site("defaults", "theme = \"t\"\n[params]\n[params.deep]\nb = \"site\"\n[languages.en]\nweight = 1\n"),
		site("params-none", "theme = \"t\"\n[params]\n_merge = \"none\"\nsp = 1\n"),
		site("params-shallow", "theme = \"t\"\n[params]\n_merge = \"shallow\"\n[params.deep]\nb = \"site\"\n"),
		site("root-deep", "theme = \"t\"\n_merge = \"deep\"\n[markup.goldmark.renderer]\nhardWraps = true\n"),
		site("root-shallow", "theme = \"t\"\n_merge = \"shallow\"\ntitle = \"site\"\n"),
		site("menus-none", "theme = \"t\"\n[menus]\n_merge = \"none\"\n[[menus.main]]\nname = \"Site\"\nurl = \"/\"\n"),
		site("menus-deep", "theme = \"t\"\n[menus]\n_merge = \"deep\"\n[[menus.main]]\nname = \"Site\"\nurl = \"/\"\n"),
		site("taxonomies-deep", "theme = \"t\"\n[taxonomies]\n_merge = \"deep\"\ntag = \"tags\"\n"),
		site("languages-deep", "theme = \"t\"\n[languages]\n_merge = \"deep\"\n[languages.en]\nweight = 1\n"),
		site("invalid-strategy", "theme = \"t\"\n[params]\n_merge = \"sideways\"\n"),
		toml("merge/imaging-root-and-lang", "[imaging]\nquality = 80\n[imaging.exif]\nincludeFields = \"a\"\n[languages.en]\nweight = 1\n[languages.fr]\nweight = 2\n[languages.fr.imaging]\nquality = 50\n"),
		toml("merge/segments-lang", "[segments.a]\n[[segments.a.includes]]\nkind = \"page\"\n[languages.en]\nweight = 1\n[languages.fr]\nweight = 2\n[languages.fr.segments.b]\n[[languages.fr.segments.b.excludes]]\nlang = \"fr\"\n"),
		toml("merge/lang-mediatypes", "[mediaTypes.\"text/x\"]\nsuffixes = [\"x\"]\n[languages.en]\nweight = 1\n[languages.fr]\nweight = 2\n[languages.fr.mediaTypes.\"text/y\"]\nsuffixes = [\"y\"]\n[languages.fr.outputFormats.yfmt]\nmediaType = \"text/y\"\n"),
	}
}

// allCases returns every oracle case.
func allCases(root string) ([]caseSpec, error) {
	var cases []caseSpec
	cases = append(cases, basicCases()...)
	ss, err := seeksnackCases(root)
	if err != nil {
		return nil, err
	}
	cases = append(cases, ss...)
	rc, err := repoCases(root)
	if err != nil {
		return nil, err
	}
	cases = append(cases, rc...)
	cases = append(cases, configDirCases()...)
	cases = append(cases, languageCases()...)
	cases = append(cases, envCases()...)
	cases = append(cases, sectionCases()...)
	cases = append(cases, mountCases()...)
	cases = append(cases, themeCases()...)
	cases = append(cases, mergeCases()...)
	// Stable order within a group.
	sort.SliceStable(cases, func(i, j int) bool {
		gi, _, _ := strings.Cut(cases[i].Name, "/")
		gj, _, _ := strings.Cut(cases[j].Name, "/")
		return gi < gj
	})
	return cases, nil
}
