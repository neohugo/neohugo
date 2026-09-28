// Package corpus collects the strings the nh-common flect and prose oracles
// run through the Go code, and writes the JSON fixtures.
//
// The seeksnack site (the private golden site) is not available, so the
// corpus is this repository's own content: every front matter title,
// linkTitle, description and taxonomy term, every Markdown heading, and every
// file and directory name under docs/content, hugolib/testsite/content and
// create/skeletons, plus the section names and titles quoted in the specs
// (docs/rust-port/specs/content-model.md §5.3, §6) and adversarial strings
// (case, separators, acronyms, apostrophes, digits, Thai, accented Latin, CJK,
// emoji, typographic quotes, invalid UTF-8, empty strings).
package corpus

import (
	"bufio"
	"bytes"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"unicode/utf8"
)

// Roots are the content trees walked for strings, relative to the repository root.
var Roots = []string{"docs/content", "hugolib/testsite/content", "create/skeletons"}

// MaxLen drops longer strings (long descriptions add bytes, not coverage).
const MaxLen = 100

var (
	fmKeyRe  = regexp.MustCompile(`^\s*(title|linkTitle|linktitle|description|categories|tags|keywords|series|authors|slug|type|section)\s*[:=]\s*(.*)$`)
	listItem = regexp.MustCompile(`^\s*-\s+(.+)$`)
	heading  = regexp.MustCompile(`^#{1,6}\s+(.+?)\s*#*\s*$`)
)

// Strings returns the sorted, de-duplicated corpus.
func Strings(root string) ([]string, error) {
	set := map[string]bool{}
	add := func(s string) {
		if len(s) <= MaxLen {
			set[s] = true
		}
	}
	for _, r := range Roots {
		dir := filepath.Join(root, r)
		err := filepath.WalkDir(dir, func(path string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			name := d.Name()
			if d.IsDir() {
				add(name)
				return nil
			}
			ext := filepath.Ext(name)
			add(strings.TrimSuffix(name, ext))
			switch ext {
			case ".md", ".markdown", ".html":
			default:
				return nil
			}
			b, err := os.ReadFile(path)
			if err != nil {
				return err
			}
			for _, s := range fromContent(b) {
				add(s)
			}
			return nil
		})
		if err != nil {
			return nil, err
		}
	}
	for _, s := range Adversarial {
		set[s] = true
	}
	out := make([]string, 0, len(set))
	for s := range set {
		out = append(out, s)
	}
	sort.Strings(out)
	return out, nil
}

// fromContent extracts front matter values and headings.
func fromContent(b []byte) []string {
	var out []string
	sc := bufio.NewScanner(bytes.NewReader(b))
	sc.Buffer(make([]byte, 1024*1024), 1024*1024)
	line := 0
	inFM := false
	fmEnd := ""
	inCode := false
	for sc.Scan() {
		l := sc.Text()
		line++
		if line == 1 && (l == "---" || l == "+++") {
			inFM = true
			fmEnd = l
			continue
		}
		if inFM {
			if l == fmEnd {
				inFM = false
				continue
			}
			if m := fmKeyRe.FindStringSubmatch(l); m != nil {
				out = append(out, values(m[2])...)
				continue
			}
			if m := listItem.FindStringSubmatch(l); m != nil {
				out = append(out, values(m[1])...)
			}
			continue
		}
		if strings.HasPrefix(l, "```") {
			inCode = !inCode
			continue
		}
		if inCode {
			continue
		}
		if m := heading.FindStringSubmatch(l); m != nil {
			out = append(out, m[1])
		}
	}
	return out
}

// values splits a front matter value: a quoted scalar, or a [a, b] list.
func values(v string) []string {
	v = strings.TrimSpace(v)
	if v == "" || v == "|" || v == ">" {
		return nil
	}
	if strings.HasPrefix(v, "[") && strings.HasSuffix(v, "]") {
		var out []string
		for _, p := range strings.Split(v[1:len(v)-1], ",") {
			out = append(out, unquote(strings.TrimSpace(p)))
		}
		return out
	}
	return []string{unquote(v)}
}

func unquote(s string) string {
	if len(s) >= 2 && (s[0] == '"' || s[0] == '\'') && s[len(s)-1] == s[0] {
		return s[1 : len(s)-1]
	}
	return s
}

