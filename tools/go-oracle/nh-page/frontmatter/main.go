// Command frontmatter is the Go oracle for pagemeta.FrontMatterHandler
// (HandleDates, the date handler chains), pagemeta.DecodeFrontMatterConfig
// and pagemeta.DecodeBuildConfig (resources/page/pagemeta) in crates/nh-page
// (Wave B task T11).
//
//	go run ./tools/go-oracle/nh-page/frontmatter [-root .] [-out crates/nh-page/tests/fixtures/frontmatter]
//
// Like the paths oracle it runs itself again with `go run -overlay`; the
// overlay adds a hook to HandleDates that records every call of the real
// builds (docs/, hugolib/testsite and the synthetic psupport sites, whose
// content has dates in YAML, TOML and JSON front matter, strings in the
// layouts cast accepts, unparsable dates, filename dates, date aliases,
// expiry/publish/lastmod precedence and time zones): the page config's params
// before the call, the descriptor (base filename, mod time, location) and the
// result (the four dates, the changed params, the slug).
//
// Every recorded input is then replayed with fresh FrontMatterHandlers over
// frontmatter config variants (the default, the docs site's, :default,
// :filename, :fileModTime, :git (a synthetic git author date), custom keys,
// a string instead of a list), time zones, and with and without a git date.
// The replays capture the errors the handler chain logs.
//
// Output: <site>.json.gz per site, decode.json.gz. Time zone data comes from
// the system (like the Rust test). Nothing depends on the platform.
package main

import (
	"flag"
	"fmt"
	"io"
	"log"
	"path/filepath"
	"sort"
	"sync"
	"time"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/resources/page/pagemeta"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-page/psupport"
)

// installHooks is set by the overlay-added hook file (see hookFile).
var installHooks func(record func(pagemeta.FrontmatterConfig, *pagemeta.FrontMatterDescriptor, bool))

const hookFile = `package main

import "github.com/neohugo/neohugo/resources/page/pagemeta"

func init() {
	installHooks = func(record func(pagemeta.FrontmatterConfig, *pagemeta.FrontMatterDescriptor, bool)) {
		pagemeta.OracleHookHandleDates = record
	}
}
`

var patches = []psupport.Patch{{
	File:   "resources/page/pagemeta/page_frontmatter.go",
	After:  "func (f FrontMatterHandler) HandleDates(d *FrontMatterDescriptor) error {",
	Insert: "\n\tif OracleHookHandleDates != nil {\n\t\tOracleHookHandleDates(f.fmConfig, d, true)\n\t\tdefer OracleHookHandleDates(f.fmConfig, d, false)\n\t}\n",
	Append: "\n// OracleHookHandleDates is set by the nh-page frontmatter oracle.\nvar OracleHookHandleDates func(FrontmatterConfig, *FrontMatterDescriptor, bool)\n",
}}

type call struct {
	cfg          pagemeta.FrontmatterConfig
	baseFilename string
	pathOrTitle  string
	before       map[string]any
	params       maps.Params // a deep copy of the params before the call
	after        map[string]any
}

var (
	mu        sync.Mutex
	recording bool
	pending   = map[*pagemeta.FrontMatterDescriptor]*call{}
	calls     []*call
)

func cloneParams(p maps.Params) maps.Params {
	if p == nil {
		return nil
	}
	c := maps.Params{}
	for k, v := range p {
		switch vv := v.(type) {
		case maps.Params:
			c[k] = cloneParams(vv)
		case map[string]any:
			c[k] = map[string]any(cloneParams(maps.Params(vv)))
		default:
			c[k] = v
		}
	}
	return c
}

func encodeDates(d pagemeta.Dates) map[string]any {
	return map[string]any{
		"date":        goval.Encode(d.Date),
		"lastmod":     goval.Encode(d.Lastmod),
		"publishDate": goval.Encode(d.PublishDate),
		"expiryDate":  goval.Encode(d.ExpiryDate),
	}
}

// changed encodes the params of after that are not in before or differ.
func changed(before, after maps.Params) any {
	out := map[string]any{}
	for k, v := range after {
		bv, ok := before[k]
		if !ok || fmt.Sprintf("%T %#v", bv, bv) != fmt.Sprintf("%T %#v", v, v) {
			out[k] = goval.Encode(v)
		}
	}
	return out
}

