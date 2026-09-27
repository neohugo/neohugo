package main

import (
	"image"
)

// edgeCases (-mode edge) are adversarial geometry cases for the pixel-buffer
// guards in crates/libwebp-sys/src/encoder.rs: inputs on which libwebp fails
// before reading the pixels must give Go's "failed to encode" (not a Rust
// out-of-range error), and inputs that Go encodes without reading out of
// bounds must encode.
func edgeCases(p *pack) {
	hugo := hugoWebpOptions
	lossless := encOpts{Quality: 0}
	add := func(name string, img image.Image) {
		for _, o := range []struct {
			n string
			o encOpts
		}{{"hugo", hugo}, {"lossless", lossless}} {
			out, exp := encodeCase(img, o.o)
			p.add("edge2/"+name+"/"+o.n, img, o.o, out, exp)
		}
	}
	grad := func(n int) []byte {
		b := make([]byte, n)
		for i := range b {
			b[i] = byte(i*7 + i/13)
		}
		return b
	}

	// Negative strides: C reads rows at rgba + y*stride, so a single row is
	// read inside the buffer and Go encodes it.
	add("nrgba-negstride-1row", &image.NRGBA{Pix: grad(40), Stride: -40, Rect: image.Rect(0, 0, 10, 1)})
	add("rgba-negstride-1row", &image.RGBA{Pix: grad(40), Stride: -44, Rect: image.Rect(0, 0, 10, 1)})
	add("gray-negstride-1row", &image.Gray{Pix: grad(10), Stride: -10, Rect: image.Rect(0, 0, 10, 1)})
	// |stride| < 4*width after int truncation: Import fails before reading.
	add("nrgba-negstride-short", &image.NRGBA{Pix: grad(40), Stride: -39, Rect: image.Rect(0, 0, 10, 2)})
	// Go int stride truncated by C.int.
	add("nrgba-stride-2p31", &image.NRGBA{Pix: grad(400), Stride: 1 << 31, Rect: image.Rect(0, 0, 10, 10)})
	add("nrgba-stride-2p32+40", &image.NRGBA{Pix: grad(400), Stride: 1<<32 + 40, Rect: image.Rect(0, 0, 10, 10)})
	add("gray-stride-2p32+10", &image.Gray{Pix: grad(100), Stride: 1<<32 + 10, Rect: image.Rect(0, 0, 10, 10)})
	// Allocation of width*height ARGB words fails (> 2^34 bytes): no read.
	add("nrgba-huge-alloc-fails", &image.NRGBA{Pix: grad(16), Stride: 400000, Rect: image.Rect(0, 0, 100000, 100000)})
	// Gray above WEBP_MAX_DIMENSION: WebPEncode fails before reading, even
	// though the buffer is shorter than Max.X.
	gparent := &image.Gray{Pix: grad(20000), Stride: 20000, Rect: image.Rect(0, 0, 20000, 1)}
	add("gray-subimage-over-maxdim", gparent.SubImage(image.Rect(10000, 0, 20000, 1)))
	add("gray-1x20000-short", &image.Gray{Pix: grad(5), Stride: 1, Rect: image.Rect(0, 0, 1, 20000)})
	// Negative / zero Max: C sees width or height <= 0.
	add("nrgba-negative-max", &image.NRGBA{Pix: grad(400), Stride: 40, Rect: image.Rect(-10, -10, 0, 0)})
	add("gray-negative-max", &image.Gray{Pix: grad(100), Stride: 10, Rect: image.Rect(-10, -3, -2, 5)})
	// Negative Min: gowebp encodes Max.X x Max.Y from Pix[0].
	add("nrgba-negative-min", &image.NRGBA{Pix: grad(15 * 15 * 4), Stride: 60, Rect: image.Rect(-5, -5, 10, 10)})
	// Gray stride shorter than the width: overlapping rows.
	add("gray-short-stride", &image.Gray{Pix: grad(60), Stride: 3, Rect: image.Rect(0, 0, 12, 17)})
	add("gray-zero-stride", &image.Gray{Pix: grad(12), Stride: 0, Rect: image.Rect(0, 0, 12, 17)})
	// Odd sizes around the macroblock size, opaque and with alpha.
	for _, sz := range [][2]int{{15, 17}, {17, 15}, {31, 33}, {33, 1}, {1, 33}, {2, 3}} {
		n := &image.NRGBA{Pix: grad(sz[0] * sz[1] * 4), Stride: sz[0] * 4, Rect: image.Rect(0, 0, sz[0], sz[1])}
		add("nrgba-odd-"+itoa(sz[0])+"x"+itoa(sz[1]), n)
		o := &image.NRGBA{Pix: grad(sz[0] * sz[1] * 4), Stride: sz[0] * 4, Rect: image.Rect(0, 0, sz[0], sz[1])}
		for i := 3; i < len(o.Pix); i += 4 {
			o.Pix[i] = 255
		}
		add("nrgba-odd-opaque-"+itoa(sz[0])+"x"+itoa(sz[1]), o)
	}
}

func itoa(i int) string {
	if i < 0 {
		return "-" + itoa(-i)
	}
	if i < 10 {
		return string(rune('0' + i))
	}
	return itoa(i/10) + string(rune('0'+i%10))
}
