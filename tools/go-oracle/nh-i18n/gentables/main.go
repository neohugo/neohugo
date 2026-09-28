// Command gentables generates the Rust tables of crates/nh-i18n (Wave B task
// T17) from the Go sources they port:
//
//	go run ./tools/go-oracle/nh-i18n/gentables [-root .]
//
// It writes two files:
//
//   - crates/nh-i18n/src/goi18n/plural/rule_gen.rs: the CLDR plural rules of
//     github.com/gohugoio/go-i18n/v2 internal/plural/rule_gen.go, translated
//     statement by statement from the Go AST (every addPluralRules call becomes
//     a Rust call in the same order; every PluralFormFunc becomes a Rust fn with
//     the same conditions and the same CLDR comments).
//   - crates/nh-i18n/src/xlanguage/tables.rs: the golang.org/x/text@v0.26.0
//     tables that the language matcher and Maximize (addTags) read
//     (internal/language/tables.go and language/tables.go). x/text indexes them
//     by its internal language, script and region ids; the Rust port keys them
//     by the subtag strings instead (the id <-> string mapping is x/text's own:
//     the ids are decoded with language.Base/Script/Region.String(), and every
//     id < langNoIndexOffset, every script id and every region id is emitted,
//     so the mapping is complete). Id 0 is the empty string.
//
// The source files are found with `go list -m` (the versions in go.mod). The
// output is deterministic; running the command again must not change the
// checked-in files.
package main

import (
	"bytes"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"unsafe"

	"golang.org/x/text/language"
)

func main() {
	root := flag.String("root", ".", "repository root")
	flag.Parse()

	i18nDir := modDir("github.com/gohugoio/go-i18n/v2")
	textDir := modDir("golang.org/x/text")

	plural := genPlural(filepath.Join(i18nDir, "internal", "plural", "rule_gen.go"))
	write(filepath.Join(*root, "crates/nh-i18n/src/goi18n/plural/rule_gen.rs"), plural)

	tables := genLanguage(textDir)
	write(filepath.Join(*root, "crates/nh-i18n/src/xlanguage/tables.rs"), tables)
}

func modDir(mod string) string {
	out, err := exec.Command("go", "list", "-m", "-f", "{{.Dir}}|{{.Version}}", mod).Output()
	if err != nil {
		log.Fatalf("go list -m %s: %v", mod, err)
	}
	parts := strings.SplitN(strings.TrimSpace(string(out)), "|", 2)
	versions[mod] = parts[1]
	return parts[0]
}

var versions = map[string]string{}

func write(path string, b []byte) {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		log.Fatal(err)
	}
	if err := os.WriteFile(path, b, 0o644); err != nil {
		log.Fatal(err)
	}
	fmt.Println("wrote", path)
}

// ---------------------------------------------------------------------------
// Plural rules

