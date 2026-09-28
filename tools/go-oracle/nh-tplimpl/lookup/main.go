// Command lookup is the Go oracle for the template lookups of the store in
// crates/nh-tplimpl (Wave B task T13): LookupPagesLayout (pages, aliases,
// render hooks), LookupPartial, LookupShortcode and LookupShortcodeByName.
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplimpl/lookup [-root .] [-out crates/nh-tplimpl/tests/fixtures/lookup]
//
// The oracle runs itself again with `go run -overlay` (tsupport.RunOverlaid),
// which adds recording hooks to the lookups: every query is logged before it
// is initialised, with every call of its Consider func (the candidate and the
// answer) and the template chosen (as the store oracle's template id). For
// every site (tsupport.AllSites) it records
//
//   - every lookup of a real build: the synthetic sites and hugolib/testsite
//     are rendered (pages and paginator pages, aliases, render hooks,
//     partials, shortcodes); docs/ resolves the template of every page and
//     output format without executing it (its templates fetch remote data);
//   - grid lookups made directly: every query path of the build and every
//     tree key with every page kind, custom layout (with and without
//     LayoutFromUserMustMatch), output format and language; render hooks of
//     every kind and variant; partial names with and without extension;
//     shortcode names with every output format, plain text and
//     AlwaysAllowPlainText; shortcode names.
//
// Output: <site>.json.gz (records sorted, so it regenerates byte for byte).
package main

