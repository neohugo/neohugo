// Command filecache is the Go oracle for cache/filecache (filecache.go,
// filecache_config.go) and helpers.GetCacheDir in crates/nh-helpers (Wave B
// task T08).
//
//		go run ./tools/go-oracle/nh-helpers/filecache [-root .] [-out crates/nh-helpers/tests/fixtures/filecache]
//
//	  - DecodeConfig: the default cache set, the seeksnack `caches` section
//	    (docs/rust-port/specs/architecture-core-data/config-en.json), custom dirs
//	    with the :cacheDir, :resourceDir and :project tokens (any case), absolute,
//	    relative and root dirs, maxAge as a duration string, an int, -1 and 0,
//	    unknown cache names, missing dirs, bad placeholders, non-map entries;
//	    over an OsFs and a MemMapFs, with two BaseConfigs.
//	  - GetCacheDir: the cacheDir argument ("", 1 character, absolute, with and
//	    without a trailing slash, an existing file) under the environment
//	    variables it reads (NETLIFY/PULL_REQUEST/DEPLOY_PRIME_URL,
//	    XDG_CACHE_HOME absolute and relative, HOME, USER, TMPDIR) in a temporary
//	    tree; plus allconfig.LoadConfig with and without a cacheDir setting and
//	    HUGO_CACHEDIR (the resolution order).
//	  - Cache operations: seeded sequences of GetBytes, Get, GetOrCreate,
//	    GetOrCreateBytes, ReadOrCreate (read errors, ErrFatal), WriteCloser,
//	    GetString and the AsHTTPCache Get/Set/Delete, on caches with maxAge -1,
//	    0 and 1h whose entries are 10 minutes or 2 hours old (expiry and
//	    removal), with odd ids (leading slash, "..", nested, Thai); the result
//	    of every operation and the files left on disk.
//	  - NewCaches: the directory each cache of a loaded site writes to.
//
// Paths below the temporary root are recorded as $ROOT/... Output:
// filecache.json.gz. Nothing here depends on the platform.
package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"io/fs"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/neohugo/neohugo/cache/filecache"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-helpers/hsupport"
	"github.com/spf13/afero"
)

var root string

func rel(s string) string {
	return strings.ReplaceAll(s, root, "$ROOT")
}

