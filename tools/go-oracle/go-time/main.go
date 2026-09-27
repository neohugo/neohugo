// Command go-time generates test fixtures for the Rust crate crates/go-time
// from the Go time package of the golden toolchain (go1.27.1).
//
// Usage:
//
//	go run ./tools/go-oracle/go-time -out crates/go-time/tests/fixtures
//
// Every fixture is a text file with one record per line and tab-separated
// fields. Fields are escaped: '\\' -> `\\`, TAB -> `\t`, LF -> `\n`,
// CR -> `\r`, other bytes < 0x20 or >= 0x7f -> `\xNN`.
//
// Local is pinned to Asia/Bangkok (from the checked-in TZif file, named
// "Local" like a /etc/localtime-derived Local), so the fixtures do not
// depend on the machine. Only system.txt and localenv.txt depend on the
// system zoneinfo; they record the hashes of the files they used.
package main

import (
	"archive/zip"
	"bufio"
	"bytes"
	"compress/gzip"
	"crypto/sha256"
	"encoding/binary"
	"encoding/hex"
	"flag"
	"fmt"
	"io"
	"math"
	"math/rand"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"time"
)

// Zones checked in as TZif files (from $GOROOT/lib/time/zoneinfo.zip).
var fixtureZones = []string{
	"Africa/Casablanca",
	"Africa/Monrovia",
	"America/Blanc-Sablon",
	"America/Los_Angeles",
	"America/New_York",
	"America/Sao_Paulo",
	"America/St_Johns",
	"Antarctica/Troll",
	"Asia/Baghdad",
	"Asia/Bangkok",
	"Asia/Kathmandu",
	"Asia/Kolkata",
	"Asia/Tehran",
	"Asia/Tokyo",
	"Australia/Lord_Howe",
	"Australia/Sydney",
	"Etc/GMT+5",
	"Etc/GMT-14",
	"Europe/Berlin",
	"Europe/Dublin",
	"Europe/Lisbon",
	"Europe/London",
	"Europe/Moscow",
	"Pacific/Apia",
	"Pacific/Chatham",
	"Pacific/Kiritimati",
}

var (
	outDir  string
	zoneDat = map[string][]byte{}
	locs    []namedLoc
	locByID = map[string]*time.Location{}
)

type namedLoc struct {
	id  string
	loc *time.Location
}

func main() {
	child := flag.String("child", "", "internal: child mode")
	flag.StringVar(&outDir, "out", "crates/go-time/tests/fixtures", "fixture output directory")
	big := flag.String("big", "", "also write a larger corpus into this directory")
	advOnly := flag.String("advonly", "", "write only the adversarial corpus (adv.txt.gz) into this directory")
	advScale := flag.Int("advscale", 20, "scale of the adversarial corpus written by -big/-advonly")
	zoneText := flag.String("zonetext", "", "with -advonly: also write zoneall.txt.gz and the full per-zone texts into this directory")
	flag.Parse()

	if *child != "" {
		runChild(*child)
		return
	}

	if *advOnly != "" {
		// Locations only; the fixture zone files go to the scratch directory.
		outDir = *advOnly
		must(os.MkdirAll(filepath.Join(outDir, "zoneinfo"), 0o755))
		loadFixtureZones()
		setupLocations()
		genAdversarial(outDir, *advScale)
		if *zoneText != "" {
			genZoneAll(outDir, *zoneText)
		}
		return
	}

	must(os.MkdirAll(filepath.Join(outDir, "zoneinfo"), 0o755))
	loadFixtureZones()
	setupLocations()

	genFormat(outDir, 1)
	genParse(outDir, 1)
	genDuration(outDir)
	genCalendar(outDir, 1)
	genBinary(outDir)
	genZones(outDir)
	genTzset(outDir)
	genZip(outDir)
	genSystem(outDir)
	genLocalEnv(outDir)
	genSeeksnack(outDir)
	genAdversarial(outDir, 1)
	genZoneAll(outDir, "")

	if *big != "" {
		must(os.MkdirAll(*big, 0o755))
		genFormat(*big, 20)
		genParse(*big, 20)
		genCalendar(*big, 20)
		genAdversarial(*big, *advScale)
	}
}

func must(err error) {
	if err != nil {
		panic(err)
	}
}

// ---------------------------------------------------------------------------
// Output helpers.

func esc(s string) string {
	var b strings.Builder
	for i := 0; i < len(s); i++ {
		c := s[i]
		switch {
		case c == '\\':
			b.WriteString(`\\`)
		case c == '\t':
			b.WriteString(`\t`)
		case c == '\n':
			b.WriteString(`\n`)
		case c == '\r':
			b.WriteString(`\r`)
		case c < 0x20 || c >= 0x7f:
			fmt.Fprintf(&b, `\x%02x`, c)
		default:
			b.WriteByte(c)
		}
	}
	return b.String()
}

type out struct {
	f  *os.File
	gz *gzip.Writer
	w  *bufio.Writer
	n  int
}

// create opens a fixture; names ending in ".gz" are gzip-compressed
// (deterministic: no name or mtime in the header).
func create(dir, name string) *out {
	f, err := os.Create(filepath.Join(dir, name))
	must(err)
	o := &out{f: f}
	var w io.Writer = f
	if strings.HasSuffix(name, ".gz") {
		o.gz, err = gzip.NewWriterLevel(f, gzip.BestCompression)
		must(err)
		w = o.gz
	}
	o.w = bufio.NewWriter(w)
	return o
}

func (o *out) rec(fields ...string) {
	for i, f := range fields {
		if i > 0 {
			o.w.WriteByte('\t')
		}
		o.w.WriteString(esc(f))
	}
	o.w.WriteByte('\n')
	o.n++
}

func (o *out) close() {
	must(o.w.Flush())
	if o.gz != nil {
		must(o.gz.Close())
	}
	must(o.f.Close())
	fmt.Fprintf(os.Stderr, "%s: %d records\n", o.f.Name(), o.n)
}

func i64(v int64) string { return strconv.FormatInt(v, 10) }
func itoa(v int) string  { return strconv.Itoa(v) }

func locKind(l *time.Location) string {
	switch l {
	case time.UTC:
		return "UTC"
	case time.Local:
		return "Local"
	}
	return "other"
}

// timeFields describes a time: unix, nsec, location kind, location name,
// zone name, zone offset, String().
func timeFields(t time.Time) []string {
	name, off := t.Zone()
	return []string{i64(t.Unix()), itoa(t.Nanosecond()), locKind(t.Location()), t.Location().String(), name, itoa(off), t.String()}
}

// ---------------------------------------------------------------------------
// Locations.

func zoneFile(name string) string {
	return strings.ReplaceAll(name, "/", "__")
}

func loadFixtureZones() {
	zr, err := zip.OpenReader(filepath.Join(runtime.GOROOT(), "lib", "time", "zoneinfo.zip"))
	must(err)
	defer zr.Close()
	want := map[string]bool{}
	for _, z := range fixtureZones {
		want[z] = true
	}
	for _, f := range zr.File {
		if !want[f.Name] {
			continue
		}
		rc, err := f.Open()
		must(err)
		var buf bytes.Buffer
		_, err = buf.ReadFrom(rc)
		must(err)
		rc.Close()
		zoneDat[f.Name] = buf.Bytes()
		must(os.WriteFile(filepath.Join(outDir, "zoneinfo", zoneFile(f.Name)), buf.Bytes(), 0o644))
	}
	for _, z := range fixtureZones {
		if zoneDat[z] == nil {
			panic("missing zone " + z)
		}
	}
}

func setupLocations() {
	local, err := time.LoadLocationFromTZData("Local", zoneDat["Asia/Bangkok"])
	must(err)
	time.Local = local

	add := func(id string, l *time.Location) {
		locs = append(locs, namedLoc{id, l})
		locByID[id] = l
	}
	add("UTC", time.UTC)
	add("Local", time.Local)
	for _, fz := range []struct {
		name string
		off  int
	}{
		{"", 0}, {"", 19800}, {"EST", -18000}, {"", -2670}, {"XYZ", 50400}, {"", -43200},
		{"", 1}, {"", -1}, {"LMT", 3600 + 23*60 + 45}, {"", 99*3600 + 99*60}, {"", 24 * 3600}, {"", -123456789},
	} {
		add(fmt.Sprintf("Fixed:%s:%d", fz.name, fz.off), time.FixedZone(fz.name, fz.off))
	}
	for _, z := range fixtureZones {
		l, err := time.LoadLocationFromTZData(z, zoneDat[z])
		must(err)
		add("Zone:"+z, l)
	}
}

// A handful of locations used for the heavier cross products.
func mainLocIDs() []string {
	return []string{"UTC", "Local", "Fixed::19800", "Fixed:EST:-18000", "Fixed::-2670",
		"Zone:America/New_York", "Zone:Europe/Berlin", "Zone:Australia/Sydney", "Zone:Europe/Dublin", "Zone:Asia/Kolkata"}
}

// ---------------------------------------------------------------------------
// Instants.

func specialInstants() [][2]int64 {
	v := [][2]int64{
		{0, 0}, {-1, 0}, {1, 0}, {0, 1}, {-1, 999999999}, {1233810057, 12345600},
		{-62135596800, 0}, {-62135596801, 999999999}, {-62135596800, 1},
		{-62167219200, 0}, {-62167219201, 0}, // year 0 / -1
		{253402300799, 999999999}, {253402300800, 0},
		{951782400, 0}, {951868800, 0}, // 2000-02-29, 03-01
		{-2203891200, 0}, {-2208988800, 0}, {4102444800, 0}, {13569465600, 0},
		{1661201140, 676836973}, {2147483647, 0}, {2147483648, 0}, {-2147483648, 0},
		{1 << 40, 5}, {-(1 << 40), 5}, {1 << 50, 0}, {-(1 << 50), 0},
		{1 << 62, 0}, {-(1 << 62), 0},
		{math.MaxInt64, 0}, {math.MinInt64, 0}, {math.MaxInt64 - 62135596800, 999999999},
		{math.MinInt64 + 62135596800, 0},
		{9223371966579724800, 0}, {-9223371966579724800, 0},
		{1500000000, 120000000}, {1500000000, 100}, {1500000000, 999999999},
		{1589728109, 238000000}, {1577775981, 671000000}, {1695653100, 876000000}, // seeksnack-like
	}
	return v
}

