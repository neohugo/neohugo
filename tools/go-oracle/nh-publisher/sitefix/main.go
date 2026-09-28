// Command sitefix is the second phase of the publisher site oracle
// (tools/go-oracle/nh-publisher/site, Wave B task T07). It needs no hugolib
// (so it builds without cgo for linux/arm64): for every build recorded by
// phase 1 it recomputes every written file with the publisher's own chain
// (createTransformerChain + Chain.Apply, through publisher.OracleChain) and the
// recorded minifier client configuration, compares with phase 1, checks that
// the Go collector over the recorded HTML gives the build's hugo_stats.json,
// and writes the full records ($FULL-arm64) and the checked-in subset
// (crates/nh-publisher/tests/fixtures/site). Run it as linux/arm64 under qemu
// (the minifiers' FMA-sensitive colour conversions, HANDOFF.md §3):
//
//	NH_T07_ORACLE_ARCH=arm64 go run ./tools/go-oracle/nh-publisher/sitefix -full $FULL
package main

import (
	"bytes"
	"compress/gzip"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"flag"
	"io"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/minifiers"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/publisher"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-publisher/osupport"
)

// hooks is set by the overlay-added hook file.
type oracleHooks struct {
	setConf func(minifiers.MinifyConfig)
	chain   func(min minifiers.Client, d publisher.Descriptor) ([]byte, error)
	collect func(conf config.BuildStats, docs [][]byte) publisher.HTMLElements
}

var hooks *oracleHooks

const hookFile = `package main

import (
	"github.com/neohugo/neohugo/minifiers"
	"github.com/neohugo/neohugo/publisher"
)

func init() {
	hooks = &oracleHooks{
		setConf: func(c minifiers.MinifyConfig) { minifiers.OracleConf = c },
		chain:   publisher.OracleChain,
		collect: publisher.OracleCollect,
	}
}
`

type variant struct {
	name   string
	subset bool
}

type pubRec struct {
	T      string `json:"t"`
	Path   string `json:"path"`
	Format string `json:"format"`
	MT     string `json:"mt"`
	HTML   bool   `json:"html"`
	Abs    string `json:"abs"`
	Gen    bool   `json:"gen"`
	LR     string `json:"lr,omitempty"`
	In     any    `json:"in"`
	Out    any    `json:"out,omitempty"`
	Sha    string `json:"sha,omitempty"`
	N      int    `json:"n"`
}

type buildRec struct {
	T          string   `json:"t"`
	Site       string   `json:"site"`
	Variant    string   `json:"variant"`
	Types      [][2]any `json:"types"`
	Formats    [][3]any `json:"formats"`
	Minify     any      `json:"minify"`
	BuildStats [4]bool  `json:"buildStats"` // enable, disableTags, disableClasses, disableIDs
	Stats      *string  `json:"stats"`
	Arch       string   `json:"arch"`
	Pubs       int      `json:"pubs"`
	Extra      []string `json:"extra,omitempty"`
	Elements   any      `json:"elements,omitempty"`
}

// confFromDump is the inverse of dumpConf (the JSON round trip of phase 1's
// record).
func confFromDump(raw json.RawMessage) minifiers.MinifyConfig {
	var d struct {
		MinifyOutput, DisableHTML, DisableCSS, DisableJS, DisableJSON, DisableSVG, DisableXML bool
		HTML                                                                                  struct {
			KeepComments, KeepConditionalComments, KeepSpecialComments, KeepDefaultAttrVals, KeepDocumentTags, KeepEndTags, KeepQuotes, KeepWhitespace bool
			TemplateDelims                                                                                                                             []string
		}
		CSS struct {
			KeepCSS2  bool
			Precision int
			Inline    bool
		}
		JS struct {
			Precision    int
			KeepVarNames bool
			Version      int
		}
		JSON struct {
			Precision   int
			KeepNumbers bool
		}
		SVG struct {
			KeepComments bool
			Precision    int
			Inline       bool
		}
		XML struct{ KeepWhitespace bool }
	}
	if err := json.Unmarshal(raw, &d); err != nil {
		log.Fatal(err)
	}
	var c minifiers.MinifyConfig
	c.MinifyOutput, c.DisableHTML, c.DisableCSS, c.DisableJS = d.MinifyOutput, d.DisableHTML, d.DisableCSS, d.DisableJS
	c.DisableJSON, c.DisableSVG, c.DisableXML = d.DisableJSON, d.DisableSVG, d.DisableXML
	h := &c.Tdewolff.HTML
	h.KeepComments, h.KeepConditionalComments, h.KeepSpecialComments = d.HTML.KeepComments, d.HTML.KeepConditionalComments, d.HTML.KeepSpecialComments
	h.KeepDefaultAttrVals, h.KeepDocumentTags, h.KeepEndTags = d.HTML.KeepDefaultAttrVals, d.HTML.KeepDocumentTags, d.HTML.KeepEndTags
	h.KeepQuotes, h.KeepWhitespace = d.HTML.KeepQuotes, d.HTML.KeepWhitespace
	h.TemplateDelims = [2]string{d.HTML.TemplateDelims[0], d.HTML.TemplateDelims[1]}
	c.Tdewolff.CSS.KeepCSS2, c.Tdewolff.CSS.Precision, c.Tdewolff.CSS.Inline = d.CSS.KeepCSS2, d.CSS.Precision, d.CSS.Inline
	c.Tdewolff.JS.Precision, c.Tdewolff.JS.KeepVarNames, c.Tdewolff.JS.Version = d.JS.Precision, d.JS.KeepVarNames, d.JS.Version
	c.Tdewolff.JSON.Precision, c.Tdewolff.JSON.KeepNumbers = d.JSON.Precision, d.JSON.KeepNumbers
	c.Tdewolff.SVG.KeepComments, c.Tdewolff.SVG.Precision, c.Tdewolff.SVG.Inline = d.SVG.KeepComments, d.SVG.Precision, d.SVG.Inline
	c.Tdewolff.XML.KeepWhitespace = d.XML.KeepWhitespace
	return c
}

