package main

// Differential corpus for libsasserrors.JsonToError (json.Unmarshal into
// libsasserrors.Error; go1.27.1 builds encoding/json on the jsonv2
// implementation with the v1 options). -json <path> writes records of
// (input, decoded fields, Error()) for LibSass-like error documents in many
// spellings, type mismatches, duplicate and case-folded keys, string escapes
// (including surrogates), invalid UTF-8, number edge cases, nesting-depth
// limits, and random mutations.

import (
	"fmt"
	"math/rand"
	"strings"

	"github.com/bep/golibsass/libsass/libsasserrors"
)

func writeJSONCases(path string, seed int64, n int) {
	var docs []string
	add := func(s string) { docs = append(docs, s) }

	// Fixed cases.
	add(`{"status": 1, "file": "stdin", "line": 3, "column": 14, "message": "Undefined variable: \"$blue\".", "formatted": "x"}`)
	add(``)
	add(`null`)
	add(`{}`)
	add(`[]`)
	add(`"str"`)
	add(`1`)
	add(`{"line": 3,}`)
	add(`{"line": 3} x`)
	add(`{"line": 3} `)
	add(" \t\r\n{\"line\":3}\n\r\t ")
	add("\ufeff{\"line\": 3}")
	add(`{"line": 1, "line": 2}`)
	add(`{"file": "a", "File": "b", "FILE": "c"}`)
	add(`{"FILE": "c", "file": "a"}`)
	add(`{"\u017ftatus": 5, "\u212aey": 1, "fi\u0131e": "x", "f\u0130le": "y", "fil\u00e9": "z"}`)
	add("{\"\u017ftatus\": 6, \"me\u017f\u017fage\": \"long s\"}")
	add(`{"fi_le": "x", "fi-le": "y", "Fi_Le": "z"}`)
	add(`{"file": "escaped key", "line": 9}`)
	add(`{"line": 1.5, "column": "x", "file": 7, "Message": "m", "STATUS": 2}`)
	add(`{"line": 1e2, "column": -0, "status": 1E0, "file": null, "message": true}`)
	add(`{"line": 9223372036854775807, "column": 9223372036854775808, "status": -9223372036854775808}`)
	add(`{"line": -9223372036854775809, "column": 99999999999999999999999999, "status": 00}`)
	add(`{"line": 01}`)
	add(`{"line": -}`)
	add(`{"line": 1.}`)
	add(`{"line": .5}`)
	add(`{"line": +1}`)
	add(`{"line": 1e}`)
	add(`{"line": 1e+}`)
	add(`{"line": 1E-5}`)
	add(`{"line": [1, 2], "column": {"a": 1}, "status": "3", "file": ["x"], "message": {"m": "n"}}`)
	add(`{"line": null, "file": null}`)
	add(`{"file": "a\"b\\c\/d\be\ff\ng\rh\ti\u0000j\u001fk\u007fl"}`)
	add(`{"file": "\ud83d\ude00 \ud800 \udc00 \ud800\u0041 \udc00\ud800 \ud800\ud800\udc00"}`)
	add(`{"file": "\ud800"}`)
	add(`{"file": "\ud800\"}`)
	add(`{"file": "\uD83D\uDE00\uFFFD\uFFFE\uffff"}`)
	add("{\"file\": \"\xff \xc3 \xe6\x97 \xed\xa0\x80 \xf4\x90\x80\x80 \xc0\xaf \xef\xbf\xbd\"}")
	add("{\"file\": \"a\x01b\"}")
	add("{\"file\": \"a\x1fb\"}")
	add("{\"file\": \"a\x7fb\"}")
	add("{\"file\": \"tab\tinside\"}")
	add(`{"file": "\x41"}`)
	add(`{"file": "\u12"}`)
	add(`{"file": "\uZZZZ"}`)
	add(`{"file": "abc`)
	add(`{"file" "abc"}`)
	add(`{file: "abc"}`)
	add(`{"file": 'abc'}`)
	add(`{"a": tru}`)
	add(`{"a": nul}`)
	add(`{"a": True}`)
	add(`{,}`)
	add(`{"a":1,,"b":2}`)
	add(`[1,]`)
	add(`{"file": "x"}{"line": 2}`)
	for _, d := range []int{9998, 9999, 10000, 10001} {
		add(`{"file": "x", "deep": ` + strings.Repeat("[", d) + strings.Repeat("]", d) + `}`)
		add(`{"file": "x", "deep": ` + strings.Repeat(`{"a":`, d) + `1` + strings.Repeat("}", d) + `}`)
		add(strings.Repeat("[", d+1) + strings.Repeat("]", d+1))
	}

	// Randomized cases.
	r := rand.New(rand.NewSource(seed))
	for i := 0; i < n; i++ {
		doc := randErrorDoc(r)
		switch k := r.Intn(10); {
		case k < 2:
			doc = mutateJSON(r, doc)
		}
		add(doc)
	}

	w := newRecordWriter(path)
	for _, d := range docs {
		e := libsasserrors.JsonToError(d)
		w.putS("json", d)
		w.putI("status", e.Status)
		w.putI("column", e.Column)
		w.putS("file", e.File)
		w.putI("line", e.Line)
		w.putS("message", e.Message)
		w.putS("error", e.Error())
	}
	w.close()
}

