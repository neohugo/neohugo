// Command pageparser is the Go oracle for the page lexer and front matter
// split of crates/nh-parser (Wave B task T03): parser/pageparser (ParseBytes
// with three configs, Item accessors, Iterator, IsProbablySourceOfItems,
// HasShortcode) and ParseFrontMatterAndContent with the decoded front matter
// (metadecoders, Go types recorded with ../tval).
//
//	go run ./tools/go-oracle/nh-parser/pageparser [-root .] [-out crates/nh-parser/tests/fixtures/pageparser]
//
// Inputs (the seeksnack site is private, so these substitute for its content):
//   - every .md/.markdown/.html/.gotmpl file under docs/content,
//     hugolib/testsite and create/skeletons;
//   - every string literal of parser/pageparser/*_test.go (the lexer's own test
//     inputs);
//   - the 218 seeksnack YAML front matters kept in
//     crates/go-yaml/tests/fixtures/seeksnack-fm.fixture.gz, wrapped in "---"
//     delimiters with a shortcode body;
//   - hand-written front matter shapes (YAML, TOML, JSON, org, BOM, CRLF,
//     unterminated, every value type);
//   - 4,000 seeded random token soups of shortcode syntax (nested, inline,
//     self-closing, unclosed, comments, quoted/raw/escaped parameters,
//     markdown/HTML delimiters, summary dividers, unicode, invalid UTF-8).
//
// Nothing here depends on the platform (no floats beyond strconv parsing).
package main

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"encoding/hex"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"io/fs"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strconv"
	"strings"

	"github.com/neohugo/neohugo/common/herrors"
	"github.com/neohugo/neohugo/parser/pageparser"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-parser/tval"
)

type input struct {
	name      string
	src       []byte
	generated bool
}

