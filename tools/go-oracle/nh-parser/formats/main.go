// Command formats is the Go oracle for the CSV and XML decoders and the YAML,
// TOML and XML encoders of crates/nh-parser (gaps follow-up of Wave B task
// T03):
//
//   - metadecoders.Decoder.Unmarshal and UnmarshalToMap of CSV documents
//     (encoding/csv) under a range of decoder options (delimiter, comment,
//     lazyQuotes, targetType);
//
//   - Unmarshal and UnmarshalToMap of XML documents (clbanning/mxj over
//     encoding/xml);
//
//   - parser.InterfaceToConfig of decoded documents to YAML (yaml.v2
//     Marshal), TOML (go-toml v2 Encoder with indented tables), XML (mxj
//     AnyXmlIndent) and JSON, with and without transform.Remarshal's
//     applyMarshalTypes.
//
//     go run ./tools/go-oracle/nh-parser/formats [-root .] [-out crates/nh-parser/tests/fixtures/formats]
//
// The encoder inputs are documents the Rust side decodes with its (verified)
// decoders: random values are serialized with yaml.Marshal (so the source is
// itself encoder output), plus hand-written YAML/TOML/JSON documents and the
// repository's data and config files. Nothing is platform dependent.
package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"io/fs"
	"log"
	"math"
	"math/rand"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"

	"github.com/neohugo/neohugo/common/herrors"
	hparser "github.com/neohugo/neohugo/parser"
	"github.com/neohugo/neohugo/parser/metadecoders"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-parser/tval"
	yaml "gopkg.in/yaml.v2"
)

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-parser/tests/fixtures/formats", "output directory")
	goroot := flag.String("goroot", "", "GOROOT of go1.27.1 (default: runtime GOROOT via $GOROOT or `go env`)")
	gomodcache := flag.String("gomodcache", "", "GOMODCACHE (default: $GOMODCACHE or ~/go/pkg/mod)")
	n := flag.Int("n", 1, "corpus size multiplier (random documents)")
	seed := flag.Int64("seed", 20260928, "seed of the random documents")
	flag.Parse()

	gr := *goroot
	if gr == "" {
		gr = os.Getenv("GOROOT")
	}
	if gr == "" {
		b, err := exec.Command("go", "env", "GOROOT").Output()
		if err != nil {
			log.Fatal(err)
		}
		gr = strings.TrimSpace(string(b))
	}
	modcache := *gomodcache
	if modcache == "" {
		modcache = os.Getenv("GOMODCACHE")
	}
	if modcache == "" {
		home, _ := os.UserHomeDir()
		modcache = filepath.Join(home, "go", "pkg", "mod")
	}
	rnd := rand.New(rand.NewSource(*seed))

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}

	csvCases := csvCorpus(gr, rnd, *n)
	xmlCases := xmlCorpus(gr, modcache, rnd, *n)
	encCases := encodeCorpus(*root, rnd, *n)

	for _, f := range []struct {
		name  string
		cases []map[string]any
	}{{"csv.json.gz", csvCases}, {"xml.json.gz", xmlCases}, {"encode.json.gz", encCases}} {
		header := map[string]any{"oracle": "nh-parser/formats", "cases": len(f.cases)}
		if err := goval.WriteCasesGz(filepath.Join(*out, f.name), header, f.cases); err != nil {
			log.Fatal(err)
		}
	}
	fmt.Fprintf(os.Stderr, "formats: %d csv, %d xml, %d encode cases\n", len(csvCases), len(xmlCases), len(encCases))
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

// decodeCase runs Unmarshal and UnmarshalToMap of src with decoder d.
func decodeCase(d metadecoders.Decoder, f metadecoders.Format, src string) map[string]any {
	c := map[string]any{
		"format": string(f), "src": goval.Str(src),
		"delim": int(d.Delimiter), "comment": int(d.Comment), "lazy": d.LazyQuotes, "target": d.TargetType,
	}
	c["any"] = call(func() map[string]any {
		v, err := d.Unmarshal([]byte(src), f)
		return result(v, err)
	})
	c["toMap"] = call(func() map[string]any {
		m, err := d.UnmarshalToMap([]byte(src), f)
		return result(m, err)
	})
	return c
}

