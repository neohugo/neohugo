// Copyright 2017-present The Hugo Authors. All rights reserved.
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

package tplimpl

import (
	"reflect"
	"strings"

	"github.com/neohugo/neohugo/tpl"

	"github.com/neohugo/neohugo/common/maps"

	template "github.com/neohugo/neohugo/tpl/internal/go_templates/htmltemplate"
	texttemplate "github.com/neohugo/neohugo/tpl/internal/go_templates/texttemplate"

	"github.com/neohugo/neohugo/deps"

	"github.com/neohugo/neohugo/tpl/internal"

	// Init the namespaces
	_ "github.com/neohugo/neohugo/tpl/cast"
	_ "github.com/neohugo/neohugo/tpl/collections"
	_ "github.com/neohugo/neohugo/tpl/compare"
	_ "github.com/neohugo/neohugo/tpl/crypto"
	_ "github.com/neohugo/neohugo/tpl/data"
	_ "github.com/neohugo/neohugo/tpl/debug"
	_ "github.com/neohugo/neohugo/tpl/encoding"
	_ "github.com/neohugo/neohugo/tpl/fmt"
	_ "github.com/neohugo/neohugo/tpl/hugo"
	_ "github.com/neohugo/neohugo/tpl/images"
	_ "github.com/neohugo/neohugo/tpl/inflect"
	_ "github.com/neohugo/neohugo/tpl/js"
	_ "github.com/neohugo/neohugo/tpl/lang"
	_ "github.com/neohugo/neohugo/tpl/math"
	_ "github.com/neohugo/neohugo/tpl/openapi/openapi3"
	_ "github.com/neohugo/neohugo/tpl/os"
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

var _ texttemplate.ExecHelper = (*templateExecHelper)(nil)
var zero reflect.Value

type templateExecHelper struct {
	running bool // whether we're in server mode.
	funcs   map[string]reflect.Value
}

func (t *templateExecHelper) GetFunc(tmpl texttemplate.Preparer, name string) (reflect.Value, bool) {
	if fn, found := t.funcs[name]; found {
		return fn, true
	}
	return zero, false
}

func (t *templateExecHelper) GetMapValue(tmpl texttemplate.Preparer, receiver, key reflect.Value) (reflect.Value, bool) {
	if params, ok := receiver.Interface().(maps.Params); ok {
		// Case insensitive.
		keystr := strings.ToLower(key.String())
		v, found := params[keystr]
		if !found {
			return zero, false
		}
		return reflect.ValueOf(v), true
	}

	v := receiver.MapIndex(key)

	return v, v.IsValid()
}

func (t *templateExecHelper) GetMethod(tmpl texttemplate.Preparer, receiver reflect.Value, name string) (method reflect.Value, firstArg reflect.Value) {
	if t.running {
		// This is a hot path and receiver.MethodByName really shows up in the benchmarks,
		// so we maintain a list of method names with that signature.
		switch name {
		case "GetPage", "Render":
			if info, ok := tmpl.(tpl.Info); ok {
				if m := receiver.MethodByName(name + "WithTemplateInfo"); m.IsValid() {
					return m, reflect.ValueOf(info)
				}
			}
		}
	}

	return receiver.MethodByName(name), zero
}

func newTemplateExecuter(d *deps.Deps) (texttemplate.Executer, map[string]reflect.Value) {
	funcs := createFuncMap(d)
	funcsv := make(map[string]reflect.Value)

	for k, v := range funcs {
		vv := reflect.ValueOf(v)
		funcsv[k] = vv
	}

	// Duplicate Go's internal funcs here for faster lookups.
	for k, v := range template.GoFuncs {
		if _, exists := funcsv[k]; !exists {
			vv, ok := v.(reflect.Value)
			if !ok {
				vv = reflect.ValueOf(v)
			}
			funcsv[k] = vv
		}
	}

	for k, v := range texttemplate.GoFuncs {
		if _, exists := funcsv[k]; !exists {
			funcsv[k] = v
		}
	}

	exeHelper := &templateExecHelper{
		running: d.Running,
		funcs:   funcsv,
	}

	return texttemplate.NewExecuter(
		exeHelper,
	), funcsv
}

func createFuncMap(d *deps.Deps) map[string]interface{} {

	funcMap := template.FuncMap{}

	// Merge the namespace funcs
	for _, nsf := range internal.TemplateFuncsNamespaceRegistry {
		ns := nsf(d)
		if _, exists := funcMap[ns.Name]; exists {
			panic(ns.Name + " is a duplicate template func")
		}
		funcMap[ns.Name] = ns.Context
		for _, mm := range ns.MethodMappings {
			for _, alias := range mm.Aliases {
				if _, exists := funcMap[alias]; exists {
					panic(alias + " is a duplicate template func")
				}
				funcMap[alias] = mm.Method
			}
		}
	}

	if d.OverloadedTemplateFuncs != nil {
		for k, v := range d.OverloadedTemplateFuncs {
			funcMap[k] = v
		}
	}

	return funcMap

}
