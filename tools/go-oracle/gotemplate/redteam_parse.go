//go:build gotemplate_oracle

// Red-team mode "rtparse": seeded random and mutated template sources for
// the lexer and parser, written in the parse.txt.gz record format (see
// parse.go) so crates/gotemplate/tests/parse_oracle.rs can replay them.
//
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate rtparse <out.txt.gz> <seed> <n>
//
// The corpus is large and not checked in: parse_oracle.rs's ignored test
// parse_redteam reads it from $GOTEMPLATE_RT_PARSE. Divergences found this
// way are kept as small regression fixtures (mode "rtfixtures").
package main

import (
	"bufio"
	"compress/gzip"
	"fmt"
	"math/rand"
	"os"
	"sort"
	"strconv"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate/parse"
)

func init() {
	register("rtparse", rtParseMain)
}

// rtArgs parses "<out> <seed> <n>".
func rtArgs(mode string, args []string) (string, int64, int, error) {
	if len(args) != 3 {
		return "", 0, 0, fmt.Errorf("usage: %s <out.txt.gz> <seed> <n>", mode)
	}
	seed, err := strconv.ParseInt(args[1], 10, 64)
	if err != nil {
		return "", 0, 0, err
	}
	n, err := strconv.Atoi(args[2])
	if err != nil {
		return "", 0, 0, err
	}
	return args[0], seed, n, nil
}

// rtWriteGz writes a (fast-compressed) gzip file.
func rtWriteGz(path string, write func(w *bufio.Writer) error) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	zw, _ := gzip.NewWriterLevel(f, gzip.BestSpeed)
	w := bufio.NewWriterSize(zw, 1<<20)
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

// ptxWriteFuncSets writes the "funcs" header lines of a parse fixture.
func ptxWriteFuncSets(w *bufio.Writer) {
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
}

func rtParseMain(args []string) error {
	out, seed, n, err := rtArgs("rtparse", args)
	if err != nil {
		return err
	}
	r := rand.New(rand.NewSource(seed))
	return rtWriteGz(out, func(w *bufio.Writer) error {
		ptxWriteFuncSets(w)
		for i := 0; i < n; i++ {
			ptxRun(w, i, rtParseCase(r))
		}
		fmt.Fprintf(os.Stderr, "rtparse: seed %d, %d cases\n", seed, n)
		return nil
	})
}

// rtDelims are left/right delimiter pairs, including ones that overlap
// the trim marker, the comment markers, spaces and multi-byte runes.
var rtDelims = [][2]string{
	{"", ""}, {"{{", "}}"}, {"<<", ">>"}, {"[[", "]]"}, {"{%", "%}"}, {"$$", "@@"}, {"(", ")"},
	{"<!--", "-->"}, {"{{-", "-}}"}, {"{{ ", " }}"}, {"/*", "*/"}, {"«", "»"}, {"<", ">"}, {"|", "|"},
	{"{", "}"}, {"-", "-"}, {"{{{", "}}}"}, {"@", "@"}, {"{{", "]]"}, {"a", "b"},
	{" ", " "}, {"{{/*", "*/}}"}, {"--", "--"}, {"%", "%"},
}

// rtParseCase builds one random parse case.
func rtParseCase(r *rand.Rand) ptxCase {
	c := ptxCase{name: "rt", funcs: "builtins"}
	switch r.Intn(8) {
	case 0:
		c.mode = parse.ParseComments
	case 1:
		c.mode = parse.SkipFuncCheck
		c.funcs = "none"
	case 2:
		c.mode = parse.ParseComments | parse.SkipFuncCheck
		c.funcs = "none"
	case 3:
		c.funcs = "hugo"
	case 4:
		c.funcs = "go"
	}
	left, right := "{{", "}}"
	if r.Intn(4) == 0 {
		d := rtDelims[r.Intn(len(rtDelims))]
		left, right = d[0], d[1]
		if r.Intn(4) == 0 {
			right = rtDelims[r.Intn(len(rtDelims))][1]
		}
	}
	c.left, c.right = left, right
	// The effective delimiters (Go: "" means the default).
	if left == "" {
		left = "{{"
	}
	if right == "" {
		right = "}}"
	}
	if r.Intn(20) == 0 {
		c.name = ptxNames[r.Intn(len(ptxNames))]
	}
	g := &rtGen{r: r, left: left, right: right}
	switch k := r.Intn(10); {
	case k < 3:
		c.src = g.soup()
	case k < 5:
		pg := &ptxGen{r: r, left: left, right: right, clean: r.Intn(10) < 6}
		c.src = pg.template(0)
	case k < 6:
		c.src = g.literals()
	default:
		c.src = g.structure(0)
	}
	if r.Intn(4) == 0 {
		c.src = g.mutate(c.src)
	}
	return c
}

