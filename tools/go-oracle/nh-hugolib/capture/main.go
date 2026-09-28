// Command capture is the Go oracle of nh-hugolib's capture phase (T20): for
// each site (hsupport: docs/, hugolib/testsite, nh-page's synthetic site, the
// reconstructed seeksnack config with a synthetic content tree, page-tree
// edge cases) it creates the HugoSites like a build, runs `process` (collect
// the content files, create the pages with setMetaPre, insert them into the
// trees) and dumps, before assembly:
//
//   - treePages and treeResources in walk order: key, node kind and language
//     slots;
//   - every page reached from the trees: kind, path info, language, file, the
//     PageConfig filled by setMetaPre (params as typed values, cascade) and
//     the parsed content items (source ranges, summary divider, shortcodes
//     with name, ordinal, doMarkup, params, inner, template);
//   - the log of the HugoSites (duplicate path warnings).
//
// The dump function is added to package hugolib with `go run -overlay`
// (overlay_capture.go.txt); the repository is never modified. Each site is
// written to a temporary directory and is recorded in the fixture (files
// taken unchanged from the repository as references with their hash), so the
// Rust test recreates it.
//
//	go run ./tools/go-oracle/nh-hugolib/capture -root . -out crates/nh-hugolib/tests/fixtures/capture
package main

import (
	_ "embed"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

//go:embed overlay_capture.go.txt
var overlayCapture string

// capture is set by the overlay-added hook file (hookFile).
var capture func(h *hugolib.HugoSites, norm func(string) string, enc func(any) any) (map[string]any, error)

const hookFile = `package main

import "github.com/neohugo/neohugo/hugolib"

func init() {
	capture = hugolib.NHOracleCapture
}
`

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "crates/nh-hugolib/tests/fixtures/capture", "output directory (relative to -root)")
	flag.Parse()

	if !hsupport.IsChild() {
		files := map[string]string{
			"hugolib/zz_nh_oracle_capture.go":                  overlayCapture,
			"tools/go-oracle/nh-hugolib/capture/zz_nh_hook.go": hookFile,
		}
		if err := hsupport.RunOverlaid(*root, "./tools/go-oracle/nh-hugolib/capture", files, nil, []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if capture == nil {
		log.Fatal("capture hook not installed")
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
	sites = append(sites, hsupport.EdgeSites()...)

	tmp, err := os.MkdirTemp("", "nh-hugolib-capture")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	outDir := *out
	if !filepath.IsAbs(outDir) {
		outDir = filepath.Join(*root, outDir)
	}
	for _, s := range sites {
		b, err := hsupport.New(s, tmp)
		if err != nil {
			log.Fatal(err)
		}
		c := map[string]any{"site": s.Describe()}
		dump, err := capture(b.H, b.Norm, goval.Encode)
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