func genPlural(path string) []byte {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, path, nil, parser.ParseComments)
	if err != nil {
		log.Fatal(err)
	}
	var body *ast.BlockStmt
	for _, d := range f.Decls {
		if fd, ok := d.(*ast.FuncDecl); ok && fd.Name.Name == "DefaultRules" {
			body = fd.Body
		}
	}
	if body == nil {
		log.Fatal("DefaultRules not found")
	}
	// Comments by the line they end on.
	commentsByEndLine := map[int][]string{}
	for _, cg := range f.Comments {
		line := fset.Position(cg.End()).Line
		for _, c := range cg.List {
			commentsByEndLine[line] = append(commentsByEndLine[line], strings.TrimSpace(strings.TrimPrefix(c.Text, "//")))
		}
	}

	var calls, funcs bytes.Buffer
	n := 0
	for _, st := range body.List {
		es, ok := st.(*ast.ExprStmt)
		if !ok {
			continue
		}
		call := es.X.(*ast.CallExpr)
		if id, ok := call.Fun.(*ast.Ident); !ok || id.Name != "addPluralRules" {
			log.Fatalf("unexpected call %T", call.Fun)
		}
		ids := call.Args[1].(*ast.CompositeLit)
		var idStrs []string
		for _, e := range ids.Elts {
			idStrs = append(idStrs, e.(*ast.BasicLit).Value)
		}
		rule := call.Args[2].(*ast.UnaryExpr).X.(*ast.CompositeLit)
		var forms []string
		var fn *ast.FuncLit
		for _, e := range rule.Elts {
			kv := e.(*ast.KeyValueExpr)
			switch kv.Key.(*ast.Ident).Name {
			case "PluralForms":
				for _, a := range kv.Value.(*ast.CallExpr).Args {
					forms = append(forms, "Form::"+a.(*ast.Ident).Name)
				}
			case "PluralFormFunc":
				fn = kv.Value.(*ast.FuncLit)
			}
		}
		fname := fmt.Sprintf("rule_%d", n)
		fmt.Fprintf(&calls, "    add_plural_rules(\n        &mut rules,\n        &[%s],\n        Rule {\n            plural_forms: new_plural_form_set(&[%s]),\n            plural_form_func: %s,\n        },\n    );\n",
			strings.Join(idStrs, ", "), strings.Join(forms, ", "), fname)

		param := "_ops"
		for _, s := range fn.Body.List {
			if _, ok := s.(*ast.IfStmt); ok {
				param = "ops"
			}
		}
		fmt.Fprintf(&funcs, "\n// Go: internal/plural/rule_gen.go:DefaultRules (PluralFormFunc of %s)\nfn %s(%s: &Operands) -> Form {\n", strings.Join(idStrs, ", "), fname, param)
		for _, s := range fn.Body.List {
			line := fset.Position(s.Pos()).Line
			for _, c := range commentsByEndLine[line-1] {
				fmt.Fprintf(&funcs, "    // %s\n", c)
			}
			switch s := s.(type) {
			case *ast.IfStmt:
				ret := s.Body.List[0].(*ast.ReturnStmt).Results[0].(*ast.Ident).Name
				fmt.Fprintf(&funcs, "    if %s {\n        return Form::%s;\n    }\n", rustExpr(s.Cond), ret)
			case *ast.ReturnStmt:
				fmt.Fprintf(&funcs, "    Form::%s\n", s.Results[0].(*ast.Ident).Name)
			default:
				log.Fatalf("unexpected stmt %T", s)
			}
		}
		funcs.WriteString("}\n")
		n++
	}

	var out bytes.Buffer
	fmt.Fprintf(&out, "// This file is generated by tools/go-oracle/nh-i18n/gentables from\n// github.com/gohugoio/go-i18n/v2@%s internal/plural/rule_gen.go; DO NOT EDIT.\n\n", versions["github.com/gohugoio/go-i18n/v2"])
	out.WriteString("//! Go: `internal/plural/rule_gen.go` — the CLDR plural rules, in Go's order (a later\n//! `addPluralRules` for the same canonical tag replaces the earlier rule, as in Go).\n\n")
	out.WriteString("use super::form::Form;\nuse super::operands::Operands;\nuse super::rule::{Rule, add_plural_rules, int_equals_any, int_in_range, new_plural_form_set};\nuse super::rules::Rules;\n\n")
	out.WriteString("/// Go: `DefaultRules()` — the Rules generated from CLDR language data.\n// Go: internal/plural/rule_gen.go:DefaultRules\npub fn default_rules() -> Rules {\n    let mut rules = Rules::default();\n")
	out.Write(calls.Bytes())
	out.WriteString("    rules\n}\n")
	out.Write(funcs.Bytes())
	return out.Bytes()
}

