// Command data is the Go oracle of the data template namespaces of
// crates/nh-tplfuncs (Wave B task T18): cast, collections, compare, crypto,
// encoding, fmt, hash, math, reflect and safe.
//
//	go run ./tools/go-oracle/nh-tplfuncs/data [-out crates/nh-tplfuncs/tests/fixtures/data]
//
// Every case calls a method of the real Go namespace (tpl/<ns>, built with a
// deps.Deps like Go's tests do) the way text/template calls a function: the
// context is injected, the argument count and typed parameters are checked
// with Go's messages and a panic becomes the error. The arguments come from
// a systematic value corpus (every int/uint/float kind, strings that parse
// as numbers or not, bool, nil, the html/template types, hstring.HTML,
// json.Number, neohugo.VersionString, time.Time, typed and untyped slices
// and maps, typed nils, maps.Params, a test struct with fields and methods,
// and the pages of an in-memory build including the pageWithWeight0 and
// *pageWithOrdinal wrappers), crossed with every function, plus the inputs
// of the Go test tables of these namespaces.
//
// Results are recorded as the typed JSON of the value (its Go type
// included) or the error text. Each case runs three times; a result that
// differs between runs (Go map order, shuffle) is recorded with all the
// results seen ("nondet").
//
// Floating point results depend on the platform (FMA): the fixtures are
// produced by a linux/arm64 build run under qemu-aarch64-static (the golden
// build ran on darwin/arm64):
//
//	GOOS=linux GOARCH=arm64 go build -o /tmp/data.arm64 ./tools/go-oracle/nh-tplfuncs/data
//	qemu-aarch64-static /tmp/data.arm64 -out crates/nh-tplfuncs/tests/fixtures/data
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
	"sort"
	"strings"

	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/testconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/langs"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tpl/cast"
	"github.com/neohugo/neohugo/tpl/collections"
	"github.com/neohugo/neohugo/tpl/compare"
	"github.com/neohugo/neohugo/tpl/crypto"
	"github.com/neohugo/neohugo/tpl/encoding"
	tplfmt "github.com/neohugo/neohugo/tpl/fmt"
	"github.com/neohugo/neohugo/tpl/hash"
	tplmath "github.com/neohugo/neohugo/tpl/math"
	tplreflect "github.com/neohugo/neohugo/tpl/reflect"
	"github.com/neohugo/neohugo/tpl/safe"
)

// oracle holds the namespaces, the value table and the cases of one run.
type oracle struct {
	ctx     context.Context
	ns      map[string]any
	enc     *encoder
	values  []any
	valueIx map[string]int
	cases   map[string][]map[string]any
	order   []string
}

func (o *oracle) ref(v any) int {
	e := o.enc.enc(v)
	k := key(e)
	if i, ok := o.valueIx[k]; ok {
		return i
	}
	o.values = append(o.values, e)
	o.valueIx[k] = len(o.values) - 1
	return len(o.values) - 1
}

func (o *oracle) result(v any, err error) map[string]any {
	if err != nil {
		return map[string]any{"err": str(err.Error())}
	}
	return map[string]any{"ok": o.enc.enc(v)}
}

// add records one case of topic: namespace ns, method m, arguments args.
func (o *oracle) add(topic, ns, m string, args ...any) {
	recv, ok := o.ns[ns]
	if !ok {
		panic("no namespace " + ns)
	}
	refs := make([]int, len(args))
	for i, a := range args {
		refs[i] = o.ref(a)
	}
	var seen []string
	var first map[string]any
	distinct := map[string]bool{}
	mask := false
	for _, a := range args {
		mask = mask || hasPtr(a)
	}
	runs := 3
	if m == "Sort" && len(args) > 0 && reflect.ValueOf(args[0]).Kind() == reflect.Map {
		// Go ranges the map: ties sort in a random order.
		runs = 50
	}
	for range runs {
		r := o.result(call(o.ctx, recv, m, args))
		if mask {
			maskAddrs(r)
		}
		if first == nil {
			first = r
		}
		k := key(r)
		if !distinct[k] {
			distinct[k] = true
			seen = append(seen, k)
		}
	}
	c := map[string]any{"ns": ns, "m": m, "a": refs, "r": first}
	switch {
	case m == "Rand":
		// Random in Go: the test checks the type and the range.
		c["r"] = map[string]any{"rand": true}
	case m == "Shuffle" && first["ok"] != nil, m == "Sort" && len(seen) > 1 && allOK(seen):
		// Random (shuffle) or map-order dependent (ties of a map sort): the items as a
		// multiset.
		c["r"] = unordered(first)
		c["unordered"] = true
	case len(seen) > 1:
		sort.Strings(seen)
		var all []any
		for _, s := range seen {
			var v any
			if err := json.Unmarshal([]byte(s), &v); err != nil {
				panic(err)
			}
			all = append(all, v)
		}
		c["nondet"] = all
		// The first run's result is random: record the first in sorted order.
		c["r"] = all[0]
	}
	if _, ok := o.cases[topic]; !ok {
		o.order = append(o.order, topic)
	}
	o.cases[topic] = append(o.cases[topic], c)
}

