// Command go-yaml is the Go oracle for the Rust crate crates/go-yaml.
//
// It has two subcommands:
//
//	go run ./tools/go-oracle/go-yaml corpus -site <seeksnack> -out <dir> [-fuzz N]
//	go run ./tools/go-oracle/go-yaml run -in <corpus> -out <fixture>
//
// "corpus" writes corpus files (one case per line: name TAB hex(input)):
//
//   - yamlv2-tests.corpus: every string literal (and constant string
//     concatenation) of gopkg.in/yaml.v2@v2.4.0 decode_test.go and
//     encode_test.go, plus the fuzz crashers and scaled-down limit tests;
//   - seeksnack-fm.corpus: the YAML front matter of every content file of
//     the site, extracted with neohugo's own pageparser, plus the site's
//     *.yml/*.yaml files;
//   - adversarial.corpus: YAML 1.1 scalars in many syntactic contexts,
//     tags, anchors/merges, directives, encodings, whitespace and limits;
//   - fuzz.corpus: deterministic random mutations of the above.
//
// "run" decodes every case with the real Go code and writes one line per
// case: name TAB hex TAB r1 TAB r2 TAB r3 TAB r4, where
//
//	r1 = yaml.Unmarshal(data, &v) with v interface{}
//	r2 = yaml.Unmarshal(data, &m) with m := make(map[string]interface{})
//	r3 = metadecoders.Default.UnmarshalToMap(data, metadecoders.YAML)
//	r4 = metadecoders.Default.Unmarshal(data, metadecoders.YAML)
//
// and each result is "ok <dump>" or "err <escaped message>" (see dump).
package main

import (
	"bufio"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"io/fs"
	"log"
	"math"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/neohugo/neohugo/parser/metadecoders"
	"github.com/neohugo/neohugo/parser/pageparser"
	"github.com/spf13/cast"
	yaml "gopkg.in/yaml.v2"
)

func main() {
	if len(os.Args) < 2 {
		log.Fatal("usage: go-yaml corpus|run ...")
	}
	switch os.Args[1] {
	case "corpus":
		cmdCorpus(os.Args[2:])
	case "run":
		cmdRun(os.Args[2:])
	case "verify":
		cmdVerify(os.Args[2:])
	case "time":
		// Time yaml.Unmarshal into interface{} for each file argument.
		for _, fn := range os.Args[2:] {
			b, err := os.ReadFile(fn)
			if err != nil {
				log.Fatal(err)
			}
			t := time.Now()
			var v any
			err = yaml.Unmarshal(b, &v)
			fmt.Printf("%s: %v err=%v\n", fn, time.Since(t), err)
		}
	default:
		log.Fatalf("unknown subcommand %q", os.Args[1])
	}
}

// ---------------------------------------------------------------------------
// Dumps (mirrored by go_yaml::dump and go_yaml::metadecoders::dump_value).

func esc(s string) string {
	var b strings.Builder
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c >= 0x20 && c < 0x7f && c != '"' && c != '\\' {
			b.WriteByte(c)
		} else {
			fmt.Fprintf(&b, "\\x%02x", c)
		}
	}
	return b.String()
}

