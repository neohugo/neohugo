// Command keys is the Go oracle of crates/nh-resources/tests/keys.rs (Wave B
// task T14): resource keys that name files.
//
// TransformationKey (resources/transform.go): the seeksnack SCSS chain (toCSS
// with the site's options | postCSS | minify | fingerprint) over
// "scss/website.scss", whose key names the resources/_gen/assets file
// (ce37005bb9b0d2e87a9f0d33876c2b52), and other chains (every element kind of
// the transformations' keys, case and path cleaning of the target key, image
// targets whose Key includes the source hash). The transformations are
// resources.NewFeatureNotAvailableTransformer values with the keys of the real
// transformers (TransformationKey never runs them).
//
// The cold image Key rule (resources/resource.go Key, image.go
// relTargetPathFromConfig): the golden watermark vector of specs/images.md
// §4.5 (the watermark is part of the private site; its source hash
// 6519743917224815147 is known) through the same hashing calls, and end to end
// through resources.Spec for the synthetic site's watermark: Resize names and
// Keys, the Overlay filter keys that embed the watermark's Key.
//
// Run (names and hashes only, no float code: any platform):
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-resources/keys -root . -out crates/nh-resources/tests/fixtures/keys
package main

import (
	"flag"
	"fmt"
	"image"
	"log"
	"path/filepath"
	"reflect"
	"sort"
	"strings"

	"github.com/disintegration/gift"
	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/images"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/resources/resource_factories/create"
	"github.com/neohugo/neohugo/resources/resource_transformers/tocss/scss"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
)

type keyer interface {
	TransformationKey() string
}

// fakeSource is an images.ImageSource with a given Key (the golden
// watermark's).
type fakeSource struct{ key string }

