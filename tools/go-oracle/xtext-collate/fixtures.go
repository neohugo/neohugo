package main

// Differential fixtures for crates/xtext-collate.
//
// Random inputs are produced by a splitmix64 generator that the Rust tests
// reimplement bit for bit (tests/common/mod.rs), so large corpora are
// checked through SHA-256 digests instead of checked-in data.

import (
	"bufio"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"unicode/utf8"
	_ "unsafe" // go:linkname

	"golang.org/x/text/collate"
	"golang.org/x/text/language"
	"golang.org/x/text/unicode/norm"
)

//go:linkname matchLang golang.org/x/text/internal/colltab.MatchLang
func matchLang(t language.Tag, tags []language.Tag) int

// ---------------------------------------------------------------------------
// PRNG (splitmix64) and random string generator; mirrored in Rust.

type rng struct{ s uint64 }

func (r *rng) next() uint64 {
	r.s += 0x9E3779B97F4A7C15
	z := r.s
	z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
	z = (z ^ (z >> 27)) * 0x94D049BB133111EB
	return z ^ (z >> 31)
}

func (r *rng) intn(n int) int { return int(r.next() % uint64(n)) }

func (r *rng) rangeRune(lo, hi int) rune { return rune(lo + r.intn(hi-lo+1)) }

const asciiAlnum = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
const asciiPunct = " -_.,'\"!?()&/:;#@%+*=~^`|<>[]{}\\$"

var digitZeros = []int{0x30, 0x660, 0x6F0, 0x966, 0x9E6, 0xE50, 0xED0, 0xFF10, 0x1D7CE, 0x2080, 0x1369, 0x1040}

var contractionSeeds = []string{"ch", "Ch", "CH", "ll", "dz", "dž", "ŀl", "aa", "Aa", "ng", "ny", "sz", "cs", "gy", "lj", "nj", "rr", "ij", "ae", "oe", "ss", "th", "ǆ", "l·l", "ch́", "å"}

// genRandom appends one random "item" (usually a rune) to b.
func genItem(r *rng, b []byte) []byte {
	k := r.intn(100)
	switch {
	case k < 20:
		return append(b, asciiAlnum[r.intn(len(asciiAlnum))])
	case k < 25:
		return append(b, asciiPunct[r.intn(len(asciiPunct))])
	case k < 45:
		return utf8.AppendRune(b, r.rangeRune(0x0E01, 0x0E5B))
	case k < 53:
		return utf8.AppendRune(b, r.rangeRune(0x00C0, 0x024F))
	case k < 61:
		return utf8.AppendRune(b, r.rangeRune(0x0300, 0x036F))
	case k < 64:
		return utf8.AppendRune(b, r.rangeRune(0xAC00, 0xD7A3))
	case k < 66:
		return utf8.AppendRune(b, r.rangeRune(0x1100, 0x11FF))
	case k < 68:
		return utf8.AppendRune(b, r.rangeRune(0x4E00, 0x9FFF))
	case k < 70:
		return utf8.AppendRune(b, r.rangeRune(0xFF01, 0xFF9F))
	case k < 73:
		z := digitZeros[r.intn(len(digitZeros))]
		return utf8.AppendRune(b, rune(z+r.intn(10)))
	case k < 76:
		return utf8.AppendRune(b, r.rangeRune(0x0370, 0x04FF))
	case k < 79:
		return utf8.AppendRune(b, r.rangeRune(0x0590, 0x097F))
	case k < 82:
		return utf8.AppendRune(b, r.rangeRune(0x2000, 0x2BFF))
	case k < 84:
		return utf8.AppendRune(b, r.rangeRune(0x1F300, 0x1FAFF))
	case k < 87:
		for {
			c := r.rangeRune(0, 0x10FFFF)
			if c < 0xD800 || c > 0xDFFF {
				return utf8.AppendRune(b, c)
			}
		}
	case k < 89:
		return append(b, byte(0x80+r.intn(0x80)))
	case k < 91:
		var t [4]byte
		n := utf8.EncodeRune(t[:], r.rangeRune(0x0800, 0xFFFF))
		return append(b, t[:1+r.intn(n-1)]...)
	case k < 94:
		b = append(b, "aeiouyAEOnNcCsSzZ"[r.intn(17)])
		n := 1 + r.intn(3)
		for i := 0; i < n; i++ {
			if r.intn(4) == 0 {
				b = utf8.AppendRune(b, r.rangeRune(0x0E31, 0x0E4E))
			} else {
				b = utf8.AppendRune(b, r.rangeRune(0x0300, 0x036F))
			}
		}
		return b
	case k < 96:
		switch r.intn(4) {
		case 0:
			return append(b, byte(r.intn(0x20)))
		case 1:
			return append(b, 0x7F)
		case 2:
			return utf8.AppendRune(b, r.rangeRune(0x200B, 0x200F))
		default:
			return utf8.AppendRune(b, 0xFEFF)
		}
	case k < 98:
		return append(b, contractionSeeds[r.intn(len(contractionSeeds))]...)
	default:
		switch r.intn(4) {
		case 0:
			return utf8.AppendRune(b, r.rangeRune(0x2150, 0x218F))
		case 1:
			return utf8.AppendRune(b, r.rangeRune(0x3300, 0x33FF))
		case 2:
			return utf8.AppendRune(b, r.rangeRune(0xFB00, 0xFB06))
		default:
			return utf8.AppendRune(b, r.rangeRune(0x2460, 0x24FF))
		}
	}
}

