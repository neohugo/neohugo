// Command getremote is the Go oracle of
// crates/nh-resource-transformers/tests/getremote.rs (Wave B task T15):
// resources.GetRemote without network.
//
// Part 1 (served): a local httptest server answers JSON, images, text, HTML,
// Content-Disposition file names, HEAD, POST with a body and headers, the
// `key` and `responseHeaders` options, media types the security config
// accepts, and error statuses (404, 403, 500 with retries). The real
// GetRemote runs against it twice with the synthetic site's config (the http
// transport dials the test server for every host, so the URLs, and with them
// the cache keys, do not depend on the port):
//
//  1. live, with a fresh getresource file cache in a temporary dir: the
//     entries Go writes are copied to the fixture dir (cache/<key>);
//  2. with a fresh client (new memory cache) over the same file cache and the
//     network disabled: this is what the Rust port must reproduce from the
//     recorded entries (a call whose response was not cached fails with a
//     network error in Go; the Rust port has no network code).
//
// Part 2 (seeksnack): the 51 getresource entries of the golden build
// (tools/rust-port/testdata/hugo_cache/seeksnack/filecache/getresource) are
// read through GetRemote. Their URLs contain the site's private API key, so
// each entry is placed under the key of a synthetic URL that names the entry
// (the Rust test places them the same way, under the key it computes).
//
//	GOTOOLCHAIN=go1.27.1 go run ./tools/go-oracle/nh-resource-transformers/getremote -root .
package main

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"log"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/common/hashing"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resource-transformers/rtsupport"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-resources/rsupport"
)

const date = "Sun, 27 Sep 2026 10:25:09 GMT"

// A tiny valid PNG (1x1).
var pngBytes = []byte("\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8\xcf\xc0\x00\x00\x03\x01\x01\x00\xc9\xfe\x92\xef\x00\x00\x00\x00IEND\xaeB`\x82")

func handler(w http.ResponseWriter, r *http.Request) {
	h := w.Header()
	h.Set("Date", date)
	h.Set("X-Custom", "custom-value")
	h.Add("X-Multi", "one")
	h.Add("X-Multi", "two")
	write := func(ct string, status int, body []byte) {
		if ct != "" {
			h.Set("Content-Type", ct)
		}
		w.WriteHeader(status)
		_, _ = w.Write(body)
	}
	echo := func() []byte {
		var keys []string
		for k := range r.Header {
			if k == "Accept-Encoding" || k == "Content-Length" {
				continue
			}
			keys = append(keys, k)
		}
		sort.Strings(keys)
		var b strings.Builder
		fmt.Fprintf(&b, "method=%s\npath=%s\nquery=%s\n", r.Method, r.URL.Path, r.URL.RawQuery)
		for _, k := range keys {
			fmt.Fprintf(&b, "%s=%s\n", k, strings.Join(r.Header[k], "|"))
		}
		buf := make([]byte, 1024)
		n, _ := r.Body.Read(buf)
		fmt.Fprintf(&b, "body=%s\n", buf[:n])
		return []byte(b.String())
	}
	switch r.URL.Path {
	case "/data.json", "/api/v1/items":
		write("application/json; charset=UTF-8", 200, []byte(`{"items":[1,2,3],"path":"`+r.URL.Path+`"}`))
	case "/nocontenttype.json":
		h["Content-Type"] = nil
		write("", 200, []byte(`{"x": 1}`))
	case "/image.png", "/image":
		write("image/png", 200, pngBytes)
	case "/image-as.js":
		write("application/octet-stream", 200, pngBytes)
	case "/text.txt":
		write("text/plain; charset=utf-8", 200, []byte("plain text\n"))
	case "/text.csv":
		write("text/plain", 200, []byte("a,b\n1,2\n"))
	case "/page":
		write("text/html; charset=utf-8", 200, []byte("<!doctype html><html><body><p>hi</p></body></html>"))
	case "/style":
		write("text/css", 200, []byte("body { color: red; }\n"))
	case "/download":
		h.Set("Content-Disposition", `attachment; filename="report.json"`)
		write("application/json", 200, []byte(`{"report":true}`))
	case "/download2":
		h.Set("Content-Disposition", `attachment; filename*=UTF-8''na%C3%AFve%20file.txt`)
		write("text/plain", 200, []byte("naive\n"))
	case "/vnd":
		write("application/vnd.t15+json; charset=utf-8", 200, []byte(`{"vnd":1}`))
	case "/vnd-unknown":
		write("application/vnd.other+json", 200, []byte(`{"vnd":2}`))
	case "/binary":
		write("application/x-unknown", 200, []byte{0, 1, 2, 3, 0xff, 0xfe})
	case "/echo", "/echo.txt":
		write("text/plain; charset=utf-8", 200, echo())
	case "/svg":
		write("image/svg+xml", 200, []byte(`<svg xmlns="http://www.w3.org/2000/svg"><rect width="1" height="1"/></svg>`))
	case "/etag.json":
		h.Set("ETag", `"abc123"`)
		write("application/json", 200, []byte(`{"etag":true}`))
	case "/nostore.json":
		h.Set("Cache-Control", "no-store")
		write("application/json", 200, []byte(`{"nostore":true}`))
	case "/forbidden":
		write("text/plain", 403, []byte("go away"))
	case "/teapot.json":
		write("application/json", 418, []byte(`{"teapot":true}`))
	case "/boom":
		write("text/plain", 500, []byte("server error"))
	case "/redirect":
		h.Set("Location", "/data.json")
		write("", 302, nil)
	default:
		write("text/plain", 404, []byte("not found"))
	}
}

