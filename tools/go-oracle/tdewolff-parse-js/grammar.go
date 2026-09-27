package main

import (
	"bytes"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"strings"
)

// Grammar-based generator of (mostly valid) JavaScript programs. Mutation
// fuzzing of short literals mostly exercises error paths; these programs go
// deep into the parser's valid-program machinery: scope analysis (Declare,
// Use, hoisting, UndeclareScope for arrow-function speculation), ASI,
// regexp-versus-division, destructuring, classes and template literals. A
// fraction of the programs is mutated afterwards for the error paths.

type gen struct {
	rnd   *rand.Rand
	buf   bytes.Buffer
	depth int
	// context flags, to generate mostly valid programs
	inFunc, inAsync, inGen, inClass, inLoop, inSwitch bool
}

var gIdents = []string{
	"a", "b", "c", "d", "e", "x", "y", "i", "arguments", "eval", "undefined",
	"async", "await", "yield", "let", "of", "get", "set", "static", "as", "from", "target", "meta",
	"é", "\\u0061", "$", "_", "a1", "α",
}

// plain identifiers (safe everywhere)
var gPlain = []string{"a", "b", "c", "d", "e", "x", "y", "i", "$", "_", "a1", "é", "α"}

func (g *gen) n(k int) int { return g.rnd.Intn(k) }

func (g *gen) p(pct int) bool { return g.rnd.Intn(100) < pct }

func (g *gen) w(s string) { g.buf.WriteString(s) }

// sp writes a token separator: usually nothing or a space, sometimes a
// newline or a comment (which changes ASI and regexp decisions).
func (g *gen) sp() {
	switch r := g.n(40); {
	case r < 20:
	case r < 34:
		g.w(" ")
	case r < 37:
		g.w("\n")
	case r == 37:
		g.w("/*c*/")
	case r == 38:
		g.w("/*\n*/")
	default:
		g.w("//c\n")
	}
}

// ws writes mandatory whitespace (between two identifier-like tokens).
func (g *gen) ws() {
	switch g.n(12) {
	case 0:
		g.w("\n")
	case 1:
		g.w("/**/")
	case 2:
		g.w("\t")
	default:
		g.w(" ")
	}
}

func (g *gen) ident() string {
	if g.p(75) {
		return gPlain[g.n(len(gPlain))]
	}
	return gIdents[g.n(len(gIdents))]
}

func (g *gen) semi() {
	switch g.n(10) {
	case 0, 1, 2:
		g.w("\n")
	case 3:
		g.w(";\n")
	default:
		g.w(";")
	}
}

func (g *gen) program(module bool) {
	if g.p(10) {
		g.w("'use strict';")
	}
	n := 1 + g.n(6)
	for i := 0; i < n; i++ {
		if module && g.p(15) {
			g.moduleItem()
		} else {
			g.stmt(true)
		}
		g.sp()
	}
}

func (g *gen) moduleItem() {
	switch g.n(10) {
	case 0:
		g.w("import ")
		g.w(g.ident())
		g.w(" from'm'")
	case 1:
		g.w("import{")
		g.w(g.ident())
		if g.p(50) {
			g.w(" as ")
			g.w(g.ident())
		}
		g.w(",'s' as ")
		g.w(gPlain[g.n(len(gPlain))])
		g.w("}from\"m\"")
	case 2:
		g.w("import*as ")
		g.w(g.ident())
		g.w(" from'm'")
	case 3:
		g.w("import'm'")
	case 4:
		g.w("export default ")
		g.assignExpr()
	case 5:
		g.w("export ")
		g.stmt(true)
	case 6:
		g.w("export{")
		g.w(g.ident())
		g.w(" as default,")
		g.w(g.ident())
		g.w("}")
		if g.p(50) {
			g.w("from'm'")
		}
	case 7:
		g.w("export*")
		if g.p(50) {
			g.w("as ")
			g.w(g.ident())
		}
		g.w(" from'm'")
	case 8:
		g.w("export default function")
		if g.p(50) {
			g.w(" ")
			g.w(g.ident())
		}
		g.funcRest(false, false)
	case 9:
		g.w("export default class")
		g.classRest()
	}
	g.semi()
}

