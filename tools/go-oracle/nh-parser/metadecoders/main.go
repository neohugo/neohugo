// Command metadecoders is the Go oracle for parser/metadecoders of
// crates/nh-parser (Wave B task T03): Decoder.UnmarshalToMap and Unmarshal for
// TOML (pelletier/go-toml/v2 v2.2.4, also called directly with toml.Unmarshal
// to record DecodeError positions and texts), YAML and JSON; UnmarshalStringTo;
// the format helpers.
//
//	go run ./tools/go-oracle/nh-parser/metadecoders [-root .] [-out rust/testdata/oracle/parser/metadecoders]
//
// Inputs (the seeksnack site is private, so these substitute for its
// hugo.toml, i18n and data files):
//   - every .toml/.yaml/.yml/.json file of this repository (docs/hugo.toml,
//     docs/data/**, test data, ...), outside rust/, tools/ and node_modules;
//   - every string literal of go-toml's own tests (the toml-test suite in
//     toml_testgen_test.go, unmarshaler_test.go, errors_test.go, ...) and its
//     fuzz corpus;
//   - hand-written documents covering every value type (ints at the int64
//     bounds, floats, inf/nan, offset/local dates and times, nested tables,
//     arrays of tables, multiline strings, escapes) and invalid documents;
//   - generated mutations: every prefix of the small valid documents, seeded
//     byte substitutions, and concatenations (duplicate keys and tables across
//     documents).
//
// Values are recorded with ../tval (Go types). Nothing is platform dependent
// (strconv parsing only).
package main

import (
	"errors"
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
	"sort"
	"strconv"
	"strings"

	"github.com/neohugo/neohugo/common/herrors"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/parser/metadecoders"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-parser/tval"
	toml "github.com/pelletier/go-toml/v2"
)

