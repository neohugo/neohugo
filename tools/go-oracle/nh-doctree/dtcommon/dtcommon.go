// Package dtcommon holds what the nh-doctree oracles share: the key sets (tree keys derived from
// this repository's sites and templates the way Hugo derives them, keys quoted in the specs, and
// synthetic keys that stress the radix tree), a test shifter that mirrors Hugo's
// contentNodeShifter, a deterministic random source and the fixture writer.
package dtcommon

import (
	"bytes"
	"compress/gzip"
	"encoding/json"
	"fmt"
	"io/fs"
	"os"
	"path"
	"path/filepath"
	"regexp"
	"sort"
	"strings"
	"unicode/utf8"

	"github.com/neohugo/neohugo/common/paths"
	"github.com/neohugo/neohugo/hugofs/files"
	"github.com/neohugo/neohugo/hugolib/doctree"
	"github.com/neohugo/neohugo/media"
	"github.com/neohugo/neohugo/output"
)

// NumLanguages is the size of the language dimension (en, th as in seeksnack).
const NumLanguages = 2

// Rand is a splitmix64 generator (reproducible independently of Go's math/rand).
type Rand struct{ s uint64 }

// NewRand returns a generator seeded with seed.
func NewRand(seed uint64) *Rand { return &Rand{s: seed} }

// Uint64 returns the next value.
func (r *Rand) Uint64() uint64 {
	r.s += 0x9E3779B97F4A7C15
	z := r.s
	z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9
	z = (z ^ (z >> 27)) * 0x94D049BB133111EB
	return z ^ (z >> 31)
}

// Intn returns a value in [0, n).
func (r *Rand) Intn(n int) int { return int(r.Uint64() % uint64(n)) }

// Pick returns a random element of s.
func Pick[T any](r *Rand, s []T) T { return s[r.Intn(len(s))] }

// WriteJSONGz writes v as JSON, gzip-compressed (best compression, no header name or time).
func WriteJSONGz(file string, v any) error {
	b, err := json.Marshal(v)
	if err != nil {
		return err
	}
	var buf bytes.Buffer
	zw, err := gzip.NewWriterLevel(&buf, gzip.BestCompression)
	if err != nil {
		return err
	}
	if _, err := zw.Write(b); err != nil {
		return err
	}
	if err := zw.Close(); err != nil {
		return err
	}
	if err := os.MkdirAll(filepath.Dir(file), 0o755); err != nil {
		return err
	}
	return os.WriteFile(file, buf.Bytes(), 0o644)
}

// ---------------------------------------------------------------------------
// The test shifter (hugolib/content_map_page.go contentNodeShifter, with *TV standing in for
// *pageState (Res false) and *resourceSource (Res true), Pages for contentNodeIs and Ress for
// resourceSources).

// Node is a tree value.
type Node interface{ isNode() }

// TV is one language version of a page or resource.
type TV struct {
	ID     string
	Lang   int
	Res    bool // a resource (else a page)
	IsPage bool // a resource that is a page (content resource)
	Branch bool // a branch page
}

// Pages is Hugo's contentNodeIs.
type Pages []*TV

// Ress is Hugo's resourceSources.
type Ress []*TV

func (*TV) isNode()   {}
func (Pages) isNode() {}
func (Ress) isNode()  {}

// Def is a TV as written to the fixtures: [id, lang, flags] with flags bit 0 = Res,
// bit 1 = IsPage, bit 2 = Branch.
func (v *TV) Def() []any {
	flags := 0
	if v.Res {
		flags |= 1
	}
	if v.IsPage {
		flags |= 2
	}
	if v.Branch {
		flags |= 4
	}
	return []any{v.ID, v.Lang, flags}
}

