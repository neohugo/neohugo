// Command images is the Go oracle of crates/nh-resources/tests/images.rs
// (Wave B task T14): chained image processing through the resource API
// (imageResource / resourceAdapter / ImageCache), never through
// resources/images directly: the seeksnack template chains (A = Resize,
// W = watermark Resize, B = A | images.Filter (images.Overlay W 0 0),
// C = B.Resize "<size> webp", the index.json 600x480 webp, render-image's
// Filter on the original) and Fit/Fill/Crop/Process/format conversions (the
// background fill for JPEG targets, Floyd-Steinberg to the source palette for
// PNG targets), other filters, over repository images (seeksnack JPEGs and
// PNGs from the go-image and go-png fixtures, resources/testdata) and the
// synthetic site's watermark. Every step decodes its parent's ENCODED bytes
// (Go reads them from the file cache) and the overlay decodes the watermark's
// encoded PNG.
//
// For every processed image: RelPermalink (publishes), Key, Name, MediaType,
// Width, Height and the length + sha256 of the encoded bytes (Content; the
// published file for linked images), full bytes for a few small ones.
//
// gift's resampling, the flate encoder and libwebp fuse float operations on
// arm64 (the golden build's platform): regen.sh builds this oracle for
// linux/arm64 and runs it under qemu-aarch64-static.
package main

import (
	"context"
	"encoding/base64"
	"flag"
	"fmt"
	"log"
	"path/filepath"

	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/images"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
)

type source struct {
	name string // target path under /images/
	file string // repository path
}

var sources = []source{
	{"almonds.jpg", "rust/testdata/site-assets/site/assets_images_categories_almonds.jpg"},
	{"pringles-paprika.jpg", "rust/testdata/site-assets/site/content_potato-crisps_pringles-paprika_pringles-paprika.jpg"},
	{"combos-pipr.jpg", "rust/testdata/site-assets/site/content_pretzels_combos-pizzeria-pretzel_combos-pipr.jpg"},
	{"hanamifoods.jpg", "rust/testdata/site-assets/site/content_companies_hanami-foods-co-ltd_hanamifoods.jpg"},
	{"600x200.jpg", "rust/testdata/site-assets/site/content_cookies_alices-pineapple-pastry_600x200.jpg"},
	{"sunset.jpg", "resources/testdata/sunset.jpg"},
	{"orientation6.jpg", "resources/testdata/exif/orientation6.jpg"},
	{"berli-jucker.png", "rust/testdata/site-assets/golden/berli-jucker-foods-ltd.berli-jucker-plc_hu_efcb259769ef42b1.png"},
	{"classic-foods-inc.png", "rust/testdata/site-assets/golden/classic-foods-inc_hu_c0871b21a075dd2d.png"},
	{"cpram.png", "rust/testdata/site-assets/golden/cpram_hu_d59b22a35402c9d8.png"},
	{"gopher-hero8.png", "resources/testdata/gopher-hero8.png"},
	{"fuzzy-circle.png", "resources/testdata/fuzzy-cirlcle.png"},
}

