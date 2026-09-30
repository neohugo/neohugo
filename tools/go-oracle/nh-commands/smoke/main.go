// Command smoke is the Go oracle for the build command of crates/nh-commands
// (Wave B task T25): `neohugo [build] --minify --clock ... -d <out>` on small
// synthetic sites whose templates use only the template functions that are
// ported at this task's base (no now, upper, site, hugo, page; RSS disabled).
//
//	go run ./tools/go-oracle/nh-commands/smoke [-out rust/testdata/oracle/commands/smoke]
//
// Each case recreates its site in a temporary directory and runs
// commands.Execute in a child process (like main.go) with an explicit
// environment. The fixture records stdout, stderr and the exit code (the
// version line and the timings masked, "$ROOT" for the temporary directory)
// and every file of the publish directory and of the site directory after the
// build (hugo_stats.json, .hugo_build.lock).
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
)

const placeholder = "$ROOT"

type caseSpec struct {
	Name string `json:"name"`
	// Files of the site (below $ROOT/site; "$ROOT" replaced).
	Files map[string]string `json:"files"`
	Args  []string          `json:"args"`
	Env   map[string]string `json:"env,omitempty"`
}

func main() {
	out := flag.String("out", "rust/testdata/oracle/commands/smoke", "fixture directory")
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

	var rows []map[string]any
	for _, c := range cases() {
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
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	header := map[string]any{"env": baseEnv()}
	if err := goval.WriteCasesGz(filepath.Join(*out, "smoke.json.gz"), header, rows); err != nil {
		log.Fatal(err)
	}
	fmt.Printf("smoke: %d cases\n", len(rows))
}

func baseEnv() map[string]string {
	return map[string]string{
		"HOME":   placeholder + "/home",
		"TMPDIR": placeholder + "/tmp",
		"PATH":   placeholder + "/bin",
	}
}

var (
	versionRe = regexp.MustCompile(`(neohugo v\d+\.\d+\.\d+(?:-DEV)?)(?:-[0-9a-f]{7,40})? (\S+) BuildDate=\S+`)
	timingRe  = regexp.MustCompile(`(?m)^(Total|Built) in \d+ ms$`)
	goosRe    = regexp.MustCompile(`(?m)^(GOOS|GOARCH)="[^"]*"$`)
)

func norm(s, tmp string) string {
	s = strings.ReplaceAll(s, tmp, placeholder)
	s = versionRe.ReplaceAllString(s, "$1 $$OS/$$ARCH BuildDate=$$DATE")
	s = goosRe.ReplaceAllString(s, `$1="$$$1"`)
	return timingRe.ReplaceAllString(s, "$1 in N ms")
}

func run(c caseSpec) (map[string]any, error) {
	tmp, err := os.MkdirTemp("", "nh-commands-smoke-")
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
		if err := os.WriteFile(p, []byte(strings.ReplaceAll(c.Files[n], placeholder, tmp)), 0o644); err != nil {
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

// dump lists every regular file below root (except the home, tmp and bin
// directories and the site's sources), with its content in hex.
func dump(root string) (map[string]string, error) {
	out := map[string]string{}
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(root, path)
		rel = filepath.ToSlash(rel)
		if d.IsDir() {
			switch rel {
			case "home", "tmp", "bin", "site/content", "site/layouts", "site/static":
				return filepath.SkipDir
			}
			return nil
		}
		if rel == "site/hugo.toml" {
			return nil
		}
		b, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		out[rel] = hex.EncodeToString(b)
		return nil
	})
	return out, err
}

func smokeSite() map[string]string {
	return map[string]string{
		"hugo.toml": `baseURL = "https://example.org/"
title = "Smoke"
languageCode = "en"
disableKinds = ["rss"]
[params]
tagline = "hello"
[build.buildStats]
enable = true
`,
		"content/_index.md":       "---\ntitle: Home\n---\nWelcome *home*.\n",
		"content/posts/first.md":  "---\ntitle: First Post\ndate: 2026-01-02\ntags: [a, b]\n---\nSome **bold** text.\n",
		"content/posts/second.md": "---\ntitle: Second\ndate: 2026-02-03\n---\nMore text.\n",
		"content/posts/draft.md":  "---\ntitle: Draft\ndate: 2026-03-04\ndraft: true\n---\nNot yet.\n",
		"content/posts/future.md": "---\ntitle: Future\ndate: 2030-01-01\n---\nLater.\n",
		"layouts/_default/baseof.html": `<!doctype html>
<html lang="en">
<head><title>{{ .Title }} | {{ .Site.Title }}</title></head>
<body class="page {{ .Kind }}">
{{ block "main" . }}{{ end }}
<footer id="foot">{{ .Site.Params.tagline }}</footer>
</body>
</html>
`,
		"layouts/_default/single.html": `{{ define "main" }}<article class="post"><h1>{{ .Title }}</h1>{{ .Content }}<p>{{ .Date.Format "2006-01-02" }}</p></article>{{ end }}
`,
		"layouts/_default/list.html": `{{ define "main" }}<h1>{{ .Title }}</h1>{{ .Content }}<ul>{{ range .Pages }}<li><a href="{{ .RelPermalink }}">{{ .Title }}</a></li>{{ end }}</ul>{{ end }}
`,
		"layouts/index.html": `{{ define "main" }}<h1>{{ .Site.Title }} - {{ .Site.Params.tagline }}</h1>{{ .Content }}<ul>{{ range first 5 .Site.RegularPages }}<li>{{ .Title }}</li>{{ end }}</ul>{{ end }}
`,
		"static/css/site.css":             "body { color: red; }\n",
		"static/.well-known/security.txt": "Contact: mailto:x@example.org\n",
		"static/.DS_Store":                "\x00\x00\x00\x01Bud1",
		"static/robots-extra.txt.bak":     "old\n",
	}
}

func with(files map[string]string, extra map[string]string) map[string]string {
	out := map[string]string{}
	for k, v := range files {
		out[k] = v
	}
	for k, v := range extra {
		out[k] = v
	}
	return out
}

func cases() []caseSpec {
	s := smokeSite()
	return []caseSpec{
		{Name: "golden-flags", Files: s, Args: []string{"--minify", "--clock", "2026-09-27T12:00:00Z", "-d", "$ROOT/out"}},
		{Name: "build-subcommand-nominify", Files: s, Args: []string{"build", "--clock", "2026-09-27T12:00:00Z", "--destination=$ROOT/out"}},
		{Name: "drafts-future", Files: s, Args: []string{"-DF", "--minify", "--clock", "2026-09-27T12:00:00Z", "-d", "$ROOT/out"}},
		{Name: "default-publishdir-quiet", Files: s, Args: []string{"--quiet", "--clock", "2026-09-27T12:00:00Z"}},
		{Name: "clean-destination", Files: with(s, map[string]string{"public/stale.txt": "stale\n", "public/.keep/x": "x\n"}), Args: []string{"--cleanDestinationDir", "--clock", "2026-09-27T12:00:00Z", "--noBuildLock"}},
		{Name: "baseurl-environment", Files: with(s, map[string]string{"config/staging/hugo.toml": "title = \"Smoke Staging\"\n"}), Args: []string{"-e", "staging", "-b", "https://staging.example.org/sub/", "--clock", "2026-09-27T12:00:00Z", "-d", "$ROOT/out"}},
		{Name: "env-var-environment", Files: with(s, map[string]string{"config/development/hugo.toml": "title = \"Smoke Dev\"\n"}), Args: []string{"--clock", "2026-09-27T12:00:00Z", "-d", "$ROOT/out"}, Env: map[string]string{"HUGO_ENVIRONMENT": "development"}},
		{Name: "source-flag", Files: s, Args: []string{"-s", "$ROOT/site", "--clock", "2026-09-27T12:00:00Z", "-d", "../out"}},
	}
}
