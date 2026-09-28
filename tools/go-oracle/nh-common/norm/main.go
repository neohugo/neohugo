// Command norm is the Go oracle of crates/nh-common's text::norm, text::xtransform,
// text::runes and text::remove_accents: golang.org/x/text/unicode/norm (NFC, NFD),
// transform.Chain/String/Bytes, runes.Remove(runes.In(unicode.Mn)) and neohugo's
// common/text RemoveAccents/RemoveAccentsString.
//
//	go run ./tools/go-oracle/nh-common/norm [-root .] [-tables crates/nh-common/src/text/norm/tables15.rs] [-out crates/nh-common/tests/fixtures/norm]
//
// It does two things:
//
//  1. It writes the Rust tables of text::norm (tables15.rs) from the x/text
//     module the neohugo module pins (go.mod: golang.org/x/text v0.26.0,
//     Unicode 15.0.0): the ccc table, the decomps bytes and their section
//     constants, the NFC trie (nfcValues, nfcIndex, nfcSparseOffset,
//     nfcSparseValues and the dense block count of nfcTrie.lookupValue) and
//     the recomposition pairs of recompMapPacked. They are read from
//     unicode/norm/tables15.0.0.go in the module cache with go/parser, so the
//     Rust trie is x/text's own data.
//  2. It writes the fixtures: runes.json.gz (every code point alone, the
//     non-starters and decomposables between letters, invalid UTF-8) and
//     strings.json.gz (the former nh-hugofs nfc inputs, seeded random strings
//     of starters, combining marks, Hangul, CGJ, long mark runs over 30,
//     invalid UTF-8 and long strings that cross transform.String's 128-byte
//     chunks and transform.Chain's 4,096-byte buffers, and the docs/ site's
//     content strings).
//
// Each case holds "in" and the results that differ from their fallback:
// "nfc"/"nfd" (norm.NFC/NFD.String; fallback: in), "nfcB"/"nfdB" (Form.Bytes;
// fallback: nfc/nfd), "tnfc"/"tnfd" (transform.String(norm.NFC/NFD, in);
// fallback: nfc/nfd), "ra" (text.RemoveAccents; fallback: in) and "ras"
// (text.RemoveAccentsString; fallback: ra). In runes.json.gz a single code
// point is listed only when one of its results differs from it.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"io/fs"
	"log"
	"math/rand"
	"os"
	"os/exec"
	"path/filepath"
	"runtime/debug"
	"sort"
	"strconv"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/neohugo/neohugo/common/text"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"golang.org/x/text/transform"
	"golang.org/x/text/unicode/norm"
)

func main() {
	root := flag.String("root", ".", "repository root")
	tables := flag.String("tables", "crates/nh-common/src/text/norm/tables15.rs", "Rust tables file")
	out := flag.String("out", "crates/nh-common/tests/fixtures/norm", "fixture directory")
	flag.Parse()

	if norm.Version != "15.0.0" {
		log.Fatalf("x/text unicode/norm is %s, want 15.0.0", norm.Version)
	}
	if err := writeTables(*tables); err != nil {
		log.Fatal(err)
	}
	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	n1 := writeRunes(filepath.Join(*out, "runes.json.gz"))
	n2 := writeStrings(*root, filepath.Join(*out, "strings.json.gz"))
	fmt.Printf("norm: runes.json.gz %d cases, strings.json.gz %d cases\n", n1, n2)
}

// ---------------------------------------------------------------------------
// Tables

func xtextDir() (dir, version string, err error) {
	bi, ok := debug.ReadBuildInfo()
	if !ok {
		return "", "", fmt.Errorf("no build info")
	}
	for _, d := range bi.Deps {
		if d.Path != "golang.org/x/text" {
			continue
		}
		if d.Replace != nil {
			return "", "", fmt.Errorf("golang.org/x/text is replaced")
		}
		cache, err := exec.Command("go", "env", "GOMODCACHE").Output()
		if err != nil {
			return "", "", err
		}
		return filepath.Join(strings.TrimSpace(string(cache)), "golang.org/x/text@"+d.Version), d.Version, nil
	}
	return "", "", fmt.Errorf("golang.org/x/text is not a dependency")
}

