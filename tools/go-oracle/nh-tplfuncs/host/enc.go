package main

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"html/template"
	"image"
	"math"
	"net/url"
	"os"
	"reflect"
	"sort"
	"strings"
	"time"
	"unicode/utf8"

	"github.com/disintegration/gift"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/neohugo"
	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/tpl"
	"github.com/spf13/afero"
)

// str returns s as a JSON-able value: the string itself when it is valid
// UTF-8, else {"hex": ...}.
func str(s string) any {
	if utf8.ValidString(s) {
		return s
	}
	return map[string]string{"hex": fmt.Sprintf("%x", s)}
}

func fbits(f float64) string { return fmt.Sprintf("%016x", math.Float64bits(f)) }

// encoder encodes values in the typed JSON format of the nh-tplfuncs data
// oracle (goval's format), extended with the host types: pages and sites (by
// path and language), resources (name, link, media type, published bytes),
// and the result types of the host namespaces.
type encoder struct {
	// publish is the publish filesystem of the site whose resources are
	// encoded (their published bytes are hashed).
	publish afero.Fs
	// dir is the site directory, replaced by /SITE in the published bytes
	// before they are hashed.
	dir string
}

func sha(b []byte) string {
	h := sha256.Sum256(b)
	return hex.EncodeToString(h[:])
}

func (e *encoder) enc(v any) any {
	switch x := v.(type) {
	case nil:
		return map[string]any{"t": "nil"}
	case page.Site:
		if reflect.ValueOf(x).Kind() == reflect.Ptr && reflect.ValueOf(x).IsNil() {
			return map[string]any{"t": "nil:" + reflect.TypeOf(x).String()}
		}
		return map[string]any{"t": fmt.Sprintf("%T", x), "site": x.Language().Lang, "title": str(x.Title())}
	case page.Page:
		if reflect.ValueOf(x).Kind() == reflect.Ptr && reflect.ValueOf(x).IsNil() {
			return map[string]any{"t": "nil:" + reflect.TypeOf(x).String()}
		}
		return map[string]any{"t": fmt.Sprintf("%T", x), "page": str(x.Path()), "lang": x.Lang()}
	case resource.Resource:
		if reflect.ValueOf(x).Kind() == reflect.Ptr && reflect.ValueOf(x).IsNil() {
			return map[string]any{"t": "nil:" + reflect.TypeOf(x).String()}
		}
		out := map[string]any{"t": fmt.Sprintf("%T", x), "name": str(x.Name()), "mt": x.MediaType().Type}
		rel := x.RelPermalink()
		out["rel"] = str(rel)
		if e.publish != nil {
			if b, err := afero.ReadFile(e.publish, rel); err == nil {
				out["pub"] = sha(bytes.ReplaceAll(b, []byte(e.dir), []byte("/SITE")))
			}
		}
		if cp, ok := x.(resource.ContentProvider); ok && !strings.HasPrefix(x.MediaType().Type, "image/") {
			c, err := cp.Content(context.Background())
			if err != nil {
				out["contentErr"] = str(err.Error())
			} else {
				out["content"] = str(fmt.Sprint(c))
			}
		}
		if ir, ok := x.(interface {
			Width() int
			Height() int
		}); ok && strings.HasPrefix(x.MediaType().Type, "image/") {
			out["w"] = ir.Width()
			out["h"] = ir.Height()
		}
		return out
	case neohugo.HugoInfo:
		return map[string]any{
			"t": "neohugo.HugoInfo", "env": x.Environment, "prod": x.IsProduction(), "dev": x.IsDevelopment(),
			"server": x.IsServer(), "multilingual": x.IsMultilingual(), "multihost": x.IsMultihost(),
		}
	case paths.DirFile:
		return map[string]any{"t": "paths.DirFile", "dir": str(x.Dir), "file": str(x.File)}
	case *url.URL:
		if x == nil {
			return map[string]any{"t": "nil:*url.URL"}
		}
		return map[string]any{
			"t": "*url.URL", "s": str(x.String()), "scheme": str(x.Scheme), "opaque": str(x.Opaque),
			"host": str(x.Host), "path": str(x.Path), "rawPath": str(x.RawPath), "rawQuery": str(x.RawQuery),
			"fragment": str(x.Fragment), "abs": x.IsAbs(), "user": x.User != nil,
		}
	case image.Config:
		return map[string]any{"t": "image.Config", "w": x.Width, "h": x.Height}
	case []os.FileInfo:
		items := []any{}
		for _, fi := range x {
			items = append(items, encFileInfo(fi))
		}
		return map[string]any{"t": "fileinfos", "items": items}
	case os.FileInfo:
		out := encFileInfo(x)
		out["t"] = "fileinfo"
		return out
	case *tpl.CurrentTemplateInfo:
		if x == nil {
			return map[string]any{"t": "nil:*tpl.CurrentTemplateInfo"}
		}
		return map[string]any{"t": "*tpl.CurrentTemplateInfo", "name": x.Name(), "filename": x.Filename(), "level": x.Level}
	case gift.Filter:
		return encFilter(e, x)
	case bool:
		return map[string]any{"t": "bool", "v": x}
	case int, int8, int16, int32, int64, uint, uint8, uint16, uint32, uint64, uintptr:
		return map[string]any{"t": reflect.TypeOf(v).String(), "v": fmt.Sprint(v)}
	case float64:
		return map[string]any{"t": "float64", "v": fbits(x)}
	case float32:
		return map[string]any{"t": "float32", "v": fbits(float64(x))}
	case string:
		return map[string]any{"t": "string", "s": str(x)}
	case template.HTML, template.URL, template.JS, template.CSS, template.HTMLAttr, template.JSStr, template.Srcset:
		return map[string]any{"t": reflect.TypeOf(v).String(), "s": str(reflect.ValueOf(v).String())}
	case time.Time:
		abbr, off := x.Zone()
		return map[string]any{"t": "time.Time", "unix": x.Unix(), "nsec": x.Nanosecond(), "loc": x.Location().String(), "abbr": abbr, "off": off}
	case maps.Params:
		if x == nil {
			return map[string]any{"t": "nil:maps.Params"}
		}
		return e.encMap("maps.Params", reflect.ValueOf(v))
	}
	rv := reflect.ValueOf(v)
	t := rv.Type().String()
	switch rv.Kind() {
	case reflect.Slice:
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
		items := []any{}
		for i := 0; i < rv.Len(); i++ {
			items = append(items, e.enc(rv.Index(i).Interface()))
		}
		return map[string]any{"t": t, "items": items}
	case reflect.Map:
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
		return e.encMap(t, rv)
	case reflect.Ptr:
		if rv.IsNil() {
			return map[string]any{"t": "nil:" + t}
		}
	case reflect.Bool, reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64,
		reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64,
		reflect.Float32, reflect.Float64, reflect.String:
		// A named basic type.
		var u any
		switch rv.Kind() {
		case reflect.Bool:
			u = rv.Bool()
		case reflect.Int:
			u = int(rv.Int())
		case reflect.Int8:
			u = int8(rv.Int())
		case reflect.Int16:
			u = int16(rv.Int())
		case reflect.Int32:
			u = int32(rv.Int())
		case reflect.Int64:
			u = rv.Int()
		case reflect.Uint:
			u = uint(rv.Uint())
		case reflect.Uint8:
			u = uint8(rv.Uint())
		case reflect.Uint16:
			u = uint16(rv.Uint())
		case reflect.Uint32:
			u = uint32(rv.Uint())
		case reflect.Uint64:
			u = rv.Uint()
		case reflect.Float32:
			u = float32(rv.Float())
		case reflect.Float64:
			u = rv.Float()
		case reflect.String:
			u = rv.String()
		}
		return map[string]any{"t": "named", "name": t, "under": e.enc(u)}
	}
	// Anything else: its type and printed form.
	return map[string]any{"t": "other:" + t, "s": str(fmt.Sprint(v))}
}

