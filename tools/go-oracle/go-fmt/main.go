// Command go-fmt is the Go oracle for crates/go-fmt.
//
// Usage:
//
//	go build -o oracle ./tools/go-oracle/go-fmt
//	./oracle -mode vectors -dir crates/go-fmt/tests/fixtures
//	./oracle -mode dump -value 17 > dump.txt
//
// "vectors" writes:
//   - values.txt: the operand corpus as value specs (see spec.go);
//   - formats.txt: the single-operand format matrix (Go-quoted, one per line);
//   - matrix.txt: per operand, the FNV-1a hash of Sprintf(format, operand)
//     over every format, and the format indexes whose output is not
//     deterministic in Go (pointer addresses), which both sides skip;
//   - cases.txt: explicit Sprint/Sprintln/Sprintf/Errorf cases with random
//     multi-operand formats (star width/precision, argument indexes, errors):
//     "fn \t format \t operands \t output [\t wrapped]" where operands are
//     space-separated "@N" (index into values.txt) or atomic specs, and
//     wrapped (errorf only) lists the operand indexes that Unwrap returns;
//   - fmttests.txt: the entries of Go's own fmt_test.go tables (fmtTests,
//     reorderTests, startests) whose operands the value model can express;
//   - fuzz_cases.txt, fuzz_formats.txt, fuzz_matrix.txt: randomized vectors
//     (see fuzz.go) with the default seed;
//   - model_cases.txt: hand-picked operands for the value-model mappings
//     (named collection Stringers, interface nils in containers, GoStringer,
//     nil *time.Location), in the cases.txt format.
//
// "fuzz" writes only the randomized vectors, with -seed, -n (cases),
// -nmatrix (dense matrix operands) and -maxout; the Rust tests read them
// from GO_FMT_FUZZ_DIR for large runs outside the checked-in fixtures.
//
// "dump" prints Sprintf(format, operand) for every format of one operand
// (Go-quoted, one per line); the Rust test writes the same file to
// GO_FMT_DUMP_DIR on a hash mismatch, so the two can be diffed.
package main

import (
	"bufio"
	"encoding/binary"
	"errors"
	"flag"
	"fmt"
	"hash/fnv"
	"log"
	"math/rand/v2"
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"strings"
)

func main() {
	mode := flag.String("mode", "vectors", "vectors | dump | fuzz")
	dir := flag.String("dir", ".", "output directory for -mode vectors")
	value := flag.Int("value", 0, "operand index for -mode dump")
	scratch := flag.String("scratch", os.TempDir(), "scratch directory for the fmt_test.go table program")
	seed := flag.Uint64("seed", 1, "random seed for -mode fuzz")
	nCases := flag.Int("n", 20000, "number of random cases for -mode fuzz")
	nMatrix := flag.Int("nmatrix", 300, "number of random operands of the dense matrix for -mode fuzz")
	maxOut := flag.Int("maxout", 4000, "longest output kept by -mode fuzz")
	flag.StringVar(&fuzzMatrixKinds, "kinds", "", "restrict the -mode fuzz matrix operands: float | int | str")
	flag.Parse()
	log.SetFlags(0)
	log.SetPrefix("go-fmt oracle: ")

	switch *mode {
	case "vectors":
		writeVectors(*dir, *scratch)
		writeFuzz(*dir, 1, 6000, 150, 4000)
		log.Printf("model cases: %d", writeModelCases(*dir))
	case "fuzz":
		writeFuzz(*dir, *seed, *nCases, *nMatrix, *maxOut)
	case "dump":
		vs := valueCorpus()
		v, err := decodeSpec(vs[*value])
		if err != nil {
			log.Fatal(err)
		}
		w := bufio.NewWriter(os.Stdout)
		for _, f := range formatMatrix() {
			fmt.Fprintln(w, strconv.Quote(fmt.Sprintf(f, v)))
		}
		if err := w.Flush(); err != nil {
			log.Fatal(err)
		}
	default:
		log.Fatalf("unknown mode %q", *mode)
	}
}

func create(dir, name string) (*os.File, *bufio.Writer) {
	f, err := os.Create(filepath.Join(dir, name))
	if err != nil {
		log.Fatal(err)
	}
	return f, bufio.NewWriter(f)
}

func finish(f *os.File, w *bufio.Writer) {
	if err := w.Flush(); err != nil {
		log.Fatal(err)
	}
	if err := f.Close(); err != nil {
		log.Fatal(err)
	}
}

