//go:build gotemplate_oracle

// Red-team mode "rtexec": seeded grammar-generated templates executed over
// the "exec" mode's data model, functions and Hugo-like ExecHelper
// (exec.go), in both "plain" and "hugo" modes, written in the exec.txt.gz
// record format so crates/gotemplate/tests/exec_oracle.rs can replay them.
//
//	GOTOOLCHAIN=go1.27.1 go run -tags gotemplate_oracle ./tools/go-oracle/gotemplate rtexec <out.txt.gz> <seed> <n>
//
// Not checked in: exec_oracle.rs's ignored test exec_redteam reads it from
// $GOTEMPLATE_RT_EXEC.
package main

import (
	"bufio"
	"fmt"
	"math/rand"
	"os"
	"strconv"
	"strings"
)

func init() {
	register("rtexec", rtExecMain)
	register("rtexeclist", rtExecListMain)
	register("rtpairs", rtPairsMain)
}

// rtPairValues are the values of the systematic modes: etxValues (exec.go)
// and a few more expressions.
func rtPairValues() []string {
	vals := append([]string(nil), etxValues...)
	vals = append(vals, ".T.Iface", ".M.a", ".M.list", ".MS.a", ".P.title", ".SI", "(index .SI 0)", "(len .SS)",
		`"a"`, `"b"`, "0", "2", "255", "-8", "0.0", "1.0", `"1"`, "(echo 3)", "(echo .U8)")
	seen := map[string]bool{}
	var out []string
	for _, v := range vals {
		if !seen[v] {
			seen[v] = true
			out = append(out, v)
		}
	}
	return out
}

// rtPairsMain writes systematic cases: "pairs" puts every pair of values
// through the comparison, index/slice, and/or, print, escaping and call
// builtins; "fmt" puts every value through many printf formats.
//
//	rtpairs <out.txt.gz> <pairs|fmt>
func rtPairsMain(args []string) error {
	if len(args) != 2 {
		return fmt.Errorf("usage: rtpairs <out.txt.gz> <pairs|fmt>")
	}
	vals := rtPairValues()
	var srcs []string
	switch args[1] {
	case "pairs":
		shapes := []string{"{{eq X Y}}", "{{ne X Y}}", "{{lt X Y}}", "{{le X Y}}", "{{gt X Y}}", "{{ge X Y}}",
			"{{eq X Y 1 X}}", "{{index X Y}}", "{{slice X Y}}", "{{slice X 0 Y}}", "{{and X Y}}", "{{or X Y}}",
			"{{print X Y}}", "{{html X Y}}", "{{js X Y}}", "{{urlquery X Y}}", "{{call X Y}}",
			`{{Y | printf "%v|%v" X}}`}
		for _, x := range vals {
			for _, y := range vals {
				for _, sh := range shapes {
					srcs = append(srcs, strings.NewReplacer("X", x, "Y", y).Replace(sh))
				}
			}
		}
	case "fmt":
		fmts := []string{"%v", "%+v", "%#v", "%T", "%d", "%s", "%q", "%x", "%X", "% x", "%#x", "%o", "%O", "%b",
			"%e", "%E", "%f", "%F", "%g", "%G", "%t", "%c", "%U", "%#U", "%p", "%5v", "%-5v|", "%05d", "%.2f", "%8.3e",
			"%+q", "%#q", "%10.4q", "%-#8x", "%.3s", "%.0f", "%#g", "%x %X", "%v %T", "%[2]v", "%!", "%", "%z", "%.*d",
			"%*v", "%6.2v", "%-08d", "% d", "%+d", "%#o", "%#b", "%#e", "%3c", "%-3c|", "%08.3f", "%+.3e", "%x"}
		for _, x := range vals {
			for _, f := range fmts {
				srcs = append(srcs, "{{printf "+strconv.Quote(f)+" "+x+"}}", "{{printf "+strconv.Quote(f)+" "+x+" "+x+"}}")
			}
		}
	default:
		return fmt.Errorf("rtpairs: unknown kind %q", args[1])
	}
	return rtWriteGz(args[0], func(w *bufio.Writer) error {
		k := 0
		for _, src := range srcs {
			c := etxCase{name: "rt", src: src, data: "root", mode: "both"}
			for _, mode := range []string{"plain", "hugo"} {
				etxRun(w, k, c, mode)
				k++
			}
		}
		fmt.Fprintf(os.Stderr, "rtpairs %s: %d cases\n", args[1], k)
		return nil
	})
}

