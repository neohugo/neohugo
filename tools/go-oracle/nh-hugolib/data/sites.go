package main

import (
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/neohugo/neohugo/tools/go-oracle/nh-hugolib/hsupport"
)

// The sites of the data oracle: the data directory of the project (and of
// its themes) in every format the metadecoders read, nested directories,
// files and directories with the same key, map merges and conflicts, empty
// and scalar documents, and this repository's docs/data.

func inline(m map[string]string) map[string]hsupport.File {
	out := map[string]hsupport.File{}
	for k, v := range m {
		out[k] = hsupport.File{Content: v}
	}
	return out
}

const baseTOML = `baseURL = "https://example.org/"
title = "Data"
disableKinds = ["taxonomy", "term", "rss", "sitemap", "robotsTXT", "404"]
`

// dataFiles is the data directory shared by the main data sites.
var dataFiles = map[string]string{
	"data/numbers.json": `{"int": 1, "float": 1.5, "big": 12345678901234567890, "neg": -3, "exp": 1e3,
 "str": "s", "empty": "", "bool": true, "null": null, "arr": [1, "a", {"k": 2}, [true, null]],
 "obj": {"nested": {"deep": [1.0, 2]}, "Camel": "c"}, "unicode": "ขนม é é"}`,
	"data/numbers.yaml": "int: 1\nfloat: 1.5\nneg: -7\nbig: 9223372036854775807\nbigger: 9223372036854775808\nhex: 0x1f\noctal: 017\nexp: 1e3\n" +
		"str: \"x\"\nbare: text\nbool: true\nyes: yes\nnull: ~\ndate: 2020-01-02\ntime: 2020-01-02T03:04:05Z\n" +
		"list: [1, two, 3.0]\nnested:\n  CamelKey: v\n  deep:\n    - a: 1\n    - b: [x]\nintkeys:\n  1: one\n  2: two\n",
	"data/numbers.toml": "int = 1\nfloat = 1.5\nneg = -7\nbig = 9223372036854775807\nhex = 0x1f\n" +
		"str = \"x\"\nbool = true\ndate = 1979-05-27T07:32:00Z\nlocaldate = 1979-05-27\nlocaltime = 07:32:00\n" +
		"localdatetime = 1979-05-27T07:32:00\nlist = [1, 2]\nmixed = [1, \"a\"]\n\n[table]\nkey = \"v\"\n\n[[aot]]\nname = \"a\"\n\n[[aot]]\nname = \"b\"\n",
	"data/list.json":                `[1, 2, {"a": "b"}]`,
	"data/emptyarr.json":            `[]`,
	"data/empty.json":               ``,
	"data/empty.yaml":               ``,
	"data/nullyaml.yaml":            "~\n",
	"data/scalar.json":              `"just a string"`,
	"data/number.json":              `42`,
	"data/nested/deep/file.yaml":    "k: v\nn: 1\n",
	"data/nested/deep.json":         `{"fromfile": true, "file": "overridden"}`,
	"data/nested/deeper/a/b/c.toml": "x = 1\n",
	"data/Upper/MixedCase.json":     `{"Key": "Value"}`,
	"data/dots/a.b.json":            `{"dots": 1}`,
	"data/conflict/x.json":          `[1]`,
	"data/conflict/x.yaml":          "a: 1\n",
	"data/conflict/y.yaml":          "a: 1\n",
	"data/conflict/y.toml":          "a = 2\nb = 3\n",
	"data/comments/2020/post-1.json": `{"_id": "c1", "name": "Alice", "date": 1600000000, "email": "",
 "body": "Hello <b>world</b>", "reply_to": "", "replies": 2}`,
	"data/comments/2020/post-2.json":  `{"_id": "c2", "name": "Bob", "date": 1600000001.5, "reply_to": "c1"}`,
	"data/comments/ขนม/ความเห็น.json": `{"_id": "th1", "name": "สมชาย"}`,
	"data/.hidden.json":               `{"hidden": true}`,
}

// themeDataFiles are the data files of the theme of the data-theme site.
var themeDataFiles = map[string]string{
	"themes/t1/data/numbers.json":              `{"int": 100, "themeonly": "t", "obj": {"x": 1}}`,
	"themes/t1/data/themeonly.yaml":            "fromtheme: true\n",
	"themes/t1/data/nested/deep/file.yaml":     "k: theme\nextra: 2\n",
	"themes/t1/data/nested/theme.toml":         "t = 1\n",
	"themes/t1/data/list.json":                 `["theme", "list"]`,
	"themes/t1/data/themelist.json":            `["only", "theme"]`,
	"themes/t1/data/conflict/x.json":           `{"theme": true}`,
	"themes/t1/data/comments/2020/post-1.json": `{"_id": "theme"}`,
	"themes/t1/data/comments/2020/post-3.json": `{"_id": "c3", "name": "Theme Carol"}`,
	"themes/t1/theme.toml":                     "name = \"t1\"\n",
}