// encFileInfo encodes the name, the kind and (for a file) the size of a
// FileInfo: a directory's size depends on the filesystem.
func encFileInfo(fi os.FileInfo) map[string]any {
	out := map[string]any{"name": str(fi.Name()), "dir": fi.IsDir()}
	if !fi.IsDir() {
		out["size"] = fi.Size()
	}
	return out
}

// encFilter encodes an images.filter: its options (Vals) and the type of the
// gift filter it embeds.
func encFilter(e *encoder, f gift.Filter) any {
	rv := reflect.ValueOf(f)
	out := map[string]any{"t": fmt.Sprintf("%T", f)}
	if rv.Kind() == reflect.Struct {
		opts := rv.FieldByName("Options")
		inner := rv.FieldByName("Filter")
		if opts.IsValid() {
			out["vals"] = e.enc(opts.FieldByName("Vals").Interface())
		}
		if inner.IsValid() && !inner.IsNil() {
			// The gift filter type without package and pointer (what the Rust
			// filter records for hashing).
			t := strings.TrimPrefix(fmt.Sprintf("%T", inner.Interface()), "*")
			out["f"] = t[strings.LastIndex(t, ".")+1:]
		}
	}
	return out
}

func (e *encoder) encMap(t string, rv reflect.Value) any {
	if rv.Type().Key().Kind() != reflect.String {
		return map[string]any{"t": "other:" + t, "s": str(fmt.Sprint(rv.Interface()))}
	}
	var keys []string
	for _, k := range rv.MapKeys() {
		keys = append(keys, k.String())
	}
	sort.Strings(keys)
	entries := []any{}
	for _, k := range keys {
		entries = append(entries, []any{str(k), e.enc(rv.MapIndex(reflect.ValueOf(k).Convert(rv.Type().Key())).Interface())})
	}
	return map[string]any{"t": t, "entries": entries}
}
