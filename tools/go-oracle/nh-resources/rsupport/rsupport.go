// Package rsupport holds what the nh-resources oracles (Wave B task T14)
// share: loading a site's config and creating its resources.Spec (one per
// language, sharing SpecCommon and the memory cache, as deps does) with the
// publish dir in memory and a cold resources/cache dir, the typed value
// encoding of the fixtures, the gzipped JSON writer and the record of a
// resource's observable attributes.
package rsupport

import (
	"bytes"
	"compress/gzip"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"runtime"
	"sort"
	"strings"
	"time"
	"unsafe"

	"github.com/bep/logg"
	"github.com/neohugo/neohugo/cache/dynacache"
	"github.com/neohugo/neohugo/cache/filecache"
	"github.com/neohugo/neohugo/common/hugio"
	"github.com/neohugo/neohugo/common/loggers"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/identity"
	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/spf13/afero"
)

// Site is a loaded site: one resources.Spec per language.
type Site struct {
	Dir       string
	Configs   *allconfig.Configs
	Specs     []*resources.Spec
	Langs     []string
	PublishFs afero.Fs
	Log       *bytes.Buffer
	TmpDir    string
}

// Flags returns the CLI-like flags the oracles load a site with: a fresh
// cache dir and resource dir under tmp (cold file caches).
func Flags(dir, tmp string) config.Provider {
	flags := config.New()
	flags.Set("workingDir", dir)
	flags.Set("noBuildLock", true)
	flags.Set("cacheDir", filepath.Join(tmp, "cache"))
	flags.Set("resourceDir", filepath.Join(tmp, "resources"))
	return flags
}

// LoadConfigs loads the site config the way the oracles do.
func LoadConfigs(dir string, flags config.Provider, logBuf *bytes.Buffer) (*allconfig.Configs, loggers.Logger, error) {
	logger := loggers.New(loggers.Options{StdOut: logBuf, StdErr: logBuf, Level: logg.LevelWarn, DistinctLevel: logg.LevelWarn})
	res, err := allconfig.LoadConfig(allconfig.ConfigSourceDescriptor{
		Flags: flags, Fs: hugofs.Os, Filename: filepath.Join(dir, "hugo.toml"), Logger: logger,
		Environ: []string{"NEOHUGO_ORACLE=1"},
	})
	return res, logger, err
}

// LoadSite loads the site in dir and creates its resource specs (without
// hugolib): the publish dir is in memory, the resource and cache dirs are
// fresh temporary dirs.
func LoadSite(dir string) (*Site, error) {
	tmp, err := os.MkdirTemp("", "nh-resources-oracle")
	if err != nil {
		return nil, err
	}
	var logBuf bytes.Buffer
	res, logger, err := LoadConfigs(dir, Flags(dir, tmp), &logBuf)
	if err != nil {
		return nil, fmt.Errorf("load config: %w", err)
	}
	fsCfg := config.New()
	fsCfg.Set("workingDir", dir)
	fsCfg.Set("publishDir", res.LoadingInfo.BaseConfig.PublishDir)
	pub := afero.NewMemMapFs()
	hfs := hugofs.NewFromSourceAndDestination(hugofs.Os, pub, fsCfg)
	memCache := dynacache.New(dynacache.Options{Log: logger})

	s := &Site{Dir: dir, Configs: res, PublishFs: pub, Log: &logBuf, TmpDir: tmp}
	var (
		common     *resources.SpecCommon
		firstPS    *helpers.PathSpec
		fileCaches filecache.Caches
	)
	for _, conf := range res.ConfigLangs() {
		var ps *helpers.PathSpec
		if firstPS == nil {
			ps, err = helpers.NewPathSpec(hfs, conf, logger)
			if err != nil {
				return nil, err
			}
			firstPS = ps
			fileCaches, err = filecache.NewCaches(ps)
			if err != nil {
				return nil, err
			}
		} else {
			ps, err = helpers.NewPathSpecWithBaseBaseFsProvided(hfs, conf, logger, firstPS.BaseFs)
			if err != nil {
				return nil, err
			}
		}
		spec, err := resources.NewSpec(ps, common, fileCaches, memCache, &identity.IncrementByOne{}, logger, nil, nil, nil, nil)
		if err != nil {
			return nil, err
		}
		common = spec.SpecCommon
		s.Specs = append(s.Specs, spec)
		s.Langs = append(s.Langs, conf.Language().Lang)
	}
	return s, nil
}

