// Command resources is the Go oracle of crates/nh-resources/tests/resources.rs
// (Wave B task T14): every asset and bundle resource of the repository's docs
// site, hugolib/testsite and the synthetic site in
// crates/nh-resources/tests/fixtures/site.
//
// The bundle resources are discovered with an in-memory hugolib build
// (SkipRender); the assets through resources.Get (the create client) over the
// assets filesystem. For each one the oracle records the descriptor hugolib or
// the create client handed to resources.Spec.NewResource (read back from the
// resource, with the media type and path cleared so that NewResource resolves
// them again), creates the resource from that descriptor with a fresh set of
// resource specs (one per language, shared SpecCommon, the publish dir in
// memory, cold caches), applies the page's front matter `resources` metadata
// (CloneWithMetadataFromMapIfNeeded) and records RelPermalink, Permalink, Key,
// Name, Title, NameNormalized, MediaType, ResourceType, Data, Params, Content
// (text) and Width/Height (images), before and after the metadata. It checks
// that these equal the attributes of the resources hugolib and the create
// client built, so the descriptors are equivalent inputs. Finally the files of
// the publish dir are recorded.
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-resources/resources -root . \
//	    -out crates/nh-resources/tests/fixtures/resources
package main

import (
	"bytes"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/resources/resource_factories/create"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
	"github.com/spf13/afero"
)

type siteDef struct {
	name string
	dir  string
	// Extra flags (the docs site's news content adapter fetches from the
	// network; it is ignored, it creates no resources).
	ignoreFiles []string
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-resources/tests/fixtures/resources", "fixture dir")
	flag.Parse()

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}

	sites := []siteDef{
		{name: "synth", dir: "crates/nh-resources/tests/fixtures/site"},
		{name: "docs", dir: "docs", ignoreFiles: []string{`_content\.gotmpl$`}},
		{name: "testsite", dir: "hugolib/testsite"},
	}

	for _, sd := range sites {
		recs := runSite(absRoot, sd)
		if err := rsupport.WriteGz(filepath.Join(*out, sd.name+".json.gz"), recs); err != nil {
			log.Fatal(err)
		}
	}
}

type bundleRes struct {
	lang  int
	page  string
	meta  []map[string]any
	r     resource.Resource
	index int
}

func runSite(absRoot string, sd siteDef) map[string]any {
	dir := filepath.Join(absRoot, sd.dir)

	// 1. Discover the bundle resources with hugolib.
	tmp, err := os.MkdirTemp("", "nh-resources-hugolib")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	// hugolib builds a copy of the site: a build writes into the working dir
	// (hugo_stats.json when the site enables build stats), and the repository
	// must stay untouched. The replay below reads the repository's files.
	buildDir := filepath.Join(tmp, "site")
	if err := os.CopyFS(buildDir, os.DirFS(dir)); err != nil {
		log.Fatalf("%s: copy site: %v", sd.name, err)
	}

	var logBuf bytes.Buffer
	flagsCfg := rsupport.Flags(buildDir, tmp)
	if len(sd.ignoreFiles) > 0 {
		flagsCfg.Set("ignoreFiles", sd.ignoreFiles)
	}
	res, logger, err := rsupport.LoadConfigs(buildDir, flagsCfg, &logBuf)
	if err != nil {
		log.Fatalf("%s: %v", sd.name, err)
	}
	fsCfg := config.New()
	fsCfg.Set("workingDir", buildDir)
	fsCfg.Set("publishDir", res.LoadingInfo.BaseConfig.PublishDir)
	hfs := hugofs.NewFromSourceAndDestination(hugofs.Os, afero.NewMemMapFs(), fsCfg)
	hfs.WorkingDirWritable = afero.NewMemMapFs()
	h, err := hugolib.NewHugoSites(deps.DepsCfg{Configs: res, Fs: hfs, LogLevel: logger.Level(), StdErr: &logBuf, StdOut: &logBuf})
	if err != nil {
		log.Fatalf("%s: new sites: %v", sd.name, err)
	}
	if err := h.Build(hugolib.BuildCfg{SkipRender: true}); err != nil {
		log.Fatalf("%s: build: %v\n%s", sd.name, err, logBuf.String())
	}

	langIdx := map[string]int{}
	for i, s := range h.Sites {
		langIdx[s.Language().Lang] = i
	}

	var bundles []bundleRes
	pages := h.Pages()
	sort.SliceStable(pages, func(i, j int) bool {
		a, b := pages[i], pages[j]
		if a.Language().Lang != b.Language().Lang {
			return langIdx[a.Language().Lang] < langIdx[b.Language().Lang]
		}
		return a.Path() < b.Path()
	})
	for _, p := range pages {
		meta := resourcesMeta(p)
		for i, r := range p.Resources() {
			if _, ok := r.(page.Page); ok {
				continue
			}
			bundles = append(bundles, bundleRes{lang: langIdx[p.Language().Lang], page: p.Path(), meta: meta, r: r, index: i})
		}
	}

	// The assets, through the create client of the first language.
	var assetPaths []string
	assetsFs := h.Assets.Fs
	_ = afero.Walk(assetsFs, "", func(p string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}
		assetPaths = append(assetPaths, filepath.ToSlash(p))
		return nil
	})
	sort.Strings(assetPaths)

	// 2. Recreate every resource from its descriptor with fresh specs.
	site, err := rsupport.LoadSite(dir)
	if err != nil {
		log.Fatalf("%s: %v", sd.name, err)
	}
	defer site.Close()

	var recs []any
	createClient := create.New(h.Sites[0].ResourceSpec)
	rp := &replay{absRoot: absRoot, buildDir: buildDir, dir: dir, site: site, byIdent: map[uintptr]int{}}

	for _, ap := range assetPaths {
		ra, err := createClient.Get(ap)
		if err != nil || ra == nil {
			log.Fatalf("%s: get %s: %v", sd.name, ap, err)
		}
		recs = append(recs, rp.record(0, ra, nil, "asset", ap))
	}

	for _, b := range bundles {
		recs = append(recs, rp.record(b.lang, b.r, b.meta, "bundle", b.page))
	}

	return map[string]any{
		"site":      sd.name,
		"dir":       sd.dir,
		"langs":     site.Langs,
		"records":   recs,
		"published": site.PublishedFiles(),
		"log":       rsupport.Rel(absRoot, site.Log.String()),
	}
}