func genString(r *rng) []byte {
	n := r.intn(12)
	var b []byte
	for i := 0; i < n; i++ {
		b = genItem(r, b)
	}
	return b
}

func genCorpus(seed uint64, n int) [][]byte {
	r := &rng{seed}
	out := make([][]byte, n)
	for i := range out {
		out[i] = genString(r)
	}
	return out
}

var stressStarters = []string{"a", "A", "l", "L", "c", "C", "e", "o", "u", "ı", "i", "z", "s", "n", "y", "ร", "ก", "เ", "ཀ", "ྐ", "क", "ल", "ل", "ا", "ㄱ", "가", "ᄀ", "ᅡ", "ch", "ll", "a\u0308", "\u0301", "", "1", "0"}

var stressMarks = []rune{0x0300, 0x0301, 0x0302, 0x0308, 0x030A, 0x030C, 0x0327, 0x0328, 0x0323, 0x0331, 0x031B, 0x0345, 0x0335, 0x05B0, 0x05B4, 0x0E31, 0x0E38, 0x0E39, 0x0E47, 0x0E48, 0x0E49, 0x0E4C, 0x0F71, 0x0F72, 0x0F74, 0x0F80, 0x1DCE, 0x093C, 0x094D, 0x0B3C, 0x0B4D, 0x3099, 0x309A, 0x0FB5, 0x0FB7, 0x0F90, 0x0FB3, 0x0F81, 0x0F73, 0x0F75}

// genStress produces long runs of combining marks after contraction
// starters (doNorm reordering, maxCombiningCharacters, discontiguous
// contractions and the 128-byte segment buffer).
func genStress(r *rng) []byte {
	var b []byte
	segs := 1 + r.intn(3)
	for i := 0; i < segs; i++ {
		b = append(b, stressStarters[r.intn(len(stressStarters))]...)
		m := r.intn(70)
		for j := 0; j < m; j++ {
			switch r.intn(40) {
			case 0:
				b = append(b, byte(0x80+r.intn(0x40)))
			case 1:
				b = append(b, stressStarters[r.intn(len(stressStarters))]...)
			default:
				b = utf8.AppendRune(b, stressMarks[r.intn(len(stressMarks))])
			}
		}
	}
	return b
}