type doc struct {
	name   string
	format metadecoders.Format
	src    string
	// human records DecodeError.String() (kept small: hand-written and suite inputs only).
	human bool
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/parser/metadecoders", "output directory")
	gomodcache := flag.String("gomodcache", "", "GOMODCACHE (default: go env GOMODCACHE via $GOMODCACHE or ~/go/pkg/mod)")
	soups := flag.Int("soups", 3000, "number of random TOML token soups")
	seed := flag.Int64("seed", 20260928, "seed of the random mutations and soups")
	flag.Parse()

	modcache := *gomodcache
	if modcache == "" {
		modcache = os.Getenv("GOMODCACHE")
	}
	if modcache == "" {
		home, _ := os.UserHomeDir()
		modcache = filepath.Join(home, "go", "pkg", "mod")
	}
	tomlDir := filepath.Join(modcache, "github.com/pelletier/go-toml/v2@v2.2.4")

	var docs []doc
	seen := map[string]bool{}
	add := func(d doc) {
		key := string(d.format) + "\x00" + d.src
		if seen[key] {
			return
		}
		seen[key] = true
		docs = append(docs, d)
	}

	// Repository files.
	for _, f := range repoFiles(*root) {
		b, err := os.ReadFile(filepath.Join(*root, f))
		if err != nil {
			log.Fatal(err)
		}
		add(doc{name: "file:" + f, format: metadecoders.FormatFromString(f), src: string(b), human: true})
	}

	// go-toml's tests and fuzz corpus.
	var suite []string
	for _, f := range []string{"toml_testgen_test.go", "unmarshaler_test.go", "errors_test.go", "localtime_test.go", "decode_test.go", "fast_test.go", "strict_test.go", "fuzz_test.go"} {
		suite = append(suite, literals(filepath.Join(tomlDir, f))...)
	}
	suite = append(suite, fuzzCorpus(filepath.Join(tomlDir, "testdata/fuzz"))...)
	suite = append(suite, handTOML...)
	for _, s := range suite {
		add(doc{name: "toml", format: metadecoders.TOML, src: s, human: true})
	}

	// Mutations of the valid, small TOML documents.
	rnd := rand.New(rand.NewSource(*seed))
	var valid []string
	for _, s := range suite {
		var v any
		if len(s) > 0 && len(s) <= 400 && toml.Unmarshal([]byte(s), &v) == nil {
			valid = append(valid, s)
		}
	}
	subst := []byte("\"'=[]{},.\n\r #\\e_09:-+TZtz\x00\x7f\xc3\xff\t")
	for _, s := range valid {
		for i := 0; i < len(s); i++ {
			add(doc{name: "prefix", format: metadecoders.TOML, src: s[:i]})
		}
		for k := 0; k < 4; k++ {
			b := []byte(s)
			b[rnd.Intn(len(b))] = subst[rnd.Intn(len(subst))]
			add(doc{name: "subst", format: metadecoders.TOML, src: string(b)})
		}
	}
	for k := 0; k < 3000 && len(valid) > 0; k++ {
		a, b := valid[rnd.Intn(len(valid))], valid[rnd.Intn(len(valid))]
		add(doc{name: "concat", format: metadecoders.TOML, src: a + "\n" + b})
	}

	for k := 0; k < *soups; k++ {
		add(doc{name: "soup", format: metadecoders.TOML, src: tomlSoup(rnd)})
	}

	for _, s := range handYAML {
		add(doc{name: "yaml", format: metadecoders.YAML, src: s, human: true})
	}
	for _, s := range handJSON {
		add(doc{name: "json", format: metadecoders.JSON, src: s, human: true})
	}
	// The TOML documents again through the other decoders' eyes (format mismatch errors).
	for _, s := range handTOML[:10] {
		add(doc{name: "toml-as-json", format: metadecoders.JSON, src: s})
		add(doc{name: "toml-as-yaml", format: metadecoders.YAML, src: s})
	}
	for _, f := range []metadecoders.Format{"", "csv", "xml", "org", "bogus"} {
		add(doc{name: "fmt", format: f, src: "a = 1"})
	}

	var cases []map[string]any
	for _, d := range docs {
		cases = append(cases, decodeCase(d))
	}

	misc := miscCases()

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	header := map[string]any{"oracle": "nh-parser/metadecoders", "docs": len(docs)}
	if err := goval.WriteCasesGz(filepath.Join(*out, "decode.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "misc.json.gz"), map[string]any{"oracle": "nh-parser/metadecoders"}, misc); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "metadecoders: %d docs, %d misc cases\n", len(docs), len(misc))
}

// result encodes (v, err) of a metadecoders call: {"ok": v} or {"err": Error(), "cause": <FileError's cause>}.
func result(v any, err error) map[string]any {
	if err != nil {
		r := map[string]any{"err": goval.Str(err.Error())}
		cause := err
		if fe := herrors.UnwrapFileError(err); fe != nil {
			cause = errors.Unwrap(fe)
		}
		r["cause"] = goval.Str(cause.Error())
		return r
	}
	return map[string]any{"ok": tval.Encode(v)}
}

func call(f func() map[string]any) (out map[string]any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	return f()
}

func decodeCase(d doc) map[string]any {
	c := map[string]any{"name": d.name, "format": string(d.format), "src": goval.Str(d.src)}
	data := withSpareCapacity(d.src)
	c["toMap"] = call(func() map[string]any {
		m, err := metadecoders.Default.UnmarshalToMap(data, d.format)
		return result(m, err)
	})
	c["any"] = call(func() map[string]any {
		v, err := metadecoders.Default.Unmarshal(data, d.format)
		return result(v, err)
	})
	if d.format == metadecoders.TOML {
		c["toml"] = call(func() map[string]any {
			var v any
			err := toml.Unmarshal(data, &v)
			if err == nil {
				return map[string]any{"ok": tval.Encode(v)}
			}
			r := map[string]any{"err": goval.Str(err.Error())}
			var de *toml.DecodeError
			if errors.As(err, &de) {
				line, col := de.Position()
				r["pos"] = []int{line, col}
				if d.human {
					r["human"] = goval.Str(de.String())
				}
			}
			return r
		})
	}
	return c
}

// withSpareCapacity returns s as a byte slice whose capacity exceeds its
// length. go-toml locates its errors with pointer arithmetic, and Go gives an
// empty tail slice b[len(b):] of a full slice (len == cap) the base pointer of
// b, not its end; with spare capacity the pointer is the end of the document.
// Hugo's TOML input always has spare capacity (afero.ReadFile grows a
// bytes.Buffer by bytes.MinRead; front matter is a sub-slice of the page
// followed by its closing delimiter), so that is the case the port reproduces.
func withSpareCapacity(s string) []byte {
	b := make([]byte, len(s), len(s)+64)
	copy(b, s)
	return b
}

// repoFiles lists the repository's TOML, YAML and JSON files (sorted), outside
// rust/, tools/, .git and node_modules, up to 256 KiB.
func repoFiles(root string) []string {
	var files []string
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, _ := filepath.Rel(root, path)
		rel = filepath.ToSlash(rel)
		if d.IsDir() {
			switch rel {
			case ".git", "rust", "tools", "node_modules", ".claude":
				return filepath.SkipDir
			}
			if d.Name() == "node_modules" {
				return filepath.SkipDir
			}
			return nil
		}
		switch filepath.Ext(path) {
		case ".toml", ".yaml", ".yml", ".json":
		default:
			return nil
		}
		info, err := d.Info()
		if err != nil {
			return err
		}
		if info.Size() > 256<<10 {
			return nil
		}
		files = append(files, rel)
		return nil
	})
	if err != nil {
		log.Fatal(err)
	}
	sort.Strings(files)
	return files
}

