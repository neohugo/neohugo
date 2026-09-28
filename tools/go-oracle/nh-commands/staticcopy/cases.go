package main

import (
	"encoding/hex"
	"strings"
)

func file(path, content string) entry { return entry{Path: path, Kind: "file", Content: content} }
func dir(path string) entry           { return entry{Path: path, Kind: "dir"} }
func link(path, target string) entry  { return entry{Path: path, Kind: "symlink", Target: target} }

func fileMode(path, content string, mode uint32, mtime int64) entry {
	return entry{Path: path, Kind: "file", Content: content, Mode: mode, Mtime: mtime}
}

// bin returns n deterministic bytes (all byte values, not UTF-8).
func bin(path string, n int) entry {
	b := make([]byte, n)
	x := uint32(2463534242)
	for i := range b {
		x ^= x << 13
		x ^= x >> 17
		x ^= x << 5
		b[i] = byte(x)
	}
	return entry{Path: path, Kind: "file", Hex: hex.EncodeToString(b)}
}

func toml(s string) entry { return file("hugo.toml", "baseURL = \"https://example.org/\"\n"+s) }

// basicTree is a static directory with the kinds of files a site ships:
// text, binary (larger than fsync's 1000-byte compare buffer), empty files,
// nested and empty directories, hidden files and directories, backups and
// Unicode names (NFC and NFD, Thai, emoji, spaces).
func basicTree(prefix string) []entry {
	return []entry{
		file(prefix+"robots.txt", "User-agent: *\n"),
		file(prefix+"favicon.ico", "\x00\x00\x01\x00icon"),
		bin(prefix+"images/logo.png", 5000),
		bin(prefix+"images/exact1000.bin", 1000),
		bin(prefix+"images/exact2000.bin", 2000),
		file(prefix+"empty.txt", ""),
		dir(prefix + "emptydir"),
		dir(prefix + "nested/deeper/emptyleaf"),
		file(prefix+"nested/deeper/a.css", "body{color:red}\n"),
		file(prefix+".DS_Store", "\x00\x00\x00\x01Bud1"),
		file(prefix+".hidden/secret.txt", "hidden\n"),
		file(prefix+"css/site.css.bak", "old\n"),
		file(prefix+"café.txt", "nfc\n"),
		file(prefix+"café-nfd.txt", "nfd\n"),
		file(prefix+"ไทย/หน้า.html", "<p>ไทย</p>\n"),
		file(prefix+"emoji-😀.txt", "smile\n"),
		file(prefix+"with space/file name.txt", "space\n"),
		fileMode(prefix+"scripts/run.sh", "#!/bin/sh\necho hi\n", 0o755, 1600000000),
		fileMode(prefix+"private.key", "k\n", 0o600, 1650000000),
		fileMode(prefix+"readonly.txt", "ro\n", 0o444, 1660000000),
	}
}

func cases() []caseSpec {
	var cs []caseSpec
	add := func(name string, es ...entry) { cs = append(cs, caseSpec{Name: name, Entries: es}) }

	add("basic", append(basicTree("static/"), toml(""))...)
	add("no-times-no-chmod", append(basicTree("static/"), toml("noTimes = true\nnoChmod = true\n"))...)
	add("no-static-dir", toml(""), file("content/_index.md", "home\n"))
	add("empty-static-dir", toml(""), dir("static"))
	add("publishdir-custom", append(basicTree("static/"), toml("publishDir = \"out/site\"\n"))...)

	// Several mounts into static, overlapping files and directories; a theme
	// with its own static dir; a mount into a sub folder.
	add("mounts-overlap",
		toml(`theme = "t"
[[module.mounts]]
source = "static"
target = "static"
[[module.mounts]]
source = "static2"
target = "static"
[[module.mounts]]
source = "extra"
target = "static/sub/extra"
[[module.mounts]]
source = "files/one.txt"
target = "static/single/one.txt"
`),
		file("static/a.txt", "from static\n"),
		file("static/shared/x.txt", "x from static\n"),
		file("static2/a.txt", "from static2 (shadowed)\n"),
		file("static2/b.txt", "only in static2\n"),
		file("static2/shared/y.txt", "y from static2\n"),
		file("static2/shared/x.txt", "x from static2 (shadowed)\n"),
		file("extra/e.txt", "extra\n"),
		file("extra/.hidden", "extra hidden\n"),
		file("files/one.txt", "single file mount\n"),
		file("themes/t/hugo.toml", ""),
		file("themes/t/static/a.txt", "from theme (shadowed)\n"),
		file("themes/t/static/theme-only.txt", "theme only\n"),
		file("themes/t/static/shared/z.txt", "z from theme\n"),
	)
	add("mounts-static-dirs",
		toml("staticDir = [\"static\", \"static-b\"]\n"),
		file("static/a.txt", "a\n"),
		file("static-b/a.txt", "a from b\n"),
		file("static-b/b.txt", "b\n"),
	)

	// Symlinks inside static: to a file and a directory outside the site,
	// inside static, and a broken one.
	add("symlinks",
		toml(""),
		file("static/real.txt", "real\n"),
		file("outside/o.txt", "outside file\n"),
		file("outside/dir/d.txt", "outside dir file\n"),
		link("static/link-to-real.txt", "real.txt"),
		link("static/link-outside.txt", "../outside/o.txt"),
		link("static/link-dir", "../outside/dir"),
		link("static/broken", "nope.txt"),
	)
	add("symlinked-static-dir",
		toml(""),
		file("realstatic/r.txt", "via symlinked static dir\n"),
		link("static", "realstatic"),
	)

	// An existing publish dir: identical, same size different content, a
	// directory where the source has a file, a file where the source has a
	// directory, and extra files and directories (visible and hidden).
	existing := []entry{
		file("static/same.txt", "same\n"),
		file("static/samesize.txt", "AAAA\n"),
		file("static/grow.txt", "longer content now\n"),
		file("static/wasdir", "now a file\n"),
		file("static/wasfile/inner.txt", "now a dir\n"),
		file("static/keep/k.txt", "k\n"),
		fileMode("public/same.txt", "same\n", 0o600, 1500000000),
		file("public/samesize.txt", "BBBB\n"),
		file("public/grow.txt", "short\n"),
		file("public/wasdir/old.txt", "old\n"),
		file("public/wasfile", "old file\n"),
		file("public/extra.txt", "extra\n"),
		file("public/extradir/e.txt", "e\n"),
		file("public/.git/HEAD", "ref\n"),
		file("public/keep/stale.txt", "stale\n"),
		file("public/index.html", "<html>rendered</html>\n"),
	}
	add("existing-dest", append(existing, toml(""))...)
	add("existing-dest-clean", append(existing, toml("cleanDestinationDir = true\n"))...)
	add("existing-dest-clean-notimes", append(existing, toml("cleanDestinationDir = true\nnoTimes = true\n"))...)

	// Many files (the plan's 23-file seeksnack static dir had .DS_Store and .bak files).
	var many []entry
	for i := 0; i < 23; i++ {
		name := strings.Repeat("f", 1+i%3)
		many = append(many, file("static/"+name+"/"+string(rune('a'+i))+".txt", strings.Repeat("x", i*97)))
	}
	add("many", append(many, toml(""))...)
	return cs
}