func intLit(e ast.Expr) uint64 {
	lit, ok := e.(*ast.BasicLit)
	if !ok || lit.Kind != token.INT {
		log.Fatalf("not an integer literal: %T", e)
	}
	v, err := strconv.ParseUint(lit.Value, 0, 64)
	if err != nil {
		log.Fatal(err)
	}
	return v
}

// elems evaluates the elements of an array or slice composite literal,
// honouring keyed elements.
func elems(cl *ast.CompositeLit, size int, f func(ast.Expr) uint64) []uint64 {
	var vals []uint64
	if size >= 0 {
		vals = make([]uint64, size)
	}
	i := 0
	for _, e := range cl.Elts {
		if kv, ok := e.(*ast.KeyValueExpr); ok {
			i = int(intLit(kv.Key))
			e = kv.Value
		}
		for size < 0 && i >= len(vals) {
			vals = append(vals, 0)
		}
		vals[i] = f(e)
		i++
	}
	return vals
}

func arrayLen(cl *ast.CompositeLit) int {
	at, ok := cl.Type.(*ast.ArrayType)
	if !ok {
		log.Fatalf("not an array type")
	}
	if at.Len == nil {
		return -1
	}
	if _, ok := at.Len.(*ast.Ellipsis); ok {
		return -1
	}
	return int(intLit(at.Len))
}

func stringConcat(e ast.Expr) string {
	switch x := e.(type) {
	case *ast.BasicLit:
		s, err := strconv.Unquote(x.Value)
		if err != nil {
			log.Fatal(err)
		}
		return s
	case *ast.BinaryExpr:
		if x.Op != token.ADD {
			log.Fatalf("unexpected operator %s", x.Op)
		}
		return stringConcat(x.X) + stringConcat(x.Y)
	}
	log.Fatalf("not a string expression: %T", e)
	return ""
}

type valueRange struct {
	value  uint64
	lo, hi uint64
}

