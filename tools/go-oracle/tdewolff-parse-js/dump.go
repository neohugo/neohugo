package main

import (
	"bytes"
	"fmt"
	"io"
	"strconv"

	"github.com/tdewolff/parse/v2"
	"github.com/tdewolff/parse/v2/js"
)

// The serializations below are mirrored byte for byte by
// crates/tdewolff-parse-js/src/dump.rs.

func esc(buf *bytes.Buffer, b []byte) {
	const hexd = "0123456789abcdef"
	for _, c := range b {
		if 0x20 <= c && c < 0x7f && c != '"' && c != '\\' {
			buf.WriteByte(c)
		} else {
			buf.WriteString(`\x`)
			buf.WriteByte(hexd[c>>4])
			buf.WriteByte(hexd[c&15])
		}
	}
}

// dumpBytes writes a []byte field: nil, or "escaped"/cap.
func dumpBytes(buf *bytes.Buffer, b []byte) {
	if b == nil {
		buf.WriteString("nil")
		return
	}
	buf.WriteByte('"')
	esc(buf, b)
	buf.WriteString(`"/`)
	buf.WriteString(strconv.Itoa(cap(b)))
}

func errString(err error) string {
	if err == nil {
		return "nil"
	}
	if err == io.EOF {
		return "EOF"
	}
	return err.Error()
}

// lexDump serializes the plain token stream, continuing after lexer errors.
// With regexps set, a DivToken/DivEqToken in a position where a regular
// expression may start (by a simple heuristic on the previous significant
// token) is re-lexed with RegExp().
func lexDump(src []byte, regexps bool) (out []byte) {
	buf := &bytes.Buffer{}
	defer func() {
		if r := recover(); r != nil {
			// the lexer can run past the end of the input after an error
			buf.WriteString("PANIC\n")
			out = buf.Bytes()
		}
	}()
	l := js.NewLexer(parse.NewInputBytes(append([]byte(nil), src...)))
	prev := js.ErrorToken
	for n := 0; n < 1<<22; n++ {
		tt, data := l.Next()
		if regexps && (tt == js.DivToken || tt == js.DivEqToken) && regexAllowed(prev) {
			buf.WriteString("R ")
			tt, data = l.RegExp()
		}
		buf.WriteString(strconv.Itoa(int(tt)))
		buf.WriteByte(' ')
		dumpBytes(buf, data)
		buf.WriteByte('\n')
		if tt == js.ErrorToken {
			if l.Err() == io.EOF {
				buf.WriteString("EOF\n")
				return buf.Bytes()
			}
			buf.WriteString("ERR ")
			esc(buf, []byte(errString(l.Err())))
			buf.WriteByte('\n')
		}
		if tt != js.WhitespaceToken && tt != js.LineTerminatorToken && tt != js.CommentToken && tt != js.CommentLineTerminatorToken {
			prev = tt
		}
	}
	buf.WriteString("LIMIT\n")
	return buf.Bytes()
}

func regexAllowed(prev js.TokenType) bool {
	if js.IsIdentifier(prev) || js.IsNumeric(prev) {
		return false
	}
	switch prev {
	case js.StringToken, js.RegExpToken, js.TemplateToken, js.TemplateEndToken, js.PrivateIdentifierToken,
		js.CloseParenToken, js.CloseBracketToken, js.CloseBraceToken,
		js.ThisToken, js.SuperToken, js.NullToken, js.TrueToken, js.FalseToken,
		js.IncrToken, js.DecrToken:
		return false
	}
	return true
}

type dumper struct {
	buf *bytes.Buffer

	vars    map[*js.Var]int
	varList []*js.Var

	scopes     map[*js.Scope]int
	scopeList  []*js.Scope
	scopeAlias map[*js.Scope]*js.Scope

	decls       map[*js.VarDecl]int
	declList    []*js.VarDecl
	declPrinted map[*js.VarDecl]bool
}

func newDumper() *dumper {
	return &dumper{
		buf:         &bytes.Buffer{},
		vars:        map[*js.Var]int{},
		scopes:      map[*js.Scope]int{},
		scopeAlias:  map[*js.Scope]*js.Scope{},
		decls:       map[*js.VarDecl]int{},
		declPrinted: map[*js.VarDecl]bool{},
	}
}

func (d *dumper) s(str string) { d.buf.WriteString(str) }

func (d *dumper) b(b []byte) { dumpBytes(d.buf, b) }

func (d *dumper) i(n int) { d.buf.WriteString(strconv.Itoa(n)) }

func (d *dumper) bool(v bool) {
	if v {
		d.s("1")
	} else {
		d.s("0")
	}
}

