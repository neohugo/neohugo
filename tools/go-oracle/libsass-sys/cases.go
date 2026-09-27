package main

import (
	"github.com/bep/golibsass/libsass"
)

// tableEntry is one resolver answer for the "table" resolver kind.
type tableEntry struct {
	url, newURL, body string
	ok                bool
}

// sassCase is one transpile; strings may contain the @SITE@ placeholder.
type sassCase struct {
	name        string
	src         string
	style       int
	precision   int
	includes    []string
	sassSyntax  bool
	sm          libsass.SourceMapOptions
	resolver    string // none | hugo | table | echo
	hugoBaseDir string
	hugoVars    string
	table       []tableEntry
}

var styles = []int{0, 1, 2, 3}

// snippets exercise LibSass features; each runs in every output style with
// Hugo's precision (8) and no resolver.
var snippets = []struct{ name, src string }{
	{"vars-nesting", "$c: #336699;\n.a { color: $c; .b { color: darken($c, 10%); &:hover { color: lighten($c, 20%); } } &-suffix { x: y; } }"},
	{"parent-selector", ".btn { &.active { a: b; } & + & { c: d; } .x & { e: f; } &::before { content: ''; } }"},
	{"extend-placeholder", "%msg { border: 1px solid #ccc; padding: 10px; }\n.ok { @extend %msg; color: green; }\n.err { @extend %msg; color: red; }\n.warn { @extend .err; color: orange; }"},
	{"media-nesting", ".a { width: 100%; @media (min-width: 768px) { width: 50%; @media (orientation: landscape) { width: 25%; } } }\n@media screen { .b { c: d; } }"},
	{"supports-fontface-keyframes", "@supports (display: grid) { .g { display: grid; } }\n@font-face { font-family: 'X'; src: url(x.woff2) format('woff2'); }\n@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }\n@-webkit-keyframes pulse { 0% { opacity: 1 } 50% { opacity: .5 } 100% { opacity: 1 } }"},
	{"comments", "/* normal */\n// silent\n/*! loud */\n.a { /* inner */ b: c; // trailing\n}\n"},
	{"charset-unicode", "@charset \"UTF-8\";\n.a:before { content: \"→ ✓ ü\"; }\n.b { font-family: \"Ärger\"; }\n.c:after { content: \"\\f101\"; }"},
	{"unicode-no-charset", ".a:before { content: \"日本語\"; }"},
	{"each-map-list", "$m: (primary: #007bff, secondary: #6c757d, success: #28a745);\n@each $k, $v in $m { .text-#{$k} { color: $v; } }\n@each $s in small, medium, large { .size-#{$s} { font-size: $s; } }"},
	{"for-while", "@for $i from 1 through 4 { .m-#{$i} { margin: #{$i * 0.25}rem; } }\n@for $i from 1 to 3 { .p-#{$i} { padding: $i * 5px; } }\n$i: 6; @while $i > 0 { .w-#{$i} { width: 10px * $i; } $i: $i - 2; }"},
	{"if-else", "@function f($x) { @if $x > 10 { @return big; } @else if $x > 5 { @return mid; } @else { @return small; } }\n.a { a: f(11); b: f(6); c: f(1); d: if(true, yes, no); e: if(null, yes, no); }"},
	{"mixins", "@mixin m($a, $b: 10px, $rest...) { width: $a; height: $b; margin: $rest; @content; }\n.a { @include m(1px) { color: red; } }\n.b { @include m(1px, 2px, 3px, 4px); }\n@mixin bp($w) { @media (min-width: $w) { @content; } }\n.c { @include bp(100px) { d: e; } }"},
	{"functions-math", ".a { a: percentage(0.5); b: round(1.5); c: ceil(1.2); d: floor(1.8); e: abs(-3px); f: min(1px, 2px, 0.5px); g: max(3em, 2em); h: 10px / 3; i: (10px / 3); j: 1 + 2 * 3; k: -(2px); }"},
	{"units", ".a { a: 1in + 1cm; b: 10px * 2; c: 2em / 1em; d: 1turn + 90deg; e: 100ms + 1s; f: 3px * 2px / 1px; g: unit(1px); h: unitless(1); i: comparable(1px, 1in); }"},
	{"slash-ambiguity", ".a { font: 12px/1.5 sans-serif; b: 12px/1.5; $x: 12px; c: $x/2; d: (12px/2); e: 1/2; f: #{12px}/#{1.5}; }"},
	{"precision-numbers", ".a { a: 1/3; b: 2/3; c: 1/7; d: 100/7; e: 0.1 + 0.2; f: 1.23456789012345; g: 12345678.9; h: 1e-7 * 1; i: -1/3; j: 3.14159265358979 * 2; k: 1000000000000 * 1000; l: 0.000000005; m: 99.999999999; n: 0.5; o: -0.5; p: 1.0; q: 1e3; r: percentage(1/3); s: 5.0000000049; t: 5.000000005; }"},
	{"strings", ".a { a: quote(foo); b: unquote(\"bar\"); c: str-length(\"hello\"); d: str-index(\"abcd\", \"c\"); e: str-insert(\"abcd\", \"X\", 2); f: str-slice(\"abcdef\", 2, 4); g: to-upper-case(\"abc\"); h: to-lower-case(\"ABC\"); i: \"a\" + b; j: a + \"b\"; k: \"#{1 + 1}\"; }"},
	{"colors", ".a { a: mix(#f00, #00f); b: rgba(#000, 0.5); c: adjust-hue(#f00, 120deg); d: saturate(#888, 20%); e: desaturate(#f00, 50%); f: grayscale(#f80); g: complement(#f80); h: invert(#123456); i: transparentize(#fff, 0.25); j: opacify(rgba(0,0,0,.2), .3); k: ie-hex-str(#abc); l: adjust-color(#f00, $blue: 20); m: scale-color(#f00, $lightness: 50%); n: change-color(#f00, $alpha: .5); o: red(#123456); p: hue(#f80); q: saturation(#f80); r: lightness(#f80); s: alpha(rgba(0,0,0,.3)); t: rgb(10%, 20%, 30%); u: hsl(120, 50%, 50%); v: hsla(120, 50%, 50%, .5); }"},
	{"color-output", ".a { a: #FFFFFF; b: #ff0000; c: white; d: rgba(0,0,0,0); e: transparent; f: #aabbcc; g: #abcdef; h: red; i: blue + 1; j: #fff - #111; k: rgba(255, 0, 0, 1); l: #FFF; m: fuchsia; n: #f0f; }"},
	{"maps-lists", "$m: (a: 1, b: 2);\n$m2: map-merge($m, (c: 3));\n$l: 1px 2px 3px;\n.a { a: map-get($m2, c); b: map-keys($m2); c: map-values($m2); d: map-has-key($m, a); e: nth($l, 2); f: length($l); g: join($l, (4px 5px)); h: append($l, 6px, comma); i: index($l, 3px); j: zip(a b, 1 2); k: list-separator((a, b)); l: set-nth($l, 1, 0); m: inspect(map-remove($m2, a)); }"},
	{"introspection", "@mixin mm {} $v: 1;\n.a { a: type-of(1px); b: type-of(\"s\"); c: type-of(#fff); d: type-of((a: 1)); e: variable-exists(v); f: mixin-exists(mm); g: function-exists(lighten); h: feature-exists(global-variable-shadowing); i: inspect((a: 1, b: (2 3))); j: call(get-function(lighten), #000, 10%); }"},
	{"selectors-fn", ".a { a: selector-nest(\".x\", \"&:hover\"); b: selector-append(\".x\", \".y\"); c: selector-replace(\".a .b\", \".b\", \".c\"); d: selector-unify(\"a.x\", \".y\"); e: is-superselector(\".a\", \".a.b\"); f: simple-selectors(\"a.b#c\"); g: selector-parse(\".a .b, .c\"); }"},
	{"at-root-global", "$g: 1 !default; $g: 2 !default;\n.a { $l: 3; $g: 4 !global; b: $l; .c { @at-root .d { e: f; } } @at-root { .x { y: z; } } }\n.z { g: $g; }"},
	{"selectors-complex", "a[href^='http'], a[target=\"_blank\"] { b: c; }\nul > li + li ~ li { d: e; }\n.x:not(.y):nth-child(2n+1)::after { f: g; }\ninput[type=checkbox]:checked + label { h: i; }\n.a, .b { .c, .d { j: k; } }"},
	{"important-calc-var", ".a { a: b !important; width: calc(100% - #{10px * 2}); c: var(--x, 1px); --custom: #{1 + 1}; d: calc(1px + 2px); e: url(foo.png); f: url(\"bar.png\"); g: url(#{\"baz\"}.png); }"},
	{"empty-rules", ".empty {}\n.a { .b {} c: d; }\n.nested { .e { } }\n@media print { .f {} }"},
	{"media-query-merge", "@media screen and (min-width: 100px) { .a { @media (max-width: 200px) { b: c; } } }\n@media not print { .d { e: f; } }\n@media only screen and (-webkit-min-device-pixel-ratio: 2) { .g { h: i; } }"},
	{"interpolation", "$p: margin; $s: top; .a { #{$p}-#{$s}: 1px; } .b-#{$s} { c: #{1 + 2}px; } @media #{'(min-width: 10px)'} { .c { d: e; } } .d { e: \"#{a}#{b}\"; }"},
	{"css-import", "@import \"foo.css\";\n@import url(bar.css);\n@import \"http://example.com/x.css\";\n@import url(\"//cdn/y.css\") screen;\n.a { b: c; }"},
	{"long-selector-list", ".a1, .a2, .a3, .a4, .a5, .a6, .a7, .a8, .a9, .a10, .a11, .a12, .a13, .a14, .a15, .a16, .a17, .a18, .a19, .a20, .a21, .a22, .a23, .a24 { b: c; }"},
	{"numbers-compressed", ".a { a: 0.5em; b: -0.5em; c: 0px; d: 00.10; e: 1.50; f: +.5; g: 10.0%; h: 0.0; i: 1e2px; j: 1.5e-3; }"},
	{"division-by-zero", ".a { a: 1/0; b: (1/0); c: -1/0; d: 0/0; e: (0/0); f: (1px/0); }"},
	{"bootstrap-like", "$enable-shadows: false; $spacers: (0: 0, 1: .25rem, 2: .5rem);\n@function tint($c, $w) { @return mix(white, $c, $w); }\n@each $k, $v in $spacers { .m-#{$k} { margin: $v !important; } }\n.btn { background: tint(#007bff, 15%); @if $enable-shadows { box-shadow: 0 1px 1px rgba(0,0,0,.075); } }"},
	{"extend-media", "@media print { .a { color: red; } .b { @extend .a; } }\n.c { d: e; } .f { @extend .c; g: h; }"},
	{"nested-props", ".a { font: { family: serif; size: 12px; weight: bold; } border: 1px solid { left: 0; } }"},
	{"string-escapes", ".a { a: \"\\\"quoted\\\"\"; b: 'single \\'q\\''; c: \"\\\\\"; d: \"a\\\nb\"; e: \"\\26\"; f: \"\\1F600\"; }"},
	{"unary-and-null", "$n: null; .a { a: $n; b: -(1px); c: +1px; d: - 1px; e: not true; f: 1 == 1.0; g: 1px == 1; h: (1 < 2) and (2 < 3); i: null == false; }"},
	{"keyframes-percent", "@keyframes k { 0%, 50% { a: b; } 100% { c: d; } }\n.x { animation: k 1s infinite; }"},
	{"attribute-and-escapes", ".\\31 0 { a: b; } .a\\:b { c: d; } [data-x=\"1\"] { e: f; } .\\@media { g: h; }"},
	{"warn-debug", "@warn \"a warning\"; @debug \"dbg\"; .a { b: c; }"},
	{"sourcemap-ish-comments", "/*# sourceMappingURL=foo.map */\n.a { b: c; }"},
	{"deep-nesting", ".a { .b { .c { .d { .e { .f { g: h; } } } } } }"},
	{"trailing-newlines", ".a { b: c; }\n\n\n\n.d { e: f; }\n\n"},
	{"crlf-input", ".a {\r\n  b: c;\r\n}\r\n.d { e: f; }\r\n"},
	{"tabs", ".a {\n\tb: c;\n\t.d {\n\t\te: f;\n\t}\n}"},
}