func readRecords(fn string) (json.RawMessage, []json.RawMessage) {
	f, err := os.Open(fn)
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = f.Close() }()
	gz, err := gzip.NewReader(f)
	if err != nil {
		log.Fatal(err)
	}
	dec := json.NewDecoder(gz)
	var first json.RawMessage
	var rest []json.RawMessage
	for {
		var m json.RawMessage
		if err := dec.Decode(&m); err == io.EOF {
			break
		} else if err != nil {
			log.Fatal(err)
		}
		if first == nil {
			first = m
		} else {
			rest = append(rest, m)
		}
	}
	return first, rest
}

func decB(raw json.RawMessage) []byte {
	var s string
	if err := json.Unmarshal(raw, &s); err == nil {
		return []byte(s)
	}
	var o struct {
		B64 string `json:"b64"`
	}
	if err := json.Unmarshal(raw, &o); err != nil {
		log.Fatal(err)
	}
	b, err := base64.StdEncoding.DecodeString(o.B64)
	if err != nil {
		log.Fatal(err)
	}
	return b
}

func fixtures(full, fullArm, out string) {
	files, err := filepath.Glob(filepath.Join(full, "*.jsonl.gz"))
	if err != nil {
		log.Fatal(err)
	}
	sort.Strings(files)
	if len(files) == 0 {
		log.Fatalf("no records in %s", full)
	}
	for _, fn := range files {
		v := variant{name: strings.TrimSuffix(filepath.Base(fn), ".jsonl.gz")}
		brRaw, pubRaws := readRecords(filepath.Join(full, v.name+".jsonl.gz"))
		var br struct {
			buildRec
			MinifyRaw json.RawMessage      `json:"minify"`
			TypesRaw  [][2]json.RawMessage `json:"types"`
		}
		if err := json.Unmarshal(brRaw, &br); err != nil {
			log.Fatal(err)
		}
		v.subset = br.Site == "docs"
		var types media.Types
		for _, t := range br.TypesRaw {
			var typ string
			var sufs []string
			_ = json.Unmarshal(t[0], &typ)
			_ = json.Unmarshal(t[1], &sufs)
			mt, err := media.FromStringAndExt(typ, sufs...)
			if err != nil {
				log.Fatal(err)
			}
			types = append(types, mt)
		}
		var formats output.Formats
		for _, f := range br.Formats {
			formats = append(formats, output.Format{Name: f[0].(string), MediaType: media.Type{Type: f[1].(string)}, IsHTML: f[2].(bool)})
		}
		hooks.setConf(confFromDump(br.MinifyRaw))
		client, err := minifiers.New(types, formats, nil)
		if err != nil {
			log.Fatal(err)
		}

		type pub struct {
			raw  map[string]json.RawMessage
			path string
			html bool
			in   []byte
			out  []byte
		}
		var pubs []pub
		differ := 0
		for _, raw := range pubRaws {
			var m map[string]json.RawMessage
			if err := json.Unmarshal(raw, &m); err != nil {
				log.Fatal(err)
			}
			var p pubRec
			if err := json.Unmarshal(raw, &p); err != nil {
				log.Fatal(err)
			}
			in := decB(m["in"])
			native := decB(m["out"])
			d := publisher.Descriptor{
				Src:                 bytes.NewReader(in),
				OutputFormat:        output.Format{Name: p.Format, MediaType: media.Type{Type: p.MT}, IsHTML: p.HTML},
				TargetPath:          p.Path,
				AddHugoGeneratorTag: p.Gen,
				AbsURLPath:          p.Abs,
			}
			if p.LR != "" {
				log.Fatalf("%s: livereload in a build", v.name)
			}
			got, err := hooks.chain(client, d)
			if err != nil {
				log.Fatalf("%s %s: chain: %v", v.name, p.Path, err)
			}
			if !bytes.Equal(got, native) {
				differ++
			}
			pubs = append(pubs, pub{raw: m, path: p.Path, html: p.HTML, in: in, out: got})
		}

		conf := config.BuildStats{Enable: br.BuildStats[0], DisableTags: br.BuildStats[1], DisableClasses: br.BuildStats[2], DisableIDs: br.BuildStats[3]}
		var allHTML [][]byte
		for _, p := range pubs {
			if p.html {
				allHTML = append(allHTML, p.out)
			}
		}
		fullElements := hooks.collect(conf, allHTML)
		if br.Stats != nil && differ == 0 {
			// The Go collector over the recorded outputs gives the build's hugo_stats.json.
			var buf bytes.Buffer
			enc := json.NewEncoder(&buf)
			enc.SetEscapeHTML(false)
			enc.SetIndent("", "  ")
			if err := enc.Encode(publisher.PublishStats{HTMLElements: fullElements}); err != nil {
				log.Fatal(err)
			}
			if buf.String() != *br.Stats {
				log.Fatalf("%s: collector over the recorded outputs != hugo_stats.json", v.name)
			}
		}

		// The arm64 full records.
		fw, err := osupport.Create(filepath.Join(fullArm, v.name+".jsonl.gz"))
		if err != nil {
			log.Fatal(err)
		}
		br.Arch = osupport.Arch()
		br.Elements = fullElements
		if err := fw.Write(buildOut(br.buildRec, br.MinifyRaw, br.TypesRaw)); err != nil {
			log.Fatal(err)
		}
		for _, p := range pubs {
			p.raw["out"], _ = json.Marshal(osupport.B(p.out))
			if err := fw.Write(p.raw); err != nil {
				log.Fatal(err)
			}
		}
		if err := fw.Close(); err != nil {
			log.Fatal(err)
		}

		// The checked-in subset.
		keep := make([]bool, len(pubs))
		var html []int
		for i, p := range pubs {
			if !p.html || !v.subset {
				keep[i] = true
			} else {
				html = append(html, i)
			}
		}
		if v.subset {
			step := len(html)/15 + 1
			for j, i := range html {
				if j%step == 0 || strings.HasSuffix(pubs[i].path, "404.html") || pubs[i].path == "/index.html" {
					keep[i] = true
				}
			}
		}
		var subsetHTML [][]byte
		cw, err := osupport.Create(filepath.Join(out, v.name+".jsonl.gz"))
		if err != nil {
			log.Fatal(err)
		}
		kept := 0
		for i, p := range pubs {
			if keep[i] && p.html {
				subsetHTML = append(subsetHTML, p.out)
			}
		}
		br.Elements = hooks.collect(conf, subsetHTML)
		if !v.subset {
			br.Elements = fullElements
		}
		if err := cw.Write(buildOut(br.buildRec, br.MinifyRaw, br.TypesRaw)); err != nil {
			log.Fatal(err)
		}
		for i, p := range pubs {
			if !keep[i] {
				continue
			}
			kept++
			sum := sha256.Sum256(p.out)
			rec := p.raw
			delete(rec, "out")
			rec["sha"], _ = json.Marshal(hex.EncodeToString(sum[:]))
			if len(p.out) <= 4096 {
				rec["out"], _ = json.Marshal(osupport.B(p.out))
			}
			if err := cw.Write(rec); err != nil {
				log.Fatal(err)
			}
		}
		if err := cw.Close(); err != nil {
			log.Fatal(err)
		}
		log.Printf("site: %s: %d publishes (%d checked in), %d differ from the %s build, collector on %d HTML files", v.name, len(pubs), kept, differ, "native", len(allHTML))
	}
}