func (d *dumper) v(v *js.Var) {
	if v == nil {
		d.s("nil")
		return
	}
	id, ok := d.vars[v]
	if !ok {
		d.varList = append(d.varList, v)
		id = len(d.varList)
		d.vars[v] = id
	}
	d.s("v")
	d.i(id)
}

func (d *dumper) sc(s *js.Scope) {
	if s == nil {
		d.s("nil")
		return
	}
	if a, ok := d.scopeAlias[s]; ok {
		s = a
	}
	id, ok := d.scopes[s]
	if !ok {
		d.scopeList = append(d.scopeList, s)
		id = len(d.scopeList)
		d.scopes[s] = id
	}
	d.s("s")
	d.i(id)
}

func (d *dumper) declRef(v *js.VarDecl) int {
	id, ok := d.decls[v]
	if !ok {
		d.declList = append(d.declList, v)
		id = len(d.declList)
		d.decls[v] = id
	}
	return id
}

func (d *dumper) lit(n js.LiteralExpr) {
	d.s("(Lit ")
	d.i(int(n.TokenType))
	d.s(" ")
	d.b(n.Data)
	d.s(")")
}

func (d *dumper) pn(n *js.PropertyName) {
	if n == nil {
		d.s("nil")
		return
	}
	d.s("(PN ")
	d.lit(n.Literal)
	d.s(" ")
	d.n(n.Computed)
	d.s(")")
}

func (d *dumper) be(n js.BindingElement) {
	d.s("(BE ")
	d.n(n.Binding)
	d.s(" ")
	d.n(n.Default)
	d.s(")")
}

func (d *dumper) bes(l []js.BindingElement) {
	d.s("[")
	for i, item := range l {
		if i != 0 {
			d.s(" ")
		}
		d.be(item)
	}
	d.s("]")
}

func (d *dumper) params(n js.Params) {
	d.s("(Params ")
	d.bes(n.List)
	d.s(" ")
	d.n(n.Rest)
	d.s(")")
}

func (d *dumper) stmts(l []js.IStmt) {
	d.s("[")
	for _, item := range l {
		d.s("\n")
		d.n(item)
	}
	d.s("]")
}

func (d *dumper) aliases(l []js.Alias) {
	d.s("[")
	for i, a := range l {
		if i != 0 {
			d.s(" ")
		}
		d.s("(Alias ")
		d.b(a.Name)
		d.s(" ")
		d.b(a.Binding)
		d.s(")")
	}
	d.s("]")
}

func (d *dumper) args(n js.Args) {
	d.s("(Args [")
	for i, a := range n.List {
		if i != 0 {
			d.s(" ")
		}
		d.s("(Arg ")
		d.n(a.Value)
		d.s(" ")
		d.bool(a.Rest)
		d.s(")")
	}
	d.s("])")
}

func (d *dumper) block(n *js.BlockStmt) {
	if n == nil {
		d.s("nil")
		return
	}
	d.s("(Block ")
	d.sc(&n.Scope)
	d.s(" ")
	d.stmts(n.List)
	d.s(")")
}

func (d *dumper) varDecl(n *js.VarDecl) {
	id := d.declRef(n)
	if d.declPrinted[n] {
		d.s("d")
		d.i(id)
		return
	}
	d.declPrinted[n] = true
	d.s("(VarDecl d")
	d.i(id)
	d.s(" ")
	d.i(int(n.TokenType))
	d.s(" ")
	d.sc(n.Scope)
	d.s(" ")
	d.bool(n.InFor)
	d.s(" ")
	d.bool(n.InForInOf)
	d.s(" ")
	d.bes(n.List)
	d.s(")")
}

