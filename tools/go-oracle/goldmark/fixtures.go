package main

import (
	"bufio"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strconv"
	"strings"

	"github.com/yuin/goldmark/util"
)

type specCase struct {
	Markdown string `json:"markdown"`
	HTML     string `json:"html"`
	Example  int    `json:"example"`
}

func writeFile(path string, f func(g *gmfWriter)) {
	fp, err := os.Create(path)
	if err != nil {
		panic(err)
	}
	defer fp.Close()
	if strings.HasSuffix(path, ".gz") {
		zw, _ := gzip.NewWriterLevel(fp, gzip.BestCompression)
		g := newGMF(zw)
		f(g)
		g.flush()
		if err := zw.Close(); err != nil {
			panic(err)
		}
		return
	}
	g := newGMF(fp)
	f(g)
	g.flush()
}

// testutil.MarkdownTestCase parsing, copied from goldmark/testutil
// (DoTestCaseFile, source, expected, applyEscapeSequence).
type tcase struct {
	No           int
	Description  string
	EnableEscape bool
	Trim         bool
	Markdown     string
	Expected     string
}

const attributeSeparator = "//- - - - - - - - -//"
const caseSeparator = "//= = = = = = = = = = = = = = = = = = = = = = = =//"

var optionsRegexp = regexp.MustCompile(`(?i)\s*options:(.*)`)

func parseCaseFile(filename string) []tcase {
	fp, err := os.Open(filename)
	if err != nil {
		panic(err)
	}
	defer fp.Close()
	scanner := bufio.NewScanner(fp)
	var cases []tcase
	for scanner.Scan() {
		if util.IsBlank([]byte(scanner.Text())) {
			continue
		}
		c := tcase{}
		header := scanner.Text()
		if strings.Contains(header, ":") {
			parts := strings.Split(header, ":")
			c.No, err = strconv.Atoi(strings.TrimSpace(parts[0]))
			c.Description = strings.Join(parts[1:], ":")
		} else {
			c.No, err = strconv.Atoi(scanner.Text())
		}
		if err != nil {
			panic(err)
		}
		scanner.Scan()
		matches := optionsRegexp.FindAllStringSubmatch(scanner.Text(), -1)
		if len(matches) != 0 {
			var o struct {
				EnableEscape bool
				Trim         bool
			}
			if err := json.Unmarshal([]byte(matches[0][1]), &o); err != nil {
				panic(err)
			}
			c.EnableEscape, c.Trim = o.EnableEscape, o.Trim
			scanner.Scan()
		}
		if scanner.Text() != attributeSeparator {
			panic("bad separator")
		}
		buf := []string{}
		for scanner.Scan() {
			text := scanner.Text()
			if text == attributeSeparator {
				break
			}
			buf = append(buf, text)
		}
		c.Markdown = strings.Join(buf, "\n")
		buf = []string{}
		for scanner.Scan() {
			text := scanner.Text()
			if text == caseSeparator {
				break
			}
			buf = append(buf, text)
		}
		c.Expected = strings.Join(buf, "\n")
		if len(c.Expected) != 0 {
			c.Expected = c.Expected + "\n"
		}
		cases = append(cases, c)
	}
	return cases
}

func (t *tcase) source() string {
	ret := t.Markdown
	if t.Trim {
		ret = strings.TrimSpace(ret)
	}
	if t.EnableEscape {
		return string(applyEscapeSequence([]byte(ret)))
	}
	return ret
}

func (t *tcase) expected() string {
	ret := t.Expected
	if t.Trim {
		ret = strings.TrimSpace(ret)
	}
	if t.EnableEscape {
		return string(applyEscapeSequence([]byte(ret)))
	}
	return ret
}