func randInstants(r *rand.Rand, n int) [][2]int64 {
	var v [][2]int64
	for i := 0; i < n; i++ {
		var sec int64
		switch i % 6 {
		case 0:
			sec = r.Int63n(4102444800)
		case 1:
			sec = r.Int63n(2*253402300800) - 253402300800 - 62167219200
		case 2:
			sec = r.Int63n(1<<45) - 1<<44
		case 3:
			sec = r.Int63() - r.Int63()
		case 4:
			sec = r.Int63n(86400*800) + 946684800 - 86400*400
		default:
			sec = r.Int63n(8000000000) - 4000000000
		}
		var ns int64
		switch r.Intn(4) {
		case 0:
			ns = 0
		case 1:
			ns = int64(r.Intn(1000)) * 1000000
		case 2:
			ns = int64(r.Intn(1000000)) * 1000
		default:
			ns = int64(r.Intn(1000000000))
		}
		v = append(v, [2]int64{sec, ns})
	}
	return v
}

// Calendar dates in UTC around interesting years.
func calendarInstants(r *rand.Rand) [][2]int64 {
	years := []int{-100001, -100000, -99999, -10001, -10000, -9999, -1001, -1000, -999, -401, -400, -399,
		-101, -100, -99, -11, -10, -9, -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 9, 10, 11, 99, 100, 101,
		399, 400, 401, 999, 1000, 1001, 1582, 1600, 1700, 1800, 1883, 1884, 1885, 1899, 1900, 1901,
		1969, 1970, 1971, 1999, 2000, 2001, 2004, 2019, 2020, 2021, 2023, 2024, 2025, 2026, 2037, 2038, 2039,
		2100, 2157, 2158, 2262, 2400, 9998, 9999, 10000, 10001, 99999, 100000, 100001, 292277026596}
	for i := 0; i < 30; i++ {
		years = append(years, r.Intn(20000)-10000)
	}
	var v [][2]int64
	for _, y := range years {
		for _, md := range [][2]int{{1, 1}, {1, 3}, {1, 4}, {2, 28}, {2, 29}, {3, 1}, {6, 30}, {12, 28}, {12, 29}, {12, 31}} {
			t := time.Date(y, time.Month(md[0]), md[1], 12, 34, 56, 0, time.UTC)
			v = append(v, [2]int64{t.Unix(), 0})
			t2 := time.Date(y, time.Month(md[0]), md[1], 0, 0, 0, 0, time.UTC).Add(-time.Nanosecond)
			v = append(v, [2]int64{t2.Unix(), int64(t2.Nanosecond())})
		}
	}
	return v
}

// ---------------------------------------------------------------------------
// Layouts.

var elementLayouts = []string{
	"2006", "06", "Jan", "January", "01", "1", "Mon", "Monday", "2", "_2", "02", "__2", "002",
	"15", "3", "03", "4", "04", "5", "05", "PM", "pm", "MST",
	"Z0700", "Z070000", "Z07", "Z07:00", "Z07:00:00", "-0700", "-070000", "-07", "-07:00", "-07:00:00",
	".0", ".00", ".000", ".000000", ".000000000", ".0000000000", ".00000000000",
	".9", ".99", ".999", ".999999", ".999999999", ".9999999999",
	",0", ",000", ",999", ",999999999", ".09", ".90", "05.0", "05.9", "5.000", "05,000",
	"15:04:05.000000000", "15:04:05.999999999", "15:04:05,999999",
}

var namedLayouts = []string{
	time.Layout, time.ANSIC, time.UnixDate, time.RubyDate, time.RFC822, time.RFC822Z, time.RFC850,
	time.RFC1123, time.RFC1123Z, time.RFC3339, time.RFC3339Nano, time.Kitchen, time.Stamp,
	time.StampMilli, time.StampMicro, time.StampNano, time.DateTime, time.DateOnly, time.TimeOnly,
}

var customLayouts = []string{
	"", "no layout elements here", "Hi Janet, the Month is January", "Jan _2 002 __2 2",
	"2006 6 06 _6 __6 ___6", "Jan January 1 01 _1", "2 02 _2 __2", "Mon Monday", "15 3 03 _3",
	"4 04 _4", "5 05 _5", "3pm", "3PM", "06 01 02", "3:4:5", "2006.01.02", "_2006", "__2006", "___2006",
	"2006-002", "200600201", "200600204", "2006-01-02 002", "15:04_20060102", "Janet", "Month", "Mondays",
	"JAN jan Jan1 January1 Mon02 MSTX MST2", "ChST MeST", "Z", "ZZ07", "-", "--0700", "PMpm", "Pm pM",
	"2006-01-02T15:04:05-07", "2006-01-02T15:04:05Z07", "2006-01-02T15:04:05Z0700",
	"2006-01-02T15:04:05Z070000", "2006-01-02T15:04:05Z07:00:00", "2006-01-02T15:04:05-070000",
	"2006-01-02T15:04:05-07:00:00", "2006-01-02 15:04:05.999999999 -0700 MST",
	"2006-01-02 15:04:05.9999 -0700 MST", "2006-01-02 15:04:05,9999 -0700 MST",
	"Mon Jan _2 15:04:05.000 2006", "Mon Jan _2 15:04:05,000000 2006", "2006.01.02.15.04.05.0",
	"2006.01.02.15.04.05.00", "Mon, 02 Jan 2006 15:04:05 -0700", "2006-01-02T15:04:05-07:00",
	"2006 ", "Jan 2, 2006", "January 2, 2006", "2006-01-02T15:04:05", "2006-01-02T15:04:05-0700",
	"2006-01-02 15:04:05Z0700", "2006-01-02 15:04:05Z07:00", "02 Jan 2006", "2006-01-02 15:04:05 -07:00",
	"2006-01-02 15:04:05 -0700", `"2006-01-02T15:04:05Z07:00"`, "01-02", "03:04PM", "03:04pm",
	"_2 Jan 06 15:04 MST", "Jan _2 002 2006", "01 MST",
	"日付 2006年01月02日 15時04分", "\xff2006\xfe", "\t2006\n01\r",
	"2006-01-02T15:04:05.000Z07:00", "2006-01-02T15:04:05.999Z07:00", "2006-01-02T15:04:05.999999999Z07:00 MST Monday",
	"2006-01-02T15:04:05.999999999Z07:00:00 Mon Monday Jan January 002 __2 _2 3 03 PM pm 06 1 01 2 02 4 04 5 05 MST .000 ,999999",
}

func allLayouts() []string {
	var v []string
	v = append(v, elementLayouts...)
	v = append(v, namedLayouts...)
	v = append(v, customLayouts...)
	return v
}

// ---------------------------------------------------------------------------
// format.txt: F <loc> <unix> <nsec> <layout> <result>
//             S <loc> <unix> <nsec> <String()> <GoString()>

func genFormat(dir string, scale int) {
	o := create(dir, "format.txt.gz")
	defer o.close()
	r := rand.New(rand.NewSource(1))

	const big = "2006-01-02T15:04:05.999999999Z07:00:00 Mon Monday Jan January 002 __2 _2 3 03 PM pm 06 1 01 2 02 4 04 5 05 MST .000 ,999999"

	// Calendar sweep in UTC with one comprehensive layout.
	insts := append(specialInstants(), calendarInstants(r)...)
	insts = append(insts, randInstants(r, 1500*scale)...)
	for _, in := range insts {
		t := time.Unix(in[0], in[1]).UTC()
		o.rec("F", "UTC", i64(in[0]), i64(in[1]), big, t.Format(big))
		o.rec("S", "UTC", i64(in[0]), i64(in[1]), t.String(), t.GoString())
	}

	// Every layout for some instants in the main locations.
	few := append(specialInstants(), randInstants(r, 12*scale)...)
	for _, in := range few {
		for _, id := range mainLocIDs() {
			t := time.Unix(in[0], in[1]).In(locByID[id])
			for _, layout := range allLayouts() {
				o.rec("F", id, i64(in[0]), i64(in[1]), layout, t.Format(layout))
			}
			o.rec("S", id, i64(in[0]), i64(in[1]), t.String(), t.GoString())
		}
	}

	// All locations, a zone-sensitive layout, many instants.
	const zl = "2006-01-02 15:04:05.999999999 MST -07:00:00 Z07 Z0700 -0700"
	many := randInstants(r, 60*scale)
	for _, nl := range locs {
		for _, in := range append(specialInstants(), many...) {
			t := time.Unix(in[0], in[1]).In(nl.loc)
			o.rec("F", nl.id, i64(in[0]), i64(in[1]), zl, t.Format(zl))
			o.rec("F", nl.id, i64(in[0]), i64(in[1]), time.RFC3339Nano, t.Format(time.RFC3339Nano))
			o.rec("S", nl.id, i64(in[0]), i64(in[1]), t.String(), t.GoString())
		}
	}
}

// ---------------------------------------------------------------------------
// parse.txt: P <mode> <loc> <layout> <value> OK <time fields...>
//            P <mode> <loc> <layout> <value> ERR <message>
// mode "Parse" (loc ignored) or "In" (ParseInLocation).

func parseRec(o *out, mode, id, layout, value string) {
	var t time.Time
	var err error
	if mode == "Parse" {
		t, err = time.Parse(layout, value)
	} else {
		t, err = time.ParseInLocation(layout, value, locByID[id])
	}
	if err != nil {
		o.rec("P", mode, id, layout, value, "ERR", err.Error())
		return
	}
	o.rec(append([]string{"P", mode, id, layout, value, "OK"}, timeFields(t)...)...)
}

