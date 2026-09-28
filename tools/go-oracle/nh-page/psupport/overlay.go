package psupport

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// Patch inserts recording code into a neohugo source file for an overlaid
// build: After must occur exactly once in the file; Insert is placed right
// after it and Append at the end of the file.
type Patch struct {
	File   string // relative to the module root
	After  string
	Insert string
	Append string
}

// ChildEnv is set in the environment of the overlaid child process.
const ChildEnv = "NH_PAGE_ORACLE_CHILD"

// IsChild reports whether this process is the overlaid child.
func IsChild() bool {
	return os.Getenv(ChildEnv) == "1"
}

// RunOverlaid runs the oracle package pkg (e.g.
// "./tools/go-oracle/nh-page/paths") again with `go run -overlay`: every
// patch is applied to a copy of its file, and hookFile is added to the oracle
// package as hookName (it installs the recording hooks). The copies are
// generated from the current sources on every run, so the patches never go
// stale silently: a missing anchor is an error.
func RunOverlaid(root, pkg string, patches []Patch, hookName, hookFile string, args []string) error {
	absRoot, err := filepath.Abs(root)
	if err != nil {
		return err
	}
	tmp, err := os.MkdirTemp("", "nh-page-oracle-overlay")
	if err != nil {
		return err
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	// Patches of the same file apply in order to one copy.
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
		if n := strings.Count(s, p.After); n != 1 {
			return fmt.Errorf("overlay: %s: anchor %q found %d times", p.File, p.After, n)
		}
		s = strings.Replace(s, p.After, p.After+p.Insert, 1)
		s += p.Append
		content[src] = s
	}
	for i, src := range order {
		dst := filepath.Join(tmp, fmt.Sprintf("patch%d.go", i))
		if err := os.WriteFile(dst, []byte(content[src]), 0o644); err != nil {
			return err
		}
		replace[src] = dst
	}
	hookDst := filepath.Join(tmp, "hook.go")
	if err := os.WriteFile(hookDst, []byte(hookFile), 0o644); err != nil {
		return err
	}
	replace[filepath.Join(absRoot, filepath.FromSlash(strings.TrimPrefix(pkg, "./")), hookName)] = hookDst

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
