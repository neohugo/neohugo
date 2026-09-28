package tsupport

import (
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

// The layout trees of tplimpl's own integration tests
// (tpl/tplimpl/*_integration_test.go): every backquoted txtar archive of a
// test function that holds files under layouts/ becomes a site. They are
// read from the test sources at run time (the files are not importable),
// so the oracles follow the tests. The shortcode tests resolve templates
// without executing them (their embedded shortcodes fetch remote data); the
// others are rendered. An archive that does not build is skipped (the tests
// patch some archives before building them).
var integrationTestFiles = []string{
	"templatestore_integration_test.go",
	"render_hook_integration_test.go",
	"legacy_integration_test.go",
	"tplimpl_integration_test.go",
	"shortcodes_integration_test.go",
}

var configNames = []string{"hugo.toml", "config.toml", "hugo.yaml", "config.yaml", "hugo.json", "config.json"}

// parseTxtar splits a txtar archive ("-- name --" lines).
func parseTxtar(s string) map[string]string {
	files := map[string]string{}
	var name string
	var b strings.Builder
	flush := func() {
		if name != "" {
			files[name] = b.String()
		}
		b.Reset()
	}
	for _, line := range strings.SplitAfter(s, "\n") {
		t := strings.TrimRight(line, "\n")
		if strings.HasPrefix(t, "-- ") && strings.HasSuffix(t, " --") && len(t) > 6 {
			flush()
			name = strings.TrimSpace(t[3 : len(t)-3])
			continue
		}
		if name != "" {
			b.WriteString(line)
		}
	}
	flush()
	return files
}

// IntegrationSites returns the sites of the integration test archives.
func IntegrationSites(root string) ([]Site, error) {
	var sites []Site
	for _, fn := range integrationTestFiles {
		fset := token.NewFileSet()
		f, err := parser.ParseFile(fset, filepath.Join(root, "tpl", "tplimpl", fn), nil, 0)
		if err != nil {
			return nil, err
		}
		short := strings.TrimSuffix(strings.TrimSuffix(fn, "_test.go"), "_integration")
		for _, decl := range f.Decls {
			fd, ok := decl.(*ast.FuncDecl)
			if !ok || !strings.HasPrefix(fd.Name.Name, "Test") {
				continue
			}
			n := 0
			ast.Inspect(fd, func(node ast.Node) bool {
				lit, ok := node.(*ast.BasicLit)
				if !ok || lit.Kind != token.STRING || !strings.HasPrefix(lit.Value, "`") {
					return true
				}
				s, err := strconv.Unquote(lit.Value)
				if err != nil {
					return true
				}
				files := parseTxtar(s)
				hasLayouts := false
				for k := range files {
					if strings.HasPrefix(k, "layouts/") || strings.HasPrefix(k, "themes/") {
						hasLayouts = true
					}
				}
				if !hasLayouts {
					return true
				}
				n++
				site := Site{
					Name:      fmt.Sprintf("it_%s_%s_%d", short, strings.TrimPrefix(fd.Name.Name, "Test"), n),
					Files:     files,
					Render:    short != "shortcodes",
					SmallGrid: true,
				}
				for _, c := range configNames {
					if _, ok := files[c]; ok {
						site.ConfigName = c
						break
					}
				}
				sites = append(sites, site)
				return true
			})
		}
	}
	sort.Slice(sites, func(i, j int) bool { return sites[i].Name < sites[j].Name })
	return sites, nil
}
