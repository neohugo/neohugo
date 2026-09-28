// Command misc is the Go oracle for the smaller parts of crates/nh-config (Wave
// B task T04): config/env.go, common/neohugo (versions, HugoInfo,
// GetExecEnviron, deprecations), the config loader (FromConfigString,
// LoadConfigFromDir over generated directory trees, RenameKeys), the security
// whitelists and policies (NewWhitelist, Accept, ToTOML, CheckAllowed*), the
// build cache busters, the dev server matchers, DecodeNamespace hashes, and Go
// regexp (RE2) matching of the patterns these use.
//
//	go run ./tools/go-oracle/nh-config/misc [-root .] [-out crates/nh-config/tests/fixtures/misc]
//
// Directory trees are written to a temporary directory; paths in the results
// have it replaced by "$ROOT".
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"log"
	"math"
	"net/http"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
	"strings"

	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/neohugo"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/security"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-config/cval"
	"github.com/pbnjay/memory"
	"github.com/spf13/afero"
)

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-config/tests/fixtures/misc", "output directory")
	flag.Parse()

	var cases []map[string]any
	add := func(c map[string]any) { cases = append(cases, c) }

	envCases(add)
	versionCases(add)
	hugoInfoCases(add)
	execEnvironCases(add)
	loaderCases(*root, add)
	securityCases(add)
	cacheBusterCases(add)
	serverCases(add)
	namespaceCases(add)
	regexpCases(add)

	header := map[string]any{"oracle": "nh-config/misc", "goarch": runtime.GOARCH}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "misc.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "misc: %d cases\n", len(cases))
}

func errStr(err error) any {
	if err == nil {
		return nil
	}
	return goval.Str(err.Error())
}

func strs(ss []string) []any {
	out := []any{}
	for _, s := range ss {
		out = append(out, goval.Str(s))
	}
	return out
}

// Env.

func envCases(add func(map[string]any)) {
	for _, v := range []string{"", "0", "-1", "1", "4", "abc", "8x", " 3", "+2", "99999999999999999999"} {
		if v == "" {
			must(os.Unsetenv("HUGO_NUMWORKERMULTIPLIER"))
		} else {
			must(os.Setenv("HUGO_NUMWORKERMULTIPLIER", v))
		}
		add(map[string]any{"op": "GetNumWorkerMultiplier", "env": v, "cpu": runtime.NumCPU(), "r": config.GetNumWorkerMultiplier()})
	}
	must(os.Unsetenv("HUGO_NUMWORKERMULTIPLIER"))
	for _, v := range []string{"", "1", "0.5", "2.5", "abc", "-1", "0", "1e3", "3.4028235e38", "1e-9", "NaN", "Inf"} {
		if v == "" {
			must(os.Unsetenv("HUGO_MEMORYLIMIT"))
		} else {
			must(os.Setenv("HUGO_MEMORYLIMIT", v))
		}
		add(map[string]any{"op": "GetMemoryLimit", "env": v, "total": memory.TotalMemory(), "r": config.GetMemoryLimit()})
	}
	must(os.Unsetenv("HUGO_MEMORYLIMIT"))
	for _, s := range []struct {
		old []string
		kv  []string
	}{
		{nil, []string{"A", "1"}},
		{[]string{"A=0", "B=2"}, []string{"A", "1", "C", "3"}},
		{[]string{"AB=0", "A=2"}, []string{"A", "x"}},
		{[]string{"A=0", "A=1"}, []string{"A", "y"}},
		{[]string{"NOEQ", "A"}, []string{"A", "z", "NOEQ", ""}},
		{[]string{"K=V=W"}, []string{"K", "=", "", "empty"}},
	} {
		vars := append([]string(nil), s.old...)
		config.SetEnvVars(&vars, s.kv...)
		add(map[string]any{"op": "SetEnvVars", "old": strs(s.old), "kv": strs(s.kv), "r": strs(vars)})
	}
	for _, v := range []string{"", "A", "A=", "A=B", "A=B=C", "=B", "=", "ÄÖ=ü"} {
		k, val := config.SplitEnvVar(v)
		add(map[string]any{"op": "SplitEnvVar", "v": goval.Str(v), "r": strs([]string{k, val})})
	}
}

// Versions.

