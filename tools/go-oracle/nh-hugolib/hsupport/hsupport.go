// Package hsupport holds what the nh-hugolib oracles share: running an oracle
// again with `go run -overlay` (files added to or patched into neohugo
// packages, e.g. an exported dump function in package hugolib), writing a
// site into a temporary directory, loading its config and creating the
// HugoSites like a build does, and the path normalisation of the dumps.
package hsupport

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"hash/fnv"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib"
)

// ChildEnv is set in the environment of the overlaid child process.
const ChildEnv = "NH_HUGOLIB_ORACLE_CHILD"

// IsChild reports whether this process is the overlaid child.
func IsChild() bool {
	return os.Getenv(ChildEnv) == "1"
}

// Patch inserts code into a neohugo source file: After must occur exactly
// once in the file; Insert is placed right after it.
type Patch struct {
	File   string // relative to the module root
	After  string
	Insert string
}

// RunOverlaid runs the oracle package pkg (e.g.
// "./tools/go-oracle/nh-hugolib/capture") again with `go run -overlay`.
// AddFiles maps module-relative paths of new files (in neohugo packages or
// in the oracle package itself) to their content; patches are applied to
// copies of existing files. The repository is never modified.
func RunOverlaid(root, pkg string, addFiles map[string]string, patches []Patch, args []string) error {
	absRoot, err := filepath.Abs(root)
	if err != nil {
		return err
	}
	tmp, err := os.MkdirTemp("", "nh-hugolib-oracle-overlay")
	if err != nil {
		return err
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	replace := map[string]string{}
	n := 0
	write := func(rel, content string) error {
		dst := filepath.Join(tmp, fmt.Sprintf("f%d.go", n))
		n++
		if err := os.WriteFile(dst, []byte(content), 0o644); err != nil {
			return err
		}
		replace[filepath.Join(absRoot, filepath.FromSlash(rel))] = dst
		return nil
	}
	keys := make([]string, 0, len(addFiles))
	for k := range addFiles {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		if _, err := os.Stat(filepath.Join(absRoot, filepath.FromSlash(k))); err == nil {
			return fmt.Errorf("overlay: %s exists", k)
		}
		if err := write(k, addFiles[k]); err != nil {
			return err
		}
	}
	content := map[string]string{}
	var order []string
	for _, p := range patches {
		s, ok := content[p.File]
		if !ok {
			b, err := os.ReadFile(filepath.Join(absRoot, filepath.FromSlash(p.File)))
			if err != nil {
				return err
			}
			s = string(b)
			order = append(order, p.File)
		}
		if c := strings.Count(s, p.After); c != 1 {
			return fmt.Errorf("overlay: %s: anchor %q found %d times", p.File, p.After, c)
		}
		content[p.File] = strings.Replace(s, p.After, p.After+p.Insert, 1)
	}
	for _, f := range order {
		if err := write(f, content[f]); err != nil {
			return err
		}
	}

	ob, err := json.Marshal(map[string]any{"Replace": replace})
	if err != nil {
		return err
	}
	overlay := filepath.Join(tmp, "overlay.json")
	if err := os.WriteFile(overlay, ob, 0o644); err != nil {
		return err
	}

	cmdArgs := append([]string{"run", "-overlay", overlay, pkg}, args...)
	cmd := exec.Command("go", cmdArgs...)
	cmd.Dir = absRoot
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Env = append(os.Environ(), ChildEnv+"=1", "HUGO_NUMWORKERMULTIPLIER=1")
	return cmd.Run()
}

// File is a file of a site: its content, or a repository file (Repo is the
// module-relative path; Content is its content and FNV the FNV-1a 64 of it,
// so a test can check that the repository still has the recorded file).
type File struct {
	Content string
	Repo    string
}

// Site is a site to build: hugo.toml plus files, all relative to the site
// directory.
type Site struct {
	Name  string
	TOML  string
	Files map[string]File
}

// FNV returns the FNV-1a 64 hash of s as 16 hex digits.
func FNV(s string) string {
	h := fnv.New64a()
	_, _ = h.Write([]byte(s))
	return fmt.Sprintf("%016x", h.Sum64())
}

// Describe returns the JSON description of a site recorded in the fixtures:
// the config and every file (inline, or as a repository reference with its
// hash).
func (s Site) Describe() map[string]any {
	keys := make([]string, 0, len(s.Files))
	for k := range s.Files {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	files := []any{}
	for _, k := range keys {
		f := s.Files[k]
		if f.Repo != "" {
			files = append(files, map[string]any{"path": k, "repo": f.Repo, "fnv": FNV(f.Content)})
		} else {
			files = append(files, map[string]any{"path": k, "content": f.Content})
		}
	}
	return map[string]any{"name": s.Name, "toml": s.TOML, "files": files}
}

// Write writes the site into dir (hugo.toml plus the files, in sorted
// order).
func (s Site) Write(dir string) error {
	if err := os.MkdirAll(dir, 0o777); err != nil {
		return err
	}
	if err := os.WriteFile(filepath.Join(dir, "hugo.toml"), []byte(s.TOML), 0o666); err != nil {
		return err
	}
	keys := make([]string, 0, len(s.Files))
	for k := range s.Files {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		fn := filepath.Join(dir, filepath.FromSlash(k))
		if err := os.MkdirAll(filepath.Dir(fn), 0o777); err != nil {
			return err
		}
		if err := os.WriteFile(fn, []byte(s.Files[k].Content), 0o666); err != nil {
			return err
		}
	}
	return nil
}

// Built is a site whose HugoSites is created (not built).
type Built struct {
	H   *hugolib.HugoSites
	Dir string
	Log *bytes.Buffer
}

// New writes the site into a new directory below tmp and creates its
// HugoSites like a build (`--environment production`, no process
// environment, the file caches below the site's cache dir). The config
// loading logs to its own buffer; the HugoSites logs to Log.
func New(s Site, tmp string) (*Built, error) {
	dir := filepath.Join(tmp, s.Name)
	if err := s.Write(dir); err != nil {
		return nil, err
	}

	var cfgLog bytes.Buffer
	cfgLogger := loggers.New(loggers.Options{StdOut: &cfgLog, StdErr: &cfgLog, Level: logg.LevelWarn})

	flags := config.New()
	flags.Set("workingDir", dir)
	flags.Set("noBuildLock", true)
	flags.Set("cacheDir", filepath.Join(tmp, "_cache"))

	res, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Flags:       flags,
		Fs:          hugofs.Os,
		Filename:    filepath.Join(dir, "hugo.toml"),
		Logger:      cfgLogger,
		Environment: "production",
		Environ:     []string{"NEOHUGO_ORACLE=1"},
	})
	if err != nil {
		return nil, fmt.Errorf("%s: load config: %w", s.Name, err)
	}

	var logBuf bytes.Buffer
	logger := loggers.New(loggers.Options{
		StdOut:        &logBuf,
		StdErr:        &logBuf,
		Level:         logg.LevelWarn,
		DistinctLevel: logg.LevelWarn,
	})

	hfs := hugofs.NewFrom(hugofs.Os, res.LoadingInfo.BaseConfig)
	h, err := hugolib.NewHugoSites(deps.DepsCfg{Configs: res, Fs: hfs, TestLogger: logger})
	if err != nil {
		return nil, fmt.Errorf("%s: new sites: %w", s.Name, err)
	}
	return &Built{H: h, Dir: dir, Log: &logBuf}, nil
}

// Norm replaces the site directory in s with "/SITE".
func (b *Built) Norm(s string) string {
	return strings.ReplaceAll(s, b.Dir, "/SITE")
}

// LogLines returns the non-empty lines of the HugoSites log, normalised.
func (b *Built) LogLines() []string {
	lines := []string{}
	for _, l := range strings.Split(b.Norm(b.Log.String()), "\n") {
		if l != "" {
			lines = append(lines, l)
		}
	}
	return lines
}

// WriteJSONGz writes v as indented JSON, gzip-compressed at best compression
// (no header name or time), to path.
func WriteJSONGz(path string, v any) error {
	b, err := json.MarshalIndent(v, "", " ")
	if err != nil {
		return err
	}
	return writeGz(path, append(b, '\n'))
}

func writeGz(path string, b []byte) error {
	var buf bytes.Buffer
	zw, err := gzip.NewWriterLevel(&buf, gzip.BestCompression)
	if err != nil {
		return err
	}
	if _, err := zw.Write(b); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o777); err != nil {
		return err
	}
	return os.WriteFile(path, buf.Bytes(), 0o644)
}