// Repr is the fixture spelling of a node: "-" for nil, the id, or P[...]/R[...] for the arrays.
func Repr(n Node) string {
	switch v := n.(type) {
	case nil:
		return "-"
	case *TV:
		if v == nil {
			return "-"
		}
		return v.ID
	case Pages:
		return "P[" + reprs(v) + "]"
	case Ress:
		return "R[" + reprs(v) + "]"
	}
	panic(fmt.Sprintf("unknown type %T", n))
}

func reprs(vs []*TV) string {
	var b strings.Builder
	for i, v := range vs {
		if i > 0 {
			b.WriteByte(',')
		}
		b.WriteString(Repr(v))
	}
	return b.String()
}

// Shifter mirrors contentNodeShifter.
type Shifter struct{}

var _ doctree.Shifter[Node] = (*Shifter)(nil)

// Delete mirrors contentNodeShifter.Delete.
func (s *Shifter) Delete(n Node, dimension doctree.Dimension) (Node, bool, bool) {
	lidx := dimension[0]
	switch v := n.(type) {
	case Pages:
		deleted := v[lidx]
		wasDeleted := deleted != nil
		v[lidx] = nil
		isEmpty := true
		for _, vv := range v {
			if vv != nil {
				isEmpty = false
				break
			}
		}
		return deleted, wasDeleted, isEmpty
	case Ress:
		deleted := v[lidx]
		wasDeleted := deleted != nil
		v[lidx] = nil
		isEmpty := true
		for _, vv := range v {
			if vv != nil {
				isEmpty = false
				break
			}
		}
		return deleted, wasDeleted, isEmpty
	case *TV:
		if lidx != v.Lang {
			return nil, false, false
		}
		return v, true, true
	default:
		panic(fmt.Sprintf("unknown type %T", n))
	}
}

// Shift mirrors contentNodeShifter.Shift.
func (s *Shifter) Shift(n Node, dimension doctree.Dimension, exact bool) (Node, bool, doctree.DimensionFlag) {
	lidx := dimension[0]
	accuracy := doctree.DimensionLanguage
	switch v := n.(type) {
	case Pages:
		if len(v) == 0 {
			panic("empty contentNodeIs")
		}
		vv := v[lidx]
		if vv != nil {
			return vv, true, accuracy
		}
		return nil, false, 0
	case Ress:
		vv := v[lidx]
		if vv != nil {
			return vv, true, doctree.DimensionLanguage
		}
		if exact {
			return nil, false, 0
		}
		// For non content resources, pick the first match.
		for _, vv := range v {
			if vv != nil {
				if vv.IsPage {
					return nil, false, 0
				}
				return vv, true, 0
			}
		}
	case *TV:
		if v.Res {
			if v.Lang == lidx {
				return v, true, doctree.DimensionLanguage
			}
			if !v.IsPage && !exact {
				return v, true, 0
			}
		} else if v.Lang == lidx {
			return n, true, doctree.DimensionLanguage
		}
	default:
		panic(fmt.Sprintf("unknown type %T", n))
	}
	return nil, false, 0
}

// ForEeachInDimension mirrors contentNodeShifter.ForEeachInDimension.
func (s *Shifter) ForEeachInDimension(n Node, d int, f func(Node) bool) {
	if d != doctree.DimensionLanguage.Index() {
		panic("only language dimension supported")
	}
	switch vv := n.(type) {
	case Pages:
		for _, v := range vv {
			if v != nil {
				if f(v) {
					return
				}
			}
		}
	default:
		f(vv)
	}
}

// InsertInto mirrors contentNodeShifter.InsertInto.
func (s *Shifter) InsertInto(old, new Node, dimension doctree.Dimension) (Node, Node, bool) {
	langi := dimension[doctree.DimensionLanguage.Index()]
	switch vv := old.(type) {
	case *TV:
		newp, ok := new.(*TV)
		if !ok || newp.Res != vv.Res {
			panic(fmt.Sprintf("unknown type %T", new))
		}
		if vv.Lang == newp.Lang && newp.Lang == langi {
			return new, vv, true
		}
		if vv.Res {
			rs := make(Ress, NumLanguages)
			rs[vv.Lang] = vv
			rs[langi] = newp
			return rs, vv, false
		}
		is := make(Pages, NumLanguages)
		is[vv.Lang] = vv
		is[langi] = newp
		return is, old, false
	case Pages:
		oldv := vv[langi]
		vv[langi] = new.(*TV)
		return vv, oldv, oldv != nil
	case Ress:
		oldv := vv[langi]
		vv[langi] = new.(*TV)
		return vv, oldv, oldv != nil
	default:
		panic(fmt.Sprintf("unknown type %T", old))
	}
}

