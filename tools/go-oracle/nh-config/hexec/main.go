// Command hexec is the Go oracle for common/hexec in crates/nh-config (Wave B
// task T04): the environment an Exec passes to external programs (os.Environ
// filtered by security.exec.osEnv, plus WithEnviron), the binary lookup order
// of Npx (node_modules/.bin, npx --no-install, PATH; PATH before npx for
// tailwindcss) and its cache, the security checks, and the errors of Run
// (binary not found, "not found:" in stderr, other failures).
//
//	go run ./tools/go-oracle/nh-config/hexec [-out crates/nh-config/tests/fixtures/hexec]
//
// Each scenario is a directory tree in a temporary root with executable shell
// scripts (they print their path, arguments, stdin and environment); the
// process environment is set to a controlled list. Results have the root
// replaced by "$ROOT". The Rust test rebuilds the same trees.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/neohugo/neohugo/common/hexec"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config/security"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// Script is the executable the scenarios use: it prints its path, arguments, stdin and
// environment (without PWD, which the shell sets from the working directory).
const Script = "#!/bin/sh\necho \"BIN=$0\"\nfor a in \"$@\"; do echo \"ARG=$a\"; done\necho \"STDIN=$(/bin/cat)\"\n/usr/bin/env -u PWD | /usr/bin/sort\n"

// Scripts by kind.
var Scripts = map[string]string{
	"print":     Script,
	"failnf":    "#!/bin/sh\necho \"sh: 1: something: not found:\" >&2\nexit 127\n",
	"failother": "#!/bin/sh\necho out\necho \"boom\" >&2\necho \"more\" >&2\nexit 3\n",
	"npx":       "#!/bin/sh\nif [ \"$2\" = \"missing-tool\" ]; then echo \"npm ERR! could not determine executable to run\" >&2; exit 1; fi\n" + strings.TrimPrefix(Script, "#!/bin/sh\n"),
	"plain":     "not executable\n",
}

type file struct {
	Path string // relative to the root
	Kind string // a Scripts key, or "dir"
	Mode uint32
}

type run struct {
	Op    string // "Npx", "New", "Delete" (removes Bin, a relative path)
	Bin   string
	Args  []string
	Env   []string
	Stdin string
	// Stderr: also pass a stderr writer.
	Stderr bool
}

type scenario struct {
	Name  string
	Sec   string // "default" or "custom"
	Files []file
	Path  string // PATH, with $ROOT
	Runs  []run
}

var baseEnv = []string{
	"HOME=/home/test", "LANG=C.UTF-8", "TERM=dumb", "GOPATH=/go", "MYSECRET=s3cret", "HTTP_PROXY=http://proxy",
	"no_proxy=localhost", "NODE_ENV=production", "XDG_CONFIG_HOME=/xdg", "TMP=/tmp/x", "USERPROFILE=u", "lower_path=x",
	"GOFLAGS=-mod=mod", "SSH_AUTH_SOCK=/sock", "DISPLAY=:0", "PATHEXT=.EXE",
}

