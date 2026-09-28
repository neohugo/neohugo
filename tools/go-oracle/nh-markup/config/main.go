// Command config is the Go oracle for the markup configuration of
// crates/nh-markup (Wave B task T06): markup_config.Decode (mapstructure
// WeakDecode, normalizeConfig, Init, highlight.ApplyLegacyConfig) of TOML
// site configs, and the converter registry of markup.NewConverterProvider
// (Get, IsGoldmark, the defaultMarkdownHandler error), ResolveMarkup and the
// Chroma lexer lookup (markup/highlight/chromalexers.Get).
//
//	go run ./tools/go-oracle/nh-markup/config [-out crates/nh-markup/tests/fixtures/config]
package main

import (
	"flag"
	"path/filepath"

	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/markup"
	"github.com/neohugo/neohugo/markup/converter"
	"github.com/neohugo/neohugo/markup/highlight/chromalexers"
	"github.com/neohugo/neohugo/markup/markup_config"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-markup/mdoracle"
)

// cases are the TOML site configs decoded.
var cases = []string{
	``,
	`[markup]`,
	`markup = "not a map"`,
	`[markup.goldmark.extensions]
typographer = true`,
	`[markup.goldmark.extensions]
typographer = false`,
	`[markup.goldmark.extensions.typographer]
disable = true
leftSingleQuote = "<"`,
	`[markup.goldmark.parser]
attribute = true`,
	`[markup.goldmark.parser]
attribute = false`,
	`[markup.goldmark.parser]
autoHeadingIDType = "blackfriday"
autoIDType = "github"`,
	`[markup.goldmark.parser]
autoDefinitionTermID = true
[markup.goldmark.extensions]
definitionList = false`,
	`[markup.goldmark.parser]
autoDefinitionTermID = true`,
	`[markup.goldmark.extensions]
table = "false"
linkify = 0
strikethrough = "1"
footnote = "nope"`,
	`[markup.goldmark.extensions]
table = [1, 2]`,
	`[markup.tableOfContents]
startLevel = "1"
endLevel = 4.0
ordered = "true"`,
	`[markup.tableOfContents]
startLevel = 1.5`,
	`[markup.highlight]
style = "dracula"
lineNos = true
hl_Lines = "1 3-4"
lineNoStart = 5
tabWidth = "8"
noClasses = false
guessSyntax = true
codeFences = false
hl_lines_parsed = [[1, 2], [4, 6]]`,
	`pygmentsStyle = "friendly"
pygmentsUseClasses = true
pygmentsCodeFences = false
pygmentsCodefencesGuessSyntax = true
pygmentsOptions = "linenos=inline, hl_lines = 1 2,LINENOSTART=3"`,
	`pygmentsOptions = "linenos=table"`,
	`pygmentsOptions = "linenos=false,style=x"`,
	`pygmentsOptions = "linenos"`,
	`pygmentsOptions = "  "`,
	`pygmentsOptions = "tabWidth=abc"`,
	`pygmentsStyle = "friendly"
[markup.highlight]
style = "dracula"`,
	`[markup.goldmark.extensions.passthrough]
enable = true
[markup.goldmark.extensions.passthrough.delimiters]
inline = [["$", "$"], ["\\(", "\\)"]]
block = [["$$", "$$"]]`,
	`[markup.goldmark.renderHooks.image]
enableDefault = true
useEmbedded = "always"
[markup.goldmark.renderHooks.link]
enableDefault = false`,
	`[markup]
defaultMarkdownHandler = "Blackfriday"
_merge = "deep"
[markup.goldmark]
_merge = "shallow"
duplicateResourceFiles = true
[markup.goldmark.extensions.cjk]
enable = true
eastAsianLineBreaksStyle = "css3draft"
[markup.goldmark.extensions.extras.mark]
enable = true`,
	`[markup.goldmark.renderer]
unsafe = "yes"`,
	`[markup.goldmark]
parser = "x"`,
	`[markup.goldmark.parser.attribute]
title = "t"`,
	// Last: mapstructure decodes the attributes into the map of the global
	// asciidocext_config.Default (Go maps are references), so every later
	// Decode in the process would see them (not reproduced by the port).
	`[markup.asciidocExt]
backend = "docbook5"
extensions = ["a", "b"]
workingFolderCurrent = true
preserveTOC = true
[markup.asciidocExt.attributes]
my-attr = "v"
Other = 1`,
}

type confStub struct {
	config.AllProvider
	m markup_config.Config
}

func (c confStub) GetConfigSection(s string) any { return c.m }
func (c confStub) EnableEmoji() bool             { return false }
func (c confStub) ContentTypes() config.ContentTypesProvider {
	return media.DefaultContentTypes
}

