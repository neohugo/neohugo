//go:build gotemplate_oracle

// Mode "rtfixtures": the checked-in red-team regression fixtures — the
// minimized cases of the differences found by the red-team modes
// (rtparse, rtexec, rthtml, rtns, rttns, rtlayout) plus a fixed-seed
// sample of each mode, so `cargo test` keeps exercising the generators'
// territory without the large corpora.
//
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate rtfixtures crates/gotemplate/tests/fixtures
//
// Writes text/redteam_parse.txt.gz, text/redteam_exec.txt.gz and
// html/redteam.txt.gz. Cases whose Go output depends on pointer addresses
// (run twice with a different heap layout, compared) are left out, so the
// fixtures regenerate byte for byte.
package main

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"regexp"
	"runtime"
)

func init() {
	register("rtfixtures", rtFixturesMain)
}

// rtRegressionExec are the exec regressions (Go: printed/escaped values
// that once differed).
var rtRegressionExec = []etxCase{
	// A nil *TryError (TryValue.Err without an error) has the Error method:
	// fmt calls it for %v %s %q %x %X and prints <nil> (catchPanic).
	{name: "tryerr", data: "root", mode: "both", src: `{{printf "%s|%q|%x|%X|%v|%d|%t" (try 1).Err (try 1).Err (try 1).Err (try 1).Err (try 1).Err (try 1).Err (try 1).Err}}`},
	{name: "tryerr", data: "root", mode: "both", src: `{{printf "%s" (try 1)}}|{{printf "%10.4q" (try false)}}|{{printf "%x" (try "a")}}|{{printf "%+v" (try 2)}}`},
	{name: "tryerr", data: "root", mode: "both", src: `{{printf "%s" (list (try 1).Err)}}|{{printf "%v" (dict "e" (try 1).Err)}}|{{print (try 1).Err}}`},
	{name: "tryerr", data: "root", mode: "both", src: `{{with try (echo 1)}}{{printf "[%s][%5s][%-5v]" .Err .Err .Err}}{{end}}`},
	{name: "tryerr", data: "root", mode: "both", src: `{{html (try 1).Err}}|{{js (try 1).Err}}|{{urlquery (try 1).Err}}|{{(try 1).Err}}|{{if (try 1).Err}}T{{else}}F{{end}}`},
	{name: "tryerr", data: "root", mode: "both", src: `{{printf "%s" (try (fail "x")).Err}}|{{printf "%q" (try (fail "x"))}}`},
	// TryValue.Value after an error is a nil `any` field: a nil interface
	// (field access fails), not the invalid Value.
	{name: "tryvalue", data: "root", mode: "both", src: `{{(try (fail 1)).Value.X}}`},
	{name: "tryvalue", data: "root", mode: "both", opt: "missingkey=error", src: `{{(try (fail 1)).Value.X}}`},
	{name: "tryvalue", data: "root", mode: "both", src: `{{with (try (fail 1))}}{{.Value.X}}{{end}}`},
	{name: "tryvalue", data: "root", mode: "both", src: `{{range (try (fail 1)).Value}}x{{else}}E{{end}}|{{len (try (fail 1)).Value}}`},
	{name: "tryvalue", data: "root", mode: "both", src: `{{(try (fail 1)).Value}}|{{printf "%v|%T" (try (fail 1)).Value (try (fail 1)).Value}}|{{eq (try (fail 1)).Value nil}}`},
	{name: "tryvalue", data: "root", mode: "both", src: `{{(try nilany).Value.X}}|{{index (try (fail 1)).Value 1}}`},
	// TryError.Err is a template.ExecError: fields Name and Err (a
	// *fmt.wrapError for "error calling", else an *errors.errorString),
	// methods Error and Unwrap.
	{name: "execerror", data: "root", mode: "both", src: `{{with try (fail "x")}}{{.Err.Err.Name}}|{{.Err.Err.Err}}|{{.Err.Cause}}|{{.Err.Unwrap}}|{{.Err.Error}}|{{.Err.Err.Unwrap}}{{end}}`},
	{name: "execerror", data: "root", mode: "both", src: `{{with try (fail "x")}}{{typeof .Err}}|{{typeof .Err.Err}}|{{typeof .Err.Cause}}|{{typeof .Err.Unwrap}}|{{typeof .Err.Err.Err}}|{{typeof .Err.Err.Err.Unwrap}}{{end}}`},
	{name: "execerror", data: "root", mode: "both", src: `{{with try (index .SS 10)}}{{typeof .Err.Err.Err}}|{{typeof .Err.Cause}}|{{.Err.Cause}}{{end}}`},
	{name: "execerror", data: "root", mode: "both", src: `{{with try (.Nope.X 1)}}{{typeof .Err.Err.Err}}|{{typeof .Err.Cause}}|{{.Err.Cause}}|{{.Err.Err.Name}}{{end}}`},
	{name: "execerror", data: "root", mode: "both", src: `{{with try (fail "x")}}{{.Err.Err.Err.Error}}|{{.Err.Cause.Error}}|{{.Err.Err.Name | len}}|{{eq .Err .Err}}{{end}}`},
	{name: "execerror", data: "root", mode: "both", src: `{{define "e"}}{{with try (fail "in e")}}{{.Err.Err.Name}}:{{.Err.Err.Err}}{{end}}{{end}}{{template "e"}}`},
	// The error values keep their identity across accesses.
	{name: "execerror", data: "root", mode: "both", src: `{{with try (fail "x")}}{{eq .Err.Cause .Err.Cause}}|{{eq .Err .Err}}|{{eq .Err.Err .Err.Err}}|{{eq .Err.Unwrap .Err.Err}}{{end}}`},
	{name: "execerror", data: "root", mode: "both", src: `{{$a := try (fail 1)}}{{$b := try (fail 1)}}{{eq $a.Err $b.Err}}|{{eq $a.Err $a.Err}}|{{eq $a.Err.Cause $b.Err.Cause}}`},
	// Go's slice builtin does not unwrap interface-kinded indexes (PORTING
	// deviation 15, classified by the test).
	{name: "slice-iface", data: "root", mode: "both", src: `{{slice .S 0 .Zero}}`},
	{name: "slice-iface", data: "root", mode: "both", src: `{{$z := .Zero}}{{slice .S $z 2}}|{{slice .S (index .SI 0)}}`},
	// A mkT name with invalid UTF-8 prints its bytes (test model fix).
	{name: "mkT", data: "root", mode: "both", src: `{{mkT .Bad | or}}{{(mkT .Bad).Name}}`},
	// Host arity errors inside try, printed through urlquery/js/html.
	{name: "try-arity", data: "root", mode: "both", src: `{{urlquery (try (echo 1 2))}}|{{js (try (nilerr 1))}}`},
}

