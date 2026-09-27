// Command libwebp-sys is the Go oracle for crates/libwebp-sys: it encodes
// images with github.com/bep/gowebp (the exact version Hugo uses) and writes
// fixture packs (inputs + expected WebP bytes) for the Rust tests.
//
// Usage:
//
//	go run ./tools/go-oracle/libwebp-sys -mode fixtures -out crates/libwebp-sys/tests/fixtures/webp \
//	    -site <pristine-seeksnack> -golden <golden/canonical>
//	go run ./tools/go-oracle/libwebp-sys -mode golden -out <scratch>/work/libwebp-sys/golden-pack \
//	    -site <pristine-seeksnack> -golden <golden/canonical>
//	go run ./tools/go-oracle/libwebp-sys -mode fuzz -out crates/libwebp-sys/tests/fixtures/webp -n 10000 -seed 1
//	go run ./tools/go-oracle/libwebp-sys -mode edge -out crates/libwebp-sys/tests/fixtures/webp-edge
package main

import (
	"bytes"
	"flag"
	"fmt"
	"image"
	"os"
	"regexp"
	"sort"
	"strings"

	"github.com/bep/gowebp/libwebp"
	"github.com/bep/gowebp/libwebp/webpoptions"
)

type encOpts = webpoptions.EncodingOptions

// encodeCase runs libwebp.Encode and records the outcome.
func encodeCase(img image.Image, o encOpts) (out []byte, expect string) {
	defer func() {
		if r := recover(); r != nil {
			out = nil
			expect = "panic"
		}
	}()
	var buf bytes.Buffer
	if err := libwebp.Encode(&buf, img, o); err != nil {
		return nil, "err:" + err.Error()
	}
	return buf.Bytes(), "ok"
}

// fixtureGolden selects the golden cases that are checked in (small, and
// covering every source kind: favicon PNGs (NRGBA, and RGB PNG -> RGBA),
// RGBA-PNG logos with alpha, an RGB PNG (RGBA type), a paletted PNG, and
// JPEG-derived product images).
var (
	fixtureGoldenFixed = regexp.MustCompile(`^(` +
		`images/favicon/favicon-32x32_hu_[0-9a-f]+\.webp|` +
		`images/favicon/mstile-70x70_hu_[0-9a-f]+\.webp|` +
		`companies/(frito-lay|classic-foods-inc|berli-jucker[^/]*)/[^/]+_hu_[0-9a-f]+\.webp` +
		`)$`)
	fixtureGoldenJPEG = 2 // plus this many JPEG-derived 300x240 product webps
)

func main() {
	mode := flag.String("mode", "fixtures", "fixtures | golden | fuzz | edge")
	out := flag.String("out", "", "output directory")
	site := flag.String("site", "", "pristine seeksnack site")
	golden := flag.String("golden", "", "golden site output")
	fuzzN := flag.Int("n", 3000, "fuzz: number of cases")
	fuzzSeed := flag.Uint64("seed", 1, "fuzz: first case seed")
	flag.Parse()
	if *mode == "fuzz" && *out != "" {
		fuzzCases(*out, *fuzzSeed, *fuzzN)
		return
	}
	if *mode == "edge" && *out != "" {
		p := newPack(*out)
		edgeCases(p)
		p.close()
		fmt.Fprintf(os.Stderr, "wrote %d cases to %s\n", p.n, *out)
		return
	}
	if *out == "" || *site == "" || *golden == "" {
		flag.Usage()
		os.Exit(2)
	}

	gw := goldenWebps(*site, *golden)
	var keys []string
	for k := range gw {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	p := newPack(*out)
	defer p.close()
	switch *mode {
	case "fixtures":
		synthCases(p)
		jpegs := 0
		for _, k := range keys {
			c := gw[k]
			sz := c.img.Bounds().Size()
			if sz.X*sz.Y > 300*240 {
				continue
			}
			if !fixtureGoldenFixed.MatchString(k) {
				if strings.HasPrefix(k, "companies/") || strings.HasPrefix(k, "images/") || jpegs >= fixtureGoldenJPEG {
					continue
				}
				jpegs++
			}
			addGolden(p, k, c)
		}
	case "golden":
		for _, k := range keys {
			addGolden(p, k, gw[k])
		}
	default:
		fmt.Fprintln(os.Stderr, "unknown mode", *mode)
		os.Exit(2)
	}
	fmt.Fprintf(os.Stderr, "wrote %d cases to %s\n", p.n, *out)
}

// addGolden re-encodes the captured image (to double check the oracle) and
// stores it with the golden bytes as expected output.
func addGolden(p *pack, name string, c capturedWebp) {
	out, exp := encodeCase(c.img, hugoWebpOptions)
	if exp != "ok" || !bytes.Equal(out, c.out) {
		panic("golden re-encode mismatch: " + name)
	}
	p.add("golden/"+name, c.img, hugoWebpOptions, c.out, "ok")
}
