package main

import (
	"reflect"
	"regexp"
	"time"

	"github.com/neohugo/neohugo/resources/page"
)

// hasPtr reports whether v holds a value whose printed form can include a
// memory address (fmt prints pointers nested in slices and maps, and the
// *time.Location of a time's internals, as addresses): a test object, a
// page, or a time outside UTC.
func hasPtr(v any) bool {
	switch x := v.(type) {
	case nil:
		return false
	case *tstObj, page.Page:
		return true
	case time.Time:
		return x.Location() != time.UTC
	}
	rv := reflect.ValueOf(v)
	switch rv.Kind() {
	case reflect.Slice:
		for i := 0; i < rv.Len(); i++ {
			if hasPtr(rv.Index(i).Interface()) {
				return true
			}
		}
	case reflect.Map:
		for _, k := range rv.MapKeys() {
			if hasPtr(rv.MapIndex(k).Interface()) {
				return true
			}
		}
	}
	return false
}

var addrRe = regexp.MustCompile(`0x[0-9a-f]{6,}`)

// maskAddrs replaces addresses in the error text and in the string values
// of an encoded result with 0xADDR (the Rust test does the same).
func maskAddrs(v any) {
	switch x := v.(type) {
	case map[string]any:
		for k, e := range x {
			if s, ok := e.(string); ok && (k == "err" || k == "s") {
				x[k] = addrRe.ReplaceAllString(s, "0xADDR")
				continue
			}
			maskAddrs(e)
		}
	case []any:
		for _, e := range x {
			maskAddrs(e)
		}
	}
}
