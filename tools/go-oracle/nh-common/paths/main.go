// Command paths is the Go oracle for common/paths in crates/nh-common (Wave B
// task T02): PathParser (every Path accessor), the path.go and url.go string
// helpers.
//
//	go run ./tools/go-oracle/nh-common/paths [-root .] [-out rust/testdata/oracle/common/paths]
//
// The seeksnack site is private, so the PathParser inputs are this
// repository's Hugo sites (docs/, hugolib/testsite, create/skeletons, every
// file and directory), the embedded templates (tpl/tplimpl/embedded/templates),
// the seeksnack paths quoted in docs/rust-port/specs, a synthetic sweep of
// Hugo's naming patterns (see sweep.go) and the string corpus of
// ../corpus as term keys. They are parsed with three parsers:
//
//   - "seeksnack": the real ContentPathParser of a config loaded by
//     allconfig from the seeksnack config dump
//     (docs/rust-port/specs/architecture-core-data/config-en.json: en + th,
//     nine disabled languages, the site's output formats and media types);
//   - "test": the testParser of common/paths/pathparser_test.go;
//   - "nolang": a nil LanguageIndex and IsLangDisabled.
//
// Every call the parsers make to IsOutputFormat, IsContentExt and
// IsLangDisabled is recorded with its answer, so the Rust test can replay the
// callbacks without the config crates.
//
// Output: pathparser.json.gz and strings.json.gz (gzip, best compression, no
// name or time), one case per line. Nothing here depends on the platform
// (no floats; unix path rules on linux and darwin). allconfig pulls in cgo
// (libwebp), so this oracle does not cross-build with CGO_ENABLED=0; replaying
// the recorded seeksnack callbacks on linux/arm64 (qemu) gave the same bytes.
package main

import (
	"flag"
	"log"
	"path/filepath"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/common/paths", "output directory")
	flag.Parse()

	writeGz := func(name string, h map[string]any, cases []map[string]any) {
		p := filepath.Join(*out, name)
		if err := goval.WriteCasesGz(p, h, cases); err != nil {
			log.Fatal(err)
		}
		log.Printf("%s: %d cases", p, len(cases))
	}

	parsers := newParsers(*root)
	inputs := pathInputs(*root)
	cases := pathParserCases(parsers, inputs)
	ph := map[string]any{"parsers": parsers.describe(), "keys": caseKeys()}
	writeGz("pathparser.json.gz", ph, cases)

	sh := map[string]any{"contextRoots": contextRoots, "permalinkHosts": permalinkHosts}
	writeGz("strings.json.gz", sh, stringCases(*root, inputs))
}
