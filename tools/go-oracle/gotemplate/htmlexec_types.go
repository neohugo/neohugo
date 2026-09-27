//go:build gotemplate_oracle

package main

// Host types of the html/template Go tests (exec_test.go, escape_test.go,
// content_test.go, ...), copied into package main so the htmlexec oracle
// can build their values from value specs. crates/gotemplate/tests/common
// has the Rust counterparts (same names, "main." instead of "template.").

import (
	"bytes"
	"errors"
	"fmt"
	"math"
	"reflect"
	"strconv"
	"strings"

	htmltemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/htmltemplate"
)

// ---------------------------------------------------------------------------
// exec_test.go

// T has lots of interesting pieces to use to test execution.
type T struct {
	// Basics
	True        bool
	I           int
	U16         uint16
	X, S        string
	FloatZero   float64
	ComplexZero complex128
	// Nested structs.
	U *U
	// Struct with String method.
	V0     V
	V1, V2 *V
	// Struct with Error method.
	W0     W
	W1, W2 *W
	// Slices
	SI      []int
	SICap   []int
	SIEmpty []int
	SB      []bool
	// Arrays
	AI [3]int
	// Maps
	MSI      map[string]int
	MSIone   map[string]int // one element, for deterministic output
	MSIEmpty map[string]int
	MXI      map[any]int
	MII      map[int]int
	MI32S    map[int32]string
	MI64S    map[int64]string
	MUI32S   map[uint32]string
	MUI64S   map[uint64]string
	MI8S     map[int8]string
	MUI8S    map[uint8]string
	SMSI     []map[string]int
	// Empty interfaces; used to see if we can dig inside one.
	Empty0 any // nil
	Empty1 any
	Empty2 any
	Empty3 any
	Empty4 any
	// Non-empty interfaces.
	NonEmptyInterface         I
	NonEmptyInterfacePtS      *I
	NonEmptyInterfaceNil      I
	NonEmptyInterfaceTypedNil I
	// Stringer.
	Str fmt.Stringer
	Err error
	// Pointers
	PI  *int
	PS  *string
	PSI *[]int
	NIL *int
	// Function (not method)
	BinaryFunc      func(string, string) string
	VariadicFunc    func(...string) string
	VariadicFuncInt func(int, ...string) string
	NilOKFunc       func(*int) bool
	ErrFunc         func() (string, error)
	PanicFunc       func() string
	// Template to test evaluation of templates.
	Tmpl *htmltemplate.Template
	// Unexported field; cannot be accessed by template.
	unexported int //nolint:unused
}

// S is a slice type with a method.
type S []string

// Method0 is a method on a slice type.
func (S) Method0() string {
	return "M0"
}

// U is a struct with one field.
type U struct {
	V string
}

// V has a pointer-receiver String method.
type V struct {
	j int
}

func (v *V) String() string {
	if v == nil {
		return "nilV"
	}
	return fmt.Sprintf("<%d>", v.j)
}

// W has a pointer-receiver Error method.
type W struct {
	k int
}

func (w *W) Error() string {
	if w == nil {
		return "nilW"
	}
	return fmt.Sprintf("[%d]", w.k)
}

// I is a non-empty interface.
type I interface {
	Method0() string
}

func newIntP(n int) *int {
	return &n
}

func newStringP(s string) *string {
	return &s
}

func newIntSliceP(n ...int) *[]int {
	p := new([]int)
	*p = make([]int, len(n))
	copy(*p, n)
	return p
}