// Close removes the temporary dirs.
func (s *Site) Close() {
	_ = os.RemoveAll(s.TmpDir)
}

// Rel replaces the absolute repository root in s with "$ROOT".
func Rel(root, s string) string {
	return strings.ReplaceAll(s, root, "$ROOT")
}

// WriteGz writes v as gzipped JSON (no timestamps in the header, so the
// output is reproducible).
func WriteGz(path string, v any) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	var b bytes.Buffer
	enc := json.NewEncoder(&b)
	enc.SetEscapeHTML(false)
	enc.SetIndent("", " ")
	if err := enc.Encode(v); err != nil {
		return err
	}
	var out bytes.Buffer
	zw, err := gzip.NewWriterLevel(&out, gzip.BestCompression)
	if err != nil {
		return err
	}
	zw.ModTime = time.Time{}
	if _, err := zw.Write(b.Bytes()); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	return os.WriteFile(path, out.Bytes(), 0o644)
}

// Sha returns the hex sha256 of b.
func Sha(b []byte) string {
	h := sha256.Sum256(b)
	return hex.EncodeToString(h[:])
}

// Enc encodes a Go value with its type for the Rust tests (which rebuild the
// go_value with the same kinds): {"t": kind, "v": value}.
func Enc(v any) any {
	switch vv := v.(type) {
	case nil:
		return map[string]any{"t": "nil"}
	case string:
		return map[string]any{"t": "string", "v": vv}
	case bool:
		return map[string]any{"t": "bool", "v": vv}
	case int:
		return map[string]any{"t": "int", "v": vv}
	case int64:
		return map[string]any{"t": "int64", "v": vv}
	case uint64:
		return map[string]any{"t": "uint64", "v": fmt.Sprint(vv)}
	case float64:
		return map[string]any{"t": "float64", "v": fmt.Sprintf("%v", vv)}
	case []string:
		out := []any{}
		for _, x := range vv {
			out = append(out, x)
		}
		return map[string]any{"t": "[]string", "v": out}
	case []any:
		out := []any{}
		for _, x := range vv {
			out = append(out, Enc(x))
		}
		return map[string]any{"t": "[]any", "v": out}
	case maps.Params:
		return map[string]any{"t": "maps.Params", "v": encMap(vv)}
	case map[string]any:
		if vv == nil {
			return map[string]any{"t": "nilmap"}
		}
		return map[string]any{"t": "map", "v": encMap(vv)}
	default:
		return map[string]any{"t": fmt.Sprintf("%T", v), "v": fmt.Sprint(v)}
	}
}

func encMap(m map[string]any) []any {
	var keys []string
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	out := []any{}
	for _, k := range keys {
		out = append(out, []any{k, Enc(m[k])})
	}
	return out
}

// JSON returns json.Marshal(v) as a string (the error text on failure).
func JSON(v any) string {
	b, err := json.Marshal(v)
	if err != nil {
		return "ERR: " + err.Error()
	}
	return string(b)
}

// Field returns the (possibly unexported) field name of the struct v points
// to or is, readable.
func Field(v reflect.Value, name string) reflect.Value {
	for v.Kind() == reflect.Ptr || v.Kind() == reflect.Interface {
		v = v.Elem()
	}
	f := v.FieldByName(name)
	if !f.IsValid() {
		panic(fmt.Sprintf("no field %s in %s", name, v.Type()))
	}
	if f.CanAddr() {
		return reflect.NewAt(f.Type(), unsafe.Pointer(f.UnsafeAddr())).Elem()
	}
	return f
}

// Descriptor returns the ResourceSourceDescriptor (after init) behind a
// resource created by resources.Spec.NewResource (a *resourceAdapter over a
// *genericResource or *imageResource), and the Go type of its target.
func Descriptor(r resource.Resource) (resources.ResourceSourceDescriptor, string) {
	inner := Field(reflect.ValueOf(r), "resourceAdapterInner")
	target := Field(inner, "target")
	tt := target.Elem().Type().String()
	gr := target
	if tt == "*resources.imageResource" {
		gr = Field(target, "baseResource")
	}
	sd := Field(gr, "sd")
	return sd.Interface().(resources.ResourceSourceDescriptor), tt
}

