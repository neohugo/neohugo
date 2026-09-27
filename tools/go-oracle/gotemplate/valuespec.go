//go:build gotemplate_oracle

package main

import (
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"html/template"
	"math"
	"reflect"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/resources/page"
)

// The value spec language of tools/go-oracle/go-fmt/spec.go (copied here
// with extra host types for html/template), shared with
// crates/gotemplate/tests/common/mod.rs.
//
//	node    := name [':' payload] ['(' node {',' node} ')']
//	name    := [a-z0-9_]+
//	payload := any bytes except '(' ')' ','
//
// Nodes:
//
//	nil                              untyped nil (nil interface)
//	bool:1 | bool:0
//	int:D int8:D int16:D int32:D int64:D uint:D uint8:D uint16:D uint32:D uint64:D uintptr:D   (decimal)
//	f64:HEX16 f32:HEX8               IEEE bits
//	str:HEX                          string bytes
//	html: htmlattr: css: js: jsstr: url: srcset:   html/template content types (HEX bytes)
//	bytes:HEX                        non-nil []byte
//	tnil:HEX                         typed nil of the registered type named by HEX
//	list:HEX(nodes)                  slice of the registered type named by HEX
//	map:HEX(kv:HEXKEY(node),...)     map of the registered type named by HEX
//	time:SEC;NSEC;LOC                time.Unix(SEC, NSEC) in LOC: nil | utc | fixed=NAME=OFFSET | zone=IANA
//	obj_str:HEX  obj_err:HEX  obj_both:HEX  obj_sv:HEX  obj_gs:HEX
//	obj_plain(a,b) obj_pplain(a,b) obj_nm(kv...) obj_ns(nodes)
//	pages:N                          page.Pages of N nil elements (String: "Pages(N)")
//	taxlist:N                        page.TaxonomyList with keys t0..tN-1 of empty page.Taxonomy (String: "TaxonomyList(N)")
//
// Extra nodes for html/template (gotemplate oracle only):
//
//	obj_jm:HEX  obj_pjm:HEX          json.Marshaler returning HEX (value / pointer receiver)
//	obj_jme:HEX                      json.Marshaler (pointer receiver) failing with message HEX
//	obj_tm:HEX                       encoding.TextMarshaler (value receiver)
//	obj_hs:HEX                       named string type with String and PrintableValue (like hstring.HTML)
//	obj_pv(node) obj_ppv(node)       PrintableValueProvider struct / pointer to it (value receiver)
//	jnum:HEX                         json.Number

// Str is a pointer-receiver fmt.Stringer (Rust: Kind::Ptr object with go_string).
type Str struct{ S string }

func (s *Str) String() string { return s.S }

// Err is a pointer-receiver error (Rust: Kind::Ptr object with go_error).
type Err struct{ Msg string }

func (e *Err) Error() string { return e.Msg }

// Both is both an error and a Stringer; fmt prefers Error.
type Both struct{ S string }

func (b *Both) Error() string  { return "E:" + b.S }
func (b *Both) String() string { return "S:" + b.S }

// SV is a value-receiver Stringer struct (Rust: Kind::Struct object with go_string).
type SV struct{ S string }

func (s SV) String() string { return "SV(" + s.S + ")" }

// GS is a pointer-receiver fmt.GoStringer and fmt.Stringer (Rust: Kind::Ptr
// object with go_go_string and go_string).
type GS struct{ S string }

func (g *GS) GoString() string { return "GS(" + strconv.Quote(g.S) + ")" }
func (g *GS) String() string   { return g.S }

// Plain is a struct without methods (Rust: Kind::Struct / Kind::Ptr object with struct_fields).
type Plain struct {
	A any
	B any
}

// JM is a value-receiver json.Marshaler (Rust: Kind::Struct object with marshal_json).
type JM struct{ Raw string }

func (j JM) MarshalJSON() ([]byte, error) { return []byte(j.Raw), nil }

