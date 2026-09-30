// Command matcher is the Go oracle for the language matching go-i18n does
// (golang.org/x/text/language NewMatcher/Match/ParseAcceptLanguage and
// go-i18n's art-aware matcher), as crates/nh-i18n ports it in
// src/xlanguage.rs and src/goi18n/bundle.rs (Wave B task T17).
//
//	go run ./tools/go-oracle/nh-i18n/matcher [-out rust/testdata/oracle/i18n/matcher]
//
// Inputs: a pool of tag strings (the i18n file names Hugo sites use: plain
// languages, regional and script variants, deprecated and macro codes,
// grandfathered tags, art-x-/x- private tags, und-based tags, variants,
// extensions) and deterministic pseudo-random supported lists of 1-7 of them.
// Cases:
//   - "match": language.NewMatcher(supported).Match(want...) index and
//     confidence, for every supported tag as the only want (the way go-i18n's
//     localizers match), for random single wants from the pool, and for random
//     lists of 2-3 wants;
//   - "bundle": a go-i18n Bundle with supported[0] as default language and
//     one message per other tag (AddMessages; tags without plural rule are
//     skipped like Hugo's art-x- retry would not apply here), and a Localizer
//     per bundle tag and per pool tag: which bundle tag answers;
//   - "accept": language.ParseAcceptLanguage of each pool string and of
//     Accept-Language headers with weights.
//
// Output: matcher.json.gz. Nothing here depends on the platform.
package main

import (
	"flag"
	"fmt"
	"log"
	"math/rand"
	"path/filepath"
	"strings"

	"github.com/gohugoio/go-i18n/v2/i18n"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"golang.org/x/text/language"
)

var pool = []string{
	"en", "en-US", "en-GB", "en-AU", "en-CA", "en-IN", "en-NZ", "en-IE", "en-ZA", "en-001", "en-150", "en-Latn", "en-Latn-US",
	"en-GB-oed", "en-US-u-co-phonebk", "en-x-foo",
	"th", "th-TH", "th-Thai", "und-TH", "und-Thai",
	"fr", "fr-FR", "fr-CA", "fr-CH", "fr-BE",
	"de", "de-DE", "de-AT", "de-CH", "gsw", "de-1996",
	"es", "es-ES", "es-419", "es-MX", "es-AR", "es-US",
	"pt", "pt-BR", "pt-PT", "pt-AO", "pt-br", "pt_PT",
	"zh", "zh-CN", "zh-TW", "zh-HK", "zh-SG", "zh-Hans", "zh-Hant", "zh-Hant-TW", "zh-Hans-CN", "zh-cn", "zh-tw", "cmn", "yue", "zh-yue",
	"sr", "sr-Latn", "sr-Cyrl", "sr-ME", "sh", "bs", "hr",
	"no", "nb", "nn", "da", "sv",
	"iw", "he", "in", "id", "ms", "tl", "fil", "mo", "ro", "ji", "yi", "jw", "jv",
	"ar", "ar-EG", "ar-SA", "fa", "ur", "hi", "bn", "pa", "pa-Arab",
	"ja", "ko", "ru", "uk", "be", "pl", "cs", "sk", "sl", "sl-rozaj", "lt", "lv", "mt", "ga", "cy", "br", "gd", "is",
	"oc", "tlh", "i-klingon", "art-x-klingon", "art-x-a1", "art-x-a2", "art-x-foo", "art", "x-foo", "x-a1",
	"und", "und-US", "und-Latn", "und-Cyrl", "und-419", "und-150", "mul",
	"kk", "az", "az-Cyrl", "uz", "tg", "mn", "ky", "af", "sw", "am", "ti", "so", "tr", "el", "hu", "fi", "et", "eu", "ca", "gl",
	"vi", "km", "lo", "my", "si", "ta", "te", "kn", "ml", "mr", "gu", "ne",
}

