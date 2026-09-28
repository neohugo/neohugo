package tsupport

import (
	"embed"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// The files the overlay adds to neohugo packages. They are not Go files of
// this package (they belong to tplimpl, hugolib, the html/template fork and
// the overlaid tsupport); `go run -overlay` compiles them into those
// packages.
//
//go:embed overlay/*.txt
var overlayFiles embed.FS

// patch replaces Old (which must occur exactly once) with New in a copy of
// File, and appends Append to the copy.
type patch struct {
	File   string
	Old    string
	New    string
	Append string
}

// The recording hooks: the lookups get named results and a deferred hook
// call (the query is logged before init mutates it, with its Consider func
// wrapped); renderAndWritePage can be switched off.
var patches = []patch{
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "func (s *TemplateStore) LookupPagesLayout(q TemplateQuery) *TemplInfo {",
		New: `func (s *TemplateStore) LookupPagesLayout(q TemplateQuery) (oracleRes *TemplInfo) {
	if h := OracleHooks; h != nil && h.Pages != nil {
		var ol *OracleConsiderLog
		q, ol = OracleWrapConsider(q)
		defer func(q0 TemplateQuery) { h.Pages(s, q0, ol, oracleRes) }(q)
	}`,
	},
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "func (s *TemplateStore) LookupPartial(pth string) *TemplInfo {",
		New: `func (s *TemplateStore) LookupPartial(pth string) (oracleRes *TemplInfo) {
	if h := OracleHooks; h != nil && h.Partial != nil {
		defer func() { h.Partial(s, pth, oracleRes) }()
	}`,
	},
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "func (s *TemplateStore) LookupShortcodeByName(name string) *TemplInfo {",
		New: `func (s *TemplateStore) LookupShortcodeByName(name string) (oracleRes *TemplInfo) {
	if h := OracleHooks; h != nil && h.ByName != nil {
		defer func(n0 string) { h.ByName(s, n0, oracleRes) }(name)
	}`,
	},
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "func (s *TemplateStore) LookupShortcode(q TemplateQuery) (*TemplInfo, error) {",
		New: `func (s *TemplateStore) LookupShortcode(q TemplateQuery) (oracleRes *TemplInfo, oracleErr error) {
	if h := OracleHooks; h != nil && h.Shortcode != nil {
		var ol *OracleConsiderLog
		q, ol = OracleWrapConsider(q)
		defer func(q0 TemplateQuery) { h.Shortcode(s, q0, ol, oracleRes, oracleErr) }(q)
	}`,
	},
	// The lookups iterate Go maps (nodeKey and TemplateDescriptor keys), and
	// bestMatch.isBetter is order dependent when two candidates tie on w1
	// but not on w2/w3, so Go's answer to such a query changes from run to
	// run. The oracle pins Go to one legal order: the keys sorted like the
	// Rust store's BTreeMaps (OracleReverseOrder reverses it, to find the
	// order-dependent queries).
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "\t\tfor d2, vv := range v {\n\t\t\tweight := s.dh.compareDescriptors(CategoryBaseof, false, d1, d2)",
		New:  "\t\tfor _, d2 := range oracleDescKeys(v) {\n\t\t\tvv := v[d2]\n\t\t\tweight := s.dh.compareDescriptors(CategoryBaseof, false, d1, d2)",
	},
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "\t\tfor _, vv := range v {\n\t\t\tif vv.category != CategoryBaseof {",
		New:  "\t\tfor _, vk := range oracleNodeKeys(v) {\n\t\t\tvv := v[vk]\n\t\t\tif vv.category != CategoryBaseof {",
	},
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "\t\tfor k, vv := range v {\n\t\t\tbest.candidates = append(best.candidates, vv)",
		New:  "\t\tfor _, k := range oracleDescKeys(v) {\n\t\t\tvv := v[k]\n\t\t\tbest.candidates = append(best.candidates, vv)",
	},
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "\tfor k, vv := range v {\n\t\tif vv.category != category {",
		New:  "\tfor _, k := range oracleNodeKeys(v) {\n\t\tvv := v[k]\n\t\tif vv.category != category {",
	},
	{
		File: "tpl/tplimpl/templatestore.go",
		Old:  "\t\tfor k, vv := range v {\n\t\t\tif vv.category != q.Category {",
		New:  "\t\tfor _, k := range oracleNodeKeys(v) {\n\t\t\tvv := v[k]\n\t\t\tif vv.category != q.Category {",
	},
	{
		File: "hugolib/site.go",
		Old:  "func (s *Site) renderAndWritePage(statCounter *uint64, name string, targetPath string, p *pageState, d any, templ *tplimpl.TemplInfo) error {",
		New: `func (s *Site) renderAndWritePage(statCounter *uint64, name string, targetPath string, p *pageState, d any, templ *tplimpl.TemplInfo) error {
	if OracleSkipWrite {
		return nil
	}`,
	},
}

