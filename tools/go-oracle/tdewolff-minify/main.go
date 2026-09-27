// Command tdewolff-minify is the Go oracle for crates/tdewolff-minify: it runs
// github.com/tdewolff/minify/v2 (the exact version pinned in go.mod, without
// the js minifier) and writes fixtures and corpus outputs that the Rust port
// is tested against.
//
// Usage:
//
//	tdewolff-minify fixtures DIR [SCALE]      # writes DIR/*.txt fixtures
//	tdewolff-minify tree SRC OUT              # minifies every .html/.xml/.json of SRC into OUT (seeksnack config, no JS)
//	tdewolff-minify digests SRC OUT.tsv [EXT...] # per-file digests of the minified output
//	tdewolff-minify nested DIR KIND OUT.tsv   # corpus2-style .in files through one minifier
//	tdewolff-minify stdin MEDIATYPE           # minifies stdin to stdout (seeksnack config, no JS)
//	tdewolff-minify tables OUTDIR             # generates the Rust hash/table sources
//	tdewolff-minify fuzz OUT N SEED ROOT...   # writes a large random differential set
//	tdewolff-minify structured OUT N SEED     # writes N grammar-shaped css/svg/html cases each
//	tdewolff-minify cfgdigests SRC OUT.tsv CFGS EXT... # digests of SRC through every listed config
//	tdewolff-minify cases OUT LISTFILE         # hand-written cases (MEDIATYPE CONFIGS QUOTED-INPUT)
//	tdewolff-minify adv OUT N SEED            # adversarial documents, page windows and helper inputs
//	tdewolff-minify replay FILE...            # re-runs fixture records, reports differences
//	tdewolff-minify rerun IN OUT              # re-answers the records of IN with this build
//	tdewolff-minify rt KIND N SEED OUT        # red-team generators (redteam.go); OUT "-" = stdout
package main

import (
	"fmt"
	"io"
	"os"
	"runtime"
	"strconv"
	"strings"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: tdewolff-minify fixtures|tree|digests|nested|stdin|tables|fuzz ...")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "fixtures":
		if len(os.Args) > 3 {
			n, err := strconv.Atoi(os.Args[3])
			if err != nil {
				panic(err)
			}
			scale = n
		}
		genFixtures(os.Args[2])
	case "tree":
		minifyTree(os.Args[2], os.Args[3])
	case "digests":
		treeDigests(os.Args[2], os.Args[3], os.Args[4:])
	case "cases":
		// cases OUTDIR LISTFILE
		genCases(os.Args[2], os.Args[3])
	case "cfgdigests":
		// cfgdigests SRC OUT.tsv CFG,CFG,... EXT...
		allConfigDigests(os.Args[2], os.Args[3], strings.Split(os.Args[4], ","), os.Args[5:])
	case "nested":
		nestedDigests(os.Args[2], os.Args[3], os.Args[4])
	case "stdin":
		m := seeksnackM()
		in, err := io.ReadAll(os.Stdin)
		if err != nil {
			panic(err)
		}
		out, err := m.Bytes(os.Args[2], in)
		_, _ = os.Stdout.Write(out)
		if err != nil {
			fmt.Fprintln(os.Stderr, "error:", err)
			os.Exit(1)
		}
	case "tables":
		genTables(os.Args[2])
	case "fuzz":
		n, err := strconv.Atoi(os.Args[3])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[4], 10, 64)
		if err != nil {
			panic(err)
		}
		fuzzGen(os.Args[2], n, seed, os.Args[5:])
	case "adv":
		// adv OUTDIR N SEED
		n, err := strconv.Atoi(os.Args[3])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[4], 10, 64)
		if err != nil {
			panic(err)
		}
		genAdversarial(os.Args[2], n, seed)
	case "rt":
		// rt KIND N SEED OUT
		n, err := strconv.Atoi(os.Args[3])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[4], 10, 64)
		if err != nil {
			panic(err)
		}
		genRedTeam(os.Args[2], n, seed, os.Args[5])
	case "rerun":
		// rerun IN OUT
		rerun(os.Args[2], os.Args[3])
	case "replay":
		// replay FILE...
		replay(os.Args[2:])
	case "structured":
		// structured OUTDIR N SEED
		n, err := strconv.Atoi(os.Args[3])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[4], 10, 64)
		if err != nil {
			panic(err)
		}
		w := newFix(os.Args[2], "structured")
		genStructured(w, n, seed)
		w.close()
	default:
		fmt.Fprintln(os.Stderr, "unknown mode", os.Args[1])
		os.Exit(2)
	}
}

func goVersion() string { return runtime.Version() }