func buildOut(br buildRec, minify json.RawMessage, types [][2]json.RawMessage) map[string]any {
	return map[string]any{
		"t": "build", "site": br.Site, "variant": br.Variant, "types": types, "formats": br.Formats,
		"minify": minify, "buildStats": br.BuildStats, "stats": br.Stats, "arch": br.Arch, "pubs": br.Pubs,
		"elements": br.Elements,
	}
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	full := flag.String("full", "", "directory of the full (phase 1) records")
	out := flag.String("out", "crates/nh-publisher/tests/fixtures/site", "output directory of the checked-in fixtures")
	flag.Parse()

	if *full == "" {
		log.Fatal("usage: sitefix -full DIR")
	}
	absFull, err := filepath.Abs(*full)
	if err != nil {
		log.Fatal(err)
	}
	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}
	if !osupport.IsChild() {
		patches := append([]osupport.Patch{osupport.MinifiersPatch}, osupport.PublisherPatches...)
		if err := osupport.RunOverlaid(absRoot, "./tools/go-oracle/nh-publisher/sitefix", patches, "hook_overlay.go", hookFile,
			[]string{"-root", absRoot, "-full", absFull, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if hooks == nil {
		log.Fatal("sitefix: hooks not installed (not run through the overlay)")
	}
	fixtures(absFull, absFull+"-arm64", *out)
}
