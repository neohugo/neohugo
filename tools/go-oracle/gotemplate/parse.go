//go:build gotemplate_oracle

// Mode "parse": parses a corpus of template sources with the forked
// text/template/parse package and dumps, per input, either the error or
// every tree (sorted by name) as Root.String() plus a structural dump.
// The Rust test crates/gotemplate/tests/parse_oracle.rs parses the same
// sources and must produce the identical dump.
//
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate parse crates/gotemplate/tests/fixtures/text
//
// Output: <outdir>/parse.txt.gz. Record format (all strings strconv.Quote'd):
//
//	funcs <kind> <name>...               (the function name sets, before the cases)
//	#case <n> <name>
//	mode <mode> <left> <right> <funcs>   (funcs: "go" = parse_test builtins, "hugo" = Hugo's names + builtins, "none")
//	src <source>
//	err <message>                        (on error), or per tree:
//	tree <name> <parseName> <mode>
//	str <Root.String()>
//	<indent><node dump>                  (see ptxDumpNode)
//	#end
package main

import (
	"bufio"
	"compress/gzip"
	"fmt"
	"io/fs"
	"math"
	"math/rand"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate/parse"
)

func init() {
	register("parse", ptxMain)
}

// ptxCase is one parse input.
type ptxCase struct {
	name        string
	src         string
	mode        parse.Mode
	left, right string
	funcs       string // "go", "hugo", "none"
}

// ptxGoFuncs is parse_test.go's builtins map.
var ptxGoFuncs = map[string]any{"printf": fmt.Sprintf, "contains": strings.Contains}

// ptxBuiltinNames are text/template's builtins (funcs.go:builtins).
var ptxBuiltinNames = []string{
	"and", "call", "html", "index", "slice", "js", "len", "not", "or", "print", "printf", "println",
	"urlquery", "eq", "ge", "gt", "le", "lt", "ne",
}

// ptxHugoNames are Hugo's template function names (namespaces and
// aliases, docs/rust-port/specs/template-engine.md §4.1) plus the
// html/template escapers; together with the builtins they are the
// parse-time function set of a Hugo template.
var ptxHugoNames = strings.Fields(`absLangURL absURL add after anchorize append apply babel
base64Decode base64Encode chomp complement cond countrunes countwords dateFormat default delimit dict div
doDefer duration emojify eq errorf erroridf fileExists findRE findRESubmatch fingerprint first float ge
getCSV getenv getJSON group gt hasPrefix hasSuffix highlight hmac htmlEscape htmlUnescape humanize
i18n T imageConfig in index int intersect isSet isset jsonify keyVals last le lower lt markdownify md5
merge minify mod modBool mul ne newScratch now partial partialCached plainify pluralize pow print printf
println querify readDir readFile ref relLangURL relref relURL replace replaceRE return safeCSS safeHTML
safeHTMLAttr safeJS safeJSStr safeURL seq sha1 sha256 shuffle singularize slice slicestr sort split
string sub substr symdiff title trim truncate try union uniq unmarshal upper urldecode urlencode urlize
warnf warnidf where cast collections compare crypto css data debug diagrams encoding fmt
hash hugo images inflect js lang math openapi3 os page partials path reflect resources safe site strings
templates time transform urls _html_template_attrescaper _html_template_commentescaper
_html_template_cssescaper _html_template_cssvaluefilter _html_template_htmlnamefilter
_html_template_htmlescaper _html_template_jsregexpescaper _html_template_jsstrescaper
_html_template_jstmpllitescaper _html_template_jsvalescaper _html_template_nospaceescaper
_html_template_rcdataescaper _html_template_srcsetescaper _html_template_urlescaper
_html_template_urlfilter _html_template_urlnormalizer _eval_args_`)

func ptxFuncMaps(kind string) []map[string]any {
	switch kind {
	case "go":
		return []map[string]any{ptxGoFuncs}
	case "hugo":
		m := map[string]any{}
		for _, n := range ptxHugoNames {
			m[n] = true
		}
		b := map[string]any{}
		for _, n := range ptxBuiltinNames {
			b[n] = true
		}
		return []map[string]any{m, b}
	case "builtins":
		b := map[string]any{}
		for _, n := range ptxBuiltinNames {
			b[n] = true
		}
		return []map[string]any{b}
	}
	return nil
}

func ptxMain(args []string) error {
	if len(args) != 1 {
		return fmt.Errorf("usage: parse <outdir>")
	}
	cases := ptxCorpus()
	f, err := os.Create(filepath.Join(args[0], "parse.txt.gz"))
	if err != nil {
		return err
	}
	defer func() { _ = f.Close() }()
	zw, _ := gzip.NewWriterLevel(f, gzip.BestCompression)
	w := bufio.NewWriter(zw)
	// The function name sets, so the Rust side need not duplicate them.
	for _, kind := range []string{"go", "hugo", "builtins", "none"} {
		var names []string
		for _, m := range ptxFuncMaps(kind) {
			for n := range m {
				names = append(names, n)
			}
		}
		sort.Strings(names)
		_, _ = fmt.Fprintf(w, "funcs %s", kind)
		for _, n := range names {
			_, _ = fmt.Fprintf(w, " %s", q(n))
		}
		_, _ = fmt.Fprintf(w, "\n")
	}
	for i, c := range cases {
		ptxRun(w, i, c)
	}
	if err := w.Flush(); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	fmt.Fprintf(os.Stderr, "parse: %d cases\n", len(cases))
	return nil
}

