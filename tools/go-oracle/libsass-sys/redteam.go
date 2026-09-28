package main

// Red-team corpus (-redteam <path>): error translation (LibSass error JSON
// -> libsasserrors.Error and its Error() string) over thousands of failing
// inputs, plus fixed deep-recursion cases. Written in the adv.rec.zz record
// format; crates/libsass-sys/tests/oracle.rs reads it.
//
// The random part draws from:
//   - @error with every kind of value: quoted strings with Sass escapes of
//     every C0 control, DEL, C1 controls, quotes, backslashes, non-printable
//     and unassigned runes, surrogates and out-of-range escapes, raw control
//     bytes and raw invalid UTF-8; unquoted text, numbers, lists, maps,
//     colors, null, interpolation, concatenations;
//   - undefined variables, mixins and placeholders, wrong builtin arguments,
//     incompatible units, @extend of missing targets, broken syntax (mutated
//     snippets), string functions on invalid UTF-8;
//   - @import of unresolvable URLs (random bytes), and errors inside imported
//     bodies served by the resolver under random new URLs (control
//     characters, quotes, backslashes, non-printables, invalid UTF-8) at
//     random lines and columns, at a random depth of an import chain, in
//     SCSS and indented syntax;
//   - random output styles, precisions and source-map options.

import (
	"fmt"
	"math/rand"
	"strings"
	"unicode/utf8"
)

// rtRunes are runes that stress %q (strconv.IsPrint) and LibSass's JSON
// string escaping.
var rtRunes = []rune{
	0x01, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x1b, 0x1e, 0x1f, 0x7f,
	0x80, 0x85, 0x9f, 0xa0, 0xad, 0xff, 0x100, 0x34f, 0x378, 0x37f, 0x5d0,
	0x600, 0x61c, 0x6dd, 0x70f, 0x8e2, 0x180e, 0x1680, 0x2000, 0x200a, 0x200b,
	0x200c, 0x200d, 0x200e, 0x200f, 0x2028, 0x2029, 0x202a, 0x202e, 0x202f,
	0x205f, 0x2060, 0x2064, 0x2066, 0x206f, 0x3000, 0x31e4, 0x31e5, 0x9fff,
	0xd7ff, 0xe000, 0xf8ff, 0xfdd0, 0xfeff, 0xfff9, 0xfffb, 0xfffc, 0xfffd,
	0xfffe, 0xffff, 0x10000, 0x110bd, 0x1d173, 0x1f600, 0x2fffe, 0x30000,
	0x3fffd, 0x40000, 0xe0001, 0xe0020, 0xe007f, 0xe01ef, 0xe01f0, 0xf0000,
	0xffffd, 0x10fffd, 0x10ffff, 'é', '日', '"', '\'', '\\', '#', '{', '}',
	';', ':', '$', '@', '%', '&', ' ',
}

func rtRune(r *rand.Rand) rune {
	switch r.Intn(6) {
	case 0, 1:
		return rtRunes[r.Intn(len(rtRunes))]
	case 2:
		return rune(0x20 + r.Intn(0x5f))
	case 3:
		return rune(r.Intn(0x800))
	case 4:
		for {
			c := rune(r.Intn(0x110000))
			if c < 0xd800 || c > 0xdfff {
				return c
			}
		}
	default:
		return randRune(r)
	}
}

// rtSassString writes s as a quoted Sass string literal. Runes are written
// raw or as Sass hex escapes; quotes and backslashes are always escaped,
// newlines always hex-escaped (a raw newline ends the string).
func rtSassString(r *rand.Rand, s string, q byte) string {
	var sb strings.Builder
	sb.WriteByte(q)
	for _, c := range s {
		switch {
		case c == rune(q) || c == '\\':
			sb.WriteByte('\\')
			sb.WriteRune(c)
		case c == '\n' || c == '\r' || c == '\f' || c < 0x20 && r.Intn(2) == 0:
			fmt.Fprintf(&sb, "\\%x ", c)
		case c == '#' && r.Intn(2) == 0:
			sb.WriteString("\\#")
		case r.Intn(5) == 0:
			fmt.Fprintf(&sb, "\\%X", c)
			if r.Intn(2) == 0 {
				sb.WriteByte(' ')
			}
		default:
			sb.WriteRune(c)
		}
	}
	sb.WriteByte(q)
	return sb.String()
}

func rtText(r *rand.Rand, n int) string {
	var sb strings.Builder
	for i := 0; i < n; i++ {
		sb.WriteRune(rtRune(r))
	}
	return sb.String()
}

