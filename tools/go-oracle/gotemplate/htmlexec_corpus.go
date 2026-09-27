//go:build gotemplate_oracle

package main

// The htmlexec differential corpus: small html templates covering every
// escaping context, each executed with every value of corpusValues.

import (
	"context"
	"fmt"
	"html/template"
	"math"
	"reflect"
	"strings"

	"github.com/neohugo/neohugo/common/hreflect"
	"github.com/neohugo/neohugo/common/maps"
	htmltemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/htmltemplate"
	texttemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate"
)

// corpusFuncs are Hugo-like safe* functions (any argument, printed with
// fmt.Sprint) so typed content also comes from function results.
var corpusFuncs = htmltemplate.FuncMap{
	"safeHTML":     func(a any) template.HTML { return template.HTML(fmt.Sprint(a)) },
	"safeHTMLAttr": func(a any) template.HTMLAttr { return template.HTMLAttr(fmt.Sprint(a)) },
	"safeCSS":      func(a any) template.CSS { return template.CSS(fmt.Sprint(a)) },
	"safeJS":       func(a any) template.JS { return template.JS(fmt.Sprint(a)) },
	"safeJSStr":    func(a any) template.JSStr { return template.JSStr(fmt.Sprint(a)) },
	"safeURL":      func(a any) template.URL { return template.URL(fmt.Sprint(a)) },
	"safeSrcset":   func(a any) template.Srcset { return template.Srcset(fmt.Sprint(a)) },
}

