// Command walk is the Go oracle for the virtual filesystems of crates/nh-hugofs
// (Wave B task T05): hugofs (RootMappingFs, componentFs, the decorators, the
// dirs mergers, Walkway, Glob), bep/overlayfs and hugolib/filesystems.BaseFs.
//
//	go run ./tools/go-oracle/nh-hugofs/walk [-root .] [-out crates/nh-hugofs/tests/fixtures/walk]
//
// Each site is a directory tree recreated in a temporary directory: this
// repository's docs site and hugolib/testsite (file names only, contents empty
// except the configuration) and synthetic trees (multilingual content dirs,
// overlapping mounts with include/exclude globs, a theme that shadows the
// project, a node_modules vendor mount, static files from several mounts,
// multihost static, Unicode NFC/NFD and Thai names, hidden files, symlinks,
// empty dirs). The configuration is loaded with allconfig.LoadConfig; the
// recorded module mounts, config values and PathParser answers are the inputs
// the Rust test rebuilds BaseFs from. Results have the root replaced by
// "$ROOT".
//
// Directory order: the component views sort their entries (componentFs), but
// the static filesystem, RootMappingFs.ReadDir and the source filesystem
// return the OS directory order, which depends on the filesystem the tree was
// written to. Those results are recorded sorted (by path, then filename) and
// marked "sorted".
package main

import (
	"errors"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/common/types"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugofs/files"
	"github.com/neohugo/neohugo/hugolib/filesystems"
	hpaths "github.com/neohugo/neohugo/hugolib/paths"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/spf13/afero"
)

const placeholder = "$ROOT"

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-hugofs/tests/fixtures/walk", "output directory")
	flag.Parse()

	var sites []site
	docs, err := docsSite(*root)
	if err != nil {
		log.Fatal(err)
	}
	ts, err := testSite(*root)
	if err != nil {
		log.Fatal(err)
	}
	sites = append(sites, docs, ts, multilangSite(), mountsSite(), multihostSite(), emptySite())

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	for _, s := range sites {
		n, err := runSite(s, *out)
		if err != nil {
			log.Fatalf("%s: %v", s.name, err)
		}
		fmt.Printf("%s: %d cases\n", s.name, n)
	}
}

// recCfg records the PathParser callbacks basefs makes.
type recCfg struct {
	config.AllProvider
	pp *paths.PathParser
}

func (c recCfg) PathParser() *paths.PathParser { return c.pp }

type dumper struct {
	root  string
	cases []map[string]any
}

func (d *dumper) rel(s string) string {
	return strings.ReplaceAll(s, d.root, placeholder)
}

func (d *dumper) errs(err error) string {
	return d.rel(err.Error())
}

func (d *dumper) add(c map[string]any) {
	d.cases = append(d.cases, c)
}

// fi encodes a FileMetaInfo: Name, IsDir, Filename, has PathInfo,
// PathInfo.Path(), Lang, LangIndex, Weight, ModuleOrdinal, Component, Module,
// IsProject, Watch, BaseDir, SourceRoot, Meta().Name, Type().
func (d *dumper) fi(fi hugofs.FileMetaInfo) []any {
	m := fi.Meta()
	pi := ""
	if m.PathInfo != nil {
		pi = m.PathInfo.Path()
	}
	return []any{
		fi.Name(), fi.IsDir(), d.rel(m.Filename), m.PathInfo != nil, pi, m.Lang, m.LangIndex,
		m.Weight, m.ModuleOrdinal, m.Component, m.Module, m.IsProject, m.Watch,
		d.rel(m.BaseDir), d.rel(m.SourceRoot), m.Name, uint32(fi.Type()),
	}
}

func sortRows(rows [][]any) {
	sort.SliceStable(rows, func(i, j int) bool {
		a, b := fmt.Sprint(rows[i]...), fmt.Sprint(rows[j]...)
		return a < b
	})
}

// walk dumps a Walkway walk of fs from root "".
func (d *dumper) walk(id string, fs afero.Fs, sorted bool) []string {
	var rows [][]any
	var paths []string
	w := hugofs.NewWalkway(hugofs.WalkwayConfig{
		Fs: fs,
		WalkFn: func(path string, fi hugofs.FileMetaInfo) error {
			rows = append(rows, append([]any{d.rel(path)}, d.fi(fi)...))
			paths = append(paths, path)
			return nil
		},
	})
	err := w.Walk()
	if sorted {
		sortRows(rows)
	}
	c := map[string]any{"op": "walk", "fs": id, "sorted": sorted, "entries": rows}
	if err != nil {
		c["err"] = d.errs(err)
	}
	d.add(c)
	return paths
}

