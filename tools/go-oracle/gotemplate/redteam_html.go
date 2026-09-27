//go:build gotemplate_oracle

// Red-team modes for html/template, written in the htmlexec script format
// (htmlexec.go) so crates/gotemplate/tests/html_exec.rs can replay them:
//
//	rthtml <out.txt.gz> <seed> <n>  random HTML skeletons with actions in
//	                                every escaping context: parse, escape
//	                                (error text and ErrorCode), the escaped
//	                                trees of the namespace, execution with
//	                                several values (plain and Hugo path)
//	rtns   <out.txt.gz> <seed> <n>  random sequences of namespace operations
//	                                (New, Parse, Lookup, AddParseTree, Clone,
//	                                CloneShallow, Funcs, Delims, Option,
//	                                Execute*, ...)
//
// Not checked in: html_exec.rs's ignored test html_redteam reads them from
// $GOTEMPLATE_RT_HTML.
//
// Extra operations (rtRun; the Rust interpreter has the same):
//
//	O execc V DATA   => output, error, html/template ErrorCode ("" if none)
//	O prepare V      => ok | err, message, ErrorCode (Hugo's Prepare: escape)
//	O dump V         => N, then name and Root.String() ("<nil>") of every
//	                    template of the text namespace, sorted by name
package main

import (
	"bufio"
	"bytes"
	"errors"
	"fmt"
	"math/rand"
	"os"
	"sort"
	"strconv"
	"strings"

	htmltemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/htmltemplate"
	"github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate/parse"
)

func init() {
	register("rthtml", rtHTMLMain)
	register("rtns", rtNSMain)
}

func rtHTMLMain(args []string) error {
	out, seed, n, err := rtArgs("rthtml", args)
	if err != nil {
		return err
	}
	r := rand.New(rand.NewSource(seed))
	var scripts []*script
	for i := 0; i < n; i++ {
		scripts = append(scripts, rtHTMLScript(r, i))
	}
	fmt.Fprintf(os.Stderr, "rthtml: seed %d, %d scripts\n", seed, n)
	return rtWriteGz(out, func(w *bufio.Writer) error { return rtWriteScripts(w, scripts) })
}

func rtNSMain(args []string) error {
	out, seed, n, err := rtArgs("rtns", args)
	if err != nil {
		return err
	}
	r := rand.New(rand.NewSource(seed))
	var scripts []*script
	for i := 0; i < n; i++ {
		scripts = append(scripts, rtNSScript(r, i))
	}
	fmt.Fprintf(os.Stderr, "rtns: seed %d, %d scripts\n", seed, n)
	return rtWriteGz(out, func(w *bufio.Writer) error { return rtWriteScripts(w, scripts) })
}

// rtWriteScripts is writeScripts (htmlexec.go) with rtRun's extra
// operations.
func rtWriteScripts(w *bufio.Writer, scripts []*script) error {
	index := map[string]int{}
	var table []string
	for _, s := range scripts {
		for _, o := range s.ops {
			for i, a := range o.args {
				if !rtIsDataArg(o.kind, i) {
					continue
				}
				if _, ok := index[a]; !ok {
					index[a] = len(table)
					table = append(table, a)
				}
			}
		}
	}
	_, _ = fmt.Fprintf(w, "# gotemplate red-team htmlexec scripts: D <i> <spec> / S <name> / O <op> <args> => <results> / E\n")
	for i, spec := range table {
		_, _ = fmt.Fprintf(w, "D\t%d\t%s\n", i, spec)
	}
	for _, s := range scripts {
		in := &interp{t: map[string]*htmltemplate.Template{}, tr: map[string]*parse.Tree{}, data: table}
		_, _ = fmt.Fprintf(w, "S\t%s\n", q(s.name))
		for _, o := range s.ops {
			args := make([]string, len(o.args))
			for i, a := range o.args {
				if rtIsDataArg(o.kind, i) {
					args[i] = "@" + strconv.Itoa(index[a])
				} else {
					args[i] = a
				}
			}
			res := rtRun(in, &sop{kind: o.kind, args: args})
			_, _ = fmt.Fprintf(w, "O\t%s", o.kind)
			for _, a := range args {
				_, _ = fmt.Fprintf(w, "\t%s", q(a))
			}
			_, _ = fmt.Fprintf(w, "\t=>")
			for _, r := range res {
				_, _ = fmt.Fprintf(w, "\t%s", q(r))
			}
			_, _ = fmt.Fprintln(w)
		}
		_, _ = fmt.Fprintln(w, "E")
	}
	return nil
}

