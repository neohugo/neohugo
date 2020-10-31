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

// Package templates contains functions for template processing of Resource objects.
package templates

import (
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/resources"
	"github.com/neohugo/neohugo/resources/internal"
	"github.com/neohugo/neohugo/resources/resource"
	"github.com/neohugo/neohugo/tpl"
	"github.com/pkg/errors"
)

// Client contains methods to perform template processing of Resource objects.
type Client struct {
	rs *resources.Spec
	t  tpl.TemplatesProvider
}

// New creates a new Client with the given specification.
func New(rs *resources.Spec, t tpl.TemplatesProvider) *Client {
	if rs == nil {
		panic("must provice a resource Spec")
	}
	if t == nil {
		panic("must provide a template provider")
	}
	return &Client{rs: rs, t: t}
}

type executeAsTemplateTransform struct {
	rs         *resources.Spec
	t          tpl.TemplatesProvider
	targetPath string
	data       interface{}
}

func (t *executeAsTemplateTransform) Key() internal.ResourceTransformationKey {
	return internal.NewResourceTransformationKey("execute-as-template", t.targetPath)
}

func (t *executeAsTemplateTransform) Transform(ctx *resources.ResourceTransformationCtx) error {
	tplStr, err := helpers.ReaderToString(ctx.From)
	if err != nil {
		return errors.Wrapf(err, "failed to read to string %q as Template:", ctx.InPath)
	}
	templ, err := t.t.TextTmpl().Parse(ctx.InPath, tplStr)
	if err != nil {
		return errors.Wrapf(err, "failed to parse Resource %q as Template:", ctx.InPath)
	}

	ctx.OutPath = t.targetPath

	return t.t.Tmpl().Execute(templ, ctx.To, t.data)
}

func (c *Client) ExecuteAsTemplate(res resources.ResourceTransformer, targetPath string, data interface{}) (resource.Resource, error) {
	return res.Transform(&executeAsTemplateTransform{
		rs:         c.rs,
		targetPath: helpers.ToSlashTrimLeading(targetPath),
		t:          c.t,
		data:       data,
	})
}