func (g *gen) block() {
	g.w("{")
	n := g.n(3)
	for i := 0; i < n; i++ {
		g.sp()
		g.stmt(true)
	}
	g.sp()
	g.w("}")
}

func (g *gen) stmt(decl bool) {
	g.depth++
	defer func() { g.depth-- }()
	if 5 < g.depth {
		g.exprStmt()
		return
	}
	switch g.n(34) {
	case 0, 1, 2:
		g.exprStmt()
	case 3:
		g.block()
	case 4, 5:
		if decl {
			kw := []string{"var", "let", "const"}[g.n(3)]
			g.w(kw)
			g.ws()
			g.varList(kw == "const")
		} else {
			g.w("var ")
			g.varList(false)
		}
		g.semi()
	case 6:
		g.w("if")
		g.sp()
		g.w("(")
		g.expr()
		g.w(")")
		g.sp()
		g.stmt(false)
		if g.p(40) {
			g.sp()
			g.w(" else ")
			g.stmt(false)
		}
	case 7:
		g.forStmt()
	case 8:
		g.w("while(")
		g.expr()
		g.w(")")
		g.loopBody()
	case 9:
		g.w("do")
		g.ws()
		g.loopBody()
		g.sp()
		g.w("while(")
		g.expr()
		g.w(")")
		if g.p(50) {
			g.semi()
		}
	case 10:
		g.w("switch(")
		g.expr()
		g.w("){")
		n := g.n(4)
		for i := 0; i < n; i++ {
			if g.p(80) {
				g.w("case ")
				g.expr()
				g.w(":")
			} else {
				g.w("default:")
			}
			m := g.n(3)
			old := g.inSwitch
			g.inSwitch = true
			for j := 0; j < m; j++ {
				g.stmt(true)
				g.sp()
			}
			g.inSwitch = old
		}
		g.w("}")
	case 11:
		g.w("try")
		g.block()
		switch g.n(4) {
		case 0:
			g.w("catch")
			g.block()
		case 1:
			g.w("catch(")
			g.binding()
			g.w(")")
			g.block()
			if g.p(30) {
				g.w("finally")
				g.block()
			}
		case 2:
			g.w("catch(")
			g.w(g.ident())
			g.w(")")
			g.block()
		default:
			g.w("finally")
			g.block()
		}
	case 12, 13:
		if !decl {
			g.exprStmt()
			return
		}
		g.funcDecl()
	case 14:
		if !decl {
			g.exprStmt()
			return
		}
		g.w("class ")
		g.w(gPlain[g.n(len(gPlain))])
		g.classRest()
	case 15:
		if g.inFunc && g.p(80) {
			g.w("return")
			if g.p(70) {
				g.sp()
				g.expr()
			}
			g.semi()
		} else {
			g.exprStmt()
		}
	case 16:
		g.w("throw ")
		g.expr()
		g.semi()
	case 17:
		if g.inLoop || g.inSwitch {
			if g.p(50) {
				g.w("break")
			} else if g.inLoop {
				g.w("continue")
			} else {
				g.w("break")
			}
			if g.p(20) {
				g.w(" l")
			}
			g.semi()
		} else {
			g.w(";")
		}
	case 18:
		g.w("l:")
		g.stmt(false)
	case 19:
		g.w("with(")
		g.expr()
		g.w(")")
		g.stmt(false)
	case 20:
		g.w("debugger")
		g.semi()
	case 21:
		g.w(";")
	case 22:
		// ASI hazards
		switch g.n(10) {
		case 0:
			g.w(g.ident())
			g.w("\n++")
			g.w(gPlain[g.n(len(gPlain))])
		case 1:
			g.w(g.ident())
			g.w("\n(")
			g.expr()
			g.w(")")
		case 2:
			g.w(g.ident())
			g.w("\n[")
			g.expr()
			g.w("]")
		case 3:
			g.w(g.ident())
			g.w("\n/")
			g.w(gPlain[g.n(len(gPlain))])
			g.w("/g")
		case 4:
			g.w("let\n")
			g.w(gPlain[g.n(len(gPlain))])
			g.w("=1")
		case 5:
			g.w(g.ident())
			g.w("\n`t`")
		case 6:
			g.w("x=y\n")
			g.w("/re/.test(s)")
		case 7:
			g.w("a\n?.b")
		case 8:
			g.w("async\n")
			g.w("function f(){}")
		case 9:
			if g.p(10) {
				g.w("x\n=>1")
			} else {
				g.w("x=>\n1")
			}
		}
		g.semi()
	case 23:
		g.w("{")
		g.w(g.ident())
		g.w("}")
	case 24:
		g.w("var ")
		g.w(g.ident())
		g.semi()
	case 25:
		if !decl {
			g.exprStmt()
			return
		}
		g.w("let ")
		g.w(gPlain[g.n(len(gPlain))])
		g.semi()
	case 26:
		if !decl {
			g.exprStmt()
			return
		}
		g.w("async function ")
		g.w(g.ident())
		g.funcRest(true, g.p(30))
	case 27:
		if g.inAsync {
			g.w("for await(")
			g.w([]string{"const ", "let ", "var ", ""}[g.n(4)])
			g.binding()
			g.w(" of ")
			g.assignExpr()
			g.w(")")
			g.loopBody()
		} else {
			g.exprStmt()
		}
	default:
		g.exprStmt()
	}
}