func main() {
	out := flag.String("out", "rust/testdata/oracle/i18n/matcher", "output directory")
	flag.Parse()

	rng := rand.New(rand.NewSource(17))
	var cases []map[string]any

	pick := func(n int) []string {
		seen := map[string]bool{}
		var s []string
		for len(s) < n {
			t := pool[rng.Intn(len(pool))]
			if !seen[t] {
				seen[t] = true
				s = append(s, t)
			}
		}
		return s
	}

	for i := 0; i < 1500; i++ {
		supported := pick(1 + rng.Intn(7))
		var wants [][]string
		for _, s := range supported {
			wants = append(wants, []string{s})
		}
		for j := 0; j < 4; j++ {
			wants = append(wants, []string{pool[rng.Intn(len(pool))]})
		}
		wants = append(wants, pick(2+rng.Intn(2)))
		cases = append(cases, matchCase(supported, wants))
		if i < 600 {
			cases = append(cases, bundleCase(supported))
		}
	}
	// Every pool tag against every pool tag, one to one.
	for _, s := range pool {
		var wants [][]string
		for _, w := range pool {
			wants = append(wants, []string{w})
		}
		cases = append(cases, matchCase([]string{"en", s}, wants))
	}
	// No wants: the default.
	cases = append(cases, matchCase([]string{"th", "en"}, [][]string{{}}))

	accepts := append([]string{}, pool...)
	accepts = append(accepts,
		"en;q=0.5, th", "th;q=0.1,en;q=0.9", "fr, de;q=0", "*", "english", "deutsch, french", "en;q=abc", "en;q=",
		"en,,th", " en , th ", "en;q=0.5;q=0.7", "en-US;q=1.0, en;q=0.8, *;q=0.1", "", "   ", "xx-invalid-!!", "en_US",
		"de-CH;q=0.9,de;q=0.9,en;q=0.9", "q=0.5", "en;Q=0.5", "en ; q = 0.5",
	)
	for _, a := range accepts {
		cases = append(cases, acceptCase(a))
	}

	if err := goval.WriteCasesGz(filepath.Join(*out, "matcher.json.gz"), map[string]any{}, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Println("wrote", len(cases), "cases")
}

func parseAll(ss []string) []language.Tag {
	var ts []language.Tag
	for _, s := range ss {
		ts = append(ts, language.Make(s))
	}
	return ts
}

func matchCase(supported []string, wants [][]string) map[string]any {
	m := language.NewMatcher(parseAll(supported))
	var res []any
	for _, w := range wants {
		_, i, c := m.Match(parseAll(w)...)
		res = append(res, map[string]any{"want": w, "index": i, "conf": c.String()})
	}
	return map[string]any{"kind": "match", "supported": supported, "results": res}
}

func bundleCase(supported []string) (out map[string]any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"kind": "bundle", "supported": supported, "panic": fmt.Sprint(r)}
		}
	}()
	b := i18n.NewBundle(language.Make(supported[0]))
	var added []string
	for _, s := range supported {
		t := language.Make(s)
		if err := b.AddMessages(t, &i18n.Message{ID: "m", Other: "msg:" + t.String()}); err != nil {
			added = append(added, "err:"+err.Error())
			continue
		}
		added = append(added, t.String())
	}
	var tags []string
	for _, t := range b.LanguageTags() {
		tags = append(tags, t.String())
	}
	var res []any
	langs := append(append([]string{}, tags...), pool...)
	for _, l := range langs {
		loc := i18n.NewLocalizer(b, l)
		s, t, err := loc.LocalizeWithTag(&i18n.LocalizeConfig{MessageID: "m"})
		r := map[string]any{"lang": l, "s": s, "tag": t.String()}
		if err != nil {
			r["err"] = err.Error()
		}
		res = append(res, r)
	}
	return map[string]any{"kind": "bundle", "supported": supported, "added": added, "tags": tags, "results": res}
}

func acceptCase(s string) map[string]any {
	tags, q, err := language.ParseAcceptLanguage(s)
	r := map[string]any{"kind": "accept", "s": s}
	if err != nil {
		r["err"] = err.Error()
		return r
	}
	var ts []string
	for _, t := range tags {
		ts = append(ts, t.String())
	}
	var qs []string
	for _, x := range q {
		qs = append(qs, fmt.Sprintf("%v", x))
	}
	r["tags"] = strings.Join(ts, ",")
	r["q"] = strings.Join(qs, ",")
	return r
}
