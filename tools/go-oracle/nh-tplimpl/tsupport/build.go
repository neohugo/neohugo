package tsupport

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"hash/fnv"
	"os"
	"path/filepath"
	"sort"
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
	Name string
	TOML string
	// Files relative to the site directory.
	Files map[string]string
	// Render executes the templates; otherwise the build resolves every
	// page's template without executing it (renderAndWritePage is off).
	Render bool
	// ConfigName is the configuration file among Files (TOML is then
	// ignored); empty: TOML is written to hugo.toml.
	ConfigName string
	// SmallGrid limits the lookup oracle's grid queries.
	SmallGrid bool
}

// SiteDir is the working directory of a site in the MemMapFs.
func SiteDir(name string) string {
	return "/sites/" + name
}

// modTime is the deterministic modification time of every file of a site
// (MemMapFs would use the wall clock).
func modTime(p string) time.Time {
	h := fnv.New32a()
	_, _ = h.Write([]byte(p))
	return time.Date(2021, 1, 1, 0, 0, 0, 0, time.UTC).Add(time.Duration(h.Sum32()%31536000) * time.Second)
}

// Built is a site whose HugoSites is created (the template store exists).
type Built struct {
	Site Site
	H    *hugolib.HugoSites
	Log  *bytes.Buffer
}

// New writes the site into a new MemMapFs and creates its HugoSites (config,
// filesystems, template store); nothing is built yet.
func New(s Site) (*Built, error) {
	afs := afero.NewMemMapFs()
	dir := SiteDir(s.Name)
	files := map[string]string{}
	configName := s.ConfigName
	if configName == "" {
		configName = "hugo.toml"
		files[configName] = s.TOML
	}
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
		mt := modTime(k)
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
		Filename: filepath.Join(dir, configName),
		Logger:   logger,
		Environ:  []string{"NEOHUGO_ORACLE=1"},
	})
	if err != nil {
		return nil, fmt.Errorf("%s: load config: %w", s.Name, err)
	}

	hfs := hugofs.NewFrom(afs, res.LoadingInfo.BaseConfig)
	h, err := hugolib.NewHugoSites(deps.DepsCfg{Configs: res, Fs: hfs, LogLevel: logger.Level(), StdErr: &logBuf, StdOut: &logBuf})
	if err != nil {
		return nil, fmt.Errorf("%s: new sites: %w\n%s", s.Name, err, logBuf.String())
	}
	return &Built{Site: s, H: h, Log: &logBuf}, nil
}

// Build builds the site (rendering or resolving templates only, see
// Site.Render).
func (b *Built) Build() error {
	Oracle.SetSkipWrite(!b.Site.Render)
	defer Oracle.SetSkipWrite(false)
	if err := b.H.Build(hugolib.BuildCfg{}); err != nil {
		return fmt.Errorf("%s: build: %w\n%s", b.Site.Name, err, b.Log.String())
	}
	return nil
}

// Modules encodes the modules the layouts filesystem is built from: each
// module's directory (relative to the site) and its layouts mounts.
func (b *Built) Modules() []any {
	dir := SiteDir(b.Site.Name)
	var out []any
	for i, m := range b.H.Configs.Modules {
		var mounts []any
		for _, mnt := range m.Mounts() {
			mounts = append(mounts, map[string]any{
				"source": mnt.Source,
				"target": mnt.Target,
				"lang":   mnt.Lang,
			})
		}
		rel, err := filepath.Rel(dir, m.Dir())
		if err != nil {
			rel = m.Dir()
		}
		out = append(out, map[string]any{
			"ordinal":   i,
			"path":      m.Path(),
			"dir":       m.Dir(),
			"rel":       filepath.ToSlash(rel),
			"isProject": m.Owner() == nil,
			"watch":     m.Watch(),
			"mounts":    mounts,
		})
	}
	return out
}

func mustJSON(v any) string {
	b, err := json.Marshal(v)
	if err != nil {
		panic(err)
	}
	return string(b)
}

// WriteFixture writes v as gzip-compressed JSON (sorted map keys, no
// timestamps in the gzip header, so it regenerates byte for byte).
func WriteFixture(dir, name string, v any) error {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return err
	}
	b, err := json.Marshal(v)
	if err != nil {
		return err
	}
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
	return os.WriteFile(filepath.Join(dir, name), buf.Bytes(), 0o644)
}
