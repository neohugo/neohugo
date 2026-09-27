//go:build gotemplate_oracle

package main

// Mode htmlexec: execution-level fixtures for html/template
// (crates/gotemplate/tests/html_exec.rs). Run sync-fork.sh first, then
//
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate htmlexec crates/gotemplate/tests/fixtures/html
//
// Every case is a script of operations on html templates (New, Parse,
// Clone, Lookup, AddParseTree, Funcs, Execute, ...). The oracle runs each
// script against the fork and records every operation's result; the Rust
// test interprets the same scripts with the Rust engine and compares. The
// ported Go tests (htmlexec_gotests.go) also carry the Go tests' own
// expectations, which the oracle checks against the fork (reported on
// stderr; the fixtures always hold what the fork does).
//
// Files (gzip, strings strconv.Quote'd):
//
//	gotests.txt.gz  the ported Go tests
//	corpus.txt.gz   the differential corpus (templates x values, with
//	                Execute and with Hugo's execution path)
//	internal.txt.gz TestEscapeText / TestEnsurePipelineContains / redundantFuncs
//
// Script format:
//
//	D <index> <value spec>              value table (data args "@<index>")
//	S <quoted name>                     script start
//	O <op> <quoted arg>... => <quoted result>...
//	E                                   script end

import (
	"bufio"
	"bytes"
	"context"
	"errors"
	"fmt"
	"html/template"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"sync"

	htmltemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/htmltemplate"
	texttemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate"
	"github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate/parse"
)

// cloneNotFoundRE matches the map-order-dependent name in Clone errors.
var cloneNotFoundRE = regexp.MustCompile(`, "[^"]*" not found$`)

func init() { register("htmlexec", runHTMLExec) }

func runHTMLExec(args []string) error {
	if len(args) != 1 {
		return fmt.Errorf("usage: htmlexec <outdir>")
	}
	dir := args[0]
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	var failed int
	if err := writeGz(filepath.Join(dir, "gotests.txt.gz"), func(w *bufio.Writer) error {
		n, err := writeScripts(w, goTestScripts())
		failed += n
		return err
	}); err != nil {
		return err
	}
	if err := writeGz(filepath.Join(dir, "corpus.txt.gz"), func(w *bufio.Writer) error {
		n, err := writeScripts(w, corpusScripts())
		failed += n
		return err
	}); err != nil {
		return err
	}
	if err := writeGz(filepath.Join(dir, "internal.txt.gz"), writeInternal); err != nil {
		return err
	}
	if failed > 0 {
		fmt.Fprintf(os.Stderr, "htmlexec: %d Go test expectations not met by the fork (see above)\n", failed)
	}
	return nil
}

// ---------------------------------------------------------------------------
// Scripts

// sop is one operation of a script.
type sop struct {
	kind string
	args []string
	// Optional expectation from the ported Go test, checked by the oracle.
	check func(res []string) error
}

type script struct {
	name string
	ops  []*sop
}

func newScript(name string) *script { return &script{name: name} }

// op appends an operation.
func (s *script) op(kind string, args ...string) *script {
	s.ops = append(s.ops, &sop{kind: kind, args: args})
	return s
}

// want sets the expectation of the last operation.
func (s *script) want(check func(res []string) error) *script {
	s.ops[len(s.ops)-1].check = check
	return s
}

// Expectations.

func wantOK() func([]string) error {
	return func(res []string) error {
		if len(res) != 1 || res[0] != "ok" {
			return fmt.Errorf("want ok, got %q", res)
		}
		return nil
	}
}

func wantNotOK() func([]string) error {
	return func(res []string) error {
		if len(res) == 1 && res[0] == "ok" {
			return fmt.Errorf("want an error, got ok")
		}
		return nil
	}
}

// wantOut: exec result (output, error) with no error and this output.
func wantOut(out string) func([]string) error {
	return func(res []string) error {
		if len(res) != 2 || res[0] != out || res[1] != "" {
			return fmt.Errorf("want output %q and no error, got %q", out, res)
		}
		return nil
	}
}

