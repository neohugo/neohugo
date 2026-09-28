// Command host is the Go oracle of the host template namespaces of
// crates/nh-tplfuncs (Wave B task T19): css, data, debug, diagrams, hugo,
// images, inflect, js, lang, openapi3, os, page, partials, path, site,
// strings, templates, time, transform and urls, and the func map of
// tplimplinit.CreateFuncMap.
//
//	go run ./tools/go-oracle/nh-tplfuncs/host [-out crates/nh-tplfuncs/tests/fixtures/host]
//
// Every case runs against a real site: the oracle builds a set of small
// sites (one language, Thai, a baseURL with a path, canonifyURLs,
// multilingual with and without the default language in a subdirectory,
// multihost, the title case styles) in memory with hugolib (process and
// assemble, no rendering), creates the func map of a site's deps with
// tplimplinit.CreateFuncMap and calls the namespace methods (or the func map
// functions) the way text/template calls them: the context is injected, the
// argument count and typed parameters are checked with Go's messages and a
// panic becomes the error. The Rust test builds the same sites with
// nh-hugolib (process + assemble + freeze) and runs the same calls, in the
// same order, through nh_tplfuncs::tplimplinit::create_func_map.
//
// Arguments are encoded values (goval's typed JSON), pages of the site
// ({"t":"@page"}), page lists ({"t":"@pages"}) or the results of nested calls
// ({"t":"@call"}). Results are the typed JSON of the value or the error text.
//
// Floating point results (lang.FormatNumber*, image filters) depend on the
// platform: the fixtures are produced by a linux/arm64 build run under
// qemu-aarch64-static (the golden build ran on darwin/arm64). That build has
// no cgo (arm64build stubs LibSass), so the css topic (css.Sass runs LibSass)
// comes from a native run:
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplfuncs/arm64build -pkg ./tools/go-oracle/nh-tplfuncs/host -o /tmp/host.arm64
//	qemu-aarch64-static /tmp/host.arm64 -skip css -out crates/nh-tplfuncs/tests/fixtures/host
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-tplfuncs/host -only css -out crates/nh-tplfuncs/tests/fixtures/host
//
// The sites are built in a temporary directory on the real filesystem (the
// os namespace's errors and listings, the published files), which is
// replaced by /SITE in the recorded strings and hashed files.
package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"reflect"
	"regexp"
	"slices"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/bep/clocks"
	"github.com/neohugo/neohugo/common/htime"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tpl"
	"github.com/neohugo/neohugo/tpl/tplimplinit"
)

// env is a site language the cases run against.
type env struct {
	name    string
	d       *deps.Deps
	site    page.Site
	funcMap map[string]any
	enc     *encoder
}

// pageArg is a page of the case's site (by path).
type pageArg struct{ path string }

// pagesArg is a page list of the case's site ("regular" or "all").
type pagesArg struct{ which string }

// callArg is the result of a nested call on the case's site.
type callArg struct {
	m    string
	args []any
}

// ctxOpts are the context options of a case.
type ctxOpts struct {
	page string
}

type oracle struct {
	envs  map[string]*env
	sites []*builtSite
	cases map[string][]map[string]any
	order []string
	clock time.Time
}

var addrRe = regexp.MustCompile(`0x[0-9a-f]{6,}`)

// maskAddrs replaces memory addresses in error texts and strings with 0xADDR.
func maskAddrs(v any) {
	switch x := v.(type) {
	case map[string]any:
		for k, e := range x {
			if s, ok := e.(string); ok && (k == "err" || k == "s") {
				x[k] = addrRe.ReplaceAllString(s, "0xADDR")
				continue
			}
			maskAddrs(e)
		}
	case []any:
		for _, e := range x {
			maskAddrs(e)
		}
	}
}

func (o *oracle) env(name string) *env {
	e, ok := o.envs[name]
	if !ok {
		panic("no env " + name)
	}
	return e
}

// encArg encodes an argument for the fixture.
func (o *oracle) encArg(e *env, a any) any {
	switch x := a.(type) {
	case pageArg:
		return map[string]any{"t": "@page", "path": x.path}
	case pagesArg:
		return map[string]any{"t": "@pages", "which": x.which}
	case callArg:
		var as []any
		for _, aa := range x.args {
			as = append(as, o.encArg(e, aa))
		}
		if as == nil {
			as = []any{}
		}
		return map[string]any{"t": "@call", "m": x.m, "a": as}
	}
	return e.enc.enc(a)
}

