//go:build go1.27

package main

import (
	"bytes"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"math"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"
)

// The "gotests" mode extracts the test tables of Go's own unicode, utf8,
// utf16, strings and bytes tests (the *_test.go files of go1.27.1) by
// parsing them and evaluating the table literals, and dumps them to
// tests/fixtures/go_test_tables.txt. The Rust test go_test_tables.rs replays
// each table with the semantics of the Go test that uses it, so the Rust port
// runs Go's own test data.
//
// Row format: table name, then one field per tab. Field encodings:
//
//	s<hex>        string
//	i<decimal>    integer / rune
//	bT, bF        bool
//	n             nil
//	d<name>       named value (predicate, e.g. "IsSpace" or "not IsSpace")
//	L<f>,<f>,...  list (elements encoded the same way; "L" alone is empty)
//	k<name>=<f>   keyed struct field

type tval struct {
	kind string // s, i, b, n, d, L, k
	s    string
	i    int64
	b    bool
	list []tval
	key  string
	kv   *tval
}

func (v tval) encode() string {
	switch v.kind {
	case "s":
		return "s" + fmt.Sprintf("%x", v.s)
	case "i":
		return "i" + strconv.FormatInt(v.i, 10)
	case "b":
		if v.b {
			return "bT"
		}
		return "bF"
	case "n":
		return "n"
	case "d":
		return "d" + v.s
	case "L":
		parts := make([]string, len(v.list))
		for i, e := range v.list {
			parts[i] = e.encode()
		}
		return "L" + strings.Join(parts, ",")
	case "k":
		return "k" + v.key + "=" + v.kv.encode()
	}
	panic("bad kind " + v.kind)
}

type evaluator struct {
	decls map[string]ast.Expr // file-level var/const initializers
	depth int
}

var pkgConsts = map[string]int64{
	"UpperCase":       unicode.UpperCase,
	"LowerCase":       unicode.LowerCase,
	"TitleCase":       unicode.TitleCase,
	"MaxCase":         unicode.MaxCase,
	"MaxRune":         unicode.MaxRune,
	"ReplacementChar": unicode.ReplacementChar,
	"MaxASCII":        unicode.MaxASCII,
	"MaxLatin1":       unicode.MaxLatin1,
	"RuneError":       utf8.RuneError,
	"RuneSelf":        utf8.RuneSelf,
	"UTFMax":          utf8.UTFMax,
	"MaxInt":          math.MaxInt,
}

