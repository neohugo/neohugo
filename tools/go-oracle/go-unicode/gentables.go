//go:build go1.27

package main

import (
	"bytes"
	"flag"
	"fmt"
	"go/ast"
	"go/format"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"unicode"
)

// parsedTable is a RangeTable literal parsed from tables.go.
type parsedTable struct {
	goName      string // e.g. "_Lu", "foldLl"
	r16         []unicode.Range16
	r32         []unicode.Range32
	latinOffset int
}

// exportedVar is an exported variable of type *RangeTable that aliases a table,
// e.g. `Lu = _Lu // Lu is the set of ...`.
type exportedVar struct {
	name    string
	target  string // name of the parsed table (e.g. "_Lu")
	comment string
}

type mapEntry struct {
	key   string
	value string // identifier (for *RangeTable maps) or string value
}

type parsedMap struct {
	name    string
	doc     string
	entries []mapEntry
	strVals bool // map[string]string
}

type caseRangeExpr struct {
	lo, hi   int64
	delta    [3]int64
	deltaSrc [3]string // Go source of each delta expression
}

type keyedByte struct {
	value   int64
	src     string // Go expression
	comment string
}

type unicodeSource struct {
	fset        *token.FileSet
	src         map[string][]byte // file name -> content
	version     string
	tables      map[string]*parsedTable
	tableOrder  []string
	exports     []exportedVar
	exportIndex map[string]int
	maps        []*parsedMap
	caseRanges  []caseRangeExpr
	caseDoc     string
	properties  [256]keyedByte
	asciiFold   [128]int64
	caseOrbit   [][2]int64
	turkish     []caseRangeExpr
	specialVars []exportedVar // TurkishCase, AzeriCase -> _TurkishCase
}

// constant values used in tables.go / casetables.go expressions.
var goConsts = map[string]int64{
	"pC":         1 << 0,
	"pP":         1 << 1,
	"pN":         1 << 2,
	"pS":         1 << 3,
	"pZ":         1 << 4,
	"pLu":        1 << 5,
	"pLl":        1 << 6,
	"pp":         1 << 7,
	"pg":         1<<7 | 1<<4,
	"pLo":        1<<6 | 1<<5,
	"pLmask":     1<<6 | 1<<5,
	"UpperLower": unicode.MaxRune + 1,
	"MaxRune":    unicode.MaxRune,
	"MaxLatin1":  unicode.MaxLatin1,
	"MaxASCII":   unicode.MaxASCII,
}

// Rust names for the Go constants above (pP and pp only differ by case).
var rustConsts = map[string]string{
	"pC":         "P_C",
	"pP":         "P_P",
	"pN":         "P_N",
	"pS":         "P_S",
	"pZ":         "P_Z",
	"pLu":        "P_LU",
	"pLl":        "P_LL",
	"pp":         "P_PRINT",
	"pg":         "P_G",
	"pLo":        "P_LO",
	"pLmask":     "P_LMASK",
	"UpperLower": "UPPER_LOWER",
}

func evalInt(e ast.Expr) (int64, error) {
	switch x := e.(type) {
	case *ast.BasicLit:
		switch x.Kind {
		case token.INT:
			return strconv.ParseInt(x.Value, 0, 64)
		case token.CHAR:
			v, _, _, err := strconv.UnquoteChar(x.Value[1:len(x.Value)-1], '\'')
			return int64(v), err
		}
	case *ast.Ident:
		if v, ok := goConsts[x.Name]; ok {
			return v, nil
		}
		return 0, fmt.Errorf("unknown identifier %s", x.Name)
	case *ast.ParenExpr:
		return evalInt(x.X)
	case *ast.UnaryExpr:
		v, err := evalInt(x.X)
		if err != nil {
			return 0, err
		}
		switch x.Op {
		case token.SUB:
			return -v, nil
		case token.ADD:
			return v, nil
		}
	case *ast.BinaryExpr:
		a, err := evalInt(x.X)
		if err != nil {
			return 0, err
		}
		b, err := evalInt(x.Y)
		if err != nil {
			return 0, err
		}
		switch x.Op {
		case token.ADD:
			return a + b, nil
		case token.SUB:
			return a - b, nil
		case token.MUL:
			return a * b, nil
		case token.OR:
			return a | b, nil
		case token.SHL:
			return a << b, nil
		}
	}
	return 0, fmt.Errorf("cannot evaluate %T", e)
}

