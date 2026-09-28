package tsupport

// The probe site of the probe oracle: the template-engine spec's probe lines
// (Appendix A: html/template, layouts/home.html; Appendix B: text/template,
// layouts/home.json), rewritten for the minimal function map (safeHTML,
// safeHTMLAttr, safeCSS, safeJS, safeURL, printf, jsonify, eq, ne, not, dict,
// slice) and the stub page (Title, Description, Kind, IsHome, Date, Params,
// Site.Title, Site.Params).

const probeHTML = `<html><head><title>{{ .Title }}</title></head><body>
PRINT-01:{{ .Params.yint }}|{{ .Params.yfloat }}|{{ .Params.yfloat0 }}|{{ .Params.ybig }}|{{ .Params.ystr }}|{{ .Params.ybool }}|{{ .Params.ynull }}|{{ .Params.nosuch }}|
PRINT-02:{{ .Params.ylist }}|{{ .Params.ylistmixed }}|{{ .Params.ymap }}|{{ .Params.ydate.Format "2006-01-02" }}|{{ .Params.yneg }}|{{ .Params.yexp }}|
PRINT-03:{{ .Date }}|{{ .Date.UTC }}|{{ .Params.ydate }}|{{ .Params.nosuch.Year }}|{{ .Date.IsZero }}|{{ .Date.Year }}|{{ .Date.Month }}|{{ .Date.Weekday }}|{{ .Date.Unix }}|{{ (.Date.AddDate 0 1 0).Format "Jan 2, 2006" }}|{{ .Date.YearDay }}|{{ .Date.Nanosecond }}|
PRINT-04:{{ .Site.Params.intv }}|{{ .Site.Params.floatv }}|{{ 2.5 }}|{{ 1e21 }}|{{ 1e-06 }}|{{ 1e-07 }}|{{ .Site.Params.arr }}|{{ .Params.Mixed_Case }}|{{ .Params.MIXED_CASE }}|{{ .Site.Params.nested }}|{{ .Site.Params.nested.KEY }}|
PRINT-05:{{ 1 }}|{{ 1.0 }}|{{ 1.5 }}|{{ 1_000 }}|{{ 0x10 }}|{{ 'a' }}|{{ true }}|{{ "s" }}|{{ print nil }}|{{ print 1 2 }}|{{ print "a" "b" }}|{{ print "a" 1 "b" }}|{{ println "x" }}|
PRINT-06:{{ printf "%T" .Params.yint }}|{{ printf "%T" .Params.yfloat }}|{{ printf "%T" .Params.ylist }}|{{ printf "%T" .Params.ymap }}|{{ printf "%T" .Date }}|{{ printf "%T" .Title }}|{{ printf "%T" 1 }}|{{ printf "%T" 1.5 }}|{{ printf "%T" (slice 1) }}|{{ printf "%T" .Date.Month }}|
PRINT-07:{{ .Params.ylist | jsonify }}|{{ .Params.ymap | jsonify }}|{{ jsonify .Title }}|{{ slice 1 2 3 }}|{{ dict "b" 1 "a" 2 }}|{{ slice }}|{{ jsonify (dict "z" 1 "a" (slice 1 2)) }}|{{ jsonify .Date }}|{{ jsonify .Params.ynull }}|
PRINT-08:{{ printf "%v|%q|%5.2f|%x|%08.3f|%d|%s" .Params.ystr .Title 3.14159 255 -1.5 .Params.yint .Params.ylist }}|{{ printf "%s" .Params.yhtml }}|{{ printf "%v" .Params.ymap }}|{{ printf "%#v" .Params.ylist }}|
PRINT-09:{{ .Title }}|{{ .Title | html }}|{{ .Title | urlquery }}|{{ .Title | js }}|{{ .Title | print }}|{{ .Params.yhtml }}|{{ .Params.yhtml | safeHTML }}|
TRUTH-01:{{ range slice .Params.ybool .Params.yfalse .Params.ynull .Params.nosuch .Params.yempty .Params.yzero .Params.yint .Params.ystr .Params.ylist .Params.yemptylist .Params.ymap .Params.yemptymap .Date .Params.yfloat 0.0 "0" }}{{ if . }}T{{ else }}F{{ end }}{{ end }}|{{ if .Params.nosuch.x }}T{{ else }}F{{ end }}|
AND-OR:{{ and 1 0 }}|{{ or 0 2 }}|{{ or "" "x" }}|{{ and "" "x" }}|{{ and true true }}|{{ or .Params.nosuch .Params.ystr }}|{{ and .Params.ystr .Params.yint }}|{{ or .Params.nosuch .Params.yzero }}|
LEN:{{ len .Title }}|{{ len .Params.ylist }}|{{ len .Params.ymap }}|{{ len (slice) }}|{{ len "日本" }}|
RANGE-MAP:{{ range $k, $v := .Params.ymap }}{{ $k }}={{ $v }};{{ end }}|{{ range $k, $v := dict "z" 1 "a" 2 "M" 3 }}{{ $k }}={{ $v }};{{ end }}|{{ range .Params.ymap }}{{ . }};{{ end }}|
RANGE-INT:{{ range 3 }}{{ . }}{{ end }}|{{ range $i, $e := slice "a" "b" }}{{ $i }}{{ $e }}{{ end }}|{{ range .Params.nosuch }}x{{ else }}EMPTY{{ end }}|{{ range .Params.yemptylist }}x{{ else }}EMPTY2{{ end }}|{{ range $i := 2 }}{{ $i }}{{ end }}|
WITH-ELSE:{{ with .Params.nosuch }}A{{ else }}B{{ end }}{{ with .Params.ystr }}{{ . }}{{ end }}|{{ with .Params.nosuch }}1{{ else with .Params.yint }}{{ . }}{{ end }}|{{ with $x := .Params.ystr }}{{ $x }}{{ end }}|
BREAK:{{ range slice 1 2 3 }}{{ if eq . 3 }}{{ break }}{{ end }}{{ . }}{{ end }}|{{ range slice 1 2 3 4 5 }}{{ if eq . 3 }}{{ continue }}{{ end }}{{ . }}{{ end }}|
EQ:{{ eq 1 1.0 }}|{{ eq 1 1 }}|{{ eq "a" "a" }}|{{ ne 1 2 }}|{{ eq .Params.yint 5 }}|{{ eq .Params.ystr "007" }}|{{ not true }}|{{ not .Params.nosuch }}|{{ eq .Params.yfloat0 5 }}|{{ eq .Params.yfloat0 5.0 }}|{{ eq .Params.nosuch nil }}|{{ eq 1 2 1 }}|{{ not .Params.ylist }}|
VARS:{{ $x := 1 }}{{ if true }}{{ $x := 2 }}{{ $x }}{{ end }}{{ $x }}|{{ $y := 1 }}{{ if true }}{{ $y = 2 }}{{ end }}{{ $y }}|{{ $.Title | printf "%.4s" }}|{{ range slice 1 }}{{ $.Kind }}{{ end }}|
ATTR:<a title="{{ .Title }}" href="{{ .Description }}" data-x="{{ .Params.ylist }}">x</a>
UNQ:<a title={{ .Title }}>x</a>
URL1:<a href="/p?q={{ .Title }}#{{ .Title }}">x</a>
URL2:<a href="{{ "javascript:alert(1)" }}">x</a><a href="{{ "http://x.org/a b?c=d&e" }}">y</a><a href="{{ "mailto:a@b" }}">z</a><img src="/a/{{ "ü b.png" }}">
SRCSET:<img srcset="{{ "javascript:x" }}, {{ "/c.png" }} 2x">
JS1:<script>var a = {{ .Title }}; var b = "{{ .Title }}"; var c = '{{ .Title }}'; var d = {{ .Params.ylist }}; var e = {{ .Params.ymap }}; var f = {{ .Params.yint }}; var g = {{ .Params.yfloat }}; var h = {{ .Date }}; var i = {{ .Params.ynull }}; var j = {{ .Params.nosuch }}; var k = {{ true }}; var l = {{ .Params.yneg }};</script>
JS2:<script>
{{/* comment */}}
 var x = 1; {{- /* trim */ -}}
var y = ` + "`" + `tmpl {{ .Title }}` + "`" + `;</script>
JS3:<button onclick="f({{ .Title }}, '{{ .Title }}')">b</button>
JSON-LD:<script type="application/ld+json">{"a": {{ .Title }}, "b": "{{ .Title }}", "d": "{{ .Date }}", "s": {{ "a'b\"c<d>&e+f" }} }</script>
NONJS:<script type="text/template">{{ .Title }} <b>{{ .Title }}</b></script>
CSS1:<style>p { color: red; background: url({{ "/a b.png" }}); font-family: "{{ .Title }}"; }   </style>
CSS2:<p style="color: {{ "expression(x)" }}">p</p>
TEXTAREA:<textarea>{{ .Title }}</textarea>
TEXTLT:a < b and c > d & e <?xml x?> <!DOCTYPE y>
SAFE:{{ "<b>x</b>" | safeHTML }}|<a href="{{ "javascript:x" | safeURL }}">x</a>|<p {{ "title=\"t+\"" | safeHTMLAttr }}>p</p>|<style>{{ "a{b:c}" | safeCSS }}</style>|<script>{{ "var x=1" | safeJS }}</script>|<p title="{{ "<b>x</b>" | safeHTML }}">p</p>
TYPES:{{ printf "%T" ("<b>x</b>" | safeHTML) }}|{{ printf "%T" (dict "a" 1) }}|{{ dict "a" 1 }}|{{ printf "%T" (jsonify 1) }}|
TPL:{{ template "tpl" .Title }}|{{ template "tpl2" }}|{{ block "blk" .Kind }}block {{ . }}{{ end }}|
H-TRIM:[{{- " x " -}}]|[  {{- "y" }}  ]|
H-ERR:{{ with try (printf "%d" 1) }}{{ .Value }}|{{ .Err }}{{ end }}|{{ with try (dict "a") }}{{ .Value }}|{{ .Err }}{{ end }}|
PRINTF-NIL:{{ printf "%s" .Params.nosuch }}|{{ printf "%s" .Params.ynull }}|<meta content="{{ printf "%s" .Params.nosuch }}">|{{ printf "%v" .Params.nosuch }}|{{ printf "%d" .Params.ystr }}|
</body></html>
{{ define "tpl" }}<i title="{{ . }}">{{ . }}</i>{{ end }}
{{ define "tpl2" }}<b>{{ . }}</b>{{ end }}
`