var corpusTemplates = []string{
	// Text.
	`{{.}}`, `<p>{{.}}</p>`, `a{{.}}b{{.}}c`, `<b>{{.}}`, `x < {{.}} > y`, `<!DOCTYPE html>{{.}}`,
	`{{.}}<br/>{{.}}`, `<p>{{- . -}}</p>`,
	// RCDATA.
	`<title>{{.}}</title>`, `<textarea>{{.}}</textarea>`, `<TITLE>{{.}}</Title>`, `<textarea title="{{.}}">{{.}}</textarea>`,
	// Attributes.
	`<a title="{{.}}">`, `<a title='{{.}}'>`, `<a title={{.}}>`, `<a title=x{{.}}>`, `<input value="{{.}}">`,
	`<div class="a {{.}} b">`, `<a data-x={{.}}>`, `<img src=x alt='{{.}}'>`, `<a title={{.}}{{.}}>`,
	`<p data-bs-target="{{.}}" id="{{.}}">`, `<meta name="description" content="{{.}}">`, `<meta http-equiv="refresh" content="0; url={{.}}">`,
	// URLs.
	`<a href="{{.}}">`, `<a href='{{.}}'>`, `<a href={{.}}>`, `<a href="/p/{{.}}">`, `<a href="/p?q={{.}}">`,
	`<a href="/p?q=1&r={{.}}#f">`, `<a href="#{{.}}">`, `<a href="{{.}}/x?y={{.}}">`, `<img src="{{.}}">`,
	`<form action="{{.}}">`, `<a href="mailto:{{.}}">`, `<a data-href="{{.}}">`, `<svg><a xlink:href="{{.}}"></a></svg>`,
	`<a href=" {{.}}">`, `<a href="https://x.com/{{.}}">`, `<a href="?{{.}}">`, `<link rel="canonical" href="{{.}}">`,
	`<a href="{{.}}{{.}}">`, `<a myurl="{{.}}">`, `<a g:tweetUrl="{{.}}">`,
	// srcset.
	`<img srcset="{{.}}">`, `<img srcset={{.}}>`, `<img srcset="{{.}} 2x">`, `<img srcset="a.png 1x, {{.}}">`,
	`<source srcset="{{.}} 200w, {{.}} 400w">`,
	// style attribute.
	`<p style="{{.}}">`, `<p style="color: {{.}}">`, `<p style='background: url({{.}})'>`,
	`<p style="background: url('{{.}}')">`, `<p style="font-family: '{{.}}'">`, `<p style={{.}}>`,
	`<p style="width: {{.}}px">`,
	// <style>.
	`<style>{{.}}</style>`, `<style>p { color: {{.}} }</style>`, `<style>p { background: url({{.}}) }</style>`,
	`<style>p { background: url("{{.}}") }</style>`, `<style>p { font-family: "{{.}}" }</style>`,
	`<style>/* {{.}} */ p {}</style>`, `<style>p { content: '{{.}}' }</style>`, `<style>// {{.}}
p { }</style>`,
	// <script>.
	`<script>{{.}}</script>`, `<script>var x = {{.}};</script>`, `<script>var x = "{{.}}";</script>`,
	`<script>var x = '{{.}}';</script>`, "<script>var x = `{{.}}`;</script>", "<script>var x = `a${ {{.}} }b`;</script>",
	`<script>var re = /{{.}}/;</script>`, `<script>var re = /x{{.}}y/g;</script>`, `<script>x = y / {{.}};</script>`,
	"<script>// {{.}}\nx = 1</script>", `<script>/* {{.}} */ x = 1</script>`, `<script>{{.}}/2</script>`,
	`<script type="application/ld+json">{{.}}</script>`,
	`<script type="application/ld+json">{"a": {{.}}, "b": "{{.}}"}</script>`,
	`<script type="text/template">{{.}}</script>`, `<script type="x-tmpl-mustache"><b>{{.}}</b></script>`,
	`<script type="module">{{.}}</script>`, `<script type="">{{.}}</script>`, `<script>alert({{.}})</script>`,
	`<script>var a = [{{.}}, {{.}}];</script>`, `<script>var o = {k: {{.}}};</script>`,
	`<script>x = "a" + {{.}} + '<\/script>';</script>`, "<script>x = 1 <!-- {{.}}\ny = 2</script>",
	// Event handlers.
	`<a onclick="{{.}}">`, `<a onclick="f({{.}})">`, `<a onclick="f('{{.}}')">`, `<a onclick='f("{{.}}")'>`,
	`<a onclick={{.}}>`, `<body onload="x=/{{.}}/">`, `<a onmouseover="f(&quot;{{.}}&quot;)">`,
	// Comments.
	`<!-- {{.}} -->x`, `a<!-- {{.}} -->b`, `{{/* c */}}<p>{{.}}</p>`,
	// Tag and attribute names.
	`<a {{.}}>`, `<a {{.}}="x">`, `<a {{.}}=x>`, `<a b {{.}}>`, `<h{{.}}>`, `<input checked {{.}}>`,
	// Predefined escapers.
	`{{. | html}}`, `{{. | urlquery}}`, `<a href="/?q={{. | urlquery}}">`, `{{html .}}`, `<a title="{{. | html}}">`,
	`{{urlquery . "x"}}`, `{{print . | html}}`, `<a href="{{. | urlquery}}">`, `<script>var x = "{{. | html}}";</script>`,
	`{{html . .}}`,
	// Builtins and typed content from functions.
	`{{printf "%v|%q" . .}}`, `<a href="{{printf "%s" .}}">`, `{{print . .}}`, `{{println .}}`, `{{printf "%T" .}}`,
	`{{js .}}`, `<a title="{{len .}}">`, `{{index . 0}}`, `{{.Foo}}`, `{{safeHTML .}}`, `<a href="{{safeURL .}}">`,
	`<p style="{{safeCSS .}}">`, `<script>var x = {{safeJS .}};</script>`, `<script>var x = "{{safeJSStr .}}";</script>`,
	`<a {{safeHTMLAttr .}}>`, `<img srcset="{{safeSrcset .}}">`, `<a title="{{safeHTML .}}">`, `<title>{{safeHTML .}}</title>`,
	`<a title={{safeHTML .}}>`, `{{if .}}{{.}}{{end}}`, `{{and . "x"}}|{{or . "y"}}|{{not .}}`,
	// Template calls in non-text contexts.
	`{{define "t"}}{{.}}{{end}}<a href="{{template "t" .}}">`,
	`{{define "t"}}{{.}}{{end}}<script>var x = {{template "t" .}}</script>`,
	`{{define "t"}}{{.}}{{end}}<p style="{{template "t" .}}">`,
	`{{define "t"}}{{.}}{{end}}<title>{{template "t" .}}</title>`,
	`{{define "t"}}"{{.}}"{{end}}<script>{{template "t" .}}</script>`,
	`{{define "t"}}{{.}}{{end}}{{template "t" .}}<a title="{{template "t" .}}">`,
	`{{block "b" .}}<i>{{.}}</i>{{end}}`,
	`{{define "t"}}<b>{{.}}</b>{{end}}<a onclick="x({{template "t" .}})">`,
	`{{define "t"}}{{if .}}{{template "t" ""}}{{else}}<i>{{.}}</i>{{end}}{{end}}{{template "t" .}}`,
	// Branches.
	`{{if .}}<a href="{{.}}">{{else}}<a href="/{{.}}">{{end}}x</a>`,
	`<a href="{{if .}}/p{{else}}/q{{end}}?x={{.}}">`, `<a href="{{if .}}/p?{{else}}/q{{end}}{{.}}">`,
	`{{with .}}<b>{{.}}</b>{{else}}none{{end}}`, `<script>{{if .}}var x = 1{{end}}/{{.}}/</script>`,
	`{{range .}}<li>{{.}}</li>{{else}}empty{{end}}`, `<script>var a = [{{range .}}{{.}},{{end}}];</script>`,
	`{{range .}}{{if .}}{{break}}{{end}}<i>{{.}}</i>{{end}}`,
	`{{range .}}{{if not .}}{{continue}}{{end}}<a href="{{.}}">{{end}}`,
	`{{range $i, $e := .}}<a title="{{$i}}:{{$e}}">{{end}}`, `<a title="{{if .}}x{{end}}{{.}}">`,
	`<p {{if .}}class="a"{{end}}>{{.}}</p>`, `<p class={{if .}}a{{else}}b{{end}}>`,
	`{{$x := .}}<a href="{{$x}}">{{$x}}</a>`, `{{with $v := .}}<b title="{{$v}}">{{end}}`,
	// Errors.
	`<a href="{{.}}`, `<script>{{.}}`, `{{if .}}<a{{end}}`, `<a title="{{.}}{{if .}}"{{end}}>`, `{{. | html | print}}`,
	`<a title={{. | html}}>`, `<script>"\{{.}}"</script>`, `<script>/[{{.}}]/</script>`, `<a "{{.}}">`,
	`<a b=c"{{.}}>`, `<a href="/foo{{if .}}?{{end}}{{.}}">`, `{{range .}}<a{{end}}`, `<p =x>{{.}}`,
	// Realistic layout snippets.
	`<meta property="og:image" content="{{.}}">`, `<link rel="alternate" hreflang="{{.}}" href="{{.}}">`,
	`<a href="{{.}}" target="_blank" rel="noopener">`, `<img src="{{.}}" alt="{{.}}" loading="lazy">`,
	`<iframe src="https://www.youtube.com/embed/{{.}}"></iframe>`, `<a href="mailto:?subject={{.}}&amp;body={{.}}">`,
	`<button data-bs-target="#{{.}}">`, `<span style="--x: {{.}}">`, `<svg><use xlink:href="#{{.}}"></use></svg>`,
	`<link href="{{.}}" rel="stylesheet">`, `<script src="{{.}}"></script>`, `<script async src="https://x/?id={{.}}"></script>`,
	`<script>gtag('config', '{{.}}');</script>`, `<noscript><img src="{{.}}"></noscript>`, `<?xml version="1.0"?>{{.}}`,
	`<script type="application/ld+json">{"@context": "https://schema.org", "name": {{.}}, "url": "{{.}}", "desc": "{{.}}"}</script>`,
	`<a title={{.}} class=x>`, `<a href={{.}}?x=1>`, `<a href="&#x6a;avascript:{{.}}">`, `<a onclick="&quot;{{.}}&quot;">`,
	`<a style="&quot;{{.}}">`, `<A HREF="{{.}}">`, `<SCRIPT>{{.}}</SCRIPT>`, `<Style>{{.}}</stYle>`,
	"<script>`${ `${ {{.}} }` }`</script>", "<script>var a = `${ {a: {{.}}} }`</script>", `<script>/[a]{{.}}/</script>`,
	`<style>@import "{{.}}";</style>`, `<style>p{background:url( '{{.}}' )}</style>`, `<textarea>{{.}}</textarea><script>{{.}}</script>`,
	`{{define "a"}}{{.}}{{template "b" .}}{{end}}{{define "b"}}<i>{{.}}</i>{{end}}<p title="{{template "a" .}}">`,
	`{{if .}}<a href="{{.}}">{{else if eq . 0}}zero{{else}}<b>{{.}}</b>{{end}}`, `<a title="<!-- {{.}} -->">`,
	`<a href="javascript:{{.}}">`, `<a href="data:text/html,{{.}}">`, `<img srcset="/a.png?w={{.}} 1x">`,
	`<input type="text" name="q" value="{{.}}" placeholder='{{.}}'>`, `<option value={{.}} selected>{{.}}</option>`,
	`<body class="{{with .}}has{{end}}">`, `<p>{{/* comment */}}{{.}}{{- /* trim */ -}} x</p>`,
	`{{range $k, $v := .}}<dt>{{$k}}</dt><dd>{{$v}}</dd>{{end}}`, `<a href="/tags/{{. | urlquery}}/">`,
	`<a href="?a={{.}}&b={{.}}">`, `<a href="#top" onclick="go('{{.}}'); return false;">`,
	// Mixed.
	`<a href="{{.}}" title="{{.}}" onclick="{{.}}" style="{{.}}">{{.}}</a>`,
	`<html><head><title>{{.}}</title><style>p{color:{{.}}}</style><script>var x={{.}};</script></head><body class="{{.}}"><a href="/s?q={{.}}">{{.}}</a></body></html>`,
}