func ptxRun(w *bufio.Writer, i int, c ptxCase) {
	_, _ = fmt.Fprintf(w, "#case %d %s\n", i, q(c.name))
	_, _ = fmt.Fprintf(w, "mode %d %s %s %s\n", c.mode, q(c.left), q(c.right), c.funcs)
	_, _ = fmt.Fprintf(w, "src %s\n", q(c.src))
	treeSet := map[string]*parse.Tree{}
	t := parse.New(c.name)
	t.Mode = c.mode
	_, err := t.Parse(c.src, c.left, c.right, treeSet, ptxFuncMaps(c.funcs)...)
	if err != nil {
		_, _ = fmt.Fprintf(w, "err %s\n", q(err.Error()))
		_, _ = fmt.Fprintf(w, "#end\n")
		return
	}
	var names []string
	for n := range treeSet {
		names = append(names, n)
	}
	sort.Strings(names)
	for _, n := range names {
		tr := treeSet[n]
		_, _ = fmt.Fprintf(w, "tree %s %s %d\n", q(tr.Name), q(tr.ParseName), tr.Mode)
		_, _ = fmt.Fprintf(w, "str %s\n", q(tr.Root.String()))
		ptxDumpNode(w, tr, tr.Root, 0)
		// A copy prints the same and keeps the original error context.
		cp := tr.Copy()
		if cp.Root.String() != tr.Root.String() {
			_, _ = fmt.Fprintf(w, "copy-differs\n")
		}
	}
	_, _ = fmt.Fprintf(w, "#end\n")
}

// ptxLoc is the ErrorContext location of n without the (repeated)
// "ParseName:" prefix, or "!" and the whole quoted location if the
// prefix differs.
func ptxLoc(tr *parse.Tree, n parse.Node) string {
	loc, _ := tr.ErrorContext(n)
	if rest, ok := strings.CutPrefix(loc, tr.ParseName+":"); ok {
		return rest
	}
	return "!" + q(loc)
}

func ptxF64(f float64) string { return fmt.Sprintf("%016x", math.Float64bits(f)) }

func ptxIdents(ss []string) string {
	var b strings.Builder
	for i, s := range ss {
		if i > 0 {
			b.WriteByte(' ')
		}
		b.WriteString(q(s))
	}
	return b.String()
}

// ptxDumpNode writes one line per node: kind, position, the ErrorContext
// location, kind-specific fields; children follow indented by one space.
func ptxDumpNode(w *bufio.Writer, tr *parse.Tree, n parse.Node, depth int) {
	ind := strings.Repeat(" ", depth)
	switch n := n.(type) {
	case *parse.ListNode:
		_, _ = fmt.Fprintf(w, "%sList %d %s\n", ind, n.Pos, ptxLoc(tr, n))
		for _, c := range n.Nodes {
			ptxDumpNode(w, tr, c, depth+1)
		}
	case *parse.TextNode:
		_, _ = fmt.Fprintf(w, "%sText %d %s %s\n", ind, n.Pos, ptxLoc(tr, n), q(string(n.Text)))
	case *parse.CommentNode:
		_, _ = fmt.Fprintf(w, "%sComment %d %s %s\n", ind, n.Pos, ptxLoc(tr, n), q(n.Text))
	case *parse.ActionNode:
		_, _ = fmt.Fprintf(w, "%sAction %d %s L%d %s\n", ind, n.Pos, ptxLoc(tr, n), n.Line, q(n.String()))
		ptxDumpNode(w, tr, n.Pipe, depth+1)
	case *parse.PipeNode:
		_, _ = fmt.Fprintf(w, "%sPipe %d %s L%d assign=%t decl=%d cmds=%d %s\n", ind, n.Pos, ptxLoc(tr, n), n.Line,
			n.IsAssign, len(n.Decl), len(n.Cmds), q(n.String()))
		for _, d := range n.Decl {
			ptxDumpNode(w, tr, d, depth+1)
		}
		for _, c := range n.Cmds {
			ptxDumpNode(w, tr, c, depth+1)
		}
	case *parse.CommandNode:
		_, _ = fmt.Fprintf(w, "%sCommand %d %s args=%d %s\n", ind, n.Pos, ptxLoc(tr, n), len(n.Args), q(n.String()))
		for _, a := range n.Args {
			ptxDumpNode(w, tr, a, depth+1)
		}
	case *parse.IdentifierNode:
		_, _ = fmt.Fprintf(w, "%sIdentifier %d %s %s\n", ind, n.Pos, ptxLoc(tr, n), q(n.Ident))
	case *parse.VariableNode:
		_, _ = fmt.Fprintf(w, "%sVariable %d %s %s\n", ind, n.Pos, ptxLoc(tr, n), ptxIdents(n.Ident))
	case *parse.DotNode:
		_, _ = fmt.Fprintf(w, "%sDot %d %s\n", ind, n.Pos, ptxLoc(tr, n))
	case *parse.NilNode:
		_, _ = fmt.Fprintf(w, "%sNil %d %s\n", ind, n.Pos, ptxLoc(tr, n))
	case *parse.FieldNode:
		_, _ = fmt.Fprintf(w, "%sField %d %s %s\n", ind, n.Pos, ptxLoc(tr, n), ptxIdents(n.Ident))
	case *parse.ChainNode:
		_, _ = fmt.Fprintf(w, "%sChain %d %s %s %s\n", ind, n.Pos, ptxLoc(tr, n), ptxIdents(n.Field), q(n.String()))
		ptxDumpNode(w, tr, n.Node, depth+1)
	case *parse.BoolNode:
		_, _ = fmt.Fprintf(w, "%sBool %d %s %t\n", ind, n.Pos, ptxLoc(tr, n), n.True)
	case *parse.NumberNode:
		_, _ = fmt.Fprintf(w, "%sNumber %d %s %s int=%t:%d uint=%t:%d float=%t:%s complex=%t:%s:%s\n", ind, n.Pos,
			ptxLoc(tr, n), q(n.Text), n.IsInt, n.Int64, n.IsUint, n.Uint64, n.IsFloat, ptxF64(n.Float64),
			n.IsComplex, ptxF64(real(n.Complex128)), ptxF64(imag(n.Complex128)))
	case *parse.StringNode:
		_, _ = fmt.Fprintf(w, "%sString %d %s %s %s\n", ind, n.Pos, ptxLoc(tr, n), q(n.Quoted), q(n.Text))
	case *parse.IfNode:
		ptxDumpBranch(w, tr, "If", &n.BranchNode, depth)
	case *parse.RangeNode:
		ptxDumpBranch(w, tr, "Range", &n.BranchNode, depth)
	case *parse.WithNode:
		ptxDumpBranch(w, tr, "With", &n.BranchNode, depth)
	case *parse.TemplateNode:
		_, _ = fmt.Fprintf(w, "%sTemplate %d %s L%d %s %s\n", ind, n.Pos, ptxLoc(tr, n), n.Line, q(n.Name), q(n.String()))
		if n.Pipe == nil {
			_, _ = fmt.Fprintf(w, "%s nopipe\n", ind)
		} else {
			ptxDumpNode(w, tr, n.Pipe, depth+1)
		}
	case *parse.BreakNode:
		_, _ = fmt.Fprintf(w, "%sBreak %d %s L%d\n", ind, n.Pos, ptxLoc(tr, n), n.Line)
	case *parse.ContinueNode:
		_, _ = fmt.Fprintf(w, "%sContinue %d %s L%d\n", ind, n.Pos, ptxLoc(tr, n), n.Line)
	default:
		_, _ = fmt.Fprintf(w, "%sUNKNOWN %T\n", ind, n)
	}
}

