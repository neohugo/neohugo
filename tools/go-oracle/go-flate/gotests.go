// The inputs and write/flush/reset programs of Go's own compress/flate and
// compress/zlib tests (deflate_test.go, writer_test.go, zlib/writer_test.go)
// as byte-exact oracle cases (Go's tests only round-trip; here the exact
// compressed bytes and Write-call sequence are compared).
//
// goTestData must stay in sync with go_test_data in
// crates/go-flate/tests/common/mod.rs.

package main

import (
	"bufio"
	"bytes"
	"fmt"
	"os"
	"strconv"
	"strings"
)

// bestSpeedCases is TestBestSpeed's testCases (tc[0] is replaced by firstN).
var bestSpeedCases = [][]int{
	{65536, 0},
	{65536, 1},
	{65536, 1, 256},
	{65536, 1, 65536},
	{65536, 14},
	{65536, 15},
	{65536, 16},
	{65536, 16, 256},
	{65536, 16, 65536},
	{65536, 127},
	{65536, 128},
	{65536, 128, 256},
	{65536, 128, 65536},
	{65536, 129},
	{65536, 65536, 256},
	{65536, 65536, 65536},
}

// goTestData resolves "go:<name>:<args...>" data specs.
func goTestData(spec string) []byte {
	p := strings.Split(spec, ":")
	arg := func(i int) int {
		v, err := strconv.Atoi(p[i])
		if err != nil {
			panic(spec)
		}
		return v
	}
	var out []byte
	switch p[1] {
	case "abcseq": // TestBestSpeed: concat(abcabc[:n] for n in tc)
		abc := make([]byte, 128)
		for i := range abc {
			abc[i] = byte(i)
		}
		abcabc := bytes.Repeat(abc, 131072/len(abc))
		tc := append([]int(nil), bestSpeedCases[arg(2)]...)
		tc[0] = arg(3)
		for _, n := range tc {
			out = append(out, abcabc[:n]...)
		}
	case "maxoff": // TestBestSpeedMaxMatchOffset
		const abc, xyz = "abcdefgh", "stuvwxyz"
		matchBefore, extra, offsetAdj := arg(2) == 1, arg(3), arg(4)
		offset := maxMatchOffset + offsetAdj
		out = make([]byte, offset+len(abc)+extra)
		copy(out, abc)
		if !matchBefore {
			copy(out[offset-len(xyz):], xyz)
		}
		copy(out[offset:], abc)
	case "hello": // TestWriterReset (testResetOutput)
		out = bytes.Repeat([]byte("hello world - how are you doing?"), arg(2))
	case "world": // TestWriterReset dictionary
		out = []byte(strings.Repeat("we are the world - how are you?", 3))
	case "asd": // TestWriteError / TestWriter_Reset
		var b bytes.Buffer
		for i := 0; i < arg(2); i++ {
			fmt.Fprintf(&b, "asdasfasf%d%dfghfgujyut%dyutyu\n", i, i, i)
		}
		out = b.Bytes()
	case "asdf": // TestDeflateFast_Reset
		var b bytes.Buffer
		for i := 0; i < arg(2); i++ {
			fmt.Fprintf(&b, "asdfasdfasdfasdf%d%dfghfgujyut%dyutyu\n", i, i, i)
		}
		out = bytes.Repeat(b.Bytes(), arg(3))
	case "det": // TestDeterministic (values 0..7; splitmix instead of math/rand)
		r := &rng{s: 1}
		out = make([]byte, arg(2))
		for i := range out {
			out[i] = byte(r.next() & 7)
		}
	case "zeros": // TestRegression2508 / TestMaxStackSize / TestVeryLongSparseChunk
		out = make([]byte, arg(2))
	case "sparse": // TestVeryLongSparseChunk's sparseReader (l bytes, last 64 KiB ones)
		l := arg(2)
		out = make([]byte, l)
		for i := max(0, l-1<<16); i < l; i++ {
			out[i] = 1
		}
	default:
		panic("bad go: spec " + spec)
	}
	return out
}

const maxMatchOffset = 1 << 15

