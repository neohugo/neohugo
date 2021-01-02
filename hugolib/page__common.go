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
	"sync"

	"github.com/bep/gitmap"
	"github.com/neohugo/neohugo/common/maps"
	"github.com/neohugo/neohugo/compare"
	"github.com/neohugo/neohugo/lazy"
	"github.com/neohugo/neohugo/navigation"
	"github.com/neohugo/neohugo/output"
	"github.com/neohugo/neohugo/resources/page"
	"github.com/neohugo/neohugo/resources/resource"
)

type treeRefProvider interface {
	getTreeRef() *contentTreeRef
}

func (p *pageCommon) getTreeRef() *contentTreeRef {
	return p.treeRef
}

type nextPrevProvider interface {
	getNextPrev() *nextPrev
}

func (p *pageCommon) getNextPrev() *nextPrev {
	return p.posNextPrev
}

type nextPrevInSectionProvider interface {
	getNextPrevInSection() *nextPrev
}

func (p *pageCommon) getNextPrevInSection() *nextPrev {
	return p.posNextPrevSection
}

type pageCommon struct {
	s *Site
	m *pageMeta

	bucket  *pagesMapBucket //nolint
	treeRef *contentTreeRef

	// Lazily initialized dependencies.
	init *lazy.Init

	// All of these represents the common parts of a page.Page
	maps.Scratcher
	navigation.PageMenusProvider
	page.AuthorProvider
	page.PageRenderProvider
	page.AlternativeOutputFormatsProvider
	page.ChildCareProvider
	page.FileProvider
	page.GetPageProvider
	page.GitInfoProvider
	page.InSectionPositioner
	page.OutputFormatsProvider
	page.PageMetaProvider
	page.Positioner
	page.RawContentProvider
	page.RelatedKeywordsProvider
	page.RefProvider
	page.ShortcodeInfoProvider
	page.SitesProvider
	page.DeprecatedWarningPageMethods
	page.TranslationsProvider
	page.TreeProvider
	resource.LanguageProvider
	resource.ResourceDataProvider
	resource.ResourceMetaProvider
	resource.ResourceParamsProvider
	resource.ResourceTypeProvider
	resource.MediaTypeProvider
	resource.TranslationKeyProvider
	compare.Eqer

	// Describes how paths and URLs for this page and its descendants
	// should look like.
	targetPathDescriptor page.TargetPathDescriptor //nolint
	layoutDescriptor     output.LayoutDescriptor   //nolint
	layoutDescriptorInit sync.Once                 //nolint

	// The parsed page content.
	pageContent

	// Set if feature enabled and this is in a Git repo.
	gitInfo *gitmap.GitInfo //nolint

	// Positional navigation
	posNextPrev        *nextPrev
	posNextPrevSection *nextPrev

	// Menus
	pageMenus *pageMenus //nolint

	// Internal use
	page.InternalDependencies

	// The children. Regular pages will have none.
	*pagePages

	// Any bundled resources
	resources            resource.Resources //nolint
	resourcesInit        sync.Once          //nolint
	resourcesPublishInit sync.Once          //nolint

	translations    page.Pages //nolint
	allTranslations page.Pages //nolint

	// Calculated an cached translation mapping key
	translationKey     string    //nolint
	translationKeyInit sync.Once //nolint

	// Will only be set for bundled pages.
	parent *pageState //nolint

	// Set in fast render mode to force render a given page.
	forceRender bool //nolint
}

type pagePages struct {
	pagesInit sync.Once  //nolint
	pages     page.Pages //nolint

	regularPagesInit          sync.Once  //nolint
	regularPages              page.Pages //nolint
	regularPagesRecursiveInit sync.Once  //nolint
	regularPagesRecursive     page.Pages //nolint
}
