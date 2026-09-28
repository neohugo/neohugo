package main

import (
	"encoding/json"
	"html/template"
	"math"
	"time"

	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/common/neohugo"
	"github.com/neohugo/neohugo/common/types/hstring"
)

var (
	t1   = time.Date(2021, 3, 4, 5, 6, 7, 8, time.UTC)
	t2   = time.Date(2021, 3, 4, 12, 6, 7, 8, time.FixedZone("UTC+7", 7*3600))
	t3   = time.Date(1999, 12, 31, 23, 59, 59, 0, time.UTC)
	t4   = time.Date(2024, 2, 29, 0, 0, 0, 0, time.FixedZone("-0530", -(5*3600+30*60)))
	zero = time.Time{}
)

// scalars is the value corpus (the data oracle's, without its test objects
// and pages) crossed with the unary functions.
func scalars() []any {
	return []any{
		nil, true, false,
		0, 1, -2, 42, int8(-8), int16(16), int32(-32), int64(64), int64(math.MaxInt64), int64(math.MinInt64),
		uint(1), uint8(8), uint16(16), uint32(32), uint64(64), uint64(math.MaxUint64), uintptr(7),
		float32(1.5), float32(0.1), 0.0, 1.0, 2.5, -3.75, 1e300, math.Inf(1), math.Inf(-1), math.Copysign(0, -1), math.NaN(),
		"", "a", "b", "abc", "ABC", "10", "2", "1e3", "-4.5", "0x1f", "Inf", "NaN", "1e999", "true", " 3", "ข้าว", "ขนม", "é", "a b&c<d>",
		"\xff\xfe", "a\x00b", "  padded\t\n", "Hello, World", "the quick brown fox", "ﬁ Straße İstanbul", "日本語のテキスト", "한국어 텍스트",
		"line1\nline2\r\n", "<p>para</p>", "<b>bold</b> and <i>it</i>", "tab\there",
		template.HTML("a"), template.HTML("<b>x</b>"), template.HTML("text\n"), template.CSS("a"), template.JS("a"), template.JSStr("a"), template.URL("a"), template.HTMLAttr("a"),
		hstring.HTML("a"), json.Number("12"), json.Number("1.5"), neohugo.VersionString("0.105.0"),
		t1, t2, zero,
		[]any{}, []any{1, "a", nil}, []any{"a", "b", "c"}, []string{}, []string{"a", "b", "c"}, []int{1, 2, 3},
		map[string]any{}, map[string]any{"a": 1, "b": "x"}, maps.Params{"a": 1}, map[string]string{"a": "b"},
		[]string(nil), map[string]any(nil), []any(nil),
	}
}

// small is a smaller corpus for the functions of two and more arguments.
func small() []any {
	return []any{
		nil, true, 0, 1, -2, int64(3), uint(2), 1.5, math.NaN(), "", "a", "2", "abc", "ab", "b", "ข้าว", "\xff",
		template.HTML("a"), hstring.HTML("a"), t1, []any{1, "a"}, []string{"a", "b"}, map[string]any{"a": 1},
	}
}

// texts are strings for the string functions (words, CJK, Thai, HTML, spaces).
func texts() []string {
	return []string{
		"",
		"a",
		"Hello World",
		"hello world",
		"  leading and trailing  ",
		"The quick brown fox jumps over the lazy dog.",
		"a tale of two cities: the best of times",
		"THE LORD OF THE RINGS",
		"o'neil and mcdonald's",
		"this is an ap-style title with a hyphen-ated word",
		"ข้าวผัดกุ้ง อร่อยมาก",
		"สวัสดีครับ ยินดีต้อนรับ",
		"日本語のテキストです。これは例です。",
		"中文 汉字 测试",
		"한국어 텍스트 입니다",
		"Mixed 日本語 and English words",
		"<p>Some <b>bold</b> text</p>\n<p>Second para</p>",
		"<div class=\"x\">Div &amp; entity</div>",
		"Hello<br>World<br />Again",
		"tabs\tand\nnewlines\r\nhere",
		"émile zola à paris",
		"\xff\xfeinvalid utf8",
		"a  b   c",
		"123 456",
		"one,two;three",
		"foo_bar-baz qux",
	}
}
