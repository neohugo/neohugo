// Command load is the Go oracle for crates/nh-allconfig (Wave B task T09):
// allconfig.LoadConfig and the project module collection of modules.Client.
//
//	go run ./tools/go-oracle/nh-allconfig/load [-root .] [-out rust/testdata/oracle/allconfig/load]
//	go run ./tools/go-oracle/nh-allconfig/load -print <case> [-lang th] [-zero]
//
// Every case is a site tree (files with their contents) recreated in a
// temporary directory, loaded like the neohugo CLI does (a flags provider with
// workingDir, environment, internal and the changed flags) with an explicit
// environment. The fixture records the tree, the descriptor and, per case,
// either the error text or: every decoded section of Configs.Base and of each
// language config (json.Marshal of *allconfig.Config), the compiled values
// (ConfigCompiled), the namespace SourceHashes, the languages (order, default
// first, IsMultihost), the ConfigLanguage answers, the modules with their
// mounts, and the `neohugo config` / `neohugo config mounts` outputs. Paths
// under the temporary root are written as "$ROOT".
//
// Nondeterminism: Go iterates maps in random order in fromLoadConfigResult and
// CompileConfig. Each case runs several times; cases whose results differ are
// marked "nondet" and not compared.
package main

import (
	"bytes"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/hugolib/segments"
	"github.com/neohugo/neohugo/modules"
	"github.com/neohugo/neohugo/parser"
	"github.com/neohugo/neohugo/parser/metadecoders"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/spf13/afero"
)

const placeholder = "$ROOT"

// baseTempDir is the process temp dir before any case changed TMPDIR.
var baseTempDir = os.TempDir()

// A caseSpec is one LoadConfig input.
type caseSpec struct {
	Name string `json:"name"`
	// Files maps a slash path below the site root to its content; a path
	// ending in "/" is an empty directory.
	Files map[string]string `json:"files"`
	// Site is the directory below the temporary root that holds the site
	// ("" = "site").
	Site string `json:"site,omitempty"`
	// Environment of the descriptor (and of the CLI flags).
	Environment string `json:"environment"`
	// Flags set on the CLI flags provider after workingDir, environment and
	// internal (like the changed CLI flags; "minifyOutput", "internal.clock").
	Flags map[string]any `json:"flags"`
	// Environ is ConfigSourceDescriptor.Environ ("$ROOT" is replaced).
	Environ []string `json:"environ"`
	// ProcEnv is the process environment GetCacheDir reads (HOME,
	// XDG_CACHE_HOME, USER, NETLIFY...).
	ProcEnv map[string]string `json:"procEnv"`
	// Filename is the --config value ("$ROOT" is replaced).
	Filename string `json:"filename,omitempty"`
	// ConfigDir is the --configDir value ("config" like the CLI when empty).
	ConfigDir                string `json:"configDir"`
	IgnoreModuleDoesNotExist bool   `json:"ignoreModuleDoesNotExist,omitempty"`
	// Probes for the per-language answers.
	UglyProbes   []string `json:"-"`
	IgnoreProbes []string `json:"-"`
}

