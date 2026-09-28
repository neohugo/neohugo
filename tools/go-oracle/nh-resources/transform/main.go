// Command transform is the Go oracle of crates/nh-resources/tests/transform.rs
// (Wave B task T14): the transformation chain engine (resources/transform.go)
// over the synthetic site's assets, driven through the real
// resourceAdapter with the integrity (fingerprint md5/sha256/sha384/sha512)
// and minifier transformations, resources.Copy, failing transformations
// (FeatureNotAvailable, a minifier error), links before and after .Content
// (publishing, the transformation cache shared by adapters with the same
// key), resources.PostProcess placeholders with GetFieldString and the
// post-publish replacement loop of hugolib, and `slice`/commonResource.Slice
// with the type check of resources.Concat.
//
// The minifier formats numbers with float code, so the fixture comes from a
// linux/arm64 build run under qemu (see regen.sh); amd64 gives the same
// output for these inputs.
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-resources/transform -root . \
//	    -out crates/nh-resources/tests/fixtures/transform
package main

import (
	"bytes"
	"context"
	"flag"
	"fmt"
	"log"
	"path/filepath"
	"sort"

	"github.com/neohugo/neohugo/common/collections"
	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/postpub"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/resources/resource_factories/create"
	"github.com/neohugo/neohugo/resources/resource_transformers/integrity"
	"github.com/neohugo/neohugo/resources/resource_transformers/minifier"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
)

type keyer interface {
	TransformationKey() string
}

// step is one transformation of a chain: fingerprint:<algo>, minify, copy:<path>,
// na:<name> (a FeatureNotAvailable transformer).
type step = string

var chains = [][]step{
	{"fingerprint:"},
	{"fingerprint:md5"},
	{"fingerprint:sha384"},
	{"fingerprint:sha512"},
	{"minify"},
	{"minify", "fingerprint:sha256"},
	{"fingerprint:sha256", "minify"},
	{"minify", "minify"},
	{"fingerprint:md5", "fingerprint:sha512"},
	{"copy:copied/x.css", "fingerprint:sha256"},
	{"copy:copied/y.txt"},
	{"na:postcss"},
	{"minify", "na:tocss"},
	{"na:babel", "minify"},
	{"fingerprint:sha1"},
}

