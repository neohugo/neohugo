// Command herrors is the Go oracle for the error texts of crates/nh-common's
// herrors: how Error() composes positioned (herrors.FileError) and plain
// errors wrapped with fmt.Errorf("%s: %w"), fmt.Errorf("%s: %v"),
// errors.Join and the herrors.NewFileError* constructors.
//
//	go run ./tools/go-oracle/nh-common/herrors [-out crates/nh-common/tests/fixtures/herrors]
//
// Every case is a base error and a sequence of up to three wrapping steps
// (every sequence over the step kinds). Output: herrors.json.gz, one record
// per case with Error(), the position of herrors.UnwrapFileError (or null) and
// errors.Is(err, fs.ErrNotExist). The program reads no files and no network;
// the output does not depend on the platform or the time. Stdout goes
// to /dev/null (text.Position.String colours its output on a terminal) and
// HUGO_FILE_LOG_FORMAT must be unset (it changes the position format).
package main

import (
	"errors"
	"flag"
	"fmt"
	"io/fs"
	"log"
	"os"
	"path/filepath"

	"github.com/neohugo/neohugo/common/herrors"
	"github.com/neohugo/neohugo/common/text"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// base is a named starting error; the Rust test builds the same errors by name.
type base struct {
	name string
	make func() error
}

var bases = []base{
	{"plain", func() error { return errors.New("boom") }},
	{"plainLine", func() error { return errors.New("yaml: line 4: did not find expected key") }},
	{"notExist", func() error { return fmt.Errorf("open data/x.json: %w", fs.ErrNotExist) }},
	{"filePos", func() error {
		return herrors.NewFileErrorFromPos(errors.New("boom"),
			text.Position{Filename: "/site/assets/js/main.js", Offset: -1, LineNumber: 3, ColumnNumber: 5})
	}},
	{"filePosNoName", func() error {
		return herrors.NewFileErrorFromPos(errors.New("boom"),
			text.Position{Offset: -1, LineNumber: 2, ColumnNumber: 1})
	}},
	{"fileName", func() error {
		return herrors.NewFileErrorFromName(errors.New("template: x.html:12:3: unexpected EOF"), "/site/layouts/x.html")
	}},
	{"fileNoPos", func() error { return herrors.NewFileError(errors.New("(7, 9): bad bundle")) }},
	{"fileNotExist", func() error {
		return herrors.NewFileErrorFromName(fmt.Errorf("open a.md: %w", fs.ErrNotExist), "/site/content/a.md")
	}},
}

// prefixes[i] is the fmt.Errorf prefix of the i-th step (the last one holds
// line numbers for the extractors of a later NewFileError step).
var prefixes = []string{
	"readAndProcessContent",
	`JSBUILD: failed to transform "js/main.js" (text/javascript)`,
	"template: t.html:9:4: executing",
}

var steps = []string{"w", "v", "join", "joinFile", "name", "pos", "new"}

func apply(step string, i int, err error) error {
	p := prefixes[i]
	switch step {
	case "w":
		return fmt.Errorf("%s: %w", p, err)
	case "v":
		return fmt.Errorf("%s: %v", p, err)
	case "join":
		return errors.Join(err, errors.New("other"))
	case "joinFile":
		return errors.Join(nil, herrors.NewFileErrorFromName(errors.New("other"), "o.md"), err)
	case "name":
		return herrors.NewFileErrorFromName(err, fmt.Sprintf("/site/outer%d.html", i))
	case "pos":
		return herrors.NewFileErrorFromPos(err, text.Position{Filename: "p.md", Offset: -1, LineNumber: 10 + i, ColumnNumber: 2})
	case "new":
		return herrors.NewFileError(err)
	}
	panic(step)
}

func main() {
	out := flag.String("out", "crates/nh-common/tests/fixtures/herrors", "output directory")
	flag.Parse()

	if os.Getenv("HUGO_FILE_LOG_FORMAT") != "" {
		log.Fatal("unset HUGO_FILE_LOG_FORMAT")
	}
	devnull, err := os.Open(os.DevNull)
	if err != nil {
		log.Fatal(err)
	}
	os.Stdout = devnull

	var seqs [][]string
	var gen func(prefix []string)
	gen = func(prefix []string) {
		seqs = append(seqs, append([]string{}, prefix...))
		if len(prefix) == len(prefixes) {
			return
		}
		for _, s := range steps {
			gen(append(prefix, s))
		}
	}
	gen(nil)

	var cases []map[string]any
	for _, b := range bases {
		for _, seq := range seqs {
			e := b.make()
			for i, s := range seq {
				e = apply(s, i, e)
			}
			rec := map[string]any{
				"base":       b.name,
				"steps":      seq,
				"error":      e.Error(),
				"isNotExist": errors.Is(e, fs.ErrNotExist),
				"pos":        nil,
			}
			if fe := herrors.UnwrapFileError(e); fe != nil {
				p := fe.Position()
				rec["pos"] = map[string]any{"filename": p.Filename, "line": p.LineNumber, "column": p.ColumnNumber}
			}
			cases = append(cases, rec)
		}
	}

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	p := filepath.Join(*out, "herrors.json.gz")
	header := map[string]any{"bases": len(bases), "sequences": len(seqs)}
	if err := goval.WriteCasesGz(p, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%d cases -> %s", len(cases), p)
}
