package main

import (
	"bufio"
	"bytes"
	_ "embed"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
)

//go:embed encode.go
var encodeSource string

// encode.go is only called from the temporary program (fmttestsMain); this
// keeps it type-checked as part of the oracle without tripping the unused
// linter.
var _ = encSpec

// The main function of the temporary program. It runs Go's own tables and
// prints "table \t fmt \t args-spec \t out" for every entry whose Sprintf
// result equals the table's expected output and whose operands the value
// model can express.
const fmttestsMain = `package main

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

func encRun() {
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	emit := func(table, f string, vals []any, want string) {
		if fmt.Sprintf(f, vals...) != want {
			fmt.Fprintf(w, "#mismatch\t%s\t%q\n", table, f)
			return
		}
		var specs []string
		for _, v := range vals {
			s, ok := encSpec(v)
			if !ok {
				fmt.Fprintf(w, "#unsupported\t%s\t%q\t%T\n", table, f, v)
				return
			}
			specs = append(specs, s)
		}
		fmt.Fprintf(w, "%s\t%s\targs(%s)\t%s\n", table, strconv.Quote(f), strings.Join(specs, ","), strconv.Quote(want))
	}
	for _, tt := range fmtTests {
		emit("fmtTests", tt.fmt, []any{tt.val}, tt.out)
	}
	for _, tt := range reorderTests {
		emit("reorderTests", tt.fmt, tt.val, tt.out)
	}
	for _, tt := range startests {
		emit("startests", tt.fmt, tt.in, tt.out)
	}
}

func main() { encRun() }
`

// extractFmtTests copies the package-level declarations of
// $GOROOT/src/fmt/fmt_test.go that the tables need into a package main.
func extractFmtTests() (string, error) {
	path := filepath.Join(runtime.GOROOT(), "src", "fmt", "fmt_test.go")
	src, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, path, src, parser.ParseComments)
	if err != nil {
		return "", err
	}
	wantFuncs := map[string]bool{"TestFmtInterface": true, "zeroFill": true, "args": true, "hideFromVet": true, "noliteral": true}
	var b strings.Builder
	b.WriteString(`package main

import (
	"bytes"
	. "fmt"
	"io"
	"math"
	"reflect"
	"strings"
	"testing"
	"time"
	"unicode"
)

var (
	_ = bytes.MinRead
	_ = Sprintf
	_ = io.EOF
	_ = math.Pi
	_ = reflect.Copy
	_ = strings.Repeat
	_ testing.T
	_ = time.Now
	_ = unicode.MaxRune
)

`)
	for _, d := range f.Decls {
		keep := false
		switch d := d.(type) {
		case *ast.GenDecl:
			keep = d.Tok != token.IMPORT
		case *ast.FuncDecl:
			keep = d.Recv != nil || wantFuncs[d.Name.Name]
		}
		if !keep {
			continue
		}
		start := fset.Position(d.Pos()).Offset
		end := fset.Position(d.End()).Offset
		b.Write(src[start:end])
		b.WriteString("\n\n")
	}
	return b.String(), nil
}

// runFmtTests builds and runs the extracted tables and returns the fixture
// lines plus statistics.
func runFmtTests(scratch string) (lines []string, stats string, err error) {
	dir, err := os.MkdirTemp(scratch, "fmttests")
	if err != nil {
		return nil, "", err
	}
	defer func() { _ = os.RemoveAll(dir) }()
	tables, err := extractFmtTests()
	if err != nil {
		return nil, "", err
	}
	files := map[string]string{
		"go.mod":    "module fmttests\n\ngo 1.24\n",
		"tables.go": tables,
		"encode.go": encodeSource,
		"main.go":   fmttestsMain,
	}
	for name, content := range files {
		if err := os.WriteFile(filepath.Join(dir, name), []byte(content), 0o644); err != nil {
			return nil, "", err
		}
	}
	cmd := exec.Command(filepath.Join(runtime.GOROOT(), "bin", "go"), "run", ".")
	cmd.Dir = dir
	cmd.Env = append(os.Environ(), "GOFLAGS=-mod=mod", "GOWORK=off")
	var stderr bytes.Buffer
	cmd.Stderr = &stderr
	out, err := cmd.Output()
	if err != nil {
		return nil, "", fmt.Errorf("running extracted fmt tables: %v\n%s", err, stderr.String())
	}
	var nOK, nMismatch, nUnsupported, nRoundTrip int
	sc := bufio.NewScanner(bytes.NewReader(out))
	sc.Buffer(make([]byte, 1<<20), 1<<24)
	for sc.Scan() {
		line := sc.Text()
		if strings.HasPrefix(line, "#mismatch") {
			nMismatch++
			continue
		}
		if strings.HasPrefix(line, "#unsupported") {
			nUnsupported++
			continue
		}
		parts := strings.Split(line, "\t")
		if len(parts) != 4 {
			return nil, "", fmt.Errorf("bad line %q", line)
		}
		// Round trip through this oracle's decoder: the decoded operands
		// must print exactly like the originals.
		f, err1 := strconv.Unquote(parts[1])
		want, err2 := strconv.Unquote(parts[3])
		vals, err3 := decodeArgs(parts[2])
		if err1 != nil || err2 != nil || err3 != nil || fmt.Sprintf(f, vals...) != want {
			nRoundTrip++
			log.Printf("dropped after decode round trip: %s", line)
			continue
		}
		nOK++
		lines = append(lines, line)
	}
	stats = fmt.Sprintf("fmt_test.go tables: %d kept, %d not expressible in the value model, %d with pointer/type-name output (PTR/fmt_test.*), %d dropped after decode round trip",
		nOK, nUnsupported, nMismatch, nRoundTrip)
	return lines, stats, nil
}

func decodeArgs(spec string) ([]any, error) {
	n, err := parseSpec(spec)
	if err != nil {
		return nil, err
	}
	if n.name != "args" {
		return nil, fmt.Errorf("want args node, got %q", n.name)
	}
	var vals []any
	for _, c := range n.children {
		v, err := decodeNode(c)
		if err != nil {
			return nil, err
		}
		vals = append(vals, v)
	}
	return vals, nil
}
