package main

// Large adversarial language-tag corpus for crates/xtext-collate: the same
// per-tag checks as tags.tsv (Parse error/result, Make, All.Canonicalize,
// Base, Parent, MatchLang, keys under collate.New(Make(s))), over random
// tags built from subtag pools and character-level mutations.

import (
	"encoding/hex"
	"flag"
	"os"
	"strings"

	"golang.org/x/text/language"
)

func genTag(r *rng) string {
	subtags := []string{"en", "th", "de", "zh", "sr", "und", "root", "x", "u", "t", "a", "b", "i", "co", "va", "ka", "ks", "kn", "kb", "kc",
		"kf", "kr", "vt", "nu", "ca", "rg", "sd", "ms", "hc", "lb", "lw", "tz", "cu", "phonebk", "posix", "pinyin", "stroke", "standard",
		"search", "trad", "dict", "unihan", "zhuyin", "big5han", "gb2312", "eor", "emoji", "shifted", "blanked", "noignore", "level1",
		"level2", "level3", "level4", "identic", "true", "false", "yes", "no", "Latn", "Hant", "Hans", "Cyrl", "Thai", "Arab", "Zzzz",
		"Zyyy", "Qaaa", "Qabx", "US", "TW", "TH", "GB", "UK", "DE", "DD", "419", "999", "001", "150", "002", "003", "005", "009", "013",
		"1901", "1996", "1606nict", "valencia", "rozaj", "biske", "njiva", "osojs", "solba", "oxendict", "fonipa", "fonupa", "heploc",
		"abc", "abcdefgh", "abcdefghi", "12", "123", "1234", "a1", "1a", "Z", "", "yue", "cmn", "nan", "hak", "nb", "no", "nn", "sh", "mo",
		"iw", "in", "ji", "jw", "tl", "fil", "qaa", "qtz", "qtx", "zxx", "mul", "mis", "oed", "gb", "attr", "cu", "usd", "m0", "ungegn",
		"private", "hk", "ca", "es", "fr", "ja", "ko", "pt", "BR", "PT", "pa", "PK", "az", "IR", "uz", "AF", "ha", "NE", "bs", "hr",
		"sgn", "BE", "FR", "zh-min-nan", "i-klingon", "art-lojban", "en-GB-oed", "x-klingon", "ar", "fa", "ur", "hi", "bn", "ta", "si",
		"my", "km", "lo", "bo", "ka", "hy", "el", "he", "yi", "ru", "uk", "be", "bg", "mk", "kk", "ky", "mn", "tt", "ug", "dz", "am"}
	seps := []string{"-", "-", "-", "_", "--", "-_", " ", ""}
	n := 1 + r.intn(8)
	var sb strings.Builder
	for j := 0; j < n; j++ {
		if j > 0 {
			sb.WriteString(seps[r.intn(len(seps))])
		}
		p := subtags[r.intn(len(subtags))]
		switch r.intn(6) {
		case 0:
			p = strings.ToUpper(p)
		case 1:
			if p != "" {
				p = strings.ToUpper(p[:1]) + p[1:]
			}
		case 2:
			// random alnum subtag of random length
			const an = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
			k := r.intn(10)
			b := make([]byte, k)
			for i := range b {
				b[i] = an[r.intn(len(an))]
			}
			p = string(b)
		}
		sb.WriteString(p)
	}
	s := sb.String()
	// Character-level mutations.
	if r.intn(5) == 0 && len(s) > 0 {
		b := []byte(s)
		switch r.intn(4) {
		case 0:
			b[r.intn(len(b))] = "-_ .x0aZ"[r.intn(8)]
		case 1:
			i := r.intn(len(b))
			b = append(b[:i], b[i+1:]...)
		case 2:
			i := r.intn(len(b) + 1)
			b = append(b[:i], append([]byte{"-_xu0"[r.intn(5)]}, b[i:]...)...)
		default:
			b = append(b, b...)
		}
		s = string(b)
	}
	return s
}