// rtExecListMain writes the exec records of a list of cases, one per line
// of the input file: `<data> <quoted option> <quoted source>` (for
// minimizing a red-team finding by hand).
//
//	rtexeclist <out.txt.gz> <cases.txt>
func rtExecListMain(args []string) error {
	if len(args) != 2 {
		return fmt.Errorf("usage: rtexeclist <out.txt.gz> <cases.txt>")
	}
	b, err := os.ReadFile(args[1])
	if err != nil {
		return err
	}
	var cs []etxCase
	for _, line := range strings.Split(string(b), "\n") {
		if strings.TrimSpace(line) == "" || strings.HasPrefix(line, "#") {
			continue
		}
		c, err := rtParseCaseLine(line)
		if err != nil {
			return fmt.Errorf("%q: %v", line, err)
		}
		cs = append(cs, c)
	}
	return rtWriteGz(args[0], func(w *bufio.Writer) error {
		k := 0
		for _, c := range cs {
			for _, mode := range []string{"plain", "hugo"} {
				etxRun(w, k, c, mode)
				k++
			}
		}
		return nil
	})
}

// rtParseCaseLine parses `<data> <quoted option> <quoted source>`.
func rtParseCaseLine(line string) (etxCase, error) {
	data, rest, _ := strings.Cut(line, " ")
	opt, err := strconv.QuotedPrefix(rest)
	if err != nil {
		return etxCase{}, err
	}
	src := strings.TrimSpace(rest[len(opt):])
	c := etxCase{name: "rt", data: data, mode: "both"}
	if c.opt, err = strconv.Unquote(opt); err != nil {
		return c, err
	}
	if c.src, err = strconv.Unquote(src); err != nil {
		return c, err
	}
	return c, nil
}

func rtExecMain(args []string) error {
	out, seed, n, err := rtArgs("rtexec", args)
	if err != nil {
		return err
	}
	r := rand.New(rand.NewSource(seed))
	return rtWriteGz(out, func(w *bufio.Writer) error {
		k := 0
		for i := 0; i < n; i++ {
			c := rtExecCase(r)
			for _, mode := range []string{"plain", "hugo"} {
				etxRun(w, k, c, mode)
				k++
			}
		}
		fmt.Fprintf(os.Stderr, "rtexec: seed %d, %d templates, %d cases\n", seed, n, k)
		return nil
	})
}

var rtExecDataKinds = []string{"root", "root", "root", "root", "root", "root", "nil", "int", "string", "typednil", "t",
	"list", "params", "siteparams"}

var rtExecOptions = []string{"", "", "", "", "missingkey=default", "missingkey=invalid", "missingkey=zero",
	"missingkey=error"}

func rtExecCase(r *rand.Rand) etxCase {
	g := &rtExecGen{r: r}
	var b strings.Builder
	// Named templates first; each may call the ones before it (no recursion).
	for i := r.Intn(3); i > 0; i-- {
		name := fmt.Sprintf("a%d", len(g.defs))
		saved := g.vars
		g.vars = nil
		body := g.list(1)
		g.vars = saved
		kw := "define"
		if r.Intn(4) == 0 {
			kw = "block"
		}
		if kw == "define" {
			fmt.Fprintf(&b, "{{define %q}}%s{{end}}", name, body)
		} else {
			fmt.Fprintf(&b, "{{block %q %s}}%s{{end}}", name, g.pipeline(0), body)
		}
		g.defs = append(g.defs, name)
	}
	b.WriteString(g.list(0))
	c := etxCase{name: "rt", src: b.String(), data: rtExecDataKinds[r.Intn(len(rtExecDataKinds))],
		opt: rtExecOptions[r.Intn(len(rtExecOptions))], mode: "both"}
	if len(g.defs) > 0 && r.Intn(8) == 0 {
		c.exec = g.defs[r.Intn(len(g.defs))]
	}
	return c
}

// rtExecGen generates templates over the exec data model.
type rtExecGen struct {
	r       *rand.Rand
	vars    []string
	inRange int
	defs    []string
	nvar    int
}

