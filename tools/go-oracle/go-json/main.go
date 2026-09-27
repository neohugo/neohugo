// Command go-json is the Go oracle for the Rust crate crates/go-json
// (encoding/json v1 of the go1.27.1 toolchain, which is built on
// encoding/json/v2 and jsontext because GOEXPERIMENT=jsonv2 is on by default;
// build the oracle with the default experiments, as the golden neohugo-go).
//
// It writes gzip-compressed record streams (see writeRec) that the Rust tests
// replay:
//
//	go run ./tools/go-oracle/go-json -mode encode -out crates/go-json/tests/fixtures/encode.rec.gz [-n N] [-seed S]
//	    random values (vdump, see dump.go) and their Marshal / MarshalIndent /
//	    Encoder outputs or errors.
//	go run ./tools/go-oracle/go-json -mode text -out crates/go-json/tests/fixtures/text.rec.gz [-n N] [-seed S]
//	    JSON texts (valid, mutated and hand-written edge cases) with Valid,
//	    Compact, Indent, HTMLEscape, Marshal(RawMessage), Unmarshal into any and
//	    into map[string]any, Decoder.Decode streams, Decoder.Token streams and
//	    random interleavings of Token/Decode/More/InputOffset.
//	go run ./tools/go-oracle/go-json -mode advtext|advencode -out FILE [-n N] [-seed S]
//	    adversarial inputs in the text/encode record formats (see adv.go).
//	go run ./tools/go-oracle/go-json -mode real -out crates/go-json/tests/fixtures/real.rec.gz FILE...
//	    real inputs (HTTP responses have their header stripped): Unmarshal,
//	    re-encoding with the options Hugo uses, and chunked streaming reads.
package main