// wantExecErr: exec result with an error containing sub (any error if "").
func wantExecErr(sub string) func([]string) error {
	return func(res []string) error {
		if len(res) != 2 || res[1] == "" || !strings.Contains(res[1], sub) {
			return fmt.Errorf("want error containing %q, got %q", sub, res)
		}
		return nil
	}
}

// wantExecOK: exec result with no error (any output).
func wantExecOK() func([]string) error {
	return func(res []string) error {
		if len(res) != 2 || res[1] != "" {
			return fmt.Errorf("want no error, got %q", res)
		}
		return nil
	}
}

// wantRes: the exact result fields.
func wantRes(fields ...string) func([]string) error {
	return func(res []string) error {
		if !reflect.DeepEqual(res, fields) {
			return fmt.Errorf("want %q, got %q", fields, res)
		}
		return nil
	}
}

// ---------------------------------------------------------------------------
// Interpreter

type interp struct {
	t    map[string]*htmltemplate.Template
	tr   map[string]*parse.Tree
	data []string // value table
}

func (in *interp) tmpl(name string) *htmltemplate.Template {
	t, ok := in.t[name]
	if !ok {
		panic(fmt.Sprintf("undefined template variable %q", name))
	}
	return t
}

// decodeData decodes a data argument ("@N" references the value table).
func (in *interp) decodeData(arg string) any {
	spec := arg
	if strings.HasPrefix(arg, "@") {
		i, err := strconv.Atoi(arg[1:])
		if err != nil {
			panic(err)
		}
		spec = in.data[i]
	}
	v, err := decodeSpec(spec)
	if err != nil {
		panic(fmt.Sprintf("%s: %v", spec, err))
	}
	return v
}

func errString(err error) string {
	if err == nil {
		return ""
	}
	return err.Error()
}

func okOrErr(err error) []string {
	if err != nil {
		return []string{"err", err.Error()}
	}
	return []string{"ok"}
}

type errorWriter struct{}

var errAlways = errors.New("always be failing")

func (errorWriter) Write(p []byte) (int, error) { return 0, errAlways }

