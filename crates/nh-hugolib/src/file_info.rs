//! Port of `hugolib/fileInfo.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).


// Wave B: port per the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/fileInfo.go (51 lines; 0/3 funcs executed)
//   types: fileInfo
//    L30-37: (fi *fileInfo) Open() (afero.File, error)
//    L39-44: (fi *fileInfo) Lang() string
//    L46-51: (fi *fileInfo) String() string
// ---------------------------------------------------------------------------
