//go:build gotemplate_oracle

// Mode "exec": a Hugo-like harness for the forked text/template. A data
// model covering the host contract clauses C2-C12 (crates/GOTEMPLATE_CONTRACT.md),
// a func map, and an ExecHelper with Hugo's semantics (templateExecHelper in
// tpl/tplimpl/template_funcs.go: case-insensitive maps.Params, methods before
// keys, the "mainsections" special case) are mirrored exactly by
// crates/gotemplate/tests/common_text/model.rs; every case is executed in
// two modes, "plain" (Template.Execute, Go's reflection lookups) and "hugo"
// (Executer.ExecuteWithContext with the helper), and the output bytes and
// error text are dumped.
//
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate exec crates/gotemplate/tests/fixtures/text
//
// Output: <outdir>/exec.txt.gz. Record format (strings strconv.Quote'd):
//
//	#case <n> <name>
//	mode <plain|hugo> <data> <option> <exec>   (exec: "" = Execute, else ExecuteTemplate(name))
//	src <source>
//	perr <message>                     (parse error), or
//	out <output>
//	err <message> <ExecError?> <cause>  (when execution failed)
//	#end
//
// Outputs that print a Go pointer address (non-deterministic) are recorded
// as `addr`.
package main

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"context"
	"errors"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"strings"
	"time"

	"github.com/neohugo/neohugo/common/herrors"
	"github.com/neohugo/neohugo/common/hreflect"
	"github.com/neohugo/neohugo/common/maps"
	texttemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate"
	htmltemplate "html/template"
)

func init() {
	register("exec", etxMain)
}

// ---------------------------------------------------------------------------
// Data model (mirrored by crates/gotemplate/tests/common_text/model.rs).

// EtxT is a struct with fields and methods with pointer and value receivers.
type EtxT struct {
	Name     string
	N        int
	F        float64
	B        bool
	Sub      *EtxT
	NilSub   *EtxT
	V        EtxV
	PV       *EtxV
	Iface    any
	NilIface any
	Err      error
	NilErr   error
	Str      fmt.Stringer
	NilStr   fmt.Stringer
	M        map[string]any
	NilM     map[string]any
	SS       []string
	NilSS    []string
	L        []any
	Fn       func(any) any
	NilFn    func(any) any
	Pages    EtxPages
	Time     time.Time
	H        htmltemplate.HTML
}

// VM has a value receiver.
func (t EtxT) VM() string { return "VM:" + t.Name }

// PM has a pointer receiver.
func (t *EtxT) PM() string { return "PM:" + t.Name }

func (t *EtxT) Echo(v any) any { return v }

func (t *EtxT) Two(a, b any) string { return fmt.Sprint(a, "|", b) }

func (t *EtxT) Var(args ...any) string { return fmt.Sprintf("%d%v", len(args), args) }

func (t *EtxT) Fail() (string, error) { return "", errors.New("method failed") }

func (t *EtxT) NilOK() string {
	if t == nil {
		return "nil receiver"
	}
	return "non-nil:" + t.Name
}

func (t *EtxT) Self() *EtxT { return t }

func (t *EtxT) GetSub() *EtxT { return t.Sub }

func (t *EtxT) RetNil() any { return nil }

func (t *EtxT) RetNilPtr() *EtxT { return nil }

func (t *EtxT) RetNilStr() fmt.Stringer { return nil }

func (t *EtxT) RetNilMap() map[string]any { return nil }

func (t *EtxT) RetM() map[string]any { return t.M }

func (t *EtxT) Panic() string { panic("method panic") }

// EtxV is a small struct used by value (addressable or not).
type EtxV struct {
	A string
	B int
}

// VV has a value receiver.
func (v EtxV) VV() string { return "VV:" + v.A }

// PV2 has a pointer receiver.
func (v *EtxV) PV2() string { return "PV:" + v.A }

// EtxPages is a named slice type with methods.
type EtxPages []*EtxT

func (p EtxPages) Len() int { return len(p) }

func (p EtxPages) First() *EtxT {
	if len(p) == 0 {
		return nil
	}
	return p[0]
}

func (p EtxPages) Reverse() EtxPages {
	r := make(EtxPages, len(p))
	for i, x := range p {
		r[len(p)-1-i] = x
	}
	return r
}

// EtxData is a named map type with methods (like page.Data): methods win
// over keys.
type EtxData map[string]any

func (d EtxData) Pages() string { return "method-pages" }

func (d EtxData) Count() int { return len(d) }

// EtxZ implements types.Zeroer.
type EtxZ struct {
	Zero  bool
	Label string
}

func (z EtxZ) IsZero() bool { return z.Zero }

// EtxStr is a pointer-receiver fmt.Stringer.
type EtxStr struct{ S string }

func (s *EtxStr) String() string { return "Str(" + s.S + ")" }

// EtxSV is a value-receiver fmt.Stringer.
type EtxSV struct{ S string }

func (s EtxSV) String() string { return "SV(" + s.S + ")" }

// EtxErr is a pointer-receiver error.
type EtxErr struct{ Msg string }

func (e *EtxErr) Error() string { return "Err(" + e.Msg + ")" }

// EtxSite owns MainSections (the helper's "mainsections" special case).
type EtxSite struct{ Title string }

func (s *EtxSite) MainSections() []string { return []string{"posts", "blog"} }

