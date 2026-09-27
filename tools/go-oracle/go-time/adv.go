package main

// adv.txt.gz: adversarial and randomized cases (seed-deterministic), written
// by the independent verification pass. Record formats reuse the other
// fixtures, plus location definitions:
//
//	DEF  <id> <hex TZif>                  location <id> = LoadLocationFromTZData(<id>, data)
//	DEFE <id> <hex TZif> <error>          LoadLocationFromTZData fails
//	F    <loc> <unix> <nsec> <layout> <result>
//	S    <loc> <unix> <nsec> <String()> <GoString()>
//	P    <mode> <loc> <layout> <value> OK <time fields> | ERR <message>
//	L    <loc> <sec> <name> <offset> <isdst> <zbstart> <zbend>
//	DT   <loc> <y> <m> <d> <h> <mi> <s> <ns> <time fields>
//	AD   <loc> <unix> <nsec> <years> <months> <days> <time fields>
//	TR   <loc> <unix> <nsec> <dur> <Truncate time fields> <Round time fields>
//	MB   <loc> <unix> <nsec> <hex|ERR:msg> <json|ERR:msg> <text|ERR:msg>
//	UB   <hex> OK <time fields> | ERR <msg>
//	PD   <input> OK <int64> | ERR <msg>
//
// Location ids are those of the other fixtures ("UTC", "Local",
// "Fixed:<name>:<offset>", "Zone:<name>") or ids defined by DEF records.
// The -advscale flag multiplies the randomized sections (for a larger corpus
// outside the repository).

import (
	"archive/zip"
	"bytes"
	"encoding/binary"
	"encoding/hex"
	"fmt"
	"io"
	"math"
	"math/rand"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"time"
	"unicode/utf8"
)

var advExtraFixed = []struct {
	name string
	off  int
}{
	{"", -1}, {"", -59}, {"", -60}, {"", -61}, {"", -3599}, {"", -3600}, {"", -3601}, {"", 59}, {"", 60},
	{"", 61}, {"", 3599}, {"", 3601}, {"A", 86399}, {"B", -86399}, {"C", 360000}, {"", -360000},
	{"", 1<<31 - 1}, {"", -(1 << 31)}, {"é", 7200}, {`a"b\c`, -7200}, {"UTC", 0}, {"UTC", 3600},
	{"Local", 25200}, {"+07", 25200}, {"", 25200}, {"LMT", 24124}, {"", -12 * 3600}, {"", 14 * 3600},
	{"", -13 * 3600}, {"", 15 * 3600}, {"", 1800},
}

func advFixedID(name string, off int) string { return fmt.Sprintf("Fixed:%s:%d", name, off) }

// advLayoutTokens are the std chunks of format.go.
var advLayoutTokens = []string{
	"2006", "06", "Jan", "January", "01", "1", "Mon", "Monday", "2", "_2", "02", "__2", "002",
	"15", "3", "03", "4", "04", "5", "05", "PM", "pm", "MST",
	"Z0700", "Z070000", "Z07", "Z07:00", "Z07:00:00", "-0700", "-070000", "-07", "-07:00", "-07:00:00",
	".0", ".00", ".000", ".000000", ".000000000", ".0000000000", ".9", ".99", ".999", ".999999",
	".999999999", ".99999999999", ",0", ",000", ",9", ",999", ",999999999",
}

// advLayoutNear are literals that are close to std chunks.
var advLayoutNear = []string{
	"J", "Ja", "Janu", "Janx", "Janet", "M", "Mo", "Mond", "Month", "Mont", "MS", "MSt", "MSTx", "0", "00",
	"000", "0000", "007", "7", "8", "9", "12", "16", "55", "_", "__", "___", "_20", "_200", "_2006", "P", "p",
	"Pm", "pM", "Z", "Z0", "Z07:", "Z07:0", "Z070", "Z07000", "-", "-0", "-07:", "-07:0", "-070", "-07000",
	".", ",", "..", ".x", ".0x", ".9x", ".09", ".90", ":", " ", "  ", "T", "t", "a", "x", "day", "uary", "onth",
	"é", "\xff", "日", "\t", "UTC", "GMT", "ChST", "MeST", "WITA", "ESAST", "am", "AM", "\"", "\\",
}

func advRandLayout(r *rand.Rand) string {
	var b strings.Builder
	n := 1 + r.Intn(7)
	for i := 0; i < n; i++ {
		if r.Intn(10) < 6 {
			b.WriteString(advLayoutTokens[r.Intn(len(advLayoutTokens))])
		} else {
			b.WriteString(advLayoutNear[r.Intn(len(advLayoutNear))])
		}
	}
	return b.String()
}

