// Command events is the Go oracle for crates/nh-doctree's WalkContext events and post hooks.
//
//	go run ./tools/go-oracle/nh-doctree/events [-out crates/nh-doctree/tests/fixtures/events/events.json.gz]
//
// Like hugolib/doctree's TestTreeEvents (and Hugo's "dates" aggregation): a walk registers a
// listener for every branch node and sends an event for every node; a listener raises its
// node's weight to the event's and sends its own event when the weight grows, and may stop the
// propagation. Random trees, weights, one or two event names, with or without
// StopPropagation, post hooks that may fail. Records every handler call, the final weights and
// the result of HandleEventsAndHooks.
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log"

	"github.com/neohugo/neohugo/hugolib/doctree"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-doctree/dtcommon"
)

type wnode struct {
	Key    string
	Weight int
	Branch bool
}

type shifter struct{}

func (shifter) ForEeachInDimension(n *wnode, d int, f func(*wnode) bool) { f(n) }
func (shifter) Insert(old, new *wnode) (*wnode, *wnode, bool)            { return new, old, true }
func (shifter) InsertInto(old, new *wnode, _ doctree.Dimension) (*wnode, *wnode, bool) {
	return new, old, true
}
func (shifter) Delete(n *wnode, _ doctree.Dimension) (*wnode, bool, bool) { return n, true, true }
func (shifter) Shift(n *wnode, _ doctree.Dimension, _ bool) (*wnode, bool, doctree.DimensionFlag) {
	return n, true, doctree.DimensionLanguage
}

type scenario struct {
	Name string `json:"name"`
	// Nodes: [key, weight, branch] in insertion order.
	Nodes [][]any `json:"nodes"`
	// Names: the event names; a node's events and listeners use Names[i % len(Names)] where i is
	// its index in walk order.
	Names []string `json:"names"`
	Stop  bool     `json:"stop"`
	// FailHook: the index of the post hook that fails (-1: none).
	FailHook int      `json:"failhook"`
	Log      []string `json:"log"`
	Weights  []int    `json:"weights"` // final, in insertion order
	Result   string   `json:"result"`
}

func main() {
	out := flag.String("out", "crates/nh-doctree/tests/fixtures/events/events.json.gz", "output file (gzip)")
	flag.Parse()

	var scenarios []scenario
	// TestTreeEvents itself.
	scenarios = append(scenarios, run("TestTreeEvents", [][]any{
		{"/a", 2, true}, {"/a/p1", 5, false}, {"/a/p", 6, false}, {"/a/s1", 5, true},
		{"/a/s1/p1", 8, false}, {"/a/s1/p1", 9, false}, {"/a/s1/s2", 6, true},
		{"/a/s1/s2/p1", 8, false}, {"/a/s1/s2/p2", 7, false},
	}, []string{"weight"}, true, -1))

	segs := []string{"a", "b", "ab", "a-b", "c"}
	for seed := uint64(1); seed <= 400; seed++ {
		rnd := dtcommon.NewRand(seed)
		var nodes [][]any
		if rnd.Intn(2) == 0 {
			nodes = append(nodes, []any{"", rnd.Intn(20), true})
		}
		for range 2 + rnd.Intn(20) {
			k := ""
			for range 1 + rnd.Intn(4) {
				k += "/" + dtcommon.Pick(rnd, segs)
			}
			nodes = append(nodes, []any{k, rnd.Intn(20), rnd.Intn(2) == 0})
		}
		names := []string{"dates"}
		if rnd.Intn(3) == 0 {
			names = append(names, "weight")
		}
		failHook := -1
		if rnd.Intn(4) == 0 {
			failHook = rnd.Intn(3)
		}
		scenarios = append(scenarios, run(fmt.Sprintf("random-%d", seed), nodes, names, rnd.Intn(2) == 0, failHook))
	}
	if err := dtcommon.WriteJSONGz(*out, map[string]any{"scenarios": scenarios}); err != nil {
		log.Fatal(err)
	}
}

func run(name string, nodes [][]any, names []string, stop bool, failHook int) scenario {
	sc := scenario{Name: name, Nodes: nodes, Names: names, Stop: stop, FailHook: failHook, Log: []string{}}
	tree := doctree.New(doctree.Config[*wnode]{Shifter: shifter{}})
	var all []*wnode
	for _, n := range nodes {
		v := &wnode{Key: n[0].(string), Weight: n[1].(int), Branch: n[2].(bool)}
		all = append(all, v)
		tree.InsertIntoValuesDimension(v.Key, v)
	}

	ctx := &doctree.WalkContext[*wnode]{}
	i := 0
	w := &doctree.NodeShiftTreeWalker[*wnode]{Tree: tree, WalkContext: ctx}
	w.Handle = func(s string, t *wnode, match doctree.DimensionFlag) (bool, error) {
		eventName := names[i%len(names)]
		i++
		if t.Branch {
			ctx.AddEventListener(eventName, s, func(e *doctree.Event[*wnode]) {
				sc.Log = append(sc.Log, fmt.Sprintf("%s:%s<-%s:%s:%d", eventName, s, e.Path, e.Source.Key, e.Source.Weight))
				if e.Source.Weight > t.Weight {
					t.Weight = e.Source.Weight
					ctx.SendEvent(&doctree.Event[*wnode]{Source: t, Path: s, Name: eventName})
				}
				if stop {
					e.StopPropagation()
				}
			})
		} else {
			ctx.SendEvent(&doctree.Event[*wnode]{Source: t, Path: s, Name: eventName})
		}
		return false, nil
	}
	if err := w.Walk(context.Background()); err != nil {
		log.Fatal(err)
	}
	for h := range 3 {
		ctx.AddPostHook(func() error {
			sc.Log = append(sc.Log, fmt.Sprintf("hook%d", h))
			if h == failHook {
				return errors.New("hook failed")
			}
			return nil
		})
	}
	sc.Result = "ok"
	if err := ctx.HandleEventsAndHooks(); err != nil {
		sc.Result = err.Error()
	}
	for _, v := range all {
		sc.Weights = append(sc.Weights, v.Weight)
	}
	return sc
}
