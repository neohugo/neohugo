package main

import (
	"bytes"

	"github.com/yuin/goldmark"
	"github.com/yuin/goldmark/ast"
	"github.com/yuin/goldmark/extension"
	"github.com/yuin/goldmark/parser"
	"github.com/yuin/goldmark/renderer"
	"github.com/yuin/goldmark/renderer/html"
	"github.com/yuin/goldmark/text"
	"github.com/yuin/goldmark/util"
)

// A test extension that exercises goldmark's plugin API the way Hugo's
// extensions do. crates/goldmark/tests/plugin_api.rs implements the same
// extension on the Rust API; the "plugin" config renders with it (plus
// extension.Strikethrough, re-implemented in that test).
//
//   - an inline parser (priority 50, trigger '{') for `{{name}}` producing a
//     custom inline node (Mustache);
//   - a container block parser (priority 850, trigger '%') for lines
//     starting with "%%" (Note);
//   - an AST transformer (priority 10) that, like hugocontext, replaces a
//     paragraph whose only child is a Mustache by the Mustache and drops the
//     soft line break of a Text before a Mustache;
//   - a node renderer (priority 50) rendering Mustache/Note and overriding
//     RawHTML (like hugocontext), reading the Unsafe option via SetOption.

var kindMustache = ast.NewNodeKind("Mustache")
var kindNote = ast.NewNodeKind("Note")

type mustache struct {
	ast.BaseInline
	Name []byte
}

func (n *mustache) Kind() ast.NodeKind            { return kindMustache }
func (n *mustache) Dump(source []byte, level int) { ast.DumpHelper(n, source, level, nil, nil) }

type note struct {
	ast.BaseBlock
}

func (n *note) Kind() ast.NodeKind            { return kindNote }
func (n *note) Dump(source []byte, level int) { ast.DumpHelper(n, source, level, nil, nil) }

type mustacheParser struct{}

func (p *mustacheParser) Trigger() []byte { return []byte{'{'} }

func (p *mustacheParser) Parse(parent ast.Node, reader text.Reader, pc parser.Context) ast.Node {
	line, _ := reader.PeekLine()
	if !bytes.HasPrefix(line, []byte("{{")) {
		return nil
	}
	end := bytes.Index(line, []byte("}}"))
	if end < 0 {
		return nil
	}
	reader.Advance(end + 2)
	return &mustache{Name: append([]byte{}, line[2:end]...)}
}

type noteParser struct{}

func (p *noteParser) Trigger() []byte { return []byte{'%'} }

func (p *noteParser) process(reader text.Reader) bool {
	line, _ := reader.PeekLine()
	w, pos := util.IndentWidth(line, reader.LineOffset())
	if w > 3 || pos+1 >= len(line) || line[pos] != '%' || line[pos+1] != '%' {
		return false
	}
	reader.Advance(pos + 2)
	return true
}

func (p *noteParser) Open(parent ast.Node, reader text.Reader, pc parser.Context) (ast.Node, parser.State) {
	if p.process(reader) {
		return &note{}, parser.HasChildren
	}
	return nil, parser.NoChildren
}

func (p *noteParser) Continue(node ast.Node, reader text.Reader, pc parser.Context) parser.State {
	if p.process(reader) {
		return parser.Continue | parser.HasChildren
	}
	return parser.Close
}

func (p *noteParser) Close(node ast.Node, reader text.Reader, pc parser.Context) {}
func (p *noteParser) CanInterruptParagraph() bool                                { return true }
func (p *noteParser) CanAcceptIndentedLine() bool                                { return false }

type mustacheTransformer struct{}

func (t *mustacheTransformer) Transform(doc *ast.Document, reader text.Reader, pc parser.Context) {
	_ = ast.Walk(doc, func(n ast.Node, entering bool) (ast.WalkStatus, error) {
		if !entering || n.Kind() != kindMustache {
			return ast.WalkContinue, nil
		}
		if p, ok := n.Parent().(*ast.Paragraph); ok {
			if p.ChildCount() == 1 {
				p.Parent().ReplaceChild(p.Parent(), p, n)
			} else if t, ok := n.PreviousSibling().(*ast.Text); ok && t.SoftLineBreak() {
				if t.Segment.Len() == 0 {
					p.RemoveChild(p, t)
				} else {
					t.SetSoftLineBreak(false)
				}
			}
		}
		return ast.WalkContinue, nil
	})
}

type pluginRenderer struct {
	html.Config
}

func (r *pluginRenderer) SetOption(name renderer.OptionName, value interface{}) {
	r.Config.SetOption(name, value)
}

func (r *pluginRenderer) RegisterFuncs(reg renderer.NodeRendererFuncRegisterer) {
	reg.Register(kindMustache, func(w util.BufWriter, source []byte, n ast.Node, entering bool) (ast.WalkStatus, error) {
		if entering {
			_, _ = w.WriteString(`<span class="m">`)
			r.Writer.Write(w, n.(*mustache).Name)
			_, _ = w.WriteString(`</span>`)
		}
		return ast.WalkContinue, nil
	})
	reg.Register(kindNote, func(w util.BufWriter, source []byte, n ast.Node, entering bool) (ast.WalkStatus, error) {
		if entering {
			_, _ = w.WriteString("<aside>\n")
		} else {
			_, _ = w.WriteString("</aside>\n")
		}
		return ast.WalkContinue, nil
	})
	reg.Register(ast.KindRawHTML, func(w util.BufWriter, source []byte, node ast.Node, entering bool) (ast.WalkStatus, error) {
		if !entering {
			return ast.WalkSkipChildren, nil
		}
		n := node.(*ast.RawHTML)
		if r.Unsafe {
			_, _ = w.WriteString("[raw:")
			for i := 0; i < n.Segments.Len(); i++ {
				seg := n.Segments.At(i)
				_, _ = w.Write(seg.Value(source))
			}
			_, _ = w.WriteString("]")
		} else {
			_, _ = w.WriteString("[omitted]")
		}
		return ast.WalkSkipChildren, nil
	})
}

type pluginExtension struct{}

func (e *pluginExtension) Extend(m goldmark.Markdown) {
	m.Parser().AddOptions(
		parser.WithInlineParsers(util.Prioritized(&mustacheParser{}, 50)),
		parser.WithBlockParsers(util.Prioritized(&noteParser{}, 850)),
		parser.WithASTTransformers(util.Prioritized(&mustacheTransformer{}, 10)),
	)
	m.Renderer().AddOptions(renderer.WithNodeRenderers(
		util.Prioritized(&pluginRenderer{Config: html.NewConfig()}, 50),
	))
}

func newPluginMarkdown(unsafe bool) goldmark.Markdown {
	opts := []goldmark.Option{
		goldmark.WithExtensions(extension.Strikethrough, &pluginExtension{}),
		goldmark.WithParserOptions(parser.WithAttribute(), parser.WithAutoHeadingID()),
	}
	if unsafe {
		opts = append(opts, goldmark.WithRendererOptions(html.WithUnsafe()))
	}
	return goldmark.New(opts...)
}
