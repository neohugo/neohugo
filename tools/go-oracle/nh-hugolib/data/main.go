// Command data is the Go oracle of nh-hugolib's `.Site.Data` (T23): for each
// site of DataSites it creates the HugoSites like a build and dumps
// `h.Data()` (loadData / handleDataFile / readData) with the Go type of every
// value (goval's typed JSON: map[string]interface {} nesting by directory,
// int vs int64 vs float64 vs string, []interface {}, time.Time, go-toml local
// dates, nil), and the log (merge warnings, unexpected types, load errors).
//
// Each site is written to a temporary directory and recorded in the fixture
// (repository files as references with their hash), so the Rust test
// recreates it.
//
//	go run ./tools/go-oracle/nh-hugolib/data -root .
package main

import (
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

func main() {
	root := flag.String("root", ".", "the neohugo module root")
	out := flag.String("out", "rust/testdata/oracle/hugolib/data", "output directory (relative to -root)")
	only := flag.String("site", "", "only this site")
	flag.Parse()

	sites, err := DataSites(*root)
	if err != nil {
		log.Fatal(err)
	}

	tmp, err := os.MkdirTemp("", "nh-hugolib-data")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	outDir := *out
	if !filepath.IsAbs(outDir) {
		outDir = filepath.Join(*root, outDir)
	}
	for _, s := range sites {
		if *only != "" && s.Name != *only {
			continue
		}
		b, err := hsupport.New(s, tmp)
		if err != nil {
			log.Fatal(err)
		}
		c := map[string]any{"site": s.Describe()}
		c["data"] = encode(b.H.Data())
		c["log"] = b.LogLines()
		if err := hsupport.WriteJSONGz(filepath.Join(outDir, s.Name+".json.gz"), c); err != nil {
			log.Fatal(err)
		}
		fmt.Printf("%s: ok\n", s.Name)
	}
}

// encode is goval.Encode, except for the go-toml local dates and times
// (encoded as their type and String()), which goval does not know.
func encode(v any) any {
	if v == nil {
		return goval.Encode(v)
	}
	rv := reflect.ValueOf(v)
	t := rv.Type()
	if st, ok := v.(fmt.Stringer); ok && strings.HasPrefix(t.String(), "toml.") {
		return map[string]any{"t": t.String(), "s": st.String()}
	}
	switch rv.Kind() {
	case reflect.Map:
		if rv.IsNil() || t.Key().Kind() != reflect.String {
			return goval.Encode(v)
		}
		var keys []string
		for _, k := range rv.MapKeys() {
			keys = append(keys, k.String())
		}
		sort.Strings(keys)
		entries := []any{}
		for _, k := range keys {
			entries = append(entries, []any{goval.Str(k), encode(rv.MapIndex(reflect.ValueOf(k).Convert(t.Key())).Interface())})
		}
		return map[string]any{"t": t.String(), "entries": entries}
	case reflect.Slice:
		if rv.IsNil() {
			return goval.Encode(v)
		}
		items := []any{}
		for i := 0; i < rv.Len(); i++ {
			items = append(items, encode(rv.Index(i).Interface()))
		}
		return map[string]any{"t": t.String(), "items": items}
	}
	return goval.Encode(v)
}
