// Package t16support holds what the Wave B task T16 (js-css-pipeline) oracles
// share: a hermetic copy of a fixture site (so nothing is ever written into
// the repository), its resource spec with an exec helper (postcss), and a
// small script runner over the real resource clients: resources.Get,
// resources.Concat, js.Build, toCSS (libsass), postCSS, minify and
// fingerprint. Absolute paths of the site copy are replaced by "$SITE" in
// everything recorded, so the Rust tests (which use their own copy) compare
// the same bytes.
package t16support

import (
	"bytes"
	"context"
	"encoding/base64"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/cache/dynacache"
	"github.com/neohugo/neohugo/cache/filecache"
	"github.com/neohugo/neohugo/common/hexec"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/security"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/identity"
	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/resources/resource_factories/bundler"
	"github.com/neohugo/neohugo/resources/resource_factories/create"
	"github.com/neohugo/neohugo/resources/resource_transformers/cssjs"
	"github.com/neohugo/neohugo/resources/resource_transformers/integrity"
	jstransform "github.com/neohugo/neohugo/resources/resource_transformers/js"
	"github.com/neohugo/neohugo/resources/resource_transformers/minifier"
	"github.com/neohugo/neohugo/resources/resource_transformers/tocss/scss"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
	"github.com/spf13/afero"
)

// CopySite copies the fixture site src into a fresh temporary directory
// (resolved through symlinks): a top-level "_node_modules" becomes
// "node_modules". only (if not nil) limits the copy to these top-level
// entries; links adds symlinks (relative name -> target).
func CopySite(src string, only []string, links map[string]string) (string, error) {
	tmp, err := os.MkdirTemp("", "nh-t16-site")
	if err != nil {
		return "", err
	}
	tmp, err = filepath.EvalSymlinks(tmp)
	if err != nil {
		return "", err
	}
	dst := filepath.Join(tmp, "site")
	err = filepath.Walk(src, func(p string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(src, p)
		if err != nil {
			return err
		}
		if only != nil && rel != "." {
			top := strings.SplitN(rel, string(filepath.Separator), 2)[0]
			keep := false
			for _, o := range only {
				keep = keep || o == top
			}
			if !keep {
				if info.IsDir() {
					return filepath.SkipDir
				}
				return nil
			}
		}
		if rel == "_node_modules" || strings.HasPrefix(rel, "_node_modules"+string(filepath.Separator)) {
			rel = "node_modules" + strings.TrimPrefix(rel, "_node_modules")
		}
		target := filepath.Join(dst, rel)
		if info.IsDir() {
			return os.MkdirAll(target, 0o755)
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		return os.WriteFile(target, b, 0o644)
	})
	if err != nil {
		return "", err
	}
	var names []string
	for name := range links {
		names = append(names, name)
	}
	sort.Strings(names)
	for _, name := range names {
		p := filepath.Join(dst, name)
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			return "", err
		}
		if err := os.Symlink(links[name], p); err != nil {
			return "", err
		}
	}
	return dst, nil
}

// Site is a loaded site copy with the clients the scripts use.
type Site struct {
	*rsupport.Site
	Spec *resources.Spec

	create   *create.Client
	bundler  *bundler.Client
	js       *jstransform.Client
	scss     *scss.Client
	postcss  *cssjs.PostCSSClient
	minifier *minifier.Client
	integ    *integrity.Client

	results   map[string]resource.Resource
	published map[string]bool
}

// LoadSite loads the site in dir (one language) like rsupport.LoadSite, with
// an exec helper (Go deps: hexec.New with the site's security config).
func LoadSite(dir string) (*Site, error) {
	tmp, err := os.MkdirTemp("", "nh-t16-oracle")
	if err != nil {
		return nil, err
	}
	var logBuf bytes.Buffer
	res, logger, err := rsupport.LoadConfigs(dir, rsupport.Flags(dir, tmp), &logBuf)
	if err != nil {
		return nil, fmt.Errorf("load config: %w", err)
	}
	fsCfg := config.New()
	fsCfg.Set("workingDir", dir)
	fsCfg.Set("publishDir", res.LoadingInfo.BaseConfig.PublishDir)
	pub := afero.NewMemMapFs()
	hfs := hugofs.NewFromSourceAndDestination(hugofs.Os, pub, fsCfg)
	memCache := dynacache.New(dynacache.Options{Log: logger})

	conf := res.ConfigLangs()[0]
	ps, err := helpers.NewPathSpec(hfs, conf, logger)
	if err != nil {
		return nil, err
	}
	fileCaches, err := filecache.NewCaches(ps)
	if err != nil {
		return nil, err
	}
	ex := hexec.New(conf.GetConfigSection("security").(security.Config), conf.WorkingDir(), logger)
	spec, err := resources.NewSpec(ps, nil, fileCaches, memCache, &identity.IncrementByOne{}, logger, nil, ex, nil, nil)
	if err != nil {
		return nil, err
	}
	sc, err := scss.New(ps.Assets, spec)
	if err != nil {
		return nil, err
	}
	mc, err := minifier.New(spec)
	if err != nil {
		return nil, err
	}
	s := &Site{
		Site: &rsupport.Site{
			Dir: dir, Configs: res, Specs: []*resources.Spec{spec}, Langs: []string{conf.Language().Lang},
			PublishFs: pub, Log: &logBuf, TmpDir: tmp,
		},
		Spec:      spec,
		create:    create.New(spec),
		bundler:   bundler.New(spec),
		js:        jstransform.New(ps.Assets, spec),
		scss:      sc,
		postcss:   cssjs.NewPostCSSClient(spec),
		minifier:  mc,
		integ:     integrity.New(spec),
		results:   map[string]resource.Resource{},
		published: map[string]bool{},
	}
	return s, nil
}