func versionCases(add func(map[string]any)) {
	add(map[string]any{"op": "CurrentVersion", "string": neohugo.CurrentVersion.String(), "version": string(neohugo.CurrentVersion.Version())})
	for _, v := range []neohugo.Version{
		{Major: 0, Minor: 149}, {Major: 0, Minor: 53}, {Major: 0, Minor: 54}, {Major: 0, Minor: 53, PatchLevel: 1},
		{Major: 1, Minor: 2, PatchLevel: 3, Suffix: "-test"}, {Major: 0, Minor: 20, Suffix: "-DEV"}, {Major: -1, Minor: -2},
	} {
		add(map[string]any{
			"op": "Version", "v": []any{v.Major, v.Minor, v.PatchLevel, v.Suffix}, "string": v.String(),
			"next": v.Next().String(), "prev": v.Prev().String(), "release": v.ReleaseVersion().String(),
			"nextPatch": v.NextPatchLevel(3).String(),
		})
	}
	for _, s := range []string{"0.149.0-DEV", "0.149.0", "0.149", "0.53", "1.2.3-test", "v0.1", "0.1.2.3", "", "abc", "1..2", "0.149.0-DEV-DEV", "0.149-test-DEV", "10.20.30", "-1.-2"} {
		v := neohugo.MustParseVersion(s)
		add(map[string]any{"op": "ParseVersion", "s": s, "r": []any{v.Major, v.Minor, v.PatchLevel, v.Suffix}, "string": v.String()})
	}
	var inputs []any
	inputs = append(inputs, 0.149, 0.14, 0.15, 0.148, 0.1, 1.0, 0.0, -0.5, 149.0, float32(0.149), 0, 1, int32(0), int64(1), int8(0), uint(0),
		"0.149.0", "0.149.0-DEV", "0.149.0-test", "0.149", "0.150", "0.148.9", "1.0", "v0.149", "", nil, true, math.NaN(), math.Inf(1))
	for _, in := range inputs {
		add(map[string]any{"op": "CompareVersion", "in": goval.Encode(in), "r": neohugo.CompareVersion(in), "eq": neohugo.CurrentVersion.Version().Eq(in)})
	}
	for _, s := range []string{"go1.27.1", "go1.27", "go1.9", "go1.21rc2", "devel go1.28", "go1", "go1.x", "gox.1", "", "go-1.-2", "go1.27.1 X:nocoverageredesign"} {
		add(map[string]any{"op": "goMinorVersion", "s": s, "r": goMinorVersion(s)})
	}
	for _, s := range []string{"0.55.0", "0.149.0", "0.146.0", "0.147.0", "0.142.0", "0.135.0", "0.134.0", "0.127.0", "v0.124.0", "1.0.0", ""} {
		add(map[string]any{"op": "deprecationLevel", "s": s, "r": deprecationLevel(s)})
	}
}

// goMinorVersion is neohugo's unexported goMinorVersion (copied: the oracle cannot call it).
func goMinorVersion(version string) int {
	if strings.HasPrefix(version, "devel") {
		return 9999 // magic
	}
	var major, minor int
	var trailing string
	n, err := fmt.Sscanf(version, "go%d.%d%s", &major, &minor, &trailing)
	if n == 2 && err == io.EOF {
		err = nil
	}
	if err != nil {
		return 0
	}
	return minor
}

// deprecationLevel is neohugo's deprecationLogLevelFromVersion (copied, as a string).
func deprecationLevel(ver string) string {
	from := neohugo.MustParseVersion(ver)
	to := neohugo.CurrentVersion
	minorDiff := to.Minor - from.Minor
	switch {
	case minorDiff >= 15:
		return "error"
	case minorDiff >= 3:
		return "warn"
	default:
		return "info"
	}
}

// HugoInfo.

type infoConf struct {
	environment  string
	running      bool
	workingDir   string
	multihost    bool
	multilingual bool
}

func (c infoConf) Environment() string  { return c.environment }
func (c infoConf) Running() bool        { return c.running }
func (c infoConf) WorkingDir() string   { return c.workingDir }
func (c infoConf) IsMultihost() bool    { return c.multihost }
func (c infoConf) IsMultilingual() bool { return c.multilingual }