// Adversarial are hand-picked edge cases.
var Adversarial = []string{
	"", " ", "  ", "\t", "\n", "-", "_", "/", ":", "--", "a", "A", "s", "S", "x", "1", "0", "-1",
	// section names and titles quoted in the specs (seeksnack)
	"biscuit", "biscuit-roll", "biscuit-rolls", "candy", "jelly", "pastry", "popcorn", "seafood",
	"crepe", "almonds", "peas", "fries", "cookies", "crackers", "pretzels", "corn-chips",
	"potato-chips", "chocolate", "cake", "snack", "noodle", "wafer", "gummy", "nut", "seaweed",
	"brand", "brands", "category", "categories", "company", "companies", "country", "countries",
	"ingredient", "ingredients", "tag", "tags", "lay's", "Lay's", "INS 160a (I)",
	"Pepsi-Cola (Thai) Trading Co.,Ltd.", "Union Snack Ltd.", "api key", "a+b",
	"ญี่ปุ่น", "เกาหลี", "เลย์ สแตคส์ ", "ขนม", "มันฝรั่ง", "ไทย", "ประเทศไทย", "ั", "่",
	// flect / prose upstream test inputs and variations
	"employee_salary", "employee_id", "employee_mobile_number", "first_Name", "firstName",
	"sentence case", "Sentence Case", "id", "ID", "IDs", "ids", "html", "HTML5", "JSON", "jsonAPI",
	"widget_id", "WidgetID", "UserID", "user_id", "userId", "HTTPRequest", "HTTPSServer",
	"Nice to see you!", "i've read a book! have you?", "This is `code` ok", "*wonderful* world",
	"bob dylan", "user", "users", "person", "people", "datum", "data", "media", "medium",
	"octopus", "cactus", "fish", "sheep", "child", "ox", "oxen", "axis", "axes", "die", "dice",
	"matrix", "index", "vertex", "alias", "aliases", "status", "bus", "quiz", "box", "wife",
	"knife", "leaf", "half", "wolf", "hero", "potato", "photo", "piano", "tomato", "lady",
	"day", "key", "boy", "guy", "soliloquy", "bureau", "equipment", "information", "rice",
	"news", "money", "jeans", "police", "series", "species", "deer", "moose", "you",
	"ProductCategory", "product_category", "Product Category", "OKAY", "OK", "ok",
	"a tale of two cities", "the lord of the rings", "gone with the wind",
	"the quick brown fox jumps over the lazy dog", "via ferrata", "vs. the world",
	"state-of-the-art", "hello/world", "hello:world", "to be or not to be",
	"q&a", "Q&A", "rock 'n' roll", "don't stop", "it's", "O'Neil", "o'neil",
	"42", "1st", "2nd", "3rd", "11", "12", "13", "21", "22", "23", "101", "111", "112", "-11",
	"-21", "007", "+5", "1e3", "3.14", "9223372036854775807", "-9223372036854775808",
	"9223372036854775808", "1_000", "0x1f",
	// case, separators
	"UPPER CASE", "lower case", "Mixed Case", "camelCase", "PascalCase", "snake_case",
	"kebab-case", "dot.case", "path/case", "colon:case", "multiple   spaces", " leading",
	"trailing ", "__double__underscore__", "--double--dash--", "-leading-dash", "trailing-dash-",
	"a-b-c", "a_b_c", "a b c", "A-B-C", "ABC", "AbC", "aBC",
	// accented Latin, CJK, emoji, other scripts
	"café", "Café", "crème brûlée", "Crème Brûlée", "naïve", "façade", "über", "Über",
	"straße", "ß", "ǆ", "ǅ", "Ǆ", "ﬁ", "ΑΒΓ", "αβγ", "ωmega", "Ωmega", "кошка", "Кошка",
	"日本語", "中文 标题", "한국어", "東京タワー", "😀", "😀 smile", "smile 😀", "🍪 cookie",
	"👍🏽", "ﾃｽﾄ", "Ⅻ", "½", "²", "٣", "۳", "१२", "๑๒๓",
	// typographic punctuation (prose sanitizer)
	"“quoted” title", "‘single’ quotes", "en–dash", "em—dash", "wait…", "…and then",
	"“a” and “the”", "the “best” of the – rest", "one—two—three", "“Hello”, world",
	"a “b” c – d … e", "the end—of the line", "“the” “a” “an” “of”",
	// invalid UTF-8
	"\xff", "a\xffb", "\xc3", "caf\xc3", "\xe0\xb8", "ok\x80ok", "\xed\xa0\x80",
	// control characters
	"a\x00b", "tab\there", "line\nbreak", "\x7f",
}

// Encode returns s as a JSON-able value: the string itself when it is valid
// UTF-8, else {"hex": ...}.
func Encode(s string) any {
	if utf8.ValidString(s) {
		return s
	}
	return map[string]string{"hex": hex.EncodeToString([]byte(s))}
}

// Call runs f, turning a panic into {"panic": message}.
func Call(f func() string) (out any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]string{"panic": fmt.Sprint(r)}
		}
	}()
	return Encode(f())
}

// WriteCases writes {"cases": [...]} with one case per line.
func WriteCases(path string, header map[string]any, cases []map[string]any) error {
	var b bytes.Buffer
	b.WriteString("{")
	keys := make([]string, 0, len(header))
	for k := range header {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		kb, err := json.Marshal(k)
		if err != nil {
			return err
		}
		vb, err := json.Marshal(header[k])
		if err != nil {
			return err
		}
		b.Write(kb)
		b.WriteString(":")
		b.Write(vb)
		b.WriteString(",\n")
	}
	b.WriteString(`"cases":[` + "\n")
	for i, c := range cases {
		cb, err := json.Marshal(c)
		if err != nil {
			return err
		}
		b.Write(cb)
		if i < len(cases)-1 {
			b.WriteString(",")
		}
		b.WriteString("\n")
	}
	b.WriteString("]}\n")
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	return os.WriteFile(path, b.Bytes(), 0o644)
}
