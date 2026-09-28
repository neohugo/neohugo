package mdoracle

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"reflect"
	"sort"
	"strconv"
	"strings"

	"github.com/neohugo/neohugo/common/hugio"
	"github.com/neohugo/neohugo/common/text"
	"github.com/neohugo/neohugo/common/types/hstring"
	"github.com/neohugo/neohugo/markup/converter/hooks"
)

// The canonical dump format shared with the Rust tests
// (crates/nh-markup/tests/common/mod.rs): every field is written as
// "<name>:<byte length>:<bytes>\n", and values as by DumpVal.

// Field appends one length-prefixed field.
func Field(b *bytes.Buffer, name string, v []byte) {
	fmt.Fprintf(b, "%s:%d:", name, len(v))
	b.Write(v)
	b.WriteByte('\n')
}

// DumpVal is the canonical text of a hook value.
func DumpVal(v any) []byte {
	switch vv := v.(type) {
	case nil:
		return []byte("nil")
	case string:
		return []byte("s:" + vv)
	case hstring.HTML:
		return []byte("h:" + string(vv))
	case bool:
		return []byte("b:" + strconv.FormatBool(vv))
	case int:
		return []byte("i:" + strconv.Itoa(vv))
	case float64:
		return []byte("f:" + strconv.FormatFloat(vv, 'g', -1, 64))
	case [][2]int:
		if vv == nil {
			return []byte("r:nil")
		}
		var b bytes.Buffer
		b.WriteString("r:")
		for _, r := range vv {
			fmt.Fprintf(&b, "[%d,%d]", r[0], r[1])
		}
		return b.Bytes()
	case map[string]any:
		keys := make([]string, 0, len(vv))
		for k := range vv {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		var b bytes.Buffer
		b.WriteString("m{")
		for _, k := range keys {
			Field(&b, k, DumpVal(vv[k]))
		}
		b.WriteString("}")
		return b.Bytes()
	default:
		panic(fmt.Sprintf("DumpVal: %T", v))
	}
}

func htmlEscape(s string) string {
	return strings.NewReplacer(`&`, "&amp;", `"`, "&quot;", `<`, "&lt;", `>`, "&gt;").Replace(s)
}

// RenderTableReplica writes a table the way the embedded
// _markup/render-table.html does (same whitespace; attribute values as
// DumpVal text, HTML-escaped).
func RenderTableReplica(w io.Writer, ctx hooks.TableContext) {
	var b bytes.Buffer
	b.WriteString("<table")
	attrs := ctx.Attributes()
	keys := make([]string, 0, len(attrs))
	for k := range attrs {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		fmt.Fprintf(&b, " %s=\"%s\"", k, htmlEscape(string(DumpVal(attrs[k]))))
	}
	b.WriteString(">\n  <thead>")
	rows := func(rows []hooks.TableRow, tag string) {
		for _, row := range rows {
			b.WriteString("\n      <tr>")
			for _, c := range row {
				b.WriteString("\n          <" + tag)
				if c.Alignment != "" {
					fmt.Fprintf(&b, " style=\"text-align: %s\"", c.Alignment)
				}
				b.WriteString(">" + string(c.Text) + "</" + tag + ">")
			}
			b.WriteString("\n      </tr>")
		}
	}
	rows(ctx.THead(), "th")
	b.WriteString("\n  </thead>\n  <tbody>")
	rows(ctx.TBody(), "td")
	b.WriteString("\n  </tbody>\n</table>\n")
	_, _ = w.Write(b.Bytes())
}

// TableReplica is the table hook of the convert oracle (Hugo always has one).
type TableReplica struct{}

// RenderTable implements hooks.TableRenderer.
func (TableReplica) RenderTable(cctx context.Context, w hugio.FlexiWriter, ctx hooks.TableContext) error {
	RenderTableReplica(w, ctx)
	return nil
}

// CodeReplica is the code block hook of the convert oracle (Hugo's default
// is the Chroma highlighter, which the port does not have).
type CodeReplica struct{}

// RenderCodeblock implements hooks.CodeBlockRenderer.
func (CodeReplica) RenderCodeblock(cctx context.Context, w hugio.FlexiWriter, ctx hooks.CodeblockContext) error {
	_, _ = fmt.Fprintf(w, "<pre data-lang=\"%s\" data-ordinal=\"%d\">%s</pre>\n", htmlEscape(ctx.Type()), ctx.Ordinal(), htmlEscape(ctx.Inner()))
	return nil
}

// Recorder records every hook call of one conversion.
type Recorder struct {
	Records [][]byte
	seq     int
}

func (r *Recorder) next(kind string, ctx any) (int, *bytes.Buffer) {
	r.seq++
	var b bytes.Buffer
	Field(&b, "kind", []byte(kind))
	Field(&b, "seq", []byte(strconv.Itoa(r.seq)))
	// The context's Go type (%T) and exported method set, as templates see them.
	t := reflect.TypeOf(ctx)
	Field(&b, "type", []byte(fmt.Sprintf("%T", ctx)))
	var names []string
	for i := 0; i < t.NumMethod(); i++ {
		names = append(names, t.Method(i).Name)
	}
	sort.Strings(names)
	Field(&b, "methods", []byte(strings.Join(names, ",")))
	return r.seq, &b
}

func (r *Recorder) done(b *bytes.Buffer) {
	r.Records = append(r.Records, b.Bytes())
}

type attrsSlicer interface {
	Attributes() map[string]any
	Options() map[string]any
}

func dumpPage(b *bytes.Buffer, p hooks.PageProvider) {
	Field(b, "Page", DumpVal(p.Page()))
	Field(b, "PageInner", DumpVal(p.PageInner()))
}

func dumpAttrs(b *bytes.Buffer, ctx any) {
	a := ctx.(attrsSlicer)
	Field(b, "Attributes", DumpVal(a.Attributes()))
	Field(b, "Options", DumpVal(a.Options()))
	s := ctx.(hooks.AttributesOptionsSliceProvider)
	var sb bytes.Buffer
	for _, x := range s.AttributesSlice() {
		Field(&sb, x.Name, DumpVal(x.Value))
	}
	Field(b, "AttributesSlice", sb.Bytes())
	sb.Reset()
	for _, x := range s.OptionsSlice() {
		Field(&sb, x.Name, DumpVal(x.Value))
	}
	Field(b, "OptionsSlice", sb.Bytes())
}

func dumpPosition(b *bytes.Buffer, p text.Position) {
	Field(b, "Position", []byte(fmt.Sprintf("%s|%d|%d|%d", p.Filename, p.LineNumber, p.ColumnNumber, p.Offset)))
}

// resolver implements hooks.ElementPositionResolver: the position carries
// the PositionerSourceTarget bytes in its file name.
type resolver struct{}

func (resolver) ResolvePosition(ctx any) text.Position {
	sample := ctx.(hooks.PositionerSourceTargetProvider).PositionerSourceTarget()
	return text.Position{Filename: "sample=" + string(sample), LineNumber: len(sample), ColumnNumber: 7, Offset: -1}
}

// LinkRec records link and image hooks.
type LinkRec struct{ R *Recorder }

// RenderLink implements hooks.LinkRenderer.
func (l LinkRec) RenderLink(cctx context.Context, w io.Writer, ctx hooks.LinkContext) error {
	kind := "link"
	img, isImage := ctx.(hooks.ImageLinkContext)
	if isImage {
		kind = "image"
	}
	seq, b := l.R.next(kind, ctx)
	Field(b, "Destination", []byte(ctx.Destination()))
	Field(b, "Title", []byte(ctx.Title()))
	Field(b, "Text", []byte(ctx.Text()))
	Field(b, "PlainText", []byte(ctx.PlainText()))
	dumpPage(b, ctx)
	dumpAttrs(b, ctx)
	if isImage {
		Field(b, "IsBlock", DumpVal(img.IsBlock()))
		Field(b, "Ordinal", DumpVal(img.Ordinal()))
	}
	l.R.done(b)
	if ctx.Destination() == "hook-error" {
		return errors.New("link hook failed")
	}
	if isImage {
		_, _ = fmt.Fprintf(w, "[I%d]", seq)
	} else {
		_, _ = fmt.Fprintf(w, "[L%d|%s]", seq, ctx.Text())
	}
	return nil
}

// HeadingRec records heading hooks.
type HeadingRec struct{ R *Recorder }

// RenderHeading implements hooks.HeadingRenderer.
func (h HeadingRec) RenderHeading(cctx context.Context, w io.Writer, ctx hooks.HeadingContext) error {
	_, b := h.R.next("heading", ctx)
	Field(b, "Level", DumpVal(ctx.Level()))
	Field(b, "Anchor", []byte(ctx.Anchor()))
	Field(b, "Text", []byte(ctx.Text()))
	Field(b, "PlainText", []byte(ctx.PlainText()))
	dumpPage(b, ctx)
	dumpAttrs(b, ctx)
	h.R.done(b)
	_, _ = fmt.Fprintf(w, "<h%d id=\"%s\">%s</h%d>\n", ctx.Level(), ctx.Anchor(), ctx.Text(), ctx.Level())
	return nil
}

// BlockquoteRec records blockquote hooks.
type BlockquoteRec struct {
	resolver
	R *Recorder
}

// RenderBlockquote implements hooks.BlockquoteRenderer.
func (q BlockquoteRec) RenderBlockquote(cctx context.Context, w hugio.FlexiWriter, ctx hooks.BlockquoteContext) error {
	seq, b := q.R.next("blockquote", ctx)
	Field(b, "Type", []byte(ctx.Type()))
	Field(b, "AlertType", []byte(ctx.AlertType()))
	Field(b, "AlertTitle", []byte(ctx.AlertTitle()))
	Field(b, "AlertSign", []byte(ctx.AlertSign()))
	Field(b, "Text", []byte(ctx.Text()))
	Field(b, "Ordinal", DumpVal(ctx.Ordinal()))
	dumpPage(b, ctx)
	dumpPosition(b, ctx.Position())
	dumpAttrs(b, ctx)
	q.R.done(b)
	if strings.Contains(string(ctx.Text()), "BQERR") {
		return errors.New("blockquote hook failed")
	}
	_, _ = fmt.Fprintf(w, "<bq%d>%s</bq>\n", seq, ctx.Text())
	return nil
}

// TableRec records table hooks.
type TableRec struct {
	resolver
	R *Recorder
}

// RenderTable implements hooks.TableRenderer.
func (t TableRec) RenderTable(cctx context.Context, w hugio.FlexiWriter, ctx hooks.TableContext) error {
	_, b := t.R.next("table", ctx)
	Field(b, "Ordinal", DumpVal(ctx.Ordinal()))
	dumpPage(b, ctx)
	dumpPosition(b, ctx.Position())
	dumpAttrs(b, ctx)
	for _, part := range []struct {
		name string
		rows []hooks.TableRow
	}{{"THead", ctx.THead()}, {"TBody", ctx.TBody()}} {
		var rb bytes.Buffer
		if part.rows == nil {
			rb.WriteString("nil")
		}
		for _, row := range part.rows {
			rb.WriteString("[")
			for _, c := range row {
				Field(&rb, c.Alignment, []byte(c.Text))
			}
			rb.WriteString("]")
		}
		Field(b, part.name, rb.Bytes())
	}
	t.R.done(b)
	RenderTableReplica(w, ctx)
	return nil
}

// CodeRec records code block hooks.
type CodeRec struct {
	resolver
	R       *Recorder
	Default bool
}

// IsDefaultCodeBlockRenderer implements hooks.IsDefaultCodeBlockRendererProvider.
func (c CodeRec) IsDefaultCodeBlockRenderer() bool {
	return c.Default
}

// RenderCodeblock implements hooks.CodeBlockRenderer.
func (c CodeRec) RenderCodeblock(cctx context.Context, w hugio.FlexiWriter, ctx hooks.CodeblockContext) error {
	seq, b := c.R.next("codeblock", ctx)
	Field(b, "Type", []byte(ctx.Type()))
	Field(b, "Inner", []byte(ctx.Inner()))
	Field(b, "Ordinal", DumpVal(ctx.Ordinal()))
	dumpPage(b, ctx)
	dumpPosition(b, ctx.Position())
	dumpAttrs(b, ctx)
	c.R.done(b)
	if ctx.Type() == "errlang" {
		return errors.New("code block hook failed")
	}
	_, _ = fmt.Fprintf(w, "<pre%d lang=\"%s\">%s</pre>\n", seq, ctx.Type(), ctx.Inner())
	return nil
}

// RecordingRenderers returns the GetRendererFunc of the hooks oracle:
// recording hooks for every type; no code block renderer for the language
// "nohook"; code block renderers for languages starting with "d" claim to
// be the default (Chroma) renderer.
func RecordingRenderers(r *Recorder) hooks.GetRendererFunc {
	return func(t hooks.RendererType, id any) any {
		switch t {
		case hooks.LinkRendererType, hooks.ImageRendererType:
			return LinkRec{R: r}
		case hooks.HeadingRendererType:
			return HeadingRec{R: r}
		case hooks.BlockquoteRendererType:
			return BlockquoteRec{R: r}
		case hooks.TableRendererType:
			return TableRec{R: r}
		case hooks.CodeBlockRendererType:
			lang := id.(string)
			if lang == "nohook" {
				return nil
			}
			return CodeRec{R: r, Default: strings.HasPrefix(lang, "d")}
		}
		return nil
	}
}

// ReplicaRenderers returns the GetRendererFunc of the convert oracle: only
// the table and code block replicas (Hugo always has both), so links,
// images, headings and blockquotes use the default renderers.
func ReplicaRenderers(t hooks.RendererType, id any) any {
	switch t {
	case hooks.TableRendererType:
		return TableReplica{}
	case hooks.CodeBlockRendererType:
		return CodeReplica{}
	}
	return nil
}
