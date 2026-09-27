package main

import (
	"bufio"
	"compress/gzip"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"hash/fnv"
	"math/rand"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
)

// Record format (both fixture files): for every input
//
//	#rec N\n
//	then N fields, each: <decimal length>\n<bytes>\n
//
// Field 0 is the input, fields 1..6 are the dumps of `modes` (full text in
// literals.rec.gz, FNV-1a 64 hex digests in fuzz.rec.gz).

type recWriter struct {
	f  *os.File
	gz *gzip.Writer
	w  *bufio.Writer
}

func newRecWriter(path string) *recWriter {
	f, err := os.Create(path)
	if err != nil {
		panic(err)
	}
	gz, _ := gzip.NewWriterLevel(f, gzip.BestCompression)
	return &recWriter{f: f, gz: gz, w: bufio.NewWriter(gz)}
}

func (r *recWriter) rec(fields ...[]byte) {
	fmt.Fprintf(r.w, "#rec %d\n", len(fields))
	for _, f := range fields {
		fmt.Fprintf(r.w, "%d\n", len(f))
		r.w.Write(f)
		r.w.WriteByte('\n')
	}
}

func (r *recWriter) close() {
	if err := r.w.Flush(); err != nil {
		panic(err)
	}
	if err := r.gz.Close(); err != nil {
		panic(err)
	}
	if err := r.f.Close(); err != nil {
		panic(err)
	}
}

func digest(b []byte) []byte {
	h := fnv.New64a()
	h.Write(b)
	return []byte(fmt.Sprintf("%016x", h.Sum64()))
}

func modCache() string {
	out, err := exec.Command("go", "env", "GOMODCACHE").Output()
	if err != nil {
		panic(err)
	}
	return strings.TrimSpace(string(out))
}

// testLiterals returns every string literal of the upstream parse/js and
// minify/js test files (inputs, expected outputs and error messages alike;
// all of them are useful parser inputs), deduplicated in file order.
func testLiterals() [][]byte {
	mc := modCache()
	var files []string
	for _, dir := range []string{
		filepath.Join(mc, "github.com/tdewolff/parse/v2@v2.8.1/js"),
		filepath.Join(mc, "github.com/tdewolff/minify/v2@v2.23.8/js"),
	} {
		m, err := filepath.Glob(filepath.Join(dir, "*_test.go"))
		if err != nil {
			panic(err)
		}
		sort.Strings(m)
		files = append(files, m...)
	}
	seen := map[string]bool{}
	var lits [][]byte
	add := func(s string) {
		if !seen[s] {
			seen[s] = true
			lits = append(lits, []byte(s))
		}
	}
	for _, e := range extraLiterals {
		add(e)
	}
	for _, fn := range files {
		fset := token.NewFileSet()
		f, err := parser.ParseFile(fset, fn, nil, 0)
		if err != nil {
			panic(err)
		}
		ast.Inspect(f, func(n ast.Node) bool {
			if bl, ok := n.(*ast.BasicLit); ok && bl.Kind == token.STRING {
				s, err := strconv.Unquote(bl.Value)
				if err == nil {
					add(s)
				}
			}
			return true
		})
	}
	return lits
}

