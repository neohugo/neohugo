// Command go-url is the Go oracle for the Rust crate crates/go-url.
//
//	go run ./tools/go-oracle/go-url -out crates/go-url/tests/fixtures/url.txt [-n N] [-seed S]
//	go run ./tools/go-oracle/go-url -out crates/go-url/tests/fixtures/adversarial.txt -adv N [-seed S]
//
// Inputs: every string literal and literal tuple of
// $GOROOT/src/net/url/url_test.go, plus random URLs, random URL structs and
// random byte soup. Each output line is
//
//	op \t nargs \t arg... \t result...
//
// with every field encoded by esc() (printable ASCII except '\', else \xHH).
// The first line is the encoding table parsed from encoding_table.go.
package main

import (
	"bufio"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"log"
	"math/rand/v2"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"slices"
	"strconv"
	"strings"
)

func esc(s string) string {
	var b strings.Builder
	for i := 0; i < len(s); i++ {
		c := s[i]
		if c >= 0x20 && c < 0x7f && c != '\\' {
			b.WriteByte(c)
		} else {
			fmt.Fprintf(&b, "\\x%02x", c)
		}
	}
	return b.String()
}

func goroot() string {
	out, err := exec.Command("go", "env", "GOROOT").Output()
	if err != nil {
		log.Fatal(err)
	}
	return strings.TrimSpace(string(out))
}

// literals returns all string literals and all tuples (>= 2 string literals
// that are direct elements of one composite literal) of the given file.
func literals(fn string) (singles []string, tuples [][]string) {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, fn, nil, 0)
	if err != nil {
		log.Fatal(err)
	}
	ast.Inspect(f, func(n ast.Node) bool {
		switch x := n.(type) {
		case *ast.BasicLit:
			if x.Kind == token.STRING {
				if s, err := strconv.Unquote(x.Value); err == nil {
					singles = append(singles, s)
				}
			}
		case *ast.CompositeLit:
			var t []string
			for _, e := range x.Elts {
				if kv, ok := e.(*ast.KeyValueExpr); ok {
					e = kv.Value
				}
				if bl, ok := e.(*ast.BasicLit); ok && bl.Kind == token.STRING {
					if s, err := strconv.Unquote(bl.Value); err == nil {
						t = append(t, s)
					}
				}
			}
			if len(t) >= 2 {
				tuples = append(tuples, t)
			}
		}
		return true
	})
	return
}

// encodingTable evaluates `var table = [256]encoding{...}` of encoding_table.go.
func encodingTable() [256]int {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, filepath.Join(goroot(), "src/net/url/encoding_table.go"), nil, 0)
	if err != nil {
		log.Fatal(err)
	}
	bits := map[string]int{}
	var table [256]int
	for _, d := range f.Decls {
		gd, ok := d.(*ast.GenDecl)
		if !ok {
			continue
		}
		switch gd.Tok {
		case token.CONST:
			for i, sp := range gd.Specs {
				vs := sp.(*ast.ValueSpec)
				if i == 0 {
					if be, ok := vs.Values[0].(*ast.BinaryExpr); !ok || be.Op != token.SHL {
						log.Fatal("unexpected const block")
					}
				}
				bits[vs.Names[0].Name] = 1 << i
			}
		case token.VAR:
			vs := gd.Specs[0].(*ast.ValueSpec)
			if vs.Names[0].Name != "table" {
				continue
			}
			cl := vs.Values[0].(*ast.CompositeLit)
			for _, e := range cl.Elts {
				kv := e.(*ast.KeyValueExpr)
				ks, err := strconv.Unquote(kv.Key.(*ast.BasicLit).Value)
				if err != nil {
					log.Fatal(err)
				}
				k := []rune(ks)[0]
				v := 0
				var walk func(ast.Expr)
				walk = func(x ast.Expr) {
					switch y := x.(type) {
					case *ast.BinaryExpr:
						walk(y.X)
						walk(y.Y)
					case *ast.Ident:
						b, ok := bits[y.Name]
						if !ok {
							log.Fatalf("unknown ident %s", y.Name)
						}
						v |= b
					default:
						log.Fatalf("unexpected expr %T", x)
					}
				}
				walk(kv.Value)
				table[k] = v
			}
		}
	}
	if len(bits) != 8 {
		log.Fatalf("expected 8 encoding bits, got %d", len(bits))
	}
	return table
}

type writer struct {
	w     *bufio.Writer
	n     int
	every int // keep only every Nth record (-every), for checked-in samples
	seen  int
}

