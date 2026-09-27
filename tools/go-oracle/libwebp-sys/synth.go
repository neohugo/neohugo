package main

import (
	"fmt"
	"image"
	"image/color"
	"image/color/palette"
	"image/draw"
	"math/rand"

	"github.com/bep/gowebp/libwebp/webpoptions"
)

// convertToNRGBA mirrors gowebp internal/libwebp.ConvertToNRGBA.
func convertToNRGBA(src image.Image) *image.NRGBA {
	dst := image.NewNRGBA(src.Bounds())
	draw.Draw(dst, dst.Bounds(), src, src.Bounds().Min, draw.Src)
	return dst
}

// pixel returns a deterministic NRGBA colour for (x, y).
func pixel(pattern string, x, y, w, h int, rng *rand.Rand, alpha bool) color.NRGBA {
	var c color.NRGBA
	switch pattern {
	case "gradient":
		c = color.NRGBA{uint8(x * 255 / max(w-1, 1)), uint8(y * 255 / max(h-1, 1)), uint8((x + y) * 7), 255}
		if alpha {
			c.A = uint8((x*3 + y*5) * 255 / max(3*(w-1)+5*(h-1), 1))
		}
	case "noise":
		c = color.NRGBA{uint8(rng.Intn(256)), uint8(rng.Intn(256)), uint8(rng.Intn(256)), 255}
		if alpha {
			c.A = uint8(rng.Intn(256))
		}
	case "checker":
		if (x/4+y/4)%2 == 0 {
			c = color.NRGBA{250, 20, 20, 255}
		} else {
			c = color.NRGBA{10, 40, 230, 255}
		}
		if alpha && (x/8+y/8)%3 == 0 {
			c.A = 0
		}
	case "flat":
		c = color.NRGBA{120, 180, 60, 255}
		if alpha {
			c.A = 128
		}
	default:
		panic(pattern)
	}
	return c
}

func makeImage(kind string, w, h int, pattern string, seed int64) image.Image {
	rng := rand.New(rand.NewSource(seed))
	r := image.Rect(0, 0, w, h)
	alpha := kind == "nrgba-alpha" || kind == "rgba-alpha"
	var img draw.Image
	switch kind {
	case "nrgba", "nrgba-alpha":
		img = image.NewNRGBA(r)
	case "rgba", "rgba-alpha":
		img = image.NewRGBA(r)
	case "gray":
		img = image.NewGray(r)
	case "ycbcr":
		// Filled below through its planes.
		y := image.NewYCbCr(r, image.YCbCrSubsampleRatio420)
		for yy := 0; yy < h; yy++ {
			for xx := 0; xx < w; xx++ {
				c := pixel(pattern, xx, yy, w, h, rng, false)
				Y, Cb, Cr := color.RGBToYCbCr(c.R, c.G, c.B)
				y.Y[y.YOffset(xx, yy)] = Y
				ci := y.COffset(xx, yy)
				y.Cb[ci] = Cb
				y.Cr[ci] = Cr
			}
		}
		return y
	case "paletted":
		img = image.NewPaletted(r, palette.Plan9)
	default:
		panic(kind)
	}
	for yy := 0; yy < h; yy++ {
		for xx := 0; xx < w; xx++ {
			img.Set(xx, yy, pixel(pattern, xx, yy, w, h, rng, alpha))
		}
	}
	return img
}

