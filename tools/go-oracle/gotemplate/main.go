//go:build gotemplate_oracle

// Command gotemplate is the Go oracle for crates/gotemplate: it runs the
// forked text/template + html/template (copied into ./fork by
// sync-fork.sh) and writes fixtures for the Rust differential tests.
//
//	tools/go-oracle/gotemplate/sync-fork.sh
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate <mode> [args]
//
// Each mode lives in its own file and registers itself in init().
package main

import (
	"fmt"
	"os"
	"sort"
)

var modes = map[string]func(args []string) error{}

func register(name string, f func(args []string) error) {
	if _, dup := modes[name]; dup {
		panic("duplicate mode " + name)
	}
	modes[name] = f
}

func main() {
	if len(os.Args) < 2 || modes[os.Args[1]] == nil {
		var names []string
		for n := range modes {
			names = append(names, n)
		}
		sort.Strings(names)
		fmt.Fprintf(os.Stderr, "usage: gotemplate <mode> [args]; modes: %v\n", names)
		os.Exit(2)
	}
	if err := modes[os.Args[1]](os.Args[2:]); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