func scenarios() []scenario {
	full := []file{
		{"wd/node_modules/.bin/postcss", "print", 0o755},
		{"wd/node_modules/.bin/babel", "plain", 0o644},
		{"wd/node_modules/.bin/tailwindcss", "print", 0o755},
		{"wd/node_modules/.bin/sass", "dir", 0o755},
		{"pathA/npx", "npx", 0o755},
		{"pathA/postcss", "print", 0o755},
		{"pathA/failnf", "failnf", 0o755},
		{"pathA/failother", "failother", 0o755},
		{"pathB/tailwindcss", "print", 0o755},
		{"pathB/sass", "print", 0o755},
		{"pathB/babel", "print", 0o755},
		{"pathB/nox", "print", 0o644},
	}
	noNodeModules := []file{
		{"wd/.keep", "plain", 0o644},
		{"pathA/npx", "npx", 0o755},
		{"pathB/tailwindcss", "print", 0o755},
		{"pathB/postcss", "print", 0o755},
	}
	pathAB := "$ROOT/pathA:$ROOT/pathB"
	return []scenario{
		{"full", "custom", full, pathAB, []run{
			{Op: "Npx", Bin: "postcss", Args: []string{"--config", "postcss.config.js", "--use", "autoprefixer"}},
			{Op: "Npx", Bin: "postcss", Args: []string{"again"}, Env: []string{"NODE_PATH=/wd/node_modules", "HUGO_ENVIRONMENT=production", "PWD=/wd"}},
			{Op: "Npx", Bin: "postcss", Stdin: "body { color: red }\nline2"},
			{Op: "Npx", Bin: "babel", Args: []string{"x.js"}},
			{Op: "Npx", Bin: "tailwindcss", Args: []string{"-i", "in.css"}},
			{Op: "Npx", Bin: "sass", Args: []string{"a.scss"}},
			{Op: "Npx", Bin: "missing-tool"},
			{Op: "New", Bin: "npx", Args: []string{"--version"}},
			{Op: "New", Bin: "failnf", Stderr: true},
			{Op: "New", Bin: "failother", Args: []string{"a b", "c"}, Stderr: true},
			{Op: "New", Bin: "failother"},
			{Op: "New", Bin: "go", Args: []string{"version"}},
			{Op: "New", Bin: "nox"},
			{Op: "New", Bin: "notallowed"},
			{Op: "Npx", Bin: "notallowed"},
			{Op: "New", Bin: "postcss", Env: []string{"HOME=/override", "EXTRA=1", "EXTRA=2", "NOEQUALS", "PATH=/custom/bin"}},
		}},
		{"full-default-security", "default", full, pathAB, []run{
			{Op: "Npx", Bin: "postcss"},
			{Op: "Npx", Bin: "babel"},
			{Op: "Npx", Bin: "tailwindcss"},
			{Op: "Npx", Bin: "sass"},
			{Op: "New", Bin: "failnf"},
		}},
		{"no-node-modules", "custom", noNodeModules, pathAB, []run{
			{Op: "Npx", Bin: "postcss", Args: []string{"x"}},
			{Op: "Npx", Bin: "tailwindcss", Args: []string{"y"}},
			{Op: "Npx", Bin: "babel"},
			{Op: "Npx", Bin: "missing-tool"},
		}},
		{"no-npx", "custom", []file{{"wd/.keep", "plain", 0o644}, {"pathB/postcss", "print", 0o755}}, "$ROOT/pathB", []run{
			{Op: "Npx", Bin: "postcss"},
			{Op: "Npx", Bin: "tailwindcss"},
			{Op: "Npx", Bin: "babel"},
		}},
		{"empty-path", "custom", full, "", []run{
			{Op: "Npx", Bin: "postcss"},
			{Op: "Npx", Bin: "babel"},
			{Op: "New", Bin: "postcss"},
		}},
		{"path-with-empty-and-missing", "custom", full, "$ROOT/missing::$ROOT/pathB:/nonexistent", []run{
			{Op: "Npx", Bin: "babel"},
			{Op: "Npx", Bin: "tailwindcss"},
			{Op: "New", Bin: "sass"},
		}},
		{"cache", "custom", full, pathAB, []run{
			{Op: "Npx", Bin: "postcss", Args: []string{"first"}},
			{Op: "Delete", Bin: "wd/node_modules/.bin/postcss"},
			{Op: "Npx", Bin: "postcss", Args: []string{"second"}},
			{Op: "Delete", Bin: "pathA/npx"},
			{Op: "Npx", Bin: "babel", Args: []string{"no npx"}},
		}},
	}
}