func (g *rtExecGen) pick(ss ...string) string { return ss[g.r.Intn(len(ss))] }

// rtExecTop are the root keys of etxRoot (exec.go), minus the huge ints
// (kept out of range positions separately).
var rtExecTop = []string{
	"T", "TV", "NilT", "Nil", "V", "PVal", "Pages", "EmptyPages", "NilPages", "D", "P", "PMerge", "PEmpty",
	"SiteParams", "Site", "M", "MS", "L", "SS", "SI", "SB", "SF", "SBytes", "SM", "Empty", "EmptySS", "EmptyM",
	"NilM", "NilSS", "I", "I8", "I16", "I32", "I64", "U", "U8", "U16", "U32", "U64", "UP", "F32", "F64", "FBig",
	"FSmall", "MaxI", "MinI", "Neg", "Zero", "ZeroU", "ZeroF", "NegZero", "NaN", "Inf", "S", "ES", "Bad", "Uni",
	"HTML", "EHTML", "JS", "True", "False", "Time", "ZeroTime", "Z", "NZ", "Str", "SV", "Err", "Fn", "Fn2", "NilFn",
	"Nested", "NumKeys", "Missing",
}

// rtExecNames are field, method and key names of the model's types.
var rtExecNames = []string{
	"Name", "N", "F", "B", "Sub", "NilSub", "V", "PV", "Iface", "NilIface", "Err", "NilErr", "Str", "NilStr", "M",
	"NilM", "SS", "NilSS", "L", "Fn", "NilFn", "Pages", "Time", "H", "VM", "PM", "NilOK", "Self", "GetSub", "RetNil",
	"RetNilPtr", "RetNilStr", "RetNilMap", "RetM", "A", "VV", "PV2", "Len", "First", "Reverse", "Count", "Singular",
	"Zero", "Label", "IsZero", "S", "String", "Error", "Title", "MainSections", "mainSections", "title", "ymap",
	"ynull", "list", "mixed_case", "iszero", "b", "c", "d", "a", "Key", "key", "nil", "sub", "x", "v", "empty",
	"Year", "Unix", "Day", "UTC", "Format", "Value", "Cause", "Unwrap", "GetNested", "Social", "facebook", "enable",
	"1", "é", "_", "Missing", "Echo", "Two", "Var", "Fail", "Panic",
}

// rtExecFuncs are the functions of etxFuncs and the builtins, with
// weights by repetition.
var rtExecFuncs = []string{
	"echo", "echo", "echo2", "fail", "failNil", "failErr", "nilany", "nilptr", "nilerr", "nilstr", "nilmap",
	"nilslice", "mkhtml", "try", "try", "typeof", "typeof", "isnil", "variadic", "panicky", "panicerr", "errval",
	"dict", "list", "mkT", "getfn",
	"and", "and", "or", "or", "not", "len", "len", "index", "index", "slice", "print", "printf", "printf",
	"println", "eq", "eq", "ne", "lt", "le", "gt", "ge", "html", "js", "urlquery", "call",
}

var rtExecFormats = []string{
	`"%v"`, `"%d"`, `"%s"`, `"%q"`, `"%x"`, `"%X"`, `"%o"`, `"%O"`, `"%b"`, `"%e"`, `"%E"`, `"%f"`, `"%F"`, `"%g"`,
	`"%G"`, `"%t"`, `"%c"`, `"%U"`, `"%#U"`, `"%T"`, `"%+v"`, `"%#v"`, `"%5d"`, `"%-5s|"`, `"%.2f"`, `"%08.3f"`,
	`"%+d"`, `"% d"`, `"%#x"`, `"%#o"`, `"%[2]v %[1]v"`, `"%*d"`, `"%.*f"`, `"%!"`, `"%%"`, `"%"`, `"%z"`,
	`"%[3]v"`, `"%v %v %v"`, `"%10.4q"`, `"%-#8x"`, `"% x"`, `"%#q"`, `"%+q"`, `"%6.2v"`, `"%v|%T"`, `"%x %X"`,
	`"%.3s"`, `"%.0f"`, `"%#g"`, `"%-08d"`, `"%[1]*[2]d"`, `"%.[2]d"`, "`%v`", `"é%vé"`, `"%\xffv"`,
}