func (g *gen) loopBody() {
	old := g.inLoop
	g.inLoop = true
	g.sp()
	g.stmt(false)
	g.inLoop = old
}

func (g *gen) forStmt() {
	g.w("for")
	g.sp()
	g.w("(")
	switch g.n(8) {
	case 0:
		g.w(";;")
	case 1:
		kw := []string{"var ", "let ", "const "}[g.n(3)]
		g.w(kw)
		g.w(gPlain[g.n(len(gPlain))])
		g.w("=")
		g.assignExpr()
		if g.p(40) {
			g.w(",")
			g.w(gPlain[g.n(len(gPlain))])
			g.w("=0")
		}
		g.w(";")
		g.expr()
		g.w(";")
		g.expr()
	case 2, 3:
		g.w([]string{"var ", "let ", "const ", ""}[g.n(4)])
		g.binding()
		g.w([]string{" in ", " of "}[g.n(2)])
		g.assignExpr()
	case 4:
		g.w(g.ident())
		g.w(" in ")
		g.expr()
	case 5:
		g.expr()
		g.w(";")
		if g.p(50) {
			g.expr()
		}
		g.w(";")
		if g.p(50) {
			g.expr()
		}
	case 6:
		g.w("[")
		g.w(g.ident())
		g.w(",")
		g.w(g.ident())
		g.w("]of ")
		g.expr()
	case 7:
		g.w("let ")
		g.w(g.ident())
		g.w(";;")
	}
	g.w(")")
	g.loopBody()
}

func (g *gen) varList(isConst bool) {
	n := 1 + g.n(3)
	for i := 0; i < n; i++ {
		if i != 0 {
			g.w(",")
			g.sp()
		}
		pat := g.p(30)
		if pat {
			g.pattern()
			g.w("=")
			g.assignExpr()
		} else {
			g.w(g.ident())
			if isConst || g.p(60) {
				g.sp()
				g.w("=")
				g.sp()
				g.assignExpr()
			}
		}
	}
}

func (g *gen) binding() {
	if g.p(60) {
		g.w(g.ident())
	} else {
		g.pattern()
	}
}

