// Command prose is the Go oracle for crates/nh-common/src/prose.rs, the port of
// github.com/jdkato/prose transform/title.go (the version in go.mod).
//
//	go run ./tools/go-oracle/nh-common/prose [-root .] [-out crates/nh-common/tests/fixtures/prose/title.json]
//
// For every string of the corpus (see ../corpus) it records
// NewTitleConverter(APStyle).Title, NewTitleConverter(ChicagoStyle).Title and
// the section-title composition Title(flect.Pluralize(s)) of hugolib
// (helpers.CreateTitle = AP style). A Go panic is recorded as {"panic": msg}.
// It also records the upstream test table (prose testdata/title.json).
package main

import (
	"encoding/json"
	"flag"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/gobuffalo/flect"
	"github.com/jdkato/prose/transform"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
)

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-common/tests/fixtures/prose/title.json", "output file")
	flag.Parse()

	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}
	upstream := upstreamCases()
	for _, c := range upstream {
		strs = append(strs, c.Input)
	}

	ap := transform.NewTitleConverter(transform.APStyle)
	chicago := transform.NewTitleConverter(transform.ChicagoStyle)
	createTitle := helpers.GetTitleFunc("ap")

	var cases []map[string]any
	seen := map[string]bool{}
	for _, s := range strs {
		if seen[s] {
			continue
		}
		seen[s] = true
		cases = append(cases, map[string]any{
			"in":           corpus.Encode(s),
			"ap":           corpus.Call(func() string { return ap.Title(s) }),
			"chicago":      corpus.Call(func() string { return chicago.Title(s) }),
			"create_title": corpus.Call(func() string { return createTitle(s) }),
			"ap_pluralize": corpus.Call(func() string { return ap.Title(flect.Pluralize(s)) }),
		})
	}

	var up []map[string]any
	for _, c := range upstream {
		up = append(up, map[string]any{"in": c.Input, "expect": c.Expect})
	}

	header := map[string]any{
		"source":   "tools/go-oracle/nh-common/prose",
		"upstream": up,
	}
	if err := corpus.WriteCases(*out, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%d cases -> %s", len(cases), *out)
}

type testCase struct {
	Input  string
	Expect string
}

// upstreamCases reads prose's own TestTitle table (testdata/title.json).
func upstreamCases() []testCase {
	cmd := exec.Command("go", "list", "-m", "-f", "{{.Dir}}", "github.com/jdkato/prose")
	cmd.Stderr = os.Stderr
	dir, err := cmd.Output()
	if err != nil {
		log.Fatal(err)
	}
	b, err := os.ReadFile(filepath.Join(strings.TrimSpace(string(dir)), "testdata", "title.json"))
	if err != nil {
		log.Fatal(err)
	}
	var cases []testCase
	if err := json.Unmarshal(b, &cases); err != nil {
		log.Fatal(err)
	}
	return cases
}