func ptxDumpBranch(w *bufio.Writer, tr *parse.Tree, kind string, b *parse.BranchNode, depth int) {
	ind := strings.Repeat(" ", depth)
	_, _ = fmt.Fprintf(w, "%s%s %d %s L%d\n", ind, kind, b.Pos, ptxLoc(tr, b), b.Line)
	ptxDumpNode(w, tr, b.Pipe, depth+1)
	ptxDumpNode(w, tr, b.List, depth+1)
	if b.ElseList == nil {
		_, _ = fmt.Fprintf(w, "%s noelse\n", ind)
	} else {
		_, _ = fmt.Fprintf(w, "%s else\n", ind)
		ptxDumpNode(w, tr, b.ElseList, depth+1)
	}
}

// ---------------------------------------------------------------------------
// Corpus

func ptxCorpus() []ptxCase {
	var cs []ptxCase
	add := func(name, src string, mode parse.Mode, left, right, funcs string) {
		cs = append(cs, ptxCase{name: name, src: src, mode: mode, left: left, right: right, funcs: funcs})
	}
	// Go's own tables (parse_test.go, lex_test.go) in every mode.
	for _, s := range ptxGoTableInputs {
		add("gotable", s, 0, "", "", "go")
		add("gotable", s, parse.ParseComments, "", "", "go")
		add("gotable", s, parse.SkipFuncCheck, "", "", "none")
		add("gotable", s, parse.ParseComments|parse.SkipFuncCheck, "", "", "builtins")
	}
	for _, s := range ptxDelimInputs {
		add("delims", s, 0, "$$", "@@", "go")
		add("delims", s, parse.ParseComments, "{{hub", "hub}}", "go")
		add("delims", s, 0, "{{- ", " -}}", "go")
	}
	// Number syntaxes, as actions, arguments and chain heads.
	for _, n := range ptxNumbers {
		add("number", "{{"+n+"}}", 0, "", "", "builtins")
		add("number", "{{print "+n+" "+n+"}}", 0, "", "", "builtins")
		add("number", "{{"+n+".X}}", 0, "", "", "builtins")
	}
	// Seeksnack-like layouts.
	for i, s := range ptxLayouts {
		add(fmt.Sprintf("layout%d", i), s, 0, "", "", "hugo")
		add(fmt.Sprintf("layout%d", i), s, parse.ParseComments, "", "", "hugo")
	}
	// Real Hugo layouts from this repository.
	for _, root := range []string{"tpl/tplimpl/embedded/templates", "docs/layouts", "create/skeletons"} {
		var files []string
		_ = filepath.WalkDir(root, func(p string, d fs.DirEntry, err error) error {
			if err == nil && !d.IsDir() {
				files = append(files, p)
			}
			return nil
		})
		sort.Strings(files)
		for _, p := range files {
			switch filepath.Ext(p) {
			case ".html", ".xml", ".txt", ".json":
			default:
				continue
			}
			b, err := os.ReadFile(p)
			if err != nil {
				continue
			}
			add(p, string(b), 0, "", "", "hugo")
			if strings.HasPrefix(p, "tpl/") {
				add(p, string(b), parse.ParseComments, "", "", "hugo")
			}
		}
	}
	// Fuzz from a fixed seed.
	r := rand.New(rand.NewSource(20260927))
	for i := 0; i < 3000; i++ {
		g := &ptxGen{r: r, left: "{{", right: "}}", clean: r.Intn(10) < 7}
		mode := parse.Mode(0)
		funcs := "builtins"
		switch r.Intn(8) {
		case 0:
			mode = parse.ParseComments
		case 1:
			mode = parse.SkipFuncCheck
			funcs = "none"
		case 2:
			g.left, g.right = ptxDelimPairs[r.Intn(len(ptxDelimPairs))][0], ptxDelimPairs[r.Intn(len(ptxDelimPairs))][1]
		case 3:
			funcs = "hugo"
		}
		src := g.template(0)
		if !g.clean && r.Intn(3) == 0 {
			src = g.mutate(src)
		}
		name := "fuzz"
		if r.Intn(10) == 0 {
			name = ptxNames[r.Intn(len(ptxNames))]
		}
		left, right := g.left, g.right
		if left == "{{" && right == "}}" && r.Intn(2) == 0 {
			left, right = "", ""
		}
		add(name, src, mode, left, right, funcs)
	}
	return cs
}

var ptxNames = []string{"", "a%b", "x/y.html", "_partials/p.html", "名前", "a\"b", "%d%s"}

var ptxDelimPairs = [][2]string{
	{"{{", "}}"}, {"<<", ">>"}, {"[[", "]]"}, {"{%", "%}"}, {"$$", "@@"}, {"{{-", "-}}"}, {"(", ")"}, {"<!--", "-->"},
}

