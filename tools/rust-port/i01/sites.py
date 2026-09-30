#!/usr/bin/env python3
"""Writes the I01 end-to-end sites into a directory (never into the repository).

Usage:
  sites.py list
  sites.py make <site> <dir>        # <dir> must not exist; for seeksnack its basename must be "seeksnack"
  sites.py cache <site> <dir>       # the HUGO_CACHEDIR contents the site needs (may be empty)

Sites:
  docs          this repository's docs/ site, patched to build offline (see DOCS_* below)
  testsite      hugolib/testsite plus a small config and layouts (TESTSITE_FILES)
  seeksnack     the reconstructed seeksnack config (rust/testdata/oracle/allconfig/load/
                seeksnack/hugo.toml) with the synthetic en/th content tree of the nh-hugolib
                oracles (read from rust/testdata/oracle/hugolib/build/seeksnack.json.gz) and
                the layouts/assets/i18n/data of seeksnack.txtar; its GetRemote calls are served
                from the 51 golden getresource cache entries
  errors        a failing build (errors.txtar): the error texts must be Go's
  t24-<name>    the T24 build-oracle sites (rust/testdata/oracle/hugolib/build/<name>.json.gz)

Both binaries build the same copy with the same flags (see compare.sh).
"""
import gzip
import json
import os
import shutil
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
TESTDATA = os.path.join(ROOT, "rust", "testdata")
BUILD_FX = os.path.join(TESTDATA, "oracle", "hugolib", "build")
GOLDEN_CACHE = os.path.join(ROOT, "tools", "rust-port", "testdata", "hugo_cache", "seeksnack",
                            "filecache", "getresource")
GETREMOTE_FX = os.path.join(TESTDATA, "oracle", "resource-transformers", "getremote",
                            "getremote.json.gz")


def write(dir_, rel, content):
    fn = os.path.join(dir_, *rel.split("/"))
    os.makedirs(os.path.dirname(fn), exist_ok=True)
    mode = "wb" if isinstance(content, bytes) else "w"
    with open(fn, mode, **({} if mode == "wb" else {"encoding": "utf-8", "newline": ""})) as fh:
        fh.write(content)


def edit(dir_, rel, old, new):
    fn = os.path.join(dir_, *rel.split("/"))
    with open(fn, encoding="utf-8", newline="") as fh:
        s = fh.read()
    if s.count(old) != 1:
        sys.exit(f"{rel}: {old!r} found {s.count(old)} times")
    with open(fn, "w", encoding="utf-8", newline="") as fh:
        fh.write(s.replace(old, new, 1))


def copy_tree(src, dst, skip=("public", "resources", "node_modules")):
    def ignore(d, names):
        if os.path.abspath(d) == os.path.abspath(src):
            return [n for n in names if n in skip or n.startswith(".")]
        return []
    shutil.copytree(src, dst, ignore=ignore, symlinks=False)


def read_txtar(path):
    """A txtar archive: `-- name --` lines start files (Go's x/tools/txtar format)."""
    files = {}
    name = None
    buf = []
    with open(path, encoding="utf-8", newline="") as fh:
        for line in fh.read().split("\n"):
            if line.startswith("-- ") and line.endswith(" --") and len(line) > 6:
                if name is not None:
                    files[name] = "\n".join(buf)
                name = line[3:-3].strip()
                buf = []
            elif name is not None:
                buf.append(line)
    if name is not None:
        files[name] = "\n".join(buf)
    # txtar: every file ends with a newline; the split above dropped the final one of each file
    # except for the last. Normalise to "content + \n" unless the file is empty.
    return {k: (v if v.endswith("\n") or v == "" else v + "\n") for k, v in files.items()}


# ---------------------------------------------------------------------------------------------
# docs/: the offline patch (the same edits the nh-publisher site oracle made, plus the features
# the Rust port does not support: goldmark passthrough, emoji, Chroma highlighting).

