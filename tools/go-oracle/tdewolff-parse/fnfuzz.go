package main

import (
	"math"
	"math/rand"
	"os"
	"sort"
	stdstrconv "strconv"
	"strings"

	mhtml "github.com/tdewolff/minify/v2/html"
	"github.com/tdewolff/parse/v2"
	"github.com/tdewolff/parse/v2/css"
	"github.com/tdewolff/parse/v2/html"
	"github.com/tdewolff/parse/v2/xml"
)

// Randomized differential sets for the function-level API (kept outside the
// repository):
//
//	tdewolff-parse fnfuzz DIR N SEED
//
// writes DIR/{entities,root,strconv,misc}.txt in the record formats of the
// checked-in fixtures, so the Rust tests run on them unchanged:
//
//	TDEWOLFF_PARSE_FIXTURES=DIR cargo test --release --test root --test strconv --test misc

func pick(r *rand.Rand, ss []string) string { return ss[r.Intn(len(ss))] }

func htmlEntityNames() []string {
	var names []string
	for k := range mhtml.EntitiesMap {
		names = append(names, k)
	}
	sort.Strings(names)
	return names
}

func randDigits(r *rand.Rand, n int, alphabet string) string {
	b := make([]byte, n)
	for i := range b {
		b[i] = alphabet[r.Intn(len(alphabet))]
	}
	return string(b)
}

func rootInput(r *rand.Rand, names []string) []byte {
	var sb strings.Builder
	switch r.Intn(9) {
	case 0, 1: // entity soup
		k := 1 + r.Intn(10)
		for i := 0; i < k; i++ {
			switch r.Intn(14) {
			case 0:
				sb.WriteString("&")
			case 1:
				sb.WriteString("&#" + randDigits(r, r.Intn(8), "0123456789") + pick(r, []string{";", "", ";", "x"}))
			case 2:
				sb.WriteString("&#" + pick(r, []string{"x", "X"}) + randDigits(r, r.Intn(20), "0123456789abcdefABCDEF") + pick(r, []string{";", "", ";", "g"}))
			case 3:
				name := pick(r, names)
				if r.Intn(4) == 0 {
					name = strings.ToUpper(name)
				}
				if r.Intn(5) == 0 && len(name) > 1 {
					name = name[:r.Intn(len(name))]
				}
				sb.WriteString("&" + name + pick(r, []string{";", ";", ";", "", " ", "x;"}))
			case 4:
				sb.WriteString(pick(r, []string{"&amp;", "&lt;", "&gt;", "&quot;", "&apos;", "&#39;", "&#34;", "&#x27;", "&#x22;", "&#60;", "&#38;", "&#x26;", "&LT;", "&AMP;", "&nbsp;"}))
			case 5:
				sb.WriteString(pick(r, []string{" ", "\t", "\n", "\r", "\f", "  ", " \n ", "\r\n", "\t\t"}))
			case 6:
				sb.WriteString(pick(r, []string{";", "#", "x", "a", "1", "é", "\x00", "<", "'", "\""}))
			case 7:
				sb.WriteString("&" + randDigits(r, 1+r.Intn(40), "abcdefghijklmnopqrstuvwxyzABCDEF0123456789") + ";")
			case 8:
				sb.WriteString("&amp;" + pick(r, []string{"amp;", "#39;", "lt;", "x", "1", "#", " "}))
			default:
				sb.WriteString(randDigits(r, r.Intn(5), "abc xyz"))
			}
		}
	case 2: // data URIs
		sb.WriteString(pick(r, []string{"data:", "data:", "data:", "DATA:", "data", "dat"}))
		k := r.Intn(5)
		for i := 0; i < k; i++ {
			sb.WriteString(pick(r, []string{"text/plain", "image/svg+xml", "image/png", ";charset=utf-8", ";base64", " base64 ", "base64", ";", "=", " ", "a=b", ";charset=US-ASCII", "application/json", ",", "\t"}))
		}
		sb.WriteString(pick(r, []string{",", ",", ",", "", ";base64,", " ;base64 ,"}))
		if r.Intn(2) == 0 {
			sb.WriteString(randDigits(r, r.Intn(40), "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"))
			sb.WriteString(pick(r, []string{"", "=", "==", "===", "\r\n", "\n", "=a", " ", "*", "=\r\n="}))
			sb.WriteString(randDigits(r, r.Intn(6), "ABCab01+/=\r\n"))
		} else {
			sb.WriteString(randDigits(r, r.Intn(30), "%%%0123456789abcdefABCDEFgz+ <>#&\"'"))
		}
	case 3: // mediatypes
		k := r.Intn(8)
		for i := 0; i < k; i++ {
			sb.WriteString(pick(r, []string{" ", " ", ";", "=", "text/html", "a", "charset", "utf-8", "b=c", "x", "  ;", "; ", "=;", "text/css;inline=1"}))
		}
	case 4: // URL encoded
		k := r.Intn(12)
		for i := 0; i < k; i++ {
			sb.WriteString(pick(r, []string{"%", "%2", "%20", "%zz", "%4a", "%4A", "%e2%80%a8", "+", "a", " ", "é", "#", "%%", "%0", "\x00", "\xff"}))
		}
	case 5: // numbers and dimensions
		sb.WriteString(pick(r, []string{"", "+", "-", ".", "+.", "-."}))
		sb.WriteString(randDigits(r, r.Intn(6), "0123456789"))
		if r.Intn(2) == 0 {
			sb.WriteString(".")
			sb.WriteString(randDigits(r, r.Intn(4), "0123456789"))
		}
		if r.Intn(2) == 0 {
			sb.WriteString(pick(r, []string{"e", "E", "e+", "e-", "E-"}))
			sb.WriteString(randDigits(r, r.Intn(3), "0123456789"))
		}
		sb.WriteString(pick(r, []string{"", "%", "px", "em", "e", "E", "x", "Px", ".", "-", "5"}))
	case 6: // whitespace soup
		k := r.Intn(12)
		for i := 0; i < k; i++ {
			sb.WriteString(pick(r, []string{" ", "\t", "\n", "\r", "\f", "a", "&amp;", "&#10;", "&#32;", "b c", "\r\n", " ", "\v"}))
		}
	case 7: // quote entities
		sb.WriteString(pick(r, []string{"&#", "&#x", "&#X", "&q", "&a", "&"}))
		sb.WriteString(randDigits(r, r.Intn(4), "0"))
		sb.WriteString(pick(r, []string{"22;", "27;", "34;", "39;", "2", "3", "uot;", "pos;", "quot;", "apos;", "22", "39"}))
		sb.WriteString(randDigits(r, r.Intn(3), "; x"))
	default: // random bytes
		b := make([]byte, r.Intn(24))
		for i := range b {
			b[i] = byte(r.Intn(256))
		}
		return b
	}
	return []byte(sb.String())
}

