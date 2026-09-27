package main

import (
	"bufio"
	"bytes"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"

	"github.com/yuin/goldmark"
	"github.com/yuin/goldmark/parser"
	"github.com/yuin/goldmark/renderer/html"
)

// configs are the goldmark instances the fixtures are rendered with. The
// Rust tests build the same instances by name (tests/common/mod.rs).
var configNames = []string{"default", "xu", "attr", "unsafe", "hard", "all", "escspace", "ea-simple", "ea-css3"}

func newMarkdown(name string) goldmark.Markdown {
	switch name {
	case "default":
		return goldmark.New()
	case "xu":
		// The configuration of goldmark's own spec/extra tests.
		return goldmark.New(goldmark.WithRendererOptions(html.WithXHTML(), html.WithUnsafe()))
	case "attr":
		// The configuration of goldmark's options_test.go.
		return goldmark.New(goldmark.WithParserOptions(parser.WithAttribute(), parser.WithAutoHeadingID()))
	case "unsafe":
		return goldmark.New(goldmark.WithRendererOptions(html.WithUnsafe()))
	case "hard":
		return goldmark.New(goldmark.WithRendererOptions(html.WithHardWraps(), html.WithXHTML()))
	case "all":
		return goldmark.New(
			goldmark.WithParserOptions(parser.WithAttribute(), parser.WithAutoHeadingID()),
			goldmark.WithRendererOptions(html.WithUnsafe(), html.WithXHTML(), html.WithHardWraps()),
		)
	case "escspace":
		return goldmark.New(
			goldmark.WithParserOptions(parser.WithEscapedSpace()),
			goldmark.WithRendererOptions(html.WithWriter(html.NewWriter(html.WithEscapedSpace())), html.WithUnsafe()),
		)
	case "ea-simple":
		return goldmark.New(goldmark.WithRendererOptions(html.WithEastAsianLineBreaks(html.EastAsianLineBreaksSimple)))
	case "ea-css3":
		return goldmark.New(goldmark.WithRendererOptions(html.WithEastAsianLineBreaks(html.EastAsianLineBreaksCSS3Draft)))
	case "plugin":
		return newPluginMarkdown(false)
	case "plugin-unsafe":
		return newPluginMarkdown(true)
	}
	return newExtMarkdown(name)
}

var mdCache = map[string]goldmark.Markdown{}

// convert renders md with the named config. A panic inside goldmark is
// reported as output "PANIC: <value>" so the Rust side can check it too.
func convert(cfg string, md []byte) (out []byte) {
	m, ok := mdCache[cfg]
	if !ok {
		m = newMarkdown(cfg)
		mdCache[cfg] = m
	}
	defer func() {
		if r := recover(); r != nil {
			out = []byte(fmt.Sprintf("PANIC: %v", r))
		}
	}()
	var b bytes.Buffer
	if err := m.Convert(md, &b); err != nil {
		return []byte("ERROR: " + err.Error())
	}
	return b.Bytes()
}

// GMF ("goldmark fixture") format, parsed by tests/common/mod.rs:
//
//	=== <record name>\n
//	<key> <byte length>\n<raw bytes>\n
//	...
//
// Raw bytes may contain anything (invalid UTF-8, NUL, CR).
type gmfWriter struct {
	w *bufio.Writer
}

func newGMF(w io.Writer) *gmfWriter {
	return &gmfWriter{bufio.NewWriter(w)}
}

func (g *gmfWriter) record(name string) {
	if strings.ContainsAny(name, "\n") {
		panic("bad record name")
	}
	_, _ = fmt.Fprintf(g.w, "=== %s\n", name)
}

func (g *gmfWriter) field(key string, v []byte) {
	_, _ = fmt.Fprintf(g.w, "%s %d\n", key, len(v))
	_, _ = g.w.Write(v)
	_ = g.w.WriteByte('\n')
}

func (g *gmfWriter) flush() {
	if err := g.w.Flush(); err != nil {
		panic(err)
	}
}

type gmfRecord struct {
	name   string
	fields map[string][]byte
	keys   []string
}

func readGMF(r io.Reader) []gmfRecord {
	data, err := io.ReadAll(r)
	if err != nil {
		panic(err)
	}
	var recs []gmfRecord
	for len(data) > 0 {
		nl := bytes.IndexByte(data, '\n')
		line := string(data[:nl])
		data = data[nl+1:]
		if strings.HasPrefix(line, "=== ") {
			recs = append(recs, gmfRecord{name: line[4:], fields: map[string][]byte{}})
			continue
		}
		sp := strings.LastIndexByte(line, ' ')
		key := line[:sp]
		n, err := strconv.Atoi(line[sp+1:])
		if err != nil {
			panic(err)
		}
		v := data[:n]
		data = data[n+1:]
		rec := &recs[len(recs)-1]
		rec.fields[key] = v
		rec.keys = append(rec.keys, key)
	}
	return recs
}

// renderMain reads GMF records with "cfg" and "md" fields from stdin and
// writes them back with an "html" field.
func renderMain(args []string) {
	recs := readGMF(os.Stdin)
	g := newGMF(os.Stdout)
	for _, r := range recs {
		g.record(r.name)
		for _, k := range r.keys {
			g.field(k, r.fields[k])
		}
		g.field("html", convert(string(r.fields["cfg"]), r.fields["md"]))
	}
	g.flush()
}