// ---------------------------------------------------------------------------
// CSV

func csvCorpus(goroot string, rnd *rand.Rand, n int) []map[string]any {
	docs := []string{
		"a,b,c\n1,2,3\n", "a,b\n1,2", "a;b\n1;2\n", "a\tb\n1\t2\n", "# comment\na,b\n1,2\n", "a,b\n# c\n1,2\n",
		"\"a\",\"b\"\n\"1\",\"2\"\n", "\"a\"\"b\",c\n", "\"a\nb\",c\n", "\"a\r\nb\",c\r\n", "a,b\r\n1,2\r\n",
		"a,\"b\nc\"d,e\n", "a,b\"c,d\n", "\"abc\"def,g\n", "\"unterminated\n", "a,\"\n", "\n\n\na,b\n\n1,2\n\n",
		"a,b\n1,2,3\n", "a,a\n1,2\n", "a\n", "a,b\n", "a,b\n1,2\n3,4\n5,6\n", " a, b\n 1, 2\n", "a,,b\n,,\n",
		",\n,\n", "é,ü\n漢,字\n", "a|b\n1|2\n", "aébéc\n1é2é3\n", "\ufeffa,b\n1,2\n", "a,b\r", "a,b\rc,d\n",
		"\"a\" ,b\n", "a ,\"b\"\n", "x", "\"", "\"\"", "\"\"\"\"", "a,\"b\"\"\",c\n", "a\x00b,c\n", "\xff,\xfe\n",
		"#only comment\n", ";a;b\n", "title,count\nHugo,1\nGo,2\n", "\"multi\nline\nfield\",x\n\"y\",\"z\"\n",
		"a,b\n\"1\n", "a,b\n1,\"2\n3\"\n", "a\"b,c\n", "a,b\n\"x\"y,z\n", "\r\n\r\n", "a\r\r\nb\n",
	}
	docs = append(docs, csvTestInputs(goroot)...)
	soup := []string{"a", "b", "é", ",", ";", "\t", "\"", "\"\"", "\n", "\r\n", "\r", "#", " ", "x y", "|", "\xff", "1"}
	for k := 0; k < 300*n; k++ {
		var sb strings.Builder
		for i := rnd.Intn(30); i >= 0; i-- {
			sb.WriteString(soup[rnd.Intn(len(soup))])
		}
		docs = append(docs, sb.String())
	}
	decs := []metadecoders.Decoder{
		metadecoders.Default,
		{Delimiter: ';', TargetType: "slice"},
		{Delimiter: '\t', TargetType: "slice"},
		{Delimiter: 'é', TargetType: "slice"},
		{Delimiter: ',', Comment: '#', TargetType: "slice"},
		{Delimiter: ',', LazyQuotes: true, TargetType: "slice"},
		{Delimiter: ',', TargetType: "map"},
		{Delimiter: ';', Comment: '#', LazyQuotes: true, TargetType: "map"},
		{Delimiter: ',', TargetType: "bogus"},
		{Delimiter: '"', TargetType: "slice"},
		{Delimiter: '\n', TargetType: "slice"},
		{Delimiter: '\ufffd', TargetType: "slice"},
		{Delimiter: ',', Comment: ',', TargetType: "slice"},
		{Delimiter: ',', Comment: '\r', TargetType: "slice"},
		{Delimiter: 0, TargetType: "slice"},
		{Delimiter: '|', Comment: 'a', TargetType: "slice"},
	}
	var cases []map[string]any
	seen := map[string]bool{}
	for _, doc := range docs {
		if seen[doc] {
			continue
		}
		seen[doc] = true
		for _, d := range decs {
			cases = append(cases, decodeCase(d, metadecoders.CSV, doc))
		}
	}
	return cases
}

