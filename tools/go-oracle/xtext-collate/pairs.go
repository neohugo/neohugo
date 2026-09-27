package main

// Adversarial comparison pairs for crates/xtext-collate.
//
// The random-corpus digests (fixtures.go) mostly compare unrelated strings,
// which are decided at the first primary weight. Hugo only ever calls
// Collator.CompareString, so this file generates pairs that are *almost*
// equal (case, diacritics, Thai tone marks and vowel order, ignorables,
// digit scripts, width, compatibility forms, contraction boundaries,
// NFC/NFD, invalid UTF-8 cuts, prefixes), which drives the incremental
// compareLevel/nextPrimary paths down to the secondary, tertiary,
// quaternary and identity levels.
//
// The pairs are written out verbatim (hex), so the Rust side does not need
// to mirror this generator.

import (
	"bufio"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"flag"
	"fmt"
	"os"
	"strings"
	"unicode"
	"unicode/utf8"

	"golang.org/x/text/collate"
	"golang.org/x/text/unicode/norm"
)

// Thai building blocks.
var (
	thaiConsonants = func() []rune {
		var rs []rune
		for r := rune(0x0E01); r <= 0x0E2E; r++ {
			rs = append(rs, r)
		}
		return rs
	}()
	thaiPrevowels   = []rune{0x0E40, 0x0E41, 0x0E42, 0x0E43, 0x0E44}
	thaiClusters    = []rune{0x0E23, 0x0E25, 0x0E27}
	thaiAboveBelow  = []rune{0x0E31, 0x0E34, 0x0E35, 0x0E36, 0x0E37, 0x0E47, 0x0E38, 0x0E39, 0x0E3A, 0x0E4D}
	thaiTones       = []rune{0x0E48, 0x0E49, 0x0E4A, 0x0E4B}
	thaiFollowing   = []rune{0x0E30, 0x0E32, 0x0E33, 0x0E45, 0x0E24, 0x0E26}
	thaiFinals      = []rune{0x0E01, 0x0E07, 0x0E14, 0x0E19, 0x0E1A, 0x0E21, 0x0E22, 0x0E27, 0x0E15, 0x0E2A, 0x0E23, 0x0E25}
	thaiMisc        = []rune{0x0E46, 0x0E2F, 0x0E3F, 0x0E4C, 0x0E4E, 0x0E4F, 0x0E5A, 0x0E5B}
	thaiDigitsFirst = rune(0x0E50)
)

var thaiWords = []string{
	"สวัสดี", "ภาษาไทย", "กรุงเทพ", "เชียงใหม่", "ไก่", "ไข่", "ขนม", "ข้าว", "ข่าว", "เขา", "เข้า", "เข่า",
	"ใหม่", "ไหม", "ไม้", "ไหม้", "หมา", "ม้า", "มา", "ม้าๆ", "น้ำ", "นํ้า", "นำ้", "กา", "ก่า", "ก้า", "ก๊า", "ก๋า",
	"เกาะ", "แก้ว", "โต๊ะ", "ใจ", "ไป", "เป็น", "เปน", "กรรม", "ศักดิ์", "ศักดิ", "พระ", "ฤดู", "ฤๅษี", "ฦๅ",
	"ฯลฯ", "ฯ", "๑๒๓", "123", "๑", "๐", "เก", "แก", "โก", "ใก", "ไก", "กเ", "เกา", "เกิ", "เกี", "เกือ",
	"ร้าน", "ขนมจีน", "ข้าวผัด", "ต้มยำ", "ส้มตำ", "ผัดไทย", "ชา", "ช้า", "ช่า", "ค่ะ", "คะ", "ครับ",
	"สินค้า", "ราคา", "บาท", "฿", "สั่งซื้อ", "หมวดหมู่", "ทั้งหมด", "ใหม่ล่าสุด", "ขายดี",
}