var ptxNumbers = []string{
	"0", "-0", "+0", "17", "-17", "+17", "1_000", "1__0", "_1", "1_", "0x1F", "0X1f", "0x_1F", "0x", "0xg",
	"0o17", "0O17", "0o8", "0b101", "0B1_0", "0b2", "017", "08", "0_7", "1.5", ".5", "5.", "-.5", "+.5",
	"1e3", "1E-3", "1e+3", "1e", "1e+", "1.5e3", "1_0.5_0e1_0", "0x1p4", "0x1.8p1", "0X1P-2", "0x1p", "0x.8p0",
	"0x1.p1", "0x_1p-4", "1i", "2.5i", "0x10i", "0i", "-0i", "0.i", "1e3i", "0x1p2i", "073i", "0b1i", "0o7i",
	"1+2i", "-1-2i", "1-0i", "+1+1i", "1.5e3+2.5e-3i", "0x1p2+0x1p1i", "0x10+1i", "1_0+2i", "017+1i", "0o7+1i",
	"0b1+1i", "1e400+1i", "1+1e400i", "1+2", "1+2.", "1+2ii", "1+i", "1++2i",
	"9223372036854775807", "9223372036854775808", "-9223372036854775808", "-9223372036854775809",
	"18446744073709551615", "18446744073709551616", "1e19", "1e20", "-1e19", "1e400", "1e-400", "4.9e-324",
	"2.5e-324", "0x1p-1074", "0x1p-1075", "0x1.fffffffffffffp1023", "0x1p1024", "1.7976931348623157e308",
	"1.7976931348623159e308", "0.1", "0.30000000000000004", "123456789012345678901234567890",
	"0x7fffffffffffffff", "0x8000000000000000", "0xffffffffffffffff", "0x10000000000000000",
	"'a'", "'\\n'", "'\\x41'", "'\\xff'", "'\\u00e9'", "'\\U0001F600'", "'é'", "'ab'", "'\\q'", "''", "'\\''",
	"'\\\"'", "'\"'", "'\\0'", "'\\000'", "'\\377'", "'\\400'", "'\\uD800'", "'\\U00110000'", "'\\a'",
	"'\xff'", "'\\t'", "'本'", "'433937734937734969526500969526500'",
	"+", "-", ".", "..", "+-2", "-+2", "0x123.", "1e.", "0xi.", "3k", "1.2.3", "1..2", "1e1e1",
}