func (d *dumper) n(node js.INode) {
	switch n := node.(type) {
	case nil:
		d.s("nil")
	case *js.Comment:
		d.s("(Comment ")
		d.b(n.Value)
		d.s(")")
	case *js.BlockStmt:
		d.block(n)
	case *js.EmptyStmt:
		d.s("(Empty)")
	case *js.ExprStmt:
		d.s("(Expr ")
		d.n(n.Value)
		d.s(")")
	case *js.IfStmt:
		d.s("(If ")
		d.n(n.Cond)
		d.s(" ")
		d.n(n.Body)
		d.s(" ")
		d.n(n.Else)
		d.s(")")
	case *js.DoWhileStmt:
		d.s("(DoWhile ")
		d.n(n.Cond)
		d.s(" ")
		d.n(n.Body)
		d.s(")")
	case *js.WhileStmt:
		d.s("(While ")
		d.n(n.Cond)
		d.s(" ")
		d.n(n.Body)
		d.s(")")
	case *js.ForStmt:
		d.s("(For ")
		d.n(n.Init)
		d.s(" ")
		d.n(n.Cond)
		d.s(" ")
		d.n(n.Post)
		d.s(" ")
		d.block(n.Body)
		d.s(")")
	case *js.ForInStmt:
		d.s("(ForIn ")
		d.n(n.Init)
		d.s(" ")
		d.n(n.Value)
		d.s(" ")
		d.block(n.Body)
		d.s(")")
	case *js.ForOfStmt:
		d.s("(ForOf ")
		d.bool(n.Await)
		d.s(" ")
		d.n(n.Init)
		d.s(" ")
		d.n(n.Value)
		d.s(" ")
		d.block(n.Body)
		d.s(")")
	case *js.SwitchStmt:
		d.s("(Switch ")
		d.sc(&n.Scope)
		d.s(" ")
		d.n(n.Init)
		d.s(" [")
		for i, c := range n.List {
			if i != 0 {
				d.s(" ")
			}
			d.s("(Case ")
			d.i(int(c.TokenType))
			d.s(" ")
			d.n(c.Cond)
			d.s(" ")
			d.stmts(c.List)
			d.s(")")
		}
		d.s("])")
	case *js.BranchStmt:
		d.s("(Branch ")
		d.i(int(n.Type))
		d.s(" ")
		d.b(n.Label)
		d.s(")")
	case *js.ReturnStmt:
		d.s("(Return ")
		d.n(n.Value)
		d.s(")")
	case *js.WithStmt:
		d.s("(With ")
		d.n(n.Cond)
		d.s(" ")
		d.n(n.Body)
		d.s(")")
	case *js.LabelledStmt:
		d.s("(Label ")
		d.b(n.Label)
		d.s(" ")
		d.n(n.Value)
		d.s(")")
	case *js.ThrowStmt:
		d.s("(Throw ")
		d.n(n.Value)
		d.s(")")
	case *js.TryStmt:
		d.s("(Try ")
		d.block(n.Body)
		d.s(" ")
		d.n(n.Binding)
		d.s(" ")
		d.block(n.Catch)
		d.s(" ")
		d.block(n.Finally)
		d.s(")")
	case *js.DebuggerStmt:
		d.s("(Debugger)")
	case *js.ImportStmt:
		d.s("(Import ")
		if n.List == nil {
			d.s("nil")
		} else {
			d.aliases(n.List)
		}
		d.s(" ")
		d.b(n.Default)
		d.s(" ")
		d.b(n.Module)
		d.s(")")
	case *js.ExportStmt:
		d.s("(Export ")
		d.aliases(n.List)
		d.s(" ")
		d.b(n.Module)
		d.s(" ")
		d.bool(n.Default)
		d.s(" ")
		d.n(n.Decl)
		d.s(")")
	case *js.DirectivePrologueStmt:
		d.s("(Directive ")
		d.b(n.Value)
		d.s(")")
	case *js.VarDecl:
		if n == nil {
			d.s("nil")
			return
		}
		d.varDecl(n)
	case *js.FuncDecl:
		d.s("(Func ")
		d.bool(n.Async)
		d.s(" ")
		d.bool(n.Generator)
		d.s(" ")
		d.v(n.Name)
		d.s(" ")
		d.params(n.Params)
		d.s(" ")
		d.block(&n.Body)
		d.s(")")
	case *js.MethodDecl:
		d.s("(Method ")
		d.bool(n.Static)
		d.s(" ")
		d.bool(n.Async)
		d.s(" ")
		d.bool(n.Generator)
		d.s(" ")
		d.bool(n.Get)
		d.s(" ")
		d.bool(n.Set)
		d.s(" ")
		d.pn(&n.Name)
		d.s(" ")
		d.params(n.Params)
		d.s(" ")
		d.block(&n.Body)
		d.s(")")
	case *js.ClassDecl:
		d.s("(Class ")
		d.v(n.Name)
		d.s(" ")
		d.n(n.Extends)
		d.s(" [")
		for i, e := range n.List {
			if i != 0 {
				d.s(" ")
			}
			d.s("(Elem ")
			d.block(e.StaticBlock)
			d.s(" ")
			if e.Method == nil {
				d.s("nil")
			} else {
				d.n(e.Method)
			}
			d.s(" (Field ")
			d.bool(e.Static)
			d.s(" ")
			d.pn(&e.Name)
			d.s(" ")
			d.n(e.Init)
			d.s("))")
		}
		d.s("])")
	case *js.Var:
		d.v(n)
	case *js.BindingArray:
		d.s("(BArray ")
		d.bes(n.List)
		d.s(" ")
		d.n(n.Rest)
		d.s(")")
	case *js.BindingObject:
		d.s("(BObject [")
		for i, item := range n.List {
			if i != 0 {
				d.s(" ")
			}
			d.s("(BItem ")
			d.pn(item.Key)
			d.s(" ")
			d.be(item.Value)
			d.s(")")
		}
		d.s("] ")
		d.v(n.Rest)
		d.s(")")
	case *js.LiteralExpr:
		d.lit(*n)
	case *js.ArrayExpr:
		d.s("(Array [")
		for i, e := range n.List {
			if i != 0 {
				d.s(" ")
			}
			d.s("(El ")
			d.n(e.Value)
			d.s(" ")
			d.bool(e.Spread)
			d.s(")")
		}
		d.s("])")
	case *js.ObjectExpr:
		d.s("(Object [")
		for i, p := range n.List {
			if i != 0 {
				d.s(" ")
			}
			d.s("(Prop ")
			d.pn(p.Name)
			d.s(" ")
			d.bool(p.Spread)
			d.s(" ")
			d.n(p.Value)
			d.s(" ")
			d.n(p.Init)
			d.s(")")
		}
		d.s("])")
	case *js.TemplateExpr:
		d.s("(Template ")
		d.n(n.Tag)
		d.s(" [")
		for i, p := range n.List {
			if i != 0 {
				d.s(" ")
			}
			d.s("(Part ")
			d.b(p.Value)
			d.s(" ")
			d.n(p.Expr)
			d.s(")")
		}
		d.s("] ")
		d.b(n.Tail)
		d.s(" ")
		d.i(int(n.Prec))
		d.s(" ")
		d.bool(n.Optional)
		d.s(")")
	case *js.GroupExpr:
		d.s("(Group ")
		d.n(n.X)
		d.s(")")
	case *js.IndexExpr:
		d.s("(Index ")
		d.n(n.X)
		d.s(" ")
		d.n(n.Y)
		d.s(" ")
		d.i(int(n.Prec))
		d.s(" ")
		d.bool(n.Optional)
		d.s(")")
	case *js.DotExpr:
		d.s("(Dot ")
		d.n(n.X)
		d.s(" ")
		d.lit(n.Y)
		d.s(" ")
		d.i(int(n.Prec))
		d.s(" ")
		d.bool(n.Optional)
		d.s(")")
	case *js.NewTargetExpr:
		d.s("(NewTarget)")
	case *js.ImportMetaExpr:
		d.s("(ImportMeta)")
	case *js.NewExpr:
		d.s("(New ")
		d.n(n.X)
		d.s(" ")
		if n.Args == nil {
			d.s("nil")
		} else {
			d.args(*n.Args)
		}
		d.s(")")
	case *js.CallExpr:
		d.s("(Call ")
		d.n(n.X)
		d.s(" ")
		d.args(n.Args)
		d.s(" ")
		d.bool(n.Optional)
		d.s(")")
	case *js.UnaryExpr:
		d.s("(Unary ")
		d.i(int(n.Op))
		d.s(" ")
		d.n(n.X)
		d.s(")")
	case *js.BinaryExpr:
		d.s("(Binary ")
		d.i(int(n.Op))
		d.s(" ")
		d.n(n.X)
		d.s(" ")
		d.n(n.Y)
		d.s(")")
	case *js.CondExpr:
		d.s("(Cond ")
		d.n(n.Cond)
		d.s(" ")
		d.n(n.X)
		d.s(" ")
		d.n(n.Y)
		d.s(")")
	case *js.YieldExpr:
		d.s("(Yield ")
		d.bool(n.Generator)
		d.s(" ")
		d.n(n.X)
		d.s(")")
	case *js.ArrowFunc:
		d.s("(Arrow ")
		d.bool(n.Async)
		d.s(" ")
		d.params(n.Params)
		d.s(" ")
		d.block(&n.Body)
		d.s(")")
	case *js.CommaExpr:
		d.s("(Comma [")
		for i, e := range n.List {
			if i != 0 {
				d.s(" ")
			}
			d.n(e)
		}
		d.s("])")
	default:
		panic(fmt.Sprintf("unknown node %T", node))
	}
}