var latinWords = []string{
	"The", "the", "THE", "a", "A", "an", "co-op", "coop", "Co-op", "résumé", "resume", "Resume", "résume",
	"naïve", "naive", "e-mail", "email", "E-Mail", "O'Brien", "OBrien", "O’Brien", "hello", "Hello", "HELLO",
	"world", "café", "cafe", "Café", "CAFÉ", "cote", "coté", "côte", "côté", "Straße", "Strasse", "STRASSE",
	"Ærø", "Aero", "œuvre", "oeuvre", "ﬁle", "file", "ﬂow", "flow", "ch", "Ch", "c", "h", "ll", "l", "Ll",
	"dz", "dž", "ǆ", "lj", "ǉ", "nj", "ǌ", "ij", "ĳ", "IJ", "Ĳ", "æ", "ae", "Æ", "AE", "ss", "ß", "ẞ",
	"1", "2", "10", "100", "1.5", "1,000", "1000", "v1.2.3", "v1.10", "2026-09-27", "09", "9", "007",
	"#1", "No.1", "No. 1", "½", "1/2", "①", "Ⅳ", "IV", "iv", "x²", "x2", "km²", "㎢", "™", "TM",
	"Ａ", "Ｂ", "ａ", "ｶ", "カ", "か", "ガ", "中文", "日本", "한국", "한", "하", "ㅎ", "Ω", "ω", "Ω",
	"İstanbul", "istanbul", "ISTANBUL", "ı", "I", "i", "Ångström", "Angstrom", "Å", "Å", "Å",
	"🙂", "🙂🙂", "❤️", "❤", "—", "–", "-", "‐", "_", "·", "•", "…", "...", " ", "\u00a0", "\u2009", "\u3000",
}

// equivalents: units that are equal at some level (or near) to each other.
var variantGroups = [][]string{
	{" ", "\u00a0", "\u2002", "\u2009", "\u3000", "\u200b", "", "  ", "\t", "\n"},
	{"-", "‐", "‑", "–", "—", "−", "_", "", "\u00ad"},
	{"'", "’", "‘", "`", "´", "ʼ", "\"", "“", "”", ""},
	{".", "．", "。", "…", "..", ""},
	{",", "，", "、", ""},
	{"a", "A", "á", "à", "â", "ä", "å", "ā", "ａ", "Ａ", "ª", "ᵃ", "á", "ä́", "á̈"},
	{"e", "E", "é", "è", "ê", "ë", "ė", "ｅ", "é", "É", "ệ", "ệ", "ệ"},
	{"o", "O", "ö", "ø", "ó", "ô", "œ", "oe", "ö", "ｏ"},
	{"c", "C", "ç", "č", "ch", "Ch", "CH", "cH", "ç"},
	{"l", "L", "ł", "ll", "Ll", "LL", "l·l", "ŀl", "l·l"},
	{"s", "S", "ss", "ß", "ẞ", "š", "ſ", "ş"},
	{"i", "I", "ı", "İ", "í", "ï", "ｉ", "ⅰ"},
	{"n", "N", "ñ", "ny", "ng", "nj", "ǌ", "Ǌ", "ǋ"},
	{"d", "D", "dz", "dž", "ǆ", "Ǆ", "ǅ", "đ", "ð"},
	{"0", "٠", "۰", "०", "০", "๐", "໐", "０", "₀", "⁰", "𝟎", "〇", "零"},
	{"1", "١", "۱", "१", "১", "๑", "໑", "１", "₁", "¹", "𝟏", "①", "Ⅰ", "፩", "၁"},
	{"2", "٢", "۲", "२", "২", "๒", "໒", "２", "₂", "²", "②", "Ⅱ", "፪"},
	{"9", "٩", "۹", "९", "৯", "๙", "໙", "９", "₉", "⁹", "⑨", "Ⅸ"},
	{"fi", "ﬁ", "f\u200bi", "FI", "Fi"},
	{"ำ", "ํา", "าํ", "า"},
	{"่", "้", "๊", "๋", "", "่่", "็"},
	{"ิ", "ี", "ึ", "ื", "ั", "็", ""},
	{"ุ", "ู", "ฺ", ""},
	{"เ", "แ", "เเ", "โ", "ใ", "ไ", ""},
	{"ๆ", "ฯ", " ๆ", "ฯลฯ", ""},
	{"์", "ํ", "๎", ""},
	{"ฤ", "ฤๅ", "ฦ", "ฦๅ", "รึ", "ลึ"},
	{"ก", "ข", "ฃ", "ค", "ฅ", "ฆ"},
	{"가", "각", "가", "각", "ㄱ", "ㄱㅏ"},
	{"か", "カ", "ｶ", "が", "ガ", "ｶﾞ", "が", "ゕ", "ヵ"},
	{"́", "̀", "̂", "̈", "̣", "̧", "̨", "\u034f", "\u200d", ""},
	{"\x00", "\x01", "\x1f", "\x7f", "\u00ad", "\u200b", "\u200c", "\u200d", "\u200e", "\u200f", "\ufeff", "\u2060", "\u034f", ""},
	{"\x80", "\xc3", "\xe0\xb8", "\xef\xbf\xbd", "\ufffd", "\xff", "\xed\xa0\x80", "\xf4\x90\x80\x80"},
}

