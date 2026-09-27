package main

import (
	"net/url"
	"regexp"

	"github.com/tdewolff/minify/v2"
	"github.com/tdewolff/minify/v2/css"
	"github.com/tdewolff/minify/v2/html"
	"github.com/tdewolff/minify/v2/json"
	"github.com/tdewolff/minify/v2/svg"
	"github.com/tdewolff/minify/v2/xml"
)

// Regexps registered by neohugo's minifiers.New (minifiers/minifiers.go).
const (
	jsPattern   = "^(application|text)/(x-)?(java|ecma)script$"
	jsonPattern = `^(application|text)/(x-|(ld|manifest)\+)?json$`
)

// seeksnackM mirrors neohugo's minifiers.New with the seeksnack [minify]
// config (effective options as decoded by minifiers.DecodeConfig), but
// WITHOUT any JS minifier: inline scripts and on* handlers pass through
// (ErrNotExist).
func seeksnackM() *minify.M {
	m := minify.New()
	m.Add("text/css", &css.Minifier{Precision: 0, KeepCSS2: true})
	m.Add("application/json", &json.Minifier{})
	m.AddRegexp(regexp.MustCompile(jsonPattern), &json.Minifier{})
	m.Add("image/svg+xml", &svg.Minifier{KeepComments: false, Precision: 0})
	m.Add("application/rss+xml", &xml.Minifier{KeepWhitespace: false})
	m.Add("application/xml", &xml.Minifier{KeepWhitespace: false})
	m.Add("text/html", &html.Minifier{
		KeepDocumentTags:    true,
		KeepSpecialComments: true,
		KeepEndTags:         true,
		KeepDefaultAttrVals: true,
		KeepWhitespace:      false,
	})
	return m
}

// config is one minifier configuration exercised by the fixtures. The Rust
// test builds the identical M from the same name.
type config struct {
	name string
	m    func() *minify.M
}

func withURL(m *minify.M, u string) *minify.M {
	m.URL, _ = url.Parse(u)
	return m
}

// allM registers every minifier (with the given options) the way the
// upstream tests and neohugo do (literal mimetypes + neohugo's regexps),
// except JS.
func allM(h *html.Minifier, c *css.Minifier, j *json.Minifier, s *svg.Minifier, x *xml.Minifier) *minify.M {
	m := minify.New()
	m.Add("text/html", h)
	m.Add("text/css", c)
	m.Add("application/json", j)
	m.AddRegexp(regexp.MustCompile(jsonPattern), j)
	m.Add("image/svg+xml", s)
	m.Add("application/xml", x)
	m.Add("application/rss+xml", x)
	m.AddRegexp(regexp.MustCompile("[/+]xml$"), x)
	return m
}

// configs are the named configurations; the Rust side (tests/common)
// mirrors this list exactly.
var configs = []config{
	{"seeksnack", seeksnackM},
	{"default", func() *minify.M {
		return allM(&html.Minifier{}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{})
	}},
	{"html-keepall", func() *minify.M {
		return allM(&html.Minifier{KeepComments: true, KeepDefaultAttrVals: true, KeepDocumentTags: true, KeepEndTags: true, KeepQuotes: true, KeepWhitespace: true}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{})
	}},
	{"html-endtags", func() *minify.M {
		return allM(&html.Minifier{KeepEndTags: true}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{})
	}},
	{"html-special", func() *minify.M {
		return allM(&html.Minifier{KeepSpecialComments: true}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{})
	}},
	{"html-ws", func() *minify.M {
		return allM(&html.Minifier{KeepWhitespace: true}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{})
	}},
	{"html-quotes", func() *minify.M {
		return allM(&html.Minifier{KeepQuotes: true}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{})
	}},
	{"html-gotmpl", func() *minify.M {
		return allM(&html.Minifier{TemplateDelims: html.GoTemplateDelims}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{})
	}},
	{"html-php", func() *minify.M {
		return allM(&html.Minifier{TemplateDelims: html.PHPTemplateDelims}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{})
	}},
	{"url-http", func() *minify.M {
		return withURL(allM(&html.Minifier{}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{}), "http://example.com/")
	}},
	{"url-https", func() *minify.M {
		return withURL(allM(&html.Minifier{}, &css.Minifier{}, &json.Minifier{}, &svg.Minifier{}, &xml.Minifier{}), "https://example.com/")
	}},
	{"css2-prec3", func() *minify.M {
		return allM(&html.Minifier{}, &css.Minifier{KeepCSS2: true, Precision: 3}, &json.Minifier{Precision: 3}, &svg.Minifier{Precision: 3}, &xml.Minifier{KeepWhitespace: true})
	}},
	{"prec1-inline", func() *minify.M {
		return allM(&html.Minifier{}, &css.Minifier{Precision: 1, Inline: true}, &json.Minifier{KeepNumbers: true}, &svg.Minifier{Precision: 1, Inline: true, KeepComments: true}, &xml.Minifier{})
	}},
}