func rtIsDataArg(kind string, i int) bool {
	switch kind {
	case "execc", "texec":
		return i == 1
	case "texectmpl":
		return i == 2
	}
	return isDataArg(kind, i)
}

func rtErrCode(err error) string {
	var e *htmltemplate.Error
	if errors.As(err, &e) {
		return strconv.Itoa(int(e.ErrorCode))
	}
	return ""
}

// rtRun runs one operation (the extra ones here, the others in in.run).
func rtRun(in *interp, o *sop) (res []string) {
	defer func() {
		if r := recover(); r != nil {
			res = []string{"PANIC", fmt.Sprint(r)}
		}
	}()
	if res, ok := rtRunText(in, o); ok {
		return res
	}
	a := o.args
	switch o.kind {
	case "funcs":
		if a[1] == "stubs" {
			in.tmpl(a[0]).Funcs(rtStubFuncs(a[2]))
			return nil
		}
	case "execc":
		var b bytes.Buffer
		err := in.tmpl(a[0]).Execute(&b, in.decodeData(a[1]))
		return []string{b.String(), errString(err), rtErrCode(err)}
	case "prepare":
		_, err := in.tmpl(a[0]).Prepare()
		if err != nil {
			return []string{"err", err.Error(), rtErrCode(err)}
		}
		return []string{"ok"}
	case "dump":
		ts := in.tmpl(a[0]).TextTemplate().Templates()
		sort.Slice(ts, func(i, j int) bool { return ts[i].Name() < ts[j].Name() })
		res := []string{strconv.Itoa(len(ts))}
		for _, t := range ts {
			s := "<nil>"
			if t.Tree != nil && t.Root != nil {
				s = t.Root.String()
			}
			res = append(res, t.Name(), s)
		}
		return res
	case "addtree":
		// A nil tree (failed parsetree, template without a tree) is not
		// representable in Rust (AddParseTree takes a tree): skip.
		var tree *parse.Tree
		switch {
		case strings.HasPrefix(a[3], "tree:"):
			tree = in.tr[a[3][5:]]
		case strings.HasPrefix(a[3], "of:"):
			tree = in.tmpl(a[3][3:]).Tree
		}
		if tree == nil {
			return []string{"NOTREE"}
		}
	}
	return in.run(o)
}

// ---------------------------------------------------------------------------
// rthtml: random HTML skeletons

type rtHTMLGen struct {
	r        *rand.Rand
	defs     []string
	vars     []string
	unquoted bool // generating an unquoted attribute value
}

func (g *rtHTMLGen) pick(ss ...string) string { return ss[g.r.Intn(len(ss))] }

// text picks a text piece; in an unquoted attribute value without the
// characters that end it.
func (g *rtHTMLGen) text(ss ...string) string {
	s := g.pick(ss...)
	if g.unquoted {
		s = strings.NewReplacer(" ", "", "\n", "", "\t", "", "\"", "", "'", "", ">", "").Replace(s)
	}
	return s
}