func (g *gen) bindingElem() {
	g.binding()
	if g.p(30) {
		g.w("=")
		g.assignExpr()
	}
}

func (g *gen) pattern() {
	g.depth++
	defer func() { g.depth-- }()
	if 6 < g.depth {
		g.w(g.ident())
		return
	}
	if g.p(50) {
		g.w("[")
		n := g.n(4)
		for i := 0; i < n; i++ {
			if i != 0 {
				g.w(",")
			}
			if g.p(15) {
				continue // hole
			}
			g.bindingElem()
		}
		if g.p(25) {
			if 0 < n {
				g.w(",")
			}
			g.w("...")
			g.binding()
		}
		g.w("]")
	} else {
		g.w("{")
		n := g.n(4)
		for i := 0; i < n; i++ {
			if i != 0 {
				g.w(",")
			}
			switch g.n(5) {
			case 0:
				g.w(gPlain[g.n(len(gPlain))])
				if g.p(40) {
					g.w("=")
					g.assignExpr()
				}
			case 1:
				g.w("[")
				g.assignExpr()
				g.w("]:")
				g.bindingElem()
			case 2:
				g.w("'k':")
				g.bindingElem()
			case 3:
				g.w("1:")
				g.bindingElem()
			default:
				g.w(g.ident())
				g.w(":")
				g.bindingElem()
			}
		}
		if g.p(20) {
			if 0 < n {
				g.w(",")
			}
			g.w("...")
			g.w(g.ident())
		}
		g.w("}")
	}
}

func (g *gen) params() {
	g.w("(")
	n := g.n(4)
	for i := 0; i < n; i++ {
		if i != 0 {
			g.w(",")
		}
		g.bindingElem()
	}
	if g.p(20) {
		if 0 < n {
			g.w(",")
		}
		g.w("...")
		g.binding()
	}
	g.w(")")
}

func (g *gen) funcBody(async, gen bool) {
	of, oa, og, ol, os := g.inFunc, g.inAsync, g.inGen, g.inLoop, g.inSwitch
	g.inFunc, g.inAsync, g.inGen, g.inLoop, g.inSwitch = true, async, gen, false, false
	g.w("{")
	if g.p(10) {
		g.w("\"use strict\"")
		g.semi()
	}
	n := g.n(4)
	for i := 0; i < n; i++ {
		g.sp()
		g.stmt(true)
	}
	g.w("}")
	g.inFunc, g.inAsync, g.inGen, g.inLoop, g.inSwitch = of, oa, og, ol, os
}

func (g *gen) funcRest(async, gen bool) {
	oa, og := g.inAsync, g.inGen
	g.inAsync, g.inGen = async, gen
	g.params()
	g.inAsync, g.inGen = oa, og
	g.funcBody(async, gen)
}

func (g *gen) funcDecl() {
	async := g.p(20)
	gen := g.p(20)
	if async {
		g.w("async ")
	}
	g.w("function")
	if gen {
		g.w("*")
	}
	g.ws()
	g.w(g.ident())
	g.funcRest(async, gen)
}

func (g *gen) propName() {
	switch g.n(8) {
	case 0:
		g.w("[")
		g.assignExpr()
		g.w("]")
	case 1:
		g.w("'s'")
	case 2:
		g.w("0x1")
	case 3:
		g.w([]string{"if", "class", "new", "get", "set", "static", "async", "constructor"}[g.n(8)])
	default:
		g.w(g.ident())
	}
}

