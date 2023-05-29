// Copyright 2017 The Hugo Authors. All rights reserved.
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

package urls

import (
	"html/template"
	"net/url"
	"testing"

	"github.com/neohugo/neohugo/config/testconfig"
	"github.com/neohugo/neohugo/htesting/hqt"
	"github.com/spf13/cast"

	qt "github.com/frankban/quicktest"
)

func newNs() *Namespace {
	return New(testconfig.GetTestDeps(nil, nil))
}

type tstNoStringer struct{}

func TestParse(t *testing.T) {
	t.Parallel()
	c := qt.New(t)
	ns := newNs()

	for _, test := range []struct {
		rawurl any
		expect any
	}{
		{
			"http://www.google.com",
			&url.URL{
				Scheme: "http",
				Host:   "www.google.com",
			},
		},
		{
			"http://j@ne:password@google.com",
			&url.URL{
				Scheme: "http",
				User:   url.UserPassword("j@ne", "password"),
				Host:   "google.com",
			},
		},
		// errors
		{tstNoStringer{}, false},
	} {

		result, err := ns.Parse(test.rawurl)

		if b, ok := test.expect.(bool); ok && !b {
			c.Assert(err, qt.Not(qt.IsNil))
			continue
		}

		c.Assert(err, qt.IsNil)
		c.Assert(result,
			qt.CmpEquals(hqt.DeepAllowUnexported(&url.URL{}, url.Userinfo{})), test.expect)
	}
}

func TestJoinPath(t *testing.T) {
	t.Parallel()
	c := qt.New(t)
	ns := newNs()

	for _, test := range []struct {
		elements any
		expect   any
	}{
		{"", `/`},
		{"a", `a`},
		{"/a/b", `/a/b`},
		{"./../a/b", `a/b`},
		{[]any{""}, `/`},
		{[]any{"a"}, `a`},
		{[]any{"/a", "b"}, `/a/b`},
		{[]any{".", "..", "/a", "b"}, `a/b`},
		{[]any{"https://example.org", "a"}, `https://example.org/a`},
		{[]any{nil}, `/`},
		// errors
		{tstNoStringer{}, false},
		{[]any{tstNoStringer{}}, false},
	} {

		result, err := ns.JoinPath(test.elements)

		if b, ok := test.expect.(bool); ok && !b {
			c.Assert(err, qt.Not(qt.IsNil))
			continue
		}

		c.Assert(err, qt.IsNil)
		c.Assert(result, qt.Equals, test.expect)
	}
}

func TestURLEncode(t *testing.T) {
	t.Parallel()
	c := qt.New(t)

	for _, test := range []struct {
		rawurl interface{}
		expect interface{}
	}{
		{
			"นีโอฮยูโก",
			"%E0%B8%99%E0%B8%B5%E0%B9%82%E0%B8%AD%E0%B8%AE%E0%B8%A2%E0%B8%B9%E0%B9%82%E0%B8%81",
		},
		// errors
		{tstNoStringer{}, false},
	} {
		ns := newNs()
		result, err := ns.URLEncode(test.rawurl)

		if b, ok := test.expect.(bool); ok && !b {
			c.Assert(err, qt.Not(qt.IsNil))
			continue
		}

		c.Assert(err, qt.IsNil)
		c.Assert(result, qt.DeepEquals, template.HTML(cast.ToString((test.expect))))
	}
}

func TestURLDecode(t *testing.T) {
	t.Parallel()
	c := qt.New(t)

	for _, test := range []struct {
		rawurl interface{}
		expect interface{}
	}{
		{
			"http://neohugo.github.io/%E0%B8%99%E0%B8%B5%E0%B9%82%E0%B8%AD%E0%B8%AE%E0%B8%A2%E0%B8%B9%E0%B9%82%E0%B8%81",
			"http://neohugo.github.io/นีโอฮยูโก",
		},
		// errors
		{tstNoStringer{}, false},
	} {
		ns := newNs()
		result, err := ns.URLDecode(test.rawurl)

		if b, ok := test.expect.(bool); ok && !b {
			c.Assert(err, qt.Not(qt.IsNil))
			continue
		}

		c.Assert(err, qt.IsNil)
		c.Assert(result, qt.DeepEquals, template.HTML(cast.ToString((test.expect))))
	}
}
