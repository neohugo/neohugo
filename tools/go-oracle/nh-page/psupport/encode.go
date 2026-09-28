package psupport

import (
	"fmt"
	"reflect"

	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-helpers/hsupport"
)

// MediaTypeDump encodes every field of a media.Type (the unexported
// mimeSuffix through reflection).
func MediaTypeDump(m media.Type) map[string]any {
	return map[string]any{
		"type":        m.Type,
		"mainType":    m.MainType,
		"subType":     m.SubType,
		"delimiter":   m.Delimiter,
		"suffix":      m.FirstSuffix.Suffix,
		"fullSuffix":  m.FirstSuffix.FullSuffix,
		"mimeSuffix":  reflect.ValueOf(m).FieldByName("mimeSuffix").String(),
		"suffixesCSV": m.SuffixesCSV,
	}
}

// FormatDump encodes every field of an output.Format, and whether it equals
// the built-in formats CreateTargetPaths compares with.
func FormatDump(f output.Format) map[string]any {
	return map[string]any{
		"name":           f.Name,
		"mediaType":      MediaTypeDump(f.MediaType),
		"path":           f.Path,
		"baseName":       f.BaseName,
		"rel":            f.Rel,
		"protocol":       f.Protocol,
		"isPlainText":    f.IsPlainText,
		"isHTML":         f.IsHTML,
		"noUgly":         f.NoUgly,
		"ugly":           f.Ugly,
		"notAlternative": f.NotAlternative,
		"root":           f.Root,
		"permalinkable":  f.Permalinkable,
		"weight":         f.Weight,
		"is404":          f == output.HTTPStatus404HTMLFormat,
		"isSitemap":      f == output.SitemapFormat,
		"isRobots":       f == output.RobotsTxtFormat,
	}
}

// PathSpecDump encodes the configuration a PathSpec's URL and path helpers
// read (the Rust test rebuilds the PathSpec from it) and the Go results it
// checks the rebuilt one against.
func PathSpecDump(conf config.AllProvider, p *helpers.PathSpec) map[string]any {
	b := conf.BaseURL()
	var langs []string
	for _, l := range conf.Languages() {
		langs = append(langs, l.Lang)
	}
	return map[string]any{
		"lang":                   conf.Language().Lang,
		"languages":              langs,
		"baseURL":                b.String(),
		"languagePrefix":         conf.LanguagePrefix(),
		"canonifyURLs":           conf.CanonifyURLs(),
		"disablePathToLower":     conf.DisablePathToLower(),
		"removePathAccents":      conf.RemovePathAccents(),
		"isMultihost":            conf.IsMultihost(),
		"getBasePath":            p.GetBasePath(false),
		"getBasePathRel":         p.GetBasePath(true),
		"targetLanguageBasePath": p.GetTargetLanguageBasePath(),
	}
}

// PathTable collects the *paths.Path values of the descriptors. Each is
// stored as the input Go parsed it from (its unnormalized path) with its
// component and type; it is parsed again here with the recording parser, so
// the Rust test can replay every PathParser callback, and the accessors
// CreateTargetPaths reads are recorded for both.
type PathTable struct {
	Rec     *hsupport.Recorder
	pp      *paths.PathParser
	Entries []map[string]any
	index   map[*paths.Path]int
	// Mismatches counts paths whose re-parse differs from the original.
	Mismatches int
}

// NewPathTable returns a table over the site's path parser.
func NewPathTable(pp *paths.PathParser) *PathTable {
	rec := hsupport.NewRecorder()
	return &PathTable{Rec: rec, pp: rec.Wrap(pp), index: map[*paths.Path]int{}}
}

func pathChecks(p *paths.Path) map[string]any {
	m := map[string]any{
		"isBundle":             p.IsBundle(),
		"base":                 p.Base(),
		"containerDir":         p.ContainerDir(),
		"baseNameNoIdentifier": p.BaseNameNoIdentifier(),
		"path":                 p.Path(),
		"type":                 p.Type().String(),
		"section":              p.Section(),
	}
	if u := p.Unnormalized(); u != nil {
		m["unBaseNameNoIdentifier"] = u.BaseNameNoIdentifier()
	}
	return m
}

// Add returns the index of p in the table (-1 for nil).
func (t *PathTable) Add(p *paths.Path) int {
	if p == nil {
		return -1
	}
	if i, ok := t.index[p]; ok {
		return i
	}
	// An unnormalized path (disablePathToLower descriptors) has no
	// unnormalized path of its own: it is the unnormalized form of the path
	// parsed from its own string.
	isUnnormalized := p.Unnormalized() == nil
	input := p.Path()
	if !isUnnormalized {
		input = p.Unnormalized().Path()
	}
	re := t.pp.Parse(p.Component(), input)
	retype := false
	if re.Type() != p.Type() {
		re = re.ForType(p.Type())
		retype = true
	}
	if isUnnormalized {
		re = re.Unnormalized()
	}
	want := pathChecks(p)
	got := pathChecks(re)
	if fmt.Sprint(want) != fmt.Sprint(got) {
		t.Mismatches++
	}
	e := map[string]any{
		"component":      p.Component(),
		"input":          input,
		"type":           p.Type().String(),
		"retype":         retype,
		"unnormalizedOf": isUnnormalized,
		"checks":         want,
	}
	t.index[p] = len(t.Entries)
	t.Entries = append(t.Entries, e)
	return len(t.Entries) - 1
}

// Describe returns the recorded parser for the fixture header.
func (t *PathTable) Describe(pp *paths.PathParser) map[string]any {
	return t.Rec.Describe(pp)
}