type call struct {
	Name string `json:"name"`
	Args []any  `json:"args"`
}

func calls() []call {
	u := func(p string) any { return rtsupport.S("http://remote.example" + p) }
	m := func(kv ...any) any {
		mm := map[string]any{}
		for i := 0; i < len(kv); i += 2 {
			mm[kv[i].(string)] = kv[i+1]
		}
		return rtsupport.V(mm)
	}
	return []call{
		{"json", []any{u("/data.json")}},
		{"json-again", []any{u("/data.json")}},
		{"json-query", []any{u("/data.json?a=1&b=2")}},
		{"json-noext", []any{u("/api/v1/items")}},
		{"json-no-content-type", []any{u("/nocontenttype.json")}},
		{"image", []any{u("/image.png")}},
		{"image-noext", []any{u("/image")}},
		{"image-as-js", []any{u("/image-as.js")}},
		{"text", []any{u("/text.txt")}},
		{"text-csv", []any{u("/text.csv")}},
		{"html", []any{u("/page")}},
		{"css", []any{u("/style")}},
		{"svg", []any{u("/svg")}},
		{"disposition", []any{u("/download")}},
		{"disposition-rfc2231", []any{u("/download2")}},
		{"vnd-accepted", []any{u("/vnd")}},
		{"vnd-not-accepted", []any{u("/vnd-unknown")}},
		{"binary", []any{u("/binary")}},
		{"etag", []any{u("/etag.json")}},
		{"nostore", []any{u("/nostore.json")}},
		{"notfound", []any{u("/missing.json")}},
		{"forbidden", []any{u("/forbidden")}},
		{"teapot", []any{u("/teapot.json")}},
		{"boom", []any{u("/boom")}},
		{"redirect", []any{u("/redirect")}},
		{"echo-get", []any{u("/echo")}},
		{"echo-headers", []any{u("/echo"), m("headers", map[string]any{"X-Api-Key": "secret", "Accept": []string{"application/json", "text/plain"}})}},
		{"echo-user-agent", []any{u("/echo"), m("headers", map[string]any{"User-Agent": "custom-agent"})}},
		{"echo-post", []any{u("/echo"), m("method", "post", "body", `{"q":1}`, "headers", map[string]any{"Content-Type": "application/json"})}},
		{"echo-post-order", []any{u("/echo"), m("headers", map[string]any{"Content-Type": "application/json"}, "body", `{"q":1}`, "method", "post")}},
		{"echo-post-upper", []any{u("/echo"), m("METHOD", "POST", "Body", `{"q":1}`, "Headers", map[string]any{"Content-Type": "application/json"})}},
		{"echo-post-bytes", []any{u("/echo"), m("method", "POST", "body", []any{int(65), int(66)})}},
		{"head", []any{u("/data.json"), m("method", "HEAD")}},
		{"head-image", []any{u("/image.png"), m("method", "head")}},
		{"method-not-allowed", []any{u("/echo"), m("method", "DELETE")}},
		{"method-not-string", []any{u("/echo"), m("method", 42)}},
		{"key", []any{u("/echo.txt"), m("key", "v1")}},
		{"key-other-uri", []any{u("/data.json?other"), m("key", "v1")}},
		{"key-upper", []any{u("/echo.txt"), m("Key", "v2")}},
		{"key-int", []any{u("/echo.txt"), m("key", 7)}},
		{"response-headers", []any{u("/data.json"), m("responseHeaders", []string{"x-custom", "Content-Type", "X-Multi", "X-Missing"})}},
		{"response-headers-error", []any{u("/forbidden"), m("responseHeaders", []string{"X-Custom"})}},
		{"bad-options", []any{u("/data.json"), m("headers", "notamap")}},
		{"bad-options-type", []any{u("/data.json"), rtsupport.S("notamap")}},
		{"empty-options", []any{u("/data.json"), m()}},
		{"too-many-args", []any{u("/data.json"), m(), m()}},
		{"no-args", []any{}},
		{"bad-url", []any{rtsupport.S("http://remote.example/%zz")}},
		{"int-url", []any{rtsupport.V(42)}},
		{"relative", []any{rtsupport.S("/relative/data.json")}},
		{"ftp", []any{rtsupport.S("ftp://remote.example/data.json")}},
	}
}