func (g *rtExecGen) literal() string {
	switch g.r.Intn(4) {
	case 0:
		return g.pick("0", "1", "-1", "2", "3", "42", "255", "256", "-8", "1.5", "0.0", "-0.0", "1e3", "1e21", "0x10",
			"'a'", "'é'", "9223372036854775807", "-9223372036854775808", "1e-7", "0.1", "2.5", "017", "0b11",
			"18446744073709551615", "1i", "1_000", "0x1p4")
	case 1:
		return g.pick(`"s"`, `""`, `"a"`, `"hello"`, `"<b>&'\""`, `"a b"`, `"\xff"`, `"é"`, "`raw`", `"%d"`,
			`"2006-01-02"`, `"Title"`, `"title"`, `"a"`, `"b"`, `"x"`, `"nil"`, `"Key"`, `"1"`, `"javascript:x"`,
			`" "`, `"\x00"`, `"a\nb"`, `"</script>"`)
	case 2:
		return g.pick("true", "false", "nil")
	}
	return g.pick("1", `"s"`, "0", "true", "nil", "2")
}

func (g *rtExecGen) path() string {
	var b strings.Builder
	switch g.r.Intn(8) {
	case 0:
		b.WriteString(".")
		if g.r.Intn(2) == 0 {
			return "."
		}
		b.WriteString(rtExecNames[g.r.Intn(len(rtExecNames))])
	case 1:
		b.WriteString(g.variable())
		if b.String() == "$" && g.r.Intn(2) == 0 {
			b.WriteString(".")
			b.WriteString(rtExecTop[g.r.Intn(len(rtExecTop))])
		}
	default:
		b.WriteString(".")
		b.WriteString(rtExecTop[g.r.Intn(len(rtExecTop))])
	}
	for i := g.r.Intn(3); i > 0; i-- {
		b.WriteString(".")
		b.WriteString(rtExecNames[g.r.Intn(len(rtExecNames))])
	}
	return b.String()
}

func (g *rtExecGen) variable() string {
	if len(g.vars) > 0 && g.r.Intn(4) != 0 {
		return g.vars[g.r.Intn(len(g.vars))]
	}
	return "$"
}

// operand is an argument (never a bare function name, so arities stay
// under the generator's control: function calls are parenthesised).
func (g *rtExecGen) operand(depth int) string {
	switch k := g.r.Intn(12); {
	case k < 4:
		return g.path()
	case k < 7:
		return g.literal()
	case k < 10 && depth < 3:
		return "(" + g.pipeline(depth+1) + ")"
	case k < 11 && depth < 3:
		return "(" + g.pipeline(depth+1) + ")." + rtExecNames[g.r.Intn(len(rtExecNames))]
	}
	return g.pick(".", "$", ".S", ".I", ".SS", ".M", ".T")
}

// rtHostArity is the Go arity of the model's host functions and methods
// (-1: variadic, -2: variadic with an even count). The engine does not
// know host signatures (PORTING deviation 6: Go checks arity and argument
// types before evaluating the arguments), so the generator mostly calls
// them correctly, to keep the comparison exact.
var rtHostArity = map[string]int{
	"echo": 1, "echo2": 2, "fail": 1, "failNil": 0, "failErr": 0, "nilany": 0, "nilptr": 0, "nilerr": 0,
	"nilstr": 0, "nilmap": 0, "nilslice": 0, "mkhtml": 1, "try": 1, "typeof": 1, "isnil": 1, "variadic": -1,
	"panicky": 0, "panicerr": 0, "errval": 0, "dict": -2, "list": -1, "mkT": 1, "getfn": 0,
	".T.Echo": 1, ".T.Two": 2, ".T.Var": -1, ".T.Sub.Echo": 1, ".T.Name": 0, ".M.a": 0, ".Fn": 0,
}

func (g *rtExecGen) args(depth, n int) string {
	var b strings.Builder
	for i := 0; i < n; i++ {
		b.WriteString(" ")
		b.WriteString(g.operand(depth))
	}
	return b.String()
}