func writeTables(path string) error {
	dir, version, err := xtextDir()
	if err != nil {
		return err
	}
	src := filepath.Join(dir, "unicode", "norm", "tables15.0.0.go")
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, src, nil, 0)
	if err != nil {
		return err
	}
	consts := map[string]uint64{}
	lits := map[string]*ast.CompositeLit{}
	var recompPacked string
	denseBlocks := uint64(0)
	for _, d := range f.Decls {
		switch d := d.(type) {
		case *ast.GenDecl:
			for _, s := range d.Specs {
				vs, ok := s.(*ast.ValueSpec)
				if !ok {
					continue
				}
				for i, name := range vs.Names {
					if i >= len(vs.Values) {
						continue
					}
					switch v := vs.Values[i].(type) {
					case *ast.BasicLit:
						if v.Kind == token.INT {
							consts[name.Name] = intLit(v)
						}
					case *ast.CompositeLit:
						lits[name.Name] = v
					case *ast.BinaryExpr:
						if name.Name == "recompMapPacked" {
							recompPacked = stringConcat(v)
						}
					}
				}
			}
		case *ast.FuncDecl:
			if d.Name.Name != "lookupValue" || d.Recv == nil {
				continue
			}
			star, ok := d.Recv.List[0].Type.(*ast.StarExpr)
			if !ok || star.X.(*ast.Ident).Name != "nfcTrie" {
				continue
			}
			ast.Inspect(d.Body, func(n ast.Node) bool {
				if be, ok := n.(*ast.BinaryExpr); ok && be.Op == token.LSS {
					if id, ok := be.X.(*ast.Ident); ok && id.Name == "n" {
						denseBlocks = intLit(be.Y)
					}
				}
				return true
			})
		}
	}
	if denseBlocks == 0 || recompPacked == "" {
		return fmt.Errorf("%s: nfcTrie.lookupValue or recompMapPacked not found", src)
	}
	u := func(e ast.Expr) uint64 { return intLit(e) }
	get := func(name string) []uint64 {
		cl, ok := lits[name]
		if !ok {
			log.Fatalf("%s: %s not found", src, name)
		}
		return elems(cl, arrayLen(cl), u)
	}
	ccc := get("ccc")
	decomps := get("decomps")
	nfcValues := get("nfcValues")
	nfcIndex := get("nfcIndex")
	nfcSparseOffset := get("nfcSparseOffset")
	svCl := lits["nfcSparseValues"]
	var sparse []valueRange
	for _, e := range svCl.Elts {
		if kv, ok := e.(*ast.KeyValueExpr); ok {
			if int(intLit(kv.Key)) != len(sparse) {
				log.Fatalf("nfcSparseValues: unexpected key")
			}
			e = kv.Value
		}
		cl := e.(*ast.CompositeLit)
		var r valueRange
		for j, fe := range cl.Elts {
			if kv, ok := fe.(*ast.KeyValueExpr); ok {
				switch kv.Key.(*ast.Ident).Name {
				case "value":
					r.value = intLit(kv.Value)
				case "lo":
					r.lo = intLit(kv.Value)
				case "hi":
					r.hi = intLit(kv.Value)
				}
				continue
			}
			switch j {
			case 0:
				r.value = intLit(fe)
			case 1:
				r.lo = intLit(fe)
			case 2:
				r.hi = intLit(fe)
			}
		}
		sparse = append(sparse, r)
	}
	if n := arrayLen(svCl); n != len(sparse) {
		log.Fatalf("nfcSparseValues: %d entries, want %d", len(sparse), n)
	}
	if len(recompPacked)%8 != 0 {
		return fmt.Errorf("recompMapPacked: length %d", len(recompPacked))
	}
	type pair struct{ key, val uint32 }
	var pairs []pair
	seen := map[uint32]bool{}
	for i := 0; i < len(recompPacked); i += 8 {
		b := []byte(recompPacked[i : i+8])
		p := pair{
			key: uint32(b[0])<<24 | uint32(b[1])<<16 | uint32(b[2])<<8 | uint32(b[3]),
			val: uint32(b[4])<<24 | uint32(b[5])<<16 | uint32(b[6])<<8 | uint32(b[7]),
		}
		if seen[p.key] {
			return fmt.Errorf("recompMapPacked: duplicate key %08X", p.key)
		}
		seen[p.key] = true
		pairs = append(pairs, p)
	}
	sort.Slice(pairs, func(i, j int) bool { return pairs[i].key < pairs[j].key })
	// Consistency checks of the parse.
	if consts["lastDecomp"] != uint64(len(decomps)) || consts["maxDecomp"] != 0x8000 {
		return fmt.Errorf("%s: decomps has %d bytes, lastDecomp is %d", src, len(decomps), consts["lastDecomp"])
	}
	if len(ccc) != 56 || len(nfcValues) != int(denseBlocks+2)<<6 {
		return fmt.Errorf("%s: unexpected table sizes (ccc %d, nfcValues %d)", src, len(ccc), len(nfcValues))
	}

	var b bytes.Buffer
	fmt.Fprintf(&b, "//! x/text unicode/norm %s tables (NFC/NFD only), from\n", norm.Version)
	fmt.Fprintf(&b, "//! golang.org/x/text@%s/unicode/norm/tables15.0.0.go.\n", version)
	b.WriteString("//!\n//! Generated by tools/go-oracle/nh-common/norm (do not edit).\n\n")
	fmt.Fprintf(&b, "pub(super) const VERSION: &str = %q;\n\n", norm.Version)
	for _, name := range []string{"firstCCC", "firstLeadingCCC", "firstCCCZeroExcept", "firstStarterWithNLead"} {
		v, ok := consts[name]
		if !ok {
			return fmt.Errorf("%s: const %s not found", src, name)
		}
		fmt.Fprintf(&b, "pub(super) const %s: u16 = 0x%X;\n", constNames[name], v)
	}
	fmt.Fprintf(&b, "\n/// `nfcTrie.lookupValue`: blocks below this index are dense (`nfcValues`).\n")
	fmt.Fprintf(&b, "pub(super) const NFC_DENSE_BLOCKS: u32 = %d;\n", denseBlocks)
	writeArray(&b, "CCC", "u8", ccc, "%d")
	writeArray(&b, "DECOMPS", "u8", decomps, "0x%02X")
	writeArray(&b, "NFC_VALUES", "u16", nfcValues, "0x%04X")
	writeArray(&b, "NFC_INDEX", "u8", nfcIndex, "0x%02X")
	writeArray(&b, "NFC_SPARSE_OFFSET", "u16", nfcSparseOffset, "0x%X")
	fmt.Fprintf(&b, "\n/// `nfcSparseValues`: (value, lo, hi).\npub(super) static NFC_SPARSE_VALUES: [(u16, u8, u8); %d] = [\n", len(sparse))
	for _, r := range sparse {
		fmt.Fprintf(&b, "    (0x%04X, 0x%02X, 0x%02X),\n", r.value, r.lo, r.hi)
	}
	b.WriteString("];\n")
	fmt.Fprintf(&b, "\n/// `recompMapPacked` as (key, composite) pairs sorted by key; the key is\n/// `uint32(uint16(a))<<16 + uint32(uint16(b))`.\npub(super) static RECOMP_MAP: [(u32, u32); %d] = [\n", len(pairs))
	for _, p := range pairs {
		fmt.Fprintf(&b, "    (0x%08X, 0x%X),\n", p.key, p.val)
	}
	b.WriteString("];\n")
	return os.WriteFile(path, b.Bytes(), 0o644)
}