// PJM is a pointer-receiver json.Marshaler (Rust: Kind::Ptr object with marshal_json).
type PJM struct{ Raw string }

func (j *PJM) MarshalJSON() ([]byte, error) { return []byte(j.Raw), nil }

// JME is a json.Marshaler that fails (Rust: Kind::Ptr object whose marshal_json errs).
type JME struct{ Msg string }

func (j *JME) MarshalJSON() ([]byte, error) { return nil, errors.New(j.Msg) }

// TM is a value-receiver encoding.TextMarshaler (Rust: Kind::Struct object with marshal_text).
type TM struct{ T string }

func (t TM) MarshalText() ([]byte, error) { return []byte(t.T), nil }

// HS is a named string type like hstring.HTML: a Stringer whose
// PrintableValue is template.HTML (Rust: Kind::Struct object with go_string
// and printable_value).
type HS string

func (h HS) String() string { return string(h) }

// PrintableValue implements types.PrintableValueProvider.
func (h HS) PrintableValue() any { return template.HTML(h) }

// PV is a types.PrintableValueProvider with a value receiver (Rust:
// Kind::Struct object with printable_value; *PV is a Kind::Ptr object).
type PV struct{ V any }

// PrintableValue implements types.PrintableValueProvider.
func (p PV) PrintableValue() any { return p.V }

// NM is a named map type (Rust: Value::Map with MapType::Named or a Kind::Map object).
type NM map[string]any

// NS is a named slice type (Rust: Value::List with SliceType::Named or a Kind::Slice object).
type NS []any

// Iface is a named interface type (its nil prints as a nil interface).
type Iface interface{ M() }

// registry maps Go type strings to types for tnil/list/map nodes.
var registry = map[string]reflect.Type{}

func reg(vs ...any) {
	for _, v := range vs {
		t := reflect.TypeOf(v).Elem()
		registry[t.String()] = t
	}
}

func init() {
	reg(
		(*[]any)(nil), (*[]string)(nil), (*[]int)(nil), (*[]int8)(nil), (*[]int16)(nil), (*[]int32)(nil),
		(*[]int64)(nil), (*[]uint)(nil), (*[]uint8)(nil), (*[]uint16)(nil), (*[]uint32)(nil), (*[]uint64)(nil),
		(*[]uintptr)(nil), (*[]float32)(nil), (*[]float64)(nil), (*[]bool)(nil), (*[]map[string]any)(nil),
		(*[][]string)(nil), (*[][]any)(nil), (*[][]byte)(nil), (*[]*Plain)(nil), (*[]Iface)(nil), (*[]*int)(nil),
		(*[]template.HTML)(nil), (*[]error)(nil), (*[]fmt.Stringer)(nil),
		(*map[string]any)(nil), (*map[string]string)(nil), (*map[string]int)(nil), (*map[string]int64)(nil),
		(*map[string]float64)(nil), (*map[string]bool)(nil), (*map[string][]string)(nil), (*map[string]uint8)(nil),
		(*map[string]*Plain)(nil), (*map[string]Iface)(nil), (*map[string][]byte)(nil),
		(*maps.Params)(nil), (*NM)(nil), (*NS)(nil), (*page.Pages)(nil), (*page.TaxonomyList)(nil), (*page.Taxonomy)(nil),
		(**int)(nil), (**string)(nil), (**Plain)(nil), (**Str)(nil), (**Err)(nil), (**time.Location)(nil),
		(*func())(nil), (*chan int)(nil), (*Iface)(nil), (*error)(nil), (*fmt.Stringer)(nil),
	)
}

type specParser struct {
	s   string
	pos int
}

type specNode struct {
	name     string
	payload  string
	children []*specNode
}

func parseSpec(s string) (*specNode, error) {
	p := &specParser{s: s}
	n, err := p.node()
	if err != nil {
		return nil, err
	}
	if p.pos != len(p.s) {
		return nil, fmt.Errorf("trailing input at %d in %q", p.pos, s)
	}
	return n, nil
}

