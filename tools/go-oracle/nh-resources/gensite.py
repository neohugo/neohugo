#!/usr/bin/env python3
"""Writes the synthetic site of the nh-resources oracles to
rust/testdata/oracle/resources/site (run from the repository root).

The site is checked in; this script documents how it was made and regenerates it byte for
byte: text files, bundles with front matter `resources` metadata (name/title/params with globs
and :counter), assets of many media types, a few small images copied from the repository and a
synthetic 600x480 RGBA watermark PNG (with alpha).
"""

import os
import shutil
import struct
import zlib

ROOT = "rust/testdata/oracle/resources/site"
ROOT_MH = "rust/testdata/oracle/resources/site-multihost"


def w(rel, data):
    p = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    if isinstance(data, str):
        data = data.encode("utf-8")
    with open(p, "wb") as f:
        f.write(data)


def cp(src, rel):
    p = os.path.join(ROOT, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    shutil.copyfile(src, p)


def png_rgba(width, height, pix):
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)

    raw = b""
    for y in range(height):
        raw += b"\x00" + bytes(pix[y * width * 4 : (y + 1) * width * 4])
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def watermark():
    wd, ht = 600, 480
    pix = bytearray(wd * ht * 4)
    for y in range(ht):
        for x in range(wd):
            i = (y * wd + x) * 4
            # Diagonal stripes of semi-transparent white text-like bars, a solid frame and a
            # fully transparent background.
            a = 0
            if (x + y) % 60 < 12:
                a = 96
            if 200 <= y < 280 and 100 <= x < 500 and (x // 10) % 2 == 0:
                a = 180
            if x < 4 or y < 4 or x >= wd - 4 or y >= ht - 4:
                a = 255
            pix[i] = 255
            pix[i + 1] = 255 - (x % 256) // 4
            pix[i + 2] = 255 - (y % 256) // 4
            pix[i + 3] = a
    return png_rgba(wd, ht, pix)


def main():
    if os.path.isdir(ROOT):
        shutil.rmtree(ROOT)

    w(
        "hugo.toml",
        """baseURL = "https://example.org/sub/"
title = "synth"
defaultContentLanguage = "en"
disableKinds = ["taxonomy", "term", "rss", "sitemap", "robotsTXT", "404"]

[languages]
  [languages.en]
    weight = 1
    title = "Synth EN"
  [languages.fr]
    weight = 2
    title = "Synth FR"

[imaging]
  [imaging.exif]
    disableDate = false
    disableLatLong = false
    excludeFields = ".*"
    includeFields = ""
""",
    )

    # Assets.
    w("assets/css/a.css", "body {\n  color: #ff0000;\n  margin: 0px 0px 0px 0px;\n}\n")
    w("assets/css/b.css", "/* b */\n.b   {  padding : 1.50em ;  }\n")
    w("assets/js/a.js", "var a = function ( x ) {\n  return x + 1 ;\n};\n")
    w("assets/js/b.js", "// b\nconsole.log( 'b' );\n")
    w("assets/data/x.json", '{\n  "a": 1,\n  "b": [ true, null, "x" ]\n}\n')
    w("assets/txt/hello.txt", "Hello, world!\n")
    w("assets/scss/website.scss", "$c: red;\nbody { color: $c; }\n")
    w("assets/Upper/MixedCase.TXT", "Mixed case name.\n")
    w("assets/a b/space ü.txt", "A name with a space and a non-ASCII rune.\n")
    w("assets/misc/x.unknownext", "unknown extension\n")
    w("assets/misc/site.webmanifest", '{"name": "synth"}\n')
    w("assets/misc/noext", "no extension\n")
    w("assets/misc/data.csv", "a,b\n1,2\n")
    w("assets/misc/doc.pdf", "%PDF-1.0\n%%EOF\n")
    w("assets/feed/rss.xml", '<?xml version="1.0"?><rss></rss>\n')
    w(
        "assets/images/logo.svg",
        '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>\n',
    )
    cp("resources/testdata/pix.gif", "assets/images/pix.gif")
    cp("resources/testdata/fuzzy-cirlcle.png", "assets/images/fuzzy-circle.png")
    w("assets/images/watermark.png", watermark())
    w("assets/images/icon.ico", b"\x00\x00\x01\x00\x00\x00")

    # Content.
    w("content/_index.md", "---\ntitle: Home\n---\n")
    w("content/_index.fr.md", "---\ntitle: Accueil\n---\n")
    w("content/blog/_index.md", "---\ntitle: Blog\n---\n")
    w(
        "content/blog/bundle1/index.md",
        """---
title: Bundle One
resources:
- src: "*.jpg"
  name: "photo-:counter"
  title: "Photo #:counter"
  params:
    credit: "Someone"
    Weight: 10
- src: "**.txt"
  title: "Text :counter"
  params:
    kind: text
    nested:
      Deep: true
- src: "sub/**"
  name: "sub/renamed-:counter.txt"
- src: "*.json"
  params:
    kind: data
    list: [1, 2.5, "x"]
- src: "*"
  params:
    all: yes
---
Bundle one.
""",
    )
    w(
        "content/blog/bundle1/index.fr.md",
        """---
title: Paquet Un
resources:
- src: "*.css"
  title: "Feuille"
---
Paquet un.
""",
    )
    cp(
        "rust/testdata/site-assets/site/content_potato-crisps_pringles-paprika_pringles-paprika.jpg",
        "content/blog/bundle1/pic1.jpg",
    )
    cp(
        "rust/testdata/site-assets/site/content_pretzels_combos-pizzeria-pretzel_combos-pipr.jpg",
        "content/blog/bundle1/Pic 2.JPG",
    )
    cp(
        "rust/testdata/site-assets/golden/berli-jucker-foods-ltd.berli-jucker-plc_hu_efcb259769ef42b1.png",
        "content/blog/bundle1/logo.png",
    )
    w("content/blog/bundle1/data.json", '{"x": [1, 2, 3]}\n')
    w("content/blog/bundle1/notes.txt", "Some notes.\n")
    w("content/blog/bundle1/Other Name.TXT", "Other.\n")
    w("content/blog/bundle1/style.css", "p { color: blue; }\n")
    w("content/blog/bundle1/sub/deep.txt", "Deep.\n")
    w("content/blog/bundle1/sub/deeper.txt", "Deeper.\n")
    w(
        "content/blog/bundle2/index.md",
        """---
title: Bundle Two
resources:
- src: "images/*"
  title: "Image :counter"
- name: "missing src"
- src: "*.txt"
  name: "never"
---
Bundle two.
""",
    )
    w("content/blog/bundle2/images/a.gif", open("resources/testdata/pix.gif", "rb").read())
    w("content/blog/bundle2/b.txt", "B.\n")
    w("content/blog/bundle3/index.md", "---\ntitle: Bundle Three\nurl: /custom/url/\n---\n")
    w("content/blog/bundle3/c.txt", "C.\n")
    w("content/blog/bundle3/d.svg", '<svg xmlns="http://www.w3.org/2000/svg"/>\n')
    w("content/blog/plain.md", "---\ntitle: Plain\n---\nPlain.\n")
    w("content/section/_index.md", "---\ntitle: Section\n---\n")
    w("content/section/e.txt", "E (branch bundle resource).\n")

    # A multihost config (the keys oracle creates its resources from the files above).
    if os.path.isdir(ROOT_MH):
        shutil.rmtree(ROOT_MH)
    os.makedirs(ROOT_MH)
    with open(os.path.join(ROOT_MH, "hugo.toml"), "w") as f:
        f.write(
            """defaultContentLanguage = "en"
title = "multihost"

[languages]
  [languages.en]
    baseURL = "https://en.example.org/docs/"
    weight = 1
  [languages.fr]
    baseURL = "https://fr.example.org/"
    weight = 2

[imaging]
  [imaging.exif]
    excludeFields = ".*"
"""
        )


if __name__ == "__main__":
    main()