func dump(v any) string {
	switch t := v.(type) {
	case nil:
		return "nil"
	case bool:
		return fmt.Sprintf("bool:%v", t)
	case int:
		return fmt.Sprintf("int:%d", t)
	case uint64:
		return fmt.Sprintf("uint64:%d", t)
	case float64:
		return fmt.Sprintf("float64:%016x", math.Float64bits(t))
	case string:
		return `str:"` + esc(t) + `"`
	case []any:
		parts := make([]string, len(t))
		for i, e := range t {
			parts[i] = dump(e)
		}
		return "[" + strings.Join(parts, ",") + "]"
	case map[any]any:
		type kv struct{ k, v string }
		var parts []kv
		for k, e := range t {
			parts = append(parts, kv{dump(k), dump(e)})
		}
		sort.Slice(parts, func(i, j int) bool {
			if parts[i].k != parts[j].k {
				return parts[i].k < parts[j].k
			}
			return parts[i].v < parts[j].v
		})
		ss := make([]string, len(parts))
		for i, p := range parts {
			ss[i] = p.k + "=" + p.v
		}
		return "imap{" + strings.Join(ss, ",") + "}"
	case map[string]any:
		if t == nil {
			return "smap(nil)"
		}
		keys := make([]string, 0, len(t))
		for k := range t {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		ss := make([]string, len(keys))
		for i, k := range keys {
			ss[i] = `str:"` + esc(k) + `"=` + dump(t[k])
		}
		return "smap{" + strings.Join(ss, ",") + "}"
	default:
		return fmt.Sprintf("unexpected:%T", v)
	}
}

func result(v any, err error) string {
	if err != nil {
		return "err " + esc(err.Error())
	}
	return "ok " + dump(v)
}

// metaErr returns the error message without the herrors file decoration:
// the "failed to unmarshal YAML: ..." error wrapped by toFileError.
func metaErr(err error) error {
	if err == nil {
		return nil
	}
	if u := errors.Unwrap(err); u != nil {
		return u
	}
	return err
}

// collides reports whether stringifyMapKeys would map two keys of one
// map[interface{}]interface{} to the same string. Go's result then depends
// on random map iteration order, so the oracle records "nondet".
func collides(v any) bool {
	switch t := v.(type) {
	case []any:
		for _, e := range t {
			if collides(e) {
				return true
			}
		}
	case map[string]any:
		for _, e := range t {
			if collides(e) {
				return true
			}
		}
	case map[any]any:
		seen := map[string]bool{}
		for k, e := range t {
			ks, ok := k.(string)
			if !ok {
				var err error
				ks, err = cast.ToStringE(k)
				if err != nil {
					ks = fmt.Sprintf("%v", k)
				}
			}
			if seen[ks] {
				return true
			}
			seen[ks] = true
			if collides(e) {
				return true
			}
		}
	}
	return false
}

// decodeAll runs the four decoders. Panics (which yaml.v2 lets through for
// non-yaml errors) are reported as "panic <msg>".
func decodeAll(data []byte) [4]string {
	var out [4]string
	var v1 any
	var m2 map[string]any
	protect := func(i int, f func() string) {
		defer func() {
			if r := recover(); r != nil {
				out[i] = "panic " + esc(fmt.Sprint(r))
			}
		}()
		out[i] = f()
	}
	protect(0, func() string {
		var v any
		err := yaml.Unmarshal(data, &v)
		if err == nil {
			v1 = v
		}
		return result(v, err)
	})
	protect(1, func() string {
		m := make(map[string]any)
		err := yaml.Unmarshal(data, &m)
		if err == nil {
			m2 = m
		}
		return result(m, err)
	})
	protect(2, func() string {
		m, err := metadecoders.Default.UnmarshalToMap(data, metadecoders.YAML)
		if err == nil && collides(m2) {
			return "nondet"
		}
		return result(m, metaErr(err))
	})
	protect(3, func() string {
		v, err := metadecoders.Default.Unmarshal(data, metadecoders.YAML)
		if err == nil && collides(v1) {
			return "nondet"
		}
		return result(v, metaErr(err))
	})
	return out
}

func cmdRun(args []string) {
	fset := flag.NewFlagSet("run", flag.ExitOnError)
	in := fset.String("in", "", "corpus file")
	outPath := fset.String("out", "", "fixture file")
	_ = fset.Parse(args)
	f, err := os.Open(*in)
	if err != nil {
		log.Fatal(err)
	}
	defer f.Close()
	o, err := os.Create(*outPath)
	if err != nil {
		log.Fatal(err)
	}
	w := bufio.NewWriter(o)
	sc := bufio.NewScanner(f)
	sc.Buffer(make([]byte, 64<<20), 64<<20)
	n := 0
	for sc.Scan() {
		line := sc.Text()
		name, hx, ok := strings.Cut(line, "\t")
		if !ok {
			continue
		}
		data, err := hex.DecodeString(hx)
		if err != nil {
			log.Fatalf("%s: %v", name, err)
		}
		r := decodeAll(data)
		fmt.Fprintf(w, "%s\t%s\t%s\t%s\t%s\t%s\n", name, hx, r[0], r[1], r[2], r[3])
		n++
	}
	if err := sc.Err(); err != nil {
		log.Fatal(err)
	}
	if err := w.Flush(); err != nil {
		log.Fatal(err)
	}
	if err := o.Close(); err != nil {
		log.Fatal(err)
	}
	log.Printf("%d cases -> %s", n, *outPath)
}

// ---------------------------------------------------------------------------
// Corpus generation

type corpus struct {
	names []string
	data  [][]byte
	seen  map[string]bool
}

func newCorpus() *corpus { return &corpus{seen: map[string]bool{}} }

func (c *corpus) add(name string, data []byte) {
	if c.seen[string(data)] {
		return
	}
	c.seen[string(data)] = true
	c.names = append(c.names, fmt.Sprintf("%s#%d", name, len(c.names)))
	c.data = append(c.data, data)
}

func (c *corpus) write(path string) {
	f, err := os.Create(path)
	if err != nil {
		log.Fatal(err)
	}
	w := bufio.NewWriter(f)
	for i := range c.names {
		fmt.Fprintf(w, "%s\t%s\n", c.names[i], hex.EncodeToString(c.data[i]))
	}
	if err := w.Flush(); err != nil {
		log.Fatal(err)
	}
	if err := f.Close(); err != nil {
		log.Fatal(err)
	}
	log.Printf("%d cases -> %s", len(c.names), path)
}

func cmdCorpus(args []string) {
	fset := flag.NewFlagSet("corpus", flag.ExitOnError)
	site := fset.String("site", "", "seeksnack site root (read only)")
	outDir := fset.String("out", "", "output directory")
	nfuzz := fset.Int("fuzz", 20000, "number of fuzz cases")
	ngen := fset.Int("gen", 0, "number of grammar-generated cases (gen.corpus)")
	seed := fset.Uint64("seed", 0x9e3779b97f4a7c15, "fuzz seed")
	yamlDir := fset.String("yamlv2", "", "gopkg.in/yaml.v2@v2.4.0 source dir")
	_ = fset.Parse(args)

	tests := newCorpus()
	for _, fn := range []string{"decode_test.go", "encode_test.go", "limit_test.go", "suite_test.go", "example_embedded_test.go"} {
		for _, s := range stringLiterals(filepath.Join(*yamlDir, fn)) {
			tests.add("t:"+fn, []byte(s))
		}
	}
	for _, s := range limitCases() {
		tests.add("limit", []byte(s))
	}
	tests.write(filepath.Join(*outDir, "yamlv2-tests.corpus"))

	fm := newCorpus()
	if *site != "" {
		siteFrontMatter(fm, *site)
	}
	fm.write(filepath.Join(*outDir, "seeksnack-fm.corpus"))

	adv := newCorpus()
	adversarial(adv)
	adv.write(filepath.Join(*outDir, "adversarial.corpus"))

	fz := newCorpus()
	fuzz(fz, [][][]byte{tests.data, fm.data, adv.data}, *nfuzz, *seed)
	fz.write(filepath.Join(*outDir, "fuzz.corpus"))

	if *ngen > 0 {
		g := newCorpus()
		generate(g, *ngen, *seed)
		g.write(filepath.Join(*outDir, "gen.corpus"))
	}
}

// rng is splitmix64.
type rng struct{ state uint64 }

func (r *rng) next() uint64 {
	r.state += 0x9e3779b97f4a7c15
	z := r.state
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
	z = (z ^ (z >> 27)) * 0x94d049bb133111eb
	return z ^ (z >> 31)
}

func (r *rng) intn(n int) int { return int(r.next() % uint64(n)) }

func (r *rng) pick(ss []string) string { return ss[r.intn(len(ss))] }

// generate builds random, mostly well-formed YAML documents from a small
// grammar (block/flow collections, all scalar styles, tags, anchors,
// aliases, merges, comments, directives, odd whitespace), then sometimes
// mutates them.
func generate(c *corpus, n int, seed uint64) {
	r := &rng{state: seed ^ 0x5851f42d4c957f2d}
	scalars := append([]string{}, adversarialScalars...)
	anchors := []string{}
	var node func(indent int, depth int, flow bool) string
	scalar := func(flow bool) string {
		s := r.pick(scalars)
		switch r.intn(8) {
		case 0:
			return `"` + strings.ReplaceAll(strings.ReplaceAll(s, `\`, `\\`), `"`, `\"`) + r.pick([]string{"", `\n`, `\t`, `\x41`, `\u00e9`, `\U0001F600`, `\ `, `\_`}) + `"`
		case 1:
			return "'" + strings.ReplaceAll(s, "'", "''") + "'"
		case 2:
			if !flow {
				return r.pick([]string{"|", ">", "|-", ">+", "|2", ">-1"}) + "\n  " + s + "\n  " + r.pick(scalars) + "\n"
			}
		}
		if strings.ContainsAny(s, ",[]{}#:\"'|>&*!%@`") || s == "" || strings.HasPrefix(s, "-") || strings.HasPrefix(s, "?") {
			return `"` + strings.ReplaceAll(strings.ReplaceAll(s, `\`, `\\`), `"`, `\"`) + `"`
		}
		return s
	}
	props := func() string {
		out := ""
		if r.intn(8) == 0 {
			a := fmt.Sprintf("a%d", r.intn(6))
			anchors = append(anchors, a)
			out += "&" + a + " "
		}
		if r.intn(10) == 0 {
			out += r.pick([]string{"!!str ", "!!int ", "!!float ", "!!bool ", "!!null ", "!!map ", "!!seq ", "!foo ", "! ", "!!binary ", "!!timestamp ", "!<tag:yaml.org,2002:str> "})
		}
		return out
	}
	node = func(indent int, depth int, flow bool) string {
		if len(anchors) > 0 && r.intn(12) == 0 {
			return "*" + r.pick(anchors)
		}
		k := r.intn(10)
		if depth > 4 {
			k = 9
		}
		pad := strings.Repeat(" ", indent)
		switch {
		case k < 2 || (flow && k < 4):
			// flow sequence
			parts := []string{}
			for i := 0; i < r.intn(4); i++ {
				parts = append(parts, node(0, depth+1, true))
			}
			return props() + "[" + strings.Join(parts, r.pick([]string{", ", ",", " , "})) + r.pick([]string{"", ",", " "}) + "]"
		case k < 4 || (flow && k < 6):
			parts := []string{}
			for i := 0; i < r.intn(4); i++ {
				parts = append(parts, node(0, depth+1, true)+r.pick([]string{": ", ":", " : "})+node(0, depth+1, true))
			}
			if r.intn(6) == 0 {
				parts = append(parts, "<<: "+node(0, depth+1, true))
			}
			return props() + "{" + strings.Join(parts, ", ") + "}"
		case k < 6 && !flow:
			// block sequence
			var b strings.Builder
			b.WriteString(props() + "\n")
			in := indent + r.intn(3)
			for i := 0; i < 1+r.intn(4); i++ {
				b.WriteString(strings.Repeat(" ", in) + "-" + r.pick([]string{" ", "  ", "\t", " "}) + node(in+2, depth+1, false) + r.pick([]string{"\n", " # c\n", "\r\n", "\n\n"}))
			}
			return b.String()
		case k < 8 && !flow:
			var b strings.Builder
			b.WriteString(props() + "\n")
			in := indent + 1 + r.intn(3)
			for i := 0; i < 1+r.intn(4); i++ {
				key := scalar(true)
				if r.intn(10) == 0 {
					key = "<<"
				}
				if r.intn(15) == 0 {
					b.WriteString(strings.Repeat(" ", in) + "? " + key + "\n" + strings.Repeat(" ", in) + ": " + node(in+2, depth+1, false) + "\n")
					continue
				}
				b.WriteString(strings.Repeat(" ", in) + key + r.pick([]string{": ", ":  ", ":\t", ": "}) + node(in+2, depth+1, false) + r.pick([]string{"\n", " # c\n", "\n"}))
			}
			return b.String()
		}
		_ = pad
		return props() + scalar(flow)
	}
	for i := 0; len(c.names) < n && i < n*3; i++ {
		anchors = anchors[:0]
		var doc strings.Builder
		if r.intn(8) == 0 {
			doc.WriteString(r.pick([]string{"%YAML 1.1\n", "%TAG !e! tag:e.com:\n", "# lead\n"}) + "---\n")
		}
		// Top level: usually a block mapping (front matter shape).
		if r.intn(4) != 0 {
			for j := 0; j < 1+r.intn(6); j++ {
				doc.WriteString(scalar(true) + ": " + node(2, 1, false) + "\n")
			}
		} else {
			doc.WriteString(node(0, 0, false) + "\n")
		}
		if r.intn(10) == 0 {
			doc.WriteString("---\n" + node(0, 0, false) + "\n")
		}
		b := []byte(doc.String())
		if r.intn(3) == 0 && len(b) > 0 {
			for m := 0; m < 1+r.intn(3); m++ {
				pos := r.intn(len(b) + 1)
				const mut = "-?:,[]{}#&*!|>'\" \t\n.~0123abxyz"
				ch := mut[r.intn(len(mut))]
				switch r.intn(3) {
				case 0:
					b = append(b[:pos], append([]byte{ch}, b[pos:]...)...)
				case 1:
					if pos < len(b) {
						b = append(b[:pos], b[pos+1:]...)
					}
				default:
					if pos < len(b) {
						b[pos] = ch
					}
				}
			}
		}
		c.add("gen", b)
	}
}

// stringLiterals returns every string literal of a Go file, evaluating
// constant concatenations ("a" + "b") as one string.
func stringLiterals(path string) []string {
	fsetT := token.NewFileSet()
	f, err := parser.ParseFile(fsetT, path, nil, 0)
	if err != nil {
		log.Fatal(err)
	}
	var out []string
	var eval func(e ast.Expr) (string, bool)
	eval = func(e ast.Expr) (string, bool) {
		switch t := e.(type) {
		case *ast.BasicLit:
			if t.Kind != token.STRING {
				return "", false
			}
			s, err := strconv.Unquote(t.Value)
			if err != nil {
				return "", false
			}
			return s, true
		case *ast.BinaryExpr:
			if t.Op != token.ADD {
				return "", false
			}
			a, ok1 := eval(t.X)
			b, ok2 := eval(t.Y)
			if ok1 && ok2 {
				return a + b, true
			}
		case *ast.ParenExpr:
			return eval(t.X)
		}
		return "", false
	}
	ast.Inspect(f, func(n ast.Node) bool {
		switch t := n.(type) {
		case *ast.BinaryExpr:
			if s, ok := eval(t); ok {
				out = append(out, s)
				return false
			}
		case *ast.BasicLit:
			if s, ok := eval(t); ok {
				out = append(out, s)
			}
		case *ast.ImportSpec:
			return false
		}
		return true
	})
	return out
}

// limitCases are yaml.v2 limit_test.go's cases, scaled down so that the
// checked-in fixtures stay small (the full-size ones are exercised by the
// Rust test directly, see tests/limits.rs).
func limitCases() []string {
	r := strings.Repeat
	return []string{
		`{a: &a [{a}` + r(`,{a}`, 1000) + `], b: &b [*a` + r(`,*a`, 99) + `]}`,
		r(`[`, 10001),
		r(`[`, 10000) + r(`]`, 10000),
		"x: " + r(`{`, 10001),
		r(`- `, 10001),
		r(`- `, 9999) + "x",
		r(r(`- `, 100)+"\n", 20),
		`a: &a [{a}` + r(`,{a}`, 255) + `]`,
		r(`[`, 200) + `1` + r(`,1`, 100) + r(`]`, 200),
		"{a,b:\n" + r(" {a,b:", 198) + ` [1` + r(",1", 50) + `]` + r(`}`, 199),
		r(`- `+r(`[`, 300)+r(`]`, 300)+"\n", 3),
		"a: &a [00,00,00,00,00,00,00,00,00]\n" +
			"b: &b [*a,*a,*a,*a,*a,*a,*a,*a,*a]\n" +
			"c: &c [*b,*b,*b,*b,*b,*b,*b,*b,*b]\n" +
			"d: &d [*c,*c,*c,*c,*c,*c,*c,*c,*c]\n",
	}
}

// siteFrontMatter adds the YAML front matter of every content file (as
// neohugo's pageparser extracts it) and the site's YAML files.
func siteFrontMatter(c *corpus, site string) {
	var files []string
	_ = filepath.WalkDir(site, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return nil
		}
		if d.IsDir() {
			if d.Name() == "node_modules" || d.Name() == "public" || d.Name() == "resources" {
				return filepath.SkipDir
			}
			return nil
		}
		files = append(files, path)
		return nil
	})
	sort.Strings(files)
	for _, path := range files {
		rel, _ := filepath.Rel(site, path)
		ext := strings.ToLower(filepath.Ext(path))
		b, err := os.ReadFile(path)
		if err != nil {
			continue
		}
		switch ext {
		case ".yml", ".yaml":
			c.add("file:"+rel, b)
		case ".md", ".html", ".markdown":
			items, err := pageparser.ParseBytes(b, pageparser.Config{})
			if err != nil {
				continue
			}
			for _, it := range items {
				if it.IsFrontMatter() {
					if it.Type == pageparser.TypeFrontMatterYAML {
						c.add("fm:"+rel, it.Val(b))
					}
					break
				}
			}
		}
	}
}

// adversarialScalars are YAML 1.1 scalars exercising yaml.v2's resolver.
var adversarialScalars = []string{
	// bools and nulls (resolve.go resolveMap) and near misses
	"y", "Y", "yes", "Yes", "YES", "yEs", "n", "N", "no", "No", "NO", "nO",
	"true", "True", "TRUE", "tRUE", "false", "False", "FALSE", "on", "On", "ON", "oN",
	"off", "Off", "OFF", "oFF", "", "~", "null", "Null", "NULL", "nULL", "nil", "None",
	// floats
	".nan", ".NaN", ".NAN", ".Nan", ".inf", ".Inf", ".INF", ".iNf", "+.inf", "+.Inf", "+.INF",
	"-.inf", "-.Inf", "-.INF", "-.iNF", "inf", "nan", "Infinity", "-Infinity", "NaN",
	"1.5", "-1.5", "+1.5", "1.", "1.0", ".5", "-.5", "+.5", ".", "-.", "+.", "..5", ".5.",
	"1e5", "1E5", "1e+5", "1e-5", "1.5e10", "6.8523e+5", "685.230_15e+03", "685_230.15",
	"190:20:30.15", "1e400", "-1e400", ".5e999", ".5e-999", "1e-400", ".5_5", "._5", ".5_",
	"1__0.5", "0.1", "00.5", "0.1e", "1e", "1e+", ".e5", "1_e5", "1e5_", "1e_5", "0x1p-2",
	"0x1.8p3", "1_000.5", "3.14159265358979323846264338327950288419716939937510",
	"2.2250738585072011e-308", "4.9e-324", "1.7976931348623157e308", "1.7976931348623159e308",
	"-0.0", "0.0", "-0", "+0", "00", "0", "000", "0.",
	// ints
	"08", "09", "010", "0777", "0778", "0x1F", "0X1f", "0x1g", "0o17", "0O17", "0o18", "0b101",
	"0B101", "0b102", "0b-101", "0b+101", "-0b101", "-0b-101", "+0b101", "+0x10", "-0x10",
	"0x", "0b", "0o", "0x_1", "0_x1", "0x1_F", "1_000", "1__000", "_1000", "1000_", "+_1",
	"-_1", "1_2_3", "12", "+12", "-12", "--12", "+-12", "1-2", "1+2", "1 2",
	"9223372036854775807", "9223372036854775808", "-9223372036854775808", "-9223372036854775809",
	"18446744073709551615", "18446744073709551616", "-18446744073709551615",
	"0x7FFFFFFFFFFFFFFF", "0x8000000000000000", "0xFFFFFFFFFFFFFFFF", "0x10000000000000000",
	"-0x8000000000000000", "-0x8000000000000001",
	"0b" + strings.Repeat("1", 63), "0b" + strings.Repeat("1", 64), "0b" + strings.Repeat("1", 65),
	"-0b" + strings.Repeat("1", 64), "0b-" + strings.Repeat("1", 63),
	"123456789012345678901234567890", "1:2", "1_2:3", "12:30:00",
	// timestamps
	"2001-12-14", "2001-12-14t21:59:43.10-05:00", "2001-12-14 21:59:43.10 -5",
	"2001-12-14T21:59:43.10Z", "2002-12-14", "2001-12-15T02:59:43.1Z", "2001-12-14 21:59:43.10",
	"2001-02-29", "2000-02-29", "1900-02-29", "2001-13-01", "2001-00-01", "2001-12-32",
	"2001-12-00", "2001-1-1", "2001-01-01T25:00:00Z", "2001-01-01T24:00:00Z",
	"2001-01-01T23:60:00Z", "2001-01-01T23:59:60Z", "2001-01-01T23:59:59+24:00",
	"2001-01-01T23:59:59+25:00", "2001-01-01T23:59:59+05:61", "2001-01-01T23:59:59+0500",
	"2001-01-01T23:59:59.1234567891234Z", "2001-01-01T1:2:3Z", "2001-01-01 1:2:3",
	"2001-01-01   1:2:3", "2001-01-01T01:02:03,5Z", "2001-01-01 01:02:03,5", "12345-01-01",
	"201-01-01", "2001-01-01x", "2001-01-01T01:02:03.Z", "2001-01-01T01:02:03", "2001-01-01T",
	"2001-01-01 ", "2021-01-02T03:04:05.000Z", "2021-01-02T03:04:05.000+07:00", "2001-1-2t3:4:5z",
	"+2001-01-01", "-2001-01-01", "2001-01-01T01:02:03.5-00:00", "2001-01-01 01:02:03.",
	// strings and specials
	"hello", "<<", "=", "a b", "é", "日本語", "\u00a0x", "a#b", "a #b", "-a", "?a", ":a", "a:b",
	"http://x.y/z", "@x", "`x", "%x", "!x", "&x", "*x", "|", ">", "'", "\"", "[x", "{x",
}

func adversarial(c *corpus) {
	ctx := []string{
		"v: %s", "%s: v", "- %s", "[%s]", "{%s: %s}", "%s", "v: [%s, %s]",
		"v: !!str %s", "v: !!int %s", "v: !!float %s", "v: !!bool %s", "v: !!null %s",
		"v: !!timestamp %s", "v: !!binary %s", "v: !!merge %s", "v: ! %s", "v: !foo %s",
		"v: !!seq %s", "v: !!map %s", "v: !<tag:yaml.org,2002:int> %s", "!!int %s: v",
		"!!bool %s: v", "!!float %s: v", "? %s\n: v",
	}
	quoted := []string{`v: "%s"`, `v: '%s'`, `"%s": v`, `v: !!int "%s"`, `v: !!float '%s'`}
	for _, s := range adversarialScalars {
		for _, cx := range ctx {
			n := strings.Count(cx, "%s")
			a := make([]any, n)
			for i := range a {
				a[i] = s
			}
			c.add("scalar", []byte(fmt.Sprintf(cx, a...)))
		}
		for _, cx := range quoted {
			q := strings.ReplaceAll(s, `"`, `\"`)
			if strings.Contains(cx, "'") {
				q = strings.ReplaceAll(s, "'", "''")
			}
			c.add("quoted", []byte(fmt.Sprintf(cx, q)))
		}
	}
	for _, s := range structural {
		c.add("struct", []byte(s))
	}
	// Encodings and odd bytes.
	for _, s := range []string{"a: b\n", "- x\n- y\n", "k: é\n"} {
		c.add("bom8", append([]byte("\xef\xbb\xbf"), s...))
		le := []byte{0xff, 0xfe}
		be := []byte{0xfe, 0xff}
		for _, r := range s {
			le = append(le, byte(r), byte(r>>8))
			be = append(be, byte(r>>8), byte(r))
		}
		c.add("utf16le", le)
		c.add("utf16be", be)
		c.add("utf16le-odd", le[:len(le)-1])
	}
	for _, b := range [][]byte{
		{0xff}, {'a', ':', ' ', 0xc3}, {'a', ':', ' ', 0xc3, 0x28}, {'a', ':', ' ', 0xed, 0xa0, 0x80},
		{'a', ':', ' ', 0xf4, 0x90, 0x80, 0x80}, {'a', ':', ' ', 0xc0, 0x80}, {'a', ':', ' ', 0x01},
		{'a', ':', ' ', 0x7f}, {'a', ':', ' ', 0xc2, 0x85, 'b'}, {'a', ':', ' ', 'x', 0xe2, 0x80, 0xa8, 'b'},
		{'a', ':', ' ', 0xef, 0xbf, 0xbe}, {'a', ':', ' ', 0xef, 0xbb, 0xbf, 'x'}, {0xfe, 0xff, 0xd8, 0x00},
		{0xff, 0xfe, 0x00, 0xdc}, {0xff, 0xfe, 0x00, 0xd8, 'a', 0x00},
		// invalid byte after the first 512-byte raw chunk: surfaces lazily
		append([]byte("a: 1\n---\nb: "+strings.Repeat("x", 600)+"\n"), 0xff),
		append([]byte("a: 1\n---\nb: "+strings.Repeat("x", 300)+"\n"), 0xff),
		append([]byte("a: 1\n...\n"+strings.Repeat("#", 700)+"\n"), 0x01),
	} {
		c.add("bytes", b)
	}
	// Line breaks.
	for _, s := range []string{"a: 1\r\nb: 2\r\n", "a: 1\rb: 2\r", "a: |\r\n  x\r\n  y\r\n", "a: \"x\r\n  y\"\r\n"} {
		c.add("crlf", []byte(s))
	}
	// Long keys around the 1024-character simple key limit.
	for _, n := range []int{1020, 1023, 1024, 1025, 1030} {
		c.add("longkey", []byte(strings.Repeat("k", n)+": v\n"))
		c.add("longkey", []byte("{"+strings.Repeat("k", n)+": v}\n"))
	}
	// BOM in the middle (is_bom checks the start of the buffer).
	for _, s := range []string{
		"\ufeff\ufeffa: 1\nb: 2\n", " \ufeff\n", "? \ufeff\n", "? \ufeff:\n", "0: \ufeff\n",
		"? \ufeff: \ufeff\n", "\ufeffa: 1\n\ufeffb: 2\n", "a: 1\n\ufeff\n",
		"\ufeff\ufeff\ufeff\na: b\nc: d\n",
	} {
		c.add("bom", []byte(s))
	}
	// Buffer-boundary cases: interesting constructs straddling 512/1536.
	for _, pad := range []int{500, 505, 508, 509, 510, 511, 512, 513, 1530, 1533, 1534, 1535, 1536} {
		p := strings.Repeat("x", pad)
		c.add("boundary", []byte("k: "+p+"\u00e9\u65e5\U0001F600\n"))
		c.add("boundary", []byte("# "+p+"\n"+"a: \"\\u00e9\\U0001F600\"\n"))
		c.add("boundary", []byte("\ufeffk: "+p+"\n\ufeffz: 1\n"))
		c.add("boundary", []byte(strings.Repeat(" ", pad)+"\n---\na: 1\n"))
	}
}

var structural = []string{
	"", " ", "\n", "#", "# only a comment\n", "---", "---\n", "...", "---\n...\n", "--- \n---\n",
	"a: 1\n---\nb: 2\n", "--- a\n--- b\n", "a: 1\n...\nb: 2\n", "a: 1\n...\n---\nb: [\n",
	"%YAML 1.1\n---\na: 1\n", "%YAML 1.2\n---\na: 1\n", "%YAML 1.1\n%YAML 1.1\n---\na: 1\n",
	"%YAML 1\n---\n", "%YAML 1.123\n---\n", "%YAML a.1\n---\n", "%FOO bar\n---\na: 1\n",
	"%TAG !e! tag:example.com,2000:\n---\na: !e!foo 1\n", "%TAG !e! tag:e.com:\n%TAG !e! tag:f.com:\n---\n",
	"%TAG ! tag:e.com:\n---\na: !foo 1\n", "%TAG !! tag:e.com:\n---\na: !!int 1\n",
	"a: !e!foo 1\n", "a: !<tag:yaml.org,2002:str> 1\n", "a: !<!> 1\n", "a: !<> 1\n",
	"a: !%41%42 1\n", "a: !%zz 1\n", "a: !%C3%A9 1\n", "a: !%C3%28 1\n", "a: !%FF 1\n",
	"a: !!str\n", "a: !!null\n", "a: !!binary aGVsbG8=\n", "a: !!binary |\n  aGVs\n  bG8=\n",
	"a: !!binary aGVsbG8\n", "a: !!binary 'aGVs bG8='\n", "a: !!binary /w==\n",
	"a: &x 1\nb: *x\n", "a: &x [1, 2]\nb: *x\n", "a: &x\n  c: 1\nb: *x\n", "a: *x\n",
	"a: &x 1\na: &x 2\nb: *x\n", "a: &a [*a]\n", "a: &a {b: *a}\n", "&a a: *a\n", "&a [*a]",
	"a: &\n", "a: *\n", "a: &x\nb: *x\n", "&x\n", "*x", "a: &x !!int 1\nb: *x\n",
	"a: !!int &x 1\nb: *x\n", "- &x a\n- *x\n- &x b\n- *x\n",
	"base: &b {x: 1, y: 2}\nm:\n  <<: *b\n  y: 3\n", "m:\n  y: 3\n  <<: {x: 1, y: 2}\n",
	"m:\n  <<: [{x: 1}, {x: 2, y: 2}]\n", "m:\n  <<: [{x: 1}, 3]\n", "m:\n  <<: 3\n",
	"m:\n  <<: ~\n", "m:\n  <<: []\n", "m:\n  '<<': {x: 1}\n", "m:\n  !!merge <<: {x: 1}\n",
	"m:\n  !!str <<: {x: 1}\n", "<<: {x: 1}\n", "<<: [{x: 1}, {y: 2}]\nz: 3\n",
	"a: &a {x: 1}\n<<: *a\n", "a: &a 1\n<<: *a\n", "a: &a [1]\n<<: [*a]\n", "<<: {<<: {x: 1}}\n",
	"a: 1\na: 2\n", "1: a\n'1': b\n", "true: a\n'true': b\n", "1: a\n1.0: b\n", "~: a\nnull: b\n",
	".nan: a\n.nan: b\n", "0.0: a\n-0.0: b\n", "? [1, 2]\n: v\n", "? {a: 1}\n: v\n",
	"[1, 2]: v\n", "{a: 1}: v\n", "? a\n? b\n", "? a\n: b\n? c\n", "?\n: v\n", ": v\n",
	"a:\n  1:\nb\n  2:", "a: b: c\n", "a: - b\n", "- a: b\n  c: d\n- e\n", "- - - a\n",
	"a:\n- b\n- c\n", "a:\n  - b\n  -\n  - ~\n", "[a, b, ]\n", "[a, , b]\n", "[, a]\n",
	"{a: b, }\n", "{a, b: c}\n", "{a: b c}\n", "{: v}\n", "[a: b, c: d]\n", "[? a : b]\n",
	"[a\n, b]\n", "{a: [b, {c: d}]}\n", "[[[]]]\n", "{{}}\n", "[{}]: v\n", "a: [\n", "a: {\n",
	"a: ]\n", "a: }\n", "]\n", "}\n", "[a]]\n", "{a: b}}\n",
	"a: |\n  line1\n  line2\n", "a: |-\n  x\n\n", "a: |+\n  x\n\n\n", "a: >\n  x\n  y\n\n  z\n",
	"a: >-\n  x\n", "a: |2\n   x\n", "a: |0\n x\n", "a: |10\n x\n", "a: |-2\n    x\n",
	"a: |2-\n    x\n", "a: | # c\n  x\n", "a: |x\n  y\n", "a: >\n\tx\n", "a: |\n  x\n\ty\n",
	"a: |\n\n\n  x\n", "a: >\n  x\n   y\n  z\n", "- |\n  x\n- y\n", "a: |\n", "a: >\n",
	"a: \"x\\ty\\n\\\"\\\\\\/\"\n", "a: \"\\x41\\u00e9\\U0001F600\"\n", "a: \"\\uD800\"\n",
	"a: \"\\U00110000\"\n", "a: \"\\q\"\n", "a: \"\\x4\"\n", "a: \"\\0\\a\\b\\v\\f\\r\\e\\ \\N\\_\\L\\P\"\n",
	"a: \"x\\\n  y\"\n", "a: \"x\n  y\n\n  z\"\n", "a: 'it''s'\n", "a: 'x\n  y'\n", "a: \"x",
	"a: 'x", "a: \"x\n---\ny\"\n", "a: \"\n...\n\"\n", "a: \"\\\t\"\n",
	"a: x\n  y\n  z\n", "a: x\n\n  y\n", "a: x #c\n", "a: x#c\n", "a:x\n", "a:\n", "a :b\n",
	"a: b\n c: d\n", "a:\n\tb: c\n", "a: \tb\n", "\ta: b\n", "a: b\t\n", "- \tx\n", "a:\n  -\tx\n",
	"a: x\n\t y\n", "a: x\n \ty\n", "a: 'x\n\ty'\n", "k: v\n  # c\n", "x: - y\n",
	"- a\nb: c\n", "a: 1\n- b\n", "a\nb: c\n", "a: 1\n b: 2\n", "  a: 1\n b: 2\n",
	"a: @x\n", "a: `x\n", "a: %x\n", "@x", "`x", "a: !\n", "! a\n", "!a\n", "!!\n", "!<a\n",
	"? |\n  x\n: y\n", "? - a\n  - b\n: c\n", "- ? a\n  : b\n", "? a\n", "?\n", "? \n:\n",
	"x: !!set {a, b}\n", "x: !!omap [a: 1]\n", "x: !!pairs [a: 1]\n", "x: !!seq [a]\n",
	"x: !!map {a: 1}\n", "x: !!seq {a: 1}\n", "x: !!map [a]\n", "x: !!str [a]\n",
	"x: !!int [a]\n", "!!map {a: 1}\n", "!!seq [a]\n", "!!str\n", "!!null {a: 1}\n",
	"a: !!float 1\nb: !!float -3\nc: !!float 0x10\nd: !!float 18446744073709551615\n",
	"- 1\n- 1.0\n- '1'\n- !!str 1\n- \"1\"\n", "? !!str 1\n: a\n? 1\n: b\n",
	"title: x\ndate: 2021-01-02\ntags: [a, b]\nweight: 010\nrating: 4.5\ndraft: no\n",
	"Title: X\ntitle: y\n", "a:\n  B: 1\n  b: 2\n", "1: 2\n2.5: 3\ntrue: 4\n~: 5\n0x10: 6\n",
	"a:\n  1: 2\n  2.5: 3\n  true: 4\n  ~: 5\n  0x10: 6\n  1e3: 7\n  .inf: 8\n  -.inf: 9\n  18446744073709551615: 10\n  -0.0: 11\n",
	"- {1: a, true: b, 1.5: c}\n", "a: [{1: x}]\n", "a: [[{b: {2: y}}]]\n",
}

// fuzz generates deterministic mutations of the seed corpora (splitmix64).
func fuzz(c *corpus, seeds [][][]byte, n int, seed uint64) {
	var all [][]byte
	for _, s := range seeds {
		for _, b := range s {
			if len(b) <= 4096 {
				all = append(all, b)
			}
		}
	}
	state := seed
	next := func() uint64 {
		state += 0x9e3779b97f4a7c15
		z := state
		z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9
		z = (z ^ (z >> 27)) * 0x94d049bb133111eb
		return z ^ (z >> 31)
	}
	alphabet := []byte("-?:,[]{}#&*!|>'\"%@` \t\n\r\\.~0123456789abexyzABTFN_+<=")
	for i := 0; c.names == nil || len(c.names) < n; i++ {
		if i > n*4 {
			break
		}
		src := all[next()%uint64(len(all))]
		b := append([]byte(nil), src...)
		nm := 1 + int(next()%4)
		for m := 0; m < nm; m++ {
			op := next() % 6
			pos := 0
			if len(b) > 0 {
				pos = int(next() % uint64(len(b)+1))
			}
			ch := alphabet[next()%uint64(len(alphabet))]
			switch op {
			case 0, 1: // insert
				b = append(b[:pos], append([]byte{ch}, b[pos:]...)...)
			case 2: // delete
				if pos < len(b) {
					b = append(b[:pos], b[pos+1:]...)
				}
			case 3: // replace
				if pos < len(b) {
					b[pos] = ch
				}
			case 4: // splice from another seed
				o := all[next()%uint64(len(all))]
				if len(o) > 0 {
					st := int(next() % uint64(len(o)))
					ln := int(next() % uint64(len(o)-st+1))
					if ln > 64 {
						ln = 64
					}
					b = append(b[:pos], append(append([]byte(nil), o[st:st+ln]...), b[pos:]...)...)
				}
			case 5: // duplicate a line
				lines := strings.SplitAfter(string(b), "\n")
				if len(lines) > 0 {
					k := int(next() % uint64(len(lines)))
					lines = append(lines[:k+1], lines[k:]...)
					b = []byte(strings.Join(lines, ""))
				}
			}
		}
		c.add("fuzz", b)
	}
}