func (w *writer) rec(op string, args []string, res ...string) {
	w.seen++
	if w.every > 1 && (w.seen-1)%w.every != 0 {
		return
	}
	w.w.WriteString(op)
	w.w.WriteByte('\t')
	w.w.WriteString(strconv.Itoa(len(args)))
	for _, a := range args {
		w.w.WriteByte('\t')
		w.w.WriteString(esc(a))
	}
	for _, r := range res {
		w.w.WriteByte('\t')
		w.w.WriteString(esc(r))
	}
	w.w.WriteByte('\n')
	w.n++
}

func b2s(b bool) string {
	if b {
		return "1"
	}
	return "0"
}

func errStr(err error) string {
	if err == nil {
		return "<nil>"
	}
	return err.Error()
}

// structFields are the 13 settable fields of a URL, in fixture order.
func structFields(u *url.URL) []string {
	uf, un, pw := "nil", "", ""
	if u.User != nil {
		un = u.User.Username()
		p, set := u.User.Password()
		pw = p
		if set {
			uf = "userpass"
		} else {
			uf = "user"
		}
	}
	return []string{u.Scheme, u.Opaque, uf, un, pw, u.Host, u.Path, u.RawPath, b2s(u.OmitHost), b2s(u.ForceQuery), u.RawQuery, u.Fragment, u.RawFragment}
}

// dump returns the fields plus every derived value.
func dump(u *url.URL) []string {
	out := structFields(u)
	us := ""
	if u.User != nil {
		us = u.User.String()
	}
	mb, _ := u.MarshalBinary()
	out = append(out, u.String(), u.EscapedPath(), u.EscapedFragment(), u.Redacted(), u.RequestURI(),
		u.Hostname(), u.Port(), b2s(u.IsAbs()), u.Query().Encode(), us, string(mb))
	return out
}

func parseRes(u *url.URL, err error) []string {
	if err != nil {
		return []string{"ERR", err.Error()}
	}
	return append([]string{"OK"}, dump(u)...)
}

func single(w *writer, s string) {
	a := []string{s}
	u, err := url.Parse(s)
	w.rec("Parse", a, parseRes(u, err)...)
	u, err = url.ParseRequestURI(s)
	w.rec("ParseRequestURI", a, parseRes(u, err)...)
	w.rec("QueryEscape", a, url.QueryEscape(s))
	w.rec("PathEscape", a, url.PathEscape(s))
	r, err := url.QueryUnescape(s)
	w.rec("QueryUnescape", a, r, errStr(err))
	r, err = url.PathUnescape(s)
	w.rec("PathUnescape", a, r, errStr(err))
	parseQueryRec(w, s)
}

func parseQueryRec(w *writer, s string) {
	a := []string{s}
	v, err := url.ParseQuery(s)
	res := []string{errStr(err)}
	// url.Values iteration: emit keys sorted by Encode's order.
	enc := v.Encode()
	res = append(res, enc, strconv.Itoa(len(v)))
	keys := make([]string, 0, len(v))
	for k := range v {
		keys = append(keys, k)
	}
	slices.Sort(keys)
	for _, k := range keys {
		res = append(res, k, strconv.Itoa(len(v[k])))
		res = append(res, v[k]...)
	}
	w.rec("ParseQuery", a, res...)
}

func resolve(w *writer, base, ref string) {
	bu, err := url.Parse(base)
	if err != nil {
		return
	}
	u, err := bu.Parse(ref)
	w.rec("Resolve", []string{base, ref}, parseRes(u, err)...)
}

func joinPath(w *writer, base string, elems []string) {
	args := append([]string{base}, elems...)
	r, err := url.JoinPath(base, elems...)
	res := []string{r, errStr(err)}
	if bu, err := url.Parse(base); err == nil {
		res = append(res, dump(bu.JoinPath(elems...))...)
	}
	w.rec("JoinPath", args, res...)
}

func structRec(w *writer, u, ref *url.URL, elems []string) {
	args := append(structFields(u), structFields(ref)...)
	args = append(args, elems...)
	res := dump(u)
	res = append(res, dump(u.ResolveReference(ref))...)
	res = append(res, dump(u.JoinPath(elems...))...)
	w.rec("Struct", args, res...)
}

type gen struct{ r *rand.Rand }

func (g gen) pick(xs []string) string { return xs[g.r.IntN(len(xs))] }

