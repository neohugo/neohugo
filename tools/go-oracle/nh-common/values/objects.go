package main

import (
	"fmt"
	"html/template"
	"math"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/types/hstring"
)

// pgI stands in for page.Page: an interface the pages implement.
type pgI interface{ ID() string }

// pg stands in for *hugolib.pageState: a collections.Slicer whose Slice
// returns the named slice pgs (like page.Pages).
type pg struct{ id string }

func (p *pg) GoType() string { return "*main.pg" }
func (p *pg) ID() string     { return p.id }

// Slice mirrors page.ToPages: a []interface{} of pages becomes pgs.
func (p *pg) Slice(items any) (any, error) {
	switch v := items.(type) {
	case []any:
		out := make(pgs, len(v))
		for i, x := range v {
			pp, ok := x.(pgI)
			if !ok {
				return nil, fmt.Errorf("type %T is not a page", x)
			}
			out[i] = pp
		}
		return out, nil
	}
	return nil, fmt.Errorf("cannot convert type %T to pgs", items)
}

// pgs stands in for page.Pages.
type pgs []pgI

var (
	p1 = &pg{"p1"}
	p2 = &pg{"p2"}
	p3 = &pg{"p3"}
)

// values is a table of values of every kind the value model has.
func values() []any {
	var nilStrings []string
	var nilAny []any
	var nilMap map[string]any
	var nilParams maps.Params
	var nilPg *pg
	return []any{
		nil, true, false, 0, 1, -1, math.MaxInt64, math.MinInt64, int8(-3), int16(300), int32(-70000), int64(0), int64(42),
		uint(0), uint(5), uint8(255), uint16(1), uint32(4000000000), uint64(math.MaxUint64), uintptr(9),
		0.0, 1.5, -2.25, math.Copysign(0, -1), math.NaN(), math.Inf(1), math.Inf(-1), 1e300, float32(0.1), float32(0),
		"", "a", "abc", "0", "1", "false", "true", " ", "200", "200ms", "4m", "1h30m", "-5s", "x y", "a,b",
		template.HTML(""), template.HTML("<b>x</b>"), template.CSS("a{}"), template.JS("x"), template.URL("/u"),
		template.HTMLAttr("a=b"), template.JSStr("s"), template.Srcset("x 1x"),
		hstring.HTML(""), hstring.HTML("<i>h</i>"), maps.ParamsMergeStrategyDeep, time.Duration(1500),
		time.Month(3), time.Weekday(2),
		time.Time{}, time.Date(2020, 1, 2, 3, 4, 5, 6, time.UTC), time.Unix(0, 0).UTC(),
		[]any{}, []any{"a", 1}, []any{nil}, nilAny, []string{}, []string{"a", "b"}, nilStrings, []int{1, 2}, []int{},
		[]int64{7}, []float64{1.5}, []bool{true}, []byte("ab"), []byte{}, []map[string]any{{"k": 1}}, [][]string{{"a"}},
		map[string]any{}, map[string]any{"k": "v"}, nilMap, map[string]string{"a": "b"}, maps.Params{},
		maps.Params{"_merge": maps.ParamsMergeStrategyNone}, maps.Params{"a": 1}, nilParams,
		p1, nilPg, pgs{p1, p2}, pgs{},
	}
}