var goParseTests = [][2]string{
	{time.ANSIC, "Thu Feb  4 21:00:57 2010"},
	{time.UnixDate, "Thu Feb  4 21:00:57 PST 2010"},
	{time.RubyDate, "Thu Feb 04 21:00:57 -0800 2010"},
	{time.RFC850, "Thursday, 04-Feb-10 21:00:57 PST"},
	{time.RFC1123, "Thu, 04 Feb 2010 21:00:57 PST"},
	{time.RFC1123, "Thu, 04 Feb 2010 22:00:57 PDT"},
	{time.RFC1123Z, "Thu, 04 Feb 2010 21:00:57 -0800"},
	{time.RFC3339, "2010-02-04T21:00:57-08:00"},
	{"2006-01-02 15:04:05-07", "2010-02-04 21:00:57-08"},
	{time.ANSIC, "Thu Feb  4 21:00:57.0 2010"},
	{time.UnixDate, "Thu Feb  4 21:00:57.01 PST 2010"},
	{time.RubyDate, "Thu Feb 04 21:00:57.012 -0800 2010"},
	{time.RFC850, "Thursday, 04-Feb-10 21:00:57.0123 PST"},
	{time.RFC1123, "Thu, 04 Feb 2010 21:00:57.01234 PST"},
	{time.RFC1123Z, "Thu, 04 Feb 2010 21:00:57.01234 -0800"},
	{time.RFC3339, "2010-02-04T21:00:57.012345678-08:00"},
	{"2006-01-02 15:04:05", "2010-02-04 21:00:57.0"},
	{time.ANSIC, "Thu Feb 4 21:00:57 2010"},
	{time.ANSIC, "Thu      Feb     4     21:00:57     2010"},
	{time.ANSIC, "THU FEB 4 21:00:57 2010"},
	{time.ANSIC, "thu feb 4 21:00:57 2010"},
	{"Mon Jan _2 15:04:05.000 2006", "Thu Feb  4 21:00:57.012 2010"},
	{"Mon Jan _2 15:04:05.000000 2006", "Thu Feb  4 21:00:57.012345 2010"},
	{"Mon Jan _2 15:04:05.000000000 2006", "Thu Feb  4 21:00:57.012345678 2010"},
	{"Mon Jan _2 15:04:05,000 2006", "Thu Feb  4 21:00:57.012 2010"},
	{"Mon Jan _2 15:04:05,000000 2006", "Thu Feb  4 21:00:57.012345 2010"},
	{"Mon Jan _2 15:04:05,000000000 2006", "Thu Feb  4 21:00:57.012345678 2010"},
	{"2006.01.02.15.04.05.0", "2010.02.04.21.00.57.0"},
	{"2006.01.02.15.04.05.00", "2010.02.04.21.00.57.01"},
	{"Hi Janet, the Month is January: Jan _2 15:04:05 2006", "Hi Janet, the Month is February: Feb  4 21:00:57 2010"},
	{time.UnixDate, "Fri Feb  5 05:00:57 GMT-8 2010"},
	{"2006-01-02 15:04:05.9999 -0700 MST", "2010-02-04 21:00:57 -0800 PST"},
	{"2006-01-02 15:04:05.999999999 -0700 MST", "2010-02-04 21:00:57 -0800 PST"},
	{"2006-01-02 15:04:05.9999 -0700 MST", "2010-02-04 21:00:57.0123 -0800 PST"},
	{"2006-01-02 15:04:05.999999999 -0700 MST", "2010-02-04 21:00:57.0123 -0800 PST"},
	{"2006-01-02 15:04:05.9999 -0700 MST", "2010-02-04 21:00:57.012345678 -0800 PST"},
	{"2006-01-02 15:04:05.999999999 -0700 MST", "2010-02-04 21:00:57.012345678 -0800 PST"},
	{"2006-01-02 15:04:05,9999 -0700 MST", "2010-02-04 21:00:57 -0800 PST"},
	{"2006-01-02 15:04:05,999999999 -0700 MST", "2010-02-04 21:00:57 -0800 PST"},
	{"2006-01-02 15:04:05,9999 -0700 MST", "2010-02-04 21:00:57.0123 -0800 PST"},
	{"2006-01-02 15:04:05,999999999 -0700 MST", "2010-02-04 21:00:57.0123 -0800 PST"},
	{"2006-01-02 15:04:05,9999 -0700 MST", "2010-02-04 21:00:57.012345678 -0800 PST"},
	{"2006-01-02 15:04:05,999999999 -0700 MST", "2010-02-04 21:00:57.012345678 -0800 PST"},
	{time.StampNano, "Feb  4 21:00:57.012345678"},
	{"Jan _2 15:04:05.999", "Feb  4 21:00:57.012300000"},
	{"Jan _2 15:04:05.999", "Feb  4 21:00:57.012345678"},
	{"Jan _2 15:04:05.999999999", "Feb  4 21:00:57.0123"},
	{"Jan _2 15:04:05.999999999", "Feb  4 21:00:57.012345678"},
	{"2006-01-02 002 15:04:05", "2010-02-04 035 21:00:57"},
	{"2006-01 002 15:04:05", "2010-02 035 21:00:57"},
	{"2006-002 15:04:05", "2010-035 21:00:57"},
	{"200600201 15:04:05", "201003502 21:00:57"},
	{"200600204 15:04:05", "201003504 21:00:57"},
	{"2006-01-02T15:04:05Z07", "2010-02-04T21:00:57Z"},
	{"2006-01-02T15:04:05Z07", "2010-02-04T21:00:57+08"},
	{"2006-01-02T15:04:05Z07", "2010-02-04T21:00:57-08"},
	{"2006-01-02T15:04:05Z0700", "2010-02-04T21:00:57Z"},
	{"2006-01-02T15:04:05Z0700", "2010-02-04T21:00:57+0800"},
	{"2006-01-02T15:04:05Z0700", "2010-02-04T21:00:57-0800"},
	{"2006-01-02T15:04:05Z07:00", "2010-02-04T21:00:57Z"},
	{"2006-01-02T15:04:05Z07:00", "2010-02-04T21:00:57+08:00"},
	{"2006-01-02T15:04:05Z07:00", "2010-02-04T21:00:57-08:00"},
	{"2006-01-02T15:04:05Z070000", "2010-02-04T21:00:57Z"},
	{"2006-01-02T15:04:05Z070000", "2010-02-04T21:00:57+080000"},
	{"2006-01-02T15:04:05Z070000", "2010-02-04T21:00:57-080000"},
	{"2006-01-02T15:04:05Z07:00:00", "2010-02-04T21:00:57Z"},
	{"2006-01-02T15:04:05Z07:00:00", "2010-02-04T21:00:57+08:00:00"},
	{"2006-01-02T15:04:05Z07:00:00", "2010-02-04T21:00:57-08:00:00"},
	{time.RubyDate, "Thu Feb 04 21:00:57 -0000 2010"},
	{time.RubyDate, "Thu Feb 04 21:00:57 +0000 2010"},
	{time.RubyDate, "Thu Feb 04 21:00:57 +1130 2010"},
	{time.RubyDate, "Thu Feb 02 16:10:03 -0500 2006"},
	{time.RubyDate, "Mon Jan 02 15:04:05 +0123 2006"},
	// parse error tests
	{time.ANSIC, "Feb  4 21:00:60 2010"},
	{time.ANSIC, "Thu Feb  4 21:00:57 @2010"},
	{time.ANSIC, "Thu Feb  4 21:00:60 2010"},
	{time.ANSIC, "Thu Feb  4 21:61:57 2010"},
	{time.ANSIC, "Thu Feb  4 24:00:60 2010"},
	{"Mon Jan _2 15:04:05.000 2006", "Thu Feb  4 23:00:59x01 2010"},
	{"Mon Jan _2 15:04:05.000 2006", "Thu Feb  4 23:00:59.xxx 2010"},
	{"Mon Jan _2 15:04:05.000 2006", "Thu Feb  4 23:00:59.-123 2010"},
	{time.StampNano, "Dec  7 11:22:01.000000"},
	{time.StampNano, "Dec  7 11:22:01.0000000000"},
	{time.RFC3339, "2006-01-02T15:04:05Z07:00"},
	{time.RFC3339, "2006-01-02T15:04_abc"},
	{time.RFC3339, "2006-01-02T15:04:05_abc"},
	{time.RFC3339, "2006-01-02T15:04:05Z_abc"},
	{time.RFC3339, "2010-02-04T21:00:67.012345678-08:00"},
	{time.RFC3339, "0000-01-01T00:00:.0+00:00"},
	{"_2 Jan 06 15:04 MST", "4 --- 00 00:00 GMT"},
	{"_2 January 06 15:04 MST", "4 --- 00 00:00 GMT"},
	{"Jan _2 002 2006", "Feb  4 034 2006"},
	{"Jan _2 002 2006", "Feb  4 004 2006"},
	{`"2006-01-02T15:04:05Z07:00"`, "0"},
	{time.RFC3339, "\""},
	{time.RFC3339, "0000-01-01T00:00:00+00:+0"},
	{time.RFC3339, "0000-01-01T00:00:00+-0:00"},
	{"2006-01-02", "22-10-25"},
	{"06-01-02", "a2-10-25"},
	{"03:04PM", "12:03pM"},
	{"03:04pm", "12:03pM"},
	{"-07", "-25"},
	{"-07:00", "+25:00"},
	{"-07:00", "-23:61"},
	{"-07:00:00", "+23:59:61"},
	{"Z07", "-25"},
	{"Z07:00", "+25:00"},
	{"Z07:00", "-23:61"},
	{"Z07:00:00", "+23:59:61"},
	{"2006-01-02T15:04:05-070000", "1871-01-01T05:33:02-003408"},
	{"2006-01-02T15:04:05-07:00:00", "1871-01-01T05:33:02-00:34:08"},
	{"2006-01-02T15:04:05-070000", "1871-01-01T05:33:02+003408"},
	{"2006-01-02T15:04:05-07:00:00", "1871-01-01T05:33:02+00:34:08"},
	{"2006-01-02T15:04:05Z070000", "1871-01-01T05:33:02-003408"},
	{"2006-01-02T15:04:05Z07:00:00", "1871-01-01T05:33:02+00:34:08"},
	{"2006-01-02T15:04:05-07", "1871-01-01T05:33:02+01"},
	{"2006-01-02T15:04:05-07", "1871-01-01T05:33:02-02"},
	{"2006-01-02T15:04:05Z07", "1871-01-01T05:33:02-02"},
	{"15:04_20060102", "14:38_20150618"},
	{"01 MST", "0 MST"},
	{"01 MST", "1 MST"},
	{time.RFC850, "Thursday, 04-Feb-1 21:00:57 PST"},
	{"01-02", "00-01"},
	{"01-02", "13-01"},
	{"01-02", "01-01"},
	{"3:04PM", "12:00PM"},
	{"03:04PM", "12:00PM"},
	{"3:04PM", "12:00AM"},
	{"03:04PM", "12:00AM"},
	{"Jan 02 2006 MST", "Feb 01 2013 AST"},
	{"2006-01-02 15:04:05 MST", "2013-02-01 00:00:00 +07"},
	{"2006-01-02 15:04:05 MST", "2013-02-01 00:00:00 -03"},
	{"2006-01-02 15:04:05 MST", "2013-02-01 00:00:00 GMT+7"},
	{"2006-01-02 15:04:05 MST", "2013-02-01 00:00:00 GMT-12"},
	{"2006-01-02 15:04:05 MST", "2013-02-01 00:00:00 GMT+24"},
	{"2006-01-02 15:04:05 MST", "2013-02-01 00:00:00 ICT"},
	{"2006-01-02 15:04:05 MST", "2013-02-01 00:00:00 BMT"},
	{"2006-01-02 15:04:05 MST", "1900-02-01 00:00:00 BMT"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 EDT"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 CEST"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 ChST"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 WITA"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 ESAST"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 ESASTT"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 MSDY"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 UTC"},
	{"2006-01-02 15:04:05 MST", "2013-07-01 00:00:00 utc"},
	{"2006-01-02 15:04:05 -0700 MST", "2013-07-01 00:00:00 +0700 +07"},
	{"2006-01-02 15:04:05 -0700 MST", "2013-07-01 00:00:00 +0700 ICT"},
	{"2006-01-02 15:04:05 -0700 MST", "2013-07-01 00:00:00 +0000 UTC"},
	{"2006-01-02 15:04:05 -0700 MST", "1900-07-01 00:00:00 +0642 LMT"},
	{"2006-01-02 15:04:05.999999999 -0700 MST", "1880-07-01 00:00:00.5 +0642 LMT"},
	{"2006-01-02 15:04:05.999999999 -0700 MST", "2020-08-14 14:59:12.921 +0000 UTC"},
	{"2006-01-02 15:04:05.999999999 -0700 MST", "2021-06-09 04:46:11 +0700 +07"},
}

