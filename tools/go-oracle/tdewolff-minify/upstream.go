package main

import (
	"bytes"
	"go/ast"
	"go/parser"
	"go/token"
	"net/url"
	"path/filepath"
	"strconv"

	"github.com/tdewolff/minify/v2"
	"github.com/tdewolff/minify/v2/css"
	"github.com/tdewolff/minify/v2/html"
	"github.com/tdewolff/minify/v2/json"
	"github.com/tdewolff/minify/v2/svg"
	"github.com/tdewolff/minify/v2/xml"
	"github.com/tdewolff/parse/v2/buffer"
)

// litValue evaluates a string/int literal expression of a test table
// (string concatenation included). ok is false for anything else.
func litValue(e ast.Expr) (any, bool) {
	switch v := e.(type) {
	case *ast.BasicLit:
		switch v.Kind {
		case token.STRING:
			s, err := strconv.Unquote(v.Value)
			if err != nil {
				return nil, false
			}
			return s, true
		case token.INT:
			n, err := strconv.ParseInt(v.Value, 0, 64)
			if err != nil {
				return nil, false
			}
			return n, true
		}
	case *ast.ParenExpr:
		return litValue(v.X)
	case *ast.BinaryExpr:
		if v.Op == token.ADD {
			a, ok1 := litValue(v.X)
			b, ok2 := litValue(v.Y)
			if sa, ok := a.(string); ok && ok1 && ok2 {
				if sb, ok := b.(string); ok {
					return sa + sb, true
				}
			}
		}
	case *ast.UnaryExpr:
		if v.Op == token.SUB {
			a, ok := litValue(v.X)
			if n, ok2 := a.(int64); ok && ok2 {
				return -n, true
			}
		}
	}
	return nil, false
}

// tableRows returns the rows of the first `[]struct{...}{...}` table in the
// test function `name` of the given minify package test file.
func tableRows(pkg, file, name string) [][]any {
	fn := filepath.Join(minifyModuleDir(), pkg, file)
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, fn, nil, 0)
	if err != nil {
		panic(err)
	}
	var rows [][]any
	for _, decl := range f.Decls {
		fd, ok := decl.(*ast.FuncDecl)
		if !ok || fd.Name.Name != name {
			continue
		}
		found := false
		ast.Inspect(fd, func(n ast.Node) bool {
			if found {
				return false
			}
			cl, ok := n.(*ast.CompositeLit)
			if !ok {
				return true
			}
			at, ok := cl.Type.(*ast.ArrayType)
			if !ok {
				return true
			}
			if _, ok := at.Elt.(*ast.StructType); !ok {
				return true
			}
			found = true
			for _, el := range cl.Elts {
				row := el.(*ast.CompositeLit)
				var vals []any
				for _, fe := range row.Elts {
					v, ok := litValue(fe)
					if !ok {
						panic("non-literal field in " + pkg + "/" + name)
					}
					vals = append(vals, v)
				}
				rows = append(rows, vals)
			}
			return false
		})
		if !found {
			panic("no table in " + pkg + "/" + name)
		}
	}
	if rows == nil {
		panic("no rows for " + pkg + "/" + name)
	}
	return rows
}

// upstreamTest describes how one upstream table test runs its rows; the Rust
// test (tests/upstream.rs) implements the same dispatch by name.
type upstreamTest struct {
	pkg, file, name string
	// layout: "io" (input, expected), "uio" (url, input, expected),
	// "iao" (input, int arg, expected)
	layout string
	// run executes one row; state persists across the rows of one test.
	newRun func() func(arg int64, scheme string, in []byte) (out []byte, err error)
}

func minifierRun(m *minify.M, mf minify.Minifier, params map[string]string) func(int64, string, []byte) ([]byte, error) {
	return func(_ int64, _ string, in []byte) ([]byte, error) {
		w := &bytes.Buffer{}
		err := mf.Minify(m, w, bytes.NewBufferString(string(in)), params)
		return w.Bytes(), err
	}
}