var (
	uglyProbes     = []string{"", "posts", "blog", "Posts", "docs", "news"}
	ignoreProbes   = []string{"content/a.md", "content/draft/b.md", "/abs/c.txt", "x.tmp", "static/.DS_Store", "a/b~", "#x#"}
	titleProbes    = []string{"hello world", "a tale of two cities", "THE END", "the lord of the rings", "über café"}
	segmentProbes  = []segments.SegmentMatcherFields{{Kind: "page", Lang: "en", Path: "/docs/a", Output: "html"}, {Kind: "home", Lang: "th", Path: "/", Output: "rss"}, {Kind: "section", Lang: "en", Path: "/blog", Output: "json"}, {Kind: "term", Lang: "fr", Path: "/tags/x", Output: "html"}, {Lang: "en"}, {Output: "html"}, {Path: "/docs/a/b"}, {}}
	urlProbes      = []string{"https://example.org/a.json", "http://localhost/x", "https://gohugo.io/", "file:///tmp/a"}
	outputKinds    = []string{"home", "page", "section", "taxonomy", "term", "rss", "sitemap", "robotstxt", "404", "sitemapindex", "status404", "taxonomyterm"}
	defaultTimeout = 60 * time.Second
)

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/allconfig/load", "output directory")
	printCase := flag.String("print", "", "print the `neohugo config` JSON of this case")
	printLang := flag.String("lang", "", "with -print: the language")
	printZero := flag.Bool("zero", false, "with -print: include zero values")
	printMounts := flag.Bool("mounts", false, "with -print: print `neohugo config mounts`")
	only := flag.String("only", "", "run only the cases whose name contains this")
	oneCase := flag.String("case", "", "run this case and print its fixture row as JSON (used for process isolation)")
	flag.Parse()

	cases, err := allCases(*root)
	if err != nil {
		log.Fatal(err)
	}

	if *oneCase != "" {
		for _, c := range cases {
			if c.Name != *oneCase {
				continue
			}
			row, err := runCase(c)
			if err != nil {
				log.Fatalf("%s: %v", c.Name, err)
			}
			if err := json.NewEncoder(os.Stdout).Encode(row); err != nil {
				log.Fatal(err)
			}
			return
		}
		log.Fatalf("no case %q", *oneCase)
	}

	if *printCase != "" {
		for _, c := range cases {
			if c.Name != *printCase {
				continue
			}
			if err := printOne(c, *printLang, *printZero, *printMounts); err != nil {
				log.Fatal(err)
			}
			return
		}
		log.Fatalf("no case %q", *printCase)
	}

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}

	groups := map[string][]caseSpec{}
	var groupNames []string
	for _, c := range cases {
		if *only != "" && !strings.Contains(c.Name, *only) {
			continue
		}
		g, _, _ := strings.Cut(c.Name, "/")
		if _, ok := groups[g]; !ok {
			groupNames = append(groupNames, g)
		}
		groups[g] = append(groups[g], c)
	}

	for _, g := range groupNames {
		var rows []map[string]any
		nondet := 0
		for _, c := range groups[g] {
			// Each case runs in its own process: some Go defaults are package-level
			// values that a decode changes (markup_config.Default's AsciidocExt
			// attribute map), which would leak into the next case.
			row, err := runIsolated(*root, c.Name)
			if err != nil {
				log.Fatalf("%s: %v", c.Name, err)
			}
			if row["nondet"] == true {
				nondet++
			}
			rows = append(rows, row)
		}
		path := filepath.Join(*out, g+".json.gz")
		if err := goval.WriteCasesGz(path, map[string]any{"group": g}, rows); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("%s: %d cases (%d nondet)\n", g, len(rows), nondet)
	}
}

// runIsolated runs one case in a child process.
func runIsolated(root, name string) (map[string]any, error) {
	cmd := exec.Command(os.Args[0], "-root", root, "-case", name)
	cmd.Stderr = os.Stderr
	out, err := cmd.Output()
	if err != nil {
		return nil, err
	}
	var row map[string]any
	dec := json.NewDecoder(bytes.NewReader(out))
	dec.UseNumber()
	if err := dec.Decode(&row); err != nil {
		return nil, err
	}
	return row, nil
}

// materialize writes the case tree below root.
func materialize(root string, files map[string]string) error {
	var names []string
	for n := range files {
		names = append(names, n)
	}
	sort.Strings(names)
	for _, n := range names {
		p := filepath.Join(root, filepath.FromSlash(n))
		if strings.HasSuffix(n, "/") {
			if err := os.MkdirAll(p, 0o755); err != nil {
				return err
			}
			continue
		}
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			return err
		}
		content := strings.ReplaceAll(files[n], placeholder, filepath.Dir(root))
		if err := os.WriteFile(p, []byte(content), 0o644); err != nil {
			return err
		}
	}
	return nil
}

type loaded struct {
	tmp     string
	root    string
	configs *allconfig.Configs
	err     error
	log     *bytes.Buffer
}