// rtRawBytes returns a few raw bytes that are not valid UTF-8 (or are
// raw controls), for splicing into sources, URLs and file names.
func rtRawBytes(r *rand.Rand) string {
	return []string{
		"\xff", "\xfe\xff", "\xc3", "\xc3(", "\xe6\x97", "\xed\xa0\x80", "\xf4\x90\x80\x80",
		"\xc0\xaf", "\xe0\x80\xaf", "\x80", "\xbf\xbf", "\xf0\x9f\x98", "\x01", "\x1f", "\x7f", "\x1b[0m",
	}[r.Intn(16)]
}

// rtName returns an identifier-ish name (variables, mixins).
func rtName(r *rand.Rand) string {
	switch r.Intn(5) {
	case 0:
		return "x" + rtText(r, 1+r.Intn(3))
	case 1:
		return "é" + fmt.Sprint(r.Intn(100))
	case 2:
		return "\\" + fmt.Sprintf("%x ", 0x20+r.Intn(0x10000)) + "n"
	default:
		return fmt.Sprintf("undefined-%d", r.Intn(1000))
	}
}

// rtValue returns a random SassScript expression for @error.
func rtValue(r *rand.Rand, depth int) string {
	switch k := r.Intn(16); {
	case k < 6:
		q := byte('"')
		if r.Intn(4) == 0 {
			q = '\''
		}
		return rtSassString(r, rtText(r, r.Intn(30)), q)
	case k < 7:
		return "\"" + rtText(r, r.Intn(6)) + rtRawBytes(r) + rtText(r, r.Intn(6)) + "\""
	case k < 8:
		return []string{"null", "true", "false", "()", "(a: 1, b: (c d))", "[1, 2]", "1px 2em", "1/3", "(1/3)", "#fff", "rgba(1,2,3,.5)", "1e21", "-0.0", "1.23456789012345", "percentage(1/7)", "unquote(\"\")"}[r.Intn(16)]
	case k < 10 && depth < 2:
		return rtValue(r, depth+1) + " + " + rtValue(r, depth+1)
	case k < 11 && depth < 2:
		return "\"#{" + rtValue(r, depth+1) + "}" + rtText(r, r.Intn(5)) + "\""
	case k < 12:
		return strings.Map(func(c rune) rune {
			if c == ';' || c == '{' || c == '}' || c == '"' || c == '\'' || c == '\\' || c == '\n' || c == '#' || c == '/' || c == '$' || c == '@' {
				return 'q'
			}
			return c
		}, rtText(r, 1+r.Intn(10)))
	case k < 13:
		return fmt.Sprintf("inspect((k%d: %s))", r.Intn(9), rtSassString(r, rtText(r, r.Intn(8)), '"'))
	default:
		return rtSassString(r, rtText(r, r.Intn(12)), '"')
	}
}

// rtFailing returns a random SCSS source that fails (or usually fails).
func rtFailing(r *rand.Rand) string {
	pad := strings.Repeat("\n", r.Intn(4)) + strings.Repeat(" ", r.Intn(3))
	colPad := ""
	if r.Intn(3) == 0 {
		colPad = "/*" + strings.Map(func(c rune) rune {
			if c == '*' || c == '/' {
				return 'x'
			}
			return c
		}, rtText(r, r.Intn(20))) + "*/ "
	}
	switch r.Intn(16) {
	case 0, 1, 2:
		return pad + colPad + "@error " + rtValue(r, 0) + ";"
	case 3:
		return pad + ".a {\n  b: c;\n  " + colPad + "@error " + rtValue(r, 0) + ";\n}"
	case 4:
		return "@function f($x) { @error " + rtValue(r, 0) + "; }\n" + pad + ".a { b: " + colPad + "f(1); }"
	case 5:
		return "@mixin m { @error " + rtValue(r, 0) + "; }\n" + pad + ".a { " + colPad + "@include m; }"
	case 6:
		return pad + ".a { b: " + colPad + "$" + rtName(r) + "; }"
	case 7:
		return pad + ".a { " + colPad + "@include " + rtName(r) + "; }"
	case 8:
		return pad + ".a { " + colPad + "@extend " + []string{".missing", "%missing", "a.b", ".x .y"}[r.Intn(4)] + "; }"
	case 9:
		return pad + ".a { b: " + colPad + []string{"1px + 1em", "1px * 1px + 1px", "lighten(1px, 1)", "nth((), 1)", "map-get(1, 2)", "str-slice(1, 2)", "unit(\"a\")", "rgba(1)", "mix(1, 2)", "nth(1 2, 5)", "if()", "call(nope)", "1 + (a: b)", "(a: b) + 1", "percentage(1px)"}[r.Intn(15)] + "; }"
	case 10:
		fn := []string{"str-length", "to-upper-case", "to-lower-case", "quote", "unquote", "str-slice", "str-index", "str-insert"}[r.Intn(8)]
		arg := "\"" + rtText(r, r.Intn(4)) + rtRawBytes(r) + rtText(r, r.Intn(4)) + "\""
		switch fn {
		case "str-slice":
			arg += fmt.Sprintf(", %d, %d", r.Intn(5)-1, r.Intn(5)-1)
		case "str-index":
			arg += ", \"" + rtRawBytes(r) + "\""
		case "str-insert":
			arg += ", \"x" + rtRawBytes(r) + "\", " + fmt.Sprint(r.Intn(5)-1)
		}
		return pad + ".a { b: " + fn + "(" + arg + "); }"
	case 11:
		sn := snippets[r.Intn(len(snippets))]
		if sn.name == "selectors-fn" {
			// LibSass 3.6.6 segfaults on some mutations of this one.
			return pad + "@error " + rtValue(r, 0) + ";"
		}
		return mutate(r, sn.src)
	case 12:
		sn := errorSnippetSrc(r)
		return pad + mutate(r, sn)
	case 13:
		return pad + "@import " + rtSassString(r, rtText(r, 1+r.Intn(8)), '"') + ";"
	case 14:
		return pad + ".a" + rtText(r, 1+r.Intn(6)) + " { b: c; }\n.d { @extend .a" + rtText(r, 1) + "; }"
	default:
		return pad + "@warn " + rtValue(r, 0) + ";\n@debug " + rtValue(r, 0) + ";\n.a { b: $" + rtName(r) + "; }"
	}
}

