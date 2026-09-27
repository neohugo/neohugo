//go:build go1.27

// Command go-unicode is the Go oracle for the Rust crate crates/go-unicode.
//
// It has three modes:
//
//	go run ./tools/go-oracle/go-unicode genexports
//	    Parses $GOROOT/src/unicode/tables.go and writes exports_gen.go in this
//	    directory: a map from every exported *unicode.RangeTable variable name
//	    to the variable itself, so the other modes can cross-check the parsed
//	    source against the compiled tables.
//
//	go run ./tools/go-oracle/go-unicode tables [-out crates/go-unicode/src/tables.rs]
//	    Parses $GOROOT/src/unicode/{tables,casetables}.go and emits the Rust
//	    tables. Every parsed table is checked against the compiled unicode
//	    package before anything is written.
//
//	go run ./tools/go-oracle/go-unicode fixtures [-dir crates/go-unicode/tests/fixtures]
//	    Writes the differential test fixtures (exhaustive per-code-point hashes
//	    for every predicate/table/case mapping, utf8 vectors, strings/bytes
//	    vectors on random input including invalid UTF-8).
//
//	go run ./tools/go-oracle/go-unicode gotests [-out crates/go-unicode/tests/fixtures/go_test_tables.txt]
//	    Extracts the test tables of Go's own unicode/utf8/utf16/strings/bytes
//	    tests by parsing and evaluating the *_test.go table literals.
//
//	go run ./tools/go-oracle/go-unicode adversarial [-out crates/go-unicode/tests/fixtures/adversarial_hashes.txt]
//	    Writes hash lines for the adversarial families (table structure,
//	    SimpleFold orbits, strings/bytes functions over every code point and
//	    every short byte string, EqualFold over fold orbits).
package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: go-unicode genexports|tables|fixtures|gotests|adversarial [flags]")
		os.Exit(2)
	}
	var err error
	switch os.Args[1] {
	case "genexports":
		err = genExports(os.Args[2:])
	case "tables":
		err = genTables(os.Args[2:])
	case "fixtures":
		err = genFixtures(os.Args[2:])
	case "gotests":
		err = genGoTests(os.Args[2:])
	case "adversarial":
		err = genAdversarial(os.Args[2:])
	default:
		err = fmt.Errorf("unknown mode %q", os.Args[1])
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "go-unicode:", err)
		os.Exit(1)
	}
}