func genStressCorpus(seed uint64, n int) [][]byte {
	r := &rng{seed}
	out := make([][]byte, n)
	for i := range out {
		out[i] = genStress(r)
	}
	return out
}

// ---------------------------------------------------------------------------
// configurations

type config struct {
	name string
	tag  string
	opts []string
}

var optByName = map[string]collate.Option{
	"loose":            collate.Loose,
	"ignorecase":       collate.IgnoreCase,
	"ignorediacritics": collate.IgnoreDiacritics,
	"ignorewidth":      collate.IgnoreWidth,
	"force":            collate.Force,
	"numeric":          collate.Numeric,
}

func (c config) collator() *collate.Collator {
	var o []collate.Option
	for _, n := range c.opts {
		o = append(o, optByName[n])
	}
	return collate.New(language.Make(c.tag), o...)
}

func (c config) line() string {
	return fmt.Sprintf("%s\t%s\t%s", c.name, c.tag, strings.Join(c.opts, ","))
}

func mainConfigs() []config {
	var cs []config
	for _, t := range []string{"und", "en", "th"} {
		cs = append(cs, config{t, t, nil})
	}
	for _, t := range []string{"und", "th"} {
		for _, o := range []string{"loose", "ignorecase", "ignorediacritics", "ignorewidth", "force", "numeric"} {
			cs = append(cs, config{t + "+" + o, t, []string{o}})
		}
	}
	cs = append(cs, config{"en+ignorecase,ignorewidth", "en", []string{"ignorecase", "ignorewidth"}})
	cs = append(cs, config{"th+numeric,force", "th", []string{"numeric", "force"}})
	for _, t := range []string{
		"en-u-ks-level1", "en-u-ks-level2", "en-u-ks-level4", "en-u-ks-identic",
		"en-u-ka-shifted", "en-u-ka-posix", "en-u-ka-blanked", "en-u-ka-shifted-ks-level4",
		"en-u-ka-posix-ks-level4", "en-u-ka-blanked-ks-level4", "en-u-kb-true", "en-u-kc-true",
		"en-u-kn-true", "th-u-kn-true", "fr-CA", "en-US-u-va-posix",
	} {
		cs = append(cs, config{t, t, nil})
	}
	return cs
}

func localeConfigs() []config {
	var cs []config
	for _, t := range collate.Supported() {
		s := t.String()
		cs = append(cs, config{"loc:" + s, s, nil})
	}
	return cs
}

// ---------------------------------------------------------------------------
// digests

func keyDigest(c *collate.Collator, strs [][]byte) (string, string) {
	hk := sha256.New()
	hc := sha256.New()
	var buf collate.Buffer
	var lb [binary.MaxVarintLen64]byte
	for _, s := range strs {
		buf.Reset()
		k := c.Key(&buf, s)
		hk.Write(lb[:binary.PutUvarint(lb[:], uint64(len(k)))])
		hk.Write(k)
	}
	n := len(strs)
	for i := 0; i < n; i++ {
		a := strs[i]
		b := strs[(i+1)%n]
		d := strs[(i*7919+13)%n]
		hc.Write([]byte{byte(c.Compare(a, b) + 1), byte(c.Compare(a, d) + 1), byte(c.Compare(d, a) + 1)})
	}
	return hex.EncodeToString(hk.Sum(nil)), hex.EncodeToString(hc.Sum(nil))
}

func keyHash(k []byte) string {
	h := sha256.Sum256(k)
	return hex.EncodeToString(h[:8])
}

// ---------------------------------------------------------------------------
// site strings