DOCS_REMOVE = [
    "content/en/news/_content.gotmpl",      # GetRemote of GitHub releases (a content adapter)
    "content/en/functions/images/Text.md",  # GetRemote of a font (images.Text)
    "hugo_stats.json",                      # written by the build
    # Explicit errors in the Rust port (not used by seeksnack): images.QR (rsc.io/qr),
    # images.Dither (makeworld-the-better-one/dither).
    "content/en/shortcodes/qr.md",
    "content/en/functions/images/QR.md",
    "content/en/functions/images/Dither.md",
    # Chroma: the embedded highlight shortcode, the styles gallery (transform.Highlight).
    "content/en/shortcodes/highlight.md",
    "content/en/quick-reference/syntax-highlighting-styles.md",
    # The embedded x shortcode calls GetRemote (publish.x.com oEmbed): the Go build fetches it
    # when the machine has network access, the Rust port never does.
    "content/en/shortcodes/x.md",
]

DOCS_REPLACE = [
    ("layouts/baseof.html", "css.TailwindCSS $opts", "minify"),  # no Tailwind CLI
    ("assets/js/main.js", "import Alpine from 'alpinejs';",
     "const Alpine = { plugin() {}, data() {}, store() {}, magic() {}, start() {}, directive() {} };"),
    ("assets/js/main.js", "import persist from '@alpinejs/persist';\nimport focus from '@alpinejs/focus';",
     "const persist = {};\nconst focus = {};"),
    ("assets/js/turbo.js", "import * as Turbo from '@hotwired/turbo';", "window.Turbo = { session: {} };"),
    # Not supported by the Rust port (explicit errors): passthrough, goldmark-emoji.
    ("hugo.toml", "[markup.goldmark.extensions.passthrough]\n        enable = true",
     "[markup.goldmark.extensions.passthrough]\n        enable = false"),
    ("hugo.toml", "enableEmoji            = true", "enableEmoji            = false"),
    # No Chroma: code fences are rendered as plain <pre><code>.
    ("hugo.toml", "  [markup.highlight]\n", "  [markup.highlight]\n    codeFences         = false\n"),
    # images.Text (a font rasterizer) and images.QR (rsc.io/qr) are explicit errors in the Rust
    # port; replaced with other image processing so the pipelines still run.
    ("layouts/_partials/opengraph/get-featured-image.html",
     "images.Filter (images.Text $text $textOptions)", "images.Filter (images.Grayscale)"),
    ("layouts/_partials/layouts/header/qr.html",
     'images.QR $.page.Permalink (dict "targetDir" "images/qr")',
     '(resources.Get "/opengraph/gohugoio-card-base-1.png").Resize "64x"'),
    ("layouts/_partials/layouts/hooks/body-main-start.html",
     'images.QR .Permalink (dict "targetDir" "images/qr")',
     '(resources.Get "/opengraph/gohugoio-card-base-1.png").Resize "48x"'),
    # Chroma (transform.Highlight, highlight) is an explicit error in the Rust port: the code is
    # escaped into a plain <pre><code> instead.
    ("layouts/_markup/render-codeblock.html",
     "transform.Highlight (strings.TrimSpace .Inner) $lang .Options",
     'printf "<pre><code class=%q>%s</code></pre>" $lang (strings.TrimSpace .Inner | htmlEscape) | safeHTML'),
    ("layouts/_shortcodes/hl.html", "transform.Highlight $code $lang $opts",
     'printf "<code class=%q>%s</code>" $lang (htmlEscape $code) | safeHTML'),
    ("layouts/_shortcodes/code-toggle.html", 'highlight $hCode . ""',
     'printf "<pre><code>%s</code></pre>" (htmlEscape $hCode)'),
    # Smart cropping (muesli/smartcrop) is an explicit error in the Rust port.
    ("content/en/content-management/image-processing/index.md",
     'spec="fill 200x200 smart"', 'spec="fill 200x200 topleft"'),
    ("content/en/content-management/image-processing/index.md",
     'spec="crop 200x200 smart"', 'spec="crop 200x200 bottomright"'),
] + ([] if os.environ.get("I01_DOCS_REMARSHAL") == "1" else [
    # transform.Remarshal to YAML/TOML reaches nh-parser's encoder stubs (gaps agent). Until they
    # land, every code-toggle tab shows JSON; set I01_DOCS_REMARSHAL=1 to keep Go's templates.
    ("layouts/_shortcodes/code-toggle.html", "$code | transform.Remarshal .",
     '$code | transform.Remarshal "json"'),
])

