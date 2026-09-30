// Command cli is the Go oracle for crates/nh-commands (Wave B task T25): the
// neohugo command line (commands/*.go, main.go) run in-process.
//
//	go run ./tools/go-oracle/nh-commands/cli [-root .] [-out rust/testdata/oracle/commands/cli]
//
// Every case is a command line run in a site tree recreated in a temporary
// directory ("$ROOT" in the arguments, the environment and the outputs is that
// directory). Each case runs in its own child process (the commands keep
// package-level state: htime.Clock, the global logger), with an explicit
// environment (HOME, TMPDIR and an empty PATH below $ROOT, plus the case's
// variables). Two modes:
//
//   - parse: the real cobra command tree of commands.newExec; the command that
//     cobra's Find resolves, the flag parse (cobra ParseFlags), the changed
//     flags and the config provider that commands.flagsToCfg builds from them
//     (the flag -> config key mapping), encoded with goval;
//   - run: commands.Execute(args) as main.go runs it: stdout, stderr (main's
//     "Error: ..." line included) and the exit code.
//
// Run mode is used for the commands that do not build a site (config, config
// mounts, version, env, help) and for command lines that fail before the build.
// The version line is masked: the VCS revision is removed, the platform becomes
// "$OS/$ARCH" and the build date "$DATE" (env prints GOOS="$GOOS" and
// GOARCH="$GOARCH").
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"regexp"
	"sort"
	"strings"
	"time"
	"unsafe"

	"github.com/bep/simplecobra"
	"github.com/neohugo/neohugo/commands"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/spf13/pflag"
)

//go:linkname newExec github.com/neohugo/neohugo/commands.newExec
func newExec() (*simplecobra.Exec, error)

//go:linkname mapLegacyArgs github.com/neohugo/neohugo/commands.mapLegacyArgs
func mapLegacyArgs(args []string) []string

//go:linkname flagsToCfg github.com/neohugo/neohugo/commands.flagsToCfg
func flagsToCfg(cd *simplecobra.Commandeer, cfg config.Provider) config.Provider

const placeholder = "$ROOT"

// A site is a tree of files (a path ending in "/" is an empty directory)
// written to $ROOT/<dir>.
type site struct {
	Dir   string            `json:"dir"`
	Files map[string]string `json:"files"`
}

// A caseSpec is one command line.
type caseSpec struct {
	Name string `json:"name"`
	// Site is the key of the site whose directory is the working directory.
	Site string   `json:"site"`
	Args []string `json:"args"`
	// Modes: "parse", "run" or both.
	Modes []string `json:"modes"`
	// Env adds process environment variables.
	Env map[string]string `json:"env,omitempty"`
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/commands/cli", "fixture directory")
	child := flag.String("child", "", "internal: run one case (JSON) in this process")
	mode := flag.String("mode", "", "internal: parse or run")
	flag.Parse()

	if *child != "" {
		var c caseSpec
		if err := json.Unmarshal([]byte(*child), &c); err != nil {
			log.Fatal(err)
		}
		switch *mode {
		case "parse":
			runParse(c)
		case "run":
			runMain(c)
		default:
			log.Fatalf("unknown mode %q", *mode)
		}
		return
	}

	sites, err := buildSites(*root)
	if err != nil {
		log.Fatal(err)
	}
	cases := buildCases()

	var rows []map[string]any
	for _, c := range cases {
		row := map[string]any{"name": c.Name, "site": c.Site, "args": c.Args, "modes": c.Modes}
		if len(c.Env) > 0 {
			row["env"] = c.Env
		}
		for _, m := range c.Modes {
			res, err := runChild(sites, c, m)
			if err != nil {
				log.Fatalf("%s/%s: %v", c.Name, m, err)
			}
			row[m] = res
		}
		rows = append(rows, row)
	}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	header := map[string]any{"sites": sites, "env": baseEnvSpec()}
	if err := goval.WriteCasesGz(filepath.Join(*out, "cli.json.gz"), header, rows); err != nil {
		log.Fatal(err)
	}
	fmt.Printf("cli: %d cases\n", len(rows))
}

// baseEnvSpec is the process environment of every case ("$ROOT" replaced).
func baseEnvSpec() map[string]string {
	return map[string]string{
		"HOME":   placeholder + "/home",
		"TMPDIR": placeholder + "/tmp",
		"PATH":   placeholder + "/bin",
	}
}

// runChild recreates the sites in a new temporary directory and runs the case
// in a child process. The same case runs twice; the results must agree.
func runChild(sites map[string]site, c caseSpec, mode string) (map[string]any, error) {
	var first map[string]any
	for i := 0; i < 2; i++ {
		res, err := runChildOnce(sites, c, mode)
		if err != nil {
			return nil, err
		}
		if i == 0 {
			first = res
			continue
		}
		a, _ := json.Marshal(first)
		b, _ := json.Marshal(res)
		if !bytes.Equal(a, b) {
			return nil, fmt.Errorf("nondeterministic result:\n%s\n%s", a, b)
		}
	}
	return first, nil
}

