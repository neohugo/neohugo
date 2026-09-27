// Command tdewolff-parse-js is the Go oracle for crates/tdewolff-parse-js: it
// runs github.com/tdewolff/parse/v2/js (the exact version pinned in go.mod)
// and writes fixtures and corpus digests that the Rust port is tested
// against.
//
// Usage:
//
//	tdewolff-parse-js fixtures DIR [NFUZZ SEED]  # literal + fuzz fixture sets (gzip)
//	tdewolff-parse-js corpus OUT.tsv ROOT...     # per-file digests of all dumps
//	tdewolff-parse-js fuzzcorpus OUT N SEED MAXLEN ROOT...  # mutated corpus windows
//	tdewolff-parse-js grammar OUT N SEED         # generated programs (digests)
//	tdewolff-parse-js grammarfull OUT N SEED     # generated programs (full dumps)
//	tdewolff-parse-js grammarsample N SEED       # prints generated programs
//	tdewolff-parse-js tables OUT SEED            # exhaustive token/rune/sort tables
//	tdewolff-parse-js limits OUT.tsv             # nesting limits and uint16 wrap-around
//	tdewolff-parse-js dump MODE FILE             # prints one dump of one file
//	tdewolff-parse-js time FILE                  # average js.Parse time
//
// MODE is one of lex, lexre, parse, parsew2f, parseinline, string, js, json.
package main

import (
	"fmt"
	"os"
	"strconv"
	"time"

	"github.com/tdewolff/parse/v2"
	"github.com/tdewolff/parse/v2/js"
)

// modes are the serializations compared for every input, in record order.
var modes = []string{"lex", "lexre", "parse", "parsew2f", "parseinline", "string", "js", "json"}

func runMode(mode string, src []byte) []byte {
	switch mode {
	case "lex":
		return lexDump(src, false)
	case "lexre":
		return lexDump(src, true)
	case "parse":
		return parseDump(src, js.Options{})
	case "parsew2f":
		return parseDump(src, js.Options{WhileToFor: true})
	case "parseinline":
		return parseDump(src, js.Options{WhileToFor: true, Inline: true})
	case "string":
		if 1<<20 < len(src) {
			return []byte("SKIP\n") // Go's String() concatenates quadratically
		}
		return stringDump(src, js.Options{})
	case "js":
		if 1<<20 < len(src) {
			return []byte("SKIP\n")
		}
		return jsDump(src, js.Options{})
	case "json":
		return jsonDump(src, js.Options{})
	}
	panic("unknown mode " + mode)
}

func parseOK(src []byte) (*js.AST, error) {
	return js.Parse(parse.NewInputBytes(append([]byte(nil), src...)), js.Options{})
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: tdewolff-parse-js fixtures|corpus|dump ...")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "fixtures":
		nfuzz, seed := 4000, int64(1)
		if len(os.Args) > 4 {
			var err error
			if nfuzz, err = strconv.Atoi(os.Args[3]); err != nil {
				panic(err)
			}
			if seed, err = strconv.ParseInt(os.Args[4], 10, 64); err != nil {
				panic(err)
			}
		}
		genFixtures(os.Args[2], nfuzz, seed)
	case "fuzzcorpus":
		// fuzzcorpus OUT N SEED MAXLEN ROOT...
		n, err := strconv.Atoi(os.Args[3])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[4], 10, 64)
		if err != nil {
			panic(err)
		}
		maxLen, err := strconv.Atoi(os.Args[5])
		if err != nil {
			panic(err)
		}
		genCorpusFuzz(os.Args[2], n, seed, maxLen, os.Args[6:])
	case "corpus":
		corpusDigests(os.Args[2], os.Args[3:])
	case "grammar", "grammarfull":
		// grammar OUT N SEED: generated programs (digests; grammarfull: full dumps)
		n, err := strconv.Atoi(os.Args[3])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[4], 10, 64)
		if err != nil {
			panic(err)
		}
		genGrammar(os.Args[2], n, seed, os.Args[1] == "grammarfull")
	case "tables":
		// tables OUT SEED: exhaustive token/rune/AsIdentifierName/VarsByUses checks
		seed, err := strconv.ParseInt(os.Args[3], 10, 64)
		if err != nil {
			panic(err)
		}
		genTables(os.Args[2], seed)
	case "limits":
		// limits OUT.tsv: nesting-limit and uint16 wrap-around inputs (specs + digests)
		genLimits(os.Args[2])
	case "grammarsample":
		// grammarsample N SEED: prints generated programs and whether they parse
		n, err := strconv.Atoi(os.Args[2])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[3], 10, 64)
		if err != nil {
			panic(err)
		}
		genGrammarSample(n, seed)
	case "time":
		// time FILE: average js.Parse time (WhileToFor) over 5 runs
		src, err := os.ReadFile(os.Args[2])
		if err != nil {
			panic(err)
		}
		t := time.Now()
		for i := 0; i < 5; i++ {
			if _, err := js.Parse(parse.NewInputBytes(append([]byte(nil), src...)), js.Options{WhileToFor: true}); err != nil {
				panic(err)
			}
		}
		fmt.Printf("%v per parse\n", time.Since(t)/5)
	case "dump":
		src, err := os.ReadFile(os.Args[3])
		if err != nil {
			panic(err)
		}
		_, _ = os.Stdout.Write(runMode(os.Args[2], src))
	default:
		fmt.Fprintln(os.Stderr, "unknown command", os.Args[1])
		os.Exit(2)
	}
}