func rustExpr(e ast.Expr) string {
	switch e := e.(type) {
	case *ast.BinaryExpr:
		switch e.Op {
		case token.LAND, token.LOR, token.REM:
		default:
			log.Fatalf("unexpected op %s", e.Op)
		}
		return rustExpr(e.X) + " " + e.Op.String() + " " + rustExpr(e.Y)
	case *ast.ParenExpr:
		return "(" + rustExpr(e.X) + ")"
	case *ast.UnaryExpr:
		if e.Op != token.NOT {
			log.Fatalf("unexpected unary %s", e.Op)
		}
		return "!" + rustExpr(e.X)
	case *ast.BasicLit:
		return e.Value
	case *ast.SelectorExpr:
		// ops.I -> ops.i
		return rustExpr(e.X) + "." + strings.ToLower(e.Sel.Name)
	case *ast.Ident:
		return e.Name
	case *ast.CallExpr:
		var args []string
		for _, a := range e.Args {
			args = append(args, rustExpr(a))
		}
		switch fn := e.Fun.(type) {
		case *ast.Ident:
			switch fn.Name {
			case "intEqualsAny":
				return fmt.Sprintf("int_equals_any(%s, &[%s])", args[0], strings.Join(args[1:], ", "))
			case "intInRange":
				return fmt.Sprintf("int_in_range(%s, %s, %s)", args[0], args[1], args[2])
			}
		case *ast.SelectorExpr:
			recv := rustExpr(fn.X)
			switch fn.Sel.Name {
			case "NEqualsAny":
				return fmt.Sprintf("%s.n_equals_any(&[%s])", recv, strings.Join(args, ", "))
			case "NModEqualsAny":
				return fmt.Sprintf("%s.n_mod_equals_any(%s, &[%s])", recv, args[0], strings.Join(args[1:], ", "))
			case "NInRange":
				return fmt.Sprintf("%s.n_in_range(%s, %s)", recv, args[0], args[1])
			case "NModInRange":
				return fmt.Sprintf("%s.n_mod_in_range(%s, %s, %s)", recv, args[0], args[1], args[2])
			}
		}
		log.Fatalf("unexpected call %#v", e.Fun)
	}
	log.Fatalf("unexpected expr %T", e)
	return ""
}

// ---------------------------------------------------------------------------
// x/text language tables

// langStr, scriptStr and regionStr decode x/text's internal ids with the
// public String methods (the public types are structs holding only the id).
func langStr(id uint64) string {
	if id == 0 {
		return ""
	}
	var b language.Base
	*(*uint16)(unsafe.Pointer(&b)) = uint16(id)
	return b.String()
}

func scriptStr(id uint64) string {
	if id == 0 {
		return ""
	}
	var s language.Script
	*(*uint16)(unsafe.Pointer(&s)) = uint16(id)
	return s.String()
}

func regionStr(id uint64) string {
	if id == 0 {
		return ""
	}
	var r language.Region
	*(*uint16)(unsafe.Pointer(&r)) = uint16(id)
	return r.String()
}

type goFile struct {
	vars   map[string]ast.Expr
	consts map[string]ast.Expr
}

func parseGo(path string) goFile {
	fset := token.NewFileSet()
	f, err := parser.ParseFile(fset, path, nil, 0)
	if err != nil {
		log.Fatal(err)
	}
	g := goFile{vars: map[string]ast.Expr{}, consts: map[string]ast.Expr{}}
	for _, d := range f.Decls {
		gd, ok := d.(*ast.GenDecl)
		if !ok {
			continue
		}
		for _, s := range gd.Specs {
			vs, ok := s.(*ast.ValueSpec)
			if !ok {
				continue
			}
			for i, n := range vs.Names {
				if i >= len(vs.Values) {
					continue
				}
				if gd.Tok == token.VAR {
					g.vars[n.Name] = vs.Values[i]
				} else {
					g.consts[n.Name] = vs.Values[i]
				}
			}
		}
	}
	return g
}

func intLit(e ast.Expr) uint64 {
	switch e := e.(type) {
	case *ast.BasicLit:
		v, err := strconv.ParseUint(e.Value, 0, 64)
		if err != nil {
			log.Fatal(err)
		}
		return v
	case *ast.Ident:
		switch e.Name {
		case "true":
			return 1
		case "false":
			return 0
		}
	}
	log.Fatalf("not an int literal: %#v", e)
	return 0
}

