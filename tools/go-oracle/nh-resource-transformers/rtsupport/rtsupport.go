// Package rtsupport holds what the nh-resource-transformers oracles (Wave B
// task T15) share: the tpl `resources` namespace over a site's resource spec
// (rsupport.LoadSite: the publish dir in memory, cold cache dirs), the template
// store of a hugolib build of a temporary copy of the site (for
// ExecuteAsTemplate), and a small script interpreter: every case is a list of
// namespace calls whose arguments are encoded values or references to earlier
// results. The Rust tests run the same scripts through nh_tplfuncs::resources.
package rtsupport

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"sort"

	"github.com/neohugo/neohugo/common/hexec"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/security"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/postpub"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
	tplresources "github.com/neohugo/neohugo/tpl/resources"
	"github.com/neohugo/neohugo/tpl/tplimpl"
	"github.com/neohugo/neohugo/tpl/tplimplinit"
	"github.com/spf13/afero"
)

// Store builds a temporary copy of the site in dir with hugolib (SkipRender)
// and returns its first site's template store and the names of the template
// functions (the Rust test parses the embedded templates with stubs of these).
func Store(dir string) (*tplimpl.TemplateStore, []string, error) {
	tmp, err := os.MkdirTemp("", "nh-resource-transformers-store")
	if err != nil {
		return nil, nil, err
	}
	defer func() { _ = os.RemoveAll(tmp) }()
	buildDir := filepath.Join(tmp, "site")
	if err := os.CopyFS(buildDir, os.DirFS(dir)); err != nil {
		return nil, nil, err
	}
	var logBuf bytes.Buffer
	res, logger, err := rsupport.LoadConfigs(buildDir, rsupport.Flags(buildDir, tmp), &logBuf)
	if err != nil {
		return nil, nil, err
	}
	fsCfg := config.New()
	fsCfg.Set("workingDir", buildDir)
	fsCfg.Set("publishDir", res.LoadingInfo.BaseConfig.PublishDir)
	hfs := hugofs.NewFromSourceAndDestination(hugofs.Os, afero.NewMemMapFs(), fsCfg)
	hfs.WorkingDirWritable = afero.NewMemMapFs()
	h, err := hugolib.NewHugoSites(deps.DepsCfg{Configs: res, Fs: hfs, LogLevel: logger.Level(), StdErr: &logBuf, StdOut: &logBuf})
	if err != nil {
		return nil, nil, err
	}
	if err := h.Build(hugolib.BuildCfg{SkipRender: true}); err != nil {
		return nil, nil, fmt.Errorf("build: %w\n%s", err, logBuf.String())
	}
	d := h.Sites[0].Deps
	var names []string
	for k := range tplimplinit.CreateFuncMap(d) {
		names = append(names, k)
	}
	sort.Strings(names)
	return d.TemplateStore, names, nil
}

// Namespace creates the tpl resources namespace over the first language's
// resource spec of s (and the template store, which may be nil).
// rsupport.LoadSite creates the specs without an exec helper; the create
// client needs its security config (GetRemote), as deps.Init sets it.
func Namespace(s *rsupport.Site, store *tplimpl.TemplateStore) (*tplresources.Namespace, error) {
	for _, sp := range s.Specs {
		if sp.ExecHelper == nil {
			sp.ExecHelper = hexec.New(sp.Cfg.GetConfigSection("security").(security.Config), s.Dir, sp.Logger)
		}
	}
	d := &deps.Deps{ResourceSpec: s.Specs[0], TemplateStore: store}
	return tplresources.New(d)
}

// Step is one namespace call of a script.
type Step struct {
	// Op is the Go method name (Get, Concat, ...) or rec/content/pp.
	Op   string `json:"op"`
	Args []any  `json:"args"`
	// As stores the result under a name.
	As string `json:"as,omitempty"`
}

// Case is a named script.
type Case struct {
	Name  string `json:"name"`
	Steps []Step `json:"steps"`
}

// Ref refers to a stored result.
func Ref(name string) any { return map[string]any{"t": "ref", "v": name} }

// Refs builds a resource.Resources from stored results.
func Refs(names ...string) any {
	out := []any{}
	for _, n := range names {
		out = append(out, n)
	}
	return map[string]any{"t": "refs", "v": out}
}

