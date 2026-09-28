// Command permalinks is the Go oracle for page.PermalinkExpander and
// page.DecodePermalinksConfig (resources/page/permalinks.go) in
// crates/nh-page (Wave B task T11).
//
//	go run ./tools/go-oracle/nh-page/permalinks [-root .] [-out crates/nh-page/tests/fixtures/permalinks]
//
// The pages are the real pages of the psupport builds (docs/,
// hugolib/testsite and the synthetic sites, whose content has varied dates,
// titles, slugs, file names, sections and bundles). For every page the
// attributes the expander reads are recorded (Kind, Date, Title, Slug,
// Section, CurrentSection().SectionsEntries/SectionsPath, the PathInfo and
// the File's path info through a recording PathParser), then:
//
//   - Expand(p.Section(), p) with the site's own [permalinks] config and with
//     a config using every token for every kind and section;
//   - ExpandPattern(pattern, p) for patterns using every :token, sections
//     slices (valid, out of range, reversed, malformed), Go date layouts,
//     escaped colons, unknown attributes and repeated tokens.
//
// DecodePermalinksConfig runs over the sites' [permalinks] sections, the
// seeksnack config dump (docs/rust-port/specs/architecture-core-data) and
// adversarial maps.
//
// Output: <site>.json.gz per site plus decode.json.gz. Nothing here depends on
// the platform.
package main