// csvTestInputs returns the Input strings of encoding/csv's reader tests (with
// the §/¶ position markers removed).
func csvTestInputs(goroot string) []string {
	var out []string
	for _, s := range literals(filepath.Join(goroot, "src/encoding/csv/reader_test.go")) {
		s = strings.NewReplacer("§", "", "¶", "", "∑", "").Replace(s)
		if len(s) < 400 {
			out = append(out, s)
		}
	}
	return out
}

// ---------------------------------------------------------------------------
// XML

func xmlCorpus(goroot, modcache string, rnd *rand.Rand, n int) []map[string]any {
	docs := []string{
		"<root><a>1</a></root>", "<root/>", "<root></root>", "<root>text</root>", "<root a=\"1\"/>",
		"<root a=\"1\">text</root>", "<root a='1' b=\"2\"><c>3</c></root>", "<root><a>1</a><a>2</a><a>3</a></root>",
		"<root><a><b>x</b></a><a>y</a></root>", "<root>t1<a>1</a>t2</root>", "<root><a>1</a>tail</root>",
		"lead<root><a>1</a></root>", "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n  <a>1</a>\n</root>\n",
		"<?xml version=\"1.1\"?><root/>", "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><root/>",
		"<?xml encoding='utf-8'?><root><x/></root>", "<!-- c --><root><a>1</a><!-- in --></root>",
		"<!DOCTYPE root [<!ENTITY e \"v\">]><root><a>&e;</a></root>", "<root><a>&lt;&gt;&amp;&apos;&quot;</a></root>",
		"<root><a>&#65;&#x42;&#x1F600;</a></root>", "<root><a>&unknown;</a></root>", "<root><a>&amp</a></root>",
		"<root><a><![CDATA[<b>raw</b>]]></a></root>", "<root><a>]]></a></root>", "<root><a>x</b></root>",
		"<root><a>", "<root>", "", "   ", "text only", "<root><a b></a></root>", "<root><a b=1></a></root>",
		"<root><a:b xmlns:a=\"u\">1</a:b></root>", "<a:root xmlns:a=\"u\"><a:x>1</a:x></a:root>",
		"<root xmlns=\"u\"><x>1</x></root>", "<root><x xml:lang=\"en\">1</x></root>", "<root><a:b>1</c:b></root>",
		"<root><a>\t\r\n x \r\n</a></root>", "<root><a>\x01</a></root>", "<root><a>\xff</a></root>",
		"<root><é>1</é></root>", "<root><1a>1</1a></root>", "<root><a-b.c_d>1</a-b.c_d></root>",
		"<root><a x=\"1\" x=\"2\"/></root>", "<root><a x=\"1\" y:x=\"2\"/></root>", "<root><a -x=\"1\"/></root>",
		"<root><a>1</a><b/><a>2</a></root>", "<root><a/><a/></root>", "<root><a x=\"1\"/><a>2</a></root>",
		"<root><a><![CDATA[]]></a></root>", "<root><?pi data?><a>1</a></root>", "<root><a>1</a></root><extra>",
		"<root><a>1</a></root>garbage", "</root>", "<root></root></root>", "<root><a>1</a>", "<root a=\"<\"/>",
		"<root a=\"&lt;&#x9;\"/>", "<root\n  a=\"1\"\n  b=\"2\"\n/>", "<root><a>x\ry</a></root>", "<?xml?><root/>",
		"<root>\n<a>1</a>\n<a>\n  <b>2</b>\n</a>\n</root>", "<root><#text>1</#text></root>", "<ro ot/>",
		"<root><a>1</a><!DOCTYPE x></root>", "<!DOCTYPE html PUBLIC \"-//W3C//DTD\" \"x\"><root><a>1</a></root>",
		"<!DOCTYPE x [ <!-- c --> <!ELEMENT x ANY> ]><root><a>1</a></root>", "<root><a>--></a></root>",
		"<root><!-- a -- b --></root>", "<root><!- x --></root>", "<root><![CDAT[x]]></root>",
	}
	for _, dir := range []string{filepath.Join(goroot, "src/encoding/xml"), filepath.Join(modcache, "github.com/clbanning/mxj/v2@v2.7.0")} {
		entries, _ := os.ReadDir(dir)
		var names []string
		for _, e := range entries {
			names = append(names, e.Name())
		}
		sort.Strings(names)
		for _, name := range names {
			p := filepath.Join(dir, name)
			switch {
			case strings.HasSuffix(name, "_test.go"):
				for _, s := range literals(p) {
					if strings.Contains(s, "<") && len(s) < 4000 {
						docs = append(docs, s)
					}
				}
			case strings.HasSuffix(name, ".xml"):
				b, err := os.ReadFile(p)
				if err == nil && len(b) < 64<<10 {
					docs = append(docs, string(b))
				}
			}
		}
	}
	// Mutations of the small documents: every prefix and seeded byte substitutions.
	var small []string
	for _, d := range docs {
		if len(d) > 0 && len(d) <= 120 {
			small = append(small, d)
		}
	}
	subst := []byte("<>/=\"'&;!?[]-: \n\rax1\x00\xc3\xff")
	for _, s := range small[:min(len(small), 80*n)] {
		for i := 0; i < len(s); i += 1 + i/16 {
			docs = append(docs, s[:i])
		}
		for k := 0; k < 3; k++ {
			b := []byte(s)
			b[rnd.Intn(len(b))] = subst[rnd.Intn(len(subst))]
			docs = append(docs, string(b))
		}
	}
	toks := []string{"<a>", "</a>", "<b>", "</b>", "<root>", "</root>", "<a x=\"1\">", "<a/>", "text", " ", "\n", "&amp;", "&x;",
		"<![CDATA[c]]>", "<!-- c -->", "<?p i?>", "é", "<a:b>", "</a:b>", "<a y='2' z=\"3\"/>", "<c>1</c>"}
	for k := 0; k < 400*n; k++ {
		var sb strings.Builder
		sb.WriteString("<root>")
		for i := rnd.Intn(12); i >= 0; i-- {
			sb.WriteString(toks[rnd.Intn(len(toks))])
		}
		if rnd.Intn(4) > 0 {
			sb.WriteString("</root>")
		}
		docs = append(docs, sb.String())
	}
	var cases []map[string]any
	seen := map[string]bool{}
	for _, doc := range docs {
		if seen[doc] {
			continue
		}
		seen[doc] = true
		cases = append(cases, decodeCase(metadecoders.Default, metadecoders.XML, doc))
	}
	return cases
}

