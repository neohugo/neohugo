// Package mdoracle is the shared part of the nh-markup Go oracles
// (tools/go-oracle/nh-markup/<topic>): the markdown corpus, the markup
// configurations, converter construction through neohugo's real
// markup/goldmark provider, the recording hook renderers and the fixture
// writer.
package mdoracle

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/markup/converter"
	"github.com/neohugo/neohugo/markup/goldmark"
	"github.com/neohugo/neohugo/markup/markup_config"
	"github.com/neohugo/neohugo/parser/pageparser"
)

// Doc is one markdown document: a name and the bytes fed to the converter.
type Doc struct {
	Name string `json:"name"`
	Src  B      `json:"src"`
}

// LoadCorpus returns the substitute corpus: every .md file under
// docs/content, hugolib/testsite and docs/rust-port (front matter stripped
// the way Hugo does, with neohugo's pageparser), then the adversarial
// documents.
func LoadCorpus(root string) []Doc {
	var docs []Doc
	for _, dir := range []string{"docs/content", "hugolib/testsite", "docs/rust-port"} {
		var files []string
		err := filepath.WalkDir(filepath.Join(root, dir), func(path string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if !d.IsDir() && strings.HasSuffix(path, ".md") {
				files = append(files, path)
			}
			return nil
		})
		if err != nil {
			panic(err)
		}
		sort.Strings(files)
		for _, f := range files {
			b, err := os.ReadFile(f)
			if err != nil {
				panic(err)
			}
			cfm, err := pageparser.ParseFrontMatterAndContent(bytes.NewReader(b))
			if err != nil {
				panic(err)
			}
			rel, _ := filepath.Rel(root, f)
			docs = append(docs, Doc{Name: filepath.ToSlash(rel), Src: cfm.Content})
		}
	}
	return append(docs, Adversarial()...)
}

// Config is a named markup configuration (a TOML site config).
type Config struct {
	Name string `json:"name"`
	TOML string `json:"toml"`
}

// Configs are the configurations the oracles render with: neohugo's
// defaults, the seeksnack [markup] section (with its legacy pygments keys),
// and variants for the other auto ID types, TOC levels, attributes, XHTML,
// hard wraps, disabled extensions and CJK.
var Configs = []Config{
	{Name: "default", TOML: ``},
	{Name: "seeksnack", TOML: `
pygmentsCodeFences = true
pygmentsOptions = "linenos=table"
pygmentsStyle = "monokai"
[markup]
defaultMarkdownHandler = "goldmark"
[markup.goldmark.extensions]
definitionList = true
footnote = true
linkify = true
strikethrough = true
table = true
taskList = true
typographer = true
[markup.goldmark.parser]
autoHeadingID = true
autoHeadingIDType = "github"
[markup.goldmark.parser.attribute]
block = false
title = true
[markup.goldmark.renderer]
hardWraps = false
unsafe = true
xhtml = false
`},
	{Name: "ascii", TOML: `
[markup.goldmark.renderer]
unsafe = true
[markup.goldmark.parser]
autoIDType = "github-ascii"
autoDefinitionTermID = true
wrapStandAloneImageWithinParagraph = false
[markup.goldmark.parser.attribute]
block = true
title = true
[markup.tableOfContents]
startLevel = 1
endLevel = 4
ordered = true
`},
	{Name: "blackfriday", TOML: `
[markup.goldmark.renderer]
unsafe = false
xhtml = true
hardWraps = true
[markup.goldmark.parser]
autoIDType = "blackfriday"
[markup.goldmark.extensions]
typographer = false
linkifyProtocol = "http"
[markup.tableOfContents]
startLevel = 3
endLevel = -1
`},
	{Name: "cjk", TOML: `
[markup.highlight]
codeFences = false
[markup.goldmark.renderer]
unsafe = true
[markup.goldmark.parser]
autoHeadingID = false
attribute = false
[markup.goldmark.extensions]
table = false
strikethrough = false
linkify = false
taskList = false
definitionList = false
footnote = false
[markup.goldmark.extensions.typographer]
leftDoubleQuote = "«"
rightDoubleQuote = "»"
apostrophe = ""
ellipsis = "…"
[markup.goldmark.extensions.cjk]
enable = true
eastAsianLineBreaks = true
eastAsianLineBreaksStyle = "css3draft"
escapedSpace = true
`},
	{Name: "noattr", TOML: `
[markup.goldmark.parser]
autoHeadingID = false
autoDefinitionTermID = true
[markup.goldmark.parser.attribute]
title = false
block = true
[markup.goldmark.extensions.cjk]
enable = true
`},
}

// ConfigByName returns the named configuration.
func ConfigByName(name string) Config {
	for _, c := range Configs {
		if c.Name == name {
			return c
		}
	}
	panic("no config " + name)
}

type confStub struct {
	config.AllProvider
	m markup_config.Config
}

func (c confStub) GetConfigSection(s string) any {
	if s != "markup" {
		panic("not implemented: " + s)
	}
	return c.m
}

func (c confStub) EnableEmoji() bool { return false }

// DecodeMarkup decodes the [markup] section of a TOML site config.
func DecodeMarkup(toml string) (markup_config.Config, error) {
	return markup_config.Decode(config.FromTOMLConfigString(toml))
}

// NewProvider creates neohugo's goldmark converter provider for a config.
func NewProvider(c Config) (converter.Provider, markup_config.Config) {
	m, err := DecodeMarkup(c.TOML)
	if err != nil {
		panic(err)
	}
	p, err := goldmark.Provider.New(converter.ProviderConfig{
		Conf:   confStub{m: m},
		Logger: loggers.New(loggers.Options{StdOut: io.Discard, StdErr: io.Discard}),
	})
	if err != nil {
		panic(err)
	}
	return p, m
}

// Result is the outcome of one conversion: the HTML or the error text
// (Err) or the panic value (Panic).
type Result struct {
	HTML  B      `json:"html,omitempty"`
	Err   string `json:"err,omitempty"`
	Panic string `json:"panic,omitempty"`
}

// Convert runs a conversion, recording errors and panics.
func Convert(conv converter.Converter, rctx converter.RenderContext) (res Result, r converter.ResultRender) {
	defer func() {
		if p := recover(); p != nil {
			res = Result{Panic: fmt.Sprint(p)}
		}
	}()
	out, err := conv.Convert(rctx)
	if err != nil {
		return Result{Err: err.Error()}, nil
	}
	return Result{HTML: append(B{}, out.Bytes()...)}, out
}

// WriteGz writes v as gzip-compressed JSON (deterministic: fixed gzip
// header, no timestamps).
func WriteGz(path string, v any) {
	var raw bytes.Buffer
	enc := json.NewEncoder(&raw)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(v); err != nil {
		panic(err)
	}
	var b bytes.Buffer
	zw, err := gzip.NewWriterLevel(&b, gzip.BestCompression)
	if err != nil {
		panic(err)
	}
	if _, err := zw.Write(raw.Bytes()); err != nil {
		panic(err)
	}
	if err := zw.Close(); err != nil {
		panic(err)
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		panic(err)
	}
	if err := os.WriteFile(path, b.Bytes(), 0o644); err != nil {
		panic(err)
	}
	fmt.Printf("%s: %d bytes (%d raw)\n", path, b.Len(), raw.Len())
}