// errorSnippets fail to compile (compressed style).
var errorSnippets = []struct{ name, src string }{
	{"undefined-variable", "\n\ndiv { color: $blue; }"},
	{"unclosed-block", ".a { b: c;"},
	{"invalid-css", ".a { b: c; } }"},
	{"undefined-mixin", ".a { @include nope; }"},
	{"error-directive", "@error \"custom failure #{1 + 1}\";"},
	{"import-not-found", "@import \"does-not-exist\";"},
	{"use-rule", "@use \"sass:math\";\n.a { b: math.div(1, 2); }"},
	{"bad-function-arg", ".a { b: lighten(red, foo); }"},
	{"incompatible-units", ".a { b: 1px + 1em; }"},
	{"unicode-in-error", "$x: \"é\";\n.a { b: nope-#{$x}(); c: $undefined-é; }"},
	{"extend-missing", ".a { @extend .missing; }"},
	{"nul-byte-truncates", ".a { b: c; }\x00 this is never seen }}}"},
}

// sassSyntaxSnippets are indented-syntax sources (SassSyntax: true).
var sassSyntaxSnippets = []struct{ name, src string }{
	{"basic", "$color: #ccc\ndiv\n  p\n    color: $color\n"},
	{"mixin", "=m($x)\n  width: $x\n.a\n  +m(10px)\n  &:hover\n    color: red\n"},
	{"comments-import", "// silent\n/* loud\n.a\n  b: c\n  .d\n    e: f\n"},
	{"multiline-selector", ".a,\n.b\n  c: d\n"},
}

