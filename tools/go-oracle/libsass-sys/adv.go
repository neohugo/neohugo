package main

// Adversarial and randomized cases (written to adv.rec.zz, read by the same
// Rust harness as cases.rec.zz). Paths in them stay inside the extracted
// site root (or are relative with at most one "../"), so that the recorded
// outputs do not depend on the depth of the temporary directory: golibsass option plumbing outside Hugo's
// values (output styles >= 4 and out-of-range ints, precision edge values
// and C.int truncation), the error path (error JSON decoding, %q quoting of
// file names with non-printable runes, messages with escapes, error JSON
// that encoding/json rejects), invalid UTF-8, NUL truncation in every
// string option, import loops, re-entrant resolvers, and fuzzed snippets,
// messages, import graphs and option combinations.

import (
	"bufio"
	"fmt"
	"math/rand"
	"os"
	"strconv"
	"strings"
	"unicode/utf8"

	"github.com/bep/golibsass/libsass"
)

const advBase = ".a { b: c; .d { e: (1/3); f: 1/3; } &:hover { g: h; } }\n" +
	"@media print { .x { y: z } }\n/* loud */\n%p { q: r; }\n.s { @extend %p; t: 1.5px + 2.25px; }\n" +
	"@font-face { font-family: X; }\n@keyframes k { from { a: b; } to { a: c; } }\n"

const advNumbers = ".a { a: (1/3); b: (2/3); c: percentage(1/7); d: (100/7); e: 1.5; f: (1e-9 * 1);\n" +
	"g: (123456789.123456789 * 1); h: round(2.5); i: (0.1 + 0.2); j: (-1/3); k: (1/3) * 3;\n" +
	"l: 0.000001 * 1; m: 1e21 * 1; n: (2/3)px; o: (5.55555555555 * 1em); p: -0.00000001 * 1; }\n"

// nonPrintables are runes for which strconv.IsPrint is false (plus a few
// printable controls for contrast).
var nonPrintables = []string{
	"\u0378", "\u061c", "\ufffe", "\U000E0001", "\u00a0", "\u2028", "\u00ad",
	"\u200b", "\ufeff", "\ue000", "\U000F0000", "\U0010FFFF", "\u3000",
	"\U00040000", "\u0085", "\u2066", "\u180e", "\U000E0020", "\U0001D173",
	"\u0600", "\u08e2", "\U000110BD", "\u1680", "\u2000", "\u205f", "\ufff9",
	"\U0001F600", "\u0300", "\u00e9", "\u65e5\u672c", "\u00ff", "\u0100", "\U00030000",
	"\U0002FFFF", "\U000E01EF", "\U000E01F0", "\u31e4", "\u31e5", "\u9fff",
}

