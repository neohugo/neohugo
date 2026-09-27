// Port of the cgo preamble of
// github.com/bep/gowebp@v0.3.0/internal/libwebp/a__encoder.go.
//
// The two encode functions are copied verbatim from the Go file's C preamble;
// only `static` was dropped and a `gowebp_` prefix added so that Rust can
// link against them. The layout helpers at the bottom are new (used by the
// Rust side to verify its #[repr(C)] mirror of WebPConfig).
#include <stdlib.h>
#include <string.h> // for memset
#include <stddef.h>
#include "src/webp/encode.h"

uint8_t* gowebp_encodeNRGBA(WebPConfig* config, const uint8_t* rgba, int width, int height, int stride, size_t* output_size) {
	WebPPicture pic;
	WebPMemoryWriter wrt;
	int ok;
	if (!WebPPictureInit(&pic)) {
		return NULL;
	}
	pic.use_argb = 1;
	pic.width = width;
	pic.height = height;
	pic.writer = WebPMemoryWrite;
	pic.custom_ptr = &wrt;
	WebPMemoryWriterInit(&wrt);
	ok = WebPPictureImportRGBA(&pic, rgba, stride) && WebPEncode(config, &pic);
	WebPPictureFree(&pic);
	if (!ok) {
		WebPMemoryWriterClear(&wrt);
		return NULL;
	}
	*output_size = wrt.size;
	return wrt.mem;
}

uint8_t* gowebp_encodeGray(WebPConfig* config, uint8_t *y, int width, int height, int stride, size_t* output_size) {
	WebPPicture pic;
	WebPMemoryWriter wrt;

	int ok;
	if (!WebPPictureInit(&pic)) {
		return NULL;
	}

	pic.use_argb = 0;
	pic.width = width;
	pic.height = height;
	pic.y_stride = stride;
	pic.writer = WebPMemoryWrite;
	pic.custom_ptr = &wrt;
	WebPMemoryWriterInit(&wrt);

	const int uvWidth = (int)(((int64_t)width + 1) >> 1);
  	const int uvHeight = (int)(((int64_t)height + 1) >> 1);
  	const int uvStride = uvWidth;
	const int uvSize = uvStride * uvHeight;
	const int gray = 128;
	uint8_t* chroma;

	chroma = malloc(uvSize);
	if (!chroma) {
		return 0;
	}
	memset(chroma, gray, uvSize);

	pic.y = y;
	pic.u = chroma;
	pic.v = chroma;
	pic.uv_stride = uvStride;

	ok = WebPEncode(config, &pic);

	free(chroma);

	WebPPictureFree(&pic);
	if (!ok) {
		WebPMemoryWriterClear(&wrt);
		return NULL;
	}
	*output_size = wrt.size;
	return wrt.mem;

}

// Frees memory returned by the encode functions (Go: C.free).
void gowebp_free(void* p) {
	free(p);
}

// Layout probes for the Rust mirror of WebPConfig.
size_t gowebp_sizeof_config(void) {
	return sizeof(WebPConfig);
}

size_t gowebp_offsetof_config_use_sharp_yuv(void) {
	return offsetof(WebPConfig, use_sharp_yuv);
}

size_t gowebp_offsetof_config_qmax(void) {
	return offsetof(WebPConfig, qmax);
}
