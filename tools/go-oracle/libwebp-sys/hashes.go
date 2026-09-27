package main

// Re-implementation of neohugo's image target-name hashing, using only
// xxhash + gohugoio/hashstructure (same as common/hashing/hashing.go).

import (
	"image"
	"image/draw"
	"strconv"

	"github.com/cespare/xxhash/v2"
	"github.com/disintegration/gift"
	"github.com/gohugoio/hashstructure"
)

func hashUint64(vs ...any) uint64 {
	var o any
	if len(vs) == 1 {
		o = vs[0]
	} else {
		o = vs // []any
	}
	h, err := hashstructure.Hash(o, &hashstructure.HashOptions{Hasher: xxhash.New()})
	if err != nil {
		panic(err)
	}
	return h
}

// hashing.HashStringHex
func hashStringHex(vs ...any) string { return strconv.FormatUint(hashUint64(vs...), 16) }

// hashing.HashString
func hashString(vs ...any) string { return strconv.FormatUint(hashUint64(vs...), 10) }

// Types mirroring resources/images/filters.go. hashstructure only uses the
// type *name* (not package path) and exported field names, so these produce
// identical hashes.
type filterOpts struct {
	Version int
	Vals    any
}

type filter struct {
	Options filterOpts
	gift.Filter
}

// The original's unexported src field is left out: it is never read here,
// and hashstructure skips unexported fields.
type overlayFilter struct {
	x, y int
}

func (f overlayFilter) Draw(dst draw.Image, src image.Image, options *gift.Options) {}
func (f overlayFilter) Bounds(b image.Rectangle) image.Rectangle {
	return image.Rect(0, 0, b.Dx(), b.Dy())
}

// Key for images.Filter(images.Overlay(wm, 0, 0)): hashing.HashString(gfilters)
func overlayFilterKey(wmKey string, x, y int) string {
	gfilters := []gift.Filter{filter{
		Options: filterOpts{Version: 0, Vals: []any{wmKey, x, y}},
		Filter:  overlayFilter{x: x, y: y},
	}}
	return hashString(gfilters)
}

// DecodeImageConfig Key: hashing.HashStringHex(options) where options is the
// lower-cased, trimmed, non-empty []string (action first).
func processKey(options ...string) string {
	return hashStringHex(options)
}

// relTargetPathFromConfig: HashStringHex(incomingID, sourceHash, conf.Key, imagingCfgSourceHash)
func targetHash(incomingID string, srcHash uint64, confKey, cfgHash string) string {
	return hashStringHex(incomingID, srcHash, confKey, cfgHash)
}

func xxFile(b []byte) uint64 { return xxhash.Sum64(b) }
