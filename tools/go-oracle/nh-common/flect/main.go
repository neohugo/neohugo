// Command flect is the Go oracle for crates/nh-common/src/flect.rs, the port of
// github.com/gobuffalo/flect (the version in go.mod).
//
//	go run ./tools/go-oracle/nh-common/flect [-root .] [-out rust/testdata/oracle/common/flect/flect.json]
//
// For every string of the corpus (see ../corpus) it records Pluralize,
// Singularize, Humanize, Ordinalize (the functions neohugo calls), Titleize and the
// template function inflect.Humanize of neohugo (tpl/inflect) with a string
// argument. A Go panic is recorded as {"panic": msg}. Capitalize
// (used inside Humanize and Pluralize) run in the custom-data cases and in
// flect's own test tables on the Rust side.
//
// flect loads inflections.json and acronyms.json from the working directory
// (or $INFLECT_PATH / $ACRONYMS_PATH) in init(). The "custom" cases run this
// program again (-child) in a temporary directory holding those files, and
// record the results, what flect printed while loading, and init panics.
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/gobuffalo/flect"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tpl/inflect"
)

var ops = []string{"pluralize", "singularize", "humanize", "titleize", "capitalize", "ordinalize"}

func apply(op, s string) any {
	switch op {
	case "pluralize":
		return corpus.Call(func() string { return flect.Pluralize(s) })
	case "singularize":
		return corpus.Call(func() string { return flect.Singularize(s) })
	case "humanize":
		return corpus.Call(func() string { return flect.Humanize(s) })
	case "titleize":
		return corpus.Call(func() string { return flect.Titleize(s) })
	case "capitalize":
		return corpus.Call(func() string { return flect.Capitalize(s) })
	case "ordinalize":
		return corpus.Call(func() string { return flect.Ordinalize(s) })
	}
	log.Fatalf("unknown op %q", op)
	return nil
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/common/flect/flect.json", "output file")
	child := flag.String("child", "", "internal: write the custom-data results for the inputs in this file")
	flag.Parse()

	if *child != "" {
		runChild(*child)
		return
	}

	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}

	ns := inflect.New()
	var cases []map[string]any
	for _, s := range strs {
		c := map[string]any{"in": corpus.Encode(s)}
		for _, op := range []string{"pluralize", "singularize", "humanize", "titleize", "ordinalize"} {
			c[op] = apply(op, s)
		}
		c["inflect_humanize"] = corpus.Call(func() string {
			r, err := ns.Humanize(s)
			if err != nil {
				return "error: " + err.Error()
			}
			return r
		})
		cases = append(cases, c)
	}

	custom, err := customCases()
	if err != nil {
		log.Fatal(err)
	}

	header := map[string]any{
		"source": "tools/go-oracle/nh-common/flect",
		"custom": custom,
	}
	if err := corpus.WriteCases(*out, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%d cases, %d custom -> %s", len(cases), len(custom), *out)
}

// customInputs are run through every op in each custom-data configuration.
var customInputs = []string{
	"snack", "snacks", "Snack", "big-snack", "seek snack", "seek_snack_id", "SeekSnack",
	"SEEKSNACK", "seek", "qqq", "QQQ", "user", "users", "aircraft", "person", "a b", "x", "c",
	"cookie2", "cookiez", "mbps", "API", "api_key", "",
}

type customConfig struct {
	Name        string  `json:"name"`
	Inflections *string `json:"inflections"`
	Acronyms    *string `json:"acronyms"`
}

func str(s string) *string { return &s }