// op is one processing step on a named image of the run.
type op struct {
	out  string   // name of the result
	in   string   // name of the input
	kind string   // resize, fit, fill, crop, process, filter
	spec string   // the spec (resize etc.)
	filt []string // filters: overlay:<wm name>:<x>:<y>, grayscale, gaussianblur:<sigma>, process:<spec>, autoorient
	link bool     // call RelPermalink (publish)
	full bool     // keep the full bytes in the fixture
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/resources/images", "fixture dir")
	flag.Parse()

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}
	site, err := rsupport.LoadSite(filepath.Join(absRoot, "rust/testdata/oracle/resources/site"))
	if err != nil {
		log.Fatal(err)
	}
	defer site.Close()
	spec := site.Specs[0]

	named := map[string]resource.Resource{}
	newRes := func(name, file string) resource.Resource {
		abs := filepath.Join(absRoot, file)
		r, err := spec.NewResource(resources.ResourceSourceDescriptor{
			TargetPath:           "/images/" + name,
			OpenReadSeekCloser:   rsupport.OpenFile(abs),
			SourceFilenameOrPath: abs,
			LazyPublish:          true,
		})
		if err != nil {
			log.Fatal(err)
		}
		return r
	}
	named["W"] = newRes("watermark.png", "rust/testdata/oracle/resources/site/assets/images/watermark.png")

	var ops []op
	for i, s := range sources {
		S := fmt.Sprintf("S%d", i)
		named[S] = newRes(s.name, s.file)
		w, h := named[S].(images.ImageResourceOps).Width(), named[S].(images.ImageResourceOps).Height()
		full := i == 1
		ops = append(ops,
			// _default/list.html & co.
			op{out: S + "A300", in: S, kind: "resize", spec: "300x240"},
			op{out: "W300", in: "W", kind: "resize", spec: "300x240"},
			op{out: S + "B300", in: S + "A300", kind: "filter", filt: []string{"overlay:W300:0:0"}, link: true, full: full},
			op{out: S + "C300", in: S + "B300", kind: "resize", spec: "300x240 webp", link: true, full: full},
			op{out: S + "D600", in: S + "B300", kind: "resize", spec: "600x480 webp", link: true},
			// _default/single.html.
			op{out: S + "A600", in: S, kind: "resize", spec: "600x480"},
			op{out: "W600", in: "W", kind: "resize", spec: "600x480"},
			op{out: S + "B600", in: S + "A600", kind: "filter", filt: []string{"overlay:W600:0:0"}, link: true},
			op{out: S + "C600", in: S + "B600", kind: "resize", spec: "600x480 webp", link: true},
			// render-image.html: the watermark at the original's size, the filter on the original.
			op{out: fmt.Sprintf("W%dx%d", w, h), in: "W", kind: "resize", spec: fmt.Sprintf("%dx%d", w, h)},
			op{out: S + "Br", in: S, kind: "filter", filt: []string{fmt.Sprintf("overlay:W%dx%d:0:0", w, h)}, link: true},
			op{out: S + "Cr", in: S + "Br", kind: "resize", spec: fmt.Sprintf("%dx%d webp", w, h), link: true},
			// Other operations.
			op{out: S + "fit", in: S, kind: "fit", spec: "200x200", link: true},
			op{out: S + "fill", in: S, kind: "fill", spec: "150x100 center", link: true},
			op{out: S + "filltl", in: S, kind: "fill", spec: "90x120 TopLeft lanczos", link: true},
			op{out: S + "crop", in: S, kind: "crop", spec: "100x100 bottomright", link: true},
			op{out: S + "q50", in: S, kind: "resize", spec: "x100 q50", link: true},
			op{out: S + "jpg", in: S, kind: "resize", spec: "120x jpg", link: true},
			op{out: S + "jpgbg", in: S, kind: "resize", spec: "120x jpg #ff0000", link: true},
			op{out: S + "png", in: S, kind: "resize", spec: "90x png", link: true},
			op{out: S + "pngbg", in: S, kind: "resize", spec: "90x png #00ff00", link: true},
			op{out: S + "r90", in: S, kind: "resize", spec: "100x r90", link: true},
			op{out: S + "proc", in: S, kind: "process", spec: "resize 80x webp q60 photo", link: true},
			op{out: S + "gray", in: S, kind: "filter", filt: []string{"grayscale"}, link: true},
			op{out: S + "multi", in: S, kind: "filter", filt: []string{"process:fit 50x50", "overlay:W300:5:5", "gaussianblur:1"}, link: true},
			op{out: S + "orient", in: S, kind: "filter", filt: []string{"autoorient"}, link: true},
			op{out: S + "chain2", in: S + "fit", kind: "resize", spec: "100x png", link: true},
			op{out: S + "chain3", in: S + "chain2", kind: "filter", filt: []string{"overlay:W300:-10:-10"}, link: true},
		)
	}

	f := images.Filters{}
	var recs []any
	for _, o := range ops {
		rec := map[string]any{"out": o.out, "in": o.in, "kind": o.kind, "spec": o.spec, "filters": o.filt, "link": o.link}
		in := named[o.in]
		if in == nil {
			rec["err"] = "missing input " + o.in
			recs = append(recs, rec)
			continue
		}
		ops := in.(images.ImageResourceOps)
		var (
			res images.ImageResource
			err error
		)
		func() {
			defer func() {
				if e := recover(); e != nil {
					err = fmt.Errorf("panic: %v", e)
				}
			}()
			switch o.kind {
			case "resize":
				res, err = ops.Resize(o.spec)
			case "fit":
				res, err = ops.Fit(o.spec)
			case "fill":
				res, err = ops.Fill(o.spec)
			case "crop":
				res, err = ops.Crop(o.spec)
			case "process":
				res, err = ops.Process(o.spec)
			case "filter":
				var fs []any
				for _, spec := range o.filt {
					fs = append(fs, filterOf(f, named, spec))
				}
				res, err = ops.Filter(fs...)
			}
		}()
		if err != nil {
			rec["err"] = rsupport.Rel(absRoot, err.Error())
			recs = append(recs, rec)
			continue
		}
		r := res.(resource.Resource)
		named[o.out] = r
		rec["key"] = r.(resource.Identifier).Key()
		rec["name"] = r.Name()
		rec["mediaType"] = r.MediaType().Type
		rec["width"] = res.Width()
		rec["height"] = res.Height()
		c, err := r.(resource.ContentProvider).Content(context.Background())
		if err != nil {
			rec["contentErr"] = err.Error()
		} else {
			b := []byte(c.(string))
			rec["len"] = len(b)
			rec["sha"] = rsupport.Sha(b)
			if o.full {
				rec["bytes"] = base64.StdEncoding.EncodeToString(b)
			}
		}
		if o.link {
			rec["relPermalink"] = r.RelPermalink()
		}
		recs = append(recs, rec)
	}

	var srcRecs []any
	for _, s := range sources {
		srcRecs = append(srcRecs, []string{s.name, s.file})
	}
	outRec := map[string]any{
		"arch":      rsupport.Arch(),
		"sources":   srcRecs,
		"ops":       recs,
		"published": site.PublishedFiles(),
	}
	if err := rsupport.WriteGz(filepath.Join(*out, "images.json.gz"), outRec); err != nil {
		log.Fatal(err)
	}
}

func filterOf(f images.Filters, named map[string]resource.Resource, spec string) any {
	switch {
	case spec == "grayscale":
		return f.Grayscale()
	case spec == "autoorient":
		return f.AutoOrient()
	case len(spec) > 8 && spec[:8] == "process:":
		return f.Process(spec[8:])
	case len(spec) > 13 && spec[:13] == "gaussianblur:":
		return f.GaussianBlur(spec[13:])
	}
	parts := splitColon(spec[len("overlay:"):])
	name, a, b := parts[0], parts[1], parts[2]
	var x, y int
	_, _ = fmt.Sscan(a, &x)
	_, _ = fmt.Sscan(b, &y)
	return f.Overlay(named[name].(images.ImageSource), x, y)
}

func splitColon(s string) []string {
	var out []string
	cur := ""
	for _, c := range s {
		if c == ':' {
			out = append(out, cur)
			cur = ""
			continue
		}
		cur += string(c)
	}
	return append(out, cur)
}