func advCases() []sassCase {
	var cs []sassCase
	add := func(c sassCase) {
		if c.resolver == "" {
			c.resolver = "none"
		}
		cs = append(cs, c)
	}

	// 1. Output styles outside nested..compressed (uint32(style) truncation;
	// LibSass 4 = inspect, 5 = to_sass).
	for _, st := range []int{4, 5, 6, 7, 100, -1, 1 << 32, 1<<32 + 3, 1<<32 + 5, -(1 << 32) + 1} {
		add(sassCase{name: "adv/style/" + itoa(st), src: advBase, style: st, precision: 8})
	}
	// 2. Precision edge values (C.int truncation; 0 keeps LibSass's 10).
	for _, p := range []int{-1, -5, -1000, 1, 2, 9, 11, 15, 17, 21, 25, 30, 50, 100, 1000, 1 << 31, 1<<32 + 3, -(1 << 31), 1 << 40} {
		for _, st := range []int{1, 3} {
			add(sassCase{name: "adv/precision/" + itoa(p) + "/style" + itoa(st), src: advNumbers, style: st, precision: p})
		}
	}
	// 3. Error file names with non-printable runes (Error() uses %q).
	var all strings.Builder
	for i, np := range nonPrintables {
		all.WriteString(np)
		add(sassCase{name: "adv/errfile/" + itoa(i), src: "@import \"x\";", style: 3, resolver: "table",
			table: []tableEntry{{url: "x", newURL: "virtual/a" + np + "b.scss", body: "\n.a { b: $undefined; }", ok: true}}})
	}
	add(sassCase{name: "adv/errfile/all", src: "@import \"x\";", style: 3, resolver: "table",
		table: []tableEntry{{url: "x", newURL: "virtual/" + all.String() + ".scss", body: "\n\n.a { b: $undefined; }", ok: true}}})
	for i, name := range []string{"a\x01b", "a\x7fb", "a\"b", "a\\b", "a\tb", "a\x0bb", "a\x07b", "a\x08b", "a\x0cb", "a\rb", "a\x1fb", "a\x1eb", "a b", "a'b", "a%b", "a\x00b", "\xff", "a\xc3", "a\xed\xa0\x80b", "a\xf4\x90\x80\x80b"} {
		add(sassCase{name: "adv/errfile-ctrl/" + itoa(i), src: "@import \"x\";", style: 3, resolver: "table",
			table: []tableEntry{{url: "x", newURL: "virtual/" + name + ".scss", body: "\n.a { b: $undefined; }", ok: true}}})
	}
	// 4. Error messages.
	for i, m := range []string{
		`"plain"`, `"with \"quotes\" and \\ backslash"`, `"tab\9 x"`, `"us\1f x"`, `"del\7f x"`,
		"\"\u00e9 \u00fc \u65e5\u672c \\1F600\"", `"\378 \61c \fffe \E0001"`, `"line1\a line2"`, `"#{1/3} #{(1/3)}"`,
		`"cr\d lf\a crlf\d\a end"`, `"\0 nul"`, `"\feff bom"`, `"\2028 ls"`, `unquoted text`, `(a: 1, b: 2)`,
		`"x" + 1px`, `1/3`, `null`, `""`, `"\\\\\\\\"`, `"a\"b\"c"`, `"\1"`, `"\10FFFF"`, `"\110000"`, `"\D800"`,
	} {
		add(sassCase{name: "adv/errmsg/" + itoa(i), src: "\n@error " + m + ";", style: 3})
	}
	add(sassCase{name: "adv/errmsg/in-mixin", src: "@mixin m($x) { @if $x > 1 { @error \"too big: #{$x}\"; } a: $x; }\n.a { @include m(1); }\n.b { @include m(2); }", style: 1})
	add(sassCase{name: "adv/errmsg/in-function", src: "@function f($x) { @error \"f: #{$x}\"; }\n.a { b: f(\"\u00e9\"); }", style: 1})
	add(sassCase{name: "adv/errmsg/warn-then-error", src: "@warn \"w\";\n@debug \"d\";\n.a { b: 1px + 1em; }", style: 1})
	// 5. Invalid UTF-8 and odd sources.
	for i, src := range []string{
		".a { b: \"\xff\xfe\"; }", "@error \"\xff\";", "\xff { a: b; }", ".a\xc3 { b: c; }",
		".a { b: \xed\xa0\x80; }", "", " ", "\n\n\n", "/* only comment */", "// only silent",
		"\ufeff.a { b: c; }", "@charset \"UTF-8\";", "@charset \"UTF-8\";\n.a { b: \"\u00e9\"; }",
		".a { b: c }\x00.d { e: f }", "\x00", ".a{b:c}", "a{b:c", "}", "{", ";;;", "@import;",
		"@media { }", ".a { @media print { b: c; } }", "@include nope;", "$x: ;", ".a { b: #{}; }",
		"@if { }", "@each $x in { }", "@for $i from 1 through 1e9 { }", "@while false { }",
		".a { b: 1/0; c: (1/0); d: (0/0); e: (-1/0); }", ".a { b: 1e308 * 10; c: -1e308 * 10; }",
		".a { b: str-slice(\"\u65e5\u672c\u8a9e\", 2); c: str-length(\"\u00e9\"); d: to-upper-case(\"\u00e9a\"); }",
		".a { b: unquote(\"\\\"\"); c: quote(\"'\"); }", ".a { content: \"\\\\\"; }",
		strings.Repeat(".a { ", 200) + "b: c;" + strings.Repeat(" }", 200),
		".a { b: " + strings.Repeat("x", 200) + "$undefined; }",
		strings.Repeat("\n", 5000) + ".a { b: $u; }",
		".a { b: \"" + strings.Repeat("\u65e5", 100) + "\" + $u; }",
	} {
		for _, st := range []int{1, 3} {
			add(sassCase{name: "adv/src/" + itoa(i) + "/style" + itoa(st), src: src, style: st, precision: 8})
		}
	}
	// 6. NUL truncation and odd values in string options.
	smSrc := "@import \"outer\";\n.main { x: y; }"
	smTable := []tableEntry{{url: "outer", newURL: "virtual/outer.scss", body: ".outer { a: b; }", ok: true}}
	for i, sm := range []libsass.SourceMapOptions{
		{Filename: "a\x00b.map", OutputPath: "o\x00ut.css", Root: "/r\x00oot", Contents: true},
		{Filename: "\u00e9 \u00fc.map", OutputPath: "\u65e5\u672c.css", InputPath: "in put.scss", Root: "/root with space", Contents: true},
		{Filename: "a.map"},
		{Filename: "a.map", OmitURL: true},
		{Filename: "a.map", EnableEmbedded: true},
		{Filename: "a.map", EnableEmbedded: true, Contents: true},
		{Filename: "a.map", EnableEmbedded: true, OmitURL: true},
		{EnableEmbedded: true},
		{Contents: true},
		{OmitURL: true},
		{InputPath: "input.scss"},
		{OutputPath: "out.css"},
		{Root: "/r"},
		{Filename: "../up/a.map", OutputPath: "sub/dir/out.css", InputPath: "../in.scss"},
		{Filename: "@SITE@/abs/a.map", OutputPath: "@SITE@/abs/out.css", InputPath: "@SITE@/abs/in.scss", Contents: true},
		{Filename: "a\"b.map", OutputPath: "q\"uote.css", Root: "r\\oot", Contents: true},
		{Filename: "x\x01y.map", OutputPath: "c\x1ftl.css", Contents: true},
	} {
		for _, st := range []int{0, 3} {
			add(sassCase{name: "adv/sm/" + itoa(i) + "/style" + itoa(st), src: smSrc, style: st, resolver: "table", table: smTable, sm: sm})
		}
	}
	for i, inc := range [][]string{
		{"@SITE@/extra/dir1\x00junk", "@SITE@/extra/dir2"},
		{"", "@SITE@/extra/dir1", "", "@SITE@/extra/dir2", ""},
		{"extra/dir1", "extra/dir2"},
		{"@SITE@/extra/dir1:@SITE@/extra/dir2"},
		{"@SITE@/extra/dir2", "@SITE@/extra/dir1"},
		{"@SITE@/nonexistent", "@SITE@/extra/dir1/_colors.scss", "@SITE@/extra/dir2"},
	} {
		add(sassCase{name: "adv/include/" + itoa(i), src: "@import \"colors\";\n@import \"content\";\ndiv { p { color: $moo; } }", style: 1, includes: inc})
	}
	// 7. Indented syntax.
	for i, src := range []string{
		"", "\x00", ".a\n\tb: c\n", ".a\n  b: c\n\t.d\n\t\te: f\n", "@import foo, bar\n.a\n  b: c\n",
		"/* loud\n   more\n.a\n  b: c\n", "=m\n  a: b\n.x\n  +m\n", ".a\n  b: c\n    d: e\n",
		"$x: 1px\n.a\n  width: $x * 2\n  &:hover\n    width: $x / 3\n", ".a { b: c; }\n", "@media print\n  .a\n    b: c\n",
		".a\n  b: \"\u00e9\\1F600\"\n", ".a\r\n  b: c\r\n", ".a\n  b: $undefined\n",
	} {
		for _, st := range []int{1, 3} {
			add(sassCase{name: "adv/sass/" + itoa(i) + "/style" + itoa(st), src: src, style: st, precision: 8, sassSyntax: true})
		}
	}
	// 8. Resolver bridging.
	add(sassCase{name: "adv/import/loop", style: 1, resolver: "table", src: "@import \"a\";",
		table: []tableEntry{
			{url: "a", newURL: "virtual/a.scss", body: "@import \"b\";\n.a { x: y; }", ok: true},
			{url: "b", newURL: "virtual/b.scss", body: "@import \"a\";\n.b { x: y; }", ok: true},
		}})
	add(sassCase{name: "adv/import/self", style: 1, resolver: "table", src: "@import \"s\";",
		table: []tableEntry{{url: "s", newURL: "virtual/s.scss", body: "@import \"s\";", ok: true}}})
	add(sassCase{name: "adv/import/same-newurl-twice", style: 1, resolver: "table", src: "@import \"a\";\n@import \"b\";",
		table: []tableEntry{
			{url: "a", newURL: "virtual/same.scss", body: ".a { x: y; }", ok: true},
			{url: "b", newURL: "virtual/same.scss", body: ".b { x: y; }", ok: true},
		}})
	var many strings.Builder
	var manyTable []tableEntry
	for i := 0; i < 60; i++ {
		fmt.Fprintf(&many, "@import \"m%d\";\n", i)
		manyTable = append(manyTable, tableEntry{url: fmt.Sprintf("m%d", i), newURL: fmt.Sprintf("virtual/d%d/m%d.scss", i%7, i), body: fmt.Sprintf(".m%d { v: %d; }", i, i), ok: i%5 != 0})
	}
	add(sassCase{name: "adv/import/many", style: 3, resolver: "table", includes: []string{"@SITE@/assets/extra"}, src: many.String(), table: manyTable})
	add(sassCase{name: "adv/import/nul-newurl", style: 1, resolver: "table", src: "@import \"n\";\n.x { y: $v; }",
		table: []tableEntry{{url: "n", newURL: "virt\x00ual/n.scss", body: "$v: 1;", ok: true}}})
	add(sassCase{name: "adv/import/nul-newurl-error", style: 1, resolver: "table", src: "@import \"n\";",
		table: []tableEntry{{url: "n", newURL: "virt\x00ual/n.scss", body: ".x { y: $v; }", ok: true}}})
	add(sassCase{name: "adv/import/body-leading-nul", style: 1, resolver: "table", src: "@import \"n\";\n.x { y: z; }",
		table: []tableEntry{{url: "n", newURL: "@SITE@/assets/extra/_partial.scss", body: "\x00.ignored { a: b; }", ok: true}}})
	add(sassCase{name: "adv/import/non-utf8-url", style: 1, resolver: "table", src: "@import \"\xff\xfe\";\n@import \"ok\";",
		table: []tableEntry{{url: "\xff\xfe", newURL: "virtual/\xff.scss", body: ".bad { a: b; }", ok: true}, {url: "ok", newURL: "ok", body: ".ok { a: b; }", ok: true}}})
	add(sassCase{name: "adv/import/relative-newurl-disk", style: 1, resolver: "table", src: "@import \"c\";\n.x { y: $moo; }",
		table: []tableEntry{{url: "c", newURL: "extra/dir1/_colors.scss", ok: true}}})
	add(sassCase{name: "adv/import/dir-newurl", style: 1, resolver: "table", src: "@import \"d\";",
		table: []tableEntry{{url: "d", newURL: "@SITE@/extra/dir1", ok: true}}})
	add(sassCase{name: "adv/import/newurl-without-ext-disk", style: 1, resolver: "table", src: "@import \"d\";",
		table: []tableEntry{{url: "d", newURL: "@SITE@/extra/dir1/_colors", ok: true}}})
	add(sassCase{name: "adv/import/url-with-interp", style: 1, resolver: "table", src: "$n: \"q\";\n@import \"#{$n}\";\n@import \"p#{1}\";",
		table: []tableEntry{{url: "q", newURL: "q", body: ".q { a: b; }", ok: true}}})
	add(sassCase{name: "adv/import/deep-chain-error", style: 1, resolver: "table", src: "@import \"l1\";",
		table: []tableEntry{
			{url: "l1", newURL: "virtual/l1/\u00e9.scss", body: "@import \"l2\";", ok: true},
			{url: "l2", newURL: "virtual/l2/\u0378.scss", body: "@import \"l3\";", ok: true},
			{url: "l3", newURL: "virtual/l3/x.scss", body: "\n\n  .z { a: $missing; }", ok: true},
		}})
	add(sassCase{name: "adv/import/sass-body-error", style: 1, resolver: "table", src: "@import \"s\";",
		table: []tableEntry{{url: "s", newURL: "virtual/s.sass", body: ".a\n  b: $missing\n", ok: true}}})
	add(sassCase{name: "adv/import/css-newurl", style: 1, resolver: "table", src: "@import \"c\";",
		table: []tableEntry{{url: "c", newURL: "virtual/c.css", body: ".c { a: b; }", ok: true}}})
	// Re-entrant resolver: runs a nested transpile for each import.
	add(sassCase{name: "adv/import/nested-transpile", style: 1, resolver: "nested",
		src: "@import \"a\";\n@import \"b\";\n.m { x: y; }",
		table: []tableEntry{
			{url: "a", newURL: "virtual/a.scss", body: "@import \"inner\";\n.a { w: $inner * 2; }", ok: true},
			{url: "b", newURL: "virtual/b.scss", body: ".b { w: $undefined; }", ok: true},
		}})

	return cs
}

