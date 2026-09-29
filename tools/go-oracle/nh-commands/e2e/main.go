// Command e2e is the Go oracle of the I01 end-to-end regression test of
// crates/nh-commands (tests/e2e.rs): complete sites built by the real Go
// command line with the golden flags (`--minify --clock 2026-09-27T12:00:00Z
// -d <out>`, one render worker), in a child process per site like main.go.
//
//	go run ./tools/go-oracle/nh-commands/e2e [-root .] [-out crates/nh-commands/tests/fixtures/e2e]
//
// Sites: mini.txtar (this directory; GetRemote served from a file cache entry
// of the golden build), tools/rust-port/i01/testsite.txtar on top of
// hugolib/testsite, and tools/rust-port/i01/errors.txtar (a failing build).
// The fixture records the site files, stdout, stderr (the version line, the
// timings and temp file names masked, "$ROOT" for the temporary directory),
// the exit code, every published file and hugo_stats.json.
//
// The sites avoid output that depends on the platform's floating point (image
// encoding, LibSass, decimal numbers in minified CSS) and external tools
// (esbuild, PostCSS), so a native build of this oracle on any architecture
// records the same bytes; tools/rust-port/i01/compare.sh covers the rest
// against the arm64 Go build.
package main

import (
	"bytes"
	"context"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"io/fs"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"time"

	"github.com/neohugo/neohugo/commands"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"golang.org/x/tools/txtar"
)

const placeholder = "$ROOT"

// The golden getresource entry (tools/rust-port/testdata/hugo_cache) and the
// file cache key of the URL mini.txtar requests for it.
const (
	goldenEntry  = "10426788187073209306"
	goldenKey    = "17211370855584179129"
	goldenCache  = "tools/rust-port/testdata/hugo_cache/seeksnack/filecache/getresource/"
	miniCacheDir = "_cache/site/filecache/getresource/"
)

type caseSpec struct {
	Name  string            `json:"name"`
	Files map[string]string `json:"files"`
	Args  []string          `json:"args"`
	Env   map[string]string `json:"env,omitempty"`
}

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "crates/nh-commands/tests/fixtures/e2e", "fixture directory (relative to -root)")
	child := flag.String("child", "", "internal: run the command line (JSON args)")
	flag.Parse()

	if *child != "" {
		var args []string
		if err := json.Unmarshal([]byte(*child), &args); err != nil {
			log.Fatal(err)
		}
		log.SetFlags(0)
		if err := commands.Execute(args); err != nil {
			log.Fatalf("Error: %s", err)
		}
		return
	}

	cs, err := cases(*root)
	if err != nil {
		log.Fatal(err)
	}
	var rows []map[string]any
	for _, c := range cs {
		res, err := run(c)
		if err != nil {
			log.Fatalf("%s: %v", c.Name, err)
		}
		again, err := run(c)
		if err != nil {
			log.Fatalf("%s: %v", c.Name, err)
		}
		a, _ := json.Marshal(res)
		b, _ := json.Marshal(again)
		if !bytes.Equal(a, b) {
			log.Fatalf("%s: nondeterministic result", c.Name)
		}
		rows = append(rows, map[string]any{"name": c.Name, "files": c.Files, "args": c.Args, "env": c.Env, "result": res})
	}
	dir := *out
	if !filepath.IsAbs(dir) {
		dir = filepath.Join(*root, dir)
	}
	if err := os.MkdirAll(dir, 0o755); err != nil {
		log.Fatal(err)
	}
	header := map[string]any{"env": baseEnv()}
	if err := goval.WriteCasesGz(filepath.Join(dir, "e2e.json.gz"), header, rows); err != nil {
		log.Fatal(err)
	}
	fmt.Printf("e2e: %d sites\n", len(rows))
}

func baseEnv() map[string]string {
	return map[string]string{
		"HOME":                     placeholder + "/home",
		"TMPDIR":                   placeholder + "/tmp",
		"PATH":                     placeholder + "/bin",
		"HUGO_NUMWORKERMULTIPLIER": "1",
	}
}

func readTxtar(path string) (map[string]string, error) {
	a, err := txtar.ParseFile(path)
	if err != nil {
		return nil, err
	}
	m := map[string]string{}
	for _, f := range a.Files {
		m[f.Name] = string(f.Data)
	}
	return m, nil
}

