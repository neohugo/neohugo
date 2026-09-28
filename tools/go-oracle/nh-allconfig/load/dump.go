package main

import (
	"encoding/json"
	"fmt"
	"sort"
	"time"

	"github.com/neohugo/neohugo/common/types"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/allconfig"
	"github.com/neohugo/neohugo/hugolib/segments"
	"github.com/neohugo/neohugo/langs"
	"github.com/neohugo/neohugo/modules"
)

type dumper struct {
	// r replaces the temporary root with "$ROOT".
	r func(string) string
}

func (d *dumper) json(v any) (string, error) {
	b, err := json.Marshal(v)
	if err != nil {
		return "", err
	}
	return d.r(string(b)), nil
}

func sortedKeys[V any](m map[string]V) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	return keys
}

func (d *dumper) configs(confs *allconfig.Configs) (map[string]any, error) {
	out := map[string]any{}

	out["configFiles"] = d.strs(confs.LoadingInfo.ConfigFiles)
	bc := confs.LoadingInfo.BaseConfig
	out["baseConfig"] = map[string]any{
		"workingDir": d.r(bc.WorkingDir),
		"cacheDir":   d.r(bc.CacheDir),
		"themesDir":  d.r(bc.ThemesDir),
		"publishDir": d.r(bc.PublishDir),
	}
	out["isMultihost"] = confs.IsMultihost

	var langs, langsDF []string
	for _, l := range confs.Languages {
		langs = append(langs, l.Lang)
	}
	for _, l := range confs.LanguagesDefaultFirst {
		langsDF = append(langsDF, l.Lang)
	}
	out["languages"] = langs
	out["languagesDefaultFirst"] = langsDF

	// The language keys in LanguageConfigSlice order (sort order of Init).
	out["languageConfigSlice"] = languageSliceKeys(confs)

	base, err := d.config(confs.Base)
	if err != nil {
		return nil, err
	}
	out["base"] = base

	lcm := map[string]any{}
	for _, k := range sortedKeys(confs.LanguageConfigMap) {
		c := confs.LanguageConfigMap[k]
		if c == confs.Base {
			lcm[k] = map[string]any{"sameAsBase": true}
			continue
		}
		dc, err := d.config(c)
		if err != nil {
			return nil, err
		}
		lcm[k] = dc
	}
	out["languageConfigs"] = lcm

	var cls []any
	for _, cl := range confs.ConfigLangs() {
		cls = append(cls, d.configLang(confs, cl))
	}
	out["configLangs"] = cls

	var mods []any
	for _, m := range confs.Modules {
		dm, err := d.module(m)
		if err != nil {
			return nil, err
		}
		mods = append(mods, dm)
	}
	out["modules"] = mods

	md, err := mountsDump(confs)
	if err != nil {
		return nil, err
	}
	out["mountsDump"] = d.r(md)

	dumps := map[string]any{}
	for _, lang := range append([]string{""}, sortedKeys(confs.LanguageConfigMap)...) {
		for _, zero := range []bool{false, true} {
			s, err := configDump(confs, lang, zero)
			if err != nil {
				return nil, err
			}
			dumps[fmt.Sprintf("%s|%v", lang, zero)] = d.r(s)
		}
	}
	out["configDumps"] = dumps

	root, err := d.json(confs.LoadingInfo.Cfg.Get(""))
	if err != nil {
		return nil, err
	}
	out["providerRoot"] = root

	return out, nil
}

// languageSliceKeys returns the language key of each LanguageConfigSlice
// entry. Init appends the configs in the sorted langKeys order; configs shared
// with Base appear once per language, so match by sort order.
func languageSliceKeys(confs *allconfig.Configs) []string {
	var keys []string
	for k := range confs.LanguageConfigMap {
		keys = append(keys, k)
	}
	sort.Slice(keys, func(i, j int) bool {
		ki, kj := keys[i], keys[j]
		li := confs.LanguageConfigMap[ki].Languages[ki]
		lj := confs.LanguageConfigMap[kj].Languages[kj]
		if li.Weight != lj.Weight {
			return li.Weight < lj.Weight
		}
		return ki < kj
	})
	return keys
}

func (d *dumper) strs(s []string) []string {
	if s == nil {
		return nil
	}
	out := make([]string, len(s))
	for i, x := range s {
		out[i] = d.r(x)
	}
	return out
}

func boolKeys(m map[string]bool) []string {
	var out []string
	for _, k := range sortedKeys(m) {
		if m[k] {
			out = append(out, k+"=true")
		} else {
			out = append(out, k+"=false")
		}
	}
	return out
}

