package main

import (
	"bufio"
	"bytes"
	"fmt"
	"os"
	"strconv"
	"strings"
)

// limitSpec describes a large input compactly: prefix + unit1*n + middle +
// unit2*n + suffix, where "%d" in a unit is replaced by the repetition index.
// The Rust test rebuilds the inputs from the specs, so the fixture stays
// small although the inputs reach megabytes.
type limitSpec struct {
	prefix, unit1, middle, unit2, suffix string
}

func (s limitSpec) build(n int) []byte {
	var b bytes.Buffer
	b.WriteString(s.prefix)
	rep := func(u string) {
		if strings.Contains(u, "%d") {
			for i := 0; i < n; i++ {
				b.WriteString(strings.ReplaceAll(u, "%d", strconv.Itoa(i)))
			}
		} else {
			b.WriteString(strings.Repeat(u, n))
		}
	}
	rep(s.unit1)
	b.WriteString(s.middle)
	rep(s.unit2)
	b.WriteString(s.suffix)
	return b.Bytes()
}

var nestSpecs = []limitSpec{
	{"", "(", "a", ")", ""},
	{"x=", "(", "a", ")", ""},
	{"", "[", "a", "]", ""},
	{"", "{", "a", "}", ""},
	{"", "{a:", "1", "}", ""},
	{"x=", "{a:", "1", "}", ""},
	{"", "if(a)", "b", "", ""},
	{"", "while(a)", "b", "", ""},
	{"", "for(;;)", "b", "", ""},
	{"", "for(a of b)", "c", "", ""},
	{"", "l:", "b", "", ""},
	{"", "a+", "a", "", ""},
	{"", "a=", "a", "", ""},
	{"", "a.b", "", "", ""},
	{"", "a[0]", "", "", ""},
	{"", "a()", "", "", ""},
	{"", "a,", "a", "", ""},
	{"x=", "a,", "a", "", ""},
	{"", "a?a:", "a", "", ""},
	{"", "!", "a", "", ""},
	{"", "- ", "a", "", ""},
	{"", "typeof ", "a", "", ""},
	{"", "new ", "a", "", ""},
	{"", "x=>", "x", "", ""},
	{"", "(x)=>", "x", "", ""},
	{"", "async x=>", "x", "", ""},
	{"", "function(){", "", "}", ""},
	{"", "(function(){", "", "})", ""},
	{"", "x=function(){", "", "}", ""},
	{"", "()=>{", "", "}", ""},
	{"", "a[", "0", "]", ""},
	{"", "a(", "0", ")", ""},
	{"", "`${", "a", "}`", ""},
	{"", "a?.b", "", "", ""},
	{"", "a`t`", "", "", ""},
	{"", "class{m(){", "", "}}", ""},
	{"", "x=class{m(){", "", "}}", ""},
	{"function*g(){", "yield ", "a", "", "}"},
	{"async function g(){", "await ", "a", "", "}"},
	{"", "[a,", "b", "]", "=c"},
	{"(", "[a,", "b", "]", ")=>1"},
	{"(", "{a:", "b", "}", ")=>1"},
	{"var ", "[", "a", "]", "=b"},
	{"var ", "{a:", "b", "}", "=c"},
	{"", "a**", "a", "", ""},
	{"", "a??", "a", "", ""},
	{"", "++", "a", "", ""},
	{"", "a\n", "", "", ""},
	{"", "try{", "", "}finally{}", ""},
	{"", "switch(a){case 1:", "", "}", ""},
	{"", "do ", "a", ";while(1)", ""},
	{"", "with(a)", "b", "", ""},
}

var nestCounts = []int{0, 1, 2, 332, 333, 334, 498, 499, 500, 501, 997, 998, 999, 1000, 1001, 1002, 1999, 2000, 2001}

// uint16 wrap-around of Uses, NumForDecls, NumFuncArgs and NumArgUses
var wrapSpecs = []struct {
	spec limitSpec
	ns   []int
}{
	{limitSpec{"", "a;", "", "", ""}, []int{65534, 65535, 65536, 65537}},
	{limitSpec{"var a;", "a;", "a=>1", "", ""}, []int{65533, 65534, 65535, 65536}},
	{limitSpec{"", "a;", "a=>1", "", ""}, []int{65533, 65534, 65535, 65536}},
	{limitSpec{"b;", "a;", "a=>1", "", ""}, []int{65533, 65534, 65535, 65536}},
	{limitSpec{"", "a;", "(a)=>1", "", ""}, []int{65534, 65535, 65536}},
	{limitSpec{"", "a;", "(a,b)", "", ""}, []int{65534, 65535, 65536}},
	{limitSpec{"function f(){", "a;", "}", "", ""}, []int{65534, 65535, 65536}},
	{limitSpec{"{", "a;", "}var a", "", ""}, []int{65534, 65535, 65536}},
	{limitSpec{"var a;function f(){", "a;", "}", "", ""}, []int{65534, 65535, 65536}},
	{limitSpec{"function f(", "a%d,", "){var a4464;let a5000}", "", ""}, []int{65535, 65536, 70000}},
	{limitSpec{"function f(", "a%d,", "){var a3;var b;b}", "", ""}, []int{65536, 65540}},
	{limitSpec{"function f(", "a%d=b,", "){var b}", "", ""}, []int{65535, 65536, 65537}},
	{limitSpec{"for(let [", "a%d,", "] of x){let a4464;let a1}", "", ""}, []int{65536, 70000}},
	{limitSpec{"for(let [", "a%d,", "] of x){let a2}", "", ""}, []int{65538}},
	{limitSpec{"for(var [", "b,", "] of x){let b}", "", ""}, []int{65536, 65537}},
	{limitSpec{"(", "a%d,", ")=>{var a1}", "", ""}, []int{65536, 65537}},
	{limitSpec{"(", "a%d=c,", ")=>{var c}", "", ""}, []int{65535, 65536, 65537}},
	{limitSpec{"x=>", "x;", "", "", ""}, []int{65535, 65536}},
}

// genLimits writes OUT (TSV, not compressed): for every spec and count, the
// spec fields (Go-quoted), the count and the FNV digests of all modes.
func genLimits(out string) {
	f, err := os.Create(out)
	if err != nil {
		panic(err)
	}
	w := bufio.NewWriter(f)
	_, _ = fmt.Fprintf(w, "# prefix\tunit1\tmiddle\tunit2\tsuffix\tn\t%s\n", strings.Join(modes, "\t"))
	emit := func(s limitSpec, n int) {
		src := s.build(n)
		_, _ = fmt.Fprintf(w, "%s\t%s\t%s\t%s\t%s\t%d", strconv.Quote(s.prefix), strconv.Quote(s.unit1), strconv.Quote(s.middle), strconv.Quote(s.unit2), strconv.Quote(s.suffix), n)
		for _, m := range modes {
			_, _ = fmt.Fprintf(w, "\t%s", digest(runMode(m, src)))
		}
		_, _ = fmt.Fprintln(w)
	}
	for _, s := range nestSpecs {
		for _, n := range nestCounts {
			emit(s, n)
		}
	}
	for _, ws := range wrapSpecs {
		for _, n := range ws.ns {
			emit(ws.spec, n)
		}
	}
	if err := w.Flush(); err != nil {
		panic(err)
	}
	if err := f.Close(); err != nil {
		panic(err)
	}
}
