// Command staticcopy is the Go oracle for the static file copy of
// crates/nh-commands (Wave B task T25): commands/hugobuilder.go copyStaticTo,
// the spf13/fsync Syncer over the static filesystem of hugolib/filesystems.
//
//	go run ./tools/go-oracle/nh-commands/staticcopy [-out crates/nh-commands/tests/fixtures/staticcopy]
//
// Every case is a site tree (files with contents, modes and modification times,
// symlinks, empty directories, a hugo.toml with mounts and the static options)
// recreated in a temporary directory. The config is loaded like the CLI
// (allconfig.LoadConfig, publishDir from the config), the static filesystems
// come from filesystems.NewBase, and each one is synced into the publish dir
// exactly as copyStaticTo does it (countingStatFs, chmodFilter, NoTimes,
// NoChmod, Delete = cleanDestinationDir with the hidden directory filter).
// The fixture records, per case, the number of files copyStaticTo reports, the
// error, and the publish dir afterwards: every entry (Lstat) with its kind,
// size, content, permission bits (when NoChmod is off) and modification time
// (when NoTimes is off).
package main

import (
	"bytes"
	"encoding/hex"
	"flag"
	"fmt"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"sync/atomic"
	"time"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	cpaths "github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib/filesystems"
	"github.com/neohugo/neohugo/hugolib/paths"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/spf13/afero"
	"github.com/spf13/fsync"
)

// An entry of a site tree.
type entry struct {
	Path string `json:"path"`
	// Kind: "file", "dir" or "symlink".
	Kind    string `json:"kind"`
	Content string `json:"content,omitempty"`
	// Hex content (binary files).
	Hex string `json:"hex,omitempty"`
	// Perm bits (0 = 0o644 for files, 0o755 for directories).
	Mode uint32 `json:"mode,omitempty"`
	// Modification time (unix seconds; 0 = baseTime).
	Mtime int64 `json:"mtime,omitempty"`
	// Symlink target.
	Target string `json:"target,omitempty"`
}

type caseSpec struct {
	Name    string  `json:"name"`
	Entries []entry `json:"entries"`
}

const baseTime = 1700000000

func main() {
	out := flag.String("out", "crates/nh-commands/tests/fixtures/staticcopy", "fixture directory")
	flag.Parse()

	var rows []map[string]any
	for _, c := range cases() {
		res, err := runCase(c)
		if err != nil {
			log.Fatalf("%s: %v", c.Name, err)
		}
		again, err := runCase(c)
		if err != nil {
			log.Fatalf("%s: %v", c.Name, err)
		}
		if fmt.Sprint(res) != fmt.Sprint(again) {
			log.Fatalf("%s: nondeterministic result", c.Name)
		}
		rows = append(rows, map[string]any{"name": c.Name, "entries": c.Entries, "result": res})
	}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "staticcopy.json.gz"), map[string]any{"baseTime": baseTime}, rows); err != nil {
		log.Fatal(err)
	}
	fmt.Printf("staticcopy: %d cases\n", len(rows))
}