// literals returns the string literals of a Go file (missing files give none).
func literals(path string) []string {
	src, err := os.ReadFile(path)
	if err != nil {
		return nil
	}
	fset := token.NewFileSet()
	af, err := parser.ParseFile(fset, path, src, 0)
	if err != nil {
		log.Fatal(err)
	}
	var out []string
	ast.Inspect(af, func(n ast.Node) bool {
		bl, ok := n.(*ast.BasicLit)
		if !ok || bl.Kind != token.STRING {
			return true
		}
		s, err := strconv.Unquote(bl.Value)
		if err == nil {
			out = append(out, s)
		}
		return true
	})
	return out
}

// fuzzCorpus reads go test fuzz v1 files ([]byte("...") or string("...") entries).
func fuzzCorpus(dir string) []string {
	var out []string
	var paths []string
	_ = filepath.WalkDir(dir, func(path string, d fs.DirEntry, err error) error {
		if err == nil && !d.IsDir() {
			paths = append(paths, path)
		}
		return nil
	})
	sort.Strings(paths)
	for _, p := range paths {
		b, err := os.ReadFile(p)
		if err != nil {
			log.Fatal(err)
		}
		for _, line := range strings.Split(string(b), "\n") {
			for _, pre := range []string{"[]byte(", "string("} {
				if strings.HasPrefix(line, pre) && strings.HasSuffix(line, ")") {
					if s, err := strconv.Unquote(line[len(pre) : len(line)-1]); err == nil {
						out = append(out, s)
					}
				}
			}
		}
	}
	return out
}

