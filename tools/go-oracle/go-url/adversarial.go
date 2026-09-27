package main

// Adversarial generator (-adv N): URLs assembled from atoms that stress the
// escaping rules (%, #, ?, Thai and other non-ASCII text, spaces, ';', '@',
// ':', brackets, invalid UTF-8), IPv6 literals built piece by piece (zones,
// embedded IPv4, ellipses, over-long groups), round trips through
// URL.String(), resolution against odd bases, JoinPath with odd elements and
// URL structs whose RawPath/RawFragment disagree with Path/Fragment.

import (
	"net/url"
	"strconv"
	"strings"
)

var (
	advSchemes = []string{"", "", "", "http:", "https:", "HTTPS:", "hTtP:", "ftp:", "mailto:", "urn:", "file:",
		"javascript:", "a1+.-:", "1a:", "-a:", "+a:", ".a:", "a_b:", "a%41:", ":", "x:", "data:", "tel:"}
	advAtoms = []string{"a", "Z", "0", "9", "-", "_", ".", "~", "..", ".", "/", "/", "/", "//", "%", "%2", "%25",
		"%2F", "%2f", "%3F", "%3f", "%23", "%20", "%E0%B8%81", "%e0%b8%81", "%E0", "%zz", "%0", "%00", "%7F",
		"%80", "%FF", "%3A", "%40", "%3B", "%2B", "%2C", "%5B", "%5D", "%7E", "%41", "%%", "%%41", "%g1",
		" ", "  ", "+", ";", "@", ":", "=", "&", "$", ",", "!", "*", "'", "(", ")", "[", "]", "?", "#", "\\",
		"^", "`", "{", "}", "|", "<", ">", "\"", "ไทย", "ก", "กิ", "é", "€", "😀", " ", "​",
		"\xff", "\xe0\xb8", "\xc3", "\x80", "\xed\xa0\x80", "\t", "\x7f", "\x00", "\r\n",
		"index.html", "p", "seeksnack", "th", "ingredients", "a b", "a+b", "a;b=c", "k=v&k2=v2"}
	advUsers = []string{"", "", "", "u@", "u:p@", "u:@", ":p@", "@", ":@", "u@v@", "u:p@q@", "%41@", "%zz@",
		"%@", "u%3Ap@", "u:p%3A@", "ไทย@", "u:ไทย@", "u p@", "u:p:q@", "u;p@", "u!$&'()*+,;=@", "u~_.-@",
		"%E0%B8%81:%E0%B8%81@", "u%40h@", "u[x]@", "u\"@", "u%2Fp@", "u%00@", "%25@"}
	advHosts = []string{"", "example.com", "EXAMPLE.COM", "ex%41mple.com", "ex%E0%B8%81.com", "ไทย.com",
		"xn--o3cw4h.com", "1.2.3.4", "1.2.3.4:80", "h:0", "h:", "h:65536", "h:+1", "h:-1", "h: 1", "h:1:2",
		"a:b:c", "h::80", ":80", ":", "[::1]", "[::1]:", "[::1]:80", "[::1]80", "[::1]:8a", "[::ffff:1.2.3.4]",
		"[::ffff:1.2.3.4]:443", "[1:2:3:4:5:6:7:8]", "[1:2:3:4:5:6:7::]", "[::1:2:3:4:5:6:7]", "[1::]",
		"[fe80::1%25en0]", "[fe80::1%25en0]:8080", "[fe80::1%25%E0%B8%81]", "[fe80::1%25%20]", "[fe80::1%25%zz]",
		"[fe80::1%25]", "[fe80::1%en0]", "[fe80::%2541]", "[::1%2525]", "[v7.x]", "[1.2.3.4]", "[::1", "::1]",
		"[[::1]]", "x[::1]", "[::1]]", "[]", "[:]", "[:::]", "[::1::]", "[0:0:0:0:0:0:0:0:0]", "[12345::]",
		"[::1.2.3.256]", "[::01.2.3.4]", "[::1.2.3]", "[::1.2.3.4.5]", "[a:b:c:d:e:f:1.2.3.4]",
		"[a:b:c:d:e:f:g:h]", "[A:B::C]", "h%2", "h%", "%25", "%41", "%80", "h<>", "h\"", "h{}", "h|", "h ",
		"h\\", "h^", "h`", "!$&'()*+,;=", "_-.~", "h%E2%82%AC", "h%e2%82%ac", "h%C0%AF", "ไทย:80", "h:๘๐",
		"[fe80::1%25en0%25]", "[fe80::1%25a%25b]", "[::1%2541%2542]", "[::%25x]:1", "[::1]:1:2", "h:80]",
		"[::1]%41", "[::1]:%38%30"}
	advBases = []string{"http://a/b/c/d;p?q", "https://seeksnack.com/th/ingredients/", "http://[::1]:80/a/",
		"mailto:x@y", "/rel/path", "rel", "", "http://h", "http://h?q#f", "file:///", "http://u:p@h/a%2Fb/c",
		"https://seeksnack.com/th/%E0%B8%81/", "http://h/ไทย/x", "http://h/a%20b/", "//h/p", "?q", "#f",
		"http://h/a/b/../c/./d", "http:opaque", "http:///p"}
)