DOCS_WRITE = {
    # GetRemote of the GitHub API.
    "layouts/_partials/helpers/funcs/get-github-info.html":
        '{{ return dict "html_url" "https://github.com/gohugoio/hugo" "stargazers_url" '
        '"https://api.github.com/repos/gohugoio/hugo/stargazers" "watchers_count" 1234 '
        '"stargazers_count" 76543 "forks_count" 7890 "contributors_url" '
        '"https://api.github.com/repos/gohugoio/hugo/contributors" "releases_url" '
        '"https://api.github.com/repos/gohugoio/hugo/releases{/id}" }}',
}


def make_docs(dir_):
    copy_tree(os.path.join(ROOT, "docs"), dir_)
    for rel in DOCS_REMOVE:
        fn = os.path.join(dir_, *rel.split("/"))
        if os.path.exists(fn):
            os.remove(fn)
    for rel, old, new in DOCS_REPLACE:
        edit(dir_, rel, old, new)
    for rel, content in DOCS_WRITE.items():
        write(dir_, rel, content)


# ---------------------------------------------------------------------------------------------
# hugolib/testsite (T25's cli oracle used it with a small config).

def make_testsite(dir_):
    copy_tree(os.path.join(ROOT, "hugolib", "testsite"), dir_)
    for k, v in read_txtar(os.path.join(HERE, "testsite.txtar")).items():
        write(dir_, k, v)


# ---------------------------------------------------------------------------------------------
# The T24 build sites.

def fixture_site(name):
    with gzip.open(os.path.join(BUILD_FX, name + ".json.gz"), "rt", encoding="utf-8") as fh:
        return json.load(fh)["site"]


def write_fixture_site(site, dir_):
    os.makedirs(dir_)
    write(dir_, "hugo.toml", site["toml"])
    for f in site["files"]:
        if f.get("repo"):
            with open(os.path.join(ROOT, *f["repo"].split("/")), "rb") as fh:
                write(dir_, f["path"], fh.read())
        else:
            write(dir_, f["path"], f["content"])


def t24_names():
    return sorted(n[:-len(".json.gz")] for n in os.listdir(BUILD_FX) if n.endswith(".json.gz"))


# ---------------------------------------------------------------------------------------------
# seeksnack.

def make_seeksnack(dir_):
    if os.path.basename(os.path.normpath(dir_)) != "seeksnack":
        sys.exit("the seeksnack site dir must be named seeksnack (it keys the GetRemote cache)")
    site = fixture_site("seeksnack")
    # The synthetic content tree, without the capture shortcodes (seeksnack.txtar has its own).
    site["files"] = [f for f in site["files"] if not f["path"].startswith("layouts/")]
    write_fixture_site(site, dir_)
    for k, v in read_txtar(os.path.join(HERE, "seeksnack.txtar")).items():
        write(dir_, k, v)
    # Real images from the repository's fixtures (the synthetic tree's image files are not images).
    for rel, src in SEEKSNACK_IMAGES.items():
        with open(os.path.join(ROOT, *src.split("/")), "rb") as fh:
            write(dir_, rel, fh.read())
    for rel, content in generated_snacks().items():
        write(dir_, rel, content)


