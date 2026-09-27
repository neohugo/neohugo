package main

import (
	"fmt"
	"math/rand"
	"strings"
)

// adversarialInputs returns inputs aimed at the order-sensitive and
// size-sensitive parts of the minifier:
//   - long `var` lists mixing plain names, defaults and patterns:
//     minifyVarDecl's sort.SliceStable has a position-dependent less
//     (`j != 0`), so insertion sort blocks of 20 and SymMerge must match Go;
//   - many locals with tied use counts: renameScope's sort.Sort (pdqsort)
//     tie order decides the names;
//   - more than 54 (and 54+54*64) locals: multi-character names;
//   - deep nesting and long chains (the parser's limits, recursion, the
//     string-concatenation merge limit of 50);
//   - long string/template escapes and numbers.
func adversarialInputs(seed int64) [][]byte {
	rnd := rand.New(rand.NewSource(seed))
	names := strings.Split("a b c d e f g h i j k l m n o p q r s t u v w x y z A B C D E F G H I J K L M N O P Q R S T U V W X Y Z _ $ ab cd é xy", " ")
	pick := func() string { return names[rnd.Intn(len(names))] }
	var out [][]byte
	add := func(s string) { out = append(out, []byte(s)) }

	for _, n := range []int{3, 12, 13, 19, 20, 21, 25, 40, 41, 64, 100, 257} {
		for rep := 0; rep < 6; rep++ {
			var items []string
			for i := 0; i < n; i++ {
				switch r := rnd.Intn(100); {
				case r < 35:
					items = append(items, pick())
				case r < 60:
					items = append(items, fmt.Sprintf("%s=%d", pick(), i))
				case r < 75:
					items = append(items, fmt.Sprintf("[%s,%s]=o", pick(), pick()))
				case r < 90:
					items = append(items, fmt.Sprintf("{%s}=o", pick()))
				default:
					items = append(items, fmt.Sprintf("{%s:%s=1}=p", pick(), pick()))
				}
			}
			body := "var " + strings.Join(items, ",") + ";"
			switch {
			case rep%2 == 1:
				add("!function(){" + body + "g(" + strings.Join(names[:10], ",") + ")}()")
			case rep%3 == 0:
				add(body + "var z;for(var q in o);var w=1")
			default:
				add("x=function(){" + body + "for(var i=0;;);var t;t=1;return t}")
			}
		}
	}
	for _, n := range []int{13, 50, 55, 200, 1000, 3600} {
		vs := make([]string, n)
		for i := range vs {
			vs[i] = fmt.Sprintf("v%d", i)
		}
		var uses []string
		for k := 0; k < 5; k++ {
			m := 1 + rnd.Intn(n)
			perm := rnd.Perm(n)[:m]
			var u []string
			for _, p := range perm {
				u = append(u, vs[p])
			}
			uses = append(uses, strings.Join(u, ","))
		}
		add("!function(" + strings.Join(vs[:3], ",") + "){var " + strings.Join(vs[3:], ",") + ";" + strings.Join(uses, ";") + "}()")
		add("!function(){let " + strings.Join(vs, ",") + ";{let x=" + strings.Join(vs, "+") + "}}()")
	}
	rep := strings.Repeat
	add("x=" + rep("a+", 20000) + "a")
	add("x=" + rep("'a'+", 120) + "'b'")
	add("x=" + rep("'a'+b+", 120) + "'b'")
	add("x=" + rep("'a\"'+'b\\''+", 60) + "c")
	add("x=" + rep("!", 3000) + "a")
	add("x=" + rep("(", 990) + "a" + rep(")", 990))
	add("x=" + rep("(", 1001) + "a" + rep(")", 1001))
	add("x=" + rep("[", 990) + rep("]", 990))
	add("function f(){" + rep("if(a)b();else ", 900) + "c()}")
	add("function f(){" + rep("if(a){return b}", 300) + "return c}")
	add("function f(){" + rep("if(a)throw b;", 300) + "throw c}")
	add("x=" + rep("a?b:", 5000) + "c")
	add("x=(" + rep("a,", 20000) + "b)")
	add("function f(){" + rep("a();", 20000) + "return b}")
	add("x=a" + rep("()", 5000))
	add("x=a" + rep(".b", 10000))
	add("x=" + rep("function(){return ", 300) + "1" + rep("}", 300))
	add("x=" + rep("()=>", 2000) + "1")
	add(rep("{", 900) + rep("}", 900))
	add(rep("{", 1001) + rep("}", 1001))
	add("x='" + rep("\\x41\\u0042\\u{43}\\103\\n", 500) + "'")
	add("x=`" + rep("\\x41${a}\\u{1F600}\\`", 300) + "`")
	add("x=/" + rep("\\a[\\-\\]]", 300) + "/g")
	add("x=" + rep("0x1F+0b101+0o17+1_000+", 300) + "1")
	return out
}
