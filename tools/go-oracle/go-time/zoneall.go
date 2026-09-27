package main

// zoneall.txt.gz: every zone of the system zoneinfo directory (the source
// the golden build loads zones from) and of $GOROOT/lib/time/zoneinfo.zip,
// reduced to a SHA-256 digest of a text that exercises the zone:
// lookups (name, offset, isDST, ZoneBounds) at every transition ±1s between
// 1800 and 2080 (walking ZoneBounds), fixed far instants, and Date() for
// wall-clock times around each transition (DST gaps and overlaps), formatted
// with "2006-01-02 15:04:05 MST -07:00:00".
//
//	ZH  <zip path> <sha256 of the zip>
//	ZZ  <name> <lines> <digest>                  (zone from the zip)
//	SZ  <name> <sha256 of the file> <lines> <digest>  (system zone)
//
// The Rust test recomputes the same text (tests/zoneall.rs) and compares
// digests for the files whose hashes still match. The -zonetext flag writes
// the full texts to a directory for debugging.

import (
	"archive/zip"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"math"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"time"
)

// zoneText is mirrored by tests/zoneall.rs:zone_text.
func zoneText(l *time.Location) string {
	var b strings.Builder
	zb := func(t time.Time) string {
		if t.IsZero() {
			return "Z"
		}
		return i64(t.Unix())
	}
	look := func(s int64) {
		t := time.Unix(s, 0).In(l)
		name, off := t.Zone()
		st, en := t.ZoneBounds()
		fmt.Fprintf(&b, "L %d %s %d %t %s %s\n", s, name, off, t.IsDST(), zb(st), zb(en))
	}
	for _, s := range []int64{math.MinInt64, -1 << 40, -5000000000, -1, 0, 1790000000, 4102444800, 1 << 40, math.MaxInt64} {
		look(s)
	}
	t := time.Date(1800, 1, 1, 0, 0, 0, 0, time.UTC)
	for i := 0; i < 5000; i++ {
		_, end := t.In(l).ZoneBounds()
		if end.IsZero() || end.Year() > 2080 {
			break
		}
		if end.Unix() <= t.Unix() {
			// tzset's "end of year" bound (ystart+365 days) repeats in
			// leap years; step past it.
			t = t.Add(24 * time.Hour)
			continue
		}
		s := end.Unix()
		look(s - 1)
		look(s)
		look(s + 1)
		lt := time.Unix(s, 0).In(l)
		y, m, d := lt.Date()
		h, mi, sec := lt.Clock()
		for _, dm := range []int{-61, -60, -30, -1, 0, 1, 59, 60} {
			dt := time.Date(y, m, d, h, mi+dm, sec, 0, l)
			fmt.Fprintf(&b, "D %d %d %s\n", dm, dt.Unix(), dt.Format("2006-01-02 15:04:05 MST -07:00:00"))
		}
		t = end
	}
	return b.String()
}

func digestOf(s string) (int, string) {
	h := sha256.Sum256([]byte(s))
	return strings.Count(s, "\n"), hex.EncodeToString(h[:])
}

// genZoneAll runs in a child process so that $ZONEINFO is empty there.
func genZoneAll(dir, textDir string) {
	cmd := exec.Command(os.Args[0], "-child", "zoneall")
	cmd.Env = append(os.Environ(), "ZONEINFO=", "ZONEALL_OUT="+dir, "ZONEALL_TEXT="+textDir)
	cmd.Stderr = os.Stderr
	must(cmd.Run())
}

func genZoneAllChild(dir, textDir string) {
	o := create(dir, "zoneall.txt.gz")
	defer o.close()
	writeText := func(kind, name, text string) {
		if textDir == "" {
			return
		}
		p := filepath.Join(textDir, kind, strings.ReplaceAll(name, "/", "__"))
		must(os.MkdirAll(filepath.Dir(p), 0o755))
		must(os.WriteFile(p, []byte(text), 0o644))
	}

	// $GOROOT/lib/time/zoneinfo.zip
	zipPath := filepath.Join(runtime.GOROOT(), "lib", "time", "zoneinfo.zip")
	o.rec("ZH", zipPath, fileHash(zipPath))
	zr, err := zip.OpenReader(zipPath)
	must(err)
	var names []string
	datas := map[string][]byte{}
	for _, f := range zr.File {
		rc, err := f.Open()
		must(err)
		d, err := io.ReadAll(rc)
		must(err)
		rc.Close()
		names = append(names, f.Name)
		datas[f.Name] = d
	}
	zr.Close()
	sort.Strings(names)
	for _, n := range names {
		l, err := time.LoadLocationFromTZData(n, datas[n])
		if err != nil {
			o.rec("ZZ", n, "ERR", err.Error())
			continue
		}
		text := zoneText(l)
		writeText("zip", n, text)
		lines, dg := digestOf(text)
		o.rec("ZZ", n, itoa(lines), dg)
	}

	// System zoneinfo (what LoadLocation reads first on darwin).
	root := "/usr/share/zoneinfo/"
	var sys []string
	filepath.Walk(root, func(p string, info os.FileInfo, err error) error {
		if err != nil || info.IsDir() {
			return nil
		}
		rel, _ := filepath.Rel(root, p)
		sys = append(sys, rel)
		return nil
	})
	sort.Strings(sys)
	for _, n := range sys {
		l, err := time.LoadLocation(n)
		if err != nil {
			o.rec("SZ", n, fileHash(root+n), "ERR", err.Error())
			continue
		}
		text := zoneText(l)
		writeText("sys", n, text)
		lines, dg := digestOf(text)
		o.rec("SZ", n, fileHash(root+n), itoa(lines), dg)
	}
}
