// Command store is the Go oracle for the template store of crates/nh-tplimpl
// (Wave B task T13): tpl/tplimpl.NewStore as hugolib creates it.
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplimpl/store [-root .] [-out rust/testdata/oracle/tplimpl/store]
//
// The oracle runs itself again with `go run -overlay` (tsupport.RunOverlaid),
// which adds dump functions to tpl/tplimpl and hugolib. For every site
// (tsupport.AllSites) it writes <site>.json.gz with the site's layout files,
// modules and configuration (what the Rust test rebuilds the store from) and
// the store dump: every template of the main tree with its category,
// sub-category, name, descriptor, parse info and base-applied variants, the
// shortcodes tree, templatesByPath, shortcodesByName, the order of
// templates(), the unused templates, the function names and the layouts
// filesystem walk.
package main

import (
	"flag"
	"log"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-tplimpl/tsupport"
)

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "rust/testdata/oracle/tplimpl/store", "output directory")
	flag.Parse()

	if !tsupport.IsChild() {
		if err := tsupport.RunOverlaid(*root, "./tools/go-oracle/nh-tplimpl/store", []string{"-root", *root, "-out", *out}); err != nil {
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
		b, err := tsupport.New(s)
		if err != nil {
			log.Fatal(err)
		}
		store := b.H.Sites[0].TemplateStore
		dump, err := tsupport.Oracle.StoreDump(store)
		if err != nil {
			log.Fatal(err)
		}
		files := map[string]string{}
		for k, v := range s.Files {
			// The layouts (possibly mounted from elsewhere); not the (large) docs content.
			if tsupport.IsIntegration(s) || strings.HasPrefix(k, "layouts/") || (strings.HasPrefix(k, "themes/") && strings.Contains(k, "/layouts/")) {
				files[k] = v
			}
		}
		n := 0
		for _, k := range []string{"main", "shortcodes"} {
			if v, ok := dump[k].([]map[string]any); ok {
				n += len(v)
			}
		}
		fixture := map[string]any{
			"site":    s.Name,
			"siteDir": tsupport.SiteDir(s.Name),
			"files":   files,
			"modules": b.Modules(),
			"config":  tsupport.Oracle.SiteConfig(b.H),
			"store":   dump,
		}
		if err := fx.Add(s, fixture); err != nil {
			log.Fatal(err)
		}
		names := make([]string, 0, len(files))
		for k := range files {
			names = append(names, k)
		}
		sort.Strings(names)
		log.Printf("store: %s: %d layout files, %d store entries", s.Name, len(names), n)
	}
	if err := fx.Close(); err != nil {
		log.Fatal(err)
	}
}