func (g *gen) classRest() {
	if g.p(30) {
		g.w(" extends ")
		g.lhsExpr()
	}
	g.w("{")
	n := g.n(5)
	oc := g.inClass
	g.inClass = true
	for i := 0; i < n; i++ {
		g.sp()
		if g.p(20) {
			g.w("static ")
		}
		switch g.n(9) {
		case 0:
			g.w("static{")
			of := g.inFunc
			g.inFunc = false
			g.stmt(true)
			g.inFunc = of
			g.w("}")
		case 1:
			g.w("#")
			g.w(gPlain[g.n(len(gPlain))])
			if g.p(50) {
				g.w("=")
				g.assignExpr()
			}
			g.semi()
		case 2:
			g.propName()
			if g.p(50) {
				g.w("=")
				g.assignExpr()
			}
			g.semi()
		case 3:
			g.w("get ")
			g.propName()
			g.w("()")
			g.funcBody(false, false)
		case 4:
			g.w("set ")
			g.propName()
			g.w("(v)")
			g.funcBody(false, false)
		case 5:
			g.w("async ")
			if g.p(30) {
				g.w("*")
			}
			g.propName()
			g.funcRest(true, false)
		case 6:
			g.w("*")
			g.propName()
			g.funcRest(false, true)
		default:
			g.propName()
			g.funcRest(false, false)
		}
	}
	g.inClass = oc
	g.w("}")
}

func (g *gen) exprStmt() {
	switch g.n(8) {
	case 0:
		// object pattern assignment must be parenthesized
		g.w("({")
		g.w(gPlain[g.n(len(gPlain))])
		g.w("=1,")
		g.w(g.ident())
		g.w("}=")
		g.assignExpr()
		g.w(")")
	case 1:
		g.w("[")
		g.w(g.ident())
		g.w(",,")
		g.w("...")
		g.w(g.ident())
		g.w("]=")
		g.assignExpr()
	default:
		start := g.buf.Len()
		g.expr()
		e := g.buf.String()[start:]
		for _, pre := range []string{"function", "class", "{", "async function", "async\nfunction", "let"} {
			if strings.HasPrefix(e, pre) && g.p(90) {
				g.buf.Truncate(start)
				g.w("(")
				g.w(e)
				g.w(")")
				break
			}
		}
	}
	g.semi()
}

func (g *gen) expr() {
	g.assignExpr()
	if g.p(10) {
		g.w(",")
		g.sp()
		g.assignExpr()
	}
}

var assignOps = []string{"=", "+=", "-=", "*=", "/=", "%=", "**=", "<<=", ">>=", ">>>=", "&=", "|=", "^=", "&&=", "||=", "??="}
var binOps = []string{"+", "-", "*", "/", "%", "**", "<<", ">>", ">>>", "<", ">", "<=", ">=", "==", "!=", "===", "!==", "&", "|", "^", "&&", "||", "??", " in ", " instanceof "}

func (g *gen) assignExpr() {
	g.depth++
	defer func() { g.depth-- }()
	if 7 < g.depth {
		g.primary()
		return
	}
	switch g.n(16) {
	case 0, 1:
		g.lhsExpr()
		g.sp()
		g.w(assignOps[g.n(len(assignOps))])
		g.sp()
		g.assignExpr()
	case 2, 3:
		g.arrow()
	case 4:
		g.condExpr()
		g.w("?")
		g.assignExpr()
		g.w(":")
		g.assignExpr()
	case 5:
		if g.inGen {
			g.w("yield")
			if g.p(30) {
				g.w("*")
			}
			if g.p(70) {
				g.ws()
				g.assignExpr()
			}
		} else {
			g.condExpr()
		}
	case 6:
		// destructuring assignment
		if g.p(50) {
			g.w("[")
			g.w(g.ident())
			g.w("=1,{")
			g.w(gPlain[g.n(len(gPlain))])
			g.w("}]=")
		} else {
			g.w("({")
			g.w(gPlain[g.n(len(gPlain))])
			g.w(":")
			g.lhsExpr()
			g.w("}=")
			g.assignExpr()
			g.w(")")
			return
		}
		g.assignExpr()
	default:
		g.condExpr()
	}
}

func (g *gen) condExpr() {
	g.binExpr(0)
}

func (g *gen) binExpr(lvl int) {
	g.unary()
	for lvl < 3 && g.p(30) {
		g.sp()
		g.w(binOps[g.n(len(binOps))])
		g.sp()
		lvl++
		g.unary()
	}
}