// synthCases adds synthetic images covering every gowebp code path.
func synthCases(p *pack) {
	hugo := hugoWebpOptions
	n := 0
	sizes := [][2]int{{1, 1}, {2, 2}, {3, 5}, {5, 3}, {16, 16}, {17, 33}, {64, 48}, {127, 1}, {1, 127}, {160, 120}, {301, 199}}
	kinds := []string{"nrgba", "nrgba-alpha", "rgba", "rgba-alpha", "gray", "ycbcr", "paletted"}
	patterns := []string{"gradient", "noise", "checker", "flat"}
	for _, sz := range sizes {
		for _, k := range kinds {
			for _, pat := range patterns {
				if pat == "noise" && sz[0]*sz[1] > 64*48 {
					continue
				}
				n++
				img := makeImage(k, sz[0], sz[1], pat, int64(n))
				out, exp := encodeCase(img, hugo)
				p.add(fmt.Sprintf("synth/%s/%dx%d/%s", k, sz[0], sz[1], pat), img, hugo, out, exp)
			}
		}
	}

	// Option matrix on a few base images.
	bases := []struct {
		name string
		img  image.Image
	}{
		{"nrgba-64x48-noise", makeImage("nrgba", 64, 48, "noise", 1001)},
		{"nrgba-alpha-64x48-gradient", makeImage("nrgba-alpha", 64, 48, "gradient", 1002)},
		{"gray-33x17-noise", makeImage("gray", 33, 17, "noise", 1003)},
		{"rgba-alpha-40x40-checker", makeImage("rgba-alpha", 40, 40, "checker", 1004)},
	}
	qualities := []int{0, 1, 25, 50, 75, 90, 100, 101, -5}
	presets := []webpoptions.EncodingPreset{0, 1, 2, 3, 4, 5, 6, -1, (1 << 32) + 2}
	for _, b := range bases {
		for _, q := range qualities {
			for _, pr := range presets {
				for _, sharp := range []bool{false, true} {
					o := webpoptions.EncodingOptions{Quality: q, EncodingPreset: pr, UseSharpYuv: sharp}
					out, exp := encodeCase(b.img, o)
					p.add(fmt.Sprintf("opts/%s/q%d/p%d/s%v", b.name, q, pr, sharp), b.img, o, out, exp)
				}
			}
		}
	}

	// Geometry edge cases.
	addEdge := func(name string, img image.Image) {
		out, exp := encodeCase(img, hugo)
		p.add("edge/"+name, img, hugo, out, exp)
	}
	parent := makeImage("nrgba", 40, 40, "gradient", 2001).(*image.NRGBA)
	// Non-zero Min: gowebp uses Max.X x Max.Y (not Dx x Dy) from &Pix[0].
	addEdge("nrgba-subimage-5-5-20-20", parent.SubImage(image.Rect(5, 5, 20, 20)))
	// Padded stride.
	addEdge("nrgba-subimage-0-0-10-10", parent.SubImage(image.Rect(0, 0, 10, 10)))
	gparent := makeImage("gray", 40, 40, "noise", 2002).(*image.Gray)
	addEdge("gray-subimage-3-4-20-25", gparent.SubImage(image.Rect(3, 4, 20, 25)))
	addEdge("gray-subimage-0-0-9-7", gparent.SubImage(image.Rect(0, 0, 9, 7)))
	rparent := makeImage("rgba-alpha", 30, 30, "gradient", 2003).(*image.RGBA)
	addEdge("rgba-subimage-2-2-12-12", rparent.SubImage(image.Rect(2, 2, 12, 12)))
	// Empty image: &Pix[0] panics in Go.
	addEdge("nrgba-0x0", image.NewNRGBA(image.Rect(0, 0, 0, 0)))
	addEdge("gray-0x0", image.NewGray(image.Rect(0, 0, 0, 0)))
	// Stride smaller than 4*width: WebPPictureImportRGBA fails before reading.
	addEdge("nrgba-short-stride", &image.NRGBA{Pix: make([]uint8, 400), Stride: 8, Rect: image.Rect(0, 0, 10, 10)})
	// Over WEBP_MAX_DIMENSION (16383).
	addEdge("nrgba-16383x1", makeImage("nrgba", 16383, 1, "gradient", 2004))
	addEdge("nrgba-16384x1", makeImage("nrgba", 16384, 1, "gradient", 2005))
	addEdge("gray-1x16384", makeImage("gray", 1, 16384, "gradient", 2006))
	// A fully transparent image and an opaque RGBA one.
	tr := image.NewNRGBA(image.Rect(0, 0, 24, 24))
	addEdge("nrgba-transparent", tr)
	addEdge("rgba-opaque-white", func() image.Image {
		m := image.NewRGBA(image.Rect(0, 0, 24, 24))
		draw.Draw(m, m.Bounds(), image.White, image.Point{}, draw.Src)
		return m
	}())
}