// remoteResourceKeys is create.remoteResourceKeys (unexported; copied).
func remoteResourceKeys(uri string, optionsm map[string]any) (string, string) {
	var userKey string
	if key, k, found := maps.LookupEqualFold(optionsm, "key"); found {
		userKey = hashing.HashString(key)
		delete(optionsm, k)
	}
	optionsKey := hashing.HashString(uri, optionsm)
	if userKey == "" {
		userKey = optionsKey
	}
	return userKey, optionsKey
}

// keyVectors records the keys of the calls' (uri, options) pairs.
func keyVectors(cs []call) []any {
	var out []any
	for _, c := range cs {
		if len(c.Args) == 0 {
			continue
		}
		uri, ok := rtsupport.Dec(c.Args[0]).(string)
		if !ok {
			continue
		}
		var optionsm map[string]any
		if len(c.Args) > 1 {
			m, ok := rtsupport.Dec(c.Args[1]).(map[string]any)
			if !ok {
				continue
			}
			optionsm = m
		}
		userKey, optionsKey := remoteResourceKeys(uri, optionsm)
		out = append(out, map[string]any{"name": c.Name, "userKey": userKey, "optionsKey": optionsKey})
	}
	return out
}

type result struct {
	Name   string         `json:"name"`
	Args   []any          `json:"args"`
	Cached map[string]any `json:"cached"`
	Live   map[string]any `json:"live"`
}

func run(site *rsupport.Site, cs []call) []map[string]any {
	ns, err := rtsupport.Namespace(site, nil)
	if err != nil {
		log.Fatal(err)
	}
	r := &rtsupport.Runner{NS: ns, Vars: map[string]any{}}
	var out []map[string]any
	for _, c := range cs {
		res := r.Run(rtsupport.Case{Name: c.Name, Steps: []rtsupport.Step{{Op: "GetRemote", Args: c.Args}}})
		out = append(out, res[0].(map[string]any))
	}
	return out
}

func setTransport(dial func(ctx context.Context, network, addr string) (net.Conn, error)) {
	http.DefaultTransport = &http.Transport{DialContext: dial, DisableCompression: true}
}