func (d *dumper) varList2(l js.VarArray) {
	d.s("[")
	for i, v := range l {
		if i != 0 {
			d.s(" ")
		}
		d.v(v)
	}
	d.s("]")
}

// astDump serializes the whole AST graph: the tree, then every reachable
// scope, VarDecl and Var with pointer identities numbered by first
// encounter.
func astDump(ast *js.AST) []byte {
	d := newDumper()
	// The module scope in ast.BlockStmt is a copy of parseModule's local
	// scope, which is the one referenced by Parent/Func/VarDecl.Scope. The
	// port has a single module scope; alias the local one to the copy.
	if mod := &ast.Scope; mod.Func != nil && mod.Func != mod {
		d.scopeAlias[mod.Func] = mod
	}
	d.block(&ast.BlockStmt)
	d.s("\n== scopes\n")
	si, di := 0, 0
	for si < len(d.scopeList) || di < len(d.declList) {
		for si < len(d.scopeList) {
			s := d.scopeList[si]
			si++
			d.s("s")
			d.i(si)
			d.s(" ")
			d.sc(s.Parent)
			d.s(" ")
			d.sc(s.Func)
			d.s(" ")
			d.varList2(s.Declared)
			d.s(" ")
			d.varList2(s.Undeclared)
			d.s(" [")
			for i, vd := range s.VarDecls {
				if i != 0 {
					d.s(" ")
				}
				d.s("d")
				d.i(d.declRef(vd))
			}
			d.s("] ")
			d.i(int(s.NumForDecls))
			d.s(" ")
			d.i(int(s.NumFuncArgs))
			d.s(" ")
			d.i(int(s.NumArgUses))
			d.s(" ")
			d.bool(s.IsGlobalOrFunc)
			d.s(" ")
			d.bool(s.HasWith)
			d.s("\n")
		}
		for di < len(d.declList) {
			vd := d.declList[di]
			di++
			if !d.declPrinted[vd] {
				d.varDecl(vd)
				d.s("\n")
			}
		}
	}
	d.s("== vars\n")
	for vi := 0; vi < len(d.varList); vi++ {
		v := d.varList[vi]
		d.s("v")
		d.i(vi + 1)
		d.s(" ")
		d.b(v.Data)
		d.s(" ")
		d.v(v.Link)
		d.s(" ")
		d.i(int(v.Uses))
		d.s(" ")
		d.i(int(v.Decl))
		d.s("\n")
	}
	return d.buf.Bytes()
}