var upstreamTests = []upstreamTest{
	{"html", "html_test.go", "TestHTML", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(configByName("t-copycssjs"), &html.Minifier{}, nil)
	}},
	{"html", "html_test.go", "TestHTMLCSSJS", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(configByName("t-htmlcsssvg"), &html.Minifier{}, nil)
	}},
	{"html", "html_test.go", "TestHTMLKeepEndTags", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &html.Minifier{KeepEndTags: true}, nil)
	}},
	{"html", "html_test.go", "TestHTMLKeepSpecialComments", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &html.Minifier{KeepSpecialComments: true}, nil)
	}},
	{"html", "html_test.go", "TestHTMLKeepWhitespace", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &html.Minifier{KeepWhitespace: true}, nil)
	}},
	{"html", "html_test.go", "TestHTMLKeepQuotes", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &html.Minifier{KeepQuotes: true}, nil)
	}},
	{"html", "html_test.go", "TestHTMLURL", "uio", func() func(int64, string, []byte) ([]byte, error) {
		m := configByName("t-html")
		return func(_ int64, u string, in []byte) ([]byte, error) {
			m.URL, _ = url.Parse(u)
			w := &bytes.Buffer{}
			err := html.Minify(m, w, bytes.NewBufferString(string(in)), nil)
			return w.Bytes(), err
		}
	}},
	{"html", "html_test.go", "TestHTMLGoTemplates", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(configByName("t-css"), &html.Minifier{TemplateDelims: html.GoTemplateDelims}, nil)
	}},
	{"html", "html_test.go", "TestHTMLPHPTemplates", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(configByName("t-css"), &html.Minifier{TemplateDelims: html.PHPTemplateDelims}, nil)
	}},
	{"css", "css_test.go", "TestCSS", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &css.Minifier{}, nil)
	}},
	{"css", "css_test.go", "TestCSSInline", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &css.Minifier{}, map[string]string{"inline": "1"})
	}},
	{"css", "css_test.go", "TestCSSKeepCSS2", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &css.Minifier{KeepCSS2: true}, map[string]string{"inline": "1"})
	}},
	{"json", "json_test.go", "TestJSON", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &json.Minifier{}, nil)
	}},
	{"json", "json_test.go", "TestJSON_IgnoreNumbers", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(nil, &json.Minifier{KeepNumbers: true}, nil)
	}},
	{"svg", "svg_test.go", "TestSVG", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &svg.Minifier{}, nil)
	}},
	{"svg", "svg_test.go", "TestSVGStyle", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(configByName("t-css"), &svg.Minifier{}, nil)
	}},
	{"svg", "svg_test.go", "TestSVGPrecision", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &svg.Minifier{Precision: 1}, nil)
	}},
	{"svg", "svg_test.go", "TestSVGInline", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &svg.Minifier{Inline: true}, nil)
	}},
	{"svg", "pathdata_test.go", "TestPathData", "io", func() func(int64, string, []byte) ([]byte, error) {
		p := svg.NewPathData(&svg.Minifier{})
		return func(_ int64, _ string, in []byte) ([]byte, error) {
			return p.ShortenPathData([]byte(string(in))), nil
		}
	}},
	{"svg", "pathdata_test.go", "TestPathDataTruncated", "io", func() func(int64, string, []byte) ([]byte, error) {
		p := svg.NewPathData(&svg.Minifier{Precision: 3})
		return func(_ int64, _ string, in []byte) ([]byte, error) {
			return p.ShortenPathData([]byte(string(in))), nil
		}
	}},
	{"xml", "xml_test.go", "TestXML", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &xml.Minifier{}, nil)
	}},
	{"xml", "xml_test.go", "TestXMLKeepWhitespace", "io", func() func(int64, string, []byte) ([]byte, error) {
		return minifierRun(minify.New(), &xml.Minifier{KeepWhitespace: true}, nil)
	}},
	{"", "common_test.go", "TestMediatype", "io", func() func(int64, string, []byte) ([]byte, error) {
		return func(_ int64, _ string, in []byte) ([]byte, error) { return minify.Mediatype([]byte(string(in))), nil }
	}},
	{"", "common_test.go", "TestDataURI", "io", func() func(int64, string, []byte) ([]byte, error) {
		m := configByName("t-datauri")
		return func(_ int64, _ string, in []byte) ([]byte, error) { return minify.DataURI(m, []byte(string(in))), nil }
	}},
	{"", "common_test.go", "TestDecimal", "io", func() func(int64, string, []byte) ([]byte, error) {
		return func(_ int64, _ string, in []byte) ([]byte, error) { return minify.Decimal([]byte(string(in)), -1), nil }
	}},
	{"", "common_test.go", "TestDecimalTruncate", "iao", func() func(int64, string, []byte) ([]byte, error) {
		return func(a int64, _ string, in []byte) ([]byte, error) {
			return minify.Decimal([]byte(string(in)), int(a)), nil
		}
	}},
	{"", "common_test.go", "TestNumber", "io", func() func(int64, string, []byte) ([]byte, error) {
		return func(_ int64, _ string, in []byte) ([]byte, error) { return minify.Number([]byte(string(in)), -1), nil }
	}},
	{"", "common_test.go", "TestNumberTruncate", "iao", func() func(int64, string, []byte) ([]byte, error) {
		return func(a int64, _ string, in []byte) ([]byte, error) {
			return minify.Number([]byte(string(in)), int(a)), nil
		}
	}},
}

// genUpstream writes one record per upstream table row:
//
//	up TEST IDX ARG SCHEME INPUT EXPECTED GOOUT GOERR
//
// ARG is the int argument ("-" if none), SCHEME the URL scheme of TestHTMLURL
// rows (as net/url parses it; "-" otherwise).
func genUpstream(w *fixWriter) {
	for _, ut := range upstreamTests {
		rows := tableRows(ut.pkg, ut.file, ut.name)
		run := ut.newRun()
		for i, row := range rows {
			var in, exp []byte
			arg := "-"
			scheme := "-"
			var argN int64
			rawURL := ""
			switch ut.layout {
			case "io":
				in, exp = []byte(row[0].(string)), []byte(row[1].(string))
			case "uio":
				rawURL = row[0].(string)
				u, _ := url.Parse(rawURL)
				if u != nil {
					scheme = hx([]byte(u.Scheme))
				}
				in, exp = []byte(row[1].(string)), []byte(row[2].(string))
			case "iao":
				in, exp = []byte(row[0].(string)), []byte(row[2].(string))
				argN = row[1].(int64)
				arg = strconv.FormatInt(argN, 10)
			}
			out, err := run(argN, rawURL, in)
			key := ut.pkg + "/" + ut.name
			if ut.pkg == "" {
				key = "minify/" + ut.name
			}
			w.rec("up", key, strconv.Itoa(i), arg, scheme, hx(in), hx(exp), hx(out), errStr(err))
		}
	}
	_ = buffer.NewReader
}