func miscCases() []map[string]any {
	var cases []map[string]any
	strs := []string{
		"", "yaml", "YAML", "yml", "Yml", "json", "toml", "TOML", "org", "csv", "xml", "bogus",
		"config.toml", "hugo.TOML", "a/b/c.yaml", "data.json", "x.yml", "noext", ".toml", "a.b.c",
		"file.tar.gz", "dir.toml/file", "İ.toml", "a.JSON", "..", ".", "a.",
	}
	for _, s := range strs {
		cases = append(cases, map[string]any{"op": "FormatFromString", "in": s, "out": string(metadecoders.FormatFromString(s))})
	}
	for _, ss := range [][]string{{}, {"", "bogus", "json"}, {"x.yaml", "toml"}, {"a", "b"}} {
		cases = append(cases, map[string]any{"op": "FormatFromStrings", "in": ss, "out": string(metadecoders.FormatFromStrings(ss...))})
	}
	contents := []string{
		"", "a = 1", "a: 1", "{\"a\": 1}", "<root/>", "a,b,c", "a = {b: 1}", "a: b = c", "x < y: z",
		"[a]\nb = 1", "- a\n- b", "title: x\n", "  a=1  ", "a;b", "{a,b}", "\"a\",\"b\"\n1,2",
	}
	for _, s := range contents {
		for _, delim := range []rune{',', ';'} {
			d := metadecoders.Decoder{Delimiter: delim, TargetType: "slice"}
			cases = append(cases, map[string]any{"op": "FormatFromContentString", "delim": string(delim), "in": goval.Str(s), "out": string(d.FormatFromContentString(s))})
		}
	}
	for _, d := range []metadecoders.Decoder{metadecoders.Default, {Delimiter: ';', Comment: '#', LazyQuotes: true, TargetType: "map"}, {}} {
		cases = append(cases, map[string]any{
			"op": "OptionsKey", "delim": int(d.Delimiter), "comment": int(d.Comment), "lazy": d.LazyQuotes,
			"target": d.TargetType, "out": goval.Str(d.OptionsKey()),
		})
	}
	// Unmarshal of empty data.
	for _, f := range []metadecoders.Format{"", "json", "toml", "yaml", "csv", "xml", "org"} {
		for _, tt := range []string{"slice", "map", "bogus"} {
			d := metadecoders.Decoder{Delimiter: ',', TargetType: tt}
			ff := f
			cases = append(cases, map[string]any{"op": "UnmarshalEmpty", "format": string(f), "target": tt, "res": call(func() map[string]any {
				v, err := d.Unmarshal(nil, ff)
				if err == nil {
					if v2, ok := v.([][]string); ok {
						return map[string]any{"ok": map[string]any{"t": "[][]string", "len": len(v2)}}
					}
				}
				return result(v, err)
			})})
		}
		ff := f
		cases = append(cases, map[string]any{"op": "UnmarshalToMapNil", "format": string(f), "res": call(func() map[string]any {
			m, err := metadecoders.Default.UnmarshalToMap(nil, ff)
			return result(m, err)
		})})
	}
	// UnmarshalStringTo.
	typs := []struct {
		name string
		v    any
	}{
		{"string", "s"}, {"map", map[string]any{}}, {"params", maps.Params{}}, {"slice", []any{}},
		{"bool", true}, {"int", 1}, {"int64", int64(1)}, {"float64", 1.5}, {"int32", int32(1)},
		{"[]string", []string{}},
	}
	datas := []string{
		"", "  hello  ", "true", "false", "1", "  42  ", "-7", "0x1F", "3.5", "1e3", "abc", "yes",
		"a = 1", "a: 1", "{\"a\": 1}", "[1, 2, 3]", "- a\n- b", "{a: 1}", "a = [1,2]\nb = 'x'",
		"9223372036854775808", "1_000", "07", "t", "F",
	}
	for _, t := range typs {
		for _, s := range datas {
			tv, ss := t.v, s
			cases = append(cases, map[string]any{"op": "UnmarshalStringTo", "typ": t.name, "in": s, "res": call(func() map[string]any {
				v, err := metadecoders.Default.UnmarshalStringTo(ss, tv)
				return result(v, err)
			})})
		}
	}
	return cases
}

var tomlTokens = []string{
	"a", "b", "key", "k-1", "_", `"q"`, "'l'", `"a.b"`, `""`, "''", ".", " . ", "=", " = ", "\n", "\r\n", "\r",
	" ", "\t", "#c", "# comment\n", "[", "]", "[[", "]]", "{", "}", ",", ", ",
	"1", "-1", "+1", "0", "00", "1_000", "_1", "1__0", "0x1F", "0o7", "0b1", "0xG", "9223372036854775807",
	"9223372036854775808", "-9223372036854775808", "1.5", "1e5", "1.e5", ".5", "5.", "1e", "e", "inf", "-inf",
	"+nan", "nan", "true", "false", "tru", `"str"`, `"sé"`, `"\U0001F600"`, `"\q"`, "'lit'",
	"\"\"\"ml\n\"\"\"", "'''ml\n'''", "\"\"\"a\\\n  b\"\"\"", `"unterminated`,
	"1979-05-27", "1979-05-27T07:32:00Z", "1979-05-27T07:32:00", "07:32:00", "07:32:00.123",
	"1979-05-27 07:32:00+05:30", "1979-05-27T07:32:00.1-07:00", "2000-02-30", "24:00:00",
	"é", "\xff", "\x00", "\x7f",
	"[t]\n", "[t.u]\n", "[[aot]]\n", "[[aot.x]]\n", "a = 1\n", "a.b = 2\n", "x = [1, 2]\n",
	"y = {z = 1}\n", "a = 1\na = 2\n", "[t]\n[t]\n",
}