// AnyRefs builds a []interface {} of stored results.
func AnyRefs(names ...string) any {
	out := []any{}
	for _, n := range names {
		out = append(out, n)
	}
	return map[string]any{"t": "anyrefs", "v": out}
}

// S is an encoded string argument.
func S(s string) any { return rsupport.Enc(s) }

// V is an encoded argument.
func V(v any) any { return rsupport.Enc(v) }

// Runner runs scripts; results are kept by name across cases (one build).
type Runner struct {
	NS   *tplresources.Namespace
	Vars map[string]any
}

func (r *Runner) dec(a any) any {
	m := a.(map[string]any)
	switch m["t"] {
	case "ref":
		return r.Vars[m["v"].(string)]
	case "refs":
		var rr resource.Resources
		for _, n := range m["v"].([]any) {
			rr = append(rr, r.Vars[n.(string)].(resource.Resource))
		}
		return rr
	case "anyrefs":
		var rr []any
		for _, n := range m["v"].([]any) {
			rr = append(rr, r.Vars[n.(string)])
		}
		return rr
	default:
		return Dec(a)
	}
}

// Dec decodes rsupport.Enc.
func Dec(a any) any {
	m := a.(map[string]any)
	v := m["v"]
	switch m["t"] {
	case "nil":
		return nil
	case "string":
		return v.(string)
	case "bool":
		return v.(bool)
	case "int":
		return toInt(v)
	case "int64":
		return int64(toInt(v))
	case "[]string":
		var out []string
		for _, x := range v.([]any) {
			out = append(out, x.(string))
		}
		return out
	case "[]any":
		out := []any{}
		for _, x := range v.([]any) {
			out = append(out, Dec(x))
		}
		return out
	case "map":
		out := map[string]any{}
		for _, e := range v.([]any) {
			kv := e.([]any)
			out[kv[0].(string)] = Dec(kv[1])
		}
		return out
	case "maps.Params":
		out := maps.Params{}
		for _, e := range v.([]any) {
			kv := e.([]any)
			out[kv[0].(string)] = Dec(kv[1])
		}
		return out
	}
	panic(fmt.Sprintf("unknown encoded type %v", m["t"]))
}

func toInt(v any) int {
	switch n := v.(type) {
	case int:
		return n
	case float64:
		return int(n)
	}
	panic(fmt.Sprintf("not a number: %T", v))
}

// wrongType is text/template's validateType error for a typed parameter.
func wrongType(expected string, v any) error {
	return fmt.Errorf("wrong type for value; expected %s; got %T", expected, v)
}

func recoverTo(err *error) {
	if e := recover(); e != nil {
		switch v := e.(type) {
		case error:
			*err = v
		default:
			*err = errors.New(fmt.Sprint(v))
		}
	}
}

// call runs one namespace method (panics become errors, as text/template's
// safeCall reports them).
func (r *Runner) call(op string, args []any) (res any, err error) {
	defer recoverTo(&err)
	ns := r.NS
	arg := func(i int) any {
		if i < len(args) {
			return args[i]
		}
		return nil
	}
	switch op {
	case "Get":
		return ns.Get(arg(0)), nil
	case "GetMatch":
		return ns.GetMatch(arg(0)), nil
	case "Match":
		return ns.Match(arg(0)), nil
	case "ByType":
		return ns.ByType(arg(0)), nil
	case "GetRemote":
		return ns.GetRemote(args...)
	case "FromString":
		return ns.FromString(arg(0), arg(1))
	case "Concat":
		return ns.Concat(arg(0), arg(1))
	case "Copy":
		rr, ok := arg(1).(resource.Resource)
		if !ok {
			return nil, wrongType("resource.Resource", arg(1))
		}
		return ns.Copy(arg(0), rr)
	case "ExecuteAsTemplate":
		return ns.ExecuteAsTemplate(context.Background(), args...)
	case "Fingerprint":
		return ns.Fingerprint(args...)
	case "Minify":
		rr, ok := arg(0).(resources.ResourceTransformer)
		if !ok {
			return nil, wrongType("resources.ResourceTransformer", arg(0))
		}
		return ns.Minify(rr)
	case "PostProcess":
		rr, ok := arg(0).(resource.Resource)
		if !ok {
			return nil, wrongType("resource.Resource", arg(0))
		}
		return ns.PostProcess(rr)
	}
	panic("unknown op " + op)
}