func hugoInfoCases(add func(map[string]any)) {
	for _, conf := range []infoConf{
		{environment: "production", workingDir: "/work/seeksnack", multilingual: true},
		{environment: "development", running: true, workingDir: "/mywork", multihost: true},
		{environment: "staging"},
		{environment: "Production"},
	} {
		info := neohugo.NewInfo(conf, nil)
		add(map[string]any{
			"op": "HugoInfo", "conf": []any{conf.environment, conf.running, conf.workingDir, conf.multihost, conf.multilingual},
			"Environment": info.Environment, "Generator": string(info.Generator()), "Version": string(info.Version()),
			"IsDevelopment": info.IsDevelopment(), "IsProduction": info.IsProduction(), "IsServer": info.IsServer(),
			"WorkingDir": info.WorkingDir(), "IsMultihost": info.IsMultihost(), "IsMultilingual": info.IsMultilingual(),
			"Deps": len(info.Deps()),
		})
	}
}

// GetExecEnviron.

type execConf struct {
	config.AllProvider
	env        string
	publishDir string
}

func (c execConf) Environment() string { return c.env }
func (c execConf) BaseConfig() config.BaseConfig {
	return config.BaseConfig{PublishDir: c.publishDir}
}

func execEnvironCases(add func(map[string]any)) {
	for _, s := range []struct {
		workDir, env, publishDir, nodePath string
		files                              []string
	}{
		{"/work/seeksnack", "production", "public", "", []string{"package.json", "postcss.config.js"}},
		{"/work/seeksnack", "production", "public", "/usr/lib/node_modules", nil},
		{"/work/site", "development", "/abs/public", "", []string{"tailwind.config.js", "babel.config.JSON", "ümlaut.file.x"}},
		{"relative/dir", "staging", "../out", "a:b", []string{"x"}},
		{"", "", "", "", nil},
	} {
		if s.nodePath == "" {
			must(os.Unsetenv("NODE_PATH"))
		} else {
			must(os.Setenv("NODE_PATH", s.nodePath))
		}
		mem := afero.NewMemMapFs()
		for _, f := range s.files {
			if err := afero.WriteFile(mem, filepath.Join("_jsconfig", f), []byte("x"), 0o644); err != nil {
				log.Fatal(err)
			}
		}
		var fs afero.Fs
		if s.files != nil {
			fs = hugofs.NewBaseFileDecorator(mem)
		}
		env := neohugo.GetExecEnviron(s.workDir, execConf{env: s.env, publishDir: s.publishDir}, fs)
		// The files the Go function saw (name, Meta().Filename), in its order.
		var seen []any
		if fs != nil {
			d, err := fs.Open("_jsconfig")
			if err == nil {
				fis, _ := d.(interface {
					ReadDir(int) ([]os.DirEntry, error)
				}).ReadDir(-1)
				for _, fi := range fis {
					seen = append(seen, []any{fi.Name(), fi.(hugofs.FileMetaInfo).Meta().Filename})
				}
			}
		}
		add(map[string]any{
			"op": "GetExecEnviron", "workDir": s.workDir, "env": s.env, "publishDir": s.publishDir, "nodePath": s.nodePath,
			"files": seen, "r": strs(env),
		})
	}
	must(os.Unsetenv("NODE_PATH"))
}

// Config loader.

