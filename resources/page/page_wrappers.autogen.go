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

// This file is autogenerated.

package page

import (
	"github.com/neohugo/neohugo/common/hugo"
	"github.com/neohugo/neohugo/helpers"
	"github.com/neohugo/neohugo/hugofs"
	"html/template"
)

// NewDeprecatedWarningPage adds deprecation warnings to the given implementation.
func NewDeprecatedWarningPage(p DeprecatedWarningPageMethods) DeprecatedWarningPageMethods {
	return &pageDeprecated{p: p}
}

type pageDeprecated struct {
	p DeprecatedWarningPageMethods
}

func (p *pageDeprecated) Filename() string {
	helpers.Deprecated("Page.Filename", "Use .File.Filename", false)
	return p.p.Filename()
}
func (p *pageDeprecated) Dir() string {
	helpers.Deprecated("Page.Dir", "Use .File.Dir", false)
	return p.p.Dir()
}
func (p *pageDeprecated) IsDraft() bool {
	helpers.Deprecated("Page.IsDraft", "Use .Draft.", false)
	return p.p.IsDraft()
}
func (p *pageDeprecated) Extension() string {
	helpers.Deprecated("Page.Extension", "Use .File.Extension", false)
	return p.p.Extension()
}
func (p *pageDeprecated) Hugo() hugo.Info {
	helpers.Deprecated("Page.Hugo", "Use the global hugo function.", false)
	return p.p.Hugo()
}
func (p *pageDeprecated) Ext() string {
	helpers.Deprecated("Page.Ext", "Use .File.Ext", false)
	return p.p.Ext()
}
func (p *pageDeprecated) LanguagePrefix() string {
	helpers.Deprecated("Page.LanguagePrefix", "Use .Site.LanguagePrefix.", false)
	return p.p.LanguagePrefix()
}
func (p *pageDeprecated) GetParam(arg0 string) interface{} {
	helpers.Deprecated("Page.GetParam", "Use .Param or .Params.myParam.", false)
	return p.p.GetParam(arg0)
}
func (p *pageDeprecated) LogicalName() string {
	helpers.Deprecated("Page.LogicalName", "Use .File.LogicalName", false)
	return p.p.LogicalName()
}
func (p *pageDeprecated) BaseFileName() string {
	helpers.Deprecated("Page.BaseFileName", "Use .File.BaseFileName", false)
	return p.p.BaseFileName()
}
func (p *pageDeprecated) RSSLink() template.URL {
	helpers.Deprecated("Page.RSSLink", "Use the Output Format's link, e.g. something like: \n    {{ with .OutputFormats.Get \"RSS\" }}{{ .RelPermalink }}{{ end }}", false)
	return p.p.RSSLink()
}
func (p *pageDeprecated) TranslationBaseName() string {
	helpers.Deprecated("Page.TranslationBaseName", "Use .File.TranslationBaseName", false)
	return p.p.TranslationBaseName()
}
func (p *pageDeprecated) URL() string {
	helpers.Deprecated("Page.URL", "Use .Permalink or .RelPermalink. If what you want is the front matter URL value, use .Params.url", false)
	return p.p.URL()
}
func (p *pageDeprecated) ContentBaseName() string {
	helpers.Deprecated("Page.ContentBaseName", "Use .File.ContentBaseName", false)
	return p.p.ContentBaseName()
}
func (p *pageDeprecated) UniqueID() string {
	helpers.Deprecated("Page.UniqueID", "Use .File.UniqueID", false)
	return p.p.UniqueID()
}
func (p *pageDeprecated) FileInfo() hugofs.FileMetaInfo {
	helpers.Deprecated("Page.FileInfo", "Use .File.FileInfo", false)
	return p.p.FileInfo()
}
