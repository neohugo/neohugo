// Command values is the Go oracle for the value helpers of crates/nh-common
// (Wave B task T01): common/maps (Params, Scratch, KeyRenamer, the map
// conversions), common/collections (Append, Slice), common/hreflect
// (IsTruthful & co.), common/types (conversions), common/htime
// (ToTimeInDefaultLocationE, TimeFormatter).
//
//	go run ./tools/go-oracle/nh-common/values [-root .] [-out crates/nh-common/tests/fixtures/values]
//
// Inputs (the seeksnack site is private, so these substitute for its decoded
// config and front matter):
//   - the decoded seeksnack config dumps in
//     docs/rust-port/specs/architecture-core-data/config-{en,th,mounts,en-printzero}.json,
//     plus case-mixed variants of them;
//   - the front matter of every content file under docs/content,
//     hugolib/testsite/content and create/skeletons, decoded by neohugo's
//     pageparser (YAML, TOML and JSON front matter);
//   - adversarial maps: case-mixed and colliding keys, nested maps of every
//     map type, _merge keys, every value kind.
//
// The output is typed JSON (see ../goval); the two params fixtures are
// gzip-compressed. Results that depend on Go's randomized map order (keys that
// differ only in case, several keys renamed to one) are recorded as
// {"nondet": true}. ConvertFloat64WithNoDecimalsToInt converts floats to
// integers, which is platform dependent for out-of-range values, so the
// checked-in fixtures come from an arm64 build (GOARCH=arm64, run under
// qemu-aarch64-static).
package main

import (
	"flag"
	"log"
	"path/filepath"
	"runtime"
	"time"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-common/tests/fixtures/values", "output directory")
	flag.Parse()

	time.Local = time.UTC

	header := map[string]any{"goarch": runtime.GOARCH}
	write := func(name string, cases []map[string]any) {
		p := filepath.Join(*out, name)
		if err := corpus.WriteCases(p, header, cases); err != nil {
			log.Fatal(err)
		}
		log.Printf("%s: %d cases", p, len(cases))
	}
	writeGz := func(name string, cases []map[string]any) {
		p := filepath.Join(*out, name)
		if err := goval.WriteCasesGz(p, header, cases); err != nil {
			log.Fatal(err)
		}
		log.Printf("%s: %d cases", p, len(cases))
	}

	inputs := paramsInputs(*root)
	writeGz("params_inputs.json.gz", encodeInputs(inputs))
	writeGz("params.json.gz", paramsCases(inputs))
	write("scratch.json", scratchCases())
	write("collections.json", collectionsCases())
	write("reflect.json", reflectCases())
	write("htime.json", htimeCases())
}
