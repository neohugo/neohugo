// Command goldmark is the Go oracle for crates/goldmark (a port of
// github.com/yuin/goldmark v1.7.12).
//
// Usage (from the repository root):
//
//	GOLDMARK_DIR=$(go env GOMODCACHE)/github.com/yuin/goldmark@v1.7.12 \
//	  go run ./tools/go-oracle/goldmark gentables crates/goldmark/src/util
//	go run ./tools/go-oracle/goldmark render < cases.json > out.json
//	go run ./tools/go-oracle/goldmark fixtures crates/goldmark/tests/fixtures
//	go run ./tools/go-oracle/goldmark fuzz -n 20000 -seed 1 > fuzz.json
//	go run ./tools/go-oracle/goldmark extfixtures crates/goldmark/tests/fixtures <seeksnack>/content
//	go run ./tools/go-oracle/goldmark extcorpus <content dir> out.gmf.gz hugo,hugo-autoid
//	go run ./tools/go-oracle/goldmark fuzz -mode ext|extbytes -cfg hugo,x-all -n N -seed S > big.gmf
//
// See crates/goldmark/PORTING.md for what each fixture covers.
package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: goldmark gentables|render|fixtures|fuzz|corpus|regex|extfixtures|extcorpus ...")
		os.Exit(2)
	}
	args := os.Args[2:]
	switch os.Args[1] {
	case "gentables":
		genTables(args[0])
	case "render":
		renderMain(args)
	case "fixtures":
		fixturesMain(args)
	case "fuzz":
		fuzzMain(args)
	case "corpus":
		corpusMain(args)
	case "regex":
		regexMain(args)
	case "extfixtures":
		extFixturesMain(args)
	case "extcorpus":
		extCorpusMain(args)
	case "extregex":
		extRegexMain(args)
	default:
		fmt.Fprintln(os.Stderr, "unknown mode", os.Args[1])
		os.Exit(2)
	}
}
