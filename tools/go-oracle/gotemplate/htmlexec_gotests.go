//go:build gotemplate_oracle

package main

// The fork's html/template tests (tpl/internal/go_templates/htmltemplate:
// escape_test.go, content_test.go, clone_test.go, multi_test.go,
// exec_test.go, template_test.go) as htmlexec scripts. The tables are in
// htmlexec_tables.go (extracted verbatim by extract_tables.py; rerun it
// after a fork sync); each test's procedure is
// rewritten as a script, with the Go test's checks as expectations.
//
// Not ported: ParseFiles/ParseGlob/ParseFS (file APIs the Rust engine does
// not have; TestParseFiles etc. are replaced by parsing the same texts),
// TestTemplateLookUp and TestIssue31810 (skipped in the fork),
// TestExecuteOnNewTemplate (nothing to check), TestJSEscaping
// (text/template's JSEscapeString), TestGoodFuncNames/TestBadFuncNames
// (Funcs panics; the Rust FuncMap is not name-checked), benchmarks and
// examples.

import (
	"fmt"
	"math/bits"
	"reflect"
	"sort"
	"strconv"
	"strings"
)

type execTest struct {
	name   string
	input  string
	output string
	data   any
	ok     bool
}

type cmpTest struct {
	expr  string
	truth string
	ok    bool
}

var iVal I = tVal

// bigInt and bigUint are hex string representing numbers either side
// of the max int boundary.
var (
	bigInt  = fmt.Sprintf("0x%x", int(1<<uint(bits.UintSize-1)-1))
	bigUint = fmt.Sprintf("0x%x", uint(1<<uint(bits.UintSize-1)))
)

func goTestScripts() []*script {
	var ss []*script
	add := func(s ...*script) { ss = append(ss, s...) }
	add(escapeTestScripts()...)
	add(escapeMiscScripts()...)
	add(contentTestScripts()...)
	add(cloneTestScripts()...)
	add(multiTestScripts()...)
	add(execTestScripts()...)
	add(templateTestScripts()...)
	return ss
}

// ---------------------------------------------------------------------------
// escape_test.go

// escapeData is TestEscape's data (field order and types as in Go).
func escapeData(ptr bool) string {
	return sStruct(ptr,
		sField("F", "bool:0"),
		sField("T", "bool:1"),
		sField("C", sStr("<Cincinnati>")),
		sField("G", sStr("<Goodbye>")),
		sField("H", sStr("<Hello>")),
		sField("I", sStr("${ asd `` }")),
		sField("A", sList("[]string", sStr("<a>"), sStr("<b>"))),
		sField("E", sList("[]string")),
		sField("B", "badm"),
		sField("M", "goodm"),
		sField("N", sInt("int", 42)),
		sFieldAny("U", "nil"),
		sField("Z", sTnil("*int")),
		sField("W", sSafe("html", `&iexcl;<b class="foo">Hello</b>, <textarea>O'World</textarea>!`)),
	)
}

func escapeTestScripts() []*script {
	var ss []*script
	data, pdata := escapeData(false), escapeData(true)
	for _, test := range escapeTestsTable {
		out := test.output
		// The Go test's marshaler types are template.badMarshaler etc.;
		// here they are main.* types.
		out = strings.ReplaceAll(out, "*template.", "*main.")
		s := newScript("TestEscape/"+test.name).
			op("new", "t", test.name).
			op("parse", "t", test.input).want(wantOK()).
			// Check for bug 6459: Tree field was not set in Parse.
			op("treesync", "t").want(wantRes("true")).
			op("exec", "t", data).want(wantOut(out)).
			op("exec", "t", pdata).want(wantOut(out)).
			op("treesync", "t").want(wantRes("true"))
		ss = append(ss, s)
	}
	return ss
}

