package main

import (
	"bufio"
	"compress/zlib"
	"fmt"
	"os"
	"sort"
	"strings"
)

// recordWriter writes the fixture record format read by
// crates/libsass-sys/tests/common/mod.rs, zlib-compressed:
//
//	@<key> <len>\n<len bytes>\n
type recordWriter struct {
	f *os.File
	z *zlib.Writer
	w *bufio.Writer
}

func newRecordWriter(path string) *recordWriter {
	f, err := os.Create(path)
	if err != nil {
		panic(err)
	}
	z, _ := zlib.NewWriterLevel(f, zlib.BestCompression)
	return &recordWriter{f: f, z: z, w: bufio.NewWriter(z)}
}

func (r *recordWriter) put(key string, val []byte) {
	if strings.ContainsAny(key, " \n") {
		panic(key)
	}
	_, _ = fmt.Fprintf(r.w, "@%s %d\n", key, len(val))
	_, _ = r.w.Write(val)
	_ = r.w.WriteByte('\n')
}

func (r *recordWriter) putS(key, val string) { r.put(key, []byte(val)) }

func (r *recordWriter) putI(key string, v int) { r.putS(key, fmt.Sprint(v)) }

func (r *recordWriter) putB(key string, v bool) {
	if v {
		r.putS(key, "1")
	} else {
		r.putS(key, "0")
	}
}

func (r *recordWriter) close() {
	if err := r.w.Flush(); err != nil {
		panic(err)
	}
	if err := r.z.Close(); err != nil {
		panic(err)
	}
	if err := r.f.Close(); err != nil {
		panic(err)
	}
}

// writeSitePack stores files (relative path -> content) as records.
func writeSitePack(path string, files map[string][]byte) {
	var names []string
	for k := range files {
		names = append(names, k)
	}
	sort.Strings(names)
	w := newRecordWriter(path)
	for _, n := range names {
		w.putS("file", n)
		w.put("data", files[n])
	}
	w.close()
}