// fuzzSass generates n randomized cases from seed.
func fuzzSass(seed int64, n int) []sassCase {
	r := rand.New(rand.NewSource(seed))
	var cs []sassCase
	for i := 0; i < n; i++ {
		name := fmt.Sprintf("fuzz/%d", i)
		style := r.Intn(4)
		if r.Intn(20) == 0 {
			style = 4 + r.Intn(3)
		}
		precision := []int{0, 8, 8, 8, r.Intn(17), r.Intn(25) - 3}[r.Intn(6)]
		switch k := r.Intn(10); {
		case k < 4:
			sn := snippets[r.Intn(len(snippets))]
			src := mutate(r, sn.src)
			if sn.name == "selectors-fn" {
				// LibSass 3.6.6 segfaults (in Go and Rust alike) on some
				// malformed selector-function arguments, e.g.
				// simple-selectors("}a.b#c"); keep this snippet intact.
				src = sn.src
			}
			cs = append(cs, sassCase{name: name + "/mutate", src: src, style: style, precision: precision, resolver: "none"})
		case k < 6:
			cs = append(cs, sassCase{name: name + "/errmsg", src: randErrorSrc(r), style: style, precision: precision, resolver: "none"})
		case k < 9:
			src, table := randImportGraph(r)
			cs = append(cs, sassCase{name: name + "/graph", src: src, style: style, precision: precision, resolver: "table", table: table,
				includes: []string{"@SITE@/assets/extra"}})
		default:
			sm := libsass.SourceMapOptions{
				Filename:       pick(r, "", "", "o.css.map", "sub/o.css.map", "\u00e9.map", "../x.map", "a\"b.map"),
				Root:           pick(r, "", "/root", "rel", "@SITE@", "/\u00e9"),
				InputPath:      pick(r, "", "in.scss", "assets/x.scss", "@SITE@/abs/in.scss", "\u00e9.scss"),
				OutputPath:     pick(r, "", "o.css", "sub/o.css", "../o.css", "\u00e9.css"),
				Contents:       r.Intn(2) == 0,
				OmitURL:        r.Intn(3) == 0,
				EnableEmbedded: r.Intn(3) == 0,
			}
			if sm.EnableEmbedded && sm.Root == "@SITE@" {
				// The embedded (base64) map cannot have the root replaced.
				sm.Root = "rel"
			}
			src, table := randImportGraph(r)
			cs = append(cs, sassCase{name: name + "/opts", src: src, style: style, precision: precision, resolver: "table", table: table, sm: sm,
				sassSyntax: false, includes: []string{"@SITE@/assets/extra", pick(r, "", "@SITE@/extra/dir1", "rel")}})
		}
	}
	return cs
}