// elems evaluates an array/slice composite literal whose elements are ints,
// keyed structs ({field: v}) or positional arrays; index keys are honoured
// and missing elements are zero (nil maps / zero slices).
func elems(e ast.Expr) []any {
	cl := e.(*ast.CompositeLit)
	n := 0
	if at, ok := cl.Type.(*ast.ArrayType); ok && at.Len != nil {
		n = int(intLit(at.Len))
	}
	out := make([]any, n)
	idx := 0
	for _, el := range cl.Elts {
		if kv, ok := el.(*ast.KeyValueExpr); ok {
			idx = int(intLit(kv.Key))
			el = kv.Value
		}
		for idx >= len(out) {
			out = append(out, nil)
		}
		out[idx] = elem(el)
		idx++
	}
	return out
}

func elem(e ast.Expr) any {
	cl, ok := e.(*ast.CompositeLit)
	if !ok {
		return intLit(e)
	}
	if len(cl.Elts) > 0 {
		if kv, ok := cl.Elts[0].(*ast.KeyValueExpr); ok {
			if _, ok := kv.Key.(*ast.Ident); ok {
				m := map[string]uint64{}
				for _, el := range cl.Elts {
					kv := el.(*ast.KeyValueExpr)
					m[kv.Key.(*ast.Ident).Name] = intLit(kv.Value)
				}
				return m
			}
		}
	}
	var vs []uint64
	for _, el := range cl.Elts {
		vs = append(vs, intLit(el))
	}
	return vs
}

func field(v any, name string) uint64 {
	if v == nil {
		return 0
	}
	return v.(map[string]uint64)[name]
}

func num(v any) uint64 {
	if v == nil {
		return 0
	}
	return v.(uint64)
}

type kv struct {
	key string
	val string
}

// sortedTable writes `pub(super) static NAME: &[(&str, T)] = &[...]` sorted by key.
func sortedTable(out *bytes.Buffer, doc, name, typ string, rows []kv) {
	sort.Slice(rows, func(i, j int) bool { return rows[i].key < rows[j].key })
	for i := 1; i < len(rows); i++ {
		if rows[i].key == rows[i-1].key {
			log.Fatalf("%s: duplicate key %q", name, rows[i].key)
		}
	}
	fmt.Fprintf(out, "\n/// %s\npub(super) static %s: &[(&str, %s)] = &[\n", doc, name, typ)
	for _, r := range rows {
		fmt.Fprintf(out, "    (%q, %s),\n", r.key, r.val)
	}
	out.WriteString("];\n")
}

