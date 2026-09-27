package main

// genLanguage extracts the tables of golang.org/x/text/internal/language and
// internal/language/compact that the collate tag matching (colltab.MatchLang,
// language.Parse, Canonicalize, Compose, Parent, Base) depends on.

import (
	"fmt"
	"go/ast"
	"sort"
)

func genLanguage() (*blob, error) {
	s, err := loadSrc("golang.org/x/text/internal/language", "tables.go")
	if err != nil {
		return nil, err
	}
	b := newBlob()

	// Named constants used by the ported code.
	for _, c := range []string{
		"CLDRVersion",
	} {
		v, err := s.constStr(c)
		if err != nil {
			return nil, err
		}
		if err := b.bytesSection(c, []byte(v)); err != nil {
			return nil, err
		}
	}
	for _, c := range []string{
		"nonCanonicalUnd", "langPrivateStart", "langPrivateEnd", "langNoIndexOffset",
		"isoRegionOffset", "nRegionGroups", "NumLanguages", "NumScripts", "NumRegions",
		// languages
		"_en", "_sh", "_nb", "_mo", "_no",
		"_jbo", "_ami", "_bnn", "_hak", "_tlh", "_lb", "_nv", "_pwn", "_tao", "_tay",
		"_tsu", "_nn", "_sfb", "_vgt", "_sgg", "_cmn", "_nan", "_hsn",
		// scripts
		"_Latn", "_Hani", "_Hans", "_Hant", "_Qaaa", "_Qaai", "_Qabx", "_Zinh", "_Zyyy", "_Zzzz",
		// regions
		"_001", "_419", "_BR", "_CA", "_ES", "_GB", "_MD", "_PT", "_UK", "_US", "_ZZ", "_XA", "_XC", "_XK",
	} {
		v, err := s.constInt(c)
		if err != nil {
			return nil, err
		}
		if err := b.section("const:"+c, 4, []int64{v}); err != nil {
			return nil, err
		}
	}

	// tag.Index strings.
	for _, c := range []string{"lang", "altLangISO3", "script", "regionISO", "altRegionISO3"} {
		v, err := s.constStr(c)
		if err != nil {
			return nil, err
		}
		if err := b.bytesSection(c, []byte(v)); err != nil {
			return nil, err
		}
	}

	// Integer arrays.
	for _, t := range []struct {
		name  string
		width int
	}{
		{"langNoIndex", 1},
		{"altLangIndex", 2},
		{"AliasTypes", 1},
		{"suppressScript", 1},
		{"regionTypes", 1},
		{"altRegionIDs", 2},
		{"m49", 2},
		{"m49Index", 2},
		{"fromM49", 2},
		{"regionContainment", 8},
		{"regionInclusion", 1},
		{"regionInclusionBits", 8},
		{"regionInclusionNext", 1},
	} {
		v, err := s.ints(t.name)
		if err != nil {
			return nil, err
		}
		if t.name == "AliasTypes" {
			// AliasType is int8 (Deprecated=0, Macro=1, Legacy=2; -1 unused here).
			for i, x := range v {
				if x < 0 {
					return nil, fmt.Errorf("AliasTypes[%d] = %d", i, x)
				}
			}
		}
		if err := b.section(t.name, t.width, v); err != nil {
			return nil, err
		}
	}

	// Struct arrays.
	for _, t := range []struct {
		name   string
		fields []string
	}{
		{"AliasMap", []string{"From", "To"}},
		{"regionOldMap", []string{"From", "To"}},
		{"likelyScript", []string{"lang", "region"}},
		{"likelyLang", []string{"region", "script", "flags"}},
		{"likelyLangList", []string{"region", "script", "flags"}},
		{"likelyRegion", []string{"lang", "script", "flags"}},
		{"likelyRegionList", []string{"lang", "script", "flags"}},
		{"likelyRegionGroup", []string{"lang", "region", "script"}},
	} {
		rows, err := s.structs(t.name, t.fields)
		if err != nil {
			return nil, err
		}
		if err := b.section(t.name, 2, flatten(rows)); err != nil {
			return nil, err
		}
	}

	// parents: lang, script, maxScript, toRegion, n, fromRegion[n]...
	px, err := s.varExpr("parents")
	if err != nil {
		return nil, err
	}
	pels, err := s.arrayElems(px)
	if err != nil {
		return nil, err
	}
	var pvals []int64
	for _, el := range pels {
		cl := el.(*ast.CompositeLit)
		row := map[string]ast.Expr{}
		for _, f := range cl.Elts {
			kv := f.(*ast.KeyValueExpr)
			row[kv.Key.(*ast.Ident).Name] = kv.Value
		}
		for _, f := range []string{"lang", "script", "maxScript", "toRegion"} {
			v, err := s.intVal(row[f])
			if err != nil {
				return nil, err
			}
			pvals = append(pvals, v)
		}
		from, err := s.intsOf(row["fromRegion"])
		if err != nil {
			return nil, err
		}
		pvals = append(pvals, int64(len(from)))
		pvals = append(pvals, from...)
	}
	if err := b.section("parents", 2, pvals); err != nil {
		return nil, err
	}

	// variantIndex: map[string]uint8 as "key\x00value" records sorted by key.
	vx, err := s.varExpr("variantIndex")
	if err != nil {
		return nil, err
	}
	type kv struct {
		k string
		v int64
	}
	var kvs []kv
	for _, el := range vx.(*ast.CompositeLit).Elts {
		e := el.(*ast.KeyValueExpr)
		k, err := s.strVal(e.Key)
		if err != nil {
			return nil, err
		}
		v, err := s.intVal(e.Value)
		if err != nil {
			return nil, err
		}
		kvs = append(kvs, kv{k, v})
	}
	sort.Slice(kvs, func(i, j int) bool { return kvs[i].k < kvs[j].k })
	var vb []byte
	for _, x := range kvs {
		vb = append(vb, x.k...)
		vb = append(vb, 0, byte(x.v))
	}
	if err := b.bytesSection("variantIndex", vb); err != nil {
		return nil, err
	}

	// compact tables.
	cs, err := loadSrc("golang.org/x/text/internal/language/compact", "tables.go")
	if err != nil {
		return nil, err
	}
	core, err := cs.ints("coreTags")
	if err != nil {
		return nil, err
	}
	if err := b.section("coreTags", 4, core); err != nil {
		return nil, err
	}
	st, err := cs.constStr("specialTagsStr")
	if err != nil {
		return nil, err
	}
	if err := b.bytesSection("specialTagsStr", []byte(st)); err != nil {
		return nil, err
	}
	return b, nil
}
