package main

import (
	"bytes"
	"encoding/hex"
	"fmt"
	"io"

	"github.com/tdewolff/parse/v2"
	"github.com/tdewolff/parse/v2/css"
	"github.com/tdewolff/parse/v2/html"
	"github.com/tdewolff/parse/v2/json"
	"github.com/tdewolff/parse/v2/xml"
)

// The serializations below are mirrored byte for byte by
// crates/tdewolff-parse/tests/common/mod.rs.

// hx hex-encodes a byte slice, "-" for nil.
func hx(b []byte) string {
	if b == nil {
		return "-"
	}
	return hex.EncodeToString(b)
}

// hs hex-encodes a string.
func hs(s string) string {
	return hex.EncodeToString([]byte(s))
}

// errStr hex-encodes err.Error(), "-" for nil.
func errStr(err error) string {
	if err == nil {
		return "-"
	}
	return hs(err.Error())
}

func b2i(b bool) int {
	if b {
		return 1
	}
	return 0
}

// cp returns a fresh copy with Go's append capacity (so NewInputBytes
// behaves as for any heap slice).
func cp(s []byte) []byte {
	return append([]byte(nil), s...)
}

func tokenLimit(in []byte) int {
	return 4*len(in) + 64
}

// htmlStream lexes in with the html lexer and serializes every token, the
// final error and the (possibly mutated) input buffer.
func htmlStream(in []byte, tmpl bool) []byte {
	if tmpl {
		return htmlStreamDelims(in, &html.GoTemplate)
	}
	return htmlStreamDelims(in, nil)
}

// htmlStreamDelims is htmlStream with the given template delimiters (nil for
// the plain lexer).
func htmlStreamDelims(in []byte, delims *[2]string) []byte {
	var sb bytes.Buffer
	b := cp(in)
	z := parse.NewInputBytes(b)
	var l *html.Lexer
	if delims != nil {
		l = html.NewTemplateLexer(z, *delims)
	} else {
		l = html.NewLexer(z)
	}
	limit := tokenLimit(in)
	for i := 0; ; i++ {
		tt, data := l.Next()
		fmt.Fprintf(&sb, "%d %d %s %s %s %d\n", z.Offset(), tt, hx(data), hx(l.Text()), hx(l.AttrVal()), b2i(l.HasTemplate()))
		if tt == html.ErrorToken || i > limit {
			break
		}
	}
	fmt.Fprintf(&sb, "err %s\n", errStr(l.Err()))
	z.Restore()
	fmt.Fprintf(&sb, "buf %s\n", hx(b))
	return sb.Bytes()
}

// cssLexStream lexes in with the css lexer.
func cssLexStream(in []byte) []byte {
	var sb bytes.Buffer
	b := cp(in)
	z := parse.NewInputBytes(b)
	l := css.NewLexer(z)
	limit := tokenLimit(in)
	for i := 0; ; i++ {
		tt, data := l.Next()
		fmt.Fprintf(&sb, "%d %d %s\n", z.Offset(), tt, hx(data))
		if tt == css.ErrorToken || i > limit {
			break
		}
	}
	fmt.Fprintf(&sb, "err %s\n", errStr(l.Err()))
	z.Restore()
	fmt.Fprintf(&sb, "buf %s\n", hx(b))
	return sb.Bytes()
}

// cssParseStream parses in with the css parser.
func cssParseStream(in []byte, inline bool) []byte {
	var sb bytes.Buffer
	b := cp(in)
	z := parse.NewInputBytes(b)
	p := css.NewParser(z, inline)
	limit := tokenLimit(in)
	for i := 0; ; i++ {
		gt, tt, data := p.Next()
		fmt.Fprintf(&sb, "%d %d %d %s %d", p.Offset(), gt, tt, hx(data), len(p.Values()))
		for _, v := range p.Values() {
			fmt.Fprintf(&sb, " %d:%s", v.TokenType, hx(v.Data))
		}
		fmt.Fprintf(&sb, " %d", b2i(p.HasParseError()))
		err := p.Err()
		if gt == css.ErrorGrammar {
			fmt.Fprintf(&sb, " %s", errStr(err))
		}
		sb.WriteByte('\n')
		if gt == css.ErrorGrammar && err == io.EOF || i > limit {
			break
		}
	}
	z.Restore()
	fmt.Fprintf(&sb, "buf %s\n", hx(b))
	return sb.Bytes()
}

// xmlStream lexes in with the xml lexer.
func xmlStream(in []byte) []byte {
	var sb bytes.Buffer
	b := cp(in)
	z := parse.NewInputBytes(b)
	l := xml.NewLexer(z)
	limit := tokenLimit(in)
	for i := 0; ; i++ {
		tt, data := l.Next()
		fmt.Fprintf(&sb, "%d %d %s %s %s\n", z.Offset(), tt, hx(data), hx(l.Text()), hx(l.AttrVal()))
		if tt == xml.ErrorToken || i > limit {
			break
		}
	}
	fmt.Fprintf(&sb, "err %s\n", errStr(l.Err()))
	z.Restore()
	fmt.Fprintf(&sb, "buf %s\n", hx(b))
	return sb.Bytes()
}

// jsonStream parses in with the json parser.
func jsonStream(in []byte) []byte {
	var sb bytes.Buffer
	b := cp(in)
	z := parse.NewInputBytes(b)
	p := json.NewParser(z)
	limit := tokenLimit(in)
	for i := 0; ; i++ {
		gt, data := p.Next()
		fmt.Fprintf(&sb, "%d %d %s %d\n", z.Offset(), gt, hx(data), p.State())
		if gt == json.ErrorGrammar || i > limit {
			break
		}
	}
	fmt.Fprintf(&sb, "err %s\n", errStr(p.Err()))
	z.Restore()
	fmt.Fprintf(&sb, "buf %s\n", hx(b))
	return sb.Bytes()
}

// streamFor dispatches on a corpus kind.
func streamFor(kind string, in []byte) []byte {
	switch kind {
	case "html":
		return htmlStream(in, false)
	case "htmltmpl":
		return htmlStream(in, true)
	case "htmlphp":
		return htmlStreamDelims(in, &html.PHPTemplate)
	case "csslex":
		return cssLexStream(in)
	case "css":
		return cssParseStream(in, false)
	case "cssinline":
		return cssParseStream(in, true)
	case "xml":
		return xmlStream(in)
	case "json":
		return jsonStream(in)
	}
	panic("unknown kind " + kind)
}

// fnv64a is FNV-1a 64 (mirrored in Rust).
func fnv64a(b []byte) uint64 {
	h := uint64(14695981039346656037)
	for _, c := range b {
		h ^= uint64(c)
		h *= 1099511628211
	}
	return h
}