// ptxGoTableInputs are the inputs of Go's parse and lex test tables.
var ptxGoTableInputs = []string{
	// parseTests
	"", "{{/*\n\n\n*/}}", " \t\n", "some text", "{{}}", "{{.X}}", "{{printf}}", "{{$}}",
	"{{with $x := 3}}{{$x 23}}{{end}}", "{{$.I}}", "{{printf `%d` 23}}", "{{.X|.Y}}", "{{$x := .X|.Y}}",
	"{{.X (.Y .Z) (.A | .B .C) (.E)}}", "{{(.Y .Z).Field}}", "{{if .X}}hello{{end}}",
	"{{if .X}}true{{else}}false{{end}}", "{{if .X}}true{{else if .Y}}false{{end}}",
	"+{{if .X}}X{{else if .Y}}Y{{else if .Z}}Z{{end}}+", "{{range .X}}hello{{end}}",
	"{{range .X.Y.Z}}hello{{end}}", "{{range .X}}hello{{range .Y}}goodbye{{end}}{{end}}",
	"{{range .X}}true{{else}}false{{end}}", "{{range .X|.M}}true{{else}}false{{end}}",
	"{{range .SI}}{{.}}{{end}}", "{{range $x := .SI}}{{.}}{{end}}", "{{range $x, $y := .SI}}{{.}}{{end}}",
	"{{range .SI}}{{.}}{{break}}{{end}}", "{{range .SI}}{{.}}{{continue}}{{end}}",
	"{{range .SI 1 -3.2i true false 'a' nil}}{{end}}", "{{template `x`}}", "{{template `x` .Y}}",
	"{{with .X}}hello{{end}}", "{{with .X}}hello{{else}}goodbye{{end}}",
	"{{with .X}}hello{{else with .Y}}goodbye{{end}}", "{{with .X}}X{{else with .Y}}Y{{else with .Z}}Z{{end}}",
	"x \r\n\t{{- 3}}", "{{3 -}}\n\n\ty", "x \r\n\t{{- 3 -}}\n\n\ty", "x\n{{-  3   -}}\ny",
	"x \r\n\t{{- /* hi */}}", "{{/* hi */ -}}\n\n\ty", "x \r\n\t{{- /* */ -}}\n\n\ty",
	`{{block "foo" .}}hello{{end}}`, "{{ $x \n := \n 1 \n }}", "{{\n}}", "{{\n\"x\"\n|\nprintf\n}}",
	"{{/*\nhello\n*/}}", "{{-\n/*\nhello\n*/\n-}}", "{{range .SI}}{{.}}{{ continue }}{{end}}",
	"{{range .SI}}{{.}}{{ break }}{{end}}", "hello{{range", "{{end}}", "{{else}}", "{{if .X}}hello{{end}}{{else}}",
	"{{if .X}}1{{else}}2{{else}}3{{end}}", "hello{{range .x}}", "hello{{range .x}}{{else}}", "hello{{undefined}}",
	"{{$x}}", "{{with $x := 4}}{{end}}{{$x}}", "{{template $v}}", "{{with $x.Y := 4}}{{end}}", "{{template .X}}",
	"{{printf 3, 4}}", "{{with $v, $u := 3}}{{end}}", "{{range $u, $v, $w := 3}}{{end}}",
	"{{printf (printf .).}}", "{{printf 3`x`}}", "{{printf `x`.}}", "{{if .X}}a{{else if .Y}}b{{end}}{{end}}",
	"{{range .}}{{end}} {{break}}", "{{range .}}{{end}} {{continue}}", "{{range .}}{{else}}{{break}}{{end}}",
	"{{range .}}{{else}}{{continue}}{{end}}", "{{$x := 0}}{{$x}}", "{{$x += 1}}{{$x}}", "{{$x ! 2}}{{$x}}",
	"{{$x % 3}}{{$x}}", "{{range $x := $y := 3}}{{end}}", "{{$x:=.}}{{$x!2}}", "{{$x:=.}}{{$x+2}}",
	"{{$x:=.}}{{$x +2}}", "{{range $x := 0}}{{$x}}{{end}}", "{{range $x = 0}}{{$x}}{{end}}", "{{1.E}}",
	"{{0.1.E}}", "{{true.E}}", "{{'a'.any}}", `{{"hello".guys}}`, "{{..E}}", "{{nil.E}}", "{{12|.}}",
	"{{.|12|printf}}", "{{.|printf|\"error\"}}", "{{12|printf|'e'}}", "{{.|true}}", "{{'c'|nil}}",
	`{{printf "%d" ( ) }}`, `{{block "foo"}}hello{{end}}`, "{{range .X}}{{break 20}}{{end}}", "{{fn 1 2}}",
	// isEmptyTests
	" \t\n \t\n", "{{/* comment */}}", `{{define "x"}}something{{end}}`,
	"{{define `x`}}something{{end}}\n\n{{define `y`}}something{{end}}\n\n",
	"{{define `x`}}something{{end}}\nx\n{{define `y`}}something{{end}}\ny\n",
	"{{define `x`}}something{{end}}{{if 3}}foo{{end}}", "{{if true}}{{end}}",
	// errorTests
	"line1\n{{", "line1\n{{define `x`}}line2\n{{", "line1\n{{\"x\"\n\"y\"\n", "{{\n\n\n\n\n", "line1\n{{\nx\n}}",
	"{{foo}}", "{{/*}}", "{{/*\nhello\n}}", "{{.X (1 2 3}}", "{{.X 1 2 3 ) }}", "{{(.X 1 2 3", "{{`x`3}}",
	"{{a#}}", "{{'a}}", `{{"a}}`, "{{`a}}", "{{0xi}}", "{{define `a`}}a{{end}}{{define `a`}}b{{end}}",
	"{{range .X}}", "{{$x := 23}}{{with $x.y := 3}}{{$x 23}}{{end}}", "{{$a,$b,$c := 23}}", "{{$a}}",
	"{{true.any}}", "{{12|false}}", `{{ ( ) }}`, "{{ $v := `\n` }} {{", "{{range $k}}{{end}}",
	"{{range $k, $v}}{{end}}", "{{range $k,}}{{end}}", "{{range $k, $v := }}{{end}}", "{{range $k, .}}{{end}}",
	"{{range $k, 123 := .}}{{end}}",
	// TestBlock, TestLineNum
	`a{{block "inner" .}}bar{{.}}baz{{end}}b`, "{{printf 1234}}\n{{printf 1234}}\n{{printf 1234}}\n",
	// lexTests
	"now is the time", "hello-{{/* this is a comment */}}-world", "{{,@% }}", "{{((3))}}", "{{for}}",
	`{{block "foo" .}}`, `{{"abc \n\t\" "}}`, "{{`abc\\n\\t\\\" `}}", "{{`now is{{\n}}the time`}}",
	"{{1 02 0x14 0X14 -7.2i 1e3 1E3 +1.2e-4 4.2i 1+2i 1_2 0x1.e_fp4 0X1.E_FP4}}",
	`{{'a' '\n' '\'' '\\' '\u00FF' '\xFF' '本'}}`, "{{true false}}", "{{.}}", "{{nil}}", "{{.x . .2 .x.y.z}}",
	"{{range if else end with}}", "{{$c := printf $ $hello $23 $ $var.Field .Method}}", "{{$x 23}}",
	`intro {{echo hi 1.2 |noargs|args 1 "hi"}} outro`, "{{$v := 3}}", "{{$v , $w := 3}}", "{{(.X).Y}}",
	"hello- {{- 3 -}} -world", "hello- {{- /* hello */ -}} -world", "#{{\x01}}", "{{", "{{range",
	"{{\"\n\"}}", "{{`xx}}", "{{'\n}}", "{{3k}}", "{{(3}}", "{{3)}}", "{{|||||}}", "hello-{{/*/}}-world",
	"hello-{{/* */ }}-world", "hello-{.}}-world", "{{,@%#}}", "0123{{hello}}xyz", "{{x -}}\n{{y}}",
	"{{x}}\n{{- y}}", "{{/*\n*/}}\n{{undefinedFunction \"test\"}}",
	// Extra edge cases.
	"{{- -3}}", "{{-3}}", "{{-3 -}}", "a {{- -}} b", "{{- /* c */}}", "{{/* c */}}{{/* d */ -}}  x",
	"{{\t-\t3\t-\t}}", "{{-\n3\n-}}", "{{- 3 -}}{{- 4 -}}", "{{ - 3 }}", "{{3 - }}", "{{3 -}}}", "{{{3}}}",
	"{{define \"a\"}}{{end}}{{define \"a\"}}x{{end}}", "{{define \"a\"}}x{{end}}{{define \"a\"}}{{end}}",
	"{{define \"a\"}}x{{end}}{{define \"a\"}}  {{end}}", "x{{define \"x\"}}y{{end}}", "{{define \"x\"}}y{{end}}",
	"{{define \"x\"}}{{define \"y\"}}{{end}}{{end}}", "{{block \"b\" .}}{{block \"c\" .}}c{{end}}{{end}}",
	"{{define `a\\x`}}x{{end}}", "{{define \"a\\xffb\"}}x{{end}}{{template \"a\\xffb\"}}", "{{define 1}}{{end}}",
	"{{define \"x\" 1}}{{end}}", "{{template \"x\" 1 2}}", "{{template}}", "{{template \"x\" | printf}}",
	"{{block}}", "{{block \"x\" . 1}}{{end}}", "{{block \"x\" .}}{{else}}{{end}}", "{{define \"x\"}}{{else}}{{end}}",
	"{{if}}{{end}}", "{{if 1}}{{else if}}{{end}}", "{{with 1}}{{else with}}{{end}}", "{{if 1}}{{else with 2}}{{end}}",
	"{{with 1}}{{else if 2}}{{end}}", "{{if 1}}{{else if 2}}{{else}}{{end}}", "{{range 1}}{{else if 2}}{{end}}",
	"{{if 1}}{{end 2}}", "{{if 1}}{{else 2}}{{end}}", "{{else if 1}}", "{{range $i, $e := .}}{{$i}}{{$e}}{{end}}",
	"{{range $i, $e = .}}{{end}}", "{{$i := 1}}{{range $i, $e = .}}{{end}}", "{{$i := 1}}{{$e := 2}}{{range $i, $e = .}}{{end}}",
	"{{range $i := .}}{{end}}{{$i}}", "{{if $x := 1}}{{$x}}{{end}}{{$x}}", "{{with $x := 1}}{{else}}{{$x}}{{end}}",
	"{{$x := 1}}{{$x = 2}}{{$x}}", "{{$x = 2}}", "{{$ := 1}}", "{{$x, $y := 1}}", "{{$x := 1, 2}}",
	"{{range .}}{{if .}}{{break}}{{else}}{{continue}}{{end}}{{end}}",
	"{{range .}}{{with .}}{{break}}{{end}}{{end}}", "{{range .}}{{define \"x\"}}{{break}}{{end}}{{end}}",
	"{{range .}}{{block \"x\" .}}{{break}}{{end}}{{end}}", "{{range .}}{{template \"x\"}}{{end}}",
	"{{range .}}{{break 1}}{{end}}", "{{range .}}{{continue}}x{{end}}", "{{break}}", "{{continue}}",
	"{{(1).X}}", "{{(.X).Y.Z}}", "{{($).X}}", "{{$.X.Y}}", "{{(printf).X}}", "{{printf.X}}", "{{.X.Y.Z}}",
	"{{. .X}}", "{{.X .}}", "{{(.X)}}", "{{((.X))}}", "{{(.X | printf)}}", "{{printf (1) (2)}}",
	"{{1 | printf}}", "{{printf | printf}}", "{{. | printf}}", "{{.X | .Y | .Z}}", "{{$ | printf}}",
	"{{printf \"%d\" 1 | printf \"%s\"}}", "{{\"a\" \"b\"}}", "{{nil}}", "{{nil 1}}", "{{printf nil}}",
	"{{true}}{{false}}{{true 1}}", "{{`raw\nstring`}}", "{{\"\\u00e9\\x41\\101\"}}", "{{\"\\q\"}}", "{{\"\\x4\"}}",
	"{{\"\xff\"}}", "{{`\xff`}}", "é{{.}}\xff", "{{.é}}", "{{$é := 1}}{{$é}}", "{{é}}", "{{.X\xff}}",
	"{{.X!}}", "{{.X-1}}", "{{$x-1}}", "{{.X,}}", "{{.X:}}", "{{.X=1}}", "{{.X:=1}}", "{{.X ,1}}",
	"{{:}}", "{{:x}}", "{{=}}", "{{,}}", "{{|}}", "{{.}}{{", "{{.}", "{{.}}}", "}}{{.}}", "{{ }}", "{{  }}",
	"{{\n\n}}", "{{/**/}}", "{{/* */}}", "{{ /* c */ }}", "{{/*a*/ /*b*/}}", "{{/* a */ 1}}", "{{1 /* a */}}",
	"{{- /**/ -}}", "{{-/**/-}}", "{{- /**/-}}", "{{-/**/ -}}", "x\n\n{{- /* */ -}}\n\ny",
}

