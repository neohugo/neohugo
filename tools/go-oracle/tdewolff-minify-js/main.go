// Command tdewolff-minify-js is the Go oracle for crates/tdewolff-minify-js:
// it runs github.com/tdewolff/minify/v2/js (the exact version pinned in
// go.mod) and writes the fixtures and corpus digests that the Rust port is
// tested against.
//
// Usage:
//
//	tdewolff-minify-js stdin CFG                  # minifies stdin to stdout
//	tdewolff-minify-js fixtures DIR SCALE SEED    # writes DIR/*.rec.gz
//	tdewolff-minify-js tables OUT.rs              # the upstream test tables as Rust source
//	tdewolff-minify-js corpus OUT.tsv CFGS ROOT... # per-file output digests
//	tdewolff-minify-js sample N SEED              # prints generated programs
//	tdewolff-minify-js gen KIND OUT.rec.gz N SEED # red-team runs (redteam.go; digests)
//	tdewolff-minify-js files OUT.rec.gz CFGS LIST # listed files x configurations (digests)
//	tdewolff-minify-js enumerate OUT CFGS ALPHA MAXLEN # all short symbol sequences (enumerate.go)
//
// A configuration CFG is a '-'-separated list of tokens: vN (Version),
// pN (Precision), keep (KeepVarNames), alpha (the unexported
// useAlphabetVarNames), inline (params {"inline": "1"}); "0" is the zero
// Minifier. neohugo's configuration is "v2022" (and "v2022-inline" for
// on* attributes). The Rust tests parse the same names.
package main

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"reflect"
	"strconv"
	"strings"
	"unsafe"

	"github.com/tdewolff/minify/v2"
	"github.com/tdewolff/minify/v2/js"
	"github.com/tdewolff/parse/v2/buffer"
)

// config returns the Minifier and the params map of a configuration name.
func config(name string) (*js.Minifier, map[string]string) {
	o := &js.Minifier{}
	var params map[string]string
	if name == "0" {
		return o, nil
	}
	for _, tok := range strings.Split(name, "-") {
		switch {
		case tok == "keep":
			o.KeepVarNames = true
		case tok == "alpha":
			setUseAlphabetVarNames(o)
		case tok == "inline":
			params = map[string]string{"inline": "1"}
		case strings.HasPrefix(tok, "v"):
			n, err := strconv.Atoi(tok[1:])
			if err != nil {
				panic("bad config " + name)
			}
			o.Version = n
		case strings.HasPrefix(tok, "p"):
			n, err := strconv.Atoi(tok[1:])
			if err != nil {
				panic("bad config " + name)
			}
			o.Precision = n
		default:
			panic("bad config " + name)
		}
	}
	return o, params
}

// setUseAlphabetVarNames sets the unexported field that the upstream tests
// use (Minifier{useAlphabetVarNames: true}).
func setUseAlphabetVarNames(o *js.Minifier) {
	f := reflect.ValueOf(o).Elem().FieldByName("useAlphabetVarNames")
	if !f.IsValid() {
		panic("js.Minifier has no useAlphabetVarNames field")
	}
	reflect.NewAt(f.Type(), unsafe.Pointer(f.UnsafeAddr())).Elem().SetBool(true)
}

// minifyBuf minifies buf (used in place, as the HTML minifier does with
// buffer.NewReader(t.Data)) and returns the output and the error; a panic is
// reported as the error "PANIC: ...".
func minifyBuf(cfg string, buf []byte) (out []byte, err error) {
	o, params := config(cfg)
	var w bytes.Buffer
	defer func() {
		if r := recover(); r != nil {
			out = w.Bytes()
			err = fmt.Errorf("PANIC: %v", r)
		}
	}()
	err = o.Minify(minify.New(), &w, buffer.NewReader(buf), params)
	return w.Bytes(), err
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintln(os.Stderr, "usage: tdewolff-minify-js stdin|fixtures|tables|corpus|sample|gen ...")
		os.Exit(2)
	}
	switch os.Args[1] {
	case "stdin":
		in, err := io.ReadAll(os.Stdin)
		if err != nil {
			panic(err)
		}
		out, err := minifyBuf(os.Args[2], cp(in))
		_, _ = os.Stdout.Write(out)
		if err != nil {
			fmt.Fprintln(os.Stderr, "\nerror:", err)
			os.Exit(1)
		}
	case "fixtures":
		scale, err := strconv.Atoi(os.Args[3])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[4], 10, 64)
		if err != nil {
			panic(err)
		}
		genFixtures(os.Args[2], scale, seed)
	case "tables":
		genTables(os.Args[2])
	case "corpus":
		corpusDigests(os.Args[2], strings.Split(os.Args[3], ","), os.Args[4:])
	case "gen":
		// gen KIND OUT.rec.gz N SEED (red-team runs, see redteam.go)
		n, err := strconv.Atoi(os.Args[4])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[5], 10, 64)
		if err != nil {
			panic(err)
		}
		genRedTeam(os.Args[2], os.Args[3], n, seed)
	case "enumerate":
		// enumerate OUT CFGS ALPHA MAXLEN: all short symbol sequences (combined digests)
		maxLen, err := strconv.Atoi(os.Args[5])
		if err != nil {
			panic(err)
		}
		enumerate(os.Args[2], strings.Split(os.Args[3], ","), os.Args[4], maxLen)
	case "files":
		// files OUT.rec.gz CFGS LIST: every listed file through every configuration (digests)
		genFiles(os.Args[2], strings.Split(os.Args[3], ","), os.Args[4])
	case "sample":
		n, err := strconv.Atoi(os.Args[2])
		if err != nil {
			panic(err)
		}
		seed, err := strconv.ParseInt(os.Args[3], 10, 64)
		if err != nil {
			panic(err)
		}
		genSample(n, seed)
	default:
		fmt.Fprintln(os.Stderr, "unknown mode", os.Args[1])
		os.Exit(2)
	}
}
