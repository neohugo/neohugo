// Command httpcache is the Go oracle for the HTTP side of the getresource file
// cache in crates/nh-helpers (Wave B task T08): Go's http.ReadResponse (+
// io.ReadAll of the body) over httputil.DumpResponse files, the
// gohugoio/httpcache Transport.RoundTrip over a real filecache.Cache
// (AsHTTPCache), and cache/httpcache's DecodeConfig/Compile.
//
//	go run ./tools/go-oracle/nh-helpers/httpcache [-out crates/nh-helpers/tests/fixtures/httpcache]
//
// The getresource cache of the seeksnack site is not available, so the
// entries are made here with Go's own httputil.DumpResponse (and, for the
// round trips, written with the filecache writer): 200/301/304/404/500
// responses over HTTP/1.0, 1.1 and 2.0, chunked and non-chunked bodies,
// multi-valued and case-mixed headers, ETags, Content-Length mismatches,
// binary bodies and trailers; plus hand-written dumps (folded headers, LF
// line ends, bad status lines, bad header lines, bad chunk sizes, truncated
// entries) and every prefix of 12 dumps and seeded byte mutations of them.
// Each dump is read as the response to a GET and to a HEAD.
//
// The round trips cover AlwaysUseCachedResponse on and off, maxAge -1, 0 and
// 1h with entries 10 minutes and 2 hours old, request headers (Range,
// Cache-Control no-cache/max-age/only-if-cached/max-stale/min-fresh,
// stale-if-error, Vary'd headers, If-None-Match inputs) and response headers
// (Date far in the past or future, max-age, no-cache, Expires, ETag,
// Last-Modified, Vary, stale-if-error). The inner transport records the
// request and fails, as a build without network access does; a few cases let
// it answer, to pin down where the Rust port reports its explicit
// unsupported error.
//
// Output: httpcache.json.gz. Nothing here depends on the platform.
package main

import (
	"bufio"
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"flag"
	"fmt"
	"io"
	"log"
	"math/rand"
	"net/http"
	"net/http/httputil"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/gohugoio/httpcache"
	"github.com/neohugo/neohugo/cache/filecache"
	hhttpcache "github.com/neohugo/neohugo/cache/httpcache"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-common/goval"
	"github.com/neohugo/neohugo/tools/go-oracle/nh-helpers/hsupport"
	"github.com/spf13/afero"
)

func sha(b []byte) string {
	s := sha256.Sum256(b)
	return hex.EncodeToString(s[:])
}