var advValuePieces = []string{
	"0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "00", "01", "09", "10", "12", "13", "23", "24", "29", "30",
	"31", "32", "59", "60", "61", "99", "100", "365", "366", "999", "2006", "2010", "1999", "0000", "9999",
	"69", "68", "Jan", "jan", "JAN", "January", "Feb", "February", "Mar", "Sep", "Sept", "Dec", "Mon",
	"Monday", "Tue", "Thu", "Thursday", "Sun", "Sunday", "AM", "PM", "am", "pm", "aM", "Pm", "Z", "UTC",
	"GMT", "GMT+3", "GMT-12", "GMT+24", "GMT+", "+07", "-03", "+0700", "-0800", "+07:00", "-08:00", "+070000",
	"+07:00:00", "-24", "+99", "PST", "PDT", "EST", "EDT", "CEST", "ChST", "MeST", "WITA", "ESAST", "ICT", "BMT",
	"LMT", "+1030", "-0330", " ", "  ", ":", ".", ",", "-", "+", "T", "_", "/", "x", "\xff", "é", ".5",
	",123", ".000000000001", ".9999999999", "\"",
}

func advMutate(r *rand.Rand, s string) string {
	b := []byte(s)
	for k := 1 + r.Intn(3); k > 0; k-- {
		switch r.Intn(6) {
		case 0: // insert a piece
			i := r.Intn(len(b) + 1)
			p := advValuePieces[r.Intn(len(advValuePieces))]
			b = append(b[:i], append([]byte(p), b[i:]...)...)
		case 1: // replace a range by a piece
			if len(b) > 0 {
				i := r.Intn(len(b))
				j := i + r.Intn(len(b)-i) + 1
				if j-i > 4 {
					j = i + 4
				}
				p := advValuePieces[r.Intn(len(advValuePieces))]
				b = append(b[:i], append([]byte(p), b[j:]...)...)
			}
		case 2: // delete a range
			if len(b) > 0 {
				i := r.Intn(len(b))
				j := i + 1 + r.Intn(3)
				if j > len(b) {
					j = len(b)
				}
				b = append(b[:i], b[j:]...)
			}
		case 3: // change a digit
			for i := range b {
				if b[i] >= '0' && b[i] <= '9' && r.Intn(4) == 0 {
					b[i] = byte('0' + r.Intn(10))
				}
			}
		case 4: // truncate
			if len(b) > 0 {
				b = b[:r.Intn(len(b))]
			}
		default: // duplicate a range
			if len(b) > 0 {
				i := r.Intn(len(b))
				j := i + 1 + r.Intn(4)
				if j > len(b) {
					j = len(b)
				}
				b = append(b[:j], append(append([]byte(nil), b[i:j]...), b[j:]...)...)
			}
		}
	}
	return string(b)
}

func advRandValue(r *rand.Rand) string {
	var b strings.Builder
	for n := 1 + r.Intn(10); n > 0; n-- {
		b.WriteString(advValuePieces[r.Intn(len(advValuePieces))])
	}
	return b.String()
}

// tzifSpec describes a TZif file for buildTZif.
type tzifSpec struct {
	version byte // 0, '2', '3'
	tx      []int64
	idx     []uint8
	zones   []tzifZone
	abbrs   string
	nleap   int
	nstd    int
	nut     int
	extend  string
	// v1 block for version >= 2: when false the v1 block holds no data
	// (a "slim" file); when true it repeats the data with 32-bit times.
	fatV1 bool
}

type tzifZone struct {
	off  int32
	dst  bool
	abbr uint8
}

func buildTZif(s tzifSpec) []byte {
	var b bytes.Buffer
	w32 := func(v uint32) { binary.Write(&b, binary.BigEndian, v) }
	block := func(is64, empty bool) {
		b.WriteString("TZif")
		b.WriteByte(s.version)
		b.Write(make([]byte, 15))
		if empty {
			// one zone, no transitions (like zic -b slim)
			for _, v := range []int{0, 0, 0, 0, 1, 4} {
				w32(uint32(v))
			}
			w32(0)
			b.WriteByte(0)
			b.WriteByte(0)
			b.WriteString("LMT\x00")
			return
		}
		for _, v := range []int{s.nut, s.nstd, s.nleap, len(s.tx), len(s.zones), len(s.abbrs)} {
			w32(uint32(v))
		}
		for _, t := range s.tx {
			if is64 {
				binary.Write(&b, binary.BigEndian, t)
			} else {
				binary.Write(&b, binary.BigEndian, int32(t))
			}
		}
		b.Write(s.idx)
		for _, z := range s.zones {
			binary.Write(&b, binary.BigEndian, z.off)
			if z.dst {
				b.WriteByte(1)
			} else {
				b.WriteByte(0)
			}
			b.WriteByte(z.abbr)
		}
		b.WriteString(s.abbrs)
		for i := 0; i < s.nleap; i++ {
			when := int64(78796800 + 15768000*i)
			if is64 {
				binary.Write(&b, binary.BigEndian, when)
			} else {
				binary.Write(&b, binary.BigEndian, int32(when))
			}
			binary.Write(&b, binary.BigEndian, int32(i+1))
		}
		b.Write(bytes.Repeat([]byte{1}, s.nstd))
		b.Write(bytes.Repeat([]byte{0}, s.nut))
	}
	if s.version == 0 {
		block(false, false)
		return b.Bytes()
	}
	block(false, !s.fatV1)
	block(true, false)
	b.WriteString("\n" + s.extend + "\n")
	return b.Bytes()
}