func (p *specParser) node() (*specNode, error) {
	start := p.pos
	for p.pos < len(p.s) {
		c := p.s[p.pos]
		if c >= 'a' && c <= 'z' || c >= '0' && c <= '9' || c == '_' {
			p.pos++
			continue
		}
		break
	}
	if p.pos == start {
		return nil, fmt.Errorf("expected name at %d in %q", p.pos, p.s)
	}
	n := &specNode{name: p.s[start:p.pos]}
	if p.pos < len(p.s) && p.s[p.pos] == ':' {
		p.pos++
		start = p.pos
		for p.pos < len(p.s) && p.s[p.pos] != '(' && p.s[p.pos] != ')' && p.s[p.pos] != ',' {
			p.pos++
		}
		n.payload = p.s[start:p.pos]
	}
	if p.pos < len(p.s) && p.s[p.pos] == '(' {
		p.pos++
		n.children = []*specNode{}
		if p.pos < len(p.s) && p.s[p.pos] == ')' {
			p.pos++
			return n, nil
		}
		for {
			c, err := p.node()
			if err != nil {
				return nil, err
			}
			n.children = append(n.children, c)
			if p.pos >= len(p.s) {
				return nil, fmt.Errorf("unterminated list in %q", p.s)
			}
			if p.s[p.pos] == ',' {
				p.pos++
				continue
			}
			if p.s[p.pos] == ')' {
				p.pos++
				break
			}
			return nil, fmt.Errorf("unexpected %q at %d in %q", p.s[p.pos], p.pos, p.s)
		}
	}
	return n, nil
}

func unhex(s string) (string, error) {
	b, err := hex.DecodeString(s)
	return string(b), err
}

func hexs(s string) string { return hex.EncodeToString([]byte(s)) }

// decodeSpec builds a fresh Go value (new allocations on every call).
func decodeSpec(s string) (any, error) {
	n, err := parseSpec(s)
	if err != nil {
		return nil, err
	}
	return decodeNode(n)
}

