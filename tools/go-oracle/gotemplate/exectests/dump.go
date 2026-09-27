//go:build gotemplate_oracle

package template

import (
	"bufio"
	"bytes"
	"errors"
	"fmt"
	"io"
	"reflect"
	"regexp"
	"strconv"
	"strings"

	. "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate" //nolint:staticcheck // ST1001: as in exec_test.go (package template)
)

// dumpCase is one execution: a Go test table entry.
type dumpCase struct {
	group, name string
	input       string
	data        any
	left, right string
	exec        string // ExecuteTemplate(name) instead of Execute
	funcs       FuncMap
	ok          *bool   // Go's expectation (when the table has one)
	output      *string // Go's expected output
}

var addrRe = regexp.MustCompile(`0x[0-9a-f]{6,}`)

// dataID names a test's data value for the Rust mirror.
func dataID(v any) string {
	if v == nil {
		return "nil"
	}
	switch x := v.(type) {
	case *T:
		switch { //nolint:staticcheck // QF1002: pointer identity cases
		case x == tVal:
			return "tVal"
		case x == nil:
			return "(*T)(nil)"
		}
		return "&T{}"
	case T:
		return "T{}"
	case *I:
		return "&iVal"
	case []*T:
		return "tSliceOfNil"
	case map[string]any:
		if _, ok := x["S"]; ok {
			return "stringerMap"
		}
		if _, ok := x["PlusOne"]; ok {
			return "ifaceValues"
		}
	case *map[string]string:
		if x == nil {
			return "(*map[string]string)(nil)"
		}
		return "&map[string]string{}"
	case *Tree:
		return "tree"
	}
	if strings.Contains(fmt.Sprintf("%T", v), "Uthree") {
		return "cmpStruct"
	}
	switch reflect.TypeOf(v).Kind() {
	case reflect.Func:
		return "func:" + fmt.Sprintf("%T", v)
	case reflect.Chan:
		return "chan"
	}
	return fmt.Sprintf("%T|%v", v, v)
}

func q(s string) string { return strconv.Quote(s) }

func bptr(b bool) *bool     { return &b }
func sptr(s string) *string { return &s }
func testFuncs() FuncMap {
	// Go: exec_test.go:testExecute's funcs.
	return FuncMap{
		"add":         add,
		"count":       count,
		"dddArg":      dddArg,
		"die":         func() bool { panic("die") },
		"echo":        echo,
		"makemap":     makemap,
		"mapOfThree":  mapOfThree,
		"oneArg":      oneArg,
		"returnInt":   returnInt,
		"stringer":    stringer,
		"twoArgs":     twoArgs,
		"typeOf":      typeOf,
		"valueString": valueString,
		"vfunc":       vfunc,
		"zeroArgs":    zeroArgs,
	}
}

func run(w *bufio.Writer, i int, c dumpCase) {
	_, _ = fmt.Fprintf(w, "#case %d %s %s\n", i, q(c.group), q(c.name))
	_, _ = fmt.Fprintf(w, "data %s\n", q(dataID(c.data)))
	_, _ = fmt.Fprintf(w, "opts %s %s %s\n", q(c.left), q(c.right), q(c.exec))
	_, _ = fmt.Fprintf(w, "src %s\n", q(c.input))
	if c.ok != nil {
		_, _ = fmt.Fprintf(w, "expect %t %s\n", *c.ok, q(*c.output))
	}
	tmpl := New(c.name)
	if c.funcs != nil {
		tmpl.Funcs(c.funcs)
	}
	tmpl.Delims(c.left, c.right)
	_, err := tmpl.Parse(c.input)
	if err != nil {
		_, _ = fmt.Fprintf(w, "perr %s\n#end\n", q(err.Error()))
		return
	}
	var b bytes.Buffer
	if c.exec != "" {
		err = tmpl.ExecuteTemplate(&b, c.exec, c.data)
	} else {
		err = tmpl.Execute(&b, c.data)
	}
	out := b.String()
	if addrRe.MatchString(out) || (err != nil && addrRe.MatchString(err.Error())) {
		_, _ = fmt.Fprintf(w, "addr\n#end\n")
		return
	}
	_, _ = fmt.Fprintf(w, "out %s\n", q(out))
	if err != nil {
		var ee ExecError
		_, _ = fmt.Fprintf(w, "err %s %t\n", q(err.Error()), errors.As(err, &ee))
	}
	_, _ = fmt.Fprintf(w, "#end\n")
}

