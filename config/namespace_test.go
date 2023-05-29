// Copyright 2023 The Hugo Authors. All rights reserved.
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

package config

import (
	"strings"
	"testing"

	qt "github.com/frankban/quicktest"
	"github.com/neohugo/neohugo/common/maps"

	"github.com/mitchellh/mapstructure"
)

func TestNamespace(t *testing.T) {
	c := qt.New(t)
	c.Assert(true, qt.Equals, true)

	// ns, err := config.DecodeNamespace[map[string]DocsMediaTypeConfig](in, defaultMediaTypesConfig, buildConfig)

	ns, err := DecodeNamespace[[]*tstNsExt](
		map[string]interface{}{"foo": "bar"},
		func(v any) (*tstNsExt, any, error) {
			t := &tstNsExt{}
			m, err := maps.ToStringMapE(v)
			if err != nil {
				return nil, nil, err
			}
			return t, nil, mapstructure.WeakDecode(m, t)
		},
	)

	c.Assert(err, qt.IsNil)
	c.Assert(ns, qt.Not(qt.IsNil))
	c.Assert(ns.SourceStructure, qt.DeepEquals, map[string]interface{}{"foo": "bar"})
	c.Assert(ns.SourceHash, qt.Equals, "14368731254619220105")
	c.Assert(ns.Config, qt.DeepEquals, &tstNsExt{Foo: "bar"})
	c.Assert(ns.Signature(), qt.DeepEquals, []*tstNsExt(nil))
}

type (
	tstNsExt struct {
		Foo string
	}
	tstNsInt struct { // nolint
		Foo string
	}
)

func (t *tstNsExt) Init() error {
	t.Foo = strings.ToUpper(t.Foo)
	return nil
}

func (t *tstNsInt) Compile(ext *tstNsExt) error { // nolint
	t.Foo = ext.Foo + " qux"
	return nil
}
