// Command structure is the Go structure oracle of the Rust rewrite
// (docs/rust-port/REWRITE_PLAN.md §6.4, §7.2): the real neohugo command line
// with recording hooks, which after the build dumps, for every (language,
// page, output format) Go rendered, the target file, the permalinks and the
// layout template and base template it chose, plus the alias files, the
// bundle resource URLs of every page, every page's output formats and the
// site's layout files with normalised (Hugo v0.146) names. The schema is
// documented in rust/testdata/golden/README.md.
//
// The hooks live in packages hugolib, tpl/tplimpl and resources; they are
// added with `go build -overlay` (the overlay_*.go.txt files, plus patched
// copies of four hugolib files, see patches.go), so the repository is never
// modified. Build the oracle binary once, from the repository root:
//
//	go run ./tools/go-oracle/structure -install tools/neohugo/bin/neohugo-structure
//
// then run it exactly like neohugo, with the dump file in the environment:
//
//	NH_STRUCTURE_OUT=structure.json neohugo-structure --clock 2026-09-27T12:00:00Z -d out
//
// tools/neohugo/oracle.sh does both (and builds the vanilla neohugo binary
// the manifests come from).
package main

import (
	_ "embed"
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/commands"
)

//go:embed overlay_hugolib.go.txt
var overlayHugolib string

//go:embed overlay_tplimpl.go.txt
var overlayTplimpl string

//go:embed overlay_resources.go.txt
var overlayResources string

// Set by the hook file the overlay adds to this package (hookFile).
var dump func() (map[string]any, error)

const hookFile = `package main

import "github.com/neohugo/neohugo/hugolib"

func init() {
	dump = hugolib.NHStructureDump
}
`

// EnvOut names the file the overlaid binary writes the dump to.
const EnvOut = "NH_STRUCTURE_OUT"

// Schema identifies the dump format (rust/testdata/golden/README.md).
const Schema = "neohugo-structure/1"

func main() {
	log.SetFlags(0)
	if dump != nil {
		runOracle()
		return
	}
	root := flag.String("root", ".", "the neohugo module root")
	install := flag.String("install", "", "build the oracle binary (with the overlay) to this path")
	vet := flag.Bool("vet", false, "run go vet on the overlaid packages instead")
	flag.Parse()
	var goArgs []string
	switch {
	case *vet:
		goArgs = []string{"vet", "./hugolib", "./tpl/tplimpl", "./resources", "./tools/go-oracle/structure"}
	case *install != "":
		out, err := filepath.Abs(*install)
		if err != nil {
			log.Fatal(err)
		}
		goArgs = []string{"build", "-trimpath", "-ldflags=-s -w", "-o", out, "./tools/go-oracle/structure"}
	default:
		log.Fatal("usage: go run ./tools/go-oracle/structure -install <binary> | -vet (see the package doc)")
	}
	if err := runOverlaid(*root, goArgs); err != nil {
		log.Fatal(err)
	}
}

// runOracle runs the neohugo command line with the arguments of this
// process and writes the dump of the build to $NH_STRUCTURE_OUT.
func runOracle() {
	out := os.Getenv(EnvOut)
	if out == "" {
		log.Fatalf("%s is not set (see tools/go-oracle/structure)", EnvOut)
	}
	if err := commands.Execute(os.Args[1:]); err != nil {
		log.Fatalf("Error: %s", err)
	}
	d, err := dump()
	if err != nil {
		log.Fatalf("structure: %s", err)
	}
	d["schema"] = Schema
	b, err := encode(d)
	if err != nil {
		log.Fatalf("structure: %s", err)
	}
	if err := os.WriteFile(out, b, 0o644); err != nil {
		log.Fatalf("structure: %s", err)
	}
}