func merge(ms ...map[string]string) map[string]string {
	out := map[string]string{}
	for _, m := range ms {
		for k, v := range m {
			out[k] = v
		}
	}
	return out
}

// DataSites returns the sites of the data oracle.
func DataSites(root string) ([]hsupport.Site, error) {
	sites := []hsupport.Site{
		{Name: "data-basic", TOML: baseTOML, Files: inline(merge(dataFiles, map[string]string{"content/_index.md": "---\ntitle: Home\n---\n"}))},
		{Name: "data-theme", TOML: baseTOML + "theme = \"t1\"\n", Files: inline(merge(dataFiles, themeDataFiles))},
		{Name: "data-themeonly", TOML: baseTOML + "theme = \"t1\"\n", Files: inline(themeDataFiles)},
		{
			Name: "data-i18n",
			TOML: baseTOML + "defaultContentLanguage = \"en\"\n[languages.en]\nweight = 1\n[languages.th]\nweight = 2\n",
			Files: inline(map[string]string{
				"data/site.yaml":       "name: site\n",
				"data/site.th.yaml":    "name: ไทย\n",
				"data/lang/en.json":    `{"hello": "Hello"}`,
				"data/lang/th.json":    `{"hello": "สวัสดี"}`,
				"content/_index.md":    "---\ntitle: Home\n---\n",
				"content/_index.th.md": "---\ntitle: หน้าแรก\n---\n",
			}),
		},
		{
			Name:  "data-mounts",
			TOML:  baseTOML + "[[module.mounts]]\nsource = \"mydata\"\ntarget = \"data\"\n[[module.mounts]]\nsource = \"more\"\ntarget = \"data/more\"\n",
			Files: inline(map[string]string{"mydata/a.json": `{"a": 1}`, "more/b.yaml": "b: 2\n", "more/sub/c.toml": "c = 3\n"}),
		},
		{Name: "data-ignore", TOML: baseTOML + "ignoreFiles = [\"\\\\.skip\\\\.json$\"]\n", Files: inline(map[string]string{"data/keep.json": `{"k": 1}`, "data/drop.skip.json": `{`})},
		{Name: "data-none", TOML: baseTOML, Files: inline(map[string]string{"content/_index.md": ""})},
		{Name: "data-badjson", TOML: baseTOML, Files: inline(map[string]string{"data/a.json": `{"a": 1}`, "data/bad.json": "{\n  \"a\": 1,\n}\n"})},
		{Name: "data-badyaml", TOML: baseTOML, Files: inline(map[string]string{"data/bad.yaml": "a: [1\nb: 2\n"})},
		{Name: "data-badtoml", TOML: baseTOML, Files: inline(map[string]string{"data/bad.toml": "a = \n"})},
		{Name: "data-unknownext", TOML: baseTOML, Files: inline(map[string]string{"data/readme.txt": "text", "data/z.json": `{"z": 1}`})},
		// XML (clbanning/mxj) and CSV data are not ported: the Rust tests expect the explicit errors.
		{Name: "data-csv", TOML: baseTOML, Files: inline(map[string]string{"data/table.csv": "a,b\n1,2\n", "data/z.json": `{"z": 1}`})},
		{Name: "data-xml", TOML: baseTOML, Files: inline(map[string]string{"data/doc.xml": `<root><a>1</a><b x="y">t</b><c><d>deep</d></c><e>1</e><e>2</e></root>`})},
		{Name: "data-dirfile", TOML: baseTOML, Files: inline(map[string]string{"data/a.json": `[1, 2]`, "data/a/b.json": `{"b": 1}`})},
	}

	// docs/data from this repository (referenced by hash).
	docs := map[string]hsupport.File{}
	entries, err := os.ReadDir(filepath.Join(root, "docs", "data"))
	if err != nil {
		return nil, err
	}
	var names []string
	for _, e := range entries {
		if !e.IsDir() {
			names = append(names, e.Name())
		}
	}
	sort.Strings(names)
	for _, n := range names {
		rel := "docs/data/" + n
		b, err := os.ReadFile(filepath.Join(root, filepath.FromSlash(rel)))
		if err != nil {
			return nil, err
		}
		docs["data/"+n] = hsupport.File{Content: string(b), Repo: rel}
	}
	sites = append(sites, hsupport.Site{Name: "data-docs", TOML: baseTOML, Files: docs})

	for i := range sites {
		if !strings.HasPrefix(sites[i].Name, "data-") {
			panic(sites[i].Name)
		}
	}
	return sites, nil
}