// constNames are the decomps section constants compInfo and CCC use
// (firstMulti/endMulti only serve the unported multiSegment).
var constNames = map[string]string{
	"firstCCC":              "FIRST_CCC",
	"firstLeadingCCC":       "FIRST_LEADING_CCC",
	"firstCCCZeroExcept":    "FIRST_CCC_ZERO_EXCEPT",
	"firstStarterWithNLead": "FIRST_STARTER_WITH_N_LEAD",
}

func writeArray(b *bytes.Buffer, name, typ string, vals []uint64, format string) {
	fmt.Fprintf(b, "\npub(super) static %s: [%s; %d] = [\n", name, typ, len(vals))
	for i := 0; i < len(vals); i += 16 {
		b.WriteString("   ")
		for j := i; j < i+16 && j < len(vals); j++ {
			b.WriteString(" ")
			fmt.Fprintf(b, format, vals[j])
			b.WriteString(",")
		}
		b.WriteString("\n")
	}
	b.WriteString("];\n")
}

// ---------------------------------------------------------------------------
// Fixtures

func str(s string) any { return goval.Str(s) }

// results runs every function on s and returns the case.
func results(s string) map[string]any {
	c := map[string]any{"in": str(s)}
	nfc := norm.NFC.String(s)
	nfd := norm.NFD.String(s)
	if nfc != s {
		c["nfc"] = str(nfc)
	}
	if nfd != s {
		c["nfd"] = str(nfd)
	}
	if b := string(norm.NFC.Bytes([]byte(s))); b != nfc {
		c["nfcB"] = str(b)
	}
	if b := string(norm.NFD.Bytes([]byte(s))); b != nfd {
		c["nfdB"] = str(b)
	}
	if t, _, _ := transform.String(norm.NFC, s); t != nfc {
		c["tnfc"] = str(t)
	}
	if t, _, _ := transform.String(norm.NFD, s); t != nfd {
		c["tnfd"] = str(t)
	}
	ra := string(text.RemoveAccents([]byte(s)))
	if ra != s {
		c["ra"] = str(ra)
	}
	if ras := text.RemoveAccentsString(s); ras != ra {
		c["ras"] = str(ras)
	}
	return c
}

// sweepInfo lists the code points whose NFC properties are not trivial.
type sweepInfo struct {
	marks        []rune // ccc != 0
	decomposable []rune // NFD differs (Hangul syllables excluded)
	backward     []rune // BoundaryBefore false
	mn           []rune // unicode.Mn
}