// Dump runs Go's exec_test.go tables through the fork.
func Dump(out io.Writer) error {
	w := bufio.NewWriter(out)
	var cs []dumpCase
	for _, t := range execTests {
		cs = append(cs, dumpCase{group: "execTests", name: t.name, input: t.input, data: t.data, funcs: testFuncs(),
			ok: bptr(t.ok), output: sptr(t.output)})
	}
	// TestComparison.
	cmpStruct := struct {
		Uthree, Ufour    uint
		NegOne, Three    int
		Ptr, NilPtr      *int
		NonNilMap        map[int]int
		Map              map[int]int
		V1, V2           V
		Iface1, NilIface fmt.Stringer
	}{
		Uthree:    3,
		Ufour:     4,
		NegOne:    -1,
		Three:     3,
		Ptr:       new(int),
		NonNilMap: make(map[int]int),
		Iface1:    new(strings.Builder),
	}
	for _, t := range cmpTests {
		text := fmt.Sprintf("{{if %s}}true{{else}}false{{end}}", t.expr)
		cs = append(cs, dumpCase{group: "cmpTests", name: "empty", input: text, data: &cmpStruct,
			ok: bptr(t.ok), output: sptr(t.truth)})
	}
	// TestInterfaceValues.
	for _, text := range []string{
		`{{index .Nil 1}}`, `{{index .Slice 2}}`, `{{index .Slice .Two}}`, `{{call .Nil 1}}`, `{{call .PlusOne 1}}`,
		`{{call .PlusOne .One}}`, `{{and (index .Slice 0) true}}`, `{{and .Zero true}}`,
		`{{and (index .Slice 1) false}}`, `{{and .One false}}`, `{{or (index .Slice 0) false}}`, `{{or .Zero false}}`,
		`{{or (index .Slice 1) true}}`, `{{or .One true}}`, `{{not (index .Slice 0)}}`, `{{not .Zero}}`,
		`{{not (index .Slice 1)}}`, `{{not .One}}`, `{{eq (index .Slice 0) .Zero}}`, `{{eq (index .Slice 1) .One}}`,
		`{{ne (index .Slice 0) .Zero}}`, `{{ne (index .Slice 1) .One}}`, `{{ge (index .Slice 0) .One}}`,
		`{{ge (index .Slice 1) .Zero}}`, `{{gt (index .Slice 0) .One}}`, `{{gt (index .Slice 1) .Zero}}`,
		`{{le (index .Slice 0) .One}}`, `{{le (index .Slice 1) .Zero}}`, `{{lt (index .Slice 0) .One}}`,
		`{{lt (index .Slice 1) .Zero}}`,
	} {
		data := map[string]any{
			"PlusOne": func(n int) int { return n + 1 },
			"Slice":   []int{0, 1, 2, 3},
			"One":     1,
			"Two":     2,
			"Nil":     nil,
			"Zero":    0,
		}
		cs = append(cs, dumpCase{group: "TestInterfaceValues", name: "tmpl", input: text, data: data})
	}
	// TestEvalFieldErrors.
	for _, c := range []struct {
		src   string
		value any
	}{
		{"{{.MissingField}}", (*T)(nil)},
		{"{{.MissingField}}", &T{}},
		{"{{.X}}", (*T)(nil)},
		{"{{.MissingKey}}", (*map[string]string)(nil)},
		{"{{.MissingKey}}", &map[string]string{}},
	} {
		cs = append(cs, dumpCase{group: "TestEvalFieldErrors", name: "tmpl", input: c.src, data: c.value})
	}
	// TestExecutePanicDuringCall.
	panicFuncs := FuncMap{"doPanic": func() string { panic("custom panic string") }}
	for _, c := range []struct {
		src  string
		data any
	}{
		{"{{doPanic}}", (*T)(nil)},
		{"{{call doPanic}}", (*T)(nil)},
		{"{{.GetU}}", (*T)(nil)},
		{"{{call .GetU}}", (*T)(nil)},
		{"{{call .PanicFunc}}", tVal},
		{"{{.NonEmptyInterfaceNil.Method0}}", tVal},
	} {
		cs = append(cs, dumpCase{group: "TestExecutePanicDuringCall", name: "t", input: c.src, data: c.data, funcs: panicFuncs})
	}
	// TestFunctionCheckDuringCall.
	for _, c := range []struct {
		src  string
		data any
	}{
		{`{{call}}`, tVal},
		{"{{call .True}}", tVal},
		{"{{call .BinaryFunc 1}}", tVal},
		{`{{call .VariadicFuncInt}}`, tVal},
		{`{{call .TooFewReturnCountFunc}}`, tVal},
		{`{{call .TooManyReturnCountFunc}}`, tVal},
		{`{{call .InvalidReturnTypeFunc}}`, tVal},
		{`{{call (len "test")}}`, nil},
	} {
		cs = append(cs, dumpCase{group: "TestFunctionCheckDuringCall", name: "t", input: c.src, data: c.data})
	}
	// TestDelims.
	delimPairs := []string{"", "", "{{", "}}", "<<", ">>", "|", "|", "(日)", "(本)"}
	for i := 0; i < len(delimPairs); i += 2 {
		left, right := delimPairs[i], delimPairs[i+1]
		trueLeft, trueRight := left, right
		if left == "" {
			trueLeft = "{{"
		}
		if right == "" {
			trueRight = "}}"
		}
		text := trueLeft + ".Str" + trueRight + trueLeft + "/*comment*/" + trueRight + trueLeft + `"` + trueLeft + `"` + trueRight
		cs = append(cs, dumpCase{group: "TestDelims", name: "delims", input: text, data: struct{ Str string }{"Hello, world"},
			left: left, right: right})
	}
	// TestExecError, TestExecuteError, TestTree, TestIssue31810, TestFinalForPrintf.
	cs = append(cs, dumpCase{group: "TestExecError", name: "top", input: execErrorText, data: 5})
	cs = append(cs, dumpCase{group: "TestExecuteError", name: "error", input: "{{.MyError true}}", data: tVal})
	tree := &Tree{1, &Tree{2, &Tree{3, &Tree{4, nil, nil}, nil}, &Tree{5, &Tree{6, nil, nil}, nil}},
		&Tree{7, &Tree{8, &Tree{9, nil, nil}, nil}, &Tree{10, &Tree{11, nil, nil}, nil}}}
	cs = append(cs, dumpCase{group: "TestTree", name: "root", input: treeTemplate, data: tree, left: "(", right: ")", exec: "tree"})
	cs = append(cs, dumpCase{group: "TestIssue31810", name: "", input: "{{ (.)  }}", data: "result"})
	cs = append(cs, dumpCase{group: "TestIssue31810", name: "", input: "{{ (.)  }}", data: func() string { return "result" }})
	cs = append(cs, dumpCase{group: "TestIssue31810", name: "", input: "{{ (call .)  }}", data: func() string { return "result" }})
	cs = append(cs, dumpCase{group: "TestFinalForPrintf", name: "", input: `{{"x" | printf}}`, data: 0})
	cs = append(cs, dumpCase{group: "TestExecuteGivesExecError", name: "X", input: "hello, {{.X.Y}}", data: 0})

	for i, c := range cs {
		run(w, i, c)
	}
	return w.Flush()
}
