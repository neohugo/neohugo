package main

import (
	"os"
	"path/filepath"
	"strings"
)

// buildSites returns the sites of the cases:
//
//   - seeksnack: the seeksnack reconstruction (crates/nh-allconfig) with the
//     directories that decide its mounts, an alternative config file, a
//     config dir and two themes directories;
//   - docs: the repository's docs/ site (config, module and package files;
//     the other top-level entries as empty directories);
//   - testsite / testsite-noconfig: hugolib/testsite (no config file of its
//     own) with and without a two-language hugo.toml;
//   - empty: an empty directory.
func buildSites(root string) (map[string]site, error) {
	toml, err := os.ReadFile(filepath.Join(root, "crates/nh-allconfig/tests/fixtures/load/seeksnack/hugo.toml"))
	if err != nil {
		return nil, err
	}
	seeksnack := map[string]string{
		"hugo.toml":                    string(toml),
		"content/":                     "",
		"static/":                      "",
		"layouts/":                     "",
		"data/":                        "",
		"assets/":                      "",
		"i18n/":                        "",
		"archetypes/":                  "",
		"node_modules/":                "",
		"package.json":                 "{}\n",
		"postcss.config.js":            "module.exports = {};\n",
		"alt/custom.toml":              "baseURL = \"https://alt.example.org/\"\ntitle = \"Alt\"\n[params]\nalt = true\n",
		"alt/second.toml":              "[params]\nsecond = \"yes\"\nalt = false\n",
		"cfg/_default/params.toml":     "fromConfigDir = \"default\"\n",
		"cfg/staging/params.toml":      "fromConfigDir = \"staging\"\n",
		"cfg/staging/hugo.toml":        "baseURL = \"https://staging.example.org/\"\n",
		"themes/mytheme/hugo.toml":     "[params]\nthemeParam = \"mytheme\"\n",
		"themes/mytheme/layouts/":      "",
		"themes/other/hugo.toml":       "[params]\nthemeParam = \"other\"\notherOnly = 1\n",
		"themes/other/static/":         "",
		"themes2/mytheme/hugo.toml":    "[params]\nthemeParam = \"themes2\"\n",
		"themes2/mytheme/i18n/":        "",
		"config/development/hugo.toml": "title = \"SeekSnack DEV\"\n",
	}

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
	tsNoConfig := map[string]string{}
	for k, v := range ts {
		tsNoConfig[k] = v
	}
	ts["hugo.toml"] = `baseURL = "https://example.org/"
defaultContentLanguage = "en"
[languages.en]
weight = 1
[languages.nn]
weight = 2
contentDir = "content_nn"
`
	return map[string]site{
		"seeksnack":         {Dir: "seeksnack", Files: seeksnack},
		"docs":              {Dir: "docs", Files: docs},
		"testsite":          {Dir: "testsite", Files: ts},
		"testsite-noconfig": {Dir: "testsite", Files: tsNoConfig},
		"empty":             {Dir: "empty", Files: map[string]string{}},
	}, nil
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

var (
	parse = []string{"parse"}
	both  = []string{"parse", "run"}
)

func buildCases() []caseSpec {
	var cs []caseSpec
	add := func(name, site string, modes []string, args ...string) {
		cs = append(cs, caseSpec{Name: name, Site: site, Args: args, Modes: modes})
	}
	addEnv := func(name, site string, modes []string, env map[string]string, args ...string) {
		cs = append(cs, caseSpec{Name: name, Site: site, Args: args, Modes: modes, Env: env})
	}

	// config / config mounts (run: they do not build).
	const s = "seeksnack"
	add("config/json", s, both, "config", "--format", "json")
	add("config/json-printzero", s, both, "config", "--format", "json", "--printZero")
	add("config/json-lang-th", s, both, "config", "--format=json", "--lang", "th")
	add("config/json-lang-missing", s, both, "config", "--format", "json", "--lang", "xx")
	add("config/json-upper", s, both, "config", "--format", "JSON")
	add("config/toml-default", s, both, "config")
	add("config/yaml", s, both, "config", "--format", "yaml")
	add("config/xml", s, both, "config", "--format", "xml")
	add("config/mounts", s, both, "config", "mounts")
	add("config/mounts-extra-arg", s, both, "config", "mounts", "extra")
	add("config/env-development", s, both, "config", "--format", "json", "-e", "development")
	add("config/env-staging", s, both, "config", "--format", "json", "--environment", "staging")
	add("config/baseurl-short", s, both, "config", "--format", "json", "-b", "https://example.com/sub/")
	add("config/baseurl-eq", s, both, "config", "--format", "json", "--baseURL=https://x.org/")
	add("config/cachedir", s, both, "config", "--format", "json", "--cacheDir", "$ROOT/cache")
	add("config/destination-short", s, both, "config", "--format", "json", "-d", "out")
	add("config/destination-abs", s, both, "config", "--format", "json", "--destination=$ROOT/pub")
	add("config/source-short", "empty", both, "config", "--format", "json", "-s", "$ROOT/seeksnack")
	add("config/source-relative", "empty", both, "config", "--format", "json", "--source=../seeksnack")
	add("config/clock", s, both, "config", "--format", "json", "--clock", "2026-09-27T12:00:00Z")
	add("config/clock-invalid", s, both, "config", "--format", "json", "--clock", "notatime")
	add("config/config-file", s, both, "config", "--format", "json", "--config", "alt/custom.toml")
	add("config/config-files", s, both, "config", "--format", "json", "--config", "alt/custom.toml,alt/second.toml")
	add("config/config-missing", s, both, "config", "--format", "json", "--config", "nope.toml")
	add("config/configdir", s, both, "config", "--format", "json", "--configDir", "cfg", "-e", "staging")
	add("config/theme", s, both, "config", "--format", "json", "--theme", "mytheme")
	add("config/themes", s, both, "config", "--format", "json", "-t", "mytheme,other")
	add("config/themes-repeated", s, both, "config", "--format", "json", "-t", "other", "-t", "mytheme")
	add("config/themesdir", s, both, "config", "--format", "json", "--themesDir", "themes2", "--theme", "mytheme")
	add("config/theme-missing", s, both, "config", "--format", "json", "--theme", "nope")
	add("config/contentdir", s, both, "config", "--format", "json", "-c", "mycontent")
	add("config/rendersegments", s, both, "config", "--format", "json", "--renderSegments", "a,b")
	add("config/nobuildlock", s, both, "config", "--format", "json", "--noBuildLock")
	add("config/ignorevendorpaths", s, both, "config", "--format", "json", "--ignoreVendorPaths", "**")
	add("config/quiet", s, both, "config", "--format", "json", "--quiet")
	add("config/rendertomemory", s, both, "config", "--format", "json", "--renderToMemory")
	add("config/rendertomemory-short", s, both, "config", "--format", "json", "-M")
	add("config/loglevel-warning", s, both, "config", "--format", "json", "--logLevel", "WARNING")
	add("config/loglevel-error", s, both, "config", "--format", "json", "--logLevel=error")
	add("config/loglevel-invalid", s, both, "config", "--format", "json", "--logLevel", "foo")
	add("config/mounts-theme", s, both, "config", "mounts", "--theme", "mytheme")
	addEnv("config/env-var-environment", s, both, map[string]string{"HUGO_ENVIRONMENT": "development"}, "config", "--format", "json")
	addEnv("config/env-var-params", s, both, map[string]string{"HUGO_PARAMS_FROMENV": "yes", "HUGO_TITLE": "From env"}, "config", "--format", "json")
	addEnv("config/env-var-cachedir", s, both, map[string]string{"HUGO_CACHEDIR": "$ROOT/hc"}, "config", "--format", "json")
	add("config/docs-json", "docs", both, "config", "--format", "json")
	add("config/docs-mounts", "docs", both, "config", "mounts")
	add("config/testsite-json", "testsite", both, "config", "--format", "json", "--lang", "nn")
	add("config/testsite-mounts", "testsite", both, "config", "mounts")
	add("config/testsite-noconfig", "testsite-noconfig", both, "config", "--format", "json")
	add("config/empty-dir", "empty", both, "config", "--format", "json")

	// Flags that config does not have, and flag syntax errors.
	add("config-err/minify", s, both, "config", "--minify")
	add("config-err/drafts-short", s, both, "config", "-D")
	add("config-err/printpathwarnings", s, both, "config", "--printPathWarnings")
	add("config-err/format-no-value", s, both, "config", "--format")
	add("config-err/printzero-invalid", s, both, "config", "--printZero=maybe")
	add("config-err/unknown-sub", s, both, "config", "foo")
	add("config-err/unknown-sub-json", s, both, "config", "foo", "--format", "json")
	add("config-err/sub-after-flag", s, both, "config", "--format", "json", "foo")
	add("config-err/mounts-format", s, both, "config", "mounts", "--format", "json")

	// version / env / help.
	add("version", s, both, "version")
	add("version/extra-arg", s, both, "version", "extra")
	add("version/quiet", s, both, "version", "--quiet")
	add("env", s, both, "env")
	add("help/short", s, both, "-h")
	add("help/long", s, both, "--help")
	add("help/command", s, both, "help")
	add("help/config", s, both, "config", "-h")
	add("help/build-minify", s, both, "--minify", "--help")

	// The build command: parse only (a run builds the site).
	add("build/golden", s, parse, "--minify", "--clock", "2026-09-27T12:00:00Z", "-d", "$ROOT/out")
	add("build/golden-subcommand", s, parse, "build", "--minify", "--clock", "2026-09-27T12:00:00Z", "-d", "$ROOT/out")
	add("build/drafts-future-expired", s, parse, "-D", "-F", "-E")
	add("build/drafts-combined", s, parse, "-DFE")
	add("build/drafts-long", s, parse, "--buildDrafts", "--buildFuture=true", "--buildExpired=false")
	add("build/drafts-eq", s, parse, "-D=true", "-F=0")
	add("build/destination-attached", s, parse, "-dout")
	add("build/destination-eq", s, parse, "-d=out")
	add("build/destination-empty", s, parse, "--destination=")
	add("build/drafts-then-destination", s, parse, "-Dd", "out")
	add("build/destination-D", s, parse, "-dD")
	add("build/gc-metrics", s, parse, "--gc", "--templateMetrics", "--templateMetricsHints")
	add("build/warnings", s, parse, "--printPathWarnings", "--printI18nWarnings", "--printUnusedTemplates", "--panicOnWarning")
	add("build/locks-vendor", s, parse, "--noBuildLock", "--ignoreVendorPaths", "github.com/*", "--ignoreCache")
	add("build/memory-quiet", s, parse, "--renderToMemory", "--quiet", "--logLevel", "info")
	add("build/memory-short", s, parse, "-M")
	add("build/dirs", s, parse, "--cacheDir", "$ROOT/c", "--config", "a.toml,b.toml", "--configDir", "cfg", "--themesDir", "t")
	add("build/themes", s, parse, "--theme", "a,b", "-t", "c")
	add("build/themes-quoted", s, parse, "--theme", `"a,b",c`)
	add("build/themes-empty", s, parse, "--theme=")
	add("build/env-baseurl-source", s, parse, "-e", "staging", "--environment=dev", "-b", "https://example.org/", "-s", ".")
	add("build/static-flags", s, parse, "--cleanDestinationDir", "--noTimes", "--noChmod", "--forceSyncStatic")
	add("build/content-layout-dirs", s, parse, "-l", "lay", "-c", "cont", "--disableKinds", "RSS,sitemap", "--renderSegments", "seg")
	add("build/watch-poll", s, parse, "-w", "--poll", "700ms")
	add("build/profiles", s, parse, "--trace", "t.out", "--profile-cpu", "c.out", "--profile-mem", "m.out", "--profile-mutex", "x.out", "--printMemoryUsage")
	add("build/devmode-gitinfo", s, parse, "--devMode", "--enableGitInfo")
	add("build/positional", s, parse, "--minify", "--", "--notaflag", "x")
	add("build/bool-then-word", s, both, "--minify", "abc")
	add("build/subcommand-positional", s, parse, "build", "extra")

	// Command lines that fail before a build (run is safe).
	add("err/unknown-flag", s, both, "--minfy")
	add("err/unknown-shorthand", s, both, "-x")
	add("err/unknown-shorthand-combined", s, both, "-Dx")
	add("err/bool-invalid", s, both, "--minify=maybe")
	add("err/bool-invalid-short", s, both, "-D=maybe")
	add("err/short-needs-arg", s, both, "-d")
	add("err/long-needs-arg", s, both, "--destination")
	add("err/bad-syntax-dashes", s, both, "---x")
	add("err/bad-syntax-eq", s, both, "--=x")
	add("err/csv-bare-quote", s, both, "--theme", `a"b`)
	add("err/csv-quote", s, both, "-t", `"a`)
	add("err/unknown-command-typo", s, both, "bulid")
	add("err/unknown-command-prefix", s, both, "co")
	add("err/unknown-command-srever", s, both, "srever")
	add("err/unknown-command", s, both, "foo")
	add("err/unknown-command-after-flag", s, both, "-d", "out", "foo")
	add("err/loglevel-invalid", s, both, "--logLevel", "foo")
	add("err/clock-invalid", s, both, "--clock", "notatime", "--renderToMemory")
	add("err/no-config", "empty", both, "--renderToMemory")
	add("err/build-no-config", "empty", both, "build", "--renderToMemory")
	add("err/source-missing", s, both, "-s", "$ROOT/nope", "--renderToMemory")

	// Commands the port does not support: parse only (a run would start them).
	add("unsupported/server", s, parse, "server", "--port", "1313")
	add("unsupported/server-alias", s, parse, "serve", "-D")
	add("unsupported/new-legacy", s, parse, "new", "posts/a.md")
	add("unsupported/new-site", s, parse, "new", "site", "x")
	add("unsupported/mod-graph", s, parse, "mod", "graph")
	add("unsupported/mod-npm-pack", s, parse, "mod", "npm", "pack")
	add("unsupported/deploy", s, parse, "deploy")
	add("unsupported/gen-man", s, parse, "gen", "man")
	add("unsupported/list-drafts", s, parse, "list", "drafts")
	add("unsupported/import", s, parse, "import", "jekyll", "a", "b")
	add("unsupported/convert", s, parse, "convert", "toJSON")
	add("unsupported/completion", s, parse, "completion", "bash")
	add("unsupported/release", s, parse, "release")
	return cs
}
