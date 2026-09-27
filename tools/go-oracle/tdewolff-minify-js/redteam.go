package main

import (
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"strings"
)

// Red-team generators (`gen KIND OUT N SEED`): large seeded runs of `min`
// or `html` records with digests, checked by the Rust port's
// `examples/check.rs`. The kinds target what the fixture fuzzers reach only
// sparsely:
//
//	lits    string, template, number and regexp literals (escapes, quote
//	        choice, `</script>`, numeric bases, separators, BigInt, precision,
//	        huge/tiny exponents) in the contexts that rewrite them
//	prog    generated programs through random configurations (all versions,
//	        precisions, keep/alpha/inline)
//	logic   expression trees over the rewritten operators (redteam_logic.go)
//	num     numeric literals only, at precisions 0-21
//	soup    random token sequences (mostly errors: messages and positions)
//	corpus  windows of the module-cache JS corpus with byte-level mutations
//	        (bit flips, random and invalid UTF-8 bytes, token splices);
//	        corpus:LIST does the same over the files listed in LIST
//	html    literals and programs embedded in HTML through neohugo's M
//
// The configuration of every record is drawn from redCfgs.

var redCfgs = []string{
	"v2022", "v2022", "v2022", "v2022-inline", "0", "keep", "keep-v2022", "alpha", "keep-alpha",
	"v2015", "v2016", "v2017", "v2018", "v2019", "v2020", "v2021", "v2019-inline", "v2014",
	"p1-v2022", "p2-v2022", "p3-v2022", "p4", "p5-v2022", "p6", "p8-v2022", "p10", "p15-v2022", "p17", "p20-v2022", "p1-keep",
}

func redCfg(rnd *rand.Rand) string { return redCfgs[rnd.Intn(len(redCfgs))] }

func genRedTeam(kind, out string, n int, seed int64) {
	dir, name := filepath.Split(out)
	name = strings.TrimSuffix(name, ".rec.gz")
	if dir == "" {
		dir = "."
	}
	w := newRecWriter(dir, name)
	rnd := rand.New(rand.NewSource(seed))
	var lits, corpus [][]byte
	if list, ok := strings.CutPrefix(kind, "corpus:"); ok {
		// corpus:LIST mutates windows of the files listed in LIST
		kind = "corpus"
		data, err := os.ReadFile(list)
		if err != nil {
			panic(err)
		}
		for _, p := range strings.Split(strings.TrimSpace(string(data)), "\n") {
			src, err := os.ReadFile(p)
			if err != nil {
				panic(err)
			}
			if 0 < len(src) {
				corpus = append(corpus, src)
			}
		}
	} else if kind == "corpus" {
		corpus = corpusFiles()
	}
	switch kind {
	case "prog", "soup", "corpus", "html":
		lits = nonEmptyLiterals()
	}
	for k := 0; k < n; k++ {
		w.rec(redTeamRecord(kind, rnd, lits, corpus)...)
	}
	w.close()
}

// genRedTeamFixture writes DIR/redteam.rec.gz, the checked-in sample of the
// red-team kinds (digests): lits, logic, num, soup, prog and html.
func genRedTeamFixture(dir string, scale int, seed int64) {
	w := newRecWriter(dir, "redteam")
	rnd := rand.New(rand.NewSource(seed))
	lits := nonEmptyLiterals()
	for _, kn := range []struct {
		kind string
		n    int
	}{{"lits", 1500}, {"logic", 1500}, {"num", 1000}, {"soup", 1000}, {"prog", 500}, {"html", 500}} {
		for k := 0; k < kn.n*scale; k++ {
			w.rec(redTeamRecord(kn.kind, rnd, lits, nil)...)
		}
	}
	w.close()
}

func nonEmptyLiterals() [][]byte {
	var lits [][]byte
	for _, l := range testLiterals() {
		if 0 < len(l) {
			lits = append(lits, l)
		}
	}
	return lits
}

// corpusFiles returns the module-cache JS corpus (as the corpuswin fixture).
func corpusFiles() [][]byte {
	roots := []string{
		filepath.Join(modCache(), "github.com/tdewolff/minify/v2@v2.23.8"),
		filepath.Join(modCache(), "github.com/evanw/esbuild@v0.25.6"),
		filepath.Join(modCache(), "golang.org/x/tools@v0.34.0"),
		repoRoot(),
	}
	var corpus [][]byte
	for _, p := range jsFiles(roots) {
		src, err := os.ReadFile(p)
		if err != nil {
			panic(err)
		}
		if 0 < len(src) {
			corpus = append(corpus, src)
		}
	}
	return corpus
}