// Insert mirrors contentNodeShifter.Insert.
func (s *Shifter) Insert(old, new Node) (Node, Node, bool) {
	newp, ok := new.(*TV)
	if !ok {
		panic(fmt.Sprintf("unknown type %T", new))
	}
	switch vv := old.(type) {
	case *TV:
		if newp.Res != vv.Res {
			panic(fmt.Sprintf("unknown type %T", new))
		}
		if vv.Lang == newp.Lang {
			return new, vv, true
		}
		if vv.Res {
			rs := make(Ress, NumLanguages)
			rs[newp.Lang] = newp
			rs[vv.Lang] = vv
			return rs, vv, false
		}
		is := make(Pages, NumLanguages)
		is[newp.Lang] = new.(*TV)
		is[vv.Lang] = vv
		return is, old, false
	case Pages:
		oldp := vv[newp.Lang]
		vv[newp.Lang] = newp
		return vv, oldp, oldp != nil
	case Ress:
		oldp := vv[newp.Lang]
		vv[newp.Lang] = newp
		return vv, oldp, oldp != nil
	default:
		panic(fmt.Sprintf("unknown type %T", old))
	}
}

// IsBranch is the predicate Hugo passes to LongestPrefix (page__tree.go).
func IsBranch(n Node) bool {
	v, ok := n.(*TV)
	return ok && v != nil && v.Branch
}

// ---------------------------------------------------------------------------
// Key sets.

// ContentFile is a file of a content directory, parsed the way Hugo's content map sees it.
type ContentFile struct {
	Site   string // the site's name
	Rel    string // the path in the content component ("/posts/p1.md")
	Key    string // the tree key (paths.Path.Base(), home "" as in cleanKey)
	Lang   int
	Page   bool // a page (treePages) or a resource (treeResources)
	IsPage bool // a resource that is a page (content file inside a leaf bundle)
	Branch bool
}

// PathParser mirrors allconfig's ContentPathParser for the languages en and th and the default
// media, content and output format configuration.
func PathParser() *paths.PathParser {
	ct, err := media.DecodeContentTypes(nil, media.DefaultTypes)
	if err != nil {
		panic(err)
	}
	return &paths.PathParser{
		LanguageIndex:  map[string]int{"en": 0, "th": 1},
		IsLangDisabled: func(string) bool { return false },
		IsContentExt:   ct.Config.IsContentSuffix,
		IsOutputFormat: func(name, ext string) bool {
			if name == "" {
				return false
			}
			if of, ok := output.DefaultFormats.GetByName(name); ok {
				if ext != "" && !of.MediaType.HasSuffix(ext) {
					return false
				}
				return true
			}
			return false
		},
	}
}

// contentDirs are this repository's content directories: (site, directory, language).
var contentDirs = []struct {
	site, dir string
	lang      int
}{
	{"docs", "docs/content/en", 0},
	{"testsite", "hugolib/testsite/content", 0},
	{"testsite", "hugolib/testsite/content_nn", 1},
	{"skeleton", "create/skeletons/theme/content", 0},
}