func cmdGoTests(args []string) {
	var cases []caseSpec
	add := func(wrapper string, level int, data, dict string, ops ...string) {
		cases = append(cases, caseSpec{wrapper, level, data, dict, ops})
	}
	w := func(n int) string { return "W" + strconv.Itoa(n) }

	// TestBestSpeed at every level, with and without Flush after each Write.
	for _, level := range allLevels {
		for i, tc := range bestSpeedCases {
			for _, firstN := range []int{1, 65534, 65535, 65536, 65537, 131072} {
				for _, flush := range []bool{false, true} {
					var ops []string
					for j, n := range tc {
						if j == 0 {
							n = firstN
						}
						ops = append(ops, w(n))
						if flush {
							ops = append(ops, "F")
						}
					}
					add("flate", level, fmt.Sprintf("go:abcseq:%d:%d", i, firstN), "none", append(ops, "C")...)
				}
			}
		}
	}
	// TestBestSpeedMaxMatchOffset at every level.
	for _, level := range allLevels {
		for _, mb := range []int{0, 1} {
			for _, extra := range []int{0, 14, 15, 16, 30} {
				for adj := -5; adj <= 5; adj++ {
					n := len(goTestData(fmt.Sprintf("go:maxoff:%d:%d:%d", mb, extra, adj)))
					add("flate", level, fmt.Sprintf("go:maxoff:%d:%d:%d", mb, extra, adj), "none", w(n), "C")
				}
			}
		}
	}
	// TestWriterReset / testResetOutput: 1024 writes, Close, Reset, 1024 writes, Close.
	hello := len("hello world - how are you doing?")
	for _, level := range allLevels {
		for _, dict := range []string{"none", "go:world"} {
			wr := fmt.Sprintf("W%dx1024", hello)
			add("flate", level, "go:hello:2048", dict, wr, "C", "R", wr, "C")
			add("zlib", level, "go:hello:2048", dict, wr, "C", "R", wr, "C")
		}
	}
	// TestWriteError: io.CopyBuffer with a 128-byte buffer into a sink that
	// fails after `fail` writes; then Write, Flush, Close, Reset, Write, Close.
	asd := len(goTestData("go:asd:65536"))
	for level := 0; level <= 9; level++ {
		for fail := 1; fail <= 256; fail *= 2 {
			ops := []string{fmt.Sprintf("W128x%d", asd/128), w(asd % 128), "W0", "F", "C", "R", "W0", "C"}
			add("flate!c"+strconv.Itoa(fail+1), level, "go:asd:65536", "none", ops...)
			add("flate!p"+strconv.Itoa(fail+1), level, "go:asd:65536", "none", ops...)
		}
	}
	// TestWriterPersistentWriteError: failWriter{i} (the i-th call fails).
	for i := 1; i <= 40; i++ {
		add("flate!c"+strconv.Itoa(i), -1, "file:Isaac.Newton-Opticks.txt", "none")
	}
	// TestDeflateFast_Reset: the input written 3 times, fresh writer.
	for level := 1; level <= 6; level++ {
		n := len(goTestData("go:asdf:65536:1"))
		add("flate", level, "go:asdf:65536:3", "none", w(n), w(n), w(n), "C")
	}
	// TestDeterministic: two chunkings of the same input must give Go's bytes.
	det := 65535*30 + 500
	for _, level := range allLevels {
		add("flate", level, fmt.Sprintf("go:det:%d", det), "none", fmt.Sprintf("W787x%d", det/787), w(det%787), "C")
		add("flate", level, fmt.Sprintf("go:det:%d", det), "none", fmt.Sprintf("W81761x%d", det/81761), w(det%81761), "C")
	}
	// TestRegression2508: 131072 writes of 1024 zero bytes at level 1 (and all levels).
	add("flate", 1, "go:zeros:134217728", "none", "W1024x131072", "C")
	for _, level := range allLevels {
		add("flate", level, "go:zeros:16777216", "none", "W1024x16384", "C")
	}
	// TestMaxStackSize: one 1 MiB write at every level, Close, Reset.
	for _, level := range allLevels {
		add("flate", level, "go:zeros:1048576", "none", "W1048576", "C", "R", "C")
	}
	// TestVeryLongSparseChunk (scaled down to 64 MiB): io.Copy with a 32 KiB buffer.
	add("flate", 1, "go:sparse:67108864", "none", "W32768x2048", "C")

	out := bufio.NewWriter(os.Stdout)
	defer func() { _ = out.Flush() }()
	_, _ = fmt.Fprintf(out, "# go-flate oracle (%s) gotests\n", goVersion())
	files := map[string][]byte{}
	for _, name := range args {
		b, err := os.ReadFile(name)
		if err != nil {
			panic(err)
		}
		files[name[strings.LastIndexByte(name, '/')+1:]] = b
	}
	for i, c := range cases {
		if len(c.ops) == 0 {
			// Persistent-error program over the first 10000 bytes of the file:
			// Write, Close, Flush (the Go test's order).
			c.ops = []string{"W10000", "C", "F"}
		}
		_, _ = fmt.Fprintln(out, formatCase(i, c, files))
	}
}