// ---------------------------------------------------------------------------
// Encoding

// applyMarshalTypes is transform.Remarshal's: integral float64s become int64s
// (maps only, not inside slices).
func applyMarshalTypes(m map[string]any) {
	for k, v := range m {
		switch t := v.(type) {
		case map[string]any:
			applyMarshalTypes(t)
		case float64:
			i := int64(t)
			if t == float64(i) {
				m[k] = i
			}
		}
	}
}

func encodeCorpus(root string, rnd *rand.Rand, n int) []map[string]any {
	type src struct {
		f metadecoders.Format
		s string
	}
	var srcs []src
	for _, s := range handYAML {
		srcs = append(srcs, src{metadecoders.YAML, s})
	}
	for _, s := range handTOML {
		srcs = append(srcs, src{metadecoders.TOML, s})
	}
	for _, s := range handJSON {
		srcs = append(srcs, src{metadecoders.JSON, s})
	}
	for _, f := range repoFiles(root) {
		b, err := os.ReadFile(filepath.Join(root, f))
		if err != nil {
			log.Fatal(err)
		}
		srcs = append(srcs, src{metadecoders.FormatFromString(f), string(b)})
	}
	for k := 0; k < 1500*n; k++ {
		m := map[string]any{}
		for i := rnd.Intn(6); i >= 0; i-- {
			m[randKey(rnd)] = randValue(rnd, 3)
		}
		// (A yaml.MapSlice in byte order: yaml.v2's key order is not deterministic for
		// every key set, see encodeResult.)
		b, err := yaml.Marshal(ordered(m))
		if err != nil {
			log.Fatal(err)
		}
		srcs = append(srcs, src{metadecoders.YAML, string(b)})
	}
	for k := 0; k < 300*n; k++ {
		m := map[string]any{}
		for i := rnd.Intn(6); i >= 0; i-- {
			m[randKey(rnd)] = randValue(rnd, 3)
		}
		if b, err := jsonMarshal(m); err == nil {
			srcs = append(srcs, src{metadecoders.JSON, string(b)})
		}
	}

	var cases []map[string]any
	seen := map[string]bool{}
	for _, s := range srcs {
		key := string(s.f) + "\x00" + s.s
		if seen[key] {
			continue
		}
		seen[key] = true
		for _, apply := range []bool{false, true} {
			m, err := metadecoders.Default.UnmarshalToMap([]byte(s.s), s.f)
			if err != nil || m == nil {
				continue
			}
			if apply {
				applyMarshalTypes(m)
			}
			c := map[string]any{"format": string(s.f), "src": goval.Str(s.s), "apply": apply}
			for _, to := range []metadecoders.Format{metadecoders.YAML, metadecoders.TOML, metadecoders.XML, metadecoders.JSON} {
				c[string(to)] = encodeResult(m, to)
			}
			cases = append(cases, c)
		}
	}
	return cases
}