func pick(r *rand.Rand, xs ...string) string { return xs[r.Intn(len(xs))] }

var mutTokens = []string{
	"{", "}", ";", ":", "$x", "#{", "}", "@include m", "&", "\"", "'", "/*", "*/", "//", "\n",
	"1.23456789e-3", "%", "px", "(1/3)", "@media print {", "@extend .a;", "!important", "!default",
	"\u00e9", "\u0378", "\U0001F600", "\\", "\\1F600 ", "@error \"x\";", "@warn \"w\";", ",", ">", "+", "~",
	"@import \"x\";", "url(", ")", "-", "*", "/", "0", "10", ".5", "#fff", "rgba(0,0,0,.5)", "null",
}

func mutate(r *rand.Rand, s string) string {
	b := []byte(s)
	for m := r.Intn(4); m >= 0; m-- {
		if len(b) == 0 {
			b = append(b, mutTokens[r.Intn(len(mutTokens))]...)
			continue
		}
		pos := r.Intn(len(b) + 1)
		switch r.Intn(4) {
		case 0: // delete a range
			end := pos + r.Intn(8)
			if end > len(b) {
				end = len(b)
			}
			b = append(b[:pos:pos], b[end:]...)
		case 1: // insert a token
			t := mutTokens[r.Intn(len(mutTokens))]
			b = append(b[:pos:pos], append([]byte(t), b[pos:]...)...)
		case 2: // duplicate a range
			end := pos + r.Intn(30)
			if end > len(b) {
				end = len(b)
			}
			dup := append([]byte(nil), b[pos:end]...)
			b = append(b[:end:end], append(dup, b[end:]...)...)
		default: // random byte
			b = append(b[:pos:pos], append([]byte{byte(r.Intn(256))}, b[pos:]...)...)
		}
	}
	return string(b)
}