func (g *gen) unary() {
	switch g.n(14) {
	case 0:
		g.w([]string{"!", "~", "-", "+", "typeof ", "void ", "delete ", "++", "--"}[g.n(9)])
		g.unary()
	case 1:
		if g.inAsync {
			g.w("await ")
			g.unary()
		} else {
			g.postfix()
		}
	default:
		g.postfix()
	}
}

func (g *gen) postfix() {
	g.lhsExpr()
	if g.p(10) {
		g.w([]string{"++", "--"}[g.n(2)])
	}
}

func (g *gen) args() {
	g.w("(")
	n := g.n(3)
	for i := 0; i < n; i++ {
		if i != 0 {
			g.w(",")
		}
		if g.p(15) {
			g.w("...")
		}
		g.assignExpr()
	}
	g.w(")")
}

func (g *gen) lhsExpr() {
	g.depth++
	defer func() { g.depth-- }()
	if g.p(10) && g.depth < 8 {
		g.w("new ")
		g.memberBase()
		if g.p(70) {
			g.args()
		}
	} else {
		g.memberBase()
	}
	n := g.n(3)
	for i := 0; i < n && g.depth < 9; i++ {
		switch g.n(9) {
		case 0, 1:
			g.w(".")
			g.w([]string{"a", "b", "if", "new", "#p", "class", "x"}[g.n(7)])
		case 2:
			g.w("[")
			g.expr()
			g.w("]")
		case 3, 4:
			g.args()
		case 5:
			g.w("?.")
			switch g.n(3) {
			case 0:
				g.w("b")
			case 1:
				g.w("[")
				g.expr()
				g.w("]")
			default:
				g.args()
			}
		case 6:
			g.template()
		default:
			g.w(".x")
		}
	}
}

func (g *gen) memberBase() {
	g.primary()
}

func (g *gen) template() {
	g.w("`")
	n := g.n(3)
	for i := 0; i < n; i++ {
		g.w([]string{"s", "\\`", "$", "{", "\\${", "\n", ""}[g.n(7)])
		g.w("${")
		g.expr()
		g.w("}")
	}
	g.w("t`")
}

func (g *gen) primary() {
	g.depth++
	defer func() { g.depth-- }()
	if 7 < g.depth {
		g.w(gPlain[g.n(len(gPlain))])
		return
	}
	switch g.n(30) {
	case 0, 1, 2, 3, 4, 5:
		g.w(g.ident())
	case 6:
		g.w([]string{"1", "0", ".5", "1e3", "0x1F", "0b10", "0o7", "1_0", "1n", "0.5e-1", "1.", "0"}[g.n(12)])
	case 7:
		g.w([]string{"'s'", "\"d\"", "'\\''", "\"\\\n\"", "'\\u0041'"}[g.n(5)])
	case 8:
		g.template()
	case 9:
		g.w([]string{"/re/", "/[/]/g", "/a\\/b/i", "/=/", "/re/u"}[g.n(5)])
	case 10:
		g.w([]string{"this", "null", "true", "false", "super.x", "new.target", "import.meta", "import('m')"}[g.n(8)])
	case 11:
		g.w("[")
		n := g.n(4)
		for i := 0; i < n; i++ {
			if i != 0 {
				g.w(",")
			}
			if g.p(10) {
				continue
			}
			if g.p(15) {
				g.w("...")
			}
			g.assignExpr()
		}
		g.w("]")
	case 12, 13:
		g.object()
	case 14:
		g.w("(")
		g.expr()
		g.w(")")
	case 15:
		g.w("function")
		if g.p(20) {
			g.w("*")
		}
		if g.p(40) {
			g.w(" ")
			g.w(g.ident())
		}
		g.funcRest(false, false)
	case 16:
		g.w("class")
		if g.p(40) {
			g.w(" ")
			g.w(gPlain[g.n(len(gPlain))])
		}
		g.classRest()
	case 17, 18:
		g.w("(")
		g.arrow()
		g.w(")")
	case 19:
		g.w("async function")
		if g.p(40) {
			g.w(" ")
			g.w(g.ident())
		}
		g.funcRest(true, false)
	case 20:
		g.w("(")
		g.arrow()
		g.w(")")
	case 21:
		g.w("async(")
		g.w(g.ident())
		g.w(")")
	default:
		g.w(gPlain[g.n(len(gPlain))])
	}
}

