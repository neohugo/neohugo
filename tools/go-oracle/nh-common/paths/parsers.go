package main

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"sort"

	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/testconfig"
)

// recorder wraps a parser's callbacks and records every distinct call.
type recorder struct {
	of map[[2]string]bool // IsOutputFormat(name, ext)
	ce map[string]bool    // IsContentExt(ext)
	ld map[string]bool    // IsLangDisabled(lang)
}

func newRecorder() *recorder {
	return &recorder{of: map[[2]string]bool{}, ce: map[string]bool{}, ld: map[string]bool{}}
}

func (r *recorder) wrap(pp *paths.PathParser) *paths.PathParser {
	c := *pp
	if pp.IsOutputFormat != nil {
		f := pp.IsOutputFormat
		c.IsOutputFormat = func(name, ext string) bool {
			v := f(name, ext)
			r.of[[2]string{name, ext}] = v
			return v
		}
	}
	if pp.IsContentExt != nil {
		f := pp.IsContentExt
		c.IsContentExt = func(ext string) bool {
			v := f(ext)
			r.ce[ext] = v
			return v
		}
	}
	if pp.IsLangDisabled != nil {
		f := pp.IsLangDisabled
		c.IsLangDisabled = func(lang string) bool {
			v := f(lang)
			r.ld[lang] = v
			return v
		}
	}
	return &c
}

// parser is one PathParser under test.
type parser struct {
	id  string
	pp  *paths.PathParser
	rec *recorder
}

type parserSet []parser

// describe returns the parsers' language indexes, which callbacks are nil and
// the recorded callback answers (sorted).
func (ps parserSet) describe() map[string]any {
	m := map[string]any{}
	for _, p := range ps {
		d := map[string]any{
			"languageIndex":   p.pp.LanguageIndex,
			"isLangDisabled":  p.pp.IsLangDisabled != nil,
			"isOutputFormat":  p.pp.IsOutputFormat != nil,
			"isContentExt":    p.pp.IsContentExt != nil,
			"outputFormatLog": [][]any{},
			"contentExtLog":   [][]any{},
			"langDisabledLog": [][]any{},
		}
		var ofk [][2]string
		for k := range p.rec.of {
			ofk = append(ofk, k)
		}
		sort.Slice(ofk, func(i, j int) bool {
			if ofk[i][0] != ofk[j][0] {
				return ofk[i][0] < ofk[j][0]
			}
			return ofk[i][1] < ofk[j][1]
		})
		var ofl [][]any
		for _, k := range ofk {
			ofl = append(ofl, []any{k[0], k[1], p.rec.of[k]})
		}
		if ofl != nil {
			d["outputFormatLog"] = ofl
		}
		d["contentExtLog"] = sortedLog(p.rec.ce)
		d["langDisabledLog"] = sortedLog(p.rec.ld)
		m[p.id] = d
	}
	return m
}

func sortedLog(m map[string]bool) [][]any {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	out := [][]any{}
	for _, k := range keys {
		out = append(out, []any{k, m[k]})
	}
	return out
}

// seeksnackParser loads the seeksnack config dump through allconfig and
// returns its ContentPathParser.
func seeksnackParser(root string) *paths.PathParser {
	b, err := os.ReadFile(filepath.Join(root, "docs/rust-port/specs/architecture-core-data/config-en.json"))
	if err != nil {
		log.Fatal(err)
	}
	var m map[string]any
	if err := json.Unmarshal(b, &m); err != nil {
		log.Fatal(err)
	}
	cfg := config.New()
	for _, k := range []string{
		"baseurl", "defaultcontentlanguage", "disablelanguages", "languages",
		"outputformats", "mediatypes", "contenttypes", "outputs", "taxonomies",
	} {
		cfg.Set(k, m[k])
	}
	confs := testconfig.GetTestConfigs(nil, cfg)
	return confs.GetFirstLanguageConfig().PathParser()
}

// testParser is common/paths/pathparser_test.go's testParser.
func testParser() *paths.PathParser {
	return &paths.PathParser{
		LanguageIndex: map[string]int{
			"no": 0,
			"en": 1,
			"fr": 2,
		},
		IsContentExt: func(ext string) bool {
			return ext == "md"
		},
		IsOutputFormat: func(name, ext string) bool {
			switch name {
			case "html", "amp", "csv", "rss":
				return true
			}
			return false
		},
	}
}

// noLangParser has a nil LanguageIndex and IsLangDisabled.
func noLangParser() *paths.PathParser {
	return &paths.PathParser{
		IsContentExt: func(ext string) bool {
			return ext == "md" || ext == "html"
		},
		IsOutputFormat: func(name, ext string) bool {
			return name == "html" || name == "json"
		},
	}
}

func newParsers(root string) parserSet {
	var ps parserSet
	for _, p := range []struct {
		id string
		pp *paths.PathParser
	}{
		{"seeksnack", seeksnackParser(root)},
		{"test", testParser()},
		{"nolang", noLangParser()},
	} {
		rec := newRecorder()
		ps = append(ps, parser{id: p.id, pp: rec.wrap(p.pp), rec: rec})
	}
	return ps
}