// pointerLike reports whether %p of v prints an address (which may be
// runtime.zerobase for empty slices and so look deterministic).
func pointerLike(v any) bool {
	if v == nil {
		return false
	}
	rv := reflect.ValueOf(v)
	switch rv.Kind() {
	case reflect.Slice, reflect.Map, reflect.Pointer, reflect.Func, reflect.Chan:
		return !rv.IsNil()
	}
	return false
}

func mustDecode(spec string) any {
	v, err := decodeSpec(spec)
	if err != nil {
		log.Fatalf("%s: %v", spec, err)
	}
	return v
}

func writeVectors(dir, scratch string) {
	values := valueCorpus()
	formats := formatMatrix()

	f, w := create(dir, "values.txt")
	for _, v := range values {
		fmt.Fprintln(w, v)
	}
	finish(f, w)

	f, w = create(dir, "formats.txt")
	for _, s := range formats {
		fmt.Fprintln(w, strconv.Quote(s))
	}
	finish(f, w)

	// The matrix: hash per operand.
	f, w = create(dir, "matrix.txt")
	total, skipped := 0, 0
	for _, spec := range values {
		v1 := mustDecode(spec)
		v2 := mustDecode(spec)
		h := fnv.New64a()
		var nondet []int
		for i, format := range formats {
			o1 := fmt.Sprintf(format, v1)
			o2 := fmt.Sprintf(format, v2)
			if o1 != o2 || (strings.ContainsRune(format, 'p') && pointerLike(v1)) {
				nondet = append(nondet, i)
				skipped++
				continue
			}
			var n [8]byte
			binary.LittleEndian.PutUint64(n[:], uint64(len(o1)))
			h.Write(n[:])
			h.Write([]byte(o1))
			total++
		}
		fmt.Fprintf(w, "%s\t%016x\t%s\n", spec, h.Sum64(), ranges(nondet))
	}
	finish(f, w)
	log.Printf("matrix: %d operands x %d formats: %d outputs hashed, %d nondeterministic skipped", len(values), len(formats), total, skipped)

	// Explicit cases.
	f, w = create(dir, "cases.txt")
	n := writeCases(w, values)
	finish(f, w)
	log.Printf("cases: %d", n)

	// Go's own tables.
	lines, stats, err := runFmtTests(scratch)
	if err != nil {
		log.Fatal(err)
	}
	f, w = create(dir, "fmttests.txt")
	for _, l := range lines {
		fmt.Fprintln(w, l)
	}
	finish(f, w)
	log.Print(stats)
}

// ranges formats sorted indexes as "a-b,c,...".
func ranges(xs []int) string {
	var parts []string
	for i := 0; i < len(xs); {
		j := i
		for j+1 < len(xs) && xs[j+1] == xs[j]+1 {
			j++
		}
		if j == i {
			parts = append(parts, strconv.Itoa(xs[i]))
		} else {
			parts = append(parts, strconv.Itoa(xs[i])+"-"+strconv.Itoa(xs[j]))
		}
		i = j + 1
	}
	return strings.Join(parts, ",")
}

// Random multi-operand cases.

var caseVerbs = []string{"v", "v", "v", "s", "s", "d", "d", "q", "x", "X", "t", "b", "o", "O", "c", "U", "e", "f", "g", "G", "T", "w", "%", "z", "é"}

func randFormat(r *rand.Rand, nargs int) string {
	var b strings.Builder
	pieces := 1 + r.IntN(4)
	idx := func() string { return "[" + strconv.Itoa(r.IntN(nargs+2)) + "]" }
	for p := 0; p < pieces; p++ {
		switch r.IntN(6) {
		case 0:
			lits := []string{"ab", " ", "日", "%%", "\xff", "x=", "[", "]", "*"}
			b.WriteString(lits[r.IntN(len(lits))])
			continue
		}
		b.WriteByte('%')
		for _, c := range "+-# 0" {
			if r.IntN(5) == 0 {
				b.WriteRune(c)
			}
		}
		if r.IntN(6) == 0 {
			b.WriteString(idx())
		}
		switch r.IntN(6) {
		case 0:
			b.WriteByte('*')
		case 1:
			b.WriteString(idx() + "*")
		case 2, 3:
			b.WriteString(strconv.Itoa(r.IntN(12)))
		}
		if r.IntN(3) == 0 {
			b.WriteByte('.')
			switch r.IntN(5) {
			case 0:
				b.WriteByte('*')
			case 1:
				b.WriteString(idx() + "*")
			case 2, 3:
				b.WriteString(strconv.Itoa(r.IntN(8)))
			}
		}
		if r.IntN(8) == 0 {
			b.WriteString(idx())
		}
		if r.IntN(30) == 0 {
			// Truncated directive at the end of the format.
			break
		}
		b.WriteString(caseVerbs[r.IntN(len(caseVerbs))])
	}
	return b.String()
}

