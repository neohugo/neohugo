//go:build strconvslow

package main

import (
	_ "unsafe" // for go:linkname
)

// haveSlowPath reports whether strconvOptimize really controls
// internal/strconv (see slowpath_stub.go for the default build).
const haveSlowPath = true

// strconvOptimize is internal/strconv.optimize (the SetOptimize test hook),
// reached through go:linkname so the oracle can produce slow-path
// (multiprecision decimal) results. Build with
// -tags strconvslow -ldflags=-checklinkname=0.
//
//go:linkname strconvOptimize internal/strconv.optimize
var strconvOptimize bool
