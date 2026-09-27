package main

// Real-site corpus vectors (-mode corpus): numeric tokens found in the
// seeksnack sources and in the golden Go build output, through ParseFloat,
// FormatFloat, ParseInt and Atoi (corpus_numbers.txt), and every line of
// every text source file through the quoting functions (one hash per file,
// corpus_quote.txt; the Rust test needs GO_STRCONV_SITE=<site dir>).

import (
	"bufio"
	"fmt"
	"io/fs"
	"log"
	"math"
	"os"
	"path/filepath"
	"regexp"
	"sort"
	"strconv"
	"strings"
)

var corpusNumRe = regexp.MustCompile(`[-+]?(0[xX][0-9a-fA-F_.]+([pP][-+]?[0-9]+)?|[0-9][0-9_]*(\.[0-9_]*)?([eE][-+]?[0-9]+)?|\.[0-9]+([eE][-+]?[0-9]+)?)`)

var corpusSmallInt = regexp.MustCompile(`^[0-9]{1,6}$`)

var corpusTextExt = map[string]bool{
	".md": true, ".yaml": true, ".yml": true, ".toml": true, ".json": true,
	".html": true, ".xml": true, ".js": true, ".css": true, ".scss": true, ".txt": true,
}

// corpusFiles lists the text files under root (skipping node_modules and
// dot directories), as slash-separated paths relative to root, sorted.
func corpusFiles(root string, dirs []string) []string {
	var out []string
	for _, d := range dirs {
		base := filepath.Join(root, d)
		if _, err := os.Stat(base); err != nil {
			continue
		}
		err := filepath.WalkDir(base, func(p string, e fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if e.IsDir() {
				if n := e.Name(); n == "node_modules" || (strings.HasPrefix(n, ".") && p != base) {
					return filepath.SkipDir
				}
				return nil
			}
			if corpusTextExt[strings.ToLower(filepath.Ext(p))] {
				rel, err := filepath.Rel(root, p)
				if err != nil {
					return err
				}
				out = append(out, filepath.ToSlash(rel))
			}
			return nil
		})
		if err != nil {
			log.Fatal(err)
		}
	}
	sort.Strings(out)
	return out
}

var corpusSiteDirs = []string{"content", "data", "i18n", "layouts", "archetypes", "assets", "hugo.toml"}

func writeCorpus(site, golden, dir string) {
	tokens := map[string]bool{}
	collect := func(root string, dirs []string, skipSmallInts bool) {
		for _, rel := range corpusFiles(root, dirs) {
			data, err := os.ReadFile(filepath.Join(root, rel))
			if err != nil {
				log.Fatal(err)
			}
			for _, t := range corpusNumRe.FindAll(data, -1) {
				// Plain integers of up to 6 digits from the (large) golden
				// output add little and would dominate the fixture.
				if len(t) <= 40 && !(skipSmallInts && corpusSmallInt.Match(t)) {
					tokens[string(t)] = true
				}
			}
		}
	}
	collect(site, corpusSiteDirs, false)
	if golden != "" {
		collect(golden, []string{"."}, true)
	}
	var list []string
	for t := range tokens {
		list = append(list, t)
	}
	sort.Strings(list)

	f, w := create(dir, "corpus_numbers.txt")
	fmt.Fprintf(w, "# numeric tokens from the seeksnack sources and golden output\n")
	fmt.Fprintf(w, "# token pf64bits pf64err pf32bits pf32err g-1 f-1 e-1 f2 g-1/32 pi10 pi10err pi0 pi0err atoi atoierr pu0 pu0err\n")
	for _, t := range list {
		v64, e64 := strconv.ParseFloat(t, 64)
		v32, e32 := strconv.ParseFloat(t, 32)
		pi10, epi10 := strconv.ParseInt(t, 10, 64)
		pi0, epi0 := strconv.ParseInt(t, 0, 64)
		a, ea := strconv.Atoi(t)
		pu0, epu0 := strconv.ParseUint(t, 0, 32)
		fmt.Fprintf(w, "%s\t%016x\t%s\t%016x\t%s\t%s\t%s\t%s\t%s\t%s\t%d\t%s\t%d\t%s\t%d\t%s\t%d\t%s\n",
			t,
			math.Float64bits(v64), errCode(e64),
			math.Float64bits(v32), errCode(e32),
			strconv.FormatFloat(v64, 'g', -1, 64),
			strconv.FormatFloat(v64, 'f', -1, 64),
			strconv.FormatFloat(v64, 'e', -1, 64),
			strconv.FormatFloat(v64, 'f', 2, 64),
			strconv.FormatFloat(v32, 'g', -1, 32),
			pi10, errCode(epi10), pi0, errCode(epi0), a, errCode(ea), pu0, errCode(epu0))
	}
	closeW(f, w)

	f, w = create(dir, "corpus_quote.txt")
	fmt.Fprintf(w, "# per-file fnv1a64 of quoting every line (and the whole file) of the seeksnack text sources\n")
	for _, rel := range corpusFiles(site, corpusSiteDirs) {
		data, err := os.ReadFile(filepath.Join(site, rel))
		if err != nil {
			log.Fatal(err)
		}
		s := newSink(nil)
		corpusQuoteFile(s, data)
		fmt.Fprintf(w, "%s\t%d\t%016x\n", rel, len(data), s.h)
	}
	closeW(f, w)
}

func corpusQuoteFile(s *sink, data []byte) {
	s.putStr(strconv.Quote(string(data)))
	s.putStr(strconv.QuoteToASCII(string(data)))
	sc := bufio.NewScanner(strings.NewReader(string(data)))
	sc.Buffer(make([]byte, 1<<20), 1<<24)
	for sc.Scan() {
		line := sc.Text()
		q := strconv.Quote(line)
		s.putStr(q)
		s.putStr(strconv.QuoteToASCII(line))
		s.putStr(strconv.QuoteToGraphic(line))
		s.putBool(strconv.CanBackquote(line))
		u, err := strconv.Unquote(q)
		s.putStr(u)
		s.putErr(err)
		u, err = strconv.Unquote(line)
		s.putStr(u)
		s.putErr(err)
		p, err := strconv.QuotedPrefix(line)
		s.putStr(p)
		s.putErr(err)
	}
	if err := sc.Err(); err != nil {
		log.Fatal(err)
	}
}
