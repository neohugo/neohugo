// Package tsupport holds what the nh-tplimpl oracles share: the overlay that
// adds recording hooks and dump functions to tpl/tplimpl, hugolib and the
// html/template fork (go run -overlay), the sites they build (this
// repository's docs/ and hugolib/testsite sites and synthetic layout trees),
// in-memory builds, and the fixture encoding.
package tsupport

import (
	"sync"

	"github.com/neohugo/neohugo/hugolib"
	"github.com/neohugo/neohugo/tpl/tplimpl"
)

// API is the part of the oracles that needs the overlay. It is set by the
// overlay-added file of this package (overlay/tsupport.go.txt), so the
// oracles compile without the overlay (go vet, CI).
type API struct {
	// StoreDump dumps the store internals (templates, trees, maps, walk).
	StoreDump func(s *tplimpl.TemplateStore) (map[string]any, error)
	// Namespaces dumps every tree of the store's template namespaces.
	Namespaces func(s *tplimpl.TemplateStore) []any
	// SiteConfig dumps the configuration the store depends on.
	SiteConfig func(h *hugolib.HugoSites) map[string]any
	// InstallLookupHooks starts (r != nil) or stops recording the lookups.
	InstallLookupHooks func(r *LookupRecorder)
	// SetSkipWrite switches renderAndWritePage off.
	SetSkipWrite func(bool)
	// NewStore creates a store for the site with the given template funcs.
	NewStore func(h *hugolib.HugoSites, funcs map[string]any) (*tplimpl.TemplateStore, error)
	// TemplID identifies a template of the store.
	TemplID func(s *tplimpl.TemplateStore, ti *tplimpl.TemplInfo) string
	// SetReverseOrder reverses the (pinned) order the lookups iterate
	// their candidates in.
	SetReverseOrder func(bool)
}

// Oracle is set by the overlay.
var Oracle *API

// DescDump encodes a template descriptor.
func DescDump(d tplimpl.TemplateDescriptor) map[string]any {
	return map[string]any{
		"kind":                    d.Kind,
		"layoutFromTemplate":      d.LayoutFromTemplate,
		"layoutFromUser":          d.LayoutFromUser,
		"outputFormat":            d.OutputFormat,
		"mediaType":               d.MediaType,
		"lang":                    d.Lang,
		"variant1":                d.Variant1,
		"variant2":                d.Variant2,
		"layoutFromUserMustMatch": d.LayoutFromUserMustMatch,
		"isPlainText":             d.IsPlainText,
		"alwaysAllowPlainText":    d.AlwaysAllowPlainText,
	}
}

// QueryDump encodes a template query (without its Consider func).
func QueryDump(q tplimpl.TemplateQuery) map[string]any {
	return map[string]any{
		"path":     q.Path,
		"name":     q.Name,
		"category": q.Category.String(),
		"desc":     DescDump(q.Desc),
	}
}

var categories = map[string]tplimpl.Category{}

func init() {
	for _, c := range []tplimpl.Category{tplimpl.CategoryLayout, tplimpl.CategoryBaseof, tplimpl.CategoryMarkup, tplimpl.CategoryShortcode, tplimpl.CategoryPartial, tplimpl.CategoryServer, tplimpl.CategoryHugo} {
		categories[c.String()] = c
	}
}

// QueryFromDump decodes QueryDump (without a Consider func).
func QueryFromDump(m map[string]any) tplimpl.TemplateQuery {
	d := m["desc"].(map[string]any)
	return tplimpl.TemplateQuery{
		Path:     m["path"].(string),
		Name:     m["name"].(string),
		Category: categories[m["category"].(string)],
		Desc: tplimpl.TemplateDescriptor{
			Kind:                    d["kind"].(string),
			LayoutFromTemplate:      d["layoutFromTemplate"].(string),
			LayoutFromUser:          d["layoutFromUser"].(string),
			OutputFormat:            d["outputFormat"].(string),
			MediaType:               d["mediaType"].(string),
			Lang:                    d["lang"].(string),
			Variant1:                d["variant1"].(string),
			Variant2:                d["variant2"].(string),
			LayoutFromUserMustMatch: d["layoutFromUserMustMatch"].(bool),
			IsPlainText:             d["isPlainText"].(bool),
			AlwaysAllowPlainText:    d["alwaysAllowPlainText"].(bool),
		},
	}
}

// LookupRecorder collects the lookups the store answers during a build.
type LookupRecorder struct {
	mu      sync.Mutex
	Records []map[string]any
	seen    map[string]bool
}

// NewLookupRecorder returns an empty recorder.
func NewLookupRecorder() *LookupRecorder {
	return &LookupRecorder{seen: map[string]bool{}}
}

// Add records one lookup (identical records are kept once).
func (r *LookupRecorder) Add(rec map[string]any) {
	key := mustJSON(rec)
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.seen[key] {
		return
	}
	r.seen[key] = true
	r.Records = append(r.Records, rec)
}