var advExtends = []string{
	"", "EST5EDT,M3.2.0,M11.1.0", "CET-1CEST,M3.5.0,M10.5.0/3", "AEST-10AEDT,M10.1.0,M4.1.0/3", "<+07>-7",
	"<-03>3", "IST-2IDT,M3.4.4/26,M10.5.0", "GMT0IST,M10.5.0,M3.5.0/1", "<+13>-13<+14>,M9.5.0/3,M4.1.0/4",
	"NZST-12NZDT,M9.5.0,M4.1.0/3", "<-04>4<-03>,M9.1.6/24,M4.1.6/24", "WET0WEST,M3.5.0/1,M10.5.0",
	"XXX3YYY,M3.2.0,M11.1.0x", "PST8PDT", "ABC", "<>0", "<>0<>,J1,J365",
}

// advRandExtend builds a TZ string from the tzset grammar with random
// (sometimes out-of-range) numbers.
func advRandExtend(r *rand.Rand) string {
	name := func() string {
		switch r.Intn(8) {
		case 0:
			return "<" + []string{"+07", "-0330", "A+B", "", "X"}[r.Intn(5)] + ">"
		case 1:
			return []string{"AB", "A", "ABCDEFG"}[r.Intn(3)]
		default:
			return []string{"EST", "EDT", "CET", "CEST", "LMT", "XYZ", "AEST", "ÄBC"}[r.Intn(8)]
		}
	}
	num := func(max int) string {
		if r.Intn(10) == 0 {
			return fmt.Sprint(max + 1 + r.Intn(3))
		}
		return fmt.Sprint(r.Intn(max + 1))
	}
	off := func() string {
		var s string
		switch r.Intn(3) {
		case 0:
			s = "+"
		case 1:
			s = "-"
		}
		s += num(24 * 7)
		if r.Intn(2) == 0 {
			s += ":" + num(59)
			if r.Intn(2) == 0 {
				s += ":" + num(59)
			}
		}
		return s
	}
	rule := func() string {
		var s string
		switch r.Intn(4) {
		case 0:
			s = "J" + num(365)
		case 1:
			s = num(365)
		default:
			s = "M" + num(12) + "." + num(5) + "." + num(6)
		}
		if r.Intn(2) == 0 {
			s += "/" + off()
		}
		return s
	}
	s := name() + off()
	if r.Intn(5) == 0 {
		return s
	}
	s += name()
	if r.Intn(2) == 0 {
		s += off()
	}
	switch r.Intn(10) {
	case 0:
		return s
	case 1:
		return s + ";" + rule() + "," + rule()
	case 2:
		return s + "," + rule()
	}
	return s + "," + rule() + "," + rule()
}

// tzifTimes returns the transition times LoadLocationFromTZData would use
// (the 64-bit block for version 2+ files).
func tzifTimes(d []byte) []int64 {
	if len(d) < 44 || string(d[:4]) != "TZif" {
		return nil
	}
	counts := func(b []byte) (n [6]int) {
		for i := range n {
			n[i] = int(binary.BigEndian.Uint32(b[20+4*i:]))
		}
		return
	}
	n := counts(d)
	size := 4
	if d[4] >= '2' {
		skip := 44 + n[3]*5 + n[4]*6 + n[5] + n[2]*8 + n[1] + n[0]
		if len(d) < skip+44 {
			return nil
		}
		d = d[skip:]
		n = counts(d)
		size = 8
	}
	p := d[44:]
	var tx []int64
	for i := 0; i < n[3] && (i+1)*size <= len(p); i++ {
		if size == 8 {
			tx = append(tx, int64(binary.BigEndian.Uint64(p[8*i:])))
		} else {
			tx = append(tx, int64(int32(binary.BigEndian.Uint32(p[4*i:]))))
		}
	}
	return tx
}

// cacheDependent reports whether Go's per-Location lookup cache (filled from
// the wall clock at load time, not modelled by the Rust port) can change
// lookups: only with unsorted transition times, which real tzdata never has.
func cacheDependent(data []byte) bool {
	tx := tzifTimes(data)
	for i := 0; i+1 < len(tx); i++ {
		if tx[i] > tx[i+1] {
			return true
		}
	}
	return false
}