// resolve returns the Go value of an argument.
func (o *oracle) resolve(ctx context.Context, e *env, a any) (any, error) {
	switch x := a.(type) {
	case pageArg:
		p, err := e.site.GetPage(x.path)
		if err != nil {
			return nil, err
		}
		if p == nil {
			return nil, fmt.Errorf("no page %s", x.path)
		}
		return p, nil
	case pagesArg:
		switch x.which {
		case "regular":
			return e.site.RegularPages(), nil
		case "all":
			return e.site.Pages(), nil
		}
		panic(x.which)
	case callArg:
		return o.invoke(ctx, e, x.m, x.args)
	}
	return a, nil
}

// invoke calls m ("ns.Method" or "f:name") on the env.
func (o *oracle) invoke(ctx context.Context, e *env, m string, args []any) (any, error) {
	var vals []any
	for _, a := range args {
		v, err := o.resolve(ctx, e, a)
		if err != nil {
			return nil, fmt.Errorf("argument: %w", err)
		}
		vals = append(vals, v)
	}
	if name, ok := strings.CutPrefix(m, "f:"); ok {
		f, ok := e.funcMap[name]
		if !ok {
			return nil, fmt.Errorf("no func %s", name)
		}
		return callFunc(ctx, reflect.ValueOf(f), name, vals)
	}
	ns, meth, ok := strings.Cut(m, ".")
	if !ok {
		panic(m)
	}
	nsf, ok := e.funcMap[ns].(func(context.Context, ...any) (any, error))
	if !ok {
		panic("no namespace " + ns)
	}
	recv, err := nsf(ctx)
	if err != nil {
		return nil, err
	}
	return call(ctx, recv, meth, vals)
}

// add records one case of topic on env: method m with the arguments.
func (o *oracle) add(topic, envName, m string, args ...any) {
	o.addCtx(topic, envName, ctxOpts{}, m, args...)
}

// maskSite replaces the site's directory in every string of r with /SITE (the
// Rust test builds the site in a temporary directory).
func maskSite(r map[string]any, dir string) map[string]any {
	b, err := json.Marshal(r)
	if err != nil {
		panic(err)
	}
	s := strings.ReplaceAll(string(b), dir+"/", "/SITE/")
	s = strings.ReplaceAll(s, dir+`"`, `/SITE"`)
	var out map[string]any
	if err := json.Unmarshal([]byte(s), &out); err != nil {
		panic(err)
	}
	return out
}

// addCtx is add with context options.
func (o *oracle) addCtx(topic, envName string, co ctxOpts, m string, args ...any) {
	e := o.env(envName)
	ctx := context.Background()
	if co.page != "" {
		p, err := e.site.GetPage(co.page)
		if err != nil || p == nil {
			panic(fmt.Sprintf("ctx page %s: %v", co.page, err))
		}
		ctx = tpl.Context.Page.Set(ctx, p)
	}
	refs := make([]any, len(args))
	for i, a := range args {
		refs[i] = o.encArg(e, a)
	}
	v, err := o.invoke(ctx, e, m, args)
	var r map[string]any
	if err != nil {
		r = map[string]any{"err": str(err.Error())}
	} else {
		r = map[string]any{"ok": e.enc.enc(v)}
	}
	if topic == "time_now" {
		// time.Now under the clock runs from the clock's start: recorded as
		// checked (the Rust test checks its own result the same way).
		t, ok := v.(time.Time)
		if !ok || t.Before(o.clock) || !t.Before(o.clock.Add(time.Hour)) {
			panic(fmt.Sprintf("time.Now: %v", v))
		}
		r = map[string]any{"now": "within an hour of the clock"}
	}
	maskAddrs(r)
	r = maskSite(r, siteDir(strings.Split(envName, "/")[0]))
	c := map[string]any{"env": envName, "m": m, "a": refs, "r": r}
	if co.page != "" {
		c["ctx"] = map[string]any{"page": co.page}
	}
	if _, ok := o.cases[topic]; !ok {
		o.order = append(o.order, topic)
	}
	o.cases[topic] = append(o.cases[topic], c)
}

// fatal removes the sites' directory and exits.
func fatal(err error) {
	_ = os.RemoveAll(sitesRoot)
	log.Fatal(err)
}