// load recreates the case in a new temporary directory and runs LoadConfig
// like the CLI (commands.rootCommand.ConfigFromProvider).
func load(c caseSpec) (*loaded, error) {
	// The cases set TMPDIR; create the case root in the original temp dir.
	tmp, err := os.MkdirTemp(baseTempDir, "nh-allconfig-")
	if err != nil {
		return nil, err
	}
	tmp, err = filepath.EvalSymlinks(tmp)
	if err != nil {
		return nil, err
	}
	site := c.Site
	if site == "" {
		site = "site"
	}
	root := filepath.Join(tmp, site)
	if err := os.MkdirAll(root, 0o755); err != nil {
		return nil, err
	}
	// The user cache dir of procEnv.
	if err := os.MkdirAll(filepath.Join(tmp, "xdg"), 0o755); err != nil {
		return nil, err
	}
	if err := materialize(root, c.Files); err != nil {
		return nil, err
	}

	// The process environment read by helpers.GetCacheDir.
	for _, k := range []string{"HOME", "XDG_CACHE_HOME", "USER", "NETLIFY", "PULL_REQUEST", "DEPLOY_PRIME_URL", "TMPDIR"} {
		if v, ok := c.ProcEnv[k]; ok {
			_ = os.Setenv(k, strings.ReplaceAll(v, placeholder, tmp))
		} else {
			_ = os.Unsetenv(k)
		}
	}

	// commands.hugoBuilder.loadConfig + flagsToCfg.
	cfg := config.New()
	cfg.Set("renderToMemory", false)
	cfg.Set("environment", c.Environment)
	cfg.Set("internal", map[string]any{
		"running":        false,
		"watch":          false,
		"verbose":        false,
		"fastRenderMode": false,
	})
	var flagKeys []string
	for k := range c.Flags {
		flagKeys = append(flagKeys, k)
	}
	sort.Strings(flagKeys)
	for _, k := range flagKeys {
		cfg.Set(k, replaceRoot(c.Flags[k], tmp))
	}
	if !cfg.IsSet("workingDir") {
		cfg.Set("workingDir", root)
	}

	var environ []string
	for _, e := range c.Environ {
		environ = append(environ, strings.ReplaceAll(e, placeholder, tmp))
	}
	configDir := c.ConfigDir
	if configDir == "" {
		configDir = "config"
	}

	logBuf := &bytes.Buffer{}
	confs, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Flags:                    cfg,
		Fs:                       afero.NewOsFs(),
		Filename:                 strings.ReplaceAll(c.Filename, placeholder, tmp),
		ConfigDir:                configDir,
		Environment:              c.Environment,
		Environ:                  environ,
		Logger:                   loggers.New(loggers.Options{Level: logg.LevelInfo, StdOut: logBuf, StdErr: logBuf}),
		IgnoreModuleDoesNotExist: c.IgnoreModuleDoesNotExist,
	})
	return &loaded{tmp: tmp, root: root, configs: confs, err: err, log: logBuf}, nil
}

func replaceRoot(v any, tmp string) any {
	switch vv := v.(type) {
	case string:
		return strings.ReplaceAll(vv, placeholder, tmp)
	case []any:
		out := make([]any, len(vv))
		for i, x := range vv {
			out[i] = replaceRoot(x, tmp)
		}
		return out
	case map[string]any:
		out := map[string]any{}
		for k, x := range vv {
			out[k] = replaceRoot(x, tmp)
		}
		return out
	}
	return v
}

func printOne(c caseSpec, lang string, zero, mounts bool) error {
	l, err := load(c)
	if err != nil {
		return err
	}
	defer func() { _ = os.RemoveAll(l.tmp) }()
	if l.err != nil {
		return l.err
	}
	if mounts {
		s, err := mountsDump(l.configs)
		if err != nil {
			return err
		}
		fmt.Print(s)
		return nil
	}
	s, err := configDump(l.configs, lang, zero)
	if err != nil {
		return err
	}
	fmt.Print(s)
	return nil
}

