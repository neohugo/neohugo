package main

import (
	"bytes"
	"errors"
	"io"
	"regexp"
	"strings"

	"github.com/tdewolff/minify/v2"
	"github.com/tdewolff/minify/v2/css"
	"github.com/tdewolff/minify/v2/html"
	"github.com/tdewolff/minify/v2/json"
	"github.com/tdewolff/minify/v2/svg"
	"github.com/tdewolff/minify/v2/xml"
	"github.com/tdewolff/parse/v2"
)

// errPlain mirrors github.com/tdewolff/test.ErrPlain.
var errPlain = errors.New("error")

// copyFunc is io.Copy (upstream tests' dummy css/js minifiers).
func copyFunc(_ *minify.M, w io.Writer, r io.Reader, _ map[string]string) error {
	_, err := io.Copy(w, r)
	return err
}

func errPlainFunc(_ *minify.M, _ io.Writer, _ io.Reader, _ map[string]string) error {
	return errPlain
}

// trimCopyFunc stands in for a JS minifier: it writes the input with
// ASCII whitespace trimmed.
func trimCopyFunc(_ *minify.M, w io.Writer, r io.Reader, _ map[string]string) error {
	data, err := io.ReadAll(r)
	if err != nil {
		return err
	}
	_, err = w.Write(parse.TrimWhitespace(data))
	return err
}

// errJSFunc returns a *parse.Error positioned in the middle of the input,
// so that minify.UpdateErrorPosition is exercised.
func errJSFunc(_ *minify.M, _ io.Writer, r io.Reader, _ map[string]string) error {
	data, err := io.ReadAll(r)
	if err != nil {
		return err
	}
	return parse.NewError(bytes.NewBuffer(data), len(data)/2, "dummy js error")
}

// specialConfigs are the configurations of the upstream tests and of the
// nested-minifier plumbing checks; tests/common/configs.rs mirrors them.
var specialConfigs = []config{
	{"t-empty", func() *minify.M { return minify.New() }},
	{"t-copycssjs", func() *minify.M {
		m := minify.New()
		m.AddFunc("text/html", html.Minify)
		m.AddFunc("text/css", copyFunc)
		m.AddFunc("application/javascript", copyFunc)
		return m
	}},
	{"t-html", func() *minify.M {
		m := minify.New()
		m.AddFunc("text/html", html.Minify)
		return m
	}},
	{"t-htmlcsssvg", func() *minify.M {
		m := minify.New()
		m.AddFunc("text/html", html.Minify)
		m.AddFunc("text/css", css.Minify)
		m.AddFunc("image/svg+xml", svg.Minify)
		return m
	}},
	{"t-css", func() *minify.M {
		m := minify.New()
		m.AddFunc("text/css", css.Minify)
		return m
	}},
	{"t-datauri", func() *minify.M {
		m := minify.New()
		m.AddFunc("text/x", copyFunc)
		return m
	}},
	{"errplain", func() *minify.M {
		m := minify.New()
		m.Add("text/html", &html.Minifier{KeepSpecialComments: true})
		m.AddFunc("text/css", errPlainFunc)
		m.AddFunc("application/javascript", errPlainFunc)
		m.AddFunc("image/svg+xml", errPlainFunc)
		m.AddFunc("application/mathml+xml", errPlainFunc)
		return m
	}},
	{"svgerr", func() *minify.M {
		m := minify.New()
		m.AddFunc("image/svg+xml", svg.Minify)
		m.AddFunc("text/css", errPlainFunc)
		return m
	}},
	{"dummyjs", func() *minify.M {
		m := seeksnackM()
		m.AddFuncRegexp(regexp.MustCompile(jsPattern), trimCopyFunc)
		return m
	}},
	{"errjs", func() *minify.M {
		m := seeksnackM()
		m.AddFuncRegexp(regexp.MustCompile(jsPattern), errJSFunc)
		return m
	}},
	{"xml-json-rx", func() *minify.M {
		// upstream Example wiring (without js): regexps for json and xml
		m := minify.New()
		m.AddFunc("text/html", html.Minify)
		m.AddFunc("text/css", css.Minify)
		m.AddFunc("image/svg+xml", svg.Minify)
		m.AddFuncRegexp(regexp.MustCompile("[/+]json$"), json.Minify)
		m.AddFuncRegexp(regexp.MustCompile("[/+]xml$"), xml.Minify)
		return m
	}},
}

func configByName(name string) *minify.M {
	if strings.HasPrefix(name, "o:") {
		return dynM(name) // redteam.go
	}
	for _, c := range configs {
		if c.name == name {
			return c.m()
		}
	}
	for _, c := range specialConfigs {
		if c.name == name {
			return c.m()
		}
	}
	panic("unknown config " + name)
}
