// Command transformers is the Go oracle of
// crates/nh-resource-transformers/tests/transformers.rs (Wave B task T15):
// the tpl resources namespace's transformations over the assets of the
// synthetic site (crates/nh-resource-transformers/tests/fixtures/site) and the
// repository's docs site:
//
//   - Concat: JavaScript parts in both orders (the "\n;\n" separator, parts
//     without trailing newlines, empty parts), CSS (no separator), mixed media
//     types, bad arguments, and first writer wins per (cleaned) target path;
//   - ExecuteAsTemplate: data, range/with/if/define, a parse error, an
//     execution error, first writer wins per target path (the data is not
//     part of the key), chains with minify and fingerprint;
//   - Fingerprint: every algorithm, the argument forms, unsupported
//     algorithms;
//   - Minify: every minifier of the site config, types without a minifier;
//   - PostProcess: placeholders and GetFieldString.
//
// Every returned resource is recorded (attributes, content, links, which
// publish), and so are the published files.
//
// The minifiers format numbers with float code that the arm64 Go compiler
// fuses into FMA (the golden build is darwin/arm64), so this oracle is built
// for linux/arm64 without cgo by ../arm64build and run under
// qemu-aarch64-static (docs/rust-port/HANDOFF.md §3):
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-resource-transformers/arm64build \
//	    -pkg ./tools/go-oracle/nh-resource-transformers/transformers -o /tmp/transformers.arm64
//	qemu-aarch64-static /tmp/transformers.arm64 -root .
package main

import (
	"flag"
	"fmt"
	"log"
	"path"
	"path/filepath"
	"runtime"
	"strings"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resource-transformers/rtsupport"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
)

type (
	step = rtsupport.Step
	cse  = rtsupport.Case
)

var (
	s    = rtsupport.S
	v    = rtsupport.V
	ref  = rtsupport.Ref
	refs = rtsupport.Refs
)

func get(p, as string) step { return step{Op: "Get", Args: []any{s(p)}, As: as} }

var algos = []string{"", "md5", "sha256", "sha384", "sha512", "SHA256", "sha1"}

