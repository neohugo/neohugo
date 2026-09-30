// Command build is the Go oracle of nh-hugolib's build driver, render loop
// and aliases (T24): for each site (the assemble oracle's 17 sites, read
// from its fixtures, with simple page layouts added where a site has none,
// and the build sites of sites.go) it runs a real in-memory Go build
// (`HugoSites.Build`, with HUGO_NUMWORKERMULTIPLIER=1, i.e. one render
// worker; see main's doc below) in a temporary directory and records:
//
//   - every file created in the publish dir, in order, tagged with the
//     phase ("" while rendering, "tpl" while a page template runs, "post"
//     in postProcess), with the bytes of the alias files;
//   - every template execution of renderAndWritePage (page or pager N, the
//     page, output format, template name, whether the output was empty and
//     the error) and every alias publish (page, format, alias path,
//     permalink);
//   - the pages whose paginator was initialised after their render, per
//     output format, with the number of pagers (the `pagers` fixture);
//   - with build stats enabled, the bytes every site's publisher fed its
//     HTML elements collector, and the hugo_stats.json written;
//   - with resources.PostProcess, the files with placeholders before and
//     after postProcess and the field values of every PostPublishResource;
//   - the jsconfig.json written for js.Build source roots;
//   - the log and the build error.
//
// The hooks are added to package hugolib with `go run -overlay`
// (overlay_build.go.txt, plus small patches of copies of site.go,
// site_render.go, alias.go and hugo_sites_build.go); the repository is never
// modified. The publish dir fs is wrapped to record file creations.
//
// Go renders pages with GetNumWorkerMultiplier() workers; with more than one
// the order of pages within a render pass, and so the last writer of
// colliding targets, is random. The golden build (default workers) is
// byte-identical to a single-worker build (HUGO_LAYER.md §7.1), so the
// single-worker order is the reference.
//
//	go run ./tools/go-oracle/nh-hugolib/build -root .
package main

import (
	"bytes"
	_ "embed"
	"encoding/hex"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"unicode/utf8"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
	"github.com/spf13/afero"
)

//go:embed overlay_build.go.txt
var overlayBuild string

// Set by the overlay-added hook file (hookFile).
var (
	oracle   func(h *hugolib.HugoSites, workingDir string, norm func(string) string, enc func([]byte) any) map[string]any
	newRecFs func(fs afero.Fs) afero.Fs
	reset    func()
)

const hookFile = `package main

import "github.com/neohugo/neohugo/hugolib"

func init() {
	oracle = hugolib.NHOracleBuild
	newRecFs = hugolib.NHNewRecFs
	reset = hugolib.NHOracleBuildReset
}
`

// The patches of the build code (inserted after the anchors).
var patches = []hsupport.Patch{
	{
		File:   "hugolib/site.go",
		After:  "\tctx = tpl.Context.DependencyManagerScopedProvider.Set(ctx, p)\n",
		Insert: "\tnhBuildRender(s, p, statCounter, templ)\n",
	},
	{
		File:   "hugolib/site.go",
		After:  "\tisHTML := of.IsHTML\n",
		Insert: "\tnhBuildNonEmpty()\n",
	},
	{
		File:   "hugolib/site.go",
		After:  "func (s *Site) renderForTemplate(ctx context.Context, name, outputFormat string, d any, w io.Writer, templ *tplimpl.TemplInfo) (err error) {\n",
		Insert: "\tdefer nhBuildTemplate()(&err)\n",
	},
	{
		File:   "hugolib/site_render.go",
		After:  "\t\tif p.paginator != nil && p.paginator.current != nil {\n",
		Insert: "\t\t\tnhBuildPager(s, p)\n",
	},
	{
		File:   "hugolib/alias.go",
		After:  "\thandler := newAliasHandler(s.GetTemplateStore(), s.Log, allowRoot)\n",
		Insert: "\tnhBuildAlias(s, path, permalink, outputFormat, p)\n",
	},
	{
		File:   "hugolib/hugo_sites_build.go",
		After:  "\t\ttoPostProcess = append(toPostProcess, r)\n\t}\n",
		Insert: "\tnhBuildPostProcess(toPostProcess)\n",
	},
}

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "rust/testdata/oracle/hugolib/build", "output directory (relative to -root)")
	only := flag.String("site", "", "only this site")
	flag.Parse()

	if !hsupport.IsChild() {
		files := map[string]string{
			"hugolib/zz_nh_oracle_build.go":                  overlayBuild,
			"tools/go-oracle/nh-hugolib/build/zz_nh_hook.go": hookFile,
		}
		args := []string{"-root", *root, "-out", *out, "-site", *only}
		if err := hsupport.RunOverlaid(*root, "./tools/go-oracle/nh-hugolib/build", files, patches, args); err != nil {
			log.Fatal(err)
		}
		return
	}
	if oracle == nil {
		log.Fatal("oracle hook not installed")
	}

	sites, err := BuildSites(*root)
	if err != nil {
		log.Fatal(err)
	}

	tmp, err := os.MkdirTemp("", "nh-hugolib-build")
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
		reset()
		b, err := newSites(s, tmp)
		if err != nil {
			log.Fatal(err)
		}
		c := map[string]any{"site": s.Describe()}
		for k, v := range oracle(b.H, b.Dir, b.Norm, enc) {
			c[k] = v
		}
		c["log"] = b.LogLines()
		if err := hsupport.WriteJSONGz(filepath.Join(outDir, s.Name+".json.gz"), c); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("%s: ok\n", s.Name)
	}
}

// enc encodes bytes: a string when they are valid UTF-8, else hex.
func enc(b []byte) any {
	if utf8.Valid(b) {
		return string(b)
	}
	return map[string]any{"hex": hex.EncodeToString(b)}
}

// newSites is hsupport.New with the publish dir wrapped by the recording fs.
func newSites(s hsupport.Site, tmp string) (*hsupport.Built, error) {
	dir := filepath.Join(tmp, s.Name)
	if err := s.Write(dir); err != nil {
		return nil, err
	}

	var cfgLog bytes.Buffer
	cfgLogger := loggers.New(loggers.Options{StdOut: &cfgLog, StdErr: &cfgLog, Level: logg.LevelWarn})

	flags := config.New()
	flags.Set("workingDir", dir)
	flags.Set("noBuildLock", true)
	flags.Set("cacheDir", filepath.Join(tmp, "_cache"))

	res, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Flags:       flags,
		Fs:          hugofs.Os,
		Filename:    filepath.Join(dir, "hugo.toml"),
		Logger:      cfgLogger,
		Environment: "production",
		Environ:     []string{"NEOHUGO_ORACLE=1"},
	})
	if err != nil {
		return nil, fmt.Errorf("%s: load config: %w", s.Name, err)
	}

	var logBuf bytes.Buffer
	logger := loggers.New(loggers.Options{
		StdOut:        &logBuf,
		StdErr:        &logBuf,
		Level:         logg.LevelWarn,
		DistinctLevel: logg.LevelWarn,
	})

	hfs := hugofs.NewFrom(hugofs.Os, res.LoadingInfo.BaseConfig)
	hfs.PublishDir = newRecFs(hfs.PublishDir)
	h, err := hugolib.NewHugoSites(deps.DepsCfg{Configs: res, Fs: hfs, TestLogger: logger})
	if err != nil {
		return nil, fmt.Errorf("%s: new sites: %w", s.Name, err)
	}
	return &hsupport.Built{H: h, Dir: dir, Log: &logBuf}, nil
}
