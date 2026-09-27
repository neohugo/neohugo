package main

import (
	"math/rand"
	"strings"
)

// The `logic` red-team kind: expression trees over the operators the
// minifier rewrites (optimizeCondExpr, optimizeUnaryExpr,
// isUndefinedOrNullVar, toNullishExpr, isTruthy/isFalsy, typeof
// comparisons, comma/conditional merging) in statement contexts that
// trigger the if/return/throw merging of optimizeStmtList.

var logicAtoms = []string{
	"a", "b", "c", "a.b", "a[b]", "a()", "a?.b", "!0", "!1", "true", "false", "null", "undefined", "void 0", "void a",
	"0", "1", "-1", "0n", "1n", "\"\"", "\"s\"", "''", "``", "`t`", "[]", "{}", "NaN", "Infinity", "-Infinity", "this",
	"typeof a", "typeof a.b", "\"undefined\"", "\"object\"", "/r/", "(a,b)", "a=b", "a+=1", "++a", "a--", "x=>x",
	"function(){}", "class{}", "new A", "!a", "!!a", "-a", "+a", "~a", "delete a.b", "await a", "(0,a.b)()",
}

var logicBinOps = []string{"&&", "||", "??", "==", "!=", "===", "!==", ",", "&&", "||", "<", "+", "=", " in ", " instanceof ", "|", "&", "**"}

func genLogic(rnd *rand.Rand, depth int) string {
	if depth <= 0 || rnd.Intn(4) == 0 {
		return logicAtoms[rnd.Intn(len(logicAtoms))]
	}
	switch rnd.Intn(12) {
	case 0, 1, 2:
		return genLogic(rnd, depth-1) + "?" + genLogic(rnd, depth-1) + ":" + genLogic(rnd, depth-1)
	case 3:
		return "!" + wrapParen(rnd, genLogic(rnd, depth-1))
	case 4:
		return "(" + genLogic(rnd, depth-1) + ")"
	case 5:
		// undefined/null checks on one variable
		v := []string{"a", "b", "a.b", "a[0]", "(a=b)"}[rnd.Intn(5)]
		n1 := []string{"null", "undefined", "void 0"}[rnd.Intn(3)]
		n2 := []string{"null", "undefined", "void 0"}[rnd.Intn(3)]
		op1 := []string{"===", "==", "!==", "!="}[rnd.Intn(4)]
		op2 := []string{"===", "==", "!==", "!="}[rnd.Intn(4)]
		l := v + op1 + n1
		if rnd.Intn(2) == 0 {
			l = n1 + op1 + v
		}
		r := v + op2 + n2
		if rnd.Intn(2) == 0 {
			r = n2 + op2 + v
		}
		e := l + []string{"||", "&&"}[rnd.Intn(2)] + r
		if rnd.Intn(2) == 0 {
			e += "?" + []string{"void 0", v + ".c", v + "[0]", v + "()", v, "b", "undefined"}[rnd.Intn(7)] + ":" + []string{"void 0", v + ".c", v, "b", "undefined"}[rnd.Intn(5)]
		}
		return e
	case 6:
		// typeof comparisons
		return "typeof " + []string{"a", "a.b", "a()"}[rnd.Intn(3)] + []string{"==", "===", "!=", "!=="}[rnd.Intn(4)] + []string{"\"undefined\"", "'object'", "\"function\"", "a", "`undefined`"}[rnd.Intn(5)]
	}
	return wrapParen(rnd, genLogic(rnd, depth-1)) + logicBinOps[rnd.Intn(len(logicBinOps))] + wrapParen(rnd, genLogic(rnd, depth-1))
}

func wrapParen(rnd *rand.Rand, s string) string {
	if rnd.Intn(3) == 0 {
		return "(" + s + ")"
	}
	return s
}

func genLogicStmt(rnd *rand.Rand, depth int) string {
	E := func() string { return genLogic(rnd, 1+rnd.Intn(3)) }
	if depth <= 0 {
		return "x=" + E()
	}
	switch rnd.Intn(16) {
	case 0, 1:
		return "x=" + E()
	case 2:
		return "if(" + E() + ")" + genLogicStmt(rnd, depth-1)
	case 3:
		return "if(" + E() + ")" + genLogicStmt(rnd, depth-1) + ";else " + genLogicStmt(rnd, depth-1)
	case 4:
		return "return " + E()
	case 5:
		return "throw " + E()
	case 6:
		return E()
	case 7:
		return "if(" + E() + ")return " + E()
	case 8:
		return "if(" + E() + ")throw " + E()
	case 9:
		return "if(" + E() + "){" + genLogicStmt(rnd, depth-1) + ";return}"
	case 10:
		return "while(" + E() + ")" + genLogicStmt(rnd, depth-1)
	case 11:
		return "for(;" + E() + ";" + E() + ")" + genLogicStmt(rnd, depth-1)
	case 12:
		return "var " + []string{"a", "b", "c", "d"}[rnd.Intn(4)] + "=" + E()
	case 13:
		return "f(" + E() + "," + E() + ")"
	case 14:
		return "{" + genLogicStmt(rnd, depth-1) + ";" + genLogicStmt(rnd, depth-1) + "}"
	}
	return "return"
}

func genLogicProgram(rnd *rand.Rand) string {
	var b strings.Builder
	fn := rnd.Intn(3) != 0
	if fn {
		b.WriteString([]string{"function f(a,b){", "x=(a,b)=>{", "!function(){var a,b,c;", "async function f(a){"}[rnd.Intn(4)])
	}
	n := 1 + rnd.Intn(5)
	for i := 0; i < n; i++ {
		s := genLogicStmt(rnd, 2)
		if !fn && strings.Contains(s, "return") {
			s = strings.ReplaceAll(s, "return", "x=0,")
		}
		b.WriteString(s)
		b.WriteString([]string{";", "\n"}[rnd.Intn(2)])
	}
	if fn {
		b.WriteString("}")
		if rnd.Intn(3) == 0 {
			b.WriteString("()")
		}
	}
	return b.String()
}
