//go:build gotemplate_oracle

package main

// Mode escfuncs: fixtures for the html/template leaf functions ported in
// crates/gotemplate/src/html (escaper funcs, transitions, stripTags and
// helpers). Run tools/go-oracle/gotemplate/sync-fork.sh first, then
//
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate escfuncs crates/gotemplate/tests/fixtures/html
//
// It writes three gzip files (all strings are strconv.Quote'd):
//
//	escfuncs.txt.gz     I <args spec>, then one "O <func index> <result>" per
//	                    escaper func (escFuncNames order); result is
//	                    =<quoted output>, P<quoted panic> or N (output differs
//	                    between two fresh decodings: pointer addresses).
//	transitions.txt.gz  T <text>, C <start context>, R <text> <ctx> <step>...
//	                    (transitionFunc applied until the text is consumed).
//	leaf.txt.gz         L <func> <inputs...> <result>.

import (
	"bufio"
	"compress/gzip"
	"fmt"
	"math"
	"math/rand/v2"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"

	htmltemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/htmltemplate"
)

func init() { register("escfuncs", runEscFuncs) }

// escFuncNames is escape.go's funcMap in source order (the Rust
// ESC_FUNC_NAMES).
var escFuncNames = []string{
	"_html_template_attrescaper",
	"_html_template_commentescaper",
	"_html_template_cssescaper",
	"_html_template_cssvaluefilter",
	"_html_template_htmlnamefilter",
	"_html_template_htmlescaper",
	"_html_template_jsregexpescaper",
	"_html_template_jsstrescaper",
	"_html_template_jstmpllitescaper",
	"_html_template_jsvalescaper",
	"_html_template_nospaceescaper",
	"_html_template_rcdataescaper",
	"_html_template_srcsetescaper",
	"_html_template_urlescaper",
	"_html_template_urlfilter",
	"_html_template_urlnormalizer",
	"_eval_args_",
}

func runEscFuncs(args []string) error {
	if len(args) != 1 {
		return fmt.Errorf("usage: escfuncs <outdir>")
	}
	dir := args[0]
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	for _, f := range []struct {
		name  string
		write func(w *bufio.Writer) error
	}{
		{"escfuncs.txt.gz", writeEscFuncs},
		{"transitions.txt.gz", writeTransitions},
		{"leaf.txt.gz", writeLeaf},
	} {
		if err := writeGz(filepath.Join(dir, f.name), f.write); err != nil {
			return fmt.Errorf("%s: %w", f.name, err)
		}
	}
	return nil
}

func writeGz(path string, write func(w *bufio.Writer) error) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	zw := gzip.NewWriter(f)
	w := bufio.NewWriter(zw)
	if err := write(w); err != nil {
		return err
	}
	if err := w.Flush(); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	return f.Close()
}

func q(s string) string { return strconv.Quote(s) }

func sObj(kind, s string) string { return kind + ":" + hexs(s) }

// ---------------------------------------------------------------------------
// String corpus

// fuzzAtoms are the building blocks of the random strings: every table
// character, the tokens the transition functions look for, schemes, CSS
// keywords, and awkward runes and bytes.
var fuzzAtoms = []string{
	"<", ">", "&", "\"", "'", "`", "=", "+", " ", "\t", "\n", "\r", "\f", "\v", "\x00", "\x01", "\x1f", "\x7f",
	"\\", "/", "*", "-", "--", "<!--", "-->", "<script", "</script>", "<SCRIPT", "</ScRiPt", "<\u017fcript",
	"<script>", "</style>", "<style>", "<title>", "</title>", "<textarea>", "</textarea>", "<a ", "<a href=",
	"<img src=", " srcset=", " style=", " onclick=", " type=", "<b>", "</b>", "<!DOCTYPE html>", "<?xml",
	"(", ")", "[", "]", "{", "}", "${", "$", "^", "|", "?", "#", "#!", "%", "%2", "%41", "%zz", "%e2%80",
	":", ";", ",", "@", ".", "_", "~", "!", "javascript:", "JavaScript:", "http:", "https://", "mailto:",
	"data:", "a", "Z", "0", "9", "f", "F", "g", "x", "url(", "URL( ", "expression", "Expression(", "mozbinding",
	"-moz-binding", "\\41", "\\0", "\\a ", "\\\n", "\\\"", "1x", "200w", " 2x", "return", "typeof", " in ",
	"x++", "--x", "42.", "/re/", "//", "/*", "*/", "\u00a0", "\u2028", "\u2029", "\ufeff", "\ufdd0", "\ufdef",
	"\ufff0", "\uffff", "\ufffd", "\U0001D11E", "\U0010FFFF", "\x80", "\xc0", "\xff", "\xed\xa0\x80",
	"\xe2\x80", "é", "日本", "\u017f", "\u212a", "\u0130", "href", "onclick", "data-", "xmlns:", "src", "=x",
	"a b", "&amp;", "&lt;", "&#39;", "&quot;", "&#x27;", "\\u0022",
}

func fuzzString(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.IntN(8)
	for i := 0; i < n; i++ {
		b.WriteString(fuzzAtoms[r.IntN(len(fuzzAtoms))])
	}
	return b.String()
}