func escapeMiscScripts() []*script {
	var ss []*script
	add := func(s ...*script) { ss = append(ss, s...) }

	// TestEscapeMap
	mapData := sMap("map[string]string",
		"html", sStr(`<h1>Hi!</h1>`),
		"urlquery", sStr(`http://www.foo.com/index.html?title=main`))
	for _, test := range []struct{ desc, input, output string }{
		{"field with predefined escaper name 1", `{{.html | print}}`, `&lt;h1&gt;Hi!&lt;/h1&gt;`},
		{"field with predefined escaper name 2", `{{.urlquery | print}}`, `http://www.foo.com/index.html?title=main`},
	} {
		add(newScript("TestEscapeMap/"+test.desc).
			op("new", "t", "").
			op("parse", "t", test.input).want(wantOK()).
			op("exec", "t", mapData).want(wantOut(test.output)))
	}

	// TestEscapeSet
	setData := "dataitemv:(" + strings.Join([]string{
		"dataitem:" + hexs("foo"),
		"dataitem:" + hexs("<bar>"),
		"dataitem:(dataitem:" + hexs("baz") + ")",
	}, ",") + ")"
	for i, test := range []struct {
		inputs map[string]string
		want   string
	}{
		// The trivial set.
		{map[string]string{"main": ``}, ``},
		// A template called in the start context.
		{map[string]string{
			"main": `Hello, {{template "helper"}}!`,
			// Not a valid top level HTML template.
			// "<b" is not a full tag.
			"helper": `{{"<World>"}}`,
		}, `Hello, &lt;World&gt;!`},
		// A template called in a context other than the start.
		{map[string]string{
			"main": `<a onclick='a = {{template "helper"}};'>`,
			// Not a valid top level HTML template.
			// "<b" is not a full tag.
			"helper": `{{"<a>"}}<b`,
		}, `<a onclick='a = &#34;\u003ca\u003e&#34;<b;'>`},
		// A recursive template that ends in its start context.
		{map[string]string{
			"main": `{{range .Children}}{{template "main" .}}{{else}}{{.X}} {{end}}`,
		}, `foo &lt;bar&gt; baz `},
		// A recursive helper template that ends in its start context.
		{map[string]string{
			"main":   `{{template "helper" .}}`,
			"helper": `{{if .Children}}<ul>{{range .Children}}<li>{{template "main" .}}</li>{{end}}</ul>{{else}}{{.X}}{{end}}`,
		}, `<ul><li>foo</li><li>&lt;bar&gt;</li><li><ul><li>baz</li></ul></li></ul>`},
		// Co-recursive templates that end in its start context.
		{map[string]string{
			"main":   `<blockquote>{{range .Children}}{{template "helper" .}}{{end}}</blockquote>`,
			"helper": `{{if .Children}}{{template "main" .}}{{else}}{{.X}}<br>{{end}}`,
		}, `<blockquote>foo<br>&lt;bar&gt;<br><blockquote>baz<br></blockquote></blockquote>`},
		// A template that is called in two different contexts.
		{map[string]string{
			"main":   `<button onclick="title='{{template "helper"}}'; ...">{{template "helper"}}</button>`,
			"helper": `{{11}} of {{"<100>"}}`,
		}, `<button onclick="title='11 of \u003c100\u003e'; ...">11 of &lt;100&gt;</button>`},
		// A non-recursive template that ends in a different context.
		// helper starts in jsCtxRegexp and ends in jsCtxDivOp.
		{map[string]string{
			"main":   `<script>var x={{template "helper"}}/{{"42"}};</script>`,
			"helper": "{{126}}",
		}, `<script>var x= 126 /"42";</script>`},
		// A recursive template that ends in a similar context.
		{map[string]string{
			"main":      `<script>var x=[{{template "countdown" 4}}];</script>`,
			"countdown": `{{.}}{{if .}},{{template "countdown" . | pred}}{{end}}`,
		}, `<script>var x=[ 4 , 3 , 2 , 1 , 0 ];</script>`},
		// A recursive template that ends in a different context (disabled in Go).
		{map[string]string{
			"main":   `<a href="/foo{{template "helper" .}}">`,
			"helper": `{{if .Children}}{{range .Children}}{{template "helper" .}}{{end}}{{else}}?x={{.X}}{{end}}`,
		}, ""},
	} {
		// Go builds the source in map order; sorted here.
		var names []string
		for name := range test.inputs {
			names = append(names, name)
		}
		sort.Strings(names)
		source := ""
		for _, name := range names {
			source += fmt.Sprintf("{{define %q}}%s{{end}} ", name, test.inputs[name])
		}
		s := newScript("TestEscapeSet/"+strconv.Itoa(i)).
			op("new", "t", "root").
			op("funcs", "t", "pred").
			op("parse", "t", source).want(wantOK())
		if test.want != "" || i == 0 {
			s.op("exectmpl", "t", "main", setData).want(wantOut(test.want))
		} else {
			s.op("exectmpl", "t", "main", setData)
		}
		add(s)
	}

	// TestErrors
	for i, test := range errorsTestsTable {
		s := newScript("TestErrors/"+strconv.Itoa(i)).
			op("new", "t", "z").
			op("parse", "t", test.input).want(wantOK())
		if test.err == "" {
			s.op("exec", "t", "nil").want(wantExecOK())
		} else {
			// Check that we get the same error if we call Execute again.
			s.op("exec", "t", "nil").want(wantExecErr(test.err)).
				op("exec", "t", "nil").want(wantExecErr(test.err))
		}
		add(s)
	}

	// TestEscapeMalformedPipelines
	for i, test := range []string{
		"{{ 0 | $ }}",
		"{{ 0 | $ | urlquery }}",
		"{{ 0 | (nil) }}",
		"{{ 0 | (nil) | html }}",
	} {
		add(newScript("TestEscapeMalformedPipelines/"+strconv.Itoa(i)).
			op("new", "t", "test").
			op("parse", "t", test).want(wantOK()).
			op("exec", "t", "nil").want(wantExecErr("")))
	}

	// TestEscapeErrorsNotIgnorable
	add(newScript("TestEscapeErrorsNotIgnorable").
		op("new", "t", "dangerous").
		op("parse", "t", "<a").
		op("exec", "t", "nil").want(func(res []string) error {
		if res[1] == "" || res[0] != "" {
			return fmt.Errorf("want an error and no output, got %q", res)
		}
		return nil
	}))

	// TestEscapeSetErrorsNotIgnorable
	add(newScript("TestEscapeSetErrorsNotIgnorable").
		op("new", "t", "root").
		op("parse", "t", `{{define "t"}}<a{{end}}`).want(wantOK()).
		op("exectmpl", "t", "t", "nil").want(func(res []string) error {
		if res[1] == "" || res[0] != "" {
			return fmt.Errorf("want an error and no output, got %q", res)
		}
		return nil
	}))

	// TestIndirectPrint (pointers to basic values: the value model has
	// none; the pointees are used, which prints the same).
	add(newScript("TestIndirectPrint").
		op("new", "t", "t").
		op("parse", "t", `{{.}}`).
		op("exec", "t", sInt("int", 3)).want(wantOut("3")).
		op("exec", "t", sStr("hello")).want(wantOut("hello")))

	// TestEmptyTemplateHTML: ParseFiles(os.DevNull) defines an empty
	// template "null" associated with "page", which stays undefined.
	add(newScript("TestEmptyTemplateHTML").
		op("new", "page", "page").
		op("newassoc", "n", "page", "null").
		op("parse", "n", "").want(wantOK()).
		op("exectmpl", "page", "page", sStr("nothing")).want(wantExecErr("")))

	// TestPipeToMethodIsEscaped
	s := newScript("TestPipeToMethodIsEscaped").
		op("new", "t", "x").
		op("parse", "t", "<html>{{0 | .SomeMethod}}</html>\n")
	for i := 0; i < 3; i++ {
		s.op("exec", "t", "issue7379:0").want(wantOut("<html>&lt;0&gt;</html>\n"))
	}
	add(s)

	// TestErrorOnUndefined
	add(newScript("TestErrorOnUndefined").
		op("new", "t", "undefined").
		op("exec", "t", "nil").want(wantExecErr("incomplete")))

	// TestIdempotentExecute
	add(newScript("TestIdempotentExecute").
		op("new", "t", "").
		op("parse", "t", `{{define "main"}}<body>{{template "hello"}}</body>{{end}}`).want(wantOK()).
		op("parse", "t", `{{define "hello"}}Hello, {{"Ladies & Gentlemen!"}}{{end}}`).want(wantOK()).
		op("exectmpl", "t", "hello", "nil").want(wantOut("Hello, Ladies &amp; Gentlemen!")).
		op("exectmpl", "t", "hello", "nil").want(wantOut("Hello, Ladies &amp; Gentlemen!")).
		op("exectmpl", "t", "main", "nil").want(wantOut("<body>Hello, Ladies &amp; Gentlemen!</body>")))

	// TestOrphanedTemplate
	add(newScript("TestOrphanedTemplate").
		op("new", "t1", "foo").
		op("parse", "t1", `<a href="{{.}}">link1</a>`).want(wantOK()).
		op("newassoc", "t2", "t1", "foo").
		op("parse", "t2", `bar`).want(wantOK()).
		op("exec", "t1", sStr("javascript:alert(1)")).
		want(wantRes("", `template: "foo" is an incomplete or empty template`)).
		op("exec", "t2", "nil").want(wantOut("bar")))

	// TestAliasedParseTreeDoesNotOverescape
	add(newScript("TestAliasedParseTreeDoesNotOverescape").
		op("new", "tpl", "foo").
		op("parse", "tpl", `{{.}}`).want(wantOK()).
		op("addtree", "bar", "tpl", "bar", "of:tpl").want(wantOK()).
		op("exectmpl", "tpl", "foo", sStr(`<baz>`)).want(wantOut(`&lt;baz&gt;`)).
		op("exectmpl", "tpl", "bar", sStr(`<baz>`)).want(wantOut(`&lt;baz&gt;`)))

	return ss
}

