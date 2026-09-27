package main

import (
	"bytes"
	"fmt"
	"math/rand"
	"os"
	"regexp"
	"strings"

	"github.com/yuin/goldmark/text"
	"github.com/yuin/goldmark/util"
)

// Copies of goldmark's unexported regular expressions (parser/html_block.go,
// parser/raw_html.go, util/util.go). The Rust port replaces them with
// hand-written matchers; these vectors pin the equivalence.
var (
	tagnamePattern    = `([A-Za-z][A-Za-z0-9-]*)`
	spaceOrOneNewline = `(?:[ \t]|(?:\r\n|\n){0,1})`
	attributePattern  = `(?:[\r\n \t]+[a-zA-Z_:][a-zA-Z0-9:._-]*(?:[\r\n \t]*=[\r\n \t]*(?:[^\"'=<>` + "`" + `\x00-\x20]+|'[^']*'|"[^"]*"))?)`
	openTagRegexp     = regexp.MustCompile("^<" + tagnamePattern + attributePattern + `*` + spaceOrOneNewline + `*/?>`)
	closeTagRegexp    = regexp.MustCompile("^</" + tagnamePattern + spaceOrOneNewline + `*>`)

	htmlBlockType1OpenRegexp  = regexp.MustCompile(`(?i)^[ ]{0,3}<(script|pre|style|textarea)(?:\s.*|>.*|/>.*|)(?:\r\n|\n)?$`)
	htmlBlockType1CloseRegexp = regexp.MustCompile(`(?i)^.*</(?:script|pre|style|textarea)>.*`)
	htmlBlockType2OpenRegexp  = regexp.MustCompile(`^[ ]{0,3}<!\-\-`)
	htmlBlockType3OpenRegexp  = regexp.MustCompile(`^[ ]{0,3}<\?`)
	htmlBlockType4OpenRegexp  = regexp.MustCompile(`^[ ]{0,3}<![A-Z]+.*(?:\r\n|\n)?$`)
	htmlBlockType5OpenRegexp  = regexp.MustCompile(`^[ ]{0,3}<\!\[CDATA\[`)
	htmlBlockType6Regexp      = regexp.MustCompile(`^[ ]{0,3}<(?:/[ ]*)?([a-zA-Z]+[a-zA-Z0-9\-]*)(?:[ ].*|>.*|/>.*|)(?:\r\n|\n)?$`)
	htmlBlockType7Regexp      = regexp.MustCompile(`^[ ]{0,3}<(/[ ]*)?([a-zA-Z]+[a-zA-Z0-9\-]*)(` + attributePattern + `*)[ ]*(?:>|/>)[ ]*(?:\r\n|\n)?$`)
	emailDomainRegexp         = regexp.MustCompile(`^[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*`)
)

var regexAlphabet = []string{
	"<", ">", "/", "/>", " ", "  ", "\t", "\n", "\r\n", "\r", "\x0c", "\x0b", "a", "b", "Z", "x-y", "1", "-", "_", ":",
	".", "=", "'", "\"", "`", "!", "?", "--", "!--", "![CDATA[", "!DOCTYPE", "!a", "script", "SCRIPT", "ſcript",
	"pre", "PrE", "style", "textarea", "div", "DIV", "table", "custom-el", "h1", "p", "li", "href", "x=y", "ü",
	"\xff", "\x00", " ", "{", "}", "&", "#", "@", "%", "ไ", "�", "K", "k",
}

func regexInputs(r *rand.Rand, n int) []string {
	seeds := []string{
		"<div>", "<div>\n", "<div >", "<div/>", "</div>", "</ div>", "<div class=\"a\">", "<div\tclass=a>",
		"<a href=\"x\">", "<a href='x'>", "<a href=x>", "<a href=x/>", "<a b=c/ >", "<a b='>' >", "<a b>",
		"<a  b  =  c>", "<a b=\"\n\">", "<x-y z='1'>\n", "</a b>", "<a/>", "<a />", "<a/ >", "<a\n>", "<a\n\nb>",
		"<script>", "<script>\n", "<SCRIPT >x", "<ſcript>", "<pre", "<pre\r", "<pre\r\n", "<prex>", "<style/>",
		"<textarea\x0c", "   <div>", "    <div>", "<!-- x", "<?php", "<!DOCTYPE html>", "<!doctype>", "<![CDATA[",
		"x</script>y", "</SCRIPT>", "</ſtyle>", "</pre >", "<a b=c d=e f>", "<a b=\"c\"d>", "<a b='c'\ne='f'>",
		"<a\r\nb>", "<a\r\n\r\nb>", "<a\tb=\tc>", "<a b=`c`>", "<a b=c=d>", "<a :b>", "<a _b>", "<a -b>", "<1a>",
		"<a b\n=\nc>", "<abc", "<", "</", "</a", "</a\n>", "</a\n\n>", "</a\t>",
	}
	ret := append([]string{}, seeds...)
	for i := 0; i < n; i++ {
		var b strings.Builder
		if r.Intn(2) == 0 {
			b.WriteString([]string{"", " ", "  ", "   ", "    "}[r.Intn(5)])
		}
		b.WriteString([]string{"<", "</", "<!", "<?", ""}[r.Intn(5)])
		k := r.Intn(10)
		for j := 0; j < k; j++ {
			b.WriteString(regexAlphabet[r.Intn(len(regexAlphabet))])
		}
		ret = append(ret, b.String())
	}
	return ret
}