var variantIndex = func() map[string][]string {
	m := map[string][]string{}
	for _, g := range variantGroups {
		for _, u := range g {
			if u != "" {
				m[u] = g
			}
		}
	}
	return m
}()

var pairMarks = []rune{0x0300, 0x0301, 0x0302, 0x0303, 0x0308, 0x030A, 0x030C, 0x0323, 0x0327, 0x0328, 0x0331, 0x0345,
	0x0E31, 0x0E34, 0x0E37, 0x0E38, 0x0E39, 0x0E3A, 0x0E47, 0x0E48, 0x0E49, 0x0E4A, 0x0E4B, 0x0E4C, 0x0E4D, 0x0E4E,
	0x093C, 0x094D, 0x05B0, 0x0F71, 0x0F72, 0x0F80, 0x3099, 0x309A, 0x034F, 0x0338, 0x20D7, 0x1DCE}

func pick[T any](r *rng, xs []T) T { return xs[r.intn(len(xs))] }

// genThaiSyllable produces a plausible (or slightly implausible) Thai
// syllable in logical (typed) order.
func genThaiSyllable(r *rng, b []byte) []byte {
	if r.intn(10) < 3 {
		b = utf8.AppendRune(b, pick(r, thaiPrevowels))
	}
	b = utf8.AppendRune(b, pick(r, thaiConsonants))
	if r.intn(10) == 0 {
		b = utf8.AppendRune(b, pick(r, thaiClusters))
	}
	if r.intn(10) < 4 {
		b = utf8.AppendRune(b, pick(r, thaiAboveBelow))
	}
	if r.intn(10) < 4 {
		b = utf8.AppendRune(b, pick(r, thaiTones))
	}
	if r.intn(10) < 3 {
		b = utf8.AppendRune(b, pick(r, thaiFollowing))
	}
	if r.intn(10) < 4 {
		b = utf8.AppendRune(b, pick(r, thaiFinals))
		if r.intn(20) == 0 {
			b = utf8.AppendRune(b, 0x0E4C)
		}
	}
	if r.intn(30) == 0 {
		b = utf8.AppendRune(b, pick(r, thaiMisc))
	}
	return b
}

func genThai(r *rng) []byte {
	var b []byte
	n := 1 + r.intn(6)
	for i := 0; i < n; i++ {
		switch r.intn(12) {
		case 0:
			b = append(b, pick(r, thaiWords)...)
		case 1:
			b = append(b, ' ')
		case 2:
			b = utf8.AppendRune(b, thaiDigitsFirst+rune(r.intn(10)))
		case 3:
			b = append(b, pick(r, latinWords)...)
		default:
			b = genThaiSyllable(r, b)
		}
	}
	return b
}

func genTitle(r *rng) []byte {
	var b []byte
	n := 1 + r.intn(5)
	for i := 0; i < n; i++ {
		if i > 0 {
			b = append(b, pick(r, []string{" ", " ", " ", "-", ", ", ": ", " - ", "/", " & ", "_", ".", "'"})...)
		}
		switch r.intn(6) {
		case 0:
			b = append(b, pick(r, thaiWords)...)
		case 1:
			b = genThaiSyllable(r, b)
		case 2:
			b = genItem(r, b)
		default:
			b = append(b, pick(r, latinWords)...)
		}
	}
	return b
}

// units splits s into runes, keeping each invalid byte as its own unit.
func units(s []byte) []string {
	var us []string
	for len(s) > 0 {
		_, sz := utf8.DecodeRune(s)
		us = append(us, string(s[:sz]))
		s = s[sz:]
	}
	return us
}

func isASCIILetter(u string) bool {
	return len(u) == 1 && (u[0] >= 'a' && u[0] <= 'z' || u[0] >= 'A' && u[0] <= 'Z')
}

