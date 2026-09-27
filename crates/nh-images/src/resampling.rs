//! Port of `resources/images/resampling.go`.
//!
//! only box is exercised; other kernels have their own FMA sites (verify with objdump before porting)
//!
//! Owner: Wave B task T10 (images).


// Wave B: port or stub per the checklist below (not exercised by seeksnack unless noted).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/resampling.go (214 lines; 0/6 funcs executed)
//   types: resamp
//    L177-179: (r resamp) String() string
//    L181-183: (r resamp) Support() float32
//    L185-187: (r resamp) Kernel(x float32) float32
//    L189-200: bcspline(x, b, c float32) float32
//    L202-207: absf32(x float32) float32
//    L209-214: sinc(x float32) float32
// ---------------------------------------------------------------------------