var ptxDelimInputs = []string{
	"$$,@%{{}}@@", "$$@@", "$$for@@", `$$"abc \n\t\" "@@`, "$$`abc\\n\\t\\\" `@@", "{{hub .host hub}}",
	"{{- .x -}} {{- - .x - -}}", "a$$.X@@b", "$$/* c */@@", "$$- /* c */ -@@", "$$- 3 -@@", "$$3@@@@",
	"$$$$3@@", "{{hub/* c */hub}}", "{{hub- 3 -hub}}", "{{- 3 -}}", "{{- - 3 - -}}",
}

var ptxLayouts = []string{
	`<!DOCTYPE html>
<html lang="{{ .Site.Language.Lang }}">
<head>
  {{- partial "head.html" . -}}
  <title>{{ block "title" . }}{{ .Title }} | {{ .Site.Title }}{{ end }}</title>
  {{ $style := resources.Get "css/main.scss" | toCSS | minify | fingerprint }}
  <link rel="stylesheet" href="{{ $style.RelPermalink }}" integrity="{{ $style.Data.Integrity }}">
</head>
<body class="{{ with .Params.bodyclass }}{{ . }}{{ else }}default{{ end }}">
  {{- block "main" . }}{{ end -}}
  {{ range $i, $p := first 10 (where .Site.RegularPages "Type" "posts") }}
    {{- if gt $i 0 }}, {{ end -}}
    <a href="{{ $p.RelPermalink }}">{{ $p.Title | upper }}</a>
  {{ else }}
    <p>No posts.</p>
  {{ end }}
  {{/* a comment
       spanning lines */}}
  {{ $json := ` + "`" + `{
    "@context": "https://schema.org",
    "name": "{{ .Title }}"
  }` + "`" + ` }}
  {{ $x := dict
       "a" 1
       "b" (slice 1 2 3)
       "c" .Params.c
  }}
  {{ with $x.c }}{{ . }}{{ else with $x.b }}{{ index . 0 }}{{ else }}none{{ end }}
  {{ if and .IsHome (not .Params.hide) }}home{{ else if eq .Kind "section" }}section{{ else }}page{{ end }}
</body>
</html>`,
	`{{ define "main" }}
{{ $paginator := .Paginate (where .Site.RegularPages "Section" "posts") 6 }}
{{ range $paginator.Pages }}
  <article>
    <h2><a href="{{ .Permalink }}">{{ .Title }}</a></h2>
    {{ with .Params.image }}<img src="{{ . | absURL }}" alt="">{{ end }}
    <time datetime="{{ .Date.Format "2006-01-02T15:04:05Z07:00" }}">{{ .Date.Format "Jan 2, 2006" }}</time>
    {{ .Summary }}
    {{ if .Truncated }}<a href="{{ .RelPermalink }}">{{ i18n "readMore" }}</a>{{ end }}
  </article>
{{ end }}
{{ template "_internal/pagination.html" . }}
{{ end }}`,
	`{{- $dateFormat := .Date.Format "2006-01-02" -}}
{{- $count := 0 -}}
{{- range $k, $v := .Site.Taxonomies.tags -}}
  {{- $count = add $count 1 -}}
  {{- if ge $count 5 }}{{ break }}{{ end -}}
  {{- if eq $k "skip" }}{{ continue }}{{ end -}}
  <li>{{ $k }} ({{ len $v }})</li>
{{- end -}}
{{ printf "%s %d %q %v" "a" 1 "b" (slice 1 2) }}
{{ $s := newScratch }}{{ $s.Set "k" 1 }}{{ $s.Get "k" }}
{{ .Site.Params.Social.Facebook.Enable }}
{{ (index .Site.Data.menus "main").items }}
{{ try (partial "x.html" .) }}
{{ with try (resources.GetRemote "https://x") }}{{ with .Err }}{{ errorf "%s" . }}{{ end }}{{ end }}`,
}

