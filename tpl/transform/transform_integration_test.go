// Copyright 2024 The Hugo Authors. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

package transform_test

import (
	"testing"

	qt "github.com/frankban/quicktest"
	"github.com/gohugoio/hugo/hugolib"
)

// Issue #11698
func TestMarkdownifyIssue11698(t *testing.T) {
	t.Parallel()

	files := `
-- config.toml --
disableKinds = ['home','section','rss','sitemap','taxonomy','term']
[markup.goldmark.parser.attribute]
title = true
block = true
-- layouts/_default/single.html --
_{{ markdownify .RawContent }}_
-- content/p1.md --
---
title: p1
---
foo bar
-- content/p2.md --
---
title: p2
---
foo

**bar**
-- content/p3.md --
---
title: p3
---
## foo

bar
-- content/p4.md --
---
title: p4
---
foo
{#bar}
  `

	b := hugolib.Test(t, files)

	b.AssertFileContent("public/p1/index.html", "_foo bar_")
	b.AssertFileContent("public/p2/index.html", "_<p>foo</p>\n<p><strong>bar</strong></p>\n_")
	b.AssertFileContent("public/p3/index.html", "_<h2 id=\"foo\">foo</h2>\n<p>bar</p>\n_")
	b.AssertFileContent("public/p4/index.html", "_<p id=\"bar\">foo</p>\n_")
}

func TestXMLEscape(t *testing.T) {
	t.Parallel()

	files := `
-- config.toml --
disableKinds = ['section','sitemap','taxonomy','term']
-- content/p1.md --
---
title: p1
---
a **b** ` + "\v" + ` c
<!--more-->
  `
	b := hugolib.Test(t, files)

	b.AssertFileContent("public/index.xml", `
	<description>&lt;p&gt;a &lt;strong&gt;b&lt;/strong&gt;  c&lt;/p&gt;</description>
	`)
}

// Issue #9642
func TestHighlightError(t *testing.T) {
	t.Parallel()

	files := `
-- hugo.toml --
disableKinds = ['page','rss','section','sitemap','taxonomy','term']
-- layouts/index.html --
{{ highlight "a" "b" 0 }}
  `
	b := hugolib.NewIntegrationTestBuilder(
		hugolib.IntegrationTestConfig{
			T:           t,
			TxtarString: files,
		},
	)

	_, err := b.BuildE()
	b.Assert(err.Error(), qt.Contains, "error calling highlight: invalid Highlight option: 0")
}

// Issue #11884
func TestUnmarshalCSVLazyDecoding(t *testing.T) {
	t.Parallel()

	files := `
-- hugo.toml --
disableKinds = ['page','rss','section','sitemap','taxonomy','term']
-- assets/pets.csv --
name,description,age
Spot,a nice dog,3
Rover,"a big dog",5
Felix,a "malicious" cat,7
Bella,"an "evil" cat",9
Scar,"a "dead cat",11
-- layouts/index.html --
{{ $opts := dict "lazyQuotes" true }}
{{ $data := resources.Get "pets.csv" | transform.Unmarshal $opts }}
{{ printf "%v" $data | safeHTML }}
  `
	b := hugolib.Test(t, files)

	b.AssertFileContent("public/index.html", `
[[name description age] [Spot a nice dog 3] [Rover a big dog 5] [Felix a "malicious" cat 7] [Bella an "evil" cat 9] [Scar a "dead cat 11]]
	`)
}