func randJSONString(r *rand.Rand) string {
	var sb strings.Builder
	sb.WriteByte('"')
	for i, n := 0, r.Intn(12); i < n; i++ {
		switch r.Intn(12) {
		case 0:
			sb.WriteString([]string{`\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`}[r.Intn(8)])
		case 1:
			fmt.Fprintf(&sb, `\u%04x`, r.Intn(0x10000))
		case 2:
			// Surrogate pairs, valid or not.
			hi := 0xD800 + r.Intn(0x800)
			lo := 0xD800 + r.Intn(0x800)
			if r.Intn(2) == 0 {
				fmt.Fprintf(&sb, `\u%04X\u%04x`, hi, lo)
			} else {
				fmt.Fprintf(&sb, `\u%04x`, hi)
			}
		case 3:
			sb.WriteByte(byte(0x80 + r.Intn(0x80))) // stray / invalid UTF-8
		case 4:
			if r.Intn(8) == 0 {
				sb.WriteByte(byte(r.Intn(0x20))) // raw control: invalid JSON
			} else {
				sb.WriteByte(0x7f)
			}
		case 5, 6:
			// An arbitrary valid rune (never a control); quotes and
			// backslashes are escaped so the document stays valid.
			c := randRune(r) | 0x20
			if c == '"' || c == '\\' {
				sb.WriteByte('\\')
			}
			sb.WriteRune(c)
		default:
			sb.WriteByte(byte('a' + r.Intn(26)))
		}
	}
	sb.WriteByte('"')
	return sb.String()
}

func randJSONNumber(r *rand.Rand) string {
	return []string{
		fmt.Sprint(r.Intn(1000)), fmt.Sprint(-r.Intn(1000)), "0", "-0", "9223372036854775807",
		"9223372036854775808", "-9223372036854775808", "-9223372036854775809", "1.5", "2.0", "1e3",
		"1E-2", "123456789012345678901234567890", "3", "0.0",
	}[r.Intn(15)]
}

func randJSONValue(r *rand.Rand, depth int) string {
	switch k := r.Intn(12); {
	case k < 5:
		return randJSONString(r)
	case k < 9:
		return randJSONNumber(r)
	case k < 10:
		return []string{"true", "false", "null"}[r.Intn(3)]
	default:
		if depth > 3 {
			return "null"
		}
		if r.Intn(2) == 0 {
			return "[" + randJSONValue(r, depth+1) + ", " + randJSONValue(r, depth+1) + "]"
		}
		return `{"file": ` + randJSONValue(r, depth+1) + `, "k": ` + randJSONValue(r, depth+1) + "}"
	}
}

var jsonKeys = []string{
	"status", "file", "line", "column", "message", "formatted",
	"Status", "FILE", "Line", "COLUMN", "Message", "\u017ftatus", "me\u017f\u017fage",
	"fi_le", "files", "", "\\u0066ile", "lin\\u0065", "\\u006D\\u0065ssage", "st\\u0061tus",
}

func randErrorDoc(r *rand.Rand) string {
	ws := func() string { return []string{"", " ", "\n\t", "\r\n  ", "  "}[r.Intn(5)] }
	var sb strings.Builder
	sb.WriteString(ws() + "{" + ws())
	nkeys := r.Intn(8)
	for i := 0; i < nkeys; i++ {
		if i > 0 {
			sb.WriteString(ws() + "," + ws())
		}
		key := jsonKeys[r.Intn(len(jsonKeys))]
		var val string
		if r.Intn(3) == 0 {
			val = randJSONValue(r, 0)
		} else {
			// Well-typed value for the key.
			switch strings.ToLower(key) {
			case "status", "line", "column", "st\\u0061tus", "lin\\u0065":
				val = randJSONNumber(r)
			default:
				val = randJSONString(r)
			}
		}
		sb.WriteString(`"` + key + `"` + ws() + ":" + ws() + val)
	}
	sb.WriteString(ws() + "}" + ws())
	return sb.String()
}

func mutateJSON(r *rand.Rand, s string) string {
	b := []byte(s)
	for m := r.Intn(3); m >= 0 && len(b) > 0; m-- {
		pos := r.Intn(len(b))
		switch r.Intn(3) {
		case 0:
			b = append(b[:pos:pos], b[pos+1:]...)
		case 1:
			const ins = "{}[],:\"\\ 0-e.tnfu"
			b = append(b[:pos:pos], append([]byte{ins[r.Intn(len(ins))]}, b[pos:]...)...)
		default:
			b[pos] = byte(r.Intn(256))
		}
	}
	return string(b)
}