func escapeTextTests() []struct{ input, want string } {
	var ts []struct{ input, want string }
	for _, in := range escapeTextInputs {
		ts = append(ts, struct{ input, want string }{in, ""})
	}
	return ts
}

func ensurePipelineTests() []struct {
	input, output string
	ids           []string
} {
	return ensurePipelineTable
}

// ---------------------------------------------------------------------------
// content_test.go

func contentTestScripts() []*script {
	var ss []*script
	var data []string
	for _, x := range typedContentData {
		data = append(data, specOfContent(x))
	}
	for i, test := range typedContentTable {
		pre := strings.Index(test.input, "{{.}}")
		post := len(test.input) - (pre + 5)
		s := newScript("TestTypedContent/"+strconv.Itoa(i)).
			op("new", "t", "x").
			op("parse", "t", test.input).want(wantOK())
		for j, x := range data {
			want := test.want[j]
			s.op("exec", "t", x).want(func(res []string) error {
				if res[1] != "" || len(res[0]) < pre+post || res[0][pre:len(res[0])-post] != want {
					return fmt.Errorf("want %q inside, got %q", want, res)
				}
				return nil
			})
		}
		ss = append(ss, s)
	}

	// TestStringer
	ss = append(ss, newScript("TestStringer").
		op("new", "t", "x").
		op("parse", "t", "{{.}}").
		op("exec", "t", "mystringer:3").want(wantOut("string=3")).
		op("exec", "t", "errorer:7").want(wantOut("error=7")))

	// TestEscapingNilNonemptyInterfaces
	ss = append(ss, newScript("TestEscapingNilNonemptyInterfaces").
		op("new", "t", "x").
		op("parse", "t", "{{.E}}").
		op("exec", "t", sStruct(false, sField("E", sTnil("error")))).want(wantOut("")).
		op("exec", "t", sStruct(false, sFieldAny("E", "nil"))).want(wantOut("")))
	return ss
}

func specOfContent(x any) string {
	switch x := x.(type) {
	case string:
		return sStr(x)
	}
	v := reflect.ValueOf(x)
	kind := map[string]string{
		"template.CSS": "css", "template.HTML": "html", "template.HTMLAttr": "htmlattr",
		"template.JS": "js", "template.JSStr": "jsstr", "template.URL": "url", "template.Srcset": "srcset",
	}[v.Type().String()]
	if kind == "" {
		panic("specOfContent: " + v.Type().String())
	}
	return sSafe(kind, v.String())
}

// ---------------------------------------------------------------------------
// clone_test.go