var assets = []string{
	"css/a.css", "css/b.css", "js/a.js", "js/b.js", "data/x.json", "txt/hello.txt",
	"misc/site.webmanifest", "feed/rss.xml", "images/logo.svg", "Upper/MixedCase.TXT",
	"a b/space ü.txt", "images/watermark.png", "misc/noext", "misc/x.unknownext",
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-resources/tests/fixtures/transform", "fixture dir")
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
	ic := integrity.New(spec)
	mc, err := minifier.New(spec)
	if err != nil {
		log.Fatal(err)
	}

	get := func(p string) resource.Resource {
		r, err := cc.Get(p)
		if err != nil || r == nil {
			log.Fatalf("get %s: %v", p, err)
		}
		return r
	}

	apply := func(r resource.Resource, chain []step) (resource.Resource, error) {
		for _, s := range chain {
			var err error
			switch {
			case s == "minify":
				r, err = mc.Minify(r.(resources.ResourceTransformer))
			case len(s) > 12 && s[:12] == "fingerprint:":
				r, err = ic.Fingerprint(r.(resources.ResourceTransformer), s[12:])
			case s == "fingerprint:":
				r, err = ic.Fingerprint(r.(resources.ResourceTransformer), "")
			case len(s) > 5 && s[:5] == "copy:":
				r = resources.Copy(r, s[5:])
			case len(s) > 3 && s[:3] == "na:":
				r, err = r.(resources.ResourceTransformer).Transform(resources.NewFeatureNotAvailableTransformer(s[3:]))
			default:
				log.Fatalf("unknown step %q", s)
			}
			if err != nil {
				return nil, err
			}
		}
		return r, nil
	}

	var cases []any
	var ppResources []postpub.PostPublishedResource
	for _, order := range []string{"content-first", "links-first"} {
		for _, a := range assets {
			for ci, chain := range chains {
				before := site.PublishedFiles()
				rec := map[string]any{"asset": a, "chain": chain, "order": order, "chainIndex": ci}
				r, err := apply(get(a), chain)
				if err != nil {
					rec["applyErr"] = err.Error()
					cases = append(cases, rec)
					continue
				}
				if k, ok := r.(keyer); ok {
					rec["transformationKey"] = k.TransformationKey()
				}
				content := func() {
					v, err := r.(resource.ContentProvider).Content(context.Background())
					if err != nil {
						rec["contentErr"] = err.Error()
						return
					}
					s := v.(string)
					if a == "images/watermark.png" {
						rec["contentSha"] = rsupport.Sha([]byte(s))
					} else {
						rec["content"] = s
					}
				}
				links := func() {
					rec["relPermalink"] = r.RelPermalink()
					rec["permalink"] = r.Permalink()
				}
				if order == "content-first" {
					content()
					links()
				} else {
					links()
					content()
				}
				rec["key"] = r.(resource.Identifier).Key()
				rec["name"] = r.Name()
				rec["title"] = r.Title()
				rec["mediaType"] = r.MediaType().Type
				rec["resourceType"] = r.ResourceType()
				rec["data"] = rsupport.JSON(r.Data())
				rec["params"] = rsupport.JSON(r.Params())
				after := site.PublishedFiles()
				added := map[string]any{}
				for k, v := range after {
					if before[k] != v {
						added[k] = v
					}
				}
				rec["published"] = added

				if order == "content-first" && (ci == 0 || ci == 5 || ci == 11) && a != "images/watermark.png" {
					// resources.PostProcess: memoized by TransformationKey.
					pp, err := spec.PostProcess(r)
					if err != nil {
						log.Fatal(err)
					}
					pp2, _ := spec.PostProcess(r)
					rec["ppSame"] = pp == pp2
					rec["pp"] = ppRec(pp)
					ppResources = append(ppResources, pp)
				}
				cases = append(cases, rec)
			}
		}
	}

	// The post-publish replacement (hugolib's postProcess loop) over a text
	// with every placeholder of every PostProcess resource.
	var doc bytes.Buffer
	doc.WriteString("<html>__h_pp_l1 not a field __e= <x>")
	for _, pp := range ppResources {
		p := pp.(*postpub.PostPublishResource)
		for _, f := range []string{"Content", "RelPermalink", "Permalink", "Name", "Title", "ResourceType", "Data.Integrity",
			"MediaType.Type", "MediaType.MainType", "MediaType.SubType", "MediaType.Delimiter", "MediaType.FirstSuffix",
			"MediaType.SuffixesCSV", "MediaType.IsText", "MediaType.IsHTML", "MediaType.IsMarkdown", "MediaType.IsZero",
			"MediaType.MarshalJSON", "MediaType.String", "MediaType.Suffixes", "MediaType.NoSuch"} {
			doc.WriteString("[" + f + ":" + fieldOf(p, f) + "]\n")
		}
	}
	doc.WriteString("</html>")
	replaced := replace(doc.Bytes(), ppResources)

	// Unknown accessors panic in Go.
	var panics []any
	for _, f := range []string{"Params", "Nope", "Data.Other"} {
		p := ppResources[0].(*postpub.PostPublishResource)
		func() {
			defer func() {
				if e := recover(); e != nil {
					panics = append(panics, map[string]any{"field": f, "panic": fmt.Sprint(e)})
				}
			}()
			v, ok := p.GetFieldString(fieldOf(p, f))
			panics = append(panics, map[string]any{"field": f, "value": v, "ok": ok})
		}()
	}

	// slice / commonResource.Slice and the type check of resources.Concat.
	r1, r2 := get("js/a.js"), get("js/b.js")
	type slicer interface {
		Slice(in any) (any, error)
	}
	sliceRec := func(args ...any) map[string]any {
		v := collections.Slice(args...)
		m := map[string]any{"type": fmt.Sprintf("%T", v), "concat": concatCheck(v)}
		if rs, ok := v.(resource.Resources); ok {
			var names []string
			for _, r := range rs {
				names = append(names, r.Name())
			}
			m["names"] = names
		}
		return m
	}
	direct := func(in any) map[string]any {
		v, err := r1.(slicer).Slice(in)
		m := map[string]any{"type": fmt.Sprintf("%T", v)}
		if err != nil {
			m["err"] = err.Error()
		}
		return m
	}
	slices := map[string]any{
		"two":         sliceRec(r1, r2),
		"one":         sliceRec(r1),
		"mixed":       sliceRec(r1, "x"),
		"stringFirst": sliceRec("x", r1),
		"none":        sliceRec(),
		"direct-any":  direct([]any{r1, r2}),
		"direct-res":  direct(resource.Resources{r1}),
		"direct-bad":  direct([]any{r1, 5}),
		"direct-str":  direct("x"),
		"direct-nil":  direct(nil),
		"concat-any":  map[string]any{"concat": concatCheck([]any{r1, r2})},
		"concat-res":  map[string]any{"concat": concatCheck(resource.Resources{r1, r2})},
		"concat-str":  map[string]any{"concat": concatCheck([]string{"a"})},
	}

	outRec := map[string]any{
		"arch":      rsupport.Arch(),
		"cases":     cases,
		"doc":       doc.String(),
		"replaced":  string(replaced),
		"panics":    panics,
		"slices":    slices,
		"published": site.PublishedFiles(),
		"log":       rsupport.Rel(absRoot, site.Log.String()),
	}
	if err := rsupport.WriteGz(filepath.Join(*out, "transform.json.gz"), outRec); err != nil {
		log.Fatal(err)
	}
}