func runChildOnce(sites map[string]site, c caseSpec, mode string) (map[string]any, error) {
	tmp, err := os.MkdirTemp("", "nh-commands-cli-")
	if err != nil {
		return nil, err
	}
	defer func() { _ = os.RemoveAll(tmp) }()
	tmp, err = filepath.EvalSymlinks(tmp)
	if err != nil {
		return nil, err
	}
	var keys []string
	for k := range sites {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		// Sites sharing a directory name: only the case's own site is written.
		if sites[k].Dir == sites[c.Site].Dir && k != c.Site {
			continue
		}
		if err := materialize(filepath.Join(tmp, sites[k].Dir), sites[k].Files, tmp); err != nil {
			return nil, err
		}
	}
	for _, d := range []string{"home", "tmp", "bin"} {
		if err := os.MkdirAll(filepath.Join(tmp, d), 0o755); err != nil {
			return nil, err
		}
	}

	spec, err := json.Marshal(c)
	if err != nil {
		return nil, err
	}
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
	defer cancel()
	cmd := exec.CommandContext(ctx, os.Args[0], "-child", string(spec), "-mode", mode)
	cmd.Dir = filepath.Join(tmp, sites[c.Site].Dir)
	var env []string
	for k, v := range baseEnvSpec() {
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
	exitCode := 0
	if err := cmd.Run(); err != nil {
		if ee, ok := err.(*exec.ExitError); ok {
			exitCode = ee.ExitCode()
		} else {
			return nil, err
		}
	}
	norm := func(s string) string {
		return maskVersion(strings.ReplaceAll(s, tmp, placeholder))
	}
	if mode == "parse" {
		if exitCode != 0 {
			return nil, fmt.Errorf("parse child failed: %s", stderr.String())
		}
		var res map[string]any
		if err := json.Unmarshal([]byte(norm(stdout.String())), &res); err != nil {
			return nil, err
		}
		return res, nil
	}
	return map[string]any{
		"stdout": norm(stdout.String()),
		"stderr": norm(stderr.String()),
		"exit":   exitCode,
	}, nil
}

var (
	versionRe = regexp.MustCompile(`(neohugo v\d+\.\d+\.\d+(?:-DEV)?)(?:-[0-9a-f]{7,40})? (\S+) BuildDate=\S+`)
	timingRe  = regexp.MustCompile(`(?m)^(Total|Built) in \d+ ms$`)
	goosRe    = regexp.MustCompile(`(?m)^(GOOS|GOARCH)="[^"]*"$`)
)

// maskVersion removes the VCS revision from the version line and masks its
// date and platform (GOOS/GOARCH, also in the env output), and masks the build
// timings ("Total in N ms").
func maskVersion(s string) string {
	s = versionRe.ReplaceAllString(s, "$1 $$OS/$$ARCH BuildDate=$$DATE")
	s = goosRe.ReplaceAllString(s, `$1="$$$1"`)
	return timingRe.ReplaceAllString(s, "$1 in N ms")
}

// runMain is main.go: commands.Execute, then log.Fatalf on error.
func runMain(c caseSpec) {
	log.SetFlags(0)
	err := commands.Execute(replaceRoot(c.Args))
	if err != nil {
		log.Fatalf("Error: %s", err)
	}
}

func replaceRoot(args []string) []string {
	// The child runs in $ROOT/<site>: the root is the parent of the working dir.
	wd, _ := os.Getwd()
	tmp := filepath.Dir(wd)
	out := make([]string, len(args))
	for i, a := range args {
		out[i] = strings.ReplaceAll(a, placeholder, tmp)
	}
	return out
}

// execRoot returns the root Commandeer of x (the unexported field c).
func execRoot(x *simplecobra.Exec) *simplecobra.Commandeer {
	f := reflect.ValueOf(x).Elem().FieldByName("c")
	return (*simplecobra.Commandeer)(unsafe.Pointer(f.Pointer()))
}

// runParse resolves the command and parses the flags like cobra's ExecuteC,
// then maps the changed flags to config keys with commands.flagsToCfg.
func runParse(c caseSpec) {
	res := map[string]any{}
	defer func() {
		if r := recover(); r != nil {
			res["panic"] = fmt.Sprint(r)
		}
		b, err := json.Marshal(res)
		if err != nil {
			log.Fatal(err)
		}
		_, _ = os.Stdout.Write(b)
	}()

	x, err := newExec()
	if err != nil {
		res["execErr"] = err.Error()
		return
	}
	args := mapLegacyArgs(replaceRoot(c.Args))
	res["mappedArgs"] = args
	rc := execRoot(x).CobraCommand
	rc.SetContext(context.Background())
	rc.InitDefaultHelpCmd()
	rc.InitDefaultCompletionCmd(args...)
	cmd, flags, err := rc.Find(args)
	if cmd != nil {
		res["command"] = cmd.CommandPath()
	}
	if err != nil {
		res["findErr"] = err.Error()
		return
	}
	res["flagArgs"] = flags
	cmd.InitDefaultHelpFlag()
	if err := cmd.ParseFlags(flags); err != nil {
		res["parseErr"] = err.Error()
		return
	}
	var changed []map[string]any
	cmd.Flags().VisitAll(func(f *pflag.Flag) {
		if f.Changed {
			changed = append(changed, map[string]any{"name": f.Name, "type": f.Value.Type(), "value": f.Value.String()})
		}
	})
	res["changed"] = changed
	res["positional"] = cmd.Flags().Args()
	cfg := flagsToCfg(&simplecobra.Commandeer{CobraCommand: cmd}, nil)
	res["cfg"] = goval.Encode(cfg.Get(""))
}

// materialize writes files below dir; "$ROOT" in contents is tmp.
func materialize(dir string, files map[string]string, tmp string) error {
	var names []string
	for n := range files {
		names = append(names, n)
	}
	sort.Strings(names)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	for _, n := range names {
		p := filepath.Join(dir, filepath.FromSlash(n))
		if strings.HasSuffix(n, "/") {
			if err := os.MkdirAll(p, 0o755); err != nil {
				return err
			}
			continue
		}
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			return err
		}
		if err := os.WriteFile(p, []byte(strings.ReplaceAll(files[n], placeholder, tmp)), 0o644); err != nil {
			return err
		}
	}
	return nil
}