func numInput(r *rand.Rand) []byte {
	var b []byte
	switch r.Intn(8) {
	case 0: // long digit strings with a dot and exponent
		b = append(b, randDigits(r, r.Intn(30), "0")...)
		b = append(b, randDigits(r, 1+r.Intn(40), "0123456789")...)
		if r.Intn(2) == 0 {
			p := r.Intn(len(b) + 1)
			b = append(b[:p], append([]byte{'.'}, b[p:]...)...)
		}
		if r.Intn(2) == 0 {
			b = append(b, "eE"[r.Intn(2)])
			if r.Intn(2) == 0 {
				b = append(b, "+-"[r.Intn(2)])
			}
			b = stdstrconv.AppendInt(b, int64(r.Intn(800)), 10)
		}
	case 1: // extreme exponents
		b = append(b, randDigits(r, 1+r.Intn(20), "0123456789")...)
		b = append(b, 'e')
		b = append(b, pick(r, []string{"", "-", "+"})...)
		b = append(b, pick(r, []string{"9223372036854775807", "9223372036854775808", "18446744073709551616", "308", "309", "324", "325", "22", "23", "37", "38", "15", "16", "1023", "1024", "1022", "0"})...)
	case 2: // formatted floats
		f := math.Float64frombits(r.Uint64())
		if r.Intn(2) == 0 {
			f = r.NormFloat64() * math.Pow10(r.Intn(60)-30)
		}
		b = stdstrconv.AppendFloat(nil, f, "efg"[r.Intn(3)], r.Intn(25)-1, 64)
	case 3: // digits around uint64 overflow
		b = append(b, pick(r, []string{"1844674407370955161", "18446744073709551615", "9223372036854775807", "922337203685477580", "99999999999999999999"})...)
		b = append(b, randDigits(r, r.Intn(4), "0123456789.")...)
	case 4: // grouped numbers
		b = append(b, pick(r, []string{"", "-", "+"})...)
		k := 1 + r.Intn(6)
		for i := 0; i < k; i++ {
			b = append(b, randDigits(r, 1+r.Intn(4), "0123456789")...)
			b = append(b, pick(r, []string{",", ".", "", ",", "·", "\xff"})...)
		}
	case 5: // leading zeros and dots (ParseDecimal)
		b = append(b, pick(r, []string{"", "-"})...)
		b = append(b, randDigits(r, r.Intn(400), "0")...)
		b = append(b, pick(r, []string{".", "", "."})...)
		b = append(b, randDigits(r, r.Intn(350), "0")...)
		b = append(b, randDigits(r, r.Intn(25), "0123456789")...)
		if r.Intn(3) == 0 {
			b = append(b, randDigits(r, r.Intn(350), "0")...)
		}
	case 6: // random alphabet
		b = []byte(randDigits(r, r.Intn(16), "0123456789.eE+-,x "))
	default: // integers
		b = stdstrconv.AppendInt(nil, r.Int63()>>uint(r.Intn(63))-r.Int63()>>uint(r.Intn(63)), 10)
	}
	if r.Intn(6) == 0 {
		b = append([]byte{"+-"[r.Intn(2)]}, b...)
	}
	return b
}