// redTeamRecord generates one input of the kind and returns its record
// (`min` or `html`, digests).
func redTeamRecord(kind string, rnd *rand.Rand, lits, corpus [][]byte) [][]byte {
	minRec := func(cfg string, src []byte) [][]byte {
		return append([][]byte{[]byte("min"), []byte(cfg), src}, digests(runMin(cfg, src))...)
	}
	switch kind {
	case "lits":
		src := []byte(genLitProgram(rnd))
		return minRec(redCfg(rnd), src)
	case "num":
		// numeric literals only (minify.Number and the base conversions
		// at every precision)
		var b strings.Builder
		b.WriteString("x=[")
		for i := 0; i < 8; i++ {
			if i != 0 {
				b.WriteByte(',')
			}
			b.WriteString([]string{"", "", "", "-", "!", "~"}[rnd.Intn(6)])
			b.WriteString(genNumber(rnd))
		}
		b.WriteString("]")
		if rnd.Intn(3) == 0 {
			b.WriteString(";y=" + genNumber(rnd) + ".a+(" + genNumber(rnd) + ").b")
		}
		src := []byte(b.String())
		return minRec(fmt.Sprintf("p%d", rnd.Intn(22)), src)
	case "logic":
		src := []byte(genLogicProgram(rnd))
		return minRec(redCfg(rnd), src)
	case "prog":
		src := genProgram(rnd)
		if rnd.Intn(4) == 0 {
			src = mutateOps(rnd, src, lits, 1+rnd.Intn(3))
		}
		return minRec(redCfg(rnd), src)
	case "soup":
		src := []byte(genSoup(rnd))
		return minRec(redCfg(rnd), src)
	case "corpus":
		src := corpus[rnd.Intn(len(corpus))]
		l := 16
		for l < 1<<15 && rnd.Intn(3) != 0 {
			l *= 2
		}
		l = 1 + rnd.Intn(l)
		i := rnd.Intn(len(src))
		j := min(len(src), i+l)
		win := byteMutate(rnd, append([]byte(nil), src[i:j]...), lits, rnd.Intn(4))
		return minRec(redCfg(rnd), win)
	case "html":
		var js string
		switch rnd.Intn(5) {
		case 0, 1:
			js = genLitProgram(rnd)
		case 2:
			js = genLogicProgram(rnd)
		case 3:
			js = genSoup(rnd)
		default:
			js = string(genProgram(rnd))
		}
		if rnd.Intn(4) == 0 {
			// multi-line scripts with errors and invalid UTF-8 (error positions)
			js = "\n  " + string(byteMutate(rnd, []byte(js), lits, 1+rnd.Intn(2)))
		}
		doc := []byte(genHTMLDoc(rnd, js))
		return append([][]byte{[]byte("html"), []byte("hugo"), doc}, digests(runHTML("hugo", doc))...)
	}
	panic("unknown kind " + kind)
}

// byteMutate applies ops byte-level mutations (bit flips, random bytes,
// invalid UTF-8, deletions, token splices from the literals).
func byteMutate(rnd *rand.Rand, src []byte, lits [][]byte, ops int) []byte {
	for k := 0; k < ops; k++ {
		switch rnd.Intn(7) {
		case 0: // bit flip
			if 0 < len(src) {
				src[rnd.Intn(len(src))] ^= 1 << rnd.Intn(8)
			}
		case 1: // random byte
			i := rnd.Intn(len(src) + 1)
			b := []byte{byte(rnd.Intn(256))}
			if rnd.Intn(2) == 0 {
				b[0] = "\x00\n\r\t \"'`\\/*{}()[];,.=<>!?:$#@\x80\xc3\xe2\xff"[rnd.Intn(30)]
			}
			src = append(src[:i:i], append(b, src[i:]...)...)
		case 2: // replace a byte
			if 0 < len(src) {
				src[rnd.Intn(len(src))] = byte(rnd.Intn(256))
			}
		default:
			src = mutateOps(rnd, src, lits, 1)
		}
	}
	return src
}

// genLitProgram returns a short program built around generated literals.
func genLitProgram(rnd *rand.Rand) string {
	var b strings.Builder
	n := 1 + rnd.Intn(3)
	for k := 0; k < n; k++ {
		if k != 0 {
			b.WriteString([]string{";", "\n", ";\n", ","}[rnd.Intn(4)])
		}
		b.WriteString(litStmt(rnd))
	}
	return b.String()
}

