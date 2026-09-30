// Command assemble is the Go oracle of nh-hugolib's assembly phase (T21):
// for each site (the capture sites of hsupport: docs/, hugolib/testsite,
// nh-page's synthetic site, the reconstructed seeksnack config with a
// synthetic content tree and the page-tree edge sites; the content site of
// hookrec/rec; the assembly sites of this package) it creates the HugoSites
// like a build, runs process and assemble, and dumps:
//
//   - every page's meta after assembly (setMetaPost: params as typed values,
//     dates, default titles, cascade, build options, sitemap, term and
//     singular), its target path descriptor, page output formats, page
//     outputs (shared by format name) with their target paths, relative
//     permalinks and permalinks, and its tree relations;
//   - per site: the render formats, the home page, lastmod, the main
//     sections (and the candidates Go picks from at random on a tie), the
//     page set in walk order, the bundled pages, .Pages/.RegularPages of every
//     node in walk order (every other term asks .RegularPages first: they
//     share a cache key), Site.Pages/RegularPages, Site.Taxonomies, .Data of
//     every node, GetTerms, GetPage for many path forms (with and without a
//     context page), the resources of every node, and the keys of the query
//     caches.
//
// The dump function is added to package hugolib with `go run -overlay`
// (overlay_assemble.go.txt); the repository is never modified. Each site is
// written to a temporary directory and recorded in the fixture (files taken
// unchanged from the repository as references with their hash), so the Rust
// test recreates it.
//
//	go run ./tools/go-oracle/nh-hugolib/assemble -root .
package main

import (
	_ "embed"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hookrec/rec"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

//go:embed overlay_assemble.go.txt
var overlayAssemble string

// oracle is set by the overlay-added hook file (hookFile).
var oracle func(h *hugolib.HugoSites, norm func(string) string, enc func(any) any) (map[string]any, error)

const hookFile = `package main

import "github.com/neohugo/neohugo/hugolib"

func init() {
	oracle = hugolib.NHOracleAssemble
}
`

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "rust/testdata/oracle/hugolib/assemble", "output directory (relative to -root)")
	only := flag.String("site", "", "only this site")
	flag.Parse()

	if !hsupport.IsChild() {
		files := map[string]string{
			"hugolib/zz_nh_oracle_assemble.go":                  overlayAssemble,
			"tools/go-oracle/nh-hugolib/assemble/zz_nh_hook.go": hookFile,
		}
		args := []string{"-root", *root, "-out", *out, "-site", *only}
		if err := hsupport.RunOverlaid(*root, "./tools/go-oracle/nh-hugolib/assemble", files, nil, args); err != nil {
			log.Fatal(err)
		}
		return
	}
	if oracle == nil {
		log.Fatal("oracle hook not installed")
	}

	sites, err := hsupport.RepoSites(*root)
	if err != nil {
		log.Fatal(err)
	}
	sites = append(sites, hsupport.SyntheticSite())
	seeksnack, err := hsupport.SeeksnackSite(*root)
	if err != nil {
		log.Fatal(err)
	}
	sites = append(sites, seeksnack)
	for _, s := range hsupport.EdgeSites() {
		if strings.HasPrefix(s.Name, "sc-err-") {
			// Capture fails for these.
			continue
		}
		sites = append(sites, s)
	}
	sites = append(sites, rec.ContentSite())
	sites = append(sites, AssembleSites()...)

	tmp, err := os.MkdirTemp("", "nh-hugolib-assemble")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	outDir := *out
	if !filepath.IsAbs(outDir) {
		outDir = filepath.Join(*root, outDir)
	}
	for _, s := range sites {
		if *only != "" && s.Name != *only {
			continue
		}
		b, err := hsupport.New(s, tmp)
		if err != nil {
			log.Fatal(err)
		}
		c := map[string]any{"site": s.Describe()}
		dump, err := oracle(b.H, b.Norm, goval.Encode)
		if err != nil {
			c["err"] = b.Norm(err.Error())
		} else {
			c["dump"] = dump
		}
		c["log"] = b.LogLines()
		if err := hsupport.WriteJSONGz(filepath.Join(outDir, s.Name+".json.gz"), c); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("%s: ok\n", s.Name)
	}
}