// materialize writes the entries below root: files and symlinks first, then
// the permissions and modification times (directories last, deepest first).
func materialize(root string, entries []entry) error {
	for _, e := range entries {
		p := filepath.Join(root, filepath.FromSlash(e.Path))
		switch e.Kind {
		case "dir":
			if err := os.MkdirAll(p, 0o755); err != nil {
				return err
			}
		case "file":
			if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
				return err
			}
			b := []byte(e.Content)
			if e.Hex != "" {
				var err error
				if b, err = hex.DecodeString(e.Hex); err != nil {
					return err
				}
			}
			if err := os.WriteFile(p, b, 0o644); err != nil {
				return err
			}
		case "symlink":
			if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
				return err
			}
			if err := os.Symlink(e.Target, p); err != nil {
				return err
			}
		default:
			return fmt.Errorf("unknown kind %q", e.Kind)
		}
	}
	for _, e := range entries {
		if e.Kind != "file" {
			continue
		}
		p := filepath.Join(root, filepath.FromSlash(e.Path))
		mode := os.FileMode(0o644)
		if e.Mode != 0 {
			mode = os.FileMode(e.Mode)
		}
		if err := os.Chmod(p, mode); err != nil {
			return err
		}
		if err := chtimes(p, e.Mtime); err != nil {
			return err
		}
	}
	// Every directory below root, deepest first.
	var dirs []string
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			dirs = append(dirs, path)
		}
		return nil
	})
	if err != nil {
		return err
	}
	sort.Slice(dirs, func(i, j int) bool {
		return len(dirs[i]) > len(dirs[j]) || (len(dirs[i]) == len(dirs[j]) && dirs[i] < dirs[j])
	})
	modes := map[string]entry{}
	for _, e := range entries {
		if e.Kind == "dir" {
			modes[filepath.Join(root, filepath.FromSlash(e.Path))] = e
		}
	}
	for _, d := range dirs {
		e := modes[d]
		if e.Mode != 0 {
			if err := os.Chmod(d, os.FileMode(e.Mode)); err != nil {
				return err
			}
		}
		if err := chtimes(d, e.Mtime); err != nil {
			return err
		}
	}
	return nil
}

func chtimes(p string, mtime int64) error {
	if mtime == 0 {
		mtime = baseTime
	}
	t := time.Unix(mtime, 0)
	return os.Chtimes(p, t, t)
}

// countingStatFs is commands.countingStatFs.
type countingStatFs struct {
	afero.Fs
	statCounter uint64
}

func (fs *countingStatFs) Stat(name string) (os.FileInfo, error) {
	f, err := fs.Fs.Stat(name)
	if err == nil {
		if !f.IsDir() {
			atomic.AddUint64(&fs.statCounter, 1)
		}
	}
	return f, err
}

// chmodFilter is commands.chmodFilter.
func chmodFilter(dst, src os.FileInfo) bool {
	return src.IsDir()
}

func runCase(c caseSpec) (map[string]any, error) {
	tmp, err := os.MkdirTemp("", "nh-commands-static-")
	if err != nil {
		return nil, err
	}
	defer func() { _ = os.RemoveAll(tmp) }()
	tmp, err = filepath.EvalSymlinks(tmp)
	if err != nil {
		return nil, err
	}
	root := filepath.Join(tmp, "site")
	if err := os.MkdirAll(filepath.Join(tmp, "home"), 0o755); err != nil {
		return nil, err
	}
	var entries []entry
	for _, e := range c.Entries {
		e.Content = strings.ReplaceAll(e.Content, "$ROOT", tmp)
		e.Target = strings.ReplaceAll(e.Target, "$ROOT", tmp)
		entries = append(entries, e)
	}
	if err := materialize(root, entries); err != nil {
		return nil, err
	}

	// commands.hugoBuilder.loadConfig + rootCommand.ConfigFromProvider.
	cfg := config.New()
	cfg.Set("renderToMemory", false)
	cfg.Set("environment", "production")
	cfg.Set("internal", map[string]any{"running": false, "watch": false, "verbose": false, "fastRenderMode": false})
	cfg.Set("workingDir", root)
	logBuf := &bytes.Buffer{}
	logger := loggers.New(loggers.Options{Level: logg.LevelWarn, StdOut: logBuf, StdErr: logBuf})
	configs, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Flags:       cfg,
		Fs:          hugofs.Os,
		ConfigDir:   "config",
		Environment: "production",
		Environ:     []string{"HOME=" + filepath.Join(tmp, "home")},
		Logger:      logger,
	})
	if err != nil {
		return map[string]any{"err": strings.ReplaceAll(err.Error(), tmp, "$ROOT")}, nil
	}
	base := configs.Base
	cfg.Set("publishDir", base.PublishDir)
	cfg.Set("publishDirStatic", base.PublishDir)
	cfg.Set("publishDirDynamic", base.PublishDir)
	hfs := hugofs.NewFromSourceAndDestination(hugofs.Os, hugofs.Os, cfg)
	p, err := paths.New(hfs, configs.GetFirstLanguageConfig())
	if err != nil {
		return nil, err
	}
	bfs, err := filesystems.NewBase(p, logger)
	if err != nil {
		return map[string]any{"err": strings.ReplaceAll(err.Error(), tmp, "$ROOT")}, nil
	}

	res := map[string]any{}
	var langs []string
	for lang := range bfs.Static {
		langs = append(langs, lang)
	}
	sort.Strings(langs)
	counts := map[string]uint64{}
	for _, lang := range langs {
		n, err := copyStaticTo(bfs.Static[lang], hfs.PublishDirStatic, base.NoTimes, base.NoChmod, base.CleanDestinationDir)
		if err != nil {
			res["syncErr"] = strings.ReplaceAll(err.Error(), tmp, "$ROOT")
			break
		}
		counts[lang] = n
	}
	res["counts"] = counts
	publishDir := cpaths.AbsPathify(root, base.PublishDir)
	tree, err := dumpTree(publishDir, base.NoTimes, base.NoChmod)
	if err != nil {
		return nil, err
	}
	res["publishDir"] = strings.ReplaceAll(publishDir, tmp, "$ROOT")
	res["tree"] = tree
	if logBuf.Len() > 0 {
		res["log"] = strings.ReplaceAll(logBuf.String(), tmp, "$ROOT")
	}
	return res, nil
}