func litAny(rnd *rand.Rand) string {
	switch rnd.Intn(10) {
	case 0, 1, 2:
		return genString(rnd)
	case 3, 4, 5:
		return genNumber(rnd)
	case 6, 7:
		return genTemplate(rnd, false)
	case 8:
		return genRegExp(rnd)
	}
	return []string{"a", "b.c", "null", "undefined", "true", "!0", "void 0", "NaN", "Infinity", "-Infinity", "this", "a()", "[]", "{}"}[rnd.Intn(14)]
}

func litStmt(rnd *rand.Rand) string {
	L := func() string { return litAny(rnd) }
	S := func() string { return genString(rnd) }
	N := func() string { return genNumber(rnd) }
	switch rnd.Intn(46) {
	case 0, 1, 2:
		return "x=" + L()
	case 3:
		return "x=" + S() + "+" + S()
	case 4:
		return "x=" + S() + "+a+" + S() + "+" + S()
	case 5:
		return "x=a+" + S() + "+" + L() + "+" + S()
	case 6:
		return "x={" + S() + ":1," + N() + ":2}"
	case 7:
		return "x=a[" + S() + "]+a[" + N() + "]"
	case 8:
		return "x=" + S() + " in a"
	case 9:
		return "switch(a){case " + L() + ":b();case " + L() + ":break}"
	case 10:
		return S() + ";" + S() + ";a"
	case 11:
		return "function f(){" + S() + ";return " + L() + "}"
	case 12:
		return "x=" + genTemplate(rnd, true)
	case 13:
		return "x=a" + genTemplate(rnd, false)
	case 14:
		return "import(" + S() + ")"
	case 15:
		return "x={[" + L() + "]:1,[" + S() + "](){}}"
	case 16:
		return "x=" + L() + ".length+" + N() + ".toString()"
	case 17:
		return "x=typeof a==" + S() + "||" + S() + "!==typeof b"
	case 18:
		return "x=-" + N() + "+-" + N()
	case 19:
		return "x=" + N() + "-" + N() + "*" + N()
	case 20:
		return "x=" + N() + "[a]+" + N() + "..a+" + N() + ".a"
	case 21:
		return "x=!" + L() + "&&!" + L()
	case 22:
		return "if(" + L() + ")a();else if(" + L() + ")b()"
	case 23:
		return "x=" + L() + "?" + L() + ":" + L()
	case 24:
		return "x=Number(" + L() + ")+isNaN(" + L() + ")+Math.pow(" + N() + "," + N() + ")"
	case 25:
		return "x=(-" + N() + ")**" + N()
	case 26:
		return "x=void " + L() + ",y=typeof " + L()
	case 27:
		return "class A{" + S() + "(){}" + N() + "=1;static " + S() + "=2}"
	case 28:
		return "x=a==" + L() + "?b:" + L()
	case 29:
		return "x=" + genRegExp(rnd) + ".test(" + S() + ")"
	case 30:
		return "a(" + genRegExp(rnd) + "," + genRegExp(rnd) + ")"
	case 31:
		return "x=a/" + N() + "/" + N()
	case 32:
		return "x={" + S() + "," + "get " + S() + "(){}}"
	case 33:
		return "import a," + "{" + S() + " as b}from" + S()
	case 34:
		return "export{a as " + S() + "}from" + S()
	case 35:
		return "var " + genIdent(rnd) + "=" + L() + "," + genIdent(rnd) + "=" + L()
	case 36:
		return "x=" + L() + "+" + L() + "+" + L()
	case 37:
		return "x=" + L() + "==" + L() + "," + L() + "!=" + L()
	case 38:
		return "x=`" + "${" + L() + "}" + "`+`${a}" + "${" + S() + "}`"
	case 39:
		return "for(;" + L() + ";)a()"
	case 40:
		return "while(" + L() + ")a()"
	case 41:
		return "x=a?.[" + L() + "]"
	case 42:
		return "x={a:" + L() + ",b:" + L() + "}." + genIdent(rnd)
	case 43:
		return "x=" + genIdent(rnd) + "=" + L()
	case 44:
		return "let{" + S() + ":a=" + L() + "}=b"
	}
	return "x=[" + L() + "," + L() + ",..." + L() + "]"
}

