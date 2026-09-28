package main

import (
	"sort"
	"sync"

	"github.com/neohugo/neohugo/common/paths"
)

// recorder wraps a PathParser's callbacks and records every distinct call (as
// the nh-common paths oracle does), so the Rust test can replay them.
type recorder struct {
	mu sync.Mutex
	of map[[2]string]bool // IsOutputFormat(name, ext)
	ce map[string]bool    // IsContentExt(ext)
	ld map[string]bool    // IsLangDisabled(lang)
}

func newRecorder() *recorder {
	return &recorder{of: map[[2]string]bool{}, ce: map[string]bool{}, ld: map[string]bool{}}
}

func (r *recorder) wrap(pp *paths.PathParser) *paths.PathParser {
	c := *pp
	if pp.IsOutputFormat != nil {
		f := pp.IsOutputFormat
		c.IsOutputFormat = func(name, ext string) bool {
			v := f(name, ext)
			r.mu.Lock()
			r.of[[2]string{name, ext}] = v
			r.mu.Unlock()
			return v
		}
	}
	if pp.IsContentExt != nil {
		f := pp.IsContentExt
		c.IsContentExt = func(ext string) bool {
			v := f(ext)
			r.mu.Lock()
			r.ce[ext] = v
			r.mu.Unlock()
			return v
		}
	}
	if pp.IsLangDisabled != nil {
		f := pp.IsLangDisabled
		c.IsLangDisabled = func(lang string) bool {
			v := f(lang)
			r.mu.Lock()
			r.ld[lang] = v
			r.mu.Unlock()
			return v
		}
	}
	return &c
}

func sortedLog(m map[string]bool) [][]any {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	out := [][]any{}
	for _, k := range keys {
		out = append(out, []any{k, m[k]})
	}
	return out
}

// describe returns the parser's language index, which callbacks are nil and
// the recorded answers.
func (r *recorder) describe(pp *paths.PathParser) map[string]any {
	var ofk [][2]string
	for k := range r.of {
		ofk = append(ofk, k)
	}
	sort.Slice(ofk, func(i, j int) bool {
		if ofk[i][0] != ofk[j][0] {
			return ofk[i][0] < ofk[j][0]
		}
		return ofk[i][1] < ofk[j][1]
	})
	ofl := [][]any{}
	for _, k := range ofk {
		ofl = append(ofl, []any{k[0], k[1], r.of[k]})
	}
	return map[string]any{
		"languageIndex":   pp.LanguageIndex,
		"isLangDisabled":  pp.IsLangDisabled != nil,
		"isOutputFormat":  pp.IsOutputFormat != nil,
		"isContentExt":    pp.IsContentExt != nil,
		"outputFormatLog": ofl,
		"contentExtLog":   sortedLog(r.ce),
		"langDisabledLog": sortedLog(r.ld),
	}
}