// randRune returns a rune from an interesting class.
func randRune(r *rand.Rand) rune {
	switch r.Intn(8) {
	case 0, 1:
		return rune(0x20 + r.Intn(0x5f))
	case 2:
		return rune(r.Intn(0x20))
	case 3:
		return rune(0x80 + r.Intn(0x180))
	case 4:
		for {
			c := rune(0x100 + r.Intn(0xFF00))
			if c < 0xD800 || c > 0xDFFF {
				return c
			}
		}
	case 5:
		return rune(0x10000 + r.Intn(0x100000))
	case 6:
		np, _ := utf8.DecodeRuneInString(nonPrintables[r.Intn(len(nonPrintables))])
		return np
	default:
		return []rune{'"', '\\', '\'', '#', '{', '}', '\n', '\r', '\t', 0x7f, 0x1f}[r.Intn(11)]
	}
}

func randText(r *rand.Rand, n int) string {
	var sb strings.Builder
	for i := 0; i < n; i++ {
		sb.WriteRune(randRune(r))
	}
	return sb.String()
}

// sassQuote writes s as a double-quoted Sass string: quotes, backslashes and
// newlines are escaped; with raw, other bytes go in verbatim, otherwise
// some runes are written as Sass hex escapes.
func sassQuote(r *rand.Rand, s string) string {
	var sb strings.Builder
	sb.WriteByte('"')
	for _, c := range s {
		switch {
		case c == '"' || c == '\\':
			sb.WriteByte('\\')
			sb.WriteRune(c)
		case c == '\n' || c == '\r':
			fmt.Fprintf(&sb, "\\%x ", c)
		case c == '#' && r.Intn(2) == 0:
			sb.WriteString("\\#")
		case r.Intn(6) == 0:
			fmt.Fprintf(&sb, "\\%x ", c)
		default:
			sb.WriteRune(c)
		}
	}
	sb.WriteByte('"')
	return sb.String()
}