var identPieces = []string{"a", "b", "$", "_", "é", "α", "\\u0061", "\\u{62}", "\\u0024", "1", "ℹ", "\u200c", "\u200d", "中", "\\u{1F600}", "\\u00e9", "\\u0030"}

func genIdent(rnd *rand.Rand) string {
	var b strings.Builder
	n := 1 + rnd.Intn(3)
	for i := 0; i < n; i++ {
		p := identPieces[rnd.Intn(len(identPieces))]
		if i == 0 && (p == "1" || p == "\u200c" || p == "\u200d" || p == "\\u0030") {
			p = "c"
		}
		b.WriteString(p)
	}
	return b.String()
}

// strPieces are string-literal content fragments (without the quote).
var strPieces = []string{
	"a", "b", "0", "1", "7", "8", "9", " ", "é", "中", "😀", "\u2028", "\u2029", "\t", "\u00a0", "\ufeff",
	"'", "\"", "`", "$", "${", "{", "}", "$\\{", "\\${",
	"<", "</script>", "</SCRIPT>", "<\\/script>", "<!--", "-->", "</scrip", "</script",
	"\\\\", "\\'", "\\\"", "\\`", "\\n", "\\r", "\\t", "\\b", "\\f", "\\v", "\\0", "\\00", "\\000", "\\1", "\\7",
	"\\8", "\\9", "\\12", "\\42", "\\47", "\\140", "\\134", "\\377", "\\400", "\\08", "\\0a",
	"\\x00", "\\x0a", "\\x0A", "\\x0d", "\\x22", "\\x27", "\\x60", "\\x5c", "\\x5C", "\\x41", "\\x7f", "\\x80", "\\xff", "\\x1", "\\x", "\\xg1", "\\x24",
	"\\u0000", "\\u000a", "\\u000A", "\\u000d", "\\u0022", "\\u0027", "\\u0060", "\\u005c", "\\u0041", "\\u2028", "\\u2029", "\\ud800", "\\uDFFF", "\\ud83d\\ude00", "\\u00e9", "\\u0024",
	"\\u{0}", "\\u{a}", "\\u{A}", "\\u{00000a}", "\\u{22}", "\\u{27}", "\\u{60}", "\\u{10FFFF}", "\\u{110000}", "\\u{1F600}", "\\u{}", "\\u{0000000}", "\\u{0000041}", "\\u{d800}", "\\u{5c}", "\\u{24}",
	"\\uZZZZ", "\\u12", "\\u", "\\u{", "\\u{12", "\\q", "\\a", "\\é", "\\\n", "\\\r\n", "\\\r", "\\\u2028", "\\\u2029", "\\\t",
	"\xff", "\xc3", "\xe2\x80", "\x00", "\x7f",
}

// genStrBody returns literal content for the quote q: a raw q (and a raw
// "${" in templates) is escaped, everything else is kept as is.
func genStrBody(rnd *rand.Rand, max int, q string) string {
	var b strings.Builder
	n := rnd.Intn(max)
	for i := 0; i < n; i++ {
		p := strPieces[rnd.Intn(len(strPieces))]
		if p == q {
			p = "\\" + q
		} else if q == "`" && p == "${" {
			p = "$\\{"
		} else if q == "`" && p == "$" && rnd.Intn(2) == 0 {
			p = "\\$"
		}
		b.WriteString(p)
	}
	return b.String()
}

func genString(rnd *rand.Rand) string {
	q := "'"
	if rnd.Intn(2) == 0 {
		q = "\""
	}
	return q + genStrBody(rnd, 8, q) + q
}

func genTemplate(rnd *rand.Rand, parts bool) string {
	var b strings.Builder
	b.WriteString("`")
	n := 1
	if parts || rnd.Intn(3) == 0 {
		n += rnd.Intn(3)
	}
	for i := 0; i < n; i++ {
		if i != 0 {
			b.WriteString("${")
			b.WriteString([]string{"a", "b+c", genString(rnd), genNumber(rnd), "`x${y}`", "{}", "a?b:c"}[rnd.Intn(7)])
			b.WriteString("}")
		}
		body := genStrBody(rnd, 6, "`")
		if rnd.Intn(4) == 0 {
			body += "\n\r\n"
		}
		b.WriteString(body)
	}
	b.WriteString("`")
	return b.String()
}