# Generated snack pages: every golden getresource entry not used by seeksnack.txtar gets a page
# whose youtube_video is that entry, so all 51 cached responses are read through GetRemote; the
# pages fill the paginators (pagerSize 12) and the taxonomies, including terms that collide on
# one URL like the golden build's ("Lay's"/"Lays", "INS 322(i)"/"ins-322i", case variants).
_CATEGORIES = ["potato-chips", "biscuit", "seafood", "candy", "bread-pan", "pretzels", "cookies"]
_BRANDS = ["Lay's", "Lays", "Lotte", "Pringles", "Le Pan", "Tao Kae Noi", "Glico", "Meiji"]
_COMPANIES = ["Frito Lay", "Thai Lotte Co., Ltd.", "Kellogg", "Le Pan Bakery", "Ezaki Glico Co., Ltd."]
_COUNTRIES = ["Thailand", "Japan", "USA", "Korea", "Malaysia"]
_INGREDIENTS = ["INS 322(i)", "ins-322i", "Potato", "Sugar", "INS 124", "Palm Oil", "Salt", "Wheat Flour"]
_TAGS = ["crispy", "Crispy", "sweet", "spicy", "Seaweed", "Chocolate", "Party", "Lay's", "Lays"]
_TH_TAGS = ["กรอบ", "หวาน", "เผ็ด", "สาหร่าย", "ปาร์ตี้", "คริสปี้พาย"]
_JPGS = ["assets_images_categories_almonds.jpg", "assets_images_categories_biscuit-stick.jpg",
         "assets_images_categories_candy-shell.jpg", "assets_images_ingredients_chocolate.jpg",
         "content_companies_hanami-foods-co-ltd_hanamifoods.jpg"]
_USED_IDS = {"10426788187073209306", "10921459942904219419", "11152450411989392523"}


def generated_snacks():
    with gzip.open(GETREMOTE_FX, "rt", encoding="utf-8") as fh:
        ids = [e["entry"] for e in json.load(fh)["seeksnack"] if e["entry"] not in _USED_IDS]
    files = {}
    for i, vid in enumerate(ids):
        pick = lambda xs, n=1, off=0: [xs[(i * 7 + off + k * 3) % len(xs)] for k in range(n)]  # noqa: E731
        slug = f"snack-{i:02d}"
        img = f"s{i:02d}.jpg"
        day = 1 + (i * 5) % 28
        month = 1 + (i * 3) % 12
        year = 2019 + i % 4
        fm = [
            "---",
            f'title: "Snack {i:02d} {pick(_BRANDS)[0]} {pick(_CATEGORIES)[0]}"',
            f"date: {year}-{month:02d}-{day:02d}T{(i * 3) % 24:02d}:15:00Z",
            "type: snacks",
            f"image: {img}",
            f'youtube_video: "{vid}"',
            f"categories: {json.dumps(pick(_CATEGORIES, 1 + i % 2))}",
            f"brands: {json.dumps(pick(_BRANDS, 1, 1))}",
            f"companies: {json.dumps(pick(_COMPANIES, 1 + i % 2, 2))}",
            f"countries: {json.dumps(pick(_COUNTRIES, 1, 3))}",
            f"ingredients: {json.dumps(pick(_INGREDIENTS, 2 + i % 3, 4))}",
            f"tags: {json.dumps(pick(_TAGS, 1 + i % 3, 5), ensure_ascii=False)}",
            f"rating: {{taste: {1 + i % 5}, smell: {(i % 9) / 2}}}",
        ]
        if i % 4 == 0:
            fm.append(f'when_seen: "2020-{month:02d}-{day:02d}"')
        if i % 6 == 0:
            fm.append(f'aliases: ["/old/{slug}/"]')
        if i % 5 == 0:
            fm.append("weight: " + str(10 - i % 10))
        fm.append("---")
        body = (f"## Review {i}\n\nA *snack* review with ![pack]({img}) and a "
                f"[link](/snacks/snack-{(i + 1) % len(ids):02d}/).\n\n<!--more-->\n\n"
                f"### Details\n\n" + "More words. " * (5 + i % 40) + "\n")
        files[f"content/snacks/{slug}/index.md"] = "\n".join(fm) + "\n" + body
        with open(os.path.join(TESTDATA, "site-assets", "site",
                               _JPGS[i % len(_JPGS)]), "rb") as fh:
            files[f"content/snacks/{slug}/{img}"] = fh.read()
        if i % 3 == 0:
            th = ["---", f'title: "ขนม {i:02d}"', f"date: {year}-{month:02d}-{day:02d}T00:00:00Z",
                  "type: snacks", f"image: {img}", f'youtube_video: "{vid}"',
                  f"categories: {json.dumps(pick(_CATEGORIES))}",
                  f"tags: {json.dumps(pick(_TH_TAGS, 2), ensure_ascii=False)}", "---"]
            files[f"content/snacks/{slug}/index.th.md"] = "\n".join(th) + f"\nรีวิวขนม {i} ![ห่อ]({img})\n"
    return files