func (d *dumper) stat(id string, fs afero.Fs, name string) {
	c := map[string]any{"op": "stat", "fs": id, "name": d.rel(name)}
	fi, err := fs.Stat(name)
	if err != nil {
		c["err"] = d.errs(err)
	} else {
		c["fi"] = d.fi(fi.(hugofs.FileMetaInfo))
	}
	d.add(c)
}

func (d *dumper) readDir(id string, fs afero.Fs, name string, sorted bool) {
	c := map[string]any{"op": "readdir", "fs": id, "name": d.rel(name), "sorted": sorted}
	f, err := fs.Open(name)
	if err != nil {
		c["err"] = d.errs(err)
		d.add(c)
		return
	}
	defer func() { _ = f.Close() }()
	rdf, ok := f.(interface {
		ReadDir(int) ([]os.DirEntry, error)
	})
	if !ok {
		c["err"] = "not a ReadDirFile"
		d.add(c)
		return
	}
	fis, err := rdf.ReadDir(-1)
	if err != nil {
		c["err"] = d.errs(err)
		d.add(c)
		return
	}
	var rows [][]any
	for _, fi := range fis {
		rows = append(rows, d.fi(fi.(hugofs.FileMetaInfo)))
	}
	if sorted {
		sortRows(rows)
	}
	c["entries"] = rows
	d.add(c)
}

var extraNames = []string{
	"missing", "missing.md", "a/b/c/missing.txt", "../outside", "/", ".", "//", "./", "/missing",
}

func componentIDs(b *filesystems.BaseFs) []struct {
	id     string
	fs     *filesystems.SourceFilesystem
	sorted bool
} {
	r := []struct {
		id     string
		fs     *filesystems.SourceFilesystem
		sorted bool
	}{
		{"content", b.Content, false},
		{"data", b.Data, false},
		{"i18n", b.I18n, false},
		{"layouts", b.Layouts, false},
		{"archetypes", b.Archetypes, false},
		{"assets", b.Assets, false},
		{"assetsDup", b.AssetsWithDuplicatesPreserved, false},
	}
	var keys []string
	for k := range b.Static {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		r = append(r, struct {
			id     string
			fs     *filesystems.SourceFilesystem
			sorted bool
		}{"static:" + k, b.Static[k], true})
	}
	return r
}