// runs is how often each case is loaded (to find map-order dependent results);
// a case with several results is loaded nondetRuns more times so that every
// variant is found and the fixture regenerates identically.
const (
	runs       = 24
	nondetRuns = 1000
)

func runCase(c caseSpec) (map[string]any, error) {
	variants := map[string]map[string]any{}
	loadN := func(n int) error {
		for i := 0; i < n; i++ {
			r, err := runOnce(c)
			if err != nil {
				return err
			}
			b, err := json.Marshal(r)
			if err != nil {
				return err
			}
			variants[string(b)] = r
		}
		return nil
	}
	if err := loadN(runs); err != nil {
		return nil, err
	}
	if len(variants) > 1 {
		if err := loadN(nondetRuns); err != nil {
			return nil, err
		}
	}
	var keys []string
	for k := range variants {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	row := map[string]any{"case": c, "result": variants[keys[0]]}
	if len(keys) > 1 {
		// Go iterates maps in random order: every result seen is recorded, and the
		// port must produce one of them.
		row["nondet"] = true
		var rest []map[string]any
		for _, k := range keys[1:] {
			rest = append(rest, variants[k])
		}
		row["variants"] = rest
	}
	return row, nil
}

func runOnce(c caseSpec) (map[string]any, error) {
	l, err := load(c)
	if err != nil {
		return nil, err
	}
	defer func() { _ = os.RemoveAll(l.tmp) }()
	r := func(s string) string { return strings.ReplaceAll(s, l.tmp, placeholder) }
	if l.err != nil {
		return map[string]any{"err": r(l.err.Error()), "log": r(l.log.String())}, nil
	}
	d := &dumper{r: r}
	out, err := d.configs(l.configs)
	if err != nil {
		return nil, err
	}
	out["log"] = r(l.log.String())
	return out, nil
}

// configDump is `neohugo config --format json [--lang l] [--printZero]`.
func configDump(confs *allconfig.Configs, lang string, zero bool) (string, error) {
	var conf *allconfig.Config
	if lang != "" {
		var found bool
		conf, found = confs.LanguageConfigMap[lang]
		if !found {
			return "", fmt.Errorf("language %q not found", lang)
		}
	} else {
		conf = confs.LanguageConfigSlice[0]
	}
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetIndent("", "  ")
	enc.SetEscapeHTML(false)
	if err := enc.Encode(parser.ReplacingJSONMarshaller{Value: conf, KeysToLower: true, OmitEmpty: !zero}); err != nil {
		return "", err
	}
	return buf.String(), nil
}

type configModMount struct {
	Source string `json:"source"`
	Target string `json:"target"`
	Lang   string `json:"lang,omitempty"`
}

type configModMounts struct {
	m modules.Module
}

// MarshalJSON is commands.configModMounts.MarshalJSON (not verbose).
func (m *configModMounts) MarshalJSON() ([]byte, error) {
	var mounts []configModMount
	for _, mount := range m.m.Mounts() {
		mounts = append(mounts, configModMount{
			Source: mount.Source,
			Target: mount.Target,
			Lang:   mount.Lang,
		})
	}
	var ownerPath string
	if m.m.Owner() != nil {
		ownerPath = m.m.Owner().Path()
	}
	return json.Marshal(&struct {
		Path    string           `json:"path"`
		Version string           `json:"version"`
		Time    time.Time        `json:"time"`
		Owner   string           `json:"owner"`
		Dir     string           `json:"dir"`
		Mounts  []configModMount `json:"mounts"`
	}{
		Path:    m.m.Path(),
		Version: m.m.Version(),
		Time:    m.m.Time(),
		Owner:   ownerPath,
		Dir:     m.m.Dir(),
		Mounts:  mounts,
	})
}

// mountsDump is `neohugo config mounts`.
func mountsDump(confs *allconfig.Configs) (string, error) {
	var buf bytes.Buffer
	for _, m := range confs.Modules {
		if err := parser.InterfaceToConfig(&configModMounts{m: m}, metadecoders.JSON, &buf); err != nil {
			return "", err
		}
	}
	return buf.String(), nil
}

var _ = defaultTimeout