func sweep() sweepInfo {
	var si sweepInfo
	for r := rune(0); r <= utf8.MaxRune; r++ {
		if r >= 0xD800 && r <= 0xDFFF {
			continue
		}
		s := string(r)
		p := norm.NFD.PropertiesString(s)
		if p.CCC() != 0 {
			si.marks = append(si.marks, r)
		}
		if !p.BoundaryBefore() {
			si.backward = append(si.backward, r)
		}
		if unicode.Is(unicode.Mn, r) {
			si.mn = append(si.mn, r)
		}
		if norm.NFD.String(s) != s && (r < 0xAC00 || r > 0xD7A3) {
			si.decomposable = append(si.decomposable, r)
		}
	}
	return si
}

var invalid = []string{
	"\x80", "\xbf", "\xc0", "\xc1\xbf", "\xc2", "\xc3", "\xc3\x28", "\xdf", "\xe0", "\xe0\x80\xaf",
	"\xe0\xa0", "\xe1\x80", "\xe2\x82", "\xed\xa0\x80", "\xed\xbf\xbf", "\xef\xbf", "\xf0", "\xf0\x90",
	"\xf0\x90\x80", "\xf4\x90\x80\x80", "\xf5", "\xf8\x88\x80\x80\x80", "\xfe", "\xff", "\xcc", "\xcd",
	"\xea\xb0", "\xe1\x84", "\xe1\x85", "\xe1\x86", "\x80\x81\x82", "\xcc\xcc\x81",
}

func writeRunes(path string) int {
	si := sweep()
	var cases []map[string]any
	for r := rune(0); r <= utf8.MaxRune; r++ {
		if r >= 0xD800 && r <= 0xDFFF {
			continue
		}
		c := results(string(r))
		if len(c) > 1 {
			cases = append(cases, c)
		}
	}
	nSingle := len(cases)
	seen := map[string]bool{}
	add := func(s string) {
		if !seen[s] {
			seen[s] = true
			cases = append(cases, results(s))
		}
	}
	// Non-starters, backward-combining runes and decomposables between letters
	// and marks (these contexts decide composition and reordering).
	ctx := func(r rune) {
		s := string(r)
		add("a" + s + "b")
		add("e" + s + "́")
		add("ọ" + s)
		add("ᄀ" + s + "ᆨ")
	}
	for _, rs := range [][]rune{si.marks, si.backward, si.decomposable} {
		for _, r := range rs {
			ctx(r)
		}
	}
	for r := rune(0xAC00); r <= 0xD7A3; r += 5 {
		ctx(r)
	}
	// Every byte, alone and between a letter and a mark; the invalid sequences.
	for b := 0x80; b <= 0xFF; b++ {
		add(string([]byte{byte(b)}))
		add("a" + string([]byte{byte(b)}) + "́")
		add("́" + string([]byte{byte(b)}))
	}
	for _, s := range invalid {
		add(s)
		add("e" + s + "́")
		add("é" + s + "̧")
		add(s + "̀")
		add("ᄀ" + s + "ᅡ")
	}
	header := map[string]any{
		"version":       norm.Version,
		"singleRunes":   nSingle,
		"singleListing": "a single code point is listed only when one of its results differs",
	}
	if err := goval.WriteCasesGz(path, header, cases); err != nil {
		log.Fatal(err)
	}
	return len(cases)
}