func digits(rnd *rand.Rand, n int, alphabet string, sep bool) string {
	var b strings.Builder
	for i := 0; i < n; i++ {
		if sep && 0 < i && rnd.Intn(6) == 0 {
			b.WriteByte('_')
		}
		// bias to 0 and the top digit (carries and trailing zeros)
		switch rnd.Intn(4) {
		case 0:
			b.WriteByte(alphabet[0])
		case 1:
			b.WriteByte(alphabet[len(alphabet)-1])
		default:
			b.WriteByte(alphabet[rnd.Intn(len(alphabet))])
		}
	}
	return b.String()
}

func genNumber(rnd *rand.Rand) string {
	sep := rnd.Intn(5) == 0
	lenDist := func(max int) int {
		if rnd.Intn(3) == 0 {
			return rnd.Intn(max + 1)
		}
		return rnd.Intn(4)
	}
	switch rnd.Intn(10) {
	case 0:
		s := "0" + []string{"x", "X"}[rnd.Intn(2)] + digits(rnd, 1+lenDist(16), "0123456789abcdefABCDEF", sep)
		if rnd.Intn(6) == 0 {
			s += "n"
		}
		return s
	case 1:
		s := "0" + []string{"o", "O"}[rnd.Intn(2)] + digits(rnd, 1+lenDist(25), "01234567", sep)
		if rnd.Intn(6) == 0 {
			s += "n"
		}
		return s
	case 2:
		s := "0" + []string{"b", "B"}[rnd.Intn(2)] + digits(rnd, 1+lenDist(70), "01", sep)
		if rnd.Intn(6) == 0 {
			s += "n"
		}
		return s
	case 3:
		// integers, sometimes BigInt
		s := digits(rnd, 1+lenDist(25), "0123456789", sep)
		if 1 < len(s) && s[0] == '0' && rnd.Intn(3) != 0 {
			s = "1" + s[1:]
		}
		if rnd.Intn(5) == 0 {
			s += "n"
		}
		return s
	}
	// decimals
	var b strings.Builder
	intLen := lenDist(22)
	if 0 < intLen {
		s := digits(rnd, intLen, "0123456789", sep)
		if 1 < len(s) && s[0] == '0' && rnd.Intn(4) != 0 {
			s = "9" + s[1:]
		}
		b.WriteString(s)
	}
	if intLen == 0 || rnd.Intn(2) == 0 {
		b.WriteByte('.')
		fl := lenDist(22)
		if intLen == 0 && fl == 0 {
			fl = 1
		}
		if 0 < fl {
			if rnd.Intn(3) == 0 {
				b.WriteString(strings.Repeat("0", rnd.Intn(12)))
			}
			b.WriteString(digits(rnd, fl, "0123456789", sep))
		}
	}
	if rnd.Intn(3) == 0 {
		b.WriteString([]string{"e", "E"}[rnd.Intn(2)])
		b.WriteString([]string{"", "+", "-"}[rnd.Intn(3)])
		switch rnd.Intn(5) {
		case 0:
			b.WriteString([]string{"308", "309", "324", "325", "400", "999", "9999999999", "2147483647", "2147483648", "9223372036854775807", "9223372036854775808", "99999999999999999999"}[rnd.Intn(12)])
		default:
			b.WriteString(digits(rnd, 1+rnd.Intn(3), "0123456789", sep))
		}
	}
	return b.String()
}

var rePieces = []string{
	"a", "b", "0", "é", " ", ".", "*", "+", "?", "|", "^", "$", "(", ")", "(?:", "(?<n>", "{2}", "{1,}", "{", "}",
	"[", "]", "[^", "-", "\\", "\\/", "\\-", "\\^", "\\]", "\\[", "\\\\", "\\.", "\\d", "\\w", "\\s", "\\b", "\\B",
	"\\a", "\\e", "\\z", "\\ ", "\\é", "\\u0041", "\\u{41}", "\\x41", "\\c", "\\cA", "\\k<n>", "\\{", "\\}", "\\|",
	"\\$", "\\(", "\\)", "\\*", "\\+", "\\?", "\\1", "\\0", "\\p{L}", "\\P{L}", "\\n", "\\t", "\\/script>", "</script>", "/",
}