var customConfigs = []customConfig{
	{Name: "none"},
	{Name: "acronyms", Acronyms: str(`["SEEK", "SNACK", "QQQ", "mbps"]`)},
	{Name: "inflections", Inflections: str(`{"snack": "snackz", "cookie2": "cookiez", "user": "usern"}`)},
	{Name: "both", Inflections: str(`{"snack": "snackz"}`), Acronyms: str(`["SEEK"]`)},
	{Name: "acronyms-invalid", Acronyms: str(`not json`)},
	{Name: "acronyms-type", Acronyms: str(`["SEEK", 1]`)},
	{Name: "acronyms-object", Acronyms: str(`{"SEEK": true}`)},
	{Name: "acronyms-null", Acronyms: str(`null`)},
	{Name: "acronyms-null-elem", Acronyms: str(`["SEEK", null]`)},
	{Name: "acronyms-huge-number", Acronyms: str(`["SEEK", 1e400]`)},
	{Name: "inflections-huge-number", Inflections: str(`{"snack": 1e400}`)},
	{Name: "acronyms-trailing", Acronyms: str(`["QQQ"] trailing garbage`)},
	{Name: "acronyms-empty-file", Acronyms: str(``)},
	{Name: "inflections-multiword", Inflections: str(`{"a b": "c"}`)},
	{Name: "inflections-multiword-plural", Inflections: str(`{"x": "c d"}`)},
	{Name: "inflections-type", Inflections: str(`{"snack": 1}`)},
	{Name: "inflections-null", Inflections: str(`{"snack": null}`)},
	{Name: "inflections-dup-singular", Inflections: str(`{"aircraft": "aircrafts"}`)},
	{Name: "inflections-dup-plural", Inflections: str(`{"folk": "people"}`)},
	{Name: "inflections-dup-alternative", Inflections: str(`{"foo": "fishes"}`)},
	{Name: "inflections-empty-plural", Inflections: str(`{"aircraft": ""}`)},
}

type customResult struct {
	Name        string         `json:"name"`
	Inflections *string        `json:"inflections"`
	Acronyms    *string        `json:"acronyms"`
	Printed     []string       `json:"printed"`
	InitPanic   string         `json:"init_panic,omitempty"`
	Results     map[string]any `json:"results,omitempty"`
}

func customCases() ([]customResult, error) {
	self, err := os.Executable()
	if err != nil {
		return nil, err
	}
	var out []customResult
	for _, cfg := range customConfigs {
		dir, err := os.MkdirTemp("", "flect-oracle")
		if err != nil {
			return nil, err
		}
		if cfg.Inflections != nil {
			if err := os.WriteFile(filepath.Join(dir, "inflections.json"), []byte(*cfg.Inflections), 0o644); err != nil {
				return nil, err
			}
		}
		if cfg.Acronyms != nil {
			if err := os.WriteFile(filepath.Join(dir, "acronyms.json"), []byte(*cfg.Acronyms), 0o644); err != nil {
				return nil, err
			}
		}
		resFile := filepath.Join(dir, "results.json")
		cmd := exec.Command(self, "-child", resFile)
		cmd.Dir = dir
		var env []string
		for _, e := range os.Environ() {
			if !strings.HasPrefix(e, "INFLECT_PATH=") && !strings.HasPrefix(e, "ACRONYMS_PATH=") {
				env = append(env, e)
			}
		}
		cmd.Env = env
		var stdout, stderr strings.Builder
		cmd.Stdout = &stdout
		cmd.Stderr = &stderr
		runErr := cmd.Run()

		r := customResult{Name: cfg.Name, Inflections: cfg.Inflections, Acronyms: cfg.Acronyms, Printed: []string{}}
		for _, l := range strings.Split(strings.TrimRight(stdout.String(), "\n"), "\n") {
			if l != "" {
				r.Printed = append(r.Printed, l)
			}
		}
		var ee *exec.ExitError
		if errors.As(runErr, &ee) {
			first, _, _ := strings.Cut(stderr.String(), "\n")
			r.InitPanic = strings.TrimPrefix(first, "panic: ")
		} else if runErr != nil {
			return nil, runErr
		} else {
			b, err := os.ReadFile(resFile)
			if err != nil {
				return nil, err
			}
			if err := json.Unmarshal(b, &r.Results); err != nil {
				return nil, err
			}
		}
		if err := os.RemoveAll(dir); err != nil {
			return nil, err
		}
		out = append(out, r)
	}
	return out, nil
}

// runChild runs in the configured working directory: flect's init has loaded
// the custom files by now.
func runChild(resFile string) {
	res := map[string]any{}
	for _, s := range customInputs {
		for _, op := range ops {
			res[op+":"+s] = apply(op, s)
		}
	}
	b, err := json.Marshal(res) // sorted keys
	if err != nil {
		log.Fatal(err)
	}
	if err := os.WriteFile(resFile, b, 0o644); err != nil {
		log.Fatal(err)
	}
}
