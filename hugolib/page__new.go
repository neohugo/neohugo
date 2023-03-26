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
	"html/template"
	"strings"

	"go.uber.org/atomic"

	"github.com/neohugo/neohugo/common/neohugo"

	"github.com/neohugo/neohugo/common/maps"

	"github.com/neohugo/neohugo/output"

	"github.com/neohugo/neohugo/lazy"

	"github.com/neohugo/neohugo/resources/page"
)

func newPageBase(metaProvider *pageMeta) (*pageState, error) {
	if metaProvider.s == nil {
		panic("must provide a Site")
	}

	s := metaProvider.s

	ps := &pageState{
		pageOutput:                        nopPageOutput,
		pageOutputTemplateVariationsState: atomic.NewUint32(0),
		pageCommon: &pageCommon{
			FileProvider:            metaProvider,
			AuthorProvider:          metaProvider,
			Scratcher:               maps.NewScratcher(),
			store:                   maps.NewScratch(),
			Positioner:              page.NopPage,
			InSectionPositioner:     page.NopPage,
			ResourceMetaProvider:    metaProvider,
			ResourceParamsProvider:  metaProvider,
			PageMetaProvider:        metaProvider,
			RelatedKeywordsProvider: metaProvider,
			OutputFormatsProvider:   page.NopPage,
			ResourceTypeProvider:    pageTypesProvider,
			MediaTypeProvider:       pageTypesProvider,
			RefProvider:             page.NopPage,
			ShortcodeInfoProvider:   page.NopPage,
			LanguageProvider:        s,
			pagePages:               &pagePages{},

			InternalDependencies: s,
			init:                 lazy.New(),
			m:                    metaProvider,
			s:                    s,
		},
	}

	ps.shortcodeState = newShortcodeHandler(ps, ps.s)

	siteAdapter := pageSiteAdapter{s: s, p: ps}

	ps.pageMenus = &pageMenus{p: ps}
	ps.PageMenusProvider = ps.pageMenus
	ps.GetPageProvider = siteAdapter
	ps.GitInfoProvider = ps
	ps.TranslationsProvider = ps
	ps.ResourceDataProvider = &pageData{pageState: ps}
	ps.RawContentProvider = ps
	ps.ChildCareProvider = ps
	ps.TreeProvider = pageTree{p: ps}
	ps.Eqer = ps
	ps.TranslationKeyProvider = ps
	ps.ShortcodeInfoProvider = ps
	ps.AlternativeOutputFormatsProvider = ps

	return ps, nil
}

func newPageBucket(p *pageState) *pagesMapBucket {
	return &pagesMapBucket{owner: p, pagesMapBucketPages: &pagesMapBucketPages{}}
}

func newPageFromMeta(
	n *contentNode,
	parentBucket *pagesMapBucket,
	meta map[string]any,
	metaProvider *pageMeta,
) (*pageState, error) {
	if metaProvider.f == nil {
		metaProvider.f = page.NewZeroFile(metaProvider.s.LogDistinct)
	}

	ps, err := newPageBase(metaProvider)
	if err != nil {
		return nil, err
	}

	bucket := parentBucket

	if ps.IsNode() {
		ps.bucket = newPageBucket(ps)
	}

	if meta != nil || parentBucket != nil {
		if err := metaProvider.setMetadata(bucket, ps, meta); err != nil {
			return nil, ps.wrapError(err)
		}
	}

	if err := metaProvider.applyDefaultValues(n); err != nil {
		return nil, err
	}

	ps.init.Add(func() (any, error) {
		pp, err := newPagePaths(metaProvider.s, ps, metaProvider)
		if err != nil {
			return nil, err
		}

		makeOut := func(f output.Format, render bool) *pageOutput {
			return newPageOutput(ps, pp, f, render)
		}

		shouldRenderPage := !ps.m.noRender()

		if ps.m.standalone {
			ps.pageOutput = makeOut(ps.m.outputFormats()[0], shouldRenderPage)
		} else {
			outputFormatsForPage := ps.m.outputFormats()

			// Prepare output formats for all sites.
			// We do this even if this page does not get rendered on
			// its own. It may be referenced via .Site.GetPage and
			// it will then need an output format.
			ps.pageOutputs = make([]*pageOutput, len(ps.s.h.renderFormats))
			created := make(map[string]*pageOutput)
			for i, f := range ps.s.h.renderFormats {
				po, found := created[f.Name]
				if !found {
					render := shouldRenderPage
					if render {
						_, render = outputFormatsForPage.GetByName(f.Name)
					}
					po = makeOut(f, render)
					created[f.Name] = po
				}
				ps.pageOutputs[i] = po
			}
		}

		if err := ps.initCommonProviders(pp); err != nil {
			return nil, err
		}

		return nil, nil
	})

	return ps, err
}

// Used by the legacy 404, sitemap and robots.txt rendering
func newPageStandalone(m *pageMeta, f output.Format) (*pageState, error) {
	m.configuredOutputFormats = output.Formats{f}
	m.standalone = true
	p, err := newPageFromMeta(nil, nil, nil, m)
	if err != nil {
		return nil, err
	}

	if err := p.initPage(); err != nil {
		return nil, err
	}

	return p, nil
}

// nolint
type pageDeprecatedWarning struct {
	p *pageState
}

// nolint
func (p *pageDeprecatedWarning) IsDraft() bool { return p.p.m.draft }

// nolint
func (p *pageDeprecatedWarning) Hugo() neohugo.Info { return p.p.s.Info.Hugo() }

// nolint
func (p *pageDeprecatedWarning) LanguagePrefix() string { return p.p.s.Info.LanguagePrefix }

// nolint
func (p *pageDeprecatedWarning) GetParam(key string) any {
	return p.p.m.params[strings.ToLower(key)]
}

// nolint
func (p *pageDeprecatedWarning) RSSLink() template.URL {
	f := p.p.OutputFormats().Get("RSS")
	if f == nil {
		return ""
	}
	return template.URL(f.Permalink())
}

// nolint
func (p *pageDeprecatedWarning) URL() string {
	if p.p.IsPage() && p.p.m.urlPaths.URL != "" {
		// This is the url set in front matter
		return p.p.m.urlPaths.URL
	}
	// Fall back to the relative permalink.
	return p.p.RelPermalink()
}
