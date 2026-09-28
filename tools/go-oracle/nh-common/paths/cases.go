package main

import (
	"fmt"

	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

// try runs f, turning a panic into {"panic": message}.
func try(f func() any) (out any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	return f()
}

func strs(ss []string) []any {
	out := make([]any, len(ss))
	for i, s := range ss {
		out[i] = goval.Str(s)
	}
	return out
}

// accessor is one exported Path method, encoded for JSON.
type accessor struct {
	name string
	f    func(p *paths.Path) any
}

func sa(name string, f func(p *paths.Path) string) accessor {
	return accessor{name, func(p *paths.Path) any { return goval.Str(f(p)) }}
}

func ba(name string, f func(p *paths.Path) bool) accessor {
	return accessor{name, func(p *paths.Path) any { return f(p) }}
}

// outAccessors is every exported accessor of a parsed Path; the fixture
// stores their values as an array in this order (header "outKeys").
var outAccessors = []accessor{
	sa("Base", (*paths.Path).Base),
	sa("BaseNameNoIdentifier", (*paths.Path).BaseNameNoIdentifier),
	sa("BaseNoLeadingSlash", (*paths.Path).BaseNoLeadingSlash),
	sa("BaseReTyped:posts", func(p *paths.Path) string { return p.BaseReTyped("posts") }),
	sa("BaseReTyped:Posts X", func(p *paths.Path) string { return p.BaseReTyped("Posts X") }),
	sa("Component", (*paths.Path).Component),
	sa("Container", (*paths.Path).Container),
	sa("ContainerDir", (*paths.Path).ContainerDir),
	sa("Dir", (*paths.Path).Dir),
	ba("Disabled", (*paths.Path).Disabled),
	sa("Ext", (*paths.Path).Ext),
	sa("Identifier:-1", func(p *paths.Path) string { return p.Identifier(-1) }),
	sa("Identifier:len", func(p *paths.Path) string { return p.Identifier(len(p.Identifiers())) }),
	sa("IdentifierBase", (*paths.Path).IdentifierBase),
	{"Identifiers", func(p *paths.Path) any { return strs(p.Identifiers()) }},
	{"IdentifiersUnknown", func(p *paths.Path) any { return strs(p.IdentifiersUnknown()) }},
	ba("IsBranchBundle", (*paths.Path).IsBranchBundle),
	ba("IsBundle", (*paths.Path).IsBundle),
	ba("IsContent", (*paths.Path).IsContent),
	ba("IsContentData", (*paths.Path).IsContentData),
	ba("IsLeafBundle", (*paths.Path).IsLeafBundle),
	sa("Kind", (*paths.Path).Kind),
	sa("Lang", (*paths.Path).Lang),
	sa("Layout", (*paths.Path).Layout),
	sa("Name", (*paths.Path).Name),
	sa("NameNoExt", (*paths.Path).NameNoExt),
	sa("NameNoIdentifier", (*paths.Path).NameNoIdentifier),
	sa("NameNoLang", (*paths.Path).NameNoLang),
	sa("OutputFormat", (*paths.Path).OutputFormat),
	sa("Path", (*paths.Path).Path),
	sa("PathBeforeLangAndOutputFormatAndExt", (*paths.Path).PathBeforeLangAndOutputFormatAndExt),
	sa("PathNoIdentifier", (*paths.Path).PathNoIdentifier),
	sa("PathNoLang", (*paths.Path).PathNoLang),
	sa("PathNoLeadingSlash", (*paths.Path).PathNoLeadingSlash),
	sa("Section", (*paths.Path).Section),
	sa("String", (*paths.Path).String),
	sa("Type", func(p *paths.Path) string { return p.Type().String() }),
}

// miniAccessors are dumped for the term keys (normalization is what they test).
var miniAccessors = []accessor{
	sa("Base", (*paths.Path).Base),
	sa("BaseNameNoIdentifier", (*paths.Path).BaseNameNoIdentifier),
	sa("Container", (*paths.Path).Container),
	sa("Path", (*paths.Path).Path),
	sa("Section", (*paths.Path).Section),
	sa("Type", func(p *paths.Path) string { return p.Type().String() }),
}

// tlsAccessors are dumped for p.TrimLeadingSlash().
var tlsAccessors = []accessor{
	sa("Base", (*paths.Path).Base),
	sa("Container", (*paths.Path).Container),
	sa("ContainerDir", (*paths.Path).ContainerDir),
	sa("Dir", (*paths.Path).Dir),
	sa("Path", (*paths.Path).Path),
	sa("PathBeforeLangAndOutputFormatAndExt", (*paths.Path).PathBeforeLangAndOutputFormatAndExt),
	sa("PathNoLang", (*paths.Path).PathNoLang),
	sa("Section", (*paths.Path).Section),
	sa("BaseReTyped:posts", func(p *paths.Path) string { return p.BaseReTyped("posts") }),
	sa("Unnormalized.Path", func(p *paths.Path) string { return p.Unnormalized().Path() }),
}

func keys(as []accessor) []string {
	out := make([]string, len(as))
	for i, a := range as {
		out[i] = a.name
	}
	return out
}

func dump(p *paths.Path, as []accessor) []any {
	out := make([]any, len(as))
	for i, a := range as {
		out[i] = try(func() any { return a.f(p) })
	}
	return out
}

// miniSets get the mini dump of the Parse result only (no TrimLeadingSlash,
// ForType, PathRel/BaseRel or ModifyPathBundleTypeResource sections).
var miniSets = map[string]bool{"term": true}

var miniSetList = []string{"term"}

func pathParserCases(ps parserSet, inputs []input) []map[string]any {
	byID := map[string]*paths.PathParser{}
	for _, p := range ps {
		byID[p.id] = p.pp
	}
	seen := map[[3]string]bool{}
	var cases []map[string]any
	for _, in := range inputs {
		for _, pid := range in.parsers {
			key := [3]string{pid, in.c, in.s}
			if seen[key] {
				continue
			}
			seen[key] = true
			pp := byID[pid]
			c := map[string]any{"set": in.set, "p": pid, "c": in.c, "in": goval.Str(in.s)}
			c["norm"] = goval.Str(paths.NormalizePathStringBasic(in.s))
			c["hasExt"] = paths.HasExt(in.s)
			c["pb"] = try(func() any {
				base, name := pp.ParseBaseAndBaseNameNoIdentifier(in.c, in.s)
				return []any{goval.Str(base), goval.Str(name)}
			})
			c["pid"] = try(func() any { return goval.Str(string(pp.ParseIdentity(in.c, in.s))) })

			var p *paths.Path
			res := try(func() any {
				p = pp.Parse(in.c, in.s)
				return nil
			})
			if res != nil {
				c["parse"] = res
				cases = append(cases, c)
				continue
			}
			as := outAccessors
			if miniSets[in.set] {
				as = miniAccessors
			}
			c["out"] = dump(p, as)
			if u := p.Unnormalized(); u == p {
				c["u"] = "self"
			} else {
				c["u"] = dump(u, as)
			}
			if miniSets[in.set] {
				cases = append(cases, c)
				continue
			}
			c["tls"] = dump(p.TrimLeadingSlash(), tlsAccessors)
			ft := p.ForType(paths.TypePartial)
			c["ft"] = []any{
				ft.Type().String(),
				ft.IsContent(),
				try(func() any { return goval.Str(ft.Base()) }),
				ft.Unnormalized().Type().String(),
			}
			owner := pp.Parse(in.c, p.Dir()+"/_index.md")
			c["rel"] = []any{
				goval.Str(owner.Base()),
				try(func() any { return goval.Str(p.PathRel(owner)) }),
				try(func() any { return goval.Str(p.BaseRel(owner)) }),
			}
			// Last: ModifyPathBundleTypeResource mutates p.
			paths.ModifyPathBundleTypeResource(p)
			c["mod"] = []any{
				p.Type().String(),
				p.IsContent(),
				try(func() any { return goval.Str(p.Base()) }),
				try(func() any { return goval.Str(p.BaseNameNoIdentifier()) }),
				p.Unnormalized().Type().String(),
			}
			cases = append(cases, c)
		}
	}
	return cases
}

// caseKeys documents the array layouts of a case.
func caseKeys() map[string]any {
	return map[string]any{
		"out":      keys(outAccessors),
		"mini":     keys(miniAccessors),
		"tls":      keys(tlsAccessors),
		"ft":       []string{"Type", "IsContent", "Base", "Unnormalized.Type"},
		"rel":      []string{"owner.Base", "PathRel", "BaseRel"},
		"mod":      []string{"Type", "IsContent", "Base", "BaseNameNoIdentifier", "Unnormalized.Type"},
		"miniSets": miniSetList,
	}
}