// hostArgs returns arguments for a host function or method, usually with
// its arity (a piped value counts as one argument).
func (g *rtExecGen) hostArgs(depth int, name string, piped bool) string {
	n, ok := rtHostArity[name]
	if !ok || g.r.Intn(12) == 0 {
		return g.args(depth, g.r.Intn(4))
	}
	switch n {
	case -1:
		n = g.r.Intn(4)
	case -2:
		n = 2 * g.r.Intn(3)
		if piped {
			n++
		}
		var b strings.Builder
		for i := 0; i < n; i++ {
			if i%2 == 0 && g.r.Intn(8) != 0 {
				b.WriteString(" " + g.pick(`"a"`, `"b"`, `"k"`, `"Title"`))
			} else {
				b.WriteString(" " + g.operand(depth))
			}
		}
		return b.String()
	default:
		if piped {
			n--
		}
		if n < 0 {
			return ""
		}
	}
	return g.args(depth, n)
}

func (g *rtExecGen) command(depth int, piped bool) string {
	if !piped && g.r.Intn(4) == 0 {
		// A value, possibly a method called with arguments.
		p := g.path()
		if g.r.Intn(4) == 0 {
			m := g.pick(".T.Echo", ".T.Two", ".T.Var", ".T.Fn", ".M.a", ".T.Name", ".Time.Format", ".P.GetNested",
				".T.Sub.Echo", ".Fn")
			switch m {
			case ".Time.Format":
				return m + " " + g.pick(`"2006-01-02"`, `"Jan 2 15:04:05.000 MST"`, `""`, `"Monday"`, "`3PM`")
			case ".P.GetNested":
				for i := g.r.Intn(3); i >= 0; i-- {
					m += " " + g.pick(`"ymap"`, `"YMAP"`, `"b"`, `"c"`, `"d"`, `"title"`, `"nope"`, `"list"`)
				}
				return m
			}
			return m + g.hostArgs(depth, m, false)
		}
		return p
	}
	if !piped && g.r.Intn(8) == 0 {
		return g.literal()
	}
	if g.r.Intn(40) == 0 {
		// Typed parameters, with arguments of the right type.
		if piped {
			return g.pick("strarg", "intarg")
		}
		return g.pick(`strarg "x"`, `strarg .S`, `intarg 3`, `intarg .I`, `intarg -1`, `strarg (print 1)`)
	}
	fn := rtExecFuncs[g.r.Intn(len(rtExecFuncs))]
	if piped && g.r.Intn(6) == 0 {
		// A method or value receiving the piped argument.
		fn = g.pick(".T.Echo", ".T.Var", ".T.Two 1", ".Fn", ".T.Name", "$")
		if fn == ".T.Two 1" {
			return fn
		}
	}
	if fn == "printf" && g.r.Intn(4) != 0 {
		return fn + " " + rtExecFormats[g.r.Intn(len(rtExecFormats))] + g.args(depth, g.r.Intn(3))
	}
	if _, ok := rtHostArity[fn]; ok {
		return fn + g.hostArgs(depth, fn, piped)
	}
	return fn + g.args(depth, g.r.Intn(4))
}

func (g *rtExecGen) pipeline(depth int) string {
	var b strings.Builder
	n := 1
	if g.r.Intn(3) == 0 {
		n += 1 + g.r.Intn(2)
	}
	for i := 0; i < n; i++ {
		if i > 0 {
			b.WriteString(" | ")
		}
		b.WriteString(g.command(depth, i > 0))
	}
	return b.String()
}

// rangePipe is a range target with bounded iteration counts.
func (g *rtExecGen) rangePipe() string {
	if g.r.Intn(5) == 0 {
		return g.pick("(slice .SS 1)", "(echo .SI)", "(list 1 2 3)", "(list)", "(dict \"b\" 1 \"a\" 2)",
			"(index .SM 0)", "(len .SS)", "(try .SS).Value", ".Pages.Reverse", "(slice .L 2)", "(.T.RetM)",
			"(nilslice)", "(nilmap)", "(echo nil)", "(index .M \"list\")", "(or .Empty .SS)", "(and .SS .SI)",
			"(.T.Echo .SI)", "(mkhtml 1)", "(print 3)", "(len .M)")
	}
	return g.pick(".SS", ".SI", ".L", ".M", ".P", ".D", ".Pages", ".EmptyPages", ".NilPages", ".NilM", ".NilSS",
		".Empty", "0", "1", "3", "-2", ".U8", ".I", ".U", ".Zero", ".Neg", ".S", ".T", ".Nil", ".Missing", ".MS",
		".SB", ".SF", ".SBytes", ".SM", ".NumKeys", ".Nested", ".T.Pages", ".T.SS", ".T.L", ".T.M", ".M.list",
		".M.sub", ".P.ymap", ".P.list", ".EmptyM", ".EmptySS", ".U16", ".I8", ".ZeroU", ".F64", ".True", ".HTML",
		".Time", ".V", ".Fn", ".NilT", ".T.NilSS", ".T.NilM", ".M.nil", ".SiteParams", ".PMerge", "'a'", "1.5",
		"true", "nil", "$")
}

