// Command collate is the Go oracle for the per-language collators of
// crates/nh-langs (Wave B task T03): langs.NewLanguage(tag) builds its
// collators with collate.New(tag) (x/text v0.26.0, CLDR 23), and
// GetCollator1(l).CompareStrings(a, b) is what page and template sorts use.
// Both the `en` and the `th` language are checked (HUGO_LAYER.md §10.10: the
// port never substitutes one tag for the other).
//
//	go run ./tools/go-oracle/nh-langs/collate [-root .] [-out crates/nh-langs/tests/fixtures/collate]
//
// Strings: the T26 corpus (tools/go-oracle/nh-common/corpus: every title,
// term, heading and file name of this repository's content, the seeksnack
// names quoted in the specs, adversarial strings) plus Thai: the Thai words of
// the specs and 1,500 seeded random Thai strings (consonants, pre-posed
// vowels, tone marks, digits, mixed with Latin and punctuation).
//
// Pairs (signs -1/0/1 written as '0'/'1'/'2'): all pairs i < j of every 8th
// string (sorted list), plus 150,000 pseudo-random pairs from a 64-bit LCG
// that the Rust test reproduces: x = x*6364136223846793005 + 1442695040888963407
// (seed 20260928), i = (x >> 33) % n, then j likewise. The stable sort of all
// strings under each collator is recorded too.
package main

import (
	"flag"
	"fmt"
	"log"
	"math/rand"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/langs"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

const (
	coreStride  = 8
	randomPairs = 150000
	seed        = 20260928
)

// Thai words from the specs (seeksnack categories, brands, UI strings).
var thaiWords = []string{
	"ขนม", "มันฝรั่ง", "ไทย", "ประเทศไทย", "ญี่ปุ่น", "เกาหลี", "เลย์ สแตคส์ ", "หน้าแรก", "สมุดไดอารีขนมขบเคี้ยว",
	"ไดอารี่ของการแสวงหาขนมขบเคี้ยว", "ขนมปังกรอบ", "ช็อกโกแลต", "ลูกอม", "เยลลี่", "ป๊อปคอร์น", "อาหารทะเล",
	"ข้าวเกรียบ", "ถั่ว", "สาหร่าย", "เวเฟอร์", "บิสกิต", "เค้ก", "คุกกี้", "แครกเกอร์", "เพรทเซล", "มันฝรั่งทอด",
	"ยี่ห้อ", "หมวดหมู่", "บริษัท", "ประเทศ", "ส่วนผสม", "แท็ก", "เก", "เกา", "แก", "กเ", "โก", "ใก", "ไก",
	"ก่", "ก้", "ก๊", "ก๋", "กั", "กา", "กำ", "กิ", "กี", "กึ", "กื", "กุ", "กู", "ก็", "ก์", "ๆ", "ฯ", "๑", "๒๓",
	"กก", "กข", "ขก", "ฃ", "ฅ", "ฤ", "ฤๅ", "ฦ", "ฦๅ", "อ", "ฮ", "เอ", "แอ", "ไอ", "ใอ", "โอ",
}

var (
	thaiConsonants = []rune("กขฃคฅฆงจฉชซฌญฎฏฐฑฒณดตถทธนบปผฝพฟภมยรฤลฦวศษสหฬอฮ")
	thaiPrevowels  = []rune("เแโใไ")
	thaiMarks      = []rune("ะัาำิีึืุู็่้๊๋์ํ")
	thaiDigits     = []rune("๐๑๒๓๔๕๖๗๘๙")
	extras         = []string{" ", "-", ".", "'", "a", "Z", "1", "é", "ๆ", "ฯ", "\u200b"}
)

func randomThai(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.Intn(8)
	for i := 0; i < n; i++ {
		switch k := r.Intn(12); {
		case k < 6:
			b.WriteRune(thaiConsonants[r.Intn(len(thaiConsonants))])
		case k < 8:
			b.WriteRune(thaiPrevowels[r.Intn(len(thaiPrevowels))])
			b.WriteRune(thaiConsonants[r.Intn(len(thaiConsonants))])
		case k < 10:
			b.WriteRune(thaiMarks[r.Intn(len(thaiMarks))])
		case k < 11:
			b.WriteRune(thaiDigits[r.Intn(len(thaiDigits))])
		default:
			b.WriteString(extras[r.Intn(len(extras))])
		}
	}
	return b.String()
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-langs/tests/fixtures/collate", "output directory")
	flag.Parse()

	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}
	set := map[string]bool{}
	for _, s := range strs {
		set[s] = true
	}
	for _, s := range thaiWords {
		set[s] = true
		set[s+" snack"] = true
		set["Snack "+s] = true
	}
	r := rand.New(rand.NewSource(seed))
	for len(set) < len(strs)+len(thaiWords)*3+1500 {
		set[randomThai(r)] = true
	}
	all := make([]string, 0, len(set))
	for s := range set {
		all = append(all, s)
	}
	sort.Strings(all)
	n := len(all)

	var core []int
	for i := 0; i < n; i += coreStride {
		core = append(core, i)
	}
	type pair struct{ i, j int }
	var pairs []pair
	for a := 0; a < len(core); a++ {
		for b := a + 1; b < len(core); b++ {
			pairs = append(pairs, pair{core[a], core[b]})
		}
	}
	x := uint64(seed)
	next := func() int {
		x = x*6364136223846793005 + 1442695040888963407
		return int((x >> 33) % uint64(n))
	}
	for k := 0; k < randomPairs; k++ {
		i := next()
		j := next()
		pairs = append(pairs, pair{i, j})
	}

	header := map[string]any{"oracle": "nh-langs/collate", "coreStride": coreStride, "randomPairs": randomPairs, "seed": seed, "pairs": len(pairs)}
	enc := make([]any, n)
	for i, s := range all {
		enc[i] = goval.Str(s)
	}
	header["strings"] = enc

	var cases []map[string]any
	for _, tag := range []string{"en", "th"} {
		l, err := langs.NewLanguage(tag, "en", "", langs.LanguageConfig{})
		if err != nil {
			log.Fatal(err)
		}
		coll := langs.GetCollator1(l)
		signs := make([]byte, len(pairs))
		for k, p := range pairs {
			signs[k] = byte('1' + coll.CompareStrings(all[p.i], all[p.j]))
		}
		order := make([]int, n)
		for i := range order {
			order[i] = i
		}
		sort.SliceStable(order, func(a, b int) bool {
			return coll.CompareStrings(all[order[a]], all[order[b]]) < 0
		})
		cases = append(cases, map[string]any{"tag": tag, "signs": string(signs), "sorted": order})
	}

	if err := os.MkdirAll(*out, 0o755); err != nil {
		log.Fatal(err)
	}
	if err := goval.WriteCasesGz(filepath.Join(*out, "collate.json.gz"), header, cases); err != nil {
		log.Fatal(err)
	}
	fmt.Fprintf(os.Stderr, "collate: %d strings, %d pairs x 2 tags\n", n, len(pairs))
}
