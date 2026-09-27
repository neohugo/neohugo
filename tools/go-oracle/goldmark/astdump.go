package main

import (
	"bytes"
	"fmt"
	"math"
	"strings"

	gast "github.com/yuin/goldmark/ast"
	east "github.com/yuin/goldmark/extension/ast"
	"github.com/yuin/goldmark/parser"
	"github.com/yuin/goldmark/renderer"
	"github.com/yuin/goldmark/text"
)

// astTextLimit: documents longer than this are dumped without the
// (deprecated, quadratic in depth) Text value of every node. The Rust side
// (tests/common/astdump.rs) uses the same limit.
const astTextLimit = 4096

// The AST dump compared by the Rust tests (tests/common/astdump.rs): one
// line per node in pre-order with the kind, type, child count, lines,
// attributes (typed values), node-specific fields and Text. It covers the
// API Hugo's nh-markup layer reads, which the HTML does not show.
func dq(b *bytes.Buffer, v []byte) {
	b.WriteByte('"')
	for _, c := range v {
		if c >= 0x20 && c < 0x7f && c != '"' && c != '\\' {
			b.WriteByte(c)
		} else {
			fmt.Fprintf(b, "\\x%02x", c)
		}
	}
	b.WriteByte('"')
}

func dseg(b *bytes.Buffer, s text.Segment) {
	fmt.Fprintf(b, "%d:%d:%d", s.Start, s.Stop, s.Padding)
	if s.ForceNewline {
		b.WriteString(":n")
	}
}

func dbool(v bool) string {
	if v {
		return "1"
	}
	return "0"
}

func dvalue(b *bytes.Buffer, v interface{}) {
	switch x := v.(type) {
	case []byte:
		b.WriteByte('b')
		dq(b, x)
	case string:
		b.WriteByte('s')
		dq(b, []byte(x))
	case float64:
		fmt.Fprintf(b, "f%016x", math.Float64bits(x))
	case bool:
		if x {
			b.WriteString("true")
		} else {
			b.WriteString("false")
		}
	case nil:
		b.WriteString("nil")
	case []interface{}:
		b.WriteByte('[')
		for i, e := range x {
			if i > 0 {
				b.WriteByte(',')
			}
			dvalue(b, e)
		}
		b.WriteByte(']')
	case parser.Attributes:
		b.WriteByte('{')
		for i, a := range x {
			if i > 0 {
				b.WriteByte(';')
			}
			dq(b, a.Name)
			b.WriteByte('=')
			dvalue(b, a.Value)
		}
		b.WriteByte('}')
	default:
		b.WriteByte('?')
	}
}

func daligns(a []east.Alignment) string {
	s := make([]string, len(a))
	for i, x := range a {
		s[i] = x.String()
	}
	return strings.Join(s, ",")
}

