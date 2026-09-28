// Command urls is the Go oracle for common/urls in crates/nh-common (Wave B
// task T02): NewBaseURLFromString and every BaseURL accessor, WithProtocol
// and WithPort.
//
//	go run ./tools/go-oracle/nh-common/urls [-root .] [-out crates/nh-common/tests/fixtures/urls]
//
// Inputs: the seeksnack baseURL (docs/rust-port/specs/architecture-core-data/
// config-en.json), the base URLs of Hugo's tests, adversarial URLs (schemes,
// opaque forms, ports, userinfo, IPv6, escapes, invalid escapes, unicode) and
// every corpus string of ../corpus. Output: baseurl.json.gz. Nothing here
// depends on the platform: linux/amd64 and linux/arm64 (qemu) produce the same
// bytes.
package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"sort"

	"github.com/neohugo/neohugo/common/urls"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/corpus"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
)

var baseURLs = []string{
	"https://seeksnack.com/", "https://seeksnack.com", "http://example.com/", "http://example.com",
	"http://example.com/sub", "http://example.com/sub/", "mailto:hugo@rules.com", "/", "",
	"http://localhost:1313/", "http://localhost:1313", "https://example.org:8443/a/b/",
	"http://[::1]:1313/", "http://[fe80::1%25en0]:8080/x", "http://user:pass@example.com/",
	"http://user@example.com:99999999999999999999/", "http://example.com:0/", "http://example.com:/",
	"http://example.com/%E0%B8%81/", "http://example.com/ก/", "http://example.com/a%20b",
	"http://example.com/a b", "http://example.com/%ff", "http://example.com/%zz",
	"http://example.com/?q=1", "http://example.com/#frag", "http://example.com/p?q#f",
	"webcal://example.com/cal", "file:///tmp/x", "//example.com/", "//example.com",
	"example.com", "example.com/sub", "https://ภาษาไทย.com/", "http://a%41.com/",
	"http://%e0%b8%81.com/", "HTTP://EXAMPLE.COM/UPPER", "https://seeksnack.com/th/",
	"a:b", ":", "http:", "http:/", "http://", "http:///", "mailto:", "mailto:%40", "urn:isbn:0451450523",
	"http://example.com/a/../b/./c/", "http://example.com//double//", "https://seeksnack.com/\x7f",
	"http://ex ample.com/", "http://example.com:80:80/",
}

var protocols = []string{"webcal://", "webcal", "mailto:", "https://", "http:", "", "://", ":", "a b://"}

var ports = []int{1313, 0, -1, 65536}

func str(s string) any { return goval.Str(s) }

func dump(b urls.BaseURL) map[string]any {
	return map[string]any{
		"String":                  str(b.String()),
		"WithPath":                str(b.WithPath),
		"WithPathNoTrailingSlash": str(b.WithPathNoTrailingSlash),
		"WithoutPath":             str(b.WithoutPath),
		"BasePath":                str(b.BasePath),
		"BasePathNoTrailingSlash": str(b.BasePathNoTrailingSlash),
		"Path":                    str(b.Path()),
		"HostURL":                 str(b.HostURL()),
		"Port":                    b.Port(),
		"URL":                     str(b.URL().String()),
	}
}

func result(b urls.BaseURL, err error) map[string]any {
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	return map[string]any{"ok": dump(b)}
}

func try(f func() any) (out any) {
	defer func() {
		if r := recover(); r != nil {
			out = map[string]any{"panic": fmt.Sprint(r)}
		}
	}()
	return f()
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "crates/nh-common/tests/fixtures/urls", "output directory")
	flag.Parse()

	set := map[string]bool{}
	full := map[string]bool{} // inputs that also get WithProtocol and WithPort
	for _, s := range baseURLs {
		set[s] = true
		full[s] = true
	}
	b, err := os.ReadFile(filepath.Join(*root, "docs/rust-port/specs/architecture-core-data/config-en.json"))
	if err != nil {
		log.Fatal(err)
	}
	var m map[string]any
	if err := json.Unmarshal(b, &m); err != nil {
		log.Fatal(err)
	}
	set[m["baseurl"].(string)] = true
	full[m["baseurl"].(string)] = true
	strs, err := corpus.Strings(*root)
	if err != nil {
		log.Fatal(err)
	}
	for _, s := range strs {
		set[s] = true
	}
	var inputs []string
	for s := range set {
		inputs = append(inputs, s)
	}
	sort.Strings(inputs)

	var cases []map[string]any
	for _, s := range inputs {
		c := map[string]any{"in": str(s)}
		bu, err := urls.NewBaseURLFromString(s)
		c["new"] = result(bu, err)
		if err == nil && full[s] {
			var wp []any
			for _, p := range protocols {
				wp = append(wp, try(func() any { return result(bu.WithProtocol(p)) }))
			}
			c["withProtocol"] = wp
			var wport []any
			for _, p := range ports {
				wport = append(wport, try(func() any { return result(bu.WithPort(p)) }))
			}
			c["withPort"] = wport
		}
		cases = append(cases, c)
	}

	header := map[string]any{"protocols": protocols, "ports": ports}
	p := filepath.Join(*out, "baseurl.json.gz")
	if err := goval.WriteCasesGz(p, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d cases", p, len(cases))
}
