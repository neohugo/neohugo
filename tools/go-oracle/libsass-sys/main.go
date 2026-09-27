// Command libsass-sys is the Go oracle for crates/libsass-sys: it runs
// github.com/bep/golibsass (the exact version Hugo uses) over the seeksnack
// SCSS (with an emulation of Hugo's import resolver) and over feature,
// error and import-bridging snippets, and writes the fixtures read by the
// Rust tests.
//
// Usage (from the repository root):
//
//	go run ./tools/go-oracle/libsass-sys -site <pristine-seeksnack> \
//	    -out crates/libsass-sys/tests/fixtures/sass [-hugocss <neohugo toCSS output>]
//	go run ./tools/go-oracle/libsass-sys -site <pristine-seeksnack> -out <scratch> \
//	    -fuzz-only <scratch>/fuzz.rec.zz -n 20000 -seed 1000
//	go run ./tools/go-oracle/libsass-sys -json crates/libsass-sys/tests/fixtures/sass/json.rec.zz
//
// The SCSS sources are stored in <out>/site.pack.zz; both this oracle and the
// Rust test extract them into a fresh temporary directory, make it the
// working directory (LibSass resolves "stdin"-relative imports and source
// map paths against the CWD) and replace that directory with "@SITE@" in
// all recorded outputs.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strings"

	"github.com/bep/golibsass/libsass"
	"github.com/bep/golibsass/libsass/libsasserrors"
)

var extraFiles = map[string]string{
	"assets/extra/_partial.scss":     "$p: 1px;\n.partial { width: $p; }\n",
	"assets/extra/_relative.scss":    ".relative { r: s; }\n",
	"assets/extra/nested/_deep.scss": "@import \"../partial\";\n.deep { z: 1; }\n",
	"assets/extra/dir/_index.scss":   ".dir-index { x: y; }\n",
	"assets/extra/dir2/index.scss":   ".dir2-index { x: y; }\n",
	"assets/extra/_both.scss":        ".both-underscore { a: b; }\n",
	"assets/extra/both.scss":         ".both-plain { a: b; }\n",
	"assets/extra/indented.sass":     ".ind-file\n  color: blue\n",
	"assets/extra/_sassy.sass":       "$s: 3px\n.sassy\n  width: $s\n",
	"assets/extra/plain.css":         ".plain { a: b; }\n",
	"extra/dir1/_colors.scss":        "\n$moo:       #f442d1 !default;\n",
	"extra/dir2/_content.scss":       "\ncontent { color: #ccc; }\n",
}