var corpusValues = []string{
	// Strings.
	sStr(""), sStr("hello"), sStr(`<b>"&'+=`), sStr("javascript:alert(1)"), sStr("JAVASCRIPT:x"),
	sStr("http://x.com/a b?c=d&e=f#g"), sStr("/path/to"), sStr("a,b 2x"), sStr("expression(x)"), sStr("red"),
	sStr("</script><!--"), sStr("  "), sStr("\x00\xff"), sStr("日本語"), sStr("`${x}`"), sStr("1.5em"),
	sStr("#fff"), sStr("O'Reilly & Sons"), sStr("x y"), sStr("42"), sStr("a\nb\tc"), sStr("<!--x-->"),
	// Typed content.
	sSafe("html", "<b>bold</b> &amp; x"), sSafe("htmlattr", ` dir="ltr"`), sSafe("htmlattr", "onclick"),
	sSafe("css", "color: red"), sSafe("js", "alert(1)"), sSafe("jsstr", "it's"), sSafe("url", "javascript:safe()"),
	sSafe("url", "/a b,c"), sSafe("srcset", "a.png 1x, b.png 2x"), sSafe("html", ""),
	// Numbers, booleans, nils.
	sInt("int", 0), sInt("int", 42), sInt("int", -1), sInt("int64", 3), sUint("uint", 7),
	sF64(1.5), sF64(1e21), sF64(math.NaN()), sF32(0.1), "bool:1", "bool:0", "nil", sTnil("*int"),
	sTnil("[]string"), sTnil("error"),
	// Time, collections.
	"time:1599407186;955000000;utc", "time:0;0;utc",
	sList("[]string", sStr("a"), sStr("<b>")), sList("[]interface {}", sInt("int", 1), sStr("x"), "nil"),
	sList("[]string"), sList("[]int", sInt("int", 0), sInt("int", 3)),
	sMap("map[string]interface {}", "a", sInt("int", 1), "b", sStr("<x>")),
	sMap("maps.Params", "title", sStr("Hi & bye"), "n", sF64(2.5)), sBytes("abc"),
	// Objects.
	sObj("obj_str", "str<i>"), sObj("obj_err", "err&"), sObj("obj_sv", "sv"),
	"obj_pplain(" + sInt("int", 1) + "," + sStr("<two>") + ")", sObj("obj_jm", `{"a":"<1>"}`),
	sObj("obj_jme", "boom */ <script>"), sObj("obj_hs", "<em>hs</em>"), "obj_pv(" + sSafe("html", "<u>pv</u>") + ")",
	sObj("jnum", "12.5"), sObj("obj_tm", "tm<>"),
	sStruct(false, sField("Foo", sStr("<foo>")), sField("Bar", sInt("int", 1))),
}