func synthCases() []cse {
	var cs []cse
	files := map[string]string{
		"a": "js/a.js", "b": "js/b.js", "c": "js/c.js", "empty": "js/empty.js", "d": "js/sub/d.js",
		"upper": "js/Upper.JS", "cssa": "css/a.css", "cssb": "css/b.css", "search": "ts/search.ts",
		"comment": "ts/comment.ts", "bad": "ts/bad.ts", "exec": "ts/exec.ts", "define": "ts/define.ts",
		"x": "data/x.json", "y": "data/y.json", "svg": "images/logo.svg", "png": "images/pix.png",
		"html": "html/snippet.html", "xml": "xml/feed.xml", "txt": "txt/readme.txt",
	}
	var gets []step
	for _, k := range []string{"a", "b", "c", "empty", "d", "upper", "cssa", "cssb", "search", "comment", "bad", "exec", "define", "x", "y", "svg", "png", "html", "xml", "txt"} {
		gets = append(gets, get(files[k], k))
	}
	cs = append(cs, cse{Name: "get", Steps: gets})

	cs = append(cs, cse{Name: "concat", Steps: []step{
		{Op: "Concat", Args: []any{s("js/ab.js"), refs("a", "b")}, As: "ab"},
		{Op: "Concat", Args: []any{s("js/ab.js"), refs("b", "a")}, As: "ab2"},
		{Op: "Concat", Args: []any{s("js/../js/ab.js"), refs("c")}},
		{Op: "Concat", Args: []any{s("js/ba.js"), refs("b", "a")}, As: "ba"},
		{Op: "Concat", Args: []any{s("/js/ab.js"), refs("b", "c")}},
		{Op: "Concat", Args: []any{s("js/all.js"), refs("a", "b", "c", "empty", "d", "upper")}, As: "all"},
		{Op: "Concat", Args: []any{s("js/all-rev.js"), refs("upper", "d", "empty", "c", "b", "a")}},
		{Op: "Concat", Args: []any{s("js/single.js"), refs("b")}},
		{Op: "Concat", Args: []any{s("js/empties.js"), refs("empty", "empty")}},
		{Op: "Concat", Args: []any{s("css/all.css"), refs("cssa", "cssb")}, As: "cssall"},
		{Op: "Concat", Args: []any{s("css/all-rev.css"), refs("cssb", "cssa")}},
		{Op: "Concat", Args: []any{s("ts/all.ts"), refs("search", "comment")}},
		{Op: "Concat", Args: []any{s("data/all.json"), refs("x", "y")}},
		{Op: "Concat", Args: []any{s("js/as-css.css"), refs("a", "b")}},
		{Op: "Concat", Args: []any{s("js/mixed.js"), refs("a", "cssa")}},
		{Op: "Concat", Args: []any{s("js/mixed2.js"), refs("cssa", "a", "b")}},
		{Op: "Concat", Args: []any{s("js/none.js"), refs()}},
		{Op: "Concat", Args: []any{s("js/anyslice.js"), rtsupport.AnyRefs("a", "b")}},
		{Op: "Concat", Args: []any{s("js/str.js"), s("notalist")}},
		{Op: "Concat", Args: []any{v(3), refs("a")}},
		{Op: "Concat", Args: []any{v(map[string]any{"a": 1}), refs("a")}},
		{Op: "Concat", Args: []any{s("js/nested.js"), refs("ab", "ba", "c")}, As: "nested"},
		{Op: "Concat", Args: []any{s("images/sprite.png"), refs("png", "png")}},
	}})

	var fp []step
	for _, k := range []string{"a", "cssa", "x", "svg", "png", "empty", "ab"} {
		for _, al := range algos {
			if al == "" {
				fp = append(fp, step{Op: "Fingerprint", Args: []any{ref(k)}})
				continue
			}
			fp = append(fp, step{Op: "Fingerprint", Args: []any{s(al), ref(k)}})
		}
	}
	fp = append(fp,
		step{Op: "Fingerprint", Args: []any{}},
		step{Op: "Fingerprint", Args: []any{s("md5"), ref("a"), ref("b")}},
		step{Op: "Fingerprint", Args: []any{s("md5"), s("notaresource")}},
		step{Op: "Fingerprint", Args: []any{s("notaresource")}},
		step{Op: "Fingerprint", Args: []any{v(3), ref("a")}},
		step{Op: "Fingerprint", Args: []any{v(nil), ref("a")}},
		step{Op: "Fingerprint", Args: []any{v(map[string]any{"a": 1}), ref("a")}},
		step{Op: "Fingerprint", Args: []any{s("md5"), ref("a")}, As: "a-md5"},
		step{Op: "Fingerprint", Args: []any{s("sha512"), ref("a-md5")}, As: "a-md5-sha512"},
	)
	cs = append(cs, cse{Name: "fingerprint", Steps: fp})

	var mn []step
	for _, k := range []string{"a", "b", "c", "empty", "d", "upper", "cssa", "cssb", "x", "y", "svg", "html", "xml", "txt", "png", "search", "ab", "ba", "all", "cssall"} {
		mn = append(mn, step{Op: "Minify", Args: []any{ref(k)}, As: "min-" + k})
	}
	mn = append(mn,
		step{Op: "Minify", Args: []any{s("notaresource")}},
		step{Op: "Minify", Args: []any{ref("min-a")}},
		step{Op: "Fingerprint", Args: []any{ref("min-cssa")}, As: "min-fp-cssa"},
		step{Op: "Minify", Args: []any{ref("a-md5")}},
		step{Op: "Fingerprint", Args: []any{s("sha384"), ref("min-all")}},
	)
	cs = append(cs, cse{Name: "minify", Steps: mn})

	site := maps.Params{"title": "The Title", "params": maps.Params{"api": "https://api.example.org"}}
	data := func(api string, extra ...any) any {
		m := map[string]any{"api": api, "list": []any{"x", "y", 3}, "site": site, "mode": "prod"}
		for i := 0; i < len(extra); i += 2 {
			m[extra[i].(string)] = extra[i+1]
		}
		return v(m)
	}
	cs = append(cs, cse{Name: "execute-as-template", Steps: []step{
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/search.ts"), data("https://first.example"), ref("search")}, As: "eat-search"},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/search.ts"), data("https://second.example"), ref("search")}, As: "eat-search2"},
		{Op: "ExecuteAsTemplate", Args: []any{s("/ts/search.ts"), data("https://third.example"), ref("search")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/search-2.ts"), data("https://second.example"), ref("search")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/comment.ts"), data("https://c.example"), ref("comment")}, As: "eat-comment"},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/comment-dev.ts"), data("https://c.example", "mode", "dev", "site", nil), ref("comment")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/comment-params.ts"), v(maps.Params{"api": "p", "mode": "prod", "list": []any{}}), ref("comment")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/comment-nil.ts"), v(nil), ref("comment")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/search.ts"), data("https://other-source.example"), ref("comment")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/bad.ts"), data("x"), ref("bad")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/exec.ts"), data("x"), ref("exec")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/define.ts"), data("x"), ref("define")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("js/a-eat.js"), data("x"), ref("a")}, As: "eat-a"},
		{Op: "Minify", Args: []any{ref("eat-a")}, As: "eat-a-min"},
		{Op: "Fingerprint", Args: []any{ref("eat-a-min")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("css/tpl.css"), data("x"), ref("min-cssa")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/x.ts"), data("x")}},
		{Op: "ExecuteAsTemplate", Args: []any{s("ts/x.ts"), data("x"), s("notaresource")}},
		{Op: "ExecuteAsTemplate", Args: []any{v(3), data("x"), ref("search")}},
		{Op: "Concat", Args: []any{s("ts/eat-all.ts"), refs("eat-search", "eat-comment")}},
	}})

	cs = append(cs, cse{Name: "postprocess", Steps: []step{
		{Op: "PostProcess", Args: []any{ref("min-fp-cssa")}},
		{Op: "PostProcess", Args: []any{ref("min-fp-cssa")}},
		{Op: "PostProcess", Args: []any{ref("a")}},
		{Op: "PostProcess", Args: []any{ref("eat-search")}},
		{Op: "PostProcess", Args: []any{s("notaresource")}},
	}})

	cs = append(cs, cse{Name: "fromstring-copy", Steps: []step{
		{Op: "FromString", Args: []any{s("gen/x.js"), s("var gen = 1\n")}, As: "gen"},
		{Op: "Fingerprint", Args: []any{ref("gen")}},
		{Op: "Minify", Args: []any{ref("gen")}},
		{Op: "Concat", Args: []any{s("gen/all.js"), refs("gen", "a")}},
		{Op: "Copy", Args: []any{s("copied/a.js"), ref("a")}, As: "copied"},
		{Op: "Fingerprint", Args: []any{ref("copied")}},
		{Op: "Copy", Args: []any{s("copied/min.js"), ref("min-a")}},
		{Op: "Copy", Args: []any{s("copied/fp.css"), ref("min-fp-cssa")}},
	}})
	return cs
}

func docsCases(dir string) []cse {
	var cs []cse
	var gets, fps, mins []step
	var js, css []string
	for i, f := range rtsupport.AssetFiles(dir) {
		ext := path.Ext(f)
		if ext != ".js" && ext != ".css" && ext != ".svg" && ext != ".json" {
			continue
		}
		name := fmt.Sprintf("d%d", i)
		gets = append(gets, get(f, name))
		switch ext {
		case ".js":
			js = append(js, name)
		case ".css":
			css = append(css, name)
		}
		for _, al := range []string{"md5", "sha256"} {
			fps = append(fps, step{Op: "Fingerprint", Args: []any{s(al), ref(name)}})
		}
		mins = append(mins, step{Op: "Minify", Args: []any{ref(name)}})
	}
	cs = append(cs, cse{Name: "get", Steps: gets}, cse{Name: "fingerprint", Steps: fps}, cse{Name: "minify", Steps: mins})
	rev := func(a []string) []string {
		var out []string
		for i := len(a) - 1; i >= 0; i-- {
			out = append(out, a[i])
		}
		return out
	}
	cs = append(cs, cse{Name: "concat", Steps: []step{
		{Op: "Concat", Args: []any{s("js/docs.js"), refs(js...)}, As: "docsjs"},
		{Op: "Concat", Args: []any{s("js/docs.js"), refs(rev(js)...)}},
		{Op: "Concat", Args: []any{s("js/docs-rev.js"), refs(rev(js)...)}, As: "docsjsrev"},
		{Op: "Concat", Args: []any{s("css/docs.css"), refs(css...)}, As: "docscss"},
		{Op: "Minify", Args: []any{ref("docsjs")}},
		{Op: "Minify", Args: []any{ref("docsjsrev")}},
		{Op: "Minify", Args: []any{ref("docscss")}},
		{Op: "Fingerprint", Args: []any{ref("docsjs")}},
	}})
	return cs
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-resource-transformers/tests/fixtures/transformers", "fixture dir")
	flag.Parse()

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}

	synthDir := filepath.Join(absRoot, "crates/nh-resource-transformers/tests/fixtures/site")
	store, funcNames, err := rtsupport.Store(synthDir)
	if err != nil {
		log.Fatal(err)
	}
	synth := rtsupport.RunSite(synthDir, store, synthCases())
	synth["funcNames"] = funcNames
	synth["arch"] = runtime.GOARCH
	docsDir := filepath.Join(absRoot, "docs")
	docs := rtsupport.RunSite(docsDir, nil, docsCases(docsDir))
	docs["arch"] = runtime.GOARCH

	for name, res := range map[string]map[string]any{"synth": synth, "docs": docs} {
		if err := rsupport.WriteGz(filepath.Join(*out, name+".json.gz"), res); err != nil {
			log.Fatal(err)
		}
		log.Printf("transformers: %s: %d cases (%s)", name, len(res["cases"].([]any)), strings.TrimSpace(runtime.GOARCH))
	}
}