// action returns an action pipeline (without delimiters).
func (g *rtHTMLGen) pipe() string {
	if g.r.Intn(3) == 0 {
		return "."
	}
	switch g.r.Intn(12) {
	case 0:
		return g.pick(`"<b>&'\"x"`, `"javascript:alert(1)"`, `"a b"`, `"/p?q=1&r=2"`, `""`, `"</script>"`, `"x y"`,
			"`${x}`", `"red"`, `"1.5em"`, `"a,b 2x"`, `"\xff"`, `"é"`, `" "`, `"-->"`, `"/*"`, `"'"`)
	case 1:
		return g.pick("1", "-1", "1.5", "true", "false", "0x10", "'a'")
	case 2:
		if g.r.Intn(4) == 0 {
			// Predefined escapers (errors in most contexts).
			return g.pick("html .", ". | html", "urlquery .", ". | urlquery", ". | print | html", "html . .",
				". | html | urlquery", "urlquery . | html")
		}
		return g.pick("js .", ". | js", "print .", "printf \"%s\" .", "printf \"%q\" .", "printf \"%v|%T\" . .",
			"println .", "len .", "index . 0", "print . .", "slice . 1")
	case 3:
		return g.pick("safeHTML .", "safeURL .", "safeCSS .", "safeJS .", "safeJSStr .", "safeHTMLAttr .",
			"safeSrcset .", "safeHTML \"<i>x</i>\"", "safeURL \"javascript:ok\"", "safeCSS \"color:red\"",
			"safeJS \"a+b\"", "safeHTMLAttr \"title=x\"", "safeJSStr \"it's\"", "safeSrcset \"a.png 1x\"")
	case 4:
		if len(g.vars) > 0 {
			return g.vars[g.r.Intn(len(g.vars))]
		}
	case 5:
		return g.pick(".Foo", ".Bar", ".title", ".Title", "$", "$.Foo")
	case 6:
		return g.pick("and . \"x\"", "or . \"y\"", "not .", "eq . 1", "ne . \"\"")
	}
	return "."
}

func (g *rtHTMLGen) act() string {
	switch g.r.Intn(10) {
	case 0:
		return "{{- " + g.pipe() + " -}}"
	case 1:
		return "{{" + g.pipe() + " -}}"
	}
	return "{{" + g.pipe() + "}}"
}

// ctrl wraps a fragment in a control structure; the branches may end in
// different contexts (escaper errors).
func (g *rtHTMLGen) ctrl(depth int, frag func(int) string) string {
	switch g.r.Intn(8) {
	case 0:
		return "{{if " + g.pipe() + "}}" + frag(depth+1) + "{{end}}"
	case 1:
		return "{{if " + g.pipe() + "}}" + frag(depth+1) + "{{else}}" + frag(depth+1) + "{{end}}"
	case 2:
		return "{{with " + g.pipe() + "}}" + frag(depth+1) + "{{else with " + g.pipe() + "}}" + frag(depth+1) + "{{end}}"
	case 3:
		v := fmt.Sprintf("$v%d", len(g.vars))
		g.vars = append(g.vars, v)
		s := "{{range $i, " + v + " := " + g.pick(".", "$", "(slice \"a\" \"b\")") + "}}" + frag(depth+1)
		if g.r.Intn(3) == 0 {
			s += "{{if " + v + "}}{{" + g.pick("break", "continue") + "}}{{end}}" + frag(depth+1)
		}
		g.vars = g.vars[:len(g.vars)-1]
		if g.r.Intn(3) == 0 {
			s += "{{else}}" + frag(depth+1)
		}
		return s + "{{end}}"
	case 4:
		v := fmt.Sprintf("$w%d", len(g.vars))
		s := "{{" + v + " := " + g.pipe() + "}}"
		g.vars = append(g.vars, v)
		s += frag(depth + 1)
		g.vars = g.vars[:len(g.vars)-1]
		return s
	case 5:
		if len(g.defs) > 0 {
			return "{{template " + strconv.Quote(g.defs[g.r.Intn(len(g.defs))]) + " " + g.pipe() + "}}"
		}
	case 6:
		return "{{/* c */}}" + frag(depth+1)
	}
	return "{{if " + g.pipe() + "}}" + frag(depth+1) + "{{else if " + g.pipe() + "}}" + frag(depth+1) + "{{end}}"
}

var rtTags = []string{"a", "p", "div", "img", "input", "iframe", "form", "option", "meta", "link", "source", "button",
	"span", "body", "svg", "object", "embed", "video", "audio", "area", "base", "blockquote", "A", "IMG", "td",
	"h1", "noscript", "template", "math", "xmp", "frame", "applet"}

var rtAttrNames = []string{"href", "src", "title", "class", "style", "onclick", "onload", "srcset", "data-x", "action",
	"xlink:href", "value", "alt", "id", "type", "content", "http-equiv", "rel", "poster", "formaction", "background",
	"cite", "codebase", "data", "longdesc", "usemap", "dir", "lang", "name", "target", "g:tweetUrl", "xmlns",
	"xmlns:x", "on", "onFoo", "data-src", "data-href", "style-x", "HREF", "SRC", "Style", "OnClick", "srcSet",
	"my-url", "xml:lang", "accept", "checked", "manifest", "icon", "profile", "archive", "classid", "ping",
	"data-uri", "fooURL", "fooUri", "urlfoo", "svg:href", "xmlns:href", "src2", "sizes", "imagesrcset"}