func (ev *evaluator) eval(e ast.Expr) (tval, error) {
	ev.depth++
	defer func() { ev.depth-- }()
	if ev.depth > 50 {
		return tval{}, fmt.Errorf("recursion")
	}
	switch x := e.(type) {
	case *ast.BasicLit:
		switch x.Kind {
		case token.STRING:
			s, err := strconv.Unquote(x.Value)
			return tval{kind: "s", s: s}, err
		case token.CHAR:
			r, _, _, err := strconv.UnquoteChar(x.Value[1:len(x.Value)-1], '\'')
			return tval{kind: "i", i: int64(r)}, err
		case token.INT:
			i, err := strconv.ParseInt(x.Value, 0, 64)
			return tval{kind: "i", i: i}, err
		}
	case *ast.ParenExpr:
		return ev.eval(x.X)
	case *ast.Ident:
		switch x.Name {
		case "true":
			return tval{kind: "b", b: true}, nil
		case "false":
			return tval{kind: "b", b: false}, nil
		case "nil":
			return tval{kind: "n"}, nil
		}
		if v, ok := pkgConsts[x.Name]; ok {
			return tval{kind: "i", i: v}, nil
		}
		if d, ok := ev.decls[x.Name]; ok {
			// predicate{unicode.IsSpace, "IsSpace"} -> its name.
			if cl, ok := d.(*ast.CompositeLit); ok {
				if id, ok := cl.Type.(*ast.Ident); ok && id.Name == "predicate" {
					nv, err := ev.eval(cl.Elts[1])
					return tval{kind: "d", s: nv.s}, err
				}
			}
			return ev.eval(d)
		}
		return tval{}, fmt.Errorf("unknown identifier %s", x.Name)
	case *ast.SelectorExpr:
		if v, ok := pkgConsts[x.Sel.Name]; ok {
			return tval{kind: "i", i: v}, nil
		}
		return tval{}, fmt.Errorf("unknown selector %s", x.Sel.Name)
	case *ast.UnaryExpr:
		v, err := ev.eval(x.X)
		if err != nil {
			return v, err
		}
		if x.Op == token.SUB && v.kind == "i" {
			return tval{kind: "i", i: -v.i}, nil
		}
	case *ast.BinaryExpr:
		a, err := ev.eval(x.X)
		if err != nil {
			return a, err
		}
		b, err := ev.eval(x.Y)
		if err != nil {
			return b, err
		}
		if a.kind == "s" && b.kind == "s" && x.Op == token.ADD {
			return tval{kind: "s", s: a.s + b.s}, nil
		}
		if a.kind == "i" && b.kind == "i" {
			switch x.Op {
			case token.ADD:
				return tval{kind: "i", i: a.i + b.i}, nil
			case token.SUB:
				return tval{kind: "i", i: a.i - b.i}, nil
			case token.MUL:
				return tval{kind: "i", i: a.i * b.i}, nil
			case token.QUO:
				return tval{kind: "i", i: a.i / b.i}, nil
			case token.SHL:
				return tval{kind: "i", i: a.i << b.i}, nil
			case token.OR:
				return tval{kind: "i", i: a.i | b.i}, nil
			}
		}
	case *ast.SliceExpr:
		v, err := ev.eval(x.X)
		if err != nil || v.kind != "s" {
			return v, fmt.Errorf("slice of non-string")
		}
		lo, hi := int64(0), int64(len(v.s))
		if x.Low != nil {
			l, err := ev.eval(x.Low)
			if err != nil {
				return l, err
			}
			lo = l.i
		}
		if x.High != nil {
			h, err := ev.eval(x.High)
			if err != nil {
				return h, err
			}
			hi = h.i
		}
		return tval{kind: "s", s: v.s[lo:hi]}, nil
	case *ast.CallExpr:
		fname := ""
		switch f := x.Fun.(type) {
		case *ast.Ident:
			fname = f.Name
		case *ast.SelectorExpr:
			fname = f.Sel.Name
		case *ast.ArrayType:
			if id, ok := f.Elt.(*ast.Ident); ok && id.Name == "byte" {
				fname = "[]byte"
			}
		}
		args := make([]tval, len(x.Args))
		for i, a := range x.Args {
			v, err := ev.eval(a)
			if err != nil {
				return v, err
			}
			args[i] = v
		}
		switch fname {
		case "len":
			if args[0].kind == "s" {
				return tval{kind: "i", i: int64(len(args[0].s))}, nil
			}
		case "string", "[]byte":
			switch args[0].kind {
			case "s":
				return args[0], nil
			case "i":
				return tval{kind: "s", s: string(rune(args[0].i))}, nil
			case "L":
				// []byte{...} literal.
				var b []byte
				for _, e := range args[0].list {
					b = append(b, byte(e.i))
				}
				return tval{kind: "s", s: string(b)}, nil
			}
		case "rune":
			return args[0], nil
		case "Repeat":
			if args[0].kind == "s" && args[1].kind == "i" {
				return tval{kind: "s", s: strings.Repeat(args[0].s, int(args[1].i))}, nil
			}
		case "not":
			if args[0].kind == "d" {
				return tval{kind: "d", s: "not " + args[0].s}, nil
			}
		}
		return tval{}, fmt.Errorf("unsupported call %s", fname)
	case *ast.CompositeLit:
		var out []tval
		for _, el := range x.Elts {
			if kv, ok := el.(*ast.KeyValueExpr); ok {
				v, err := ev.eval(kv.Value)
				if err != nil {
					return v, err
				}
				key := ""
				switch k := kv.Key.(type) {
				case *ast.Ident:
					key = k.Name
				case *ast.BasicLit:
					key = k.Value
				}
				out = append(out, tval{kind: "k", key: key, kv: &v})
				continue
			}
			v, err := ev.eval(el)
			if err != nil {
				return v, err
			}
			out = append(out, v)
		}
		return tval{kind: "L", list: out}, nil
	}
	return tval{}, fmt.Errorf("unsupported expression %T", e)
}

