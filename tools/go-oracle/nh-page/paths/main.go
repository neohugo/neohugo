// Command paths is the Go oracle for page.CreateTargetPaths,
// TargetPaths.RelPermalink and TargetPaths.PermalinkForOutputFormat
// (resources/page/page_paths.go) in crates/nh-page (Wave B task T11).
//
//	go run ./tools/go-oracle/nh-page/paths [-root .] [-out crates/nh-page/tests/fixtures/paths]
//
// The oracle runs itself again with `go run -overlay`: the overlay adds a
// recording hook to CreateTargetPaths (generated from the current
// resources/page/page_paths.go, see psupport.RunOverlaid), so EVERY
// TargetPathDescriptor a real neohugo build creates is captured with its
// result: page outputs, paginator pages and aliases, pagination URLs,
// standalone pages (404, sitemap, robots). The builds (psupport sites) are
// this repository's docs/ and hugolib/testsite sites and the synthetic sites
// (en/th with and without defaultContentLanguageInSubdir, multihost, uglyURLs
// global and per section, disablePathToLower, a baseURL with a path, every
// built-in and several custom output formats, url/slug front matter,
// permalinks with every token, the CM §9.4 seeksnack examples).
//
// Adversarial descriptors are derived from the recorded ones of every site:
// each base descriptor with every output format of the site, uglyURLs on and
// off, and url, slug/baseName, addends, expanded permalink and prefix
// variants; they are computed with direct CreateTargetPaths calls.
//
// Every *paths.Path is stored as its input and re-parsed with the site's
// recording PathParser (psupport.PathTable), so the Rust test rebuilds it
// with the same parser answers. Output: <site>.json.gz per site. Nothing here
// depends on the platform.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"path/filepath"
	"sort"
	"sync"

	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/resources/kinds"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// installHooks is set by the overlay-added hook file (see hookFile).
var installHooks func(record func(page.TargetPathDescriptor, page.TargetPaths))

const hookFile = `package main

import "github.com/neohugo/neohugo/resources/page"

func init() {
	installHooks = func(record func(page.TargetPathDescriptor, page.TargetPaths)) {
		page.OracleHookCreateTargetPaths = record
	}
}
`

var patches = []psupport.Patch{{
	File:   "resources/page/page_paths.go",
	After:  "func CreateTargetPaths(d TargetPathDescriptor) (tp TargetPaths) {",
	Insert: "\n\tif OracleHookCreateTargetPaths != nil {\n\t\tdefer func(d0 TargetPathDescriptor) { OracleHookCreateTargetPaths(d0, tp) }(d)\n\t}\n",
	Append: "\n// OracleHookCreateTargetPaths is set by the nh-page paths oracle.\nvar OracleHookCreateTargetPaths func(TargetPathDescriptor, TargetPaths)\n",
}}

type record struct {
	d  page.TargetPathDescriptor
	tp page.TargetPaths
}

var (
	mu        sync.Mutex
	recording bool
	records   []record
)

