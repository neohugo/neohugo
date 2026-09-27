package main

import (
	"bytes"
	"fmt"
	"math/rand"
	"strings"

	"github.com/tdewolff/parse/v2"
	pjs "github.com/tdewolff/parse/v2/js"
)

// Grammar-based generator of (mostly valid) JavaScript programs, adapted
// from tools/go-oracle/tdewolff-parse-js/grammar.go for the minifier: more
// function scopes (renaming), var/let/const declarations and assignments
// (hoisting and var merging), if/else with return/throw/break (statement
// merging), and the expressions the minifier rewrites (undefined, Infinity,
// ==null, typeof, isNaN, Number, Math.*, string concatenation and escapes,
// numbers, regexps, templates, optional chains).

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
		} else if g.p(35) {
			// a function scope: the renamer and hoistVars only work there
			switch g.n(4) {
			case 0:
				g.w("!function")
				g.funcRest(false, false)
				g.w("()")
			case 1:
				g.w("(function(")
				g.w("a,b,c){")
				g.funcStmts()
				g.w("})(x,y,z)")
			case 2:
				g.w("f=(a,b)=>{")
				g.funcStmts()
				g.w("}")
			default:
				g.w("function f")
				g.funcRest(g.p(10), false)
			}
			g.semi()
		} else {
			g.stmt(true)
		}
		g.sp()
	}
}

// funcStmts writes a function body's statements (without braces) that
// declare and use many locals.
func (g *gen) funcStmts() {
	of, oa, og, ol, os := g.inFunc, g.inAsync, g.inGen, g.inLoop, g.inSwitch
	g.inFunc, g.inAsync, g.inGen, g.inLoop, g.inSwitch = true, false, false, false, false
	n := 1 + g.n(6)
	for i := 0; i < n; i++ {
		g.sp()
		g.stmt(true)
	}
	g.inFunc, g.inAsync, g.inGen, g.inLoop, g.inSwitch = of, oa, og, ol, os
}

// minStmt writes a statement shaped for the minifier's statement-level
// optimizations.
func (g *gen) minStmt() {
	v := gPlain[g.n(len(gPlain))]
	switch g.n(14) {
	case 0:
		g.w("var ")
		g.w(v)
		g.semi()
		g.w(v)
		g.w("=")
		g.assignExpr()
		g.semi()
	case 1:
		g.w("if(")
		g.expr()
		g.w(")")
		if g.inFunc {
			g.w("return ")
		} else {
			g.w("throw ")
		}
		g.assignExpr()
		g.semi()
		if g.p(50) {
			g.w("else ")
			g.stmt(false)
		}
		if g.inFunc && g.p(60) {
			g.w("return")
			if g.p(70) {
				g.w(" ")
				g.assignExpr()
			}
			g.semi()
		}
	case 2:
		g.w("if(!")
		g.primary()
		g.w("){")
		g.exprStmt()
		g.w("}else{")
		g.exprStmt()
		g.w("}")
	case 3:
		g.w("if(")
		g.expr()
		g.w("){")
		g.stmt(true)
		g.w("}")
		if g.p(60) {
			g.w("else if(")
			g.expr()
			g.w(")")
			g.stmt(false)
		}
	case 4:
		kw := []string{"var ", "let ", "const "}[g.n(3)]
		g.w(kw)
		g.w(v)
		g.w("=")
		g.assignExpr()
		g.w(";")
		g.w(kw)
		g.w(gPlain[g.n(len(gPlain))])
		g.w("=")
		g.assignExpr()
		g.semi()
	case 5:
		g.exprStmt()
		g.exprStmt()
		if g.inFunc {
			g.w("return ")
			g.expr()
			g.semi()
		}
	case 6:
		g.w("var ")
		g.w(v)
		g.w(";for(")
		g.w(v)
		g.w("=0;")
		g.w(v)
		g.w("<10;")
		g.w(v)
		g.w("++)")
		g.loopBody()
	case 7:
		g.w("{let ")
		g.w(v)
		g.w("=")
		g.assignExpr()
		g.w(";")
		g.exprStmt()
		g.w("}")
	case 8:
		g.w("while(")
		g.expr()
		g.w("){")
		old := g.inLoop
		g.inLoop = true
		g.stmt(true)
		g.w("if(")
		g.expr()
		g.w(")continue;")
		g.inLoop = old
		g.w("}")
	case 9:
		g.w("if(")
		g.expr()
		g.w("){")
		g.stmt(true)
		g.w("}else{")
		g.stmt(true)
		g.w("}")
	case 10:
		g.w("try{")
		g.stmt(true)
		g.w("}catch(")
		g.w(v)
		g.w("){")
		if g.p(50) {
			g.w(v)
		}
		g.w("}")
	case 11:
		g.w("switch(")
		g.expr()
		g.w("){case 1:")
		g.exprStmt()
		g.exprStmt()
		g.w("default:")
		g.stmt(true)
		g.w("}")
	case 12:
		g.w("for(var ")
		g.w(v)
		g.w(" in ")
		g.primary()
		g.w(")")
		g.loopBody()
	default:
		g.w(v)
		g.w("=")
		g.minExpr()
		g.semi()
	}
}

