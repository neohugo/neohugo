//go:build gotemplate_oracle

// Red-team mode "rttns": random sequences of text/template namespace
// operations (New, Parse, Lookup, AddParseTree, Clone, Funcs, Delims,
// Option, Execute, ExecuteTemplate, Templates, DefinedTemplates), in the
// htmlexec script format with text operations (prefix "t"), replayed by
// crates/gotemplate/tests/html_exec.rs (html_redteam).
//
//	rttns <out.txt.gz> <seed> <n>
//
// Text operations (rtRunText; the Rust interpreter has the same):
//
//	O tnew V NAME / tnewassoc V FROM NAME            =>
//	O tparse V SRC                                    => ok | err, message
//	O tclone V FROM                                   => ok | err, message
//	O tlookup V FROM NAME                             => ok | nil
//	O taddtree V FROM NAME (tree:TR | of:V2)          => ok | err, message | NOTREE
//	O tfuncs V SET / tdelims V L R / toption V OPT    =>
//	O texec V DATA / texectmpl V NAME DATA            => output, error
//	O ttemplates V                                    => N, sorted names
//	O tdefined V                                      => DefinedTemplates (names sorted)
//	O tname V / thastree V / tptreq A B               => name | true/false
package main

import (
	"bufio"
	"bytes"
	"fmt"
	"math/rand"
	"os"
	"sort"
	"strconv"
	"strings"

	texttemplate "github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate"
	"github.com/neohugo/neohugo/tools/go-oracle/gotemplate/fork/texttemplate/parse"
)

func init() {
	register("rttns", rtTNSMain)
}

func rtTNSMain(args []string) error {
	out, seed, n, err := rtArgs("rttns", args)
	if err != nil {
		return err
	}
	r := rand.New(rand.NewSource(seed))
	var scripts []*script
	for i := 0; i < n; i++ {
		scripts = append(scripts, rtTNSScript(r, i))
	}
	fmt.Fprintf(os.Stderr, "rttns: seed %d, %d scripts\n", seed, n)
	return rtWriteGz(out, func(w *bufio.Writer) error { return rtWriteScripts(w, scripts) })
}

// rtTexts holds the text templates of each interpreter.
var rtTexts = map[*interp]map[string]*texttemplate.Template{}

func (in *interp) ttmpl(name string) *texttemplate.Template {
	t, ok := rtTexts[in][name]
	if !ok {
		panic(fmt.Sprintf("undefined text template variable %q", name))
	}
	return t
}

// rtRunText runs a text operation; ok is false for other operations.
func rtRunText(in *interp, o *sop) (res []string, ok bool) {
	if rtTexts[in] == nil {
		rtTexts[in] = map[string]*texttemplate.Template{}
	}
	tt := rtTexts[in]
	a := o.args
	okOrErrT := func(err error) []string {
		if err != nil {
			return []string{"err", err.Error()}
		}
		return []string{"ok"}
	}
	switch o.kind {
	case "tnew":
		tt[a[0]] = texttemplate.New(a[1])
		return nil, true
	case "tnewassoc":
		tt[a[0]] = in.ttmpl(a[1]).New(a[2])
		return nil, true
	case "tparse":
		_, err := in.ttmpl(a[0]).Parse(a[1])
		return okOrErrT(err), true
	case "tclone":
		c, err := in.ttmpl(a[1]).Clone()
		if err == nil {
			tt[a[0]] = c
		}
		return okOrErrT(err), true
	case "tlookup":
		x := in.ttmpl(a[1]).Lookup(a[2])
		if x == nil {
			return []string{"nil"}, true
		}
		tt[a[0]] = x
		return []string{"ok"}, true
	case "taddtree":
		var tree *parse.Tree
		switch {
		case strings.HasPrefix(a[3], "tree:"):
			tree = in.tr[a[3][5:]]
		case strings.HasPrefix(a[3], "of:"):
			tree = in.ttmpl(a[3][3:]).Tree
		}
		if tree == nil {
			return []string{"NOTREE"}, true
		}
		x, err := in.ttmpl(a[1]).AddParseTree(a[2], tree)
		if err == nil {
			tt[a[0]] = x
		}
		return okOrErrT(err), true
	case "tfuncs":
		fm := texttemplate.FuncMap{}
		var set map[string]any
		if a[1] == "stubs" {
			set = rtStubFuncs(a[2])
		} else {
			set = in.funcSet(a[1:])
		}
		for k, v := range set {
			fm[k] = v
		}
		in.ttmpl(a[0]).Funcs(fm)
		return nil, true
	case "tdelims":
		in.ttmpl(a[0]).Delims(a[1], a[2])
		return nil, true
	case "toption":
		in.ttmpl(a[0]).Option(a[1])
		return nil, true
	case "texec":
		var b bytes.Buffer
		err := in.ttmpl(a[0]).Execute(&b, in.decodeData(a[1]))
		return []string{b.String(), errString(err)}, true
	case "texectmpl":
		var b bytes.Buffer
		err := in.ttmpl(a[0]).ExecuteTemplate(&b, a[1], in.decodeData(a[2]))
		return []string{b.String(), errString(err)}, true
	case "ttemplates":
		var names []string
		for _, t := range in.ttmpl(a[0]).Templates() {
			names = append(names, t.Name())
		}
		sort.Strings(names)
		return append([]string{strconv.Itoa(len(names))}, names...), true
	case "tdefined":
		// Go lists the names in map order; the Rust port sorts them
		// (PORTING deviation 4): record them sorted.
		s := in.ttmpl(a[0]).DefinedTemplates()
		if rest, found := strings.CutPrefix(s, "; defined templates are: "); found {
			names := strings.Split(rest, ", ")
			sort.Strings(names)
			s = "; defined templates are: " + strings.Join(names, ", ")
		}
		return []string{s}, true
	case "tname":
		return []string{in.ttmpl(a[0]).Name()}, true
	case "thastree":
		return []string{strconv.FormatBool(in.ttmpl(a[0]).Tree != nil)}, true
	case "tptreq":
		return []string{strconv.FormatBool(in.ttmpl(a[0]) == in.ttmpl(a[1]))}, true
	}
	return nil, false
}