// copyStaticTo is commands.(*hugoBuilder).copyStaticTo.
func copyStaticTo(sourceFs *filesystems.SourceFilesystem, dest afero.Fs, noTimes, noChmod, clean bool) (uint64, error) {
	publishDir := string(filepath.Separator)
	if sourceFs.PublishFolder != "" {
		publishDir = filepath.Join(publishDir, sourceFs.PublishFolder)
	}
	fs := &countingStatFs{Fs: sourceFs.Fs}
	syncer := fsync.NewSyncer()
	syncer.NoTimes = noTimes
	syncer.NoChmod = noChmod
	syncer.ChmodFilter = chmodFilter
	syncer.DestFs = dest
	syncer.Delete = clean
	syncer.SrcFs = fs
	if syncer.Delete {
		syncer.DeleteFilter = func(f fsync.FileInfo) bool {
			return f.IsDir() && strings.HasPrefix(f.Name(), ".")
		}
	}
	if err := syncer.Sync(publishDir, string(filepath.Separator)); err != nil {
		return 0, err
	}
	return fs.statCounter / 2, nil
}

// mtimeOf is the modification time in unix seconds, or "now" for a time
// after every time the cases set (a directory the sync created without a
// source time: the root mapping's virtual directories report time.Now).
func mtimeOf(fi os.FileInfo) any {
	m := fi.ModTime().Unix()
	if m > nowThreshold {
		return "now"
	}
	return m
}

// nowThreshold is later than every modification time of the cases.
const nowThreshold = 1750000000

// dumpTree lists every entry below dir (Lstat, sorted by path).
func dumpTree(dir string, noTimes, noChmod bool) ([]map[string]any, error) {
	var out []map[string]any
	if _, err := os.Lstat(dir); err != nil {
		return nil, nil
	}
	err := filepath.WalkDir(dir, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if path == dir {
			return nil
		}
		rel, _ := filepath.Rel(dir, path)
		fi, err := os.Lstat(path)
		if err != nil {
			return err
		}
		e := map[string]any{"path": filepath.ToSlash(rel)}
		switch {
		case fi.Mode()&os.ModeSymlink != 0:
			e["kind"] = "symlink"
			t, _ := os.Readlink(path)
			e["target"] = t
		case fi.IsDir():
			e["kind"] = "dir"
			if !noTimes {
				e["mtime"] = mtimeOf(fi)
			}
		default:
			e["kind"] = "file"
			b, err := os.ReadFile(path)
			if err != nil {
				return err
			}
			e["size"] = len(b)
			e["hex"] = hex.EncodeToString(b)
			if !noChmod {
				e["mode"] = uint32(fi.Mode().Perm())
			}
			if !noTimes {
				e["mtime"] = mtimeOf(fi)
			}
		}
		out = append(out, e)
		return nil
	})
	return out, err
}
