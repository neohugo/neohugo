// Package osupport holds what the nh-transform and nh-publisher oracles
// (Wave B task T07) share: running an oracle again with recording hooks
// patched into neohugo packages through `go build -overlay` (optionally as a
// linux/arm64 binary under qemu, for the float-sensitive minifier outputs),
// the overlay patches of publisher/ and minifiers/, and the gzipped JSON-lines
// fixture writer with its byte encoding.
package osupport

import (
	"bufio"
	"compress/gzip"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"unicode/utf8"
)

// Patch inserts code into a neohugo source file for an overlaid build: After
// must occur exactly once in the file; Insert is placed right after it and
// Append at the end of the file.
type Patch struct {
	File   string // relative to the module root
	After  string
	Insert string
	Append string
}

// ChildEnv is set in the environment of the overlaid child process.
const ChildEnv = "NH_T07_ORACLE_CHILD"

// ArchEnv selects the architecture of the overlaid child: "arm64" builds it
// for linux/arm64 and runs it under qemu-aarch64-static (the golden build's
// FMA behaviour, docs/rust-port/archive/HANDOFF-old-port.md §3); anything else runs it natively.
const ArchEnv = "NH_T07_ORACLE_ARCH"

// CCEnv and CXXEnv name the C/C++ cross compilers for an arm64 child that
// needs cgo (e.g. wrappers around `zig cc -target aarch64-linux-musl`; see
// crates/gift/PORTING.md); without them the child is built with CGO_ENABLED=0.
const (
	CCEnv  = "NH_T07_ORACLE_CC"
	CXXEnv = "NH_T07_ORACLE_CXX"
)

// IsChild reports whether this process is the overlaid child.
func IsChild() bool {
	return os.Getenv(ChildEnv) == "1"
}

// Arch is the architecture the (child) oracle computes on.
func Arch() string {
	return runtime.GOOS + "/" + runtime.GOARCH
}

// RunOverlaid builds the oracle package pkg (e.g.
// "./tools/go-oracle/nh-publisher/site") with `go build -overlay`: every patch
// is applied to a copy of its file, and hookFile is added to the oracle
// package as hookName (it installs the recording hooks). It then runs the
// binary with args. The copies are generated from the current sources on
// every run, so the patches never go stale silently: a missing anchor is an
// error.
func RunOverlaid(root, pkg string, patches []Patch, hookName, hookFile string, args []string) error {
	absRoot, err := filepath.Abs(root)
	if err != nil {
		return err
	}
	tmp, err := os.MkdirTemp("", "nh-t07-oracle-overlay")
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
		if p.After != "" {
			if n := strings.Count(s, p.After); n != 1 {
				return fmt.Errorf("overlay: %s: anchor %q found %d times", p.File, p.After, n)
			}
			s = strings.Replace(s, p.After, p.After+p.Insert, 1)
		}
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
	if hookFile != "" {
		hookDst := filepath.Join(tmp, "hook.go")
		if err := os.WriteFile(hookDst, []byte(hookFile), 0o644); err != nil {
			return err
		}
		replace[filepath.Join(absRoot, filepath.FromSlash(strings.TrimPrefix(pkg, "./")), hookName)] = hookDst
	}

	ob, err := json.Marshal(map[string]any{"Replace": replace})
	if err != nil {
		return err
	}
	overlay := filepath.Join(tmp, "overlay.json")
	if err := os.WriteFile(overlay, ob, 0o644); err != nil {
		return err
	}

	arm64 := os.Getenv(ArchEnv) == "arm64"
	bin := filepath.Join(tmp, "oracle")
	buildArgs := []string{"build", "-overlay", overlay, "-o", bin}
	cc := os.Getenv(CCEnv)
	if arm64 && cc != "" {
		// A cgo package (the publisher imports libwebp/libsass through resources):
		// a static binary from a C cross compiler (zig cc -target aarch64-linux-musl).
		buildArgs = append(buildArgs, "-ldflags", "-linkmode external -extldflags -static")
	}
	build := exec.Command("go", append(buildArgs, pkg)...)
	build.Dir = absRoot
	build.Stdout = os.Stdout
	build.Stderr = os.Stderr
	build.Env = os.Environ()
	if arm64 {
		build.Env = append(build.Env, "GOOS=linux", "GOARCH=arm64")
		if cc != "" {
			build.Env = append(build.Env, "CGO_ENABLED=1", "CC="+cc, "CXX="+os.Getenv(CXXEnv))
		} else {
			build.Env = append(build.Env, "CGO_ENABLED=0")
		}
	}
	if err := build.Run(); err != nil {
		return err
	}

	var cmd *exec.Cmd
	if arm64 && runtime.GOARCH != "arm64" {
		cmd = exec.Command("qemu-aarch64-static", append([]string{bin}, args...)...)
	} else {
		cmd = exec.Command(bin, args...)
	}
	cmd.Dir = absRoot
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Env = append(os.Environ(), ChildEnv+"=1", "HUGO_NUMWORKERMULTIPLIER=1")
	return cmd.Run()
}

// B encodes bytes for a fixture: a JSON string when they are valid UTF-8
// (Go's encoder would replace invalid bytes), else {"b64": base64}.
func B(b []byte) any {
	if utf8.Valid(b) {
		return string(b)
	}
	return map[string]string{"b64": base64.StdEncoding.EncodeToString(b)}
}

// S is B for a string.
func S(s string) any {
	return B([]byte(s))
}

// Writer writes gzipped JSON lines (one record per line).
type Writer struct {
	f  *os.File
	gz *gzip.Writer
	bw *bufio.Writer
	n  int
}

// Create opens a fixture file (its directory is created). The gzip header
// carries no name and no time, so a regenerated file is byte-identical.
func Create(path string) (*Writer, error) {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return nil, err
	}
	f, err := os.Create(path)
	if err != nil {
		return nil, err
	}
	gz, err := gzip.NewWriterLevel(f, gzip.BestCompression)
	if err != nil {
		_ = f.Close()
		return nil, err
	}
	return &Writer{f: f, gz: gz, bw: bufio.NewWriterSize(gz, 1<<20)}, nil
}

// Write writes one record.
func (w *Writer) Write(rec any) error {
	b, err := json.Marshal(rec)
	if err != nil {
		return err
	}
	w.n++
	if _, err := w.bw.Write(b); err != nil {
		return err
	}
	return w.bw.WriteByte('\n')
}

// N is the number of records written.
func (w *Writer) N() int {
	return w.n
}

// Close flushes and closes the file.
func (w *Writer) Close() error {
	if err := w.bw.Flush(); err != nil {
		return err
	}
	if err := w.gz.Close(); err != nil {
		return err
	}
	return w.f.Close()
}

// PanicString runs f and returns the text of a panic ("" when f returned).
func PanicString(f func()) (msg string) {
	defer func() {
		if r := recover(); r != nil {
			msg = fmt.Sprint(r)
			if msg == "" {
				msg = "panic"
			}
		}
	}()
	f()
	return ""
}