// nfcInputs are the inputs of the former nh-hugofs nfc oracle.
func nfcInputs(si sweepInfo, add func(string)) {
	for r := rune(0); r <= utf8.MaxRune; r++ {
		if r >= 0xD800 && r <= 0xDFFF {
			continue
		}
		if norm.NFC.String(string(r)) != string(r) {
			add(string(r))
		}
	}
	for _, r := range si.decomposable {
		add(norm.NFD.String(string(r)))
		add("a" + norm.NFD.String(string(r)) + "b")
	}
	for r := rune(0xAC00); r <= 0xD7A3; r += 7 {
		add(norm.NFD.String(string(r)))
	}
	for _, s := range []string{
		"café.md", "café.md", "Café́.md", "potato-chips/wise-chili-olé", "บทความ/ไทย.md",
		"เกม", "ก่ำ", "กํา", "ȩ́", "ȩ́", "각",
		"가", "가ᆧ", "각", "á́́", "̈́", "ཱི", "ཱྀ",
		"ÅÅÅ", "ẛ̣", "ﬁ", "क़", "क़", "אַ", "",
		strings.Repeat("́", 40), "a" + strings.Repeat("́", 29), "a" + strings.Repeat("́", 30),
		"a" + strings.Repeat("́", 31), "a" + strings.Repeat("́", 61) + "b",
		"e" + strings.Repeat("̧́", 20), strings.Repeat("ᅡ", 35), "ᄀ" + strings.Repeat("ᅡ", 3),
	} {
		add(s)
	}
	r := rand.New(rand.NewSource(15))
	starters := []rune{'a', 'e', 'o', 'u', 'A', 'E', 'O', 'U', 'n', 'c', 'y', 0x0391, 0x03b1, 0x0415, 0x0435, 0x05d0, 0x0627, 0x0915, 0x0e01, 0x1100, 0x1161, 0x11a8, 0xac00, 0x3042, 0x304b, '/', '.', '-', 0x0e40}
	marks := si.marks
	for range 20000 {
		var b strings.Builder
		n := 1 + r.Intn(8)
		for range n {
			switch r.Intn(4) {
			case 0, 1:
				b.WriteRune(starters[r.Intn(len(starters))])
			case 2:
				b.WriteRune(marks[r.Intn(len(marks))])
			default:
				b.WriteString(norm.NFD.String(string(si.decomposable[r.Intn(len(si.decomposable))])))
			}
		}
		add(b.String())
	}
}

// piece returns one random fragment.
func piece(r *rand.Rand, si sweepInfo) string {
	starters := []rune{'a', 'e', 'o', 'u', 'A', 'E', 'n', 'c', 'y', 's', ' ', '-', '/', '.', '0', 0x0391, 0x03b1, 0x0415, 0x0435, 0x05d0, 0x0627, 0x0915, 0x0e01, 0x0e40, 0x1100, 0x1112, 0xac00, 0xd7a3, 0x3042, 0x304b, 0x4e2d, 0x1f600, 0x11099, 0x110a5, 0x1d15e}
	switch k := r.Intn(40); k / 2 {
	case 0, 1, 2, 3, 4:
		return string(starters[r.Intn(len(starters))])
	case 5, 6:
		return string(si.marks[r.Intn(len(si.marks))])
	case 7:
		return string(si.backward[r.Intn(len(si.backward))])
	case 8:
		return string(si.mn[r.Intn(len(si.mn))])
	case 9, 10:
		d := si.decomposable[r.Intn(len(si.decomposable))]
		if r.Intn(2) == 0 {
			return string(d)
		}
		return norm.NFD.String(string(d))
	case 11:
		return string(rune(0xAC00 + r.Intn(11172)))
	case 12:
		return string(rune(0x1161+r.Intn(21))) + string(rune(0x11A8+r.Intn(27)))
	case 13:
		return invalid[r.Intn(len(invalid))]
	case 14:
		return "͏"
	case 15:
		if k%2 == 1 {
			return string(si.marks[r.Intn(len(si.marks))])
		}
		// A long run of marks (stream-safe: a CGJ after 30 non-starters).
		n := 25 + r.Intn(50)
		var b strings.Builder
		one := r.Intn(2) == 0
		m := si.marks[r.Intn(len(si.marks))]
		for range n {
			if !one {
				m = si.marks[r.Intn(len(si.marks))]
			}
			b.WriteRune(m)
		}
		return b.String()
	case 16:
		if k%2 == 1 {
			return string(rune(0x1161 + r.Intn(21)))
		}
		// A long run of non-starters that are not Mn (they survive runes.Remove):
		// Hangul V and T jamo and Mc marks with a combining class.
		n := 25 + r.Intn(50)
		nonMn := []rune{0x1161, 0x1175, 0x11A8, 0x11C2, 0x1D165, 0x1D166, 0x1D16D, 0x302E, 0x302F, 0x16FF0, 0x16FF1}
		var b strings.Builder
		for range n {
			b.WriteRune(nonMn[r.Intn(len(nonMn))])
		}
		return b.String()
	case 17:
		return string(rune(r.Intn(0x80)))
	case 18:
		return string(rune(0x300 + r.Intn(0x70)))
	default:
		return string(rune(0xC0 + r.Intn(0x250)))
	}
}