func loaderCases(root string, add func(map[string]any)) {
	docs, err := os.ReadFile(filepath.Join(root, "docs/hugo.toml"))
	if err != nil {
		log.Fatal(err)
	}
	for _, s := range []struct{ content, typ string }{
		{string(docs), "toml"},
		{"title = \"T\"\n[menu]\n[[menu.main]]\nname = \"a\"\n[languages.en.menu]\n[[languages.en.menu.main]]\nname = \"b\"\n", "toml"},
		{"Title: T\nParams:\n  A: 1\n  b: [1, 2]\nmenu:\n  main:\n    - name: x\n", "yaml"},
		{`{"Title": "T", "params": {"X": 1.5, "y": null}, "languages": {"en": {"menu": {"m": []}}}}`, "json"},
		{"title = ", "toml"},
		{"a: b: c", "yaml"},
		{"{", "json"},
		{"", "toml"},
		{"x = 1", "bogus"},
	} {
		c := map[string]any{"op": "FromConfigString", "content": goval.Str(s.content), "type": s.typ}
		cfg, err := config.FromConfigString(s.content, s.typ)
		if err != nil {
			c["err"] = goval.Str(err.Error())
		} else {
			c["r"] = goval.Encode(cfg.Get(""))
		}
		add(c)
	}
	for _, f := range []string{"hugo.toml", "config.YAML", "a/b/params.json", "x.txt", "noext", ".toml", "a.yml", "a.Json", "a.toml.bak"} {
		add(map[string]any{"op": "IsValidConfigFilename", "s": f, "r": config.IsValidConfigFilename(f)})
	}
	for i, tree := range configTrees() {
		dir, err := os.MkdirTemp("", "nhconfig")
		if err != nil {
			log.Fatal(err)
		}
		var files []any
		for _, f := range tree.files {
			p := filepath.Join(dir, f[0])
			if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
				log.Fatal(err)
			}
			if f[1] == "<dir>" {
				if err := os.MkdirAll(p, 0o755); err != nil {
					log.Fatal(err)
				}
			} else if err := os.WriteFile(p, []byte(f[1]), 0o644); err != nil {
				log.Fatal(err)
			}
			files = append(files, []any{f[0], goval.Str(f[1])})
		}
		cfg, dirnames, err := config.LoadConfigFromDir(afero.NewOsFs(), filepath.Join(dir, "config"), tree.env)
		repl := func(s string) string { return strings.ReplaceAll(s, dir, "$ROOT") }
		var dn []string
		for _, d := range dirnames {
			dn = append(dn, repl(d))
		}
		c := map[string]any{"op": "LoadConfigFromDir", "name": fmt.Sprintf("tree#%d", i), "files": files, "env": tree.env, "dirnames": strs(dn)}
		if err != nil {
			c["err"] = goval.Str(repl(err.Error()))
		}
		if cfg != nil {
			c["r"] = goval.Encode(cfg.Get(""))
		}
		add(c)
		must(os.RemoveAll(dir))
	}
}

type tree struct {
	env   string
	files [][2]string
}

func configTrees() []tree {
	full := [][2]string{
		{"config/_default/hugo.toml", "title = \"Default\"\nbaseURL = \"https://example.org/\"\n[params]\nA = 1\n"},
		{"config/_default/params.toml", "B = \"b\"\n[Nested]\nX = true\n"},
		{"config/_default/menus.en.toml", "[[main]]\nname = \"Home\"\nweight = 1\n"},
		{"config/_default/menu.th.yaml", "main:\n  - name: หน้าแรก\n"},
		{"config/_default/params.th.yaml", "greeting: สวัสดี\n"},
		{"config/_default/languages.toml", "[en]\nweight = 1\n[th]\nweight = 2\nlanguageName = \"ไทย\"\n"},
		{"config/_default/config.json", `{"summaryLength": 30, "Params": {"C": [1, 2]}}`},
		{"config/_default/notes.txt", "ignored"},
		{"config/_default/sub/deep.toml", "Deep = 1\n"},
		{"config/_default/markup.yml", "goldmark:\n  renderer:\n    unsafe: true\n"},
		{"config/production/hugo.toml", "title = \"Prod\"\n[params]\nA = 2\n"},
		{"config/production/params.toml", "B = \"prod\"\n"},
		{"config/development/hugo.toml", "title = \"Dev\"\n"},
	}
	return []tree{
		{"production", full},
		{"development", full},
		{"staging", full},
		{"production", [][2]string{{"config/production/hugo.toml", "title = \"OnlyEnv\"\n"}}},
		{"production", [][2]string{{"config/other/hugo.toml", "title = \"x\"\n"}}},
		{"production", [][2]string{{"config/_default/hugo.toml", "title = \n"}}},
		{"production", [][2]string{{"config/_default/params.toml", "a = 1\n"}, {"config/_default/zz.toml", "[x\n"}}},
		{"production", [][2]string{{"config/_default/empty", "<dir>"}, {"config/_default/Params.EN.toml", "Q = 1\n"}, {"config/_default/HUGO.toml", "T = 1\n"}}},
		{"production", [][2]string{{"config/_default/menus.toml", "[[main]]\nname = \"m\"\n"}, {"config/_default/menu.en.toml", "[[main]]\nname = \"e\"\n"}}},
	}
}

