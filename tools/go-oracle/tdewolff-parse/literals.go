package main

import (
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

// parseModuleDir returns the module cache directory of the tdewolff/parse
// version pinned in go.mod.
func parseModuleDir() string {
	out, err := exec.Command("go", "list", "-m", "-f", "{{.Dir}}", "github.com/tdewolff/parse/v2").Output()
	if err != nil {
		panic(err)
	}
	return strings.TrimSpace(string(out))
}

// testLiterals returns every string literal that appears inside a Test*
// function of the _test.go files of the given package directory (relative to
// the module root, "" for the root package). These are the inputs (and
// expected outputs) of upstream's test tables; the oracle runs the real code
// on all of them.
func testLiterals(pkg string) []string {
	dir := filepath.Join(parseModuleDir(), pkg)
	fset := token.NewFileSet()
	matches, err := filepath.Glob(filepath.Join(dir, "*_test.go"))
	if err != nil {
		panic(err)
	}
	seen := map[string]bool{}
	var lits []string
	for _, fn := range matches {
		src, err := os.ReadFile(fn)
		if err != nil {
			panic(err)
		}
		f, err := parser.ParseFile(fset, fn, src, 0)
		if err != nil {
			panic(err)
		}
		for _, decl := range f.Decls {
			fd, ok := decl.(*ast.FuncDecl)
			if !ok || !strings.HasPrefix(fd.Name.Name, "Test") {
				continue
			}
			ast.Inspect(fd, func(n ast.Node) bool {
				if bl, ok := n.(*ast.BasicLit); ok && bl.Kind == token.STRING {
					s, err := strconv.Unquote(bl.Value)
					if err == nil && !seen[s] {
						seen[s] = true
						lits = append(lits, s)
					}
				}
				return true
			})
		}
	}
	sort.Strings(lits)
	return lits
}