func (g gen) atoms(max int) string {
	var b strings.Builder
	for k := g.r.IntN(max + 1); k > 0; k-- {
		b.WriteString(g.pick(advAtoms))
	}
	return b.String()
}

// ipv6 builds a random (often invalid) IPv6 literal body.
func (g gen) ipv6() string {
	var b strings.Builder
	groups := g.r.IntN(10)
	ell := -1
	if g.r.IntN(2) == 0 {
		ell = g.r.IntN(groups + 1)
	}
	for i := 0; i < groups; i++ {
		if i == ell {
			b.WriteString("::")
		} else if i > 0 {
			b.WriteByte(':')
		}
		nd := g.r.IntN(6)
		if g.r.IntN(4) != 0 {
			nd = 1 + g.r.IntN(4)
		}
		for d := 0; d < nd; d++ {
			b.WriteByte("0123456789abcdefABCDEFg"[g.r.IntN(23)])
		}
	}
	if ell == groups {
		b.WriteString("::")
	}
	if g.r.IntN(5) == 0 {
		if b.Len() > 0 {
			b.WriteByte(':')
		}
		n := 3 + g.r.IntN(3)
		for i := 0; i < n; i++ {
			if i > 0 {
				b.WriteByte('.')
			}
			b.WriteString(g.pick([]string{"0", "1", "01", "255", "256", "192", "", "999", "00"}))
		}
	}
	switch g.r.IntN(6) {
	case 0:
		b.WriteString("%25" + g.pick([]string{"en0", "", "%20", "%41", "%E0%B8%81", "%zz", "x%25y", "eth0.1", "%2F", "a b", "ไทย"}))
	case 1:
		b.WriteString("%" + g.pick([]string{"en0", "", "25"}))
	}
	return b.String()
}

func (g gen) advHost() string {
	switch g.r.IntN(4) {
	case 0:
		h := "[" + g.ipv6() + "]"
		if g.r.IntN(2) == 0 {
			h += g.pick([]string{":", ":80", ":0", ":65535", ":99999999999999999999", ":8a", "x", ":80:90"})
		}
		return h
	case 1:
		return g.atoms(4)
	default:
		return g.pick(advHosts)
	}
}

func (g gen) advRawURL() string {
	var b strings.Builder
	b.WriteString(g.pick(advSchemes))
	switch g.r.IntN(4) {
	case 0, 1:
		b.WriteString("//")
		b.WriteString(g.pick(advUsers))
		b.WriteString(g.advHost())
	case 2:
		b.WriteString(g.pick([]string{"", "/", "///", "////"}))
	}
	if g.r.IntN(4) != 0 {
		b.WriteByte('/')
	}
	b.WriteString(g.atoms(8))
	if g.r.IntN(2) == 0 {
		b.WriteByte('?')
		b.WriteString(g.atoms(6))
	}
	if g.r.IntN(2) == 0 {
		b.WriteByte('#')
		b.WriteString(g.atoms(6))
	}
	s := b.String()
	for k := g.r.IntN(3); k > 0; k-- {
		i := g.r.IntN(len(s) + 1)
		s = s[:i] + g.pick(advAtoms) + s[i:]
	}
	if g.r.IntN(2) == 0 {
		// no control characters at all, so Parse gets past the CTL check
		s = strings.Map(func(r rune) rune {
			if r < ' ' || r == 0x7f {
				return -1
			}
			return r
		}, s)
	}
	return s
}

func (g gen) advField(xs []string) string {
	if g.r.IntN(2) == 0 {
		return g.atoms(6)
	}
	return g.pick(xs)
}

