// Command escdump is the Go oracle for the parse trees of the template
// store of crates/nh-tplimpl (Wave B task T13) after Hugo's AST transforms
// (templatetransform.go) and html/template escaping (prepareTemplates).
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplimpl/escdump [-root .] [-out crates/nh-tplimpl/tests/fixtures/escdump]
//
// For every site (tsupport.AllSites; the Rust test rebuilds the store from
// the store oracle's fixture of the same site) it writes <site>.json.gz with
// every tree of the store's namespaces as NewStore leaves them: the main html
// namespace (with the derived name$htmltemplate_* templates), the main text
// namespace and the namespace of every base-applied variant (only the trees
// that are not the main namespace's tree of the same name, plus how many
// are). Template names show a name counter as "-N" (which template gets
// which number depends on Go's map order).
package main

import (
	"flag"
	"log"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-tplimpl/tsupport"
)

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-tplimpl/tests/fixtures/escdump", "output directory")
	flag.Parse()

	if !tsupport.IsChild() {
		if err := tsupport.RunOverlaid(*root, "./tools/go-oracle/nh-tplimpl/escdump", []string{"-root", *root, "-out", *out}); err != nil {
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
		ns := tsupport.Oracle.Namespaces(b.H.Sites[0].TemplateStore)
		if err := fx.Add(s, map[string]any{
			"site":       s.Name,
			"namespaces": ns,
		}); err != nil {
			log.Fatal(err)
		}
		n := 0
		for _, v := range ns {
			if m, ok := v.(map[string]any); ok {
				if l, ok := m["templates"].([][]string); ok {
					n += len(l)
				}
			}
		}
		log.Printf("escdump: %s: %d namespaces, %d trees", s.Name, len(ns), n)
	}
	if err := fx.Close(); err != nil {
		log.Fatal(err)
	}
}
