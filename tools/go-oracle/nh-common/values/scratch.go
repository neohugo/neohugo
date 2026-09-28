package main

import (
	"html/template"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

type sop struct {
	op   string
	args []any
}

func op(name string, args ...any) sop { return sop{name, args} }

// scratchSequences exercise every Scratch method: Add on every kind (numbers,
// strings, slices of several types, maps, pages, nil), Set/Get/Delete,
// SetInMap/DeleteInMap/GetSortedMapValues and Values. A sequence ends after a
// panic (Go panics in SetInMap with the lock held).
var scratchSequences = map[string][]sop{
	"add-int": {
		op("Add", "k", 1), op("Add", "k", 2), op("Get", "k"), op("Add", "k", int64(3)), op("Add", "k", 2.5),
		op("Add", "k", "x"), op("Get", "k"),
	},
	"add-int-uint": {
		op("Add", "k", 1), op("Add", "k", uint(2)), op("Add", "k", -5), op("Add", "k", uint8(1)), op("Get", "k"),
	},
	"add-uint": {
		op("Add", "u", uint(1)), op("Add", "u", 1), op("Add", "u", -2), op("Get", "u"), op("Add", "u", float32(0.5)),
	},
	"add-float": {
		op("Add", "f", 0.1), op("Add", "f", 0.2), op("Add", "f", 1), op("Add", "f", uint(3)), op("Add", "f", float32(0.1)),
	},
	"add-string": {
		op("Add", "s", "a"), op("Add", "s", "b"), op("Add", "s", template.HTML("<c>")), op("Add", "s", 1), op("Get", "s"),
	},
	"add-html": {
		op("Add", "h", template.HTML("<a>")), op("Add", "h", "b"), op("Get", "h"),
	},
	"add-bool": {
		op("Add", "b", true), op("Add", "b", true), op("Get", "b"),
	},
	"add-pages": {
		op("Add", "p", []any{}), op("Add", "p", p1), op("Add", "p", p2), op("Get", "p"), op("Add", "p", "str"), op("Get", "p"),
	},
	"add-dicts": {
		op("Add", "d", []any{}), op("Add", "d", map[string]any{"position": 1, "page": p1}),
		op("Add", "d", map[string]any{"position": 2}), op("Get", "d"), op("Add", "d", maps.Params{"p": 1}), op("Get", "d"),
	},
	"add-strings": {
		op("Add", "l", []string{"a"}), op("Add", "l", "b"), op("Add", "l", []string{"c", "d"}), op("Add", "l", 1), op("Get", "l"),
	},
	"add-ints": {
		op("Add", "l", []int{1}), op("Add", "l", []int{2, 3}), op("Add", "l", 4), op("Add", "l", int64(5)), op("Get", "l"),
	},
	"add-any-slice": {
		op("Add", "l", []any{"a"}), op("Add", "l", []string{"b"}), op("Add", "l", nil), op("Add", "l", []any{"c"}), op("Get", "l"),
	},
	"add-nil-first": {
		op("Add", "n", nil), op("Get", "n"), op("Add", "n", 1),
	},
	"add-to-nil": {
		op("Set", "n", nil), op("Add", "n", 1),
	},
	"add-duration": {
		op("Add", "t", time.Duration(5)), op("Add", "t", 1), op("Get", "t"),
	},
	"add-map": {
		op("Add", "m", map[string]any{"a": 1}), op("Add", "m", map[string]any{"b": 2}),
	},
	"set-get-delete": {
		op("Get", "missing"), op("Set", "a", 1), op("Set", "b", "two"), op("Set", "c", nil), op("Get", "a"), op("Get", "c"),
		op("Values"), op("Delete", "a"), op("Delete", "missing"), op("Get", "a"), op("Set", "b", []string{"x"}), op("Values"),
	},
	"in-map": {
		op("GetSortedMapValues", "m"), op("SetInMap", "m", "b", 2), op("SetInMap", "m", "a", "one"),
		op("SetInMap", "m", "c", nil), op("GetSortedMapValues", "m"), op("DeleteInMap", "m", "a"),
		op("DeleteInMap", "m", "zz"), op("DeleteInMap", "missing", "a"), op("GetSortedMapValues", "m"), op("Get", "m"), op("Values"),
	},
	"in-map-on-string": {
		op("Set", "m", "str"), op("SetInMap", "m", "a", 1),
	},
	"in-map-on-nil": {
		op("Set", "m", nil), op("GetSortedMapValues", "m"), op("SetInMap", "m", "a", 1),
	},
	"in-map-on-params": {
		op("Set", "m", maps.Params{"a": 1}), op("DeleteInMap", "m", "a"),
	},
	"sorted-on-string": {
		op("Set", "m", "str"), op("GetSortedMapValues", "m"),
	},
	"in-map-after-set": {
		op("Set", "m", map[string]any{"z": 26}), op("SetInMap", "m", "a", 1), op("GetSortedMapValues", "m"),
	},
}

func scratchCases() []map[string]any {
	var names []string
	for n := range scratchSequences {
		names = append(names, n)
	}
	sortStrings(names)
	var cases []map[string]any
	for _, n := range names {
		s := maps.NewScratch()
		var ops []any
		for _, o := range scratchSequences[n] {
			var enc []any
			for _, a := range o.args {
				enc = append(enc, goval.Encode(a))
			}
			ret := goval.Call(func() (any, error) {
				a := o.args
				switch o.op {
				case "Add":
					return s.Add(a[0].(string), a[1])
				case "Set":
					return s.Set(a[0].(string), a[1]), nil
				case "Get":
					return s.Get(a[0].(string)), nil
				case "Delete":
					return s.Delete(a[0].(string)), nil
				case "Values":
					return s.Values(), nil
				case "SetInMap":
					return s.SetInMap(a[0].(string), a[1].(string), a[2]), nil
				case "DeleteInMap":
					return s.DeleteInMap(a[0].(string), a[1].(string)), nil
				case "GetSortedMapValues":
					return s.GetSortedMapValues(a[0].(string)), nil
				}
				panic("unknown op " + o.op)
			})
			rec := map[string]any{"op": o.op, "args": enc, "ret": ret}
			ops = append(ops, rec)
			if ret["panic"] != nil {
				break
			}
			rec["state"] = goval.Encode(s.Values())
		}
		cases = append(cases, map[string]any{"name": n, "ops": ops})
	}
	return cases
}
