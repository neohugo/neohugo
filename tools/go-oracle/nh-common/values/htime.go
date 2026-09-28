package main

import (
	"time"

	"github.com/gohugoio/localescompressed"
	"github.com/neohugo/neohugo/common/htime"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// zones are the default locations of ToTimeInDefaultLocationE (the Rust test
// rebuilds them by index).
var zones = []*time.Location{time.UTC, time.FixedZone("ICT", 7*3600), time.FixedZone("X", -5*3600)}

func htimeCases() []map[string]any {
	var cases []map[string]any
	ict := zones[1]
	ins := []any{
		time.Date(2024, 2, 29, 13, 14, 15, 123456789, time.UTC), time.Date(1999, 12, 31, 23, 59, 59, 0, ict),
		time.Time{}, time.Date(2021, 6, 1, 0, 0, 0, 0, zones[2]),
		"2020-01-02", "2020-01-02T03:04:05Z", "2020-01-02T03:04:05+07:00", "2020-01-02T03:04:05.123-05:00",
		"2020-01-02 03:04:05", "2020-01-02 03:04:05 +0100", "02 Jan 06 15:04 MST", "Jan 2, 2006", "Mon, 02 Jan 2006 15:04:05 GMT",
		"", "bad", "1579314000", "2006-01-02T15:04:05", 1579314000, int64(1579314000), 1.5e9, nil, true,
	}
	for _, v := range ins {
		for zi, loc := range zones {
			cases = append(cases, map[string]any{
				"op": "toTime", "v": goval.Encode(v), "zone": zi,
				"r": goval.Call(func() (any, error) { return htime.ToTimeInDefaultLocationE(v, loc) }),
			})
		}
	}

	layouts := []string{
		"", ":date_full", ":date_long", ":date_medium", ":date_short", ":time_full", ":time_long", ":time_medium",
		":time_short", ":DATE_FULL", ":Date_Short", ":date", ":foo", ":", "Monday, January 2, 2006", "Mon Jan 2 2006",
		"January", "Jan", "Monday", "Mon", "2006-01-02", "02 Jan 06 15:04 MST", "January Jan Monday Mon",
		"Jan January", "Mon Monday", "3:04PM", time.RFC3339, time.RFC1123, "Janu", "Mo", "MondayJanuary",
	}
	var times []time.Time
	for m := 1; m <= 12; m++ {
		times = append(times, time.Date(2023, time.Month(m), m+(m%7), m, m*4, 5, 0, time.UTC))
	}
	times = append(times, time.Date(2019, 12, 31, 23, 0, 0, 0, ict), time.Date(1, 1, 1, 0, 0, 0, 0, time.UTC))
	for _, lang := range []string{"en", "th"} {
		f := htime.NewTimeFormatter(localescompressed.GetTranslator(lang))
		for _, t := range times {
			var outs []any
			for _, l := range layouts {
				outs = append(outs, goval.Str(f.Format(t, l)))
			}
			cases = append(cases, map[string]any{"op": "format", "lang": lang, "t": goval.Encode(t), "layouts": layouts, "out": outs})
		}
	}
	return cases
}
