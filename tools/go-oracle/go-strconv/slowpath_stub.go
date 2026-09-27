//go:build !strconvslow

package main

// Default build (so that `go build ./...` links without special flags):
// strconvOptimize is a plain variable with no effect, and the modes that
// need Go's slow path refuse to run. See slowpath.go.
const haveSlowPath = false

var strconvOptimize = true