func decodeNode(n *specNode) (any, error) {
	switch n.name {
	case "nil":
		return nil, nil
	case "bool":
		return n.payload == "1", nil
	case "int", "int8", "int16", "int32", "int64":
		i, err := strconv.ParseInt(n.payload, 10, 64)
		if err != nil {
			return nil, err
		}
		switch n.name {
		case "int":
			return int(i), nil
		case "int8":
			return int8(i), nil
		case "int16":
			return int16(i), nil
		case "int32":
			return int32(i), nil
		}
		return i, nil
	case "uint", "uint8", "uint16", "uint32", "uint64", "uintptr":
		u, err := strconv.ParseUint(n.payload, 10, 64)
		if err != nil {
			return nil, err
		}
		switch n.name {
		case "uint":
			return uint(u), nil
		case "uint8":
			return uint8(u), nil
		case "uint16":
			return uint16(u), nil
		case "uint32":
			return uint32(u), nil
		case "uintptr":
			return uintptr(u), nil
		}
		return u, nil
	case "f64":
		b, err := strconv.ParseUint(n.payload, 16, 64)
		if err != nil {
			return nil, err
		}
		return math.Float64frombits(b), nil
	case "f32":
		b, err := strconv.ParseUint(n.payload, 16, 32)
		if err != nil {
			return nil, err
		}
		return math.Float32frombits(uint32(b)), nil
	case "str", "html", "htmlattr", "css", "js", "jsstr", "url", "srcset":
		s, err := unhex(n.payload)
		if err != nil {
			return nil, err
		}
		switch n.name {
		case "html":
			return template.HTML(s), nil
		case "htmlattr":
			return template.HTMLAttr(s), nil
		case "css":
			return template.CSS(s), nil
		case "js":
			return template.JS(s), nil
		case "jsstr":
			return template.JSStr(s), nil
		case "url":
			return template.URL(s), nil
		case "srcset":
			return template.Srcset(s), nil
		}
		return s, nil
	case "bytes":
		s, err := unhex(n.payload)
		if err != nil {
			return nil, err
		}
		return append([]byte{}, s...), nil
	case "tnil":
		t, err := regType(n.payload)
		if err != nil {
			return nil, err
		}
		return reflect.Zero(t).Interface(), nil
	case "list", "obj_ns":
		var t reflect.Type
		if n.name == "obj_ns" {
			t = reflect.TypeOf(NS(nil))
		} else {
			var err error
			if t, err = regType(n.payload); err != nil {
				return nil, err
			}
		}
		if t.Kind() != reflect.Slice {
			return nil, fmt.Errorf("list of non-slice type %s", t)
		}
		v := reflect.MakeSlice(t, len(n.children), len(n.children))
		for i, c := range n.children {
			e, err := decodeNode(c)
			if err != nil {
				return nil, err
			}
			if err := setElem(v.Index(i), e); err != nil {
				return nil, err
			}
		}
		return v.Interface(), nil
	case "map", "obj_nm":
		var t reflect.Type
		if n.name == "obj_nm" {
			t = reflect.TypeOf(NM(nil))
		} else {
			var err error
			if t, err = regType(n.payload); err != nil {
				return nil, err
			}
		}
		if t.Kind() != reflect.Map || t.Key().Kind() != reflect.String {
			return nil, fmt.Errorf("map of bad type %s", t)
		}
		v := reflect.MakeMap(t)
		for _, c := range n.children {
			if c.name != "kv" || len(c.children) != 1 {
				return nil, fmt.Errorf("bad map entry %q", c.name)
			}
			k, err := unhex(c.payload)
			if err != nil {
				return nil, err
			}
			e, err := decodeNode(c.children[0])
			if err != nil {
				return nil, err
			}
			ev := reflect.New(t.Elem()).Elem()
			if err := setElem(ev, e); err != nil {
				return nil, err
			}
			v.SetMapIndex(reflect.ValueOf(k).Convert(t.Key()), ev)
		}
		return v.Interface(), nil
	case "time":
		parts := strings.SplitN(n.payload, ";", 3)
		if len(parts) != 3 {
			return nil, fmt.Errorf("bad time %q", n.payload)
		}
		sec, err := strconv.ParseInt(parts[0], 10, 64)
		if err != nil {
			return nil, err
		}
		nsec, err := strconv.ParseInt(parts[1], 10, 64)
		if err != nil {
			return nil, err
		}
		t := time.Unix(sec, nsec)
		loc := parts[2]
		switch {
		case loc == "nil" || loc == "utc":
			return t.UTC(), nil
		case strings.HasPrefix(loc, "fixed="):
			f := strings.SplitN(loc, "=", 3)
			off, err := strconv.Atoi(f[2])
			if err != nil {
				return nil, err
			}
			return t.In(time.FixedZone(f[1], off)), nil
		case strings.HasPrefix(loc, "zone="):
			l, err := time.LoadLocation(loc[len("zone="):])
			if err != nil {
				return nil, err
			}
			return t.In(l), nil
		}
		return nil, fmt.Errorf("bad loc %q", loc)
	case "obj_str", "obj_err", "obj_both", "obj_sv", "obj_gs":
		s, err := unhex(n.payload)
		if err != nil {
			return nil, err
		}
		switch n.name {
		case "obj_str":
			return &Str{s}, nil
		case "obj_err":
			return &Err{s}, nil
		case "obj_both":
			return &Both{s}, nil
		case "obj_gs":
			return &GS{s}, nil
		}
		return SV{s}, nil
	case "obj_jm", "obj_pjm", "obj_jme", "obj_tm", "obj_hs", "jnum":
		s, err := unhex(n.payload)
		if err != nil {
			return nil, err
		}
		switch n.name {
		case "obj_jm":
			return JM{s}, nil
		case "obj_pjm":
			return &PJM{s}, nil
		case "obj_jme":
			return &JME{s}, nil
		case "obj_tm":
			return TM{s}, nil
		case "obj_hs":
			return HS(s), nil
		}
		return json.Number(s), nil
	case "obj_pv", "obj_ppv":
		if len(n.children) != 1 {
			return nil, fmt.Errorf("%s needs 1 child", n.name)
		}
		v, err := decodeNode(n.children[0])
		if err != nil {
			return nil, err
		}
		if n.name == "obj_pv" {
			return PV{v}, nil
		}
		return &PV{v}, nil
	case "pages":
		n, err := strconv.Atoi(n.payload)
		if err != nil {
			return nil, err
		}
		return make(page.Pages, n), nil
	case "taxlist":
		n, err := strconv.Atoi(n.payload)
		if err != nil {
			return nil, err
		}
		tl := page.TaxonomyList{}
		for i := 0; i < n; i++ {
			tl["t"+strconv.Itoa(i)] = page.Taxonomy{}
		}
		return tl, nil
	case "obj_plain", "obj_pplain":
		if len(n.children) != 2 {
			return nil, fmt.Errorf("plain needs 2 fields")
		}
		a, err := decodeNode(n.children[0])
		if err != nil {
			return nil, err
		}
		b, err := decodeNode(n.children[1])
		if err != nil {
			return nil, err
		}
		if n.name == "obj_plain" {
			return Plain{a, b}, nil
		}
		return &Plain{a, b}, nil
	}
	return nil, fmt.Errorf("unknown node %q", n.name)
}