var starArgs = []string{"int:3", "int:-4", "int:0", "int:12", "int:1000001", "int:-1000001", "uint:5", "uint8:2",
	"uint64:18446744073709551615", "int64:-9223372036854775808", "int32:7", "str:35", "nil", "f64:4014000000000000"}

// randArgs returns operands as "@N" (index into values.txt) or atomic specs.
func randArgs(r *rand.Rand, values []string, n int) []string {
	var args []string
	for i := 0; i < n; i++ {
		if r.IntN(3) == 0 {
			args = append(args, starArgs[r.IntN(len(starArgs))])
		} else {
			args = append(args, "@"+strconv.Itoa(r.IntN(len(values))))
		}
	}
	return args
}

func writeCases(w *bufio.Writer, values []string) int {
	r := rand.New(rand.NewPCG(1, 2))
	count := 0
	emit := func(fn, format string, args []string, out string, extra string) {
		fmt.Fprintf(w, "%s\t%s\t%s\t%s%s\n", fn, strconv.Quote(format), strings.Join(args, " "), strconv.Quote(out), extra)
		count++
	}
	decodeAll := func(args []string) []any {
		var vs []any
		for _, a := range args {
			if strings.HasPrefix(a, "@") {
				i, err := strconv.Atoi(a[1:])
				if err != nil {
					log.Fatal(err)
				}
				a = values[i]
			}
			vs = append(vs, mustDecode(a))
		}
		return vs
	}
	// Sprint / Sprintln / Append*: exercises the operand-spacing rule.
	for i := 0; i < 2500; i++ {
		args := randArgs(r, values, r.IntN(6))
		fn := "sprint"
		if i%5 == 0 {
			fn = "sprintln"
		}
		a1, a2 := decodeAll(args), decodeAll(args)
		var o1, o2 string
		if fn == "sprint" {
			o1, o2 = fmt.Sprint(a1...), fmt.Sprint(a2...)
		} else {
			o1, o2 = fmt.Sprintln(a1...), fmt.Sprintln(a2...)
		}
		if o1 != o2 {
			continue
		}
		emit(fn, "", args, o1, "")
	}
	// Sprintf with random formats.
	for i := 0; i < 7000; i++ {
		nargs := r.IntN(6)
		args := randArgs(r, values, nargs)
		format := randFormat(r, nargs)
		a1, a2 := decodeAll(args), decodeAll(args)
		o1, o2 := fmt.Sprintf(format, a1...), fmt.Sprintf(format, a2...)
		// Skip nondeterministic output and huge paddings (star widths of 1e6).
		if o1 != o2 || strings.Contains(format, "p") || len(o1) > 2000 {
			continue
		}
		emit("sprintf", format, args, o1, "")
	}
	// Errorf with %w.
	errArgs := []string{sObj("obj_err", "e1"), sObj("obj_err", "e2"), sObj("obj_both", "b"), sObj("obj_str", "s"), "int:1", "nil", sStr("x"), "@5", "@200"}
	wformats := []string{"%w", "x: %w", "%w %w", "%[2]w %[1]w", "%w %[1]w", "%v %w", "%#w", "%+w", "%5w", "%w %d", "no verbs", "", "%[3]w %w %w", "%w%%"}
	for i := 0; i < 600; i++ {
		format := wformats[r.IntN(len(wformats))]
		var args []string
		for j := 0; j < r.IntN(4); j++ {
			args = append(args, errArgs[r.IntN(len(errArgs))])
		}
		a1 := decodeAll(args)
		err := fmt.Errorf(format, a1...)
		var wrapped []error
		switch u := err.(type) {
		case interface{ Unwrap() []error }:
			wrapped = u.Unwrap()
		case interface{ Unwrap() error }:
			if e := u.Unwrap(); e != nil {
				wrapped = []error{e}
			}
		}
		var idx []string
		for _, we := range wrapped {
			for k, a := range a1 {
				if ae, ok := a.(error); ok && errors.Is(ae, we) && ae == we {
					idx = append(idx, strconv.Itoa(k))
					break
				}
			}
		}
		emit("errorf", format, args, err.Error(), "\t"+strings.Join(idx, ","))
	}
	return count
}