// errorSnippetSrc picks one of the checked-in error snippets.
func errorSnippetSrc(r *rand.Rand) string {
	return errorSnippets[r.Intn(len(errorSnippets))].src
}

// rtURL returns a random import URL or resolver new_url.
func rtURL(r *rand.Rand, dir string) string {
	var sb strings.Builder
	sb.WriteString(dir)
	for i, n := 0, 1+r.Intn(6); i < n; i++ {
		switch r.Intn(8) {
		case 0:
			sb.WriteString(rtRawBytes(r))
		case 1, 2:
			sb.WriteRune(rtRunes[r.Intn(len(rtRunes))])
		default:
			sb.WriteRune(rtRune(r))
		}
	}
	s := strings.ReplaceAll(sb.String(), "\x00", "")
	switch r.Intn(4) {
	case 0:
		return s + ".scss"
	case 1:
		return s + ".sass"
	default:
		return s
	}
}

// rtImportCase builds an import chain served by the table resolver with an
// error at a random level.
func rtImportCase(r *rand.Rand) (string, []tableEntry, bool) {
	depth := 1 + r.Intn(5)
	errAt := r.Intn(depth)
	var table []tableEntry
	sassBody := r.Intn(5) == 0
	for i := 0; i < depth; i++ {
		url := fmt.Sprintf("lvl%d", i)
		var body string
		if i+1 < depth {
			body = fmt.Sprintf("@import \"lvl%d\";\n", i+1)
		}
		if i == errAt {
			if sassBody {
				body += strings.Repeat("\n", r.Intn(3)) + ".e\n  x: $" + rtName(r) + "\n"
			} else {
				body += rtFailing(r)
			}
		} else if sassBody {
			body += fmt.Sprintf(".n%d\n  a: b\n", i)
		} else {
			body += fmt.Sprintf(".n%d { a: b; }\n", i)
		}
		newURL := rtURL(r, fmt.Sprintf("virtual/d%d/", r.Intn(3)))
		if !utf8.ValidString(newURL) {
			// LibSass 3.6.6 dies (in Go and Rust alike: std::terminate on an
			// uncaught utf8::invalid_utf8) when it prints a warning for, or
			// imports below, a file whose path is not UTF-8. Only single
			// imports with a plain error get such paths.
			if depth > 1 {
				newURL = strings.ToValidUTF8(newURL, "_")
			} else {
				body = strings.Repeat("\n", r.Intn(3)) + ".e { x: $" + rtName(r) + "; }\n"
				sassBody = false
			}
		}
		if sassBody {
			newURL = strings.TrimSuffix(strings.TrimSuffix(newURL, ".scss"), ".sass") + ".sass"
		}
		switch r.Intn(10) {
		case 0:
			newURL = ""
		case 1:
			newURL = url
		}
		table = append(table, tableEntry{url: url, newURL: newURL, body: body, ok: true})
	}
	return "@import \"lvl0\";\n.main { x: y; }", table, sassBody
}