func mutate(r *rng, s []byte) []byte {
	us := units(s)
	pos := func(extra int) int { return r.intn(len(us) + extra) }
	insert := func(i int, u string) {
		us = append(us, "")
		copy(us[i+1:], us[i:])
		us[i] = u
	}
	switch r.intn(20) {
	case 0: // toggle case of one ASCII letter
		for try := 0; try < 4 && len(us) > 0; try++ {
			i := pos(0)
			if isASCIILetter(us[i]) {
				us[i] = string(us[i][0] ^ 0x20)
				break
			}
		}
	case 1, 2: // insert a combining mark after a unit
		insert(pos(1), string(pick(r, pairMarks)))
	case 3: // delete a unit
		if len(us) > 0 {
			i := pos(0)
			us = append(us[:i], us[i+1:]...)
		}
	case 4: // swap adjacent units (reorders marks: doNorm, Thai prevowels)
		if len(us) > 1 {
			i := r.intn(len(us) - 1)
			us[i], us[i+1] = us[i+1], us[i]
		}
	case 5, 6, 7: // replace a unit with a variant of its group
		var cand []int
		for i, u := range us {
			if _, ok := variantIndex[u]; ok {
				cand = append(cand, i)
			}
		}
		if len(cand) > 0 {
			i := pick(r, cand)
			us[i] = pick(r, variantIndex[us[i]])
		} else if len(us) > 0 {
			us[pos(0)] = pick(r, pick(r, variantGroups))
		}
	case 8: // insert an ignorable / control / zero-width
		insert(pos(1), pick(r, variantGroups[len(variantGroups)-2]))
	case 9: // normalization forms of the whole string
		switch r.intn(4) {
		case 0:
			return norm.NFD.Bytes(s)
		case 1:
			return norm.NFC.Bytes(s)
		case 2:
			return norm.NFKD.Bytes(s)
		default:
			return norm.NFKC.Bytes(s)
		}
	case 10: // append / prepend something small
		var x []byte
		switch r.intn(4) {
		case 0:
			x = genItem(r, nil)
		case 1:
			x = genThaiSyllable(r, nil)
		case 2:
			x = []byte(pick(r, []string{" ", "s", "1", "0", "ๆ", "่", "́", "a", "A", ".", "-"}))
		default:
			x = []byte(pick(r, latinWords))
		}
		if r.intn(2) == 0 {
			return append(append([]byte{}, s...), x...)
		}
		return append(x, s...)
	case 11: // truncate at a random byte (may cut a UTF-8 sequence)
		if len(s) > 0 {
			return append([]byte{}, s[:r.intn(len(s))]...)
		}
	case 12: // duplicate a unit
		if len(us) > 0 {
			i := pos(0)
			insert(i, us[i])
		}
	case 13: // Thai: change / move / drop a tone mark or vowel
		var cand []int
		for i, u := range us {
			rr, _ := utf8.DecodeRuneInString(u)
			if rr >= 0x0E30 && rr <= 0x0E4E {
				cand = append(cand, i)
			}
		}
		if len(cand) == 0 {
			insert(pos(1), string(pick(r, thaiTones)))
			break
		}
		i := pick(r, cand)
		switch r.intn(4) {
		case 0:
			us[i] = string(pick(r, thaiTones))
		case 1:
			us[i] = string(pick(r, thaiAboveBelow))
		case 2:
			if i > 0 {
				us[i], us[i-1] = us[i-1], us[i]
			}
		default:
			us = append(us[:i], us[i+1:]...)
		}
	case 14: // contraction boundaries
		insert(pos(1), pick(r, []string{"h", "l", "·", "́", "̈", "z", "j", "y", "s", "c", "H", "L"}))
	case 15: // whole-string case mapping
		if r.intn(2) == 0 {
			return []byte(strings.ToUpper(string(s)))
		}
		return []byte(strings.ToLower(string(s)))
	case 16: // insert space / punctuation
		insert(pos(1), pick(r, []string{" ", "-", ".", ",", "'", "_", "/", "(", ")", "!", "?", ":", "&", "#"}))
	case 17: // Thai prevowel moved after its consonant (or before)
		for i, u := range us {
			rr, _ := utf8.DecodeRuneInString(u)
			if rr >= 0x0E40 && rr <= 0x0E44 && i+1 < len(us) {
				us[i], us[i+1] = us[i+1], us[i]
				break
			}
		}
	case 18: // digit run changes (numeric option: leading zeros, lengths)
		insert(pos(1), pick(r, []string{"0", "00", "1", "9", "10", "๐", "๑", "٠", "０", "₀", "0́"}))
	default: // replace a unit with a random item
		if len(us) > 0 {
			us[pos(0)] = string(genItem(r, nil))
		}
	}
	return []byte(strings.Join(us, ""))
}