func (g *rtExecGen) newVar() string {
	g.nvar++
	return fmt.Sprintf("$v%d", g.nvar)
}

func (g *rtExecGen) list(depth int) string {
	var b strings.Builder
	n := 1 + g.r.Intn(4)
	for i := 0; i < n; i++ {
		b.WriteString(g.item(depth))
	}
	return b.String()
}

func (g *rtExecGen) item(depth int) string {
	switch k := g.r.Intn(20); {
	case k < 3:
		return g.pick("x", " ", "\n", "|", "<", "é", ";")
	case k < 9:
		return "{{" + g.pipeline(0) + "}}"
	case k < 11:
		if len(g.vars) > 0 && g.r.Intn(3) == 0 {
			return "{{" + g.vars[g.r.Intn(len(g.vars))] + " = " + g.pipeline(0) + "}}"
		}
		v := g.newVar()
		s := "{{" + v + " := " + g.pipeline(0) + "}}"
		g.vars = append(g.vars, v)
		return s
	case k < 16 && depth < 3:
		mark := len(g.vars)
		var head string
		kw := g.pick("if", "with", "range", "if", "with", "range")
		switch kw {
		case "range":
			p := g.rangePipe()
			switch g.r.Intn(4) {
			case 0:
				i, e := g.newVar(), g.newVar()
				head = "range " + i + ", " + e + " := " + p
				g.vars = append(g.vars, i, e)
			case 1:
				e := g.newVar()
				head = "range " + e + " := " + p
				g.vars = append(g.vars, e)
			default:
				head = "range " + p
			}
			g.inRange++
		case "with":
			if g.r.Intn(3) == 0 {
				v := g.newVar()
				head = "with " + v + " := " + g.pipeline(0)
				g.vars = append(g.vars, v)
			} else {
				head = "with " + g.pipeline(0)
			}
		default:
			head = "if " + g.pipeline(0)
		}
		body := g.list(depth + 1)
		if kw == "range" {
			g.inRange--
		}
		g.vars = g.vars[:mark]
		var els string
		switch g.r.Intn(4) {
		case 0:
			els = "{{else}}" + g.list(depth+1)
		case 1:
			if kw != "range" {
				ek := g.pick("if", "with")
				els = "{{else " + ek + " " + g.pipeline(0) + "}}" + g.list(depth+1)
				if g.r.Intn(2) == 0 {
					els += "{{else}}" + g.list(depth+1)
				}
			}
		}
		g.vars = g.vars[:mark]
		return "{{" + head + "}}" + body + els + "{{end}}"
	case k < 17:
		if g.inRange > 0 {
			return g.pick("{{break}}", "{{continue}}", "{{if "+g.pipeline(0)+"}}{{break}}{{end}}",
				"{{if "+g.pipeline(0)+"}}{{continue}}{{end}}")
		}
	case k < 19:
		if len(g.defs) > 0 {
			name := g.defs[g.r.Intn(len(g.defs))]
			if g.r.Intn(3) == 0 {
				return fmt.Sprintf("{{template %q}}", name)
			}
			return fmt.Sprintf("{{template %q %s}}", name, g.pipeline(0))
		}
		if g.r.Intn(4) == 0 {
			return `{{template "missing"}}`
		}
	default:
		return g.pick("{{/* c */}}", "{{- /* c */ -}}", " {{- 1 -}} ", "{{- .S}}")
	}
	return "{{" + g.pipeline(0) + "}}"
}