// Zones used by the ported Go tests (tests/go_tables.rs), exported next to
// the fixture zones; and Go's slim TZif test files ($GOROOT/src/time/testdata).
var extraTestZones = []string{"Asia/Shanghai", "Australia/Brisbane", "Etc/GMT+1", "PST8PDT", "Pacific/Fakaofo", "Asia/Jerusalem"}
var slimTestFiles = []string{"2020b_Europe_Berlin", "2021a_America_Nuuk", "2021a_Asia_Gaza", "2021a_Europe_Dublin"}

// exportExtraZones writes extraTestZones and slimTestFiles into dir/zoneinfo
// and returns their data by file name.
func exportExtraZones(dir string) map[string][]byte {
	out := map[string][]byte{}
	zr, err := zip.OpenReader(filepath.Join(runtime.GOROOT(), "lib", "time", "zoneinfo.zip"))
	must(err)
	defer zr.Close()
	for _, f := range zr.File {
		for _, z := range extraTestZones {
			if f.Name == z {
				rc, err := f.Open()
				must(err)
				d, err := io.ReadAll(rc)
				must(err)
				rc.Close()
				out[zoneFile(z)] = d
			}
		}
	}
	for _, n := range slimTestFiles {
		d, err := os.ReadFile(filepath.Join(runtime.GOROOT(), "src", "time", "testdata", n))
		must(err)
		out["testdata__"+n] = d
	}
	must(os.MkdirAll(filepath.Join(dir, "zoneinfo"), 0o755))
	for n, d := range out {
		must(os.WriteFile(filepath.Join(dir, "zoneinfo", n), d, 0o644))
	}
	if len(out) != len(extraTestZones)+len(slimTestFiles) {
		panic("missing extra test zones")
	}
	return out
}