// etxRoot builds the root data (fresh on every call).
func etxRoot() (map[string]any, maps.Params, *EtxSite) {
	sub := &EtxT{Name: "sub", N: 2, V: EtxV{"sv", 3}}
	t := &EtxT{
		Name:  "t",
		N:     1,
		F:     1.5,
		B:     true,
		Sub:   sub,
		V:     EtxV{"v", 7},
		PV:    &EtxV{"pv", 8},
		Iface: 42,
		Err:   &EtxErr{"field"},
		Str:   &EtxStr{"field"},
		M:     map[string]any{"a": 1, "b": "two", "nil": nil, "Key": "K", "key": "k"},
		SS:    []string{"x", "y"},
		L:     []any{1, "two", nil},
		Fn:    func(v any) any { return fmt.Sprint("fn:", v) },
		Pages: EtxPages{sub},
		Time:  time.Date(2020, 5, 6, 7, 8, 9, 10, time.UTC),
		H:     "<b>h</b>",
	}
	siteParams := maps.Params{
		"title":        "Site",
		"mainsections": []string{"from-params"},
		"social":       maps.Params{"facebook": maps.Params{"enable": true}},
	}
	site := &EtxSite{Title: "Site"}
	root := map[string]any{
		"T":          t,
		"TV":         EtxT{Name: "tv", V: EtxV{"tvv", 1}},
		"NilT":       (*EtxT)(nil),
		"Nil":        nil,
		"V":          EtxV{"val", 9},
		"PVal":       &EtxV{"ptr", 10},
		"Pages":      EtxPages{t, sub},
		"EmptyPages": EtxPages{},
		"NilPages":   EtxPages(nil),
		"D":          EtxData{"Singular": "s", "Pages": "key-pages", "Count": "key-count"},
		"P": maps.Params{
			"title":      "Title",
			"ymap":       maps.Params{"b": 2, "c": maps.Params{"d": "deep"}},
			"ynull":      nil,
			"list":       []any{"a", 1},
			"mixed_case": "mc",
			"iszero":     "key-iszero",
		},
		"PMerge":     maps.Params{"_merge": "deep"},
		"PEmpty":     maps.Params{},
		"SiteParams": siteParams,
		"Site":       site,
		"M": map[string]any{
			"a": 1, "b": "two", "nil": nil, "Key": "K", "key": "k",
			"sub":   map[string]any{"x": "X", "nil": nil},
			"list":  []any{1, 2, 3},
			"v":     EtxV{"inmap", 5},
			"empty": map[string]any{},
		},
		"MS":       map[string]string{"a": "A", "b": "B"},
		"L":        []any{1, "two", nil, 3.5, true, []any{"n"}, map[string]any{"k": "v"}},
		"SS":       []string{"a", "b", "c"},
		"SI":       []int{3, 4, 5},
		"SB":       []bool{true, false},
		"SF":       []float64{1.5, 2},
		"SBytes":   []byte("hi"),
		"SM":       []map[string]any{{"a": 1}, {"a": 2}},
		"Empty":    []any{},
		"EmptySS":  []string{},
		"EmptyM":   map[string]any{},
		"NilM":     map[string]any(nil),
		"NilSS":    []string(nil),
		"I":        42,
		"I8":       int8(-8),
		"I16":      int16(-16),
		"I32":      int32(-32),
		"I64":      int64(-64),
		"U":        uint(7),
		"U8":       uint8(255),
		"U16":      uint16(16),
		"U32":      uint32(32),
		"U64":      uint64(math.MaxUint64),
		"UP":       uintptr(9),
		"F32":      float32(1.5),
		"F64":      2.5,
		"FBig":     1e21,
		"FSmall":   1e-7,
		"MaxI":     math.MaxInt64,
		"MinI":     math.MinInt64,
		"Neg":      -3,
		"Zero":     0,
		"ZeroU":    uint(0),
		"ZeroF":    0.0,
		"NegZero":  math.Copysign(0, -1),
		"NaN":      math.NaN(),
		"Inf":      math.Inf(1),
		"S":        "hello",
		"ES":       "",
		"Bad":      "a\xffb",
		"Uni":      "héllo wörld",
		"HTML":     htmltemplate.HTML("<b>x</b>"),
		"EHTML":    htmltemplate.HTML(""),
		"JS":       htmltemplate.JS("x<y"),
		"True":     true,
		"False":    false,
		"Time":     time.Date(2021, 1, 2, 3, 4, 5, 6, time.UTC),
		"ZeroTime": time.Time{},
		"Z":        EtxZ{Zero: true, Label: "z"},
		"NZ":       EtxZ{Zero: false, Label: "nz"},
		"Str":      &EtxStr{"s"},
		"SV":       EtxSV{"v"},
		"Err":      &EtxErr{"e"},
		"Fn":       func(v any) any { return fmt.Sprint("fn:", v) },
		"Fn2":      func(a, b any) (any, error) { return nil, errors.New("fn2 failed") },
		"NilFn":    (func(any) any)(nil),
		"Nested":   map[string]any{"a": map[string]any{"b": map[string]any{"c": "abc"}}},
		"NumKeys":  map[string]any{"1": "one", "10": "ten", "2": "two", "A": "a", "a": "lower", "_": "u", "é": "e"},
	}
	return root, siteParams, site
}

// etxData returns the data value for a case.
func etxData(kind string) (any, maps.Params, *EtxSite) {
	root, sp, site := etxRoot()
	switch kind {
	case "root":
		return root, sp, site
	case "nil":
		return nil, sp, site
	case "int":
		return 7, sp, site
	case "string":
		return "str", sp, site
	case "typednil":
		return (*EtxT)(nil), sp, site
	case "t":
		return root["T"], sp, site
	case "list":
		return root["L"], sp, site
	case "params":
		return root["P"], sp, site
	case "siteparams":
		return sp, sp, site
	}
	panic("unknown data " + kind)
}

// ---------------------------------------------------------------------------
// Functions.

func etxFuncs() map[string]any {
	return map[string]any{
		"echo":     func(v any) any { return v },
		"echo2":    func(a, b any) string { return fmt.Sprint(a, "|", b) },
		"fail":     func(msg any) (any, error) { return nil, errors.New(fmt.Sprint(msg)) },
		"failNil":  func() (any, error) { return nil, nil },
		"failErr":  func() (any, error) { return nil, &EtxErr{"typed"} },
		"nilany":   func() any { return nil },
		"nilptr":   func() *EtxT { return nil },
		"nilerr":   func() error { return nil },
		"nilstr":   func() fmt.Stringer { return nil },
		"nilmap":   func() map[string]any { return nil },
		"nilslice": func() []string { return nil },
		"mkhtml":   func(v any) htmltemplate.HTML { return htmltemplate.HTML(fmt.Sprint(v)) },
		"try":      func(v any) (any, error) { return v, nil },
		"typeof":   func(v any) string { return fmt.Sprintf("%T", v) },
		"isnil":    func(v any) bool { return v == nil },
		"variadic": func(args ...any) string { return fmt.Sprintf("%d:%v", len(args), args) },
		"panicky":  func() any { panic("host panic") },
		"panicerr": func() any { panic(errors.New("host panic err")) },
		"errval":   func() error { return errors.New("an error value") },
		"dict": func(kv ...any) (map[string]any, error) {
			if len(kv)%2 != 0 {
				return nil, errors.New("invalid dictionary call")
			}
			m := map[string]any{}
			for i := 0; i < len(kv); i += 2 {
				k, ok := kv[i].(string)
				if !ok {
					return nil, errors.New("dictionary keys must be strings")
				}
				m[k] = kv[i+1]
			}
			return m, nil
		},
		"list":   func(v ...any) []any { return v },
		"mkT":    func(name any) *EtxT { return &EtxT{Name: fmt.Sprint(name)} },
		"getfn":  func() func(any) any { return func(v any) any { return fmt.Sprint("got:", v) } },
		"strarg": func(s string) string { return "s:" + s },
		"intarg": func(i int) int { return i * 2 },
	}
}

// ---------------------------------------------------------------------------
// The Hugo-like ExecHelper (tpl/tplimpl/template_funcs.go:templateExecHelper
// without dependency tracking and context injection).

type etxHelper struct {
	site       reflect.Value
	siteParams reflect.Value
	funcs      map[string]reflect.Value
}

var etxZero reflect.Value

var etxTypeParams = reflect.TypeOf(maps.Params{})

func (t *etxHelper) Init(ctx context.Context, tmpl texttemplate.Preparer) {}

func (t *etxHelper) GetFunc(ctx context.Context, tmpl texttemplate.Preparer, name string) (reflect.Value, reflect.Value, bool) {
	if fn, found := t.funcs[name]; found {
		return fn, etxZero, true
	}
	return etxZero, etxZero, false
}

func (t *etxHelper) GetMapValue(ctx context.Context, tmpl texttemplate.Preparer, receiver, key reflect.Value) (reflect.Value, bool) {
	if params, ok := receiver.Interface().(maps.Params); ok {
		// Case insensitive.
		keystr := strings.ToLower(key.String())
		v, found := params[keystr]
		if !found {
			return etxZero, false
		}
		return reflect.ValueOf(v), true
	}
	v := receiver.MapIndex(key)
	return v, v.IsValid()
}

func (t *etxHelper) GetMethod(ctx context.Context, tmpl texttemplate.Preparer, receiver reflect.Value, name string) (method reflect.Value, firstArg reflect.Value) {
	if strings.EqualFold(name, "mainsections") && receiver.Type() == etxTypeParams && receiver.Pointer() == t.siteParams.Pointer() {
		receiver = t.site
		name = "MainSections"
	}
	fn := hreflect.GetMethodByName(receiver, name)
	if !fn.IsValid() {
		return etxZero, etxZero
	}
	return fn, etxZero
}

