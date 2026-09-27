package main

import (
	"bufio"
	"compress/gzip"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"runtime"
	"strconv"
	"strings"

	"github.com/tdewolff/minify/v2"
	"github.com/tdewolff/minify/v2/svg"
)

// replayer re-runs fixture records through this Go build.
type replayer struct {
	ms map[string]*minify.M
}

func (p *replayer) m(cfg string) *minify.M {
	if m, ok := p.ms[cfg]; ok {
		return m
	}
	if len(p.ms) > 256 {
		p.ms = map[string]*minify.M{} // each M holds compiled regexps
	}
	m := configByName(cfg)
	p.ms[cfg] = m
	return m
}

func unhexStr(s string) []byte {
	b, err := hex.DecodeString(s)
	if err != nil {
		panic(err)
	}
	return b
}

// recompute returns the record re-run through this build (same fields,
// fresh results), or nil for kinds that are not replayed.
func (p *replayer) recompute(r []string) (rec []string) {
	switch r[0] {
	case "min":
		in := unhexStr(r[3])
		defer func() {
			if v := recover(); v != nil {
				rec = panicRec(r[1], string(unhexStr(r[2])), in, v)
			}
		}()
		out, after, err := runM(p.m(r[1]), string(unhexStr(r[2])), in)
		return []string{r[0], r[1], r[2], r[3], sameOr(out, in), errStr(err), sameOr(after, in)}
	case "num", "dec":
		prec, _ := strconv.Atoi(r[1])
		in := unhexStr(r[2])
		buf := cp(in)
		var out []byte
		if r[0] == "num" {
			out = minify.Number(buf, prec)
		} else {
			out = minify.Decimal(buf, prec)
		}
		return []string{r[0], r[1], r[2], sameOr(out, in), sameOr(buf, in)}
	case "mt":
		in := unhexStr(r[1])
		buf := cp(in)
		out := minify.Mediatype(buf)
		return []string{r[0], r[1], sameOr(out, in), sameOr(buf, in)}
	case "duri":
		in := unhexStr(r[2])
		buf := cp(in)
		out := minify.DataURI(p.m(r[1]), buf)
		return []string{r[0], r[1], r[2], sameOr(out, in), sameOr(buf, in)}
	case "path":
		prec, _ := strconv.Atoi(r[1])
		in := unhexStr(r[2])
		buf := cp(in)
		out := svg.NewPathData(&svg.Minifier{Precision: prec}).ShortenPathData(buf)
		return []string{r[0], r[1], r[2], sameOr(out, in), sameOr(buf, in)}
	}
	return nil // upstream table rows and other kinds are not replayed
}

// eachRecord calls fn for every record line of the given fixture files
// (.txt.gz, plain text, or "-" for stdin).
func eachRecord(files []string, fn func(line string, r []string)) {
	for _, name := range files {
		var rd io.Reader = os.Stdin
		var f *os.File
		if name != "-" {
			var err error
			if f, err = os.Open(name); err != nil {
				panic(err)
			}
			rd = f
			if strings.HasSuffix(name, ".gz") {
				gz, err := gzip.NewReader(f)
				if err != nil {
					panic(err)
				}
				rd = gz
			}
		}
		sc := bufio.NewScanner(rd)
		sc.Buffer(make([]byte, 1<<20), 1<<30)
		for sc.Scan() {
			line := sc.Text()
			if line != "" && !strings.HasPrefix(line, "#") {
				fn(line, strings.Split(line, "\t"))
			}
		}
		if err := sc.Err(); err != nil {
			panic(err)
		}
		if f != nil {
			_ = f.Close()
		}
	}
}

// replay FILE... re-runs every record of the given fixture files through
// this Go build and reports the records whose recomputed result differs. It
// checks that a build (e.g. the linux/arm64 oracle under qemu) reproduces
// fixtures whose inputs cannot be regenerated without the site corpora, and
// compares amd64 with arm64.
func replay(files []string) {
	p := &replayer{map[string]*minify.M{}}
	total, bad := 0, 0
	eachRecord(files, func(line string, r []string) {
		got := p.recompute(r)
		if got == nil {
			return
		}
		total++
		if strings.Join(got, "\t") != line {
			bad++
			if bad <= 20 {
				fmt.Printf("DIFF %s\n  rec: %s\n  now: %s\n", line[:min(len(line), 300)], strings.Join(r[len(r)-3:], " "), strings.Join(got[len(got)-3:], " "))
			}
		}
	})
	fmt.Printf("replay: %d records, %d differ (%s %s)\n", total, bad, goVersion(), runtime.GOARCH)
	if bad > 0 {
		os.Exit(1)
	}
}

// rerun IN OUT writes the records of IN re-run through this build to OUT
// ("-" = stdout): e.g. amd64 mismatches re-answered by the arm64 oracle.
func rerun(in, out string) {
	p := &replayer{map[string]*minify.M{}}
	w := newFix(out, "rerun")
	eachRecord([]string{in}, func(_ string, r []string) {
		if got := p.recompute(r); got != nil {
			w.rec(got...)
		}
	})
	w.close()
}