import (
	"encoding/json"
	"flag"
	"log"
	"os"
	"path/filepath"
	"sort"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// Patterns for ExpandPattern.
var patterns = []string{
	":year", ":month", ":monthname", ":day", ":weekday", ":weekdayname", ":yearday",
	":section", ":sections", ":title", ":slug", ":slugorfilename", ":filename",
	":contentbasename", ":slugorcontentbasename",
	":sections[1:]", ":sections[:1]", ":sections[0]", ":sections[1]", ":sections[last]",
	":sections[5]", ":sections[-1]", ":sections[1:last]", ":sections[ 1 : 2 ]", ":sections[]",
	":sections[x]", ":sections[1:2:3]", ":sections[LAST]", ":sections[]]", ":sections[0][1]",
	":2006", ":Jan", ":02", ":Monday", ":15", ":06-01-02", ":Jan2", ":MST",
	":unknown", ":yearsuffix", "/posts/:year/:month/:title/", "\\:escaped/:slug",
	"no-tokens", ":title:slug", ":slug/:slug", "/:year/:month/:day/:title.html",
	":section/:filename/", ":sections[1:]/:title", "/a/:sections[0]/b/:sections[last]/c",
	"", "/", ":", "::title", ":_x", ":title-:slug-:filename",
}

// allTokens is an expander config using every token, for every kind.
func allTokens() map[string]map[string]string {
	m := map[string]string{
		"": "/:year/:month/:day/:title/",
		// Not "/": it trims to the same key as "" and Go would keep a random one.
		" /x/ ":     "/trimmed/:slug/",
		"blog":      "/:sections[1:]/:year/:monthname/:day/:slug/",
		"news":      "/n/:year-:month-:day/:weekday-:weekdayname-:yearday/:filename/",
		"docs":      "/d/:sections/:slugorfilename/",
		"misc":      "/m/:section/:contentbasename/:slugorcontentbasename/",
		"dated":     "/dated/:2006/:Jan/:02/:title/",
		"esc":       "/e/\\:literal/:sections[last]/:sections[0]/:sections[:1]/:sections[1]/",
		"tags":      "/topics/:slug/",
		"biscuit":   "/b/:title/:section/",
		"slugs":     "/s/:slug/:slugorfilename/:filename/",
		"dates":     "/dt/:year/:yearday/",
		"url-pages": "/u/:slugorcontentbasename/",
	}
	return map[string]map[string]string{"page": m, "section": m, "taxonomy": m, "term": m}
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-page/tests/fixtures/permalinks", "output directory")
	flag.Parse()
	outDir := psupport.OutDir(*root, *out)

	sites, err := psupport.RepoSites(*root)
	if err != nil {
		log.Fatal(err)
	}
	sites = append(sites, psupport.SyntheticSites()...)
	total := 0
	var decodeCases []map[string]any
	for _, s := range sites {
		n, dc, err := runSite(s, outDir)
		if err != nil {
			log.Fatal(err)
		}
		total += n
		decodeCases = append(decodeCases, dc)
	}
	decodeCases = append(decodeCases, decodeAdversarial(*root)...)
	if err := goval.WriteCasesGz(filepath.Join(outDir, "decode.json.gz"), map[string]any{}, decodeCases); err != nil {
		log.Fatal(err)
	}
	log.Printf("permalinks: %d page cases, %d decode cases", total, len(decodeCases))
}

func str(s string) any { return goval.Str(s) }

func result(s string, err error) any {
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	return map[string]any{"ok": str(s)}
}

func decodeCase(name string, m map[string]any) map[string]any {
	res := goval.CallRaw(func() (any, error) {
		c, err := page.DecodePermalinksConfig(m)
		if err != nil {
			return nil, err
		}
		return c, nil
	})
	return map[string]any{"name": name, "in": goval.Encode(maps.Params(m)), "want": res}
}

func decodeAdversarial(root string) []map[string]any {
	var out []map[string]any
	b, err := os.ReadFile(filepath.Join(root, "docs/rust-port/specs/architecture-core-data/config-en.json"))
	if err != nil {
		log.Fatal(err)
	}
	var dump map[string]any
	if err := json.Unmarshal(b, &dump); err != nil {
		log.Fatal(err)
	}
	pm, _ := maps.ToParamsAndPrepare(dump["permalinks"])
	out = append(out, decodeCase("config-en.json", pm))
	ins := []map[string]any{
		{"posts": "/posts/:year/:month/:title"},
		{"Posts": "/p/", "page": maps.Params{"blog": "/b/:slug/"}},
		{"page": maps.Params{"a": "/a/", "B": "/b/"}, "section": maps.Params{"s": "/s/"}, "taxonomy": maps.Params{"t": "/t/"}, "term": maps.Params{"x": "/x/"}},
		{"home": maps.Params{"a": "/a/"}},
		{"page": maps.Params{"a": 42}},
		{"posts": 42},
		{"posts": true},
		{"posts": []any{"a"}},
		{"page": map[string]any{"a": "/a/"}},
		{},
		// Not {"x": ..., "page": {"x": ...}}: the result depends on Go's random map order.
		{"x": "/x/:title/", "page": maps.Params{"y": "/py/"}},
	}
	for i, m := range ins {
		out = append(out, decodeCase("adv"+string(rune('a'+i)), m))
	}
	return out
}

func runSite(s psupport.Site, outDir string) (int, map[string]any, error) {
	b, err := psupport.Build(s)
	if err != nil {
		return 0, nil, err
	}
	h := b.H
	table := psupport.NewPathTable(h.Configs.ContentPathParser)

	var pathspecs []any
	for _, st := range h.Sites {
		pathspecs = append(pathspecs, psupport.PathSpecDump(st.Conf, st.PathSpec))
	}

	siteCfg := h.Configs.Base.Permalinks
	var cases []map[string]any
	for si, st := range h.Sites {
		urlize := st.URLize
		siteExp, err := page.NewPermalinkExpander(urlize, siteCfg)
		if err != nil {
			return 0, nil, err
		}
		allExp, err := page.NewPermalinkExpander(urlize, allTokens())
		if err != nil {
			return 0, nil, err
		}
		pages := st.Pages()
		sorted := append(page.Pages(nil), pages...)
		sort.SliceStable(sorted, func(i, j int) bool { return sorted[i].Path() < sorted[j].Path() })
		for _, p := range sorted {
			c := map[string]any{
				"ps":       si,
				"kind":     p.Kind(),
				"date":     goval.Encode(p.Date()),
				"title":    str(p.Title()),
				"slug":     str(p.Slug()),
				"section":  str(p.Section()),
				"pathInfo": table.Add(p.PathInfo()),
			}
			if f := p.File(); f != nil {
				c["file"] = map[string]any{
					"pathInfo":            table.Add(f.FileInfo().Meta().PathInfo),
					"dir":                 str(f.Dir()),
					"translationBaseName": str(f.TranslationBaseName()),
				}
			}
			cs := p.CurrentSection()
			c["sectionsEntries"] = goval.Encode(cs.SectionsEntries())
			c["sectionsPath"] = str(cs.SectionsPath())
			c["expandSite"] = result(siteExp.Expand(p.Section(), p))
			c["expandAll"] = result(allExp.Expand(p.Section(), p))
			var pr []any
			for _, pat := range patterns {
				pr = append(pr, goval.CallRaw(func() (any, error) {
					s, err := allExp.ExpandPattern(pat, p)
					if err != nil {
						return nil, err
					}
					return str(s), nil
				}))
			}
			c["patterns"] = pr
			cases = append(cases, c)
		}
	}
	header := map[string]any{
		"site":       s.Name,
		"pathspecs":  pathspecs,
		"paths":      table.Entries,
		"parser":     table.Describe(h.Configs.ContentPathParser),
		"siteConfig": siteCfg,
		"allConfig":  allTokens(),
		"patterns":   patterns,
	}
	if table.Mismatches > 0 {
		log.Fatalf("%s: %d paths do not re-parse", s.Name, table.Mismatches)
	}
	dc := decodeCase("site:"+s.Name, h.Configs.LoadingInfo.Cfg.GetStringMap("permalinks"))
	log.Printf("%s: %d pages", s.Name, len(cases))
	return len(cases), dc, goval.WriteCasesGz(filepath.Join(outDir, s.Name+".json.gz"), header, cases)
}