// attrValue builds an attribute value body (between quotes).
func (g *rtHTMLGen) attrValue(depth int) string {
	var b strings.Builder
	for i := g.r.Intn(4); i >= 0; i-- {
		switch g.r.Intn(9) {
		case 0, 1, 2:
			b.WriteString(g.act())
		case 3:
			b.WriteString(g.text("http://x.com/", "/p/", "?q=", "&r=", "#f", "javascript:", "mailto:", "data:",
				"a b", "x", "1x, ", " 2x", "color: ", "url(", ")", "'", "\"", "&amp;", "&quot;", "&#x6a;", "\\",
				"/*", "*/", "expression(", ";", "{", "}", "<", ">", "=", "`", "${", "&#39;", "%20", " ", ""))
		case 4:
			if depth < 3 {
				b.WriteString(g.ctrl(depth, g.attrValue))
			}
		case 5:
			b.WriteString(g.text("f(", "alert(", "x=", "'a'+", "\"b\"", "/re/", "//c\n", "return ", "a/b"))
		default:
			b.WriteString(g.act())
		}
	}
	return b.String()
}

func (g *rtHTMLGen) attr(depth int) string {
	name := rtAttrNames[g.r.Intn(len(rtAttrNames))]
	switch g.r.Intn(12) {
	case 0:
		name = g.act()
	case 1:
		name = name + g.act()
	case 2:
		return " " + name
	case 3:
		return " " + g.act()
	case 4:
		if depth < 3 {
			return " " + g.ctrl(depth, func(d int) string { return g.attr(d) })
		}
	}
	sp := g.pick("", "", "", " ", "\n")
	switch g.r.Intn(8) {
	case 0:
		return " " + name + sp + "=" + sp + "'" + g.attrValue(depth) + g.pick("'", "'", "'", "")
	case 1, 2:
		g.unquoted = true
		v := g.attrValue(depth)
		g.unquoted = false
		return " " + name + "=" + v
	}
	return " " + name + sp + "=" + sp + "\"" + g.attrValue(depth) + g.pick("\"", "\"", "\"", "\"", "")
}

var rtJSTypes = []string{"", "", "", "text/javascript", "module", "application/json", "application/ld+json",
	"text/template", "x-tmpl-mustache", "text/babel", "JavaScript", "application/ecmascript", "text/html",
	"importmap", "speculationrules", "\"\"", "text/javascript1.5"}

func (g *rtHTMLGen) jsBody(depth int) string {
	var b strings.Builder
	for i := g.r.Intn(8); i >= 0; i-- {
		switch g.r.Intn(14) {
		case 0, 1, 2:
			b.WriteString(g.act())
		case 3:
			b.WriteString(g.pick("\"", "'", "`", "\"a", "'b", "`c", "\\", "\\\"", "\\'", "\\`", "\\\\"))
		case 4:
			b.WriteString(g.pick("/", "/re/", "/[", "]", "/g", "x / ", "return /", "(/", "= /", "++ /", ") /",
				"} /", "] /", "typeof /", "in /", "/*", "*/", "//", "\n", "<!--", "-->", "</script", "<\\/script>",
				"<script", "</SCRIPT", "</scriptx"))
		case 5:
			b.WriteString(g.pick("${", "}", "{", "(", ")", ";", ",", "=", ":", "?", "!", "var x = ", "if (x) ",
				"function f() {", "=> ", "x.y", "a[0]", "1.5", "0x1f", "null", " ", "\t", " ", " "))
		case 6:
			if depth < 3 {
				b.WriteString(g.ctrl(depth, g.jsBody))
			}
		case 7:
			b.WriteString("`a${ " + g.act() + " }b`")
		case 8:
			b.WriteString("\"" + g.act() + "\"")
		case 9:
			b.WriteString("'" + g.act() + "'")
		case 10:
			b.WriteString("/" + g.act() + "/")
		case 11:
			b.WriteString("{\"k\": " + g.act() + "}")
		default:
			b.WriteString(g.act())
		}
	}
	return b.String()
}