// rtCases returns n random cases from seed plus the fixed deep cases.
func rtCases(seed int64, n int) []sassCase {
	r := rand.New(rand.NewSource(seed))
	var cs []sassCase
	cs = append(cs, rtDeepCases()...)
	for i := 0; i < n; i++ {
		name := fmt.Sprintf("rt/%d/%d", seed, i)
		c := sassCase{name: name, resolver: "none", precision: 8}
		c.style = r.Intn(4)
		if r.Intn(25) == 0 {
			c.style = []int{4, 5, 6, -1, 1<<32 + 3}[r.Intn(5)]
		}
		if r.Intn(8) == 0 {
			c.precision = []int{0, 1, 3, 5, 10, 15, 16, 17, 20, -1, 1 << 31}[r.Intn(11)]
		}
		switch k := r.Intn(10); {
		case k < 5:
			c.name += "/src"
			c.src = rtFailing(r)
		case k < 8:
			c.name += "/import"
			c.src, c.table, _ = rtImportCase(r)
			c.resolver = "table"
		case k < 9:
			c.name += "/unresolved"
			url := rtURL(r, "")
			// Quote the URL as a Sass string; LibSass reports it unquoted.
			c.src = strings.Repeat("\n", r.Intn(3)) + "@import " + rtSassString(r, url, '"') + ";"
			if r.Intn(2) == 0 {
				c.resolver = "table"
				c.table = []tableEntry{{url: "never", newURL: "never", body: "", ok: true}}
			}
		default:
			c.name += "/sass"
			c.sassSyntax = true
			c.src = []string{
				".a\n  b: $" + rtName(r) + "\n",
				"@error " + rtValue(r, 0) + "\n",
				".a\n  b: c\n    d: e\n  +" + rtName(r) + "\n",
				"=m\n  @error " + rtValue(r, 0) + "\n.x\n  +m\n",
				strings.Repeat("\n", r.Intn(3)) + ".a\n\tb: " + rtValue(r, 0) + " + $u\n",
			}[r.Intn(5)]
		}
		if r.Intn(6) == 0 {
			c.sm.Filename = pick(r, "o.css.map", "a\"b.map", "é.map")
			c.sm.Contents = r.Intn(2) == 0
			c.sm.EnableEmbedded = r.Intn(3) == 0
			c.sm.OmitURL = r.Intn(3) == 0
			c.sm.OutputPath = pick(r, "", "o.css", "sub/o.css")
		}
		cs = append(cs, c)
	}
	return cs
}

// rtDeepCases are inputs on which LibSass recurses deeply but which Go
// compiles on its 8 MiB cgo stack (Rust's default thread stack is 2 MiB).
func rtDeepCases() []sassCase {
	var cs []sassCase
	fn := "@function f($n) { @if $n <= 0 { @return 0; } @return f($n - 1) + 1; }\n"
	for _, d := range []int{300, 1000, 1024, 1025} {
		cs = append(cs, sassCase{name: fmt.Sprintf("rt/deep/function-%d", d), resolver: "none", style: 3,
			src: fn + fmt.Sprintf(".a { b: f(%d); }", d)})
	}
	for _, d := range []int{400, 1200} {
		var table []tableEntry
		for i := 1; i <= d; i++ {
			body := fmt.Sprintf("@import \"i%d\";\n.n%d { a: b; }", i+1, i)
			if i == d {
				body = ".leaf { a: $missing; }"
			}
			table = append(table, tableEntry{url: fmt.Sprintf("i%d", i), newURL: fmt.Sprintf("v/i%d.scss", i), body: body, ok: true})
		}
		cs = append(cs, sassCase{name: fmt.Sprintf("rt/deep/imports-%d", d), resolver: "table", style: 3, src: "@import \"i1\";", table: table})
	}
	mix := "@mixin m($n) { @if $n > 0 { .x { @include m($n - 1); } } @else { y: z; } }\n"
	cs = append(cs, sassCase{name: "rt/deep/mixin-499", resolver: "none", style: 3, src: mix + ".a { @include m(499); }"})
	cs = append(cs, sassCase{name: "rt/deep/blocks-511", resolver: "none", style: 3, src: strings.Repeat(".a{", 511) + "b:c;" + strings.Repeat("}", 511)})
	cs = append(cs, sassCase{name: "rt/deep/blocks-512", resolver: "none", style: 3, src: strings.Repeat(".a{", 512) + "b:c;" + strings.Repeat("}", 512)})
	cs = append(cs, sassCase{name: "rt/deep/parens-99", resolver: "none", style: 3, src: ".a{b:" + strings.Repeat("(", 99) + "1" + strings.Repeat(")", 99) + ";}"})
	return cs
}