// rtGen generates adversarial template sources.
type rtGen struct {
	r           *rand.Rand
	left, right string
	defs        int
}

func (g *rtGen) pick(ss ...string) string { return ss[g.r.Intn(len(ss))] }

// number builds a random numeric literal (valid or not).
func (g *rtGen) number() string {
	if g.r.Intn(4) == 0 {
		return ptxNumbers[g.r.Intn(len(ptxNumbers))]
	}
	var b strings.Builder
	one := func() {
		b.WriteString(g.pick("", "", "", "+", "-"))
		prefix := g.pick("", "", "", "0x", "0X", "0o", "0O", "0b", "0B", "0")
		b.WriteString(prefix)
		digits := "0123456789"
		switch prefix {
		case "0x", "0X":
			digits = "0123456789abcdefABCDEF"
		case "0b", "0B":
			digits = "01"
		case "0o", "0O":
			digits = "01234567"
		}
		wrong := "_89aefgpxi."
		n := g.r.Intn(4)
		for i := 0; i < n; i++ {
			switch g.r.Intn(10) {
			case 0:
				b.WriteByte('_')
			case 1:
				b.WriteByte(wrong[g.r.Intn(len(wrong))])
			default:
				b.WriteByte(digits[g.r.Intn(len(digits))])
			}
		}
		if g.r.Intn(3) == 0 {
			b.WriteByte('.')
			for i := g.r.Intn(3); i > 0; i-- {
				b.WriteByte(digits[g.r.Intn(len(digits))])
			}
		}
		if g.r.Intn(3) == 0 {
			b.WriteString(g.pick("e", "E", "p", "P"))
			b.WriteString(g.pick("", "+", "-"))
			for i := g.r.Intn(3); i > 0; i-- {
				b.WriteByte("0123456789_"[g.r.Intn(11)])
			}
		}
		if g.r.Intn(6) == 0 {
			b.WriteByte('i')
		}
	}
	one()
	if g.r.Intn(8) == 0 {
		b.WriteString(g.pick("+", "-"))
		one()
		b.WriteString(g.pick("i", "i", ""))
	}
	if g.r.Intn(10) == 0 {
		// Huge digit strings (overflow paths).
		b.WriteString(strings.Repeat(g.pick("9", "0", "1", "f"), 1+g.r.Intn(30)))
	}
	return b.String()
}

// char builds a random character constant.
func (g *rtGen) char() string {
	body := g.pick("a", "é", "本", "\\n", "\\t", "\\x41", "\\xff", "\\xZZ", "\\x4", "\\u00e9", "\\uD800", "\\u12",
		"\\U0001F600", "\\U00110000", "\\'", "\\\"", "\\\\", "\\0", "\\000", "\\101", "\\377", "\\400", "\\8",
		"ab", "", "\xff", "\xc3", "\n", "\\a", "\\b", "\\f", "\\r", "\\v", "\\q", " ", "'", "\\", "\x00", " ")
	end := g.pick("'", "'", "'", "", "''")
	return "'" + body + end
}

// str builds a random interpreted or raw string literal.
func (g *rtGen) str() string {
	if g.r.Intn(3) == 0 {
		body := g.pick("", "x", "a\nb", "}}", g.right, g.left, "\"", "'", "\\n", "\xff", "é", "`", "\r\n")
		return "`" + body + g.pick("`", "`", "`", "")
	}
	var b strings.Builder
	b.WriteByte('"')
	for i := g.r.Intn(4); i > 0; i-- {
		b.WriteString(g.pick("a", " ", "é", "\\n", "\\\"", "\\x41", "\\xff", "\\u00e9", "\\U0001F600", "\\q",
			"\\", "\\uD800", "\\101", "\\400", "%d", g.right, "\xff", "\t", "\\'", "'"))
	}
	if g.r.Intn(12) == 0 {
		b.WriteString(g.pick("\n", "\r"))
	}
	b.WriteString(g.pick("\"", "\"", "\"", "\"", ""))
	return b.String()
}

// ident returns a random identifier-like word.
func (g *rtGen) ident() string {
	return g.pick("printf", "print", "len", "index", "and", "or", "not", "eq", "ne", "lt", "html", "js", "call",
		"slice", "urlquery", "println", "x", "é", "名前", "_a", "a1", "A", "partial", "T", "try", "dict", "undefinedFn",
		"truex", "nilx", "if1", "end_", "a\xffb", "Ωmega", "x٣", "ǅ", "ʰ")
}