const probeJSON = `T-PRINT-01:{{ .Params.yint }}|{{ .Params.yfloat }}|{{ .Params.nosuch }}|{{ .Params.ynull }}|{{ .Params.ylist }}|{{ .Params.ymap }}|{{ .Date }}|{{ .Title }}|
T-PRINT-02:{{ .Site.Params.intv }}|{{ .Site.Params.nosuch }}|{{ .Params.nosuch.x }}|{{ .Title | html }}|{{ .Title | urlquery }}|{{ .Title | js }}|
T-PRINT-03:{{ print "a" 1 2 "b" }}|{{ print 1 2 }}|{{ print "a" "b" }}|{{ print 1.5 true }}|{{ .Params.ylistmixed }}|{{ print (slice 1 nil "x") }}|{{ print (dict "a" nil) }}|{{ printf "%s" .Params.nosuch }}|{{ printf "%v" .Params.ynull }}|{{ printf "%d" "x" }}|{{ printf "a %s" }}|{{ printf "a" "b" }}|{{ printf "%q" .Params.ylist }}|{{ printf "%x" "hi" }}|{{ printf "%5.1f" 3.14159 }}|{{ printf "%-4d" 7 }}|{{ printf "%04d" 7 }}|
T-PRINT-04:{{ printf "%T" .Params.yint }}|{{ printf "%T" .Params.ylistmixed }}|{{ printf "%T" 3 }}|{{ len .Params.ylist }}|{{ .Params.yint }}|{{ .Params.yfloat0 }}|{{ printf "%T" .Params.ybig }}|{{ .Params.yexp }}|
T-CASE:{{ .Site.Params.INTV }}|{{ .Params.Mixed_Case }}|{{ .Params.mixed_Case }}|
T-TRIM:[{{- "x" -}}]|[{{- "" -}}]|[{{ "a" -}} {{- "b" }}]|
T-SCOPE:{{ $x := 1 }}{{ if true }}{{ $x := 2 }}{{ $x }}{{ end }}{{ $x }}|{{ $y := 1 }}{{ if true }}{{ $y = 2 }}{{ end }}{{ $y }}|{{ template "sc" "dotval" }}|{{ with "w" }}{{ . }}{{ $.Title }}{{ end }}|
T-RANGE:{{ range $k, $v := .Params }}{{ $k }},{{ end }}|{{ range $i, $p := slice "P1" }}{{ $i }}:{{ $p }};{{ end }}|{{ range 1 }}{{ . }}{{ end }}{{ range 2 }}{{ . }}{{ end }}|
T-NUM:{{ 3 }}|{{ 1e6 }}|{{ 123456 }}|{{ 1234567.0 }}|{{ 0.0001 }}|{{ 0.00001 }}|{{ -0.0 }}|{{ 1e20 }}|{{ 0.1 }}|{{ 2.675 }}|{{ 1_000 }}|{{ 0b101 }}|{{ 0o17 }}|{{ 0xF }}|
T-JSON:{{ jsonify .Params }}|{{ jsonify .Site.Params }}|{{ .Params.ylist | jsonify }}|
{{ define "sc" }}{{ . }}{{ $ }}{{ end }}`

const probePlain = `P-01:{{ .Title }}|{{ .Params.yhtml }}|{{ .Params.yhtml | safeHTML }}|{{ printf "%T" (.Params.yhtml | safeHTML) }}|{{ .Params.ylist }}|
`

// ProbeSite is the probe oracle's site.
func ProbeSite() Site {
	return Site{
		Name: "probe",
		TOML: `baseURL = "https://example.org/"
title = "Probe"
[outputFormats.plain]
mediaType = "text/plain"
baseName = "plain"
isPlainText = true
[outputs]
home = ["html", "json", "plain"]
`,
		Files: map[string]string{
			"layouts/home.html":      probeHTML,
			"layouts/home.json":      probeJSON,
			"layouts/home.plain.txt": probePlain,
		},
		Render: false,
	}
}