func configsDump(c filecache.Configs) any {
	keys := make([]string, 0, len(c))
	for k := range c {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	var out []any
	for _, k := range keys {
		v := c[k]
		out = append(out, []any{k, map[string]any{
			"maxAge": int64(v.MaxAge), "dir": rel(v.Dir), "dirCompiled": rel(v.DirCompiled), "isResourceDir": v.IsResourceDir,
		}})
	}
	return out
}

func decodeConfigCases(repo string) []map[string]any {
	b, err := os.ReadFile(filepath.Join(repo, "docs/rust-port/specs/architecture-core-data/config-en.json"))
	if err != nil {
		log.Fatal(err)
	}
	var m map[string]any
	if err := json.Unmarshal(b, &m); err != nil {
		log.Fatal(err)
	}
	seek := maps.Params{}
	for k, v := range m["caches"].(map[string]any) {
		p := maps.Params{}
		for k2, v2 := range v.(map[string]any) {
			if f, ok := v2.(float64); ok {
				v2 = int(f)
			}
			p[k2] = v2
		}
		seek[k] = p
	}
	inputs := []map[string]any{
		nil,
		{},
		map[string]any(seek),
		{"getresource": maps.Params{"dir": ":cacheDir/:project", "maxAge": "1h30m"}},
		{"getresource": maps.Params{"dir": ":CACHEDIR/:Project/x", "maxage": 0}},
		{"images": maps.Params{"dir": ":resourceDir/_gen", "maxAge": "-1s"}, "assets": maps.Params{"dir": ":resourceDir/other"}},
		{"misc": maps.Params{"dir": "/abs/cache/dir/"}},
		{"misc": maps.Params{"dir": "rel/dir"}},
		{"misc": maps.Params{"dir": "/"}},
		{"misc": maps.Params{"dir": "/a/../"}},
		{"misc": maps.Params{"dir": ":cacheDir/../x"}},
		{"misc": maps.Params{"dir": "_gen/x"}},
		{"misc": maps.Params{"dir": ":resourceDir"}},
		{"misc": maps.Params{"dir": ":resourceDir/:project"}},
		{"misc": maps.Params{"dir": ""}},
		{"misc": maps.Params{"maxAge": "10s"}},
		{"misc": maps.Params{"dir": ":bogus/x"}},
		{"misc": maps.Params{"dir": "/x", "maxAge": "fast"}},
		{"misc": maps.Params{"dir": "/x", "maxAge": true}},
		{"misc": maps.Params{"dir": "/x", "maxAge": 12.5}},
		{"MISC": maps.Params{"dir": "/x"}},
		{"nope": maps.Params{"dir": "/x"}},
		{"misc": "notparams"},
		{"misc": map[string]any{"dir": "/plainmap"}},
		{"modules": maps.Params{"dir": ":cacheDir/mods", "maxAge": 3600000000000}},
		{"getjson": maps.Params{"dir": ":cacheDir/:project", "dirCompiled": "ignored", "isResourceDir": true}},
	}
	bcfgs := []config.BaseConfig{
		{WorkingDir: "/work/seeksnack", CacheDir: "/cache/hugo_cache"},
		{WorkingDir: "rel/site", CacheDir: "relcache"},
	}
	var cases []map[string]any
	for i, in := range inputs {
		for j, bc := range bcfgs {
			for _, fsName := range []string{"os", "mem"} {
				afs := afero.Fs(afero.NewOsFs())
				if fsName == "mem" {
					afs = afero.NewMemMapFs()
				}
				c := map[string]any{"in": i, "bcfg": j, "fs": fsName}
				cfg, err := filecache.DecodeConfig(afs, bc, in)
				if err != nil {
					c["err"] = err.Error()
					// Go ranges over maps in random order: collect every error text.
					seen := map[string]bool{err.Error(): true}
					for range 200 {
						if _, err2 := filecache.DecodeConfig(afs, bc, in); err2 != nil {
							seen[err2.Error()] = true
						}
					}
					if len(seen) > 1 {
						var errs []string
						for e := range seen {
							errs = append(errs, e)
						}
						sort.Strings(errs)
						c["nondetErrs"] = errs
						delete(c, "err")
					}
				} else {
					c["ok"] = configsDump(cfg)
					c["modulesDir"] = cfg.CacheDirModules()
				}
				cases = append(cases, c)
			}
		}
	}
	var encIn []any
	for _, in := range inputs {
		encIn = append(encIn, goval.Encode(in))
	}
	var encB []any
	for _, bc := range bcfgs {
		encB = append(encB, map[string]any{"workingDir": bc.WorkingDir, "cacheDir": bc.CacheDir})
	}
	return append([]map[string]any{{"inputs": encIn, "bcfgs": encB}}, cases...)
}

var envKeys = []string{"NETLIFY", "PULL_REQUEST", "DEPLOY_PRIME_URL", "XDG_CACHE_HOME", "HOME", "USER", "TMPDIR"}

func getCacheDirCases() []map[string]any {
	type env map[string]string
	fileAt := filepath.Join(root, "afile")
	if err := os.WriteFile(fileAt, []byte("x"), 0o666); err != nil {
		log.Fatal(err)
	}
	args := []string{"", "x", "/", "$ROOT/cfgcache", "$ROOT/cfgcache/", "$ROOT/a/b/c", "$ROOT/afile", "$ROOT/afile/sub", "rel/dir"}
	envs := []env{
		{"HOME": "$ROOT/home", "USER": "alice", "TMPDIR": "$ROOT/tmp"},
		{"XDG_CACHE_HOME": "$ROOT/xdg", "HOME": "$ROOT/home", "TMPDIR": "$ROOT/tmp"},
		{"XDG_CACHE_HOME": "relxdg", "HOME": "$ROOT/home", "USER": "bob", "TMPDIR": "$ROOT/tmp"},
		{"TMPDIR": "$ROOT/tmp"},
		{"USER": "carol", "TMPDIR": "$ROOT/tmp/"},
		{"USER": "d@v ภาษา", "TMPDIR": "$ROOT/tmp"},
		{"NETLIFY": "true", "PULL_REQUEST": "1", "DEPLOY_PRIME_URL": "u", "HOME": "$ROOT/home", "TMPDIR": "$ROOT/tmp"},
		{"NETLIFY": "true", "PULL_REQUEST": "", "DEPLOY_PRIME_URL": "u", "HOME": "$ROOT/home", "TMPDIR": "$ROOT/tmp"},
		{"HOME": "$ROOT/afile", "USER": "e", "TMPDIR": "$ROOT/tmp"},
		{"HOME": "$ROOT/home", "USER": "f", "TMPDIR": "$ROOT/tmp", "_mk": "home"},
		{"HOME": "$ROOT/home", "USER": "g", "TMPDIR": "$ROOT/tmp", "_mk": "home,hugo"},
		{"XDG_CACHE_HOME": "$ROOT/xdg", "HOME": "$ROOT/home", "TMPDIR": "$ROOT/tmp", "_mk": "xdg"},
		{"XDG_CACHE_HOME": "$ROOT/xdg/", "HOME": "$ROOT/home", "TMPDIR": "$ROOT/tmp", "_mk": "xdg,home"},
		{"XDG_CACHE_HOME": "relxdg", "HOME": "$ROOT/home", "TMPDIR": "$ROOT/tmp", "_mk": "xdg,home"},
		{"HOME": "$ROOT/home", "TMPDIR": "$ROOT/tmp", "_mk": "tmphugo"},
	}
	var cases []map[string]any
	n := 0
	for _, e := range envs {
		for _, a := range args {
			n++
			// A fresh home and tmp per case, so that directory creation is visible.
			for _, d := range []string{"home", "xdg", "tmp", "cfgcache", "a"} {
				_ = os.RemoveAll(filepath.Join(root, d))
			}
			if err := os.MkdirAll(filepath.Join(root, "home"), 0o777); err != nil {
				log.Fatal(err)
			}
			for _, mk := range strings.Split(e["_mk"], ",") {
				d := map[string]string{
					"home": "home/.cache", "hugo": "home/.cache/hugo_cache", "xdg": "xdg", "tmphugo": "tmp/hugo_cache",
				}[mk]
				if d == "" {
					continue
				}
				if err := os.MkdirAll(filepath.Join(root, d), 0o777); err != nil {
					log.Fatal(err)
				}
			}
			for _, k := range envKeys {
				if v, ok := e[k]; ok {
					_ = os.Setenv(k, strings.ReplaceAll(v, "$ROOT", root))
				} else {
					_ = os.Unsetenv(k)
				}
			}
			arg := strings.ReplaceAll(a, "$ROOT", root)
			if strings.HasPrefix(a, "rel/") {
				continue
			}
			// Netlify's cache dir is outside the temporary tree: use a MemMapFs.
			afs := afero.Fs(afero.NewOsFs())
			if e["NETLIFY"] == "true" {
				afs = afero.NewMemMapFs()
			}
			res, err := helpers.GetCacheDir(afs, arg)
			c := map[string]any{"env": e, "arg": a, "mem": e["NETLIFY"] == "true"}
			if err != nil {
				c["err"] = rel(err.Error())
			} else {
				c["ok"] = rel(res)
				fi, statErr := afs.Stat(res)
				c["isDir"] = statErr == nil && fi.IsDir()
			}
			cases = append(cases, c)
		}
	}
	for _, k := range envKeys {
		_ = os.Unsetenv(k)
	}
	return cases
}

func loadConfigCases(tmp string) []map[string]any {
	var cases []map[string]any
	_ = os.Setenv("HOME", filepath.Join(root, "home"))
	_ = os.Setenv("TMPDIR", filepath.Join(root, "tmp"))
	for i, tc := range []struct {
		toml    string
		environ []string
	}{
		{toml: "title = \"x\"\n"},
		{toml: "cacheDir = \"" + filepath.Join(root, "fromconfig") + "\"\n"},
		{toml: "title = \"x\"\n", environ: []string{"HUGO_CACHEDIR=" + filepath.Join(root, "fromenv")}},
		{toml: "cacheDir = \"" + filepath.Join(root, "fromconfig") + "\"\n", environ: []string{"HUGO_CACHEDIR=" + filepath.Join(root, "fromenv")}},
		{toml: "cacheDir = \"" + filepath.Join(root, "fromconfig") + "\"\n", environ: []string{"HUGO_CACHEDIR="}},
	} {
		site, err := hsupport.LoadSite(tmp, fmt.Sprintf("lc%d", i), tc.toml, nil, tc.environ)
		c := map[string]any{"case": i}
		if err != nil {
			c["err"] = rel(err.Error())
		} else {
			c["cacheDir"] = rel(site.Confs.GetFirstLanguageConfig().BaseConfig().CacheDir)
			caches := site.Confs.GetFirstLanguageConfig().GetConfigSection("caches").(filecache.Configs)
			c["getresource"] = rel(caches[filecache.CacheKeyGetResource].DirCompiled)
		}
		cases = append(cases, c)
	}
	return cases
}

func listDir(dir string) []any {
	var out []any
	_ = filepath.WalkDir(dir, func(p string, d fs.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return nil
		}
		b, _ := os.ReadFile(p)
		r, _ := filepath.Rel(dir, p)
		out = append(out, []any{filepath.ToSlash(r), hsupport.Str(string(b))})
		return nil
	})
	return out
}

