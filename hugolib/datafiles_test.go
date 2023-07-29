// Copyright 2019 The Hugo Authors. All rights reserved.
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

package hugolib

import (
	"testing"
)

func TestData(t *testing.T) {
	t.Run("with theme", func(t *testing.T) {
		t.Parallel()

		files := `
-- hugo.toml --
baseURL = "https://example.com"
disableKinds = ["taxonomy", "term", "RSS", "sitemap", "robotsTXT", "page", "section"]
theme = "mytheme"
-- data/a.toml --
v1 = "a_v1"
-- data/b.yaml --
v1: b_v1
-- data/c/d.yaml --
v1: c_d_v1
-- themes/mytheme/data/a.toml --
v1 = "a_v1_theme"
-- themes/mytheme/data/d.toml --
v1 = "d_v1_theme"
-- layouts/index.html --
a: {{  site.Data.a.v1 }}|
b: {{  site.Data.b.v1 }}|
cd: {{ site.Data.c.d.v1 }}|
d: {{  site.Data.d.v1 }}|
`
		b := NewIntegrationTestBuilder(
			IntegrationTestConfig{
				T:           t,
				TxtarString: files,
			},
		).Build()

		b.AssertFileContent("public/index.html", "a: a_v1|\nb: b_v1|\ncd: c_d_v1|\nd: d_v1_theme|")
	})
}
