// Command plural is the Go oracle for the CLDR plural rules of go-i18n
// (internal/plural: Operands, Rules.Rule, the generated rule_gen.go) as
// crates/nh-i18n ports them (Wave B task T17).
//
//	go run ./tools/go-oracle/nh-i18n/plural [-out rust/testdata/oracle/i18n/plural]
//
// internal/plural cannot be imported, so every rule is exercised through the
// public API the way Hugo reaches it: a Bundle with English as default
// language, a message with all six plural forms ("zero", "one", ...) added for
// the locale's tag, and Localizer.LocalizeWithTag with a PluralCount. The
// output is the chosen form (or the error text).
//
// Inputs:
//   - every locale id of rule_gen.go (except "root"), found by parsing the Go
//     source, against a fixed list of counts: Go ints -1..120 and powers of ten,
//     int64/int8, decimal strings (0.0..3.0, trailing zeros, CLDR edge cases),
//     negative strings and invalid values (floats, "", "1e3", "abc", ...);
//   - the CLDR samples of rule_gen_test.go (appendIntegerTests /
//     appendDecimalTests, expanded like expandExamples), with the expected
//     form of the Go test, for every locale of each test function.
//
// Output: plural.json.gz. Nothing here depends on the platform.
package main

import (
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"log"
	"os/exec"
	"path/filepath"
	"strconv"
	"strings"

	"github.com/gohugoio/go-i18n/v2/i18n"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"golang.org/x/text/language"
)

