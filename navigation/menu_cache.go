// Copyright 2021 The Hugo Authors. All rights reserved.
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

package navigation

import (
	"sync"
)

type menuCacheEntry struct {
	in  []Menu
	out Menu
}

func (entry menuCacheEntry) matches(menuList []Menu) bool {
	if len(entry.in) != len(menuList) {
		return false
	}
	for i, m := range menuList {
		if !menuEqual(m, entry.in[i]) {
			return false
		}
	}

	return true
}

func newMenuCache() *menuCache {
	return &menuCache{m: make(map[string][]menuCacheEntry)}
}

//func (c *menuCache) clear() {
//c.Lock()
//defer c.Unlock()
//c.m = make(map[string][]menuCacheEntry)
//}

type menuCache struct {
	sync.RWMutex
	m map[string][]menuCacheEntry
}

func menuEqual(m1, m2 Menu) bool {
	if m1 == nil && m2 == nil {
		return true
	}

	if m1 == nil || m2 == nil {
		return false
	}

	if len(m1) != len(m2) {
		return false
	}

	if len(m1) == 0 {
		return true
	}

	for i := 0; i < len(m1); i++ {
		if m1[i] != m2[i] {
			return false
		}
	}
	return true
}

func (c *menuCache) get(key string, apply func(m Menu), menuLists ...Menu) (Menu, bool) {
	return c.getP(key, func(m *Menu) {
		if apply != nil {
			apply(*m)
		}
	}, menuLists...)
}

func (c *menuCache) getP(key string, apply func(m *Menu), menuLists ...Menu) (Menu, bool) {
	c.Lock()
	defer c.Unlock()

	if cached, ok := c.m[key]; ok {
		for _, entry := range cached {
			if entry.matches(menuLists) {
				return entry.out, true
			}
		}
	}

	m := menuLists[0]
	menuCopy := append(Menu(nil), m...)

	if apply != nil {
		apply(&menuCopy)
	}

	entry := menuCacheEntry{in: menuLists, out: menuCopy}
	if v, ok := c.m[key]; ok {
		c.m[key] = append(v, entry)
	} else {
		c.m[key] = []menuCacheEntry{entry}
	}

	return menuCopy, false
}
