package main

import (
	"math"
	"strings"
)

// valueCorpus is the list of operands of the verb × flag × width ×
// precision matrix.
func valueCorpus() []string {
	var vs []string
	add := func(s ...string) { vs = append(vs, s...) }

	add("nil", "bool:1", "bool:0")

	// Integers of every kind.
	for _, i := range []int64{0, 1, -1, 7, -7, 42, -42, 255, 256, 1000, -1000, 0x263a, 0x10ffff, 0x110000, -0x80, 1e6, 1<<31 - 1, -1 << 31, 1<<53 + 1, math.MaxInt64, math.MinInt64} {
		add(sInt("int", i))
	}
	for _, i := range []int64{0, 1, -1, 97, 127, -128} {
		add(sInt("int8", i))
	}
	for _, i := range []int64{0, -300, 32767, -32768, 0x6f22} {
		add(sInt("int16", i))
	}
	for _, i := range []int64{0, 'x', '日', -5, 0xd800, 0x1f600, math.MaxInt32, math.MinInt32} {
		add(sInt("int32", i))
	}
	for _, i := range []int64{0, 12345678901, -12345678901, math.MaxInt64, math.MinInt64} {
		add(sInt("int64", i))
	}
	for _, u := range []uint64{0, 1, 42, 0x263a, 1 << 63, math.MaxUint64} {
		add(sUint("uint", u))
	}
	for _, u := range []uint64{0, 9, 10, 65, 128, 255} {
		add(sUint("uint8", u))
	}
	for _, u := range []uint64{0, 0xfeff, 65535} {
		add(sUint("uint16", u))
	}
	for _, u := range []uint64{0, 0x10ffff, 0x110000, math.MaxUint32} {
		add(sUint("uint32", u))
	}
	for _, u := range []uint64{0, 1e19, math.MaxUint64, 0xdeadbeef} {
		add(sUint("uint64", u))
	}
	add(sUint("uintptr", 0), sUint("uintptr", 0xc000123456))

	// Floats.
	for _, f := range []float64{0, math.Copysign(0, -1), 1, -1, 0.5, 2.5, -2.5, 1.0 / 3, math.Pi, 1e6, 123456, 1234567,
		1e20, 1e21, 1e23, 1e-4, 1e-5, 1e-7, 2.675, 100, 1e100, -1.5e-300, math.MaxFloat64, math.SmallestNonzeroFloat64,
		0x1p-1022, 9007199254740993, 0.1, 0.000123456789, 999999.5, 9.5, 0.05,
		math.NaN(), math.Inf(1), math.Inf(-1)} {
		add(sF64(f))
	}
	for _, f := range []float32{0, 1, -1, 1.1, 0.1, 3.4028235e38, 1e-45, 16777216, 16777217, 123.456, -0.0625, float32(math.Inf(1)), float32(math.NaN())} {
		add(sF32(f))
	}

	// Strings: ASCII, Unicode, invalid UTF-8, quoting-sensitive bytes.
	for _, s := range []string{"", "a", "abc", "hello, world", "日本語", "☺", "⌘x", "\x00", "\xff\xfe", "abc\xffdef",
		"\xed\xa0\x80", "\U0010ffff", "\U0010fffe", "a\tb\nc\r", "\a\b\f\v", "\"quoted\"", "back`tick", `back\slash`,
		"\u2028\u00a0\u200b", "é", "🙂 emoji", "\x7f", "tab\tonly", strings.Repeat("xy", 30), "Ä", "%d%s"} {
		add(sStr(s))
	}
	for _, k := range []string{"html", "htmlattr", "css", "js", "jsstr", "url", "srcset"} {
		add(sSafe(k, "<b>x & y</b>"))
	}
	add(sSafe("html", ""), sSafe("html", "日本\xff"))

	// Byte slices.
	add(sBytes(""), sBytes("abc"), sBytes("\xff\x00\x10"), sBytes("日本"), sBytes("a\"b"), sTnil("[]uint8"))

	// Typed nils. (Not "main.NM"/"main.NS": the Rust side cannot tell an
	// unknown named slice/map type from an interface type; not
	// "*time.Location": fmt calls its String method on the nil receiver.)
	for _, t := range []string{"[]string", "[]interface {}", "[]int", "map[string]interface {}", "map[string]string",
		"maps.Params", "*int", "*main.Plain", "func()", "chan int", "main.Iface", "error", "fmt.Stringer"} {
		add(sTnil(t))
	}

	// Lists.
	add(sList("[]interface {}"),
		sList("[]interface {}", sInt("int", 1), sStr("two"), sF64(3.5), "bool:1", "nil"),
		sList("[]interface {}", "nil", "nil"),
		sList("[]interface {}", sStr("a"), sBytes("b"), sTnil("[]string"), sTnil("*int")),
		sList("[]interface {}", sList("[]interface {}", sInt("int", 1), sList("[]string", sStr("x"))), sMap("map[string]interface {}", "k", sStr("v"))),
		sList("[]string"),
		sList("[]string", sStr("a"), sStr("b c"), sStr(""), sStr("日本"), sStr("\xff")),
		sList("[]int", sInt("int", 1), sInt("int", -2), sInt("int", 300)),
		sList("[]int64", sInt("int64", 42)),
		sList("[]int8", sInt("int8", -1), sInt("int8", 65)),
		sList("[]int32", sInt("int32", 'a'), sInt("int32", '☺')),
		sList("[]uint16", sUint("uint16", 1), sUint("uint16", 0x263a)),
		sList("[]float64", sF64(1), sF64(2.5), sF64(1e21), sF64(math.NaN())),
		sList("[]float32", sF32(1.1), sF32(-0.5)),
		sList("[]bool", "bool:1", "bool:0"),
		sList("[]uint8", sUint("uint8", 104), sUint("uint8", 105)),
		sList("[]map[string]interface {}", sMap("map[string]interface {}", "a", sInt("int", 1)), "nil"),
		sList("[][]string", sList("[]string", sStr("x"), sStr("y")), sTnil("[]string")),
		sList("[][]uint8", sBytes("hi"), sBytes("")),
		sList("[]main.Iface", "nil"),
		sList("[]*main.Plain", "nil"),
		sList("[]template.HTML", sSafe("html", "<p>")),
		sList("[]error", sObj("obj_err", "boom"), "nil"),
		sList("[]fmt.Stringer", sObj("obj_str", "st"), sObj("obj_sv", "v")),
		sList("main.NS", sInt("int", 1), sStr("s"), "nil"),
		sObjNS(sInt("int", 1), sStr("s"), "nil"),
		sObjNS(),
	)

	// Maps.
	add(sMap("map[string]interface {}"),
		sMap("map[string]interface {}", "b", sInt("int", 2), "a", sInt("int", 1), "c", sList("[]int", sInt("int", 1), sInt("int", 2))),
		sMap("map[string]interface {}", "a", "nil", "日", sStr("本"), "", sF64(0.5), "Z", sBytes("zz")),
		sMap("map[string]string", "x", sStr("1"), "y", sStr("two words")),
		sMap("map[string]int", "one", sInt("int", 1), "minus", sInt("int", -1)),
		sMap("map[string]uint8", "k", sUint("uint8", 200)),
		sMap("map[string][]string", "tags", sList("[]string", sStr("a"), sStr("b")), "none", sTnil("[]string")),
		sMap("map[string]main.Iface", "n", "nil"),
		sMap("map[string]*main.Plain", "n", "nil"),
		sMap("maps.Params", "title", sStr("Hi"), "draft", "bool:0", "weight", sInt("int", 10)),
		sMap("main.NM", "k", sStr("v"), "e", sObj("obj_err", "bad")),
		sObjNM("k", sStr("v"), "e", sObj("obj_err", "bad"), "n", "nil"),
		sObjNM(),
	)

	// Times (no monotonic reading).
	add("time:-62135596800;0;nil",
		"time:0;0;utc",
		"time:1790510400;123456789;nil",
		"time:1790510400;120000000;utc",
		"time:1790510400;0;fixed=MST=-25200",
		"time:1790510400;5;fixed==5400", // (an unnamed whole-hour FixedZone is a shared cached pointer)
		"time:1790510400;999999999;zone=Asia/Bangkok",
		"time:1700000000;0;zone=America/New_York",
		"time:-1000000000000;0;nil",
		sList("[]interface {}", "time:0;0;utc", "time:1790510400;0;fixed=CET=3600"),
	)

	// Host objects.
	add(sObj("obj_str", "stringer"), sObj("obj_str", ""), sObj("obj_str", "日本\xff"),
		sObj("obj_err", "an error"), sObj("obj_both", "x"), sObj("obj_sv", "val"),
		"obj_plain("+sInt("int", 1)+","+sStr("two")+")",
		"obj_plain(nil,"+sList("[]string", sStr("a"))+")",
		"obj_plain("+sObj("obj_sv", "in")+","+sObj("obj_err", "e")+")",
		"obj_plain("+sBytes("ab")+","+sMap("map[string]interface {}", "k", sUint("uint", 3))+")",
		"obj_plain(obj_plain("+sF64(1.5)+",nil),"+sTnil("*main.Plain")+")",
		"obj_pplain("+sInt("int", 1)+","+sStr("two")+")",
		"obj_pplain("+sObj("obj_str", "s")+","+sObj("obj_both", "b")+")",
		"obj_pplain(nil,"+sList("[]interface {}", sInt("int8", 5))+")",
		sList("[]interface {}", sObj("obj_str", "a"), sObj("obj_err", "b"), sObj("obj_both", "c"), sObj("obj_sv", "d"), "obj_plain(nil,nil)"),
		sMap("map[string]interface {}", "s", sObj("obj_str", "a"), "p", "obj_plain("+sInt("int", 1)+",nil)"),
		sList("[]interface {}", "obj_pplain(nil,nil)"),
	)
	return vs
}