func main() {
	out := flag.String("out", "rust/testdata/oracle/i18n/plural", "output directory")
	flag.Parse()

	dir := modDir("github.com/gohugoio/go-i18n/v2")
	ids := ruleIDs(filepath.Join(dir, "internal", "plural", "rule_gen.go"))
	samples := ruleSamples(filepath.Join(dir, "internal", "plural", "rule_gen_test.go"))

	counts := countList()
	var cases []map[string]any
	for _, id := range ids {
		if id == "root" {
			continue
		}
		var res []any
		for _, c := range counts {
			res = append(res, localize(id, c.v))
		}
		cases = append(cases, map[string]any{"kind": "grid", "locale": id, "results": res})
	}
	for _, s := range samples {
		for _, id := range s.locales {
			if id == "root" {
				continue
			}
			var res []any
			for _, t := range s.tests {
				res = append(res, localize(id, t.num))
			}
			cases = append(cases, map[string]any{"kind": "sample", "test": s.name, "locale": id, "results": res, "tests": s.encoded()})
		}
	}
	var enc []any
	for _, c := range counts {
		enc = append(enc, c.enc)
	}
	header := map[string]any{"counts": enc}
	if err := goval.WriteCasesGz(filepath.Join(*out, "plural.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Println("wrote", len(cases), "cases")
}

func modDir(mod string) string {
	out, err := exec.Command("go", "list", "-m", "-f", "{{.Dir}}", mod).Output()
	if err != nil {
		log.Fatalf("go list -m %s: %v", mod, err)
	}
	return strings.TrimSpace(string(out))
}

type count struct {
	v   any
	enc map[string]any
}

func countList() []count {
	var cs []count
	add := func(t string, v any, js any) {
		cs = append(cs, count{v, map[string]any{"t": t, "v": js}})
	}
	for i := -1; i <= 120; i++ {
		add("int", i, i)
	}
	for _, i := range []int{1000, 1001, 10000, 100000, 1000000, 1000001, 2000000, 10000000, 1000000000, -21, -101} {
		add("int", i, i)
	}
	add("int64", int64(21), 21)
	add("int8", int8(3), 3)
	for i := 0; i <= 30; i++ {
		s := fmt.Sprintf("%d.%d", i/10, i%10)
		add("string", s, s)
	}
	for _, s := range []string{
		"0.00", "0.01", "0.02", "0.04", "0.05", "0.10", "0.11", "0.2", "1.00", "1.000", "1.01", "1.10", "1.50",
		"2.00", "2.5", "10.0", "11.0", "21.0", "100.0", "101.0", "1000.0", "1000000.0", "1000000.00",
		"0", "1", "2", "3", "5", "11", "21", "101", "111", "1000000", "-1", "-1.5", "-0", "-0.0", "01", "007",
		"1.", ".5", "1e3", "1.2.3", " 1", "abc", "", "-", "1_000", "0x10", "9223372036854775807", "9223372036854775808",
		"1.12345678901234567890", "12345678901234567890.1",
	} {
		add("string", s, s)
	}
	add("float64", 1.5, 1.5)
	add("float64", 1.0, 1.0)
	add("uint", uint(3), 3)
	add("bool", true, true)
	add("nil", nil, nil)
	return cs
}

// localize returns the plural form chosen for n in the locale's bundle.
func localize(id string, n any) (out any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	tag := language.MustParse(id)
	b := i18n.NewBundle(language.English)
	m := &i18n.Message{ID: "m", Zero: "zero", One: "one", Two: "two", Few: "few", Many: "many", Other: "other"}
	if err := b.AddMessages(tag, m); err != nil {
		return map[string]any{"adderr": err.Error()}
	}
	l := i18n.NewLocalizer(b, tag.String())
	s, t, err := l.LocalizeWithTag(&i18n.LocalizeConfig{MessageID: "m", PluralCount: n})
	r := map[string]any{"s": s, "tag": t.String()}
	if err != nil {
		r["err"] = err.Error()
	}
	return r
}

func ruleIDs(path string) []string {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, path, nil, 0)
	if err != nil {
		log.Fatal(err)
	}
	var ids []string
	ast.Inspect(f, func(n ast.Node) bool {
		call, ok := n.(*ast.CallExpr)
		if !ok {
			return true
		}
		if id, ok := call.Fun.(*ast.Ident); !ok || id.Name != "addPluralRules" {
			return true
		}
		for _, e := range call.Args[1].(*ast.CompositeLit).Elts {
			s, _ := strconv.Unquote(e.(*ast.BasicLit).Value)
			ids = append(ids, s)
		}
		return true
	})
	return ids
}

type sampleTest struct {
	num  any // string or int64 (appendIntegerTests adds both)
	form string
}

type sample struct {
	name    string
	locales []string
	tests   []sampleTest
}

func (s sample) encoded() []any {
	var out []any
	for _, t := range s.tests {
		switch n := t.num.(type) {
		case string:
			out = append(out, map[string]any{"t": "string", "v": n, "form": t.form})
		case int64:
			out = append(out, map[string]any{"t": "int64", "v": n, "form": t.form})
		}
	}
	return out
}

// ruleSamples parses rule_gen_test.go: per test function the appended
// (form, examples) pairs and the locale list.
func ruleSamples(path string) []sample {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, path, nil, 0)
	if err != nil {
		log.Fatal(err)
	}
	var out []sample
	for _, d := range f.Decls {
		fd, ok := d.(*ast.FuncDecl)
		if !ok {
			continue
		}
		s := sample{name: fd.Name.Name}
		ast.Inspect(fd.Body, func(n ast.Node) bool {
			switch n := n.(type) {
			case *ast.CallExpr:
				id, ok := n.Fun.(*ast.Ident)
				if !ok || (id.Name != "appendIntegerTests" && id.Name != "appendDecimalTests") {
					return true
				}
				form := strings.ToLower(n.Args[1].(*ast.Ident).Name)
				var examples []string
				for _, e := range n.Args[2].(*ast.CompositeLit).Elts {
					v, _ := strconv.Unquote(e.(*ast.BasicLit).Value)
					examples = append(examples, v)
				}
				for _, ex := range expandExamples(examples) {
					if id.Name == "appendIntegerTests" {
						i, err := strconv.ParseInt(ex, 10, 64)
						if err != nil {
							log.Fatal(err)
						}
						s.tests = append(s.tests, sampleTest{ex, form}, sampleTest{i, form})
					} else {
						s.tests = append(s.tests, sampleTest{ex, form})
					}
				}
			case *ast.AssignStmt:
				if len(n.Lhs) == 1 {
					if id, ok := n.Lhs[0].(*ast.Ident); ok && id.Name == "locales" {
						for _, e := range n.Rhs[0].(*ast.CompositeLit).Elts {
							v, _ := strconv.Unquote(e.(*ast.BasicLit).Value)
							s.locales = append(s.locales, v)
						}
					}
				}
			}
			return true
		})
		if len(s.locales) > 0 {
			out = append(out, s)
		}
	}
	return out
}

// expandExamples and increment are copied from internal/plural/rule_test.go.
func expandExamples(examples []string) []string {
	var expanded []string
	for _, ex := range examples {
		if parts := strings.Split(ex, "~"); len(parts) == 2 {
			for ex := parts[0]; ; ex = increment(ex) {
				expanded = append(expanded, ex)
				if ex == parts[1] {
					break
				}
			}
		} else {
			expanded = append(expanded, ex)
		}
	}
	return expanded
}

func increment(dec string) string {
	runes := []rune(dec)
	carry := true
	for i := len(runes) - 1; carry && i >= 0; i-- {
		switch runes[i] {
		case '.':
			continue
		case '9':
			runes[i] = '0'
		default:
			runes[i]++
			carry = false
		}
	}
	if carry {
		runes = append([]rune{'1'}, runes...)
	}
	return string(runes)
}