// realisticStrings are hand-picked inputs for every escaper and helper.
var realisticStrings = []string{
	"", " ", "  ", "a", "foo", "Hello, World!", "foo&amp;bar", "<b>bold</b>", "a < b and c > d & e",
	"Lay's Rock \"Q\" & 'A' <b>", "Home \"Q\" & 'A' <b>", "2020-12-30T07:40:22+00:00",
	`Hello <a href="www.example.com/">World</a>!`, "Foo <textarea>Bar</textarea> Baz", "Foo <!-- Bar --> Baz",
	"<", "foo < bar", `Foo<script type="text/javascript">alert(1337)</script>Bar`, `Foo<div title="1>2">Bar`,
	`I <3 Ponies!`, `<script>foo()</script>`, "<p>para</p><br><br />", "<img src=x onerror=alert(1)>",
	"<a href='x' title=\"y\" data-x=z>link</a>", "<div\n class=\"c\"\n>t</div>", "<!-- c --><p>x</p>",
	"<style>p{color:red}</style>after", "<title>T</title>", "<textarea><b></textarea>", "<a b=c d>e",
	"<a b='c>d'>e</a>", "text <script>if (a < b) { x = '</div>' }</script> tail", "<unclosed",
	"<a href=\"x", "<a href='", "<a href=", "<a b", "<ScRiPt>alert(1)</sCrIpT>x", "<script>a</script >b",
	"<script>a</script\tb>c", "<script>a</scriptx>b</script>c", "</p>stray", "<p>a<p>b", "&iexcl;Hi!",
	"http://example.com:80/foo/bar?q=foo%20&bar=x+y#frag", "https://seeksnack.com/images/favicon/mstile-70x70.png",
	"/path/to/page/", "relative/path?x=1", "?q=1", "#frag", "//cdn.example.com/x.js", "javascript:alert(1)",
	"JAVASCRIPT:alert(1)", " javascript:x", "java\nscript:x", "data:text/html,<b>", "mailto:a@b.c",
	"MailTo:x", "HTTP://X", "ftp://x", "/a:b", "a/b:c", "a:b/c", "tel:123", ":", "%", "%2", "%z", "%7c", "%7C",
	"/foo|bar/%5c\u1234", "a b c", "é", "日本語", "\u2028\u2029", "x=1&y=2", "http://x/?a=<b>&c='d'",
	"http://example.com/img.png", " /img.png 200w", "javascript:alert(1) 200w", "foo.png, bar.png",
	"javascript:alert(1), /foo.png", "/bogus#, javascript:alert(1)", "/a b.png 1x", "a.png 1x, b.png 2x",
	"a.png\t1x,\nb.png 2x", "a.png 1.5x", "a.png (1x)", ",", ",,", " , ", "a.png,", "x.png 100w, y.png 200w",
	"0", "0px", "-5px", "1.25in", "+.33em", "100%", "12.5%", ".foo", "#bar", "corner-radius",
	"-moz-corner-radius", "#000", "#48f", "#123456", "U+00-FF, U+980-9FF", "color: red", "<!--", "-->",
	"<![CDATA[", "]]>", "</style", "`", "\x00", "/* foo */", "//", "[href=~", "expression(alert(1337))",
	"-expression(alert(1337))", "expression", "Expression", "EXPRESSION", "-moz-binding",
	"-expr\x00ession(alert(1337))", `-expr\0ession(alert(1337))`, `-express\69on(alert(1337))`,
	`-express\69 on(alert(1337))`, `-exp\72 ession(alert(1337))`, `-exp\52 ession(alert(1337))`,
	`-exp\000052 ession(alert(1337))`, `-expre\0000073sion`, `@import url evil.css`, "Times New Roman",
	`  e\78preS\0Sio/**/n(alert(1337))`, `\A`, `\a`, `\0a`, `\00000a`, `\000000a`, `\1234 5`, `\1234\20 5`,
	`\1234\A 5`, "\\1234\t5", "\\1234\n5", "\\1234\r\n5", `\12345`, `\\`, `\\ `, `\"`, `\'`, `\.`, `\. .`,
	`\110000`, `\d800`, `\FFFFFF`, `\fffffff`, "\\\xff", "\\\xe6\x97\xa5", `\`, `foo\`,
	`The \3c i\3equick\3c/i\3e,\d\A\3cspan style=\27 color:brown\27\3e brown\3c/span\3e  fox jumps\2028over the \3c canine class=\22lazy\22 \3e dog\3c/canine\3e`,
	"The <i>quick</i>,\r\n<span style='color:brown'>brown</span> fox jumps\u2028over the <canine class=\"lazy\">dog</canine>",
	"\\", "\\n", "foo\r\nbar", "&amp;", "</script>", "+ADw-script+AD4-alert(1)+ADw-/script+AD4-",
	"foo\xA0bar", "foo\xed\xa0\x80bar", "*", "+", "?", "[](){}", "$foo|x.y", "x^y", "`${x}`", "a${b}c",
	"href", "HREF", "onclick", "onClick", "data-href", "data-foo", "data-", "xmlns", "xmlns:svg", "svg:href",
	"xlink:href", "g:tweetUrl", "srcset", "src", "style", "content", "type", "value", "rel", "a-b", "a_b",
	"a1", "A1", "checked", "\u017f", "k\u212a", "\u0130d", "title", "alt", "class", "id", "name",
	"mysrc", "myuri", "myurl", "on", "o", "data-on", "data-src", "d:e:f",
	"text/javascript", "application/javascript;version=1.8", "application/javascript;version=1.8;foo=bar",
	"application/javascript/version=1.8", "application/json", "application/ld+json", "module", "MODULE",
	" text/javascript ", "text/javascript1.5", "text/javascript1.6", "text/x-javascript", "x-tmpl-mustache",
	"text/template", "Text/JavaScript", "text/ecmascript;", "\ttext/jscript\n", "text/livescript",
}

// runeStrings are single runes (and invalid sequences) of interest.
var runeStrings = []string{
	"\u0080", "\u009f", "\u00a0", "\u00ad", "\u0100", "\u017f", "\u0130", "\u0131", "\u212a", "\u2028",
	"\u2029", "\u202f", "\u205f", "\u3000", "\ufdcf", "\ufdd0", "\ufdd1", "\ufdef", "\ufdf0", "\ufeff",
	"\ufff0", "\ufff9", "\ufffd", "\ufffe", "\uffff", "\U00010000", "\U0001D11E", "\U0010FFFF", "\ue000",
	"\ud7ff", "\xc0\x80", "\xed\xa0\x80", "\xed\xbf\xbf", "\xf4\x90\x80\x80", "\xe2\x80", "\xf0\x9d\x84",
	"\xc2", "\xe2\x80\xa8x", "x\xe2\x80\xa9",
}

// goTestInput is the input of TestHTMLNospaceEscaper and friends.
const goTestInput = "\x00\x01\x02\x03\x04\x05\x06\x07\x08\t\n\x0b\x0c\r\x0e\x0f" +
	"\x10\x11\x12\x13\x14\x15\x16\x17\x18\x19\x1a\x1b\x1c\x1d\x1e\x1f" +
	` !"#$%&'()*+,-./` +
	`0123456789:;<=>?` +
	`@ABCDEFGHIJKLMNO` +
	`PQRSTUVWXYZ[\]^_` +
	"`abcdefghijklmno" +
	"pqrstuvwxyz{|}~\x7f" +
	"\u00A0\u0100\u2028\u2029\ufeff\ufdec\U0001D11E" +
	"erroneous\x960"

// stringCorpus returns the strings every escaper is run on.
func stringCorpus() []string {
	var ss []string
	for b := 0; b < 256; b++ {
		ss = append(ss, string([]byte{byte(b)}))
	}
	ss = append(ss, runeStrings...)
	ss = append(ss, goTestInput)
	for _, r := range goTestInput {
		ss = append(ss, string(r))
	}
	ss = append(ss, realisticStrings...)
	// Every table byte followed by a hex digit, a CSS space and a letter
	// (cssEscaper's trailing-space rule), and doubled.
	for _, c := range "\x00\t\n\f\r\"&'()+/:;<>\\{}" {
		for _, next := range []string{"", "a", "F", "0", " ", "\t", "\n", "g", "\\"} {
			ss = append(ss, string(c)+next)
		}
	}
	r := rand.New(rand.NewPCG(1, 2))
	for i := 0; i < 1200; i++ {
		ss = append(ss, fuzzString(r))
	}
	return ss
}

// valueArgs returns argument lists of non-string operands and of several
// operands.
func valueArgs() []string {
	var as []string
	one := func(specs ...string) {
		for _, s := range specs {
			as = append(as, sArgs(s))
		}
	}
	one("nil", "bool:1", "bool:0")
	for _, i := range []int64{0, 1, -1, 42, -42, 1 << 53, 1<<53 + 1, math.MaxInt64, math.MinInt64} {
		one(sInt("int", i), sInt("int64", i))
	}
	one(sInt("int8", -5), sInt("int16", 300), sInt("int32", 'x'), sUint("uint", 42), sUint("uint8", 200),
		sUint("uint16", 65535), sUint("uint32", 1<<31), sUint("uint64", math.MaxUint64), sUint("uintptr", 7))
	for _, f := range []float64{0, math.Copysign(0, -1), 1, -1, 0.5, -0.5, 4.5, 1.0 / 3, 1e20, 1e21, 1e-6,
		1e-7, 123456789, 2.675, math.MaxFloat64, math.SmallestNonzeroFloat64, math.NaN(), math.Inf(1), math.Inf(-1)} {
		one(sF64(f))
	}
	for _, f := range []float32{0, 1, -1, 0.5, 1.0 / 256, 1.1, 16777217, 3.4028235e38, float32(math.Inf(-1))} {
		one(sF32(f))
	}
	for _, k := range []string{"html", "htmlattr", "css", "js", "jsstr", "url", "srcset"} {
		for _, s := range []string{"", "<b>x & y</b>", "a\"b'c", "javascript:x", "a, b 1x", "\u2028", "</script>",
			"x&amp;y", "<a href=\"q\">t</a> <!-- c --> z", "expression(x)", "Z"} {
			one(sSafe(k, s))
		}
	}
	one(sBytes(""), sBytes("abc"), sBytes("<\xff>"), sTnil("[]uint8"))
	for _, t := range []string{"[]string", "[]interface {}", "map[string]interface {}", "maps.Params", "*int",
		"*main.Plain", "error", "fmt.Stringer", "main.Iface", "func()"} {
		one(sTnil(t))
	}
	one(sList("[]interface {}"),
		sList("[]interface {}", sInt("int", 42), sStr("foo"), "nil"),
		sList("[]interface {}", sInt("int", 1), sStr("two"), sF64(3.5), "bool:1", "nil"),
		sList("[]string", sStr("<!--"), sStr("</script>"), sStr("-->")),
		sList("[]string", sStr("a"), sStr("b c"), sStr(""), sStr("日本"), sStr("\xff")),
		sList("[]string"),
		sList("[]int", sInt("int", 1), sInt("int", -2)),
		sList("[]float64", sF64(1), sF64(2.5), sF64(1e21)),
		sList("[]bool", "bool:1", "bool:0"),
		sList("[]template.HTML", sSafe("html", "<p>")),
		sList("[]interface {}", sList("[]interface {}", sInt("int", 1)), sMap("map[string]interface {}", "k", sStr("v"))),
		sList("[]interface {}", sF64(math.NaN())),
		sList("[]interface {}", "time:1790510400;0;utc", sSafe("js", "x")),
		sMap("map[string]interface {}"),
		sMap("map[string]interface {}", "b", sInt("int", 2), "a", sInt("int", 1), "c", sList("[]int", sInt("int", 1), sInt("int", 2))),
		sMap("map[string]interface {}", "t", sStr("Home \"Q\" & 'A' <b>"), "n", "nil", "e", sStr("\u2028")),
		sMap("map[string]string", "x", sStr("1"), "y", sStr("<y>")),
		sMap("maps.Params", "title", sStr("Hi"), "draft", "bool:0", "weight", sInt("int", 10)),
		"time:1599407186;955000000;utc", "time:0;0;utc", "time:-62135596800;0;nil",
		"time:1790510400;123456789;fixed=MST=-25200", "time:1790510400;0;zone=Asia/Bangkok",
		"time:253402300800;0;utc", "time:-62135596801;0;utc",
		sObj("obj_str", "stringer <b>"), sObj("obj_str", ""), sObj("obj_err", "an error </script>"),
		sObj("obj_both", "x"), sObj("obj_sv", "val <i>"), sObj("obj_gs", "gs"),
		"obj_plain("+sInt("int", 1)+","+sStr("two")+")",
		"obj_plain(nil,"+sList("[]string", sStr("a"))+")",
		"obj_pplain("+sInt("int", 1)+","+sStr("two <b>")+")",
		"obj_pplain("+sObj("obj_str", "s")+","+sObj("obj_both", "b")+")",
		"obj_pplain(nil,nil)",
		sObjNM("k", sStr("v"), "n", "nil"), sObjNS(sInt("int", 1), sStr("s"), "nil"),
		"pages:0", "pages:3", "taxlist:2", sTnil("page.Pages"),
		sObj("obj_jm", `{"a":"<b>","b":[1, 2]}`), sObj("obj_jm", ` "x\u2028y" `), sObj("obj_jm", "42"),
		sObj("obj_jm", "true"), sObj("obj_jm", "\"</script>\""), sObj("obj_jm", "{bad"), sObj("obj_jm", ""),
		sObj("obj_jm", "\"\xe2\x80\xa8\""), sObj("obj_jm", "\"\xff\""),
		sObj("obj_pjm", "[1,\"a\"]"), sObj("obj_pjm", "nul"),
		sObj("obj_jme", "a */ b <script c </script d <!-- e <sCrIpT f </sCrIpT"),
		sObj("obj_jme", "<\u017fcript </\u017fCRIPT <scrip <!- */*/ **/"), sObj("obj_jme", ""),
		sObj("obj_tm", "text <b>"), sObj("obj_tm", ""),
		sObj("obj_hs", "<b>hs</b> & 'q'"), sObj("obj_hs", ""),
		"obj_pv("+sSafe("html", "<b>pv</b> & x")+")", "obj_pv("+sStr("plain <b>")+")",
		"obj_pv("+sInt("int", 5)+")", "obj_pv("+sSafe("js", "a+b")+")", "obj_pv("+sSafe("url", "javascript:y")+")",
		"obj_pv(nil)", "obj_pv(obj_pv("+sStr("inner")+"))",
		"obj_ppv("+sSafe("html", "<i>ppv</i>")+")", "obj_ppv("+sInt("int", 7)+")",
		sObj("jnum", "123"), sObj("jnum", "1.5e3"), sObj("jnum", "x<y"),
	)
	// Several arguments (stringify's nil skipping, fmt.Sprint spacing,
	// jsValEscaper's Sprint path).
	as = append(as,
		sArgs(), sArgs("nil"), sArgs("nil", "nil"), sArgs(sStr("a"), sStr("b")), sArgs(sStr("a"), "nil", sStr("b")),
		sArgs(sInt("int", 1), sInt("int", 2)), sArgs(sStr("a"), sInt("int", 1), sInt("int", 2), sStr("b")),
		sArgs("nil", sInt("int", 3)), sArgs(sSafe("html", "<b>"), sStr("<i>")), sArgs(sSafe("js", "x"), sSafe("js", "y")),
		sArgs(sStr("<a href='x'>"), sF64(1.5), "bool:1"), sArgs(sObj("obj_str", "s"), sObj("obj_err", "e")),
		sArgs("obj_pplain("+sInt("int", 1)+",nil)", sInt("int", 2)), sArgs(sTnil("*int"), sStr("x")),
		sArgs(sTnil("error"), sStr("x")), sArgs(sList("[]string", sStr("a")), sStr("b")),
		sArgs(sStr(""), sStr("")), sArgs(sStr("\u2028"), sStr("x")), sArgs(sStr("javascript:"), sStr("alert(1)")),
		sArgs(sStr("a b"), sStr(", c")), sArgs(sObj("obj_jm", "1"), sObj("obj_jm", "2")),
		sArgs("obj_pv("+sSafe("html", "<b>")+")", sStr("x")), sArgs(sStr("\xff"), sInt("int", 0)),
		sArgs("time:0;0;utc", sStr("t")), sArgs(sBytes("hi"), sStr("x")),
	)
	return as
}

func escArgsCorpus() []string {
	var as []string
	for _, s := range stringCorpus() {
		as = append(as, sArgs(sStr(s)))
	}
	return append(as, valueArgs()...)
}

// ---------------------------------------------------------------------------
// escfuncs.txt.gz

func decodeArgs(spec string) ([]any, error) {
	n, err := parseSpec(spec)
	if err != nil {
		return nil, err
	}
	if n.name != "args" {
		return nil, fmt.Errorf("not an args node: %q", spec)
	}
	vs := make([]any, len(n.children))
	for i, c := range n.children {
		if vs[i], err = decodeNode(c); err != nil {
			return nil, err
		}
	}
	return vs, nil
}

func callEsc(f func(...any) string, spec string) (out, panicMsg string, err error) {
	vs, err := decodeArgs(spec)
	if err != nil {
		return "", "", err
	}
	defer func() {
		if r := recover(); r != nil {
			panicMsg = fmt.Sprint(r)
		}
	}()
	return f(vs...), "", nil
}

func writeEscFuncs(w *bufio.Writer) error {
	funcs := htmltemplate.EscFuncs()
	if len(funcs) != len(escFuncNames) {
		return fmt.Errorf("funcMap has %d entries, want %d", len(funcs), len(escFuncNames))
	}
	fs := make([]func(...any) string, len(escFuncNames))
	for i, name := range escFuncNames {
		f, ok := funcs[name].(func(...any) string)
		if !ok {
			return fmt.Errorf("funcMap[%q] is %T", name, funcs[name])
		}
		fs[i] = f
	}
	fmt.Fprintf(w, "# gotemplate escfuncs fixture: I <args spec> / O <func index> =<quoted>|P<quoted panic>|N\n")
	for _, spec := range escArgsCorpus() {
		fmt.Fprintf(w, "I\t%s\n", spec)
		for i, f := range fs {
			out1, p1, err := callEsc(f, spec)
			if err != nil {
				return err
			}
			out2, p2, _ := callEsc(f, spec)
			switch {
			case p1 != "":
				fmt.Fprintf(w, "O\t%d\tP%s\n", i, q(p1))
			case out1 != out2 || p2 != "":
				fmt.Fprintf(w, "O\t%d\tN\n", i)
			default:
				fmt.Fprintf(w, "O\t%d\t=%s\n", i, q(out1))
			}
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// transitions.txt.gz

type startCtx struct {
	st, dl, up, jc, at, el uint8
	braces                 []int // nil: nil slice
}

func (c startCtx) String() string {
	b := "-"
	if c.braces != nil {
		b = intsString(c.braces)
	}
	return fmt.Sprintf("%d,%d,%d,%d,%d,%d\t%s", c.st, c.dl, c.up, c.jc, c.at, c.el, b)
}

func intsString(xs []int) string {
	parts := make([]string, len(xs))
	for i, x := range xs {
		parts[i] = strconv.Itoa(x)
	}
	return "[" + strings.Join(parts, ",") + "]"
}

// Go state and element numbering (context.go).
const (
	stText        = 0
	stTag         = 1
	stAttrName    = 2
	stAfterName   = 3
	stBeforeValue = 4
	stRCDATA      = 6
	stAttr        = 7
	stURL         = 8
	stSrcset      = 9
	stJS          = 10
	stJSTmplLit   = 13
	stCSSURL      = 24
	stError       = 27
	elScript      = 1
	elStyle       = 2
	elTextarea    = 3
	elTitle       = 4
)

func startContexts() []startCtx {
	var cs []startCtx
	for st := uint8(0); st <= stError; st++ {
		cs = append(cs, startCtx{st: st})
	}
	for _, el := range []uint8{elScript, elStyle, elTextarea, elTitle} {
		for _, st := range []uint8{stText, stTag, stAttrName, stAfterName, stBeforeValue, stRCDATA, stJS, 11, 12,
			stJSTmplLit, 14, 15, 16, 17, 18, 19, 20, 25, 26} {
			cs = append(cs, startCtx{st: st, el: el})
		}
	}
	for at := uint8(0); at <= 5; at++ {
		cs = append(cs, startCtx{st: stBeforeValue, at: at}, startCtx{st: stBeforeValue, at: at, el: elScript})
	}
	for jc := uint8(1); jc <= 2; jc++ {
		cs = append(cs, startCtx{st: stJS, jc: jc})
	}
	for up := uint8(1); up <= 3; up++ {
		cs = append(cs, startCtx{st: stURL, up: up}, startCtx{st: stSrcset, up: up}, startCtx{st: stCSSURL, up: up})
	}
	cs = append(cs,
		startCtx{st: stJS, braces: []int{0}},
		startCtx{st: stJS, braces: []int{2}},
		startCtx{st: stJS, braces: []int{0, 1}, jc: 1},
		startCtx{st: stJSTmplLit, braces: []int{0}},
		startCtx{st: stJS, braces: []int{}},
		startCtx{st: stJS, braces: []int{0}, el: elScript},
		startCtx{st: stText, braces: []int{3}},
		startCtx{st: stAttr, dl: 1, at: 0},
	)
	return cs
}

func transitionTexts() []string {
	ts := []string{
		"", "x", "<", "<a", "<a>", "</a>", "<a href=\"x\">y</a>", "<a href='x?y#z'>", "<a href=x>", "<a href= x >",
		"<a b c=d e='f' g=\"h\">", "<a\tb\n=\fc\r>", "<a b=>", "<a =b>", "<a \"b\">", "<a b\"c>", "<a b<c>",
		"<!-- c -->x", "<!-- c", "<!-->", "<!--->", "<script>var x = 1;</script>", "<script type=\"text/template\">",
		"<script type='module'>", "<style>p { color: red }</style>", "<title>T</title>", "<textarea>x</textarea>",
		"<SCRIPT>x</SCRIPT>", "<x-y>", "<x:y>", "<x--y>", "<x->", "<1a>", "</>", "</ a>", "< a>", "<a/b>",
		"<img srcset=\"a.png 1x, b.png 2x\">", "<p style=\"color: red; background: url('x.png')\">",
		"<p onclick=\"a = b / c; d = /re/g;\">", "<p data-href=x>", "<svg xmlns:xlink=\"http://x\">",
		"var x = 1 / 2; y = /re[/]x/ + 1;", "a = b\n/re/", "return /x/", "x++ / 2", "x = `a${b}c`;",
		"x = `a${ {a:1} }c`", "`${`${x}`}`", "}", "{", "{}", "}}", "`", "${", "a${b", "\\`", "`\\", "x = `a\\",
		"\"a\\\"b\"", "'a\\'b'", "\"</script>\"", "/a</script>b/", "/a[/]b/", "/a[b", "\"abc", "'abc", "\"a\\",
		"// comment\nx", "/* c */ x", "/* c", "<!-- x\ny", "--> x\ny", "#! x\ny", "#x", "a <!- b", "a -- > b",
		"x\u2028y", "// c\u2029z", "a / b", "a /= b", "(a) / b", "x / /", "if (a) /re/.test(b)", "3./2",
		"url(x)", "url( 'x' )", "url(\"x\")", "URL(x)", "myurl(x)", "a url (x)", "url(", "background:url(a\\)b)",
		"'a\\'b'", "\"a\\\"b\"", "\"a\\", "'a", "a\\41 b\"", "x /* c */ y", "x // c\ny", "x // c\fy", "a/b",
		"\"a?b\"", "'#'", "a) b", "a\\\nb", "a\tb)", "a b", "a?b", "  ", " x", "#", "?", "x?y", "javascript:x",
		"p { background: url(/a?b#c) }", "</style>", "</title >", "</textarea\t", "</script/", "</scriptx>",
		"a</script>b", "a</STYLE>b", "</titlex></title>", "a < b", "<<a>", "<a<b>", "<a\xffb>", "<a b\xff=c>",
		"<a B=\"C\">", "<a HREF=x>", "<a SrC=x>", "<a onClick=x>", "<a on=x>", "<a style=x>", "<a type=x>",
		"&amp;&lt;", "\xff\xfe", "日本<b>語</b>", "\x00<a\x00>",
		// jsBraceDepth pops then pushes: append writes into the shared array.
		"}${", "}${{", "}x${ { }", "} ${ `${ } `", "}}${${", "{}}${", "`${ } ${ { } }`", "}`${a}`${",
	}
	r := rand.New(rand.NewPCG(3, 4))
	for i := 0; i < 250; i++ {
		ts = append(ts, fuzzString(r))
	}
	return ts
}

func stepString(res htmltemplate.OracleCtx, n int, initial []int) string {
	b := intsString(res.Braces)
	init := intsString(initial)
	errq := ""
	if res.Err != "" {
		errq = q(res.Err)
	}
	return fmt.Sprintf("%d|%d,%d,%d,%d,%d,%d|%s|%d|%s|%s", n, res.State, res.Delim, res.URLPart, res.JSCtx,
		res.Attr, res.Element, b, cap(res.Braces), init, errq)
}

func sameOracleCtx(a, b htmltemplate.OracleCtx) bool {
	if a.State != b.State || a.Delim != b.Delim || a.URLPart != b.URLPart || a.JSCtx != b.JSCtx ||
		a.Attr != b.Attr || a.Element != b.Element || a.Err != b.Err || len(a.Braces) != len(b.Braces) {
		return false
	}
	for i := range a.Braces {
		if a.Braces[i] != b.Braces[i] {
			return false
		}
	}
	return true
}

func step(c htmltemplate.OracleCtx, s []byte) (res htmltemplate.OracleCtx, n int, panicMsg string) {
	defer func() {
		if r := recover(); r != nil {
			panicMsg = fmt.Sprint(r)
		}
	}()
	res, n = htmltemplate.TransitionStepExported(c, s)
	return res, n, ""
}

func writeTransitions(w *bufio.Writer) error {
	texts := transitionTexts()
	ctxs := startContexts()
	fmt.Fprintf(w, "# gotemplate transitions fixture: T <text> / C <ctx> <braces> / R <ti> <ci> <step>...\n")
	for _, t := range texts {
		fmt.Fprintf(w, "T\t%s\n", q(t))
	}
	for _, c := range ctxs {
		fmt.Fprintf(w, "C\t%s\n", c)
	}
	for ti, t := range texts {
		for ci, sc := range ctxs {
			var braces []int
			if sc.braces != nil {
				braces = make([]int, len(sc.braces))
				copy(braces, sc.braces)
			}
			c := htmltemplate.OracleCtx{State: sc.st, Delim: sc.dl, URLPart: sc.up, JSCtx: sc.jc, Attr: sc.at,
				Element: sc.el, Braces: braces}
			s := []byte(t)
			var steps []string
			for len(s) > 0 {
				if len(steps) >= 2000 {
					steps = append(steps, "LIMIT")
					break
				}
				res, n, p := step(c, s)
				if p != "" {
					steps = append(steps, "PANIC"+q(p))
					break
				}
				steps = append(steps, stepString(res, n, braces))
				if n == 0 && sameOracleCtx(c, res) {
					steps = append(steps, "STALL")
					break
				}
				c, s = res, s[n:]
			}
			fmt.Fprintf(w, "R\t%d\t%d", ti, ci)
			for _, st := range steps {
				fmt.Fprintf(w, "\t%s", st)
			}
			fmt.Fprintln(w)
		}
	}
	return nil
}

// ---------------------------------------------------------------------------
// leaf.txt.gz

func b01(b bool) int {
	if b {
		return 1
	}
	return 0
}

func writeLeaf(w *bufio.Writer) error {
	fmt.Fprintf(w, "# gotemplate leaf fixture: L <func> <inputs...> <result>\n")
	strs := stringCorpus()
	strs = append(strs, transitionTexts()...)

	// stripTags (Hugo's exported StripTags).
	var html []string
	html = append(html, strs...)
	r := rand.New(rand.NewPCG(5, 6))
	htmlAtoms := []string{"<a href=\"x\">", "</a>", "<b>", "</b>", "<script>", "</script>", "<style>", "</style>",
		"<title>", "</title>", "<textarea>", "</textarea>", "<!--", "-->", "text", " ", "<", ">", "\"", "'", "=",
		"<p ", "class=c", "title='a>b'", "x=\"y\"", "<br/>", "<img src=x>", "a<3", "&amp;", "\n", "日本",
		"<script type=\"text/template\">", "var s = '</script>';", "<div", " data-x=1 ", ">", "<!DOCTYPE html>",
		"<a b=c", "<svg:path d='M0'>", "</SCRIPT >", "\xff", "<1>", "</ >"}
	for i := 0; i < 1500; i++ {
		var b strings.Builder
		n := 1 + r.IntN(12)
		for j := 0; j < n; j++ {
			b.WriteString(htmlAtoms[r.IntN(len(htmlAtoms))])
		}
		html = append(html, b.String())
	}
	for _, s := range html {
		fmt.Fprintf(w, "L\tstriptags\t%s\t%s\n", q(s), q(htmltemplate.StripTagsExported(s)))
	}

	// nextJSCtx.
	jsInputs := []string{";", "}", ")", "]", "(", "[", "{", "=", "+=", "*=", "*", "!", "+", "-", "--", "++",
		"x--", "x---", "return", "return ", "return\t", "return\n", "return\u2028", "x", "x ", "x\t", "x\n",
		"x\u2028", "preturn", "0", "0.", "=\u00A0", "   ", "", ".", "a.", "1 .", "typeof", "instanceof", "in",
		"void", "do", "else", "finally", "throw", "try", "case", "delete", "break", "continue", "x.return",
		"$return", "_in", "\xffin", "in\ufeff", "x\u3000", "x\u00a0\u1680", "?", ":", "%", "^", "|", "&", "~",
		",", "<", ">", "\"", "'", "/", "#", "@", "\\", "\u2029", "a\xe2\x80", "日本", "+++", "++++", "-+-"}
	jsInputs = append(jsInputs, strs...)
	for _, s := range jsInputs {
		for p := uint8(0); p <= 2; p++ {
			fmt.Fprintf(w, "L\tnextjs\t%s\t%d\t%d\n", q(s), p, htmltemplate.NextJSCtxExported([]byte(s), p))
		}
	}

	// attrType, isJSType, isSafeURL, decodeCSS, special script tags.
	for _, s := range strs {
		fmt.Fprintf(w, "L\tattrtype\t%s\t%d\n", q(s), htmltemplate.AttrTypeExported(s))
		fmt.Fprintf(w, "L\tisjstype\t%s\t%d\n", q(s), b01(htmltemplate.IsJSTypeExported(s)))
		fmt.Fprintf(w, "L\tissafeurl\t%s\t%d\n", q(s), b01(htmltemplate.IsSafeURLExported(s)))
		fmt.Fprintf(w, "L\tdecodecss\t%s\t%s\n", q(s), q(string(htmltemplate.DecodeCSSExported([]byte(s)))))
		fmt.Fprintf(w, "L\tspecialtag\t%s\t%d\t%s\n", q(s), b01(htmltemplate.ContainsSpecialScriptTagExported([]byte(s))),
			q(string(htmltemplate.EscapeSpecialScriptTagsExported([]byte(s)))))
		fmt.Fprintf(w, "L\tscripttagre\t%s\t%s\n", q(s), q(string(htmltemplate.ScriptTagReplaceExported([]byte(s)))))
	}
	// Case folding of the script-tag regexps: every rune whose simple-fold
	// orbit contains an ASCII byte, Latin-1/Latin Extended runes, and a few
	// others.
	var foldRunes []rune
	for r := rune(0); r <= unicode.MaxRune; r++ {
		if r < 0x250 {
			foldRunes = append(foldRunes, r)
			continue
		}
		for f := unicode.SimpleFold(r); f != r; f = unicode.SimpleFold(f) {
			if f < utf8.RuneSelf {
				foldRunes = append(foldRunes, r)
				break
			}
		}
	}
	foldRunes = append(foldRunes, 0x2028, 0x2029, 0xfffd, 0xffff, 0x10400, 0x10ffff)
	for _, fr := range foldRunes {
		if !utf8.ValidRune(fr) {
			continue
		}
		for _, pat := range []string{"<%cscript", "<s%cript", "<scrip%c", "</%cscript", "</s%cript", "<!-%c", "<%c!--"} {
			s := fmt.Sprintf(pat, fr)
			fmt.Fprintf(w, "L\tspecialtag\t%s\t%d\t%s\n", q(s), b01(htmltemplate.ContainsSpecialScriptTagExported([]byte(s))),
				q(string(htmltemplate.EscapeSpecialScriptTagsExported([]byte(s)))))
			fmt.Fprintf(w, "L\tscripttagre\t%s\t%s\n", q(s), q(string(htmltemplate.ScriptTagReplaceExported([]byte(s)))))
		}
	}

	// indexTagEnd.
	for _, s := range strs {
		for _, tag := range []string{"script", "style", "textarea", "title", "tag"} {
			fmt.Fprintf(w, "L\tindextagend\t%s\t%s\t%d\n", q(s), q(tag), htmltemplate.IndexTagEndExported([]byte(s), []byte(tag)))
		}
	}
	for _, s := range []string{"", "hello </textarea> hello", "hello </TEXTarea> hello", "hello </textAREA>",
		"hello </textarea", "hello tag </textarea", "hello </tag> </other> </textarea> <other>", "</textarea> <other>",
		"<div> </div> </TEXTAREA>", "<div> </div> </TEXTAREA\t>", "<div> </div> </TEXTAREA >", "<div> </div> </TEXTAREAfoo",
		"</TEXTAREAfoo </textarea>", "<</script >", "</script>", "</\u017fcript>", "</scrip\u0130>", "</SCRIPT/", "</script\f",
		"</script\r", "</script\v", "</</script>", "</script></script>"} {
		for _, tag := range []string{"script", "textarea", "textareax", "tag"} {
			fmt.Fprintf(w, "L\tindextagend\t%s\t%s\t%d\n", q(s), q(tag), htmltemplate.IndexTagEndExported([]byte(s), []byte(tag)))
		}
	}

	// endsWithCSSKeyword.
	for _, s := range append([]string{"", "url", "URL", "Url", "important", "image-url", "imageurl", "image url",
		"x\u00e9url", "\xffurl", "\u212aurl", "ur\u017f", "a:url", "_url", "9url", "\u2028url", "uRl"}, strs...) {
		for _, kw := range []string{"url", "important"} {
			fmt.Fprintf(w, "L\tendswithcss\t%s\t%s\t%d\n", q(s), q(kw), b01(htmltemplate.EndsWithCSSKeywordExported([]byte(s), kw)))
		}
	}

	// context String and mangle: every state with the zero and the maximal
	// field values and 16 random combinations.
	cr := rand.New(rand.NewPCG(7, 8))
	for st := uint8(0); st <= 28; st++ {
		combos := [][5]uint8{{0, 0, 0, 0, 0}, {3, 3, 2, 5, 4}}
		for i := 0; i < 16; i++ {
			combos = append(combos, [5]uint8{uint8(cr.IntN(4)), uint8(cr.IntN(4)), uint8(cr.IntN(3)), uint8(cr.IntN(6)), uint8(cr.IntN(5))})
		}
		for _, f := range combos {
			o := htmltemplate.OracleCtx{State: st, Delim: f[0], URLPart: f[1], JSCtx: f[2], Attr: f[3], Element: f[4]}
			fmt.Fprintf(w, "L\tctx\t%d,%d,%d,%d,%d,%d\t%s\t%s\n", st, f[0], f[1], f[2], f[3], f[4],
				q(htmltemplate.ContextStringExported(o)), q(htmltemplate.MangleExported(o, "name")))
		}
	}

	// Sorted, so the file is stable even if map-based helpers change order.
	keys := make([]string, 0, len(htmltemplate.EscFuncs()))
	for k := range htmltemplate.EscFuncs() {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	fmt.Fprintf(w, "L\tfuncmap\t%s\n", strings.Join(keys, ","))
	return nil
}
