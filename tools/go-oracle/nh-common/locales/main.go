// Command locales is the Go oracle for crates/nh-common/src/locales.rs, the
// port of the en and th translators of github.com/gohugoio/localescompressed
// (the version in go.mod).
//
//	go run ./tools/go-oracle/nh-common/locales [-out rust/testdata/oracle/common/locales/locales.json]
//
// It records, for en and th:
//   - FmtDateShort/Medium/Long/Full and FmtTimeShort/Medium/Long/Full over a
//     systematic sweep of instants (every month and weekday, every hour, minute
//     and second boundary, year bounds, DST transitions) in UTC, fixed zones and
//     named zones (their TZif data is copied from $GOROOT/lib/time/zoneinfo.zip
//     into the fixture so the Rust test loads the same rules);
//   - FmtNumber, FmtPercent, FmtCurrency and FmtAccounting over a table of
//     numbers (negative, huge, tiny, rounding ties, NaN, ±Inf) at every
//     precision 0..7, 10, 15..17 and 20 (Hugo caps precisions at 20), a Go runtime panic recorded as {"panic": msg};
//   - the plural rules, month and weekday names and lists;
//   - GetTranslator and GetCurrency over valid, case-variant and unknown keys.
//
// strconv.FormatFloat has no FMA site, but float formatting is where the
// platforms could differ, so the checked-in fixture is produced by an arm64
// build (GOARCH=arm64, run under qemu-aarch64-static) and compared with amd64.
package main

import (
	"archive/zip"
	"encoding/hex"
	"flag"
	"fmt"
	"io"
	"log"
	"math"
	"path/filepath"
	"runtime"
	"sort"
	"time"

	"github.com/gohugoio/locales"
	"github.com/gohugoio/localescompressed"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
)

type zoneSpec struct {
	Name   string `json:"name"`
	Kind   string `json:"kind"` // "utc", "fixed", "tzdata"
	Offset int    `json:"offset,omitempty"`
	TZData string `json:"tzdata,omitempty"` // hex
	loc    *time.Location
}