// allOK reports whether every encoded result is a value (not an error).
func allOK(results []string) bool {
	for _, r := range results {
		if !strings.HasPrefix(r, `{"ok"`) {
			return false
		}
	}
	return true
}

// unordered returns the result r of a list with its items sorted by their
// JSON text.
func unordered(r map[string]any) map[string]any {
	ok, _ := r["ok"].(map[string]any)
	items, _ := ok["items"].([]any)
	sorted := make([]any, len(items))
	copy(sorted, items)
	sort.Slice(sorted, func(i, j int) bool { return key(sorted[i]) < key(sorted[j]) })
	out := map[string]any{"t": ok["t"], "items": sorted}
	if items == nil {
		out = ok
	}
	return map[string]any{"ok": out}
}

func mustDeps(lang string) *deps.Deps {
	cfg := config.New()
	cfg.Set("defaultContentLanguage", lang)
	return testconfig.GetTestDeps(nil, cfg)
}

func main() {
	out := flag.String("out", "crates/nh-tplfuncs/tests/fixtures/data", "output directory")
	flag.Parse()

	h, pages, err := buildPages()
	if err != nil {
		log.Fatal(err)
	}

	dEn := mustDeps("en")
	dTh := mustDeps("th")
	if dTh.Conf.Language().Lang != "th" {
		log.Fatalf("th deps: got language %q", dTh.Conf.Language().Lang)
	}

	pt := newPageTable(h.Sites[0].Pages())
	enc := &encoder{pages: pt}
	pt.enc = enc

	o := &oracle{
		ctx: context.Background(),
		ns: map[string]any{
			"cast":             cast.New(),
			"collections":      collections.New(dEn),
			"collections@th":   collections.New(dTh),
			"collections@site": collections.New(h.Sites[0].Deps),
			"compare":          compare.New(langs.GetLocation(dEn.Conf.Language()), false),
			"crypto":           crypto.New(),
			"encoding":         encoding.New(),
			"fmt":              tplfmt.New(dEn),
			"hash":             hash.New(),
			"math":             tplmath.New(dEn),
			"reflect":          tplreflect.New(),
			"safe":             safe.New(),
		},
		enc:     enc,
		valueIx: map[string]int{},
		cases:   map[string][]map[string]any{},
	}

	genScalar(o)
	genCompare(o)
	genMath(o)
	genFmt(o)
	genCollections(o)
	genWhere(o)
	genSort(o)
	genPages(o, pages)
	genTables(o)
	genCounter(o)

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	header := map[string]any{"values": o.values, "pages": pt.entries}
	if err := goval.WriteCasesGz(filepath.Join(*out, "values.json.gz"), header, nil); err != nil {
		log.Fatal(err)
	}
	total := 0
	for _, topic := range o.order {
		cs := o.cases[topic]
		total += len(cs)
		if err := goval.WriteCasesGz(filepath.Join(*out, topic+".json.gz"), map[string]any{"topic": topic}, cs); err != nil {
			log.Fatal(err)
		}
	}
	fmt.Printf("%d values, %d pages, %d cases in %d topics\n", len(o.values), len(pt.entries), total, len(o.order))
}

// counterTopic records the math.Counter sequence of a fresh namespace.
func genCounter(o *oracle) {
	ns := tplmath.New(mustDeps("en"))
	var seq []any
	for range 3 {
		seq = append(seq, o.enc.enc(ns.Counter()))
	}
	o.cases["counter"] = []map[string]any{{"ns": "math", "m": "Counter", "a": []int{}, "r": map[string]any{"seq": seq}}}
	o.order = append(o.order, "counter")
}

// pagesList returns the page lists as []any values.
func pagesAny(ps page.Pages) []any {
	out := make([]any, len(ps))
	for i, p := range ps {
		out[i] = p
	}
	return out
}