var inlineSourceMapRe = regexp.MustCompile(`data:application/json;base64,([A-Za-z0-9+/=]+)`)

// Norm replaces the site dir (and its parent temp dir) with "$SITE"/"$TMP",
// also inside inline (base64) source maps, which are decoded to
// "data:application/json;base64,DECODED(<json>)".
func (s *Site) Norm(v string) string {
	v = inlineSourceMapRe.ReplaceAllStringFunc(v, func(m string) string {
		b, err := base64.StdEncoding.DecodeString(strings.TrimPrefix(m, "data:application/json;base64,"))
		if err != nil {
			return m
		}
		return "data:application/json;base64,DECODED(" + string(b) + ")"
	})
	v = strings.ReplaceAll(v, s.Dir, "$SITE")
	return strings.ReplaceAll(v, filepath.Dir(s.Dir), "$TMP")
}

// Step is one operation of a script.
type Step struct {
	Op     string   `json:"op"`
	Path   string   `json:"path,omitempty"`
	Name   string   `json:"name,omitempty"`
	Target string   `json:"target,omitempty"`
	Refs   []string `json:"refs,omitempty"`
	// Opts is the options map (nil = a nil map); OptsEnc its typed encoding.
	Opts    map[string]any `json:"-"`
	OptsEnc any            `json:"opts,omitempty"`
}

// Case is one script: steps applied in order to a resource, then recorded.
type Case struct {
	Name  string `json:"name"`
	Steps []Step `json:"steps"`
	// Links: also call RelPermalink (publishes).
	Links bool `json:"links"`
}

// Get is resources.Get.
func Get(p string) Step { return Step{Op: "get", Path: p} }

// Ref is the result of an earlier case.
func Ref(name string) Step { return Step{Op: "ref", Name: name} }

// Concat is resources.Concat of earlier results.
func Concat(target string, refs ...string) Step {
	return Step{Op: "concat", Target: target, Refs: refs}
}

// JS is js.Build.
func JS(m map[string]any) Step { return Step{Op: "js", Opts: m, OptsEnc: rsupport.Enc(m)} }

// ToCSS is toCSS (libsass; scss.DecodeOptions then ToCSS).
func ToCSS(m map[string]any) Step { return Step{Op: "tocss", Opts: m, OptsEnc: rsupport.Enc(m)} }

// PostCSS is postCSS.
func PostCSS(m map[string]any) Step { return Step{Op: "postcss", Opts: m, OptsEnc: rsupport.Enc(m)} }

// Minify is minify.
func Minify() Step { return Step{Op: "minify"} }

// Fingerprint is fingerprint (sha256).
func Fingerprint() Step { return Step{Op: "fingerprint"} }

// Run runs the cases in order and returns their records.
func (s *Site) Run(cases []Case) []any {
	var out []any
	for _, c := range cases {
		out = append(out, s.run(c))
	}
	return out
}

func (s *Site) run(c Case) map[string]any {
	rec := map[string]any{"name": c.Name}
	var r resource.Resource
	for i, st := range c.Steps {
		var err error
		switch st.Op {
		case "get":
			r, err = s.create.Get(st.Path)
			if err == nil && r == nil {
				err = fmt.Errorf("resource %q not found", st.Path)
			}
		case "ref":
			r = s.results[st.Name]
		case "concat":
			var rs resource.Resources
			for _, n := range st.Refs {
				rs = append(rs, s.results[n])
			}
			r, err = s.bundler.Concat(st.Target, rs)
		case "js":
			r, err = s.js.Process(r.(resources.ResourceTransformer), st.Opts)
		case "tocss":
			var o scss.Options
			o, err = scss.DecodeOptions(st.Opts)
			if err == nil {
				r, err = s.scss.ToCSS(r.(resources.ResourceTransformer), o)
			}
		case "postcss":
			r, err = s.postcss.Process(r.(resources.ResourceTransformer), st.Opts)
		case "minify":
			r, err = s.minifier.Minify(r.(resources.ResourceTransformer))
		case "fingerprint":
			r, err = s.integ.Fingerprint(r.(resources.ResourceTransformer), "")
		default:
			panic(fmt.Sprintf("unknown op %v", st.Op))
		}
		if err != nil {
			rec["stepErr"] = s.Norm(err.Error())
			rec["stepErrAt"] = i
			return rec
		}
	}
	s.results[c.Name] = r
	if tk, ok := r.(interface{ TransformationKey() string }); ok {
		rec["transformationKey"] = tk.TransformationKey()
	}
	v, err := r.(resource.ContentProvider).Content(context.Background())
	if err != nil {
		rec["contentErr"] = s.Norm(err.Error())
	} else {
		rec["content"] = s.Norm(v.(string))
	}
	rec["mediaType"] = r.MediaType().Type
	rec["data"] = rsupport.JSON(r.Data())
	if c.Links {
		rec["relPermalink"] = r.RelPermalink()
	}
	rec["published"] = s.newPublished()
	return rec
}

// newPublished returns the files published since the last call (path
// relative to the publish dir -> normalized content).
func (s *Site) newPublished() map[string]any {
	out := map[string]any{}
	prefix := filepath.Join(s.Dir, s.Configs.LoadingInfo.BaseConfig.PublishDir) + string(filepath.Separator)
	_ = afero.Walk(s.PublishFs, "/", func(p string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() || s.published[p] {
			return nil
		}
		s.published[p] = true
		f, err := s.PublishFs.Open(p)
		if err != nil {
			return nil
		}
		defer func() { _ = f.Close() }()
		b, err := io.ReadAll(f)
		if err != nil {
			return nil
		}
		out[filepath.ToSlash(strings.TrimPrefix(p, prefix))] = s.Norm(string(b))
		return nil
	})
	return out
}
