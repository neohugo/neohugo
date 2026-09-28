// Command srcfile is the Go oracle for source.File (fileInfo.go) and
// source.SourceSpec (sourceSpec.go) in crates/nh-helpers (Wave B task T08).
//
//	go run ./tools/go-oracle/nh-helpers/srcfile [-root .] [-out crates/nh-helpers/tests/fixtures/srcfile]
//
// Inputs: every file and directory of docs/content and
// hugolib/testsite/content (the seeksnack site is private), plus synthetic
// content files: leaf and branch bundles, _index files, language suffixes of
// enabled, disabled and unknown languages, output format and kind
// identifiers, content adapters (_content.gotmpl), several dots, no
// extension, upper case, spaces, Thai, CJK and emoji names. Each path is
// parsed with the real PathParser of a site loaded by allconfig.LoadConfig
// (en, th, nn and 9 disabled languages, like the seeksnack config); every
// PathParser callback is recorded for the Rust test to replay. Each file is
// also built with source.NewContentFileInfoFrom.
//
// SourceSpec.IgnoreFile runs over the same names plus dot, '#' and '~' names
// and "" with 4 inclusion filters, an ignoreFiles config and both an OsFs and
// a MemMapFs source filesystem; the cfg.IgnoreFile answers are recorded.
//
// Output: srcfile.json.gz. Nothing here depends on the platform.
package main

import (
	"flag"
	"io/fs"
	"log"
	"os"
	"path/filepath"
	"sort"
	"sync"

	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugofs/files"
	"github.com/neohugo/neohugo/hugofs/glob"
	"github.com/neohugo/neohugo/source"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-helpers/hsupport"
	"github.com/spf13/afero"
)

const siteTOML = `baseURL = "https://seeksnack.com/"
canonifyURLs = true
defaultContentLanguage = "en"
disableLanguages = ["de", "es", "fr", "ja", "nl", "pl", "pt", "zh-cn", "zh-tw"]
ignoreFiles = ["\\.bak$", "^draft-", "(?i)SECRET"]
[languages.en]
weight = 1
[languages.th]
weight = 2
[languages.nn]
weight = 3
[languages.de]
weight = 6
[languages.es]
weight = 7
[languages.fr]
weight = 4
[languages.ja]
weight = 10
[languages.nl]
weight = 5
[languages.pl]
weight = 8
[languages.pt]
weight = 9
[languages.zh-cn]
weight = 11
[languages.zh-tw]
weight = 12
`

var synthetic = []string{
	"_index.md", "_index.th.md", "_index.nn.md", "_index.fr.md", "index.md", "index.th.md",
	"posts/_index.md", "posts/_index.th.md", "posts/my-post.md", "posts/my-post.th.md",
	"posts/my-post.fr.md", "posts/my-post.xx.md", "posts/bundle/index.md", "posts/bundle/index.th.md",
	"posts/bundle/index.nn.md", "posts/bundle/image.jpg", "posts/bundle/data.th.json",
	"posts/bundle/sub/deep.md", "posts/branch/_index.md", "posts/branch/child.md",
	"biscuit/koalas-march-chocolate/index.md", "biscuit/koalas-march-chocolate/index.th.md",
	"a/b/c/d.md", "a/b.c/d.e.f.md", "a/_content.gotmpl", "a/_content.th.gotmpl", "noext",
	"a/noext", ".hidden.md", "a/.hidden/x.md", "UPPER/Case.MD", "UPPER/Mixed Case File.Md",
	"space dir/My File.md", "ขนม/ไทย.md", "ขนม/ไทย.th.md", "ขนม/บันเดิล/index.th.md",
	"日本語/テスト.md", "emoji/😀.md", "sect/doc.html", "sect/doc.th.html", "sect/doc.amp.html",
	"sect/doc.en.amp.html", "sect/list.rss.xml", "sect/page.json", "sect/page.th.json",
	"a.b.c.md", "a..md", "a.md.md", "a.th.th.md", "x/index.xml", "x/_index.en.md",
	"sect/sub/_index.html", "posts/2024-01-02-dated.md", "posts/post with  spaces.md",
	"%20/a%20b.md", "a/b/../c.md", "headless/index.md", "r/robots.txt", "s/sitemap.xml",
	"s/single.en.html", "s/baseof.html", "x.markdown", "x.adoc", "x.org", "x.pdc", "x.rst",
	"x.htm", "x.th.htm", "x.mdown",
}

var extraNames = []string{
	"", ".", ".git", ".DS_Store", "#emacs#", "file~", "a/#b.md", "a/b.md~", "a/.b.md",
	"draft-post.md", "a/draft-post.md", "post.bak", "a/b.bak.md", "secret.md", "a/Secret/x.md",
	"a/b~/c.md", "a/.x/c.md", "~", "#", "a/", "/", "/a/b.md", "a/b",
}

type recCfg struct {
	config.AllProvider
	mu  *sync.Mutex
	log map[string]bool
}

func (c recCfg) IgnoreFile(s string) bool {
	v := c.AllProvider.IgnoreFile(s)
	c.mu.Lock()
	c.log[s] = v
	c.mu.Unlock()
	return v
}

type filterDef struct {
	Name     string   `json:"name"`
	Includes []string `json:"includes"`
	Excludes []string `json:"excludes"`
}