func recordTargetPaths(d page.TargetPathDescriptor, tp page.TargetPaths) {
	mu.Lock()
	defer mu.Unlock()
	if recording {
		records = append(records, record{d, tp})
	}
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-page/tests/fixtures/paths", "output directory")
	flag.Parse()

	if !psupport.IsChild() {
		if err := psupport.RunOverlaid(*root, "./tools/go-oracle/nh-page/paths", patches, "zz_hooks_overlay.go", hookFile, []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if installHooks == nil {
		log.Fatal("the recording hook is not installed")
	}
	installHooks(recordTargetPaths)

	sites, err := psupport.RepoSites(*root)
	if err != nil {
		log.Fatal(err)
	}
	sites = append(sites, psupport.SyntheticSites()...)
	total := 0
	for _, s := range sites {
		n, err := runSite(s, psupport.OutDir(*root, *out))
		if err != nil {
			log.Fatal(err)
		}
		total += n
	}
	log.Printf("paths: %d cases", total)
}

// siteTables are the fixture header tables of one build.
type siteTables struct {
	h         *hugolib.HugoSites
	psIndex   map[*helpers.PathSpec]int
	pathspecs []any
	formats   []any
	fmtIndex  map[string]int
	paths     *psupport.PathTable
}

func (t *siteTables) format(f output.Format) int {
	d := psupport.FormatDump(f)
	b, err := json.Marshal(d)
	if err != nil {
		panic(err)
	}
	if i, ok := t.fmtIndex[string(b)]; ok {
		return i
	}
	t.fmtIndex[string(b)] = len(t.formats)
	t.formats = append(t.formats, d)
	return len(t.formats) - 1
}

func str(s string) any { return goval.Str(s) }

// encode returns the descriptor and Go's results.
func (t *siteTables) encode(d page.TargetPathDescriptor, src string, result func() (page.TargetPaths, error)) map[string]any {
	ps, ok := t.psIndex[d.PathSpec]
	if !ok {
		panic("descriptor with an unknown PathSpec")
	}
	c := map[string]any{
		"src":               src,
		"ps":                ps,
		"type":              t.format(d.Type),
		"kind":              d.Kind,
		"path":              t.paths.Add(d.Path),
		"section":           t.paths.Add(d.Section),
		"baseName":          str(d.BaseName),
		"prefixFilePath":    str(d.PrefixFilePath),
		"prefixLink":        str(d.PrefixLink),
		"forcePrefix":       d.ForcePrefix,
		"url":               str(d.URL),
		"addends":           str(d.Addends),
		"expandedPermalink": str(d.ExpandedPermalink),
		"uglyURLs":          d.UglyURLs,
	}
	c["want"] = goval.CallRaw(func() (any, error) {
		tp, err := result()
		if err != nil {
			return nil, err
		}
		return map[string]any{
			"targetFilename":        str(tp.TargetFilename),
			"subResourceBaseTarget": str(tp.SubResourceBaseTarget),
			"subResourceBaseLink":   str(tp.SubResourceBaseLink),
			"link":                  str(tp.Link),
			"relPermalink":          str(tp.RelPermalink(d.PathSpec)),
			"permalink":             str(tp.PermalinkForOutputFormat(d.PathSpec, d.Type)),
		}, nil
	})
	return c
}

// Variation inputs for the adversarial descriptors.
var (
	advURLs = []string{
		"", "/", "/about/", "/about", "about/", "about", "/a/b/c.html", "a/b/c.html", "/a/b/c.xml",
		"/x.y/", "../escape/", "/a/../b/", "/a/./b", "..", "/..", "/ภาษา/หน้า/", "/Café Crème/",
		"/UPPER/Case/", "/with space/", "/a#frag/", "/q?x=1", "/%zz/", "/a%20b/",
		"https://example.com/x/", "/th/already/", "th/rel/", "/en/x/", "//double//slash//",
		"/index.html", "/feed.xml", "/sub/path/x/", "/.hidden/", "/trailing.dot./", "noext",
	}
	advBaseNames = []string{"", "index", "slug-x", "Ümlaut Slug", "a/b", "_index", "404", "sitemap", "feed", "data", "rootfile", "Mixed.Case"}
	advAddends   = []string{"", "/page/2", "/page/2/", "page/3", "/p/1", "/Page/Ü/"}
	advExpanded  = []string{"", "/2023/01/slug/", "/2023/01/slug", "/posts/x.html", "/section//", "/Upper/Ü/", "rel/perm/"}
	advPrefixes  = []struct {
		file, link string
		force      bool
	}{
		{"", "", false}, {"", "", true}, {"th", "th", false}, {"th", "th", true}, {"en", "", true}, {"sub/th", "th", false},
	}
)

// shortURLs are the url variants used with every output format.
var shortURLs = []string{"/", "about", "/a/b/c.html", "/th/x/", "../up/"}

// variations returns the adversarial variants of d for format f: all of
// them when full is set (the site's first format), else the base and a few
// url, addends and permalink variants.
func variations(d page.TargetPathDescriptor, f output.Format, full bool) []page.TargetPathDescriptor {
	var out []page.TargetPathDescriptor
	for _, ugly := range []bool{false, true} {
		b := d
		b.Type = f
		b.UglyURLs = ugly
		b.Addends = ""
		b.URL = ""
		b.ExpandedPermalink = ""
		out = append(out, b)
		if !full {
			for _, u := range shortURLs {
				v := b
				v.URL = u
				out = append(out, v)
			}
			v := b
			v.Addends = "/page/2"
			out = append(out, v)
			v.ExpandedPermalink = "/x/y/"
			out = append(out, v)
			v = b
			v.BaseName = f.BaseName
			out = append(out, v)
			continue
		}
		for _, u := range advURLs {
			v := b
			v.URL = u
			out = append(out, v)
		}
		for _, n := range advBaseNames {
			v := b
			v.BaseName = n
			out = append(out, v)
		}
		for _, a := range advAddends {
			v := b
			v.Addends = a
			out = append(out, v)
			v.URL = "/u/"
			out = append(out, v)
		}
		for _, e := range advExpanded {
			v := b
			v.ExpandedPermalink = e
			out = append(out, v)
			v.Addends = "/page/2"
			out = append(out, v)
		}
		for _, p := range advPrefixes {
			v := b
			v.PrefixFilePath, v.PrefixLink, v.ForcePrefix = p.file, p.link, p.force
			out = append(out, v)
			v.URL = "rel/"
			out = append(out, v)
		}
	}
	return out
}

func runSite(s psupport.Site, outDir string) (int, error) {
	mu.Lock()
	records = nil
	recording = true
	mu.Unlock()

	b, err := psupport.Build(s)

	mu.Lock()
	recording = false
	recs := records
	records = nil
	mu.Unlock()
	if err != nil {
		return 0, err
	}
	h := b.H

	t := &siteTables{
		h:        h,
		psIndex:  map[*helpers.PathSpec]int{},
		fmtIndex: map[string]int{},
		paths:    psupport.NewPathTable(h.Configs.ContentPathParser),
	}
	for i, st := range h.Sites {
		t.psIndex[st.PathSpec] = i
		t.pathspecs = append(t.pathspecs, psupport.PathSpecDump(st.Conf, st.PathSpec))
	}

	// Recorded descriptors, deduplicated.
	seen := map[string]bool{}
	var cases []map[string]any
	add := func(c map[string]any) {
		k, err := json.Marshal(c)
		if err != nil {
			panic(err)
		}
		if seen[string(k)] {
			return
		}
		seen[string(k)] = true
		cases = append(cases, c)
	}
	for _, r := range recs {
		r := r
		add(t.encode(r.d, "build", func() (page.TargetPaths, error) { return r.tp, nil }))
	}
	nBuild := len(cases)

	// Adversarial variants of one recorded descriptor per (kind, bundle,
	// url, prefix) class, over all the site's output formats.
	type class struct {
		kind, prefix string
		bundle, url  bool
		ps           int
	}
	bases := map[class]page.TargetPathDescriptor{}
	var classKeys []class
	sorted := append([]record(nil), recs...)
	sort.SliceStable(sorted, func(i, j int) bool {
		a, b := sorted[i].d, sorted[j].d
		if a.Path.Path() != b.Path.Path() {
			return a.Path.Path() < b.Path.Path()
		}
		return a.Type.Name < b.Type.Name
	})
	for _, r := range sorted {
		if r.d.Addends != "" {
			continue
		}
		k := class{kind: r.d.Kind, prefix: r.d.PrefixFilePath, bundle: r.d.Path.IsBundle(), url: r.d.URL != "", ps: t.psIndex[r.d.PathSpec]}
		if _, ok := bases[k]; ok {
			continue
		}
		bases[k] = r.d
		classKeys = append(classKeys, k)
	}
	formats := h.Configs.Base.OutputFormats.Config
	for _, k := range classKeys {
		base := bases[k]
		for i, f := range formats {
			for _, v := range variations(base, f, i == 0) {
				v := v
				add(t.encode(v, "adv", func() (page.TargetPaths, error) { return page.CreateTargetPaths(v), nil }))
			}
		}
	}

	header := map[string]any{
		"site":           s.Name,
		"pathspecs":      t.pathspecs,
		"formats":        t.formats,
		"paths":          t.paths.Entries,
		"parser":         t.paths.Describe(h.Configs.ContentPathParser),
		"pathMismatches": t.paths.Mismatches,
		"kinds":          []string{kinds.KindHome, kinds.KindSection, kinds.KindPage, kinds.KindTaxonomy, kinds.KindTerm},
	}
	if t.paths.Mismatches > 0 {
		return 0, fmt.Errorf("%s: %d paths do not re-parse to the same path", s.Name, t.paths.Mismatches)
	}
	log.Printf("%s: %d recorded descriptors (%d distinct), %d adversarial", s.Name, len(recs), nBuild, len(cases)-nBuild)
	return len(cases), goval.WriteCasesGz(filepath.Join(outDir, s.Name+".json.gz"), header, cases)
}