func genParse(dir string, scale int) {
	o := create(dir, "parse.txt.gz")
	defer o.close()
	r := rand.New(rand.NewSource(2))

	for _, tc := range goParseTests {
		parseRec(o, "Parse", "", tc[0], tc[1])
		for _, id := range mainLocIDs() {
			parseRec(o, "In", id, tc[0], tc[1])
		}
		parseRec(o, "In", "Zone:Asia/Baghdad", tc[0], tc[1])
		parseRec(o, "In", "Zone:America/Blanc-Sablon", tc[0], tc[1])
	}
	// parseTimeZone cases through a "MST" layout.
	for _, v := range []string{"gmt hi there", "GMT hi there", "GMT+12 hi there", "GMT+00 hi there", "GMT+", "GMT+3",
		"GMT+a", "GMT+3a", "GMT-5 hi there", "GMT-51 hi there", "ChST hi there", "MeST hi there", "MSDx", "MSDY",
		"ESAST hi", "ESASTT hi", "ESATY hi", "WITA hi", "+03 hi", "-04 hi", "+00", "-11", "-12", "-23", "-24", "+13",
		"+14", "+23", "+24", "GMT+23", "GMT-99999999999999999999", "+99999999999999999999", "GMT", "AB", "ABCD", "ABCDE"} {
		parseRec(o, "Parse", "", "MST", v)
		parseRec(o, "Parse", "", "MST rest", v)
		parseRec(o, "In", "Zone:Europe/Moscow", "MST", v)
	}
	// Day of year 2020 and 2021.
	for i := 0; i <= 367; i++ {
		parseRec(o, "Parse", "", "2006-002", fmt.Sprintf("2020-%03d", i))
		parseRec(o, "Parse", "", "2006-__2", fmt.Sprintf("2021-%3d", i))
		parseRec(o, "Parse", "", "2006-01-02 002", fmt.Sprintf("2021-03-01 %03d", i))
	}
	// Long fractional digits.
	for _, v := range []string{"2021-09-29T16:04:33.000000000Z", "2021-09-29T16:04:33.000000001Z",
		"2021-09-29T16:04:33.100000000Z", "2021-09-29T16:04:33.999999999Z", "2021-09-29T16:04:33.0000000001Z",
		"2021-09-29T16:04:33.1000000009Z", "2021-09-29T16:04:33.0123456789Z", "2021-09-29T16:04:33.00123456789Z",
		"2021-09-29T16:04:33.9999999999999999Z", "2021-09-29T16:04:33,5Z", "2021-09-29T16:04:33.Z",
		"2000-01-01T1:12:34Z", "2000-01-01T00:00:00,000Z", "2000-01-01T00:00:00+24:00", "2000-01-01T00:00:00+00:60",
		"2000-01-01T00:00:00+123:45", "2000-02-30T00:00:00Z", "2000-02-29T00:00:00Z", "1900-02-29T00:00:00Z",
		"2021-06-09T04:46:11+07:00", "2021-06-09T04:46:11+07:30", "1900-06-09T04:46:11+07:00",
		"1900-06-09T04:46:11+06:42", "2022-01-07T07:35:39Z", "2026-09-27T12:00:00Z", "9999-12-31T23:59:59.999999999Z",
		"0000-01-01T00:00:00Z", "2006-01-02T15:04:05", "2006-01-02T15:04:05 Z", "2006-01-02T15:04:05z",
		"2006-01-02T15:04:05-00:00", "2006-01-02T15:04:05+00:00", "+2006-01-02T15:04:05Z", "2006-1-02T15:04:05Z"} {
		parseRec(o, "Parse", "", time.RFC3339, v)
		parseRec(o, "Parse", "", time.RFC3339Nano, v)
		for _, id := range mainLocIDs() {
			parseRec(o, "In", id, time.RFC3339, v)
		}
	}

	// Round trips and mutations.
	insts := append(specialInstants(), randInstants(r, 25*scale)...)
	layouts := allLayouts()
	mutate := func(s string) string {
		b := []byte(s)
		switch r.Intn(7) {
		case 0:
			if len(b) > 0 {
				i := r.Intn(len(b))
				b = append(b[:i], b[i+1:]...)
			}
		case 1:
			if len(b) > 0 {
				const repl = "0123456789 :-+.,ZTAPMJFabc\xff"
				b[r.Intn(len(b))] = repl[r.Intn(len(repl))]
			}
		case 2:
			i := r.Intn(len(b) + 1)
			const ins = "0123456789 :-+.,ZTAPM"
			c := ins[r.Intn(len(ins))]
			b = append(b[:i], append([]byte{c}, b[i:]...)...)
		case 3:
			if len(b) > 0 {
				b = b[:r.Intn(len(b))]
			}
		case 4:
			const app = "0123456789 x"
			b = append(b, app[r.Intn(len(app))])
		case 5:
			// swap digits
			for i := range b {
				if b[i] >= '0' && b[i] <= '9' && r.Intn(3) == 0 {
					b[i] = byte('0' + r.Intn(10))
				}
			}
		default:
			if len(b) > 1 {
				i := r.Intn(len(b) - 1)
				b[i], b[i+1] = b[i+1], b[i]
			}
		}
		return string(b)
	}
	for k, in := range insts {
		for li, layout := range layouts {
			ids := mainLocIDs()
			id := ids[(k+li)%len(ids)]
			t := time.Unix(in[0], in[1]).In(locByID[id])
			s := t.Format(layout)
			parseRec(o, "Parse", "", layout, s)
			parseRec(o, "In", id, layout, s)
			for m := 0; m < 3; m++ {
				parseRec(o, "Parse", "", layout, mutate(s))
			}
		}
	}
}

// ---------------------------------------------------------------------------
// duration.txt:
//   D <int64> <String> <Hours bits> <Minutes bits> <Seconds bits> <ms> <us> <Abs>
//   DR <int64> <m> <Round> <Truncate>
//   PD <input> OK <int64> | ERR <message>