func genBase(r *rng, site [][]byte) []byte {
	switch k := r.intn(10); {
	case k < 3:
		return genThai(r)
	case k < 5:
		return genTitle(r)
	case k < 7:
		return genString(r)
	case k < 8 && len(site) > 0:
		return append([]byte{}, pick(r, site)...)
	case k < 9:
		return genStress(r)
	default: // long concatenations (many elements, > 512 at times)
		var b []byte
		n := 5 + r.intn(60)
		for i := 0; i < n; i++ {
			if i > 0 {
				b = append(b, ' ')
			}
			if r.intn(2) == 0 {
				b = append(b, genThai(r)...)
			} else {
				b = append(b, genTitle(r)...)
			}
		}
		return b
	}
}

func genPairs(seed uint64, n int, site [][]byte) [][2][]byte {
	r := &rng{seed}
	out := make([][2][]byte, n)
	for i := range out {
		a := genBase(r, site)
		var b []byte
		switch r.intn(12) {
		case 0: // identical
			b = append([]byte{}, a...)
		case 1: // unrelated
			b = genBase(r, site)
		default:
			b = append([]byte{}, a...)
			m := 1 + r.intn(3)
			for j := 0; j < m; j++ {
				b = mutate(r, b)
			}
		}
		if r.intn(2) == 0 {
			a, b = b, a
		}
		out[i] = [2][]byte{a, b}
	}
	return out
}

// pairResults returns, per pair, Compare(a,b)+'1' and Compare(b,a)+'1',
// plus a digest of the keys of all strings.
func pairResults(c *collate.Collator, pairs [][2][]byte) ([]byte, string) {
	res := make([]byte, 0, 2*len(pairs))
	hk := sha256.New()
	var buf collate.Buffer
	var lb [binary.MaxVarintLen64]byte
	for _, p := range pairs {
		res = append(res, byte('1'+c.Compare(p[0], p[1])), byte('1'+c.CompareString(string(p[1]), string(p[0]))))
		for _, s := range p {
			buf.Reset()
			k := c.Key(&buf, s)
			hk.Write(lb[:binary.PutUvarint(lb[:], uint64(len(k)))])
			hk.Write(k)
		}
	}
	return res, hex.EncodeToString(hk.Sum(nil))
}

// cmdPairs writes a pairs file:
//
//	N	<n>	<seed>
//	P	<hex a>	<hex b>            (n lines)
//	C	<name>	<tag>	<opts>	<sha256(results)>	<keys digest>	[<results>]
func cmdPairs(args []string) error {
	fs := flag.NewFlagSet("pairs", flag.ExitOnError)
	out := fs.String("out", "", "output file")
	n := fs.Int("n", 3000, "pairs")
	seed := fs.Uint64("seed", 7, "seed")
	full := fs.String("full", "main", "configs with full per-pair results: main|all|none")
	locales := fs.Bool("locales", true, "also run every supported locale")
	site := fs.String("site", "", "site-strings.hex file used as extra bases")
	kind := fs.String("kind", "random", "random|numeric|nondigits")
	every := fs.Int("every", 1, "keep only every k-th pair (numeric kinds; for small checked-in fixtures)")
	_ = fs.Parse(args)
	var siteStrs [][]byte
	if *site != "" {
		data, err := os.ReadFile(*site)
		if err != nil {
			return err
		}
		for _, l := range strings.Split(string(data), "\n") {
			if l == "" {
				continue
			}
			b, err := hex.DecodeString(l)
			if err != nil {
				return err
			}
			siteStrs = append(siteStrs, b)
		}
	}
	var pairs [][2][]byte
	switch *kind {
	case "random":
		pairs = genPairs(*seed, *n, siteStrs)
	case "numeric":
		pairs = numericPairs(false)
	case "nondigits":
		pairs = numericPairs(true)
	default:
		return fmt.Errorf("unknown kind %q", *kind)
	}
	if *every > 1 {
		var kept [][2][]byte
		for i := 0; i < len(pairs); i += *every {
			kept = append(kept, pairs[i])
		}
		pairs = kept
	}
	*n = len(pairs)
	f, err := os.Create(*out)
	if err != nil {
		return err
	}
	w := bufio.NewWriter(f)
	fmt.Fprintf(w, "N\t%d\t%d\n", *n, *seed)
	for _, p := range pairs {
		fmt.Fprintf(w, "P\t%s\t%s\n", hex.EncodeToString(p[0]), hex.EncodeToString(p[1]))
	}
	cfgs := pairConfigs()
	nmain := len(cfgs)
	if *locales {
		cfgs = append(cfgs, localeConfigs()...)
	}
	for i, cfg := range cfgs {
		res, kd := pairResults(cfg.collator(), pairs)
		h := sha256.Sum256(res)
		fmt.Fprintf(w, "C\t%s\t%s\t%s", cfg.line(), hex.EncodeToString(h[:]), kd)
		if *full == "all" || *full == "main" && i < nmain {
			fmt.Fprintf(w, "\t%s", res)
		}
		w.WriteByte('\n')
	}
	if err := w.Flush(); err != nil {
		return err
	}
	return f.Close()
}