func recordHandleDates(cfg pagemeta.FrontmatterConfig, d *pagemeta.FrontMatterDescriptor, before bool) {
	mu.Lock()
	defer mu.Unlock()
	if !recording {
		return
	}
	if before {
		c := &call{cfg: cfg, params: cloneParams(d.PageConfig.Params), baseFilename: d.BaseFilename, pathOrTitle: d.PathOrTitle}
		c.before = map[string]any{
			"params":               goval.Encode(c.params),
			"baseFilename":         goval.Str(d.BaseFilename),
			"pathOrTitle":          goval.Str(d.PathOrTitle),
			"modTime":              goval.Encode(d.ModTime),
			"gitAuthorDate":        goval.Encode(d.GitAuthorDate),
			"location":             d.Location.String(),
			"isFromContentAdapter": d.PageConfig.IsFromContentAdapter,
			"dates":                encodeDates(d.PageConfig.Dates),
			"slug":                 goval.Str(d.PageConfig.Slug),
		}
		pending[d] = c
		return
	}
	c := pending[d]
	delete(pending, d)
	c.after = map[string]any{
		"dates":  encodeDates(d.PageConfig.Dates),
		"params": changed(c.params, d.PageConfig.Params),
		"slug":   goval.Str(d.PageConfig.Slug),
	}
	calls = append(calls, c)
}

// configVariants are [frontmatter] sections decoded with DecodeFrontMatterConfig.
var configVariants = []map[string]any{
	nil,
	{"date": []any{"date"}, "expiryDate": []any{"expirydate"}, "lastmod": []any{":git", "lastmod", "publishdate", "date"}, "publishDate": []any{"publishdate", "date"}},
	{"date": []any{":filename", ":default"}},
	{"date": []any{":fileModTime"}, "lastmod": []any{":FileModTime"}},
	{"lastmod": []any{":git", ":fileModTime"}},
	{"date": []any{"myDate", "date"}, "publishDate": []any{"pubdate"}, "expiryDate": []any{":default", "myExpiry"}},
	{"date": "date", "Lastmod": []any{"modified", ":default"}},
	{"date": []any{":filename"}, "lastmod": []any{":filename"}, "publishDate": []any{":filename"}, "expiryDate": []any{":filename"}},
}

func decodeConfig(m map[string]any) (pagemeta.FrontmatterConfig, error) {
	cfg := config.New()
	if m != nil {
		cfg.Set("frontmatter", m)
	}
	return pagemeta.DecodeFrontMatterConfig(cfg)
}

func encodeConfig(c pagemeta.FrontmatterConfig) map[string]any {
	return map[string]any{"date": c.Date, "lastmod": c.Lastmod, "publishDate": c.PublishDate, "expiryDate": c.ExpiryDate}
}

var locationNames = []string{"UTC", "Asia/Bangkok", "America/New_York"}

// replaySites are the sites whose recorded calls are replayed with every
// config and location (the other synthetic sites have the same content).
var replaySites = map[string]bool{"docs": true, "testsite": true, "seeksnack": true, "subdir": true, "multihost": true}

func replay(c *call, cfg pagemeta.FrontmatterConfig, loc *time.Location, git bool) map[string]any {
	logger := loggers.New(loggers.Options{Level: logg.LevelWarn, StdErr: io.Discard, StdOut: io.Discard, StoreErrors: true})
	h, err := pagemeta.NewFrontmatterHandler(logger, cfg)
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	pc := &pagemeta.PageConfig{Params: cloneParams(c.params)}
	var modTime, gitDate time.Time
	modTime = decodeTime(c.before["modTime"])
	if git {
		gitDate = modTime.Add(36 * time.Hour)
	}
	d := &pagemeta.FrontMatterDescriptor{
		PageConfig:    pc,
		BaseFilename:  c.baseFilename,
		PathOrTitle:   c.pathOrTitle,
		ModTime:       modTime,
		GitAuthorDate: gitDate,
		Location:      loc,
	}
	out := map[string]any{}
	if err := h.HandleDates(d); err != nil {
		out["err"] = err.Error()
	}
	out["dates"] = encodeDates(pc.Dates)
	out["params"] = changed(c.params, pc.Params)
	out["slug"] = goval.Str(pc.Slug)
	out["errors"] = logger.Errors()
	return out
}

func decodeTime(v any) time.Time {
	m := v.(map[string]any)
	t := time.Unix(m["unix"].(int64), int64(m["nsec"].(int)))
	return t.In(time.UTC)
}

