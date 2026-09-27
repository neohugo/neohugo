package main

import (
	"bytes"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/parser/pageparser"
)

// corpusMain renders every .md file below a content directory (front matter
// stripped with neohugo's pageparser, the same split Hugo uses) with the
// given configs, writing GMF to the output path (gzip if it ends in .gz).
//
//	goldmark corpus <content dir> <out.gmf.gz> [cfg,cfg,...]
func corpusMain(args []string) {
	root := args[0]
	cfgs := []string{"default"}
	if len(args) > 2 {
		cfgs = strings.Split(args[2], ",")
	}
	var files []string
	err := filepath.WalkDir(root, func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if !d.IsDir() && strings.HasSuffix(path, ".md") {
			files = append(files, path)
		}
		return nil
	})
	if err != nil {
		panic(err)
	}
	sort.Strings(files)
	writeFile(args[1], func(g *gmfWriter) {
		for _, f := range files {
			b, err := os.ReadFile(f)
			if err != nil {
				panic(err)
			}
			cfm, err := pageparser.ParseFrontMatterAndContent(bytes.NewReader(b))
			if err != nil {
				panic(err)
			}
			rel, _ := filepath.Rel(root, f)
			for _, cfg := range cfgs {
				// the body as Hugo feeds it to goldmark
				g.record("corpus/" + cfg + "/" + rel)
				g.field("cfg", []byte(cfg))
				g.field("md", cfm.Content)
				g.field("html", convert(cfg, cfm.Content))
			}
			// the whole file (front matter included) exercises more block
			// structure (thematic breaks, setext headings, tables of YAML).
			g.record("corpus-full/default/" + rel)
			g.field("cfg", []byte("default"))
			g.field("md", b)
			g.field("html", convert("default", b))
		}
	})
}
