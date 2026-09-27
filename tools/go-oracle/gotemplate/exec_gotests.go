//go:build gotemplate_oracle

// Mode "exectests": runs the tables of Go's texttemplate/exec_test.go
// (execTests, cmpTests and the table-driven Test* functions; copied into
// ./exectests) through the forked engine and dumps output and errors. The
// fork replaces isTrue by hreflect.IsTruthfulValue, so its results are the
// expectations; Go's own expectations are recorded next to them.
//
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate exectests crates/gotemplate/tests/fixtures/text
//
// Output: <outdir>/exectests.txt (read by crates/gotemplate/tests/exec_go.rs).
package main

import (
	"fmt"
	"os"
	"path/filepath"

	gotests "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/exectests"
)

func init() {
	register("exectests", etgMain)
}

func etgMain(args []string) error {
	if len(args) != 1 {
		return fmt.Errorf("usage: exectests <outdir>")
	}
	f, err := os.Create(filepath.Join(args[0], "exectests.txt"))
	if err != nil {
		return err
	}
	defer func() { _ = f.Close() }()
	return gotests.Dump(f)
}