var rtTNSSources = []string{
	`a{{.}}b`, `{{define "x"}}X{{.}}{{end}}`, `{{define "x"}}{{end}}`, `{{define "x"}}  {{end}}`,
	`{{define "x"}}{{/* c */}}{{end}}`, `{{define "y"}}<{{template "x" .}}>{{end}}`, `{{template "x" .}}`,
	`{{template "y" .}}`, `{{template "missing" .}}`, `{{block "b" .}}B{{.}}{{end}}`, `{{define "b"}}redef{{end}}`,
	``, `  `, `{{/* only */}}`, `{{`, `{{end}}`, `{{define "t"}}T{{end}}`, `{{define "c"}}{{.}}{{end}}`,
	`[{{template "c" .}}]`, `[[.]]`, `<<.>>`, `[[define "x"]]dx[[end]]`, `{{define "r"}}{{if .}}{{template "r" ""}}{{end}}{{end}}{{template "r" .}}`,
	`{{.Missing}}`, `{{.Missing.X}}`, `{{safeHTML .}}`, `{{define "z"}}z{{end}}{{define "x"}}new x{{end}}`,
	`{{template "z"}}`, `{{define "_internal/p"}}p{{end}}`, `{{define "t"}}{{end}}`, `{{define "t"}}self{{end}}{{template "t"}}`,
	`{{index . "a"}}`, `{{with .}}{{.}}{{end}}`,
}

// rtTNSScript builds a random sequence of text namespace operations.
func rtTNSScript(r *rand.Rand, i int) *script {
	s := newScript(fmt.Sprintf("rttns/%d", i))
	vars := []string{"v0"}
	s.op("tnew", "v0", rtNSNames[r.Intn(4)])
	trees := 0
	pickVar := func() string { return vars[r.Intn(len(vars))] }
	newVar := func() string {
		v := fmt.Sprintf("v%d", len(vars))
		vars = append(vars, v)
		return v
	}
	pickName := func() string { return rtNSNames[r.Intn(len(rtNSNames))] }
	pickSrc := func() string { return rtTNSSources[r.Intn(len(rtTNSSources))] }
	value := func() string { return corpusValues[r.Intn(len(corpusValues))] }
	for k := 3 + r.Intn(10); k > 0; k-- {
		switch r.Intn(18) {
		case 0, 1, 2, 3:
			s.op("tparse", pickVar(), pickSrc())
		case 4:
			from := pickVar()
			s.op("tnewassoc", newVar(), from, pickName())
		case 5:
			s.op("tlookup", pickVar(), pickVar(), pickName())
			s.op("tname", pickVar())
		case 6:
			if r.Intn(2) == 0 {
				v := newVar()
				s.op("tnew", v, pickName())
				s.op("tclone", v, pickVar())
			} else {
				s.op("tclone", pickVar(), pickVar())
			}
		case 7, 8:
			s.op("texec", pickVar(), value())
		case 9:
			s.op("texectmpl", pickVar(), pickName(), value())
		case 10:
			s.op("ttemplates", pickVar())
			s.op("tdefined", pickVar())
		case 11:
			tr := fmt.Sprintf("tr%d", trees)
			trees++
			name := pickName()
			s.op("parsetree", tr, name, pickSrc(), name)
			if r.Intn(4) == 0 {
				s.op("taddtree", pickVar(), pickVar(), pickName(), "of:"+pickVar())
			} else {
				s.op("taddtree", pickVar(), pickVar(), pickName(), "tree:"+tr)
			}
		case 12:
			d := [][2]string{{"[[", "]]"}, {"{{", "}}"}, {"<<", ">>"}, {"", ""}}[r.Intn(4)]
			s.op("tdelims", pickVar(), d[0], d[1])
		case 13:
			s.op("toption", pickVar(), rtExecOptions[4+r.Intn(4)])
		case 14:
			s.op("tfuncs", pickVar(), "corpus")
		case 15:
			a, b := pickVar(), pickVar()
			s.op("tptreq", a, b)
			s.op("thastree", a)
		default:
			s.op("tnew", newVar(), pickName())
		}
	}
	return s
}