func main() {
	root := flag.String("root", ".", "neohugo module root")
	out := flag.String("out", "crates/nh-page/tests/fixtures/frontmatter", "output directory")
	flag.Parse()

	if !psupport.IsChild() {
		if err := psupport.RunOverlaid(*root, "./tools/go-oracle/nh-page/frontmatter", patches, "zz_hooks_overlay.go", hookFile, []string{"-root", *root, "-out", *out}); err != nil {
			log.Fatal(err)
		}
		return
	}
	if installHooks == nil {
		log.Fatal("the recording hook is not installed")
	}
	installHooks(recordHandleDates)
	outDir := psupport.OutDir(*root, *out)

	var cfgs []pagemeta.FrontmatterConfig
	var cfgDump []any
	for _, m := range configVariants {
		c, err := decodeConfig(m)
		if err != nil {
			log.Fatal(err)
		}
		cfgs = append(cfgs, c)
		cfgDump = append(cfgDump, encodeConfig(c))
	}
	var locs []*time.Location
	for _, n := range locationNames {
		l, err := time.LoadLocation(n)
		if err != nil {
			log.Fatal(err)
		}
		locs = append(locs, l)
	}

	sites, err := psupport.RepoSites(*root)
	if err != nil {
		log.Fatal(err)
	}
	sites = append(sites, psupport.SyntheticSites()...)
	total := 0
	for _, s := range sites {
		mu.Lock()
		recording = true
		calls = nil
		mu.Unlock()
		b, err := psupport.Build(s)
		mu.Lock()
		recording = false
		recs := calls
		calls = nil
		mu.Unlock()
		if err != nil {
			log.Fatal(err)
		}
		_ = b
		sort.SliceStable(recs, func(i, j int) bool {
			return fmt.Sprint(recs[i].before["pathOrTitle"], recs[i].before["location"]) < fmt.Sprint(recs[j].before["pathOrTitle"], recs[j].before["location"])
		})
		var cases []map[string]any
		for i, c := range recs {
			cs := map[string]any{"before": c.before, "after": c.after, "cfg": encodeConfig(c.cfg)}
			if replaySites[s.Name] {
				var vs []any
				for ci, cfg := range cfgs {
					for li, l := range locs {
						git := (i+ci+li)%2 == 0
						if s.Name == "docs" && li > 0 {
							continue
						}
						vs = append(vs, map[string]any{"cfg": ci, "loc": li, "git": git, "out": replay(c, cfg, l, git)})
					}
				}
				cs["variants"] = vs
			}
			cases = append(cases, cs)
		}
		header := map[string]any{"site": s.Name, "configs": cfgDump, "locations": locationNames}
		if err := goval.WriteCasesGz(filepath.Join(outDir, s.Name+".json.gz"), header, cases); err != nil {
			log.Fatal(err)
		}
		total += len(cases)
		log.Printf("%s: %d HandleDates calls", s.Name, len(cases))
	}

	if err := writeDecode(outDir); err != nil {
		log.Fatal(err)
	}
	log.Printf("frontmatter: %d calls", total)
}

// writeDecode writes the DecodeFrontMatterConfig and DecodeBuildConfig cases.
func writeDecode(outDir string) error {
	var cases []map[string]any
	fmIns := append([]map[string]any{}, configVariants...)
	fmIns = append(fmIns,
		map[string]any{"date": []any{":default", ":default"}, "unknown": []any{"x"}},
		map[string]any{"DATE": []any{"Date", "PUBLISHDATE"}},
		map[string]any{"expirydate": []any{}},
		map[string]any{"publishdate": 42},
		map[string]any{"lastmod": []any{"lastmod", "modified", "lastmod"}},
		map[string]any{"date": nil},
	)
	for _, m := range fmIns {
		cases = append(cases, map[string]any{
			"fn": "frontmatter",
			"in": goval.Encode(maps.Params(m)),
			"want": goval.CallRaw(func() (any, error) {
				c, err := decodeConfig(m)
				if err != nil {
					return nil, err
				}
				return encodeConfig(c), nil
			}),
		})
	}
	buildIns := []any{
		nil, map[string]any{}, map[string]any{"list": "never", "render": "link", "publishResources": false},
		map[string]any{"list": true, "render": false}, map[string]any{"list": false, "render": true},
		map[string]any{"list": "local", "render": "never"}, map[string]any{"list": "LOCAL", "render": "Always"},
		map[string]any{"list": "bogus", "render": "bogus"}, map[string]any{"list": 0, "render": 1},
		map[string]any{"publishResources": "false"}, map[string]any{"publishResources": "notabool"},
		map[string]any{"List": "never", "RENDER": "link"}, "notamap", []any{"a"},
		map[string]any{"list": []any{"x"}},
	}
	for _, in := range buildIns {
		b, err := pagemeta.DecodeBuildConfig(in)
		res := map[string]any{"list": b.List, "render": b.Render, "publishResources": b.PublishResources, "isZero": b.IsZero()}
		if err != nil {
			res["err"] = err.Error()
		}
		cases = append(cases, map[string]any{"fn": "build", "in": goval.Encode(in), "want": res})
	}
	return goval.WriteCasesGz(filepath.Join(outDir, "decode.json.gz"), map[string]any{}, cases)
}