func genAdversarial(dir string, scale int) {
	o := create(dir, "adv.txt.gz")
	defer o.close()
	r := rand.New(rand.NewSource(1001))
	extra := exportExtraZones(dir)

	b2s := func(b bool) string {
		if b {
			return "1"
		}
		return "0"
	}
	zb := func(t time.Time) string {
		if t.IsZero() {
			return "Z"
		}
		return i64(t.Unix())
	}
	// cacheWindow[id] = [lo, hi): lookups that Go answers from the
	// Location's load-time cache although the transition table says
	// otherwise (the cached tzset range starts before the last transition;
	// deviation 2 in PORTING.md). Skipped so fixtures do not depend on the
	// wall clock at generation time.
	cacheWindow := map[string][2]int64{}
	inCacheWindow := func(id string, sec int64) bool {
		w, ok := cacheWindow[id]
		return ok && w[0] <= sec && sec < w[1]
	}
	fmtRec := func(id string, sec, ns int64, layout string) {
		if inCacheWindow(id, sec) {
			return
		}
		t := time.Unix(sec, ns).In(locByID[id])
		if name, _ := t.Zone(); !utf8.ValidString(name) {
			return
		}
		o.rec("F", id, i64(sec), i64(ns), layout, t.Format(layout))
	}
	strRec := func(id string, sec, ns int64) {
		t := time.Unix(sec, ns).In(locByID[id])
		o.rec("S", id, i64(sec), i64(ns), t.String(), t.GoString())
	}
	lookupRec := func(id string, sec int64) {
		if inCacheWindow(id, sec) {
			return
		}
		t := time.Unix(sec, 0).In(locByID[id])
		name, off := t.Zone()
		if !utf8.ValidString(name) {
			// Non-UTF-8 zone names cannot be represented by the Rust port
			// (go_value::Zone::name is a String; see PORTING.md): compare
			// everything but the name.
			st, en := t.ZoneBounds()
			o.rec("LQ", id, i64(sec), "", itoa(off), b2s(t.IsDST()), zb(st), zb(en))
			return
		}
		st, en := t.ZoneBounds()
		o.rec("L", id, i64(sec), name, itoa(off), b2s(t.IsDST()), zb(st), zb(en))
	}
	mbRec := func(id string, sec, ns int64) []byte {
		t := time.Unix(sec, ns).In(locByID[id])
		mb, err := t.MarshalBinary()
		mbs := hex.EncodeToString(mb)
		if err != nil {
			mbs = "ERR:" + err.Error()
		}
		js, err := t.MarshalJSON()
		jss := string(js)
		if err != nil {
			jss = "ERR:" + err.Error()
		}
		tx, err := t.MarshalText()
		txs := string(tx)
		if err != nil {
			txs = "ERR:" + err.Error()
		}
		o.rec("MB", id, i64(sec), i64(ns), mbs, jss, txs)
		return mb
	}
	ubRec := func(b []byte) {
		var t time.Time
		if err := t.UnmarshalBinary(b); err != nil {
			o.rec("UB", hex.EncodeToString(b), "ERR", err.Error())
			return
		}
		o.rec(append([]string{"UB", hex.EncodeToString(b), "OK"}, timeFields(t)...)...)
	}
	dtRec := func(id string, y, m, d, h, mi, s, ns int) {
		t := time.Date(y, time.Month(m), d, h, mi, s, ns, locByID[id])
		o.rec(append([]string{"DT", id, itoa(y), itoa(m), itoa(d), itoa(h), itoa(mi), itoa(s), itoa(ns)}, timeFields(t)...)...)
	}
	def := func(id string, data []byte) bool {
		l, err := time.LoadLocationFromTZData(id, data)
		if err != nil {
			o.rec("DEFE", id, hex.EncodeToString(data), err.Error())
			return false
		}
		o.rec("DEF", id, hex.EncodeToString(data))
		locByID[id] = l
		if tx := tzifTimes(data); len(tx) > 0 {
			// Only a cache filled from the footer (now at or after the last
			// transition) can diverge; at "now" the lookup hits the cache
			// and returns its bounds.
			now := time.Now()
			last := tx[len(tx)-1]
			if st, _ := now.In(l).ZoneBounds(); now.Unix() >= last && !st.IsZero() && st.Unix() < last {
				cacheWindow[id] = [2]int64{st.Unix(), last}
			}
		}
		return true
	}

	// Extra fixed zones.
	var fixedIDs []string
	for _, fz := range advExtraFixed {
		id := advFixedID(fz.name, fz.off)
		if _, ok := locByID[id]; !ok {
			locByID[id] = time.FixedZone(fz.name, fz.off)
		}
		fixedIDs = append(fixedIDs, id)
	}
	allIDs := []string{}
	for _, nl := range locs {
		allIDs = append(allIDs, nl.id)
	}
	allIDs = append(allIDs, fixedIDs...)

	insts := append(specialInstants(), randInstants(r, 200)...)
	pickInst := func() [2]int64 {
		if r.Intn(3) == 0 {
			return insts[r.Intn(len(insts))]
		}
		return randInstants(r, 1)[0]
	}

	// 1. Random layouts: Format, round-trip Parse/ParseInLocation and mutations.
	for i := 0; i < 4500*scale; i++ {
		layout := advRandLayout(r)
		in := pickInst()
		id := allIDs[r.Intn(len(allIDs))]
		fmtRec(id, in[0], in[1], layout)
		s := time.Unix(in[0], in[1]).In(locByID[id]).Format(layout)
		parseRec(o, "Parse", "", layout, s)
		parseRec(o, "In", id, layout, s)
		parseRec(o, "Parse", "", layout, advMutate(r, s))
		if i%3 == 0 {
			parseRec(o, "In", allIDs[r.Intn(len(allIDs))], layout, advMutate(r, s))
		}
	}

	// 2. Value fuzz against named and custom layouts.
	layouts := allLayouts()
	for i := 0; i < 6000*scale; i++ {
		layout := layouts[r.Intn(len(layouts))]
		var v string
		if i%4 == 0 {
			v = advRandValue(r)
		} else {
			in := pickInst()
			v = advMutate(r, time.Unix(in[0], in[1]).In(locByID[allIDs[r.Intn(len(allIDs))]]).Format(layout))
		}
		if i%2 == 0 {
			parseRec(o, "Parse", "", layout, v)
		} else {
			parseRec(o, "In", allIDs[r.Intn(len(allIDs))], layout, v)
		}
	}

	// 3. Zone abbreviations and offsets through ParseInLocation (lookupName).
	for _, z := range fixtureZones {
		id := "Zone:" + z
		l := locByID[id]
		abbrs := map[string]bool{}
		type zo struct {
			name string
			off  int
		}
		var seen []zo
		t := time.Date(1700, 1, 1, 0, 0, 0, 0, time.UTC)
		for k := 0; k < 3000; k++ {
			name, off := t.In(l).Zone()
			if !abbrs[name] {
				abbrs[name] = true
			}
			seen = append(seen, zo{name, off})
			_, end := t.In(l).ZoneBounds()
			if end.IsZero() || end.Year() > 2100 {
				break
			}
			t = end
		}
		var names []string
		for n := range abbrs {
			names = append(names, n)
		}
		sort.Strings(names)
		names = append(names, "UTC", "GMT", "XYZ", "GMT+5", "+05", "-0330")
		for k := 0; k < 60; k++ {
			y := 1800 + r.Intn(320)
			v0 := fmt.Sprintf("%04d-%02d-%02d %02d:%02d:%02d", y, 1+r.Intn(12), 1+r.Intn(28), r.Intn(24), r.Intn(60), r.Intn(60))
			name := names[r.Intn(len(names))]
			parseRec(o, "In", id, "2006-01-02 15:04:05 MST", v0+" "+name)
			parseRec(o, "Parse", "", "2006-01-02 15:04:05 MST", v0+" "+name)
			so := seen[r.Intn(len(seen))]
			off := so.off
			if r.Intn(4) == 0 {
				off += 60 * (r.Intn(5) - 2)
			}
			sign := "+"
			if off < 0 {
				sign = "-"
				off = -off
			}
			zs := fmt.Sprintf("%s%02d%02d", sign, off/3600, off/60%60)
			zss := fmt.Sprintf("%s%02d:%02d:%02d", sign, off/3600, off/60%60, off%60)
			nm := so.name
			if r.Intn(3) == 0 {
				nm = names[r.Intn(len(names))]
			}
			parseRec(o, "In", id, "2006-01-02 15:04:05 -0700 MST", v0+" "+zs+" "+nm)
			parseRec(o, "In", id, "2006-01-02 15:04:05 -07:00:00", v0+" "+zss)
			parseRec(o, "In", id, time.RFC3339, strings.Replace(v0, " ", "T", 1)+zss[:6])
		}
	}

	// 4. Format/String/GoString/Marshal in the extra fixed zones and all zones.
	fixedInsts := append(specialInstants(), randInstants(r, 40*scale)...)
	for _, id := range fixedIDs {
		for _, in := range fixedInsts {
			fmtRec(id, in[0], in[1], "2006-01-02 15:04:05.999999999 MST -07:00:00 Z07 Z0700 -0700 Z07:00:00 -07")
			fmtRec(id, in[0], in[1], time.RFC3339Nano)
			fmtRec(id, in[0], in[1], time.RFC3339)
			strRec(id, in[0], in[1])
			if mb := mbRec(id, in[0], in[1]); mb != nil {
				ubRec(mb)
			}
		}
	}
	for _, id := range allIDs {
		for k := 0; k < 8*scale; k++ {
			in := pickInst()
			if mb := mbRec(id, in[0], in[1]); mb != nil && k%2 == 0 {
				ubRec(mb)
			}
		}
	}
	// UnmarshalBinary: Local offset matches, near misses, v2 seconds.
	mk := func(version byte, sec int64, nsec int32, offMin int16, offSec int8) []byte {
		b := []byte{version}
		b = binary.BigEndian.AppendUint64(b, uint64(sec))
		b = binary.BigEndian.AppendUint32(b, uint32(nsec))
		b = binary.BigEndian.AppendUint16(b, uint16(offMin))
		if version == 2 {
			b = append(b, byte(offSec))
		}
		return b
	}
	for k := 0; k < 400*scale; k++ {
		sec := r.Int63n(1<<37) + 1<<36 // around the present, in internal seconds
		if k%5 == 0 {
			sec = r.Int63() - r.Int63()
		}
		var ns int32
		switch k % 4 {
		case 0:
			ns = int32(r.Intn(1e9))
		case 1:
			ns = int32(r.Uint32())
		}
		offMin := int16([]int{420, 419, 421, -1, 0, 1, -60, 402, 401, 42, -32768, 32767}[r.Intn(12)])
		ver := byte(1 + r.Intn(2))
		ubRec(mk(ver, sec, ns, offMin, int8(r.Intn(256)-128)))
		ubRec(mk(2, sec, ns, 401, int8([]int{56, 4, -4, 60, -60, 0}[r.Intn(6)])))
	}

	// 5. Synthetic TZif files: leap seconds, v1/v2/v3, slim/fat, 32-bit
	// wrap of transition times, many zones, bad indices.
	defN := 0
	defID := func() string {
		defN++
		return fmt.Sprintf("Def:%d", defN)
	}
	lookupAll := func(id string, tx []int64) {
		var secs []int64
		for _, w := range tx {
			secs = append(secs, w-1, w, w+1)
		}
		for i := 0; i < 20; i++ {
			secs = append(secs, r.Int63n(1<<36)-1<<35)
		}
		secs = append(secs, math.MinInt64, math.MaxInt64, 0, -1, 1<<31, -1<<31-1)
		for _, s := range secs {
			lookupRec(id, s)
		}
		for i := 0; i < 6; i++ {
			in := pickInst()
			fmtRec(id, in[0], in[1], "2006-01-02 15:04:05 MST -07:00:00")
		}
	}
	baseZones := []tzifZone{{-17762, false, 0}, {-18000, false, 4}, {-14400, true, 8}, {-10800, true, 12}, {3600, false, 16}}
	baseAbbrs := "LMT\x00EST\x00EDT\x00EWT\x00CET\x00"
	for k := 0; k < 120*scale; k++ {
		var s tzifSpec
		s.version = []byte{0, '2', '3'}[r.Intn(3)]
		n := r.Intn(12)
		w := int64(-3000000000 + r.Int63n(1000000000))
		for i := 0; i < n; i++ {
			w += r.Int63n(400000000) + 1
			s.tx = append(s.tx, w)
			s.idx = append(s.idx, uint8(r.Intn(len(baseZones))))
		}
		s.zones = baseZones[:1+r.Intn(len(baseZones))]
		for i := range s.idx {
			s.idx[i] %= uint8(len(s.zones))
		}
		s.abbrs = baseAbbrs
		s.nleap = []int{0, 0, 1, 27}[r.Intn(4)]
		if r.Intn(3) == 0 {
			s.nstd = len(s.zones)
			s.nut = len(s.zones)
		}
		if r.Intn(2) == 0 {
			s.extend = advExtends[r.Intn(len(advExtends))]
		} else {
			s.extend = advRandExtend(r)
		}
		s.fatV1 = r.Intn(2) == 0
		id := defID()
		data := buildTZif(s)
		if def(id, data) && !cacheDependent(data) {
			lookupAll(id, s.tx)
		}
	}
	// Footers with invalid UTF-8: tzset ranges over runes, an invalid byte is
	// a width-1 RuneError (it must not become a 3-byte U+FFFD).
	for _, ext := range []string{"IST-5\xcd30", "A\xff3", "AB\xff3", "A\xffB3C\xffD,M3.2.0,M11.1.0", "<A\xffB>3",
		"EST5EDT,M3.2.0,M11.1.0\xff", "EST5\xffEDT,M3.2.0,M11.1.0", "EST5EDT\xff,M3.2.0,M11.1.0", "EST5EDT,M3.2.0\xff,M11.1.0",
		"\xff\xff\xff5\xfe\xfe\xfe,M3.2.0,M11.1.0", "\xc3\xa4BC3", "\xc3BC3", "\xef\xbf\xbd3", "E\xef\xbf\xbd3D\xed\xa0\x80T"} {
		for _, withTx := range []bool{false, true} {
			id := defID()
			data := tzif(ext, withTx)
			if def(id, data) {
				var secs []int64
				for y := 1990; y <= 2040; y += 7 {
					for _, md := range [][2]int{{1, 1}, {3, 8}, {3, 15}, {6, 1}, {11, 2}, {11, 9}, {12, 31}} {
						secs = append(secs, time.Date(y, time.Month(md[0]), md[1], 12, 0, 0, 0, time.UTC).Unix())
					}
				}
				for _, sec := range secs {
					lookupRec(id, sec)
				}
			}
		}
	}

	// The extra test zones and Go's slim testdata files.
	var extraNames []string
	for n := range extra {
		extraNames = append(extraNames, n)
	}
	sort.Strings(extraNames)
	for _, n := range extraNames {
		id := "Extra:" + n
		if def(id, extra[n]) {
			l := locByID[id]
			var secs []int64
			t := time.Date(1800, 1, 1, 0, 0, 0, 0, time.UTC)
			for i := 0; i < 400; i++ {
				_, end := t.In(l).ZoneBounds()
				if end.IsZero() || end.Year() > 2060 || !end.After(t) {
					break
				}
				secs = append(secs, end.Unix())
				t = end
			}
			lookupAll(id, secs)
		}
	}
	// Malformed variants.
	bad := []tzifSpec{
		{version: '2', zones: []tzifZone{{3600, false, 9}}, abbrs: "ABC\x00"},                                  // abbr index out of range
		{version: '2', tx: []int64{0}, idx: []uint8{3}, zones: []tzifZone{{3600, false, 0}}, abbrs: "ABC\x00"}, // zone index out of range
		{version: 0, zones: []tzifZone{{3600, false, 3}}, abbrs: "ABC"},                                        // abbr at end, no NUL
		{version: '3', zones: []tzifZone{{-1, true, 0}}, abbrs: "\x00"},                                        // empty abbr
		{version: '2', zones: []tzifZone{{math.MaxInt32, false, 0}, {math.MinInt32, true, 0}}, abbrs: "X\x00", tx: []int64{-1 << 40, 1 << 40}, idx: []uint8{1, 0}},
		{version: '2', tx: []int64{5000000000, 100}, idx: []uint8{0, 1}, zones: []tzifZone{{3600, false, 0}, {7200, true, 0}}, abbrs: "UNS\x00"}, // unsorted
		{version: 0, tx: []int64{3000000000, -3000000000}, idx: []uint8{0, 1}, zones: []tzifZone{{3600, false, 0}, {-3600, false, 0}}, abbrs: "W32\x00"},
	}
	for _, s := range bad {
		id := defID()
		data := buildTZif(s)
		if def(id, data) && !cacheDependent(data) {
			lookupAll(id, s.tx)
		}
	}
	// Byte-level mutations of real data.
	for k := 0; k < 150*scale; k++ {
		z := fixtureZones[r.Intn(len(fixtureZones))]
		data := append([]byte(nil), zoneDat[z]...)
		switch k % 5 {
		case 0: // flip a header byte (counts)
			data[20+r.Intn(24)] = byte(r.Intn(256))
		case 1: // truncate
			data = data[:r.Intn(len(data))]
		case 2: // random byte anywhere
			data[r.Intn(len(data))] = byte(r.Intn(256))
		case 3: // replace the footer
			if i := bytes.LastIndexByte(data[:len(data)-1], '\n'); i > 0 {
				data = append(data[:i+1], []byte(advRandExtend(r)+"\n")...)
			}
		default: // version byte
			data[4] = []byte{0, '1', '2', '3', '4'}[r.Intn(5)]
		}
		id := defID()
		if def(id, data) && !cacheDependent(data) {
			var secs []int64
			for i := 0; i < 12; i++ {
				secs = append(secs, r.Int63n(1<<35)-1<<34)
			}
			for _, s := range secs {
				lookupRec(id, s)
			}
		}
	}

	// 6. Date/AddDate/Truncate/Round with extreme arguments.
	ext := []int{math.MinInt64, math.MinInt64 + 1, -1 << 62, -1 << 40, -1, 0, 1, 1 << 40, 1 << 62, math.MaxInt64 - 1, math.MaxInt64}
	for k := 0; k < 1500*scale; k++ {
		id := allIDs[r.Intn(len(allIDs))]
		v := func(small int) int {
			if r.Intn(4) == 0 {
				return ext[r.Intn(len(ext))]
			}
			return r.Intn(2*small+1) - small
		}
		dtRec(id, v(300000), v(30), v(400), v(100), v(200), v(200), v(3e9))
	}
	for k := 0; k < 1000*scale; k++ {
		id := allIDs[r.Intn(len(allIDs))]
		in := pickInst()
		t := time.Unix(in[0], in[1]).In(locByID[id])
		y, m, d := r.Intn(41)-20, r.Intn(61)-30, r.Intn(1001)-500
		if k%7 == 0 {
			y, m, d = ext[r.Intn(len(ext))], ext[r.Intn(len(ext))], ext[r.Intn(len(ext))]
		}
		o.rec(append([]string{"AD", id, i64(in[0]), i64(in[1]), itoa(y), itoa(m), itoa(d)}, timeFields(t.AddDate(y, m, d))...)...)
		var dur int64
		switch k % 4 {
		case 0:
			dur = r.Int63()
		case 1:
			dur = r.Int63n(1e9) + 1
		case 2:
			dur = (r.Int63n(100000) + 1) * 1e9
		default:
			dur = []int64{-1, 0, 1, 2, 3, 500000000, 999999999, 1e9 + 1, 1 << 62, math.MaxInt64, 86400e9 * 7}[r.Intn(11)]
		}
		o.rec(append(append([]string{"TR", id, i64(in[0]), i64(in[1]), i64(dur)}, timeFields(t.Truncate(time.Duration(dur)))...),
			timeFields(t.Round(time.Duration(dur)))...)...)
	}
	// Truncate/Round at the extremes of the representable range.
	// 9223371974719179008 is internal second MinInt64 (the negation in
	// div wraps), 9223371974719179007 internal MaxInt64.
	for _, sec := range []int64{math.MinInt64, math.MinInt64 + 1, math.MinInt64 + 62135596800, math.MinInt64 + 62135596801,
		-62135596800, -62135596801, math.MaxInt64, math.MaxInt64 - 62135596800, math.MaxInt64 - 62135596801,
		9223371974719179008, 9223371974719179009, 9223371974719179007} {
		for _, ns := range []int64{0, 1, 999999999} {
			for _, dur := range []int64{1, 3, 7, 1e9, 2e9, 3e9, 7e9, 1<<62 + 1, math.MaxInt64, 5e8, 1000000007,
				9223372036000000000, 9223372035000000000, 4611686018000000000} {
				t := time.Unix(sec, ns).UTC()
				o.rec(append(append([]string{"TR", "UTC", i64(sec), i64(ns), i64(dur)}, timeFields(t.Truncate(time.Duration(dur)))...),
					timeFields(t.Round(time.Duration(dur)))...)...)
			}
		}
	}

	// 7. ParseDuration fuzz.
	durPieces := []string{"0", "1", "2", "5", "9", "00", "10", "99", "123", ".", ".5", ".05", "-", "+", "ns", "us",
		"µs", "μs", "ms", "s", "m", "h", "d", "x", "\xff", "\xc2", "µ", "9223372036854775807", "9223372036854775808",
		"0000000000000000000000001", "18446744073709551616", "2562047", "0.0000000000000000000001", " ", "e3"}
	for k := 0; k < 3000*scale; k++ {
		var b strings.Builder
		for n := 1 + r.Intn(6); n > 0; n-- {
			b.WriteString(durPieces[r.Intn(len(durPieces))])
		}
		pd(o, b.String())
	}
}