var (
	schemes   = []string{"", "", "http:", "https:", "HTTP:", "mailto:", "file:", "a+b.c-d:", "1http:", ":", "ht tp:", "javascript:", "HtTpS:", "x:"}
	slashes   = []string{"", "//", "//", "//", "///", "/"}
	userinfos = []string{"", "", "", "user@", "user:pass@", "us%20er:p%40ss@", "u@v@", "\xc3\xbc@", "a:b:c@", "%zz@", "@", ":@", "u:@", "u%3Ax@", "a b@", "u:p%zz@"}
	hosts     = []string{"", "example.com", "EXAMPLE.com", "host:80", "host:", "host:x", "[::1]", "[::1]:8080", "[fe80::1%25en0]",
		"[fe80::1%en0]", "[::1", "::1", "1.2.3.4", "[1.2.3.4]", "[::ffff:1.2.3.4]", "h%41st", "h%C3%BCst", "h ost", "h\xc3\xb6",
		"a:1:2", "[::1]x", "x[::1]", "%25", "h<t", "h\"t", "[::1%25%41]", "[::1%2525]", "[v1.x]", "a,b:1,c:2", "[::1]:", "[::1]:a",
		"[fe80::1%25%20x]", "[fe80::1%25%0A]", "[::1%25]", "[1::2::3]", "[::12345]", "[1:2:3:4:5:6:7:8:9]", "[::1.2.3]", "[::1.2.3.04]",
		"[1:2:3:4:5:6:1.2.3.4]", "[1:2:3:4:5:6:7:1.2.3.4]", "[%31::1]", "[fe80::%31]", "h:1:", "a%2", "x%e2%82%ac", "[::]", "[]",
		"h%7Ex", "h%3Ax", "h%80x", "%C3%BC", "h%25x", "[fe80::1%25%7E]", "[fe80::1%25%80]", "[fe80::1%25%3C]", "[fe80::1%25a%2Fb]", "[::1]:80:90", "h:80:90"}
	paths = []string{"", "", "/", "/a/b", "/a%2Fb", "/a b", "/\xc3\xbc", "/%zz", "//x", "a/b", "a:b/c", "/./a/../b", "/%41",
		"/a%2fb%2F", "*", "/%E2%82%AC", "/\xff", "/a;b,c", "/a@b:c", "/%", "/%2", "/a+b", "/~_-.", "/a%20b", "/..", "/a/./", "/a/../../b", "/!$&'()*=", "/[x]", "/a?"}
	queries = []string{"", "", "?", "?a=1&b=2", "?a=%zz", "?a;b", "?q=a+b", "??", "?a=1?", "?%", "?x=\xc3\xbc", "?a=1&a=2&b", "?&&=&", "?=x", "?a%3Db=c", "?%41=%42"}
	frags   = []string{"", "", "#", "#frag", "#a%20b", "#a b", "#%zz", "#!$&'()*+,;=", "#\xc3\xbc", "#%41", "#a#b", "#/?", "#%2F", "#[]", "#%"}
	soup    = []string{"a", "Z", "0", "%", "%2", "%25", "%41", "%zz", "%E2%82%AC", "+", " ", "/", "?", "#", ":", "@", "[", "]", "!", "$", "&", "'", "(", ")", "*", ",", ";", "=", "-", ".", "_", "~", "\"", "<", ">", "\\", "^", "`", "{", "|", "}", "\x00", "\x1f", "\x7f", "\x80", "\xff", "\xc3\xbc", "\xe2\x82\xac", "\xf0\x9f\x98\x80", "http", "://", "..", "%2F", "%2f"}
)

func (g gen) rawURL() string {
	var b strings.Builder
	b.WriteString(g.pick(schemes))
	sl := g.pick(slashes)
	b.WriteString(sl)
	if sl == "//" || g.r.IntN(4) == 0 {
		b.WriteString(g.pick(userinfos))
		b.WriteString(g.pick(hosts))
	}
	b.WriteString(g.pick(paths))
	if g.r.IntN(3) == 0 {
		b.WriteString(g.pick(paths))
	}
	b.WriteString(g.pick(queries))
	b.WriteString(g.pick(frags))
	s := b.String()
	// random mutations
	for k := g.r.IntN(3); k > 0 && len(s) > 0; k-- {
		i := g.r.IntN(len(s) + 1)
		s = s[:i] + g.pick(soup) + s[i:]
	}
	return s
}

func (g gen) soupString() string {
	var b strings.Builder
	for k := g.r.IntN(8); k > 0; k-- {
		b.WriteString(g.pick(soup))
	}
	return b.String()
}

func (g gen) field(xs []string) string {
	if g.r.IntN(3) == 0 {
		return g.soupString()
	}
	return xs[g.r.IntN(len(xs))]
}