type decoded struct {
	TOML string `json:"toml"`
	Dump string `json:"dump,omitempty"`
	Err  string `json:"err,omitempty"`
}

type registry struct {
	Handler string            `json:"handler"`
	Err     string            `json:"err,omitempty"`
	Get     map[string]string `json:"get,omitempty"`
	IsGM    map[string]bool   `json:"is_goldmark,omitempty"`
}

type fixture struct {
	Decoded    []decoded         `json:"decoded"`
	Registries []registry        `json:"registries"`
	Resolve    map[string]string `json:"resolve"`
	// Chroma is chromalexers.Get(name) != nil for lexer names, aliases,
	// extensions and file names.
	Chroma map[string]bool `json:"chroma"`
}

var lexerNames = []string{
	"go", "Go", "GO", "golang", "bash", "sh", "zsh", "shell", "console", "py", "python", "Python3",
	"js", "javascript", "jsx", "ts", "tsx", "html", "HTML", "htm", "xml", "svg", "css", "scss",
	"json", "yaml", "yml", "toml", "md", "markdown", "go-html-template", "go-text-template",
	"tmpl", "gotmpl", "c", "cpp", "c++", "h", "hpp", "rs", "rust", "java", "kotlin", "kt",
	"Makefile", "makefile", "GNUmakefile", "Dockerfile", "dockerfile", "docker", "CMakeLists.txt",
	"cmake", "nginx", "nginx.conf", "Caddyfile", "vim", ".vimrc", "vimrc", "x.go", "dir/x.py",
	"dir/", "/", ".bashrc", "PKGBUILD", "foo.go.bak", "foo.py~", "x.in", "Makefile.in",
	"go.mod", "go.sum", "text", "plaintext", "txt", "none", "nohook", "errlang", "dmylang", "d",
	"", "-", "*", "[", "a[", "fish", "ps1", "powershell", "sql", "SQL", "diff", "patch", "ini",
	"cfg", "properties", "graphql", "gql", "hcl", "tf", "terraform", "lua", "rb", "ruby", "php",
	"php3", "erb", "haskell", "hs", "elixir", "ex", "exs", "clojure", "clj", "edn", "ocaml",
	"ml", "fsharp", "fs", "swift", "objective-c", "objc", "m", "tex", "latex", "bib", "r", "R",
	"julia", "jl", "matlab", "octave", "perl", "pl", "pm", "raku", "zig", "nim", "v", "sv",
	"verilog", "vhdl", "asm", "nasm", "gas", "wasm", "wat", "proto", "protobuf", "thrift",
	"avro", "csv", "tsv", "mermaid", "goat", "chroma", "Thai", "ไทย", "日本語",
}

var names = []string{"md", "markdown", "goldmark", "MD", "Markdown", "mdown", "html", "htm", "asciidoc", "asciidocext", "adoc", "ad", "rst", "pandoc", "pdc", "org", "blackfriday", "", "nope"}

func main() {
	out := flag.String("out", "crates/nh-markup/tests/fixtures/config", "output directory")
	flag.Parse()

	var fx fixture
	for _, c := range cases {
		m, err := markup_config.Decode(config.FromTOMLConfigString(c))
		d := decoded{TOML: c}
		if err != nil {
			d.Err = err.Error()
		} else {
			d.Dump = mdoracle.DumpMarkupConfig(m)
		}
		fx.Decoded = append(fx.Decoded, d)
	}

	for _, h := range []string{"goldmark", "GoldMark", "blackfriday", "nope", ""} {
		m, err := markup_config.Decode(config.FromTOMLConfigString("[markup]\ndefaultMarkdownHandler = \"" + h + "\""))
		if err != nil {
			panic(err)
		}
		r := registry{Handler: h}
		cp, err := markup.NewConverterProvider(converter.ProviderConfig{
			Conf:   confStub{m: m},
			Logger: loggers.NewDefault(),
		})
		if err != nil {
			r.Err = err.Error()
		} else {
			r.Get = map[string]string{}
			r.IsGM = map[string]bool{}
			for _, n := range names {
				if p := cp.Get(n); p != nil {
					r.Get[n] = p.Name()
				}
				r.IsGM[n] = cp.IsGoldmark(n)
			}
		}
		fx.Registries = append(fx.Registries, r)
	}

	fx.Resolve = map[string]string{}
	for _, n := range names {
		fx.Resolve[n] = markup.ResolveMarkup(n)
	}

	fx.Chroma = map[string]bool{}
	for _, n := range lexerNames {
		fx.Chroma[n] = chromalexers.Get(n) != nil
	}

	mdoracle.WriteGz(filepath.Join(*out, "config.json.gz"), fx)
}
