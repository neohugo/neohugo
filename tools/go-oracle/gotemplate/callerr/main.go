// Command callerr is a small hermetic Go oracle for crates/gotemplate: which
// errors of a function or method call text/template reports with the
// "error calling X: " prefix. Go's evalCall checks the argument count, the
// argument types (validateType) and the result count (goodFunc) itself, before
// the call, and reports those without the prefix; only an error the callee
// returns is wrapped ("error calling %s: %w"), even when its text looks like
// an argument-count error.
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/gotemplate/callerr > crates/gotemplate/tests/fixtures/callerr.tsv
//
// The Rust test (crates/gotemplate/tests/callerr.rs) hosts the same methods
// and functions, which report the pre-call errors with Go's texts (as the
// Hugo layer's argument helpers do), and must print the same errors.
//
// The forked text/template (tpl/internal/go_templates/texttemplate) cannot be
// imported from tools/; its evalCall and validateType messages are the
// standard library's. Its goodFunc text differs (the fork's is "can't call
// method/function %q with %d results"), so a bad result count is only called
// with a wrong argument count here, which evalCall reports first.
//
// Output: one line per case, "src<TAB>output<TAB>error" (no case contains a
// tab or a newline).
package main

import (
	"errors"
	"fmt"
	"strings"
	"text/template"
)

// T is the data: its methods cover each call-error path.
type T struct{}

func (T) M0() string                  { return "m0" }
func (T) M1(s string) string          { return "m1:" + s }
func (T) M2(a, b int) int             { return a + b }
func (T) V(s string, r ...string) int { return len(r) }
func (T) Err() (string, error)        { return "", errors.New("boom") }
func (T) ErrLike() (string, error) {
	return "", errors.New("wrong number of args for ErrLike: want 9 got 9")
}
func (T) Bad() (int, int)               { return 1, 2 }
func (T) Arg1Err(s string) (int, error) { return 0, fmt.Errorf("bad %s", s) }

func main() {
	funcs := template.FuncMap{
		"f0":   func() string { return "f0" },
		"f1":   func(s string) string { return s },
		"fv":   func(s string, r ...string) int { return len(r) },
		"ferr": func() (string, error) { return "", errors.New("ferr failed") },
	}
	srcs := []string{
		"{{.M0}}", "{{.M0 1}}", "{{.M0 1 2}}", "{{.M1}}", "{{.M1 \"a\"}}", "{{.M1 \"a\" \"b\"}}",
		"{{$x := 1}}{{.M1 $x}}", "{{.M2 1}}", "{{.M2 1 2 3}}", "{{.V}}", "{{.V \"a\"}}", "{{.V \"a\" \"b\"}}",
		"{{.Err}}", "{{.ErrLike}}", "{{.Bad 1}}", "{{.Arg1Err \"x\"}}", "{{.Arg1Err}}",
		"{{f0}}", "{{f0 1}}", "{{f1}}", "{{f1 \"a\" \"b\"}}", "{{$x := 2}}{{f1 $x}}", "{{fv}}", "{{fv \"a\"}}",
		"{{ferr}}", "{{\"a\" | f1}}", "{{\"a\" | f0}}", "{{\"a\" | .M0}}", "{{\"a\" | .M1}}",
		"{{\"a\" | .M1 \"b\"}}", "{{1 | .M2 1}}", "{{$x := 3}}{{$x | .M1}}", "{{$y := 4}}{{\"a\" | .V $y}}",
	}
	for _, src := range srcs {
		t := template.Must(template.New("t").Funcs(funcs).Parse(src))
		var sb strings.Builder
		err := t.Execute(&sb, T{})
		msg := ""
		if err != nil {
			msg = err.Error()
		}
		fmt.Printf("%s\t%s\t%s\n", src, sb.String(), msg)
	}
}