func regType(h string) (reflect.Type, error) {
	name, err := unhex(h)
	if err != nil {
		return nil, err
	}
	t, ok := registry[name]
	if !ok {
		return nil, fmt.Errorf("unregistered type %q", name)
	}
	return t, nil
}

func setElem(dst reflect.Value, e any) error {
	if e == nil {
		dst.SetZero()
		return nil
	}
	v := reflect.ValueOf(e)
	if !v.Type().AssignableTo(dst.Type()) {
		return fmt.Errorf("cannot assign %s to %s", v.Type(), dst.Type())
	}
	dst.Set(v)
	return nil
}

// Spec builders used by the corpus.

func sInt(kind string, i int64) string   { return kind + ":" + strconv.FormatInt(i, 10) }
func sUint(kind string, u uint64) string { return kind + ":" + strconv.FormatUint(u, 10) }
func sF64(f float64) string              { return fmt.Sprintf("f64:%016x", math.Float64bits(f)) }
func sF32(f float32) string              { return fmt.Sprintf("f32:%08x", math.Float32bits(f)) }
func sStr(s string) string               { return "str:" + hexs(s) }
func sSafe(kind, s string) string        { return kind + ":" + hexs(s) }
func sBytes(s string) string             { return "bytes:" + hexs(s) }
func sTnil(t string) string              { return "tnil:" + hexs(t) }
func sList(t string, elems ...string) string {
	return "list:" + hexs(t) + "(" + strings.Join(elems, ",") + ")"
}
func sArgs(elems ...string) string { return "args(" + strings.Join(elems, ",") + ")" }

// sMap builds a map node; kvs alternates key, value-spec. Keys are sorted
// only for readability (map order does not matter).
func sMap(t string, kvs ...string) string {
	return "map:" + hexs(t) + "(" + kvList(kvs) + ")"
}

func kvList(kvs []string) string {
	type kv struct{ k, v string }
	var es []kv
	for i := 0; i+1 < len(kvs); i += 2 {
		es = append(es, kv{kvs[i], kvs[i+1]})
	}
	sort.Slice(es, func(i, j int) bool { return es[i].k < es[j].k })
	var parts []string
	for _, e := range es {
		parts = append(parts, "kv:"+hexs(e.k)+"("+e.v+")")
	}
	return strings.Join(parts, ",")
}

func sObjNM(kvs ...string) string { return "obj_nm(" + kvList(kvs) + ")" }
func sObjNS(elems ...string) string {
	return "obj_ns(" + strings.Join(elems, ",") + ")"
}