var roots = []string{"docs/content", "hugolib/testsite", "create/skeletons"}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-parser/tests/fixtures/pageparser", "output directory")
	soups := flag.Int("soups", 4000, "number of random shortcode token soups")
	seed := flag.Int64("seed", 20260928, "seed of the soups")
	flag.Parse()

	var inputs []input
	add := func(name string, src []byte, generated bool) {
		inputs = append(inputs, input{name: name, src: src, generated: generated})
	}

	// Repository content.
	for _, r := range roots {
		dir := filepath.Join(*root, r)
		err := filepath.WalkDir(dir, func(path string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if d.IsDir() {
				return nil
			}
			switch filepath.Ext(path) {
			case ".md", ".markdown", ".html", ".gotmpl":
			default:
				return nil
			}
			b, err := os.ReadFile(path)
			if err != nil {
				return err
			}
			rel, _ := filepath.Rel(*root, path)
			add("file:"+filepath.ToSlash(rel), b, false)
			return nil
		})
		if err != nil {
			log.Fatal(err)
		}
	}

	// The lexer's own test inputs.
	for _, s := range testLiterals(filepath.Join(*root, "parser/pageparser")) {
		add("test", []byte(s), true)
	}

	// Seeksnack front matter.
	for _, fm := range seeksnackFrontMatter(filepath.Join(*root, "crates/go-yaml/tests/fixtures/seeksnack-fm.fixture.gz")) {
		src := "---\n" + fm.src + "---\n\nSome text {{< youtube id=\"iIsZs0m-BVU\" >}}\n<!--more-->\nMore.\n"
		add("seeksnack:"+fm.name, []byte(src), false)
	}

	for _, s := range frontMatterShapes() {
		add("shape", []byte(s), true)
	}

	rnd := rand.New(rand.NewSource(*seed))
	for i := 0; i < *soups; i++ {
		add(fmt.Sprintf("soup#%d", i), []byte(soup(rnd)), true)
	}

	var cases []map[string]any
	for _, in := range inputs {
		cases = append(cases, lexCase(in))
	}

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	header := map[string]any{"oracle": "nh-parser/pageparser", "inputs": len(inputs)}
	if err := goval.WriteCasesGz(filepath.Join(*out, "pages.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "pageparser: %d inputs\n", len(inputs))
}

var configs = []pageparser.Config{{}, {NoFrontMatter: true}, {NoSummaryDivider: true}}

func lexCase(in input) map[string]any {
	c := map[string]any{"name": in.name, "src": goval.Str(string(in.src))}
	var lexes []any
	for ci, cfg := range configs {
		items, err := pageparser.ParseBytes(in.src, cfg)
		if err != nil {
			log.Fatalf("%s: ParseBytes: %v", in.name, err)
		}
		var enc []any
		for _, it := range items {
			enc = append(enc, encodeItem(in, it))
		}
		l := map[string]any{"items": enc, "probably": pageparser.IsProbablySourceOfItems(in.src, items)}
		if ci == 0 {
			// Iterator: LineNumber after each Next, and Consume/Backup/Peek round trips.
			it := pageparser.NewIterator(items)
			var lines []int
			for i := 0; i < len(items); i++ {
				it.Next()
				lines = append(lines, it.LineNumber(in.src))
			}
			l["lines"] = lines
			it2 := pageparser.NewIterator(items)
			it2.Consume(3)
			l["consume3"] = it2.Pos()
			l["valueNext"] = it2.IsValueNext()
		}
		lexes = append(lexes, l)
	}
	c["lex"] = lexes
	c["hasShortcode"] = pageparser.HasShortcode(string(in.src))

	cf, err := pageparser.ParseFrontMatterAndContent(bytes.NewReader(in.src))
	fm := map[string]any{"format": string(cf.FrontMatterFormat)}
	if cf.Content == nil {
		fm["content"] = nil
	} else {
		fm["content"] = len(in.src) - len(cf.Content)
	}
	if err != nil {
		fm["err"] = goval.Str(err.Error())
		if fe := herrors.UnwrapFileError(err); fe != nil {
			fm["cause"] = goval.Str(errorsUnwrap(err).Error())
		}
	} else {
		fm["fm"] = tval.Encode(cf.FrontMatter)
	}
	c["fm"] = fm
	return c
}

func errorsUnwrap(err error) error {
	if u, ok := err.(interface{ Unwrap() error }); ok && u.Unwrap() != nil {
		return u.Unwrap()
	}
	return err
}

// encodeItem returns [type, low, high, firstByte, isString, segments, err, pos, valTyped, toString, val].
func encodeItem(in input, it pageparser.Item) any {
	rv := reflect.ValueOf(it)
	var segs any
	if s := rv.FieldByName("segments"); s.Len() > 0 {
		var ss [][2]int64
		for i := 0; i < s.Len(); i++ {
			ss = append(ss, [2]int64{s.Index(i).Field(0).Int(), s.Index(i).Field(1).Int()})
		}
		segs = ss
	}
	var errMsg any
	if it.Err != nil {
		errMsg = goval.Str(it.Err.Error())
	}
	var typed any
	if it.IsShortcodeParam() || it.IsShortcodeParamVal() || it.IsShortcodeName() || it.IsText() {
		typed = tval.Encode(it.ValTyped(in.src))
	}
	var ts, val any
	if in.generated || segs != nil {
		ts = goval.Str(it.ToString(in.src))
		val = goval.Str(string(it.Val(in.src)))
	}
	return []any{
		int(it.Type), rv.FieldByName("low").Int(), rv.FieldByName("high").Int(),
		rv.FieldByName("firstByte").Uint(), rv.FieldByName("isString").Bool(),
		segs, errMsg, it.Pos(), typed, ts, val,
	}
}

// testLiterals returns the string literals of the lexer's Go tests.
func testLiterals(dir string) []string {
	fset := token.NewFileSet()
	files, err := filepath.Glob(filepath.Join(dir, "*_test.go"))
	if err != nil {
		log.Fatal(err)
	}
	sort.Strings(files)
	seen := map[string]bool{}
	var out []string
	for _, f := range files {
		af, err := parser.ParseFile(fset, f, nil, 0)
		if err != nil {
			log.Fatal(err)
		}
		ast.Inspect(af, func(n ast.Node) bool {
			bl, ok := n.(*ast.BasicLit)
			if !ok || bl.Kind != token.STRING {
				return true
			}
			s, err := strconv.Unquote(bl.Value)
			if err != nil || seen[s] {
				return true
			}
			seen[s] = true
			out = append(out, s)
			return true
		})
	}
	return out
}

type namedFM struct {
	name, src string
}

// seeksnackFrontMatter reads the "fm:" rows (name, hex source) of the go-yaml fixture.
func seeksnackFrontMatter(path string) []namedFM {
	f, err := os.Open(path)
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = f.Close() }()
	zr, err := gzip.NewReader(f)
	if err != nil {
		log.Fatal(err)
	}
	sc := bufio.NewScanner(zr)
	sc.Buffer(make([]byte, 16<<20), 16<<20)
	var out []namedFM
	for sc.Scan() {
		cols := strings.Split(sc.Text(), "\t")
		if len(cols) < 2 || !strings.HasPrefix(cols[0], "fm:") {
			continue
		}
		b, err := hex.DecodeString(cols[1])
		if err != nil {
			log.Fatal(err)
		}
		out = append(out, namedFM{name: strings.TrimPrefix(cols[0], "fm:"), src: string(b)})
	}
	if err := sc.Err(); err != nil {
		log.Fatal(err)
	}
	return out
}

