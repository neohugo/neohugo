// Copyright 2017 The Hugo Authors. All rights reserved.
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

package cast

import (
	"github.com/neohugo/neohugo/config"
	"github.com/neohugo/neohugo/config/testconfig"
	"github.com/neohugo/neohugo/deps"
	"github.com/neohugo/neohugo/docshelper"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/tpl/internal"
)

// This file provides documentation support and is randomly put into this package.
func init() {
	docsProvider := func() docshelper.DocProvider {
		d := &deps.Deps{Conf: testconfig.GetTestConfig(nil, nil)}
		if err := d.Init(); err != nil {
			panic(err)
		}
		conf := testconfig.GetTestConfig(nil, newTestConfig())
		d.Site = page.NewDummyHugoSite(conf)

		var namespaces internal.TemplateFuncsNamespaces

		for _, nsf := range internal.TemplateFuncsNamespaceRegistry {
			nf := nsf(d)
			namespaces = append(namespaces, nf)

		}

		return docshelper.DocProvider{"tpl": map[string]any{"funcs": namespaces}}
	}

	docshelper.AddDocProviderFunc(docsProvider)
}

func newTestConfig() config.Provider {
	v := config.New()
	v.Set("contentDir", "content")
	return v
}