import (
	"bufio"
	"bytes"
	"compress/gzip"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"html/template"
	"io"
	"log"
	"math/rand/v2"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

func main() {
	mode := flag.String("mode", "", "encode | text | real")
	out := flag.String("out", "", "output file (.gz)")
	n := flag.Int("n", 3000, "number of random cases")
	seed := flag.Uint64("seed", 1, "random seed")
	flag.Parse()
	if *out == "" {
		log.Fatal("missing -out")
	}
	f, err := os.Create(*out)
	if err != nil {
		log.Fatal(err)
	}
	zw, _ := gzip.NewWriterLevel(f, gzip.BestCompression)
	w := bufio.NewWriter(zw)
	r := rand.New(rand.NewPCG(*seed, 0x9e3779b97f4a7c15))
	switch *mode {
	case "encode":
		writeEncode(w, r, *n)
	case "text":
		writeText(w, r, *n)
	case "real":
		writeReal(w, flag.Args())
	case "advtext":
		writeAdvText(w, r, *n)
	case "advencode":
		writeAdvEncode(w, r, *n)
	default:
		log.Fatalf("unknown -mode %q", *mode)
	}
	if err := w.Flush(); err != nil {
		log.Fatal(err)
	}
	if err := zw.Close(); err != nil {
		log.Fatal(err)
	}
	if err := f.Close(); err != nil {
		log.Fatal(err)
	}
}

// writeRec writes one record: "<name> <len>\n<data>\n".
func writeRec(w io.Writer, name string, data []byte) {
	fmt.Fprintf(w, "%s %d\n", name, len(data))
	w.Write(data)
	w.Write([]byte{'\n'})
}

// errRec formats an error as "E<kind>:<offset>:<message>".
func errRec(err error) []byte {
	kind, off := byte('O'), int64(-1)
	var se *json.SyntaxError
	var te *json.UnmarshalTypeError
	var me *json.MarshalerError
	var ue *json.UnsupportedValueError
	var ut *json.UnsupportedTypeError
	switch {
	case errors.As(err, &me):
		kind = 'M'
	case errors.As(err, &se):
		kind, off = 'S', se.Offset
	case errors.As(err, &te):
		kind, off = 'T', te.Offset
	case errors.As(err, &ue):
		kind = 'U'
	case errors.As(err, &ut):
		kind = 'Y'
	case err == io.EOF:
		kind = 'F'
	case err == io.ErrUnexpectedEOF:
		kind = 'G'
	}
	return fmt.Appendf(nil, "E%c:%d:%s", kind, off, err.Error())
}

func result(b []byte, err error) []byte {
	if err != nil {
		return errRec(err)
	}
	return append([]byte{'O'}, b...)
}

var indentArgs = [][2]string{{"", "  "}, {"", "\t"}, {"", ""}, {">", "."}, {"\n", " "}, {"  ", ""}, {"<&>", "\u2028"}, {"", "   "}}

func pickIndent(r *rand.Rand) (string, string) {
	if advIndent {
		return pickAdvIndent(r, nil)
	}
	a := indentArgs[r.IntN(len(indentArgs))]
	return a[0], a[1]
}

func encodeWith(v any, escape bool, prefix, indent string) ([]byte, error) {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(escape)
	enc.SetIndent(prefix, indent)
	err := enc.Encode(v)
	return buf.Bytes(), err
}

func writeEncode(w io.Writer, r *rand.Rand, n int) {
	var vals []any
	// Every float special as a top-level value and inside a list.
	for _, f := range f64s {
		vals = append(vals, f, -f)
	}
	for _, f := range f32s {
		vals = append(vals, f, -f)
	}
	for c := 0; c < 256; c++ {
		vals = append(vals, string([]byte{byte(c)}), template.HTML(string([]byte{'<', byte(c), '>'})))
	}
	for _, s := range strPieces {
		vals = append(vals, s, map[string]any{s: s})
	}
	for i := 0; i < n; i++ {
		bad := r.IntN(5) == 0
		vals = append(vals, genValue(r, 4, bad))
	}
	for _, v := range vals {
		writeEncodeCase(w, r, v)
	}
}

// writeEncodeCase writes one encode case: the vdump of v and its Marshal,
// Encoder (no HTML escaping), MarshalIndent and Encoder+SetIndent outputs.
func writeEncodeCase(w io.Writer, r *rand.Rand, v any) {
	d := dump(nil, v)
	uv := unwrap(v)
	writeRec(w, "case", nil)
	writeRec(w, "value", d)
	writeRec(w, "marshal", result(json.Marshal(uv)))
	writeRec(w, "nohtml", result(encodeWith(uv, false, "", "")))
	p, in := pickIndent(r)
	writeRec(w, "indentargs", []byte(p+"\x00"+in))
	writeRec(w, "marshalindent", result(json.MarshalIndent(uv, p, in)))
	esc := r.IntN(2) == 0
	p, in = pickIndent(r)
	writeRec(w, "encargs", []byte(fmt.Sprintf("%d%s\x00%s", b2i(esc), p, in)))
	writeRec(w, "encoder", result(encodeWith(uv, esc, p, in)))
}

func b2i(b bool) int {
	if b {
		return 1
	}
	return 0
}

// chunkReader returns its data in chunks of the given size, then (0, EOF).
type chunkReader struct {
	data  []byte
	chunk int
}

func (c *chunkReader) Read(p []byte) (int, error) {
	if len(c.data) == 0 {
		return 0, io.EOF
	}
	n := min(c.chunk, len(p), len(c.data))
	copy(p, c.data[:n])
	c.data = c.data[n:]
	return n, nil
}

func textCase(w io.Writer, r *rand.Rand, in []byte) {
	writeRec(w, "case", nil)
	writeRec(w, "input", in)
	writeRec(w, "valid", []byte(strconv.Itoa(b2i(json.Valid(in)))))

	var cb bytes.Buffer
	cb.WriteString("pre")
	err := json.Compact(&cb, in)
	writeRec(w, "compact", result(cb.Bytes(), err))

	writeRec(w, "rawmsg", result(json.Marshal(json.RawMessage(in))))

	p, ind := pickIndent(r)
	if advIndent {
		p, ind = pickAdvIndent(r, in)
	}
	writeRec(w, "indentargs", []byte(p+"\x00"+ind))
	var ib bytes.Buffer
	ib.WriteString("pre")
	err = json.Indent(&ib, in, p, ind)
	writeRec(w, "indent", result(ib.Bytes(), err))

	var hb bytes.Buffer
	json.HTMLEscape(&hb, in)
	writeRec(w, "htmlescape", hb.Bytes())

	var v any
	err = json.Unmarshal(in, &v)
	if err != nil {
		writeRec(w, "unmarshal", append(append(errRec(err), '\n'), dump(nil, v)...))
	} else {
		writeRec(w, "unmarshal", append([]byte{'O'}, dump(nil, v)...))
	}

	m := make(map[string]any)
	err = json.Unmarshal(in, &m)
	if err != nil {
		writeRec(w, "unmarshalmap", append(append(errRec(err), '\n'), dump(nil, m)...))
	} else {
		writeRec(w, "unmarshalmap", append([]byte{'O'}, dump(nil, m)...))
	}

	// Decoder.Decode stream.
	chunk := 1 + r.IntN(64)
	if r.IntN(4) == 0 {
		chunk = 1 << 20
	}
	useNumber := r.IntN(3) == 0
	writeRec(w, "streamargs", []byte(fmt.Sprintf("%d %d", chunk, b2i(useNumber))))
	dec := json.NewDecoder(&chunkReader{data: in, chunk: chunk})
	if useNumber {
		dec.UseNumber()
	}
	for i := 0; i < 40; i++ {
		var x any
		err := dec.Decode(&x)
		var res []byte
		if err != nil {
			res = errRec(err)
		} else {
			res = append([]byte{'D'}, dump(nil, x)...)
		}
		writeRec(w, "dstep", append(fmt.Appendf(nil, "%d|", dec.InputOffset()), res...))
		if err != nil && (err == io.EOF || !isTypeErr(err)) {
			break
		}
	}
	buf, _ := io.ReadAll(dec.Buffered())
	writeRec(w, "buffered", buf)

	// Decoder.Token stream.
	dec = json.NewDecoder(&chunkReader{data: in, chunk: chunk})
	if useNumber {
		dec.UseNumber()
	}
	for i := 0; i < 200; i++ {
		tok, err := dec.Token()
		off := dec.InputOffset()
		more := dec.More()
		var res []byte
		switch t := tok.(type) {
		case json.Delim:
			res = []byte{'D', byte(t)}
		default:
			res = append([]byte{'V'}, dump(nil, t)...)
		}
		if err != nil {
			res = errRec(err)
		}
		writeRec(w, "tstep", append(fmt.Appendf(nil, "%d|%d|", off, b2i(more)), res...))
		if err != nil && (err == io.EOF || !isTypeErr(err)) {
			break
		}
	}

	// Interleaved Token/Decode/More/InputOffset calls.
	ops := randomOps(r)
	writeRec(w, "mixargs", []byte(ops))
	mixedOps(w, in, ops, chunk, useNumber)
}

// mixedOps interleaves Token, Decode, More and InputOffset calls on one
// Decoder, as callers of the streaming API do.
func mixedOps(w io.Writer, in []byte, ops string, chunk int, useNumber bool) {
	dec := json.NewDecoder(&chunkReader{data: in, chunk: chunk})
	if useNumber {
		dec.UseNumber()
	}
	for _, op := range ops {
		var res []byte
		switch op {
		case 'T':
			tok, err := dec.Token()
			switch t := tok.(type) {
			case json.Delim:
				res = []byte{'D', byte(t)}
			default:
				res = append([]byte{'V'}, dump(nil, t)...)
			}
			if err != nil {
				res = errRec(err)
			}
		case 'D':
			var x any
			err := dec.Decode(&x)
			if err != nil {
				res = append(errRec(err), '\n')
			} else {
				res = []byte{'O'}
			}
			res = append(res, dump(nil, x)...)
		case 'M':
			res = []byte{'M', byte('0' + b2i(dec.More()))}
		case 'O':
			res = fmt.Appendf(nil, "O%d", dec.InputOffset())
		}
		writeRec(w, "mstep", res)
	}
	buf, _ := io.ReadAll(dec.Buffered())
	writeRec(w, "mbuffered", buf)
}

func randomOps(r *rand.Rand) string {
	n := 1 + r.IntN(60)
	b := make([]byte, n)
	for i := range b {
		b[i] = "TTTDDMMO"[r.IntN(8)]
	}
	return string(b)
}

func isTypeErr(err error) bool {
	var te *json.UnmarshalTypeError
	return errors.As(err, &te)
}

func writeText(w io.Writer, r *rand.Rand, n int) {
	for _, s := range handTexts() {
		textCase(w, r, []byte(s))
	}
	for i := 0; i < n; i++ {
		var in []byte
		switch r.IntN(6) {
		case 0:
			// Several values in a row (streams).
			k := 1 + r.IntN(4)
			ws := []string{" ", "\n", "\t", "", "  \r\n"}
			for j := 0; j < k; j++ {
				in = append(in, genJSONText(r, false)...)
				in = append(in, ws[r.IntN(len(ws))]...)
			}
			if r.IntN(3) == 0 {
				in = mutate(r, in)
			}
		case 1, 2:
			in = []byte(genJSONText(r, false))
		default:
			in = mutate(r, []byte(genJSONText(r, false)))
		}
		textCase(w, r, in)
	}
}

func handTexts() []string {
	t := []string{
		"", " ", "\n\t\r ", "null", "true", "false", "0", "-0", "1", "-1", "1.5", "1e5", "1E+5", "1e-5", "0.1e1", "-0.0e-0",
		"1e400", "-1e400", "1e-400", "123456789012345678901234567890", "1e999999999999", "4.9e-324", "2.2250738585072011e-308",
		"17976931348623157e292", "179769313486231580793728971405303415079934132710037826936173778980444968292764750946649017977587207096330286416692887910946555547851940402630657488671505820681908902000708383676273854845817711531764475730270069855571366959622842914819860834936475292719074168444365510704342711559699508093042880177904174497792",
		"-", "01", "1.", "1.e5", ".5", "1e", "1e+", "+1", "0x10", "NaN", "Infinity", "- 1", "00", "-01",
		"tru", "truex", "nul", "nulll", "fals", "fa1se", "t", "n", "f", "True", "NULL",
		`""`, `"a"`, `"\""`, `"\\"`, `"\/"`, `"\b\f\n\r\t"`, `"\u0041"`, `"\u00e9"`, `"\u2028\u2029"`, `"\ud83d\ude00"`, `"\uD834\uDD1E"`,
		`"\ud800"`, `"\udc00"`, `"\ud800\u0041"`, `"\ud800\ud800"`, `"\udc00\ud800"`, `"\ud800x"`, `"\ud800\"`, `"\ud83d\ude0"`,
		`"\u12"`, `"\u12g4"`, `"\x"`, `"\'"`, `"\a"`, `"\U00000041"`, "\"\x00\"", "\"\x1f\"", "\"\t\"", "\"\x7f\"",
		"\"\xff\"", "\"\xc3\xa9\"", "\"\xed\xa0\x80\"", "\"\xe2\x80\xa8<&>\"", "\"a\xe2\x80\"", "\"\ufffd\"", `"<script>&amp;</script>"`,
		"\x80", "\xff", "\xc3\xa9", "\xe2\x80\xa8", "'a'", "\x00", "\x1f", "[\x80]", "{\"a\":\xff}",
		"[]", "{}", "[ ]", "{ }", "[1,2,3]", "[1,]", "[,1]", "[1 2]", "[1,,2]", "[", "]", "{", "}", "[}", "{]", "[[]", "[]]",
		`{"a":1}`, `{"a" 1}`, `{"a":}`, `{a:1}`, `{"a":1,}`, `{,"a":1}`, `{"a":1 "b":2}`, `{"a":1,"a":2}`, `{"a":{"b":1},"a":{"c":2}}`,
		`{"":0}`, `{"\u0000":1,"\ud800":2,"\ufffd":3}`, "{\"\xff\":1,\"\xfe\":2}", `{"a":1}}`, `{"a":[1,{"b":[]}]}`, `{1:2}`, `{"a":1,"b"}`,
		"1 2", "1\n2", `"a" "b"`, "{} {}", "[] x", "{}{}", "[][]", "1x", "\"a\"x", "null null", "true false", " 1 ", "1\t",
		"{\"a\":1}\n{\"a\":2}\n", "[1]\n[2", "1 {", "{\"a\":1e400}", "[1e400, 2]", "{\"x\":[-1e999]}",
		strings.Repeat("[", 10001) + strings.Repeat("]", 10001),
		strings.Repeat("[", 10000) + strings.Repeat("]", 10000),
		strings.Repeat(`{"a":`, 10001) + "1" + strings.Repeat("}", 10001),
		strings.Repeat("[", 100),
		`{"kind": "youtube#video", "statistics": {"viewCount": "1234"}, "n": [1.0, 2.50, -3e2]}`,
	}
	return t
}

// stripHTTP removes an HTTP response header block (Hugo's filecache stores
// the raw response).
func stripHTTP(b []byte) []byte {
	if bytes.HasPrefix(b, []byte("HTTP/")) {
		if i := bytes.Index(b, []byte("\r\n\r\n")); i >= 0 {
			return b[i+4:]
		}
	}
	return b
}

func writeReal(w io.Writer, files []string) {
	for _, fn := range files {
		raw, err := os.ReadFile(fn)
		if err != nil {
			log.Fatal(err)
		}
		in := stripHTTP(raw)
		writeRec(w, "case", nil)
		writeRec(w, "name", []byte(filepath.Base(fn)))
		writeRec(w, "input", in)
		var v any
		err = json.Unmarshal(in, &v)
		if err != nil {
			writeRec(w, "unmarshal", errRec(err))
			continue
		}
		writeRec(w, "unmarshal", append([]byte{'O'}, dump(nil, v)...))
		writeRec(w, "marshal", result(json.Marshal(v)))
		writeRec(w, "marshalindent", result(json.MarshalIndent(v, "", "  ")))
		writeRec(w, "encoder", result(encodeWith(v, false, "", "  ")))
		writeRec(w, "encoderhtml", result(encodeWith(v, true, "", "")))
		var ib bytes.Buffer
		err = json.Indent(&ib, in, "", "\t")
		writeRec(w, "indent", result(ib.Bytes(), err))
		var cb bytes.Buffer
		err = json.Compact(&cb, in)
		writeRec(w, "compact", result(cb.Bytes(), err))
		m := make(map[string]any)
		err = json.Unmarshal(in, &m)
		if err != nil {
			writeRec(w, "unmarshalmap", errRec(err))
		} else {
			writeRec(w, "unmarshalmap", append([]byte{'O'}, dump(nil, m)...))
		}
		// Streaming reads with several chunk sizes (buffer growth policy).
		for _, chunk := range []int{1, 7, 100, 4096, 1 << 20} {
			writeRec(w, "mixargs", fmt.Appendf(nil, "%d", chunk))
			mixedOps(w, in, "ODOTTTMOTOD", chunk, chunk == 7)
		}
	}
}

func indentBytes(b []byte, prefix, indent string) ([]byte, error) {
	var buf bytes.Buffer
	err := json.Indent(&buf, b, prefix, indent)
	return buf.Bytes(), err
}
