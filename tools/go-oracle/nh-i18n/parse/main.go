// Command parse is the Go oracle for go-i18n's message file parsing
// (i18n.ParseMessageFileBytes with the unmarshalers Hugo registers: toml =
// pelletier/go-toml/v2, yaml/yml = gopkg.in/yaml.v2, json = encoding/json)
// as crates/nh-i18n ports it in src/goi18n/{parse,message}.rs (Wave B T17).
//
//	go run ./tools/go-oracle/nh-i18n/parse [-out crates/nh-i18n/tests/fixtures/parse]
//
// Inputs: hand-written message files in TOML, YAML and JSON covering flat
// `key = "..."` messages, `[key] one/other` tables, nested namespaces, dotted
// keys, reserved keys in other cases, custom ids, delimiters, the v1
// `id`/`translation` arrays, HTML, Thai, empty files and every error path of
// recGetMessages/stringMap/stringSubmap, under a set of file paths (language
// tags, art-x-, unknown formats).
//
// go-i18n ranges over Go maps, so a few inputs (e.g. both `translation` and
// `other` in one message, or two errors in one map) give a random result.
// Every input is parsed 50 times and inputs with more than one outcome are
// left out (their count is recorded in the header).
//
// Output: parse.json.gz. Messages are sorted by id.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"log"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/gohugoio/go-i18n/v2/i18n"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	toml "github.com/pelletier/go-toml/v2"
	yaml "gopkg.in/yaml.v2"
)

type input struct {
	path    string
	content string
}

