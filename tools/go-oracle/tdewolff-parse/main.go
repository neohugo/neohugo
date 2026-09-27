// Command tdewolff-parse is the Go oracle for crates/tdewolff-parse: it runs
// github.com/tdewolff/parse/v2 (the exact version pinned in go.mod) and writes
// fixtures and corpus digests that the Rust port is tested against.
//
// Usage:
//
//	tdewolff-parse fixtures DIR           # writes DIR/*.txt fixtures
//	tdewolff-parse corpus KIND ROOT OUT   # writes per-file digests for a corpus
//	tdewolff-parse extract HTMLROOT OUT   # extracts embedded css/svg/js/json from html
//	tdewolff-parse dump KIND FILE         # prints the serialized stream of one file
//	tdewolff-parse fuzz OUT N SEED EXHAUST MAXWIN ROOT...  # random stream digests
//	tdewolff-parse fnfuzz DIR N SEED                       # random root/strconv/misc records
//	tdewolff-parse corpusnums DIR ROOT...                  # Parse* records for corpus numbers
package main

import (
	"fmt"
	"os"
	"runtime"
	"strconv"
)

func goVersion() string { return runtime.Version() }

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: tdewolff-parse fixtures|corpus|extract|dump ...")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "fixtures":
		// fixtures DIR [SCALE]
		if len(os.Args) > 3 {
			n, err := strconv.Atoi(os.Args[3])
			if err != nil {
				panic(err)
			}
			scale = n
		}
		genFixtures(os.Args[2])
	case "corpus":
		// corpus KIND ROOT OUT [EXT...]
		corpusDigests(os.Args[2], os.Args[3], os.Args[4], os.Args[5:])
	case "extract":
		// extract HTMLROOT OUTDIR
		extractEmbedded(os.Args[2], os.Args[3])
	case "fuzz":
		// fuzz OUTDIR N SEED EXHAUST MAXWIN CORPUSROOT...
		var nums [4]int
		for i := range nums {
			v, err := strconv.Atoi(os.Args[3+i])
			if err != nil {
				panic(err)
			}
			nums[i] = v
		}
		fuzzGen(os.Args[2], nums[0], int64(nums[1]), nums[2], nums[3], os.Args[7:])
	case "fnfuzz":
		// fnfuzz DIR N SEED
		n, err := strconv.Atoi(os.Args[3])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[4], 10, 64)
		if err != nil {
			panic(err)
		}
		fnFuzz(os.Args[2], n, seed)
	case "corpusnums":
		// corpusnums DIR ROOT...
		corpusNumbers(os.Args[2], os.Args[3:])
	case "dump":
		// dump KIND FILE
		in, err := os.ReadFile(os.Args[3])
		if err != nil {
			panic(err)
		}
		os.Stdout.Write(streamFor(os.Args[2], in))
	default:
		fmt.Fprintln(os.Stderr, "unknown mode", os.Args[1])
		os.Exit(2)
	}
}