// rustExpr prints a constant Go expression as Rust source.
func rustExpr(e ast.Expr) (string, error) {
	switch x := e.(type) {
	case *ast.BasicLit:
		if x.Kind == token.INT {
			return x.Value, nil
		}
		v, err := evalInt(x)
		return strconv.FormatInt(v, 10), err
	case *ast.Ident:
		if r, ok := rustConsts[x.Name]; ok {
			return r, nil
		}
		return "", fmt.Errorf("no rust name for %s", x.Name)
	case *ast.ParenExpr:
		s, err := rustExpr(x.X)
		return "(" + s + ")", err
	case *ast.UnaryExpr:
		s, err := rustExpr(x.X)
		return x.Op.String() + s, err
	case *ast.BinaryExpr:
		a, err := rustExpr(x.X)
		if err != nil {
			return "", err
		}
		b, err := rustExpr(x.Y)
		return a + " " + x.Op.String() + " " + b, err
	}
	return "", fmt.Errorf("cannot print %T", e)
}

func commentText(cg *ast.CommentGroup) string {
	if cg == nil {
		return ""
	}
	return strings.TrimSpace(cg.Text())
}

func parseRangeTable(name string, cl *ast.CompositeLit) (*parsedTable, error) {
	t := &parsedTable{goName: name}
	for _, el := range cl.Elts {
		kv, ok := el.(*ast.KeyValueExpr)
		if !ok {
			return nil, fmt.Errorf("%s: unkeyed RangeTable field", name)
		}
		key := kv.Key.(*ast.Ident).Name
		switch key {
		case "R16", "R32":
			lit := kv.Value.(*ast.CompositeLit)
			for _, re := range lit.Elts {
				rl := re.(*ast.CompositeLit)
				if len(rl.Elts) != 3 {
					return nil, fmt.Errorf("%s: bad range", name)
				}
				var v [3]int64
				for i := range 3 {
					x, err := evalInt(rl.Elts[i])
					if err != nil {
						return nil, err
					}
					v[i] = x
				}
				if key == "R16" {
					t.r16 = append(t.r16, unicode.Range16{Lo: uint16(v[0]), Hi: uint16(v[1]), Stride: uint16(v[2])})
				} else {
					t.r32 = append(t.r32, unicode.Range32{Lo: uint32(v[0]), Hi: uint32(v[1]), Stride: uint32(v[2])})
				}
			}
		case "LatinOffset":
			v, err := evalInt(kv.Value)
			if err != nil {
				return nil, err
			}
			t.latinOffset = int(v)
		default:
			return nil, fmt.Errorf("%s: unknown field %s", name, key)
		}
	}
	return t, nil
}

func parseCaseRange(e ast.Expr) (caseRangeExpr, error) {
	var cr caseRangeExpr
	cl := e.(*ast.CompositeLit)
	if len(cl.Elts) != 3 {
		return cr, fmt.Errorf("bad CaseRange")
	}
	var err error
	if cr.lo, err = evalInt(cl.Elts[0]); err != nil {
		return cr, err
	}
	if cr.hi, err = evalInt(cl.Elts[1]); err != nil {
		return cr, err
	}
	d := cl.Elts[2].(*ast.CompositeLit)
	if len(d.Elts) != 3 {
		return cr, fmt.Errorf("bad delta")
	}
	for i := range 3 {
		if cr.delta[i], err = evalInt(d.Elts[i]); err != nil {
			return cr, err
		}
		if cr.deltaSrc[i], err = rustExpr(d.Elts[i]); err != nil {
			return cr, err
		}
	}
	return cr, nil
}

