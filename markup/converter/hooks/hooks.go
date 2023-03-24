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

package hooks

import (
	"io"

	"github.com/neohugo/neohugo/common/hugio"
	"github.com/neohugo/neohugo/common/text"
	"github.com/neohugo/neohugo/common/types/hstring"
	"github.com/neohugo/neohugo/identity"
	"github.com/neohugo/neohugo/markup/internal/attributes"
)

var _ AttributesOptionsSliceProvider = (*attributes.AttributesHolder)(nil)

type AttributesProvider interface {
	// Attributes passed in from Markdown (e.g. { attrName1=attrValue1 attrName2="attr Value 2" }).
	Attributes() map[string]any
}

type LinkContext interface {
	// The Page being rendered.
	Page() any

	// The link URL.
	Destination() string

	// The link title attribute.
	Title() string

	// The rendered (HTML) text.
	Text() hstring.RenderedString

	// The plain variant of Text.
	PlainText() string
}

type ImageLinkContext interface {
	LinkContext

	// Returns true if this is a standalone image and the config option
	// markup.goldmark.parser.wrapStandAloneImageWithinParagraph is disabled.
	IsBlock() bool

	// Zero-based ordinal for all the images in the current document.
	Ordinal() int
}

// CodeblockContext is the context passed to a code block render hook.
type CodeblockContext interface {
	AttributesProvider
	text.Positioner

	// Chroma highlighting processing options. This will only be filled if Type is a known Chroma Lexer.
	Options() map[string]any

	// The type of code block. This will be the programming language, e.g. bash, when doing code highlighting.
	Type() string

	// The text between the code fences.
	Inner() string

	// Zero-based ordinal for all code blocks in the current document.
	Ordinal() int

	// The owning Page.
	Page() any
}

type AttributesOptionsSliceProvider interface {
	AttributesSlice() []attributes.Attribute
	OptionsSlice() []attributes.Attribute
}

type LinkRenderer interface {
	RenderLink(w io.Writer, ctx LinkContext) error
	identity.Provider
}

type CodeBlockRenderer interface {
	RenderCodeblock(w hugio.FlexiWriter, ctx CodeblockContext) error
	identity.Provider
}

type IsDefaultCodeBlockRendererProvider interface {
	IsDefaultCodeBlockRenderer() bool
}

// HeadingContext contains accessors to all attributes that a HeadingRenderer
// can use to render a heading.
type HeadingContext interface {
	// Page is the page containing the heading.
	Page() any
	// Level is the level of the header (i.e. 1 for top-level, 2 for sub-level, etc.).
	Level() int
	// Anchor is the HTML id assigned to the heading.
	Anchor() string
	// Text is the rendered (HTML) heading text, excluding the heading marker.
	Text() hstring.RenderedString
	// PlainText is the unrendered version of Text.
	PlainText() string

	// Attributes (e.g. CSS classes)
	AttributesProvider
}

// HeadingRenderer describes a uniquely identifiable rendering hook.
type HeadingRenderer interface {
	// Render writes the rendered content to w using the data in w.
	RenderHeading(w io.Writer, ctx HeadingContext) error
	identity.Provider
}

// ElementPositionResolver provides a way to resolve the start Position
// of a markdown element in the original source document.
// This may be both slow and approximate, so should only be
// used for error logging.
type ElementPositionResolver interface {
	ResolvePosition(ctx any) text.Position
}

type RendererType int

const (
	LinkRendererType RendererType = iota + 1
	ImageRendererType
	HeadingRendererType
	CodeBlockRendererType
)

type GetRendererFunc func(t RendererType, id any) any
