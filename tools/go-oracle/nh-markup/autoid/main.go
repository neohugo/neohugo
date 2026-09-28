// Command autoid is the Go oracle for the anchor name sanitizers of
// crates/nh-markup (Wave B task T06): SanitizeAnchorName of neohugo's
// goldmark converter (converter.AnchorNameSanitizer) for the three auto ID
// types (github, github-ascii, blackfriday).
//
//	go run ./tools/go-oracle/nh-markup/autoid [-out crates/nh-markup/tests/fixtures/autoid]
//
// Inputs:
//
//   - a Unicode sweep: every rune below U+3400 and every 13th rune above,
//     in chunks of 48 runes joined by rotating separators ("", " ", "-",
//     "_", NBSP, two spaces), some chunks with leading U+3000 or trailing
//     NBSP (the Rust test regenerates these strings, see sweepChunks);
//   - 4,000 seeded random strings over ASCII, Latin-1 and Latin Extended
//     letters, combining marks, Greek, Cyrillic, Thai, CJK, Hangul, digits
//     of other scripts, number forms, compatibility characters, Unicode
//     spaces and invalid UTF-8 bytes (stored in the fixture).
package main

import (
	"flag"
	"math/rand"
	"path/filepath"
	"strings"
	"unicode/utf8"

	"github.com/neohugo/neohugo/markup/converter"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-markup/mdoracle"
)

var idTypes = []string{"github", "github-ascii", "blackfriday"}

var seps = []string{"", " ", "-", "_", "\u00a0", "  "}

// sweepChunks is mirrored by the Rust test.
func sweepChunks() []string {
	var runes []rune
	for r := rune(0); r < 0x3400; r++ {
		if utf8.ValidRune(r) {
			runes = append(runes, r)
		}
	}
	for r := rune(0x3400); r <= 0x10FFFF; r += 13 {
		if utf8.ValidRune(r) {
			runes = append(runes, r)
		}
	}
	var out []string
	for i := 0; i*48 < len(runes); i++ {
		end := (i + 1) * 48
		if end > len(runes) {
			end = len(runes)
		}
		var b strings.Builder
		if i%5 == 0 {
			b.WriteString("\u3000 ")
		}
		for j, r := range runes[i*48 : end] {
			if j > 0 {
				b.WriteString(seps[(i+j)%len(seps)])
			}
			b.WriteRune(r)
		}
		if i%7 == 0 {
			b.WriteString(" \u00a0")
		}
		out = append(out, b.String())
	}
	return out
}

var pools = [][]rune{
	[]rune("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"),
	[]rune(" -_.,;:!?'\"()[]{}<>/\\|@#$%^&*+=~`"),
	[]rune("ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖØÙÚÛÜÝÞßàáâãäåæçèéêëìíîïðñòóôõöøùúûüýþÿĀāĂăĄąĆćČčĎďĒēĘęĚěĞğĲĳŁłŃńŇňŒœŘřŚśŠšŤťŮůŸŹźŻżŽžƒǅǈǋǲȘșȚț"),
	[]rune("\u0300\u0301\u0302\u0303\u0308\u030a\u0327\u0328\u0338\u034f\u0345\u20dd\u0e31\u0e34\u0e35\u0e47\u0e48\u0e4d\u093f\u0bbe"),
	[]rune("αβγδεζηθικλμνξοπρστυφχψωΑΒΓΔΩάέήίόύώϊϋΐΰАБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯабвгдеёжзийклмнопрстуфхцчшщъыьэюяїєґ"),
	[]rune("กขคงจฉชซญดตถทธนบปผพฟภมยรลวศษสหอฮะาำเแโใไๅ๐๑๒๓๔๕๖๗๘๙"),
	[]rune("日本語中文漢字見出しの韓國한국어가각간갈감"),
	[]rune("٠١٢٣٤٥٦٧٨٩०१२३४५६७८९½¼¾²³¹ⅠⅡⅢⅫⅰⅱ①②⑩ﬁﬂﬀﬃ™℃℉KÅΩ№ǅǄǆİıſẞ"),
	[]rune("\u00a0\u1680\u2000\u2001\u2002\u2003\u2009\u200a\u2028\u2029\u202f\u205f\u3000\u0085\t\n\v\f\r"),
	[]rune("🍫😀👍🏽\u200d\ufe0f\u200b\u00ad\ufffd"),
}

var invalid = []string{"\xff", "\xfe", "\xc3", "\xe0\xb8", "\xf0\x9f", "\xed\xa0\x80", "\xc0\xaf", "\x80"}

func randomStrings(n int) []string {
	rng := rand.New(rand.NewSource(20260928))
	out := make([]string, 0, n)
	for i := 0; i < n; i++ {
		l := 1 + rng.Intn(24)
		var b strings.Builder
		for j := 0; j < l; j++ {
			if rng.Intn(30) == 0 {
				b.WriteString(invalid[rng.Intn(len(invalid))])
				continue
			}
			p := pools[rng.Intn(len(pools))]
			b.WriteRune(p[rng.Intn(len(p))])
		}
		out = append(out, b.String())
	}
	return out
}

type fixture struct {
	IDTypes []string       `json:"id_types"`
	Sweep   [][]mdoracle.B `json:"sweep"`
	Inputs  []mdoracle.B   `json:"inputs"`
	Random  [][]mdoracle.B `json:"random"`
}

func main() {
	out := flag.String("out", "crates/nh-markup/tests/fixtures/autoid", "output directory")
	flag.Parse()

	var fx fixture
	fx.IDTypes = idTypes
	sweep := sweepChunks()
	random := randomStrings(4000)
	for _, s := range random {
		fx.Inputs = append(fx.Inputs, mdoracle.B(s))
	}
	for _, t := range idTypes {
		toml := "[markup.goldmark.parser]\nautoIDType = \"" + t + "\"\n"
		p, _ := mdoracle.NewProvider(mdoracle.Config{Name: t, TOML: toml})
		conv, err := p.New(converter.DocumentContext{})
		if err != nil {
			panic(err)
		}
		san := conv.(converter.AnchorNameSanitizer)
		var sw, rnd []mdoracle.B
		for _, s := range sweep {
			sw = append(sw, mdoracle.B(san.SanitizeAnchorName(s)))
		}
		for _, s := range random {
			rnd = append(rnd, mdoracle.B(san.SanitizeAnchorName(s)))
		}
		fx.Sweep = append(fx.Sweep, sw)
		fx.Random = append(fx.Random, rnd)
	}
	mdoracle.WriteGz(filepath.Join(*out, "autoid.json.gz"), fx)
}