// encode writes a JSON object with sorted keys: one member per line, and
// one element per line for arrays of objects, so the dumps diff well.
func encode(d map[string]any) ([]byte, error) {
	keys := make([]string, 0, len(d))
	for k := range d {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	var sb strings.Builder
	sb.WriteString("{\n")
	for i, k := range keys {
		kb, err := marshal(k)
		if err != nil {
			return nil, err
		}
		sb.Write(kb)
		sb.WriteString(": ")
		if list, ok := d[k].([]map[string]any); ok {
			if len(list) == 0 {
				sb.WriteString("[]")
			} else {
				sb.WriteString("[\n")
				for j, e := range list {
					eb, err := marshal(e)
					if err != nil {
						return nil, err
					}
					sb.Write(eb)
					if j < len(list)-1 {
						sb.WriteString(",")
					}
					sb.WriteString("\n")
				}
				sb.WriteString("]")
			}
		} else {
			vb, err := marshal(d[k])
			if err != nil {
				return nil, err
			}
			sb.Write(vb)
		}
		if i < len(keys)-1 {
			sb.WriteString(",")
		}
		sb.WriteString("\n")
	}
	sb.WriteString("}\n")
	return []byte(sb.String()), nil
}

// marshal is json.Marshal without HTML escaping.
func marshal(v any) ([]byte, error) {
	var sb strings.Builder
	enc := json.NewEncoder(&sb)
	enc.SetEscapeHTML(false)
	if err := enc.Encode(v); err != nil {
		return nil, err
	}
	return []byte(strings.TrimSuffix(sb.String(), "\n")), nil
}

// runOverlaid runs the go command (`go <args[0]> -overlay <file> <args[1:]…>`)
// in the module root with the overlay: the hook files added to hugolib,
// tpl/tplimpl, resources and this package, and the patched hugolib files.
func runOverlaid(root string, args []string) error {
	absRoot, err := filepath.Abs(root)
	if err != nil {
		return err
	}
	if _, err := os.Stat(filepath.Join(absRoot, "go.mod")); err != nil {
		return fmt.Errorf("%s is not the neohugo module root: %w", absRoot, err)
	}
	tmp, err := os.MkdirTemp("", "nh-structure-overlay")
	if err != nil {
		return err
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	replace := map[string]string{}
	n := 0
	add := func(rel, content string) error {
		dst := filepath.Join(tmp, fmt.Sprintf("f%d.go", n))
		n++
		if err := os.WriteFile(dst, []byte(content), 0o644); err != nil {
			return err
		}
		replace[filepath.Join(absRoot, filepath.FromSlash(rel))] = dst
		return nil
	}
	newFiles := [][2]string{
		{"hugolib/zz_nh_structure.go", overlayHugolib},
		{"tpl/tplimpl/zz_nh_structure.go", overlayTplimpl},
		{"resources/zz_nh_structure.go", overlayResources},
		{"tools/go-oracle/structure/zz_nh_hook.go", hookFile},
	}
	for _, f := range newFiles {
		if _, err := os.Stat(filepath.Join(absRoot, filepath.FromSlash(f[0]))); err == nil {
			return fmt.Errorf("overlay: %s exists", f[0])
		}
		if err := add(f[0], f[1]); err != nil {
			return err
		}
	}
	patched, err := applyPatches(absRoot, patches)
	if err != nil {
		return err
	}
	files := make([]string, 0, len(patched))
	for f := range patched {
		files = append(files, f)
	}
	sort.Strings(files)
	for _, f := range files {
		if err := add(f, patched[f]); err != nil {
			return err
		}
	}
	ob, err := json.Marshal(map[string]any{"Replace": replace})
	if err != nil {
		return err
	}
	overlay := filepath.Join(tmp, "overlay.json")
	if err := os.WriteFile(overlay, ob, 0o644); err != nil {
		return err
	}
	goArgs := append([]string{args[0], "-overlay", overlay}, args[1:]...)
	cmd := exec.Command("go", goArgs...)
	cmd.Dir = absRoot
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("go %s: %w", args[0], err)
	}
	return nil
}

// patch replaces Old, which must occur exactly once, with New in a copy of
// File (relative to the module root).
type patch struct {
	File, Old, New string
}

func applyPatches(root string, ps []patch) (map[string]string, error) {
	content := map[string]string{}
	for _, p := range ps {
		s, ok := content[p.File]
		if !ok {
			b, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(p.File)))
			if err != nil {
				return nil, err
			}
			s = string(b)
		}
		if c := strings.Count(s, p.Old); c != 1 {
			return nil, fmt.Errorf("overlay: %s: %q found %d times", p.File, p.Old, c)
		}
		content[p.File] = strings.Replace(s, p.Old, p.New, 1)
	}
	return content, nil
}
