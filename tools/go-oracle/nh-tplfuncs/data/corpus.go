package main

import (
	"encoding/json"
	"errors"
	"html/template"
	"math"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/neohugo"
	"github.com/neohugo/neohugo/common/types/hstring"
)

// tstObj is a test struct (like the TstX of the Go tests): exported fields
// and methods with every result shape evaluateSubElem distinguishes.
type tstObj struct {
	ID string
	A  string
	B  int
	P  maps.Params
}

// GetA returns the A field.
func (t *tstObj) GetA() string { return t.A }

// Double returns 2*B.
func (t *tstObj) Double() int { return 2 * t.B }

// Fail returns an error.
func (t *tstObj) Fail() (string, error) { return "", errors.New("boom") }

// WithArg needs an argument.
func (t *tstObj) WithArg(s string) string { return s + t.A }

var (
	obj1 = &tstObj{ID: "o1", A: "x", B: 1, P: maps.Params{"k": "v1"}}
	obj2 = &tstObj{ID: "o2", A: "y", B: 2, P: maps.Params{"k": "v2"}}
	obj3 = &tstObj{ID: "o3", A: "x", B: 3}
)

var (
	t1   = time.Date(2021, 3, 4, 5, 6, 7, 8, time.UTC)
	t2   = time.Date(2021, 3, 4, 12, 6, 7, 8, time.FixedZone("UTC+7", 7*3600))
	t3   = time.Date(1999, 12, 31, 23, 59, 59, 0, time.UTC)
	zone = time.Time{}
)

// scalars is the value corpus crossed with every unary and binary function.
func scalars() []any {
	return []any{
		nil, true, false,
		0, 1, -2, 42, int8(-8), int16(16), int32(-32), int64(64), int64(math.MaxInt64), int64(math.MinInt64),
		uint(1), uint8(8), uint16(16), uint32(32), uint64(64), uint64(math.MaxUint64), uintptr(7),
		float32(1.5), float32(0.1), 0.0, 1.0, 2.5, -3.75, 1e300, math.Inf(1), math.Inf(-1), math.Copysign(0, -1), math.NaN(),
		"", "a", "b", "abc", "ABC", "10", "2", "1e3", "-4.5", "0x1f", "Inf", "NaN", "1e999", "true", " 3", "ข้าว", "ขนม", "é", "a b&c<d>",
		template.HTML("a"), template.HTML("<b>x</b>"), template.CSS("a"), template.JS("a"), template.JSStr("a"), template.URL("a"), template.HTMLAttr("a"),
		hstring.HTML("a"), json.Number("12"), json.Number("1.5"), neohugo.VersionString("0.105.0"), neohugo.VersionString("0.99"),
		t1, t2, t3, zone,
		[]any{}, []any{1, "a", nil}, []any{"a", "b", "c"}, []any{1, 2, 3}, []any{1.0, 2, int64(3)}, []any{"a", 1},
		[]string{}, []string{"a", "b", "c"}, []string{"b", "a", "b"}, []int{1, 2, 3}, []int{3, 1, 2, 3}, []int64{1, 2}, []float64{1.5, 2}, []bool{true},
		map[string]any{}, map[string]any{"a": 1, "b": "x"}, maps.Params{"a": 1, "b": maps.Params{"c": "d"}}, map[string]string{"a": "b"}, map[string]int{"a": 1, "b": 2},
		[]string(nil), map[string]any(nil), maps.Params(nil), []any(nil), (*tstObj)(nil),
		obj1,
	}
}

// small is a smaller corpus for the functions of three and more arguments.
func small() []any {
	return []any{
		nil, true, 0, 1, -2, int64(3), uint(2), 1.5, math.NaN(), "", "a", "2", "abc", template.HTML("a"), hstring.HTML("a"),
		t1, []any{1, "a", nil}, []string{"a", "b"}, []int{1, 2}, map[string]any{"a": 1}, maps.Params{"a": 1}, []string(nil), obj1,
	}
}
