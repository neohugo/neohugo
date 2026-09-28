// Command arm64build builds an nh-resource-transformers oracle (Wave B task
// T15) for linux/arm64 without cgo, so that it can run under
// qemu-aarch64-static on a Linux x86_64 machine (the minifier fixtures must
// come from arm64 Go: the golden build ran on darwin/arm64, whose compiler
// fuses x*y+z into FMA). It is the nh-tplfuncs arm64build (Wave B task T18)
// with the package to build as a flag.
//
// The oracles never encode WebP or compile SCSS, but hugolib imports the cgo
// packages of gowebp and golibsass. The build uses a temporary copy of go.mod
// (-modfile) that replaces both modules with local copies with a `!cgo` stub
// file added to their internal packages (nothing in the repository or the
// module cache is modified):
//
//	go run ./tools/go-oracle/nh-resource-transformers/arm64build \
//	    -pkg ./tools/go-oracle/nh-resource-transformers/transformers -o /tmp/transformers.arm64
//	qemu-aarch64-static /tmp/transformers.arm64 -root .
package main

import (
	"flag"
	"io/fs"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

const webpStub = `//go:build !cgo

package libwebp

import (
	"errors"
	"image"
	"io"

	"github.com/bep/gowebp/libwebp/webpoptions"
)

// Encode is a stub (the arm64 oracle build has no cgo).
func Encode(w io.Writer, src image.Image, o webpoptions.EncodingOptions) error {
	return errors.New("libwebp stub")
}

// ConvertToNRGBA is a stub.
func ConvertToNRGBA(src image.Image) *image.NRGBA { panic("libwebp stub") }
`

const sassStub = `//go:build !cgo

package libsass

import "errors"

type (
	SassCompiler    *struct{ c int }
	SassContext     *struct{ x int }
	SassDataContext *struct{ d int }
	SassOptions     *struct{ o int }
	ImportResolver  func(currPath string, prevPath string) (newPath string, body string, resolved bool)
)

func stub() { panic("libsass stub") }

func AddImportResolver(opts SassOptions, resolver ImportResolver) int { stub(); return 0 }
func DeleteImportResolver(i int) error                                 { return errors.New("libsass stub") }
func SassCompilerExecute(compiler SassCompiler)                        { stub() }
func SassCompilerParse(compiler SassCompiler)                          { stub() }
func SassContextGetErrorJSON(ctx SassContext) string                   { stub(); return "" }
func SassContextGetErrorStatus(ctx SassContext) int                    { stub(); return 0 }
func SassContextGetOutputString(ctx SassContext) string                { stub(); return "" }
func SassContextGetSourceMapString(ctx SassContext) string             { stub(); return "" }
func SassDataContextGetContext(ctx SassDataContext) SassContext        { stub(); return nil }
func SassDataContextGetOptions(ctx SassDataContext) SassOptions        { stub(); return nil }
func SassDataContextSetOptions(ctx SassDataContext, opt SassOptions)   { stub() }
func SassDeleteCompiler(compiler SassCompiler)                         { stub() }
func SassDeleteDataContext(ctx SassDataContext)                        { stub() }
func SassMakeDataCompiler(ctx SassDataContext) SassCompiler            { stub(); return nil }
func SassMakeDataContext(s string) SassDataContext                     { stub(); return nil }
func SassOptionGetSourceMapFile(opts SassOptions) string               { stub(); return "" }
func SassOptionSetIncludePath(o SassOptions, s string)                 { stub() }
func SassOptionSetInputPath(o SassOptions, s string)                   { stub() }
func SassOptionSetOmitSourceMapURL(o SassOptions, b bool)              { stub() }
func SassOptionSetOutputPath(o SassOptions, s string)                  { stub() }
func SassOptionSetOutputStyle(o SassOptions, i int)                    { stub() }
func SassOptionSetPrecision(o SassOptions, i int)                      { stub() }
func SassOptionSetSourceComments(o SassOptions, b bool)                { stub() }
func SassOptionSetSourceMapContents(o SassOptions, b bool)             { stub() }
func SassOptionSetSourceMapEmbed(o SassOptions, b bool)                { stub() }
func SassOptionSetSourceMapFile(o SassOptions, s string)               { stub() }
func SassOptionSetSourceMapRoot(o SassOptions, s string)               { stub() }
func SassToScss(src string) string                                     { stub(); return "" }
`

func goEnv(env []string, key string) string {
	cmd := exec.Command("go", "env", key)
	cmd.Env = env
	b, err := cmd.Output()
	if err != nil {
		log.Fatalf("go env %s: %v", key, err)
	}
	return strings.TrimSpace(string(b))
}

// copyDir copies the module directory src to dst (files made writable).
func copyDir(src, dst string) error {
	return filepath.WalkDir(src, func(p string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		rel, err := filepath.Rel(src, p)
		if err != nil {
			return err
		}
		target := filepath.Join(dst, rel)
		if d.IsDir() {
			return os.MkdirAll(target, 0o755)
		}
		b, err := os.ReadFile(p)
		if err != nil {
			return err
		}
		return os.WriteFile(target, b, 0o644)
	})
}

func main() {
	out := flag.String("o", "oracle.arm64", "output binary")
	pkg := flag.String("pkg", "./tools/go-oracle/nh-resource-transformers/transformers", "package to build")
	flag.Parse()

	env := append(os.Environ(), "GOOS=linux", "GOARCH=arm64", "CGO_ENABLED=0")
	modcache := goEnv(env, "GOMODCACHE")
	gomod := goEnv(env, "GOMOD")

	dir, err := os.MkdirTemp("", "t15-arm64build")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(dir) }()

	// A copy of go.mod (and go.sum) that replaces the two cgo modules with
	// local copies that have the stub file added.
	modb, err := os.ReadFile(gomod)
	if err != nil {
		log.Fatal(err)
	}
	sumb, err := os.ReadFile(strings.TrimSuffix(gomod, ".mod") + ".sum")
	if err != nil {
		log.Fatal(err)
	}
	mod := string(modb)
	for _, s := range []struct{ path, version, pkg, src string }{
		{"github.com/bep/gowebp", "v0.3.0", "internal/libwebp", webpStub},
		{"github.com/bep/golibsass", "v1.2.0", "internal/libsass", sassStub},
	} {
		local := filepath.Join(dir, filepath.Base(s.path))
		if err := copyDir(filepath.Join(modcache, s.path+"@"+s.version), local); err != nil {
			log.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(local, s.pkg, "zz_t15_stub.go"), []byte(s.src), 0o644); err != nil {
			log.Fatal(err)
		}
		mod += "\nreplace " + s.path + " " + s.version + " => " + local + "\n"
	}
	modfile := filepath.Join(dir, "go.mod")
	if err := os.WriteFile(modfile, []byte(mod), 0o644); err != nil {
		log.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "go.sum"), sumb, 0o644); err != nil {
		log.Fatal(err)
	}

	cmd := exec.Command("go", "build", "-modfile", modfile, "-o", *out, *pkg)
	cmd.Env = env
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		log.Fatal(err)
	}
}