func applyEscapeSequence(b []byte) []byte {
	result := make([]byte, 0, len(b))
	for i := 0; i < len(b); i++ {
		if b[i] == '\\' && i != len(b)-1 {
			switch b[i+1] {
			case 'a':
				result = append(result, '\a')
				i++
				continue
			case 'b':
				result = append(result, '\b')
				i++
				continue
			case 'f':
				result = append(result, '\f')
				i++
				continue
			case 'n':
				result = append(result, '\n')
				i++
				continue
			case 'r':
				result = append(result, '\r')
				i++
				continue
			case 't':
				result = append(result, '\t')
				i++
				continue
			case 'v':
				result = append(result, '\v')
				i++
				continue
			case '\\':
				result = append(result, '\\')
				i++
				continue
			case 'x':
				if len(b) >= i+3 && util.IsHexDecimal(b[i+2]) && util.IsHexDecimal(b[i+3]) {
					v, _ := strconv.ParseUint(string(b[i+2:i+4]), 16, 8)
					result = append(result, byte(v))
					i += 3
					continue
				}
			case 'u', 'U':
				if len(b) > i+2 {
					num := []byte{}
					for j := i + 2; j < len(b); j++ {
						if util.IsHexDecimal(b[j]) {
							num = append(num, b[j])
							continue
						}
						break
					}
					if len(num) >= 4 && len(num) < 8 {
						v, _ := strconv.ParseInt(string(num[:4]), 16, 32)
						result = append(result, []byte(string(rune(v)))...)
						i += 5
						continue
					}
					if len(num) >= 8 {
						v, _ := strconv.ParseInt(string(num[:8]), 16, 32)
						result = append(result, []byte(string(rune(v)))...)
						i += 9
						continue
					}
				}
			}
		}
		result = append(result, b[i])
	}
	return result
}

func fixturesMain(args []string) {
	out := args[0]
	gm := goldmarkDir()

	// CommonMark spec examples, goldmark's TestSpec configuration ("xu") plus
	// the default configuration.
	bs, err := os.ReadFile(filepath.Join(gm, "_test", "spec.json"))
	if err != nil {
		panic(err)
	}
	var spec []specCase
	if err := json.Unmarshal(bs, &spec); err != nil {
		panic(err)
	}
	writeFile(filepath.Join(out, "spec.gmf"), func(g *gmfWriter) {
		for _, c := range spec {
			for _, cfg := range []string{"xu", "default", "attr"} {
				g.record(fmt.Sprintf("spec/%s/%d", cfg, c.Example))
				g.field("cfg", []byte(cfg))
				g.field("md", []byte(c.Markdown))
				if cfg == "xu" {
					g.field("spec", []byte(c.HTML))
				}
				g.field("html", convert(cfg, []byte(c.Markdown)))
			}
		}
	})

	// extra.txt (TestExtras, "xu") and options.txt (TestAttributeAndAutoHeadingID, "attr").
	for _, f := range []struct{ file, cfg string }{{"extra.txt", "xu"}, {"options.txt", "attr"}} {
		cases := parseCaseFile(filepath.Join(gm, "_test", f.file))
		name := strings.TrimSuffix(f.file, ".txt")
		writeFile(filepath.Join(out, name+".gmf"), func(g *gmfWriter) {
			for _, c := range cases {
				for _, cfg := range []string{f.cfg, "default", "all"} {
					g.record(fmt.Sprintf("%s/%s/%d", name, cfg, c.No))
					g.field("cfg", []byte(cfg))
					g.field("md", []byte(c.source()))
					if cfg == f.cfg {
						g.field("spec", []byte(c.expected()))
					}
					g.field("html", convert(cfg, []byte(c.source())))
				}
			}
		})
	}

	// Hand-written edge cases, every configuration.
	writeFile(filepath.Join(out, "edge.gmf"), func(g *gmfWriter) {
		for i, md := range edgeCases {
			for _, cfg := range configNames {
				g.record(fmt.Sprintf("edge/%s/%d", cfg, i))
				g.field("cfg", []byte(cfg))
				g.field("md", []byte(md))
				g.field("html", convert(cfg, []byte(md)))
			}
		}
	})

	// A fixed-seed fuzz corpus (checked in, compressed).
	writeFile(filepath.Join(out, "fuzz.gmf.gz"), func(g *gmfWriter) {
		writeFuzz(g, 3000, 20260927, configNames)
	})

	// The plugin-API test extension (plugin.go / tests/plugin_api.rs).
	writeFile(filepath.Join(out, "plugin.gmf.gz"), func(g *gmfWriter) {
		for i, md := range edgeCases {
			for _, cfg := range []string{"plugin", "plugin-unsafe"} {
				g.record(fmt.Sprintf("plugin-edge/%s/%d", cfg, i))
				g.field("cfg", []byte(cfg))
				g.field("md", []byte(md))
				g.field("html", convert(cfg, []byte(md)))
			}
		}
		writeFuzzMode(g, 4000, 99, []string{"plugin", "plugin-unsafe"}, "plugin")
	})

	writeFile(filepath.Join(out, "util.gmf.gz"), writeUtilVectors)
	writeFile(filepath.Join(out, "regex.gmf.gz"), writeRegexVectors)
	writeFile(filepath.Join(out, "cjk.gmf"), writeCJK)
}