// ContentFiles parses every file of the repository's content directories.
func ContentFiles(root string) ([]ContentFile, error) {
	pp := PathParser()
	var out []ContentFile
	for _, cd := range contentDirs {
		dir := filepath.Join(root, cd.dir)
		var rels []string
		err := filepath.WalkDir(dir, func(p string, d fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if d.IsDir() {
				return nil
			}
			rel, err := filepath.Rel(dir, p)
			if err != nil {
				return err
			}
			rels = append(rels, "/"+filepath.ToSlash(rel))
			return nil
		})
		if err != nil {
			return nil, err
		}
		// Leaf bundles: directories with an index file.
		leaf := map[string]bool{}
		for _, rel := range rels {
			if pi := pp.Parse(files.ComponentFolderContent, rel); pi.Type() == paths.TypeLeaf {
				leaf[path.Dir(rel)] = true
			}
		}
		inLeaf := func(rel string) bool {
			for d := path.Dir(rel); ; d = path.Dir(d) {
				if leaf[d] {
					return true
				}
				if d == "/" {
					return false
				}
			}
		}
		for _, rel := range rels {
			pi := pp.Parse(files.ComponentFolderContent, rel)
			if pi.Type() == paths.TypeContentData {
				// Content adapters (.gotmpl) create pages when executed; not keys by themselves.
				continue
			}
			cf := ContentFile{Site: cd.site, Rel: rel, Lang: cd.lang}
			if l := pi.Lang(); l != "" {
				cf.Lang = map[string]int{"en": 0, "th": 1}[l]
			}
			switch pi.Type() {
			case paths.TypeLeaf, paths.TypeBranch:
				cf.Page = true
				cf.Branch = pi.Type() == paths.TypeBranch
			case paths.TypeContentSingle:
				if inLeaf(rel) {
					cf.IsPage = true
				} else {
					cf.Page = true
				}
			}
			cf.Key = pi.Base()
			if cf.Key == "/" {
				cf.Key = ""
			}
			out = append(out, cf)
		}
	}
	return out, nil
}

// TemplateKeys returns template store keys: for every template of the embedded templates, the
// docs site and the skeleton theme, the template path itself ("/_partials/opengraph.html") and
// the store key Hugo's TemplateStore derives from it (toKeyCategoryAndDescriptor: the directory,
// the path without identifiers for partials, the shortcode base, "/_default" and "/_markup"
// trimmed).
func TemplateKeys(root string) ([]string, error) {
	pp := PathParser()
	dirs := []string{
		"tpl/tplimpl/embedded/templates",
		"docs/layouts",
		"create/skeletons/theme/layouts",
	}
	var keys []string
	for _, d := range dirs {
		dir := filepath.Join(root, d)
		var rels []string
		err := filepath.WalkDir(dir, func(p string, de fs.DirEntry, err error) error {
			if err != nil {
				return err
			}
			if de.IsDir() {
				return nil
			}
			rel, err := filepath.Rel(dir, p)
			if err != nil {
				return err
			}
			rels = append(rels, "/"+filepath.ToSlash(rel))
			return nil
		})
		if err != nil {
			return nil, err
		}
		sort.Strings(rels)
		for _, rel := range rels {
			pi := pp.Parse(files.ComponentFolderLayouts, rel)
			keys = append(keys, rel, templateStoreKey(pi))
		}
	}
	return keys, nil
}

func templateStoreKey(p *paths.Path) string {
	k1 := p.Dir()
	switch p.Type() {
	case paths.TypePartial:
		k1 = p.PathNoIdentifier()
	case paths.TypeShortcode:
		k1 = p.PathNoIdentifier()
		k1 = strings.Split(k1, "/_shortcodes/")[0]
		k1 = paths.AddLeadingSlash(k1)
		if k1 == "/" {
			k1 = ""
		} else {
			k1 = paths.TrimTrailing(k1)
		}
	}
	k1 = strings.TrimPrefix(k1, "/_default")
	if k1 == "/" {
		k1 = ""
	}
	if p.Type() == paths.TypeMarkup {
		k1 = strings.TrimSuffix(k1, "/_markup")
	}
	return k1
}

var specKeyRe = regexp.MustCompile("`(/[^`\\s*<>…]*)`")