func readSiteStrings(paths []string) ([][]byte, error) {
	seen := map[string]bool{}
	var out [][]byte
	add := func(s string) {
		if !seen[s] {
			seen[s] = true
			out = append(out, []byte(s))
		}
	}
	for _, p := range paths {
		data, err := os.ReadFile(p)
		if err != nil {
			return nil, err
		}
		switch {
		case strings.HasSuffix(p, ".json"):
			var ss []string
			if err := json.Unmarshal(data, &ss); err != nil {
				return nil, fmt.Errorf("%s: %w", p, err)
			}
			for _, s := range ss {
				add(s)
			}
		case strings.HasSuffix(p, "sortdiff.txt"):
			// SORTDIFF lines hold fmt %v lists; use every list token and the
			// raw list bodies.
			for _, line := range strings.Split(string(data), "\n") {
				for _, f := range strings.Split(line, "\t") {
					i := strings.IndexByte(f, '[')
					if i < 0 || !strings.HasSuffix(f, "]") {
						continue
					}
					body := f[i+1 : len(f)-1]
					add(body)
					for _, tok := range strings.Split(body, " ") {
						add(tok)
					}
				}
			}
		default:
			for _, line := range strings.Split(string(data), "\n") {
				add(line)
			}
		}
	}
	return out, nil
}

// ---------------------------------------------------------------------------
// tags corpus