func (g gen) urlStruct() *url.URL {
	u := &url.URL{}
	if g.r.IntN(2) == 0 {
		u.Scheme = strings.TrimSuffix(g.pick(schemes), ":")
	}
	if g.r.IntN(6) == 0 {
		u.Opaque = g.field(paths)
	}
	switch g.r.IntN(4) {
	case 0:
		u.User = url.User(g.field([]string{"user", "u:v", "a@b", ""}))
	case 1:
		u.User = url.UserPassword(g.field([]string{"user", "u:v", ""}), g.field([]string{"pass", "p@ss", "", "a/b?"}))
	}
	if g.r.IntN(2) == 0 {
		u.Host = g.field(hosts)
	}
	u.Path = g.field(paths)
	if g.r.IntN(3) == 0 {
		u.RawPath = g.field(paths)
	}
	u.OmitHost = g.r.IntN(3) == 0
	u.ForceQuery = g.r.IntN(4) == 0
	if g.r.IntN(2) == 0 {
		u.RawQuery = strings.TrimPrefix(g.field(queries), "?")
	}
	if g.r.IntN(2) == 0 {
		u.Fragment = strings.TrimPrefix(g.field(frags), "#")
	}
	if g.r.IntN(3) == 0 {
		u.RawFragment = strings.TrimPrefix(g.field(frags), "#")
	}
	return u
}

func main() {
	out := flag.String("out", "", "output file")
	n := flag.Int("n", 400, "number of random cases per generator")
	seed := flag.Uint64("seed", 1, "random seed")
	inputs := flag.String("inputs", "", "optional file of extra inputs, one per line (e.g. URLs extracted from the golden site)")
	onlyInputs := flag.Bool("only-inputs", false, "emit only the records for -inputs")
	adv := flag.Int("adv", 0, "emit only N adversarial cases (see adversarial.go)")
	every := flag.Int("every", 1, "keep only every Nth record")
	flag.Parse()
	if *out == "" {
		log.Fatal("-out required")
	}
	g := gen{rand.New(rand.NewPCG(*seed, *seed^0x9e3779b97f4a7c15))}

	f, err := os.Create(*out)
	if err != nil {
		log.Fatal(err)
	}
	w := &writer{w: bufio.NewWriter(f), every: *every}

	if *adv > 0 {
		adversarial(w, g, *adv)
	} else if !*onlyInputs {
		table := encodingTable()
		var ts []string
		for _, v := range table {
			ts = append(ts, strconv.Itoa(v))
		}
		w.rec("Table", nil, ts...)

		singles, tuples := literals(filepath.Join(goroot(), "src/net/url/url_test.go"))
		seen := map[string]bool{}
		var pool []string
		for _, s := range singles {
			if !seen[s] {
				seen[s] = true
				pool = append(pool, s)
				single(w, s)
			}
		}
		for _, t := range tuples {
			resolve(w, t[0], t[1])
			resolve(w, t[1], t[0])
			joinPath(w, t[0], t[1:])
		}
		// Exhaustive scheme x userinfo x host grid (host parsing, strict colons).
		for _, sc := range []string{"http://", "https://", "HTTP://", "ftp://", "//", "mongodb://"} {
			for _, ui := range []string{"", "u:p@"} {
				for _, h := range hosts {
					single(w, sc+ui+h+"/p")
				}
			}
		}
		for i := 0; i < *n; i++ {
			joinPath(w, pool[g.r.IntN(len(pool))], []string{pool[g.r.IntN(len(pool))], g.pick(paths)})
			resolve(w, pool[g.r.IntN(len(pool))], pool[g.r.IntN(len(pool))])
		}

		for i := 0; i < *n; i++ {
			single(w, g.rawURL())
			single(w, g.soupString())
			resolve(w, g.rawURL(), g.rawURL())
			resolve(w, g.rawURL(), strings.TrimPrefix(g.pick(paths), "/")+g.pick(queries)+g.pick(frags))
			elems := make([]string, g.r.IntN(4))
			for j := range elems {
				elems[j] = g.field(paths)
			}
			joinPath(w, g.rawURL(), elems)
			structRec(w, g.urlStruct(), g.urlStruct(), elems)
		}
	}

	if *inputs != "" {
		data, err := os.ReadFile(*inputs)
		if err != nil {
			log.Fatal(err)
		}
		base := "https://seeksnack.com/th/ingredients/"
		for _, line := range strings.Split(string(data), "\n") {
			single(w, line)
			resolve(w, base, line)
			joinPath(w, base, []string{line})
		}
	}

	if err := w.w.Flush(); err != nil {
		log.Fatal(err)
	}
	if err := f.Close(); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "wrote %d records to %s\n", w.n, *out)
}