func main() {
	out := flag.String("out", "rust/testdata/oracle/common/locales/locales.json", "output file")
	zipPath := flag.String("zoneinfo", filepath.Join(runtime.GOROOT(), "lib", "time", "zoneinfo.zip"), "zoneinfo.zip")
	flag.Parse()

	zones := []*zoneSpec{
		{Name: "UTC", Kind: "utc", loc: time.UTC},
		{Name: "", Kind: "fixed", Offset: 7 * 3600},
		{Name: "ICT", Kind: "fixed", Offset: 7 * 3600},
		{Name: "", Kind: "fixed", Offset: -(9*3600 + 30*60)},
		{Name: "MST", Kind: "fixed", Offset: -7 * 3600},
	}
	for _, name := range []string{"America/New_York", "Asia/Bangkok", "Europe/Berlin", "Australia/Sydney", "Asia/Tokyo", "America/Los_Angeles", "Asia/Kolkata", "America/St_Johns"} {
		data := readZone(*zipPath, name)
		loc, err := time.LoadLocationFromTZData(name, data)
		if err != nil {
			log.Fatal(err)
		}
		zones = append(zones, &zoneSpec{Name: name, Kind: "tzdata", TZData: hex.EncodeToString(data), loc: loc})
	}
	for _, z := range zones {
		if z.Kind == "fixed" {
			z.loc = time.FixedZone(z.Name, z.Offset)
		}
	}

	trs := map[string]locales.Translator{}
	for _, l := range []string{"en", "th"} {
		trs[l] = localescompressed.GetTranslator(l)
	}

	var cases []map[string]any

	// Dates and times.
	all := instants()
	edges := edgeInstants()
	for zi, z := range zones {
		// The full sweep in UTC and Asia/Bangkok (seeksnack's +07:00); the DST
		// edges (zone names and offsets) in every zone.
		sweep := edges
		switch z.Name {
		case "UTC", "Asia/Bangkok":
			sweep = all
		}
		for _, u := range sweep {
			t := u.In(z.loc)
			c := map[string]any{"kind": "time", "unix": t.Unix(), "nsec": t.Nanosecond(), "zone": zi}
			for _, l := range []string{"en", "th"} {
				tr := trs[l]
				c[l] = map[string]string{
					"date_short":  tr.FmtDateShort(t),
					"date_medium": tr.FmtDateMedium(t),
					"date_long":   tr.FmtDateLong(t),
					"date_full":   tr.FmtDateFull(t),
					"time_short":  tr.FmtTimeShort(t),
					"time_medium": tr.FmtTimeMedium(t),
					"time_long":   tr.FmtTimeLong(t),
					"time_full":   tr.FmtTimeFull(t),
				}
			}
			cases = append(cases, c)
		}
	}

	// Numbers.
	currencies := []string{"USD", "THB", "JPY", "usd", "ZZZ"}
	for _, n := range numbers() {
		for _, v := range []uint64{0, 1, 2, 3, 4, 5, 6, 7, 10, 15, 16, 17, 20} {
			c := map[string]any{"kind": "number", "n": fmt.Sprintf("%016x", math.Float64bits(n)), "v": v}
			for _, l := range []string{"en", "th"} {
				tr := trs[l]
				m := map[string]any{
					"number":   corpus.Call(func() string { return tr.FmtNumber(n, v) }),
					"percent":  corpus.Call(func() string { return tr.FmtPercent(n, v) }),
					"cardinal": tr.CardinalPluralRule(n, v).String(),
					"ordinal":  tr.OrdinalPluralRule(n, v).String(),
					"range":    tr.RangePluralRule(n, v, n, v).String(),
				}
				// Currencies at a few precisions only (the digits are FmtNumber's).
				if v <= 3 || v == 20 {
					for _, cur := range currencies {
						ct := localescompressed.GetCurrency(cur)
						m["currency:"+cur] = corpus.Call(func() string { return tr.FmtCurrency(n, v, ct) })
						m["accounting:"+cur] = corpus.Call(func() string { return tr.FmtAccounting(n, v, ct) })
					}
				}
				c[l] = m
			}
			cases = append(cases, c)
		}
	}

	// Names and lists.
	for _, l := range []string{"en", "th"} {
		tr := trs[l]
		c := map[string]any{
			"kind":                "names",
			"locale":              l,
			"Locale":              tr.Locale(),
			"PluralsCardinal":     rules(tr.PluralsCardinal()),
			"PluralsOrdinal":      rules(tr.PluralsOrdinal()),
			"PluralsRange":        rules(tr.PluralsRange()),
			"MonthsAbbreviated":   tr.MonthsAbbreviated(),
			"MonthsNarrow":        tr.MonthsNarrow(),
			"MonthsWide":          tr.MonthsWide(),
			"WeekdaysAbbreviated": tr.WeekdaysAbbreviated(),
			"WeekdaysNarrow":      tr.WeekdaysNarrow(),
			"WeekdaysShort":       tr.WeekdaysShort(),
			"WeekdaysWide":        tr.WeekdaysWide(),
		}
		var ma, mn, mw, wa, wn, ws, ww []string
		for m := time.January; m <= time.December; m++ {
			ma = append(ma, tr.MonthAbbreviated(m))
			mn = append(mn, tr.MonthNarrow(m))
			mw = append(mw, tr.MonthWide(m))
		}
		for d := time.Sunday; d <= time.Saturday; d++ {
			wa = append(wa, tr.WeekdayAbbreviated(d))
			wn = append(wn, tr.WeekdayNarrow(d))
			ws = append(ws, tr.WeekdayShort(d))
			ww = append(ww, tr.WeekdayWide(d))
		}
		c["MonthAbbreviated"], c["MonthNarrow"], c["MonthWide"] = ma, mn, mw
		c["WeekdayAbbreviated"], c["WeekdayNarrow"], c["WeekdayShort"], c["WeekdayWide"] = wa, wn, ws, ww
		cases = append(cases, c)
	}

	// GetTranslator.
	for _, key := range []string{"en", "EN", "En", "th", "TH", "th-TH", "th_TH", "en-US", "en_us", "en-GB", "fr", "nn-NO", "zh-Hant-TW", "", "xx", "english", "thai", "en-", "-en", "e-n"} {
		tr := localescompressed.GetTranslator(key)
		c := map[string]any{"kind": "translator", "key": key, "found": tr != nil}
		if tr != nil {
			c["locale"] = tr.Locale()
		}
		cases = append(cases, c)
	}

	// GetCurrency.
	codes := []string{"", "usd", "USD", "Usd", "thb", "THB", "EUR", "JPY", "ADP", "ZWR", "XXX", "ZZZ", "US", "USDX", "€", "ｕｓｄ", "ß"}
	for _, key := range codes {
		cases = append(cases, map[string]any{"kind": "currency", "key": key, "type": int(localescompressed.GetCurrency(key))})
	}

	header := map[string]any{
		"source": "tools/go-oracle/nh-common/locales",
		"goarch": runtime.GOARCH,
		"zones":  zones,
	}
	if err := corpus.WriteCases(*out, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%d cases -> %s", len(cases), *out)
}

func rules(rs []locales.PluralRule) []string {
	out := []string{}
	for _, r := range rs {
		out = append(out, r.String())
	}
	return out
}

func readZone(zipPath, name string) []byte {
	zr, err := zip.OpenReader(zipPath)
	if err != nil {
		log.Fatal(err)
	}
	var data []byte
	for _, f := range zr.File {
		if f.Name != name {
			continue
		}
		rc, err := f.Open()
		if err != nil {
			log.Fatal(err)
		}
		data, err = io.ReadAll(rc)
		if err != nil {
			log.Fatal(err)
		}
		if err := rc.Close(); err != nil {
			log.Fatal(err)
		}
		break
	}
	if err := zr.Close(); err != nil {
		log.Fatal(err)
	}
	if data == nil {
		log.Fatalf("zone %s not in %s", name, zipPath)
	}
	return data
}

// instants returns the UTC instants of the date sweep.
func instants() []time.Time {
	set := map[time.Time]bool{}
	add := func(t time.Time) { set[t.UTC()] = true }
	// every month, and days that exercise every weekday and month end
	for m := time.January; m <= time.December; m++ {
		for _, d := range []int{1, 9, 10, 15, 28, 29, 30, 31} {
			add(time.Date(2024, m, d, 12, 30, 45, 0, time.UTC))
		}
	}
	// year bounds (negative years, year 0, one-digit to five-digit years)
	for _, y := range []int{-10000, -1000, -101, -100, -99, -10, -9, -1, 0, 1, 9, 10, 11, 99, 100, 101, 999, 1000, 1582, 1900, 1969, 1970, 1999, 2000, 2020, 2026, 2099, 9999, 10000, 12345, 292277026} {
		add(time.Date(y, time.August, 14, 9, 5, 3, 0, time.UTC))
	}
	// every hour, and minute/second boundaries
	for h := 0; h < 24; h++ {
		for _, ms := range [][2]int{{0, 0}, {9, 9}, {10, 10}, {59, 59}} {
			add(time.Date(2020, time.August, 14, h, ms[0], ms[1], 0, time.UTC))
		}
	}
	// sub-seconds
	add(time.Date(2021, time.June, 9, 23, 59, 59, 999999999, time.UTC))
	add(time.Date(2021, time.June, 9, 0, 0, 0, 1, time.UTC))
	for _, t := range edgeInstants() {
		add(t)
	}
	// the zero time, the Unix epoch, far future
	add(time.Time{})
	add(time.Unix(0, 0))
	add(time.Unix(1<<40, 0))
	out := make([]time.Time, 0, len(set))
	for t := range set {
		out = append(out, t)
	}
	sort.Slice(out, func(i, j int) bool { return out[i].Before(out[j]) })
	return out
}

// edgeInstants are the DST transitions (US, EU, AU, Newfoundland) and the
// instants either side, plus a winter and a summer instant.
func edgeInstants() []time.Time {
	var out []time.Time
	for _, t := range []time.Time{
		time.Date(2021, time.March, 14, 7, 0, 0, 0, time.UTC),
		time.Date(2021, time.November, 7, 6, 0, 0, 0, time.UTC),
		time.Date(2021, time.March, 28, 1, 0, 0, 0, time.UTC),
		time.Date(2021, time.October, 31, 1, 0, 0, 0, time.UTC),
		time.Date(2021, time.April, 3, 16, 0, 0, 0, time.UTC),
		time.Date(2021, time.October, 2, 16, 0, 0, 0, time.UTC),
		time.Date(2021, time.March, 14, 5, 30, 0, 0, time.UTC),
	} {
		out = append(out, t.Add(-time.Second), t, t.Add(time.Hour))
	}
	out = append(out,
		time.Date(2020, time.January, 15, 23, 59, 59, 0, time.UTC),
		time.Date(2020, time.July, 15, 0, 0, 0, 0, time.UTC))
	return out
}

func numbers() []float64 {
	return []float64{
		0, math.Copysign(0, -1), 1, -1, 2, 0.5, -0.5, 1.5, 2.5, -2.5, 0.05, 0.15, 0.25, 0.35,
		1.005, 1.015, 1.025, 999.995, 0.1 + 0.2, 12, 123, 1234, 12345, 123456, 1234567,
		-1234567.891, 1234.5678, 98765.4321, 1e6, 1e9, 1e15, 1e16, 123456789.987654321,
		1e20, 1e21, 1e22, 1e100, -1e21, 0.000001234, 1e-7, 5e-324, -0.004, 0.0049999,
		math.MaxFloat64, -math.MaxFloat64, math.SmallestNonzeroFloat64, 9007199254740993,
		4503599627370495.5, 100, 1000, 10000, 100000, -100, -1000, 3.14159265358979,
		math.NaN(), math.Inf(1), math.Inf(-1), 1.1, 11, 11.5, 21, 101, 111, 112, 113, 122, 123.4,
	}
}