func (s fakeSource) DecodeImage() (image.Image, error) { return nil, fmt.Errorf("not used") }
func (s fakeSource) Key() string                       { return s.key }

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-resources/tests/fixtures/keys", "fixture dir")
	flag.Parse()

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}
	site, err := rsupport.LoadSite(filepath.Join(absRoot, "crates/nh-resources/tests/fixtures/site"))
	if err != nil {
		log.Fatal(err)
	}
	defer site.Close()
	spec := site.Specs[0]
	cc := create.New(spec)

	get := func(p string) resource.Resource {
		r, err := cc.Get(p)
		if err != nil || r == nil {
			log.Fatalf("get %s: %v", p, err)
		}
		return r
	}

	// The seeksnack toCSS options: (dict "enableSourceMap" false "includePaths"
	// (slice "node_modules" "assets/scss") "outputStyle" "compressed").
	scssOpts, err := scss.DecodeOptions(map[string]any{
		"enableSourceMap": false,
		"includePaths":    []any{"node_modules", "assets/scss"},
		"outputStyle":     "compressed",
	})
	if err != nil {
		log.Fatal(err)
	}
	scssOpts2, err := scss.DecodeOptions(map[string]any{
		"targetPath": "/css/main.css",
		"precision":  6,
		"vars":       map[string]any{"primary": "#fff", "n": 3},
	})
	if err != nil {
		log.Fatal(err)
	}

	type elem struct {
		name     string
		elements []any
		enc      []any
	}
	tr := func(name string, elements ...any) elem {
		e := elem{name: name, elements: elements}
		for _, x := range elements {
			switch v := x.(type) {
			case scss.Options:
				e.enc = append(e.enc, map[string]any{"t": "scss.Options", "v": map[string]any{
					"TargetPath": v.TargetPath, "IncludePaths": v.IncludePaths, "OutputStyle": v.OutputStyle,
					"Precision": v.Precision, "EnableSourceMap": v.EnableSourceMap, "Vars": rsupport.Enc(v.Vars),
				}})
			default:
				e.enc = append(e.enc, rsupport.Enc(x))
			}
		}
		return e
	}

	chains := []struct {
		name   string
		target string
		trs    []elem
	}{
		{"seeksnack-scss", "scss/website.scss", []elem{
			tr("tocss", scssOpts), tr("postcss", map[string]any(nil)), tr("minify"), tr("fingerprint", "sha256"),
		}},
		{"empty", "scss/website.scss", nil},
		{"fingerprint-md5", "css/a.css", []elem{tr("fingerprint", "md5")}},
		{"fingerprint-sha512", "css/a.css", []elem{tr("minify"), tr("fingerprint", "sha512")}},
		{"jsbuild", "js/a.js", []elem{
			tr("jsbuild", map[string]any{"target": "es2015"}),
			tr("jsbuild", map[string]any{"targetPath": "js/website.js", "minify": true}), tr("fingerprint", "sha256"),
		}},
		{"jsbuild-nil", "js/b.js", []elem{tr("jsbuild", map[string]any(nil))}},
		{"execute-as-template", "js/a.js", []elem{tr("execute-as-template", "ts/search.ts"), tr("jsbuild", map[string]any{"target": "es2015"})}},
		{"tocss-2", "scss/website.scss", []elem{tr("tocss", scssOpts2), tr("postcss", map[string]any{"config": "x.js", "noMap": true})}},
		{"upper", "Upper/MixedCase.TXT", []elem{tr("minify")}},
		{"space", "a b/space ü.txt", []elem{tr("fingerprint", "sha384")}},
		{"image", "images/watermark.png", []elem{tr("fingerprint", "sha256")}},
		{"elements", "txt/hello.txt", []elem{tr("x", 1, int64(2), 2.5, true, []string{"a", "b"}, []any{"c", 3}, map[string]any{"k": "v"})}},
	}

	var chainRecs []any
	for _, c := range chains {
		r := get(c.target)
		var trs []resources.ResourceTransformation
		var trRecs []any
		for _, e := range c.trs {
			t := resources.NewFeatureNotAvailableTransformer(e.name, e.elements...)
			trs = append(trs, t)
			trRecs = append(trRecs, map[string]any{"name": e.name, "elements": e.enc, "value": t.Key().Value()})
		}
		rt, err := r.(resources.ResourceTransformer).Transform(trs...)
		if err != nil {
			log.Fatal(err)
		}
		chainRecs = append(chainRecs, map[string]any{
			"name":              c.name,
			"target":            c.target,
			"targetKey":         r.(resource.Identifier).Key(),
			"transformations":   trRecs,
			"transformationKey": rt.(keyer).TransformationKey(),
		})
	}

	// The golden cold image Key (specs/images.md §4.5).
	cfg := rsupport.Field(reflect.ValueOf(spec), "imaging").Interface().(*images.ImageProcessor).Cfg
	resize600, err := images.DecodeImageConfig([]string{"resize", "600x480"}, cfg, images.PNG)
	if err != nil {
		log.Fatal(err)
	}
	const goldenHash = uint64(6519743917224815147)
	goldenName := "watermark_hu_" + hashing.HashStringHex("", goldenHash, resize600.Key, cfg.SourceHash) + ".png"
	goldenKey := "/images/" + goldenName + fmt.Sprintf("_%d", goldenHash)
	f := images.Filters{}
	goldenFilterKey := hashing.HashString([]gift.Filter{f.Overlay(fakeSource{goldenKey}, 0, 0)})

	// End to end: the synthetic watermark.
	wm := get("images/watermark.png")
	var e2e []any
	for _, spec := range []string{"600x480", "300x240", "600x480 webp", "640x480 webp", "x100", "300x240 q50", "120x jpg #ff0000"} {
		ir, err := wm.(images.ImageResourceOps).Resize(spec)
		if err != nil {
			log.Fatal(err)
		}
		r := ir.(resource.Resource)
		rec := map[string]any{
			"spec":         spec,
			"key":          r.(resource.Identifier).Key(),
			"relPermalink": r.RelPermalink(),
			"name":         r.Name(),
		}
		for _, xy := range [][2]any{{0, 0}, {20, 10}, {0.5, int64(3)}} {
			fk := hashing.HashString([]gift.Filter{f.Overlay(ir.(images.ImageSource), xy[0], xy[1])})
			rec[fmt.Sprintf("overlay %v %v", xy[0], xy[1])] = fk
		}
		// A second step: its name embeds the first step's hash id. (Not
		// after a WebP step: the port does not decode WebP.)
		if !strings.Contains(spec, "webp") {
			ir2, err := ir.Resize("50x40")
			if err != nil {
				log.Fatal(err)
			}
			rec["chainKey"] = ir2.(resource.Identifier).Key()
			rec["chainRelPermalink"] = ir2.(resource.Resource).RelPermalink()
		}
		e2e = append(e2e, rec)
	}
	_, wmBase := paths.FileAndExt("images/watermark.png")

	// Multihost: the language in the Key and the image memory key, the resources
	// published under each language's dir, and the second language's image found in
	// the file cache the first one filled (Go's read path: a Key without the hash).
	mh, err := rsupport.LoadSite(filepath.Join(absRoot, "crates/nh-resources/tests/fixtures/site-multihost"))
	if err != nil {
		log.Fatal(err)
	}
	defer mh.Close()
	var mhRecs []any
	for i, spec := range mh.Specs {
		abs := filepath.Join(absRoot, "crates/nh-resources/tests/fixtures/site/assets/images/watermark.png")
		r, err := spec.NewResource(resources.ResourceSourceDescriptor{
			TargetPath: "/images/watermark.png", OpenReadSeekCloser: rsupport.OpenFile(abs),
			SourceFilenameOrPath: abs, LazyPublish: true,
		})
		if err != nil {
			log.Fatal(err)
		}
		rec := map[string]any{"lang": mh.Langs[i], "key": r.(resource.Identifier).Key(), "relPermalink": r.RelPermalink(), "permalink": r.Permalink()}
		for _, s := range []string{"300x240", "120x jpg"} {
			ir, err := r.(images.ImageResourceOps).Resize(s)
			if err != nil {
				log.Fatal(err)
			}
			rr := ir.(resource.Resource)
			rec[s] = map[string]any{
				"key": rr.(resource.Identifier).Key(), "relPermalink": rr.RelPermalink(), "permalink": rr.Permalink(),
				"mediaType": rr.MediaType().Type, "width": ir.Width(), "height": ir.Height(),
			}
		}
		mhRecs = append(mhRecs, rec)
	}

	outRec := map[string]any{
		"multihost":          mhRecs,
		"multihostPublished": publishedNames(mh),
		"chains":             chainRecs,
		"golden": map[string]any{
			"imagingSourceHash": cfg.SourceHash,
			"resize600Key":      resize600.Key,
			"hash":              fmt.Sprint(goldenHash),
			"name":              goldenName,
			"key":               goldenKey,
			"filterKey":         goldenFilterKey,
		},
		"watermark": map[string]any{
			"key":    wm.(resource.Identifier).Key(),
			"ext":    wmBase,
			"resize": e2e,
		},
	}
	if err := rsupport.WriteGz(filepath.Join(*out, "keys.json.gz"), outRec); err != nil {
		log.Fatal(err)
	}
}

// publishedNames lists the published files (the names only: the processed
// images' bytes depend on the platform's float fusion).
func publishedNames(s *rsupport.Site) []string {
	var names []string
	for k := range s.PublishedFiles() {
		names = append(names, k)
	}
	sort.Strings(names)
	return names
}