func cloneTestScripts() []*script {
	var ss []*script
	add := func(s ...*script) { ss = append(ss, s...) }

	// TestAddParseTreeHTML
	add(newScript("TestAddParseTreeHTML").
		op("new", "root", "root").
		op("parse", "root", `{{define "a"}} {{.}} {{template "b"}} {{.}} "></a>{{end}}`).want(wantOK()).
		op("parsetree", "tr", "t", `{{define "b"}}<a href="{{end}}`, "b").want(wantOK()).
		op("addtree", "added", "root", "b", "tree:tr").want(wantOK()).
		op("exectmpl", "added", "a", sStr("1>0")).want(wantOut(` 1&gt;0 <a href=" 1%3e0 "></a>`)))

	// TestClone
	const tmpl = `{{define "a"}}{{template "lhs"}}{{.}}{{template "rhs"}}{{end}}`
	d := sStr("<i>*/")
	add(newScript("TestClone").
		// Create an incomplete template t0.
		op("new", "t0", "t0").op("parse", "t0", tmpl).want(wantOK()).
		// Clone t0 as t1.
		op("clone", "t1", "t0").want(wantOK()).
		op("parse", "t1", `{{define "lhs"}} <a href=" {{end}}`).want(wantOK()).
		op("parse", "t1", `{{define "rhs"}} "></a> {{end}}`).want(wantOK()).
		op("exectmpl", "t1", "a", d).want(wantOut(` <a href=" %3ci%3e*/ "></a> `)).
		// Clone t0 as t2.
		op("clone", "t2", "t0").want(wantOK()).
		op("parse", "t2", `{{define "lhs"}} <p onclick="javascript: {{end}}`).want(wantOK()).
		op("parse", "t2", `{{define "rhs"}} "></p> {{end}}`).want(wantOK()).
		op("exectmpl", "t2", "a", d).want(wantOut(` <p onclick="javascript: &#34;\u003ci\u003e*/&#34; "></p> `)).
		// Clone t0 as t3, but do not execute t3 yet.
		op("clone", "t3", "t0").want(wantOK()).
		op("parse", "t3", `{{define "lhs"}} <style> {{end}}`).want(wantOK()).
		op("parse", "t3", `{{define "rhs"}} </style> {{end}}`).want(wantOK()).
		// Complete t0.
		op("parse", "t0", `{{define "lhs"}} ( {{end}}`).want(wantOK()).
		op("parse", "t0", `{{define "rhs"}} ) {{end}}`).want(wantOK()).
		// Clone t0 as t4. Redefining the "lhs" template should not fail.
		op("clone", "t4", "t0").want(wantOK()).
		op("parse", "t4", `{{define "lhs"}} OK {{end}}`).want(wantOK()).
		// Cloning t1 should fail as it has been executed.
		op("clone", "x", "t1").want(wantNotOK()).
		// Redefining the "lhs" template in t1 should fail as it has been executed.
		op("parse", "t1", `{{define "lhs"}} OK {{end}}`).want(wantNotOK()).
		// Execute t0.
		op("exectmpl", "t0", "a", d).want(wantOut(` ( &lt;i&gt;*/ ) `)).
		// Clone t0. This should fail, as t0 has already executed.
		op("clone", "x", "t0").want(wantNotOK()).
		// Similarly, cloning sub-templates should fail.
		op("lookup", "a", "t0", "a").want(wantRes("ok")).
		op("clone", "x", "a").want(wantNotOK()).
		op("lookup", "lhs", "t0", "lhs").want(wantRes("ok")).
		op("clone", "x", "lhs").want(wantNotOK()).
		// Execute t3.
		op("exectmpl", "t3", "a", d).want(wantOut(` <style> ZgotmplZ </style> `)))

	// TestTemplates
	add(newScript("TestTemplates").
		op("new", "t0", "t0").
		op("parse", "t0", `
		{{define "a"}}{{template "lhs"}}{{.}}{{template "rhs"}}{{end}}
		{{define "lhs"}} <a href=" {{end}}
		{{define "rhs"}} "></a> {{end}}`).want(wantOK()).
		op("templates", "t0").want(wantRes("4", "a", "lhs", "rhs", "t0")))

	// TestCloneCrash
	add(newScript("TestCloneCrash").
		op("new", "t1", "all").
		op("newassoc", "x", "t1", "t1").
		op("parse", "x", `{{define "foo"}}foo{{end}}`).want(wantOK()).
		op("clone", "c", "t1"))

	// TestCloneThenParse
	add(newScript("TestCloneThenParse").
		op("new", "t0", "t0").
		op("parse", "t0", `{{define "a"}}{{template "embedded"}}{{end}}`).want(wantOK()).
		op("clone", "t1", "t0").want(wantOK()).
		op("parse", "t1", `{{define "embedded"}}t1{{end}}`).want(wantOK()).
		op("templates", "t0").
		op("templates", "t1").
		op("exectmpl", "t0", "a", "nil").want(wantExecErr("")))

	// TestFuncMapWorksAfterClone
	add(newScript("TestFuncMapWorksAfterClone").
		op("new", "u", "").op("funcs", "u", "issue5980").
		op("parse", "u", "{{customFunc}}").want(wantOK()).
		op("exec", "u", "nil").want(wantExecErr("issue5980")).
		op("new", "tc", "").op("funcs", "tc", "issue5980").
		op("parse", "tc", "{{customFunc}}").want(wantOK()).
		op("clone", "c", "tc").want(wantOK()).
		op("exec", "c", "nil").want(wantExecErr("issue5980")))

	// TestTemplateCloneExecuteRace
	add(newScript("TestTemplateCloneExecuteRace").
		op("new", "outer", "outer").
		op("parse", "outer", `<title>{{block "a" .}}a{{end}}</title><body>{{block "b" .}}b{{end}}<body>`).want(wantOK()).
		op("clone", "c", "outer").want(wantOK()).
		op("parse", "c", `{{define "b"}}A{{end}}`).want(wantOK()).
		op("execpar", "c", sStr("data"), "10").want(wantExecOK()))

	// TestTemplateCloneLookup
	add(newScript("TestTemplateCloneLookup").
		op("new", "t", "x").
		op("parse", "t", "a").want(wantOK()).
		op("clone", "c", "t").want(wantOK()).
		op("lookup", "l", "c", "x").want(wantRes("ok")).
		op("ptreq", "l", "c").want(wantRes("true")))

	// TestCloneGrowth
	s := newScript("TestCloneGrowth").
		op("new", "t", "root").
		op("parse", "t", `<title>{{block "B". }}Arg{{end}}</title>`).want(wantOK()).
		op("clone", "c", "t").want(wantOK()).
		op("parse", "c", `{{define "B"}}Text{{end}}`).want(wantOK())
	for i := 0; i < 10; i++ {
		s.op("exec", "c", "nil")
	}
	add(s.op("templates", "c"))

	// TestCloneRedefinedName
	s = newScript("TestCloneRedefinedName").
		op("new", "t1", "a").
		op("parse", "t1", `
{{ define "a" -}}<title>{{ template "b" . -}}</title>{{ end -}}
{{ define "b" }}{{ end -}}
`).want(wantOK())
	for i := 0; i < 2; i++ {
		s.op("clone", "t2", "t1").want(wantOK()).
			op("newassoc", "t3", "t2", strconv.Itoa(i)).
			op("parse", "t3", `{{ template "a" . }}`).want(wantOK()).
			op("exec", "t3", "nil").want(wantExecOK())
	}
	add(s)

	// TestClonePipe
	add(newScript("TestClonePipe").
		op("new", "a", "a").
		op("parse", "a", `{{define "a"}}{{range $v := .A}}{{$v}}{{end}}{{end}}`).want(wantOK()).
		op("clone", "b", "a").want(wantOK()).
		op("exec", "b", sStruct(true, sField("A", sList("[]string", sStr("hi"))))).want(wantOut("hi")))
	return ss
}

// ---------------------------------------------------------------------------
// multi_test.go

// multiText1/2 are also the contents of testdata/file1.tmpl, file2.tmpl.
const multiText1 = `
	{{define "x"}}TEXT{{end}}
	{{define "dotV"}}{{.V}}{{end}}
`

const multiText2 = `
	{{define "dot"}}{{.}}{{end}}
	{{define "nested"}}{{template "dot" .}}{{end}}
`

// testExecuteScript is testExecute: each test parses its input as a new
// template (in a clone of root when root != "").
func testExecuteScripts(prefix string, tests []execTest, setup func(s *script)) []*script {
	var ss []*script
	for _, test := range tests {
		s := newScript(prefix + "/" + test.name)
		if setup == nil {
			s.op("new", "t", test.name)
		} else {
			setup(s)
			s.op("clone", "c", "root").want(wantOK()).
				op("newassoc", "t", "c", test.name)
		}
		s.op("funcs", "t", "exec").
			op("parse", "t", test.input).want(wantOK())
		output, ok := test.output, test.ok
		s.op("exec", "t", specOf(test.data)).want(func(res []string) error {
			switch {
			case !ok && res[1] == "":
				return fmt.Errorf("expected error; got none")
			case ok && res[1] != "":
				return fmt.Errorf("unexpected execute error: %s", res[1])
			}
			if res[0] != output {
				return fmt.Errorf("expected %q got %q", output, res[0])
			}
			return nil
		})
		ss = append(ss, s)
	}
	return ss
}