// frontMatterShapes are hand-written pages covering every front matter kind and value type.
func frontMatterShapes() []string {
	yamlBody := `title: "Hello"
int: 42
negative: -17
big: 9223372036854775807
bigger: 9223372036854775808
huge: 18446744073709551616
small: -9223372036854775808
octal: 0o17
oldoctal: 017
hex: 0x1F
float: 3.14159
exp: 1e10
inf: .inf
nan: .NaN
bool: true
yes: yes
nulled: ~
empty:
date: 2020-05-17T15:05:09.238Z
dateonly: 2020-05-17
list: [a, b, 1, 2.5, true, null]
nested:
  Key: value
  1: one
  true: yes
  list:
    - x: 1
    - y: [1, 2]
multi: |
  line one
  line two
folded: >
  folded
  text
anchors: &a {k: v}
alias: *a
`
	tomlBody := `title = "Hello"
int = 42
negative = -17
big = 9223372036854775807
small = -9223372036854775808
hex = 0xDEAD_BEEF
oct = 0o755
bin = 0b1101
under = 1_000_000
float = 3.14159
exp = 6.626e-34
neg0 = -0.0
inf = inf
ninf = -inf
nan = nan
bool = false
date = 2019-12-31T07:06:21.671Z
offset = 1979-05-27T00:32:00.999999-07:00
zero = 1979-05-27T00:32:00+00:00
lower = 1979-05-27t07:32:00z
space = 1979-05-27 07:32:00Z
localdt = 1979-05-27T07:32:00.123456789123
localdate = 1979-05-27
localtime = 07:32:00.5
leap = 1990-12-31T23:59:60Z
arr = [1, 2, 3]
mixed = ["a", 1, 2.5, true, 1979-05-27, [1, 2], {x = 1}]
empty = []
inline = {a = 1, "b c" = "d", e.f = 2}
ml = """
Roses are red\
   Violets are blue"""
lit = '''
C:\Users\
'''
esc = "tab\there \u00e9 \U0001F600 \e"
"quoted key" = 1
'lit key' = 2
dotted.key.here = 3
[table]
a = 1
[table.sub]
b = 2
[[aot]]
name = "first"
[[aot]]
name = "second"
[aot.sub]
c = 3
`
	jsonBody := `{
  "title": "Hello",
  "int": 42,
  "float": 3.5,
  "big": 9223372036854775808,
  "neg": -0,
  "bool": true,
  "null": null,
  "list": [1, "a", null, {"x": [true]}],
  "nested": {"Key": {"deep": 1e300}},
  "esc": "\u00e9\ud83d\ude00"
}`
	shapes := []string{
		"---\n" + yamlBody + "---\n\nBody\n",
		"+++\n" + tomlBody + "+++\n\nBody\n",
		jsonBody + "\n\nBody {{< x >}}\n",
		"---\r\n" + strings.ReplaceAll(yamlBody, "\n", "\r\n") + "---\r\n\r\nBody\r\n",
		"+++\r\n" + strings.ReplaceAll(tomlBody, "\n", "\r\n") + "+++\r\nBody\r\n",
		"\ufeff---\ntitle: bom\n---\nBody",
		"\ufeff\ufeff+++\ntitle = 'bom'\n+++\nBody",
		"  \n\n---\ntitle: leading space\n---\nBody",
		"\t---\ntitle: tab\n---\n",
		"---\ntitle: no end\n",
		"+++\ntitle = 'no end'",
		"--\ntitle: bad delim\n---\n",
		"-+-\n",
		"---",
		"---\n---\n",
		"---\n---",
		"+++\n+++\nBody",
		"---\n\n---\nBody",
		"---\nnull\n---\n",
		"---\n~\n---\n",
		"---\n[1, 2]\n---\n",
		"---\n: bad\n---\n",
		"---\nkey: [unclosed\n---\n",
		"---\ntitle: a\n----\n---\nx",
		"---\ntitle: a\n---- \n---\n",
		"---\ntitle: a\n --- \n---\n",
		"+++\ntitle = \n+++\n",
		"+++\na = 1\na = 2\n+++\n",
		"+++\n[a]\n[a]\n+++\n",
		"+++\nx = 99999999999999999999\n+++\n",
		"+++\nx = 1979-02-30\n+++\n",
		"{}\n",
		"{\n}\nBody",
		"{\"a\": {\"b\": \"}\"}}\nrest",
		"{\"a\": \"\\\"}\"}\n\nrest",
		"{\"unterminated\": 1\n",
		"{\"a\":1}\r\n\r\nBody",
		"null",
		"{\"a\": [1, 2}\n",
		"#+TITLE: Org title\n#+AUTHOR: Me\n#+DATE: <2020-01-02 Thu>\n#+TAGS[]: a b\n\n* Heading\n# more\nrest",
		"#+TITLE: only\n",
		"# not org\ncontent",
		"#+\n",
		"\n\n\n",
		"",
		"plain text only",
		"<!--more-->",
		"---\ntitle: x\n---\nSummary<!--more-->   \n\nRest <!--more--> again",
		"Summary\n\n<!--more-->\n\n{{< sc >}}\n",
		"---\nsummary: x\n---\n# more\n<!--more",
		"<!--more-->{{< a >}}{{% b %}}",
	}
	return shapes
}