func idx(m []int) []byte {
	return []byte(fmt.Sprint(m))
}

func writeRegexVectors(g *gmfWriter) {
	r := rand.New(rand.NewSource(11))
	for i, s := range regexInputs(r, 6000) {
		v := []byte(s)
		g.record(fmt.Sprintf("regex/%d", i))
		g.field("in", v)
		g.field("type1open", boolb(htmlBlockType1OpenRegexp.Match(v)))
		g.field("type1close", boolb(htmlBlockType1CloseRegexp.Match(v)))
		g.field("type2open", boolb(htmlBlockType2OpenRegexp.Match(v)))
		g.field("type3open", boolb(htmlBlockType3OpenRegexp.Match(v)))
		g.field("type4open", boolb(htmlBlockType4OpenRegexp.Match(v)))
		g.field("type5open", boolb(htmlBlockType5OpenRegexp.Match(v)))
		if m := htmlBlockType6Regexp.FindSubmatchIndex(v); m != nil {
			g.field("type6", v[m[2]:m[3]])
		} else {
			g.field("type6", []byte("nil"))
		}
		if m := htmlBlockType7Regexp.FindSubmatchIndex(v); m != nil {
			isCloseTag := m[2] > -1 && bytes.Equal(v[m[2]:m[3]], []byte("/"))
			hasAttr := m[6] != m[7]
			g.field("type7", []byte(fmt.Sprintf("%v %v %s", isCloseTag, hasAttr, v[m[4]:m[5]])))
		} else {
			g.field("type7", []byte("nil"))
		}
		// open/close tag through a text.Reader (multi-line) and through a
		// BlockReader whose segments are the lines with a padding of 1 on
		// the second line.
		for _, re := range []struct {
			name string
			re   *regexp.Regexp
		}{{"opentag", openTagRegexp}, {"closetag", closeTagRegexp}} {
			rd := text.NewReader(v)
			ok := rd.Match(re.re)
			l, pos := rd.Position()
			g.field(re.name, []byte(fmt.Sprintf("%v %d %d %d %d", ok, l, pos.Start, pos.Stop, pos.Padding)))

			segs := text.NewSegments()
			start := 0
			for li := 0; start < len(v); li++ {
				e := bytes.IndexByte(v[start:], '\n')
				stop := len(v)
				if e >= 0 {
					stop = start + e + 1
				}
				pad := 0
				if li == 1 {
					pad = 1
				}
				segs.Append(text.NewSegmentPadding(start, stop, pad))
				start = stop
			}
			br := text.NewBlockReader(v, segs)
			ok = br.Match(re.re)
			l, pos = br.Position()
			g.field(re.name+"/block", []byte(fmt.Sprintf("%v %d %d %d %d", ok, l, pos.Start, pos.Stop, pos.Padding)))
		}
	}
	// email domains
	dom := []string{"a", "b", "0", "-", ".", "x", "Z", "_", "@", "ü"}
	for i := 0; i < 3000; i++ {
		var b strings.Builder
		k := r.Intn(12)
		if r.Intn(10) == 0 {
			k = 60 + r.Intn(10)
		}
		for j := 0; j < k; j++ {
			b.WriteString(dom[r.Intn(len(dom))])
		}
		v := []byte(b.String())
		g.record(fmt.Sprintf("email/%d", i))
		g.field("in", v)
		g.field("domain", idx(emailDomainRegexp.FindSubmatchIndex(v)))
		g.field("FindEmailIndex", itoa(util.FindEmailIndex(append([]byte("ab@"), v...))))
	}
}

func regexMain(args []string) {
	g := newGMF(os.Stdout)
	writeRegexVectors(g)
	g.flush()
}