func errRel(err error) any {
	if err == nil {
		return nil
	}
	return map[string]any{"err": rel(err.Error())}
}

var ids = []string{"a", "/a", "b/c.json", "../x", "a/../b", " sp", "ขนม/ไทย", "deep/er/id", "5844198154546968338"}

type cacheSetup struct {
	name   string
	maxAge time.Duration
}

var cacheSetups = []cacheSetup{{"forever", -1}, {"disabled", 0}, {"hour", time.Hour}}

func opsCases() []map[string]any {
	rng := rand.New(rand.NewSource(3))
	var cases []map[string]any
	for seq := range 240 {
		cs := cacheSetups[seq%len(cacheSetups)]
		dir := filepath.Join(root, fmt.Sprintf("ops%d", seq))
		if err := os.MkdirAll(dir, 0o777); err != nil {
			log.Fatal(err)
		}
		// Pre-existing entries with ages.
		var pre []any
		for _, id := range ids {
			if rng.Intn(2) == 0 {
				continue
			}
			clean := strings.TrimPrefix(filepath.Clean(id), "/")
			if strings.HasPrefix(clean, "..") {
				continue
			}
			age := []int{10, 120}[rng.Intn(2)]
			p := filepath.Join(dir, clean)
			if err := os.MkdirAll(filepath.Dir(p), 0o777); err != nil {
				continue
			}
			content := "pre-" + clean
			if err := os.WriteFile(p, []byte(content), 0o666); err != nil {
				continue
			}
			mt := time.Now().Add(-time.Duration(age) * time.Minute)
			if err := os.Chtimes(p, mt, mt); err != nil {
				log.Fatal(err)
			}
			pre = append(pre, []any{clean, age})
		}
		c := filecache.NewCache(hugofs.NewBasePathFs(afero.NewOsFs(), dir), cs.maxAge, "")
		hc := c.AsHTTPCache()
		var ops []any
		for range 6 {
			id := ids[rng.Intn(len(ids))]
			op := rng.Intn(11)
			sub := rng.Intn(3)
			res := func() (res any) {
				defer func() {
					if r := recover(); r != nil {
						res = map[string]any{"panic": rel(fmt.Sprint(r))}
					}
				}()
				switch op {
				case 0:
					info, b, err := c.GetBytes(id)
					var bv any
					if b != nil {
						bv = hsupport.Str(string(b))
					}
					res = []any{info.Name, bv, errRel(err)}
				case 1:
					info, r, err := c.Get(id)
					var bv any
					if r != nil {
						b, _ := io.ReadAll(r)
						_ = r.Close()
						bv = hsupport.Str(string(b))
					}
					res = []any{info.Name, bv, errRel(err)}
				case 2:
					info, b, err := c.GetOrCreateBytes(id, func() ([]byte, error) { return []byte("created-" + id), nil })
					res = []any{info.Name, hsupport.Str(string(b)), errRel(err)}
				case 3:
					info, b, err := c.GetOrCreateBytes(id, func() ([]byte, error) { return nil, errors.New("create failed") })
					var bv any
					if b != nil {
						bv = hsupport.Str(string(b))
					}
					res = []any{info.Name, bv, errRel(err)}
				case 4:
					info, r, err := c.GetOrCreate(id, func() (io.ReadCloser, error) {
						return io.NopCloser(strings.NewReader("streamed-" + id)), nil
					})
					var bv any
					if r != nil {
						b, _ := io.ReadAll(r)
						_ = r.Close()
						bv = hsupport.Str(string(b))
					}
					res = []any{info.Name, bv, errRel(err)}
				case 5, 6, 7:
					readErr := []error{nil, errors.New("corrupt"), filecache.ErrFatal}[op-5]
					var read string
					info, err := c.ReadOrCreate(id, func(info filecache.ItemInfo, r io.ReadSeeker) error {
						b, _ := io.ReadAll(r)
						read = string(b)
						return readErr
					}, func(info filecache.ItemInfo, w io.WriteCloser) error {
						_, err := w.Write([]byte("rc-" + id))
						_ = w.Close()
						return err
					})
					res = []any{op - 5, info.Name, hsupport.Str(read), errRel(err)}
				case 8:
					info, w, err := c.WriteCloser(id)
					if err == nil {
						_, _ = w.Write([]byte("wc-" + id))
						_ = w.Close()
					}
					res = []any{info.Name, errRel(err)}
				case 9:
					b, ok := hc.Get(id)
					var bv any
					if b != nil {
						bv = hsupport.Str(string(b))
					}
					res = []any{bv, ok}
				case 10:
					switch sub {
					case 0:
						hc.Set(id, []byte("set-"+id))
						res = []any{"set"}
					case 1:
						hc.Delete(id)
						res = []any{"delete"}
					default:
						res = []any{"string", hsupport.Str(c.GetString(id))}
					}
				}
				return res
			}()
			ops = append(ops, []any{op, id, sub, res})
		}
		cases = append(cases, map[string]any{"setup": cs.name, "pre": pre, "ops": ops, "files": listDir(dir)})
	}
	return cases
}