var soupTokens = []string{
	"{{<", "{{%", ">}}", "%}}", "{{< ", " >}}", "{{% ", " %}}",
	"{{</*", "*/>}}", "{{%/*", "*/%}}", "{{< /", "{{% /", "/", "/*", "*/",
	" ", "  ", "\t", "\n", "\r\n", "\r", "\n\n",
	"name", "sc", "figure", "ns/sc", "my-sc", "a.inline", "b.inline", ".inline", "inline", "sc.x",
	"=", "\"", "\\\"", "`", "\\`", "\\", "\"quoted value\"", "\"with \\\"escaped\\\" quotes\"",
	"`raw`", "`raw \"with\" quotes`", "123", "-3.5", ".5", "+7", "1e3", "007", "true", "false",
	"param=", "p=\"v\"", "p=`v`", "p=v", "p= \"spaced\"", "p=\\\"esc\\\"", "=\"\"",
	"é", "ไทย", "日本", "\xff", "\xc3", "\u00a0", "\u2003", "\ufeff",
	"<!--more-->", "# more", "{{", "}}", "{", "}", ".", "-", "_", "a", "Z9",
	"text ", "Lorem ipsum dolor ", "*emphasis* ", "`code` ", "<div>", "</div>",
	"{{< name >}}", "{{< /name >}}", "{{% name %}}", "{{% /name %}}", "{{< name />}}",
	"{{< a.inline >}}", "{{< /a.inline >}}", "{{< a.inline />}}", "{{< sc \"pos\" >}}",
	"{{< sc k=\"v\" k2=`r` >}}", "{{< sc \"a\" k=\"v\" >}}", "{{< sc k=\"v\" \"a\" >}}",
	"{{< sc >}}{{< /sc >}}", "{{% sc %}}inner{{% /sc %}}", "{{< outer >}}{{< inner >}}{{< /inner >}}{{< /outer >}}",
	"---\n", "+++\n", "#+", "{\"a\":1}",
}

func soup(r *rand.Rand) string {
	var b strings.Builder
	if r.Intn(4) == 0 {
		switch r.Intn(4) {
		case 0:
			b.WriteString("---\ntitle: soup\n---\n")
		case 1:
			b.WriteString("+++\ntitle = \"soup\"\n+++\n")
		case 2:
			b.WriteString("{\"title\": \"soup\"}\n")
		default:
			b.WriteString("#+TITLE: soup\n")
		}
	}
	n := 1 + r.Intn(30)
	for i := 0; i < n; i++ {
		b.WriteString(soupTokens[r.Intn(len(soupTokens))])
	}
	return b.String()
}