func genLanguage(textDir string) []byte {
	in := parseGo(filepath.Join(textDir, "internal", "language", "tables.go"))
	pub := parseGo(filepath.Join(textDir, "language", "tables.go"))

	langNoIndexOffset := intLit(in.consts["langNoIndexOffset"])
	nRegionGroups := intLit(in.consts["nRegionGroups"])

	var out bytes.Buffer
	fmt.Fprintf(&out, "// This file is generated by tools/go-oracle/nh-i18n/gentables from\n// golang.org/x/text@%s internal/language/tables.go and language/tables.go; DO NOT EDIT.\n\n", versions["golang.org/x/text"])
	out.WriteString("//! x/text's language matching tables (CLDR 32), keyed by subtag strings (\"\" = x/text id 0).\n\n")
	out.WriteString("use super::{LikelyLangRegion, LikelyLangScript, LikelyScriptRegion, LikelyTag};\nuse super::{MutualIntelligibility, RegionIntelligibility, ScriptIntelligibility};\n")
	fmt.Fprintf(&out, "\n/// Go: `nRegionGroups`.\npub(super) const N_REGION_GROUPS: u8 = %d;\n", nRegionGroups)

	// likelyLang: every language id below langNoIndexOffset (membership is the
	// Go `t.LangID < langNoIndexOffset` test).
	var rows []kv
	ll := elems(in.vars["likelyLang"])
	if uint64(len(ll)) != langNoIndexOffset {
		log.Fatalf("likelyLang has %d entries, want %d", len(ll), langNoIndexOffset)
	}
	for id, v := range ll {
		rows = append(rows, kv{langStr(uint64(id)), scriptRegion(v)})
	}
	sortedTable(&out, "Go: `likelyLang` (every language id < `langNoIndexOffset`).", "LIKELY_LANG", "LikelyScriptRegion", rows)

	var list bytes.Buffer
	list.WriteString("\n/// Go: `likelyLangList`.\npub(super) static LIKELY_LANG_LIST: &[LikelyScriptRegion] = &[\n")
	for _, v := range elems(in.vars["likelyLangList"]) {
		fmt.Fprintf(&list, "    %s,\n", scriptRegion(v))
	}
	list.WriteString("];\n")
	out.Write(list.Bytes())

	rows = nil
	for id, v := range elems(in.vars["likelyScript"]) {
		if v == nil {
			continue
		}
		rows = append(rows, kv{scriptStr(uint64(id)), fmt.Sprintf("LikelyLangRegion { lang: %q, region: %q }", langStr(field(v, "lang")), regionStr(field(v, "region")))})
	}
	sortedTable(&out, "Go: `likelyScript` (non-zero entries).", "LIKELY_SCRIPT", "LikelyLangRegion", rows)

	rows = nil
	lr := elems(in.vars["likelyRegion"])
	for id, v := range lr {
		rows = append(rows, kv{regionStr(uint64(id)), langScript(v)})
	}
	sortedTable(&out, "Go: `likelyRegion` (every region id).", "LIKELY_REGION", "LikelyLangScript", rows)

	list.Reset()
	list.WriteString("\n/// Go: `likelyRegionList`.\npub(super) static LIKELY_REGION_LIST: &[LikelyLangScript] = &[\n")
	for _, v := range elems(in.vars["likelyRegionList"]) {
		fmt.Fprintf(&list, "    %s,\n", langScript(v))
	}
	list.WriteString("];\n")
	out.Write(list.Bytes())

	list.Reset()
	list.WriteString("\n/// Go: `likelyRegionGroup`.\npub(super) static LIKELY_REGION_GROUP: &[LikelyTag] = &[\n")
	for _, v := range elems(in.vars["likelyRegionGroup"]) {
		fmt.Fprintf(&list, "    LikelyTag { lang: %q, region: %q, script: %q },\n", langStr(field(v, "lang")), regionStr(field(v, "region")), scriptStr(field(v, "script")))
	}
	list.WriteString("];\n")
	out.Write(list.Bytes())

	u64s := func(doc, name string, vs []any) {
		fmt.Fprintf(&out, "\n/// %s\npub(super) static %s: &[u64] = &[\n", doc, name)
		for _, v := range vs {
			fmt.Fprintf(&out, "    0x%016x,\n", num(v))
		}
		out.WriteString("];\n")
	}
	u64s("Go: `regionContainment`.", "REGION_CONTAINMENT", elems(in.vars["regionContainment"]))
	u64s("Go: `regionInclusionBits`.", "REGION_INCLUSION_BITS", elems(in.vars["regionInclusionBits"]))

	rows = nil
	for id, v := range elems(in.vars["regionInclusion"]) {
		rows = append(rows, kv{regionStr(uint64(id)), fmt.Sprintf("%d", num(v))})
	}
	sortedTable(&out, "Go: `regionInclusion` (every region id).", "REGION_INCLUSION", "u8", rows)

	rows = nil
	for id, v := range elems(in.vars["suppressScript"]) {
		if num(v) == 0 {
			continue
		}
		rows = append(rows, kv{langStr(uint64(id)), fmt.Sprintf("%q", scriptStr(num(v)))})
	}
	sortedTable(&out, "Go: `suppressScript` (non-zero entries).", "SUPPRESS_SCRIPT", "&str", rows)

	am := elems(in.vars["AliasMap"])
	at := elems(in.vars["AliasTypes"])
	if len(am) != len(at) {
		log.Fatal("AliasMap/AliasTypes length mismatch")
	}
	out.WriteString("\n/// Go: `AliasMap` + `AliasTypes` (from, to, type: 0 Deprecated, 1 Macro, 2 Legacy).\npub(super) static ALIAS_MAP: &[(&str, &str, u8)] = &[\n")
	for i, v := range am {
		fmt.Fprintf(&out, "    (%q, %q, %d),\n", langStr(field(v, "From")), langStr(field(v, "To")), num(at[i]))
	}
	out.WriteString("];\n")

	rows = nil
	for id, v := range elems(pub.vars["regionToGroups"]) {
		rows = append(rows, kv{regionStr(uint64(id)), fmt.Sprintf("%d", num(v))})
	}
	sortedTable(&out, "Go: `regionToGroups` (every region id).", "REGION_TO_GROUPS", "u8", rows)

	out.WriteString("\n/// Go: `paradigmLocales` (lang, region, region; \"\" is filled in by the matcher's init).\npub(super) static PARADIGM_LOCALES: &[(&str, &str, &str)] = &[\n")
	for _, v := range elems(pub.vars["paradigmLocales"]) {
		a := v.([]uint64)
		fmt.Fprintf(&out, "    (%q, %q, %q),\n", langStr(a[0]), regionStr(a[1]), regionStr(a[2]))
	}
	out.WriteString("];\n")

	out.WriteString("\n/// Go: `matchLang`.\npub(super) static MATCH_LANG: &[MutualIntelligibility] = &[\n")
	for _, v := range elems(pub.vars["matchLang"]) {
		fmt.Fprintf(&out, "    MutualIntelligibility { want: %q, have: %q, distance: %d, oneway: %t },\n",
			langStr(field(v, "want")), langStr(field(v, "have")), field(v, "distance"), field(v, "oneway") == 1)
	}
	out.WriteString("];\n")

	out.WriteString("\n/// Go: `matchScript`.\npub(super) static MATCH_SCRIPT: &[ScriptIntelligibility] = &[\n")
	for _, v := range elems(pub.vars["matchScript"]) {
		fmt.Fprintf(&out, "    ScriptIntelligibility { want_lang: %q, have_lang: %q, want_script: %q, have_script: %q, distance: %d },\n",
			langStr(field(v, "wantLang")), langStr(field(v, "haveLang")), scriptStr(field(v, "wantScript")), scriptStr(field(v, "haveScript")), field(v, "distance"))
	}
	out.WriteString("];\n")

	out.WriteString("\n/// Go: `matchRegion`.\npub(super) static MATCH_REGION: &[RegionIntelligibility] = &[\n")
	for _, v := range elems(pub.vars["matchRegion"]) {
		fmt.Fprintf(&out, "    RegionIntelligibility { lang: %q, script: %q, group: 0x%02x, distance: %d },\n",
			langStr(field(v, "lang")), scriptStr(field(v, "script")), field(v, "group"), field(v, "distance"))
	}
	out.WriteString("];\n")
	return out.Bytes()
}

// scriptRegion formats a likelyScriptRegion: the subtag strings, or, when
// flags&isList != 0, the list index and size (region, script) as numbers.
func scriptRegion(v any) string {
	r, s, f := field(v, "region"), field(v, "script"), field(v, "flags")
	if f&1 != 0 {
		return fmt.Sprintf("LikelyScriptRegion { region: \"\", script: \"\", region_n: %d, script_n: %d, flags: %d }", r, s, f)
	}
	return fmt.Sprintf("LikelyScriptRegion { region: %q, script: %q, region_n: 0, script_n: 0, flags: %d }", regionStr(r), scriptStr(s), f)
}

// langScript formats a likelyLangScript like scriptRegion.
func langScript(v any) string {
	l, s, f := field(v, "lang"), field(v, "script"), field(v, "flags")
	if f&1 != 0 {
		return fmt.Sprintf("LikelyLangScript { lang: \"\", script: \"\", lang_n: %d, script_n: %d, flags: %d }", l, s, f)
	}
	return fmt.Sprintf("LikelyLangScript { lang: %q, script: %q, lang_n: 0, script_n: 0, flags: %d }", langStr(l), scriptStr(s), f)
}
