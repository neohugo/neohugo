package main

// gen writes the binary table blobs embedded by crates/xtext-collate:
//
//	collate.bin  golang.org/x/text/collate tables.go (CLDR 23 / UCA 6.2.0)
//	norm.bin     golang.org/x/text/unicode/norm tables15.0.0.go (subset)
//	unicode.bin  unicode.Ideographic and unicode.Nd of the running toolchain
//
// Blob format: magic "XTBLOB01", then sections until EOF:
//
//	u8 name length, name bytes, u8 element width (1, 2, 4 or 8), u32 count,
//	count little-endian elements of that width.

import (
	"bytes"
	"encoding/binary"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"unicode"
)

type blob struct {
	buf bytes.Buffer
}

func newBlob() *blob {
	b := &blob{}
	b.buf.WriteString("XTBLOB01")
	return b
}

func (b *blob) section(name string, width int, vals []int64) error {
	if len(name) > 255 {
		return fmt.Errorf("section name too long")
	}
	b.buf.WriteByte(byte(len(name)))
	b.buf.WriteString(name)
	b.buf.WriteByte(byte(width))
	var tmp [4]byte
	binary.LittleEndian.PutUint32(tmp[:], uint32(len(vals)))
	b.buf.Write(tmp[:])
	for _, v := range vals {
		if width < 8 {
			var max int64 = 1<<(8*width) - 1
			if v < 0 || v > max {
				return fmt.Errorf("section %s: value %d does not fit in %d bytes", name, v, width)
			}
		}
		switch width {
		case 1:
			b.buf.WriteByte(byte(v))
		case 2:
			binary.LittleEndian.PutUint16(tmp[:2], uint16(v))
			b.buf.Write(tmp[:2])
		case 4:
			binary.LittleEndian.PutUint32(tmp[:], uint32(v))
			b.buf.Write(tmp[:])
		case 8:
			var t8 [8]byte
			binary.LittleEndian.PutUint64(t8[:], uint64(v))
			b.buf.Write(t8[:])
		default:
			return fmt.Errorf("bad width %d", width)
		}
	}
	return nil
}

func (b *blob) bytesSection(name string, data []byte) error {
	vals := make([]int64, len(data))
	for i, c := range data {
		vals[i] = int64(c)
	}
	return b.section(name, 1, vals)
}

func flatten(rows [][]int64) []int64 {
	var out []int64
	for _, r := range rows {
		out = append(out, r...)
	}
	return out
}

func genCollate() (*blob, error) {
	s, err := loadSrc("golang.org/x/text/collate", "tables.go")
	if err != nil {
		return nil, err
	}
	b := newBlob()
	avail, err := s.constStr("availableLocales")
	if err != nil {
		return nil, err
	}
	if err := b.bytesSection("availableLocales", []byte(avail)); err != nil {
		return nil, err
	}
	for _, c := range []string{"UnicodeVersion", "CLDRVersion"} {
		v, err := s.constStr(c)
		if err != nil {
			return nil, err
		}
		if err := b.bytesSection(c, []byte(v)); err != nil {
			return nil, err
		}
	}
	vt, err := s.constInt("varTop")
	if err != nil {
		return nil, err
	}
	if err := b.section("varTop", 4, []int64{vt}); err != nil {
		return nil, err
	}
	locs, err := s.structs("locales", []string{"lookupOffset", "valuesOffset"})
	if err != nil {
		return nil, err
	}
	if err := b.section("locales", 4, flatten(locs)); err != nil {
		return nil, err
	}
	for _, t := range []struct {
		name  string
		width int
	}{
		{"mainExpandElem", 4},
		{"mainContractElem", 4},
		{"mainValues", 4},
		{"mainLookup", 2},
	} {
		v, err := s.ints(t.name)
		if err != nil {
			return nil, err
		}
		if err := b.section(t.name, t.width, v); err != nil {
			return nil, err
		}
	}
	ct, err := s.structs("mainCTEntries", []string{"L", "H", "N", "I"})
	if err != nil {
		return nil, err
	}
	if err := b.section("mainCTEntries", 1, flatten(ct)); err != nil {
		return nil, err
	}
	return b, nil
}