func (g *gen) object() {
	g.w("{")
	n := g.n(4)
	for i := 0; i < n; i++ {
		if i != 0 {
			g.w(",")
		}
		switch g.n(9) {
		case 0:
			g.w(g.ident())
		case 1:
			g.w("...")
			g.assignExpr()
		case 2:
			g.propName()
			g.w("(")
			g.w(g.ident())
			g.w(")")
			g.funcBody(false, false)
		case 3:
			g.w("get ")
			g.propName()
			g.w("()")
			g.funcBody(false, false)
		case 4:
			g.w("set ")
			g.propName()
			g.w("(v)")
			g.funcBody(false, false)
		case 5:
			g.w("async*")
			g.propName()
			g.funcRest(true, true)
		default:
			g.propName()
			g.w(":")
			g.assignExpr()
		}
	}
	if g.p(15) {
		g.w(",")
	}
	g.w("}")
}

func (g *gen) arrow() {
	async := g.p(20)
	if async {
		g.w("async ")
	}
	if !async && g.p(30) {
		g.w(g.ident())
	} else if async && g.p(30) {
		g.w(gPlain[g.n(len(gPlain))])
	} else {
		oa := g.inAsync
		g.inAsync = async
		g.params()
		g.inAsync = oa
	}
	if g.p(5) {
		g.w("/*c*/")
	}
	g.w("=>")
	g.sp()
	if g.p(50) {
		g.funcBody(async, false)
	} else {
		oa, og := g.inAsync, g.inGen
		g.inAsync, g.inGen = async, false
		if g.p(15) {
			g.w("(")
			g.object()
			g.w(")")
		} else {
			g.assignExpr()
		}
		g.inAsync, g.inGen = oa, og
	}
}

func genProgram(rnd *rand.Rand) []byte {
	g := &gen{rnd: rnd}
	g.program(true)
	return g.buf.Bytes()
}

// genGrammar writes OUT: n generated programs (a fifth of them mutated with
// 1-2 operations), with the input and the digests (or, with full, the full
// dumps) of every mode.
func genGrammar(out string, n int, seed int64, full bool) {
	if err := os.MkdirAll(filepath.Dir(out), 0o755); err != nil {
		panic(err)
	}
	lits := testLiterals()
	rnd := rand.New(rand.NewSource(seed))
	w := newRecWriter(out)
	for k := 0; k < n; k++ {
		src := genProgram(rnd)
		if rnd.Intn(5) == 0 {
			src = mutateOps(rnd, src, lits, 1+rnd.Intn(2))
		}
		fields := [][]byte{src}
		for _, m := range modes {
			if full {
				fields = append(fields, runMode(m, src))
			} else {
				fields = append(fields, digest(runMode(m, src)))
			}
		}
		w.rec(fields...)
	}
	w.close()
	fmt.Fprintf(os.Stderr, "grammar: %d programs\n", n)
}

// genGrammarSample prints n generated programs (for eyeballing the
// generator).
func genGrammarSample(n int, seed int64) {
	rnd := rand.New(rand.NewSource(seed))
	for k := 0; k < n; k++ {
		src := genProgram(rnd)
		ok := "OK "
		if _, err := parseOK(src); err != nil {
			ok = "ERR " + strings.ReplaceAll(err.Error(), "\n", " | ") + " "
		}
		fmt.Printf("%s%q\n", ok, src)
	}
}