func (d *dumper) config(c *allconfig.Config) (map[string]any, error) {
	out := map[string]any{}
	j, err := d.json(c)
	if err != nil {
		return nil, err
	}
	out["json"] = j

	cc := c.C
	comp := map[string]any{
		"timeout":             int64(cc.Timeout),
		"baseURL":             d.r(cc.BaseURL.String()),
		"baseURLWithPath":     d.r(cc.BaseURL.WithPath),
		"baseURLBasePath":     cc.BaseURL.BasePath,
		"baseURLLiveReload":   d.r(cc.BaseURLLiveReload.String()),
		"serverInterface":     cc.ServerInterface,
		"defaultOutputFormat": cc.DefaultOutputFormat.Name,
		"disabledKinds":       boolKeys(cc.DisabledKinds),
		"disabledLanguages":   boolKeys(cc.DisabledLanguages),
		"ignoredLogs":         boolKeys(cc.IgnoredLogs),
		"mainSections":        cc.MainSections,
		"isMainSectionsSet":   cc.IsMainSectionsSet(),
		"clock":               cc.Clock.Format(time.RFC3339Nano),
		"clockIsZero":         cc.Clock.IsZero(),
	}
	kof := map[string]any{}
	for _, k := range sortedKeys(cc.KindOutputFormats) {
		var names []string
		for _, f := range cc.KindOutputFormats[k] {
			names = append(names, f.Name)
		}
		kof[k] = names
	}
	comp["kindOutputFormats"] = kof

	var titles []string
	for _, p := range titleProbes {
		titles = append(titles, cc.CreateTitle(p))
	}
	comp["createTitle"] = titles
	var ugly []bool
	for _, p := range uglyProbes {
		ugly = append(ugly, cc.IsUglyURLSection(p))
	}
	comp["isUglyURLSection"] = ugly
	var ign []bool
	for _, p := range ignoreProbes {
		ign = append(ign, cc.IgnoreFile(p))
	}
	comp["ignoreFile"] = ign
	comp["segments"] = segmentAnswers(cc.SegmentFilter)

	var hc []any
	for _, u := range urlProbes {
		row := map[string]any{"url": u, "for": cc.HTTPCache.For(u)}
		pc := cc.HTTPCache.PollConfigFor(u)
		row["poll"] = map[string]any{"disable": pc.Config.Disable, "low": int64(pc.Config.Low), "high": int64(pc.Config.High), "ok": pc.For != nil}
		hc = append(hc, row)
	}
	comp["httpCache"] = hc

	var kinds []any
	for _, k := range outputKinds {
		kinds = append(kinds, []any{k, c.IsKindEnabled(k)})
	}
	comp["isKindEnabled"] = kinds
	var ld []any
	for _, k := range sortedKeys(c.Languages) {
		ld = append(ld, []any{k, c.IsLangDisabled(k)})
	}
	ld = append(ld, []any{"xx", c.IsLangDisabled("xx")})
	comp["isLangDisabled"] = ld
	out["compiled"] = comp

	hashes := map[string]any{}
	if c.Imaging != nil {
		hashes["imaging"] = c.Imaging.SourceHash
	}
	if c.MediaTypes != nil {
		hashes["mediaTypes"] = c.MediaTypes.SourceHash
	}
	if c.OutputFormats != nil {
		hashes["outputFormats"] = c.OutputFormats.SourceHash
	}
	if c.ContentTypes != nil {
		hashes["contentTypes"] = c.ContentTypes.SourceHash
	}
	if c.Cascade != nil {
		hashes["cascade"] = c.Cascade.SourceHash
	}
	if c.Segments != nil {
		hashes["segments"] = c.Segments.SourceHash
	}
	if c.Menus != nil {
		hashes["menus"] = c.Menus.SourceHash
	}
	out["hashes"] = hashes

	// Values json.Marshal does not show.
	caches := map[string]any{}
	for _, k := range sortedKeys(c.Caches) {
		fc := c.Caches[k]
		caches[k] = map[string]any{"dirCompiled": d.r(fc.DirCompiled), "isResourceDir": fc.IsResourceDir}
	}
	out["cachesCompiled"] = caches
	out["cacheDirModules"] = d.r(c.Caches.CacheDirModules())
	var ordering []string
	for _, re := range c.Deployment.Ordering {
		ordering = append(ordering, re.String())
	}
	out["deploymentOrdering"] = ordering
	out["hlLinesParsed"] = c.Markup.Highlight.HL_lines_parsed
	out["autoHeadingIDType"] = c.Markup.Goldmark.Parser.AutoHeadingIDType

	return out, nil
}

func segmentAnswers(f segments.SegmentFilter) []any {
	var out []any
	for _, p := range segmentProbes {
		out = append(out, []bool{f.ShouldExcludeCoarse(p), f.ShouldExcludeFine(p)})
	}
	return out
}