// GenericPtr returns the address of the *genericResource behind a resource
// adapter (its identity: metadata clones share it).
func GenericPtr(r resource.Resource) uintptr {
	inner := Field(reflect.ValueOf(r), "resourceAdapterInner")
	target := Field(inner, "target")
	if target.Elem().Type().String() == "*resources.imageResource" {
		return Field(target, "baseResource").Elem().Pointer()
	}
	return target.Elem().Pointer()
}

// Arch is GOARCH (fixtures record the platform they were made on).
func Arch() string {
	return runtime.GOARCH
}

// PublishedFiles lists the files of the publish fs (the destination fs the
// publish dir lives in) with their sizes and sha256, relative to the publish
// dir.
func (s *Site) PublishedFiles() map[string]any {
	fs := s.PublishFs
	prefix := filepath.Join(s.Dir, s.Configs.LoadingInfo.BaseConfig.PublishDir) + string(filepath.Separator)
	if filepath.IsAbs(s.Configs.LoadingInfo.BaseConfig.PublishDir) {
		prefix = s.Configs.LoadingInfo.BaseConfig.PublishDir + string(filepath.Separator)
	}
	out := map[string]any{}
	_ = afero.Walk(fs, "/", func(path string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}
		b, err := afero.ReadFile(fs, path)
		if err != nil {
			return nil
		}
		out[filepath.ToSlash(strings.TrimPrefix(path, prefix))] = fmt.Sprintf("%d:%s", len(b), Sha(b))
		return nil
	})
	return out
}

// Rec records the observable attributes of a resource: the methods that do
// not publish first, then Content, then the links (which publish).
func Rec(r resource.Resource, withContent bool) map[string]any {
	o := map[string]any{}
	o["name"] = r.Name()
	o["title"] = r.Title()
	if k, ok := r.(resource.Identifier); ok {
		o["key"] = k.Key()
	}
	if n, ok := r.(resource.NameNormalizedProvider); ok {
		o["nameNormalized"] = n.NameNormalized()
	}
	mt := r.MediaType()
	o["mediaType"] = mt.Type
	o["mediaTypeJSON"] = JSON(mt)
	o["resourceType"] = r.ResourceType()
	o["data"] = JSON(r.Data())
	o["params"] = JSON(r.Params())
	if withContent {
		if c, ok := r.(resource.ContentProvider); ok {
			v, err := c.Content(context.Background())
			if err != nil {
				o["contentErr"] = err.Error()
			} else {
				s := v.(string)
				if mt.IsText() || mt.MainType == "text" || mt.SubType == "json" {
					o["content"] = s
				} else {
					o["contentSha"] = Sha([]byte(s))
				}
				o["contentLen"] = len(s)
			}
		}
	}
	type wh interface {
		Width() int
		Height() int
	}
	if img, ok := r.(wh); ok && r.ResourceType() == "image" {
		func() {
			defer func() {
				if e := recover(); e != nil {
					o["whPanic"] = fmt.Sprint(e)
				}
			}()
			o["width"] = img.Width()
			o["height"] = img.Height()
		}()
	}
	o["relPermalink"] = r.RelPermalink()
	o["permalink"] = r.Permalink()
	return o
}

// OpenFile returns an opener of the OS file filename.
func OpenFile(filename string) hugio.OpenReadSeekCloser {
	return func() (hugio.ReadSeekCloser, error) {
		return os.Open(filename)
	}
}

// ReadAll reads a ReadSeekCloser provider fully.
func ReadAll(r resource.Resource) ([]byte, error) {
	rc, ok := r.(resource.ReadSeekCloserResource)
	if !ok {
		return nil, fmt.Errorf("%T is not a ReadSeekCloserResource", r)
	}
	f, err := rc.ReadSeekCloser()
	if err != nil {
		return nil, err
	}
	defer func() { _ = f.Close() }()
	return io.ReadAll(f)
}
