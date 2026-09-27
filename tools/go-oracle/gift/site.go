package main

import (
	"fmt"
	"image"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
	"sync"
)

// site dumps every JPEG/PNG under <siteRoot>/{content,assets} (decoded with
// Go image/jpeg and image/png) to <outDir>/imgs/<n>.gz and writes
// <outDir>/index.tsv (n, path, Go type, bounds) and <outDir>/ops.tsv (the
// real-image op table with digests; see realOpsFor). The watermark overlay
// uses assets/images/watermark.png.
func site(siteRoot, outDir string) {
	var files []string
	for _, dir := range []string{"content", "assets"} {
		filepath.Walk(filepath.Join(siteRoot, dir), func(p string, fi os.FileInfo, err error) error {
			if err != nil || fi.IsDir() {
				return nil
			}
			switch strings.ToLower(filepath.Ext(p)) {
			case ".jpg", ".jpeg", ".png":
				files = append(files, p)
			}
			return nil
		})
	}
	sort.Strings(files)
	if err := os.MkdirAll(filepath.Join(outDir, "imgs"), 0o755); err != nil {
		panic(err)
	}
	wmPath := filepath.Join(siteRoot, "assets/images/watermark.png")
	wm := decodeFile(wmPath)
	wmName := ""
	for i, f := range files {
		if f == wmPath {
			wmName = fmt.Sprint(i)
		}
	}
	if wmName == "" {
		panic("no watermark")
	}
	dumps := func(n string) image.Image {
		if n != wmName {
			panic(n)
		}
		return wm
	}

	type result struct {
		index string
		ops   []string
	}
	results := make([]result, len(files))
	work := make(chan int)
	var wg sync.WaitGroup
	for w := 0; w < runtime.GOMAXPROCS(0); w++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for i := range work {
				img := decodeFile(files[i])
				name := fmt.Sprint(i)
				saveDump(filepath.Join(outDir, "imgs", name+".gz"), img)
				rel, _ := filepath.Rel(siteRoot, files[i])
				b := img.Bounds()
				res := result{index: fmt.Sprintf("%s\t%s\t%T\t%d,%d,%d,%d", name, rel, img, b.Min.X, b.Min.Y, b.Max.X, b.Max.Y)}
				level := 1
				if i%8 == 0 {
					level = 2
				}
				for _, op := range realOpsFor(b, i, wmName, level) {
					res.ops = append(res.ops, fmt.Sprintf("%s\t%s\t%s\t%s", name, op.kind, op.arg, digest(runRealOp(img, op, dumps))))
				}
				results[i] = res
				fmt.Fprintf(os.Stderr, "%d/%d %s\n", i+1, len(files), rel)
			}
		}()
	}
	for i := range files {
		work <- i
	}
	close(work)
	wg.Wait()

	idx, err := os.Create(filepath.Join(outDir, "index.tsv"))
	if err != nil {
		panic(err)
	}
	ops, err := os.Create(filepath.Join(outDir, "ops.tsv"))
	if err != nil {
		panic(err)
	}
	for _, r := range results {
		fmt.Fprintln(idx, r.index)
		for _, o := range r.ops {
			fmt.Fprintln(ops, o)
		}
	}
	idx.Close()
	ops.Close()
}