func loadUnicodeSource() (*unicodeSource, error) {
	dir := filepath.Join(runtime.GOROOT(), "src", "unicode")
	us := &unicodeSource{
		fset:        token.NewFileSet(),
		src:         map[string][]byte{},
		tables:      map[string]*parsedTable{},
		exportIndex: map[string]int{},
	}
	for _, fn := range []string{"tables.go", "casetables.go"} {
		path := filepath.Join(dir, fn)
		data, err := os.ReadFile(path)
		if err != nil {
			return nil, err
		}
		us.src[fn] = data
		f, err := parser.ParseFile(us.fset, path, data, parser.ParseComments)
		if err != nil {
			return nil, err
		}
		if err := us.parseFile(f); err != nil {
			return nil, fmt.Errorf("%s: %w", fn, err)
		}
	}
	return us, nil
}

func (us *unicodeSource) parseFile(f *ast.File) error {
	for _, decl := range f.Decls {
		gd, ok := decl.(*ast.GenDecl)
		if !ok {
			continue
		}
		if gd.Tok == token.CONST {
			for _, spec := range gd.Specs {
				vs := spec.(*ast.ValueSpec)
				for i, n := range vs.Names {
					if n.Name == "Version" {
						s, err := strconv.Unquote(vs.Values[i].(*ast.BasicLit).Value)
						if err != nil {
							return err
						}
						us.version = s
					}
				}
			}
			continue
		}
		if gd.Tok != token.VAR {
			continue
		}
		for _, spec := range gd.Specs {
			vs := spec.(*ast.ValueSpec)
			doc := commentText(vs.Doc)
			if doc == "" && len(gd.Specs) == 1 {
				doc = commentText(gd.Doc)
			}
			for i, n := range vs.Names {
				name := n.Name
				val := vs.Values[i]
				if err := us.parseVar(name, vs.Type, val, doc, commentText(vs.Comment)); err != nil {
					return fmt.Errorf("var %s: %w", name, err)
				}
			}
		}
	}
	return nil
}

func (us *unicodeSource) parseVar(name string, typ ast.Expr, val ast.Expr, doc, comment string) error {
	switch v := val.(type) {
	case *ast.UnaryExpr:
		cl, ok := v.X.(*ast.CompositeLit)
		if !ok || v.Op != token.AND {
			return fmt.Errorf("unexpected unary")
		}
		if id, ok := cl.Type.(*ast.Ident); !ok || id.Name != "RangeTable" {
			return fmt.Errorf("unexpected &T{}")
		}
		t, err := parseRangeTable(name, cl)
		if err != nil {
			return err
		}
		us.tables[name] = t
		us.tableOrder = append(us.tableOrder, name)
		return nil
	case *ast.Ident:
		if typ != nil {
			if id, ok := typ.(*ast.Ident); ok && id.Name == "SpecialCase" {
				us.specialVars = append(us.specialVars, exportedVar{name: name, target: v.Name, comment: comment})
				return nil
			}
		}
		if name == "CaseRanges" {
			if doc != "" {
				us.caseDoc = doc
			}
			return nil
		}
		if doc != "" && comment == "" {
			comment = doc
		}
		ev := exportedVar{name: name, target: v.Name, comment: comment}
		us.exports = append(us.exports, ev)
		us.exportIndex[name] = len(us.exports) - 1
		return nil
	case *ast.CompositeLit:
		switch t := v.Type.(type) {
		case *ast.MapType:
			m := &parsedMap{name: name, doc: doc}
			if id, ok := t.Value.(*ast.Ident); ok && id.Name == "string" {
				m.strVals = true
			}
			for _, el := range v.Elts {
				kv := el.(*ast.KeyValueExpr)
				key, err := strconv.Unquote(kv.Key.(*ast.BasicLit).Value)
				if err != nil {
					return err
				}
				var value string
				if m.strVals {
					value, err = strconv.Unquote(kv.Value.(*ast.BasicLit).Value)
					if err != nil {
						return err
					}
				} else {
					value = kv.Value.(*ast.Ident).Name
				}
				m.entries = append(m.entries, mapEntry{key, value})
			}
			us.maps = append(us.maps, m)
			return nil
		case *ast.Ident:
			if t.Name != "SpecialCase" {
				return fmt.Errorf("unexpected composite type %s", t.Name)
			}
			for _, el := range v.Elts {
				cr, err := parseCaseRange(el)
				if err != nil {
					return err
				}
				us.turkish = append(us.turkish, cr)
			}
			return nil
		case *ast.ArrayType:
			elt := t.Elt.(*ast.Ident).Name
			switch {
			case t.Len == nil && elt == "CaseRange":
				if doc != "" {
					us.caseDoc = doc
				}
				for _, el := range v.Elts {
					cr, err := parseCaseRange(el)
					if err != nil {
						return err
					}
					us.caseRanges = append(us.caseRanges, cr)
				}
				return nil
			case t.Len == nil && elt == "foldPair":
				for _, el := range v.Elts {
					cl := el.(*ast.CompositeLit)
					a, err := evalInt(cl.Elts[0])
					if err != nil {
						return err
					}
					b, err := evalInt(cl.Elts[1])
					if err != nil {
						return err
					}
					us.caseOrbit = append(us.caseOrbit, [2]int64{a, b})
				}
				return nil
			case t.Len != nil && elt == "uint8" && name == "properties":
				file := us.fset.File(v.Pos())
				src := us.src[filepath.Base(file.Name())]
				for _, el := range v.Elts {
					kv := el.(*ast.KeyValueExpr)
					k, err := evalInt(kv.Key)
					if err != nil {
						return err
					}
					val, err := evalInt(kv.Value)
					if err != nil {
						return err
					}
					rs, err := rustExpr(kv.Value)
					if err != nil {
						return err
					}
					// Trailing comment on this line (e.g. // 'ý').
					line := file.Line(kv.Pos())
					start := file.LineStart(line)
					end := len(src)
					if line < file.LineCount() {
						end = file.Offset(file.LineStart(line + 1))
					}
					text := string(src[file.Offset(start):end])
					cmt := ""
					if idx := strings.Index(text, "//"); idx >= 0 {
						cmt = strings.TrimSpace(text[idx+2:])
					}
					us.properties[k] = keyedByte{value: val, src: rs, comment: cmt}
				}
				return nil
			case t.Len != nil && elt == "uint16" && name == "asciiFold":
				if len(v.Elts) != 128 {
					return fmt.Errorf("asciiFold has %d elements", len(v.Elts))
				}
				for i, el := range v.Elts {
					x, err := evalInt(el)
					if err != nil {
						return err
					}
					us.asciiFold[i] = x
				}
				return nil
			}
			return fmt.Errorf("unexpected array var")
		}
	}
	return fmt.Errorf("unexpected value %T", val)
}