func genDuration(dir string) {
	o := create(dir, "duration.txt.gz")
	defer o.close()
	r := rand.New(rand.NewSource(3))
	ds := []int64{0, 1, -1, 999, 1000, 1001, 1100, 999999, 1000000, 2200000, 999999999, 1000000000, 3300000000,
		245000000000, 245001000000, 18367001000000, 480000000001, math.MaxInt64, math.MinInt64, math.MinInt64 + 1,
		3600000000000, 5400000000000, 86400000000000, 1 << 53, 1<<53 + 1}
	for i := 0; i < 2000; i++ {
		var d int64
		switch i % 5 {
		case 0:
			d = r.Int63() - r.Int63()
		case 1:
			d = r.Int63n(2000000) - 1000000
		case 2:
			d = r.Int63n(2000000000000) - 1000000000000
		case 3:
			d = int64(r.Intn(100000)) * 1000000
		default:
			d = r.Int63n(1 << 45)
		}
		ds = append(ds, d)
	}
	bits := func(f float64) string { return strconv.FormatUint(math.Float64bits(f), 16) }
	for _, v := range ds {
		d := time.Duration(v)
		o.rec("D", i64(v), d.String(), bits(d.Hours()), bits(d.Minutes()), bits(d.Seconds()),
			i64(d.Milliseconds()), i64(d.Microseconds()), i64(int64(d.Abs())))
		for _, m := range []int64{0, -1, 1, 2, 3, 7, 1000, 1500, 1000000, 1000000000, 60000000000, 3600000000000,
			5400000000000, math.MaxInt64, 1 << 62} {
			o.rec("DR", i64(v), i64(m), i64(int64(d.Round(time.Duration(m)))), i64(int64(d.Truncate(time.Duration(m)))))
		}
		pd(o, d.String())
	}
	for _, s := range []string{"0", "5s", "30s", "1478s", "-5s", "+5s", "-0", "+0", "5.0s", "5.6s", "5.s", ".5s",
		"1.0s", "1.00s", "1.004s", "1.0040s", "100.00100s", "10ns", "11us", "12µs", "12μs", "13ms", "14s", "15m",
		"16h", "3h30m", "10.5s4m", "-2m3.4s", "1h2m3s4ms5us6ns", "39h9m14.425s", "52763797000ns",
		"0.3333333333333333333h", "9007199254740993ns", "9223372036854775807ns", "9223372036854775.807us",
		"9223372036s854ms775us807ns", "-9223372036854775808ns", "-9223372036854775.808us",
		"-9223372036s854ms775us808ns", "-2562047h47m16.854775808s", "0.100000000000000000000h",
		"0.830103483285477580700h", "", "3", "-", "s", ".", "-.", ".s", "+.s", "1d", "\x85\x85", "\xffff",
		"hello \xffff world", "�", "� hello � world", "9223372036854775810ns",
		"9223372036854775808ns", "-9223372036854775809ns", "9223372036854776us", "3000000h",
		"9223372036854775.808us", "9223372036854ms775us808ns", "1.5h", "1.5.5h", "1h1", "1H", "1µ", "1 s",
		" 1s", "1s ", "0.0000000001s", "0.9999999999s", "1.000000000000000000000000001h", "00000000000000000000001s",
		"2562047.788015215h", "2562048h", "106751.99116730063d", "1e3s", "0x10s", "-+1s", "+-1s", "1ms-1s"} {
		pd(o, s)
	}
}

func pd(o *out, s string) {
	d, err := time.ParseDuration(s)
	if err != nil {
		o.rec("PD", s, "ERR", err.Error())
		return
	}
	o.rec("PD", s, "OK", i64(int64(d)))
}

// ---------------------------------------------------------------------------
// calendar.txt:
//   C <loc> <unix> <nsec> <y> <m> <d> <h> <mi> <s> <yday> <wday> <isoy> <isow>
//     <unixmilli> <unixmicro> <unixnano> <iszero> <isdst> <zbstart> <zbend> <Month.String> <Weekday.String>
//   AD <loc> <unix> <nsec> <years> <months> <days> <time fields>
//   AS <loc> <unix> <nsec> <dur> <time fields of Add>
//   SUB <unix1> <nsec1> <unix2> <nsec2> <dur> <compare> <before> <after> <equal>
//   TR <loc> <unix> <nsec> <dur> <Truncate time fields> <Round time fields>
//   DT <loc> <y> <m> <d> <h> <mi> <s> <ns> <time fields>
//   U <sec> <nsec> <time fields>  (time.Unix), UM <msec> ..., UU <usec> ...
//   MS <int> <Month.String> <Weekday.String>

func genCalendar(dir string, scale int) {
	o := create(dir, "calendar.txt.gz")
	defer o.close()
	r := rand.New(rand.NewSource(4))

	insts := append(specialInstants(), calendarInstants(r)...)
	insts = append(insts, randInstants(r, 300*scale)...)
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
	for k, in := range insts {
		ids := mainLocIDs()
		for _, id := range []string{"UTC", ids[k%len(ids)]} {
			t := time.Unix(in[0], in[1]).In(locByID[id])
			y, m, d := t.Date()
			h, mi, s := t.Clock()
			iy, iw := t.ISOWeek()
			st, en := t.ZoneBounds()
			o.rec("C", id, i64(in[0]), i64(in[1]), itoa(y), itoa(int(m)), itoa(d), itoa(h), itoa(mi), itoa(s),
				itoa(t.YearDay()), itoa(int(t.Weekday())), itoa(iy), itoa(iw), i64(t.UnixMilli()), i64(t.UnixMicro()),
				i64(t.UnixNano()), b2s(t.IsZero()), b2s(t.IsDST()), zb(st), zb(en), t.Month().String(), t.Weekday().String())
		}
	}
	// AddDate.
	for k, in := range insts[:len(insts)/2] {
		ids := mainLocIDs()
		id := ids[k%len(ids)]
		t := time.Unix(in[0], in[1]).In(locByID[id])
		for j := 0; j < 3; j++ {
			var y, m, d int
			switch j {
			case 0:
				y, m, d = r.Intn(21)-10, r.Intn(41)-20, r.Intn(801)-400
			case 1:
				y, m, d = 0, 1, 0
			default:
				y, m, d = r.Intn(2000001)-1000000, r.Intn(200001)-100000, r.Intn(20000001)-10000000
			}
			o.rec(append([]string{"AD", id, i64(in[0]), i64(in[1]), itoa(y), itoa(m), itoa(d)}, timeFields(t.AddDate(y, m, d))...)...)
		}
	}
	// Add, Sub, comparisons.
	durs := []int64{0, 1, -1, 999999999, 1000000000, -1000000000, 1500000000, 3600000000000, math.MaxInt64, math.MinInt64}
	for k, in := range insts {
		ids := mainLocIDs()
		id := ids[k%len(ids)]
		t := time.Unix(in[0], in[1]).In(locByID[id])
		d := durs[k%len(durs)]
		if k%3 == 0 {
			d = r.Int63() - r.Int63()
		}
		o.rec(append([]string{"AS", id, i64(in[0]), i64(in[1]), i64(d)}, timeFields(t.Add(time.Duration(d)))...)...)
		u := insts[(k*7+3)%len(insts)]
		if k%4 == 0 {
			u = [2]int64{in[0] + int64(r.Intn(3)) - 1, int64(r.Intn(1000000000))}
		}
		tu := time.Unix(u[0], u[1])
		o.rec("SUB", i64(in[0]), i64(in[1]), i64(u[0]), i64(u[1]), i64(int64(t.Sub(tu))), itoa(t.Compare(tu)),
			b2s(t.Before(tu)), b2s(t.After(tu)), b2s(t.Equal(tu)))
	}
	// Truncate / Round.
	tdurs := []int64{0, -5, 1, 2, 3, 7, 1000, 1500, 250000000, 1000000000, 7000000000, 60000000000, 3600000000000,
		5400000000000, 86400000000000, 90000000000000, 1 << 40, math.MaxInt64, 3*1000000000 + 1}
	for k, in := range insts {
		ids := mainLocIDs()
		id := ids[k%len(ids)]
		t := time.Unix(in[0], in[1]).In(locByID[id])
		for j := 0; j < 3; j++ {
			d := tdurs[(k*3+j)%len(tdurs)]
			o.rec(append(append([]string{"TR", id, i64(in[0]), i64(in[1]), i64(d)}, timeFields(t.Truncate(time.Duration(d)))...),
				timeFields(t.Round(time.Duration(d)))...)...)
		}
	}
	// Date() with normalisation and zone transitions.
	dt := func(id string, y, m, d, h, mi, s, ns int) {
		t := time.Date(y, time.Month(m), d, h, mi, s, ns, locByID[id])
		o.rec(append([]string{"DT", id, itoa(y), itoa(m), itoa(d), itoa(h), itoa(mi), itoa(s), itoa(ns)}, timeFields(t)...)...)
	}
	for _, c := range [][7]int{
		{2011, 11, 6, 1, 0, 0, 0}, {2011, 11, 6, 1, 59, 59, 0}, {2011, 11, 6, 2, 0, 0, 0},
		{2011, 3, 13, 1, 0, 0, 0}, {2011, 3, 13, 1, 59, 59, 0}, {2011, 3, 13, 3, 0, 0, 0}, {2011, 3, 13, 2, 30, 0, 0},
		{2012, 12, 24, 0, 0, 0, 0}, {2011, 11, 18, 7, 56, 35, 0}, {2011, 11, 19, -17, 56, 35, 0},
		{2011, 11, 17, 31, 56, 35, 0}, {2011, 11, 18, 6, 116, 35, 0}, {2011, 10, 49, 7, 56, 35, 0},
		{2011, 11, 18, 7, 55, 95, 0}, {2011, 11, 18, 7, 56, 34, 1e9}, {2011, 12, -12, 7, 56, 35, 0},
		{2012, 1, -43, 7, 56, 35, 0}, {2012, -1, 18, 7, 56, 35, 0}, {2010, 23, 18, 7, 56, 35, 0},
		{1970, 1, 15297, 7, 56, 35, 0}, {1970, 1, -25508, 0, 0, 0, 0},
		{2024, 3, 31, 2, 30, 0, 0}, {2024, 10, 27, 2, 30, 0, 0}, {2024, 3, 10, 2, 30, 0, 0}, {2024, 11, 3, 1, 30, 0, 0},
		{2024, 4, 7, 2, 30, 0, 0}, {2024, 10, 6, 2, 30, 0, 0}, {1883, 11, 18, 12, 3, 58, 0}, {1883, 11, 18, 11, 59, 59, 0},
		{0, 1, 1, 0, 0, 0, 0}, {1, 1, 1, 0, 0, 0, 0}, {-1, 12, 31, 23, 59, 59, 999999999}, {10000, 1, 1, 0, 0, 0, 0},
		{2000, 2, 29, 24, 0, 0, 0}, {1900, 2, 29, 0, 0, 0, 0}, {2000, 0, 0, 0, 0, 0, -1},
	} {
		for _, id := range append(mainLocIDs(), "Zone:America/Los_Angeles", "Zone:Antarctica/Troll", "Zone:Australia/Lord_Howe",
			"Zone:Pacific/Apia", "Zone:America/Sao_Paulo", "Zone:Africa/Casablanca", "Zone:America/St_Johns") {
			dt(id, c[0], c[1], c[2], c[3], c[4], c[5], c[6])
		}
	}
	for i := 0; i < 1500*scale; i++ {
		id := locs[r.Intn(len(locs))].id
		var y, m, d, h, mi, s, ns int
		switch i % 3 {
		case 0:
			y, m, d, h, mi, s, ns = r.Intn(400)+1800, r.Intn(12)+1, r.Intn(31)+1, r.Intn(24), r.Intn(60), r.Intn(60), r.Intn(1e9)
		case 1:
			y, m, d, h, mi, s, ns = r.Intn(20000)-10000, r.Intn(61)-30, r.Intn(201)-100, r.Intn(101)-50, r.Intn(201)-100, r.Intn(201)-100, r.Intn(4e9)-2e9
		default:
			y, m, d, h, mi, s, ns = r.Intn(1e9)-5e8, r.Intn(1e6)-5e5, r.Intn(1e8)-5e7, r.Intn(1e7), r.Intn(1e9), r.Intn(2e9)-1e9, r.Int()
		}
		dt(id, y, m, d, h, mi, s, ns)
	}
	// time.Unix / UnixMilli / UnixMicro normalisation.
	for _, v := range [][2]int64{{0, 0}, {0, -1}, {0, 1e9}, {0, -1e9}, {5, -1e9 - 1}, {math.MaxInt64, 1e9}, {math.MinInt64, -1},
		{1, math.MaxInt64}, {1, math.MinInt64}, {-62135596800, 0}, {1e12, 999999999}} {
		o.rec(append([]string{"U", i64(v[0]), i64(v[1])}, timeFields(time.Unix(v[0], v[1]))...)...)
	}
	for _, v := range []int64{0, 1, -1, 999, -999, 1001, -1001, math.MaxInt64, math.MinInt64, 1700000000123} {
		o.rec(append([]string{"UM", i64(v)}, timeFields(time.UnixMilli(v))...)...)
		o.rec(append([]string{"UU", i64(v)}, timeFields(time.UnixMicro(v))...)...)
	}
	for _, v := range []int{-2, -1, 0, 1, 6, 7, 11, 12, 13, 100, math.MinInt64, math.MaxInt64} {
		o.rec("MS", itoa(v), time.Month(v).String(), time.Weekday(v).String())
	}
}

