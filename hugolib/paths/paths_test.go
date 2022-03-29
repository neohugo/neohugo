// Copyright 2018 The Hugo Authors. All rights reserved.
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

package paths

import (
	"testing"

	qt "github.com/frankban/quicktest"
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/hugofs"
	"github.com/neohugo/neohugo/langs"
)

func TestNewPaths(t *testing.T) {
	c := qt.New(t)

	v := config.New()
	fs := hugofs.NewMem(v)

	v.Set("languages", map[string]any{
		"no": map[string]any{},
		"en": map[string]any{},
	})
	v.Set("defaultContentLanguageInSubdir", true)
	v.Set("defaultContentLanguage", "no")
	v.Set("contentDir", "content")
	v.Set("workingDir", "work")
	v.Set("resourceDir", "resources")
	v.Set("publishDir", "public")

	_, err := langs.LoadLanguageSettings(v, nil)
	c.Assert(err, qt.IsNil)

	p, err := New(fs, v)
	c.Assert(err, qt.IsNil)

	c.Assert(p.defaultContentLanguageInSubdir, qt.Equals, true)
	c.Assert(p.DefaultContentLanguage, qt.Equals, "no")
	c.Assert(p.multilingual, qt.Equals, true)
}