// newTVal returns exec_test.go's tVal (a fresh copy).
func newTVal() *T {
	siVal := I(S{"a", "b"})
	return &T{
		True:   true,
		I:      17,
		U16:    16,
		X:      "x",
		S:      "xyz",
		U:      &U{"v"},
		V0:     V{6666},
		V1:     &V{7777}, // leave V2 as nil
		W0:     W{888},
		W1:     &W{999}, // leave W2 as nil
		SI:     []int{3, 4, 5},
		SICap:  make([]int, 5, 10),
		AI:     [3]int{3, 4, 5},
		SB:     []bool{true, false},
		MSI:    map[string]int{"one": 1, "two": 2, "three": 3},
		MSIone: map[string]int{"one": 1},
		MXI:    map[any]int{"one": 1},
		MII:    map[int]int{1: 1},
		MI32S:  map[int32]string{1: "one", 2: "two"},
		MI64S:  map[int64]string{2: "i642", 3: "i643"},
		MUI32S: map[uint32]string{2: "u322", 3: "u323"},
		MUI64S: map[uint64]string{2: "ui642", 3: "ui643"},
		MI8S:   map[int8]string{2: "i82", 3: "i83"},
		MUI8S:  map[uint8]string{2: "u82", 3: "u83"},
		SMSI: []map[string]int{
			{"one": 1, "two": 2},
			{"eleven": 11, "twelve": 12},
		},
		Empty1:                    3,
		Empty2:                    "empty2",
		Empty3:                    []int{7, 8},
		Empty4:                    &U{"UinEmpty"},
		NonEmptyInterface:         &T{X: "x"},
		NonEmptyInterfacePtS:      &siVal,
		NonEmptyInterfaceTypedNil: (*T)(nil),
		Str:                       bytes.NewBuffer([]byte("foozle")),
		Err:                       errors.New("erroozle"),
		PI:                        newIntP(23),
		PS:                        newStringP("a string"),
		PSI:                       newIntSliceP(21, 22, 23),
		BinaryFunc:                func(a, b string) string { return fmt.Sprintf("[%s=%s]", a, b) },
		VariadicFunc:              func(s ...string) string { return fmt.Sprint("<", strings.Join(s, "+"), ">") },
		VariadicFuncInt:           func(a int, s ...string) string { return fmt.Sprint(a, "=<", strings.Join(s, "+"), ">") },
		NilOKFunc:                 func(s *int) bool { return s == nil },
		ErrFunc:                   func() (string, error) { return "bla", nil },
		PanicFunc:                 func() string { panic("test panic") },
		Tmpl:                      htmltemplate.Must(htmltemplate.New("x").Parse("test template")), // "x" is the value of .X
	}
}

// Simple methods with and without arguments.

// Method0 returns "M0".
func (t *T) Method0() string {
	return "M0"
}

// Method1 returns its argument.
func (t *T) Method1(a int) int {
	return a
}

// Method2 formats its arguments.
func (t *T) Method2(a uint16, b string) string {
	return fmt.Sprintf("Method2: %d %s", a, b)
}

// Method3 formats its argument.
func (t *T) Method3(v any) string {
	return fmt.Sprintf("Method3: %v", v)
}

// Copy returns a copy of t.
func (t *T) Copy() *T {
	n := new(T)
	*n = *t
	return n
}

// MAdd adds a to every element of b.
func (t *T) MAdd(a int, b []int) []int {
	v := make([]int, len(b))
	for i, x := range b {
		v[i] = x + a
	}
	return v
}

var myError = errors.New("my error")

// MyError returns a value and an error according to its argument.
func (t *T) MyError(error bool) (bool, error) {
	if error {
		return true, myError
	}
	return false, nil
}

// GetU returns t.U (chaining).
func (t *T) GetU() *U {
	return t.U
}

// TrueFalse returns "true" or "".
func (u *U) TrueFalse(b bool) string {
	if b {
		return "true"
	}
	return ""
}

func typeOf(arg any) string {
	return fmt.Sprintf("%T", arg)
}

func zeroArgs() string {
	return "zeroArgs"
}

func oneArg(a string) string {
	return "oneArg=" + a
}

func twoArgs(a, b string) string {
	return "twoArgs=" + a + b
}

func dddArg(a int, b ...string) string {
	return fmt.Sprintln(a, b)
}

// count returns a channel that will deliver n sequential 1-letter strings starting at "a"
func count(n int) chan string {
	if n == 0 {
		return nil
	}
	c := make(chan string)
	go func() {
		for i := 0; i < n; i++ {
			c <- "abcdefghijklmnop"[i : i+1]
		}
		close(c)
	}()
	return c
}

// vfunc takes a *V and a V
func vfunc(V, *V) string {
	return "vfunc"
}

// valueString takes a string, not a pointer.
func valueString(v string) string {
	return "value is ignored"
}

// returnInt returns an int
func returnInt() int {
	return 7
}

func add(args ...int) int {
	sum := 0
	for _, x := range args {
		sum += x
	}
	return sum
}

func echo(arg any) any {
	return arg
}

func makemap(arg ...string) map[string]string {
	if len(arg)%2 != 0 {
		panic("bad makemap")
	}
	m := make(map[string]string)
	for i := 0; i < len(arg); i += 2 {
		m[arg[i]] = arg[i+1]
	}
	return m
}

