package main

// A tiny evaluator for the literal-only generated table files of
// golang.org/x/text (collate/tables.go, unicode/norm/tables15.0.0.go,
// internal/language/tables.go, ...). The tables are unexported, so instead of
// importing them we parse the exact source files of the module version pinned
// in go.mod and evaluate their composite literals.

import (
	"fmt"
	"go/ast"
	"go/build"
	"go/constant"
	"go/parser"
	"go/token"
	"path/filepath"
)

type srcFile struct {
	path   string
	fset   *token.FileSet
	file   *ast.File
	consts map[string]ast.Expr
	vars   map[string]ast.Expr
	types  map[string]ast.Expr
	funcs  map[string]*ast.FuncDecl // "Recv.Name" or "Name"
}

// pkgDir returns the directory of an import path as resolved by the go
// command for the current module (so the pinned x/text version is used).
func pkgDir(importPath string) (string, error) {
	p, err := build.Default.Import(importPath, ".", build.FindOnly)
	if err != nil {
		return "", err
	}
	return p.Dir, nil
}

func loadSrc(importPath, name string) (*srcFile, error) {
	dir, err := pkgDir(importPath)
	if err != nil {
		return nil, err
	}
	path := filepath.Join(dir, name)
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, path, nil, 0)
	if err != nil {
		return nil, err
	}
	s := &srcFile{
		path:   path,
		fset:   fset,
		file:   f,
		consts: map[string]ast.Expr{},
		vars:   map[string]ast.Expr{},
		types:  map[string]ast.Expr{},
		funcs:  map[string]*ast.FuncDecl{},
	}
	for _, d := range f.Decls {
		switch d := d.(type) {
		case *ast.GenDecl:
			var last ast.Expr
			for _, spec := range d.Specs {
				switch sp := spec.(type) {
				case *ast.ValueSpec:
					for i, n := range sp.Names {
						var v ast.Expr
						if i < len(sp.Values) {
							v = sp.Values[i]
						}
						if d.Tok == token.CONST {
							if v == nil {
								v = last
							}
							s.consts[n.Name] = v
						} else {
							s.vars[n.Name] = v
						}
					}
					if len(sp.Values) > 0 {
						last = sp.Values[0]
					}
				case *ast.TypeSpec:
					s.types[sp.Name.Name] = sp.Type
				}
			}
		case *ast.FuncDecl:
			name := d.Name.Name
			if d.Recv != nil && len(d.Recv.List) == 1 {
				t := d.Recv.List[0].Type
				if st, ok := t.(*ast.StarExpr); ok {
					t = st.X
				}
				if id, ok := t.(*ast.Ident); ok {
					name = id.Name + "." + name
				}
			}
			s.funcs[name] = d
		}
	}
	return s, nil
}

func (s *srcFile) pos(e ast.Node) string {
	return s.fset.Position(e.Pos()).String()
}

// constVal evaluates a constant expression.
func (s *srcFile) constVal(e ast.Expr) (constant.Value, error) {
	switch e := e.(type) {
	case *ast.BasicLit:
		v := constant.MakeFromLiteral(e.Value, e.Kind, 0)
		if v.Kind() == constant.Unknown {
			return nil, fmt.Errorf("%s: bad literal %s", s.pos(e), e.Value)
		}
		return v, nil
	case *ast.Ident:
		if x, ok := s.consts[e.Name]; ok && x != nil {
			return s.constVal(x)
		}
		switch e.Name {
		case "true":
			return constant.MakeBool(true), nil
		case "false":
			return constant.MakeBool(false), nil
		}
		return nil, fmt.Errorf("%s: unknown identifier %s", s.pos(e), e.Name)
	case *ast.ParenExpr:
		return s.constVal(e.X)
	case *ast.UnaryExpr:
		x, err := s.constVal(e.X)
		if err != nil {
			return nil, err
		}
		return constant.UnaryOp(e.Op, x, 0), nil
	case *ast.BinaryExpr:
		x, err := s.constVal(e.X)
		if err != nil {
			return nil, err
		}
		y, err := s.constVal(e.Y)
		if err != nil {
			return nil, err
		}
		switch e.Op {
		case token.SHL, token.SHR:
			n, _ := constant.Uint64Val(y)
			return constant.Shift(x, e.Op, uint(n)), nil
		case token.EQL, token.NEQ, token.LSS, token.LEQ, token.GTR, token.GEQ:
			return constant.MakeBool(constant.Compare(x, e.Op, y)), nil
		}
		if e.Op == token.QUO && x.Kind() == constant.Int && y.Kind() == constant.Int {
			return constant.BinaryOp(x, token.QUO_ASSIGN, y), nil
		}
		return constant.BinaryOp(x, e.Op, y), nil
	case *ast.CallExpr:
		// Type conversion such as uint16(x).
		if len(e.Args) == 1 {
			return s.constVal(e.Args[0])
		}
	}
	return nil, fmt.Errorf("%s: unsupported constant expression %T", s.pos(e), e)
}

func (s *srcFile) intVal(e ast.Expr) (int64, error) {
	v, err := s.constVal(e)
	if err != nil {
		return 0, err
	}
	if v.Kind() == constant.Int {
		if i, ok := constant.Int64Val(v); ok {
			return i, nil
		}
		if u, ok := constant.Uint64Val(v); ok {
			return int64(u), nil
		}
	}
	return 0, fmt.Errorf("%s: not an integer: %v", s.pos(e), v)
}

func (s *srcFile) strVal(e ast.Expr) (string, error) {
	v, err := s.constVal(e)
	if err != nil {
		return "", err
	}
	if v.Kind() != constant.String {
		return "", fmt.Errorf("%s: not a string: %v", s.pos(e), v)
	}
	return constant.StringVal(v), nil
}