func multiTestScripts() []*script {
	var ss []*script
	add := func(s ...*script) { ss = append(ss, s...) }

	// TestMultiExecute (and TestParseFiles/ParseGlob/ParseFS with the same
	// texts).
	add(testExecuteScripts("TestMultiExecute", multiExecTestsTable, func(s *script) {
		s.op("new", "root", "root").
			op("parse", "root", multiText1).want(wantOK()).
			op("parse", "root", multiText2).want(wantOK())
	})...)
	add(testExecuteScripts("TestParseFiles", multiExecTestsTable, func(s *script) {
		s.op("new", "root", "root").
			op("newassoc", "f1", "root", "file1.tmpl").
			op("parse", "f1", multiText1).want(wantOK()).
			op("newassoc", "f2", "root", "file2.tmpl").
			op("parse", "f2", multiText2).want(wantOK())
	})...)

	// TestParseFilesWithData (testdata/tmpl1.tmpl, tmpl2.tmpl)
	add(testExecuteScripts("TestParseFilesWithData", []execTest{
		{"test", `{{template "tmpl1.tmpl"}}{{template "tmpl2.tmpl"}}`, "template1\n\ny\ntemplate2\n\nx\n", 0, true},
	}, func(s *script) {
		s.op("new", "root", "root").
			op("newassoc", "f1", "root", "tmpl1.tmpl").
			op("parse", "f1", "template1\n{{define \"x\"}}x{{end}}\n{{template \"y\"}}\n").want(wantOK()).
			op("newassoc", "f2", "root", "tmpl2.tmpl").
			op("parse", "f2", "template2\n{{define \"y\"}}y{{end}}\n{{template \"x\"}}\n").want(wantOK())
	})...)

	// TestAddParseTreeToUnparsedTemplate
	add(newScript("TestAddParseTreeToUnparsedTemplate").
		op("new", "t", "master").
		op("parsetree", "tr", "master", "{{define \"master\"}}{{end}}", "master").want(wantOK()).
		op("addtree", "m", "t", "master", "tree:tr").want(wantOK()))

	// TestRedefinition
	add(newScript("TestRedefinition").
		op("new", "t", "tmpl1").
		op("parse", "t", `{{define "test"}}foo{{end}}`).want(wantOK()).
		op("parse", "t", `{{define "test"}}bar{{end}}`).want(wantOK()).
		op("newassoc", "t2", "t", "tmpl2").
		op("parse", "t2", `{{define "test"}}bar{{end}}`).want(wantOK()))

	// TestEmptyTemplateCloneCrash
	add(newScript("TestEmptyTemplateCloneCrash").
		op("new", "t", "base").
		op("clone", "c", "t"))

	// TestParse
	add(newScript("TestParse").
		op("new", "t", "test").
		op("parse", "t", `{{define "test"}}{{end}}`).want(wantOK()).
		op("parse", "t", `{{define "test"}}{{/* this is a comment */}}{{end}}`).want(wantOK()).
		op("parse", "t", `{{define "test"}}foo{{end}}`).want(wantOK()))

	// TestEmptyTemplate
	for i, c := range []struct {
		defn []string
		in   string
		want string
	}{
		{[]string{"x", "y"}, "", "y"},
		{[]string{""}, "once", ""},
		{[]string{"", ""}, "twice", ""},
		{[]string{"{{.}}", "{{.}}"}, "twice", "twice"},
		{[]string{"{{/* a comment */}}", "{{/* a comment */}}"}, "comment", ""},
		{[]string{"{{.}}", ""}, "twice", "twice"},
	} {
		s := newScript("TestEmptyTemplate/"+strconv.Itoa(i)).op("new", "root", "root")
		for _, d := range c.defn {
			s.op("newassoc", "m", "root", c.in).op("parse", "m", d).want(wantOK())
		}
		add(s.op("exec", "m", sStr(c.in)).want(wantOut(c.want)))
	}

	// TestIssue19294
	for i := 0; i < 10; i++ {
		add(newScript("TestIssue19294/"+strconv.Itoa(i)).
			op("new", "res", "title.xhtml").
			op("parse", "res", `{{template "xhtml" .}}`).want(wantOK()).
			op("newassoc", "a", "res", "stylesheet").
			op("parse", "a", `{{define "stylesheet"}}stylesheet{{end}}`).want(wantOK()).
			op("newassoc", "b", "res", "xhtml").
			op("parse", "b", `{{block "stylesheet" .}}{{end}}`).want(wantOK()).
			op("exec", "res", sInt("int", 0)).want(wantOut("stylesheet")))
	}
	return ss
}

// ---------------------------------------------------------------------------
// exec_test.go

