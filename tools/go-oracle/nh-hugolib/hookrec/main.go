// Command hookrec records every render-hook and shortcode template execution
// of a real Go build (T22): the recorder of hookrec/rec wraps
// hookRendererTemplate.Render* and renderShortcodeWithPage (a patch applied
// with `go run -overlay`; the repository is never modified), and each record
// maps (page path + language, output format of the content output, kind,
// ordinal) to the output bytes. After the build it dumps the rendered-content
// caches of every page with a file (Go's /cont/ren, /cont/toc and /cont/pla
// partitions: content, summary, table of contents and plain text per page and
// output format name), so the Rust test can render the same entries with the
// recorded hooks and shortcodes replayed.
//
// Sites: the content-focused synthetic site, the reconstructed seeksnack site,
// hugolib/testsite and the shortcode extraction site of hookrec/rec (docs/ is
// covered by the content oracle), each with simple page layouts that read
// .Content, .Summary, .Plain and .TableOfContents (templates that call
// markdownify or .RenderString would execute hooks outside the content
// caches).
//
//	go run ./tools/go-oracle/nh-hugolib/hookrec -root .
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
var oracle func(h *hugolib.HugoSites, norm func(string) string) (map[string]any, error)

const hookFile = `package main

import "github.com/neohugo/neohugo/hugolib"

func init() {
	oracle = hugolib.NHOracleHookrecBuild
}
`

// layouts are the page layouts of the recorded builds (every site's own
// layouts are replaced).
var layouts = map[string]string{
	"layouts/single.html": `{{ .Title }}|{{ .Content }}|{{ .Summary }}|{{ .TableOfContents }}|{{ .WordCount }}`,
	"layouts/list.html":   `{{ .Title }}|{{ .Content }}|{{ range .RegularPages }}{{ .RelPermalink }}|{{ .Summary }}|{{ .Truncated }}{{ end }}`,
	"layouts/single.json": `{{ .Plain | jsonify }}`,
	"layouts/list.json":   `[{{ range $i, $p := .RegularPages }}{{ if $i }},{{ end }}{{ $p.Plain | jsonify }}{{ end }}]`,
}

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "rust/testdata/oracle/hugolib/hookrec", "output directory (relative to -root)")
	only := flag.String("site", "", "only this site")
	flag.Parse()

	if !hsupport.IsChild() {
		files := rec.Files()
		files["tools/go-oracle/nh-hugolib/hookrec/zz_nh_hook.go"] = hookFile
		args := []string{"-root", *root, "-out", *out, "-site", *only}
		if err := hsupport.RunOverlaid(*root, "./tools/go-oracle/nh-hugolib/hookrec", files, rec.Patches(), args); err != nil {
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

	tmp, err := os.MkdirTemp("", "nh-hugolib-hookrec")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	outDir := *out
	if !filepath.IsAbs(outDir) {
		outDir = filepath.Join(*root, outDir)
	}
	for _, s := range sites {
		if s.Name == "docs" || (*only != "" && s.Name != *only) {
			continue
		}
		for k := range s.Files {
			if len(k) > 8 && k[:8] == "layouts/" && filepath.Dir(k) == "layouts" {
				delete(s.Files, k)
			}
		}
		for k, v := range layouts {
			s.Files[k] = hsupport.File{Content: v}
		}
		b, err := hsupport.New(s, tmp)
		if err != nil {
			log.Fatal(err)
		}
		c := map[string]any{"site": s.Describe()}
		dump, err := oracle(b.H, b.Norm)
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
