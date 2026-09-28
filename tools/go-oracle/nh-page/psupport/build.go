// Package psupport holds what the nh-page oracles share: building a site
// in-process with hugolib from files in a MemMapFs, running an oracle with
// recording hooks patched into neohugo packages through `go run -overlay`,
// and the typed encodings of page.TargetPathDescriptor inputs (path specs,
// output formats, paths.Path values with their PathParser recordings).
package psupport

import (
	"bytes"
	"fmt"
	"hash/fnv"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/spf13/afero"
)

// Site is a site to build: hugo.toml plus files, all relative to the site
// directory.
type Site struct {
	Name  string
	TOML  string
	Files map[string]string
}

// ModTime is the deterministic modification time given to every file of a
// built site (MemMapFs would use the wall clock): 2021-01-01 plus a hash of
// the path in seconds (under a year).
func ModTime(p string) time.Time {
	h := fnv.New32a()
	_, _ = h.Write([]byte(p))
	return time.Date(2021, 1, 1, 0, 0, 0, 0, time.UTC).Add(time.Duration(h.Sum32()%31536000) * time.Second)
}

// Built is a built site.
type Built struct {
	H   *hugolib.HugoSites
	Log *bytes.Buffer
}

// SiteDir is the working directory of a site in the MemMapFs.
func SiteDir(name string) string {
	return "/sites/" + name
}

// Build writes the site into a new MemMapFs and builds it with hugolib (the
// real build, rendering included). Warnings and errors logged by the build
// are in Log.
func Build(s Site) (*Built, error) {
	afs := afero.NewMemMapFs()
	dir := SiteDir(s.Name)
	files := map[string]string{"hugo.toml": s.TOML}
	for k, v := range s.Files {
		files[k] = v
	}
	keys := make([]string, 0, len(files))
	for k := range files {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		fn := filepath.Join(dir, filepath.FromSlash(k))
		if err := afs.MkdirAll(filepath.Dir(fn), 0o777); err != nil {
			return nil, err
		}
		if err := afero.WriteFile(afs, fn, []byte(files[k]), 0o666); err != nil {
			return nil, err
		}
		mt := ModTime(k)
		if err := afs.Chtimes(fn, mt, mt); err != nil {
			return nil, err
		}
	}

	var logBuf bytes.Buffer
	logger := loggers.New(loggers.Options{
		StdOut:        &logBuf,
		StdErr:        &logBuf,
		Level:         logg.LevelWarn,
		DistinctLevel: logg.LevelWarn,
	})

	flags := config.New()
	flags.Set("workingDir", dir)
	flags.Set("noBuildLock", true)

	res, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Flags:    flags,
		Fs:       afs,
		Filename: filepath.Join(dir, "hugo.toml"),
		Logger:   logger,
		Environ:  []string{"NEOHUGO_ORACLE=1"},
	})
	if err != nil {
		return nil, fmt.Errorf("%s: load config: %w", s.Name, err)
	}

	hfs := hugofs.NewFrom(afs, res.LoadingInfo.BaseConfig)
	h, err := hugolib.NewHugoSites(deps.DepsCfg{Configs: res, Fs: hfs, LogLevel: logger.Level(), StdErr: &logBuf, StdOut: &logBuf})
	if err != nil {
		return nil, fmt.Errorf("%s: new sites: %w", s.Name, err)
	}
	if err := h.Build(hugolib.BuildCfg{}); err != nil && !onlyDateErrors(err, logBuf.String()) {
		return nil, fmt.Errorf("%s: build: %w\n%s", s.Name, err, logBuf.String())
	}
	return &Built{H: h, Log: &logBuf}, nil
}

// onlyDateErrors reports whether a build failed only because of logged
// front matter date errors (the synthetic content has unparsable dates on
// purpose; the page is built without that date).
func onlyDateErrors(err error, log string) bool {
	if !strings.Contains(err.Error(), "logged") || !strings.Contains(err.Error(), "error(s)") {
		return false
	}
	for _, line := range strings.Split(log, "\n") {
		if strings.HasPrefix(line, "ERROR") && !strings.Contains(line, "front matter field is not a parsable date") {
			return false
		}
	}
	return true
}