import (
	"encoding/json"
	"flag"
	"log"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-tplimpl/tsupport"
	"github.com/neohugo/neohugo/tpl/tplimpl"
)

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-tplimpl/tests/fixtures/lookup", "output directory")
	flag.Parse()

	if !tsupport.IsChild() {
		if err := tsupport.RunOverlaid(*root, "./tools/go-oracle/nh-tplimpl/lookup", []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if tsupport.Oracle == nil {
		log.Fatal("the overlay is not installed")
	}

	sites, err := tsupport.AllSites(*root)
	if err != nil {
		log.Fatal(err)
	}
	fx := tsupport.NewFixtures(tsupport.OutDir(*root, *out))
	for _, s := range sites {
		if err := runSite(s, fx); err != nil {
			log.Fatal(err)
		}
	}
	if err := fx.Close(); err != nil {
		log.Fatal(err)
	}
}

func runSite(s tsupport.Site, fx *tsupport.Fixtures) error {
	b, err := tsupport.New(s)
	if err != nil {
		return err
	}
	rec := tsupport.NewLookupRecorder()
	tsupport.Oracle.InstallLookupHooks(rec)
	defer tsupport.Oracle.InstallLookupHooks(nil)
	if err := b.Build(); err != nil {
		return err
	}
	nBuild := len(rec.Records)
	nBuildOrderDependent := 0
	tsupport.Oracle.InstallLookupHooks(nil)

	store := b.H.Sites[0].TemplateStore
	// Is a layout lookup of the build order dependent?
	tsupport.Oracle.SetReverseOrder(true)
	for _, r := range rec.Records {
		if r["op"] != "pages" || r["hadConsider"] == true {
			continue
		}
		q := tsupport.QueryFromDump(r["query"].(map[string]any))
		if rid := tsupport.Oracle.TemplID(store, store.LookupPagesLayout(q)); rid != r["result"] {
			r["reverseResult"] = rid
			nBuildOrderDependent++
		}
	}
	tsupport.Oracle.SetReverseOrder(false)
	grid(b, store, rec)

	records := rec.Records
	keys := make([]string, len(records))
	for i, r := range records {
		if c, ok := r["considered"].([][]any); ok {
			sort.Slice(c, func(i, j int) bool { return c[i][0].(string) < c[j][0].(string) })
		}
		bb, err := json.Marshal(r)
		if err != nil {
			return err
		}
		keys[i] = string(bb)
	}
	sort.Sort(byKey{records, keys})
	if err := fx.Add(s, map[string]any{
		"site":    s.Name,
		"records": records,
	}); err != nil {
		return err
	}
	counts := map[string]int{}
	for _, r := range records {
		counts[r["op"].(string)]++
		if _, ok := r["reverseResult"]; ok {
			counts["orderDependent"]++
		}
	}
	log.Printf("lookup: %s: %d records (%d from the build, %d of them order dependent): %v", s.Name, len(records), nBuild, nBuildOrderDependent, counts)
	return nil
}

type byKey struct {
	r []map[string]any
	k []string
}

func (b byKey) Len() int           { return len(b.r) }
func (b byKey) Less(i, j int) bool { return b.k[i] < b.k[j] }
func (b byKey) Swap(i, j int) {
	b.r[i], b.r[j] = b.r[j], b.r[i]
	b.k[i], b.k[j] = b.k[j], b.k[i]
}

// lookups makes direct lookups and records them like the hooks do, with the
// answer in the reverse candidate order when it differs (an order-dependent
// query: Go's own answer changes from run to run).
type lookups struct {
	store *tplimpl.TemplateStore
	rec   *tsupport.LookupRecorder
	id    func(*tplimpl.TemplInfo) string
}

func (l lookups) pages(q tplimpl.TemplateQuery) {
	r := map[string]any{"op": "pages", "query": tsupport.QueryDump(q), "hadConsider": false, "considered": nil}
	id := l.id(l.store.LookupPagesLayout(q))
	r["result"] = id
	tsupport.Oracle.SetReverseOrder(true)
	if rid := l.id(l.store.LookupPagesLayout(q)); rid != id {
		r["reverseResult"] = rid
	}
	tsupport.Oracle.SetReverseOrder(false)
	l.rec.Add(r)
}

func (l lookups) shortcode(q tplimpl.TemplateQuery) {
	r := map[string]any{"op": "shortcode", "query": tsupport.QueryDump(q), "hadConsider": false, "considered": nil}
	ti, err := l.store.LookupShortcode(q)
	id := l.id(ti)
	r["result"] = id
	r["err"] = ""
	if err != nil {
		r["err"] = err.Error()
	}
	tsupport.Oracle.SetReverseOrder(true)
	ti, _ = l.store.LookupShortcode(q)
	if rid := l.id(ti); rid != id {
		r["reverseResult"] = rid
	}
	tsupport.Oracle.SetReverseOrder(false)
	l.rec.Add(r)
}

// grid makes the direct lookups (see the package doc).
func grid(b *tsupport.Built, store *tplimpl.TemplateStore, rec *tsupport.LookupRecorder) {
	lk := lookups{store: store, rec: rec, id: func(ti *tplimpl.TemplInfo) string { return tsupport.Oracle.TemplID(store, ti) }}
	pathSet := map[string]bool{"": true, "/": true, "/nosuch": true, "/blog/nosuch/deep": true}
	scPathSet := map[string]bool{"": true, "/docs/d1": true, "/blog/post1": true}
	// The query paths of the build: all of them for the small sites, every
	// 40th (sorted) for docs.
	buildPaths := map[string]bool{}
	for _, r := range rec.Records {
		if q, ok := r["query"].(map[string]any); ok {
			buildPaths[q["path"].(string)] = true
		}
	}
	for i, p := range sortedKeys(buildPaths) {
		if len(buildPaths) < 100 || i%40 == 0 {
			pathSet[p] = true
		}
	}
	small := b.Site.SmallGrid
	dump, err := tsupport.Oracle.StoreDump(store)
	if err != nil {
		log.Fatal(err)
	}
	var partialNames, scNames []string
	for _, e := range dump["main"].([]map[string]any) {
		k := e["key"].(string)
		if strings.HasPrefix(k, "/_partials/") {
			n := strings.TrimPrefix(k, "/_partials/")
			partialNames = append(partialNames, n, n+".html")
			continue
		}
		pathSet[k] = true
	}
	for _, e := range dump["shortcodes"].([]map[string]any) {
		scNames = append(scNames, e["scName"].(string))
		scPathSet[e["key"].(string)] = true
	}
	partialNames = append(partialNames, "nosuch.html", "nosuch", "partials/footer.html", "footer.json", "json.json", "text.txt", "text", "_funcs/f", "/footer.html", "Footer.html")
	scNames = append(scNames, "nosuch", "NOTE", "tweet")

	paths := sortedKeys(pathSet)
	scPaths := sortedKeys(scPathSet)

	conf := b.H.Configs.Base
	var formats []tplimpl.TemplateDescriptor
	for _, name := range []string{"html", "rss", "json", "plain", "fancy", "alias", "404"} {
		of, ok := conf.OutputFormats.Config.GetByName(name)
		if !ok {
			continue
		}
		formats = append(formats, tplimpl.TemplateDescriptor{
			OutputFormat: of.Name,
			MediaType:    of.MediaType.Type,
			IsPlainText:  of.IsPlainText,
		})
	}
	var langs []string
	for _, l := range b.H.Configs.Languages {
		langs = append(langs, l.Lang)
	}
	langs = append(langs, "")

	kinds := []string{"home", "page", "section", "taxonomy", "term", "404", "temporary", ""}
	if small {
		kinds = kinds[:5]
	}
	type layout struct {
		name      string
		mustMatch bool
	}
	layouts := []layout{{"", false}, {"mylayout", false}, {"single", false}, {"special", false}, {"nosuch", false}, {"mylayout", true}}
	if small {
		layouts = layouts[:1]
	}

	for _, p := range paths {
		for _, k := range kinds {
			for _, l := range layouts {
				for _, f := range formats {
					for _, lang := range langs {
						d := f
						d.Kind = k
						d.Lang = lang
						d.LayoutFromUser = l.name
						d.LayoutFromUserMustMatch = l.mustMatch
						lk.pages(tplimpl.TemplateQuery{Path: p, Category: tplimpl.CategoryLayout, Desc: d})
					}
				}
			}
		}
	}

	hooks := [][2]string{{"link", ""}, {"image", ""}, {"heading", ""}, {"codeblock", ""}, {"codeblock", "go"}, {"codeblock", "goat"}, {"codeblock", "python"}, {"table", ""}, {"blockquote", ""}, {"blockquote", "alert"}, {"passthrough", ""}, {"passthrough", "inline"}}
	for _, p := range paths {
		for _, h := range hooks {
			for _, f := range formats {
				for _, lang := range langs[:1] {
					d := f
					d.Kind = "page"
					d.Lang = lang
					d.Variant1 = h[0]
					d.Variant2 = h[1]
					lk.pages(tplimpl.TemplateQuery{Path: p, Category: tplimpl.CategoryMarkup, Desc: d})
				}
			}
		}
	}

	for _, n := range partialNames {
		rec.Add(map[string]any{"op": "partial", "name": n, "result": lk.id(store.LookupPartial(n))})
	}

	for _, p := range scPaths {
		for _, n := range scNames {
			for _, f := range formats {
				for _, lang := range langs {
					for _, always := range []bool{false, true} {
						d := f
						d.Kind = "page"
						d.Lang = lang
						d.AlwaysAllowPlainText = always
						lk.shortcode(tplimpl.TemplateQuery{Path: p, Name: n, Category: tplimpl.CategoryShortcode, Desc: d})
					}
				}
			}
		}
	}
	for _, n := range scNames {
		rec.Add(map[string]any{"op": "byName", "name": n, "result": lk.id(store.LookupShortcodeByName(n))})
	}
}

func sortedKeys(m map[string]bool) []string {
	out := make([]string, 0, len(m))
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}