func tagCorpus() []string {
	seen := map[string]bool{}
	var out []string
	add := func(s string) {
		if !seen[s] {
			seen[s] = true
			out = append(out, s)
		}
	}
	sup := collate.Supported()
	for _, t := range sup {
		s := t.String()
		add(s)
		add(strings.ToLower(s))
		add(strings.ToUpper(s))
		add(strings.ReplaceAll(s, "-", "_"))
	}
	for _, s := range []string{
		"", "en", "th", "und", "root", "x-klingon", "i-klingon", "en-GB-oed", "en_US_POSIX",
		"en-us-posix", "zh-min-nan", "zh-guoyu", "no-bok", "no-nyn", "sgn-BE-FR", "art-lojban",
		"cel-gaulish", "i-default", "i-enochian", "i-mingo", "zh-min", "i-ami", "i-amũ", "i-ami\x00",
		"nb", "no", "nn", "sh", "sr-Latn-RS", "mo", "iw", "in", "ji", "jw", "tl", "fil", "cmn",
		"zh-cmn", "zh-yue", "yue", "zh-Hans", "zh-Hant", "zh-TW", "zh-HK", "zh-MO", "zh-SG", "zh-CN",
		"zh-Hant-TW", "zh-Hans-TW", "zh-Hant-CN", "zh-u-co-pinyin", "zh-Hant-u-co-stroke",
		"de-u-co-phonebk", "de-DE-u-co-phonebk", "de-CH-u-co-phonebk", "de-u-co-phonebk-ka-shifted",
		"en-US-u-va-posix", "en-u-va-posix", "en-GB-u-va-posix", "fi-u-co-standard", "sv-u-co-standard",
		"ln-u-co-phonetic", "es-u-co-trad", "es-419", "es-MX", "pt-BR", "pt-PT", "en-150", "en-001",
		"sr-ME", "sr-Cyrl-ME", "sr-BA", "bs-Cyrl", "bs-Cyrl-BA", "bs-Latn", "uz-AF", "pa-PK", "az-IR",
		"ca-ES-valencia", "ca-valencia", "de-1901", "de-1996-1901", "sl-rozaj-biske-1994",
		"en-u-ks-level1", "en-u-ka-shifted", "en-u-kn-true", "en-u-kb-true", "en-u-kc-false",
		"th-u-kn", "en-u-co-phonebk-co-pinyin", "en-u-attr-co-phonebk", "en-u-cu-usd-co-phonebk",
		"en-a-bcd-u-co-x-private", "en-t-zh-hant-m0-ungegn", "en-x-a-b-c", "x-a", "x-", "-en", "en-",
		"en--us", "en-US-", "a", "abcd", "abcdefghi", "123", "en-123", "en-419", "en-999", "en-000",
		"en-940", "en-958", "en-990", "en-901", "de-DD", "en-UK", "ar-001", "es-ES_tradnl",
		"Latn", "und-Latn", "und-Thai", "und-TH", "und-US", "und-419", "und-Hant", "und-Cyrl-RS",
		"und-150", "und-ZZ", "und-QO", "und-XK", "und-u-co-phonebk", "en-Qaai", "en-Zinh",
		"en-Latn", "en-Latn-US", "th-Thai", "th-TH", "th-Thai-TH", "th-u-ca-buddhist",
		"th-TH-u-nu-thai", "th-Latn", "en-US-POSIX", "EN", "Th", "tH-th", "en_us", "EN_GB",
		"ﬀ", "enü", "én", "en-ü",
	} {
		add(s)
	}
	letters := "abcdefghijklmnopqrstuvwxyz"
	for _, a := range letters {
		for _, b := range letters {
			add(string(a) + string(b))
		}
	}
	for _, a := range letters {
		for _, b := range letters {
			for _, c := range letters {
				add(string(a) + string(b) + string(c))
			}
		}
	}
	regions := []string{"US", "GB", "TW", "CN", "HK", "MO", "SG", "BR", "PT", "419", "001", "150", "TH", "IN", "DE", "AT", "CH", "FR", "CA", "BE", "ES", "MX", "RS", "ME", "BA", "AF", "PK", "IR", "ZZ", "XK", "QO", "029", "142", "AU", "NZ", "IE", "ZA", "JP", "KR"}
	scripts := []string{"Latn", "Cyrl", "Hant", "Hans", "Arab", "Thai", "Deva", "Grek", "Zyyy", "Zzzz", "Qaaa"}
	for _, t := range sup {
		b, _, _ := t.Raw()
		bs := b.String()
		for _, r := range regions {
			add(bs + "-" + r)
		}
		for _, sc := range scripts {
			add(bs + "-" + sc)
			add(bs + "-" + sc + "-" + regions[len(bs)%len(regions)])
		}
		add(bs + "-u-co-phonebk")
		add(bs + "-u-co-standard")
		add(bs + "-u-va-posix")
	}
	// Fuzz: random combinations of plausible subtags.
	r := &rng{0x5eed}
	pool := []string{"en", "th", "de", "zh", "sr", "und", "x", "u", "t", "a", "co", "va", "ka", "ks", "kn", "phonebk", "posix", "pinyin", "stroke", "standard", "shifted", "level1", "true", "Latn", "Hant", "Hans", "Cyrl", "US", "TW", "419", "999", "001", "1901", "1996", "valencia", "rozaj", "biske", "abc", "abcdefgh", "abcdefghi", "12", "123", "1234", "a1", "Z", "", "yue", "cmn", "nb", "no", "sh", "mo", "qaa", "qtz", "zxx", "mul", "i", "oed", "gb", "attr", "attr2", "cu", "usd", "m0", "ungegn", "private", "hk", "ca"}
	seps := []string{"-", "_", "-", "-", "--", " "}
	for i := 0; i < 8000; i++ {
		n := 1 + r.intn(6)
		var sb strings.Builder
		for j := 0; j < n; j++ {
			if j > 0 {
				sb.WriteString(seps[r.intn(len(seps))])
			}
			p := pool[r.intn(len(pool))]
			switch r.intn(4) {
			case 0:
				p = strings.ToUpper(p)
			case 1:
				if p != "" {
					p = strings.ToUpper(p[:1]) + p[1:]
				}
			}
			sb.WriteString(p)
		}
		add(sb.String())
	}
	return out
}

func confName(c language.Confidence) string { return c.String() }

// encodeInput writes s verbatim when it is printable ASCII without tabs and
// does not start with "hex:", else as "hex:" + hex.
func encodeInput(s string) string {
	ok := !strings.HasPrefix(s, "hex:") && s != ""
	for i := 0; i < len(s); i++ {
		if s[i] < 0x21 || s[i] > 0x7e {
			ok = false
		}
	}
	if ok {
		return s
	}
	return "hex:" + hex.EncodeToString([]byte(s))
}