var filters = []filterDef{
	{Name: "nil"},
	{Name: "md-only", Includes: []string{"**.md"}},
	{Name: "exclude-posts", Excludes: []string{"posts/**", "**.json"}},
	{Name: "both", Includes: []string{"**/*.md", "*.md"}, Excludes: []string{"**/_index*"}},
}

func contentPaths(root string) ([]string, error) {
	var out []string
	for _, r := range []string{"docs/content", "hugolib/testsite/content"} {
		base := filepath.Join(root, r)
		err := filepath.WalkDir(base, func(p string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			rel, err := filepath.Rel(base, p)
			if err != nil {
				return err
			}
			if rel == "." {
				return nil
			}
			out = append(out, filepath.ToSlash(rel))
			return nil
		})
		if err != nil {
			return nil, err
		}
	}
	return out, nil
}

func fileDump(f *source.File) map[string]any {
	return map[string]any{
		"Filename":            hsupport.Str(f.Filename()),
		"Path":                hsupport.Call(f.Path),
		"Dir":                 hsupport.Call(f.Dir),
		"Ext":                 hsupport.Call(f.Ext),
		"LogicalName":         hsupport.Call(f.LogicalName),
		"BaseFileName":        hsupport.Call(f.BaseFileName),
		"TranslationBaseName": hsupport.Call(f.TranslationBaseName),
		"ContentBaseName":     hsupport.Call(f.ContentBaseName),
		"Section":             hsupport.Call(f.Section),
		"UniqueID":            hsupport.Call(f.UniqueID),
		"String":              hsupport.Call(f.String),
		"IsContentAdapter":    f.IsContentAdapter(),
		"IsZero":              f.IsZero(),
	}
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-helpers/tests/fixtures/srcfile", "output directory")
	flag.Parse()

	tmp, err := os.MkdirTemp("", "nh-helpers-source")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	site, err := hsupport.LoadSite(tmp, "site", siteTOML, nil, nil)
	if err != nil {
		log.Fatal(err)
	}
	conf := site.Confs.GetFirstLanguageConfig()
	rec := hsupport.NewRecorder()
	pp := rec.Wrap(conf.PathParser())

	cpaths, err := contentPaths(*root)
	if err != nil {
		log.Fatal(err)
	}
	set := map[string]bool{}
	var paths []string
	for _, p := range append(append([]string{}, synthetic...), cpaths...) {
		if !set[p] {
			set[p] = true
			paths = append(paths, p)
		}
	}
	sort.Strings(paths)

	var cases []map[string]any
	for _, p := range paths {
		pi := pp.Parse(files.ComponentFolderContent, "/"+p)
		meta := &hugofs.FileMeta{
			Filename: filepath.Join("/site/content", p),
			PathInfo: pi,
			Lang:     pi.Lang(),
		}
		f := source.NewFileInfo(hugofs.NewFileMetaInfo(nil, meta))
		c := map[string]any{"path": p, "lang": pi.Lang(), "file": fileDump(f)}
		c["from"] = fileDump(source.NewContentFileInfoFrom(p, "/x/"+p))
		cases = append(cases, c)
	}
	var nilFile *source.File
	nilCase := map[string]any{"nilIsZero": nilFile.IsZero()}

	// IgnoreFile.
	mu := &sync.Mutex{}
	ilog := map[string]bool{}
	rc := recCfg{AllProvider: conf, mu: mu, log: ilog}
	fsys := hugofs.NewFrom(afero.NewOsFs(), conf.BaseConfig())
	ps, err := helpers.NewPathSpec(fsys, rc, loggers.NewDefault())
	if err != nil {
		log.Fatal(err)
	}
	var ignoreCases []map[string]any
	names := append(append([]string{}, paths...), extraNames...)
	for _, fd := range filters {
		var ff *glob.FilenameFilter
		if fd.Name != "nil" {
			ff, err = glob.NewFilenameFilter(fd.Includes, fd.Excludes)
			if err != nil {
				log.Fatal(err)
			}
		}
		for _, fsName := range []string{"os", "mem"} {
			sfs := afero.Fs(afero.NewOsFs())
			if fsName == "mem" {
				sfs = afero.NewMemMapFs()
			}
			ss := source.NewSourceSpec(ps, ff, sfs)
			var res []bool
			for _, n := range names {
				res = append(res, ss.IgnoreFile(n))
			}
			ignoreCases = append(ignoreCases, map[string]any{"filter": fd.Name, "fs": fsName, "r": res})
		}
	}
	var ikeys []string
	for k := range ilog {
		ikeys = append(ikeys, k)
	}
	sort.Strings(ikeys)
	var ignoreLog [][]any
	for _, k := range ikeys {
		ignoreLog = append(ignoreLog, []any{k, ilog[k]})
	}

	header := map[string]any{
		"parser":      rec.Describe(conf.PathParser()),
		"nil":         nilCase,
		"filters":     filters,
		"ignoreNames": names,
		"ignore":      ignoreCases,
		"ignoreLog":   ignoreLog,
	}
	p := filepath.Join(*out, "srcfile.json.gz")
	if err := hsupport.WriteGz(p, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d files, %d ignore names", p, len(cases), len(names))
}