func floatInput(r *rand.Rand) float64 {
	switch r.Intn(8) {
	case 0:
		return math.Float64frombits(r.Uint64())
	case 1:
		return r.NormFloat64() * math.Pow10(r.Intn(60)-30)
	case 2: // near powers of ten
		f := math.Pow10(r.Intn(640) - 320)
		for k := r.Intn(4); k > 0; k-- {
			if r.Intn(2) == 0 {
				f = math.Nextafter(f, math.Inf(1))
			} else {
				f = math.Nextafter(f, 0)
			}
		}
		if r.Intn(2) == 0 {
			f = -f
		}
		return f
	case 3: // halves (rounding)
		return (float64(r.Intn(2000000)) + 0.5) / math.Pow10(r.Intn(20))
	case 4: // subnormals
		return math.Float64frombits(r.Uint64() & 0x800FFFFFFFFFFFFF)
	case 5: // short decimals
		return float64(r.Int63n(1000000)-500000) / math.Pow10(r.Intn(10))
	case 6: // large integers
		return float64(r.Int63()) * math.Pow10(r.Intn(10))
	}
	return r.ExpFloat64() * math.Pow10(r.Intn(40)-20)
}

func intInput(r *rand.Rand) int64 {
	switch r.Intn(4) {
	case 0:
		p := int64(1)
		for k := r.Intn(19); k > 0; k-- {
			p *= 10
		}
		v := p + int64(r.Intn(3)-1)
		if r.Intn(2) == 0 {
			v = -v
		}
		return v
	case 1:
		return int64(r.Uint64())
	}
	v := r.Int63() >> uint(r.Intn(63))
	if r.Intn(2) == 0 {
		v = -v
	}
	return v
}

func cssIdentInput(r *rand.Rand) []byte {
	var sb strings.Builder
	k := r.Intn(8)
	for i := 0; i < k; i++ {
		sb.WriteString(pick(r, []string{"-", "--", "a", "Z", "_", "0", "\\", "\\31 ", "\\\n", "\\\r\n", "\\é", "é", "\\fffff0", "\\1234567", " ", "(", ")", "'", "\"", "\x00", "\x7f", "\x1f", "\xc3", "\\\x00", ".", "%", "url(", "\\)"}))
	}
	return []byte(sb.String())
}

func attrInput(r *rand.Rand) []byte {
	var sb strings.Builder
	k := r.Intn(10)
	for i := 0; i < k; i++ {
		sb.WriteString(pick(r, []string{"a", "\"", "'", " ", "=", "<", ">", "`", "\t", "\n", "\f", "\r", "&", "&amp;", "é", "b c", "<!", "]]>"}))
	}
	return []byte(sb.String())
}