func tagLine(s string, probes []string) string {
	sup := collate.Supported()
	pt, perr := language.Parse(s)
	perrs := ""
	if perr != nil {
		perrs = perr.Error()
	}
	mk := language.Make(s)
	all, _ := language.All.Canonicalize(mk)
	b, conf := mk.Base()
	idx := matchLang(mk, sup)
	// End-to-end: keys of the probe strings under collate.New(language.Make(s)).
	c := collate.New(mk)
	var buf collate.Buffer
	h := sha256.New()
	for _, p := range probes {
		buf.Reset()
		h.Write(c.KeyFromString(&buf, p))
		h.Write([]byte{0xff})
		h.Write([]byte{byte(c.CompareString(p, "a") + 1)})
	}
	ms := mk.String()
	same := func(x string) string {
		if x == ms {
			return "="
		}
		return x
	}
	return strings.Join([]string{
		encodeInput(s),
		perrs,
		same(pt.String()),
		ms,
		same(all.String()),
		b.String(),
		confName(conf),
		same(mk.Parent().String()),
		fmt.Sprint(idx),
		hex.EncodeToString(h.Sum(nil)[:4]),
	}, "\t")
}

var probeStrings = []string{
	"a", "A", "ä", "Ä", "å", "aa", "Aa", "ae", "æ", "b", "c", "č", "ch", "Ch", "CH", "cs", "ç", "d", "dz", "dž", "ǆ", "đ", "ð",
	"e", "é", "è", "ê", "ë", "ə", "f", "g", "ğ", "gy", "h", "i", "ı", "İ", "î", "j", "k", "l", "ł", "ll", "ŀl", "l·l", "lj", "m",
	"n", "ñ", "ng", "nj", "ny", "o", "ö", "ø", "ő", "œ", "oe", "p", "q", "r", "ř", "rr", "s", "ş", "š", "ß", "ss", "sz", "t",
	"th", "þ", "u", "ü", "ű", "v", "w", "x", "y", "ÿ", "z", "ž", "ʒ", "ё", "е", "й", "и", "ї", "і", "ґ", "г", "ў", "у",
	"ع", "غ", "ه", "ی", "ي", "क", "क्ष", "ক", "ত", "ৎ", "ಕ", "ක", "ཀ", "ཀྵ", "ក", "က", "ა", "ㄱ", "가", "각", "あ", "ア", "ｱ", "ゃ",
	"中", "丁", "𠀀", "一", "乙", "ﬃ", "Ⅳ", "①", "½", "12", "2", "10", "a b", "a-b", "a_b", "-", " ", "'", "’", "", "ก", "ข", "เก",
	"กา", "ไก", "ฤ", "ๆ", "ฯ", "๑", "1", "١", "٠", "α", "ά", "ω", "а", "я", "ﾀ", "ｶﾞ", "ガ", "カ", "か", "が", "ǅ", "Ǆ", "ĳ",
}

// ---------------------------------------------------------------------------
// norm properties

func normDigest() (string, int) {
	h := sha256.New()
	n := 0
	var lb [binary.MaxVarintLen64]byte
	put := func(v int) { h.Write(lb[:binary.PutVarint(lb[:], int64(v))]) }
	b2i := func(b bool) int {
		if b {
			return 1
		}
		return 0
	}
	emit := func(s string) {
		p := norm.NFD.PropertiesString(s)
		put(p.Size())
		put(int(p.CCC()))
		put(int(p.LeadCCC()))
		put(int(p.TrailCCC()))
		put(b2i(p.BoundaryBefore()))
		put(b2i(p.BoundaryAfter()))
		d := p.Decomposition()
		put(len(d))
		h.Write(d)
		k := norm.NFKD.PropertiesString(s)
		put(k.Size())
		kd := k.Decomposition()
		put(len(kd))
		h.Write(kd)
		put(norm.NFD.FirstBoundaryInString(s))
		n++
	}
	for r := rune(0); r <= 0x10FFFF; r++ {
		if r >= 0xD800 && r <= 0xDFFF {
			continue
		}
		emit(string(r))
	}
	// Invalid and incomplete byte sequences.
	for a := 0x80; a < 0x100; a++ {
		emit(string([]byte{byte(a)}))
		for b := 0; b < 0x100; b++ {
			emit(string([]byte{byte(a), byte(b)}))
		}
	}
	// FirstBoundary over random strings.
	for _, s := range genCorpus(0xB0B, 50000) {
		if len(s) == 0 {
			continue
		}
		emit(string(s))
	}
	return hex.EncodeToString(h.Sum(nil)), n
}