// genWellFormedTag builds mostly well-formed tags:
// lang[-script][-region][-variant...][-u-key-type...][-t-...][-a-...][-x-...].
func genWellFormedTag(r *rng) string {
	langs := []string{"en", "th", "de", "zh", "sr", "und", "yue", "cmn", "nb", "no", "sh", "mo", "iw", "tl", "fil", "ja", "ko",
		"es", "pt", "fr", "ca", "bs", "hr", "pa", "az", "uz", "ha", "ar", "fa", "hi", "ru", "uk", "el", "he", "am", "qaa", "zxx",
		"mul", "sgn", "art", "i", "x", "gsw", "als", "nds", "fy", "tlh", "abc", "ab", "aa", "zu", "sv", "fi", "da", "is", "lt", "lv"}
	scripts := []string{"Latn", "Hant", "Hans", "Cyrl", "Thai", "Arab", "Zzzz", "Zyyy", "Qaaa", "Deva", "Grek", "Hebr", "Jpan",
		"Kore", "Hira", "Kana", "Brai", "Zinh", "Aran", "Guru"}
	regions := []string{"US", "TW", "TH", "GB", "UK", "DE", "DD", "419", "999", "001", "150", "CN", "HK", "MO", "SG", "BR",
		"PT", "ES", "MX", "RS", "ME", "BA", "XK", "ZZ", "QO", "AA", "XA", "EU", "UN", "CS", "YU", "SU", "FX", "AN", "BU", "TP", "ZR"}
	variants := []string{"1901", "1996", "1606nict", "valencia", "rozaj", "biske", "njiva", "osojs", "solba", "oxendict",
		"fonipa", "fonupa", "heploc", "posix", "pinyin", "wadegile", "polyton", "monoton", "baku1926", "saaho", "abcde", "12345"}
	keys := [][]string{
		{"co", "phonebk", "pinyin", "stroke", "standard", "search", "trad", "dict", "unihan", "zhuyin", "big5han", "gb2312", "eor", "emoji", "reformed", "compat", "foo"},
		{"va", "posix", "foo", "0abc"},
		{"ka", "shifted", "blanked", "posix", "noignore"},
		{"ks", "level1", "level2", "level3", "level4", "identic"},
		{"kn", "true", "false", ""},
		{"kb", "true", "false"},
		{"kc", "true", "false"},
		{"kf", "upper", "lower"},
		{"nu", "thai", "latn", "arab"},
		{"ca", "buddhist", "gregory"},
		{"rg", "uszzzz", "gbzzzz", "thzzzz", "dezzzz", "twzzzz", "cnzzzz", "zzzzzz", "ptzzzz", "brzzzz", "rszzzz", "ukzzzz", "xxzzzz", "usxxxx", "chzzzz", "atzzzz", "hkzzzz"},
		{"sd", "gbsct", "usca"},
		{"cu", "usd", "thb"},
	}
	var parts []string
	parts = append(parts, pick(r, langs))
	if r.intn(4) == 0 {
		parts = append(parts, pick(r, scripts))
	}
	if r.intn(3) == 0 {
		parts = append(parts, pick(r, regions))
	}
	for k := r.intn(4); k > 2; k-- {
		parts = append(parts, pick(r, variants))
	}
	if r.intn(10) == 0 {
		parts = append(parts, pick(r, variants))
	}
	if r.intn(2) == 0 {
		parts = append(parts, "u")
		if r.intn(8) == 0 {
			parts = append(parts, "attr")
		}
		for k := 1 + r.intn(3); k > 0; k-- {
			kv := pick(r, keys)
			parts = append(parts, kv[0])
			if v := kv[1+r.intn(len(kv)-1)]; v != "" {
				parts = append(parts, v)
			}
		}
	}
	if r.intn(10) == 0 {
		parts = append(parts, "t", pick(r, langs), "m0", "ungegn")
	}
	if r.intn(12) == 0 {
		parts = append(parts, "a", "bcd")
	}
	if r.intn(10) == 0 {
		parts = append(parts, "x", "priv", "u", "co")
	}
	for i := range parts {
		switch r.intn(12) {
		case 0:
			parts[i] = strings.ToUpper(parts[i])
		case 1:
			parts[i] = strings.ToLower(parts[i])
		}
	}
	sep := "-"
	if r.intn(10) == 0 {
		sep = "_"
	}
	return strings.Join(parts, sep)
}

// tagFuzzRegressions are the inputs of the bugs listed in PORTING.md
// ("Bugs found by adversarial verification"): duplicate -u keys / duplicate
// variants followed by another extension (Go reads the shifted scanner
// buffer through a stale token), -u-rg-XXzzzz compact tags, and the 8-bit
// script overflow of CompactCoreInfo.
var tagFuzzRegressions = []string{
	"hr-u-ka-noignore-ka-shifted-x-PRIV-u-co",
	"uk-u-co-zhuyin-kc-false-co-search-a-BCD",
	"ru-PT-valencia-u-va-posix-ka-posix-ka-posix-a-BCD",
	"mo_u_co_search_kn_false_co_UNIHAN_A_bcd",
	"sl-rozaj-biske-rozaj-a-bcd",
	"ca-ES-valencia-valencia-u-co-trad",
	"de-1901-1996-1901-x-priv",
	"en-u-co-phonebk-co-pinyin-t-zh-m0-ungegn",
	"th-TH-u-rg-thzzzz",
	"en-u-rg-gbzzzz",
	"en-US-u-rg-gbzzzz",
	"th-u-rg-uszzzz",
	"de-u-co-phonebk-rg-chzzzz",
	"zh-u-rg-twzzzz",
	"en-u-rg-zzzzzz",
	"en-u-rg-usxxxx",
	"pa-CN",
	"pa-Zzzz",
}