func newCachesCases(tmp string) []map[string]any {
	toml := `baseURL = "https://example.org/"
cacheDir = "` + filepath.Join(root, "nccache") + `"
resourceDir = "myresources"
[caches]
[caches.getresource]
dir = ":cacheDir/:project"
maxAge = -1
[caches.images]
dir = ":resourceDir/_gen"
[caches.misc]
dir = ":resourceDir/misc"
maxAge = "1h"
[caches.getjson]
dir = "` + filepath.Join(root, "absjson") + `"
`
	site, err := hsupport.LoadSite(tmp, "seeksnack", toml, nil, nil)
	if err != nil {
		log.Fatal(err)
	}
	conf := site.Confs.GetFirstLanguageConfig()
	fsys := hugofs.NewFrom(afero.NewOsFs(), conf.BaseConfig())
	p, err := helpers.NewPathSpec(fsys, conf, loggers.NewDefault())
	if err != nil {
		log.Fatal(err)
	}
	caches, err := filecache.NewCaches(p)
	if err != nil {
		log.Fatal(err)
	}
	var cases []map[string]any
	names := make([]string, 0, len(caches))
	for k := range caches {
		names = append(names, k)
	}
	sort.Strings(names)
	for _, k := range names {
		c := caches.Get(strings.ToUpper(k))
		if _, _, err := c.GetOrCreateBytes("probe/"+k, func() ([]byte, error) { return []byte(k), nil }); err != nil {
			log.Fatal(err)
		}
		var found []string
		_ = filepath.WalkDir(root, func(p string, d fs.DirEntry, err error) error {
			if err == nil && !d.IsDir() && d.Name() == k && filepath.Base(filepath.Dir(p)) == "probe" {
				found = append(found, rel(p))
			}
			return nil
		})
		cases = append(cases, map[string]any{"cache": k, "found": found})
	}
	cfgs := conf.GetConfigSection("caches").(filecache.Configs)
	return append([]map[string]any{{
		"workingDir": rel(conf.BaseConfig().WorkingDir),
		"cacheDir":   rel(conf.BaseConfig().CacheDir),
		"configs":    configsDump(cfgs),
		"resources":  rel(p.AbsResourcesDir),
	}}, cases...)
}

func main() {
	repo := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-helpers/tests/fixtures/filecache", "output directory")
	flag.Parse()

	tmp, err := os.MkdirTemp("", "nh-helpers-filecache")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()
	tmp, err = filepath.EvalSymlinks(tmp)
	if err != nil {
		log.Fatal(err)
	}
	root = tmp

	decode := decodeConfigCases(*repo)
	header := map[string]any{
		"decodeInputs": decode[0],
		"decode":       decode[1:],
		"getCacheDir":  getCacheDirCases(),
		"loadConfig":   loadConfigCases(filepath.Join(tmp, "sites")),
		"newCaches":    newCachesCases(filepath.Join(tmp, "ncsites")),
		"envKeys":      envKeys,
		"ids":          ids,
	}
	cases := opsCases()
	p := filepath.Join(*out, "filecache.json.gz")
	if err := hsupport.WriteGz(p, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d decode, %d op sequences", p, len(decode)-1, len(cases))
}