func (g *rtHTMLGen) cssBody(depth int) string {
	var b strings.Builder
	for i := g.r.Intn(6); i >= 0; i-- {
		switch g.r.Intn(12) {
		case 0, 1, 2:
			b.WriteString(g.act())
		case 3:
			b.WriteString(g.pick("p { color: ", " }", "url(", ")", "url('", "')", "url(\"", "\")", "\"", "'", "\\",
				"/*", "*/", "//", "\n", "@import ", ";", "{", "}", "expression(", "font-family: ", "-->", "<!--",
				"</style", "</STYLE>", " ", ":", "#", ".c", "\\22", "\\\n"))
		case 4:
			if depth < 3 {
				b.WriteString(g.ctrl(depth, g.cssBody))
			}
		case 5:
			b.WriteString("url(" + g.act() + ")")
		case 6:
			b.WriteString("\"" + g.act() + "\"")
		case 7:
			b.WriteString("/* " + g.act() + " */")
		default:
			b.WriteString(g.act())
		}
	}
	return b.String()
}

func (g *rtHTMLGen) fragment(depth int) string {
	var b strings.Builder
	for i := g.r.Intn(5); i >= 0; i-- {
		switch g.r.Intn(22) {
		case 0, 1, 2:
			b.WriteString(g.act())
		case 3:
			b.WriteString(g.pick("x", " ", "\n", "a < b", "&amp;", "<", ">", "<br/>", "</p>", "</a>", "<!DOCTYPE html>",
				"<?xml ?>", "é", "\xff", "&", "=", "\"", "'", "<3", "<a", "</", "<!", "<!-", "</div >"))
		case 4, 5, 6:
			tag := rtTags[g.r.Intn(len(rtTags))]
			if g.r.Intn(10) == 0 {
				tag = g.pick("h"+g.act(), g.act(), "a"+g.act())
			}
			b.WriteString("<" + tag)
			for j := g.r.Intn(3); j > 0; j-- {
				b.WriteString(g.attr(depth))
			}
			b.WriteString(g.pick(">", ">", ">", " />", "/>", "", " >"))
		case 7, 8:
			typ := rtJSTypes[g.r.Intn(len(rtJSTypes))]
			open := "<script>"
			if typ != "" {
				open = "<script type=\"" + typ + "\">"
			}
			if g.r.Intn(8) == 0 {
				open = g.pick("<SCRIPT>", "<script src=\""+g.act()+"\">", "<script async>", "<script\n>",
					"<script type="+g.act()+">", "<script type=\"text/javascript\" "+g.act()+">")
			}
			b.WriteString(open + g.jsBody(depth) + g.pick("</script>", "</script>", "</script>", "</SCRIPT>", ""))
		case 9:
			b.WriteString(g.pick("<style>", "<STYLE>", "<style type=\"text/css\">") + g.cssBody(depth) +
				g.pick("</style>", "</style>", "</Style >", ""))
		case 10:
			b.WriteString("<p style=\"" + g.cssBody(depth) + "\">")
		case 11:
			tag := g.pick("textarea", "title", "TEXTAREA", "Title")
			b.WriteString("<" + tag + ">" + g.pick("", "<b>", "</x>") + g.act() + g.pick("", "</textarea", "&lt;") +
				g.pick("</"+tag+">", "</"+strings.ToLower(tag)+">", ""))
		case 12:
			b.WriteString("<!--" + g.pick(" ", "", "-") + g.act() + g.pick(" -->", "-->", "", "--!>", " --"))
		case 13:
			b.WriteString("<a onclick=\"" + g.jsBody(depth) + "\">")
		case 14, 15:
			if depth < 3 {
				b.WriteString(g.ctrl(depth, g.fragment))
			}
		case 16:
			b.WriteString("<img srcset=\"" + g.attrValue(depth) + "\">")
		case 17:
			b.WriteString("<a href=\"" + g.attrValue(depth) + "\">")
		case 18:
			// Escaper error paths: range loop re-entry, missing templates,
			// output contexts of recursive templates.
			s := g.pick(
				"{{range .}}<a title=\"{{.}}{{end}}\">",
				"{{range .}}<script>{{.}}/{{end}}</script>",
				"{{range .}}{{if .}}<b {{end}}{{.}}{{end}}",
				"{{template \"nosuch\" .}}",
				"<a href=\"{{template \"nosuch\"}}\">",
				"{{define \"rr\"}}{{if .}}{{template \"rr\" .}}{{end}}<a href=\"{{end}}{{template \"rr\" .}}",
				"{{define \"rr\"}}{{if .}}<a href=\"{{template \"rr\" .}}\">{{end}}{{end}}{{template \"rr\" .}}",
				"{{define \"rr\"}}<script>{{if .}}{{template \"rr\" .}}{{end}}</script>{{end}}<p>{{template \"rr\" .}}",
				"<script>{{range .}}`${ {{.}}{{end}} }`</script>",
				"<script>var x = `{{range .}}${ {{.}} }{{end}}`</script>",
				"<script>/{{range .}}{{.}}{{end}}/</script>",
				"{{with .}}<a onclick=\"{{.}}{{else}}<a onclick='{{.}}{{end}}'\">",
			)
			if depth > 0 && strings.Contains(s, "{{define") {
				s = g.act() // define is only allowed at the top level
			}
			b.WriteString(s)
		default:
			b.WriteString(g.act())
		}
	}
	return b.String()
}

