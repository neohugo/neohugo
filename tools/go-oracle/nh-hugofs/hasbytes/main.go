// Command hasbytes is the Go oracle for hugofs.NewHasBytesReceiver in
// crates/nh-hugofs (Wave B task T05): the publish filesystem wrapper that
// records which written files contain the post-process placeholder prefix
// (`__h_pp_l1`) or the deferred-template prefix (`__hdeferred/`).
//
//	go run ./tools/go-oracle/nh-hugofs/hasbytes [-out crates/nh-hugofs/tests/fixtures/hasbytes]
//
// The wrapper is set up exactly as deps.Deps.Init does (patterns in that order,
// shouldCheck = the default media types' IsTextSuffix of the file extension)
// over a BasePathFs on a temporary publish dir. Each case writes one file in
// seeded random chunks through Create or OpenFile and records the callbacks
// made on Close, in order, and the bytes that reached the disk.
package main

import (
	"flag"
	"fmt"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"strings"

	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/spf13/afero"
)

const (
	deferredPrefix = "__hdeferred/" // tpl.HugoDeferredTemplatePrefix
	postPrefix     = "__h_pp_l1"    // postpub.PostProcessPrefix
)

var names = []string{
	"index.html", "sub/dir/page.html", "a.css", "b.js", "img.png", "data.json", "feed.xml",
	"noext", "x.HTML", "robots.txt", "sitemap.xml", "manifest.webmanifest", "a.svg", "b.jpg",
	"c.webp", "d.scss", "e.md", "f.csv", "g.ics", "h.toml", "i.yaml", "j.mjs", "k.ts", "l.htm",
	"dir.with.dots/file", "_redirects", ".hidden.html", "trailing.html.", "ภาษาไทย.html", "sp ace.css",
}

var fillers = []string{
	"", "x", "hello world ", "<p>", "__h_pp", "__h_pp_l", "_l1", "__hdeferred", "/", "__", "éไ",
	"__h_pp_l1_0_", "__hdeferred/abc", "\x00\x00", strings.Repeat("a", 40),
}

func content(r *rand.Rand) string {
	var b strings.Builder
	n := r.Intn(12)
	for range n {
		b.WriteString(fillers[r.Intn(len(fillers))])
	}
	return b.String()
}

func main() {
	out := flag.String("out", "crates/nh-hugofs/tests/fixtures/hasbytes", "output directory")
	flag.Parse()

	tmp, err := os.MkdirTemp("", "nhfs-hasbytes-")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()
	publishDir := filepath.Join(tmp, "public")

	mediaTypes := media.DefaultTypes
	shouldCheck := func(name string) bool {
		ext := strings.TrimPrefix(filepath.Ext(name), ".")
		return mediaTypes.IsTextSuffix(ext)
	}

	var cases []map[string]any
	r := rand.New(rand.NewSource(20260928))
	for i := range 1500 {
		name := names[i%len(names)]
		if i >= len(names) {
			name = names[r.Intn(len(names))]
		}
		s := content(r)
		// Chunk boundaries.
		var chunks []string
		rest := s
		for len(rest) > 0 {
			n := 1 + r.Intn(len(rest))
			if r.Intn(3) == 0 {
				n = len(rest)
			}
			chunks = append(chunks, rest[:n])
			rest = rest[n:]
		}
		if len(chunks) == 0 && r.Intn(2) == 0 {
			chunks = append(chunks, "")
		}
		mode := []string{"create", "openfile", "openfile-rdwr", "openfile-read"}[r.Intn(4)]

		var calls [][]string
		cb := func(n string, match []byte) {
			calls = append(calls, []string{n, string(match)})
		}
		pub := hugofs.NewBasePathFs(afero.NewOsFs(), publishDir)
		fs := hugofs.NewHasBytesReceiver(pub, shouldCheck, cb, []byte(deferredPrefix), []byte(postPrefix))
		if err := fs.MkdirAll(filepath.Dir(name), 0o777); err != nil {
			log.Fatal(err)
		}
		if mode == "openfile-read" {
			if err := afero.WriteFile(pub, name, []byte("seed"), 0o666); err != nil {
				log.Fatal(err)
			}
		}

		var jchunks []any
		for _, ch := range chunks {
			jchunks = append(jchunks, goval.Str(ch))
		}
		c := map[string]any{"name": name, "mode": mode, "chunks": jchunks, "shouldCheck": shouldCheck(name)}
		var f afero.File
		switch mode {
		case "create":
			f, err = fs.Create(name)
		case "openfile":
			f, err = fs.OpenFile(name, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0o666)
		case "openfile-rdwr":
			f, err = fs.OpenFile(name, os.O_RDWR|os.O_CREATE|os.O_TRUNC, 0o666)
		case "openfile-read":
			f, err = fs.OpenFile(name, os.O_RDONLY, 0)
		}
		if err != nil {
			log.Fatal(err)
		}
		if mode != "openfile-read" {
			for _, ch := range chunks {
				if _, err := f.Write([]byte(ch)); err != nil {
					log.Fatal(err)
				}
			}
		}
		if err := f.Close(); err != nil {
			log.Fatal(err)
		}
		b, err := afero.ReadFile(pub, name)
		if err != nil {
			log.Fatal(err)
		}
		c["calls"] = calls
		c["written"] = goval.Str(string(b))
		cases = append(cases, c)
		if err := pub.RemoveAll(name); err != nil {
			log.Fatal(err)
		}
	}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "hasbytes.json.gz"), map[string]any{
		"patterns": []string{deferredPrefix, postPrefix},
	}, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Printf("hasbytes: %d cases\n", len(cases))
}