// ---------------------------------------------------------------------------
// per-rune keys

func runeKeyDigest(c *collate.Collator, step int) string {
	h := sha256.New()
	var buf collate.Buffer
	var lb [binary.MaxVarintLen64]byte
	for r := rune(0); r <= 0x10FFFF; r += rune(step) {
		if r >= 0xD800 && r <= 0xDFFF {
			continue
		}
		buf.Reset()
		k := c.KeyFromString(&buf, string(r))
		h.Write(lb[:binary.PutUvarint(lb[:], uint64(len(k)))])
		h.Write(k)
	}
	return hex.EncodeToString(h.Sum(nil))
}

// ---------------------------------------------------------------------------

func writeLines(path string, lines []string) error {
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	w := bufio.NewWriter(f)
	for _, l := range lines {
		w.WriteString(l)
		w.WriteByte('\n')
	}
	if err := w.Flush(); err != nil {
		return err
	}
	return f.Close()
}

func siteFixtures(dir string, inputs []string) error {
	strs, err := readSiteStrings(inputs)
	if err != nil {
		return err
	}
	var lines []string
	for _, s := range strs {
		lines = append(lines, hex.EncodeToString(s))
	}
	if err := writeLines(filepath.Join(dir, "site-strings.hex"), lines); err != nil {
		return err
	}
	var out []string
	for _, cfg := range []config{{"und", "und", nil}, {"en", "en", nil}, {"th", "th", nil}, {"th+loose", "th", []string{"loose"}}, {"en+numeric", "en", []string{"numeric"}}} {
		c := cfg.collator()
		out = append(out, "C\t"+cfg.line())
		var buf collate.Buffer
		var kh []string
		for _, s := range strs {
			buf.Reset()
			kh = append(kh, keyHash(c.Key(&buf, s)))
		}
		out = append(out, "K\t"+strings.Join(kh, " "))
		// Hugo sorts with sort.SliceStable + CompareString.
		idx := make([]int, len(strs))
		for i := range idx {
			idx[i] = i
		}
		sort.SliceStable(idx, func(i, j int) bool {
			return c.CompareString(string(strs[idx[i]]), string(strs[idx[j]])) < 0
		})
		var is []string
		for _, i := range idx {
			is = append(is, fmt.Sprint(i))
		}
		out = append(out, "O\t"+strings.Join(is, " "))
		kd, cd := keyDigest(c, strs)
		out = append(out, "D\t"+kd+"\t"+cd)
	}
	return writeLines(filepath.Join(dir, "site-keys.txt"), out)
}