// rtHTMLScript builds one script.
func rtHTMLScript(r *rand.Rand, i int) *script {
	g := &rtHTMLGen{r: r}
	var src strings.Builder
	for k := r.Intn(3); k > 0; k-- {
		name := fmt.Sprintf("d%d", len(g.defs))
		saved := g.vars
		g.vars = nil
		body := g.fragment(1)
		if r.Intn(6) == 0 && len(g.defs) > 0 {
			// Calls into an earlier template (in another context).
			body += "{{template " + strconv.Quote(g.defs[r.Intn(len(g.defs))]) + " .}}"
		}
		if r.Intn(8) == 0 {
			// Recursion (the escaper computes a fixed point).
			body = "{{if .}}" + body + "{{template " + strconv.Quote(name) + " \"\"}}{{end}}"
		}
		g.vars = saved
		if r.Intn(4) == 0 {
			src.WriteString("{{block " + strconv.Quote(name) + " .}}" + body + "{{end}}")
		} else {
			src.WriteString("{{define " + strconv.Quote(name) + "}}" + body + "{{end}}")
		}
		g.defs = append(g.defs, name)
	}
	src.WriteString(g.fragment(0))
	// Text ending in "{" next to an action would lex as "{{{".
	text := src.String()
	for strings.Contains(text, "{{{") {
		text = strings.ReplaceAll(text, "{{{", "{ {{")
	}
	s := newScript(fmt.Sprintf("rthtml/%d", i)).
		op("new", "t", "c").
		op("funcs", "t", "corpus").
		op("parse", "t", text)
	if r.Intn(2) == 0 {
		s.op("prepare", "t")
	}
	s.op("dump", "t")
	for k := 1 + r.Intn(3); k > 0; k-- {
		s.op("execc", "t", corpusValues[r.Intn(len(corpusValues))])
	}
	if r.Intn(3) == 0 {
		s.op("exechugo", "t", hugoValues[r.Intn(len(hugoValues))])
	}
	if len(g.defs) > 0 && r.Intn(3) == 0 {
		s.op("exectmpl", "t", g.defs[r.Intn(len(g.defs))], corpusValues[r.Intn(len(corpusValues))])
		s.op("dump", "t")
	}
	return s
}

// ---------------------------------------------------------------------------
// rtns: namespace operations