// run executes one operation and returns its result fields.
func (in *interp) run(o *sop) (res []string) {
	defer func() {
		if r := recover(); r != nil {
			res = []string{"PANIC", fmt.Sprint(r)}
		}
	}()
	a := o.args
	switch o.kind {
	case "new": // V NAME
		in.t[a[0]] = htmltemplate.New(a[1])
		return nil
	case "newassoc": // V FROM NAME
		in.t[a[0]] = in.tmpl(a[1]).New(a[2])
		return nil
	case "parse": // V SRC
		_, err := in.tmpl(a[0]).Parse(a[1])
		return okOrErr(err)
	case "clone", "cloneshallow": // V FROM
		var c *htmltemplate.Template
		var err error
		if o.kind == "clone" {
			c, err = in.tmpl(a[1]).Clone()
		} else {
			c, err = in.tmpl(a[1]).CloneShallow()
		}
		if err == nil {
			in.t[a[0]] = c
		}
		// Clone's "..., %q not found" names whichever template Go's map
		// iteration visits first; record a fixed name so the fixture is
		// reproducible (the Rust test compares only error vs no error).
		if err != nil {
			err = errors.New(cloneNotFoundRE.ReplaceAllString(err.Error(), `, "<map order>" not found`))
		}
		return okOrErr(err)
	case "lookup": // V FROM NAME
		x := in.tmpl(a[1]).Lookup(a[2])
		if x == nil {
			return []string{"nil"}
		}
		in.t[a[0]] = x
		return []string{"ok"}
	case "funcs": // V SET [ARG]
		in.tmpl(a[0]).Funcs(in.funcSet(a[1:]))
		return nil
	case "delims": // V L R
		in.tmpl(a[0]).Delims(a[1], a[2])
		return nil
	case "option": // V OPT
		in.tmpl(a[0]).Option(a[1])
		return nil
	case "exec", "exectmpl", "execerrw": // V [NAME] DATA
		var b bytes.Buffer
		var err error
		switch o.kind {
		case "exec":
			err = in.tmpl(a[0]).Execute(&b, in.decodeData(a[1]))
		case "exectmpl":
			err = in.tmpl(a[0]).ExecuteTemplate(&b, a[1], in.decodeData(a[2]))
		default:
			err = in.tmpl(a[0]).Execute(errorWriter{}, in.decodeData(a[1]))
		}
		return []string{b.String(), errString(err)}
	case "exechugo": // V DATA: Hugo's execution path (see hugoHelper)
		var b bytes.Buffer
		err := texttemplate.NewExecuter(hugoExecHelper).ExecuteWithContext(context.Background(), in.tmpl(a[0]), &b, in.decodeData(a[1]))
		return []string{b.String(), errString(err)}
	case "execrecur": // V TARGET: data is a recursiveInvoker for TARGET
		var b bytes.Buffer
		err := in.tmpl(a[0]).Execute(&b, &recursiveInvoker{tmpl: in.tmpl(a[1])})
		return []string{b.String(), errString(err)}
	case "execpar": // V DATA N: N concurrent executions (race tests)
		n, _ := strconv.Atoi(a[2])
		outs := make([]string, n)
		errs := make([]string, n)
		var wg sync.WaitGroup
		for i := 0; i < n; i++ {
			wg.Add(1)
			go func(i int) {
				defer wg.Done()
				var b bytes.Buffer
				errs[i] = errString(in.tmpl(a[0]).Execute(&b, in.decodeData(a[1])))
				outs[i] = b.String()
			}(i)
		}
		wg.Wait()
		for i := 1; i < n; i++ {
			if outs[i] != outs[0] || errs[i] != errs[0] {
				return []string{"inconsistent"}
			}
		}
		return []string{outs[0], errs[0]}
	case "templates": // V: the sorted names
		var names []string
		for _, t := range in.tmpl(a[0]).Templates() {
			names = append(names, t.Name())
		}
		sort.Strings(names)
		return append([]string{strconv.Itoa(len(names))}, names...)
	case "name": // V
		return []string{in.tmpl(a[0]).Name()}
	case "ptreq": // A B
		return []string{strconv.FormatBool(in.tmpl(a[0]) == in.tmpl(a[1]))}
	case "treesync": // V: t.Tree == t.text.Tree
		t := in.tmpl(a[0])
		return []string{strconv.FormatBool(t.Tree == t.TextTemplate().Tree)}
	case "hastree": // V
		return []string{strconv.FormatBool(in.tmpl(a[0]).Tree != nil)}
	case "parsetree": // TR PNAME SRC PICK [comments]
		if len(a) > 4 && a[4] == "comments" {
			tr := parse.New(a[1])
			tr.Mode = parse.ParseComments
			t, err := tr.Parse(a[2], "", "", make(map[string]*parse.Tree))
			if err == nil {
				in.tr[a[0]] = t
			}
			return okOrErr(err)
		}
		trees, err := parse.Parse(a[1], a[2], "", "", nil)
		if err == nil {
			in.tr[a[0]] = trees[a[3]]
		}
		return okOrErr(err)
	case "addtree": // V FROM NAME TREEREF (tree:TR | of:V2)
		var tree *parse.Tree
		switch {
		case strings.HasPrefix(a[3], "tree:"):
			tree = in.tr[a[3][5:]]
		case strings.HasPrefix(a[3], "of:"):
			tree = in.tmpl(a[3][3:]).Tree
		default:
			panic("bad tree ref " + a[3])
		}
		x, err := in.tmpl(a[1]).AddParseTree(a[2], tree)
		if err == nil {
			in.t[a[0]] = x
		}
		return okOrErr(err)
	}
	panic("unknown op " + o.kind)
}