// soup is a random sequence of lexer-relevant tokens.
func (g *rtGen) soup() string {
	var b strings.Builder
	n := 1 + g.r.Intn(24)
	inAction := false
	for i := 0; i < n; i++ {
		if !inAction {
			switch g.r.Intn(6) {
			case 0:
				b.WriteString(g.pick("x", " ", "\n", "  \t", "é", "\xff", "{", "}", "-", "/*", "*/", g.right, "a b"))
			default:
				b.WriteString(g.left)
				switch g.r.Intn(5) {
				case 0:
					b.WriteString(g.pick("-", "- ", "-\n", "-\t", "-\r", "--", "- -"))
				case 1:
					b.WriteString(g.pick("/*", "/* c */", "/**/", "/* a\nb */", "/* x", "/*/", "- /* c */"))
				}
				inAction = true
			}
			continue
		}
		switch g.r.Intn(22) {
		case 0:
			b.WriteString(g.pick(" -", "-", " - ", "\n-", "\t-", "- ") + g.right)
			inAction = false
		case 1, 2:
			b.WriteString(g.right)
			inAction = false
		case 3:
			b.WriteString(g.pick("if", "else", "end", "range", "with", "define", "block", "template", "break",
				"continue", "nil", "true", "false", "else if", "else with"))
		case 4:
			b.WriteString(g.ident())
		case 5:
			b.WriteString(g.pick(".", ".X", ".Y.Z", ".é", ".X.", "..", ". .", ".1", ".x1", ".Ω", ".\xff"))
		case 6:
			b.WriteString(g.pick("$", "$x", "$1", "$é", "$x.Y", "$.X", "$$", "$ x"))
		case 7:
			b.WriteString(g.number())
		case 8:
			b.WriteString(g.str())
		case 9:
			b.WriteString(g.char())
		case 10, 11, 12:
			b.WriteString(g.pick(" ", " ", "\t", "\n", "\r", "\r\n", "  "))
		case 13:
			b.WriteString(g.pick("(", ")", "|", ",", ":=", "=", ":", "!", "@", "#", "%", "&", "*", "+", "-", ";",
				"<", ">", "?", "[", "]", "^", "{", "}", "~", "\\", "/", "\x00", "\x01", " ", " "))
		case 14:
			b.WriteString(g.pick("/*", "*/", "/* c */", " /* c */ "))
		case 15:
			b.WriteString(g.left)
		case 16:
			b.WriteString(g.pick("(", "(", ")", "("+g.ident()+")"))
		default:
			b.WriteString(" ")
		}
	}
	if inAction && g.r.Intn(3) != 0 {
		b.WriteString(g.right)
	}
	return b.String()
}

// literals exercises the literal syntaxes in every position.
func (g *rtGen) literals() string {
	var b strings.Builder
	for i := 1 + g.r.Intn(3); i > 0; i-- {
		var lit string
		switch g.r.Intn(3) {
		case 0:
			lit = g.number()
		case 1:
			lit = g.char()
		default:
			lit = g.str()
		}
		shape := g.pick("%s", "print %s", "%s.X", "(%s)", "%s | print", "print (%s) %s", "$x := %s", "if %s",
			"printf %s %s", "(%s).X.Y", "%s %s", "- %s -", "-%s-")
		act := strings.ReplaceAll(shape, "%s", lit)
		b.WriteString(g.left + act + g.right)
		if strings.HasPrefix(shape, "$x") {
			b.WriteString(g.left + "$x" + g.right)
		}
		if strings.HasPrefix(shape, "if") {
			b.WriteString("y" + g.left + "end" + g.right)
		}
		b.WriteString(g.pick("", " ", "\n", "t"))
	}
	return b.String()
}

func (g *rtGen) act(s string) string {
	l, r := g.left, g.right
	switch g.r.Intn(6) {
	case 0:
		l += g.pick("- ", "-\n", "-\t")
	case 1:
		r = g.pick(" -", "\n-", "\t-") + r
	case 2:
		l += g.pick("- ", "-\n")
		r = g.pick(" -", "\n-") + r
	}
	if g.r.Intn(8) == 0 {
		s = g.pick("/* c */ ", "/**/", " ") + s
	}
	return l + g.pick("", " ", "  ", "\n") + s + g.pick("", " ", "\n") + r
}

func (g *rtGen) comment() string {
	c := g.pick("/* c */", "/**/", "/* multi\nline */", "/* {{x}} */", "/* */ ")
	switch g.r.Intn(5) {
	case 0:
		return g.left + "- " + c + " -" + g.right
	case 1:
		return g.left + "- " + c + g.right
	case 2:
		return g.left + c + " -" + g.right
	case 3:
		return g.left + g.pick(" ", "\n", "") + c + g.pick(" ", "\n", "") + g.right
	}
	return g.left + c + g.right
}

func (g *rtGen) operand(depth int) string {
	switch g.r.Intn(12) {
	case 0:
		return g.number()
	case 1:
		return g.str()
	case 2:
		return g.char()
	case 3:
		return g.pick(".", ".X", ".X.Y", "$", "$.X", "$x", "$x.Y", "$y")
	case 4:
		return g.pick("true", "false", "nil")
	case 5:
		if depth < 3 {
			return "(" + g.pipeline(depth+1) + ")" + g.pick("", "", ".X", ".X.Y")
		}
	case 6:
		return g.ident()
	}
	return g.pick(".X", "1", `"s"`, "$", ".", "printf", "2.5", "$x")
}