// numericPairs mirrors colltab's TestNumericCompare (every Nd range of the
// golden toolchain's unicode tables, every block of ten digits, all
// prefixes) and TestNonDigits (every non-Nd rune of unicode.N against "0"
// and "999999"; with all=true every rune of the Go test's range), plus
// digit runs with leading zeros, > 30 digits, marks and ignorables.
func numericPairs(all bool) [][2][]byte {
	var out [][2][]byte
	add := func(a, b string) { out = append(out, [2][]byte{[]byte(a), []byte(b)}) }
	lo, hi := rune(unicode.N.R16[0].Lo), rune(unicode.N.R32[0].Hi)
	if all {
		for r := lo; r <= hi; r++ {
			if !unicode.In(r, unicode.Nd) && utf8.ValidRune(r) {
				add(string(r), "0")
				add(string(r), "999999")
			}
		}
		return out
	}
	for r := lo; r <= 0x10FFFF; r++ {
		if unicode.In(r, unicode.N) && !unicode.In(r, unicode.Nd) {
			add(string(r), "0")
			add(string(r), "999999")
			add(string(r)+"1", "1"+string(r))
		}
	}
	prefixes := []struct {
		prefix string
		b      [11]string
	}{
		{"", [11]string{"0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"}},
		{"1", [11]string{"10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20"}},
		{"0", [11]string{"00", "01", "02", "03", "04", "05", "06", "07", "08", "09", "10"}},
		{"00", [11]string{"000", "001", "002", "003", "004", "005", "006", "007", "008", "009", "010"}},
		{"9", [11]string{"90", "91", "92", "93", "94", "95", "96", "97", "98", "99", "100"}},
	}
	var zeros []rune
	for _, rt := range unicode.Nd.R16 {
		for z := rune(rt.Lo); z+9 <= rune(rt.Hi); z += 10 {
			zeros = append(zeros, z)
		}
	}
	for _, rt := range unicode.Nd.R32 {
		for z := rune(rt.Lo); z+9 <= rune(rt.Hi); z += 10 {
			zeros = append(zeros, z)
		}
	}
	conv := func(s string, z rune) string {
		var b []byte
		for _, c := range s {
			b = utf8.AppendRune(b, z+(c-'0'))
		}
		return string(b)
	}
	for _, z := range zeros {
		for _, tt := range prefixes {
			for i := 0; i < 10; i++ {
				a := tt.prefix + string(z+rune(i))
				for _, b := range tt.b {
					add(a, b)
					add(a, conv(b, z))
					add(conv(tt.prefix, z)+string(z+rune(i)), b)
				}
			}
		}
		// Long runs, leading zeros, separators, marks, ignorables.
		d := func(s string) string { return conv(s, z) }
		add(d("0000000000000000000000000000000000001"), d("1"))
		add(d(strings.Repeat("9", 40)), d("1"+strings.Repeat("0", 40)))
		add(d(strings.Repeat("1", 35)), d(strings.Repeat("1", 34)+"2"))
		add(d("12")+"a", d("012")+"b")
		add(d("12")+" "+d("3"), d("12")+d("3"))
		add(d("1")+"\u0301"+d("2"), d("12"))
		add(d("1")+"\u200b"+d("2"), d("12"))
		add(d("1")+"."+d("5"), d("1")+"."+d("50"))
		add(d("1")+","+d("000"), d("1000"))
		add("v"+d("1")+"."+d("10"), "v"+d("1")+"."+d("9"))
		add(d("0"), "")
		add(d("00"), d("0"))
		add(d("0")+"a", "a")
	}
	return out
}