// tomlSoup returns a random sequence of TOML tokens (mostly key = value lines).
func tomlSoup(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.Intn(25)
	for i := 0; i < n; i++ {
		if r.Intn(3) == 0 {
			fmt.Fprintf(&b, "k%d = ", r.Intn(4))
		}
		b.WriteString(tomlTokens[r.Intn(len(tomlTokens))])
		if r.Intn(4) == 0 {
			b.WriteString("\n")
		}
	}
	return b.String()
}

var handTOML = []string{
	"",
	"# only a comment",
	"\n\n\r\n",
	"a = 1",
	"a = 9223372036854775807\nb = -9223372036854775808\nc = +9223372036854775807",
	"a = 9223372036854775808",
	"a = -9223372036854775809",
	"a = 0x7FFFFFFFFFFFFFFF\nb = 0x8000000000000000",
	"a = 0o777\nb = 0o1_7\nc = 0b1111_0000\nd = 0xdead_BEEF",
	"a = 0xG",
	"a = 0x",
	"a = 0b2",
	"a = 00",
	"a = 01",
	"a = +0\nb = -0\nc = 0",
	"a = 1__0",
	"a = _1",
	"a = 1_",
	"a = 1.0\nb = -0.0\nc = 1e308\nd = 5e-324\ne = 1.7976931348623157e308\nf = 2.2250738585072014e-308",
	"a = 1e309",
	"a = -1e400",
	"a = 1e-400",
	"a = 3.141592653589793238462643383279",
	"a = inf\nb = +inf\nc = -inf\nd = nan\ne = +nan\nf = -nan",
	"a = infinity",
	"a = na",
	"a = 1.\nb = 2",
	"a = .1",
	"a = 1.e5",
	"a = 1e_5",
	"a = 1_e5",
	"a = 1._5",
	"a = 1_.5",
	"a = 01.5",
	"a = -01.5",
	"a = 1.5.5",
	"a = 1e5e5",
	"a = 1_000.000_1e1_0",
	"a = 1e",
	"a = e",
	"a = +",
	"a = -",
	"a = true\nb = false",
	"a = tru",
	"a = falsey",
	"a = True",
	"d1 = 1979-05-27T07:32:00Z\nd2 = 1979-05-27T00:32:00-07:00\nd3 = 1979-05-27T00:32:00.999999-07:00\nd4 = 1979-05-27 07:32:00Z\nd5 = 1979-05-27t07:32:00z",
	"d = 1979-05-27T07:32:00+00:00\ne = 1979-05-27T07:32:00-00:00\nf = 1979-05-27T07:32:00+23:59\ng = 1979-05-27T07:32:00-23:59",
	"d = 1979-05-27T07:32:00+24:00",
	"d = 1979-05-27T07:32:00+05:60",
	"d = 1979-05-27T07:32:00+0500",
	"d = 1979-05-27T07:32:00+05:00:00",
	"d = 1979-05-27T07:32:00 +05:00",
	"d = 1979-05-27T07:32:00X",
	"d = 1979-05-27T07:32:60Z\ne = 1990-12-31T23:59:60Z",
	"d = 1979-05-27T07:32:61Z",
	"d = 1979-05-27T24:00:00Z",
	"d = 1979-05-27T23:60:00Z",
	"d = 1979-05-27T07:32:00.123456789123456Z",
	"d = 1979-05-27T07:32:00.Z",
	"d = 1979-05-27T07:32:00.1234567891Z",
	"d = 1979-05-27T07:32Z",
	"d = 1979-05-27T7:32:00Z",
	"d = 1979-5-27T07:32:00Z",
	"d = 0000-01-01T00:00:00Z\ne = 9999-12-31T23:59:59.999999999Z",
	"d = 2000-02-29T00:00:00Z\ne = 2100-02-28",
	"d = 1900-02-29",
	"d = 2000-02-30",
	"d = 2001-13-01",
	"d = 2001-00-01",
	"d = 2001-01-00",
	"d = 2001-01-32",
	"ld = 1979-05-27\nlt = 07:32:00\nlt2 = 00:32:00.999999\nldt = 1979-05-27T07:32:00\nldt2 = 1979-05-27T00:32:00.999999\nldt3 = 1979-05-27 07:32:00",
	"lt = 07:32:00.1\nlt2 = 23:59:59.123456789\nlt3 = 00:00:00.000000000\nlt4 = 12:00:00.0000000001",
	"lt = 07:32",
	"lt = 7:32:00",
	"lt = 07:32:00Z",
	"lt = 07:32:00+01:00",
	"lt = 24:00:00",
	"lt = 07:60:00",
	"lt = 07:32:61",
	"lt = 07:32:60",
	"lt = 07:32:00.",
	"lt = 07:32:00.x",
	"ldt = 1979-05-27T07:32",
	"ldt = 1979-05-27T07:32:00.",
	"ldt = 1979-05-27T07:32:00.5 # comment",
	"ld = 1979-05-27 # comment\nx = 1",
	"ld = 1979-05-27 07",
	"ld = 1979-05-27 x",
	"ld = 1979-05-2",
	"ld = 197-05-27",
	"s = \"basic\"\nl = 'literal'\ne = \"\"\nel = ''",
	"s = \"\\b\\t\\n\\f\\r\\\"\\\\\\e \\u00E9 \\U0001F600\"",
	"s = \"\\u00\"",
	"s = \"\\uD800\"",
	"s = \"\\U00110000\"",
	"s = \"\\uGGGG\"",
	"s = \"\\x41\"",
	"s = \"\\ \"",
	"s = \"unterminated",
	"s = 'unterminated",
	"s = \"new\nline\"",
	"s = 'new\nline'",
	"s = \"tab\tok\"",
	"s = \"ctrl\x01\"",
	"s = 'ctrl\x7f'",
	"s = \"\xff\"",
	"s = '\xc3\x28'",
	"s = \"caf\xc3\xa9\"",
	"s = \"\"\"\nmulti\nline\"\"\"",
	"s = \"\"\"\r\nmulti\r\nline\"\"\"",
	"s = \"\"\"The quick brown \\\n\n\n  fox jumps over \\\n    the lazy dog.\"\"\"",
	"s = \"\"\"\\\n       The quick brown \\\n       fox.\\\n       \"\"\"",
	"s = \"\"\"trailing \\   \n  spaces\"\"\"",
	"s = \"\"\"a \\\r\n  b\"\"\"",
	"s = \"\"\"Here are two quotation marks: \"\". Simple enough.\"\"\"",
	"s = \"\"\"Here are three: \"\"\\\".\"\"\"",
	"s = \"\"\"\"This,\" she said, \"is just a pointless statement.\"\"\"\"",
	"s = \"\"\"a\"\"\"\"\"\"",
	"s = \"\"\"unterminated",
	"s = \"\"\"\\u00e9\\U0001F600\\e\"\"\"",
	"s = \"\"\"\\q\"\"\"",
	"s = \"\"\"\\u12\"\"\"",
	"s = \"\"\"bad \x01\"\"\"",
	"s = \"\"\"a\rb\"\"\"",
	"s = '''\nraw\\n\n'''",
	"s = '''Here are fifteen quotation marks: \"\"\"\"\"\"\"\"\"\"\"\"\"\"\"'''",
	"s = ''''That,' she said, 'is still pointless.''''",
	"s = '''a''''''",
	"s = '''a\r\nb'''",
	"s = '''a\rb'''",
	"s = '''unterminated",
	"s = '''bad \x01'''",
	"a = []\nb = [ ]\nc = [\n]\nd = [1,]\ne = [\n  1,\n  2, # comment\n]",
	"a = [1, \"a\", 2.5, true, 1979-05-27, 07:32:00, [1, [2, [3]]], {x = 1}]",
	"a = [,]",
	"a = [1 2]",
	"a = [1,,2]",
	"a = [1",
	"a = [",
	"a = [\n# c\n1 # c\n, # c\n2 # c\n]",
	"a = { }\nb = {x = 1, y = \"2\", z = {w = [1]}}\nc = {a.b = 1, a.c = 2}",
	"a = {x = 1,}",
	"a = {x = 1\n}",
	"a = {x = 1, x = 2}",
	"a = {x.y = 1, x = 2}",
	"a = {",
	"a = {x",
	"a = {x = ",
	"a = {,}",
	"[table]\na = 1\n[table.sub]\nb = 2\n[other]\nc = 3",
	"[a.b.c]\nd = 1\n[a]\ne = 2",
	"[a]\nb = 1\n[a]\nc = 2",
	"[a]\n[a.b]\n[a]",
	"[a.b]\n[a]\n[a.b]",
	"a.b = 1\n[a]",
	"[a]\nb.c = 1\n[a.b]",
	"[a]\nb.c = 1\n[a.b.d]",
	"[ a . b ]\nc = 1",
	"[ \"quoted\" . 'lit' . bare ]\nx = 1",
	"[]\na = 1",
	"[a\nb = 1",
	"[a]]\nb = 1",
	"[a] x",
	"[a] # comment\nb = 1",
	"[[aot]]\nx = 1\n[[aot]]\nx = 2\n[aot.sub]\ny = 3\n[[aot.sub2]]\nz = 4\n[[aot]]",
	"[[a.b]]\nx = 1\n[[a.b]]\nx = 2\n[a]\ny = 3",
	"[[a]]\n[[a.b]]\n[[a.b]]\n[[a]]\n[[a.b]]",
	"[[a]]\n[a.b]\nc = 1\n[[a]]\n[a.b]\nc = 2",
	"a = []\n[[a]]",
	"a = [{}]\n[[a]]",
	"[a]\n[[a]]",
	"[[a]]\n[a]",
	"[a.b]\n[[a]]",
	"[[a]\nx = 1",
	"[[a]] x",
	"[[ a ]]\nb = 1",
	"[[a.'b.c'.\"d\"]]\ne = 1",
	"a.b.c = 1\na.b.d = 2\na.e = 3",
	"a.b = 1\na.b.c = 2",
	"a = 1\na.b = 2",
	"a.b = 1\na = 2",
	"a = 1\na = 2",
	"\"a\" = 1\na = 2",
	"'a' = 1\n\"a\" = 2",
	"\"\" = 1",
	"'' = 1\n\"\" = 2",
	"\"a.b\" = 1\na.b = 2",
	"\"\\u00e9\" = 1\n\"é\" = 2",
	"= 1",
	"a",
	"a =",
	"a = 1 b = 2",
	"a = 1 # comment\nb = 2 # comment",
	"a = 1 #\x01",
	"a = 1 #\xff",
	"a = 1 # ok\r\nb = 2\r\n",
	"a = 1\rb = 2",
	"a = 1\r",
	"a=1\nb=2\n\n\n[c]\n\n\nd=3\n",
	"   a   =   1   \n\t\tb\t=\t2",
	"a-b_c = 1\n123 = 2\n-_- = 3\nA = 4",
	"é = 1",
	"a b = 1",
	"a.\"\" = 1",
	"a . b . c = 1",
	"a.b.c.d.e.f.g.h.i.j.k.l.m.n.o.p = 1",
	"[a.b.c.d.e.f.g]\n[a.b.c.d.e.f.h]\n[a.b.c.d.e]",
	"key = \"value\"\n[servers]\n  [servers.alpha]\n  ip = \"10.0.0.1\"\n  dc = \"eqdc10\"\n  [servers.beta]\n  ip = \"10.0.0.2\"\n  dc = \"eqdc10\"",
	"[[fruits]]\nname = \"apple\"\n[fruits.physical]\ncolor = \"red\"\nshape = \"round\"\n[[fruits.varieties]]\nname = \"red delicious\"\n[[fruits.varieties]]\nname = \"granny smith\"\n[[fruits]]\nname = \"banana\"\n[[fruits.varieties]]\nname = \"plantain\"",
	"points = [ { x = 1, y = 2, z = 3 },\n           { x = 7, y = 8, z = 9 },\n           { x = 2, y = 4, z = 8 } ]",
	"[languages]\n[languages.en]\nlanguageName = \"🇺🇸 English\"\nweight = 1\n[languages.th]\nlanguageName = \"🇹🇭 ไทย\"\nweight = 2.0\n[languages.fr]\ndisabled = true\nweight = 3",
	"[menu]\n[[menu.main]]\nname = \"Home\"\nweight = 1.0\nurl = \"/\"\n[[menu.main]]\nname = \"About\"\nweight = 2\n",
	"[params]\nyearcreate = \"2019\"\nratings = [1, 2.5, \"3\"]\n[params.social]\ntwitter = \"x\"",
	"[readingTime]\nother = \"{{ .Count }} minute read\"\none = \"1 minute read\"\n[home]\nother = \"หน้าแรก\"",
	"title = 'x'\ndate = 2019-12-31T07:06:21.671Z\ntags = [\"\"]\ningredients = [\"\"]\ndraft = false",
	"a = 1\n\x00",
	"\xef\xbb\xbfa = 1",
	"a = 1\x0bb = 2",
	"[a]\n\n[b]\n\n[a.c]\nd = 1",
}