func fieldOf(p *postpub.PostPublishResource, f string) string {
	// The placeholder: the prefix of p (read from Content's placeholder) + f + suffix.
	c, _ := p.Content(context.Background())
	s := c.(string)
	prefix := s[:len(s)-len("Content"+postpub.PostProcessSuffix)]
	return prefix + f + postpub.PostProcessSuffix
}

func ppRec(pp postpub.PostPublishedResource) map[string]any {
	c, _ := pp.(resource.ContentProvider).Content(context.Background())
	m := map[string]any{
		"content":      c,
		"relPermalink": pp.RelPermalink(),
		"permalink":    pp.Permalink(),
		"name":         pp.Name(),
		"title":        pp.Title(),
		"resourceType": pp.ResourceType(),
		"data":         rsupport.JSON(pp.Data()),
		"mediaType":    rsupport.JSON(pp.MediaType()),
	}
	return m
}

// replace is hugolib's postProcess loop (hugolib/hugo_sites_build.go
// handleFile) over one file's content.
func replace(content []byte, toPostProcess []postpub.PostPublishedResource) []byte {
	content = append([]byte(nil), content...)
	sort.SliceStable(toPostProcess, func(i, j int) bool { return i < j })
	k := 0
	for {
		l := bytes.Index(content[k:], []byte(postpub.PostProcessPrefix))
		if l == -1 {
			break
		}
		m := bytes.Index(content[k+l:], []byte(postpub.PostProcessSuffix)) + len(postpub.PostProcessSuffix)

		low, high := k+l, k+l+m

		field := content[low:high]

		forward := l + m

		for _, r := range toPostProcess {
			v, ok := r.GetFieldString(string(field))
			if ok {
				content = append(content[:low], append([]byte(v), content[high:]...)...)
				forward = len(v)
				break
			}
		}

		k += forward
	}
	return content
}

// concatCheck is the type switch of tpl/resources Namespace.Concat.
func concatCheck(r any) string {
	switch v := r.(type) {
	case resource.Resources:
		if len(v) == 0 {
			return "must provide one or more Resource objects to concat"
		}
		return "ok"
	case resource.ResourcesConverter:
		return "ok"
	default:
		return fmt.Sprintf("expected slice of Resource objects, received %T instead", r)
	}
}