func cmdFixtures(args []string) error {
	fs := flag.NewFlagSet("fixtures", flag.ExitOnError)
	dir := fs.String("dir", "crates/xtext-collate/tests/fixtures", "output directory")
	n := fs.Int("n", 3000, "random strings per main config")
	nloc := fs.Int("nloc", 800, "random strings per locale config")
	nstress := fs.Int("nstress", 300, "combining-mark stress strings per config")
	runeStep := fs.Int("runestep", 7, "code point step for per-rune key digests")
	site := fs.String("site", "", "comma-separated site string files (bytes-*.txt, strings.json, sortdiff.txt)")
	_ = fs.Parse(args)
	if err := os.MkdirAll(*dir, 0o755); err != nil {
		return err
	}

	// Tags.
	var tl []string
	for _, s := range tagCorpus() {
		tl = append(tl, tagLine(s, probeStrings))
	}
	if err := writeLines(filepath.Join(*dir, "tags.tsv"), tl); err != nil {
		return err
	}

	// Random corpora digests.
	var dl []string
	dl = append(dl, fmt.Sprintf("N\t%d\t%d\t%d\t%d", *n, *nloc, *runeStep, *nstress))
	main := genCorpus(1, *n)
	for _, cfg := range mainConfigs() {
		kd, cd := keyDigest(cfg.collator(), main)
		dl = append(dl, "R\t"+cfg.line()+"\t"+kd+"\t"+cd)
	}
	loc := genCorpus(2, *nloc)
	for _, cfg := range localeConfigs() {
		kd, cd := keyDigest(cfg.collator(), loc)
		dl = append(dl, "R\t"+cfg.line()+"\t"+kd+"\t"+cd)
	}
	stress := genStressCorpus(3, *nstress)
	for _, cfg := range append(mainConfigs(), localeConfigs()...) {
		kd, cd := keyDigest(cfg.collator(), stress)
		dl = append(dl, "S\t"+cfg.line()+"\t"+kd+"\t"+cd)
	}
	for _, t := range []string{"und", "th"} {
		c := collate.New(language.Make(t))
		dl = append(dl, fmt.Sprintf("P\t%s\t%s", t, runeKeyDigest(c, *runeStep)))
	}
	nd, nn := normDigest()
	dl = append(dl, fmt.Sprintf("M\t%d\t%s", nn, nd))
	if err := writeLines(filepath.Join(*dir, "digests.txt"), dl); err != nil {
		return err
	}

	if *site != "" {
		if err := siteFixtures(*dir, strings.Split(*site, ",")); err != nil {
			return err
		}
	}
	return nil
}

// cmdCorpus writes large-corpus digests (and optionally per-string key
// hashes for mismatch localisation) outside the repository.
func cmdCorpus(args []string) error {
	fs := flag.NewFlagSet("corpus", flag.ExitOnError)
	out := fs.String("out", "", "output file")
	n := fs.Int("n", 200000, "random strings")
	seed := fs.Uint64("seed", 1, "corpus seed")
	cfgName := fs.String("config", "", "only this config; also dumps per-string key hashes")
	all := fs.Bool("locales", false, "also run every supported locale")
	kind := fs.String("kind", "random", "random|stress")
	runes := fs.Bool("runes", false, "also write per-rune key digests (every code point) for all configs")
	_ = fs.Parse(args)
	var strs [][]byte
	switch *kind {
	case "random":
		strs = genCorpus(*seed, *n)
	case "stress":
		strs = genStressCorpus(*seed, *n)
	default:
		return fmt.Errorf("unknown kind %q", *kind)
	}
	var lines []string
	lines = append(lines, fmt.Sprintf("N\t%d\t%d\t%s", *n, *seed, *kind))
	cfgs := mainConfigs()
	if *all {
		cfgs = append(cfgs, localeConfigs()...)
	}
	for _, cfg := range cfgs {
		if *cfgName != "" && cfg.name != *cfgName {
			continue
		}
		c := cfg.collator()
		if *cfgName != "" {
			var buf collate.Buffer
			for i, s := range strs {
				buf.Reset()
				lines = append(lines, fmt.Sprintf("H\t%d\t%s\t%s", i, keyHash(c.Key(&buf, s)), hex.EncodeToString(s)))
			}
		}
		kd, cd := keyDigest(c, strs)
		lines = append(lines, "R\t"+cfg.line()+"\t"+kd+"\t"+cd)
		if *runes {
			lines = append(lines, "P\t"+cfg.line()+"\t"+runeKeyDigest(c, 1))
		}
	}
	if *out == "" {
		for _, l := range lines {
			fmt.Println(l)
		}
		return nil
	}
	return writeLines(*out, lines)
}