func rtFixturesMain(args []string) error {
	if len(args) != 1 {
		return fmt.Errorf("usage: rtfixtures <fixtures dir>")
	}
	dir := args[0]
	if err := rtWriteBestGz(filepath.Join(dir, "text", "redteam_parse.txt.gz"), rtParseFixture); err != nil {
		return err
	}
	if err := rtWriteBestGz(filepath.Join(dir, "text", "redteam_exec.txt.gz"), rtExecFixture); err != nil {
		return err
	}
	return rtWriteBestGz(filepath.Join(dir, "html", "redteam.txt.gz"), rtHTMLFixture)
}

func rtWriteBestGz(path string, write func(w *bufio.Writer) error) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	zw, _ := gzip.NewWriterLevel(f, gzip.BestCompression)
	w := bufio.NewWriter(zw)
	if err := write(w); err != nil {
		return err
	}
	if err := w.Flush(); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	return f.Close()
}

func rtParseFixture(w *bufio.Writer) error {
	ptxWriteFuncSets(w)
	r := rand.New(rand.NewSource(20260927))
	k := 0
	for i := 0; i < 2500; i++ {
		c := rtParseCase(r)
		// Template names with % verbs can print pointers (deviation 17).
		out, ok := rtStable(func(bw *bufio.Writer) { ptxRun(bw, k, c) })
		if !ok {
			continue
		}
		if _, err := w.Write(out); err != nil {
			return err
		}
		k++
	}
	return nil
}

// rtHeapPad shifts later allocations between the two runs of a case.
var rtHeapPad [][]byte

// rtPointerLikeRE matches numbers that may be Go pointer addresses printed
// in decimal, binary or hex (a freed address can be reused by the second
// run of rtStable, so equal runs do not prove the absence of pointers).
var rtPointerLikeRE = regexp.MustCompile(`[0-9]{10,}|[01]{30,}|[0-9a-fA-F]{9,}`)

// rtStable runs f twice with a different heap layout and reports whether
// the two outputs are equal and hold nothing like a pointer address.
func rtStable(f func(w *bufio.Writer)) ([]byte, bool) {
	var a, b bytes.Buffer
	wa := bufio.NewWriter(&a)
	f(wa)
	_ = wa.Flush()
	rtHeapPad = append(rtHeapPad, make([]byte, 4096+len(rtHeapPad)*64))
	runtime.GC()
	wb := bufio.NewWriter(&b)
	f(wb)
	_ = wb.Flush()
	return a.Bytes(), bytes.Equal(a.Bytes(), b.Bytes()) && !rtPointerLikeRE.Match(rtResults(a.Bytes()))
}

