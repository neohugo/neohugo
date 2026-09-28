// Command hashing is the Go oracle for crates/nh-common/src/hashing.rs, the
// port of neohugo's common/hashing over the go-hashstructure crate.
//
//	go run ./tools/go-oracle/nh-common/hashing [-root .] [-out crates/nh-common/tests/fixtures/hashing/hashing.json.gz]
//
// It records, over typed values of every kind (incl. maps.Params with
// ParamsMergeStrategy values, hstring.HTML and Key() providers) and over the
// nh-common string corpus (../corpus: this repository's content titles,
// headings, terms and file names plus adversarial strings):
//   - HashString, HashStringHex and HashUint64 of one value, of pairs and of
//     all values at once, and Hash;
//   - XXHashFromString, XxHashFromStringHexEncoded, MD5FromStringHexEncoded;
//   - XXHashFromReader over a strings.Reader (size = length) and a reader
//     without WriteTo (size = the last Read, 0 at EOF).
//
// The checked-in fixture comes from an arm64 build (GOARCH=arm64, run under
// qemu-aarch64-static), as for every hashing fixture of the port.
package main

import (
	"flag"
	"html/template"
	"io"
	"log"
	"math"
	"runtime"
	"strings"
	"time"

	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/types/hstring"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// keyed is a hashing keyer (a Key() string provider).
type keyed struct{ k string }

func (k keyed) Key() string    { return k.k }
func (k keyed) GoType() string { return "main.keyed" }
func (k keyed) ID() string     { return k.k }

// onlyReader hides strings.Reader's WriteTo.
type onlyReader struct{ r io.Reader }

func (o onlyReader) Read(p []byte) (int, error) { return o.r.Read(p) }

func values() []any {
	var nilMap map[string]any
	var nilStrings []string
	return []any{
		nil, true, false, 0, 1, -1, int8(-8), int16(16), int32(-32), int64(1 << 40), uint(7), uint8(255), uint16(1),
		uint32(1 << 31), uint64(math.MaxUint64), 0.0, 1.5, math.Copysign(0, -1), math.NaN(), float32(2.5), "", "x", "ab",
		template.HTML("<b>"), hstring.HTML("<i>"), maps.ParamsMergeStrategyNone, time.Duration(3),
		time.Date(2024, 2, 29, 13, 14, 15, 123456789, time.UTC), time.Time{},
		[]string{}, []string{"resize", "600x480"}, nilStrings, []any{"a", 1}, []int{1, 2}, []any{},
		map[string]any{"a": 1, "b": "c"}, map[string]any{}, nilMap, map[string]string{"k": "v"},
		maps.Params{"_merge": maps.ParamsMergeStrategyNone, "exif": maps.Params{
			"_merge": maps.ParamsMergeStrategyNone, "disabledate": false, "disablelatlong": false,
			"excludefields": ".*", "includefields": "",
		}},
		maps.Params{"a": []any{maps.Params{"b": 1}}}, keyed{"c"}, []any{"a", "b", keyed{"c"}},
	}
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-common/tests/fixtures/hashing/hashing.json.gz", "output file (gzip)")
	flag.Parse()

	var cases []map[string]any
	vals := values()
	one := func(vs ...any) map[string]any {
		var enc []any
		for _, v := range vs {
			enc = append(enc, goval.Encode(v))
		}
		return map[string]any{
			"op": "hash", "vs": enc,
			"string":    goval.Call(func() (any, error) { return hashing.HashString(vs...), nil }),
			"stringHex": goval.Call(func() (any, error) { return hashing.HashStringHex(vs...), nil }),
			"uint64":    goval.Call(func() (any, error) { return hashing.HashUint64(vs...), nil }),
			"hash":      goval.Call(func() (any, error) { return hashing.Hash(vs...) }),
		}
	}
	cases = append(cases, one())
	for i, v := range vals {
		cases = append(cases, one(v))
		cases = append(cases, one(v, vals[(i+7)%len(vals)]))
	}
	cases = append(cases, one(vals...))

	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}
	for i, s := range strs {
		c := map[string]any{"op": "string", "s": goval.Str(s)}
		c["xxhash"] = goval.Call(func() (any, error) { return hashing.XXHashFromString(s) })
		c["xxhashHex"] = goval.Str(hashing.XxHashFromStringHexEncoded(s))
		c["md5"] = goval.Str(hashing.MD5FromStringHexEncoded(s))
		if i%4 == 0 {
			c["hashString"] = goval.Str(hashing.HashString(s))
			c["reader"] = goval.Call(func() (any, error) {
				h, n, err := hashing.XXHashFromReader(strings.NewReader(s))
				return []any{h, n}, err
			})
			c["onlyReader"] = goval.Call(func() (any, error) {
				h, n, err := hashing.XXHashFromReader(onlyReader{strings.NewReader(s)})
				return []any{h, n}, err
			})
			c["readerHex"] = goval.Call(func() (any, error) {
				return hashing.XxHashFromReaderHexEncoded(strings.NewReader(s))
			})
		}
		cases = append(cases, c)
	}
	// Longer inputs cross the 48 KiB read buffer.
	for _, n := range []int{0, 1, 31, 32, 33, 48*1024 - 1, 48 * 1024, 48*1024 + 1, 200000} {
		s := strings.Repeat("Hello ", n/6+1)[:n]
		cases = append(cases, map[string]any{
			"op": "long", "n": n,
			"xxhash": goval.Call(func() (any, error) { return hashing.XXHashFromString(s) }),
			"reader": goval.Call(func() (any, error) {
				h, size, err := hashing.XXHashFromReader(strings.NewReader(s))
				return []any{h, size}, err
			}),
			"onlyReader": goval.Call(func() (any, error) {
				h, size, err := hashing.XXHashFromReader(onlyReader{strings.NewReader(s)})
				return []any{h, size}, err
			}),
		})
	}

	header := map[string]any{"goarch": runtime.GOARCH}
	if err := goval.WriteCasesGz(*out, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d cases", *out, len(cases))
}