// ReadTree reads every file below root/rel into a map keyed by prefix + the
// slash path relative to root/rel.
func ReadTree(root, rel, prefix string) (map[string]string, error) {
	out := map[string]string{}
	base := filepath.Join(root, rel)
	err := filepath.WalkDir(base, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			return nil
		}
		r, err := filepath.Rel(base, p)
		if err != nil {
			return err
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		out[prefix+filepath.ToSlash(r)] = string(b)
		return nil
	})
	return out, err
}

// ShortcodeNames returns the names of the shortcodes used in the content
// files (`{{< name` and `{{% name`; escaped `{{</* */>}}` ones are skipped),
// each with whether its template must use .Inner: Hugo requires every use of
// such a shortcode to be closed or self-closed, and every use of the others
// to be neither. A name is an .Inner one unless a file opens it (without a
// self-closing slash) more often than it closes it.
func ShortcodeNames(files map[string]string) map[string]bool {
	inner := map[string]bool{}
	for k, v := range files {
		if !strings.HasSuffix(k, ".md") {
			continue
		}
		opens := map[string]int{}
		closes := map[string]int{}
		for _, open := range []string{"{{<", "{{%"} {
			end := ">}}"
			if open == "{{%" {
				end = "%}}"
			}
			rest := v
			for {
				i := strings.Index(rest, open)
				if i < 0 {
					break
				}
				rest = rest[i+3:]
				s := strings.TrimLeft(rest, " \t\n")
				if strings.HasPrefix(s, "/*") {
					continue
				}
				closing := strings.HasPrefix(s, "/")
				s = strings.TrimLeft(strings.TrimPrefix(s, "/"), " \t\n")
				n := 0
				for n < len(s) && (s[n] == '-' || s[n] == '_' || s[n] == '/' || s[n] == '.' ||
					(s[n] >= 'a' && s[n] <= 'z') || (s[n] >= 'A' && s[n] <= 'Z') || (s[n] >= '0' && s[n] <= '9')) {
					n++
				}
				if n == 0 {
					continue
				}
				name := s[:n]
				if _, ok := inner[name]; !ok {
					inner[name] = true
				}
				j := strings.Index(s, end)
				selfClosed := j > 0 && strings.HasSuffix(strings.TrimRight(s[:j], " \t\n"), "/")
				switch {
				case closing:
					closes[name]++
				case !selfClosed:
					opens[name]++
				}
			}
		}
		for name, n := range opens {
			if n > closes[name] {
				inner[name] = false
			}
		}
	}
	return inner
}

// Layouts are the minimal templates the oracle sites are built with: every
// list paginates (so paginator descriptors are created), every page renders
// its content.
func Layouts() map[string]string {
	return map[string]string{
		"layouts/list.html":   `{{ .Title }}|{{ if .IsNode }}{{ range .Paginator.Pages }}{{ .RelPermalink }}|{{ .Summary }}{{ end }}{{ else }}{{ .Content }}{{ end }}{{ range .AlternativeOutputFormats }}{{ .RelPermalink }}{{ end }}`,
		"layouts/single.html": `{{ .Title }}|{{ .Content }}|{{ .Summary }}|{{ range .OutputFormats }}{{ .Permalink }}{{ end }}`,
		"layouts/404.html":    `404|{{ range .Paginator.Pages }}{{ .RelPermalink }}{{ end }}`,
		"layouts/list.json":   `{{ range .Pages }}{{ .RelPermalink }}{{ end }}`,
		"layouts/single.json": `{{ .RelPermalink }}`,
	}
}

// ShortcodeStubs returns a template for every shortcode name (the content's
// shortcodes only need to render): shortcodes used with a closing tag render
// their .Inner (Hugo then requires every use to be closed), the others
// nothing.
func ShortcodeStubs(names map[string]bool) map[string]string {
	out := map[string]string{}
	for n, closed := range names {
		t := ``
		if closed {
			t = `{{ with .Inner }}{{ . }}{{ end }}`
		}
		out["layouts/_shortcodes/"+n+".html"] = t
	}
	return out
}