// encodeResult is InterfaceToConfig's output ({"ok"}) or error ({"err"}). mxj
// checks a map's attributes in Go's random map order, so with several invalid
// attributes the error names any of them: such results list every error seen
// in 200 runs instead ({"alts"}, sorted).
func encodeResult(m map[string]any, to metadecoders.Format) map[string]any {
	run := func() map[string]any {
		return call(func() map[string]any {
			var buf bytes.Buffer
			if err := hparser.InterfaceToConfig(m, to, &buf); err != nil {
				return map[string]any{"err": goval.Str(err.Error())}
			}
			return map[string]any{"ok": goval.Str(buf.String())}
		})
	}
	r := run()
	e, isErr := r["err"].(string)
	if (isErr && strings.HasPrefix(e, "invalid attribute value for: ")) || to == metadecoders.YAML {
		// yaml.v2 sorts the map keys (in Go's random map order) with sort.Sort and
		// a comparator that is not a strict weak order for keys mixing digits and
		// other characters ("01", "1.5", "0x1F"): the result can depend on the
		// initial order.
		key := func(r map[string]any) string {
			b, _ := json.Marshal(r)
			return string(b)
		}
		seen := map[string]map[string]any{key(r): r}
		for i := 0; i < 200; i++ {
			rr := run()
			seen[key(rr)] = rr
		}
		if len(seen) == 1 {
			return r
		}
		var keys []string
		for k := range seen {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		var alts []any
		for _, k := range keys {
			alts = append(alts, seen[k])
		}
		return map[string]any{"alts": alts}
	}
	return r
}

// ordered returns v with every map[string]any replaced by a yaml.MapSlice in
// byte order of the keys.
func ordered(v any) any {
	switch x := v.(type) {
	case map[string]any:
		keys := make([]string, 0, len(x))
		for k := range x {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		ms := yaml.MapSlice{}
		for _, k := range keys {
			ms = append(ms, yaml.MapItem{Key: k, Value: ordered(x[k])})
		}
		return ms
	case []any:
		out := make([]any, len(x))
		for i, e := range x {
			out[i] = ordered(e)
		}
		return out
	}
	return v
}

func jsonMarshal(v any) ([]byte, error) {
	var buf bytes.Buffer
	err := hparser.InterfaceToConfig(v, metadecoders.JSON, &buf)
	return buf.Bytes(), err
}

var keyPool = []string{
	"a", "b", "title", "Title", "a1", "a2", "a10", "a01", "a001", "a0", "1", "2", "10", "01", "_x", "x_y", "x-y",
	"x.y", "x y", "", "-attr", "-", "#text", "é", "ü", "漢字", "key's", "q\"k", "a:b", "a#b", "yes", "null", "true",
	"1.5", "0x1F", "~", "@at", "[x]", "{x}", "a\tb", "a\nb", "Z", "z", "B", "aB", "Ab", "b1c", "b01c", "b10c", "-x",
	"-y", "#", "a\u00a0", "ⅷ", "٣", "a٣", "a3", "a03",
}

func randKey(rnd *rand.Rand) string {
	if rnd.Intn(8) == 0 {
		return randString(rnd)
	}
	return keyPool[rnd.Intn(len(keyPool))]
}

var strPool = []string{
	"", " ", "x", "hello world", "yes", "no", "on", "off", "y", "n", "null", "~", "true", "False", "1", "-1", "1.5",
	"1e5", "0x1F", "0o7", "0b10", ".inf", "-.inf", ".nan", "1:20", "1:20:30.5", "-1:59", "190:20:30", "2001-12-14",
	"2001-12-14t21:59:43.10-05:00", "2021-01-01T00:00:00Z", "a: b", "a:b", "a #b", "a#b", "#x", "- x", "-x", "--- x",
	"---", "...", "? x", "?x", ": x", "[x]", "{x}", "!tag", "&a", "*a", "|x", ">x", "'q'", "\"q\"", "%x", "@x", "`x",
	" lead", "trail ", "a\nb", "a\n", "\na", "a\n\n", "\n", "a \nb", "a\n b", "tab\tx", "cr\rx", "nul\x00x",
	"bell\x07", "esc\x1b", "del\x7f", "\u0085", "\u00a0", "\u2028", "\u2029", "\ufeffbom", "x\ufeff", "\ufffe",
	"é", "漢字", "😀", "a\u200bb", "<tag>", "a&b", "a<b", "a>b", "it's", "say \"hi\"", "back\\slash", "a,b",
	"long " + strings.Repeat("word ", 30), strings.Repeat("x", 100), strings.Repeat("ab ", 40) + "end",
	"a  b", "  ", "_", "=", "+", "0", "00", "0.0", "-0", "+1", "1_000", "Yes", "NO", "<<", "=",
	"\xff\xfe", "ok\xffbad", "multi\nline\ntext\n", "trailing\n\n", strings.Repeat("\xf0", 60),
}

func randString(rnd *rand.Rand) string {
	if rnd.Intn(4) == 0 {
		var sb strings.Builder
		for i := rnd.Intn(5); i >= 0; i-- {
			sb.WriteString(strPool[rnd.Intn(len(strPool))])
		}
		return sb.String()
	}
	return strPool[rnd.Intn(len(strPool))]
}

func randValue(rnd *rand.Rand, depth int) any {
	k := rnd.Intn(14)
	if depth <= 0 && k >= 10 {
		k = rnd.Intn(10)
	}
	switch k {
	case 0, 1, 2:
		return randString(rnd)
	case 3:
		return []int64{0, 1, -1, 42, math.MaxInt64, math.MinInt64, 1 << 53, 1000000}[rnd.Intn(8)]
	case 4:
		return []float64{0, 1, 1.5, -2.25, 1e21, 1e20, 1e-7, 0.1, 3.0, 123456789.0, math.Inf(1), math.Inf(-1), math.NaN(), 1e100, 5e-324}[rnd.Intn(15)]
	case 5:
		return rnd.Intn(2) == 0
	case 6:
		return nil
	case 7:
		return uint64(math.MaxUint64 - uint64(rnd.Intn(3)))
	case 8, 9:
		return strPool[rnd.Intn(len(strPool))]
	case 10, 11:
		var l []any
		for i := rnd.Intn(5); i > 0; i-- {
			l = append(l, randValue(rnd, depth-1))
		}
		if l == nil {
			return []any{}
		}
		return l
	default:
		m := map[string]any{}
		for i := rnd.Intn(5); i > 0; i-- {
			m[randKey(rnd)] = randValue(rnd, depth-1)
		}
		return m
	}
}

var handYAML = []string{
	"a: 1\nb: [1, 2]\n", "title: Hello\nparams:\n  x: 1\n  y: [a, b]\n", "a: 1.0\nb: 2.5\nc: {d: 3.0}\ne: [1.0]\n",
	"list:\n- {a: 1}\n- {b: 2}\n", "list:\n- {a: 1}\n- 2\n", "empty: {}\nel: []\nn: null\n", "nested:\n  deeper:\n    deepest: x\n",
	"arr:\n- - 1\n  - 2\n- - 3\n", "m:\n  -a: attr\n  '#text': t\n", "m:\n  -a: attr\n  '#text': t\n  c: 1\n",
	"m:\n  -a: 1\n  -b: true\n  -c: 1.5\n", "m:\n  -a: [1]\n", "m:\n  -a: {x: 1}\n", "m:\n  -a: null\n", "m:\n  '#text': x\n",
	"m:\n  -a: x\n", "m:\n  '#text': [1, 2]\n", "m:\n  '#text': null\n  -a: x\n", "x: !!binary aGVsbG8=\n",
	"d: 2001-12-14\nt: 2001-12-14t21:59:43.10-05:00\n", "big: 18446744073709551615\n", "neg: -9223372036854775808\n",
	"'': empty\n", "a: [[], {}]\n", "a: [{}]\n", "a: [[{}]]\n", "a: [{b: [{c: 1}]}]\n", "a: [{b: {c: 1}}]\n",
	"s: \"a\\tb\\u0000c\"\n", "k:\n  z: 1\n  a: 2\n  m: 3\n", "a:\n  b:\n    c:\n      d: 1\n    e: 2\n  f: 3\ng: 4\n",
	"tables:\n- a: 1\n  sub: {x: 1}\n- a: 2\n", "mixed: [1, {a: 1}, [2]]\n", "x: [null, 1]\n", "x: [[null]]\n",
}

var handTOML = []string{
	"a = 1\nb = 1.5\n", "title = 'T'\n[[arr]]\nx = 1\n[[arr]]\nx = 2\n", "a = 1\nb = \"x\"\n[c]\nd = 2021-01-01T00:00:00Z\ne = 1979-05-27\n",
	"d1 = 1979-05-27T07:32:00Z\nd2 = 1979-05-27T00:32:00.999999-07:00\nd3 = 1979-05-27T07:32:00\nd4 = 07:32:00.5\nd5 = 1979-05-27\n",
	"[a.b.c]\nd = 1\n[a]\nx = 2\n", "arr = [[1, 2], [3]]\ntab = [{a = 1}, {b = 2}]\n", "x = [1979-05-27, 07:32:00]\n",
	"f = inf\ng = -inf\nh = nan\ni = 1e300\nj = -0.0\n", "s = '''\nmulti\nline'''\n", "\"key with space\" = 1\n'q' = 2\n",
	"title = \"Test Metadata\"\n\n[[resources]]\n  src = \"**image-4.png\"\n  title = \"The Fourth Image!\"\n  [resources.params]\n    byline = \"picasso\"\n",
	"[server]\n[[server.headers]]\nfor = '/**'\n[server.headers.values]\nX-Frame-Options = 'DENY'\n",
	"a = {b = {c = 1}}\n", "a = [{b = [{c = 1}]}]\n", "nano = 2021-01-01T00:00:00.123456789+07:00\n",
}

var handJSON = []string{
	`{"a": 1, "b": 2.5, "c": {"d": 3.0}, "e": [1.0]}`, `{"a": [1, "x", null, true, {"b": []}]}`, `{"x": {}}`,
	`{"-a": "1", "#text": "t"}`, `{"n": 12345678901234567890, "f": 1e21, "g": 0.000001, "h": 1e-7}`, `{"":{"":""}}`,
	`{"a": [[1], [2, [3]]], "b": [{"c": {"d": [{"e": 1}]}}]}`, `{"s": "\u0000\u001f\u007f\u0085\u2028"}`,
}

// repoFiles lists the repository's TOML, YAML and JSON data/config files
// (sorted), outside crates/, tools/, .git, testdata-like large trees and
// node_modules, up to 16 KiB.
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
			case ".git", "crates", "tools", "node_modules", ".claude":
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
		if info.Size() > 16<<10 {
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
