#!/usr/bin/env python3
"""Writes the synthetic site of the nh-resource-transformers oracles (Wave B task T15) to
crates/nh-resource-transformers/tests/fixtures/site (run from the repository root).

The site holds assets of every kind the resources namespace handles: JavaScript parts with and
without trailing newlines (Concat's "\\n;\\n" separator), CSS with numbers and a data URI
(minify), TypeScript templates for ExecuteAsTemplate (data, range/with/if, a define, a parse
error, an execution error), JSON, SVG, HTML, XML, text, a PNG, mixed-case and non-ASCII names
(Get/GetMatch/Match/ByType globs and case sensitivity). The config enables HEAD for GetRemote
and accepts one extra media type.
"""

import os
import struct
import zlib

ROOT = "crates/nh-resource-transformers/tests/fixtures/site"

HUGO_TOML = """\
# The synthetic site of the nh-resource-transformers oracles (Wave B task T15),
# written by tools/go-oracle/nh-resource-transformers/gensite.py.
baseURL = "https://example.org/sub/"
title = "T15 synthetic"
disableKinds = ["taxonomy", "term", "rss", "sitemap", "robotstxt", "404"]
# GetRemote retries temporary HTTP errors until this timeout.
timeout = "1s"

[params]
  api = "https://api.example.org"
  [params.comment]
    apipro = "https://api.seeksnack.example"

[security]
  [security.http]
    methods = ['(?i)GET|POST|HEAD']
    mediaTypes = ['^application/vnd\\.t15\\+json']
"""

COMMENT_TS = """\
// {{ printf "%s" .api }}
const endpoint = "{{ .api }}/comments";
{{- range $i, $e := .list }}
const item{{ $i }} = "{{ $e }}";
{{- end }}
{{ with .site }}const title = "{{ .title }}";{{ else }}const title = "none";{{ end }}
{{ if eq .mode "prod" }}const prod = true;{{ else }}const prod = false;{{ end }}
"""

SEARCH_TS = """\
const api: string = "{{ .api }}";
export function search(q: string): string {
  return api + "/search?q=" + q;
}
"""

B_CSS = (
    "/* b */\n.b { padding: 10px 0px 10px 0px; background: "
    "url(\"data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' "
    "viewBox='0 0 16 16'%3E%3Cpath d='M 1 1 L 2 2'/%3E%3C/svg%3E\"); }\n"
)

FILES = {
    "hugo.toml": HUGO_TOML,
    "content/_index.md": "---\ntitle: Home\n---\n",
    "layouts/home.html": "<!doctype html><title>{{ .Title }}</title>\n",
    "assets/js/a.js": "var a = 1;\n",
    "assets/js/b.js": "(function () { return 2 })()",
    "assets/js/c.js": "// only a comment",
    "assets/js/empty.js": "",
    "assets/js/sub/d.js": 'export const d = "sub/d";\nconsole.log(d)\n',
    "assets/js/Upper.JS": "window.UPPER = true;\n",
    "assets/css/a.css": "body {\n  margin: 0.50em;\n  color: #ff0000;\n}\n",
    "assets/css/b.css": B_CSS,
    "assets/ts/search.ts": SEARCH_TS,
    "assets/ts/comment.ts": COMMENT_TS,
    "assets/ts/bad.ts": 'const bad = "{{ .api ";\n',
    "assets/ts/exec.ts": 'const x = "{{ .api.foo }}";\n',
    "assets/ts/define.ts": '{{ define "x" }}X{{ end }}{{ template "x" }}-{{ len .list }}-{{ index .site "title" }}\n',
    "assets/data/x.json": '{\n  "a": 1,\n  "b": [1, 2, 3],\n  "c": { "d": "e" }\n}\n',
    "assets/data/y.json": '[ 1.50, 2.0e3, "x" ]\n',
    "assets/images/logo.svg": (
        '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16">\n'
        "  <!-- comment -->\n"
        '  <rect x="0.500" y="0" width="16" height="16" fill="#ff0000"/>\n'
        "</svg>\n"
    ),
    "assets/html/snippet.html": '<div class="a">\n  <p>Hello   world</p>\n</div>\n',
    "assets/xml/feed.xml": '<?xml version="1.0"?>\n<feed>\n  <entry> x </entry>\n</feed>\n',
    "assets/txt/readme.txt": "plain text\n",
    "assets/Upper/MixedCase.TXT": "MIXED\n",
    "assets/a b/space ü.txt": "space\n",
}


def png() -> bytes:
    def chunk(t: bytes, d: bytes) -> bytes:
        return struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    w, h = 2, 4
    raw = b""
    for y in range(h):
        raw += b"\x00" + bytes([y * 60, 10, 200, 255 - y * 60, 20, 30])
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def main() -> None:
    for rel, content in FILES.items():
        p = os.path.join(ROOT, rel)
        os.makedirs(os.path.dirname(p), exist_ok=True)
        with open(p, "wb") as f:
            f.write(content.encode("utf-8"))
    p = os.path.join(ROOT, "assets/images/pix.png")
    with open(p, "wb") as f:
        f.write(png())


if __name__ == "__main__":
    main()