func headerDump(h http.Header) any {
	if h == nil {
		return nil
	}
	keys := make([]string, 0, len(h))
	for k := range h {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	var out [][]any
	for _, k := range keys {
		var vs []any
		for _, v := range h[k] {
			vs = append(vs, hsupport.Str(v))
		}
		out = append(out, []any{k, vs})
	}
	if out == nil {
		return [][]any{}
	}
	return out
}

func respDump(r *http.Response) map[string]any {
	body, err := io.ReadAll(r.Body)
	var berr any
	if err != nil {
		berr = err.Error()
	}
	return map[string]any{
		"status":           hsupport.Str(r.Status),
		"statusCode":       r.StatusCode,
		"proto":            r.Proto,
		"major":            r.ProtoMajor,
		"minor":            r.ProtoMinor,
		"header":           headerDump(r.Header),
		"contentLength":    r.ContentLength,
		"transferEncoding": r.TransferEncoding,
		"close":            r.Close,
		"trailer":          headerDump(r.Trailer),
		"bodyLen":          len(body),
		"bodySha":          sha(body),
		"bodyErr":          berr,
	}
}

func readResp(dump []byte, method string) map[string]any {
	req, err := http.NewRequest(method, "https://example.com/x", nil)
	if err != nil {
		log.Fatal(err)
	}
	r, err := http.ReadResponse(bufio.NewReader(bytes.NewReader(dump)), req)
	if err != nil {
		return map[string]any{"err": err.Error()}
	}
	return map[string]any{"ok": respDump(r)}
}

type synthResp struct {
	proto        string
	major, minor int
	status       int
	header       map[string][]string
	body         []byte
	cl           int64 // -2 = len(body)
	te           []string
	trailer      map[string][]string
	close        bool
}

func (s synthResp) dump() ([]byte, error) {
	r := &http.Response{
		Status:           fmt.Sprintf("%d %s", s.status, http.StatusText(s.status)),
		StatusCode:       s.status,
		Proto:            s.proto,
		ProtoMajor:       s.major,
		ProtoMinor:       s.minor,
		Header:           http.Header{},
		TransferEncoding: s.te,
		Close:            s.close,
	}
	keys := make([]string, 0, len(s.header))
	for k := range s.header {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		r.Header[k] = s.header[k]
	}
	if s.trailer != nil {
		r.Trailer = http.Header{}
		for k, v := range s.trailer {
			r.Trailer[k] = v
		}
	}
	if s.body != nil {
		r.Body = io.NopCloser(bytes.NewReader(s.body))
	}
	r.ContentLength = s.cl
	if s.cl == -2 {
		r.ContentLength = int64(len(s.body))
	}
	return httputil.DumpResponse(r, true)
}

func randBytes(rng *rand.Rand, n int) []byte {
	b := make([]byte, n)
	for i := range b {
		b[i] = byte(rng.Intn(256))
	}
	return b
}

var youtubeJSON = []byte(`{
  "kind": "youtube#videoListResponse",
  "etag": "abc",
  "items": [
    {
      "kind": "youtube#video",
      "id": "iIsZs0m-BVU",
      "statistics": {"viewCount": "12345", "likeCount": "67"}
    }
  ],
  "pageInfo": {"totalResults": 1, "resultsPerPage": 1}
}
`)

func synthDumps(rng *rand.Rand) [][]byte {
	jsonHdr := map[string][]string{
		"Content-Type":  {"application/json; charset=UTF-8"},
		"Vary":          {"Origin", "X-Origin", "Referer"},
		"Date":          {"Sat, 27 Sep 2025 10:00:00 GMT"},
		"Cache-Control": {"private"},
		"Server":        {"scaffolding on HTTPServer2"},
		"X-Etags-1":     {"5f8d6c1b2a"},
		"X-Etags-2":     {"5f8d6c1b2a"},
		"Alt-Svc":       {`h3=":443"; ma=2592000,h3-29=":443"; ma=2592000`},
	}
	var specs []synthResp
	for _, p := range []struct {
		proto        string
		major, minor int
	}{{"HTTP/2.0", 2, 0}, {"HTTP/1.1", 1, 1}, {"HTTP/1.0", 1, 0}} {
		for _, st := range []int{200, 301, 304, 404, 500, 204} {
			specs = append(specs,
				synthResp{proto: p.proto, major: p.major, minor: p.minor, status: st, header: jsonHdr, body: youtubeJSON, cl: -1},
				synthResp{proto: p.proto, major: p.major, minor: p.minor, status: st, header: jsonHdr, body: youtubeJSON, cl: -2},
				synthResp{proto: p.proto, major: p.major, minor: p.minor, status: st, header: nil, body: nil, cl: 0},
				synthResp{proto: p.proto, major: p.major, minor: p.minor, status: st, header: jsonHdr, body: youtubeJSON, cl: -1, te: []string{"chunked"}},
			)
		}
		specs = append(specs,
			synthResp{proto: p.proto, major: p.major, minor: p.minor, status: 200, header: map[string][]string{
				"Set-Cookie": {"a=1", "b=2"}, "x-lower-case": {"v"}, "X-MiXeD": {"1", "2", "3"},
				"Etag": {`"xyz"`}, "Last-Modified": {"Mon, 02 Jan 2006 15:04:05 GMT"}, "Content-Type": {"image/png"},
			}, body: randBytes(rng, 3000), cl: -2},
			synthResp{proto: p.proto, major: p.major, minor: p.minor, status: 200, header: map[string][]string{"Content-Type": {"text/plain"}},
				body: []byte("a\r\nb\nc\x00d\r\n\r\n"), cl: -1, te: []string{"chunked"}, trailer: map[string][]string{"X-Trail": {"t1"}}},
			synthResp{proto: p.proto, major: p.major, minor: p.minor, status: 200, header: map[string][]string{"Connection": {"close"}},
				body: []byte("closing body"), cl: -1, close: true},
			synthResp{proto: p.proto, major: p.major, minor: p.minor, status: 200, header: map[string][]string{"Connection": {"keep-alive"}, "Pragma": {"no-cache"}},
				body: []byte("pragma"), cl: -2},
			synthResp{proto: p.proto, major: p.major, minor: p.minor, status: 200, body: randBytes(rng, 9000), cl: -1, te: []string{"chunked"}},
		)
	}
	var out [][]byte
	for _, s := range specs {
		b, err := s.dump()
		if err != nil {
			continue
		}
		out = append(out, b)
	}
	return out
}

var rawDumps = []string{
	"",
	"HTTP/1.1",
	"HTTP/1.1 200 OK",
	"HTTP/1.1 200 OK\r\n",
	"HTTP/1.1 200 OK\r\n\r\n",
	"HTTP/1.1 200 OK\r\n\r\nbody without length",
	"HTTP/1.0 200 OK\r\n\r\nbody until eof",
	"HTTP/2.0 200 OK\r\n\r\nh2 body",
	"HTTP/1.1 200 OK\nContent-Length: 3\n\nabcdef",
	"HTTP/1.1 200\r\nContent-Length: 2\r\n\r\nok",
	"HTTP/1.1  200  OK\r\nContent-Length: 0\r\n\r\n",
	"HTTP/1.1 20 OK\r\n\r\n",
	"HTTP/1.1 2000 OK\r\n\r\n",
	"HTTP/1.1 abc OK\r\n\r\n",
	"HTTP/1.1 -12 Neg\r\n\r\n",
	"HTTP/1.1 +12 Plus\r\n\r\n",
	"HTTP/1.1 099 Low\r\n\r\n",
	"HTTP/9 200 OK\r\n\r\n",
	"HTTP/1.x 200 OK\r\n\r\n",
	"HTTP/0.9 200 OK\r\n\r\nzero nine",
	"HTTP/0.0 200 OK\r\nContent-Length: 1\r\n\r\nz",
	"HTTP/3.0 200 OK\r\nContent-Length: 1\r\n\r\nz",
	"http/1.1 200 OK\r\n\r\n",
	"FOO 200 OK\r\n\r\n",
	" HTTP/1.1 200 OK\r\n\r\n",
	"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nab",
	"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nabcdef",
	"HTTP/1.1 200 OK\r\nContent-Length: \r\n\r\nx",
	"HTTP/1.1 200 OK\r\nContent-Length: abc\r\n\r\nx",
	"HTTP/1.1 200 OK\r\nContent-Length: -1\r\n\r\nx",
	"HTTP/1.1 200 OK\r\nContent-Length: +5\r\n\r\nhello",
	"HTTP/1.1 200 OK\r\nContent-Length:  5 \r\n\r\nhello",
	"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Length: 5\r\n\r\nhello",
	"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Length: 6\r\n\r\nhello!",
	"HTTP/1.1 200 OK\r\nContent-Length: 99999999999999999999\r\n\r\nx",
	"HTTP/1.1 200 OK\r\nContent-Length: 9223372036854775807\r\n\r\nx",
	"HTTP/1.1 200 OK\r\nX-A: 1\r\nX-B: 2\r\n",
	"HTTP/1.1 200 OK\r\nX-A: 1\r\nX-B",
	"HTTP/1.1 200 OK\r\nX-A 1\r\n\r\n",
	"HTTP/1.1 200 OK\r\n X-A: 1\r\n\r\n",
	"HTTP/1.1 200 OK\r\n\tX-A: 1\r\n\r\n",
	"HTTP/1.1 200 OK\r\n " + strings.Repeat("x", 100) + "\r\n\r\n",
	"HTTP/1.1 200 OK\r\n: empty key\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX A: space key\r\n\r\n",
	"HTTP/1.1 200 OK\r\nx-a b: space key lower\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX(A): bad key\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-A: bad\x01value\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-A: obs \xe0\xb8\x81 text\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-A: cr\rin value\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-Folded: first\r\n   second\r\n\tthird\r\nX-B: b\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-Folded: first\r\n   \r\nX-B: b\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-Folded:\r\n continued only\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-Folded: v\r\n ",
	"HTTP/1.1 200 OK\r\nx-lower: 1\r\nX-LOWER: 2\r\nx-Lower: 3\r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-Spaces:    lead and trail   \r\n\r\n",
	"HTTP/1.1 200 OK\r\nX-Tab:\tv\t\r\n\r\n",
	"HTTP/1.1 200 OK\r\nPragma: no-cache\r\n\r\n",
	"HTTP/1.1 200 OK\r\nPragma: no-cache\r\nCache-Control: max-age=5\r\n\r\n",
	"HTTP/1.1 200 OK\r\nPragma: No-Cache\r\n\r\n",
	"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 2\r\n\r\nhi",
	"HTTP/1.1 200 OK\r\nConnection: keep-alive, Close\r\n\r\nrest",
	"HTTP/1.0 200 OK\r\nConnection: keep-alive\r\nContent-Length: 2\r\n\r\nhi",
	"HTTP/1.0 200 OK\r\nConnection: keep-alive\r\n\r\nrest",
	"HTTP/2.0 200 OK\r\nConnection: close\r\n\r\nrest",
	"HTTP/1.1 204 No Content\r\nContent-Length: 5\r\n\r\nhello",
	"HTTP/1.1 304 Not Modified\r\n\r\nignored",
	"HTTP/1.1 100 Continue\r\n\r\nx",
	"HTTP/1.1 101 Switching\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\nx",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: Chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip\r\n\r\nx",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked, gzip\r\n\r\nx",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n\r\nx",
	"HTTP/1.0 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Length: 3\r\n\r\n3\r\nabc\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n12345678901234567\r\nx",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nffffffffffffffff\r\nx",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;ext=1\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5 \t\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n 5\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhelloXX0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhel",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\nX-T: 1\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: X-T, x-u\r\n\r\n5\r\nhello\r\n0\r\nX-T: 1\r\nX-U: 2\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: Content-Length\r\n\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTrailer: X-T\r\nContent-Length: 1\r\n\r\nx",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-T: 1\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-T 1\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n X-T: 1\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-T: " + strings.Repeat("v", 5000) + "\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-T: " + strings.Repeat("v", 4080) + "\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n" + strings.Repeat("0", 4100) + "5\r\nhello\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;" + strings.Repeat("e", 4090) + "\r\nx\r\n0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n" + strings.Repeat("1;"+strings.Repeat("e", 300)+"\r\nx\r\n", 80) + "0\r\n\r\n",
	"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n" + strings.Repeat("1\r\nx\r\n", 500) + "0\r\n\r\n",
	"HTTP/1.1 304 Not Modified\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n",
	"HTTP/2.0 200 OK\r\nContent-Type: application/json; charset=UTF-8\r\nDate: Sat, 27 Sep 2025 10:00:00 GMT\r\nX-Etags-1: e1\r\nX-Etags-2: e2\r\n\r\n{\"a\":1}\n",
}

type roundTripRec struct {
	calls  []any
	answer *http.Response
}

func (r *roundTripRec) RoundTrip(req *http.Request) (*http.Response, error) {
	r.calls = append(r.calls, map[string]any{
		"method": req.Method,
		"url":    req.URL.String(),
		"header": headerDump(req.Header),
	})
	if r.answer != nil {
		return r.answer, nil
	}
	return nil, errors.New("network disabled")
}

type rtEntry struct {
	name   string
	header map[string][]string
	status int
}

var rtEntries = []rtEntry{
	{name: "plain", status: 200, header: map[string][]string{"Content-Type": {"application/json"}}},
	{name: "past-maxage", status: 200, header: map[string][]string{"Date": {"Sat, 01 Jan 2000 00:00:00 GMT"}, "Cache-Control": {"max-age=60"}, "Etag": {`"e1"`}, "Last-Modified": {"Fri, 31 Dec 1999 00:00:00 GMT"}}},
	{name: "future-date", status: 200, header: map[string][]string{"Date": {"Fri, 01 Jan 2100 00:00:00 GMT"}}},
	{name: "future-maxage", status: 200, header: map[string][]string{"Date": {"Fri, 01 Jan 2100 00:00:00 GMT"}, "Cache-Control": {"public, max-age=31536000"}}},
	{name: "expires", status: 200, header: map[string][]string{"Date": {"Sat, 01 Jan 2000 00:00:00 GMT"}, "Expires": {"Fri, 01 Jan 2100 00:00:00 GMT"}}},
	{name: "bad-expires", status: 200, header: map[string][]string{"Date": {"Sat, 01 Jan 2000 00:00:00 GMT"}, "Expires": {"soon"}}},
	{name: "no-cache", status: 200, header: map[string][]string{"Date": {"Fri, 01 Jan 2100 00:00:00 GMT"}, "Cache-Control": {"no-cache"}, "Etag": {`"e2"`}}},
	{name: "stale-if-error", status: 200, header: map[string][]string{"Date": {"Sat, 01 Jan 2000 00:00:00 GMT"}, "Cache-Control": {"max-age=1, stale-if-error"}}},
	{name: "stale-if-error-n", status: 200, header: map[string][]string{"Date": {"Sat, 01 Jan 2000 00:00:00 GMT"}, "Cache-Control": {"stale-if-error=100"}}},
	{name: "stale-if-error-future", status: 200, header: map[string][]string{"Date": {"Fri, 01 Jan 2100 00:00:00 GMT"}, "Cache-Control": {"no-cache, stale-if-error=5"}}},
	{name: "vary", status: 200, header: map[string][]string{"Date": {"Fri, 01 Jan 2100 00:00:00 GMT"}, "Vary": {"Accept, x-custom"}, "X-Varied-Accept": {"text/html"}}},
	{name: "etag-pair", status: 200, header: map[string][]string{"X-Etags-1": {"a"}, "X-Etags-2": {"b"}, "Etag": {"W/\"w\""}}},
	{name: "404", status: 404, header: map[string][]string{"Date": {"Fri, 01 Jan 2100 00:00:00 GMT"}}},
	{name: "500-stale", status: 500, header: map[string][]string{"Cache-Control": {"stale-if-error"}}},
	{name: "chunked", status: 200, header: map[string][]string{"Date": {"Fri, 01 Jan 2100 00:00:00 GMT"}}},
	{name: "corrupt", status: 0},
}

type rtReq struct {
	name   string
	method string
	header map[string][]string
}

var rtReqs = []rtReq{
	{name: "get", method: "GET"},
	{name: "head", method: "HEAD"},
	{name: "post", method: "POST"},
	{name: "range", method: "GET", header: map[string][]string{"Range": {"bytes=0-1"}}},
	{name: "no-cache", method: "GET", header: map[string][]string{"Cache-Control": {"no-cache"}}},
	{name: "max-age-0", method: "GET", header: map[string][]string{"Cache-Control": {"max-age=0"}}},
	{name: "max-age-big", method: "GET", header: map[string][]string{"Cache-Control": {"max-age=999999999999"}}},
	{name: "only-if-cached", method: "GET", header: map[string][]string{"Cache-Control": {"only-if-cached"}}},
	{name: "max-stale", method: "GET", header: map[string][]string{"Cache-Control": {"max-stale"}}},
	{name: "max-stale-n", method: "GET", header: map[string][]string{"Cache-Control": {"max-stale=999999999999"}}},
	{name: "min-fresh", method: "GET", header: map[string][]string{"Cache-Control": {"min-fresh=10"}}},
	{name: "stale-if-error", method: "GET", header: map[string][]string{"Cache-Control": {"stale-if-error"}}},
	{name: "accept-html", method: "GET", header: map[string][]string{"Accept": {"text/html"}}},
	{name: "accept-json", method: "GET", header: map[string][]string{"Accept": {"application/json"}}},
	{name: "etag-set", method: "GET", header: map[string][]string{"Etag": {`"mine"`}}},
}

type rtCache struct {
	name   string
	maxAge time.Duration
	age    time.Duration
}

var rtCaches = []rtCache{
	{name: "forever", maxAge: -1, age: 2 * time.Hour},
	{name: "disabled", maxAge: 0, age: 10 * time.Minute},
	{name: "fresh", maxAge: time.Hour, age: 10 * time.Minute},
	{name: "expired", maxAge: time.Hour, age: 2 * time.Hour},
	{name: "missing", maxAge: -1, age: -1},
}

func entryDump(e rtEntry) []byte {
	if e.status == 0 {
		return []byte("HTTP/1.1 2x0 corrupt\r\n\r\n")
	}
	s := synthResp{proto: "HTTP/2.0", major: 2, minor: 0, status: e.status, header: e.header, body: youtubeJSON, cl: -1}
	if e.name == "chunked" {
		s.proto, s.major, s.minor, s.te = "HTTP/1.1", 1, 1, []string{"chunked"}
	}
	b, err := s.dump()
	if err != nil {
		log.Fatal(err)
	}
	return b
}

func roundTrips(tmp string) []map[string]any {
	var cases []map[string]any
	n := 0
	for _, e := range rtEntries {
		dump := entryDump(e)
		for _, cc := range rtCaches {
			for _, rq := range rtReqs {
				for _, always := range []bool{true, false} {
					for _, answer := range []int{0, 200} {
						if answer != 0 && (always || rq.name != "get") {
							continue
						}
						n++
						dir := filepath.Join(tmp, fmt.Sprintf("rt%d", n))
						bfs := hugofs.NewBasePathFs(afero.NewOsFs(), dir)
						c := filecache.NewCache(bfs, cc.maxAge, "")
						const id = "5844198154546968338"
						if cc.age >= 0 {
							info, w, err := c.WriteCloser(id)
							if err != nil {
								log.Fatal(err)
							}
							if _, err := w.Write(dump); err != nil {
								log.Fatal(err)
							}
							if err := w.Close(); err != nil {
								log.Fatal(err)
							}
							mt := time.Now().Add(-cc.age)
							if err := os.Chtimes(filepath.Join(dir, info.Name), mt, mt); err != nil {
								log.Fatal(err)
							}
						}
						rec := &roundTripRec{}
						if answer != 0 {
							rec.answer = &http.Response{StatusCode: answer, Status: "200 OK", Proto: "HTTP/1.1", ProtoMajor: 1, ProtoMinor: 1,
								Header: http.Header{}, Body: io.NopCloser(strings.NewReader("fresh"))}
						}
						alwaysV := always
						t := &httpcache.Transport{
							Cache:    c.AsHTTPCache(),
							CacheKey: func(req *http.Request) string { return id },
							Around: func(req *http.Request, key string) func() {
								return c.NamedLock(key)
							},
							AlwaysUseCachedResponse: func(req *http.Request, key string) bool { return alwaysV },
							ShouldCache: func(req *http.Request, resp *http.Response, key string) bool {
								return resp.StatusCode == 200
							},
							MarkCachedResponses: true,
							EnableETagPair:      true,
							Transport:           rec,
						}
						req, err := http.NewRequest(rq.method, "https://www.googleapis.com/youtube/v3/videos?id=iIsZs0m-BVU&part=statistics", nil)
						if err != nil {
							log.Fatal(err)
						}
						for k, v := range rq.header {
							req.Header[k] = v
						}
						resp, err := t.RoundTrip(req)
						var res map[string]any
						if err != nil {
							res = map[string]any{"err": err.Error()}
						} else {
							res = map[string]any{"ok": respDump(resp)}
						}
						_, statErr := os.Stat(filepath.Join(dir, id))
						cases = append(cases, map[string]any{
							"rt":     e.name,
							"cache":  cc.name,
							"req":    rq.name,
							"always": always,
							"answer": answer,
							"r":      res,
							"calls":  rec.calls,
							"exists": statErr == nil,
						})
					}
				}
			}
		}
	}
	return cases
}

var httpConfigs = []map[string]any{
	nil,
	{},
	{"cache": maps.Params{"for": maps.Params{"includes": []any{"**"}}}},
	{"cache": maps.Params{"for": maps.Params{"excludes": []any{"**"}, "includes": []any{"https://api.example.com/**"}}}},
	{"cache": maps.Params{"for": maps.Params{"includes": "**.json"}}},
	{"polls": []any{maps.Params{"for": maps.Params{"includes": []any{"**"}}, "low": "1s", "high": "30s"}}},
	{"polls": []any{maps.Params{"low": "1s"}, maps.Params{"for": maps.Params{"excludes": []any{"**.png"}}, "disable": true}}},
	{"polls": []any{maps.Params{"for": maps.Params{"includes": []any{"{a,b}**"}}, "low": 5000000000, "high": "1m30s"}}},
	{"polls": []any{maps.Params{"for": maps.Params{"includes": []any{"[z-a]"}}}}},
	{"cache": maps.Params{"for": maps.Params{"includes": []any{"[unclosed"}}}},
	{"polls": []any{maps.Params{"low": "fast"}}},
	{"polls": "notalist"},
	{"cache": "notamap"},
}

var httpURLs = []string{
	"https://www.googleapis.com/youtube/v3/videos?id=iIsZs0m-BVU&part=statistics",
	"https://api.example.com/v1/x.json", "https://api.example.com", "http://a/b.png", "a", "b/c",
	"", "/", "https://example.org/data.json", "ftp://x/y.json",
}

func httpConfigCases() []map[string]any {
	var cases []map[string]any
	for i, m := range httpConfigs {
		c := map[string]any{"cfg": i, "in": goval.Encode(m)}
		cfg, err := hhttpcache.DecodeConfig(config.BaseConfig{}, m)
		if err != nil {
			c["err"] = err.Error()
			cases = append(cases, c)
			continue
		}
		var polls []any
		for _, p := range cfg.Polls {
			b, _ := p.MarshalJSON()
			polls = append(polls, string(b))
		}
		c["decoded"] = map[string]any{
			"cacheIncludes": cfg.Cache.For.Includes, "cacheExcludes": cfg.Cache.For.Excludes, "polls": polls,
		}
		cc, err := cfg.Compile()
		if err != nil {
			c["compileErr"] = err.Error()
			cases = append(cases, c)
			continue
		}
		var fors []any
		for _, u := range httpURLs {
			pc := cc.PollConfigFor(u)
			var pcd any
			if !pc.IsZero() {
				b, _ := pc.Config.MarshalJSON()
				pcd = string(b)
			}
			fors = append(fors, []any{cc.For(u), pcd})
		}
		c["for"] = fors
		c["pollingDisabled"] = cc.IsPollingDisabled()
		cases = append(cases, c)
	}
	return cases
}

func main() {
	out := flag.String("out", "crates/nh-helpers/tests/fixtures/httpcache", "output directory")
	flag.Parse()

	rng := rand.New(rand.NewSource(81))
	tmp, err := os.MkdirTemp("", "nh-helpers-httpcache")
	if err != nil {
		log.Fatal(err)
	}
	defer func() { _ = os.RemoveAll(tmp) }()

	var dumps [][]byte
	dumps = append(dumps, synthDumps(rng)...)
	for _, s := range rawDumps {
		dumps = append(dumps, []byte(s))
	}

	var cases []map[string]any
	for i, d := range dumps {
		cases = append(cases, map[string]any{
			"dump": i,
			"get":  readResp(d, "GET"),
			"head": readResp(d, "HEAD"),
		})
	}
	// Every prefix of some dumps (truncated entries) and seeded mutations.
	prefixOf := []int{0, 1, 3, 18, 19, 20, len(dumps) - 1, len(dumps) - 30, len(dumps) - 20, len(dumps) - 15, len(dumps) - 10, len(dumps) - 7}
	for _, i := range prefixOf {
		d := dumps[i]
		for n := 0; n < len(d) && n < 700; n++ {
			cases = append(cases, map[string]any{"dump": i, "prefix": n, "get": readResp(d[:n], "GET")})
		}
	}
	for k := range 1500 {
		i := rng.Intn(len(dumps))
		d := append([]byte(nil), dumps[i]...)
		if len(d) == 0 {
			continue
		}
		var muts [][2]int
		for range 1 + rng.Intn(3) {
			pos := rng.Intn(len(d))
			alphabet := []byte("\r\n :;0aZ\t\x00-5fF")
			b := alphabet[rng.Intn(len(alphabet))]
			if rng.Intn(4) == 0 {
				b = byte(rng.Intn(256))
			}
			d[pos] = b
			muts = append(muts, [2]int{pos, int(b)})
		}
		method := "GET"
		if k%5 == 0 {
			method = "HEAD"
		}
		cases = append(cases, map[string]any{"dump": i, "mut": muts, "method": method, "get": readResp(d, method)})
	}

	var dumpsOut []any
	for _, d := range dumps {
		dumpsOut = append(dumpsOut, hsupport.Str(string(d)))
	}

	rtEntryDumps := map[string]any{}
	for _, e := range rtEntries {
		rtEntryDumps[e.name] = hsupport.Str(string(entryDump(e)))
	}

	header := map[string]any{
		"dumps":       dumpsOut,
		"rtEntries":   rtEntryDumps,
		"roundTrips":  roundTrips(tmp),
		"httpConfigs": httpConfigCases(),
		"httpURLs":    httpURLs,
	}
	p := filepath.Join(*out, "httpcache.json.gz")
	if err := hsupport.WriteGz(p, header, cases); err != nil {
		log.Fatal(err)
	}
	log.Printf("%s: %d dumps, %d read cases", p, len(dumps), len(cases))
}