// minExpr writes one of the expressions the printer rewrites.
func (g *gen) minExpr() {
	v := gPlain[g.n(len(gPlain))]
	switch g.n(24) {
	case 0:
		g.w(v)
		g.w([]string{"==null", "!=null", "===null", "===undefined", "!==void 0", "==void 0"}[g.n(6)])
		g.w([]string{"?", "&&", "||"}[g.n(3)])
		g.assignExpr()
		g.w(":")
		g.w(v)
	case 1:
		g.w(v)
		g.w("===null||")
		g.w(v)
		g.w("===void 0?void 0:")
		g.w(v)
		g.w([]string{".b", "[b]", "(b)", ".b.c()", "`t`"}[g.n(5)])
	case 2:
		g.w("typeof ")
		g.w(v)
		g.w([]string{"===", "!==", "=="}[g.n(3)])
		g.w([]string{"'undefined'", "\"object\"", "`function`", "x"}[g.n(4)])
	case 3:
		g.w([]string{"isNaN(", "Number(", "Math.abs(", "Math.trunc(", "Math.pow(2,", "Math.pow("}[g.n(6)])
		if g.p(50) {
			g.w(v)
		} else {
			g.assignExpr()
		}
		g.w(")")
	case 4:
		g.w([]string{"undefined", "Infinity", "NaN", "void 0", "void a()", "-Infinity", "!0", "!1", "!!a", "!''", "!'s'", "!0n", "!1.0", "!/r/"}[g.n(14)])
	case 5:
		g.w(strLits[g.n(len(strLits))])
		for g.p(60) {
			g.w("+")
			if g.p(30) {
				g.w(v)
				g.w("+")
			}
			g.w(strLits[g.n(len(strLits))])
		}
	case 6:
		g.w(numLits[g.n(len(numLits))])
	case 7:
		g.w(reLits[g.n(len(reLits))])
	case 8:
		g.w("`")
		g.w(tplParts[g.n(len(tplParts))])
		if g.p(50) {
			g.w("${")
			g.expr()
			g.w("}")
			g.w(tplParts[g.n(len(tplParts))])
		}
		g.w("`")
	case 9:
		g.w("!(")
		g.expr()
		g.w([]string{"&&", "||", "==", "!=", "===", "<"}[g.n(6)])
		g.expr()
		g.w(")")
	case 10:
		g.w(v)
		g.w("?")
		g.w([]string{"!0", "!1", "true", "false", "b(1)", "c"}[g.n(6)])
		g.w(":")
		g.w([]string{"!0", "!1", "true", "false", "b(2)", "c", v}[g.n(7)])
	case 11:
		g.w("(")
		g.expr()
		g.w(",")
		g.expr()
		g.w(")")
		g.w([]string{"?b:c", "&&d", "||e"}[g.n(3)])
	case 12:
		g.w(v)
		g.w("?.")
		g.w([]string{"b", "[0]", "(1)", "b.c"}[g.n(4)])
	case 13:
		g.w("(")
		g.w(numLits[g.n(len(numLits))])
		g.w(").toString()")
	case 14:
		g.w(v)
		g.w("[")
		g.w([]string{"'a'", "\"b c\"", "'1'", "'01'", "'1.5'", "\"if\"", "'$_'", "'\\u0061'"}[g.n(8)])
		g.w("]")
	case 15:
		g.w(v)
		g.w([]string{"+ +", "- -", "+ ++", "- --", "/ /x/", "< !--", "-->"}[g.n(7)])
		g.w(gPlain[g.n(len(gPlain))])
	default:
		g.assignExpr()
	}
}

