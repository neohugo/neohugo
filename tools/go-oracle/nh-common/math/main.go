// Command math is the Go oracle for crates/nh-common/src/math.rs, the port of
// neohugo's common/math.DoArithmetic (used by add/sub/mul/div and
// Scratch.Add).
//
//	go run ./tools/go-oracle/nh-common/math [-out crates/nh-common/tests/fixtures/math/math.json]
//
// It runs DoArithmetic for every ordered pair of a table of operands (every
// int, uint and float kind with edge values, strings, the html/template
// string types, named basic types, bool, nil, time.Time, slices and maps) and
// every operator '+', '-', '*', '/', '%' and '^'. Results are typed JSON (see
// ../goval).
//
// Float results are platform sensitive in principle, so the checked-in
// fixture comes from an arm64 build (GOARCH=arm64, run under
// qemu-aarch64-static); DoArithmetic has no fused multiply-add sites.
package main

import (
	"flag"
	"html/template"
	"log"
	"math"
	"runtime"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	hmath "github.com/neohugo/neohugo/common/math"
	"github.com/neohugo/neohugo/common/types/hstring"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

func operands() []any {
	return []any{
		nil, true, "", "foo", "12", template.HTML("<b>"), template.URL("/u"), hstring.HTML("h"),
		0, 1, -1, 3, -3, 7, math.MaxInt64, math.MinInt64, int8(-128), int8(127), int16(-2), int32(2147483647), int64(-7), int64(1 << 53),
		uint(0), uint(2), uint(3), uint8(255), uint16(65535), uint32(4294967295), uint64(math.MaxUint64), uint64(1 << 63), uintptr(3),
		0.0, math.Copysign(0, -1), 0.5, 1.5, -2.25, 3.0, 1e308, -1e308, 5e-324, math.Inf(1), math.Inf(-1), math.NaN(),
		0.1, 0.2, 1.0 / 3, 9007199254740993.0, float32(0.1), float32(-3.5), float32(math.MaxFloat32),
		time.Duration(1500), time.Month(3), time.Time{}, []int{1}, map[string]any{"a": 1}, maps.Params{},
	}
}

func main() {
	out := flag.String("out", "crates/nh-common/tests/fixtures/math/math.json.gz", "output file (gzip)")
	flag.Parse()

	ops := []rune{'+', '-', '*', '/', '%', '^'}
	vals := operands()
	var enc []any
	for _, v := range vals {
		enc = append(enc, goval.Encode(v))
	}
	var cases []map[string]any
	for i, a := range vals {
		for j, b := range vals {
			var rs []any
			for _, op := range ops {
				rs = append(rs, goval.Call(func() (any, error) { return hmath.DoArithmetic(a, b, op) }))
			}
			cases = append(cases, map[string]any{"a": i, "b": j, "r": rs})
		}
	}
	header := map[string]any{"goarch": runtime.GOARCH, "operands": enc, "ops": string(ops)}
	if err := goval.WriteCasesGz(*out, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d cases", *out, len(cases))
}