// goTestTables lists, per test file, the tables to extract.
var goTestTables = []struct {
	file   string // relative to $GOROOT/src
	prefix string
	tables []string
}{
	{"unicode/letter_test.go", "unicode.", []string{"upperTest", "notupperTest", "letterTest", "notletterTest", "spaceTest", "caseTest", "simpleFoldTests"}},
	{"unicode/digit_test.go", "unicode.", []string{"testDigit", "testLetter"}},
	{"unicode/script_test.go", "unicode.", []string{"inCategoryTest", "inPropTest"}},
	{"unicode/utf8/utf8_test.go", "utf8.", []string{"utf8map", "surrogateMap", "testStrings", "invalidSequenceTests", "runecounttests", "runelentests", "validTests", "validrunetests"}},
	{"unicode/utf16/utf16_test.go", "utf16.", []string{"encodeTests", "decodeTests", "decodeRuneTests", "surrogateTests"}},
	{"strings/strings_test.go", "strings.", []string{
		"indexTests", "lastIndexTests", "indexAnyTests", "lastIndexAnyTests", "splittests", "splitaftertests",
		"fieldstests", "FieldsFuncTests", "upperTests", "lowerTests", "trimSpaceTests", "trimTests",
		"trimFuncTests", "indexFuncTests", "RepeatTests", "RunesTests", "ReplaceTests", "TitleTests",
		"ContainsTests", "ContainsAnyTests", "ContainsRuneTests", "EqualFoldTests", "CountTests",
		"cutTests", "cutPrefixTests", "cutSuffixTests",
	}},
	{"bytes/bytes_test.go", "bytes.", []string{
		"indexTests", "lastIndexTests", "indexAnyTests", "lastIndexAnyTests", "splittests", "splitaftertests",
		"fieldstests", "upperTests", "lowerTests", "trimSpaceTests", "RepeatTests", "RunesTests", "trimTests",
		"trimFuncTests", "indexFuncTests", "ReplaceTests", "TitleTests", "ToTitleTests", "EqualFoldTests",
		"cutTests", "cutPrefixTests", "cutSuffixTests", "containsTests", "ContainsAnyTests", "ContainsRuneTests",
	}},
}

func genGoTests(args []string) error {
	fs := flag.NewFlagSet("gotests", flag.ExitOnError)
	out := fs.String("out", "crates/go-unicode/tests/fixtures/go_test_tables.txt", "output file")
	if err := fs.Parse(args); err != nil {
		return err
	}
	var b bytes.Buffer
	b.WriteString("# Test tables extracted from go1.27.1 $GOROOT/src/{unicode,strings,bytes}/*_test.go by tools/go-oracle/go-unicode gotests.\n")
	for _, tf := range goTestTables {
		path := filepath.Join(runtime.GOROOT(), "src", tf.file)
		fset := token.NewFileSet()
		f, err := parser.ParseFile(fset, path, nil, 0)
		if err != nil {
			return err
		}
		ev := &evaluator{decls: map[string]ast.Expr{}}
		for _, d := range f.Decls {
			gd, ok := d.(*ast.GenDecl)
			if !ok || (gd.Tok != token.VAR && gd.Tok != token.CONST) {
				continue
			}
			for _, sp := range gd.Specs {
				vs := sp.(*ast.ValueSpec)
				for i, n := range vs.Names {
					if i < len(vs.Values) {
						ev.decls[n.Name] = vs.Values[i]
					}
				}
			}
		}
		for _, name := range tf.tables {
			d, ok := ev.decls[name]
			if !ok {
				return fmt.Errorf("%s: table %s not found", tf.file, name)
			}
			cl, ok := d.(*ast.CompositeLit)
			if !ok {
				return fmt.Errorf("%s: %s is not a composite literal", tf.file, name)
			}
			rows, skipped := 0, 0
			for _, el := range cl.Elts {
				v, err := ev.eval(el)
				if err != nil {
					skipped++
					fmt.Fprintf(os.Stderr, "%s%s: skip row at %s: %v\n", tf.prefix, name, fset.Position(el.Pos()), err)
					continue
				}
				fields := []tval{v}
				if v.kind == "L" && isRow(el) {
					fields = v.list
				}
				b.WriteString(tf.prefix + name)
				for _, fv := range fields {
					b.WriteByte('\t')
					b.WriteString(fv.encode())
				}
				b.WriteByte('\n')
				rows++
			}
			fmt.Fprintf(os.Stderr, "%s%s: %d rows, %d skipped\n", tf.prefix, name, rows, skipped)
		}
	}
	return os.WriteFile(*out, b.Bytes(), 0o644)
}

// isRow reports whether a table element is a struct row (untyped or struct
// typed composite literal) rather than a scalar or slice value.
func isRow(e ast.Expr) bool {
	cl, ok := e.(*ast.CompositeLit)
	if !ok {
		return false
	}
	if at, ok := cl.Type.(*ast.ArrayType); ok && at != nil {
		return false
	}
	return true
}