func execTestScripts() []*script {
	var ss []*script
	add := func(s ...*script) { ss = append(ss, s...) }

	// TestExecute
	add(testExecuteScripts("TestExecute", execTestsTable, nil)...)

	// TestDelims
	delimPairs := []string{
		"", "", // default
		"{{", "}}", // same as default
		"|", "|", // same
		"(日)", "(本)", // peculiar
	}
	const hello = "Hello, world"
	value := sStruct(false, sField("Str", sStr(hello)))
	for i := 0; i < len(delimPairs); i += 2 {
		text := ".Str"
		left := delimPairs[i+0]
		trueLeft := left
		right := delimPairs[i+1]
		trueRight := right
		if left == "" { // default case
			trueLeft = "{{"
		}
		if right == "" { // default case
			trueRight = "}}"
		}
		text = trueLeft + text + trueRight
		// Now add a comment
		text += trueLeft + "/*comment*/" + trueRight
		// Now add  an action containing a string.
		text += trueLeft + `"` + trueLeft + `"` + trueRight
		add(newScript("TestDelims/"+strconv.Itoa(i/2)).
			op("new", "t", "delims").
			op("delims", "t", left, right).
			op("parse", "t", text).want(wantOK()).
			op("exec", "t", value).want(wantOut(hello + trueLeft)))
	}

	// TestExecuteError
	add(newScript("TestExecuteError").
		op("new", "t", "error").
		op("parse", "t", "{{.MyError true}}").want(wantOK()).
		op("exec", "t", "tval").want(wantExecErr("my error")))

	// TestExecError
	add(newScript("TestExecError").
		op("new", "t", "top").
		op("parse", "t", `line 1
line 2
line 3
{{template "one" .}}
{{define "one"}}{{template "two" .}}{{end}}
{{define "two"}}{{template "three" .}}{{end}}
{{define "three"}}{{index "hi" $}}{{end}}`).want(wantOK()).
		op("exec", "t", sInt("int", 5)).
		want(wantExecErr(`template: top:7:20: executing "three" at <index "hi" $>: error calling index: index out of range: 5`)))

	// TestTree
	var tree func(v int, l, r string) string
	tree = func(v int, l, r string) string { return "tree:" + strconv.Itoa(v) + "(" + l + "," + r + ")" }
	treeVal := tree(1,
		tree(2, tree(3, tree(4, "nil", "nil"), "nil"), tree(5, tree(6, "nil", "nil"), "nil")),
		tree(7, tree(8, tree(9, "nil", "nil"), "nil"), tree(10, tree(11, "nil", "nil"), "nil")))
	add(newScript("TestTree").
		op("new", "t", "root").
		op("delims", "t", "(", ")").
		op("parse", "t", `
	(- define "tree" -)
	[
		(- .Val -)
		(- with .Left -)
			(template "tree" . -)
		(- end -)
		(- with .Right -)
			(- template "tree" . -)
		(- end -)
	]
	(- end -)
`).want(wantOK()).
		op("lookup", "tr", "t", "tree").want(wantRes("ok")).
		op("exec", "tr", treeVal).want(wantOut("[1[2[3[4]][5[6]]][7[8[9]][10[11]]]]")).
		op("exectmpl", "t", "tree", treeVal).want(wantOut("[1[2[3[4]][5[6]]][7[8[9]][10[11]]]]")))

	// TestMessageForExecuteEmpty
	add(newScript("TestMessageForExecuteEmpty").
		op("new", "t", "empty").
		op("exec", "t", sInt("int", 0)).want(wantExecErr(`template: "empty" is an incomplete or empty template`)).
		op("new", "t", "empty").
		op("new", "tests", "").
		op("parse", "tests", `{{define "one"}}one{{end}}{{define "two"}}two{{end}}`).want(wantOK()).
		op("addtree", "sec", "t", "secondary", "of:tests").
		op("exec", "t", sInt("int", 0)).want(wantExecErr(`template: "empty" is an incomplete or empty template`)).
		op("exectmpl", "t", "secondary", sInt("int", 0)).want(wantExecOK()))

	// TestFinalForPrintf
	add(newScript("TestFinalForPrintf").
		op("new", "t", "").
		op("parse", "t", `{{"x" | printf}}`).want(wantOK()).
		op("exec", "t", sInt("int", 0)).want(wantExecOK()))

	// TestComparison
	cmpStruct := sStruct(true,
		sField("Uthree", sUint("uint", 3)), sField("Ufour", sUint("uint", 4)),
		sField("NegOne", sInt("int", -1)), sField("Three", sInt("int", 3)),
		sField("Ptr", "newint"), sField("NilPtr", sTnil("*int")),
		sField("NonNilMap", sMap("map[int]int")), sField("Map", sTnil("map[int]int")),
		sField("V1", "vval:0"), sField("V2", "vval:0"),
		sField("Iface1", "bufstr:"), sField("Iface2", sTnil("fmt.Stringer")),
	)
	for _, test := range cmpTestsTable {
		text := fmt.Sprintf("{{if %s}}true{{else}}false{{end}}", test.expr)
		truth, ok := test.truth, test.ok
		add(newScript("TestComparison/"+test.expr).
			op("new", "t", "empty").
			op("parse", "t", text).want(wantOK()).
			op("exec", "t", cmpStruct).want(func(res []string) error {
			if ok && res[1] != "" {
				return fmt.Errorf("errored incorrectly: %s", res[1])
			}
			if !ok && res[1] == "" {
				return fmt.Errorf("did not error")
			}
			if res[0] != truth {
				return fmt.Errorf("want %s; got %s", truth, res[0])
			}
			return nil
		}))
	}

	// TestMissingMapKey
	mm := sMap("map[string]int", "x", sInt("int", 99))
	add(newScript("TestMissingMapKey").
		op("new", "t", "t1").
		op("parse", "t", "{{.x}} {{.y}}").want(wantOK()).
		op("exec", "t", mm).want(wantOut("99 ")).
		op("option", "t", "missingkey=default").
		op("exec", "t", mm).want(wantOut("99 ")).
		op("option", "t", "missingkey=zero").
		op("exec", "t", mm).want(wantOut("99 0")).
		op("option", "t", "missingkey=error").
		op("exec", "t", mm).want(wantExecErr("")).
		op("exec", "t", "nil").want(wantExecErr("")))

	// TestUnterminatedStringError
	add(newScript("TestUnterminatedStringError").
		op("new", "t", "X").
		op("parse", "t", "hello\n\n{{`unterminated\n\n\n\n}}\n some more\n\n").want(func(res []string) error {
		if len(res) != 2 || !strings.Contains(res[1], "X:3: unterminated raw quoted string") {
			return fmt.Errorf("unexpected: %q", res)
		}
		return nil
	}))

	// TestExecuteGivesExecError
	add(newScript("TestExecuteGivesExecError").
		op("new", "t", "X").
		op("parse", "t", "hello").want(wantOK()).
		op("execerrw", "t", sInt("int", 0)).want(wantRes("", "always be failing")).
		op("new", "t2", "X").
		op("parse", "t2", "hello, {{.X.Y}}").want(wantOK()).
		op("exec", "t2", sInt("int", 0)).want(wantExecErr("field X in type int")))

	// TestBlock
	add(newScript("TestBlock").
		op("new", "t", "outer").
		op("parse", "t", `a({{block "inner" .}}bar({{.}})baz{{end}})b`).want(wantOK()).
		op("clone", "t2", "t").want(wantOK()).
		op("parse", "t2", `{{define "inner"}}foo({{.}})bar{{end}}`).want(wantOK()).
		op("exec", "t", sStr("hello")).want(wantOut(`a(bar(hello)baz)b`)).
		op("exec", "t2", sStr("goodbye")).want(wantOut(`a(foo(goodbye)bar)b`)))

	// TestEvalFieldErrors
	for _, tc := range []struct{ name, src, value, want string }{
		{"MissingFieldOnNil", "{{.MissingField}}", sTnil("*main.T"), "can't evaluate field MissingField in type *main.T"},
		{"MissingFieldOnNonNil", "{{.MissingField}}", "tzeroptr", "can't evaluate field MissingField in type *main.T"},
		{"ExistingFieldOnNil", "{{.X}}", sTnil("*main.T"), "nil pointer evaluating *main.T.X"},
		{"MissingKeyOnNilMap", "{{.MissingKey}}", sTnil("*map[string]string"), "nil pointer evaluating *map[string]string.MissingKey"},
	} {
		want := tc.want
		add(newScript("TestEvalFieldErrors/"+tc.name).
			op("new", "t", "tmpl").
			op("parse", "t", tc.src).want(wantOK()).
			op("exec", "t", tc.value).want(func(res []string) error {
			if !strings.HasSuffix(res[1], want) {
				return fmt.Errorf("got error %q, want %q", res[1], want)
			}
			return nil
		}))
	}

	// TestAddrOfIndex
	for i, text := range []string{
		`{{range .}}{{.String}}{{end}}`,
		`{{with index . 0}}{{.String}}{{end}}`,
	} {
		add(newScript("TestAddrOfIndex/"+strconv.Itoa(i)).
			op("new", "t", "tmpl").
			op("parse", "t", text).want(wantOK()).
			op("exec", "t", "lvval(vval:1)").want(wantOut("&lt;1&gt;")))
	}

	// TestInterfaceValues
	ivData := sMap("map[string]interface {}",
		"PlusOne", "fnplusone",
		"Slice", sList("[]int", sInt("int", 0), sInt("int", 1), sInt("int", 2), sInt("int", 3)),
		"One", sInt("int", 1),
		"Two", sInt("int", 2),
		"Nil", "nil",
		"Zero", sInt("int", 0))
	for _, tt := range []struct{ text, out string }{
		{`{{index .Nil 1}}`, "ERROR: index of untyped nil"},
		{`{{index .Slice 2}}`, "2"},
		{`{{index .Slice .Two}}`, "2"},
		{`{{call .Nil 1}}`, "ERROR: call of nil"},
		{`{{call .PlusOne 1}}`, "2"},
		{`{{call .PlusOne .One}}`, "2"},
		{`{{and (index .Slice 0) true}}`, "0"},
		{`{{and .Zero true}}`, "0"},
		{`{{and (index .Slice 1) false}}`, "false"},
		{`{{and .One false}}`, "false"},
		{`{{or (index .Slice 0) false}}`, "false"},
		{`{{or .Zero false}}`, "false"},
		{`{{or (index .Slice 1) true}}`, "1"},
		{`{{or .One true}}`, "1"},
		{`{{not (index .Slice 0)}}`, "true"},
		{`{{not .Zero}}`, "true"},
		{`{{not (index .Slice 1)}}`, "false"},
		{`{{not .One}}`, "false"},
		{`{{eq (index .Slice 0) .Zero}}`, "true"},
		{`{{eq (index .Slice 1) .One}}`, "true"},
		{`{{ne (index .Slice 0) .Zero}}`, "false"},
		{`{{ne (index .Slice 1) .One}}`, "false"},
		{`{{ge (index .Slice 0) .One}}`, "false"},
		{`{{ge (index .Slice 1) .Zero}}`, "true"},
		{`{{gt (index .Slice 0) .One}}`, "false"},
		{`{{gt (index .Slice 1) .Zero}}`, "true"},
		{`{{le (index .Slice 0) .One}}`, "true"},
		{`{{le (index .Slice 1) .Zero}}`, "false"},
		{`{{lt (index .Slice 0) .One}}`, "true"},
		{`{{lt (index .Slice 1) .Zero}}`, "false"},
	} {
		s := newScript("TestInterfaceValues/"+tt.text).
			op("new", "t", "tmpl").
			op("parse", "t", tt.text).want(wantOK())
		if strings.HasPrefix(tt.out, "ERROR:") {
			s.op("exec", "t", ivData).want(wantExecErr(strings.TrimSpace(strings.TrimPrefix(tt.out, "ERROR:"))))
		} else {
			s.op("exec", "t", ivData).want(wantOut(tt.out))
		}
		add(s)
	}

	// TestExecutePanicDuringCall
	for _, tc := range []struct{ name, input, data, wantErr string }{
		{"direct func call panics", "{{doPanic}}", sTnil("*main.T"),
			`template: t:1:2: executing "t" at <doPanic>: error calling doPanic: custom panic string`},
		{"indirect func call panics", "{{call doPanic}}", sTnil("*main.T"),
			`template: t:1:7: executing "t" at <doPanic>: error calling doPanic: custom panic string`},
		{"direct method call panics", "{{.GetU}}", sTnil("*main.T"),
			`template: t:1:2: executing "t" at <.GetU>: error calling GetU: runtime error: invalid memory address or nil pointer dereference`},
		{"indirect method call panics", "{{call .GetU}}", sTnil("*main.T"),
			`template: t:1:7: executing "t" at <.GetU>: error calling GetU: runtime error: invalid memory address or nil pointer dereference`},
		{"func field call panics", "{{call .PanicFunc}}", "tval",
			`template: t:1:2: executing "t" at <call .PanicFunc>: error calling call: test panic`},
		{"method call on nil interface", "{{.NonEmptyInterfaceNil.Method0}}", "tval",
			`template: t:1:23: executing "t" at <.NonEmptyInterfaceNil.Method0>: nil pointer evaluating main.I.Method0`},
	} {
		add(newScript("TestExecutePanicDuringCall/"+tc.name).
			op("new", "t", "t").
			op("funcs", "t", "panic").
			op("parse", "t", tc.input).want(wantOK()).
			op("exec", "t", tc.data).want(wantExecErr(tc.wantErr)))
	}

	// TestEscapeRace (sequential)
	s := newScript("TestEscapeRace").
		op("new", "t", "").
		op("newassoc", "x", "t", "templ.html").
		op("parse", "x", `
{{- define "jstempl" -}}
var v = "v";
{{- end -}}
<script type="application/javascript">
{{ template "jstempl" $ }}
</script>
`).want(wantOK())
	for i := 0; i < 5; i++ {
		s.op("newassoc", "x", "t", fmt.Sprintf("x%d.html", i)).
			op("parse", "x", `{{ template "templ.html" .}}`).want(wantOK())
	}
	for i := 0; i < 5; i++ {
		s.op("lookup", "sub", "t", fmt.Sprintf("x%d.html", i)).want(wantRes("ok")).
			op("exec", "sub", "nil").want(wantExecOK())
	}
	add(s)

	// TestRecursiveExecute
	add(newScript("TestRecursiveExecute").
		op("new", "t", "").
		op("newassoc", "top", "t", "x.html").
		op("funcs", "top", "recur", "t").
		op("parse", "top", `{{recur}}`).want(wantOK()).
		op("newassoc", "sub", "t", "subroutine").
		op("parse", "sub", `<a href="/x?p={{"'a<b'"}}">`).want(wantOK()).
		op("exec", "top", "nil").want(wantExecOK()))

	// TestRecursiveExecuteViaMethod
	add(newScript("TestRecursiveExecuteViaMethod").
		op("new", "t", "").
		op("newassoc", "top", "t", "x.html").
		op("parse", "top", `{{.Recur}}`).want(wantOK()).
		op("newassoc", "sub", "t", "subroutine").
		op("parse", "sub", `<a href="/x?p={{"'a<b'"}}">`).want(wantOK()).
		op("execrecur", "top", "t").want(wantExecOK()))

	// TestTemplateFuncsAfterClone
	add(newScript("TestTemplateFuncsAfterClone").
		op("new", "o", "orig").
		op("funcs", "o", "identity").
		op("newassoc", "orig", "o", "child").
		op("clone", "c", "orig").want(wantOK()).
		op("parse", "c", `{{ f . }}`).want(wantOK()).
		op("exec", "c", sStr("test")).want(wantOut("test")))
	return ss
}

