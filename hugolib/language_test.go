// Copyright 2020 The Hugo Authors. All rights reserved.
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
	"fmt"
	"strings"
	"testing"

	"github.com/neohugo/neohugo/htesting"

	qt "github.com/frankban/quicktest"
)

func TestI18n(t *testing.T) {
	c := qt.New(t)

	// https://github.com/neohugo/neohugo/issues/7804
	c.Run("pt-br should be case insensitive", func(c *qt.C) {
		b := newTestSitesBuilder(c)
		langCode := func() string {
			c := "pt-br"
			if htesting.RandBool() {
				c = strings.ToUpper(c)
			}
			return c
		}

		b.WithConfigFile(`toml`, fmt.Sprintf(`
baseURL = "https://example.com"
defaultContentLanguage = "%s"

[languages]
[languages.%s]
weight = 1
`, langCode(), langCode()))

		b.WithI18n(fmt.Sprintf("i18n/%s.toml", langCode()), `hello.one = "Hello"`)
		b.WithTemplates("index.html", `Hello: {{ i18n "hello" 1 }}`)
		b.WithContent("p1.md", "")
		b.Build(BuildCfg{})

		b.AssertFileContent("public/index.html", "Hello: Hello")
	})
}
