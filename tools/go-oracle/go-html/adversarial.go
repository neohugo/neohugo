package main

// Adversarial inputs (-adv N) aimed at the longest-prefix rule of
// html.UnescapeString: every prefix of every entity name in several
// contexts, every no-semicolon entity followed by every byte, entity-name
// soups (glued entity fragments, repeated '&', stray ';'), long numeric
// references (int32 wrap-around) and non-ASCII neighbours.

import (
	"math/rand/v2"
	"strings"
)

func adversarialInputs(e1 []ent1, e2 []ent2, n int, rng *rand.Rand) []string {
	var inputs []string
	add := func(s string) { inputs = append(inputs, s) }

	var names []string
	for _, e := range e1 {
		names = append(names, e.name)
	}
	for _, e := range e2 {
		names = append(names, e.name)
	}
	suffixes := []string{"", ";", "x", "X", "1", "=", "&", " ", "\xc3\xa9", "\xe0\xb8\x81", "\xff", ";;", "&amp;", "#", "0;"}
	for _, nm := range names {
		for k := 1; k <= len(nm); k++ {
			p := nm[:k]
			for _, s := range suffixes {
				add("&" + p + s)
			}
		}
	}
	// Every no-semicolon entity followed by every byte and by digit/letter pairs.
	for _, e := range e1 {
		if strings.HasSuffix(e.name, ";") {
			continue
		}
		for c := 0; c < 256; c++ {
			add("&" + e.name + string([]byte{byte(c)}))
			add("a&" + e.name + string([]byte{byte(c)}) + ";b")
		}
		for _, s := range []string{"aa", "a1", "1a", "Zz", "99", "a;", "1;", "zzzzzzzzz;", "\xe0\xb8\x81;"} {
			add("&" + e.name + s)
		}
	}
	// Long names whose 2..6-byte prefixes are (or are not) entities.
	alnum := "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	for i := 0; i < n; i++ {
		var b strings.Builder
		for k := 1 + rng.IntN(6); k > 0; k-- {
			switch rng.IntN(8) {
			case 0, 1, 2:
				nm := names[rng.IntN(len(names))]
				lo := 0
				if rng.IntN(3) == 0 {
					lo = rng.IntN(len(nm))
				}
				hi := lo + 1 + rng.IntN(len(nm)-lo)
				if rng.IntN(2) == 0 {
					b.WriteByte('&')
				}
				b.WriteString(nm[lo:hi])
			case 3:
				for d := rng.IntN(10); d > 0; d-- {
					b.WriteByte(alnum[rng.IntN(len(alnum))])
				}
			case 4:
				b.WriteString("&#")
				if rng.IntN(2) == 0 {
					b.WriteByte("xX"[rng.IntN(2)])
					for d := rng.IntN(24); d > 0; d-- {
						b.WriteByte("0123456789abcdefABCDEFgG"[rng.IntN(24)])
					}
				} else {
					for d := rng.IntN(30); d > 0; d-- {
						b.WriteByte("0123456789a"[rng.IntN(11)])
					}
				}
				if rng.IntN(2) == 0 {
					b.WriteByte(';')
				}
			case 5:
				b.WriteString([]string{"&", ";", "&&", "&;", "#", "=", " ", "\xe0\xb8\x81", "\xff", "\x00", "<", "'", "\"", ">"}[rng.IntN(14)])
			case 6:
				b.WriteString("&" + names[rng.IntN(len(names))])
			case 7:
				b.WriteString(strings.ToUpper(names[rng.IntN(len(names))][:1]) + names[rng.IntN(len(names))])
			}
		}
		add(b.String())
	}
	return inputs
}