var inputs = []input{
	// TOML
	{"en.toml", "[hello]\nother = \"Hello, World!\""},
	{"en.toml", "hello = \"Hello\"\nbye = \"Bye\""},
	{"en.toml", "\"shop_nextPage.one\" = \"Show Me The Money\"\n"},
	{"en.toml", "foo.one =  \"abc\""},
	{"en.toml", "[readingTime]\none = \"One minute to read\"\nother = \"{{ .Count }} minutes to read\"\n"},
	{"en.toml", "[a]\n[a.b]\nother = \"x\"\n[a.c]\none = \"c1\"\n"},
	{"en.toml", "[a]\nother = \"x\"\n[a.b]\nother = \"y\"\n"},
	{"en.toml", "[x]\nOther = \"X\"\n"},
	{"en.toml", "[x]\nOTHER = \"X\"\nOne = \"o\"\n"},
	{"en.toml", "[x]\nid = \"custom\"\nother = \"y\"\n"},
	{"en.toml", "[x]\nid = \"custom\"\n"},
	{"en.toml", "[x]\nother = 5\n"},
	{"en.toml", "[x]\nother = \"a\"\none = 5\n"},
	{"en.toml", "[x]\nother = \"a\"\nfoo = [1, 2]\n"},
	{"en.toml", "[x]\nother = \"a\"\nfoo = 1.5\n"},
	{"en.toml", "[x]\nother = \"a\"\nfoo = true\n"},
	{"en.toml", "[x]\nother = \"a\"\ndescription = \"d\"\nhash = \"sha1-x\"\nleftDelim = \"<<\"\nrightDelim = \">>\"\nzero = \"z\"\ntwo = \"t\"\nfew = \"f\"\nmany = \"m\"\n"},
	{"en.toml", "[x]\nother = \"a\"\nignored = \"i\"\n"},
	{"en.toml", "a = 1\n"},
	{"en.toml", "a = true\n"},
	{"en.toml", "a = 1979-05-27\n"},
	{"en.toml", "a = 1979-05-27T07:32:00Z\n"},
	{"en.toml", "a = [\"x\", \"y\"]\n"},
	{"en.toml", "\"a.b.c\" = \"x\"\n[d]\n[d.e]\n[d.e.f]\nother = \"deep\"\n"},
	{"th.toml", "[categories]\nother = \"หมวดหมู่\"\n[servings]\nother = \"จำนวนหน่วยบริโภคต่อ{{ .Context }}\"\n"},
	{"en.toml", "[html]\nother = \"<b>bold</b> & 'q' \\\"dq\\\"\"\n"},
	{"en.toml", "[x]\nother = \"\"\none = \"\"\n"},
	{"en.toml", "[x]\nother = \"\"\n"},
	{"en.toml", ""},
	{"en.toml", "  \n\n"},
	{"en.toml", "# only a comment\n"},
	{"en.toml", "[x\nother = 1"},
	{"en.toml", "a = \"x\"\na = \"y\"\n"},
	{"en.toml", "[[x]]\nother = \"a\"\n[[x]]\nother = \"b\"\n"},
	{"en.toml", "[[x]]\nid = \"a\"\ntranslation = \"A\"\n[[x]]\nid = \"b\"\ntranslation = { one = \"b1\", other = \"bn\" }\n"},
	{"en.toml", "[x]\ntranslation = \"t\"\n"},
	{"en.toml", "[x]\nid = \"i\"\ntranslation = \"t\"\n"},
	{"en.toml", "[x]\nid = \"i\"\ntranslation = 5\n"},
	{"en.toml", "[x]\nid = \"i\"\ntranslation = { one = 1 }\n"},
	// YAML
	{"en.yaml", "hello: Hello\nbye: Bye\n"},
	{"en.yaml", "hello:\n  other: Hello\n  one: One\n"},
	{"en.yml", "ns:\n  sub:\n    other: nested\n  flat: f\n"},
	{"en.yaml", "1: x\n"},
	{"en.yaml", "x:\n  1: a\n"},
	{"en.yaml", "x:\n  other: a\n  1: b\n"},
	{"en.yaml", "- id: hello\n  translation: Hi\n- id: bye\n  translation:\n    one: One bye\n    other: Byes\n"},
	{"en.yaml", "- hello\n- world\n"},
	{"en.yaml", "- [a, b]\n"},
	{"en.yaml", "hello: ~\n"},
	{"en.yaml", "x:\n  other: a\n  one: ~\n"},
	{"en.yaml", "x: yes\n"},
	{"en.yaml", "x: 12\n"},
	{"en.yaml", "x: 1.5\n"},
	{"en.yaml", "hello\n"},
	{"en.yaml", "a: [\n"},
	{"en.yaml", "x:\n  other: \"{{ .Count }} items\"\n  one: \"{{ .Count }} item\"\n"},
	{"en.yaml", "x:\n  Other: X\n"},
	{"en.yaml", "x:\n  id: custom\n  other: y\n"},
	{"en.yaml", "x:\n  other: [a]\n"},
	{"en.yaml", "x:\n  other: a\n  one: [b]\n"},
	{"en.yaml", "x:\n  other: a\n  one: {k: v}\n"},
	{"en.yaml", "x:\n  translation:\n    1: a\n"},
	{"en.yaml", "- id: x\n  translation: ~\n"},
	{"en.yaml", ""},
	{"en.yaml", "---\n"},
	{"en.yaml", "~\n"},
	// JSON
	{"en.json", `{"hello": "Hi"}`},
	{"en.json", `{"hello": {"other": "Hi", "one": "One"}}`},
	{"en.json", `[{"id": "a", "translation": "b"}, {"id": "c", "translation": {"one": "c1", "other": "cn"}}]`},
	{"en.json", `"str"`},
	{"en.json", `5`},
	{"en.json", `null`},
	{"en.json", `true`},
	{"en.json", `[]`},
	{"en.json", `{}`},
	{"en.json", `{"a": 1}`},
	{"en.json", `{"x": {"other": "a", "one": 1}}`},
	{"en.json", `{"x": {"other": "a", "one": null}}`},
	{"en.json", `{"x": {"other": "a", "one": [1, "b"]}}`},
	{"en.json", `{"x": {"other": "a", "one": {"k": 1}}}`},
	{"en.json", `{"a": {"b": {"c": "abc"}}}`},
	{"en.json", `{"a": `},
	{"en.json", `{"a": "\u00e9\u0e01"}`},
	{"en.json", "{\"a\": \"x\"} trailing"},
	{"en.json", `{"a": "b", "a": "c"}`},
}

var paths = []string{
	"en.toml", "i18n/th.toml", "fr.yml", "pl.json", "pt-br.toml", "zh-CN.json", "en.all.toml", "klingon.toml",
	"art-x-klingon.toml", "foo.txt", "toml", ".toml", "en.TOML", "a/b/c/ar.json", "sr-Latn.yaml", "en-US.yaml",
	"x-foo.toml", "und.json", "art-x-art-x-oc.toml",
}