func (g gen) advStruct() *url.URL {
	u := &url.URL{}
	if g.r.IntN(2) == 0 {
		u.Scheme = strings.TrimSuffix(g.pick(advSchemes), ":")
	}
	if g.r.IntN(8) == 0 {
		u.Opaque = g.atoms(4)
	}
	switch g.r.IntN(4) {
	case 0:
		u.User = url.User(g.advField([]string{"user", "u:v", "a@b", "", "ไทย"}))
	case 1:
		u.User = url.UserPassword(g.advField([]string{"user", "u:v", ""}), g.advField([]string{"pass", "p@ss", "", "a/b?", "ไทย"}))
	}
	if g.r.IntN(2) == 0 {
		u.Host = g.advField(advHosts)
	}
	u.Path = g.atoms(8)
	switch g.r.IntN(4) {
	case 0:
		u.RawPath = g.atoms(8)
	case 1:
		// a plausible alternative encoding of Path
		u.RawPath = strings.ReplaceAll(url.PathEscape(u.Path), "%2F", "/")
	case 2:
		u.RawPath = strings.ReplaceAll(u.Path, "/", "%2F")
	}
	u.OmitHost = g.r.IntN(3) == 0
	u.ForceQuery = g.r.IntN(4) == 0
	if g.r.IntN(2) == 0 {
		u.RawQuery = g.atoms(5)
	}
	if g.r.IntN(2) == 0 {
		u.Fragment = g.atoms(5)
	}
	switch g.r.IntN(3) {
	case 0:
		u.RawFragment = g.atoms(5)
	case 1:
		u.RawFragment = url.PathEscape(u.Fragment)
	}
	return u
}

func adversarial(w *writer, g gen, n int) {
	for i := 0; i < n; i++ {
		raw := g.advRawURL()
		single(w, raw)
		if u, err := url.Parse(raw); err == nil {
			// Round trip: String() must re-parse to the same URL.
			s := u.String()
			single(w, s)
			if u2, err := url.Parse(s); err == nil {
				single(w, u2.String())
			}
		}
		if u, err := url.ParseRequestURI(raw); err == nil {
			single(w, u.String())
		}
		resolve(w, g.pick(advBases), raw)
		resolve(w, raw, g.atoms(5))
		resolve(w, raw, g.pick(advBases))
		elems := make([]string, g.r.IntN(4))
		for j := range elems {
			elems[j] = g.atoms(4)
		}
		joinPath(w, raw, elems)
		joinPath(w, g.pick(advBases), elems)
		structRec(w, g.advStruct(), g.advStruct(), elems)
		// Pure escape functions over atom soup.
		s := g.atoms(10)
		w.rec("QueryEscape", []string{s}, url.QueryEscape(s))
		w.rec("PathEscape", []string{s}, url.PathEscape(s))
		r, err := url.QueryUnescape(s)
		w.rec("QueryUnescape", []string{s}, r, errStr(err))
		r, err = url.PathUnescape(s)
		w.rec("PathUnescape", []string{s}, r, errStr(err))
	}
	// GODEBUG urlmaxqueryparams=0 (golden build): no limit on the number of
	// query parameters, around Go's default limit of 10000.
	saved := w.every
	w.every = 1 // never sampled away
	for _, k := range []int{9999, 10000, 10001, 20000} {
		parseQueryRec(w, strings.Repeat("&", k-1)+"a=1")
	}
	w.every = saved
	// Exhaustive two-byte %-escapes in every component.
	for hi := 0; hi < 16; hi++ {
		for lo := 0; lo < 16; lo++ {
			e := "%" + strconv.FormatUint(uint64(hi), 16) + strings.ToUpper(strconv.FormatUint(uint64(lo), 16))
			single(w, "http://u"+e+":p"+e+"@h"+e+"/p"+e+"?q"+e+"#f"+e)
			single(w, "http://[fe80::1%25"+e+"]/")
			single(w, "http://h"+e+".com:80/")
			single(w, "/p"+e)
			single(w, "#f"+e)
		}
	}
	// Every single byte in every component.
	for c := 0; c < 256; c++ {
		b := string([]byte{byte(c)})
		for _, s := range []string{b, "http://h/" + b, "http://h" + b + "/", "http://u" + b + "@h/", "http://h/?" + b,
			"http://h/#" + b, "http://[::1" + b + "]/", "http://[::1%25" + b + "]/", "http://h:8" + b + "/", "a" + b + ":b",
			"http:" + b, "/" + b + "/..", b + "/x", "x/" + b} {
			single(w, s)
		}
	}
}