func (t *etxHelper) OnCalled(ctx context.Context, tmpl texttemplate.Preparer, name string, args []reflect.Value, result reflect.Value) {
}

func etxNewHelper(site *EtxSite, siteParams maps.Params) *etxHelper {
	h := &etxHelper{
		site:       reflect.ValueOf(site),
		siteParams: reflect.ValueOf(siteParams),
		funcs:      map[string]reflect.Value{},
	}
	for k, v := range etxFuncs() {
		h.funcs[k] = reflect.ValueOf(v)
	}
	// As Hugo's configureSiteStorage: the builtins only if not present.
	for k, v := range texttemplate.GoFuncs {
		if _, found := h.funcs[k]; !found {
			h.funcs[k] = v
		}
	}
	return h
}

// ---------------------------------------------------------------------------
// Running.

type etxCase struct {
	name string
	src  string
	data string
	opt  string
	exec string
	mode string // "both", "plain", "hugo"
}

var etxAddrRe = regexp.MustCompile(`0x[0-9a-f]{6,}`)

func etxRun(w *bufio.Writer, i int, c etxCase, mode string) {
	_, _ = fmt.Fprintf(w, "#case %d %s\n", i, q(c.name))
	_, _ = fmt.Fprintf(w, "mode %s %s %s %s\n", mode, c.data, q(c.opt), q(c.exec))
	_, _ = fmt.Fprintf(w, "src %s\n", q(c.src))
	data, sp, site := etxData(c.data)
	tmpl := texttemplate.New("t")
	if c.opt != "" {
		tmpl.Option(c.opt)
	}
	tmpl.Funcs(etxFuncs())
	var err error
	if c.src != "\x00noparse" {
		_, err = tmpl.Parse(c.src)
	}
	if err != nil {
		_, _ = fmt.Fprintf(w, "perr %s\n#end\n", q(err.Error()))
		return
	}
	var buf bytes.Buffer
	switch mode {
	case "plain":
		if c.exec != "" {
			err = tmpl.ExecuteTemplate(&buf, c.exec, data)
		} else {
			err = tmpl.Execute(&buf, data)
		}
	case "hugo":
		p := tmpl
		if c.exec != "" {
			p = tmpl.Lookup(c.exec)
		}
		err = texttemplate.NewExecuter(etxNewHelper(site, sp)).ExecuteWithContext(context.Background(), p, &buf, data)
	}
	out := buf.String()
	if etxAddrRe.MatchString(out) || (err != nil && etxAddrRe.MatchString(err.Error())) {
		_, _ = fmt.Fprintf(w, "addr\n#end\n")
		return
	}
	_, _ = fmt.Fprintf(w, "out %s\n", q(out))
	if err != nil {
		var ee texttemplate.ExecError
		isExec := errors.As(err, &ee)
		_, _ = fmt.Fprintf(w, "err %s %t %s\n", q(err.Error()), isExec, q(herrors.Cause(err).Error()))
	}
	_, _ = fmt.Fprintf(w, "#end\n")
}

func etxMain(args []string) error {
	if len(args) != 1 {
		return fmt.Errorf("usage: exec <outdir>")
	}
	f, err := os.Create(filepath.Join(args[0], "exec.txt.gz"))
	if err != nil {
		return err
	}
	defer func() { _ = f.Close() }()
	zw, _ := gzip.NewWriterLevel(f, gzip.BestCompression)
	w := bufio.NewWriter(zw)
	n := 0
	for _, c := range etxCorpus() {
		for _, mode := range []string{"plain", "hugo"} {
			if c.mode != "both" && c.mode != mode {
				continue
			}
			etxRun(w, n, c, mode)
			n++
		}
	}
	if err := w.Flush(); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	fmt.Fprintf(os.Stderr, "exec: %d cases\n", n)
	return nil
}

// ---------------------------------------------------------------------------
// Corpus.

// etxValues are value expressions combined with every shape in etxShapes.
var etxValues = []string{
	".I", ".I8", ".I16", ".I32", ".I64", ".U", ".U8", ".U16", ".U32", ".U64", ".UP", ".F32", ".F64", ".FBig",
	".FSmall", ".MaxI", ".MinI", ".Neg", ".Zero", ".ZeroU", ".ZeroF", ".NegZero", ".NaN", ".Inf", ".S", ".ES",
	".Bad", ".Uni", ".HTML", ".EHTML", ".JS", ".True", ".False", ".Time", ".ZeroTime", ".Z", ".NZ", ".Str",
	".SV", ".Err", ".V", ".PVal", ".TV.V", ".T.V", ".T.PV", ".Nil", ".NilT", ".Missing", ".M.nil", ".P.ynull",
	".P.missing", ".T.NilSub", ".T.NilIface", ".T.NilErr", ".T.NilStr", ".T.Iface", ".T.Err", ".T.Str",
	".T.NilM", ".T.NilSS", ".T.NilFn", ".Fn", ".NilFn", ".L", ".SS", ".SI", ".SB", ".SF", ".SBytes", ".SM",
	".Empty", ".EmptySS", ".EmptyM", ".NilM", ".NilSS", ".M", ".MS", ".P", ".PMerge", ".PEmpty", ".D",
	".Pages", ".EmptyPages", ".NilPages", ".NumKeys", "nilany", "nilptr", "nilerr", "nilstr", "nilmap",
	"nilslice", "(mkhtml 1)", "(failNil)", "1", "-1", "1.5", "1e3", "0x10", "'a'", `"s"`, `""`, "true",
	"false", "nil", ".", "$",
}

// etxShapes are template shapes; X is replaced by a value expression.
var etxShapes = []string{
	"{{X}}", "{{if X}}T{{else}}F{{end}}", "{{with X}}[{{.}}]{{else}}E{{end}}", "{{not X}}",
	"{{and X 1}}", "{{or X 0}}", "{{and 1 X}}", "{{or 0 X}}", "{{len X}}",
	"{{range X}}[{{.}}]{{else}}E{{end}}", "{{range $i, $e := X}}{{$i}}={{$e}};{{end}}",
	"{{printf \"%v|%T|%d|%s|%q|%x\" X X X X X X}}", "{{print X}}", "{{println X X}}",
	"{{typeof X}}", "{{isnil X}}", "{{X | typeof}}", "{{X | isnil}}", "{{index X 0}}", "{{index X \"a\"}}",
	"{{slice X 1}}", "{{eq X X}}", "{{eq X 1}}", "{{ne X nil}}", "{{lt X 1}}", "{{html X}}", "{{js X}}",
	"{{urlquery X}}", "{{X.Foo}}", "{{X.Name}}", "{{X.a}}", "{{X.VM}}", "{{X.PM}}", "{{X.IsZero}}",
	"{{X.Len}}", "{{X.Format \"2006-01-02\"}}", "{{X.String}}", "{{X.Error}}", "{{call X 1}}",
	"{{define \"x\"}}<{{.}}>{{end}}{{template \"x\" X}}", "{{$v := X}}{{$v}}", "{{$v := X}}{{$v.Foo}}",
	"{{with $v := X}}{{$v}}{{end}}", "{{(try X).Value}}", "{{echo X}}", "{{echo X | typeof}}",
	"{{variadic X X}}", "{{X | echo2 1}}", "{{.T.Echo X}}", "{{.T.Two X 2}}", "{{.T.Var X}}",
	"{{X.A}}", "{{X.B.C}}", "{{(X).Foo}}", "{{mkhtml X}}", "{{dict \"k\" X}}", "{{(list X).x}}",
}