func genNorm() (*blob, error) {
	s, err := loadSrc("golang.org/x/text/unicode/norm", "tables15.0.0.go")
	if err != nil {
		return nil, err
	}
	b := newBlob()
	ver, err := s.constStr("Version")
	if err != nil {
		return nil, err
	}
	if err := b.bytesSection("Version", []byte(ver)); err != nil {
		return nil, err
	}
	var consts []int64
	for _, c := range []string{"firstMulti", "firstCCC", "endMulti", "firstLeadingCCC", "firstCCCZeroExcept", "firstStarterWithNLead", "lastDecomp", "maxDecomp"} {
		v, err := s.constInt(c)
		if err != nil {
			return nil, err
		}
		consts = append(consts, v)
	}
	if err := b.section("consts", 4, consts); err != nil {
		return nil, err
	}
	for _, t := range []struct {
		name  string
		width int
	}{
		{"ccc", 1},
		{"decomps", 1},
		{"nfcValues", 2},
		{"nfcIndex", 1},
		{"nfcSparseOffset", 2},
		{"nfkcValues", 2},
		{"nfkcIndex", 2},
		{"nfkcSparseOffset", 2},
	} {
		v, err := s.ints(t.name)
		if err != nil {
			return nil, err
		}
		if err := b.section(t.name, t.width, v); err != nil {
			return nil, err
		}
	}
	for _, n := range []string{"nfcSparseValues", "nfkcSparseValues"} {
		rows, err := s.structs(n, []string{"value", "lo", "hi"})
		if err != nil {
			return nil, err
		}
		// Pack as u32: value | lo<<16 | hi<<24.
		var packed []int64
		for _, r := range rows {
			packed = append(packed, r[0]|r[1]<<16|r[2]<<24)
		}
		if err := b.section(n, 4, packed); err != nil {
			return nil, err
		}
	}
	nfcCut, err := s.caseLessThan("nfcTrie.lookupValue")
	if err != nil {
		return nil, err
	}
	nfkcCut, err := s.caseLessThan("nfkcTrie.lookupValue")
	if err != nil {
		return nil, err
	}
	if err := b.section("cutoffs", 4, []int64{nfcCut, nfkcCut}); err != nil {
		return nil, err
	}
	return b, nil
}

func rangeTable(t *unicode.RangeTable) []int64 {
	var out []int64
	for _, r := range t.R16 {
		out = append(out, int64(r.Lo), int64(r.Hi), int64(r.Stride))
	}
	for _, r := range t.R32 {
		out = append(out, int64(r.Lo), int64(r.Hi), int64(r.Stride))
	}
	return out
}

func genUnicode() (*blob, error) {
	b := newBlob()
	if err := b.bytesSection("Version", []byte(unicode.Version)); err != nil {
		return nil, err
	}
	if err := b.section("Ideographic", 4, rangeTable(unicode.Ideographic)); err != nil {
		return nil, err
	}
	if err := b.section("Nd", 4, rangeTable(unicode.Nd)); err != nil {
		return nil, err
	}
	return b, nil
}

func cmdGen(args []string) error {
	fs := flag.NewFlagSet("gen", flag.ExitOnError)
	out := fs.String("out", "crates/xtext-collate/data", "output directory")
	_ = fs.Parse(args)
	gens := []struct {
		name string
		f    func() (*blob, error)
	}{
		{"collate.bin", genCollate},
		{"norm.bin", genNorm},
		{"unicode.bin", genUnicode},
		{"language.bin", genLanguage},
	}
	for _, g := range gens {
		b, err := g.f()
		if err != nil {
			return fmt.Errorf("%s: %w", g.name, err)
		}
		p := filepath.Join(*out, g.name)
		if err := os.WriteFile(p, b.buf.Bytes(), 0o644); err != nil {
			return err
		}
		fmt.Fprintf(os.Stderr, "wrote %s (%d bytes)\n", p, b.buf.Len())
	}
	return nil
}