func fnFuzz(dir string, n int, seed int64) {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		panic(err)
	}
	genEntities(dir)
	names := htmlEntityNames()
	r := rand.New(rand.NewSource(seed))

	fw := newFixture(dir, "root.txt")
	for i := 0; i < n; i++ {
		rootRecords(fw, rootInput(r, names), i%4 == 0)
	}
	fw.close()

	fw = newFixture(dir, "strconv.txt")
	for i := 0; i < 4*n; i++ {
		numberRecords(fw, numInput(r), true)
	}
	for i := 0; i < n; i++ {
		floatRecords(fw, floatInput(r))
	}
	for i := 0; i < n/2; i++ {
		intRecords(fw, intInput(r))
	}
	for i := 0; i < n; i++ {
		var h, s, l float64
		switch r.Intn(3) {
		case 0:
			h, s, l = r.Float64()*1.4-0.2, r.Float64(), r.Float64()
		case 1:
			h, s, l = float64(r.Intn(3600))/3600, float64(r.Intn(101))/100, float64(r.Intn(101))/100
		default:
			h, s, l = float64(r.Intn(361))/360, float64(r.Intn(1001))/1000, float64(r.Intn(1001))/1000
		}
		rr, g, b := css.HSL2RGB(h, s, l)
		fw.line("hsl", fbits(h), fbits(s), fbits(l), fbits(rr), fbits(g), fbits(b))
	}
	fw.close()

	fw = newFixture(dir, "misc.txt")
	var buf, xbuf []byte
	for i := 0; i < n; i++ {
		v := attrInput(r)
		q := []byte{0, '"', '\''}[r.Intn(3)]
		must := r.Intn(2) == 0
		fw.line("hescape", hs(string(v)), itoa(int(q)), bstr(must), hx(html.EscapeAttrVal(&buf, v, q, must)))
		fw.line("xescapeattr", hs(string(v)), hx(xml.EscapeAttrVal(&xbuf, v)))
		data, ok := xml.EscapeCDATAVal(&xbuf, v)
		fw.line("xescapecdata", hs(string(v)), hx(data), bstr(ok))
		id := cssIdentInput(r)
		fw.line("isident", hs(string(id)), bstr(css.IsIdent(append([]byte(nil), id...))))
		fw.line("isurlunquoted", hs(string(id)), bstr(css.IsURLUnquoted(append([]byte(nil), id...))))
		nm := []byte(randDigits(r, r.Intn(11), "abcdefghijklmnopqrstuvwxyz-ACS"))
		fw.line("hhash", hs(string(nm)), u64s(uint64(html.ToHash(nm))))
		fw.line("chash", hs(string(nm)), u64s(uint64(css.ToHash(nm))))
	}
	fw.close()
}

// corpusNumbers writes DIR/strconv.txt with the Parse* records for every
// distinct number-like substring (as delimited by parse.Number, plus a
// following unit or exponent-looking tail) found in the corpus files:
//
//	tdewolff-parse corpusnums DIR ROOT...
func corpusNumbers(dir string, roots []string) {
	if err := os.MkdirAll(dir, 0o755); err != nil {
		panic(err)
	}
	seen := map[string]bool{}
	var nums []string
	for _, root := range roots {
		for _, p := range listFiles(root, []string{".html", ".css", ".scss", ".svg", ".xml", ".json", ".in", ".out"}) {
			b, err := os.ReadFile(p)
			if err != nil {
				panic(err)
			}
			for i := 0; i < len(b); i++ {
				c := b[i]
				if !('0' <= c && c <= '9' || c == '.' || c == '-' || c == '+') {
					continue
				}
				if i > 0 && ('0' <= b[i-1] && b[i-1] <= '9' || b[i-1] == '.') {
					continue
				}
				n := parse.Number(b[i:])
				if n == 0 {
					continue
				}
				end := min(len(b), i+n+4)
				for _, s := range []string{string(b[i : i+n]), string(b[i:end])} {
					if !seen[s] {
						seen[s] = true
						nums = append(nums, s)
					}
				}
			}
		}
	}
	sort.Strings(nums)
	fw := newFixture(dir, "strconv.txt")
	for _, s := range nums {
		numberRecords(fw, []byte(s), true)
	}
	fw.close()
}