// SpecKeys returns the paths quoted in docs/rust-port/specs/*.md (seeksnack tree keys, output
// paths and template keys), plus their lower-cased, space-to-hyphen form.
func SpecKeys(root string) ([]string, error) {
	matches, err := filepath.Glob(filepath.Join(root, "docs/rust-port/specs/*.md"))
	if err != nil {
		return nil, err
	}
	sort.Strings(matches)
	var keys []string
	for _, m := range matches {
		b, err := os.ReadFile(m)
		if err != nil {
			return nil, err
		}
		for _, sm := range specKeyRe.FindAllStringSubmatch(string(b), -1) {
			k := sm[1]
			if strings.HasPrefix(k, "/opt/") || strings.HasPrefix(k, "/private/") || strings.HasPrefix(k, "/Users/") {
				continue
			}
			keys = append(keys, k, paths.NormalizePathStringBasic(k))
		}
	}
	return keys, nil
}

// SyntheticKeys returns keys that stress the radix tree: shared prefixes, "/" vs "-" vs "."
// neighbours, trailing slashes, unicode, empty segments, deep paths and keys that differ only
// after a prefix match.
func SyntheticKeys() []string {
	keys := []string{
		"", "/", "//", "/a", "/a/", "/a//b", "/a/b", "/a/b/", "/a/b/c", "/a/bc", "/a/b-c", "/a/b.c",
		"/a/b_c", "/a/b c", "/a-b", "/a.b", "/a_b", "/ab", "/abc", "/abd", "/ab/c", "/a/b/c/d/e/f/g",
		"/b", "/b/a", "/ba", "/b-a", "/b.a", "/b/", "/b//", "/0", "/9", "/A", "/Z", "/~", "/!",
		"/tags", "/tags/a", "/tags/ab", "/tags/a-b", "/tags/a.b", "/tags/a/b", "/tagsx", "/tags-x",
		"/tags.x", "/tag", "/t", "/brands/lay's", "/brands/lays", "/brands/lays-", "/brands/lay",
		"/posts", "/posts/p1", "/posts/p1/i.jpg", "/posts/p1.md", "/posts/p10", "/posts/p1-x",
		"/posts/p2", "/posts/sub", "/posts/sub/p3", "/posts/sub/p3/a/b/c.png",
		"/ขนม", "/ขนม/ขนมปัง", "/ขนม/ขนมปังกรอบ", "/ขนม-ไทย", "/ขนมไทย", "/ข", "/日本/語", "/日本語",
		"/é", "/é", "/ß", "/ss", "/Ω", "/ω", "/😀", "/😀/😁", "/a/😀",
		"/x/y/z", "/x/y/z/", "/x/yz", "/x/y.z", "/x/y-z", "/xy", "/x-y",
	}
	deep := ""
	for i := 0; i < 40; i++ {
		deep += fmt.Sprintf("/d%d", i%3)
		keys = append(keys, deep)
	}
	// Siblings that share long prefixes and differ in one late byte.
	long := "/" + strings.Repeat("common-", 12)
	for _, c := range []string{"a", "b", "a/x", "a-x", "a.x", "ab", "", "/"} {
		keys = append(keys, long+c)
	}
	// Every single-byte label under one node (many edges: Go's edge-slice growth).
	for c := 0x21; c < 0x7f; c++ {
		keys = append(keys, "/w/"+string(rune(c)))
	}
	return keys
}

// Dedupe returns keys without duplicates, keeping the first occurrence.
func Dedupe(keys []string) []string {
	seen := map[string]bool{}
	var out []string
	for _, k := range keys {
		if !utf8.ValidString(k) {
			panic("invalid UTF-8 key")
		}
		if !seen[k] {
			seen[k] = true
			out = append(out, k)
		}
	}
	return out
}

// DropLastRune returns s without its last rune.
func DropLastRune(s string) string {
	_, size := utf8.DecodeLastRuneInString(s)
	return s[:len(s)-size]
}