// constInt returns the value of a named constant.
func (s *srcFile) constInt(name string) (int64, error) {
	x, ok := s.consts[name]
	if !ok || x == nil {
		return 0, fmt.Errorf("%s: const %s not found", s.path, name)
	}
	return s.intVal(x)
}

func (s *srcFile) constStr(name string) (string, error) {
	x, ok := s.consts[name]
	if !ok {
		if x, ok = s.vars[name]; !ok {
			return "", fmt.Errorf("%s: const %s not found", s.path, name)
		}
	}
	return s.strVal(x)
}

func (s *srcFile) varExpr(name string) (ast.Expr, error) {
	x, ok := s.vars[name]
	if !ok || x == nil {
		return nil, fmt.Errorf("%s: var %s not found", s.path, name)
	}
	return x, nil
}

// arrayElems expands an array or slice composite literal into its element
// expressions, honouring keyed indices. Missing elements are nil (zero).
func (s *srcFile) arrayElems(e ast.Expr) ([]ast.Expr, error) {
	cl, ok := e.(*ast.CompositeLit)
	if !ok {
		return nil, fmt.Errorf("%s: not a composite literal", s.pos(e))
	}
	n := -1
	if at, ok := cl.Type.(*ast.ArrayType); ok && at.Len != nil {
		if _, ok := at.Len.(*ast.Ellipsis); !ok {
			l, err := s.intVal(at.Len)
			if err != nil {
				return nil, err
			}
			n = int(l)
		}
	}
	m := map[int]ast.Expr{}
	idx, max := 0, 0
	for _, el := range cl.Elts {
		if kv, ok := el.(*ast.KeyValueExpr); ok {
			k, err := s.intVal(kv.Key)
			if err != nil {
				return nil, err
			}
			idx = int(k)
			el = kv.Value
		}
		m[idx] = el
		idx++
		if idx > max {
			max = idx
		}
	}
	if n < 0 {
		n = max
	}
	if max > n {
		return nil, fmt.Errorf("%s: literal has %d elements, array length %d", s.pos(e), max, n)
	}
	out := make([]ast.Expr, n)
	for i, x := range m {
		out[i] = x
	}
	return out, nil
}

// ints evaluates an array literal of integers.
func (s *srcFile) ints(name string) ([]int64, error) {
	x, err := s.varExpr(name)
	if err != nil {
		return nil, err
	}
	return s.intsOf(x)
}

func (s *srcFile) intsOf(x ast.Expr) ([]int64, error) {
	els, err := s.arrayElems(x)
	if err != nil {
		return nil, err
	}
	out := make([]int64, len(els))
	for i, el := range els {
		if el == nil {
			continue
		}
		v, err := s.intVal(el)
		if err != nil {
			return nil, err
		}
		out[i] = v
	}
	return out, nil
}

// structs evaluates an array literal of structs with the given field order.
func (s *srcFile) structs(name string, fields []string) ([][]int64, error) {
	x, err := s.varExpr(name)
	if err != nil {
		return nil, err
	}
	return s.structsOf(x, fields)
}

func (s *srcFile) structsOf(x ast.Expr, fields []string) ([][]int64, error) {
	els, err := s.arrayElems(x)
	if err != nil {
		return nil, err
	}
	out := make([][]int64, len(els))
	for i, el := range els {
		row := make([]int64, len(fields))
		out[i] = row
		if el == nil {
			continue
		}
		v, err := s.structOf(el, fields)
		if err != nil {
			return nil, err
		}
		copy(row, v)
	}
	return out, nil
}

func (s *srcFile) structOf(el ast.Expr, fields []string) ([]int64, error) {
	row := make([]int64, len(fields))
	cl, ok := el.(*ast.CompositeLit)
	if !ok {
		return nil, fmt.Errorf("%s: struct element is %T", s.pos(el), el)
	}
	for j, f := range cl.Elts {
		if kv, ok := f.(*ast.KeyValueExpr); ok {
			id := kv.Key.(*ast.Ident).Name
			k := -1
			for fi, fn := range fields {
				if fn == id {
					k = fi
				}
			}
			if k < 0 {
				return nil, fmt.Errorf("%s: unknown field %s", s.pos(f), id)
			}
			v, err := s.intVal(kv.Value)
			if err != nil {
				return nil, err
			}
			row[k] = v
			continue
		}
		if j >= len(fields) {
			return nil, fmt.Errorf("%s: too many struct fields", s.pos(f))
		}
		v, err := s.intVal(f)
		if err != nil {
			return nil, err
		}
		row[j] = v
	}
	return row, nil
}

// caseLessThan finds the constant n in a "case n < X:" clause inside the
// named function (used for the trie block cutoffs of norm tries).
func (s *srcFile) caseLessThan(fn string) (int64, error) {
	f, ok := s.funcs[fn]
	if !ok {
		return 0, fmt.Errorf("%s: func %s not found", s.path, fn)
	}
	var res int64 = -1
	var ferr error
	ast.Inspect(f.Body, func(n ast.Node) bool {
		cc, ok := n.(*ast.CaseClause)
		if !ok || res >= 0 {
			return true
		}
		for _, e := range cc.List {
			if be, ok := e.(*ast.BinaryExpr); ok && be.Op == token.LSS {
				v, err := s.intVal(be.Y)
				if err != nil {
					ferr = err
					return false
				}
				res = v
				return false
			}
		}
		return true
	})
	if ferr != nil {
		return 0, ferr
	}
	if res < 0 {
		return 0, fmt.Errorf("%s: no case n < X in %s", s.path, fn)
	}
	return res, nil
}