// extraLiterals are hand-written edge cases (scope analysis, arrow-function
// speculation, ASI, regexps, templates, unicode, errors).
var extraLiterals = []string{
	"",
	"a",
	"#!shebang\nvar a",
	"#!",
	"(a,b)=>a+b",
	"(a,b)+(a,b)",
	"async(a,b)",
	"async (a,b)=>{await a}",
	"async a=>a",
	"async\na=>a",
	"(a=1,{b,c:[d,...e]}={},...f)=>a+b+d+e+f",
	"({a,b=c,...d})=>1",
	"({a,b=c,...d})",
	"({a:1,b,c(){},get d(){},set e(v){},async f(){},*g(){},async*h(){},[i]:2,'j':3,5:4})",
	"(a)=>{var a;let b;{var c}}",
	"x=>x,y=>y",
	"a?b=>c:d=>e",
	"for(let i=0;i<n;i++){let j=i;setTimeout(()=>j)}",
	"for(const k in o)f(k);for(const v of o)g(v);for await(const v of o);",
	"while(a)b;do c;while(d)",
	"while(a){let b}",
	"label:for(;;){break label;continue label}",
	"switch(a){case 1:let b;case 2:{var c}default:d}",
	"try{a}catch({b,c}){var d}finally{e}",
	"try{}catch{}",
	"class A extends B{static #p=1;#q;static{var x=this.#p}get [k](){}set v(x){}static async*m(){}}",
	"new new A()()",
	"new A.b.c()",
	"a?.b?.[c]?.(d)?.`e`",
	"a`b${c}d${e}f`",
	"`${`${`${a}`}`}`",
	"/re/g.test(s)",
	"a=/[/]/.source",
	"x = a / b / c",
	"x = a\n/b/g",
	"x = (a)/2/g",
	"let\nx=1",
	"let [a]=[1]",
	"yield=1;await=2",
	"function*g(){yield;yield*a;yield\na}",
	"async function f(){await a;for await(x of y);}",
	"import a,{b as c,d,'e' as f}from'g';import*as h from'i';import'j';import('k');import.meta.url",
	"export default class{};export*from'a';export*as b from'c';export{d as e,f};export const g=1;export async function h(){}",
	"export default async()=>1",
	"export default async function(){}",
	"'use strict';'use strict';a",
	"function f(){'use strict';return 1}",
	"a\n++b",
	"a\n--b",
	"return 1",
	"if(a)function f(){}",
	"var a;var a;function a(){}",
	"let a;var a",
	"let a;let a",
	"function f(a,a){}",
	"(a,a)=>1",
	"{function f(){}function f(){}}",
	"a=>{a=>a}",
	"(a=b=>b)=>a",
	"(b)=>(a)=>b",
	"(function(){var e=1;return function(t){return e+t}})()",
	"!function(e,t){\"object\"==typeof module?module.exports=t():e.x=t()}(this,function(){return 1})",
	"var \u00e9l\u00e8ve=1,\u03b1\u03b2=2,\u0561=3;\u00e9l\u00e8ve+\u03b1\u03b2",
	"\\u0061\\u{62}=1",
	"a\u2028b\u2029c",
	"\ufeffvar a",
	"x=1_000_000+0x_1+0b1_0+0o7_7+.5e-1_0+1n",
	"0.toString()",
	"0..toString()",
	"1.e5",
	"'\\\n'",
	"\"a\\\"b\"",
	"<!-- comment\nx=1\n--> comment",
	"/*! bang */\n//! bang2\nfunction f(){/*! inner */}",
	"a\x00b",
	"a\xffb",
	"a\xc3",
	"\xe2\x80",
	"with(a)b",
	"debugger",
	"({}).toString()",
	"({a}=b)",
	"[a,,b,...c]=d",
	"({a=1})",
	"({a:b=1})=>0",
	"((a))=>1",
	"(...a,b)=>1",
	"(a,...b)",
	"async(...a)",
	"a??b||c",
	"a**-b",
	"delete a[b];void 0;typeof c",
	"x=function f(){f=1}",
	"x=class C{m(){C}}",
	"var {a,b:{c}}=d,[e,[f]]=g",
	"for(var [a,b] of c);for(var {d} in e);",
	"for(a.b in c);for([a]of b);",
	"for(let;;);",
	"for(let in a);",
	"let\n[a]=b",
	"async\nfunction f(){}",
	"a\n?.5:1",
	"a?.5:1",
	"x={if:1,class:2,new:3}.if",
	"({if})",
	"o={get,set,async,static}",
	"class A{get;set;async;static;static static;static async;static get;static set}",
	"class A{static(){}async(){}get(){}set(){}}",
	"class A{'constructor'(){}}",
	"class A{[a]=1;[b](){}}",
	"a=>{}\n(b)",
	"()=>{}",
	"async()=>{}",
	"async=>async",
	"(async)=>async",
	"x=async",
	"async.x",
	"new.target",
	"super.x",
	"function f(a=arguments){}",
	"function f(){return function(){this}}",
	"({a(){super.b}})",
	"var a=1,b=a,c=b",
	"{a} {a} var a",
	"a;b;c;var a,b,c",
	"function a(){}function a(){}",
	"if(a){}else if(b){}else{}",
}

var vocab = []string{
	"(", ")", "{", "}", "[", "]", "=>", "/", "/=", "\n", ";", ",", "...", "?.", "?", ":", "=",
	"async", "await", "yield", "let", "var", "const", "function", "class", "return", "new", "in", "of",
	"`", "${", "'", "\"", "#x", "\\u0061", "\u00e9", "\u2028", "/*!c*/", "//c\n", "<!--", "-->", "\x00",
	"a", "b", " ", "1", ".5", "0x", "++", "--", "*", "import", "export", "default", "static", "get",
}

func mutate(rnd *rand.Rand, lits [][]byte) []byte {
	src := append([]byte(nil), lits[rnd.Intn(len(lits))]...)
	return mutateOps(rnd, src, lits, 1+rnd.Intn(3))
}

func mutateOps(rnd *rand.Rand, src []byte, lits [][]byte, ops int) []byte {
	for k := 0; k < ops; k++ {
		switch rnd.Intn(6) {
		case 0, 1: // insert a vocabulary token
			i := rnd.Intn(len(src) + 1)
			v := vocab[rnd.Intn(len(vocab))]
			src = append(src[:i:i], append([]byte(v), src[i:]...)...)
		case 2: // delete a range
			if 0 < len(src) {
				i := rnd.Intn(len(src))
				j := i + 1 + rnd.Intn(min(8, len(src)-i))
				src = append(src[:i:i], src[j:]...)
			}
		case 3: // splice with another literal
			o := lits[rnd.Intn(len(lits))]
			i := rnd.Intn(len(src) + 1)
			j := rnd.Intn(len(o) + 1)
			src = append(src[:i:i], o[j:]...)
		case 4: // duplicate a range
			if 0 < len(src) {
				i := rnd.Intn(len(src))
				j := i + 1 + rnd.Intn(min(16, len(src)-i))
				src = append(src[:j:j], append(append([]byte(nil), src[i:j]...), src[j:]...)...)
			}
		case 5: // truncate
			if 0 < len(src) {
				src = src[:rnd.Intn(len(src))]
			}
		}
	}
	return src
}