func etxCorpus() []etxCase {
	var cs []etxCase
	add := func(name, src string) {
		cs = append(cs, etxCase{name: name, src: src, data: "root", mode: "both"})
	}
	addd := func(name, data, src string) {
		cs = append(cs, etxCase{name: name, src: src, data: data, mode: "both"})
	}
	addm := func(name, mode, src string) {
		cs = append(cs, etxCase{name: name, src: src, data: "root", mode: mode})
	}
	addo := func(name, opt, data, src string) {
		cs = append(cs, etxCase{name: name, src: src, data: data, opt: opt, mode: "both"})
	}
	for _, s := range etxHand {
		add(s[0], s[1])
	}
	for _, d := range []string{"nil", "int", "string", "typednil", "t", "list", "params", "siteparams"} {
		for _, s := range etxDotTemplates {
			addd("dot-"+d, d, s)
		}
	}
	for _, opt := range []string{"missingkey=default", "missingkey=invalid", "missingkey=zero", "missingkey=error"} {
		for _, s := range etxMissingKey {
			addo("missingkey", opt, "root", s)
			addo("missingkey", opt, "nil", s)
		}
	}
	// ExecuteTemplate.
	cs = append(cs, etxCase{name: "exectmpl", src: `{{define "a"}}A{{.S}}{{end}}x`, data: "root", exec: "a", mode: "plain"})
	cs = append(cs, etxCase{name: "exectmpl", src: `{{define "a"}}A{{end}}x`, data: "root", exec: "nope", mode: "plain"})
	cs = append(cs, etxCase{name: "exectmpl", src: `{{define "a"}}{{end}}`, data: "root", exec: "t", mode: "both"})
	cs = append(cs, etxCase{name: "noparse", src: "\x00noparse", data: "root", mode: "both"})
	// Hugo-only behaviour.
	for _, s := range etxHugoOnly {
		addm("hugo", "both", s)
	}
	// Generated combinations.
	for _, v := range etxValues {
		for _, sh := range etxShapes {
			if (v == ".MaxI" || v == ".U64") && strings.HasPrefix(sh, "{{range X}}") {
				continue // 2^63 iterations
			}
			add("combo", strings.ReplaceAll(sh, "X", v))
		}
	}
	return cs
}

// etxDotTemplates run with different data values as dot.
var etxDotTemplates = []string{
	"{{.}}", "{{$}}", "{{.X}}", "{{.X.Y}}", "{{$.X}}", "{{if .}}T{{else}}F{{end}}", "{{with .}}W{{else}}E{{end}}",
	"{{range .}}[{{.}}]{{else}}E{{end}}", "{{len .}}", "{{printf \"%v %T\" . .}}", "{{.Name}}", "{{.NilOK}}",
	"{{.PM}}", "{{.VM}}", "{{template \"t2\" .}}{{define \"t2\"}}<{{.}}>{{end}}", "{{index . 0}}", "{{.title}}",
	"{{.Title}}", "{{.mainSections}}", "{{.MainSections}}", "{{.IsZero}}", "{{html .}}", "{{(.)}}", "{{(.).X}}",
	"{{and . .}}", "{{or . 0}}", "{{not .}}", "{{eq . nil}}",
}

var etxMissingKey = []string{
	"{{.M.a}} {{.M.nope}}", "{{.M.nil}}", "{{.MS.a}} {{.MS.nope}}", "{{.P.title}} {{.P.nope}}",
	"{{.P.ynull}}", "{{.NilM.x}}", "{{.D.nope}}", "{{.Nope.Deeper}}", "{{.M.sub.nope}}", "{{index .MS \"nope\"}}",
	"{{index .M \"nope\"}}", "{{with .M.nope}}Y{{else}}N{{end}}", "{{.}}", "{{.X}}",
}

var etxHugoOnly = []string{
	"{{.P.Title}} {{.P.TITLE}} {{.P.title}}", "{{.P.YMAP.B}} {{.P.ymap.C.D}}", "{{.P.Mixed_Case}}",
	"{{.SiteParams.mainSections}}", "{{.SiteParams.MainSections}}", "{{.SiteParams.MAINSECTIONS}}",
	"{{.SiteParams.mainsections}}", "{{.P.mainSections}}", "{{.SiteParams.Social.Facebook.Enable}}",
	"{{.D.Pages}} {{.D.Singular}} {{.D.Count}}", "{{.P.IsZero}} {{.PMerge.IsZero}} {{.PEmpty.IsZero}}",
	"{{.P.iszero}}", "{{.P.GetNested \"ymap\" \"c\" \"d\"}}", "{{.P.GetNested \"YMAP\" \"B\"}}",
	"{{.P.GetNested \"nope\"}}", "{{if .PMerge}}T{{else}}F{{end}}", "{{with .PEmpty}}T{{else}}F{{end}}",
	"{{.Pages.Len}} {{.Pages.First.Name}} {{(.Pages.Reverse).First.Name}}", "{{.EmptyPages.First}}",
	"{{.NilPages.Len}}", "{{range .Pages.Reverse}}{{.Name}}{{end}}", "{{.Time.Year}}",
	"{{.Time.Format \"2006-01-02T15:04:05.000Z07:00\"}}", "{{.Time.IsZero}} {{.ZeroTime.IsZero}}",
	"{{.ZeroTime.Format \"Jan 2, 2006\"}}", "{{.Time.Unix}}", "{{.Time.UTC}}", "{{.Time.Day}}",
	"{{.T.Time.Year}}", "{{if .ZeroTime}}T{{else}}F{{end}}", "{{with .Time}}{{.Year}}{{end}}",
}

