package main

// nfd-dump prints, for every code point whose x/text NFD (or NFKD) differs
// from itself, "U+XXXX\tNFD hex\tNFKD hex". Used to quantify how far the
// Rust unicode-normalization crate (a newer Unicode version) is from
// x/text's Unicode 15.0.0 tables; the crate itself ports x/text's norm
// subset instead of using unicode-normalization.

import (
	"bufio"
	"encoding/hex"
	"fmt"
	"os"

	"golang.org/x/text/unicode/norm"
)

func cmdNFDDump(args []string) error {
	w := bufio.NewWriter(os.Stdout)
	_, _ = fmt.Fprintf(w, "# x/text norm.Version=%s\n", norm.Version)
	for r := rune(0); r <= 0x10FFFF; r++ {
		if r >= 0xD800 && r <= 0xDFFF {
			continue
		}
		s := string(r)
		d := norm.NFD.String(s)
		k := norm.NFKD.String(s)
		if d != s || k != s {
			_, _ = fmt.Fprintf(w, "U+%04X\t%s\t%s\n", r, hex.EncodeToString([]byte(d)), hex.EncodeToString([]byte(k)))
		}
	}
	return w.Flush()
}
