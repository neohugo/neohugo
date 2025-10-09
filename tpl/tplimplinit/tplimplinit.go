// Copyright 2025 The Hugo Authors. All rights reserved.
//
// Portions Copyright The Go Authors.

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

package tplimplinit

import (
	// Init the template funcs namespaces
	"context"
	"html/template"

	"github.com/neohugo/neohugo/deps"
	_ "github.com/neohugo/neohugo/tpl/cast"
	_ "github.com/neohugo/neohugo/tpl/collections"
	_ "github.com/neohugo/neohugo/tpl/compare"
	_ "github.com/neohugo/neohugo/tpl/crypto"
	_ "github.com/neohugo/neohugo/tpl/css"
	_ "github.com/neohugo/neohugo/tpl/data"
	_ "github.com/neohugo/neohugo/tpl/debug"
	_ "github.com/neohugo/neohugo/tpl/diagrams"
	_ "github.com/neohugo/neohugo/tpl/encoding"
	_ "github.com/neohugo/neohugo/tpl/fmt"
	_ "github.com/neohugo/neohugo/tpl/hash"
	_ "github.com/neohugo/neohugo/tpl/hugo"
	_ "github.com/neohugo/neohugo/tpl/images"
	_ "github.com/neohugo/neohugo/tpl/inflect"
	"github.com/neohugo/neohugo/tpl/internal"
	_ "github.com/neohugo/neohugo/tpl/js"
	_ "github.com/neohugo/neohugo/tpl/lang"
	_ "github.com/neohugo/neohugo/tpl/math"
	_ "github.com/neohugo/neohugo/tpl/openapi/openapi3"
	_ "github.com/neohugo/neohugo/tpl/os"
	_ "github.com/neohugo/neohugo/tpl/page"
	_ "github.com/neohugo/neohugo/tpl/partials"
	_ "github.com/neohugo/neohugo/tpl/path"
	_ "github.com/neohugo/neohugo/tpl/reflect"
	_ "github.com/neohugo/neohugo/tpl/resources"
	_ "github.com/neohugo/neohugo/tpl/safe"
	_ "github.com/neohugo/neohugo/tpl/site"
	_ "github.com/neohugo/neohugo/tpl/strings"
	_ "github.com/neohugo/neohugo/tpl/templates"
	_ "github.com/neohugo/neohugo/tpl/time"
	_ "github.com/neohugo/neohugo/tpl/transform"
	_ "github.com/neohugo/neohugo/tpl/urls"
)

// CreateFuncMap creates a template.FuncMap with all of Hugo's template funcs,
// excluding the Go built-ins.
func CreateFuncMap(d *deps.Deps) map[string]any {
	funcMap := template.FuncMap{}
	nsMap := make(map[string]any)
	var onCreated []func(namespaces map[string]any)

	// Merge the namespace funcs
	for _, nsf := range internal.TemplateFuncsNamespaceRegistry {
		ns := nsf(d)
		if _, exists := funcMap[ns.Name]; exists {
			panic(ns.Name + " is a duplicate template func")
		}
		funcMap[ns.Name] = ns.Context
		contextV, err := ns.Context(context.Background())
		if err != nil {
			panic(err)
		}
		nsMap[ns.Name] = contextV
		for _, mm := range ns.MethodMappings {
			for _, alias := range mm.Aliases {
				if _, exists := funcMap[alias]; exists {
					panic(alias + " is a duplicate template func")
				}
				funcMap[alias] = mm.Method
			}
		}

		if ns.OnCreated != nil {
			onCreated = append(onCreated, ns.OnCreated)
		}
	}

	for _, f := range onCreated {
		f(nsMap)
	}

	return funcMap
}