// ptxGen generates random template sources. A clean generator produces
// (mostly) valid templates; a dirty one mixes in syntax errors.
type ptxGen struct {
	r           *rand.Rand
	left, right string
	clean       bool
	vars        []string
	inRange     int
	inDefine    bool
	defs        int
}

func (g *ptxGen) pick(ss ...string) string { return ss[g.r.Intn(len(ss))] }

func (g *ptxGen) text() string {
	switch g.r.Intn(12) {
	case 0:
		return ""
	case 1:
		return g.pick(" ", "\n", "  \t", "\r\n", " \n ")
	case 2:
		return g.pick("é", "\xff", "{", "}", "-", "{ {", "} }", "{-", "-}", "a{", "}b")
	}
	return g.pick("a", "x y", "hello", "<p>", "</p>\n", " text ", "\n\t", "1", "é ", "k")
}

func (g *ptxGen) open() string {
	if g.r.Intn(4) == 0 {
		return g.left + g.pick("- ", "-\n", "-\t", "-\r")
	}
	return g.left + g.pick("", "", " ", "\n")
}

func (g *ptxGen) close() string {
	if g.r.Intn(4) == 0 {
		return g.pick(" -", "\n-", "\t-") + g.right
	}
	return g.pick("", "", " ", "\n") + g.right
}

func (g *ptxGen) varName() string {
	if len(g.vars) > 0 && g.r.Intn(3) != 0 {
		return g.vars[g.r.Intn(len(g.vars))]
	}
	if g.clean || g.r.Intn(4) != 0 {
		return "$"
	}
	return g.pick("$x", "$y", "$i", "$e", "$undef", "$1")
}

var ptxGoodNumbers = []string{
	"0", "-0", "17", "-17", "+17", "1_000", "0x1F", "0X1f", "0x_1F", "0o17", "0b101", "017", "1.5", ".5", "5.",
	"-.5", "1e3", "1E-3", "1e+3", "1_0.5_0e1_0", "0x1p4", "0x1.8p1", "0X1P-2", "1i", "2.5i", "0i", "1+2i",
	"-1-2i", "1-0i", "0x1p2+0x1p1i", "9223372036854775807", "-9223372036854775808", "18446744073709551615",
	"1e19", "1e20", "4.9e-324", "0x1p-1074", "0.1", "'a'", "'\\n'", "'\\x41'", "'é'", "'\\U0001F600'", "'\\''",
}

func (g *ptxGen) operand(depth int) string {
	switch g.r.Intn(16) {
	case 0:
		if g.clean {
			return ptxGoodNumbers[g.r.Intn(len(ptxGoodNumbers))]
		}
		return ptxNumbers[g.r.Intn(len(ptxNumbers))]
	case 1:
		return g.pick(`"a"`, `"x\"y"`, `""`, `"\n"`, "`raw`", "`multi\nline`", `"\xff"`, `"é"`, `"%d"`, `"\u00e9"`)
	case 2:
		return g.pick(".", ".X", ".X.Y", ".Params.x", ".A.B.C", ".é")
	case 3:
		return g.varName() + g.pick("", "", ".F", ".F.G")
	case 4:
		return g.pick("true", "false", "nil")
	case 5:
		if depth < 3 {
			return "(" + g.pipeline(depth+1) + ")" + g.pick("", "", ".X", ".X.Y")
		}
	case 6:
		if g.clean {
			return g.pick("printf", "len", "index", "print", "and", "or", "not", "eq", "html", "js", "slice", "call",
				"urlquery", "ne", "lt", "println")
		}
		return g.pick("printf", "len", "index", "print", "and", "or", "not", "eq", "html", "js", "slice", "call",
			"urlquery", "partial", "dict", "undefinedfn", "break", "continue", "T")
	case 7:
		return g.pick("'a'", "'\\n'", "'é'")
	}
	return g.pick(".X", "1", `"s"`, "$", ".", "printf", "2.5")
}

func (g *ptxGen) command(depth int, stage int) string {
	n := 1 + g.r.Intn(3)
	var parts []string
	for i := 0; i < n; i++ {
		if i == 0 && stage > 0 && (g.clean || g.r.Intn(8) != 0) {
			parts = append(parts, g.pick("printf \"%v\"", "print", "len", "not", ".X", "$.F", "html", "and 1",
				"or 0", "eq 1", "index", "urlquery", "(printf \"%d\")", "call .F"))
			continue
		}
		parts = append(parts, g.operand(depth))
	}
	return strings.Join(parts, g.pick(" ", " ", "  ", "\n", "\t"))
}