func main() {
	root := flag.String("root", ".", "repository root")
	out := flag.String("out", "rust/testdata/oracle/resource-transformers/getremote", "fixture dir")
	flag.Parse()

	absRoot, err := filepath.Abs(*root)
	if err != nil {
		log.Fatal(err)
	}
	siteDir := filepath.Join(absRoot, "rust/testdata/oracle/resource-transformers/site")

	srv := httptest.NewServer(http.HandlerFunc(handler))
	addr := srv.Listener.Addr().String()
	var d net.Dialer
	setTransport(func(ctx context.Context, network, _ string) (net.Conn, error) {
		return d.DialContext(ctx, network, addr)
	})

	cs := calls()

	// 1. Live.
	live, err := rsupport.LoadSite(siteDir)
	if err != nil {
		log.Fatal(err)
	}
	liveRes := run(live, cs)
	srv.Close()
	if os.Getenv("T15_DEBUG") != "" {
		for i, c := range cs {
			log.Printf("%s: %v", c.Name, liveRes[i])
		}
	}

	cacheDir := filepath.Join(live.TmpDir, "cache", "site", "filecache", "getresource")
	entries, err := os.ReadDir(cacheDir)
	if err != nil {
		log.Fatal(err)
	}
	fixtureCache := filepath.Join(*out, "cache")
	if err := os.RemoveAll(fixtureCache); err != nil {
		log.Fatal(err)
	}
	if err := os.MkdirAll(fixtureCache, 0o755); err != nil {
		log.Fatal(err)
	}
	var keys []string
	for _, e := range entries {
		b, err := os.ReadFile(filepath.Join(cacheDir, e.Name()))
		if err != nil {
			log.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(fixtureCache, e.Name()), b, 0o644); err != nil {
			log.Fatal(err)
		}
		keys = append(keys, e.Name())
	}

	// 2. From the file cache only.
	setTransport(func(context.Context, string, string) (net.Conn, error) {
		return nil, errors.New("oracle: network disabled")
	})
	cached, err := rsupport.LoadSite(siteDir)
	if err != nil {
		log.Fatal(err)
	}
	cachedDir := filepath.Join(cached.TmpDir, "cache", "site", "filecache", "getresource")
	if err := os.MkdirAll(cachedDir, 0o755); err != nil {
		log.Fatal(err)
	}
	for _, k := range keys {
		b, err := os.ReadFile(filepath.Join(fixtureCache, k))
		if err != nil {
			log.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(cachedDir, k), b, 0o644); err != nil {
			log.Fatal(err)
		}
	}
	cachedRes := run(cached, cs)
	cachedAfter, err := os.ReadDir(cachedDir)
	if err != nil {
		log.Fatal(err)
	}
	if len(cachedAfter) != len(keys) {
		log.Fatalf("the cached run wrote %d entries", len(cachedAfter)-len(keys))
	}

	var results []result
	for i, c := range cs {
		results = append(results, result{Name: c.Name, Args: c.Args, Cached: cachedRes[i], Live: liveRes[i]})
	}

	// 3. The golden build's entries.
	seeksnackDir := filepath.Join(absRoot, "tools/rust-port/testdata/hugo_cache/seeksnack/filecache/getresource")
	golden, err := os.ReadDir(seeksnackDir)
	if err != nil {
		log.Fatal(err)
	}
	ss, err := rsupport.LoadSite(siteDir)
	if err != nil {
		log.Fatal(err)
	}
	ssDir := filepath.Join(ss.TmpDir, "cache", "site", "filecache", "getresource")
	if err := os.MkdirAll(ssDir, 0o755); err != nil {
		log.Fatal(err)
	}
	var ssCalls []call
	var ssKeys []string
	for _, e := range golden {
		uri := "https://www.googleapis.com/youtube/v3/videos?key=API_KEY&part=snippet,contentDetails,statistics&id=" + e.Name()
		key := hashing.HashString(uri, map[string]any(nil))
		b, err := os.ReadFile(filepath.Join(seeksnackDir, e.Name()))
		if err != nil {
			log.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(ssDir, key), b, 0o644); err != nil {
			log.Fatal(err)
		}
		ssCalls = append(ssCalls, call{Name: e.Name(), Args: []any{rtsupport.S(uri)}})
		ssKeys = append(ssKeys, key)
	}
	ssRes := run(ss, ssCalls)
	var seeksnack []map[string]any
	for i, c := range ssCalls {
		seeksnack = append(seeksnack, map[string]any{"entry": c.Name, "args": c.Args, "fileCacheKey": ssKeys[i], "result": ssRes[i]})
	}

	for _, s := range []*rsupport.Site{live, cached, ss} {
		s.Close()
	}

	if err := rsupport.WriteGz(filepath.Join(*out, "getremote.json.gz"), map[string]any{
		"cacheEntries": keys,
		"keys":         keyVectors(cs),
		"calls":        results,
		"seeksnack":    seeksnack,
	}); err != nil {
		log.Fatal(err)
	}
	log.Printf("getremote: %d calls, %d cache entries, %d seeksnack entries", len(results), len(keys), len(seeksnack))
}