_SITE_JPG = "rust/testdata/site-assets/site/"
_REPO_PNG = "rust/testdata/site-assets/repo/"
_GOLDEN_PNG = "rust/testdata/site-assets/golden/"
SEEKSNACK_IMAGES = {
    "content/biscuit/koalas-march-chocolate/koala.jpg": _SITE_JPG + "assets_images_categories_biscuit-stick.jpg",
    "content/biscuit/koalas-march-chocolate/koala_pack.jpg": _SITE_JPG + "assets_images_categories_almonds.jpg",
    "content/potato-chips/wise-chili-olé-chili-&-spice-flavor-potato-chips/olé.jpg":
        _SITE_JPG + "content_potato-crisps_pringles-paprika_pringles-paprika.jpg",
    "content/companies/Berli-Jucker-Foods-Ltd.Berli-Jucker-PLC/logo.png":
        _GOLDEN_PNG + "berli-jucker-foods-ltd.berli-jucker-plc_hu_9745137e13631ab1.png",
    "content/companies/le-pan-bakery/logo.png": _GOLDEN_PNG + "frito-lay_hu_8f37120362e997d1.png",
    "content/ingredients/ins-124/ins124.jpg": _SITE_JPG + "assets_images_ingredients_chocolate.jpg",
    "content/ขนม/ข้าวเกรียบ/รูป.jpg": _SITE_JPG + "content_pretzels_combos-pizzeria-pretzel_combos-pipr.jpg",
    "content/snacks/lays-rock-prawn/prawn.jpg": _SITE_JPG + "content_potato-crisps_pringles-paprika_pringles-paprika.jpg",
    "content/snacks/koala/koala.jpg": _SITE_JPG + "assets_images_categories_candy-shell.jpg",
    "content/snacks/pringles-paprika/pringles.jpg": _SITE_JPG + "content_cookies_alices-pineapple-pastry_600x200.jpg",
    "content/snacks/combos/combos.jpg": _SITE_JPG + "content_pretzels_combos-pizzeria-pretzel_combos-pipr.jpg",
    "content/snacks/taro-bread/taro.jpg": _SITE_JPG + "content_bread-pan_taro-custard-filled-panbread_lepan_cake_taro-preview-.jpg",
    "assets/images/watermark.png": _REPO_PNG + "fuzzy-cirlcle.png",
    "assets/images/favicon/favicon-16x16.png": _REPO_PNG + "favicon-16x16.png",
    "assets/images/favicon/favicon-32x32.png": _REPO_PNG + "android-chrome-72x72.png",
    "assets/images/favicon/mstile-70x70.png": _GOLDEN_PNG + "mstile-70x70_hu_80634bc5fec9785.png",
    "assets/images/favicon/mstile-150x150.png": _REPO_PNG + "apple-touch-icon.png",
}


def seeksnack_cache(dir_):
    """The golden getresource entries under the keys of the synthetic URLs the layouts use."""
    with gzip.open(GETREMOTE_FX, "rt", encoding="utf-8") as fh:
        entries = json.load(fh)["seeksnack"]
    gdir = os.path.join(dir_, "seeksnack", "filecache", "getresource")
    os.makedirs(gdir, exist_ok=True)
    for e in entries:
        shutil.copyfile(os.path.join(GOLDEN_CACHE, e["entry"]), os.path.join(gdir, e["fileCacheKey"]))