func docsStrings(root string, add func(string)) {
	strs, err := corpus.Strings(root)
	if err != nil {
		log.Fatal(err)
	}
	for _, s := range strs {
		add(s)
	}
	// Every line of docs/content that is not ASCII.
	var lines []string
	err = filepath.WalkDir(filepath.Join(root, "docs", "content"), func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || !strings.HasSuffix(p, ".md") {
			return nil
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		for _, l := range strings.Split(string(b), "\n") {
			for i := 0; i < len(l); i++ {
				if l[i] >= utf8.RuneSelf {
					lines = append(lines, l)
					break
				}
			}
		}
		return nil
	})
	if err != nil {
		log.Fatal(err)
	}
	sort.Strings(lines)
	for _, l := range lines {
		add(l)
	}
}

func writeStrings(root, path string) int {
	si := sweep()
	var cases []map[string]any
	seen := map[string]bool{}
	add := func(s string) {
		if !seen[s] {
			seen[s] = true
			cases = append(cases, results(s))
		}
	}
	nfcInputs(si, add)
	nNFC := len(cases)

	// Hugo's TestRemoveAccents and adversarial strings.
	for _, s := range []string{
		"Resumé", "Hugö", "Àccénts", "Ðó ÿöü ßéé þïß?", "crème brûlée", "naïve café", "Ελληνικά", "Ǆǅǆ",
		"ﬁ ﬂ", "Å Å Å", "각 각", "ベ ベ", "é́́", "͏", "͏́", "a͏́",
		strings.Repeat("́", 31) + "a", "a" + strings.Repeat("̧́", 16), strings.Repeat("ᴖ5", 40),
		"a" + strings.Repeat("\U0001d165", 40) + "b", "ᄀ" + strings.Repeat("ᅡ", 40),
		strings.Repeat("é", 2100), strings.Repeat("é", 2100), strings.Repeat("́", 5000),
		strings.Repeat("a\xff", 3000), strings.Repeat("각", 1500),
	} {
		add(s)
	}

	r := rand.New(rand.NewSource(271))
	for range 12000 {
		var b strings.Builder
		n := 1 + r.Intn(10)
		for range n {
			b.WriteString(piece(r, si))
		}
		add(b.String())
	}
	// Long strings across transform.String's 128-byte chunks and the chain's
	// 4,096-byte buffers.
	for range 40 {
		var b strings.Builder
		n := 100 + r.Intn(6000)
		for b.Len() < n {
			b.WriteString(piece(r, si))
		}
		add(b.String())
	}
	// Mark runs at every offset around the 128 and 4,096 boundaries.
	for _, base := range []int{120, 4080} {
		for off := 0; off < 24; off++ {
			for _, run := range []string{strings.Repeat("́", 35), strings.Repeat("\U0001d165", 35), "ȩ́", "각", "\xe2\x82"} {
				add(strings.Repeat("x", base+off) + run + "y")
			}
		}
	}
	nRandom := len(cases) - nNFC
	docsStrings(root, add)

	header := map[string]any{
		"version": norm.Version,
		"nfc":     nNFC,
		"random":  nRandom,
		"docs":    len(cases) - nNFC - nRandom,
	}
	if err := goval.WriteCasesGz(path, header, cases); err != nil {
		log.Fatal(err)
	}
	return len(cases)
}
