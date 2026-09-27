// Command go-path is the Go oracle for the Rust crate crates/go-path.
//
//	go run ./tools/go-oracle/go-path -out crates/go-path/tests/fixtures/path.txt [-n N] [-seed S]
//	go run ./tools/go-oracle/go-path -out crates/go-path/tests/fixtures/adversarial.txt -adv N [-seed S]
//
// Inputs are every string literal (and every tuple of string literals inside
// one composite literal) of $GOROOT/src/path/*_test.go and
// $GOROOT/src/path/filepath/*_test.go, plus random paths and patterns.
// Each output line is
//
//	op \t nargs \t arg... \t result...
//
// with every field encoded by esc() (printable ASCII except '\', else \xHH).
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
	"os"
	"os/exec"
	"path"
	"path/filepath"
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
// that are direct elements of one composite literal) of the given test files.
func literals(globs ...string) (singles []string, tuples [][]string) {
	for _, g := range globs {
		files, _ := filepath.Glob(g)
		for _, fn := range files {
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
		}
	}
	return
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

func boolStr(b bool) string {
	if b {
		return "true"
	}
	return "false"
}

func errStr(err error) string {
	if err == nil {
		return "<nil>"
	}
	return err.Error()
}

func single(w *writer, s string) {
	a := []string{s}
	w.rec("path.Clean", a, path.Clean(s))
	d, f := path.Split(s)
	w.rec("path.Split", a, d, f)
	w.rec("path.Ext", a, path.Ext(s))
	w.rec("path.Base", a, path.Base(s))
	w.rec("path.Dir", a, path.Dir(s))
	w.rec("path.IsAbs", a, boolStr(path.IsAbs(s)))
	w.rec("filepath.Clean", a, filepath.Clean(s))
	d, f = filepath.Split(s)
	w.rec("filepath.Split", a, d, f)
	w.rec("filepath.Ext", a, filepath.Ext(s))
	w.rec("filepath.Base", a, filepath.Base(s))
	w.rec("filepath.Dir", a, filepath.Dir(s))
	w.rec("filepath.IsAbs", a, boolStr(filepath.IsAbs(s)))
	w.rec("filepath.IsLocal", a, boolStr(filepath.IsLocal(s)))
	w.rec("filepath.ToSlash", a, filepath.ToSlash(s))
	w.rec("filepath.FromSlash", a, filepath.FromSlash(s))
	w.rec("filepath.VolumeName", a, filepath.VolumeName(s))
	w.rec("filepath.SplitList", a, filepath.SplitList(s)...)
}

func pair(w *writer, a, b string) {
	args := []string{a, b}
	m, err := path.Match(a, b)
	w.rec("path.Match", args, boolStr(m), errStr(err))
	m, err = filepath.Match(a, b)
	w.rec("filepath.Match", args, boolStr(m), errStr(err))
	r, err := filepath.Rel(a, b)
	w.rec("filepath.Rel", args, r, errStr(err))
}

var (
	pathMatch     = path.Match
	filepathMatch = filepath.Match
)

func join(w *writer, elems []string) {
	w.rec("path.Join", elems, path.Join(elems...))
	w.rec("filepath.Join", elems, filepath.Join(elems...))
}

func main() {
	out := flag.String("out", "", "output file")
	n := flag.Int("n", 1000, "number of random cases per generator")
	seed := flag.Uint64("seed", 1, "random seed")
	inputs := flag.String("inputs", "", "optional file of extra paths, one per line (e.g. files of the golden site)")
	onlyInputs := flag.Bool("only-inputs", false, "emit only the records for -inputs")
	adv := flag.Int("adv", 0, "emit only N adversarial cases (see adversarial.go)")
	every := flag.Int("every", 1, "keep only every Nth record")
	flag.Parse()
	if *out == "" {
		log.Fatal("-out required")
	}
	rng := rand.New(rand.NewPCG(*seed, *seed^0x9e3779b97f4a7c15))

	f, err := os.Create(*out)
	if err != nil {
		log.Fatal(err)
	}
	w := &writer{w: bufio.NewWriter(f), every: *every}

	if *adv > 0 {
		adversarial(w, rng, *adv)
	} else if !*onlyInputs {
		root := goroot()
		singles, tuples := literals(filepath.Join(root, "src/path/*_test.go"), filepath.Join(root, "src/path/filepath/*_test.go"))
		seen := map[string]bool{}
		for _, s := range singles {
			if !seen[s] {
				seen[s] = true
				single(w, s)
			}
		}
		for _, t := range tuples {
			join(w, t)
			pair(w, t[0], t[1])
			pair(w, t[1], t[0])
		}

		pathParts := []string{"/", "/", "/", ".", "..", "a", "b", "abc", "", "\\", ":", ".x", "x.", "é", "\xff", " ", "~", "//", "./", "../"}
		randPath := func() string {
			var b strings.Builder
			for k := rng.IntN(9); k > 0; k-- {
				b.WriteString(pathParts[rng.IntN(len(pathParts))])
			}
			return b.String()
		}
		for i := 0; i < *n; i++ {
			single(w, randPath())
			elems := make([]string, rng.IntN(5))
			for j := range elems {
				elems[j] = randPath()
			}
			join(w, elems)
			pair(w, randPath(), randPath())
		}

		patParts := []string{"*", "*", "?", "[", "]", "^", "-", "\\", "a", "b", "c", "/", "é", "[a-c]", "[^a]", "\\*", "[\\]]", "[é-ü]", "\xff", "[a-", "[]"}
		nameParts := []string{"a", "b", "c", "/", "é", "ü", "*", "-", "]", "\xff", "[", "\\", "ab"}
		for i := 0; i < *n*3; i++ {
			var p, s strings.Builder
			for k := rng.IntN(7); k > 0; k-- {
				p.WriteString(patParts[rng.IntN(len(patParts))])
			}
			for k := rng.IntN(7); k > 0; k-- {
				s.WriteString(nameParts[rng.IntN(len(nameParts))])
			}
			args := []string{p.String(), s.String()}
			m, err := path.Match(args[0], args[1])
			w.rec("path.Match", args, boolStr(m), errStr(err))
			m, err = filepath.Match(args[0], args[1])
			w.rec("filepath.Match", args, boolStr(m), errStr(err))
		}
	}

	if *inputs != "" {
		data, err := os.ReadFile(*inputs)
		if err != nil {
			log.Fatal(err)
		}
		lines := strings.Split(string(data), "\n")
		for i, line := range lines {
			single(w, line)
			single(w, "/"+line)
			join(w, []string{"public", line, "..", "index.html"})
			pair(w, "/site/public", "/site/public/"+line)
			pair(w, line, lines[(i+1)%len(lines)])
			pair(w, "*/*.html", line)
			pair(w, "[a-z]*/*/index.*", line)
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