func corpusScripts() []*script {
	var ss []*script
	seen := map[string]bool{}
	for _, src := range corpusTemplates {
		if seen[src] {
			panic("duplicate corpus template " + src)
		}
		seen[src] = true
		// The script name is the template source (stable when the corpus grows).
		s := newScript("corpus/"+src).
			op("new", "t", "c").
			op("funcs", "t", "corpus").
			op("parse", "t", src)
		for _, v := range corpusValues {
			s.op("exec", "t", v)
		}
		ss = append(ss, s)
	}
	// Hugo's execution path (texttemplate.NewExecuter(helper)): every
	// function, the escapers included, is found through the helper.
	for _, src := range append(corpusTemplates, hugoTemplates...) {
		s := newScript("hugo/"+src).
			op("new", "t", "c").
			op("funcs", "t", "corpus").
			op("parse", "t", src)
		for _, v := range hugoValues {
			s.op("exechugo", "t", v)
		}
		ss = append(ss, s)
	}
	return ss
}

// hugoTemplates are the extra templates of the Hugo-path scripts
// (case-insensitive maps.Params keys, methods found through the helper).
var hugoTemplates = []string{
	`{{.Title}}`, `<a title="{{.TITLE}}">{{.n}}</a>`, `{{.title}}|{{.N}}|{{.Missing}}`, `{{.IsZero}}`,
	`{{if .IsZero}}empty{{else}}full{{end}}`, `{{index . "Title"}}|{{index . "title"}}`,
	`<script>var t = {{.Title}};</script>`, `<a href="/{{.Title | urlquery}}">`, `{{with .Title}}<b>{{.}}</b>{{end}}`,
	`{{range $k, $v := .}}{{$k}}={{$v}};{{end}}`, `{{.Error}}`, `{{.String}}`, `{{eq .Title "Hi & bye"}}`,
	`{{call .}}`, `{{html .Title}}|{{js .Title}}`,
}