// rtResults keeps the result parts of a record (the output, error and
// dump lines, not the sources, which may hold long numbers).
func rtResults(rec []byte) []byte {
	var out []byte
	for _, line := range bytes.Split(rec, []byte("\n")) {
		switch {
		case bytes.HasPrefix(line, []byte("out ")), bytes.HasPrefix(line, []byte("err ")),
			bytes.HasPrefix(line, []byte("perr ")):
			out = append(append(out, line...), '\n')
		case bytes.HasPrefix(line, []byte("O\t")):
			if i := bytes.Index(line, []byte("\t=>")); i >= 0 {
				out = append(append(out, line[i:]...), '\n')
			}
		}
	}
	return out
}

func rtExecFixture(w *bufio.Writer) error {
	cases := append([]etxCase(nil), rtRegressionExec...)
	r := rand.New(rand.NewSource(20260927))
	for i := 0; i < 1500; i++ {
		cases = append(cases, rtExecCase(r))
	}
	k := 0
	for _, c := range cases {
		for _, mode := range []string{"plain", "hugo"} {
			out, ok := rtStable(func(bw *bufio.Writer) { etxRun(bw, k, c, mode) })
			if !ok {
				continue
			}
			if _, err := w.Write(out); err != nil {
				return err
			}
			k++
		}
	}
	return nil
}

// rtRegressionNilAnySources exercise a host function returning a nil `any`
// (Rust hosts return Invalid, contract C7): Go's call result is a nil
// interface, so a field of it is an error, unlike a field of the invalid
// Value a pipeline yields.
var rtRegressionNilAnySources = []string{
	`{{ site.Config }}`,
	`a{{ site.Config.Privacy.Disable }}b`,
	`{{ (site).Config }}|{{ $s := site }}{{ $s.Config }}|{{ site | print }}|{{ print site }}|{{ site }}`,
	`{{ (try site.Config).Err }}`,
	`{{ with try (site.Config) }}{{ .Err }}|{{ .Value }}{{ end }}`,
	`{{ (try site).Value }}|{{ (try site).Value.X }}`,
	`{{ (partial "x" .).Foo }}`,
	`{{ partial "x" . }}|{{ if partial "x" . }}T{{ else }}F{{ end }}|{{ and (partial "x" .) 1 }}`,
	`<a href="{{ site.BaseURL }}">`,
	`{{ range site }}x{{ else }}empty{{ end }}|{{ len site }}`,
	`{{ eq site nil }}|{{ index site }}`,
	`{{ site.Config | print }}`,
}

func rtRegressionScripts() []*script {
	var ss []*script
	for i, src := range rtRegressionNilAnySources {
		s := newScript(fmt.Sprintf("regression/nilany/html/%d", i)).
			op("new", "t", "c").
			op("funcs", "t", "stubs", "site,partial,try").
			op("parse", "t", src).
			op("execc", "t", sStr("dot")).
			op("exechugo", "t", "nil").
			op("dump", "t")
		ss = append(ss, s)
		ss = append(ss, newScript(fmt.Sprintf("regression/nilany/text/%d", i)).
			op("tnew", "t", "c").
			op("tfuncs", "t", "stubs", "site,partial,try").
			op("tparse", "t", src).
			op("texec", "t", sStr("dot")).
			op("toption", "t", "missingkey=error").
			op("texec", "t", "nil"))
	}
	return ss
}

func rtHTMLFixture(w *bufio.Writer) error {
	scripts := rtRegressionScripts()
	r := rand.New(rand.NewSource(20260927))
	for i := 0; i < 500; i++ {
		scripts = append(scripts, rtHTMLScript(r, i))
	}
	for i := 0; i < 300; i++ {
		scripts = append(scripts, rtNSScript(r, i))
	}
	for i := 0; i < 300; i++ {
		scripts = append(scripts, rtTNSScript(r, i))
	}
	layouts, err := rtLayoutScripts(r, 60)
	if err != nil {
		return err
	}
	scripts = append(scripts, layouts...)
	// Keep only scripts whose results do not depend on pointer addresses,
	// and leave out runaway recursions (maxExecDepth: slow, and tested by
	// exec_depth.rs).
	var stable []*script
	for _, s := range scripts {
		out, ok := rtStable(func(bw *bufio.Writer) { _ = rtWriteScripts(bw, []*script{s}) })
		if ok && !bytes.Contains(out, []byte("exceeded maximum template depth")) {
			stable = append(stable, s)
		}
	}
	return rtWriteScripts(w, stable)
}