func main() {
	out := flag.String("out", "crates/nh-config/tests/fixtures/hexec", "output directory")
	flag.Parse()

	custom := security.DefaultConfig
	custom.Exec.Allow = security.MustNewWhitelist("^(npx|postcss|tailwindcss|babel|sass|failnf|failother|missing-tool|go|nox)$")

	var cases []map[string]any
	for _, s := range scenarios() {
		root, err := os.MkdirTemp("", "nhhexec")
		if err != nil {
			log.Fatal(err)
		}
		root, err = filepath.EvalSymlinks(root)
		if err != nil {
			log.Fatal(err)
		}
		for _, f := range s.Files {
			p := filepath.Join(root, f.Path)
			if f.Kind == "dir" {
				if err := os.MkdirAll(p, os.FileMode(f.Mode)); err != nil {
					log.Fatal(err)
				}
				continue
			}
			if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
				log.Fatal(err)
			}
			if err := os.WriteFile(p, []byte(Scripts[f.Kind]), os.FileMode(f.Mode)); err != nil {
				log.Fatal(err)
			}
			if err := os.Chmod(p, os.FileMode(f.Mode)); err != nil {
				log.Fatal(err)
			}
		}
		repl := func(v string) string { return strings.ReplaceAll(v, root, "$ROOT") }
		os.Clearenv()
		env := append([]string{"PATH=" + strings.ReplaceAll(s.Path, "$ROOT", root)}, baseEnv...)
		for _, kv := range env {
			k, v, _ := strings.Cut(kv, "=")
			must(os.Setenv(k, v))
		}
		sc := security.DefaultConfig
		if s.Sec == "custom" {
			sc = custom
		}
		ex := hexec.New(sc, filepath.Join(root, "wd"), loggers.NewDefault())
		var results []any
		for _, r := range s.Runs {
			res := map[string]any{}
			if r.Op == "Delete" {
				if err := os.Remove(filepath.Join(root, r.Bin)); err != nil {
					log.Fatal(err)
				}
				results = append(results, res)
				continue
			}
			var stdout, stderr bytes.Buffer
			args := []any{}
			for _, a := range r.Args {
				args = append(args, a)
			}
			args = append(args, hexec.WithStdout(&stdout))
			if r.Stderr {
				args = append(args, hexec.WithStderr(&stderr))
			}
			if r.Env != nil {
				args = append(args, hexec.WithEnviron(r.Env))
			}
			if r.Stdin != "" {
				args = append(args, hexec.WithStdin(strings.NewReader(r.Stdin)))
			}
			var runner hexec.Runner
			var err error
			if r.Op == "Npx" {
				runner, err = ex.Npx(r.Bin, args...)
			} else {
				runner, err = ex.New(r.Bin, args...)
			}
			if err != nil {
				res["err"] = goval.Str(repl(err.Error()))
				res["notFound"] = hexec.IsNotFound(err)
			} else {
				err = runner.Run()
				if err != nil {
					res["runErr"] = goval.Str(repl(err.Error()))
					res["notFound"] = hexec.IsNotFound(err)
				}
				res["stdout"] = goval.Str(repl(stdout.String()))
				if r.Stderr {
					res["stderr"] = goval.Str(repl(stderr.String()))
				}
			}
			results = append(results, res)
		}
		var files []any
		for _, f := range s.Files {
			files = append(files, []any{f.Path, f.Kind, f.Mode})
		}
		var runs []any
		for _, r := range s.Runs {
			runs = append(runs, map[string]any{"op": r.Op, "bin": r.Bin, "args": r.Args, "env": r.Env, "stdin": r.Stdin, "stderr": r.Stderr})
		}
		cases = append(cases, map[string]any{
			"name": s.Name, "sec": s.Sec, "path": s.Path, "env": env[1:], "files": files, "runs": runs, "results": results,
		})
		must(os.RemoveAll(root))
	}

	header := map[string]any{"oracle": "nh-config/hexec", "goarch": runtime.GOARCH, "scripts": Scripts}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "hexec.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "hexec: %d scenarios\n", len(cases))
}

// must stops the oracle on an unexpected error.
func must(err error) {
	if err != nil {
		log.Fatal(err)
	}
}