// ---------------------------------------------------------------------------
// binary.txt:
//   MB <loc> <unix> <nsec> <hex|ERR:msg> <json|ERR:msg> <text|ERR:msg>
//   UB <hex> OK <time fields> | ERR <msg>
//   UJ <input> OK <time fields> | ERR <msg>   (UnmarshalJSON on a zero Time)
//   UT <input> OK <time fields> | ERR <msg>

func genBinary(dir string) {
	o := create(dir, "binary.txt.gz")
	defer o.close()
	r := rand.New(rand.NewSource(5))
	insts := append(specialInstants(), randInstants(r, 150)...)
	var blobs [][]byte
	var jsons []string
	for _, nl := range locs {
		for _, in := range insts {
			t := time.Unix(in[0], in[1]).In(nl.loc)
			mb, err := t.MarshalBinary()
			mbs := hex.EncodeToString(mb)
			if err != nil {
				mbs = "ERR:" + err.Error()
			} else if len(blobs) < 4000 {
				blobs = append(blobs, mb)
			}
			js, err := t.MarshalJSON()
			jss := string(js)
			if err != nil {
				jss = "ERR:" + err.Error()
			} else {
				jsons = append(jsons, jss)
			}
			tx, err := t.MarshalText()
			txs := string(tx)
			if err != nil {
				txs = "ERR:" + err.Error()
			}
			o.rec("MB", nl.id, i64(in[0]), i64(in[1]), mbs, jss, txs)
		}
	}
	ub := func(b []byte) {
		var t time.Time
		if err := t.UnmarshalBinary(b); err != nil {
			o.rec("UB", hex.EncodeToString(b), "ERR", err.Error())
			return
		}
		o.rec(append([]string{"UB", hex.EncodeToString(b), "OK"}, timeFields(t)...)...)
	}
	for i, b := range blobs {
		if i%5 == 0 {
			ub(b)
		}
	}
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
	ub(nil)
	ub([]byte{3})
	ub([]byte{1, 2, 3})
	ub(mk(1, 63082281600, 0, -1, 0))
	ub(mk(1, 63082281600, 0, 420, 0))
	ub(mk(1, 63082281600, 0, 0, 0))
	ub(mk(2, 63082281600, 0, 0, 0))
	ub(mk(2, 63082281600, 0, -45, -30))
	ub(mk(2, 63082281600, 0, 45, 30))
	ub(mk(2, 59000000000, 0, 401, 56))
	ub(mk(1, 0, 0, -1, 0))
	ub(mk(1, 63082281600, 999999999, 1, 0))
	ub(mk(1, 63082281600, 1000000000, 1, 0))
	ub(mk(1, 63082281600, 1<<30, 1, 0))
	ub(mk(1, 63082281600, -1, -1, 0))
	ub(mk(1, 63082281600, math.MinInt32, 420, 0))
	ub(mk(1, 63082281600, -500000000, 0, 0))
	ub(mk(1, math.MaxInt64, 0, -1, 0))
	ub(mk(1, math.MinInt64, 0, 32767, 0))
	ub(mk(1, 63082281600, 0, -32768, 0))
	ub(append(mk(1, 63082281600, 0, -1, 0), 0))
	ub(mk(2, 63082281600, 0, -1, 0)[:15])
	for i := 0; i < 300; i++ {
		v := byte(1 + r.Intn(2))
		ub(mk(v, r.Int63()-r.Int63(), int32(r.Uint32()), int16(r.Intn(65536)-32768), int8(r.Intn(256)-128)))
	}
	uj := func(s string) {
		var t time.Time
		if err := t.UnmarshalJSON([]byte(s)); err != nil {
			o.rec(append([]string{"UJ", s, "ERR", err.Error()}, timeFields(t)...)...)
			return
		}
		o.rec(append([]string{"UJ", s, "OK"}, timeFields(t)...)...)
	}
	ut := func(s string) {
		var t time.Time
		if err := t.UnmarshalText([]byte(s)); err != nil {
			o.rec(append([]string{"UT", s, "ERR", err.Error()}, timeFields(t)...)...)
			return
		}
		o.rec(append([]string{"UT", s, "OK"}, timeFields(t)...)...)
	}
	for i, j := range jsons {
		if i%7 == 0 {
			uj(j)
			ut(strings.Trim(j, `"`))
		}
	}
	for _, s := range []string{"null", `{}`, `[]`, `""`, `"`, ``, `"2000-01-01T1:12:34Z"`, `"2000-01-01T00:00:00,000Z"`,
		`"2000-01-01T00:00:00+24:00"`, `"2000-01-01T00:00:00+00:60"`, `"2000-01-01T00:00:00+123:45"`,
		`"9999-04-12T23:20:50.52Z"`, `"1996-12-19T16:39:57-08:00"`, `"0000-01-01T00:00:00.000000001+00:01"`,
		`"2020-01-01T00:00:00+23:59"`, `"2020-01-01T00:00:00+07:00"`, `"2020-01-01T00:00:00 +07:00"`, `"x"`, `"A"`} {
		uj(s)
		ut(strings.Trim(s, `"`))
	}
}

// ---------------------------------------------------------------------------
// zones.txt:
//   L <zone> <sec> <name> <offset> <isdst> <zbstart> <zbend>
//   E <label> <hex of TZif data> <ERR msg | OK name>

func genZones(dir string) {
	o := create(dir, "zones.txt.gz")
	defer o.close()
	r := rand.New(rand.NewSource(6))
	zb := func(t time.Time) string {
		if t.IsZero() {
			return "Z"
		}
		return i64(t.Unix())
	}
	for _, z := range fixtureZones {
		l := locByID["Zone:"+z]
		// Transition instants from the zone's own ZoneBounds walk.
		var secs []int64
		t := time.Date(1800, 1, 1, 0, 0, 0, 0, time.UTC)
		for i := 0; i < 2000; i++ {
			_, end := t.In(l).ZoneBounds()
			if end.IsZero() || end.Year() > 2450 {
				break
			}
			secs = append(secs, end.Unix()-1, end.Unix(), end.Unix()+1)
			t = end
		}
		for i := 0; i < 150; i++ {
			secs = append(secs, r.Int63n(20000000000)-5000000000)
		}
		secs = append(secs, math.MinInt64, math.MaxInt64, -1<<62, 1<<62, 0, -62135596800)
		sort.Slice(secs, func(i, j int) bool { return secs[i] < secs[j] })
		for _, s := range secs {
			tt := time.Unix(s, 0).In(l)
			name, off := tt.Zone()
			st, en := tt.ZoneBounds()
			isdst := "0"
			if tt.IsDST() {
				isdst = "1"
			}
			o.rec("L", z, i64(s), name, itoa(off), isdst, zb(st), zb(en))
		}
	}
	// Malformed data.
	e := func(label string, data []byte) {
		l, err := time.LoadLocationFromTZData("X", data)
		if err != nil {
			o.rec("E", label, hex.EncodeToString(data), "ERR "+err.Error())
			return
		}
		name, off := time.Date(2020, 6, 1, 0, 0, 0, 0, l).Zone()
		o.rec("E", label, hex.EncodeToString(data), fmt.Sprintf("OK %s %d", name, off))
	}
	ny := zoneDat["America/New_York"]
	e("empty", nil)
	e("magic", []byte("TZiX0000000000000000000000000000000000000000000"))
	e("v9", append([]byte("TZif9"), ny[5:]...))
	for _, n := range []int{4, 5, 20, 44, 45, 60, 200, 1000, len(ny) / 2, len(ny) - 30, len(ny) - 1} {
		e(fmt.Sprintf("trunc%d", n), ny[:n])
	}
	e("v1only", v1Only(zoneDat["Europe/Berlin"]))
	e("etcgmt", zoneDat["Etc/GMT+5"])
}