// funcSet returns a named FuncMap (the Rust test has the same sets).
func (in *interp) funcSet(a []string) htmltemplate.FuncMap {
	switch a[0] {
	case "exec":
		return execFuncs
	case "pred":
		// pred is a template function that returns the predecessor of a
		// natural number for testing recursive templates.
		return htmltemplate.FuncMap{"pred": func(a ...any) (any, error) {
			if len(a) == 1 {
				if i, _ := a[0].(int); i > 0 {
					return i - 1, nil
				}
			}
			return nil, fmt.Errorf("undefined pred(%v)", a)
		}}
	case "issue5980":
		return htmltemplate.FuncMap{"customFunc": func() (string, error) {
			return "", errors.New("issue5980")
		}}
	case "panic":
		return htmltemplate.FuncMap{"doPanic": func() string {
			panic("custom panic string")
		}}
	case "identity":
		return htmltemplate.FuncMap{"f": func(in string) string {
			return in
		}}
	case "recur": // ARG: the template variable to execute "subroutine" of
		tmpl := in.tmpl(a[1])
		return htmltemplate.FuncMap{"recur": func() (template.HTML, error) {
			var sb strings.Builder
			if err := tmpl.ExecuteTemplate(&sb, "subroutine", nil); err != nil {
				return "", err
			}
			return template.HTML(sb.String()), nil
		}}
	case "badname": // ARG: a function name (Funcs panics on a bad one)
		return htmltemplate.FuncMap{a[1]: func() int { return 0 }}
	case "corpus":
		return corpusFuncs
	}
	panic("unknown func set " + a[0])
}

// writeScripts runs the scripts and writes them with their results. It
// returns the number of unmet Go test expectations.
func writeScripts(w *bufio.Writer, scripts []*script, extraData ...string) (int, error) {
	// Collect the data table: every data argument that is a spec.
	index := map[string]int{}
	var table []string
	for _, s := range scripts {
		for _, o := range s.ops {
			for i, a := range o.args {
				if !isDataArg(o.kind, i) {
					continue
				}
				if _, ok := index[a]; !ok {
					index[a] = len(table)
					table = append(table, a)
				}
			}
		}
	}
	fmt.Fprintf(w, "# gotemplate htmlexec fixture: D <i> <spec> / S <name> / O <op> <args> => <results> / E\n")
	for i, spec := range table {
		fmt.Fprintf(w, "D\t%d\t%s\n", i, spec)
	}
	failed := 0
	for _, s := range scripts {
		in := &interp{t: map[string]*htmltemplate.Template{}, tr: map[string]*parse.Tree{}, data: table}
		fmt.Fprintf(w, "S\t%s\n", q(s.name))
		for _, o := range s.ops {
			args := make([]string, len(o.args))
			for i, a := range o.args {
				if isDataArg(o.kind, i) {
					args[i] = "@" + strconv.Itoa(index[a])
				} else {
					args[i] = a
				}
			}
			res := in.run(&sop{kind: o.kind, args: args})
			if o.check != nil {
				if err := o.check(res); err != nil {
					failed++
					fmt.Fprintf(os.Stderr, "%s: %s %q: %v\n", s.name, o.kind, o.args, err)
				}
			}
			fmt.Fprintf(w, "O\t%s", o.kind)
			for _, a := range args {
				fmt.Fprintf(w, "\t%s", q(a))
			}
			fmt.Fprintf(w, "\t=>")
			for _, r := range res {
				fmt.Fprintf(w, "\t%s", q(r))
			}
			fmt.Fprintln(w)
		}
		fmt.Fprintln(w, "E")
	}
	return failed, nil
}

// isDataArg reports whether argument i of an operation is a data spec.
func isDataArg(kind string, i int) bool {
	switch kind {
	case "exec", "execerrw", "execpar", "exechugo":
		return i == 1
	case "exectmpl":
		return i == 2
	}
	return false
}

// ---------------------------------------------------------------------------
// Go values -> value specs (for the tables copied from the Go tests)

var (
	tVal        = newTVal()
	tSliceOfNil = []*T{nil}
)

func sStruct(ptr bool, fields ...string) string {
	name := "st"
	if ptr {
		name = "pst"
	}
	return name + "(" + strings.Join(fields, ",") + ")"
}

func sField(name, spec string) string    { return "f:" + hexs(name) + "(" + spec + ")" }
func sFieldAny(name, spec string) string { return "fa:" + hexs(name) + "(" + spec + ")" }