// hugoValues are the corpusValues used on the Hugo path.
var hugoValues = []string{
	sStr(""), sStr("hello"), sStr(`<b>"&'+=`), sStr("javascript:alert(1)"), sStr("http://x.com/a b?c=d&e=f#g"),
	sStr("</script><!--"), sStr("\x00\xff"),
	sSafe("html", "<b>bold</b> &amp; x"), sSafe("htmlattr", ` dir="ltr"`), sSafe("css", "color: red"),
	sSafe("js", "alert(1)"), sSafe("jsstr", "it's"), sSafe("url", "javascript:safe()"), sSafe("srcset", "a.png 1x, b.png 2x"),
	sInt("int", 0), sInt("int", 42), sF64(1.5), "bool:1", "bool:0", "nil", sTnil("*int"), sTnil("error"),
	sList("[]string", sStr("a"), sStr("<b>")), sList("[]interface {}", sInt("int", 1), sStr("x"), "nil"),
	sMap("map[string]interface {}", "a", sInt("int", 1), "b", sStr("<x>")),
	sMap("maps.Params", "title", sStr("Hi & bye"), "n", sF64(2.5)), sMap("maps.Params"),
	sObj("obj_str", "str<i>"), sObj("obj_err", "err&"), sObj("obj_hs", "<em>hs</em>"), "obj_pv(" + sSafe("html", "<u>pv</u>") + ")",
	sObj("obj_jm", `{"a":"<1>"}`), sStruct(false, sField("Foo", sStr("<foo>")), sField("Bar", sInt("int", 1))),
}

// hugoHelper mirrors Hugo's tpl/tplimpl/template_funcs.go:templateExecHelper
// (not watching) as configured by templatestore.go:configureSiteStorage:
// the functions are corpusFuncs, then htmltemplate.GoFuncs, then
// texttemplate.GoFuncs; maps.Params lookups are case-insensitive; methods
// are found by name (hreflect.GetMethodByName).
type hugoHelper struct{ funcs map[string]reflect.Value }

var hugoExecHelper = newHugoHelper(corpusFuncs)

func newHugoHelper(fm htmltemplate.FuncMap) *hugoHelper {
	funcs := make(map[string]reflect.Value)
	for k, v := range fm {
		funcs[k] = reflect.ValueOf(v)
	}
	for k, v := range htmltemplate.GoFuncs {
		if _, exists := funcs[k]; !exists {
			vv, ok := v.(reflect.Value)
			if !ok {
				vv = reflect.ValueOf(v)
			}
			funcs[k] = vv
		}
	}
	for k, v := range texttemplate.GoFuncs {
		if _, exists := funcs[k]; !exists {
			funcs[k] = v
		}
	}
	return &hugoHelper{funcs: funcs}
}

func (h *hugoHelper) Init(ctx context.Context, tmpl texttemplate.Preparer) {}

func (h *hugoHelper) GetFunc(ctx context.Context, tmpl texttemplate.Preparer, name string) (reflect.Value, reflect.Value, bool) {
	if fn, found := h.funcs[name]; found {
		return fn, reflect.Value{}, true
	}
	return reflect.Value{}, reflect.Value{}, false
}

func (h *hugoHelper) GetMapValue(ctx context.Context, tmpl texttemplate.Preparer, receiver, key reflect.Value) (reflect.Value, bool) {
	if params, ok := receiver.Interface().(maps.Params); ok {
		// Case insensitive.
		v, found := params[strings.ToLower(key.String())]
		if !found {
			return reflect.Value{}, false
		}
		return reflect.ValueOf(v), true
	}
	v := receiver.MapIndex(key)
	return v, v.IsValid()
}

func (h *hugoHelper) GetMethod(ctx context.Context, tmpl texttemplate.Preparer, receiver reflect.Value, name string) (reflect.Value, reflect.Value) {
	return hreflect.GetMethodByName(receiver, name), reflect.Value{}
}

func (h *hugoHelper) OnCalled(ctx context.Context, tmpl texttemplate.Preparer, name string, args []reflect.Value, result reflect.Value) {
}