func genFixtures(dir string, nfuzz int, seed int64) {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		panic(err)
	}
	lits := testLiterals()

	w := newRecWriter(filepath.Join(dir, "literals.rec.gz"))
	for _, src := range lits {
		fields := [][]byte{src}
		for _, m := range modes {
			fields = append(fields, runMode(m, src))
		}
		w.rec(fields...)
	}
	w.close()

	var nonEmpty [][]byte
	for _, l := range lits {
		if 0 < len(l) {
			nonEmpty = append(nonEmpty, l)
		}
	}
	rnd := rand.New(rand.NewSource(seed))
	w = newRecWriter(filepath.Join(dir, "fuzz.rec.gz"))
	for n := 0; n < nfuzz; n++ {
		src := mutate(rnd, nonEmpty)
		fields := [][]byte{src}
		for _, m := range modes {
			fields = append(fields, digest(runMode(m, src)))
		}
		w.rec(fields...)
	}
	w.close()
	fmt.Fprintf(os.Stderr, "fixtures: %d literals, %d fuzz\n", len(lits), nfuzz)
}

// corpusDigests writes OUT (TSV): root base name, relative path, size, and
// the FNV-1a 64 digest of every mode's dump, for every file under the roots
// accepted by isJSPath.
func corpusDigests(out string, roots []string) {
	f, err := os.Create(out)
	if err != nil {
		panic(err)
	}
	w := bufio.NewWriter(f)
	fmt.Fprintf(w, "# root\tpath\tsize\t%s\n", strings.Join(modes, "\t"))
	n := 0
	for _, root := range roots {
		var paths []string
		err := filepath.Walk(root, func(p string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if info.Mode().IsRegular() && isJSPath(p) {
				paths = append(paths, p)
			}
			return nil
		})
		if err != nil {
			panic(err)
		}
		sort.Strings(paths)
		for _, p := range paths {
			src, err := os.ReadFile(p)
			if err != nil {
				panic(err)
			}
			rel, _ := filepath.Rel(root, p)
			fmt.Fprintf(w, "%s\t%s\t%d", filepath.Base(root), rel, len(src))
			for _, m := range modes {
				fmt.Fprintf(w, "\t%s", digest(runMode(m, src)))
			}
			fmt.Fprintln(w)
			n++
		}
	}
	if err := w.Flush(); err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
	fmt.Fprintf(os.Stderr, "corpus: %d files\n", n)
}

func isJSPath(p string) bool {
	switch filepath.Ext(p) {
	case ".js", ".mjs", ".cjs", ".ts", ".mts", ".cts":
		// TypeScript sources are not JS; they exercise the error paths
		return true
	case ".in", ".out":
		// the recorded minifier inputs and outputs of work/minify/corpus2
		return strings.Contains(filepath.ToSlash(p), "/js/")
	}
	return false
}

// genCorpusFuzz writes OUT: random windows (log-uniform length up to
// maxLen bytes) of the JS files under the roots, with 0-2 mutations each;
// fields are the input and the digests of every mode.
func genCorpusFuzz(out string, n int, seed int64, maxLen int, roots []string) {
	var paths []string
	for _, root := range roots {
		err := filepath.Walk(root, func(p string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if info.Mode().IsRegular() && isJSPath(p) && 0 < info.Size() {
				paths = append(paths, p)
			}
			return nil
		})
		if err != nil {
			panic(err)
		}
	}
	sort.Strings(paths)
	lits := testLiterals()
	rnd := rand.New(rand.NewSource(seed))
	w := newRecWriter(out)
	for k := 0; k < n; k++ {
		src, err := os.ReadFile(paths[rnd.Intn(len(paths))])
		if err != nil {
			panic(err)
		}
		l := 1
		for l < maxLen && rnd.Intn(4) != 0 {
			l *= 2
		}
		l = 1 + rnd.Intn(l)
		i := rnd.Intn(len(src))
		j := min(len(src), i+l)
		win := append([]byte(nil), src[i:j]...)
		win = mutateOps(rnd, win, lits, rnd.Intn(3))
		fields := [][]byte{win}
		for _, m := range modes {
			fields = append(fields, digest(runMode(m, win)))
		}
		w.rec(fields...)
	}
	w.close()
	fmt.Fprintf(os.Stderr, "corpus fuzz: %d windows of %d files\n", n, len(paths))
}
