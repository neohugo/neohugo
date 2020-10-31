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

package collections

import (
	"reflect"
	"testing"

	qt "github.com/frankban/quicktest"
	"github.com/neohugo/neohugo/deps"
)

// Also see tests in common/collection.
func TestAppend(t *testing.T) {
	t.Parallel()
	c := qt.New(t)

	ns := New(&deps.Deps{})

	for i, test := range []struct {
		start    interface{}
		addend   []interface{}
		expected interface{}
	}{
		{[]string{"a", "b"}, []interface{}{"c"}, []string{"a", "b", "c"}},
		{[]string{"a", "b"}, []interface{}{"c", "d", "e"}, []string{"a", "b", "c", "d", "e"}},
		{[]string{"a", "b"}, []interface{}{[]string{"c", "d", "e"}}, []string{"a", "b", "c", "d", "e"}},
		// Errors
		{"", []interface{}{[]string{"a", "b"}}, false},
		{[]string{"a", "b"}, []interface{}{}, false},
		// No string concatenation.
		{"ab",
			[]interface{}{"c"},
			false},
	} {

		errMsg := qt.Commentf("[%d]", i)

		args := append(test.addend, test.start)

		result, err := ns.Append(args...)

		if b, ok := test.expected.(bool); ok && !b {
			c.Assert(err, qt.Not(qt.IsNil), errMsg)
			continue
		}

		c.Assert(err, qt.IsNil, errMsg)

		if !reflect.DeepEqual(test.expected, result) {
			t.Fatalf("%s got\n%T: %v\nexpected\n%T: %v", errMsg, result, result, test.expected, test.expected)
		}
	}

}