var strLits = []string{
	"'s'", "\"d\"", "''", "\"\"", "'\\''", "\"\\\"\"", "'\"'", "\"'\"", "'`'", "'${'", "\"\\n\"", "'\\r'",
	"'\\x41\\x0a'", "'\\u0041'", "'\\u{1F600}'", "'\\101'", "'\\0'", "'\\00'", "'\\x00'", "'\\u0000'",
	"'\\t\\v\\f\\b'", "'\\a\\c'", "'\\\n'", "'</script>'", "'<\\/script>'", "'\\u2028'", "'é'", "'\\é'",
	"'a\"b\\'c`d${e'", "'\\u{a}\\u{d}'", "'\\12\\15'", "'\\8\\9'",
}

var numLits = []string{
	"0", "1", "10", "100", "1000", "1e3", "1e21", "1.0", "1.50", ".5", "0.5", "5.", "1_000", "0x0", "0xFF",
	"0xDEADBEEF", "0xe000000000", "0b0", "0b1010", "0o0", "0o777", "1n", "0x1Fn", "0b11n", "0o7n", "123456789012345678901234",
	"1e-7", "0.0000001", "1.2e+3", "00", "07", "08", "0.1e1", "9007199254740993",
}

var reLits = []string{
	"/a/", "/[a-z]/gi", "/\\d+/", "/\\//", "/[/]/", "/\\a\\b\\c/", "/[\\a\\b\\-\\]]/", "/[\\^a]/", "/[a\\-z]/",
	"/\\$\\(\\)/", "/<\\/script>/", "/\\u0041/u", "/(?:a)/y",
}

var tplParts = []string{
	"", "s", "\\`", "$", "{", "\\${", "\n", "\\n", "\\x41", "\\u{42}", "\\0", "\\\n", "a\\'b", "\\r",
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
	if g.p(30) {
		g.minStmt()
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
	if g.p(15) {
		g.w("(")
		g.minExpr()
		g.w(")")
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
	if 0 < n && g.p(15) {
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

// genProgram returns a generated program. Long random programs are often
// invalid somewhere, and an early syntax error hides everything after it
// from the minifier, so for 85% of the programs up to 20 attempts are made
// to get one that parses.
func genProgram(rnd *rand.Rand) []byte {
	wantValid := rnd.Intn(100) < 85
	var src []byte
	for attempt := 0; attempt < 20; attempt++ {
		g := &gen{rnd: rnd}
		g.program(true)
		src = g.buf.Bytes()
		if !wantValid {
			break
		}
		if _, err := pjs.Parse(parse.NewInputBytes(cp(src)), pjs.Options{WhileToFor: true}); err == nil {
			break
		}
	}
	return src
}

// genSample prints n generated programs and their minified output (for
// eyeballing the generator).
func genSample(n int, seed int64) {
	rnd := rand.New(rand.NewSource(seed))
	for k := 0; k < n; k++ {
		src := genProgram(rnd)
		out, err := minifyBuf("v2022", cp(src))
		if err != nil {
			fmt.Printf("ERR %s\n  %q\n", strings.ReplaceAll(err.Error(), "\n", " | "), src)
		} else {
			fmt.Printf("%q\n  %q\n", src, out)
		}
	}
}