PROBE_FX = os.path.join(TESTDATA, "oracle", "tplimpl", "probe", "probe.json.gz")

PROBE_CONFIG = """baseURL = "https://example.org/"
title = "Site"
disableKinds = ["rss", "sitemap", "taxonomy", "term", "robotsTXT", "404"]
[minify]
disableJSON = true
[outputs]
home = ["html", "json", "plain"]
[outputFormats.plain]
mediaType = "text/plain"
baseName = "index"
isPlainText = true
[params]
intv = 3
floatv = 1.0
arr = ["x", "y"]
[params.nested]
key = "v"
"""

PROBE_HOME = """+++
title = "Home \\"Q\\" & 'A' <b>"
description = "desc + plus / slash"
date = 2020-09-06T15:46:26.955Z
yint = 5
yfloat = 4.5
yfloat0 = 5.0
ybig = 12345678901
ystr = "007"
ybool = true
yfalse = false
yempty = ""
yzero = 0
ylist = ["a", "b", "c"]
ylistmixed = [1, "x", 2.5]
yemptylist = []
ymap = {b = 2, a = 1, c = [1, 2]}
yemptymap = {}
ydate = 2021-01-02T00:00:00Z
yneg = -3
yexp = 1.5e10
mixed_case = "mc"
yhtml = "<em>h</em>"
+++
"""


def make_probe(dir_):
    """T13's probe site (the template-engine spec's Appendix A/B probe lines) built for real:
    the store oracle's three home layouts, and a home page whose front matter holds the values of
    the oracle's stub page (the real page and func map, not the minimal test FuncMap)."""
    with gzip.open(PROBE_FX, "rt", encoding="utf-8") as fh:
        files = json.load(fh)["files"]
    os.makedirs(dir_)
    for k, v in files.items():
        # With the real func map `js` is Hugo's js namespace: `.Title | js` prints the namespace
        # struct, which Go prints with its pointer addresses (different in every Go run).
        if "{{ .Title | js }}" in v:
            v = v.replace("{{ .Title | js }}", "JS-NAMESPACE")
        write(dir_, k, v)
    write(dir_, "hugo.toml", PROBE_CONFIG)
    write(dir_, "content/_index.md", PROBE_HOME)


def make_errors(dir_):
    for k, v in read_txtar(os.path.join(HERE, "errors.txtar")).items():
        write(dir_, k, v)


SITES = {"docs": make_docs, "testsite": make_testsite, "seeksnack": make_seeksnack,
         "errors": make_errors, "probe": make_probe}


def main():
    sys.path.insert(0, HERE)
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    cmd = sys.argv[1]
    if cmd == "list":
        for n in list(SITES) + ["t24-" + n for n in t24_names()]:
            print(n)
        return
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    name, dir_ = sys.argv[2], sys.argv[3]
    if cmd == "cache":
        os.makedirs(dir_, exist_ok=True)
        if name == "seeksnack":
            seeksnack_cache(dir_)
        return
    if cmd != "make":
        sys.exit(__doc__)
    if os.path.exists(dir_):
        sys.exit(f"{dir_} exists")
    if os.path.commonpath([os.path.abspath(dir_), ROOT]) == ROOT:
        sys.exit("refusing to write a site into the repository tree")
    if name in SITES:
        SITES[name](dir_)
    elif name.startswith("t24-"):
        site = fixture_site(name[4:])
        if name == "t24-docs":
            # No Chroma in the Rust port: code fences are rendered as plain <pre><code>.
            old = "  [markup.highlight]\n"
            if site["toml"].count(old) != 1:
                sys.exit("t24-docs: highlight anchor not found")
            site["toml"] = site["toml"].replace(old, old + "    codeFences         = false\n")
        write_fixture_site(site, dir_)
    else:
        sys.exit(f"unknown site {name}")


if __name__ == "__main__":
    main()