// v1Only rewrites a TZif v2+ file as version 1 (keeps the 32-bit block).
func v1Only(d []byte) []byte {
	c := append([]byte(nil), d...)
	c[4] = 0
	return c
}

// ---------------------------------------------------------------------------
// tzset.txt: synthetic TZif v2 files with one zone, no transitions and an
// extend string; lookups exercise tzset.
//   T <extend> <hex TZif> <sec> <name> <offset> <isdst> <zbstart> <zbend>

func tzif(extend string, withTx bool) []byte {
	var b bytes.Buffer
	hdr := func(ntime, ntype, nchar int) {
		b.WriteString("TZif2")
		b.Write(make([]byte, 15))
		for _, v := range []int{0, 0, 0, ntime, ntype, nchar} {
			binary.Write(&b, binary.BigEndian, uint32(v))
		}
	}
	abbr := "LMT\x00"
	// v1 block (empty-ish)
	hdr(0, 1, len(abbr))
	binary.Write(&b, binary.BigEndian, int32(3600))
	b.WriteByte(0)
	b.WriteByte(0)
	b.WriteString(abbr)
	// v2 block
	if withTx {
		hdr(1, 1, len(abbr))
		binary.Write(&b, binary.BigEndian, int64(-2000000000))
		b.WriteByte(0)
	} else {
		hdr(0, 1, len(abbr))
	}
	binary.Write(&b, binary.BigEndian, int32(3600))
	b.WriteByte(0)
	b.WriteByte(0)
	b.WriteString(abbr)
	b.WriteString("\n" + extend + "\n")
	return b.Bytes()
}

func genTzset(dir string) {
	o := create(dir, "tzset.txt.gz")
	defer o.close()
	r := rand.New(rand.NewSource(7))
	zb := func(t time.Time) string {
		if t.IsZero() {
			return "Z"
		}
		return i64(t.Unix())
	}
	exts := []string{"EST5EDT,M3.2.0,M11.1.0", "PST8PDT", "CET-1CEST,M3.5.0,M10.5.0/3", "AEST-10AEDT,M10.1.0,M4.1.0/3",
		"<+07>-7", "<-03>3", "IST-2IDT,M3.4.4/26,M10.5.0", "<+1030>-10:30<+11>-11,M10.1.0,M4.1.0",
		"NZST-12NZDT,M9.5.0,M4.1.0/3", "<+0545>-5:45", "GMT0IST,M10.5.0,M3.5.0/1", "EST5EDT,J60/2,J300/2",
		"EST5EDT,59/2,300/2", "EST5EDT,J1,J365", "ABC+1:30:45DEF,M1.1.0,M12.5.6/-1", "XYZ-167XZZ,M3.2.0,M11.1.0",
		"<+13>-13<+14>,M9.5.0/3,M4.1.0/4", "AAA3BBB,M3.2.0/-2:30,M11.1.0/25", "AAA3BBB;M3.2.0,M11.1.0",
		"AB3", "ABC", "ABC3DEF4", "ABC3DEF,M13.1.0,M1.1.0", "ABC3DEF,M3.6.0,M1.1.0", "ABC3DEF,M3.2.7,M1.1.0",
		"ABC3DEF,J0,J1", "ABC3DEF,366,1", "ABC3DEF,M3.2.0", "ABC3DEF,M3.2.0,M11.1.0x", "ABC169", "<ABC", "ABC3DEF,",
		"ABC3DEF4,M3.2.0,M11.1.0", "ABC+3", "ABC-3:30:15", "ABC3:60", "WET0WEST,M3.5.0/1,M10.5.0",
		"<-04>4<-03>,M9.1.6/24,M4.1.6/24", "<+0330>-3:30", "MSK-3", "ÄBC3DEF,M3.2.0,M11.1.0"}
	for _, ext := range exts {
		for _, withTx := range []bool{false, true} {
			data := tzif(ext, withTx)
			l, err := time.LoadLocationFromTZData("T", data)
			if err != nil {
				o.rec("TE", ext, hex.EncodeToString(data), err.Error())
				continue
			}
			var secs []int64
			for y := 1965; y <= 2045; y += 4 {
				for _, md := range [][2]int{{1, 1}, {3, 8}, {3, 15}, {3, 29}, {4, 5}, {6, 1}, {10, 6}, {10, 26}, {11, 2}, {11, 9}, {12, 31}} {
					secs = append(secs, time.Date(y, time.Month(md[0]), md[1], r.Intn(24), 0, 0, 0, time.UTC).Unix())
				}
			}
			for i := 0; i < 30; i++ {
				secs = append(secs, r.Int63n(1<<36)-1<<35)
			}
			secs = append(secs, -2000000001, -2000000000, math.MaxInt64, math.MinInt64+1)
			for _, s := range secs {
				t := time.Unix(s, 0).In(l)
				name, off := t.Zone()
				st, en := t.ZoneBounds()
				isdst := "0"
				if t.IsDST() {
					isdst = "1"
				}
				o.rec("T", ext, hex.EncodeToString(data), i64(s), name, itoa(off), isdst, zb(st), zb(en))
			}
			// Date() through the extend rules (DST gaps).
			for _, c := range [][4]int{{2024, 3, 10, 2}, {2024, 11, 3, 1}, {2024, 3, 31, 2}, {2024, 10, 27, 2}, {2024, 4, 7, 2}, {2024, 10, 6, 2}} {
				t := time.Date(c[0], time.Month(c[1]), c[2], c[3], 30, 0, 0, l)
				o.rec(append([]string{"TD", ext, hex.EncodeToString(data), itoa(c[0]), itoa(c[1]), itoa(c[2]), itoa(c[3])}, timeFields(t)...)...)
			}
		}
	}
}

// ---------------------------------------------------------------------------
// zip.txt + test.zip: LoadLocation through $ZONEINFO pointing at a zip.
//   Z <name> OK <zone at 2020-06-01> | ERR <msg>

func genZip(dir string) {
	var buf bytes.Buffer
	zw := zip.NewWriter(&buf)
	add := func(name string, data []byte, method uint16) {
		w, err := zw.CreateHeader(&zip.FileHeader{Name: name, Method: method})
		must(err)
		_, err = w.Write(data)
		must(err)
	}
	add("Test/NewYork", zoneDat["America/New_York"], zip.Store)
	add("Test/Bangkok", zoneDat["Asia/Bangkok"], zip.Store)
	add("Test/Compressed", zoneDat["Europe/Berlin"], zip.Deflate)
	add("Test/Bad", []byte("not a tzif file"), zip.Store)
	add("Europe/Berlin", zoneDat["Asia/Tokyo"], zip.Store) // shadows the system zone
	must(zw.Close())
	zipPath := filepath.Join(dir, "test.zip")
	must(os.WriteFile(zipPath, buf.Bytes(), 0o644))

	abs, err := filepath.Abs(zipPath)
	must(err)
	cmd := exec.Command(os.Args[0], "-child", "zip")
	cmd.Env = append(os.Environ(), "ZONEINFO="+abs, "GOROOT=/nonexistent-goroot")
	outb, err := cmd.Output()
	must(err)
	// The error messages embed the zip path; record it for substitution.
	outb = append([]byte("P\t"+esc(abs)+"\n"), outb...)
	must(os.WriteFile(filepath.Join(dir, "zip.txt"), outb, 0o644))
}

var zipNames = []string{"Test/NewYork", "Test/Bangkok", "Test/Compressed", "Test/Bad", "Europe/Berlin", "No/Such", "Test"}

// ---------------------------------------------------------------------------
// system.txt: LoadLocation against the system zoneinfo.
//   H <path> <sha256>
//   Y <name> OK <locname> <lookups...> | ERR <msg>

var systemNames = []string{"", "UTC", "America/New_York", "Asia/Bangkok", "Europe/Berlin", "Australia/Sydney",
	"Asia/Kolkata", "Etc/GMT+5", "../etc/passwd", "a..b", "/etc/localtime", "\\x", "No/Such_Zone", "America",
	"zone.tab", "posixrules", "Asia/Bangkok/", "asia/bangkok", "America/Argentina/Buenos_Aires", "Factory",
	"a\x00b", strings.Repeat("A", 300),
	// adversarial additions
	"Local", "UTC ", "utc", "Etc/UTC", "Etc/GMT-14", "Asia//Tokyo", "Asia/./Tokyo", "./Asia/Tokyo", "Asia/Tokyo/..",
	"Asia/.", ".", "a.b", "posixrules/x", "+VERSION", "leapseconds", "tzdata.zi", "America/Argentina",
	"/", "\\", "Asia\\Tokyo", "Europe/London\x00", "EST5EDT", "Factory", "GMT+0", "Etc/GMT+12",
	strings.Repeat("A/", 200) + "B", "Asia/Tokyo" + strings.Repeat("/", 5)}