func (g *ptxGen) pipeline(depth int) string {
	var b strings.Builder
	var decl string
	if g.r.Intn(5) == 0 {
		decl = g.pick("$x", "$y", "$i", "$é", "$")
		if g.clean || g.r.Intn(3) != 0 {
			b.WriteString(decl + g.pick(" := ", ":=", "\n:=\n"))
		} else {
			b.WriteString(decl + g.pick(" = ", "="))
		}
	}
	n := 1 + g.r.Intn(3)
	for i := 0; i < n; i++ {
		if i > 0 {
			b.WriteString(g.pick(" | ", "|", " |\n"))
		}
		b.WriteString(g.command(depth, i))
	}
	if decl != "" {
		g.vars = append(g.vars, decl)
	}
	return b.String()
}

func (g *ptxGen) rangePipe(depth int) string {
	switch g.r.Intn(5) {
	case 0:
		p := g.pipeline(depth)
		g.vars = append(g.vars, "$i", "$e")
		if g.clean {
			return g.pick("$i, $e := ", "$i,$e:=") + p
		}
		return g.pick("$i, $e := ", "$i,$e:=", "$i, $e = ") + p
	case 1:
		p := g.pipeline(depth)
		g.vars = append(g.vars, "$e")
		return g.pick("$e := ", "$e := ", "$e = ") + p
	}
	return g.pipeline(depth)
}

func (g *ptxGen) item(depth int) string {
	switch g.r.Intn(14) {
	case 0, 1, 2:
		return g.text()
	case 3, 4, 5:
		return g.open() + g.pipeline(0) + g.close()
	case 6:
		if g.clean {
			c := g.pick("/* c */", "/**/", "/* multi\nline */")
			if g.r.Intn(3) == 0 {
				return g.left + g.pick("- ", "-\n") + c + g.pick(" -", "") + g.right
			}
			return g.left + c + g.right
		}
		c := g.pick("/* c */", "/**/", "/* multi\nline */", "/* x", "/* x */ y")
		if g.r.Intn(3) == 0 {
			return g.left + g.pick("- ", "-\n") + c + g.pick(" -", "") + g.right
		}
		return g.left + c + g.right
	case 7, 8:
		if depth < 4 {
			kw := g.pick("if", "with", "range", "if", "with")
			mark := len(g.vars)
			var head string
			if kw == "range" {
				head = g.open() + "range " + g.rangePipe(0) + g.close()
				g.inRange++
			} else {
				head = g.open() + kw + " " + g.pipeline(0) + g.close()
			}
			body := g.template(depth + 1)
			if kw == "range" {
				g.inRange--
			}
			var els string
			switch g.r.Intn(4) {
			case 0:
				els = g.open() + "else" + g.close() + g.template(depth+1)
			case 1:
				ek := kw
				if !g.clean && (kw == "range" || g.r.Intn(3) == 0) {
					ek = g.pick("if", "with")
				}
				if ek != "range" {
					m2 := len(g.vars)
					els = g.open() + "else " + ek + " " + g.pipeline(0) + g.close() + g.template(depth+1)
					if g.r.Intn(2) == 0 {
						els += g.open() + "else" + g.close() + g.template(depth+1)
					}
					g.vars = g.vars[:m2]
				}
			}
			g.vars = g.vars[:mark]
			return head + body + els + g.open() + "end" + g.close()
		}
	case 9:
		if g.inRange > 0 || (!g.clean && g.r.Intn(4) == 0) {
			return g.open() + g.pick("break", "continue") + g.close()
		}
	case 10:
		if depth < 3 {
			var name string
			if g.clean {
				g.defs++
				name = fmt.Sprintf("%q", fmt.Sprintf("t%d", g.defs))
			} else {
				name = g.pick(`"t"`, `"u"`, "`t`", `"a b"`, `""`, `"\xff"`)
			}
			saved, savedRange, savedDef := g.vars, g.inRange, g.inDefine
			g.vars, g.inRange, g.inDefine = nil, 0, true
			body := g.template(depth + 1)
			g.vars, g.inRange, g.inDefine = saved, savedRange, savedDef
			if (!g.clean || (depth == 0 && !g.inDefine)) && g.r.Intn(2) == 0 {
				return g.open() + "define " + name + g.close() + body + g.open() + "end" + g.close()
			}
			return g.open() + "block " + name + " " + g.pipeline(0) + g.close() + body + g.open() + "end" + g.close()
		}
	case 11:
		name := g.pick(`"t"`, `"u"`, "`t`", `"missing"`, `"t1"`)
		if g.r.Intn(2) == 0 {
			return g.open() + "template " + name + g.close()
		}
		return g.open() + "template " + name + " " + g.pipeline(0) + g.close()
	case 12:
		if !g.clean {
			return g.open() + g.pick("end", "else", "else if 1", "if", "range", "with", "define", "block", "template",
				"nil", ".X.", "..", "$x.", "(", ")", "|", "1 |", "| 1", ":= 1", "$x :=", "$1", "#", "\x01", "é") + g.close()
		}
	}
	return g.text()
}

func (g *ptxGen) template(depth int) string {
	var b strings.Builder
	n := g.r.Intn(6)
	if depth == 0 {
		n++
	}
	for i := 0; i < n; i++ {
		b.WriteString(g.item(depth))
	}
	return b.String()
}

// mutate applies a few random byte edits (to reach error paths).
func (g *ptxGen) mutate(s string) string {
	b := []byte(s)
	n := 1 + g.r.Intn(3)
	for i := 0; i < n; i++ {
		if len(b) == 0 {
			b = append(b, '{')
			continue
		}
		p := g.r.Intn(len(b))
		switch g.r.Intn(3) {
		case 0:
			b = append(b[:p], b[p+1:]...)
		case 1:
			c := "{}()|$.-\"`'/*\n 0aé:="[g.r.Intn(21)]
			b = append(b[:p], append([]byte{c}, b[p:]...)...)
		case 2:
			b = append(b[:p], b[p:]...)
			if p+1 < len(b) {
				b[p], b[p+1] = b[p+1], b[p]
			}
		}
	}
	return string(b)
}