func (d *dumper) configLang(confs *allconfig.Configs, c config.AllProvider) map[string]any {
	out := map[string]any{
		"lang":                           c.Language().Lang,
		"languagePrefix":                 c.LanguagePrefix(),
		"baseURL":                        d.r(c.BaseURL().String()),
		"isMultihost":                    c.IsMultihost(),
		"isMultilingual":                 c.IsMultilingual(),
		"environment":                    c.Environment(),
		"workingDir":                     d.r(c.WorkingDir()),
		"defaultContentLanguage":         c.DefaultContentLanguage(),
		"defaultContentLanguageInSubdir": c.DefaultContentLanguageInSubdir(),
		"timeout":                        int64(c.Timeout()),
		"staticDirs":                     d.strs(c.StaticDirs()),
		"summaryLength":                  c.SummaryLength(),
		"canonifyURLs":                   c.CanonifyURLs(),
		"disablePathToLower":             c.DisablePathToLower(),
		"removePathAccents":              c.RemovePathAccents(),
		"buildDrafts":                    c.BuildDrafts(),
		"buildFuture":                    c.BuildFuture(),
		"buildExpired":                   c.BuildExpired(),
		"enableEmoji":                    c.EnableEmoji(),
		"noBuildLock":                    c.NoBuildLock(),
		"quiet":                          c.Quiet(),
		"running":                        c.Running(),
		"watching":                       c.Watching(),
		"pagination":                     c.Pagination(),
		"ignoredLogs":                    boolKeys(c.IgnoredLogs()),
	}
	dirs := func(cd config.CommonDirs) map[string]any {
		// The legacy component dirs are deprecated in Go but still decoded (and mapped to
		// mounts): the port must reproduce them.
		return map[string]any{
			"themesDir":    d.r(cd.ThemesDir),
			"publishDir":   d.r(cd.PublishDir),
			"resourceDir":  d.r(cd.ResourceDir),
			"workingDir":   d.r(cd.WorkingDir),
			"cacheDir":     d.r(cd.CacheDir),
			"contentDir":   d.r(cd.ContentDir),   //nolint:staticcheck // legacy dir under test
			"dataDir":      d.r(cd.DataDir),      //nolint:staticcheck // legacy dir under test
			"layoutDir":    d.r(cd.LayoutDir),    //nolint:staticcheck // legacy dir under test
			"i18nDir":      d.r(cd.I18nDir),      //nolint:staticcheck // legacy dir under test
			"archeTypeDir": d.r(cd.ArcheTypeDir), //nolint:staticcheck // legacy dir under test
			"assetDir":     d.r(cd.AssetDir),     //nolint:staticcheck // legacy dir under test
		}
	}
	out["dirs"] = dirs(c.Dirs())
	out["dirsBase"] = dirs(c.DirsBase())
	var ugly []bool
	for _, p := range uglyProbes {
		ugly = append(ugly, c.IsUglyURLs(p))
	}
	out["isUglyURLs"] = ugly
	var ld []any
	for _, l := range confs.Languages {
		ld = append(ld, []any{l.Lang, c.IsLangDisabled(l.Lang)})
	}
	out["isLangDisabled"] = ld
	lang := c.Language()
	out["language"] = map[string]any{
		"lang":              lang.Lang,
		"languageName":      lang.LanguageName,
		"languageCode":      lang.LanguageCode(),
		"title":             lang.Title,
		"languageDirection": lang.LanguageDirection,
		"weight":            lang.Weight,
		"disabled":          lang.Disabled,
		"location":          langs.GetLocation(lang).String(),
	}
	return out
}

func (d *dumper) module(m modules.Module) (map[string]any, error) {
	out := map[string]any{
		"path":            d.r(m.Path()),
		"dir":             d.r(m.Dir()),
		"version":         m.Version(),
		"vendor":          m.Vendor(),
		"isGoMod":         m.IsGoMod(),
		"watch":           m.Watch(),
		"configFilenames": d.strs(m.ConfigFilenames()),
	}
	if m.Owner() != nil {
		out["owner"] = d.r(m.Owner().Path())
	} else {
		out["owner"] = ""
	}
	var mounts []any
	for _, mnt := range m.Mounts() {
		// The port keeps the normalized lists (what hugofs reads) for the collected
		// mounts; the raw `any` values are compared through the config's Module section.
		inc, err := d.json(types.ToStringSlicePreserveString(mnt.IncludeFiles))
		if err != nil {
			return nil, err
		}
		exc, err := d.json(types.ToStringSlicePreserveString(mnt.ExcludeFiles))
		if err != nil {
			return nil, err
		}
		mounts = append(mounts, map[string]any{
			"source": d.r(mnt.Source), "target": mnt.Target, "lang": mnt.Lang,
			"includeFiles": inc, "excludeFiles": exc, "disableWatch": mnt.DisableWatch,
		})
	}
	out["mounts"] = mounts
	cj, err := d.json(m.Config())
	if err != nil {
		return nil, err
	}
	out["config"] = cj
	if m.Cfg() != nil {
		cfgj, err := d.json(m.Cfg().Get(""))
		if err != nil {
			return nil, err
		}
		out["cfg"] = cfgj
	}
	return out, nil
}