// specOf converts the data values of the copied Go test tables to value
// specs. Complex numbers (which the Rust value model cannot represent)
// become cplx nodes, which the Rust test skips.
func specOf(v any) string {
	switch v := v.(type) {
	case nil:
		return "nil"
	case bool:
		if v {
			return "bool:1"
		}
		return "bool:0"
	case int:
		return sInt("int", int64(v))
	case int32:
		return sInt("int32", int64(v))
	case uint:
		return sUint("uint", uint64(v))
	case float64:
		return sF64(v)
	case string:
		return sStr(v)
	case complex128:
		return fmt.Sprintf("cplx:%016x;%016x", math.Float64bits(real(v)), math.Float64bits(imag(v)))
	case []int:
		var es []string
		for _, x := range v {
			es = append(es, sInt("int", int64(x)))
		}
		return sList("[]int", es...)
	case map[string]int:
		var kvs []string
		for k, x := range v {
			kvs = append(kvs, k, sInt("int", int64(x)))
		}
		return sMap("map[string]int", kvs...)
	case map[string]string:
		var kvs []string
		for k, x := range v {
			kvs = append(kvs, k, sStr(x))
		}
		return sMap("map[string]string", kvs...)
	case map[string]any:
		var kvs []string
		for k, x := range v {
			kvs = append(kvs, k, specOf(x))
		}
		return sMap("map[string]interface {}", kvs...)
	case *bytes.Buffer:
		return "bufstr:" + hexs(v.String())
	case *T:
		if v == tVal {
			return "tval"
		}
	case T:
		if reflect.ValueOf(v).IsZero() {
			return "tzero"
		}
	case []*T:
		if len(v) == 1 && v[0] == nil {
			return sList("[]*main.T", sTnil("*main.T"))
		}
	case *I:
		if *v == I(tVal) {
			return "ivalptr"
		}
	}
	// exec_test.go's `struct{ a int; b string }{7, "seven"}` (unexported
	// fields: reflect.StructOf cannot build it, so it is a named type here).
	if rv := reflect.ValueOf(v); rv.Kind() == reflect.Struct && rv.Type().String() == "struct { a int; b string }" {
		if rv.Field(0).Int() == 7 && rv.Field(1).String() == "seven" {
			return "seven"
		}
	}
	panic(fmt.Sprintf("specOf: unsupported %T %v", v, v))
}

// ---------------------------------------------------------------------------
// internal.txt.gz: TestEscapeText, TestEnsurePipelineContains, redundantFuncs

func writeInternal(w *bufio.Writer) error {
	fmt.Fprintf(w, "# gotemplate htmlexec internal fixture\n")
	fmt.Fprintf(w, "# X <text> <context> <unmodified> / P <src> <ids> <pipeline or error> / F <a> <b>\n")
	for _, tc := range escapeTextTests() {
		ctx, same := htmltemplate.EscapeTextExported(tc.input)
		if tc.want != "" && ctx != tc.want {
			fmt.Fprintf(os.Stderr, "TestEscapeText %q: want %s got %s\n", tc.input, tc.want, ctx)
		}
		fmt.Fprintf(w, "X\t%s\t%s\t%v\n", q(tc.input), q(ctx), same)
	}
	for _, tc := range ensurePipelineTests() {
		got, err := htmltemplate.EnsurePipelineContainsExported(tc.input, tc.ids)
		if err != nil {
			got = "ERR " + err.Error()
		} else if got != tc.output {
			fmt.Fprintf(os.Stderr, "TestEnsurePipelineContains %q %q: want %s got %s\n", tc.input, tc.ids, tc.output, got)
		}
		qs := make([]string, len(tc.ids))
		for i, id := range tc.ids {
			qs[i] = q(id)
		}
		fmt.Fprintf(w, "P\t%s\t%s\t%s\n", q(tc.input), strings.Join(qs, ","), q(got))
	}
	var pairs []string
	for a, m := range htmltemplate.RedundantFuncsExported() {
		for b := range m {
			pairs = append(pairs, a+"\t"+b)
		}
	}
	sort.Strings(pairs)
	for _, p := range pairs {
		fmt.Fprintf(w, "F\t%s\n", p)
	}
	return nil
}