// Security.

func securityCases(add func(map[string]any)) {
	names := []string{"", "a", "npx", "Npx", "postcss", "sass", "dart-sass", "dart-sass-embedded", "sass-embedded", "go", "git", "gofmt",
		"PATH", "path", "HOME", "XDG_CONFIG_HOME", "GOROOT", "GO", "HTTPS_PROXY", "http_proxy", "no_proxy", "SSH_AUTH_SOCK", "TMP", "TEMP", "TERM", "LANG", "MYSECRET",
		"HUGO_ENV", "CI", "CIX", "GET", "get", "POST", "DELETE", "https://example.org", "ftp://x", "K", "tailwindcss", "babel"}
	for _, patterns := range [][]string{
		nil, {""}, {"  ", " "}, {"none"}, {"none", "a"}, {"a", "none"}, {"^a$"}, {" ^a$ ", "b"}, {"^foo.*", "^bar.*"},
		{"(?i)get|post"}, {"^\\w+$"}, {"^[\\w.-]+$"}, {"\\bgo\\b"}, {"^\\s*$"}, {"(?i)k"}, {"+Inf"}, {"*.js"}, {"(abc"}, {"abc)"},
		{"[abc"}, {"\\q"}, {"a\\"}, {"[z-a]"}, {"a**"}, {"x{2,1}"}, {"(?P<n>"}, {"ok", "(bad"},
	} {
		c := map[string]any{"op": "Whitelist", "patterns": strs(patterns)}
		w, err := security.NewWhitelist(patterns...)
		if err != nil {
			c["err"] = goval.Str(err.Error())
		} else {
			var acc []any
			for _, n := range names {
				acc = append(acc, w.Accept(n))
			}
			c["accept"] = acc
			c["string"] = w.String()
			b, err := json.Marshal(w)
			if err != nil {
				log.Fatal(err)
			}
			c["json"] = string(b)
		}
		add(c)
	}
	add(map[string]any{"op": "Names", "names": strs(names)})
	for _, in := range []map[string]any{
		nil,
		{"exec": map[string]any{"allow": []any{"^a$", "b"}, "osEnv": "PATH"}, "enableInlineShortcodes": true},
		{"exec": map[string]any{"allow": "none"}, "funcs": map[string]any{"getenv": []any{}}, "http": map[string]any{"urls": []any{"it's", "new\nline", "tab\t", "q\"uote", "back\\slash", "ctl\x01"}, "mediaTypes": []any{"application/json"}}},
	} {
		cfg := config.New()
		if in != nil {
			cfg.Set("security", in)
		}
		sc, err := security.DecodeConfig(cfg)
		c := map[string]any{"op": "Security", "in": goval.Encode(in), "toml": sc.ToTOML(), "err": errStr(err)}
		var checks []any
		for _, n := range names {
			checks = append(checks, []any{errStr(sc.CheckAllowedExec(n)), errStr(sc.CheckAllowedGetEnv(n)), errStr(sc.CheckAllowedHTTPURL(n)), errStr(sc.CheckAllowedHTTPMethod(n))})
		}
		c["checks"] = checks
		add(c)
	}
}

// Cache busters.