var rtNSSources = []string{
	`a{{.}}b`, `<a href="{{.}}">x</a>`, `{{define "x"}}X{{.}}{{end}}`, `{{define "x"}}{{end}}`, `{{define "x"}}  {{end}}`,
	`{{define "x"}}{{/* c */}}{{end}}`, `{{define "y"}}<b>{{template "x" .}}</b>{{end}}`, `{{template "x" .}}`,
	`{{template "y" .}}`, `{{template "missing" .}}`, `<script>var a = {{template "x" .}};</script>`,
	`{{block "b" .}}B{{.}}{{end}}`, `{{define "b"}}redef{{end}}`, ``, `  `, `{{/* only */}}`, `{{`, `{{end}}`,
	`<a title="{{template "x" .}}">`, `{{define "t"}}T{{end}}`, `{{define "c"}}{{.}}{{end}}`, `<p>{{template "c" .}}</p>`,
	`<a href="{{template "c" .}}">`, `{{define "x"}}<a href="{{end}}`, `{{if .}}<a{{end}}`, `[[.]]`, `<<.>>`,
	`{{define "r"}}{{if .}}{{template "r" ""}}{{end}}{{end}}{{template "r" .}}`, `{{.Missing}}`, `{{safeHTML .}}`,
	`{{define "z"}}z{{end}}{{define "x"}}new x{{end}}`, `{{template "z"}}`, `{{define "_internal/p"}}p{{end}}`,
}

var rtNSNames = []string{"t", "x", "y", "b", "c", "z", "r", "u", "", "_internal/p", "missing"}

// rtNSScript builds a random sequence of namespace operations. Template
// variables v0..vN are only used after they are defined.
func rtNSScript(r *rand.Rand, i int) *script {
	s := newScript(fmt.Sprintf("rtns/%d", i))
	vars := []string{"v0"}
	s.op("new", "v0", rtNSNames[r.Intn(4)])
	if r.Intn(2) == 0 {
		s.op("funcs", "v0", "corpus")
	}
	trees := 0
	pickVar := func() string { return vars[r.Intn(len(vars))] }
	newVar := func() string {
		v := fmt.Sprintf("v%d", len(vars))
		vars = append(vars, v)
		return v
	}
	pickName := func() string { return rtNSNames[r.Intn(len(rtNSNames))] }
	pickSrc := func() string { return rtNSSources[r.Intn(len(rtNSSources))] }
	value := func() string { return corpusValues[r.Intn(len(corpusValues))] }
	for k := 3 + r.Intn(10); k > 0; k-- {
		switch r.Intn(20) {
		case 0, 1, 2, 3:
			s.op("parse", pickVar(), pickSrc())
		case 4:
			from := pickVar()
			s.op("newassoc", newVar(), from, pickName())
		case 5:
			// lookup, clone and addtree assign their variable only on
			// success: target an existing one (replaced on success).
			s.op("lookup", pickVar(), pickVar(), pickName())
			s.op("name", pickVar())
		case 6:
			op := "clone"
			if r.Intn(2) == 0 {
				op = "cloneshallow"
			}
			if r.Intn(2) == 0 {
				// A fresh variable that holds the clone (defined first).
				v := newVar()
				s.op("new", v, pickName())
				s.op(op, v, pickVar())
			} else {
				s.op(op, pickVar(), pickVar())
			}
		case 7:
			s.op("exec", pickVar(), value())
		case 8:
			s.op("execc", pickVar(), value())
		case 9:
			s.op("exectmpl", pickVar(), pickName(), value())
		case 10:
			s.op("templates", pickVar())
		case 11:
			s.op("dump", pickVar())
		case 12:
			s.op("prepare", pickVar())
		case 13:
			tr := fmt.Sprintf("tr%d", trees)
			trees++
			src := pickSrc()
			name := pickName()
			s.op("parsetree", tr, name, src, name)
			if r.Intn(4) == 0 {
				// A tree of another template of the namespace.
				s.op("addtree", pickVar(), pickVar(), pickName(), "of:"+pickVar())
			} else {
				s.op("addtree", pickVar(), pickVar(), pickName(), "tree:"+tr)
			}
		case 14:
			d := [][2]string{{"[[", "]]"}, {"{{", "}}"}, {"<<", ">>"}, {"", ""}}[r.Intn(4)]
			s.op("delims", pickVar(), d[0], d[1])
		case 15:
			// Option panics on an empty or unknown option (in Go and Rust).
			s.op("option", pickVar(), rtExecOptions[4+r.Intn(4)])
		case 16:
			s.op("funcs", pickVar(), "corpus")
		case 17:
			s.op("exechugo", pickVar(), hugoValues[r.Intn(len(hugoValues))])
		case 18:
			a, b := pickVar(), pickVar()
			s.op("ptreq", a, b)
			s.op("treesync", a)
			s.op("hastree", b)
		default:
			s.op("new", newVar(), pickName())
		}
	}
	return s
}