func stringer(s fmt.Stringer) string {
	return s.String()
}

func mapOfThree() any {
	return map[string]int{"three": 3}
}

// execFuncs is testExecute's FuncMap.
var execFuncs = htmltemplate.FuncMap{
	"add":         add,
	"count":       count,
	"dddArg":      dddArg,
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

// Tree is exec_test.go's binary tree.
type Tree struct {
	Val         int
	Left, Right *Tree
}

// ---------------------------------------------------------------------------
// escape_test.go, content_test.go

type badMarshaler struct{}

func (x *badMarshaler) MarshalJSON() ([]byte, error) {
	// Keys in valid JSON must be double quoted as must all strings.
	return []byte("{ foo: 'not quite valid JSON' }"), nil
}

type goodMarshaler struct{}

func (x *goodMarshaler) MarshalJSON() ([]byte, error) {
	return []byte(`{ "<foo>": "O'Reilly" }`), nil
}

// Issue7379 is an int type with a method.
type Issue7379 int

// SomeMethod formats x.
func (Issue7379) SomeMethod(x int) string {
	return fmt.Sprintf("<%d>", x)
}

type myStringer struct {
	v int
}

func (s *myStringer) String() string {
	return fmt.Sprintf("string=%d", s.v)
}

type errorer struct {
	v int
}

func (s *errorer) Error() string {
	return fmt.Sprintf("error=%d", s.v)
}

// sevenStruct stands for exec_test.go's struct{ a int; b string }{7, "seven"}.
type sevenStruct struct {
	a int
	b string
}

// dataItem is TestEscapeSet's recursive data.
type dataItem struct {
	Children []*dataItem
	X        string
}

// recursiveInvoker is for TestRecursiveExecuteViaMethod.
type recursiveInvoker struct {
	tmpl *htmltemplate.Template
}

// Recur executes the "subroutine" template.
func (r *recursiveInvoker) Recur() (string, error) {
	var sb strings.Builder
	if err := r.tmpl.ExecuteTemplate(&sb, "subroutine", nil); err != nil {
		return "", err
	}
	return sb.String(), nil
}

// ---------------------------------------------------------------------------
// Value spec nodes for these types (see valuespec.go).

func init() {
	reg(
		(**T)(nil), (*[]*T)(nil), (**V)(nil), (**W)(nil), (**U)(nil), (**Tree)(nil),
		(**map[string]string)(nil), (*map[int]int)(nil), (**dataItem)(nil), (*I)(nil),
	)
}

// decodeTestNode decodes the value spec nodes of the Go test types:
//
//	tval tzero tzeroptr            exec_test.go tVal, T{}, &T{}
//	tree(int,node,node)            *Tree (children: tree(...) or nil)
//	st(field...) pst(field...)     struct / pointer to struct (reflect.StructOf);
//	                               fields f:HEXNAME(node) (the node's type, any for nil)
//	                               or fa:HEXNAME(node) (type any)
//	bufstr:HEX                     bytes.NewBufferString (a *bytes.Buffer Stringer)
//	newint                         new(int)
//	badm goodm                     &badMarshaler{}, &goodMarshaler{}
//	issue7379:N                    Issue7379(N)
//	mystringer:N errorer:N         &myStringer{N}, &errorer{N}
//	dataitem:HEX(node...)          *dataItem{X: HEX, Children: nodes}
//	dataitemv:HEX(node...)         dataItem value
//	fnplusone                      func(n int) int { return n + 1 }
//	cplx:RE;IM                     complex128 (IEEE bits; not representable in Rust)
//	vval:N vptr:N wval:N wptr:N    V{N}, &V{N}, W{N}, &W{N}
//	lvval(N...)                    []V{{N}...}
//	ivalptr                        &iVal (a *I holding a fresh tVal)
//	seven                          struct{ a int; b string }{7, "seven"} (as main.sevenStruct)
func decodeTestNode(n *specNode) (any, bool, error) {
	switch n.name {
	case "tval":
		return newTVal(), true, nil
	case "tzero":
		return T{}, true, nil
	case "tzeroptr":
		return &T{}, true, nil
	case "tree":
		t, err := decodeTree(n)
		return t, true, err
	case "st", "pst":
		v, err := decodeStruct(n)
		if err != nil {
			return nil, true, err
		}
		if n.name == "pst" {
			p := reflect.New(v.Type())
			p.Elem().Set(v)
			return p.Interface(), true, nil
		}
		return v.Interface(), true, nil
	case "bufstr":
		s, err := unhex(n.payload)
		return bytes.NewBufferString(s), true, err
	case "newint":
		return new(int), true, nil
	case "badm":
		return &badMarshaler{}, true, nil
	case "goodm":
		return &goodMarshaler{}, true, nil
	case "issue7379", "mystringer", "errorer", "vval", "vptr", "wval", "wptr":
		i, err := strconv.Atoi(n.payload)
		if err != nil {
			return nil, true, err
		}
		switch n.name {
		case "issue7379":
			return Issue7379(i), true, nil
		case "mystringer":
			return &myStringer{i}, true, nil
		case "errorer":
			return &errorer{i}, true, nil
		case "vval":
			return V{i}, true, nil
		case "vptr":
			return &V{i}, true, nil
		case "wval":
			return W{i}, true, nil
		}
		return &W{i}, true, nil
	case "lvval":
		var vs []V
		for _, c := range n.children {
			i, err := strconv.Atoi(c.payload)
			if err != nil {
				return nil, true, err
			}
			vs = append(vs, V{i})
		}
		return vs, true, nil
	case "dataitem", "dataitemv":
		d, err := decodeDataItem(n)
		if err != nil {
			return nil, true, err
		}
		if n.name == "dataitemv" {
			return *d, true, nil
		}
		return d, true, nil
	case "cplx":
		parts := strings.SplitN(n.payload, ";", 2)
		re, err := strconv.ParseUint(parts[0], 16, 64)
		if err != nil {
			return nil, true, err
		}
		im, err := strconv.ParseUint(parts[1], 16, 64)
		if err != nil {
			return nil, true, err
		}
		return complex(math.Float64frombits(re), math.Float64frombits(im)), true, nil
	case "fnplusone":
		return func(n int) int { return n + 1 }, true, nil
	case "ivalptr":
		iv := I(newTVal())
		return &iv, true, nil
	case "seven":
		return sevenStruct{7, "seven"}, true, nil
	}
	return nil, false, nil
}

func decodeTree(n *specNode) (*Tree, error) {
	if n.name == "nil" {
		return nil, nil
	}
	if n.name != "tree" || len(n.children) != 2 {
		return nil, fmt.Errorf("bad tree node %q", n.name)
	}
	val, err := strconv.Atoi(n.payload)
	if err != nil {
		return nil, err
	}
	l, err := decodeTree(n.children[0])
	if err != nil {
		return nil, err
	}
	r, err := decodeTree(n.children[1])
	if err != nil {
		return nil, err
	}
	return &Tree{val, l, r}, nil
}

func decodeDataItem(n *specNode) (*dataItem, error) {
	x, err := unhex(n.payload)
	if err != nil {
		return nil, err
	}
	d := &dataItem{X: x}
	for _, c := range n.children {
		cd, err := decodeDataItem(c)
		if err != nil {
			return nil, err
		}
		d.Children = append(d.Children, cd)
	}
	return d, nil
}

var anyType = reflect.TypeFor[any]()

func decodeStruct(n *specNode) (reflect.Value, error) {
	var fields []reflect.StructField
	var vals []reflect.Value
	for _, c := range n.children {
		if (c.name != "f" && c.name != "fa") || len(c.children) != 1 {
			return reflect.Value{}, fmt.Errorf("bad struct field %q", c.name)
		}
		name, err := unhex(c.payload)
		if err != nil {
			return reflect.Value{}, err
		}
		v, err := decodeNode(c.children[0])
		if err != nil {
			return reflect.Value{}, err
		}
		typ := anyType
		switch {
		case c.name == "f" && c.children[0].name == "tnil":
			// The field has the typed nil's type (also for interface types,
			// whose zero value is a nil any).
			if typ, err = regType(c.children[0].payload); err != nil {
				return reflect.Value{}, err
			}
		case c.name == "f" && v != nil:
			typ = reflect.TypeOf(v)
		}
		fields = append(fields, reflect.StructField{Name: name, Type: typ})
		rv := reflect.New(typ).Elem()
		if v != nil {
			rv.Set(reflect.ValueOf(v))
		}
		vals = append(vals, rv)
	}
	st := reflect.New(reflect.StructOf(fields)).Elem()
	for i, v := range vals {
		st.Field(i).Set(v)
	}
	return st, nil
}
