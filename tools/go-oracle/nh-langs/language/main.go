// Command language is the Go oracle for crates/nh-langs (Wave B task T03):
// langs.NewLanguage (tag, collator tag, translator, location, errors) and
// langs.DecodeConfig (mitchellh/mapstructure WeakDecode into
// map[string]LanguageConfig).
//
//	go run ./tools/go-oracle/nh-langs/language [-root .] [-out crates/nh-langs/tests/fixtures/language]
//
// Inputs: the languages of the committed seeksnack config dumps
// (docs/rust-port/specs/architecture-core-data/config-{en,th}.json: en, th and
// the nine disabled languages), BCP 47 edge cases (unknown, private use,
// grandfathered, malformed, case variants, extensions), time zones, and
// adversarial language maps (weights as strings/floats/bools, nil values,
// wrong kinds, mixed-case keys, _merge).
//
// mapstructure converts out-of-range floats with int64(f), which differs
// between arm64 and amd64, so the fixture comes from an arm64 build run under
// qemu-aarch64-static (see crates/nh-langs/PORTING.md).
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"math"
	"os"
	"path/filepath"
	"runtime"
	"sort"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/langs"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-parser/tval"
	"golang.org/x/text/language"
)

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-langs/tests/fixtures/language", "output directory")
	flag.Parse()

	var cases []map[string]any

	// DecodeConfig on the seeksnack language maps (as decoded from JSON: numbers are float64).
	var langKeys []string
	for _, f := range []string{"config-en.json", "config-th.json"} {
		b, err := os.ReadFile(filepath.Join(*root, "docs/rust-port/specs/architecture-core-data", f))
		if err != nil {
			log.Fatal(err)
		}
		var cfg map[string]any
		if err := json.Unmarshal(b, &cfg); err != nil {
			log.Fatal(err)
		}
		m := cfg["languages"].(map[string]any)
		cases = append(cases, decodeCase("seeksnack:"+f, m))
		for k := range m {
			langKeys = append(langKeys, k)
		}
		// The same map as maps.Params values (the way allconfig passes them).
		p := map[string]any{}
		for k, v := range m {
			p[k] = maps.Params(v.(map[string]any))
		}
		cases = append(cases, decodeCase("seeksnack-params:"+f, p))
	}

	for i, m := range adversarialMaps() {
		cases = append(cases, decodeCase(fmt.Sprintf("adv#%d", i), m))
	}

	// NewLanguage.
	tags := append([]string{}, langKeys...)
	tags = append(tags,
		"en", "th", "", "und", "en-US", "en-us", "EN", "en_US", "th-TH", "th-TH-u-nu-thai", "th-u-co-trad",
		"sr-Latn", "zh-Hant-TW", "zh-cn", "zh-CN", "zh-tw", "x-klingon", "i-klingon", "tlh", "abc-def-ghi",
		"invalid!", "de-1996", "de-u-co-phonebk", "fil", "no", "nb", "iw", "he", "sh", "mo", "pt-BR", "es-419",
		"en-GB-oed", "root", "a", "toolongsubtag", "en-", "-en", "fr-FR", "ja-JP", "nl", "pl", "ar", "hi",
		"ko", "ru", "uk", "vi", "tr", "az", "lt", "sv", "da", "fi", "is", "et", "lv", "cs", "sk", "hu", "ro",
		"hr", "sl", "bg", "mk", "el", "ka", "hy", "fa", "ur", "bn", "ta", "te", "kn", "ml", "si", "lo", "km",
		"my", "mn", "bo", "ln", "ha", "yo", "zu", "cy", "ga", "eu", "ca", "gl", "haw", "chr", "ee", "fo",
		"kl", "nn", "se", "smn", "wae", "yue", "zh-Hans", "zh-Hant", "ja-u-co-unihan", "ko-u-co-search",
	)
	sort.Strings(tags)
	seen := map[string]bool{}
	for _, tag := range tags {
		if seen[tag] {
			continue
		}
		seen[tag] = true
		for _, tz := range []string{"", "UTC"} {
			cases = append(cases, languageCase(tag, "en", tz))
		}
	}
	for _, tz := range []string{"Asia/Bangkok", "Europe/Oslo", "America/New_York", "bogus/zone", "utc", "GMT", "Etc/GMT+7", "../etc/passwd"} {
		cases = append(cases, languageCase("en", "en", tz))
	}
	for _, dcl := range []string{"th", "fr", "zz", ""} {
		cases = append(cases, languageCase("zz", dcl, ""))
		cases = append(cases, languageCase("zh-cn", dcl, ""))
	}

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	header := map[string]any{"oracle": "nh-langs/language", "goarch": runtime.GOARCH}
	if err := goval.WriteCasesGz(filepath.Join(*out, "language.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "language: %d cases\n", len(cases))
}

func decodeCase(name string, m map[string]any) map[string]any {
	c := map[string]any{"op": "DecodeConfig", "name": name, "in": tval.Encode(m)}
	langsCfg, err := langs.DecodeConfig(m)
	if err != nil {
		c["err"] = goval.Str(err.Error())
		return c
	}
	var keys []string
	for k := range langsCfg {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	var outs []any
	for _, k := range keys {
		l := langsCfg[k]
		outs = append(outs, []any{goval.Str(k), goval.Str(l.LanguageName), goval.Str(l.LanguageCode), goval.Str(l.Title), goval.Str(l.LanguageDirection), l.Weight, l.Disabled})
	}
	c["out"] = outs
	return c
}

func languageCase(lang, dcl, tz string) map[string]any {
	c := map[string]any{"op": "NewLanguage", "lang": lang, "dcl": dcl, "tz": tz}
	l, err := langs.NewLanguage(lang, dcl, tz, langs.LanguageConfig{LanguageCode: ""})
	tag, perr := language.Parse(lang)
	c["tag"] = tag.String()
	c["parseOK"] = perr == nil
	if err != nil {
		c["err"] = goval.Str(err.Error())
		return c
	}
	c["languageCode"] = l.LanguageCode()
	c["string"] = l.String()
	c["translator"] = langs.GetTranslator(l).Locale()
	c["location"] = langs.GetLocation(l).String()
	// A few collation signs (the full matrix is the collate oracle's job).
	words := []string{"a", "A", "b", "é", "e", "z", "ä", "ö", "o", "ch", "c", "h", "ß", "ss", "ก", "เก", "ข", "1", "-", " "}
	var signs []int
	coll := langs.GetCollator1(l)
	coll2 := langs.GetCollator2(l)
	for _, a := range words {
		for _, b := range words {
			s1 := coll.CompareStrings(a, b)
			if s2 := coll2.CompareStrings(a, b); s2 != s1 {
				log.Fatalf("collators differ for %q", lang)
			}
			signs = append(signs, s1)
		}
	}
	c["signs"] = signs
	return c
}

func adversarialMaps() []map[string]any {
	lc := func(kv ...any) map[string]any {
		m := map[string]any{}
		for i := 0; i < len(kv); i += 2 {
			m[kv[i].(string)] = kv[i+1]
		}
		return m
	}
	return []map[string]any{
		{},
		{"en": nil},
		{"en": map[string]any{}},
		{"en": "not a map"},
		{"en": 1},
		{"en": []any{"a"}},
		{"en": lc("weight", "3", "disabled", "true", "languagename", "English")},
		{"en": lc("weight", "0x10"), "th": lc("weight", "abc"), "fr": lc("weight", "")},
		{"en": lc("weight", true), "th": lc("weight", false), "fr": lc("weight", 2.7), "de": lc("weight", -2.7)},
		{"en": lc("weight", 1e20), "th": lc("weight", -1e20), "fr": lc("weight", math.Inf(1)), "de": lc("weight", math.NaN())},
		{"en": lc("weight", uint64(18446744073709551615)), "th": lc("weight", int64(-5)), "fr": lc("weight", int8(3))},
		{"en": lc("weight", []any{1}), "th": lc("weight", map[string]any{"a": 1}), "fr": lc("weight", nil)},
		{"en": lc("disabled", "yes"), "th": lc("disabled", "1"), "fr": lc("disabled", "t"), "de": lc("disabled", "")},
		{"en": lc("disabled", 0), "th": lc("disabled", 1.5), "fr": lc("disabled", uint(2)), "de": lc("disabled", []any{})},
		{"en": lc("languagename", 42), "th": lc("languagename", 2.5), "fr": lc("languagename", true), "de": lc("languagename", false)},
		{"en": lc("languagename", []any{"a", 1}), "th": lc("languagename", map[string]any{"x": 1}), "fr": lc("languagename", []byte("bytes"))},
		{"en": lc("languagename", 1e21), "th": lc("languagename", math.Copysign(0, -1)), "fr": lc("languagename", 1e-7), "de": lc("languagename", uint64(7))},
		{"en": lc("LanguageName", "Exact", "languagename", "lower"), "th": lc("LANGUAGENAME", "Upper"), "fr": lc("LanguageNAME", "Mixed")},
		{"en": lc("languagecode", "en-US", "title", "T", "languagedirection", "rtl", "weight", 1, "disabled", false, "extra", "ignored")},
		{"_merge": "deep", "en": lc("weight", 1, "_merge", "shallow"), "th": maps.Params{"weight": 2, "_merge": "none"}},
		{"_merge": "deep"},
		{"en": maps.Params{"languagename": "P", "weight": int64(4)}},
		{"en": lc("title", nil, "weight", nil, "disabled", nil)},
		{"en": lc("weight", "1e3"), "th": lc("weight", "  7"), "fr": lc("weight", "-0b11"), "de": lc("weight", "9223372036854775808")},
	}
}