// ---------------------------------------------------------------------------
// template_test.go

func templateTestScripts() []*script {
	var ss []*script
	add := func(s ...*script) { ss = append(ss, s...) }

	// TestTemplateClone
	add(newScript("TestTemplateClone").
		op("new", "orig", "name").
		op("clone", "clone", "orig").want(wantOK()).
		op("templates", "orig").
		op("templates", "clone").
		op("parse", "clone", "stuff").want(wantOK()).
		op("exec", "clone", "nil").want(wantOut("stuff")))

	type step struct{ kind, tmpl, text, data, want string }
	cases := []struct {
		name  string
		steps []step
	}{
		{"TestRedefineNonEmptyAfterExecution", []step{
			{"parse", "root", `foo`, "", ""},
			{"exec", "root", "", "nil", "foo"},
			{"noparse", "root", `bar`, "", ""},
		}},
		{"TestRedefineEmptyAfterExecution", []step{
			{"parse", "root", ``, "", ""},
			{"exec", "root", "", "nil", ""},
			{"noparse", "root", `foo`, "", ""},
			{"exec", "root", "", "nil", ""},
		}},
		{"TestRedefineAfterNonExecution", []step{
			{"parse", "root", `{{if .}}<{{template "X"}}>{{end}}{{define "X"}}foo{{end}}`, "", ""},
			{"exec", "root", "", sInt("int", 0), ""},
			{"noparse", "root", `{{define "X"}}bar{{end}}`, "", ""},
			{"exec", "root", "", sInt("int", 1), "&lt;foo>"},
		}},
		{"TestRedefineAfterNamedExecution", []step{
			{"parse", "root", `<{{template "X" .}}>{{define "X"}}foo{{end}}`, "", ""},
			{"exec", "root", "", "nil", "&lt;foo>"},
			{"noparse", "root", `{{define "X"}}bar{{end}}`, "", ""},
			{"exec", "root", "", "nil", "&lt;foo>"},
		}},
		{"TestRedefineNestedByNameAfterExecution", []step{
			{"parse", "root", `{{define "X"}}foo{{end}}`, "", ""},
			{"exec", "X", "", "nil", "foo"},
			{"noparse", "root", `{{define "X"}}bar{{end}}`, "", ""},
			{"exec", "X", "", "nil", "foo"},
		}},
		{"TestRedefineNestedByTemplateAfterExecution", []step{
			{"parse", "root", `{{define "X"}}foo{{end}}`, "", ""},
			{"exec", "X", "", "nil", "foo"},
			{"noparse", "X", `bar`, "", ""},
			{"exec", "X", "", "nil", "foo"},
		}},
		{"TestRedefineSafety", []step{
			{"parse", "root", `<html><a href="{{template "X"}}">{{define "X"}}{{end}}`, "", ""},
			{"exec", "root", "", "nil", `<html><a href="">`},
			{"noparse", "root", `{{define "X"}}" bar="baz{{end}}`, "", ""},
			{"exec", "root", "", "nil", `<html><a href="">`},
		}},
		{"TestRedefineTopUse", []step{
			{"parse", "root", `{{template "X"}}{{.}}{{define "X"}}{{end}}`, "", ""},
			{"exec", "root", "", sInt("int", 42), `42`},
			{"noparse", "root", `{{define "X"}}<script>{{end}}`, "", ""},
			{"exec", "root", "", sInt("int", 42), `42`},
		}},
		{"TestNumbers", []step{
			{"parse", "root", `{{print 1_2.3_4}} {{print 0x0_1.e_0p+02}}`, "", ""},
			{"exec", "root", "", "nil", "12.34 7.5"},
		}},
	}
	for _, c := range cases {
		s := newScript(c.name).op("new", "root", "root")
		for _, st := range c.steps {
			v := st.tmpl
			if v != "root" {
				s.op("lookup", v, "root", v)
			}
			switch st.kind {
			case "parse":
				s.op("parse", v, st.text).want(wantOK())
			case "noparse":
				s.op("parse", v, st.text).want(wantNotOK())
			case "exec":
				s.op("exec", v, st.data).want(wantOut(st.want))
			}
		}
		add(s)
	}

	// TestRedefineOtherParsers (AddParseTree only)
	add(newScript("TestRedefineOtherParsers").
		op("new", "root", "root").
		op("parse", "root", ``).want(wantOK()).
		op("exec", "root", "nil").want(wantOut("")).
		op("addtree", "x", "root", "t1", "of:root").want(func(res []string) error {
		if len(res) != 2 || !strings.Contains(res[1], "Execute") {
			return fmt.Errorf("wanted error about already having Executed, got %q", res)
		}
		return nil
	}))

	// TestStringsInScriptsWithJsonContentTypeAreCorrectlyEscaped
	s := newScript("TestStringsInScriptsWithJsonContentTypeAreCorrectlyEscaped").
		op("new", "t", "JS string is JSON string").
		op("parse", "t", `<script type="application/ld+json">"{{.}}"</script>`).want(wantOK())
	for _, in := range []string{"", string(rune(-1)), "\u0000", "\u001F", "\t", "<>", `'"`, "ASCII letters", "ʕ⊙ϖ⊙ʔ", "🍕"} {
		s.op("exec", "t", sStr(in)).want(wantExecOK())
	}
	add(s)

	// TestSkipEscapeComments
	add(newScript("TestSkipEscapeComments").
		op("new", "root", "root").
		op("parsetree", "tr", "root", "{{/* A comment */}}{{ 1 }}{{/* Another comment */}}", "root", "comments").want(wantOK()).
		op("addtree", "root", "root", "root", "tree:tr").want(wantOK()).
		op("exec", "root", "nil").want(wantOut("1")))
	return ss
}