func dnode(b *bytes.Buffer, n gast.Node, src []byte, depth int, withText bool) {
	b.WriteString(strings.Repeat(" ", depth))
	fmt.Fprintf(b, "%s t=%d cc=%d", n.Kind().String(), n.Type(), n.ChildCount())
	if n.Type() != gast.TypeInline {
		b.WriteString(" lines=")
		lines := n.Lines()
		for i := 0; i < lines.Len(); i++ {
			if i > 0 {
				b.WriteByte(',')
			}
			dseg(b, lines.At(i))
		}
		fmt.Fprintf(b, " blank=%s", dbool(n.HasBlankPreviousLines()))
	}
	fmt.Fprintf(b, " raw=%s", dbool(n.IsRaw()))
	if attrs := n.Attributes(); attrs != nil {
		b.WriteString(" attrs={")
		for i, a := range attrs {
			if i > 0 {
				b.WriteByte(';')
			}
			dq(b, a.Name)
			b.WriteByte('=')
			dvalue(b, a.Value)
		}
		b.WriteByte('}')
	}
	switch x := n.(type) {
	case *gast.Heading:
		fmt.Fprintf(b, " lvl=%d toc=", x.Level)
		dq(b, tocText(x, src))
	case *gast.FencedCodeBlock:
		b.WriteString(" info=")
		if x.Info != nil {
			dseg(b, x.Info.Segment)
		} else {
			b.WriteByte('-')
		}
		b.WriteString(" lang=")
		if l := x.Language(src); l != nil {
			dq(b, l)
		} else {
			b.WriteByte('-')
		}
	case *gast.HTMLBlock:
		fmt.Fprintf(b, " ht=%d cl=", x.HTMLBlockType)
		dseg(b, x.ClosureLine)
	case *gast.List:
		fmt.Fprintf(b, " m=%d tight=%s start=%d", x.Marker, dbool(x.IsTight), x.Start)
	case *gast.ListItem:
		fmt.Fprintf(b, " off=%d", x.Offset)
	case *gast.Text:
		b.WriteString(" seg=")
		dseg(b, x.Segment)
		fmt.Fprintf(b, " soft=%s hard=%s", dbool(x.SoftLineBreak()), dbool(x.HardLineBreak()))
	case *gast.String:
		b.WriteString(" v=")
		dq(b, x.Value)
		fmt.Fprintf(b, " code=%s", dbool(x.IsCode()))
	case *gast.Emphasis:
		fmt.Fprintf(b, " lvl=%d", x.Level)
	case *gast.Link:
		b.WriteString(" dest=")
		dq(b, x.Destination)
		b.WriteString(" title=")
		dq(b, x.Title)
	case *gast.Image:
		b.WriteString(" dest=")
		dq(b, x.Destination)
		b.WriteString(" title=")
		dq(b, x.Title)
	case *gast.AutoLink:
		fmt.Fprintf(b, " alt=%d proto=", x.AutoLinkType)
		dq(b, x.Protocol)
		b.WriteString(" url=")
		dq(b, x.URL(src))
		b.WriteString(" label=")
		dq(b, x.Label(src))
	case *gast.RawHTML:
		b.WriteString(" segs=")
		for i := 0; i < x.Segments.Len(); i++ {
			if i > 0 {
				b.WriteByte(',')
			}
			dseg(b, x.Segments.At(i))
		}
	case *east.Table:
		fmt.Fprintf(b, " al=%s", daligns(x.Alignments))
	case *east.TableHeader:
		fmt.Fprintf(b, " al=%s", daligns(x.Alignments))
	case *east.TableRow:
		fmt.Fprintf(b, " al=%s", daligns(x.Alignments))
	case *east.TableCell:
		fmt.Fprintf(b, " al=%s", x.Alignment.String())
	case *east.TaskCheckBox:
		fmt.Fprintf(b, " checked=%s", dbool(x.IsChecked))
	case *east.DefinitionList:
		fmt.Fprintf(b, " off=%d tmp=%s", x.Offset, dbool(x.TemporaryParagraph != nil))
	case *east.DefinitionDescription:
		fmt.Fprintf(b, " tight=%s", dbool(x.IsTight))
	case *east.Footnote:
		b.WriteString(" ref=")
		dq(b, x.Ref)
		fmt.Fprintf(b, " idx=%d", x.Index)
	case *east.FootnoteLink:
		fmt.Fprintf(b, " idx=%d rc=%d ri=%d", x.Index, x.RefCount, x.RefIndex)
	case *east.FootnoteBacklink:
		fmt.Fprintf(b, " idx=%d rc=%d ri=%d", x.Index, x.RefCount, x.RefIndex)
	case *east.FootnoteList:
		fmt.Fprintf(b, " count=%d", x.Count)
	}
	if withText {
		b.WriteString(" text=")
		//nolint:staticcheck // Node.Text is deprecated but part of the API Hugo can call.
		dq(b, n.Text(src))
	}
	b.WriteByte('\n')
}

// dumpRenderer renders the heading subtrees of the AST dump (parseDump).
var dumpRenderer renderer.Renderer

// tocText renders a heading's inline subtrees one by one, as Hugo's TOC
// transformer does (markup/goldmark/toc.go): the renderer API on nodes
// that are not the document.
func tocText(h gast.Node, src []byte) []byte {
	if dumpRenderer == nil {
		return nil
	}
	var buf bytes.Buffer
	_ = gast.Walk(h, func(n gast.Node, entering bool) (gast.WalkStatus, error) {
		if !entering || n == h {
			return gast.WalkContinue, nil
		}
		switch n.Kind() {
		case gast.KindCodeSpan, gast.KindLink, gast.KindImage, gast.KindEmphasis, east.KindStrikethrough:
			if err := dumpRenderer.Render(&buf, src, n); err != nil {
				buf.WriteString("ERROR")
			}
			return gast.WalkSkipChildren, nil
		case gast.KindAutoLink, gast.KindRawHTML, gast.KindText, gast.KindString:
			if err := dumpRenderer.Render(&buf, src, n); err != nil {
				buf.WriteString("ERROR")
			}
		}
		return gast.WalkContinue, nil
	})
	return buf.Bytes()
}

// astDump dumps the tree below root in pre-order (explicit stack: documents
// can be deeply nested).
func astDump(root gast.Node, src []byte) []byte {
	var b bytes.Buffer
	withText := len(src) <= astTextLimit
	type item struct {
		n     gast.Node
		depth int
	}
	stack := []item{{root, 0}}
	for len(stack) > 0 {
		it := stack[len(stack)-1]
		stack = stack[:len(stack)-1]
		dnode(&b, it.n, src, it.depth, withText)
		var children []gast.Node
		for c := it.n.FirstChild(); c != nil; c = c.NextSibling() {
			children = append(children, c)
		}
		for i := len(children) - 1; i >= 0; i-- {
			stack = append(stack, item{children[i], it.depth + 1})
		}
	}
	return b.Bytes()
}

// parseDump parses md with the named config and dumps the AST ("PANIC" when
// goldmark panics).
func parseDump(cfg string, md []byte) (out []byte) {
	m, ok := mdCache[cfg]
	if !ok {
		m = newMarkdown(cfg)
		mdCache[cfg] = m
	}
	defer func() {
		if r := recover(); r != nil {
			out = []byte("PANIC")
		}
	}()
	doc := m.Parser().Parse(text.NewReader(md))
	dumpRenderer = m.Renderer()
	defer func() { dumpRenderer = nil }()
	return astDump(doc, md)
}