func genSystem(dir string) {
	cmd := exec.Command(os.Args[0], "-child", "system")
	cmd.Env = append(os.Environ(), "ZONEINFO=", "GOROOT=/nonexistent-goroot")
	outb, err := cmd.Output()
	must(err)
	must(os.WriteFile(filepath.Join(dir, "system.txt"), outb, 0o644))
}

// ---------------------------------------------------------------------------
// localenv.txt: Local as initialised from $TZ (child processes).
//   V <tz or "<unset>"> <Local.String()> <zone lookups...>

var tzValues = []string{"<unset>", "", "UTC", ":UTC", "Asia/Tokyo", ":Asia/Tokyo", "/usr/share/zoneinfo/Europe/Berlin",
	":/usr/share/zoneinfo/Europe/Berlin", "/etc/localtime", ":/etc/localtime", "No/Such", "/no/such/file", ":",
	"America/New_York", "EST5EDT", "Europe/Dublin",
	// adversarial additions
	"Asia/Bangkok/", "../zoneinfo/Asia/Tokyo", "::Asia/Tokyo", "/usr/share/zoneinfo/", "/usr/share/zoneinfo",
	"UTC0", "posixrules", "Factory", "GMT", "EST", " Asia/Tokyo", "/etc/localtime/", "/dev/null",
	"/private/etc/localtime", "zone.tab", "asia/tokyo", "Etc/GMT-14", "/usr/share/zoneinfo/Asia/../Asia/Tokyo",
	"Asia//Tokyo", "Asia/Tokyo ", "PST8PDT,M3.2.0,M11.1.0", "<+07>-7", "localtime", "/etc/./localtime"}

func genLocalEnv(dir string) {
	o := create(dir, "localenv.txt")
	defer o.close()
	for _, tz := range tzValues {
		cmd := exec.Command(os.Args[0], "-child", "localenv")
		env := []string{}
		for _, kv := range os.Environ() {
			if !strings.HasPrefix(kv, "TZ=") {
				env = append(env, kv)
			}
		}
		if tz != "<unset>" {
			env = append(env, "TZ="+tz)
		}
		cmd.Env = append(env, "GOROOT=/nonexistent-goroot")
		outb, err := cmd.Output()
		must(err)
		o.rec(append([]string{"V", tz}, strings.Split(strings.TrimSuffix(string(outb), "\n"), "\t")...)...)
	}
	// hashes of the files involved
	for _, p := range []string{"/etc/localtime", "/usr/share/zoneinfo/Asia/Tokyo", "/usr/share/zoneinfo/Europe/Berlin",
		"/usr/share/zoneinfo/America/New_York", "/usr/share/zoneinfo/EST5EDT", "/usr/share/zoneinfo/Europe/Dublin",
		"/usr/share/zoneinfo/posixrules", "/usr/share/zoneinfo/Factory", "/usr/share/zoneinfo/GMT", "/usr/share/zoneinfo/EST",
		"/usr/share/zoneinfo/Etc/GMT-14", "/usr/share/zoneinfo/zone.tab", "/usr/share/zoneinfo/UTC0",
		"/usr/share/zoneinfo/localtime", "/usr/share/zoneinfo/Asia/Bangkok"} {
		o.rec("H", p, fileHash(p))
	}
}

func fileHash(p string) string {
	b, err := os.ReadFile(p)
	if err != nil {
		return "missing"
	}
	s := sha256.Sum256(b)
	return hex.EncodeToString(s[:])
}

func lookupsOf(l *time.Location) []string {
	var v []string
	for _, s := range []int64{-5000000000, 0, 1600000000, 1720000000, 1735000000, 4102444800} {
		name, off := time.Unix(s, 0).In(l).Zone()
		v = append(v, name+" "+itoa(off))
	}
	return v
}

func runChild(mode string) {
	w := bufio.NewWriter(os.Stdout)
	defer w.Flush()
	rec := func(fields ...string) {
		for i, f := range fields {
			if i > 0 {
				w.WriteByte('\t')
			}
			w.WriteString(esc(f))
		}
		w.WriteByte('\n')
	}
	switch mode {
	case "zoneall":
		genZoneAllChild(os.Getenv("ZONEALL_OUT"), os.Getenv("ZONEALL_TEXT"))
	case "zip":
		for _, n := range zipNames {
			l, err := time.LoadLocation(n)
			if err != nil {
				rec("Z", n, "ERR", err.Error())
				continue
			}
			rec(append([]string{"Z", n, "OK", l.String()}, lookupsOf(l)...)...)
		}
	case "system":
		for _, n := range systemNames {
			l, err := time.LoadLocation(n)
			if err != nil {
				rec("Y", n, "ERR", err.Error())
				continue
			}
			rec(append([]string{"Y", n, "OK", l.String()}, lookupsOf(l)...)...)
		}
		for _, n := range systemNames {
			if n == "" || strings.Contains(n, "..") || strings.ContainsRune(n, 0) || n[0] == '/' || n[0] == '\\' || len(n) > 200 {
				continue
			}
			rec("H", "/usr/share/zoneinfo/"+n, fileHash("/usr/share/zoneinfo/"+n))
		}
	case "localenv":
		w.WriteString(esc(time.Local.String()))
		for _, s := range lookupsOf(time.Local) {
			w.WriteByte('\t')
			w.WriteString(esc(s))
		}
		w.WriteByte('\n')
	}
}

// ---------------------------------------------------------------------------
// seeksnack.txt: every date-like string of the seeksnack sources parsed with
// the cast.ToTime layout list (internal.TimeFormats) via time.Parse, then
// formatted with the layouts the site templates use.
//   K <value> <layout index or -1> <time fields after ParseDateWith(UTC)> <formats...>

var castLayouts = []struct {
	f   string
	typ int
}{
	{"2006-01-02", 0}, {time.RFC3339, 2}, {"2006-01-02T15:04:05", 0}, {time.RFC1123Z, 2}, {time.RFC1123, 1},
	{time.RFC822Z, 2}, {time.RFC822, 1}, {time.RFC850, 1}, {"2006-01-02 15:04:05.999999999 -0700 MST", 3},
	{"2006-01-02T15:04:05-0700", 2}, {"2006-01-02 15:04:05Z0700", 2}, {"2006-01-02 15:04:05", 0}, {time.ANSIC, 0},
	{time.UnixDate, 1}, {time.RubyDate, 2}, {"2006-01-02 15:04:05Z07:00", 2}, {"02 Jan 2006", 0},
	{"2006-01-02 15:04:05 -07:00", 2}, {"2006-01-02 15:04:05 -0700", 2}, {time.Kitchen, 4}, {time.Stamp, 4},
	{time.StampMilli, 4}, {time.StampMicro, 4}, {time.StampNano, 4},
}

var siteFormats = []string{"Mon, 02 Jan 2006 15:04:05 -0700", "2006-01-02T15:04:05-07:00", "2006", "2006 ",
	"Jan 2, 2006", "January 2, 2006", "2006-01-02", "2006-01-02T15:04:05Z07:00", time.RFC3339}

func genSeeksnack(dir string) {
	o := create(dir, "seeksnack.txt.gz")
	defer o.close()
	vals := map[string]bool{}
	root := os.Getenv("SEEKSNACK")
	if root != "" {
		re := func(p string) {
			b, err := os.ReadFile(p)
			if err != nil {
				return
			}
			s := string(b)
			for i := 0; i+10 <= len(s); i++ {
				if isDateStart(s[i:]) && (i == 0 || !isDigitB(s[i-1])) {
					j := i + 10
					for j < len(s) && strings.IndexByte("0123456789:.TZ+- ", s[j]) >= 0 && j-i < 40 {
						j++
					}
					v := strings.TrimRight(s[i:j], " ")
					vals[v] = true
				}
			}
		}
		for _, sub := range []string{"content", "data"} {
			filepath.Walk(filepath.Join(root, sub), func(p string, info os.FileInfo, err error) error {
				if err == nil && !info.IsDir() {
					re(p)
				}
				return nil
			})
		}
	}
	for _, v := range []string{"2020-05-17T15:05:09.238Z", "2019-12-31T07:06:21.671Z", "2021-06-09T04:46:11+07:00",
		"2022-01-07T07:35:39Z", "2020-05-17", "2020-05-17 15:05:09", "2020-05-17T15:05:09", "17 May 2020",
		"Sun, 17 May 2020 15:05:09 +0700", "Sun, 17 May 2020 15:05:09 ICT", "Sun, 17 May 2020 15:05:09 PST",
		"2020-08-14 14:59:12.921 +0000 UTC", "2021-06-09 04:46:11 +0700 +07", "3:04PM", "May 17 15:05:09",
		"2026-09-27T12:00:00Z", "not a date", ""} {
		vals[v] = true
	}
	var keys []string
	for k := range vals {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, v := range keys {
		idx := -1
		var d time.Time
		for i, cl := range castLayouts {
			t, err := time.Parse(cl.f, v)
			if err == nil {
				idx = i
				d = t
				if cl.typ <= 1 {
					y, m, dd := d.Date()
					h, mi, s := d.Clock()
					d = time.Date(y, m, dd, h, mi, s, d.Nanosecond(), time.UTC)
				}
				break
			}
		}
		f := []string{"K", v, itoa(idx)}
		f = append(f, timeFields(d)...)
		for _, sf := range siteFormats {
			f = append(f, d.Format(sf))
		}
		// htime.ToTimeInDefaultLocationE re-formats time.Time values with RFC3339.
		f = append(f, d.Format(time.RFC3339))
		o.rec(f...)
	}
}

func isDigitB(c byte) bool { return c >= '0' && c <= '9' }

func isDateStart(s string) bool {
	return len(s) >= 10 && isDigitB(s[0]) && isDigitB(s[1]) && isDigitB(s[2]) && isDigitB(s[3]) && s[4] == '-' &&
		isDigitB(s[5]) && isDigitB(s[6]) && s[7] == '-' && isDigitB(s[8]) && isDigitB(s[9])
}