// resourcesMeta returns the page's front matter `resources` metadata
// (pageState.m.pageConfig.ResourcesMeta).
func resourcesMeta(p page.Page) []map[string]any {
	v := reflect.ValueOf(p)
	if v.Type().String() != "*hugolib.pageState" {
		return nil
	}
	m := rsupport.Field(v, "m")
	pc := rsupport.Field(m, "pageConfig")
	if pc.IsNil() {
		return nil
	}
	return rsupport.Field(pc, "ResourcesMeta").Interface().([]map[string]any)
}

// replay recreates the resources in order. Resources that hugolib shares
// between pages (a translation without its own copies uses the default
// language's resources) are recreated once and reused, so the front matter
// metadata of each page is applied to the same object in the same order
// (Go's maps.Params are shared: the metadata writes into the resource's own
// params).
type replay struct {
	absRoot  string
	buildDir string // the copy hugolib built
	dir      string // the site in the repository
	site     *rsupport.Site
	byIdent  map[uintptr]int
	res      []resource.Resource
}

func (rp *replay) record(lang int, orig resource.Resource, meta []map[string]any, kind, where string) map[string]any {
	rd, targetType := rsupport.Descriptor(orig)
	// Back from the build copy to the repository.
	if rest, ok := strings.CutPrefix(rd.SourceFilenameOrPath, rp.buildDir); ok {
		rd.SourceFilenameOrPath = rp.dir + rest
	}
	filename := rd.SourceFilenameOrPath
	rel, err := filepath.Rel(rp.absRoot, filename)
	if err != nil || strings.HasPrefix(rel, "..") {
		log.Fatalf("%s: source %q is outside the repository", where, filename)
	}

	// hugolib passes nil Params for resources without a content adapter
	// resource config (the only kind in these sites); the descriptor read
	// back holds the map the front matter metadata has written into since.
	rd.Params = nil

	in := map[string]any{
		"file":                 filepath.ToSlash(rel),
		"nameNormalized":       rd.NameNormalized,
		"nameOriginal":         rd.NameOriginal,
		"title":                rd.Title,
		"targetBasePaths":      rd.TargetBasePaths,
		"targetPath":           rd.TargetPath,
		"basePathRelPermalink": rd.BasePathRelPermalink,
		"basePathTargetPath":   rd.BasePathTargetPath,
		"sourceFilenameOrPath": rsupport.Rel(rp.absRoot, rd.SourceFilenameOrPath),
		"lazyPublish":          rd.LazyPublish,
		"params":               rsupport.Enc(map[string]any(rd.Params)),
		"data":                 rsupport.Enc(rd.Data),
	}

	rec := map[string]any{
		"kind":       kind,
		"where":      where,
		"lang":       lang,
		"targetType": targetType,
		"rd":         in,
	}

	ident := rsupport.GenericPtr(orig)
	var r resource.Resource
	if i, ok := rp.byIdent[ident]; ok {
		r = rp.res[i]
		rec["same"] = i
	} else {
		rd2 := rd
		rd2.MediaType = media.Type{}
		rd2.Path = nil
		rd2.OpenReadSeekCloser = rsupport.OpenFile(filename)
		rd2.GroupIdentity = nil
		rd2.DependencyManager = nil

		r, err = rp.site.Specs[lang].NewResource(rd2)
		if err != nil {
			log.Fatalf("%s: NewResource: %v", where, err)
		}
		rp.byIdent[ident] = len(rp.res)
	}
	rp.res = append(rp.res, r)

	rec["base"] = rsupport.Rec(r, true)
	if !rd.LazyPublish {
		// hugolib publishes bundle resources itself (publishResources).
		if err := r.(resource.Source).Publish(); err != nil {
			rec["publishErr"] = err.Error()
		}
	}

	got := rec["base"].(map[string]any)
	if meta != nil {
		var encMeta []any
		for _, m := range meta {
			encMeta = append(encMeta, rsupport.Enc(m))
		}
		rec["meta"] = encMeta
		r2 := resources.CloneWithMetadataFromMapIfNeeded(meta, r)
		got = rsupport.Rec(r2, false)
		rec["final"] = got
		rec["baseParamsAfter"] = rsupport.JSON(r.Params())
	}

	// The descriptors must be equivalent inputs: compare with the resource
	// hugolib (or the create client) built.
	want := rsupport.Rec(orig, false)
	for _, k := range []string{"name", "title", "key", "nameNormalized", "mediaType", "resourceType", "data", "params", "relPermalink", "permalink", "width", "height"} {
		if fmt.Sprint(want[k]) != fmt.Sprint(got[k]) {
			log.Fatalf("%s %s: %s: hugolib %v, recreated %v", where, filepath.ToSlash(rel), k, want[k], got[k])
		}
	}
	return rec
}