func main() {
	out := flag.String("out", "crates/nh-tplfuncs/tests/fixtures/host", "output directory")
	only := flag.String("only", "", "comma-separated generators to run (default: all)")
	skip := flag.String("skip", "", "comma-separated generators to skip")
	flag.Parse()

	root, err := os.MkdirTemp("", "nh-tplfuncs-host")
	if err != nil {
		log.Fatal(err)
	}
	if sitesRoot, err = filepath.EvalSymlinks(root); err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(sitesRoot) }()

	// The clock of time.Now (Hugo's --clock).
	const clock = "2021-11-17T20:34:10+07:00"
	start, err := time.Parse(time.RFC3339, clock)
	if err != nil {
		fatal(err)
	}
	htime.Clock = clocks.Start(start)

	o := &oracle{envs: map[string]*env{}, cases: map[string][]map[string]any{}, clock: start}
	var siteFixtures []any
	for _, s := range sites() {
		b, err := buildSite(s)
		if err != nil {
			fatal(err)
		}
		o.sites = append(o.sites, b)
		siteFixtures = append(siteFixtures, encodeSite(s))
		// The build stopped the error collector; errors sent after it (a
		// failed transformation) would panic on its closed channel. The
		// handler is shared by the sites of the build.
		b.errs = b.h.Sites[0].StartErrorCollector()
		for i, hs := range b.h.Sites {
			name := s.Name + "/" + strconv.Itoa(i)
			d := hs.Deps
			o.envs[name] = &env{
				name:    name,
				d:       d,
				site:    page.WrapSite(hs),
				funcMap: tplimplinit.CreateFuncMap(d),
				enc:     &encoder{publish: d.Fs.PublishDir, dir: siteDir(s.Name)},
			}
		}
	}

	gens := []struct {
		name string
		f    func(*oracle)
	}{
		{"funcnames", genFuncNames}, {"strings", genStrings}, {"path", genPath}, {"urls", genURLs},
		{"time", genTime}, {"transform", genTransform}, {"lang", genLang}, {"inflect", genInflect},
		{"os", genOS}, {"debug", genDebug}, {"templates", genTemplates}, {"partials", genPartials},
		{"host", genHost}, {"images", genImages}, {"css", genCSS}, {"js", genJS}, {"stubs", genStubs},
		{"tables", genTables},
	}
	for _, g := range gens {
		if (*only != "" && !slices.Contains(strings.Split(*only, ","), g.name)) || slices.Contains(strings.Split(*skip, ","), g.name) {
			continue
		}
		g.f(o)
	}

	if err := os.MkdirAll(*out, 0o755); err != nil {
		fatal(err)
	}
	// The sites and what their builds and the cases logged (a partial run
	// leaves the sites file alone).
	if *only == "" {
		var logs []any
		for _, b := range o.sites {
			errs := []any{}
		drain:
			for {
				select {
				case err := <-b.errs:
					errs = append(errs, str(strings.ReplaceAll(err.Error(), siteDir(b.s.Name), "/SITE")))
				default:
					break drain
				}
			}
			logs = append(logs, map[string]any{"site": b.s.Name, "log": str(strings.ReplaceAll(b.log.String(), siteDir(b.s.Name), "/SITE")), "errors": errs})
		}
		header := map[string]any{"sites": siteFixtures, "clock": clock, "logs": logs}
		if err := goval.WriteCasesGz(filepath.Join(*out, "sites.json.gz"), header, nil); err != nil {
			fatal(err)
		}
	}
	total := 0
	sort.Strings(o.order)
	for _, topic := range o.order {
		cs := o.cases[topic]
		total += len(cs)
		if err := goval.WriteCasesGz(filepath.Join(*out, topic+".json.gz"), map[string]any{"topic": topic}, cs); err != nil {
			fatal(err)
		}
	}
	fmt.Printf("%d sites, %d envs, %d cases in %d topics\n", len(o.sites), len(o.envs), total, len(o.order))
}

// genFuncNames records the sorted names of every site's func map.
func genFuncNames(o *oracle) {
	var names []string
	for k := range o.env("en/0").funcMap {
		names = append(names, k)
	}
	sort.Strings(names)
	o.cases["funcnames"] = []map[string]any{{"names": names}}
	o.order = append(o.order, "funcnames")
}