func main() {
	site := flag.String("site", "", "pristine seeksnack site")
	out := flag.String("out", "", "fixture output directory")
	hugoCSS := flag.String("hugocss", "", "optional neohugo toCSS output (compressed) to compare with")
	fuzzN := flag.Int("n", 1500, "number of randomized cases in adv.rec.zz")
	fuzzSeed := flag.Int64("seed", 1, "seed of the randomized cases in adv.rec.zz")
	fuzzOnly := flag.String("fuzz-only", "", "write only -n randomized cases (seed -seed) to this file")
	jsonOut := flag.String("json", "", "write the JsonToError corpus (-json-n random documents) to this file and exit")
	jsonN := flag.Int("json-n", 3000, "number of random documents for -json")
	flag.Parse()
	if *jsonOut != "" {
		writeJSONCases(*jsonOut, *fuzzSeed, *jsonN)
		return
	}
	if *site == "" || *out == "" {
		flag.Usage()
		os.Exit(2)
	}
	outDir, err := filepath.Abs(*out)
	if err != nil {
		panic(err)
	}
	if err := os.MkdirAll(outDir, 0o755); err != nil {
		panic(err)
	}

	// 1. Collect the SCSS sources.
	files := map[string][]byte{}
	for _, d := range []string{"assets/scss", "node_modules/bootstrap/scss", "node_modules/@fortawesome/fontawesome-free/scss"} {
		err := filepath.WalkDir(filepath.Join(*site, d), func(p string, e fs.DirEntry, err error) error {
			if err != nil || e.IsDir() {
				return err
			}
			rel, _ := filepath.Rel(*site, p)
			b, err := os.ReadFile(p)
			if err != nil {
				return err
			}
			files[filepath.ToSlash(rel)] = b
			return nil
		})
		if err != nil {
			panic(err)
		}
	}
	for k, v := range extraFiles {
		files[k] = []byte(v)
	}
	writeSitePack(filepath.Join(outDir, "site.pack.zz"), files)

	// 2. Extract into a fresh directory and make it the CWD.
	tmp, err := os.MkdirTemp("", "libsass-oracle-")
	if err != nil {
		panic(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()
	root, err := filepath.EvalSymlinks(tmp)
	if err != nil {
		panic(err)
	}
	// A fixed base name: source-map paths that climb out of the root (for
	// example OutputPath "../o.css") contain it.
	root = filepath.Join(root, "site")
	for k, v := range files {
		p := filepath.Join(root, filepath.FromSlash(k))
		if err := os.MkdirAll(filepath.Dir(p), 0o755); err != nil {
			panic(err)
		}
		if err := os.WriteFile(p, v, 0o644); err != nil {
			panic(err)
		}
	}
	if err := os.Chdir(root); err != nil {
		panic(err)
	}
	website, err := os.ReadFile(filepath.Join(root, "assets/scss/website.scss"))
	if err != nil {
		panic(err)
	}

	// 3. Run the cases.
	env := runEnv{root: root, website: string(website), hugoCSS: *hugoCSS}
	if *fuzzOnly != "" {
		// A larger randomized corpus kept outside the repository (the Rust
		// test reads it via LIBSASS_EXTRA_CASES).
		env.run(fuzzSass(*fuzzSeed, *fuzzN), *fuzzOnly)
		return
	}
	env.run(allCases(), filepath.Join(outDir, "cases.rec.zz"))
	adv := append(advCases(), fuzzSass(*fuzzSeed, *fuzzN)...)
	env.run(adv, filepath.Join(outDir, "adv.rec.zz"))
	writeQuoteTable(filepath.Join(outDir, "quote.tsv"))
}

// runEnv holds what running a case needs besides the case itself.
type runEnv struct {
	root, website, hugoCSS string
}

// run executes cases and writes their records to path.
func (env runEnv) run(cases []sassCase, path string) {
	root := env.root
	sub := func(s string) string { return strings.ReplaceAll(s, "@SITE@", root) }
	unsub := func(s string) string { return strings.ReplaceAll(s, root, "@SITE@") }
	w := newRecordWriter(path)
	n, nerr := 0, 0
	for _, c := range cases {
		src := sub(c.src)
		if c.src == "@WEBSITE@" {
			src = env.website
		}
		var includes []string
		for _, ip := range c.includes {
			includes = append(includes, sub(ip))
		}
		opts := libsass.Options{
			OutputStyle:  libsass.OutputStyle(c.style),
			Precision:    c.precision,
			IncludePaths: includes,
			SassSyntax:   c.sassSyntax,
			SourceMapOptions: libsass.SourceMapOptions{
				Filename: sub(c.sm.Filename), Root: sub(c.sm.Root), InputPath: sub(c.sm.InputPath), OutputPath: sub(c.sm.OutputPath),
				Contents: c.sm.Contents, OmitURL: c.sm.OmitURL, EnableEmbedded: c.sm.EnableEmbedded,
			},
		}
		var trace []string
		switch c.resolver {
		case "none":
		case "hugo":
			opts.ImportResolver = hugoResolver(hugoFs{root: root}, c.hugoBaseDir, c.hugoVars, &trace)
		case "echo":
			opts.ImportResolver = func(url, prev string) (string, string, bool) {
				trace = append(trace, url+"\t"+prev)
				return url, `$white:    #fff`, true
			}
		case "table":
			opts.ImportResolver = func(url, prev string) (string, string, bool) {
				trace = append(trace, url+"\t"+prev)
				for _, e := range c.table {
					if e.url == url {
						return sub(e.newURL), e.body, e.ok
					}
				}
				return "", "", false
			}
		case "nested":
			// Re-entrant resolver: runs a nested transpile of the table body
			// (whose own imports are answered with `$inner: 7px;`).
			opts.ImportResolver = func(url, prev string) (string, string, bool) {
				trace = append(trace, url+"\t"+prev)
				for _, e := range c.table {
					if e.url == url {
						inner, _ := libsass.New(libsass.Options{
							OutputStyle: libsass.CompressedStyle,
							ImportResolver: func(u, p string) (string, string, bool) {
								trace = append(trace, "inner:"+u+"\t"+p)
								return u, "$inner: 7px;", true
							},
						})
						r, err := inner.Execute(e.body)
						if err != nil {
							return sub(e.newURL), "/* " + err.Error() + " */", true
						}
						return sub(e.newURL), r.CSS, e.ok
					}
				}
				return "", "", false
			}
		default:
			panic(c.resolver)
		}

		t, err := libsass.New(opts)
		if err != nil {
			panic(err)
		}
		if os.Getenv("LIBSASS_ORACLE_VERBOSE") != "" {
			fmt.Fprintf(os.Stderr, "CASE %s %q\n", c.name, src)
		}
		res, err := t.Execute(src)

		w.putS("case", c.name)
		w.putS("src", c.src)
		w.putI("style", c.style)
		w.putI("precision", c.precision)
		w.putS("include", strings.Join(c.includes, "\n"))
		w.putB("sass_syntax", c.sassSyntax)
		w.putS("sm_filename", c.sm.Filename)
		w.putS("sm_root", c.sm.Root)
		w.putS("sm_input", c.sm.InputPath)
		w.putS("sm_output", c.sm.OutputPath)
		w.putB("sm_contents", c.sm.Contents)
		w.putB("sm_omit", c.sm.OmitURL)
		w.putB("sm_embed", c.sm.EnableEmbedded)
		w.putS("resolver", c.resolver)
		w.putS("hugo_basedir", c.hugoBaseDir)
		w.putS("hugo_vars", c.hugoVars)
		for _, e := range c.table {
			w.putS("rt_url", e.url)
			w.putS("rt_new", e.newURL)
			w.putS("rt_body", e.body)
			w.putB("rt_ok", e.ok)
		}
		if err != nil {
			nerr++
			lerr, ok := err.(libsasserrors.Error)
			if !ok {
				panic(err)
			}
			w.putS("x_status", "err")
			w.putS("x_err", unsub(lerr.Error()))
			w.putI("x_err_status", lerr.Status)
			w.putI("x_err_line", lerr.Line)
			w.putI("x_err_column", lerr.Column)
			w.putS("x_err_file", unsub(lerr.File))
			w.putS("x_err_message", unsub(lerr.Message))
		} else {
			w.putS("x_status", "ok")
			w.putS("x_css", unsub(res.CSS))
			w.putS("x_smfile", unsub(res.SourceMapFilename))
			w.putS("x_smcontent", unsub(res.SourceMapContent))
		}
		w.putS("x_trace", unsub(strings.Join(trace, "\n")))
		w.putS("end", "")
		n++

		if c.name == "site/hugo/style3" && env.hugoCSS != "" {
			want, err := os.ReadFile(env.hugoCSS)
			if err != nil {
				panic(err)
			}
			if !bytes.Equal(want, []byte(res.CSS)) {
				panic("site/hugo/style3 differs from the neohugo toCSS output")
			}
			fmt.Fprintf(os.Stderr, "site/hugo/style3 == %s (%d bytes)\n", env.hugoCSS, len(want))
		}
	}
	w.close()
	fmt.Fprintf(os.Stderr, "wrote %d cases (%d errors) to %s\n", n, nerr, path)
}
