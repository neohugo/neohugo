package main

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"
)

// reparse IN.rec.gz OUT.rec.gz: red-team runs that reuse the input
// generators of the minify-js oracle (tools/go-oracle/tdewolff-minify-js
// `gen`/`files`: literal-heavy programs, token soup, byte-mutated corpus
// windows, generated programs, whole files). It reads the inputs of IN
// (`min` and `multi` records: field 2; `html` records are skipped) and
// writes OUT in the fuzz.rec.gz format (input + the digests of every mode),
// which the Rust port checks with `examples/check.rs`.
func reparse(in, out string) {
	f, err := os.Open(in)
	if err != nil {
		panic(err)
	}
	gz, err := gzip.NewReader(f)
	if err != nil {
		panic(err)
	}
	data, err := io.ReadAll(gz)
	if err != nil {
		panic(err)
	}
	_ = f.Close()

	w := newRecWriter(out)
	n := 0
	for pos := 0; pos < len(data); {
		line := func() []byte {
			e := bytes.IndexByte(data[pos:], '\n')
			l := data[pos : pos+e]
			pos += e + 1
			return l
		}
		k, err := strconv.Atoi(string(bytes.TrimPrefix(line(), []byte("#rec "))))
		if err != nil {
			panic(err)
		}
		var fields [][]byte
		for i := 0; i < k; i++ {
			l, err := strconv.Atoi(string(line()))
			if err != nil {
				panic(err)
			}
			fields = append(fields, data[pos:pos+l])
			pos += l + 1
		}
		if len(fields) < 3 {
			continue // the generator comment
		}
		switch string(fields[0]) {
		case "min", "multi":
		default:
			continue
		}
		src := fields[2]
		rec := [][]byte{src}
		for _, m := range modes {
			rec = append(rec, digest(runMode(m, src)))
		}
		w.rec(rec...)
		n++
	}
	w.close()
	fmt.Fprintf(os.Stderr, "reparse: %d inputs\n", n)
}

// enumerate OUT ALPHAHEX MAXLEN: every sequence of 1..MAXLEN symbols of
// the alphabet (hex bytes, or comma-separated hex tokens), in length-major
// lexicographic order. OUT (gzip text) has a header line
// "# enumerate ALPHAHEX MAXLEN" and then one line per input: the FNV-1a
// digest of the concatenated digests of every mode. The Rust port
// (`examples/enumerate.rs`, `tests/fixtures.rs`) enumerates the same
// inputs and compares line by line, so no inputs are stored.
func enumerate(out, alphaHex string, maxLen int) {
	// the alphabet is a hex string of single bytes, or a comma-separated
	// list of hex-encoded tokens
	var alpha [][]byte
	if strings.Contains(alphaHex, ",") {
		for _, t := range strings.Split(alphaHex, ",") {
			b, err := hex.DecodeString(t)
			if err != nil {
				panic(err)
			}
			alpha = append(alpha, b)
		}
	} else {
		b, err := hex.DecodeString(alphaHex)
		if err != nil {
			panic(err)
		}
		for _, c := range b {
			alpha = append(alpha, []byte{c})
		}
	}
	f, err := os.Create(out)
	if err != nil {
		panic(err)
	}
	gz, _ := gzip.NewWriterLevel(f, gzip.BestCompression)
	w := bufio.NewWriter(gz)
	_, _ = fmt.Fprintf(w, "# enumerate %s %d\n", alphaHex, maxLen)
	n := 0
	for l := 1; l <= maxLen; l++ {
		idx := make([]int, l)
		for {
			var src []byte
			for _, k := range idx {
				src = append(src, alpha[k]...)
			}
			var all []byte
			for _, m := range modes {
				all = append(all, digest(runMode(m, src))...)
			}
			_, _ = w.Write(digest(all))
			_ = w.WriteByte('\n')
			n++
			// next index vector
			i := l - 1
			for ; 0 <= i; i-- {
				idx[i]++
				if idx[i] < len(alpha) {
					break
				}
				idx[i] = 0
			}
			if i < 0 {
				break
			}
		}
	}
	if err := w.Flush(); err != nil {
		panic(err)
	}
	if err := gz.Close(); err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
	fmt.Fprintf(os.Stderr, "enumerate: %d inputs\n", n)
}