// parseDump parses src with the given options and serializes the result
// (or the error).
func parseDump(src []byte, o js.Options) (out []byte) {
	defer func() {
		if r := recover(); r != nil {
			out = []byte("PANIC\n")
		}
	}()
	ast, err := js.Parse(parse.NewInputBytes(append([]byte(nil), src...)), o)
	if err != nil {
		buf := &bytes.Buffer{}
		buf.WriteString("ERR ")
		esc(buf, []byte(err.Error()))
		buf.WriteByte('\n')
		return buf.Bytes()
	}
	return astDump(ast)
}

// stringDump is ast.String() of a successful parse with default options.
func stringDump(src []byte, o js.Options) (out []byte) {
	defer func() {
		if r := recover(); r != nil {
			out = []byte("PANIC\n")
		}
	}()
	ast, err := js.Parse(parse.NewInputBytes(append([]byte(nil), src...)), o)
	if err != nil {
		return []byte("ERR\n")
	}
	buf := &bytes.Buffer{}
	esc(buf, []byte(ast.String()))
	buf.WriteByte('\n')
	return buf.Bytes()
}

// jsDump is ast.JSString() of a successful parse with default options.
func jsDump(src []byte, o js.Options) (out []byte) {
	defer func() {
		if r := recover(); r != nil {
			out = []byte("PANIC\n")
		}
	}()
	ast, err := js.Parse(parse.NewInputBytes(append([]byte(nil), src...)), o)
	if err != nil {
		return []byte("ERR\n")
	}
	buf := &bytes.Buffer{}
	esc(buf, []byte(ast.JSString()))
	buf.WriteByte('\n')
	return buf.Bytes()
}

// jsonDump is ast.JSONString() of a successful parse: output and error.
func jsonDump(src []byte, o js.Options) (out []byte) {
	defer func() {
		if r := recover(); r != nil {
			out = []byte("PANIC\n")
		}
	}()
	ast, err := js.Parse(parse.NewInputBytes(append([]byte(nil), src...)), o)
	if err != nil {
		return []byte("ERR\n")
	}
	buf := &bytes.Buffer{}
	s, err := ast.JSONString()
	esc(buf, []byte(s))
	buf.WriteByte('\n')
	if err == nil {
		buf.WriteString("nil")
	} else {
		esc(buf, []byte(err.Error()))
	}
	buf.WriteByte('\n')
	return buf.Bytes()
}