func cacheBusterCases(add func(map[string]any)) {
	paths := []string{"postcss.config.js", "assets/tailwind.config.js", "tailwind.config.jsx", "config.js", "main.css", "a/b/styles.scss",
		"foo.bar", "js/main.js", "x.ts", "data/a.json", "", "POSTCSS.CONFIG.JS"}
	keys := []string{"css/main.css", "styles", "scss/a", "js/x", "main.js", "", "_gen/images/x.png", "foo", "bar.baz", "CSS"}
	for _, bc := range []any{
		nil,
		map[string]any{"cacheBusters": []any{map[string]any{"source": "assets/.*\\.(js|ts|jsx|tsx)", "target": "(js|scripts|javascript)"}}},
		map[string]any{"cacheBusters": []any{map[string]any{"source": "(foo)\\.(bar)", "target": "$1$2"}, map[string]any{"source": ".*", "target": "$1"}}},
		map[string]any{"cacheBusters": []any{map[string]any{"source": "data/(.*)\\.json", "target": "$1"}, map[string]any{"source": "(", "target": "x"}}},
		map[string]any{"cacheBusters": []any{map[string]any{"source": "(a)(b)?(c)", "target": "[$1$2$3"}}},
	} {
		cfg := config.New()
		if bc != nil {
			cfg.Set("build", bc)
		}
		b := config.DecodeBuildConfig(cfg)
		logger := loggers.NewDefault()
		c := map[string]any{"op": "CacheBuster", "in": goval.Encode(bc), "build": cval.Dump(b)}
		if err := b.CompileConfig(logger); err != nil {
			c["err"] = goval.Str(err.Error())
			add(c)
			continue
		}
		var res []any
		for _, p := range paths {
			m, err := b.MatchCacheBuster(logger, p)
			if err != nil {
				res = append(res, goval.Str(err.Error()))
				continue
			}
			if m == nil {
				res = append(res, nil)
				continue
			}
			var r []any
			for _, k := range keys {
				r = append(r, m(k))
			}
			res = append(res, r)
		}
		c["paths"] = strs(paths)
		c["keys"] = strs(keys)
		c["r"] = res
		for _, e := range []error{nil, fmt.Errorf("x")} {
			c["useCache"] = append(anySlice(c["useCache"]), b.UseResourceCache(e))
		}
		add(c)
	}
}

func anySlice(v any) []any {
	if v == nil {
		return nil
	}
	return v.([]any)
}

// Server.

func serverCases(add func(map[string]any)) {
	patterns := []string{"/", "/index.html", "/a/b", "/a/b/index.html", "/c/x/y", "/docs/", "/404.html", "/sw.js", "/img/x.png", "", "/C/x"}
	headers := []http.Header{nil, {"Accept": {"text/html"}}, {"Accept": {"application/json"}, "X-Lang": {"th"}}}
	for _, in := range []any{
		nil,
		map[string]any{
			"headers": []any{
				map[string]any{"for": "/**", "values": map[string]any{"X-Frame-Options": "DENY", "N": 1, "B": true}},
				map[string]any{"for": "/*.js", "values": map[string]any{"Cache-Control": "max-age=1", "A": "a"}},
				map[string]any{"for": "/img/**", "values": map[string]any{"X-Frame-Options": "SAMEORIGIN"}},
			},
			"redirects": []any{
				map[string]any{"from": "/a/**", "to": "/b/index.html", "status": 301},
				map[string]any{"fromRe": "^/c/(.*)/(.*)$", "to": "/d/$1/$2/$1", "force": true},
				map[string]any{"from": "/docs/", "fromHeaders": map[string]any{"Accept": "text/html"}, "to": "/html/", "status": 200},
				map[string]any{"fromRe": "^/C/(.*)$", "fromHeaders": map[string]any{"X-Lang": "t*"}, "to": "/th/$1"},
				map[string]any{"from": "/**", "to": "/404.html", "status": 404},
			},
		},
		map[string]any{"redirects": []any{map[string]any{"to": "/x"}}},
		map[string]any{"redirects": []any{map[string]any{"from": "[", "to": "/x"}}},
		map[string]any{"redirects": []any{map[string]any{"fromRe": "(", "to": "/x"}}},
		map[string]any{"headers": []any{map[string]any{"for": "{a", "values": map[string]any{}}}},
	} {
		cfg := config.New()
		if in != nil {
			cfg.Set("server", in)
		}
		s, err := config.DecodeServer(cfg)
		c := map[string]any{"op": "Server", "in": goval.Encode(in), "server": cval.Dump(s), "err": errStr(err)}
		if err := s.CompileConfig(loggers.NewDefault()); err != nil {
			c["compileErr"] = goval.Str(err.Error())
			add(c)
			continue
		}
		var mh, mr []any
		for _, p := range patterns {
			var kv []any
			for _, h := range s.MatchHeaders(p) {
				kv = append(kv, []any{h.Key, h.Value})
			}
			mh = append(mh, kv)
			var rs []any
			for _, h := range headers {
				rs = append(rs, cval.Dump(s.MatchRedirect(p, h)))
			}
			mr = append(mr, rs)
		}
		c["patterns"] = strs(patterns)
		c["matchHeaders"] = mh
		c["matchRedirect"] = mr
		add(c)
	}
}

