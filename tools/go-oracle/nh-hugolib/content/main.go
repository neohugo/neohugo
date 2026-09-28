// Command content is the Go oracle of nh-hugolib's content rendering (T22):
// for each site of hookrec/rec (a content-focused synthetic en/th site, the
// reconstructed seeksnack site, hugolib/testsite, docs/, the shortcode
// extraction site) it creates the HugoSites like a build, runs process and
// assemble, and then, like the render loop, for every site and render format
// shifts every page to the format (preparePagesForRender on all sites) and
// reads .TableOfContents, .Content, .ContentWithoutSummary, the summary type,
// .Summary, .Truncated, .Plain, .PlainWords, .WordCount, .FuzzyWordCount,
// .ReadingTime, .Len and .Fragments of every page with a file of the
// rendering site. Every render-hook and shortcode template execution is
// recorded (hookrec/rec), so the Rust test can replay them through
// template_exec::TemplateExecutor.
//
// The recorder is added to package hugolib with `go run -overlay`; the
// repository is never modified. Each site is written to a temporary directory
// and recorded in the fixture (files taken unchanged from the repository as
// references with their hash).
//
//	go run ./tools/go-oracle/nh-hugolib/content -root .
package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"

	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hookrec/rec"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

// oracle is set by the overlay-added hook file (hookFile).
var oracle func(h *hugolib.HugoSites, norm func(string) string, hash bool) (map[string]any, error)

const hookFile = `package main

import "github.com/neohugo/neohugo/hugolib"

func init() {
	oracle = hugolib.NHOracleContent
}
`

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "crates/nh-hugolib/tests/fixtures/content", "output directory (relative to -root)")
	only := flag.String("site", "", "only this site")
	flag.Parse()

	if !hsupport.IsChild() {
		files := rec.Files()
		files["tools/go-oracle/nh-hugolib/content/zz_nh_hook.go"] = hookFile
		args := []string{"-root", *root, "-out", *out, "-site", *only}
		if err := hsupport.RunOverlaid(*root, "./tools/go-oracle/nh-hugolib/content", files, rec.Patches(), args); err != nil {
			log.Fatal(err)
		}
		return
	}
	if oracle == nil {
		log.Fatal("oracle hook not installed")
	}

	sites, err := rec.Sites(*root)
	if err != nil {
		log.Fatal(err)
	}

	tmp, err := os.MkdirTemp("", "nh-hugolib-content")
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
		// The docs site is large: its values are recorded as hashes.
		dump, err := oracle(b.H, b.Norm, s.Name == "docs")
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