func allCases() []sassCase {
	var cs []sassCase
	siteIncludes := []string{"@SITE@/assets/scss", "@SITE@/node_modules", "@SITE@/assets/scss"}
	website := "@WEBSITE@" // replaced by the entry file content

	// 1. The seeksnack toCSS call (tocss.go + client_extended.go).
	for _, st := range styles {
		cs = append(cs, sassCase{name: "site/hugo/style" + itoa(st), src: website, style: st, precision: 8, includes: siteIncludes, resolver: "hugo", hugoBaseDir: "scss"})
	}
	for _, st := range []int{1, 3} {
		cs = append(cs, sassCase{name: "site/noresolver/style" + itoa(st), src: website, style: st, precision: 8, includes: siteIncludes, resolver: "none"})
	}
	for _, p := range []int{0, 3, 5, 10} {
		cs = append(cs, sassCase{name: "site/hugo/precision" + itoa(p), src: website, style: 3, precision: p, includes: siteIncludes, resolver: "hugo", hugoBaseDir: "scss"})
	}
	// Hugo's enableSourceMap settings.
	for _, st := range []int{1, 3} {
		cs = append(cs, sassCase{
			name: "site/hugo/sourcemap/style" + itoa(st), src: website, style: st, precision: 8, includes: siteIncludes,
			resolver: "hugo", hugoBaseDir: "scss",
			sm: libsass.SourceMapOptions{Filename: "website.css.map", Root: "@SITE@", OutputPath: "website.css", Contents: true},
		})
	}
	cs = append(cs, sassCase{
		name: "site/hugo/sourcemap-embedded", src: website, style: 0, precision: 8, includes: siteIncludes,
		resolver: "hugo", hugoBaseDir: "scss",
		// The embedded map is base64: keep absolute (@SITE@) paths out of it.
		sm: libsass.SourceMapOptions{Filename: "website.css.map", Root: "/site-root", OutputPath: "website.css", InputPath: "assets/scss/website.scss", Contents: false, EnableEmbedded: true},
	})
	cs = append(cs, sassCase{
		name: "site/hugo/sourcemap-omiturl", src: website, style: 3, precision: 8, includes: siteIncludes,
		resolver: "hugo", hugoBaseDir: "scss",
		sm: libsass.SourceMapOptions{Filename: "out/website.css.map", OutputPath: "out/website.css", OmitURL: true},
	})

	// 2. golibsass transpiler_test.go cases.
	cs = append(cs,
		sassCase{name: "golibsass/compressed", src: "div { color: #ccc; }", style: 3, resolver: "none"},
		sassCase{name: "golibsass/invalid", src: "div { color: $white; }", style: 3, resolver: "none"},
		sassCase{name: "golibsass/import-not-found", src: "@import \"foo\"", style: 3, resolver: "none"},
		sassCase{name: "golibsass/sass-syntax", src: "$color: #ccc\ndiv { p { color: $color; } }", style: 3, sassSyntax: true, resolver: "none"},
		sassCase{name: "golibsass/import-resolver", src: "@import \"colors\";\ndiv { p { color: $white; } }", resolver: "echo"},
		sassCase{name: "golibsass/precision", src: "div { width: percentage(1 / 3); }", precision: 3, resolver: "none"},
		sassCase{name: "golibsass/sourcemap", src: "div { p { color: blue; } }", resolver: "none",
			sm: libsass.SourceMapOptions{Contents: true, Filename: "source.map", OutputPath: "outout.css", InputPath: "input.scss", Root: "/my/root"}},
		sassCase{name: "golibsass/include-paths", src: "\n@import \"colors\";\n@import \"content\";\ndiv { p { color: $moo; } }", style: 3,
			includes: []string{"@SITE@/extra/dir1", "@SITE@/extra/dir2"}, resolver: "table"},
	)

	// 3. Feature snippets.
	for _, s := range snippets {
		for _, st := range styles {
			cs = append(cs, sassCase{name: "snippet/" + s.name + "/style" + itoa(st), src: s.src, style: st, precision: 8, resolver: "none"})
		}
	}
	for _, p := range []int{0, 1, 3, 5, 10, 12, 16, 20} {
		cs = append(cs, sassCase{name: "precision/" + itoa(p), src: snippetSrc("precision-numbers"), style: 1, precision: p, resolver: "none"})
	}
	for _, s := range errorSnippets {
		cs = append(cs, sassCase{name: "error/" + s.name, src: s.src, style: 3, precision: 8, resolver: "none"})
	}
	for _, s := range sassSyntaxSnippets {
		for _, st := range []int{1, 3} {
			cs = append(cs, sassCase{name: "sass/" + s.name + "/style" + itoa(st), src: s.src, style: st, precision: 8, sassSyntax: true, resolver: "none"})
		}
	}

	// 4. Import resolver bridging (BridgeImport) semantics.
	cs = append(cs,
		sassCase{name: "import/body-nested", style: 1, resolver: "table",
			src: "@import \"outer\";\n.main { x: $outer + $inner; }",
			table: []tableEntry{
				{url: "outer", newURL: "virtual/outer-new", body: "@import \"inner\";\n$outer: 1;\n.outer { a: b; }", ok: true},
				{url: "inner", newURL: "virtual/sub/inner-new.scss", body: "$inner: 2;\n.inner { c: d; }", ok: true},
			}},
		sassCase{name: "import/path-only", style: 1, resolver: "table", includes: []string{"@SITE@/assets/extra"},
			src: "@import \"partial\";\n@import \"relative\";\n.m { w: $p; }",
			table: []tableEntry{
				{url: "partial", newURL: "@SITE@/assets/extra/_partial.scss", ok: true},
			}},
		sassCase{name: "import/unresolved-then-nested", style: 1, resolver: "table", includes: []string{"@SITE@/assets/extra"},
			src: "@import \"nested/deep\";\n.m { n: o; }",
			table: []tableEntry{
				{url: "../partial", newURL: "@SITE@/assets/extra/_partial.scss", ok: true},
			}},
		sassCase{name: "import/nul-in-body", style: 1, resolver: "table",
			src:   "@import \"n\";",
			table: []tableEntry{{url: "n", newURL: "n", body: ".before { a: b; }\x00.after { c: d; }", ok: true}}},
		sassCase{name: "import/empty-newurl-with-body", style: 1, resolver: "table",
			src:   "@import \"e\";\n.m { x: $e; }",
			table: []tableEntry{{url: "e", newURL: "", body: "$e: 5;", ok: true}}},
		sassCase{name: "import/resolved-false-with-values", style: 1, resolver: "table", includes: []string{"@SITE@/assets/extra"},
			src:   "@import \"partial\";",
			table: []tableEntry{{url: "partial", newURL: "ignored", body: ".ignored { a: b; }", ok: false}}},
		sassCase{name: "import/missing-file", style: 1, resolver: "table",
			src:   "@import \"gone\";",
			table: []tableEntry{{url: "gone", newURL: "@SITE@/assets/extra/_nope.scss", ok: true}}},
		sassCase{name: "import/unicode-and-spaces", style: 1, resolver: "table",
			src: "@import \"dir with space/ünï\";\n@import \"a b\", \"c\";",
			table: []tableEntry{
				{url: "dir with space/ünï", newURL: "x/ünï", body: ".u { a: \"ü\"; }", ok: true},
				{url: "a b", newURL: "ab", body: ".ab { b: c; }", ok: true},
				{url: "c", newURL: "c", body: ".c { d: e; }", ok: true},
			}},
		sassCase{name: "import/css-and-url", style: 1, resolver: "table",
			src: "@import \"foo.css\";\n@import url(bar);\n@import \"http://x/y\";\n@import \"plain\";\n@import \"q\" screen;",
			table: []tableEntry{
				{url: "foo.css", newURL: "foo.css", body: ".foo { a: b; }", ok: true},
				{url: "plain", newURL: "@SITE@/assets/extra/plain.css", ok: true},
			}},
		sassCase{name: "import/inside-rule", style: 1, resolver: "table",
			src:   ".wrap { @import \"inner\"; }",
			table: []tableEntry{{url: "inner", newURL: "inner", body: ".x { y: z; } a { b: c; }", ok: true}}},
		sassCase{name: "import/indented-body", style: 1, resolver: "table",
			src:   "@import \"ind\";",
			table: []tableEntry{{url: "ind", newURL: "ind.sass", body: ".ind\n  color: red\n", ok: true}}},
		sassCase{name: "import/indented-file", style: 1, resolver: "table",
			src:   "@import \"indented\";",
			table: []tableEntry{{url: "indented", newURL: "@SITE@/assets/extra/indented.sass", ok: true}}},
		sassCase{name: "import/body-error", style: 1, resolver: "table",
			src:   "@import \"broken\";",
			table: []tableEntry{{url: "broken", newURL: "virtual/broken.scss", body: "\n.a { b: $undefined; }", ok: true}}},
		sassCase{name: "import/sourcemap", style: 1, resolver: "table",
			src: "@import \"outer\";\n.main { x: y; }",
			sm:  libsass.SourceMapOptions{Filename: "o.css.map", OutputPath: "o.css", Contents: true},
			table: []tableEntry{
				{url: "outer", newURL: "virtual/outer-new", body: ".outer { a: b; }", ok: true},
			}},
		sassCase{name: "import/echo-many", style: 3, resolver: "echo",
			src: "@import \"a\";\n@import \"b\";\n.x { c: $white; }"},
	)

	// 5. The Hugo resolver over extra files.
	extraIncludes := []string{"@SITE@/assets/extra"}
	hugoExtra := func(name, src string) sassCase {
		return sassCase{name: "hugo-extra/" + name, src: src, style: 1, precision: 8, includes: extraIncludes, resolver: "hugo", hugoBaseDir: "extra"}
	}
	cs = append(cs,
		hugoExtra("partials", "@import \"partial\";\n@import \"_partial\";\n@import \"partial.scss\";\n.m { w: $p; }"),
		hugoExtra("index", "@import \"dir\";\n@import \"dir2\";"),
		hugoExtra("both", "@import \"both\";"),
		hugoExtra("nested-relative", "@import \"nested/deep\";"),
		hugoExtra("sass-partial", "@import \"indented\";\n@import \"sassy\";"),
		hugoExtra("css-file", "@import \"plain.css\";\n@import \"plain\";"),
		hugoExtra("vendor", "@import \"../vendor/bootstrap/scss/functions\";\n.m { a: tint-color(#007bff, 10%); }"),
		hugoExtra("hugo-vars", "@import \"hugo:vars\";\n.m { a: $a; b: $b; }"),
		hugoExtra("not-found", "@import \"missing\";"),
	)
	cs[len(cs)-2].hugoVars = "$a: 24px;\n$b: unquote(\"x y\");"
	return cs
}

func snippetSrc(name string) string {
	for _, s := range snippets {
		if s.name == name {
			return s.src
		}
	}
	panic(name)
}

func itoa(i int) string {
	if i < 0 {
		return "-" + itoa(-i)
	}
	if i < 10 {
		return string(rune('0' + i))
	}
	return itoa(i/10) + string(rune('0'+i%10))
}