func (g *rtGen) pipeline(depth int) string {
	var parts []string
	if g.r.Intn(5) == 0 {
		parts = append(parts, g.pick("$x := ", "$x = ", "$y := ", "$x, $y := ", "$x, $y = ", "$ := ", "$x.Y := "))
	}
	for i := 1 + g.r.Intn(3); i > 0; i-- {
		var args []string
		for j := 1 + g.r.Intn(3); j > 0; j-- {
			args = append(args, g.operand(depth))
		}
		parts = append(parts, strings.Join(args, g.pick(" ", "  ", "\n")))
		if i > 1 {
			parts = append(parts, g.pick(" | ", "|", " |\n"))
		}
	}
	return strings.Join(parts, "")
}

// structure builds nested control structures, defines and blocks, with
// break/continue and variables in and out of scope.
func (g *rtGen) structure(depth int) string {
	var b strings.Builder
	n := 1 + g.r.Intn(5)
	for i := 0; i < n; i++ {
		switch k := g.r.Intn(16); {
		case k < 3:
			b.WriteString(g.pick("x", " ", "\n", "  ", "é", "a\n\n", "\t"))
		case k < 5:
			b.WriteString(g.act(g.pipeline(0)))
		case k < 6:
			b.WriteString(g.comment())
		case k < 9 && depth < 4:
			kw := g.pick("if", "with", "range", "if", "with", "range")
			head := kw + " " + g.pipeline(0)
			if kw == "range" && g.r.Intn(3) == 0 {
				head = kw + " " + g.pick("$i, $e := ", "$e := ", "$i, $e = ", "$e = ", "$i,$e:=") + g.pipeline(0)
			}
			b.WriteString(g.act(head))
			b.WriteString(g.structure(depth + 1))
			for e := g.r.Intn(3); e > 0; e-- {
				b.WriteString(g.act(g.pick("else", "else if "+g.pipeline(0), "else with "+g.pipeline(0),
					"else range "+g.pipeline(0), "else if", "else with", "else "+g.pipeline(0))))
				b.WriteString(g.structure(depth + 1))
			}
			if g.r.Intn(12) != 0 {
				b.WriteString(g.act(g.pick("end", "end", "end", "end 1", "end\n")))
			}
		case k < 11:
			b.WriteString(g.act(g.pick("break", "continue", "break 1", "continue $x", "break\n")))
		case k < 13 && depth < 3:
			g.defs++
			name := g.pick(fmt.Sprintf("%q", fmt.Sprintf("d%d", g.defs)), `"d"`, "`d`", `""`, `"a b"`, `"\xff"`,
				`"x"`, `"_internal/x"`)
			kw := g.pick("define ", "block ", "define ", "block ")
			head := kw + name
			if kw == "block " || g.r.Intn(8) == 0 {
				head += " " + g.pipeline(0)
			}
			b.WriteString(g.act(head))
			b.WriteString(g.structure(depth + 1))
			if g.r.Intn(12) != 0 {
				b.WriteString(g.act("end"))
			}
		case k < 14:
			name := g.pick(`"d1"`, `"d"`, "`d`", `"missing"`, `"x"`, "$x", ".X", `"a" "b"`, "")
			if g.r.Intn(2) == 0 {
				b.WriteString(g.act("template " + name))
			} else {
				b.WriteString(g.act("template " + name + " " + g.pipeline(0)))
			}
		default:
			b.WriteString(g.act(g.pick("$x := 1", "$x = 2", "$y := $x", "$x", "$y", "$x := $x", "$e", "$i")))
		}
	}
	return b.String()
}

// mutate applies a few random byte edits.
func (g *rtGen) mutate(s string) string {
	b := []byte(s)
	for n := 1 + g.r.Intn(3); n > 0; n-- {
		if len(b) == 0 {
			b = append(b, g.left...)
			continue
		}
		p := g.r.Intn(len(b))
		switch g.r.Intn(4) {
		case 0:
			b = append(b[:p], b[p+1:]...)
		case 1:
			ins := g.pick("{", "}", "(", ")", "|", "$", ".", "-", "\"", "`", "'", "/", "*", "\n", " ", "0", "a", "é",
				":", "=", g.left, g.right, "\xff", "\\")
			b = append(b[:p], append([]byte(ins), b[p:]...)...)
		case 2:
			if p+1 < len(b) {
				b[p], b[p+1] = b[p+1], b[p]
			}
		case 3:
			q := p + g.r.Intn(len(b)-p)
			b = append(b[:p], b[q:]...)
		}
	}
	return string(b)
}