// etxHand are hand-written cases by contract clause.
var etxHand = [][2]string{
	// C2: field, method and key resolution.
	{"c2", "{{.T.Name}} {{.T.N}} {{.T.F}} {{.T.B}}"},
	{"c2", "{{.T.Sub.Name}} {{.T.Sub.Sub}} {{.T.Sub.Sub.Name}}"},
	{"c2", "{{.T.NilSub.Name}}"},
	{"c2", "{{.T.NilSub.Nope}}"},
	{"c2", "{{.T.NilSub.NilOK}}"},
	{"c2", "{{.T.NilSub.PM}}"},
	{"c2", "{{.T.NilSub.VM}}"},
	{"c2", "{{.NilT.NilOK}}"},
	{"c2", "{{.T.NilIface.X}}"},
	{"c2", "{{.T.NilErr.Error}}"},
	{"c2", "{{.T.NilStr.String}}"},
	{"c2", "{{.T.Err.Error}} {{.T.Str.String}}"},
	{"c2", "{{.T.Iface}} {{.T.Iface.X}}"},
	{"c2", "{{.T.VM}} {{.T.PM}} {{.TV.VM}}"},
	{"c2", "{{.TV.PM}}"},
	{"c2", "{{.TV.Name}} {{.TV.V.A}} {{.TV.V.VV}}"},
	{"c2", "{{.TV.V.PV2}}"},
	{"c2", "{{.T.V.VV}} {{.T.V.PV2}} {{.T.PV.VV}} {{.T.PV.PV2}}"},
	{"c2", "{{.V.VV}}"},
	{"c2", "{{.V.PV2}}"},
	{"c2", "{{.PVal.PV2}} {{.PVal.VV}} {{.PVal.A}}"},
	{"c2", "{{.M.v.VV}}"},
	{"c2", "{{.M.v.PV2}}"},
	{"c2", "{{.T.Name 1}}"},
	{"c2", "{{.T.Nope}}"},
	{"c2", "{{.T.nope}}"},
	{"c2", "{{.M.a}} {{.M.Key}} {{.M.key}} {{.M.KEY}}"},
	{"c2", "{{.M.a 1}}"},
	{"c2", "{{.M.sub.x}} {{.M.sub.nope}} {{.M.sub.nil}} {{.M.nope.deeper}}"},
	{"c2", "{{.M.nil.x}}"},
	{"c2", "{{.M.sub.nil.x}}"},
	{"c2", "{{.T.M.nil.x}}"},
	{"c2", "{{.L.x}}"},
	{"c2", "{{.S.x}}"},
	{"c2", "{{.I.x}}"},
	{"c2", "{{.Nil.x}} {{.Nil.x.y}}"},
	{"c2", "{{.Missing.x}}"},
	{"c2", "{{.NilM.x}}"},
	{"c2", "{{.MS.a}} {{.MS.nope}}"},
	{"c2", "{{.D.Pages}} {{.D.Singular}} {{.D.Count}} {{.D.nope}}"},
	{"c2", "{{.P.title}} {{.P.Title}}"},
	{"c2", "{{.P.ymap.b}} {{.P.ymap.c.d}}"},
	{"c2", "{{.P.IsZero}} {{.P.iszero}}"},
	{"c2", "{{.T.Self.Self.Name}} {{.T.GetSub.Name}} {{.T.Sub.GetSub}}"},
	{"c2", "{{.T.Sub.GetSub.Name}}"},
	{"c2", "{{.T.RetM.a}} {{.T.RetNilMap.a}} {{.T.RetNil}} {{.T.RetNilPtr}} {{.T.RetNilStr}}"},
	{"c2", "{{.T.RetNil.x}}"},
	{"c2", "{{.T.RetNilPtr.Name}}"},
	{"c2", "{{.T.RetNilStr.String}}"},
	{"c2", "{{.T.Echo 1}} {{.T.Echo \"s\"}} {{.T.Echo nil}} {{.T.Echo .M.nil}}"},
	{"c2", "{{.T.Two 1 2}} {{.T.Var}} {{.T.Var 1}} {{.T.Var 1 \"a\" nil}}"},
	{"c2", "{{.T.Fail}}"},
	{"c2", "{{.T.Panic}}"},
	{"c2", "{{.T.Echo}}"},
	{"c2", "{{.T.Echo 1 2}}"},
	{"c2", "{{.T.Two 1}}"},
	{"c2", "{{1 | .T.Echo}} {{2 | .T.Two 1}} {{3 | .T.Var 1 2}}"},
	{"c2", "{{.T.Fn}}"},
	{"c2", "{{.T.Pages}}"},
	{"c2", "{{.T.Time}} {{.T.H}}"},
	{"c2", "{{$x := .T}}{{$x.Name}} {{$x.Sub.Name}} {{$.T.Name}}"},
	{"c2", "{{(.T).Name}} {{(.T.Sub).Name}} {{(echo .T).Name}} {{(.M).a}}"},
	{"c2", "{{(nil).x}}"},
	{"c2", "{{$x := nil}}{{$x.x}}"},
	{"c2", "{{(1).x}}"},
	{"c2", "{{(\"s\").x}}"},
	{"c2", "{{(mkT \"m\").Name}} {{(mkT 1).PM}} {{mkT.Name}}"},
	{"c2", "{{echo.x}}"},
	{"c2", "{{.T.Sub.V.A}} {{.T.Sub.V.VV}} {{.T.Sub.V.PV2}}"},
	{"c2", "{{range .Pages}}{{.Name}},{{.PM}},{{.VM}};{{end}}"},
	// C3: literals.
	{"c3", "{{1}} {{-1}} {{+1}} {{1.0}} {{1e3}} {{1E-3}} {{0x10}} {{0o17}} {{0b101}} {{017}} {{1_000}}"},
	{"c3", "{{'a'}} {{'\\n'}} {{'é'}} {{'\\x41'}} {{'\\U0001F600'}}"},
	{"c3", "{{typeof 1}} {{typeof 1.0}} {{typeof 1e3}} {{typeof 0x10}} {{typeof 0x1p4}} {{typeof 'a'}} {{typeof -0.0}}"},
	{"c3", "{{typeof 9223372036854775807}} {{typeof -9223372036854775808}}"},
	{"c3", "{{9223372036854775808}}"},
	{"c3", "{{typeof 18446744073709551615}}"},
	{"c3", "{{-0.0}} {{0.0}} {{1e21}} {{1e20}} {{123456789.0}} {{0.000001}} {{0.0000001}} {{1.7976931348623157e308}}"},
	{"c3", "{{1i}}"},
	{"c3", "{{typeof 1i}}"},
	{"c3", "{{1+2i}}"},
	{"c3", "{{printf \"%v\" 1i}}"},
	{"c3", "{{0i}}"},
	{"c3", "{{typeof \"s\"}} {{typeof `r`}} {{typeof true}} {{typeof nil}}"},
	{"c3", "{{nil}}"},
	{"c3", "{{print nil}} {{printf \"%v\" nil}} {{echo nil}} {{isnil nil}}"},
	{"c3", "{{\"a\\tb\"}} {{`a\\tb`}} {{\"\\u00e9\"}} {{\"\\xff\"}}"},
	{"c3", "{{true}} {{false}} {{true | not}}"},
	{"c3", "{{1 2}}"},
	{"c3", "{{(1) 2}}"},
	{"c3", "{{(1)}}"},
	{"c3", "{{\"x\" 1}}"},
	{"c3", "{{. 1}}"},
	{"c3", "{{$ 1}}"},
	{"c3", "{{1 | 2}}"},
	{"c3", "{{$x := 1}}{{$x 2}}"},
	{"c3", "{{$x := 1}}{{3 | $x}}"},
	// C5/C6: truthiness, and/or/not.
	{"c5", "{{if .Z}}T{{else}}F{{end}} {{if .NZ}}T{{else}}F{{end}} {{if .ZeroTime}}T{{else}}F{{end}} {{if .Time}}T{{else}}F{{end}}"},
	{"c5", "{{if .PMerge}}T{{else}}F{{end}} {{if .PEmpty}}T{{else}}F{{end}} {{if .P}}T{{else}}F{{end}}"},
	{"c5", "{{if .EmptyPages}}T{{else}}F{{end}} {{if .NilPages}}T{{else}}F{{end}} {{if .Pages}}T{{else}}F{{end}}"},
	{"c5", "{{if .T.NilSub}}T{{else}}F{{end}} {{if .T.NilIface}}T{{else}}F{{end}} {{if .T.NilErr}}T{{else}}F{{end}}"},
	{"c5", "{{if .T.Fn}}T{{else}}F{{end}} {{if .T.NilFn}}T{{else}}F{{end}} {{if .V}}T{{else}}F{{end}} {{if .TV}}T{{else}}F{{end}}"},
	{"c5", "{{if .NaN}}T{{else}}F{{end}} {{if .NegZero}}T{{else}}F{{end}} {{if .EHTML}}T{{else}}F{{end}}"},
	{"c5", "{{not .Z}} {{not .NZ}} {{not .ZeroTime}} {{not .PMerge}} {{not .T.NilSub}} {{not .Nil}}"},
	{"c5", "{{and .Z 1}} {{or .Z 0}} {{and .NZ 1}} {{or .ZeroTime 5}} {{and 1 .PMerge}}"},
	{"c6", "{{and 1 0 2}}|{{or 0 \"\" \"x\"}}|{{or 0 \"\"}}|{{and}}"},
	{"c6", "{{or}}"},
	{"c6", "{{not}}"},
	{"c6", "{{not 1 2}}"},
	{"c6", "{{and 1 (fail \"no\")}}"},
	{"c6", "{{and 0 (fail \"no\")}}|{{or 1 (fail \"no\")}}"},
	{"c6", "{{or 0 (fail \"yes\")}}"},
	{"c6", "{{and 1 .Missing}}|{{or 0 .Missing}}|{{and .Missing 1}}"},
	{"c6", "{{1 | and 2}}|{{0 | and 2}}|{{0 | or 0}}|{{nilany | or 0}}|{{.M.nil | or 5}}"},
	{"c6", "{{and 1 nil}}|{{or nil 0}}|{{typeof (and 1 2.5)}}|{{typeof (or .Nil .ZeroF)}}"},
	{"c6", "{{and .T.NilSub 1}}|{{typeof (and .T.NilSub 1)}}|{{typeof (or 0 .T.NilIface)}}"},
	{"c6", "{{if and true 1 `hi`}}T{{else}}F{{end}}{{if and true 1 `hi` | not}}T{{else}}F{{end}}"},
	// C7: arguments.
	{"c7", "{{isnil .T.NilIface}} {{isnil .T.NilErr}} {{isnil .T.NilStr}} {{isnil .T.NilSub}} {{isnil .NilM}} {{isnil .Nil}}"},
	{"c7", "{{typeof .T.NilErr}} {{typeof .T.NilSub}} {{typeof .NilM}} {{typeof .NilSS}} {{typeof .Nil}} {{typeof .M.nil}}"},
	{"c7", "{{.T.NilErr | typeof}} {{.T.NilSub | typeof}} {{nilerr | typeof}} {{nilptr | typeof}} {{nilany | typeof}}"},
	{"c7", "{{nilerr}} {{nilptr}} {{nilany}} {{nilmap}} {{nilslice}} {{nilstr}}"},
	{"c7", "{{isnil nilerr}} {{isnil nilptr}} {{isnil nilany}} {{isnil nilstr}}"},
	{"c7", "{{nilerr.Error}}"},
	{"c7", "{{nilany.x}}"},
	{"c7", "{{nilptr.Name}}"},
	{"c7", "{{nilstr.String}}"},
	{"c7", "{{(nilerr).Error}}"},
	{"c7", "{{$e := nilerr}}{{$e}} {{typeof $e}} {{isnil $e}}"},
	{"c7", "{{$e := nilany}}{{$e}} {{typeof $e}}"},
	{"c7", "{{echo .T.NilErr}} {{echo .T.NilSub}} {{echo nilerr}}"},
	{"c7", "{{variadic}} {{variadic nil}} {{variadic 1 nil .Nil .T.NilErr}}"},
	{"c7", "{{strarg \"x\"}} {{intarg 3}} {{\"y\" | strarg}}"},
	{"c7", "{{printf \"%s\" .S}} {{printf .S}} {{\"%d-%s\" | printf}}"},
	{"c7", "{{printf 1}}"},
	{"c7", "{{printf .I}}"},
	{"c7", "{{printf .Nil}}"},
	{"c7", "{{printf .HTML}}"},
	{"c7", "{{printf nil}}"},
	{"c7", "{{printf .M.nil}}"},
	{"c7", "{{printf}}"},
	{"c7", "{{1 | printf}}"},
	{"c7", "{{printf \"%d %s %v %q %x %T\" 1 \"s\" .Nil .S .S .F32}}"},
	{"c7", "{{printf \"%d\" \"x\"}} {{printf \"%s\"}} {{printf \"%s\" 1 2}} {{printf \"%!\"}}"},
	{"c7", "{{printf \"%v %v %v %v\" .T.NilErr .T.NilSub .NilM .NilSS}}"},
	{"c7", "{{printf \"%v %+v %#v\" .V .V .V}}"},
	{"c7", "{{printf \"%v\" .Z}} {{printf \"%v\" .SV}} {{printf \"%v\" .Str}} {{printf \"%v\" .Err}} {{printf \"%s\" .Time}}"},
	// C8: printing.
	{"c8", "{{.Nil}}|{{.Missing}}|{{.M.nil}}|{{.P.ynull}}|{{.T.NilSub}}|{{.T.NilIface}}|{{.T.NilErr}}|{{.T.NilStr}}"},
	{"c8", "{{.NilM}}|{{.NilSS}}|{{.NilPages}}|{{.T.NilM}}|{{.T.NilSS}}|{{nilmap}}|{{nilslice}}"},
	{"c8", "{{.T.NilFn}}"},
	{"c8", "{{.Fn}}"},
	{"c8", "{{nilptr}}|{{nilerr}}|{{nilstr}}|{{nilany}}"},
	{"c8", "{{.V}}|{{.PVal}}|{{.TV.V}}|{{.T.V}}|{{.T.PV}}|{{.M.v}}"},
	{"c8", "{{.Z}}|{{.NZ}}|{{.Str}}|{{.SV}}|{{.Err}}|{{.T.Err}}|{{.T.Str}}"},
	{"c8", "{{.L}}|{{.SS}}|{{.SI}}|{{.SB}}|{{.SF}}|{{.SBytes}}|{{.SM}}|{{.Empty}}|{{.EmptySS}}"},
	{"c8", "{{.MS}}|{{.EmptyM}}|{{.NumKeys}}|{{.PMerge}}|{{.PEmpty}}|{{.Nested}}"},
	{"c8", "{{.P}}"},
	{"c8", "{{.M}}"},
	{"c8", "{{.D}}"},
	{"c8", "{{.I}} {{.I8}} {{.U64}} {{.UP}} {{.F32}} {{.F64}} {{.FBig}} {{.FSmall}} {{.MaxI}} {{.MinI}} {{.NegZero}} {{.NaN}} {{.Inf}}"},
	{"c8", "{{.S}}|{{.Bad}}|{{.Uni}}|{{.HTML}}|{{.JS}}|{{.True}}|{{.Time}}|{{.ZeroTime}}"},
	{"c8", "{{print .S .S}}|{{print 1 2}}|{{print \"a\" 1 2 \"b\"}}|{{print .HTML 1}}|{{print .Nil 1}}|{{println .S 1}}"},
	{"c8", "{{print .V .PVal .Str .SV .Err .Z}}"},
	{"c8", "{{print .T.NilErr .T.NilSub .NilM .NilSS .Nil .M.nil}}"},
	{"c8", "{{html .Nil}} {{html .T.NilSub}} {{html .M.nil}} {{html .T.NilErr}} {{html .NilM}}"},
	{"c8", "{{html \"<a href='x'>&\\\"\\x00\"}} {{html 1 2}} {{html .HTML}} {{html .V}} {{html .PVal}}"},
	{"c8", "{{js \"a'b<c>=&\\\"\\\\\\n\\u2028é\\x00\\xff\"}} {{js 1 \"x\"}} {{js .Nil}}"},
	{"c8", "{{urlquery \"a b&c=d/é\"}} {{urlquery 1 2}} {{urlquery .Nil}} {{urlquery .PVal}}"},
	// C11: range.
	{"c11", "{{range 3}}{{.}}{{end}}|{{range $i := 3}}{{$i}}{{end}}|{{range 0}}x{{else}}E{{end}}|{{range -2}}x{{else}}E{{end}}"},
	{"c11", "{{range $i, $e := 3}}{{end}}"},
	{"c11", "{{range .U8}}{{end}}{{range $i := .I8}}{{typeof $i}}{{end}}"},
	{"c11", "{{range .ZeroU}}x{{else}}E{{end}}{{range .U16}}{{.}}{{end}}"},
	{"c11", "{{range .S}}{{.}}{{end}}"},
	{"c11", "{{range .HTML}}{{.}}{{end}}"},
	{"c11", "{{range .F64}}{{.}}{{end}}"},
	{"c11", "{{range .True}}{{.}}{{end}}"},
	{"c11", "{{range .V}}{{.}}{{end}}"},
	{"c11", "{{range .T}}{{.}}{{end}}"},
	{"c11", "{{range .Time}}{{.}}{{end}}"},
	{"c11", "{{range .Fn}}{{.}}{{end}}"},
	{"c11", "{{range .NilT}}{{.}}{{end}}"},
	{"c11", "{{range .Nil}}x{{else}}E{{end}}|{{range .Missing}}x{{else}}E{{end}}|{{range .NilM}}x{{else}}E{{end}}|{{range .NilSS}}x{{else}}E{{end}}|{{range .NilPages}}x{{else}}E{{end}}"},
	{"c11", "{{range .M}}{{.}};{{end}}|{{range $k, $v := .M}}{{$k}}={{$v}};{{end}}"},
	{"c11", "{{range $k, $v := .NumKeys}}{{$k}}={{$v}};{{end}}"},
	{"c11", "{{range $k, $v := .P}}{{$k}};{{end}}|{{range .P.ymap}}{{.}};{{end}}"},
	{"c11", "{{range $k, $v := .D}}{{$k}}={{$v}};{{end}}"},
	{"c11", "{{range $i, $p := .Pages}}{{$i}}={{$p.Name}};{{end}}|{{range .EmptyPages}}x{{else}}E{{end}}"},
	{"c11", "{{range .L}}[{{.}}]{{end}}|{{range $e := .L}}[{{$e}}]{{end}}|{{range $i, $e := .L}}[{{$i}}:{{$e}}]{{end}}"},
	{"c11", "{{range .L}}{{.x}}{{end}}"},
	{"c11", "{{range $e := .L}}{{$e.x}}{{end}}"},
	{"c11", "{{range .M.list}}{{.}}{{end}}|{{range .SM}}{{.a}}{{end}}"},
	{"c11", "{{range .M}}{{.x}}{{end}}"},
	{"c11", "{{range .SI}}{{if eq . 4}}{{break}}{{end}}{{.}}{{end}}|{{range .SI}}{{if eq . 4}}{{continue}}{{end}}{{.}}{{end}}"},
	{"c11", "{{range .SI}}{{range .}}{{end}}{{end}}"},
	{"c11", "{{range $i, $e := .SI}}{{range $j, $f := $.SS}}{{$i}}{{$f}}{{if eq $j 1}}{{break}}{{end}}{{end}}{{if eq $i 1}}{{continue}}{{end}};{{end}}"},
	{"c11", "{{$i := 0}}{{$e := 0}}{{range $i, $e = .SS}}{{end}}{{$i}}{{$e}}"},
	{"c11", "{{$e := 0}}{{range $e = .SS}}{{end}}{{$e}}"},
	{"c11", "{{range $i, $e := .SS}}{{$e := 1}}{{$e}}{{end}}"},
	{"c11", "{{range .SS}}{{$x := .}}{{end}}{{range .SS}}{{.}}{{else}}E{{end}}"},
	{"c11", "{{range .}}{{.}};{{end}}"},
	{"c11", "{{range .Pages.Reverse}}{{.Name}}{{end}}"},
	{"c11", "{{range .SBytes}}{{.}};{{end}}"},
	// C12: errors and try.
	{"c12", "a{{fail \"boom\"}}b"},
	{"c12", "{{failErr}}"},
	{"c12", "{{failNil}}|{{typeof failNil}}"},
	{"c12", "{{panicky}}"},
	{"c12", "{{panicerr}}"},
	{"c12", "{{errval}}|{{typeof errval}}|{{errval.Error}}"},
	{"c12", "{{.Nope.x.y}}{{.T.Nope}}"},
	{"c12", "{{with try (fail \"boom\")}}{{.Err}}|{{.Value}}|{{.Err.Cause}}|{{.Err.Err}}{{end}}"},
	{"c12", "{{with try (echo 1)}}{{.Value}}|{{.Err}}|{{if .Err}}E{{else}}N{{end}}{{end}}"},
	{"c12", "{{(try (echo 1)).Value}}|{{(try (fail 1)).Value}}|{{(try nilany).Value}}"},
	{"c12", "{{try (echo 1)}}"},
	{"c12", "{{try (fail \"x\")}}"},
	{"c12", "{{with try .T.Fail}}{{.Err}}{{end}}"},
	{"c12", "{{with try (.T.Panic)}}{{.Err}}{{end}}"},
	{"c12", "{{with try (panicky)}}{{.Err}}{{end}}"},
	{"c12", "{{with try (printf)}}{{.Err}}|{{.Err.Cause}}{{end}}"},
	{"c12", "{{with try (.Nope.X 1)}}{{.Err}}{{end}}"},
	{"c12", "{{with try (index .SS 10)}}{{.Err}}|{{.Err.Cause}}{{end}}"},
	{"c12", "{{try}}"},
	{"c12", "{{try 1 2}}"},
	{"c12", "{{(try (fail 2)).Err.Error}}"},
	{"c12", "{{(try (fail 2)).Err.Unwrap}}"},
	{"c12", "{{typeof (try 1)}}|{{typeof (try (fail 1)).Err}}|{{typeof (try 1).Err}}"},
	{"c12", "{{with try (fail 3)}}{{with .Err}}{{.}}{{end}}{{end}}"},
	{"c12", "{{define \"e\"}}{{fail \"inner\"}}{{end}}{{template \"e\"}}"},
	{"c12", "x\ny\n{{define \"one\"}}{{template \"two\" .}}{{end}}{{define \"two\"}}{{index \"hi\" 5}}{{end}}{{template \"one\"}}"},
	{"c12", "{{template \"nope\"}}"},
	{"c12", "{{.T.Fn 1}}"},
	{"c12", "{{call .Fn 1}}|{{call .T.Fn \"x\"}}|{{call getfn 2}}"},
	{"c12", "{{call .Fn2 1 2}}"},
	{"c12", "{{call .NilFn 1}}"},
	{"c12", "{{call .Nil 1}}"},
	{"c12", "{{call nil}}"},
	{"c12", "{{call}}"},
	{"c12", "{{call .S}}"},
	{"c12", "{{call (len \"test\")}}"},
	{"c12", "{{.Fn | call}}"},
	{"c12", "{{1 | call .Fn}}"},
	{"c12", "{{call .T.PM}}"},
	// Variables and scoping.
	{"vars", "{{$x := 1}}{{if true}}{{$x = 2}}{{end}}{{$x}}{{with $y := 3}}{{$x = $y}}{{end}}{{$x}}"},
	{"vars", "{{$x := 1}}{{if true}}{{$x := 2}}{{$x}}{{end}}{{$x}}"},
	{"vars", "{{$x := 1}}{{range .SS}}{{$x = .}}{{end}}{{$x}}"},
	{"vars", "{{$}} {{$.S}} {{with .T}}{{$.S}}{{.Name}}{{end}}"},
	{"vars", "{{define \"d\"}}{{$}}{{end}}{{template \"d\" 5}}{{template \"d\"}}"},
	{"vars", "{{$x := .S}}{{define \"d\"}}{{.}}{{end}}{{template \"d\" $x}}"},
	{"vars", "{{$a, $b := 1}}"},
	{"vars", "{{$x := 1 | printf \"%d\"}}{{$x}}"},
	{"vars", "{{$x := .M}}{{$x.a}}{{$y := $x.sub}}{{$y.x}}"},
	// template/define/block.
	{"tmpl", "{{define \"a\"}}A{{.}}{{end}}{{template \"a\" 1}}{{template \"a\"}}{{template \"a\" .S}}"},
	{"tmpl", "{{block \"b\" .S}}[{{.}}]{{end}}|{{template \"b\" 2}}"},
	{"tmpl", "{{define \"r\"}}{{if gt . 0}}{{.}}{{template \"r\" (len (slice \"xxxxxxxxx\" 1 .))}}{{end}}{{end}}{{template \"r\" 5}}"},
	{"tmpl", "{{define \"t\"}}redefined{{end}}x"},
	{"tmpl", "{{define \"a\"}}{{end}}[{{template \"a\"}}]"},
	{"tmpl", "{{template \"t\"}}"},
	// Builtins.
	{"builtins", "{{len .S}} {{len .Uni}} {{len .SS}} {{len .M}} {{len .P}} {{len .Pages}} {{len .NilM}} {{len .NilSS}} {{len .HTML}} {{len .D}}"},
	{"builtins", "{{len 3}}"},
	{"builtins", "{{len .Nil}}"},
	{"builtins", "{{len nil}}"},
	{"builtins", "{{len .T.NilSub}}"},
	{"builtins", "{{len .T.NilIface}}"},
	{"builtins", "{{len .T}}"},
	{"builtins", "{{len .V}}"},
	{"builtins", "{{len}}"},
	{"builtins", "{{len 1 2}}"},
	{"builtins", "{{index .SS 1}} {{index .SI 0}} {{index .L 1}} {{index .M \"a\"}} {{index .M \"nope\"}} {{index .MS \"nope\"}} {{index .S 1}} {{index .SM 1 \"a\"}}"},
	{"builtins", "{{index .SS}} {{index .S}} {{index 1}}"},
	{"builtins", "{{index .SS 3}}"},
	{"builtins", "{{index .SS 5}}"},
	{"builtins", "{{index .SS -1}}"},
	{"builtins", "{{index .SS \"x\"}}"},
	{"builtins", "{{index .SS nil}}"},
	{"builtins", "{{index .M 1}}"},
	{"builtins", "{{index .M nil}}"},
	{"builtins", "{{index .M .HTML}}"},
	{"builtins", "{{index nil 1}}"},
	{"builtins", "{{index .Nil 1}}"},
	{"builtins", "{{index .T.NilSub 1}}"},
	{"builtins", "{{index .T.NilIface 1}}"},
	{"builtins", "{{index .I 1}}"},
	{"builtins", "{{index .NilM \"a\"}}|{{index .NilSS 0}}"},
	{"builtins", "{{index .Pages 0}}"},
	{"builtins", "{{(index .Pages 1).Name}} {{index .P \"title\"}} {{index .P \"Title\"}} {{index .D \"Pages\"}}"},
	{"builtins", "{{index .L 2}}|{{typeof (index .L 2)}}|{{isnil (index .L 2)}}|{{(index .L 2).x}}"},
	{"builtins", "{{index .S 5}}"},
	{"builtins", "{{index .U8 0}}"},
	{"builtins", "{{index .SS .U8}}"},
	{"builtins", "{{index .SS 0 1}}"},
	{"builtins", "{{slice .SS}} {{slice .SS 1}} {{slice .SS 1 2}} {{slice .SS 1 2 3}} {{slice .S 1 3}} {{slice .HTML 1 2}} {{typeof (slice .HTML 1)}} {{slice .Uni 1 3}}"},
	{"builtins", "{{slice .SS 2 1}}"},
	{"builtins", "{{slice .SS 0 4}}"},
	{"builtins", "{{slice .SS 1 2 1}}"},
	{"builtins", "{{slice .S 1 2 3}}"},
	{"builtins", "{{slice .SS 1 2 3 4}}"},
	{"builtins", "{{slice nil}}"},
	{"builtins", "{{slice .M 1}}"},
	{"builtins", "{{slice .NilSS}}|{{slice .NilSS 0 0}}|{{typeof (slice .Pages 1)}}|{{slice .EmptyPages}}"},
	{"builtins", "{{slice .SS -1}}"},
	{"builtins", "{{slice .SS \"a\"}}"},
	{"builtins", "{{slice .SS nil}}"},
	{"builtins", "{{eq 1 1}} {{eq 1 2 1}} {{eq \"a\" \"a\"}} {{eq .HTML .HTML}} {{eq .S \"hello\"}} {{eq .U8 255}} {{eq .I8 -8}} {{eq .U64 -1}} {{eq .F32 1.5}}"},
	{"builtins", "{{eq 1 1.0}}"},
	{"builtins", "{{eq \"a\" 1}}"},
	{"builtins", "{{eq 1}}"},
	{"builtins", "{{eq}}"},
	{"builtins", "{{eq .Nil .Nil}} {{eq .Nil nil}} {{eq nil nil}} {{eq .T.NilSub nil}} {{eq .NilM nil}} {{eq .T .T}} {{eq .T .T.Sub}} {{eq .V .V}} {{eq .Time .Time}} {{eq .Z .NZ}}"},
	{"builtins", "{{eq .SS .SS}}"},
	{"builtins", "{{eq .M .M}}"},
	{"builtins", "{{eq .SS nil}}"},
	{"builtins", "{{eq .T 1}}"},
	{"builtins", "{{eq .SS .M}}"},
	{"builtins", "{{eq .V .PVal}}"},
	{"builtins", "{{eq .Fn .Fn}}"},
	{"builtins", "{{ne 1 2}} {{ne \"a\" \"a\"}} {{ne .Nil 1}} {{ne .I .I}}"},
	{"builtins", "{{ne 1 2 3}}"},
	{"builtins", "{{lt 1 2}} {{lt 2 1}} {{lt 1.5 2.5}} {{lt \"a\" \"b\"}} {{lt .U8 256}} {{lt -1 .U8}} {{lt .U8 -1}} {{le 1 1}} {{gt 2 1}} {{ge 1 2}} {{le .U 7}} {{ge .Neg .U}}"},
	{"builtins", "{{lt true false}}"},
	{"builtins", "{{lt 1 \"a\"}}"},
	{"builtins", "{{lt .Nil 1}}"},
	{"builtins", "{{lt .T 1}}"},
	{"builtins", "{{lt 1 2.5}}"},
	{"builtins", "{{le 1 \"a\"}}"},
	{"builtins", "{{gt .Nil .Nil}}"},
	{"builtins", "{{lt .HTML \"z\"}} {{le .Time .Time}}"},
	{"builtins", "{{print}}|{{println}}|{{print \"\"}}|{{printf \"\"}}"},
	{"builtins", "{{html}}|{{js}}|{{urlquery}}"},
}

// Keep the fork's html/template reachable (template.HTML etc. live there).
var _ = htmltemplate.HTML("")