func sObj(kind, s string) string { return kind + ":" + hexs(s) }

// formatMatrix is the list of single-operand formats.
func formatMatrix() []string {
	verbs := []string{"v", "d", "s", "q", "x", "X", "t", "b", "o", "O", "c", "U", "e", "E", "f", "F", "g", "G", "T", "p", "%", "w", "z", "!", "é"}
	flagChars := "+-# 0"
	widths := []string{"", "1", "5", "13"}
	precs := []string{"", ".", ".0", ".2", ".5", ".11"}
	var fs []string
	for _, verb := range verbs {
		for mask := 0; mask < 32; mask++ {
			var flags strings.Builder
			for i := 0; i < 5; i++ {
				if mask&(1<<i) != 0 {
					flags.WriteByte(flagChars[i])
				}
			}
			for _, w := range widths {
				for _, p := range precs {
					fs = append(fs, "%"+flags.String()+w+p+verb)
				}
			}
		}
	}
	fs = append(fs,
		"", "x", "%", "%-", "%5", "%.", "%[1]v", "%[2]v", "%[0]v", "%[1]*v", "%v %v", "a%vb%%c", "%!v",
		"%-+# 012.5v", "%10.3q", "%#-10.4x", "% 08.3f", "%+.3e", "%#g", "%#.0f", "%#.0e", "%#x %X",
		"%[1]v|%[1]q|%[1]x", "%.*v", "%*v", "%.[1]v", "%[1].2v", "%3.[1]v", "%1000001v", "%.1000001v",
		"%v%", "%\xffv", "%é", "%#08.3U", "%+10.4q",
	)
	return fs
}