func cases(root string) ([]caseSpec, error) {
	args := []string{"--minify", "--clock", "2026-09-27T12:00:00Z", "-d", "$ROOT/out"}

	mini, err := readTxtar(filepath.Join(root, "tools/go-oracle/nh-commands/e2e/mini.txtar"))
	if err != nil {
		return nil, err
	}
	entry, err := os.ReadFile(filepath.Join(root, goldenCache+goldenEntry))
	if err != nil {
		return nil, err
	}
	mini[miniCacheDir+goldenKey] = string(entry)

	testsite, err := readTxtar(filepath.Join(root, "tools/rust-port/i01/testsite.txtar"))
	if err != nil {
		return nil, err
	}
	err = filepath.WalkDir(filepath.Join(root, "hugolib/testsite"), func(p string, d fs.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return err
		}
		rel, _ := filepath.Rel(filepath.Join(root, "hugolib/testsite"), p)
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		testsite[filepath.ToSlash(rel)] = string(b)
		return nil
	})
	if err != nil {
		return nil, err
	}

	errs, err := readTxtar(filepath.Join(root, "tools/rust-port/i01/errors.txtar"))
	if err != nil {
		return nil, err
	}

	return []caseSpec{
		{Name: "mini", Files: mini, Args: args, Env: map[string]string{"HUGO_CACHEDIR": "$ROOT/site/_cache"}},
		{Name: "testsite", Files: testsite, Args: args},
		{Name: "errors", Files: errs, Args: args},
	}, nil
}

var (
	versionRe = regexp.MustCompile(`(neohugo v\d+\.\d+\.\d+(?:-DEV)?)(?:-[0-9a-f]{7,40})? (\S+) BuildDate=\S+`)
	timingRe  = regexp.MustCompile(`(?m)^(Total|Built) in \d+ ms$`)
	tempRe    = regexp.MustCompile(`hugo-transform-error\d+`)
)

func norm(s, tmp string) string {
	s = strings.ReplaceAll(s, tmp, placeholder)
	s = versionRe.ReplaceAllString(s, "$1 $$OS/$$ARCH BuildDate=$$DATE")
	s = tempRe.ReplaceAllString(s, "hugo-transform-errorN")
	return timingRe.ReplaceAllString(s, "$1 in N ms")
}

func run(c caseSpec) (map[string]any, error) {
	tmp, err := os.MkdirTemp("", "nh-commands-e2e-")
	if err != nil {
		return nil, err
	}
	defer func() { _ = os.RemoveAll(tmp) }()
	tmp, err = filepath.EvalSymlinks(tmp)
	if err != nil {
		return nil, err
	}
	site := filepath.Join(tmp, "site")
	var names []string
	for n := range c.Files {
		names = append(names, n)
	}
	sort.Strings(names)
	for _, n := range names {
		p := filepath.Join(site, filepath.FromSlash(n))
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			return nil, err
		}
		if err := os.WriteFile(p, []byte(c.Files[n]), 0o644); err != nil {
			return nil, err
		}
	}
	for _, d := range []string{"home", "tmp", "bin"} {
		if err := os.MkdirAll(filepath.Join(tmp, d), 0o755); err != nil {
			return nil, err
		}
	}
	var args []string
	for _, a := range c.Args {
		args = append(args, strings.ReplaceAll(a, placeholder, tmp))
	}
	spec, _ := json.Marshal(args)
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
	defer cancel()
	cmd := exec.CommandContext(ctx, os.Args[0], "-child", string(spec))
	cmd.Dir = site
	var env []string
	for k, v := range baseEnv() {
		env = append(env, k+"="+strings.ReplaceAll(v, placeholder, tmp))
	}
	for k, v := range c.Env {
		env = append(env, k+"="+strings.ReplaceAll(v, placeholder, tmp))
	}
	sort.Strings(env)
	cmd.Env = env
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	exit := 0
	if err := cmd.Run(); err != nil {
		ee, ok := err.(*exec.ExitError)
		if !ok {
			return nil, err
		}
		exit = ee.ExitCode()
	}
	tree, err := dump(tmp)
	if err != nil {
		return nil, err
	}
	return map[string]any{
		"stdout": norm(stdout.String(), tmp),
		"stderr": norm(stderr.String(), tmp),
		"exit":   exit,
		"tree":   tree,
	}, nil
}

// dump lists every published file (below out/) and the site's hugo_stats.json,
// with the content in hex.
func dump(root string) (map[string]string, error) {
	out := map[string]string{}
	add := func(rel string) error {
		b, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(rel)))
		if err != nil {
			return err
		}
		out[rel] = hex.EncodeToString(b)
		return nil
	}
	if _, err := os.Stat(filepath.Join(root, "site", "hugo_stats.json")); err == nil {
		if err := add("site/hugo_stats.json"); err != nil {
			return nil, err
		}
	}
	outDir := filepath.Join(root, "out")
	if _, err := os.Stat(outDir); err != nil {
		return out, nil
	}
	err := filepath.WalkDir(outDir, func(path string, d fs.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return err
		}
		rel, _ := filepath.Rel(root, path)
		return add(filepath.ToSlash(rel))
	})
	return out, err
}