// DecodeNamespace.

func namespaceCases(add func(map[string]any)) {
	for _, in := range []any{
		map[string]any{}, map[string]any(nil), maps.Params{}, maps.Params{"_merge": maps.ParamsMergeStrategyNone},
		map[string]any{"a": 1}, maps.Params{"a": 1}, map[string]any{"a": int64(1)}, map[string]any{"a": "1"},
		map[string]any{"quality": 75, "resamplefilter": "box"}, []any{"a"}, "s", 42, nil,
	} {
		c := map[string]any{"op": "DecodeNamespace", "in": goval.Encode(in)}
		func() {
			defer func() {
				if r := recover(); r != nil {
					c["panic"] = fmt.Sprint(r)
				}
			}()
			ns, err := config.DecodeNamespace[map[string]any](in, func(v any) (any, any, error) { return nil, nil, nil })
			if err != nil {
				c["err"] = goval.Str(err.Error())
			} else {
				c["hash"] = ns.SourceHash
				b, err := ns.MarshalJSON()
				c["json"] = string(b)
				c["jsonErr"] = errStr(err)
			}
		}()
		c["hashDirect"] = hashing.HashStringHex(in)
		add(c)
	}
}

// Go regexp (RE2) matching.

func regexpCases(add func(map[string]any)) {
	patterns := []string{
		"^(dart-)?sass(-embedded)?$", "^go$", "^npx$", "(?i)GET|POST", ".*", "^HUGO_", "^CI$",
		`(?i)^((HTTPS?|NO)_PROXY|PATH(EXT)?|APPDATA|TE?MP|TERM|GO\w+|(XDG_CONFIG_)?HOME|USERPROFILE|SSH_AUTH_SOCK|DISPLAY|LANG|SYSTEMDRIVE)$`,
		`(postcss|tailwind)\.config\.js`, "(css|styles|scss|sass)", `\w+`, `^\w+$`, `^\s*$`, `\d{2,3}`, `\bfoo\b`, `\Bo\B`, `(?i)straße`, `[\w.-]+`,
		`[^\d]+`, `\S+@\S+`, `(?i)k`, `a|b|`, `x*`, `(a)(b)?(c)`, `(?s)a.b`, `(?m)^a$`, `[[:alpha:]]+`, `\pL+`, `\p{Greek}+`, `[\d\s]+`, `\W+`,
		`\D\S`, `(?U)a+`, `[\W]`, `[^\w\s]`, `é`, `(?i)É`, `\x41`, `\101`, `[\]a]`, `[a-]`, `^$`, `\z`, `\Aab`, `(?:ab)+`, `a{0}`, `(?i)[k]`,
		`\Q.*\E`, `\C`, `[[:^alpha:]]`, `\p{^L}`, `(?P<x>a)`, `(?<y>b)`,
	}
	inputs := []string{"", "a", "ab", "abc", "aXb", "a\nb", "A_1", "ß", "é", "É", "K", "K", "ſ", "foo bar", "foo", "xfoox", "  ", "\t\n\v\f\r",
		"123", "١٢٣", "x@y", "straSSe", "STRASSE", "Straße", " ", "PATH", "gopath", "GOPATH", "dart-sass", "postcss.config.js", "main.css",
		"aaa", "b", "c", "]", "-", "Ωμέγα", "日本語", ".*", "A", "HUGO_X", "CI", "https://x"}
	for _, p := range patterns {
		re, err := regexp.Compile(p)
		c := map[string]any{"op": "Regexp", "pattern": goval.Str(p)}
		if err != nil {
			c["err"] = goval.Str(err.Error())
			add(c)
			continue
		}
		var res []any
		for _, in := range inputs {
			var sub any
			if m := re.FindStringSubmatch(in); m != nil {
				sub = strs(m)
			}
			res = append(res, []any{re.MatchString(in), sub})
		}
		c["r"] = res
		add(c)
	}
	add(map[string]any{"op": "RegexpInputs", "inputs": strs(inputs)})
}

// must stops the oracle on an unexpected error.
func must(err error) {
	if err != nil {
		log.Fatal(err)
	}
}