// Rec records a result value.
func Rec(v any) map[string]any {
	switch vv := v.(type) {
	case nil:
		return map[string]any{"nil": true}
	case resource.Resources:
		if vv == nil {
			return map[string]any{"nilList": true}
		}
		var items []any
		for _, x := range vv {
			items = append(items, map[string]any{"name": x.Name(), "key": x.(resource.Identifier).Key(), "mediaType": x.MediaType().Type, "resourceType": x.ResourceType()})
		}
		return map[string]any{"list": items}
	case *postpub.PostPublishResource:
		return recPP(vv)
	case resource.Resource:
		return map[string]any{"res": recRes(vv)}
	}
	return map[string]any{"other": fmt.Sprintf("%T", v)}
}

// recRes is rsupport.Rec(r, true) with Go panics recorded (a resource over a
// directory panics when read: "this operation is not supported").
func recRes(r resource.Resource) (o map[string]any) {
	defer func() {
		if e := recover(); e != nil {
			o = map[string]any{"panic": fmt.Sprint(e)}
		}
	}()
	return rsupport.Rec(r, true)
}

func recPP(p *postpub.PostPublishResource) map[string]any {
	c, _ := p.Content(context.Background())
	o := map[string]any{
		"relPermalink": p.RelPermalink(),
		"permalink":    p.Permalink(),
		"name":         p.Name(),
		"title":        p.Title(),
		"resourceType": p.ResourceType(),
		"content":      c,
		"data":         rsupport.JSON(p.Data()),
		"mediaType":    rsupport.JSON(p.MediaType()),
	}
	fields := map[string]any{}
	for _, f := range []string{"Content", "RelPermalink", "Permalink", "Name", "Title", "ResourceType", "Data.Integrity", "MediaType.Type", "MediaType.SubType", "MediaType.Suffixes"} {
		pattern := p.RelPermalink()
		pattern = pattern[:len(pattern)-len("RelPermalink__e=")] + f + "__e="
		func() {
			defer func() {
				if e := recover(); e != nil {
					fields[f] = map[string]any{"panic": fmt.Sprint(e)}
				}
			}()
			s, ok := p.GetFieldString(pattern)
			fields[f] = map[string]any{"s": s, "ok": ok}
		}()
	}
	o["fields"] = fields
	return map[string]any{"pp": o}
}

// AssetFiles lists the files below dir/assets (slash paths relative to it,
// sorted).
func AssetFiles(dir string) []string {
	var out []string
	root := filepath.Join(dir, "assets")
	_ = filepath.WalkDir(root, func(p string, d os.DirEntry, err error) error {
		if err != nil || d.IsDir() {
			return err
		}
		rel, err := filepath.Rel(root, p)
		if err != nil {
			return err
		}
		out = append(out, filepath.ToSlash(rel))
		return nil
	})
	sort.Strings(out)
	return out
}

// RunSite runs the cases over a fresh load of the site in dir (one runner for
// all cases, like one build) and returns the fixture object: the cases with
// their results and the published files.
func RunSite(dir string, store *tplimpl.TemplateStore, cases []Case) map[string]any {
	s, err := rsupport.LoadSite(dir)
	if err != nil {
		panic(err)
	}
	defer s.Close()
	ns, err := Namespace(s, store)
	if err != nil {
		panic(err)
	}
	r := &Runner{NS: ns, Vars: map[string]any{}}
	var out []any
	for _, c := range cases {
		out = append(out, map[string]any{"name": c.Name, "steps": c.Steps, "results": r.Run(c)})
	}
	if os.Getenv("T15_DEBUG") != "" {
		fmt.Fprintln(os.Stderr, s.Log.String())
	}
	return map[string]any{"cases": out, "published": s.PublishedFiles()}
}

// Run runs a case and returns the result of each step.
func (r *Runner) Run(c Case) []any {
	var out []any
	for _, st := range c.Steps {
		var args []any
		for _, a := range st.Args {
			args = append(args, r.dec(a))
		}
		var res map[string]any
		switch st.Op {
		case "rec":
			res = Rec(args[0])
		default:
			v, err := r.call(st.Op, args)
			if err != nil {
				res = map[string]any{"err": err.Error()}
			} else {
				if st.As != "" {
					r.Vars[st.As] = v
				}
				res = Rec(v)
			}
		}
		out = append(out, res)
	}
	return out
}