func randErrorSrc(r *rand.Rand) string {
	msg := sassQuote(r, randText(r, r.Intn(40)))
	switch r.Intn(4) {
	case 0:
		return "@error " + msg + ";"
	case 1:
		return strings.Repeat("\n", r.Intn(5)) + ".a {\n  b: c;\n  @error " + msg + ";\n}"
	case 2:
		return "@function f($x) { @error " + msg + " + $x; }\n.a { b: f(" + itoa(r.Intn(100)) + "); }"
	default:
		return "@warn " + msg + ";\n.a { b: " + msg + "; c: $undefined-" + itoa(r.Intn(10)) + "; }"
	}
}

// randImportGraph builds a random tree of virtual files served by the table
// resolver; some nodes are left to LibSass (resolved=false or path-only),
// and one may contain an error.
func randImportGraph(r *rand.Rand) (string, []tableEntry) {
	nodes := 1 + r.Intn(6)
	var table []tableEntry
	errAt := -1
	if r.Intn(4) == 0 {
		errAt = r.Intn(nodes)
	}
	urls := make([]string, nodes)
	for i := range urls {
		urls[i] = fmt.Sprintf("n%d", i)
		if r.Intn(4) == 0 {
			urls[i] += "-" + strings.Map(func(c rune) rune {
				if c == '"' || c == '\\' || c == '\n' || c == '\r' || c < 0x20 || c == 0x7f || c == '#' || c == '\'' {
					return 'q'
				}
				return c
			}, randText(r, 1+r.Intn(4)))
		}
	}
	children := make([][]int, nodes)
	for i := 1; i < nodes; i++ {
		p := r.Intn(i)
		children[p] = append(children[p], i)
	}
	body := func(i int) string {
		var sb strings.Builder
		for _, c := range children[i] {
			// urls never contain '"', '\\' or controls: quote them verbatim.
			fmt.Fprintf(&sb, "@import \"%s\";\n", urls[c])
		}
		fmt.Fprintf(&sb, "$v%d: %dpx;\n.n%d { w: $v%d * %d; }\n", i, r.Intn(50), i, i, 1+r.Intn(3))
		if i == errAt {
			sb.WriteString(strings.Repeat("\n", r.Intn(3)) + ".err { x: $undefined; }\n")
		}
		return sb.String()
	}
	for i := 1; i < nodes; i++ {
		e := tableEntry{url: urls[i], ok: true}
		switch k := r.Intn(20); {
		case k < 13:
			e.newURL = "virtual/" + fmt.Sprintf("d%d/", r.Intn(3)) + strings.Map(func(c rune) rune {
				if c < 0x20 || c == 0x7f {
					return '_'
				}
				return c
			}, randText(r, 1+r.Intn(6))) + ".scss"
			e.body = body(i)
		case k < 15:
			e.newURL = ""
			e.body = body(i)
		case k < 16:
			// LibSass resolves it (include paths): usually not found.
			e.ok = false
			e.newURL = "ignored"
			e.body = ".ignored { a: b; }"
		case k < 18:
			e.newURL = "@SITE@/assets/extra/_partial.scss"
		default:
			e.newURL = urls[i]
			e.body = body(i)
		}
		table = append(table, e)
	}
	return body(0), table
}

// writeQuoteTable writes, for every block of 256 code points (surrogates
// skipped), the FNV-1a-64 hash of strconv.Quote of the block's runes, plus a
// few whole strings, so crates/libsass-sys can check its %q (Error()) port
// against Go's strconv.IsPrint tables exhaustively.
func writeQuoteTable(path string) {
	f, err := os.Create(path)
	if err != nil {
		panic(err)
	}
	w := bufio.NewWriter(f)
	_, _ = fmt.Fprintln(w, "#first_rune\tquoted_len\tquoted_fnv64")
	for lo := rune(0); lo < 0x110000; lo += 256 {
		var sb strings.Builder
		for c := lo; c < lo+256; c++ {
			if c >= 0xD800 && c <= 0xDFFF {
				continue
			}
			sb.WriteRune(c)
		}
		q := strconv.Quote(sb.String())
		h := uint64(0xcbf29ce484222325)
		for i := 0; i < len(q); i++ {
			h ^= uint64(q[i])
			h *= 0x100000001b3
		}
		_, _ = fmt.Fprintf(w, "%d\t%d\t%016x\n", lo, len(q), h)
	}
	if err := w.Flush(); err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
}