// goI18nTestFiles extracts the (file, path) pairs of go-i18n's tests from their
// source: composite literals with `file:` and `path:` string fields, and
// MustParseMessageFileBytes([]byte(file), path) calls.
func goI18nTestFiles() []input {
	out, err := exec.Command("go", "list", "-m", "-f", "{{.Dir}}", "github.com/gohugoio/go-i18n/v2").Output()
	if err != nil {
		log.Fatal(err)
	}
	dir := filepath.Join(strings.TrimSpace(string(out)), "i18n")
	var ins []input
	str := func(e ast.Expr) (string, bool) {
		if c, ok := e.(*ast.CallExpr); ok && len(c.Args) == 1 {
			e = c.Args[0] // []byte("...")
		}
		bl, ok := e.(*ast.BasicLit)
		if !ok || bl.Kind != token.STRING {
			return "", false
		}
		v, err := strconv.Unquote(bl.Value)
		return v, err == nil
	}
	for _, name := range []string{"parse_test.go", "bundle_test.go"} {
		f, err := parser.ParseFile(token.NewFileSet(), filepath.Join(dir, name), nil, 0)
		if err != nil {
			log.Fatal(err)
		}
		ast.Inspect(f, func(n ast.Node) bool {
			switch n := n.(type) {
			case *ast.CompositeLit:
				var file, path string
				var hasFile, hasPath bool
				for _, e := range n.Elts {
					kv, ok := e.(*ast.KeyValueExpr)
					if !ok {
						continue
					}
					k, ok := kv.Key.(*ast.Ident)
					if !ok {
						continue
					}
					switch k.Name {
					case "file":
						file, hasFile = str(kv.Value)
					case "path":
						path, hasPath = str(kv.Value)
					}
				}
				if hasFile && hasPath {
					ins = append(ins, input{path, file})
				}
			case *ast.CallExpr:
				sel, ok := n.Fun.(*ast.SelectorExpr)
				if !ok || sel.Sel.Name != "MustParseMessageFileBytes" || len(n.Args) != 2 {
					return true
				}
				file, ok1 := str(n.Args[0])
				path, ok2 := str(n.Args[1])
				if ok1 && ok2 {
					ins = append(ins, input{path, file})
				}
			}
			return true
		})
	}
	if len(ins) < 10 {
		log.Fatalf("only %d go-i18n test files found", len(ins))
	}
	return ins
}

func unmarshalers() map[string]i18n.UnmarshalFunc {
	return map[string]i18n.UnmarshalFunc{
		"toml": toml.Unmarshal,
		"yaml": yaml.Unmarshal,
		"yml":  yaml.Unmarshal,
		"json": json.Unmarshal,
	}
}

func parseOnce(buf []byte, path string) (out map[string]any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	mf, err := i18n.ParseMessageFileBytes(buf, path, unmarshalers())
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	var msgs []map[string]string
	for _, m := range mf.Messages {
		msgs = append(msgs, map[string]string{
			"id": m.ID, "hash": m.Hash, "description": m.Description, "leftDelim": m.LeftDelim, "rightDelim": m.RightDelim,
			"zero": m.Zero, "one": m.One, "two": m.Two, "few": m.Few, "many": m.Many, "other": m.Other,
		})
	}
	sort.SliceStable(msgs, func(i, j int) bool {
		a, _ := json.Marshal(msgs[i])
		b, _ := json.Marshal(msgs[j])
		return string(a) < string(b)
	})
	if msgs == nil {
		msgs = []map[string]string{}
	}
	return map[string]any{"tag": mf.Tag.String(), "format": mf.Format, "messages": msgs}
}

func main() {
	out := flag.String("out", "crates/nh-i18n/tests/fixtures/parse", "output directory")
	flag.Parse()

	var cases []map[string]any
	nondet := 0
	add := func(content, path string) {
		first := parseOnce([]byte(content), path)
		fb, _ := json.Marshal(first)
		for i := 0; i < 50; i++ {
			b, _ := json.Marshal(parseOnce([]byte(content), path))
			if string(b) != string(fb) {
				nondet++
				return
			}
		}
		cases = append(cases, map[string]any{"path": path, "content": content, "result": first})
	}
	for _, in := range inputs {
		add(in.content, in.path)
	}
	// The files of go-i18n's own tests (i18n/parse_test.go TestParseMessageFileBytes,
	// i18n/bundle_test.go TestJSON/TestYAML/TestTOML/TestV1Format/TestV1FlatFormat).
	for _, in := range goI18nTestFiles() {
		add(in.content, in.path)
	}
	// The path variants with a small valid file per format.
	for _, p := range paths {
		for _, c := range []string{"[hello]\nother = \"Hello\"\n", "hello: Hello\n", `{"hello": "Hello"}`} {
			add(c, p)
		}
	}
	header := map[string]any{"nondeterministic_inputs_left_out": nondet}
	if err := goval.WriteCasesGz(filepath.Join(*out, "parse.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Println("wrote", len(cases), "cases;", nondet, "nondeterministic left out")
}