func cmdTagFuzz(args []string) error {
	fs := flag.NewFlagSet("tagfuzz", flag.ExitOnError)
	out := fs.String("out", "", "output file")
	n := fs.Int("n", 100000, "tags")
	seed := fs.Uint64("seed", 11, "seed")
	in := fs.String("in", "", "file of explicit tag inputs (one per line, \"hex:\"-encoded like column 1) written before the n fuzzed tags")
	regressions := fs.Bool("regressions", false, "write tagFuzzRegressions before the fuzzed tags")
	_ = fs.Parse(args)
	r := &rng{*seed}
	seen := map[string]bool{}
	var lines []string
	if *regressions {
		for _, s := range tagFuzzRegressions {
			if !seen[s] {
				seen[s] = true
				lines = append(lines, tagLine(s, probeStrings))
			}
		}
	}
	if *in != "" {
		data, err := os.ReadFile(*in)
		if err != nil {
			return err
		}
		for _, l := range strings.Split(strings.TrimSuffix(string(data), "\n"), "\n") {
			s := l
			if h, ok := strings.CutPrefix(l, "hex:"); ok {
				b, err := hex.DecodeString(h)
				if err != nil {
					return err
				}
				s = string(b)
			}
			if seen[s] {
				continue
			}
			seen[s] = true
			lines = append(lines, tagLine(s, probeStrings))
		}
	}
	for want := len(lines) + *n; len(lines) < want; {
		var s string
		if r.intn(2) == 0 {
			s = genTag(r)
		} else {
			s = genWellFormedTag(r)
		}
		if seen[s] {
			continue
		}
		seen[s] = true
		lines = append(lines, tagLine(s, probeStrings))
	}
	return writeLines(*out, lines)
}

var setTypeCases = [][2]string{
	{"co", "phonebk"}, {"co", ""}, {"va", "posix"}, {"va", ""}, {"ka", "shifted"}, {"kn", "true"},
	{"rg", "gbzzzz"}, {"rg", "thzzzz"}, {"rg", ""}, {"kn", "ab-kn-cd"}, {"kn", "kn-ab"}, {"co", "abc-co"},
	{"co", "co-abc"}, {"kn", "ABC"}, {"co", "PHONEBK"}, {"xx", "a-b"}, {"co", "toolongvalue"},
	{"ka", "ab_cd"}, {"k", "abc"}, {"kkk", "abc"}, {"co", "ab"}, {"co", "a-bcd"}, {"co", "abc-"},
	{"co", "-abc"}, {"co", "ab--cd"}, {"cu", "usd-eur"}, {"nu", "thai"}, {"ab", "cd-ab-ef"},
	{"zz", "aa-bb-cc"}, {"co", "x-abc"}, {"co", "u-abc"}, {"co", "abc-u-de"}, {"co", "ab-x-cd"},
}

// cmdSetType writes Tag.SetTypeForKey results for adversarial key/value
// pairs over fuzzed tags:
//
//	<tag>	<key>	<value>	<result>	<error>	<TypeForKey(key) of result>
func cmdSetType(args []string) error {
	fs := flag.NewFlagSet("settype", flag.ExitOnError)
	out := fs.String("out", "", "output file")
	n := fs.Int("n", 20000, "tags")
	seed := fs.Uint64("seed", 17, "seed")
	_ = fs.Parse(args)
	r := &rng{*seed}
	var lines []string
	for i := 0; i < *n; i++ {
		var s string
		if r.intn(2) == 0 {
			s = genTag(r)
		} else {
			s = genWellFormedTag(r)
		}
		t := language.Make(s)
		for _, kv := range setTypeCases {
			if r.intn(4) != 0 {
				continue
			}
			res, err := t.SetTypeForKey(kv[0], kv[1])
			es := ""
			if err != nil {
				es = err.Error()
			}
			lines = append(lines, strings.Join([]string{encodeInput(s), kv[0], encodeInput(kv[1]), res.String(), es, res.TypeForKey(kv[0])}, "\t"))
		}
	}
	return writeLines(*out, lines)
}
