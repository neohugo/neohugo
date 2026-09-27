// Command xtext-collate is the Go oracle for the Rust crate
// crates/xtext-collate (a port of golang.org/x/text@v0.26.0 collate,
// internal/colltab and the parts of unicode/norm and language it needs).
//
//	go run ./tools/go-oracle/xtext-collate gen [-out crates/xtext-collate/data]
//	    Extracts the generated tables (collate, norm, language) from the
//	    x/text sources pinned in go.mod and writes the binary blobs the crate
//	    embeds.
//
//	go run ./tools/go-oracle/xtext-collate fixtures [-dir crates/xtext-collate/tests/fixtures]
//	    Writes the checked-in differential fixtures.
//
//	go run ./tools/go-oracle/xtext-collate corpus -out FILE [-n N] [-seed S] [-kind random|stress] [-locales] [-runes] [-config NAME]
//	    Writes large differential corpora digests (outside the repo); checked
//	    by the ignored Rust test big_corpus (XTEXT_COLLATE_CORPUS=FILE).
//
//	go run ./tools/go-oracle/xtext-collate pairs -out FILE [-n N] [-seed S] [-full main|all|none] [-locales=false] [-site site-strings.hex]
//	    Writes adversarial near-equal comparison pairs and Go's Compare /
//	    CompareString results for every config (pairs.go).
//
//	go run ./tools/go-oracle/xtext-collate enum -out FILE [-alpha thai|latin|marks] [-maxlen N] [-locales] [-dump CONFIG]
//	    Keys and comparisons of every string of length <= N over a small
//	    alphabet (enum.go).
//
//	go run ./tools/go-oracle/xtext-collate tagfuzz -out FILE [-n N] [-seed S] [-regressions] [-in FILE]
//	    Adversarial language tags in the tags.tsv format (tagfuzz.go);
//	    -regressions / -in prepend fixed inputs (`tagfuzz -regressions -n 3600
//	    -seed 12` writes the equivalent of tests/fixtures/tags-fuzz.tsv; see
//	    crates/xtext-collate/PORTING.md).
//
//	go run ./tools/go-oracle/xtext-collate settype -out FILE [-n N] [-seed S]
//	    Tag.SetTypeForKey over fuzzed tags and adversarial key/values.
//
//	go run ./tools/go-oracle/xtext-collate nfd-dump
//	    Prints x/text NFD/NFKD of every code point that changes (research:
//	    comparison with the unicode-normalization crate).
package main

import (
	"fmt"
	"os"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: xtext-collate gen|fixtures|corpus [flags]")
		os.Exit(2)
	}
	var err error
	switch os.Args[1] {
	case "gen":
		err = cmdGen(os.Args[2:])
	case "fixtures":
		err = cmdFixtures(os.Args[2:])
	case "corpus":
		err = cmdCorpus(os.Args[2:])
	case "nfd-dump":
		err = cmdNFDDump(os.Args[2:])
	case "pairs":
		err = cmdPairs(os.Args[2:])
	case "enum":
		err = cmdEnum(os.Args[2:])
	case "tagfuzz":
		err = cmdTagFuzz(os.Args[2:])
	case "settype":
		err = cmdSetType(os.Args[2:])
	default:
		err = fmt.Errorf("unknown command %q", os.Args[1])
	}
	if err != nil {
		fmt.Fprintln(os.Stderr, "error:", err)
		os.Exit(1)
	}
}