var handYAML = []string{
	"",
	"~",
	"null",
	"title: x\nint: 1\nbig: 9223372036854775808\nneg: -9223372036854775808\nfloat: 1.5\nexp: 1e3\n08: octal-ish\nk: 08",
	"a: [1, 2, {b: c}]\n1: one\ntrue: yes\n~: nil\n1.5: f",
	"nested:\n  1: a\n  true: b\n  ~: c\n  1.50: d\n  0x10: e\n  [1, 2]: f",
	"date: 2020-05-17T15:05:09.238Z\nday: 2020-05-17\nts: 2001-12-14 21:59:43.10 -5",
	"a: &x {b: 1}\nc: *x\nd:\n  <<: *x\n  e: 2",
	"- a\n- b",
	"scalar",
	"a: b: c",
	"a:\n\t- tab",
	"key: 'single' \nkey2: \"double \\u00e9\"\nkey3: plain text",
	"a: .inf\nb: -.Inf\nc: .NaN\nd: 0o17\ne: 0b101\nf: 1_000",
	"a: y\nb: n\nc: on\nd: off\ne: Yes\nf: NO",
	"dup: 1\ndup: 2",
	"a: |\n  literal\n  block\nb: >-\n  folded\n  block\n",
	"a: !!str 1\nb: !!int '2'\nc: !!float 3",
	"a: !!binary aGVsbG8=",
	"a: [unclosed",
	"a: {unclosed",
	"\xff: bad",
}

var handJSON = []string{
	"",
	"{}",
	"null",
	"[]",
	"[1, 2]",
	"1",
	"\"s\"",
	"{\"a\": 1, \"b\": 1.5, \"c\": 1e400, \"d\": -0, \"e\": 12345678901234567890}",
	"{\"a\": {\"b\": [1, {\"c\": null}]}, \"d\": true}",
	"{\"a\": \"\\u00e9\\ud83d\\ude00\\ud800\"}",
	"{\"a\": \"\xff\"}",
	"{\"a\": 1,}",
	"{\"a\" 1}",
	"{\"a\": 1} extra",
	"{\"a\":",
	"{'a': 1}",
	"{\"a\": 01}",
	"{\"a\": .5}",
	"{\"a\": NaN}",
	"{\"dup\": 1, \"dup\": 2}",
	"{\"Title\": \"x\", \"title\": \"y\"}",
	" \n {\"ws\": true} \n ",
	"{\"deep\": [[[[[[[[[[1]]]]]]]]]]}",
}