// Added files: overlay source -> destination (relative to the module root).
var added = map[string]string{
	"overlay/tplimpl.go.txt":      "tpl/tplimpl/zz_nh_tplimpl_oracle.go",
	"overlay/hugolib.go.txt":      "hugolib/zz_nh_tplimpl_oracle.go",
	"overlay/htmltemplate.go.txt": "tpl/internal/go_templates/htmltemplate/zz_nh_tplimpl_oracle.go",
	"overlay/tsupport.go.txt":     "tools/go-oracle/nh-tplimpl/tsupport/zz_nh_tplimpl_oracle.go",
}

// ChildEnv is set in the environment of the overlaid child process.
const ChildEnv = "NH_TPLIMPL_ORACLE_CHILD"

// IsChild reports whether this process is the overlaid child.
func IsChild() bool {
	return os.Getenv(ChildEnv) == "1"
}

// RunOverlaid runs the oracle package pkg (e.g.
// "./tools/go-oracle/nh-tplimpl/store") again with `go run -overlay`: the
// patched copies of the neohugo sources and the added files are generated
// from the current sources on every run, so a stale anchor is an error.
func RunOverlaid(root, pkg string, args []string) error {
	absRoot, err := filepath.Abs(root)
	if err != nil {
		return err
	}
	tmp, err := os.MkdirTemp("", "nh-tplimpl-oracle-overlay")
	if err != nil {
		return err
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	replace := map[string]string{}
	content := map[string]string{}
	var order []string
	for _, p := range patches {
		src := filepath.Join(absRoot, filepath.FromSlash(p.File))
		s, ok := content[src]
		if !ok {
			b, err := os.ReadFile(src)
			if err != nil {
				return err
			}
			s = string(b)
			order = append(order, src)
		}
		if n := strings.Count(s, p.Old); n != 1 {
			return fmt.Errorf("overlay: %s: anchor %q found %d times", p.File, p.Old, n)
		}
		s = strings.Replace(s, p.Old, p.New, 1) + p.Append
		content[src] = s
	}
	for i, src := range order {
		dst := filepath.Join(tmp, fmt.Sprintf("patch%d.go", i))
		if err := os.WriteFile(dst, []byte(content[src]), 0o644); err != nil {
			return err
		}
		replace[src] = dst
	}
	i := 0
	for from, to := range added {
		b, err := overlayFiles.ReadFile(from)
		if err != nil {
			return err
		}
		dst := filepath.Join(tmp, fmt.Sprintf("added%d.go", i))
		i++
		if err := os.WriteFile(dst, b, 0o644); err != nil {
			return err
		}
		replace[filepath.Join(absRoot, filepath.FromSlash(to))] = dst
	}

	ob, err := json.Marshal(map[string]any{"Replace": replace})
	if err != nil {
		return err
	}
	overlay := filepath.Join(tmp, "overlay.json")
	if err := os.WriteFile(overlay, ob, 0o644); err != nil {
		return err
	}

	cmdArgs := append([]string{"run", "-overlay", overlay, pkg}, args...)
	cmd := exec.Command("go", cmdArgs...)
	cmd.Dir = absRoot
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Env = append(os.Environ(), ChildEnv+"=1", "HUGO_NUMWORKERMULTIPLIER=1")
	return cmd.Run()
}

// OutDir resolves the -out flag against the -root flag (an absolute out is
// used as is).
func OutDir(root, out string) string {
	if filepath.IsAbs(out) {
		return out
	}
	return filepath.Join(root, out)
}