func runSite(s site, out string) (int, error) {
	tmp, err := os.MkdirTemp("", "nhfs-walk-")
	if err != nil {
		return 0, err
	}
	defer func() { _ = os.RemoveAll(tmp) }()
	root := filepath.Join(tmp, "site")
	if err := materialize(root, s.tree); err != nil {
		return 0, err
	}

	flags := config.New()
	flags.Set("workingDir", root)
	for k, v := range s.flags {
		flags.Set(k, v)
	}
	osfs := afero.NewOsFs()
	confs, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Fs:       osfs,
		Filename: filepath.Join(root, "hugo.toml"),
		Flags:    flags,
		Environ:  []string{"NEOHUGO_ORACLE=1"},
		Logger:   loggers.NewDefault(),
	})
	if err != nil {
		return 0, err
	}
	conf := confs.GetFirstLanguageConfig()
	rec := newRecorder()
	pp := rec.wrap(conf.PathParser())
	cfg := recCfg{AllProvider: conf, pp: pp}

	fs := hugofs.NewFrom(osfs, conf.BaseConfig())
	p, err := hpaths.New(fs, cfg)
	if err != nil {
		return 0, err
	}
	b, err := filesystems.NewBase(p, nil)
	if err != nil {
		return 0, err
	}

	d := &dumper{root: root}

	// Walks and component views.
	for _, cv := range componentIDs(b) {
		paths := d.walk(cv.id, cv.fs.Fs, cv.sorted)
		names := append(append([]string{}, paths...), extraNames...)
		for _, n := range names {
			d.stat(cv.id, cv.fs.Fs, n)
		}
		fileDone := false
		for _, n := range names {
			fi, err := cv.fs.Fs.Stat(n)
			if err == nil && !fi.IsDir() {
				// ReadDir of one file per view is enough (Go's error text).
				if fileDone {
					continue
				}
				fileDone = true
			}
			d.readDir(cv.id, cv.fs.Fs, n, cv.sorted)
		}
	}

	// The root mapping filesystems.
	for i, rfs := range b.RootFss {
		id := fmt.Sprintf("rfs%d", i)
		for _, comp := range files.ComponentFolders {
			c := map[string]any{"op": "mounts", "fs": id, "name": comp}
			fis, err := rfs.Mounts(comp)
			if err != nil {
				c["err"] = d.errs(err)
			} else {
				var rows [][]any
				for _, fi := range fis {
					rows = append(rows, d.fi(fi))
				}
				c["entries"] = rows
			}
			d.add(c)
		}
		names := []string{""}
		names = append(names, files.ComponentFolders...)
		for _, comp := range files.ComponentFolders {
			names = append(names, comp+"/missing", comp+"/missing/deeper.md")
			f, err := rfs.Open(comp)
			if err != nil {
				continue
			}
			rdf := f.(interface {
				ReadDir(int) ([]os.DirEntry, error)
			})
			fis, err := rdf.ReadDir(-1)
			_ = f.Close()
			if err != nil {
				continue
			}
			for _, fi := range fis {
				names = append(names, filepath.Join(comp, fi.Name()))
			}
		}
		names = append(names, extraNames...)
		seen := map[string]bool{}
		for _, n := range names {
			if seen[n] {
				continue
			}
			seen[n] = true
			d.stat(id, rfs, n)
			d.readDir(id, rfs, n, true)
		}
	}

	// Files of the tree (absolute filenames), plus some that are not.
	var filenames []string
	for _, n := range s.tree {
		filenames = append(filenames, filepath.Join(root, filepath.FromSlash(n.P)))
	}
	sort.Strings(filenames)
	filenames = append(filenames, root, filepath.Join(root, "missing.txt"), "/elsewhere/file.txt",
		filepath.Join(root, "node_modules"), filepath.Join(root, "node_modules", "missing.js"), "")

	for _, cv := range componentIDs(b) {
		for _, fn := range filenames {
			for _, check := range []bool{true, false} {
				rel, ok := cv.fs.MakePathRelative(fn, check)
				d.add(map[string]any{"op": "makePathRelative", "fs": cv.id, "name": d.rel(fn), "check": check, "rel": rel, "ok": ok})
			}
			d.add(map[string]any{"op": "contains", "fs": cv.id, "name": d.rel(fn), "res": cv.fs.Contains(fn)})
		}
		for _, from := range []string{"", "scss", "css", "vendor", "vendor/bootstrap", "missing", "_default", "posts", "sub"} {
			var rd []string
			for _, x := range cv.fs.RealDirs(from) {
				rd = append(rd, d.rel(x))
			}
			d.add(map[string]any{"op": "realDirs", "fs": cv.id, "name": from, "res": rd})
		}
	}
	for _, fn := range filenames {
		var cps [][]any
		for _, cp := range b.ResolvePaths(fn) {
			cps = append(cps, []any{cp.Component, cp.Path, cp.Lang, cp.Watch})
		}
		d.add(map[string]any{"op": "resolvePaths", "name": d.rel(fn), "res": cps})
		d.add(map[string]any{"op": "isContent", "name": d.rel(fn), "res": b.IsContent(fn)})
		d.add(map[string]any{"op": "isStatic", "name": d.rel(fn), "res": b.IsStatic(fn)})
		d.add(map[string]any{"op": "makeStaticPathRelative", "name": d.rel(fn), "res": b.MakeStaticPathRelative(fn)})
	}
	for _, name := range []string{"package.json", "postcss.config.js", "babel.config.js", "hugo.toml", "missing.js", "package.hugo.json"} {
		d.add(map[string]any{"op": "resolveJSConfigFile", "name": name, "res": d.rel(b.ResolveJSConfigFile(name))})
	}
	for _, name := range []string{"posts/x.md", "content/posts/x.md", "content/en/x.md", filepath.Join(root, "content", "a.md"), "/elsewhere/a.md", "x.md", filepath.Join(root, "content2", "b.md")} {
		c := map[string]any{"op": "absProjectContentDir", "name": d.rel(name)}
		rel, abs, err := b.AbsProjectContentDir(name)
		if err != nil {
			c["err"] = d.errs(err)
		} else {
			c["res"] = []string{d.rel(rel), d.rel(abs)}
		}
		d.add(c)
	}
	{
		var wf []string
		for _, x := range b.WatchFilenames() {
			wf = append(wf, d.rel(x))
		}
		sort.Strings(wf)
		d.add(map[string]any{"op": "watchFilenames", "res": wf})
	}
	for _, lang := range []string{"", "en", "th", "nn"} {
		for _, name := range []string{"a.txt", "css/main.css", "missing", "_index.md", "robots.bak", "img/a.png", "th.txt", "common.txt"} {
			c := map[string]any{"op": "statResource", "lang": lang, "name": name}
			fi, fs, err := b.StatResource(lang, name)
			which := ""
			switch fs {
			case b.Assets.Fs:
				which = "assets"
			case b.Content.Fs:
				which = "content"
			default:
				which = "static"
			}
			c["which"] = which
			if err != nil {
				c["err"] = d.errs(err)
			} else {
				c["fi"] = d.fi(fi.(hugofs.FileMetaInfo))
			}
			d.add(c)
		}
	}
	for _, name := range []string{"hugo.toml", "missing", "content", "themes/mytheme/hugo.toml", "resources"} {
		d.stat("work", b.Work, name)
	}
	for _, name := range []string{filepath.Join(root, "missing"), filepath.Join(root, "hugo.toml"), root} {
		d.stat("source", b.SourceFs, name)
	}
	for _, g := range []struct{ id, pattern string }{
		{"assets", "**.css"}, {"assets", "css/*.css"}, {"assets", "**/*.{png,jpg,svg}"}, {"assets", "images/**"},
		{"assets", "vendor/**.js"}, {"assets", "**"}, {"assets", "*"}, {"assets", "/js/*.js"}, {"assets", "CSS/**"},
		{"content", "**/*.md"}, {"content", "posts/*"}, {"content", "**.th.md"}, {"layouts", "_default/*"},
		{"layouts", "**.html"}, {"data", "**"}, {"assets", "missing/**"}, {"assets", ""},
	} {
		var fs afero.Fs
		for _, cv := range componentIDs(b) {
			if cv.id == g.id {
				fs = cv.fs.Fs
			}
		}
		var matched []string
		c := map[string]any{"op": "glob", "fs": g.id, "name": g.pattern}
		err := hugofs.Glob(fs, g.pattern, func(fi hugofs.FileMetaInfo) (bool, error) {
			matched = append(matched, d.rel(fi.Meta().Filename))
			return false, nil
		})
		if err != nil {
			c["err"] = d.errs(err)
		}
		c["res"] = matched
		d.add(c)
	}
	{
		// Glob with an early stop.
		var matched []string
		err := hugofs.Glob(b.Content.Fs, "**", func(fi hugofs.FileMetaInfo) (bool, error) {
			matched = append(matched, d.rel(fi.Meta().Filename))
			return len(matched) == 2, nil
		})
		c := map[string]any{"op": "globStop", "fs": "content", "name": "**", "res": matched}
		if err != nil {
			c["err"] = d.errs(err)
		}
		d.add(c)
	}
	{
		// A walk with FailOnNotExist on a missing root, and a handle error.
		err := hugofs.Glob(b.Content.Fs, "nope/**", func(fi hugofs.FileMetaInfo) (bool, error) { return false, nil })
		c := map[string]any{"op": "globMissing", "fs": "content", "name": "nope/**"}
		if err != nil {
			c["err"] = d.errs(err)
		}
		d.add(c)
		err = hugofs.Glob(b.Content.Fs, "**", func(fi hugofs.FileMetaInfo) (bool, error) { return false, errors.New("handle failed") })
		c = map[string]any{"op": "globErr", "fs": "content", "name": "**"}
		if err != nil {
			c["err"] = d.errs(err)
		}
		d.add(c)
	}

	// The inputs the Rust test rebuilds BaseFs from.
	var mods []any
	for _, m := range confs.Modules {
		var mounts []any
		for _, mnt := range m.Mounts() {
			mounts = append(mounts, map[string]any{
				"source":       d.rel(mnt.Source),
				"target":       mnt.Target,
				"lang":         mnt.Lang,
				"includeFiles": types.ToStringSlicePreserveString(mnt.IncludeFiles),
				"excludeFiles": types.ToStringSlicePreserveString(mnt.ExcludeFiles),
				"disableWatch": mnt.DisableWatch,
			})
		}
		owner := ""
		if m.Owner() != nil {
			owner = m.Owner().Path()
		}
		mods = append(mods, map[string]any{
			"path":   m.Path(),
			"dir":    d.rel(m.Dir()),
			"owner":  owner,
			"watch":  m.Watch(),
			"mounts": mounts,
		})
	}
	var langs []string
	for _, l := range conf.Languages() {
		langs = append(langs, l.Lang)
	}
	header := map[string]any{
		"site":    s.name,
		"tree":    s.tree,
		"modules": mods,
		"config": map[string]any{
			"publishDir":             conf.BaseConfig().PublishDir,
			"resourceDir":            conf.Dirs().ResourceDir,
			"defaultContentLanguage": conf.DefaultContentLanguage(),
			"isMultihost":            conf.IsMultihost(),
			"noBuildLock":            conf.NoBuildLock(),
			"languages":              langs,
			"absPublishDir":          d.rel(p.AbsPublishDir),
			"absResourcesDir":        d.rel(p.AbsResourcesDir),
		},
		"parser": rec.describe(pp),
	}
	if err := goval.WriteCasesGz(filepath.Join(out, s.name+".json.gz"), header, d.cases); err != nil {
		return 0, err
	}
	return len(d.cases), nil
}
