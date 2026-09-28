// Command funcnames records the sorted names of tplimplinit.CreateFuncMap
// (every template function and namespace of a site's func map). T20's Rust
// tests bind each name to a stub (the names-only func map of
// NewHugoSitesCfg.func_map_factory): the template store only needs the names
// to parse the templates, so capture does not wait for the template
// functions (T18/T19).
//
//	go run ./tools/go-oracle/nh-hugolib/funcnames -root . -out crates/nh-hugolib/tests/fixtures/funcnames/funcnames.json.gz
package main

import (
	"flag"
	"log"
	"os"
	"path/filepath"
	"sort"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
	"github.com/neohugo/neohugo/tpl/tplimplinit"
)

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "crates/nh-hugolib/tests/fixtures/funcnames/funcnames.json.gz", "output file (relative to -root)")
	flag.Parse()

	tmp, err := os.MkdirTemp("", "nh-hugolib-funcnames")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	site := hsupport.Site{Name: "funcnames", TOML: "baseURL = \"https://example.org/\"\n", Files: map[string]hsupport.File{}}
	b, err := hsupport.New(site, tmp)
	if err != nil {
		log.Fatal(err)
	}
	fm := tplimplinit.CreateFuncMap(b.H.Sites[0].Deps)
	names := make([]string, 0, len(fm))
	for k := range fm {
		names = append(names, k)
	}
	sort.Strings(names)

	outFile := *out
	if !filepath.IsAbs(outFile) {
		outFile = filepath.Join(*root, outFile)
	}
	if err := hsupport.WriteJSONGz(outFile, map[string]any{"funcNames": names}); err != nil {
		log.Fatal(err)
	}
}