// resolve maps an exported or private identifier to its parsed table.
func (us *unicodeSource) resolve(ident string) (*parsedTable, error) {
	seen := 0
	for {
		if t, ok := us.tables[ident]; ok {
			return t, nil
		}
		idx, ok := us.exportIndex[ident]
		if !ok {
			return nil, fmt.Errorf("unresolved identifier %s", ident)
		}
		ident = us.exports[idx].target
		seen++
		if seen > 10 {
			return nil, fmt.Errorf("alias loop at %s", ident)
		}
	}
}

func (us *unicodeSource) mapByName(name string) *parsedMap {
	for _, m := range us.maps {
		if m.name == name {
			return m
		}
	}
	return nil
}

func sameTable(p *parsedTable, rt *unicode.RangeTable) bool {
	a16, b16 := p.r16, rt.R16
	if len(a16) == 0 && len(b16) == 0 {
		a16, b16 = nil, nil
	}
	a32, b32 := p.r32, rt.R32
	if len(a32) == 0 && len(b32) == 0 {
		a32, b32 = nil, nil
	}
	return reflect.DeepEqual(a16, b16) && reflect.DeepEqual(a32, b32) && p.latinOffset == rt.LatinOffset
}

// verify cross-checks every parsed value against the compiled unicode package.
func (us *unicodeSource) verify() error {
	if us.version != unicode.Version {
		return fmt.Errorf("version %q != unicode.Version %q", us.version, unicode.Version)
	}
	// Exported variables.
	if len(exportedTables) != len(us.exports) {
		return fmt.Errorf("exports_gen.go is stale (%d vs %d exports); run genexports", len(exportedTables), len(us.exports))
	}
	for _, ev := range us.exports {
		rt, ok := exportedTables[ev.name]
		if !ok {
			return fmt.Errorf("exports_gen.go lacks %s; run genexports", ev.name)
		}
		p, err := us.resolve(ev.name)
		if err != nil {
			return err
		}
		if !sameTable(p, rt) {
			return fmt.Errorf("parsed table for %s differs from compiled unicode.%s", ev.name, ev.name)
		}
	}
	// Maps.
	runtimeMaps := map[string]map[string]*unicode.RangeTable{
		"Categories":   unicode.Categories,
		"Scripts":      unicode.Scripts,
		"Properties":   unicode.Properties,
		"FoldCategory": unicode.FoldCategory,
		"FoldScript":   unicode.FoldScript,
	}
	for name, rm := range runtimeMaps {
		pm := us.mapByName(name)
		if pm == nil {
			return fmt.Errorf("map %s not parsed", name)
		}
		if len(pm.entries) != len(rm) {
			return fmt.Errorf("map %s: %d parsed entries vs %d compiled", name, len(pm.entries), len(rm))
		}
		for _, e := range pm.entries {
			p, err := us.resolve(e.value)
			if err != nil {
				return err
			}
			rt, ok := rm[e.key]
			if !ok || !sameTable(p, rt) {
				return fmt.Errorf("map %s[%q] differs from compiled", name, e.key)
			}
		}
	}
	ca := us.mapByName("CategoryAliases")
	if ca == nil || len(ca.entries) != len(unicode.CategoryAliases) {
		return fmt.Errorf("CategoryAliases mismatch")
	}
	for _, e := range ca.entries {
		if unicode.CategoryAliases[e.key] != e.value {
			return fmt.Errorf("CategoryAliases[%q] mismatch", e.key)
		}
	}
	// CaseRanges.
	if len(us.caseRanges) != len(unicode.CaseRanges) {
		return fmt.Errorf("CaseRanges length mismatch")
	}
	for i, cr := range us.caseRanges {
		rc := unicode.CaseRanges[i]
		if uint32(cr.lo) != rc.Lo || uint32(cr.hi) != rc.Hi {
			return fmt.Errorf("CaseRanges[%d] mismatch", i)
		}
		for j := range 3 {
			if rune(cr.delta[j]) != rc.Delta[j] {
				return fmt.Errorf("CaseRanges[%d].Delta mismatch", i)
			}
		}
	}
	// TurkishCase.
	if len(us.turkish) != len(unicode.TurkishCase) {
		return fmt.Errorf("TurkishCase length mismatch")
	}
	for i, cr := range us.turkish {
		rc := unicode.TurkishCase[i]
		if uint32(cr.lo) != rc.Lo || uint32(cr.hi) != rc.Hi {
			return fmt.Errorf("TurkishCase[%d] mismatch", i)
		}
		for j := range 3 {
			if rune(cr.delta[j]) != rc.Delta[j] {
				return fmt.Errorf("TurkishCase[%d].Delta mismatch", i)
			}
		}
	}
	// properties (unexported): check through the exported predicates.
	for c := range 256 {
		p := us.properties[c].value
		r := rune(c)
		if (p&goConsts["pC"] != 0) != unicode.IsControl(r) ||
			(p&goConsts["pP"] != 0) != unicode.IsPunct(r) ||
			(p&goConsts["pN"] != 0) != unicode.IsNumber(r) ||
			(p&goConsts["pS"] != 0) != unicode.IsSymbol(r) ||
			(p&goConsts["pp"] != 0) != unicode.IsPrint(r) ||
			(p&goConsts["pg"] != 0) != unicode.IsGraphic(r) ||
			(p&goConsts["pLmask"] == goConsts["pLu"]) != unicode.IsUpper(r) ||
			(p&goConsts["pLmask"] == goConsts["pLl"]) != unicode.IsLower(r) ||
			(p&goConsts["pLmask"] != 0) != unicode.IsLetter(r) ||
			(p&goConsts["pZ"] != 0) != unicode.Is(unicode.Z, r) {
			return fmt.Errorf("properties[%#x] inconsistent with compiled predicates", c)
		}
	}
	// asciiFold and caseOrbit (unexported): check through SimpleFold.
	for c := range 128 {
		if rune(us.asciiFold[c]) != unicode.SimpleFold(rune(c)) {
			return fmt.Errorf("asciiFold[%#x] inconsistent with SimpleFold", c)
		}
	}
	for _, fp := range us.caseOrbit {
		if fp[0] >= 128 && unicode.SimpleFold(rune(fp[0])) != rune(fp[1]) {
			return fmt.Errorf("caseOrbit %#x inconsistent with SimpleFold", fp[0])
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// Rust emission.

func rustTableName(goName string) string {
	switch {
	case strings.HasPrefix(goName, "_"):
		return "TAB" + strings.ToUpper(goName)
	case strings.HasPrefix(goName, "fold"):
		return "FOLD_" + strings.ToUpper(goName[4:])
	}
	panic("unexpected table name " + goName)
}

func rustExportName(goName string) string {
	return strings.ToUpper(goName)
}

var rustMapNames = map[string]string{
	"Categories":      "CATEGORIES",
	"CategoryAliases": "CATEGORY_ALIASES",
	"Scripts":         "SCRIPTS",
	"Properties":      "PROPERTIES",
	"FoldCategory":    "FOLD_CATEGORY",
	"FoldScript":      "FOLD_SCRIPT",
}

func writeDoc(b *bytes.Buffer, indent, doc string) {
	for _, l := range strings.Split(doc, "\n") {
		l = strings.TrimRight(l, " ")
		if l == "" {
			fmt.Fprintf(b, "%s///\n", indent)
		} else {
			fmt.Fprintf(b, "%s/// %s\n", indent, l)
		}
	}
}

func (us *unicodeSource) emitRust() ([]byte, error) {
	var b bytes.Buffer
	used := map[string]string{}
	claim := func(rust, goName string) error {
		if prev, ok := used[rust]; ok {
			return fmt.Errorf("rust name %s used by both %s and %s", rust, prev, goName)
		}
		used[rust] = goName
		return nil
	}

	fmt.Fprintf(&b, "// Code generated by tools/go-oracle/go-unicode (mode \"tables\") from\n")
	fmt.Fprintf(&b, "// %s $GOROOT/src/unicode/tables.go and casetables.go. DO NOT EDIT.\n", runtime.Version())
	b.WriteString(`//
// Go: src/unicode/tables.go, src/unicode/casetables.go

use crate::{CaseRange, FoldPair, Range16, Range32, RangeTable, Rune, SpecialCase, UPPER_LOWER};
#[allow(unused_imports)]
use crate::{P_C, P_G, P_LL, P_LMASK, P_LO, P_LU, P_N, P_P, P_PRINT, P_S, P_Z};

const fn r16(lo: u16, hi: u16, stride: u16) -> Range16 {
    Range16 { lo, hi, stride }
}

const fn r32(lo: u32, hi: u32, stride: u32) -> Range32 {
    Range32 { lo, hi, stride }
}

const fn cr(lo: u32, hi: u32, delta: [Rune; 3]) -> CaseRange {
    CaseRange { lo, hi, delta }
}

const fn fp(from: u16, to: u16) -> FoldPair {
    FoldPair { from, to }
}

`)
	b.WriteString("/// Version is the Unicode edition from which the tables are derived.\n")
	fmt.Fprintf(&b, "pub const VERSION: &str = %q;\n\n", us.version)

	// Maps, in source order, sorted by key (byte order) for binary search.
	for _, m := range us.maps {
		rn, ok := rustMapNames[m.name]
		if !ok {
			return nil, fmt.Errorf("no rust name for map %s", m.name)
		}
		if err := claim(rn, m.name); err != nil {
			return nil, err
		}
		entries := append([]mapEntry(nil), m.entries...)
		sort.Slice(entries, func(i, j int) bool { return entries[i].key < entries[j].key })
		if m.doc != "" {
			writeDoc(&b, "", m.doc)
		}
		b.WriteString("///\n/// Go map; here a slice sorted by key (byte order). Look up with [`crate::lookup`].\n")
		if m.strVals {
			fmt.Fprintf(&b, "pub static %s: &[(&str, &str)] = &[\n", rn)
			for _, e := range entries {
				fmt.Fprintf(&b, "    (%q, %q),\n", e.key, e.value)
			}
		} else {
			fmt.Fprintf(&b, "pub static %s: &[(&str, &RangeTable)] = &[\n", rn)
			for _, e := range entries {
				p, err := us.resolve(e.value)
				if err != nil {
					return nil, err
				}
				fmt.Fprintf(&b, "    (%q, &%s),\n", e.key, rustTableName(p.goName))
			}
		}
		b.WriteString("];\n\n")
	}

	// Exported aliases.
	for _, ev := range us.exports {
		rn := rustExportName(ev.name)
		if err := claim(rn, ev.name); err != nil {
			return nil, err
		}
		p, err := us.resolve(ev.name)
		if err != nil {
			return nil, err
		}
		if ev.comment != "" {
			writeDoc(&b, "", ev.comment)
		} else {
			fmt.Fprintf(&b, "/// Go: unicode.%s\n", ev.name)
		}
		fmt.Fprintf(&b, "pub static %s: &RangeTable = &%s;\n", rn, rustTableName(p.goName))
	}
	b.WriteString("\n")

	// All exported names (test support).
	b.WriteString("/// Every exported `*RangeTable` variable of Go's unicode package, by Go name,\n")
	b.WriteString("/// in source order (test support; not a Go API).\n")
	b.WriteString("#[doc(hidden)]\npub static ALL_EXPORTED_TABLES: &[(&str, &RangeTable)] = &[\n")
	for _, ev := range us.exports {
		p, _ := us.resolve(ev.name)
		fmt.Fprintf(&b, "    (%q, &%s),\n", ev.name, rustTableName(p.goName))
	}
	b.WriteString("];\n\n")

	// Tables.
	for _, name := range us.tableOrder {
		t := us.tables[name]
		rn := rustTableName(name)
		if err := claim(rn, name); err != nil {
			return nil, err
		}
		fmt.Fprintf(&b, "pub(crate) static %s: RangeTable = RangeTable {\n", rn)
		if len(t.r16) == 0 {
			b.WriteString("    r16: &[],\n")
		} else {
			b.WriteString("    r16: &[\n")
			for _, r := range t.r16 {
				fmt.Fprintf(&b, "        r16(0x%04x, 0x%04x, %d),\n", r.Lo, r.Hi, r.Stride)
			}
			b.WriteString("    ],\n")
		}
		if len(t.r32) == 0 {
			b.WriteString("    r32: &[],\n")
		} else {
			b.WriteString("    r32: &[\n")
			for _, r := range t.r32 {
				fmt.Fprintf(&b, "        r32(0x%x, 0x%x, %d),\n", r.Lo, r.Hi, r.Stride)
			}
			b.WriteString("    ],\n")
		}
		fmt.Fprintf(&b, "    latin_offset: %d,\n", t.latinOffset)
		b.WriteString("};\n\n")
	}

	// CaseRanges.
	if us.caseDoc != "" {
		writeDoc(&b, "", us.caseDoc)
	}
	b.WriteString("pub static CASE_RANGES: &[CaseRange] = &TAB_CASE_RANGES;\n")
	fmt.Fprintf(&b, "static TAB_CASE_RANGES: [CaseRange; %d] = [\n", len(us.caseRanges))
	for _, c := range us.caseRanges {
		fmt.Fprintf(&b, "    cr(0x%04X, 0x%04X, [%s, %s, %s]),\n", c.lo, c.hi, c.deltaSrc[0], c.deltaSrc[1], c.deltaSrc[2])
	}
	b.WriteString("];\n\n")

	// properties.
	for _, n := range []string{"LATIN1_PROPERTIES", "ASCII_FOLD", "CASE_ORBIT", "TAB_CASE_RANGES", "CASE_RANGES", "TAB_TURKISH_CASE", "TURKISH_CASE", "AZERI_CASE", "ALL_EXPORTED_TABLES", "VERSION"} {
		if err := claim(n, "(fixed)"); err != nil {
			return nil, err
		}
	}
	b.WriteString("/// Go: unicode.properties (renamed: Rust statics are case-insensitive to Go's\n/// `properties`/`Properties` distinction).\n")
	b.WriteString("pub(crate) static LATIN1_PROPERTIES: [u8; 256] = [\n")
	for c := range 256 {
		p := us.properties[c]
		src := p.src
		if src == "" {
			src = "0"
		}
		if p.comment != "" {
			fmt.Fprintf(&b, "    %s, // 0x%02X %s\n", src, c, p.comment)
		} else {
			fmt.Fprintf(&b, "    %s, // 0x%02X\n", src, c)
		}
	}
	b.WriteString("];\n\n")

	// asciiFold.
	b.WriteString("pub(crate) static ASCII_FOLD: [u16; 128] = [\n")
	for c := range 128 {
		fmt.Fprintf(&b, "    0x%04X,\n", us.asciiFold[c])
	}
	b.WriteString("];\n\n")

	// caseOrbit.
	fmt.Fprintf(&b, "pub(crate) static CASE_ORBIT: [FoldPair; %d] = [\n", len(us.caseOrbit))
	for _, p := range us.caseOrbit {
		fmt.Fprintf(&b, "    fp(0x%04X, 0x%04X),\n", p[0], p[1])
	}
	b.WriteString("];\n\n")

	// TurkishCase / AzeriCase.
	fmt.Fprintf(&b, "static TAB_TURKISH_CASE: [CaseRange; %d] = [\n", len(us.turkish))
	for _, c := range us.turkish {
		fmt.Fprintf(&b, "    cr(0x%04X, 0x%04X, [%s, %s, %s]),\n", c.lo, c.hi, c.deltaSrc[0], c.deltaSrc[1], c.deltaSrc[2])
	}
	b.WriteString("];\n\n")
	for _, sv := range us.specialVars {
		if sv.target != "_TurkishCase" {
			return nil, fmt.Errorf("unexpected SpecialCase target %s", sv.target)
		}
		fmt.Fprintf(&b, "/// Go: unicode.%s\n", sv.name)
		if sv.comment != "" {
			writeDoc(&b, "", sv.comment)
		}
		fmt.Fprintf(&b, "pub static %s: SpecialCase<'static> = SpecialCase(&TAB_TURKISH_CASE);\n", strings.ToUpper(camelToSnake(sv.name)))
	}
	return b.Bytes(), nil
}

func camelToSnake(s string) string {
	var out []rune
	for i, r := range s {
		if i > 0 && unicode.IsUpper(r) {
			out = append(out, '_')
		}
		out = append(out, r)
	}
	return string(out)
}

func genTables(args []string) error {
	fs := flag.NewFlagSet("tables", flag.ExitOnError)
	out := fs.String("out", "crates/go-unicode/src/tables.rs", "output Rust file")
	if err := fs.Parse(args); err != nil {
		return err
	}
	us, err := loadUnicodeSource()
	if err != nil {
		return err
	}
	if err := us.verify(); err != nil {
		return err
	}
	data, err := us.emitRust()
	if err != nil {
		return err
	}
	return os.WriteFile(*out, data, 0o644)
}

func genExports(args []string) error {
	fs := flag.NewFlagSet("genexports", flag.ExitOnError)
	out := fs.String("out", "tools/go-oracle/go-unicode/exports_gen.go", "output Go file")
	if err := fs.Parse(args); err != nil {
		return err
	}
	us, err := loadUnicodeSource()
	if err != nil {
		return err
	}
	var b bytes.Buffer
	b.WriteString("// Code generated by \"go run ./tools/go-oracle/go-unicode genexports\". DO NOT EDIT.\n\n")
	b.WriteString("//go:build go1.27\n\n")
	b.WriteString("package main\n\nimport \"unicode\"\n\n")
	b.WriteString("// exportedTables maps every exported *unicode.RangeTable variable to its value.\n")
	b.WriteString("var exportedTables = map[string]*unicode.RangeTable{\n")
	for _, ev := range us.exports {
		fmt.Fprintf(&b, "\t%q: unicode.%s,\n", ev.name, ev.name)
	}
	b.WriteString("}\n")
	src, err := format.Source(b.Bytes())
	if err != nil {
		return err
	}
	return os.WriteFile(*out, src, 0o644)
}
