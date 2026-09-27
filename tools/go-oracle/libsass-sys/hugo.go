package main

// Emulation of the Hugo import resolver (neohugo
// resources/resource_transformers/tocss/scss/tocss.go) over the seeksnack
// mounts: assets -> <root>/assets, assets/vendor -> <root>/node_modules.
// crates/libsass-sys/tests/common/mod.rs implements the same logic in Rust.

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

type hugoFs struct {
	root string
}

func (h hugoFs) assets() string { return filepath.Join(h.root, "assets") }
func (h hugoFs) vendor() string { return filepath.Join(h.root, "node_modules") }

func exists(p string) bool {
	_, err := os.Stat(p)
	return err == nil
}

// stat is assetsFs.Stat: returns the real filename or "".
func (h hugoFs) stat(rel string) string {
	rel = filepath.Clean(rel)
	if rel == "vendor" || strings.HasPrefix(rel, "vendor/") {
		r := h.vendor() + strings.TrimPrefix(rel, "vendor")
		if exists(r) {
			return r
		}
	}
	r := filepath.Join(h.assets(), rel)
	if exists(r) {
		return r
	}
	return ""
}

// makePathRelative is SourceFilesystem.MakePathRelative(filename, true).
func (h hugoFs) makePathRelative(filename string) string {
	f := filepath.Clean(filename)
	for _, m := range []struct{ real, target string }{{h.assets(), ""}, {h.vendor(), "vendor"}} {
		if f == m.real || strings.HasPrefix(f, m.real+"/") {
			p := strings.TrimPrefix(filepath.Join("/", m.target, strings.TrimPrefix(f, m.real)), "/")
			if p == "" {
				// The component root: Stat("") succeeds, Path is "".
				return ""
			}
			if h.stat(p) != "" {
				return p
			}
			return ""
		}
	}
	return ""
}

// hugoResolver mirrors the ImportResolver closure in tocss.go.
func hugoResolver(h hugoFs, baseDir, varsStylesheet string, trace *[]string) func(url, prev string) (string, string, bool) {
	return func(url string, prev string) (newUrl string, body string, resolved bool) {
		*trace = append(*trace, url+"\t"+prev)
		if url == "hugo:vars" {
			return url, varsStylesheet, true
		}

		// We get URL paths from LibSASS, but we need file paths.
		url = filepath.FromSlash(url)
		prev = filepath.FromSlash(prev)

		var basePath string
		urlDir := filepath.Dir(url)
		var prevDir string

		if prev == "stdin" {
			prevDir = baseDir
		} else {
			prevDir = h.makePathRelative(filepath.Dir(prev))

			if prevDir == "" {
				// Not a member of this filesystem. Let LibSASS handle it.
				return "", "", false
			}
		}

		basePath = filepath.Join(prevDir, urlDir)
		name := filepath.Base(url)

		// Libsass throws an error in cases where you have several possible candidates.
		// We make this simpler and pick the first match.
		var namePatterns []string
		if strings.Contains(name, ".") {
			namePatterns = []string{"_%s", "%s"}
		} else if strings.HasPrefix(name, "_") {
			namePatterns = []string{"_%s.scss", "_%s.sass"}
		} else {
			namePatterns = []string{
				"_%s.scss", "%s.scss",
				"_%s.sass", "%s.sass",
				"%s/_index.scss", "%s/_index.sass",
				"%s/index.scss", "%s/index.sass",
			}
		}

		name = strings.TrimPrefix(name, "_")

		for _, namePattern := range namePatterns {
			filenameToCheck := filepath.Join(basePath, fmt.Sprintf(namePattern, name))
			if real := h.stat(filenameToCheck); real != "" {
				return real, "", true
			}
		}

		// Not found, let LibSASS handle it
		return "", "", false
	}
}
