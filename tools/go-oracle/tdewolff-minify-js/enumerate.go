package main

import (
	"bufio"
	"compress/gzip"
	"encoding/hex"
	"fmt"
	"os"
	"strings"
)

// enumerate OUT CFGS ALPHA MAXLEN: every sequence of 1..MAXLEN symbols of
// the alphabet (hex bytes, or comma-separated hex tokens), in length-major
// lexicographic order, minified with every configuration of CFGS
// (comma-separated). OUT (gzip text) has a header line
// "# enumerate CFGS ALPHA MAXLEN" and then one line per input: the FNV-1a
// digest of the concatenated digests of [output, err, after] of every
// configuration (on a Go panic, output and after are left empty). The Rust
// port's `examples/enumerate.rs` enumerates the same inputs and compares
// line by line, so no inputs are stored.
func enumerate(out string, cfgs []string, alphaHex string, maxLen int) {
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
	_, _ = fmt.Fprintf(w, "# enumerate %s %s %d\n", strings.Join(cfgs, ","), alphaHex, maxLen)
	n := 0
	for l := 1; l <= maxLen; l++ {
		idx := make([]int, l)
		for {
			var src []byte
			for _, k := range idx {
				src = append(src, alpha[k]...)
			}
			var all []byte
			for _, cfg := range cfgs {
				fields := runMin(cfg, src)
				if string(fields[1]) == "PANIC" {
					// only the panic itself is compared
					fields[0], fields[2] = nil, nil
				}
				for _, d := range digests(fields) {
					all = append(all, d...)
				}
			}
			_, _ = w.Write(digest(all))
			_ = w.WriteByte('\n')
			n++
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