func genRegExp(rnd *rand.Rand) string {
	var b strings.Builder
	b.WriteString("/")
	n := 1 + rnd.Intn(8)
	inClass := false
	for i := 0; i < n; i++ {
		p := rePieces[rnd.Intn(len(rePieces))]
		if p == "\\" {
			p = "\\\\"
		}
		if p == "/" && !inClass {
			p = "\\/"
		}
		switch p {
		case "[", "[^":
			inClass = true
		case "]":
			inClass = false
		}
		b.WriteString(p)
	}
	if inClass {
		b.WriteString("]")
	}
	s := b.String()
	if s == "/" || strings.HasPrefix(s, "/*") {
		s = "/a"
	}
	return s + "/" + []string{"", "", "g", "i", "gimsuy", "d", "v", "u"}[rnd.Intn(8)]
}

var soupTokens = []string{
	" ", "\n", ";", ",", "(", ")", "{", "}", "[", "]", "=", "==", "===", "!", "!=", "!==", "?", ":", "?.", "??", "??=",
	"=>", "...", ".", "+", "-", "*", "/", "%", "**", "++", "--", "<", ">", "<=", ">=", "<<", ">>", ">>>", ">>>=", "&&",
	"||", "&&=", "||=", "&", "|", "^", "~", "+=", "-=", "/=", "#x", "@", "\\u0061", "a", "b", "c", "x", "1", ".5", "0x1",
	"1n", "'s'", "\"d\"", "`t`", "`a${", "}`", "/re/", "var", "let", "const", "function", "return", "if", "else", "for",
	"while", "do", "in", "of", "new", "class", "extends", "super", "this", "typeof", "void", "delete", "await", "yield",
	"async", "static", "get", "set", "import", "export", "default", "from", "as", "try", "catch", "finally", "throw",
	"break", "continue", "switch", "case", "with", "debugger", "null", "true", "false", "undefined", "NaN", "Infinity",
	"arguments", "eval", "target", "meta", "/*c*/", "//c\n", "<!--", "-->", "\u2028", "é", "\x00", "\xff",
}

func genSoup(rnd *rand.Rand) string {
	var b strings.Builder
	n := 1 + rnd.Intn(30)
	for i := 0; i < n; i++ {
		b.WriteString(soupTokens[rnd.Intn(len(soupTokens))])
		if rnd.Intn(3) == 0 {
			b.WriteByte(' ')
		}
	}
	return b.String()
}

var htmlRedWrappers = []string{
	"<script>%s</script>",
	"<script type=module>%s</script>",
	"<script type=text/javascript>\n%s\n</script><p>x",
	"<button onclick=\"%s\">b</button>",
	"<a onclick='%s' onmouseover=\"javascript:%s\">x</a>",
	"<body onload=\"  %s  \"><script>%s</script>",
	"<svg><script>%s</script></svg>",
	"<script>%s",
	"<p onclick=%s>",
}

func genHTMLDoc(rnd *rand.Rand, js string) string {
	wrap := htmlRedWrappers[rnd.Intn(len(htmlRedWrappers))]
	s := js
	if strings.Contains(wrap, "on") && rnd.Intn(2) == 0 {
		s = strings.NewReplacer("&", "&amp;", "\"", "&#34;", "'", "&#39;", "<", "&lt;", ">", "&gt;").Replace(s)
	}
	if strings.Count(wrap, "%s") == 1 {
		return fmt.Sprintf(wrap, s)
	}
	return fmt.Sprintf(wrap, s, s)
}

// genFiles writes OUT: one `multi` record per file listed in LIST (one
// path per line), minified with every configuration of cfgs (digests). The
// configuration "html" runs the file as the body of a <script> element
// through neohugo's M instead (an extra `html` record).
func genFiles(out string, cfgs []string, list string) {
	data, err := os.ReadFile(list)
	if err != nil {
		panic(err)
	}
	dir, name := filepath.Split(out)
	if dir == "" {
		dir = "."
	}
	w := newRecWriter(dir, strings.TrimSuffix(name, ".rec.gz"))
	for _, p := range strings.Split(strings.TrimSpace(string(data)), "\n") {
		src, err := os.ReadFile(p)
		if err != nil {
			panic(err)
		}
		fields := [][]byte{[]byte("multi"), []byte(p), src}
		for _, cfg := range cfgs {
			if cfg == "html" {
				doc := append(append([]byte("<script>"), src...), "</script>"...)
				w.rec(append([][]byte{[]byte("html"), []byte("hugo"), doc}, digests(runHTML("hugo", doc))...)...)
				continue
			}
			fields = append(fields, []byte(cfg))
			fields = append(fields, digests(runMin(cfg, src))...)
		}
		w.rec(fields...)
	}
	w.close()
}
