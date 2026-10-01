#!/usr/bin/env python3
"""Writes the end-to-end sites into a directory (never into the repository).

Usage:
  sites.py list
  sites.py make <site> <dir> [--docs-patches i01|reduced] [--overlay rust/sites/<site>]
                                    # <dir> must not exist; for seeksnack and mini its basename
                                    # must be the site's name (it keys the GetRemote cache)
  sites.py cache <site> <dir>       # the HUGO_CACHEDIR contents the site needs (may be empty)
  sites.py patches [--check]        # write patches.json / check it and the Tera patch files

Sites:
  docs          this repository's docs/ site, patched to build offline; --docs-patches picks the
                variant (i01, the default, or reduced; DOCS_* below, patches.json); docs-i01
                and docs-reduced name the variants too
  testsite      Hugo's hugolib/testsite (rust/testdata/upstream) plus a small config and layouts
                (testsite.txtar)
  seeksnack     the reconstructed seeksnack config (rust/testdata/oracle/allconfig/load/
                seeksnack/hugo.toml) with the synthetic en/th content tree of the nh-hugolib
                oracles (read from rust/testdata/oracle/hugolib/build/seeksnack.json.gz) and
                the layouts/assets/i18n/data of seeksnack.txtar; its GetRemote calls are served
                from the 51 golden getresource cache entries
  mini          the e2e oracle's small en/th site (rust/testdata/oracle/commands/e2e/mini.txtar)
  images        the golden image recipes (rust/testdata/golden/images/manifest.json) as a site
  errors        a failing build (errors.txtar): the error texts must be Go's
  probe         T13's template probe site
  t24-<name>    the T24 build-oracle sites (rust/testdata/oracle/hugolib/build/<name>.json.gz)

--overlay makes the input of the Rust build: the same site with its layouts replaced by the Tera
layouts of the overlay directory, the overlay's assets copied over, and for docs the variant's
Tera patch files (rust/sites/docs/patches/<variant>/) layered on top (REWRITE_PLAN.md §7.4).
The Go build that wrote the golden data built the site without an overlay
(tools/neohugo/oracle.sh, frozen at 44529028).
"""
import argparse
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
# Hugo's test data by its Go-tree path, as the fixtures record it, moved to
# rust/testdata/upstream (the sites use only these; neohugo_testkit::fixture::UPSTREAM lists all).
UPSTREAM = ("hugolib/testsite",)


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


def repo_file(rel):
    """A repository file by the path the fixtures record (neohugo_testkit::fixture::repo_file)."""
    upstream = any(rel == p or rel.startswith(p + "/") for p in UPSTREAM)
    return os.path.join(os.path.join(TESTDATA, "upstream") if upstream else ROOT, *rel.split("/"))


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
# docs/: the offline patch variants (docs/rust-port/REWRITE_PLAN.md §7.3). Every entry names the
# variants it belongs to:
#   i01      the I01 site: offline, no Chroma, passthrough, emoji, Tailwind or node modules
#            (acceptance gate A-D1);
#   reduced  offline, with Chroma highlighting, passthrough, emoji, remarshal, Tailwind and the
#            real Alpine/Turbo imports (node.sh modules; gate A-D2).
# The Go build always builds these Go-template patches. A patch of a file below layouts/ has a
# Tera counterpart at rust/sites/docs/patches/<variant>/<same path> for every variant it belongs
# to (`sites.py patches --check` asserts the 1:1 correspondence); all other patches change the
# site input both builds share. `sites.py patches` writes the list as patches.json.

I01 = "i01"
REDUCED = "reduced"
DOCS_VARIANTS = (I01, REDUCED)
BOTH = DOCS_VARIANTS

DOCS_REMOVE = [  # (file, variants, why)
    ("content/en/news/_content.gotmpl", BOTH, "GetRemote of GitHub releases (a content adapter)"),
    ("content/en/functions/images/Text.md", BOTH, "GetRemote of a font (images.Text)"),
    ("hugo_stats.json", BOTH, "written by the build"),
    # COULD features (T72): images.QR (rsc.io/qr), images.Dither.
    ("content/en/shortcodes/qr.md", BOTH, "images.QR (COULD, T72)"),
    ("content/en/functions/images/QR.md", BOTH, "images.QR (COULD, T72)"),
    ("content/en/functions/images/Dither.md", BOTH, "images.Dither (COULD, T72)"),
    # Chroma: the embedded highlight shortcode, the styles gallery (transform.Highlight).
    ("content/en/shortcodes/highlight.md", (I01,), "the embedded highlight shortcode (Chroma)"),
    ("content/en/quick-reference/syntax-highlighting-styles.md", BOTH,
     "the Chroma style gallery (T72)"),
    # The embedded x shortcode calls GetRemote (publish.x.com oEmbed): the Go build fetches it
    # when the machine has network access, the Rust port never does.
    ("content/en/shortcodes/x.md", BOTH, "the embedded x shortcode calls GetRemote (oEmbed)"),
]

DOCS_REPLACE = [  # (file, old, new, variants, why)
    ("layouts/baseof.html", "css.TailwindCSS $opts", "minify", (I01,), "no Tailwind CLI"),
    ("assets/js/main.js", "import Alpine from 'alpinejs';",
     "const Alpine = { plugin() {}, data() {}, store() {}, magic() {}, start() {}, directive() {} };",
     (I01,), "no node modules (Alpine.js)"),
    ("assets/js/main.js", "import persist from '@alpinejs/persist';\nimport focus from '@alpinejs/focus';",
     "const persist = {};\nconst focus = {};", (I01,), "no node modules (Alpine.js plugins)"),
    ("assets/js/turbo.js", "import * as Turbo from '@hotwired/turbo';", "window.Turbo = { session: {} };",
     (I01,), "no node modules (Turbo)"),
    ("hugo.toml", "[markup.goldmark.extensions.passthrough]\n        enable = true",
     "[markup.goldmark.extensions.passthrough]\n        enable = false", (I01,),
     "no goldmark passthrough"),
    ("hugo.toml", "enableEmoji            = true", "enableEmoji            = false", (I01,),
     "no goldmark emoji"),
    ("hugo.toml", "  [markup.highlight]\n", "  [markup.highlight]\n    codeFences         = false\n",
     (I01,), "no Chroma: code fences are rendered as plain <pre><code>"),
    # images.Text (a font rasterizer) and images.QR (rsc.io/qr) are COULD features (T72):
    # replaced with other image processing so the pipelines still run.
    ("layouts/_partials/opengraph/get-featured-image.html",
     "images.Filter (images.Text $text $textOptions)", "images.Filter (images.Grayscale)", BOTH,
     "images.Text (COULD, T72)"),
    ("layouts/_partials/layouts/header/qr.html",
     'images.QR $.page.Permalink (dict "targetDir" "images/qr")',
     '(resources.Get "/opengraph/gohugoio-card-base-1.png").Resize "64x"', BOTH,
     "images.QR (COULD, T72)"),
    ("layouts/_partials/layouts/hooks/body-main-start.html",
     'images.QR .Permalink (dict "targetDir" "images/qr")',
     '(resources.Get "/opengraph/gohugoio-card-base-1.png").Resize "48x"', BOTH,
     "images.QR (COULD, T72)"),
    # No Chroma (transform.Highlight, highlight): the code is escaped into a plain <pre><code>.
    ("layouts/_markup/render-codeblock.html",
     "transform.Highlight (strings.TrimSpace .Inner) $lang .Options",
     'printf "<pre><code class=%q>%s</code></pre>" $lang (strings.TrimSpace .Inner | htmlEscape) | safeHTML',
     (I01,), "no Chroma (transform.Highlight)"),
    ("layouts/_shortcodes/hl.html", "transform.Highlight $code $lang $opts",
     'printf "<code class=%q>%s</code>" $lang (htmlEscape $code) | safeHTML', (I01,),
     "no Chroma (transform.Highlight)"),
    ("layouts/_shortcodes/code-toggle.html", 'highlight $hCode . ""',
     'printf "<pre><code>%s</code></pre>" (htmlEscape $hCode)', (I01,), "no Chroma (highlight)"),
    # Smart cropping (muesli/smartcrop) is a COULD feature (T72): anchors instead.
    ("content/en/content-management/image-processing/index.md",
     'spec="fill 200x200 smart"', 'spec="fill 200x200 topleft"', BOTH, "smartcrop (COULD, T72)"),
    ("content/en/content-management/image-processing/index.md",
     'spec="crop 200x200 smart"', 'spec="crop 200x200 bottomright"', BOTH,
     "smartcrop (COULD, T72)"),
    # Every code-toggle tab shows JSON (transform.Remarshal to YAML and TOML stays in reduced).
    ("layouts/_shortcodes/code-toggle.html", "$code | transform.Remarshal .",
     '$code | transform.Remarshal "json"', (I01,), "remarshal to JSON only"),
]

DOCS_WRITE = [  # (file, content, variants, why)
    # The GitHub API stub holds what `resources.GetRemote | transform.Unmarshal` gives in Go: JSON
    # numbers are float64 (Go prints 76543.0 as 76543; with ints `printf "%0.1fk" (div 76543 1000)`
    # printed "%!f(int64=76)k"). The Tera patch keeps integers: the Rust `unmarshal` gives Int for
    # integral JSON numbers.
    ("layouts/_partials/helpers/funcs/get-github-info.html",
     '{{ return dict "html_url" "https://github.com/gohugoio/hugo" "stargazers_url" '
     '"https://api.github.com/repos/gohugoio/hugo/stargazers" "watchers_count" 1234.0 '
     '"stargazers_count" 76543.0 "forks_count" 7890.0 "contributors_url" '
     '"https://api.github.com/repos/gohugoio/hugo/contributors" "releases_url" '
     '"https://api.github.com/repos/gohugoio/hugo/releases{/id}" }}',
     BOTH, "GetRemote of the GitHub API"),
]

PATCHES_JSON = os.path.join(HERE, "patches.json")
TERA_PATCHES = os.path.join(ROOT, "rust", "sites", "docs", "patches")


def docs_patches():
    """Every DOCS_* entry as the patches.json document (in application order)."""
    out = []

    def add(op, file, variants, why, **kw):
        tera = file if file.startswith("layouts/") else None
        e = {"op": op, "file": file, "variants": list(variants), "why": why, "tera": tera}
        e.update(kw)
        out.append(e)

    for file, variants, why in DOCS_REMOVE:
        add("remove", file, variants, why)
    for file, old, new, variants, why in DOCS_REPLACE:
        add("replace", file, variants, why, old=old, new=new)
    for file, content, variants, why in DOCS_WRITE:
        add("write", file, variants, why, content=content)
    return {
        "schema": "neohugo-docs-patches/1",
        "about": "Written by tools/rust-port/i01/sites.py (`sites.py patches`) from its DOCS_REMOVE, "
                 "DOCS_REPLACE and DOCS_WRITE lists: the edits of the docs site per variant. `tera` is "
                 "the file below rust/sites/docs/patches/<variant>/ that mirrors a layout patch in the "
                 "Tera overlay (null: the patch changes the site input both builds share).",
        "variants": list(DOCS_VARIANTS),
        "patches": out,
    }


def patches_json_text():
    """patches.json: sorted keys, one patch per line."""
    doc = docs_patches()
    dump = lambda v: json.dumps(v, ensure_ascii=False, sort_keys=True)  # noqa: E731
    lines = [f"{dump(k)}: {dump(v)}" for k, v in sorted(doc.items()) if k != "patches"]
    patches = ",\n".join(dump(p) for p in doc["patches"])
    lines.append(f'"patches": [\n{patches}\n]')
    return "{\n" + ",\n".join(sorted(lines)) + "\n}\n"


def check_patches():
    """Errors: patches.json out of date, or the Tera patch files not 1:1 with the layout patches."""
    errors = []
    try:
        with open(PATCHES_JSON, encoding="utf-8") as fh:
            if fh.read() != patches_json_text():
                errors.append(f"{PATCHES_JSON} is out of date (run sites.py patches)")
    except FileNotFoundError:
        errors.append(f"{PATCHES_JSON} is missing (run sites.py patches)")
    doc = docs_patches()
    for v in DOCS_VARIANTS:
        want = sorted({p["tera"] for p in doc["patches"] if p["tera"] and v in p["variants"]})
        vdir = os.path.join(TERA_PATCHES, v)
        have = sorted(os.path.relpath(os.path.join(d, f), vdir).replace(os.sep, "/")
                      for d, _, fs in os.walk(vdir) for f in fs)
        errors += [f"{v}: no Tera patch file for {f}" for f in want if f not in have]
        errors += [f"{v}: Tera patch file {f} has no entry in patches.json" for f in have if f not in want]
    extra = sorted(set(os.listdir(TERA_PATCHES)) - set(DOCS_VARIANTS)) if os.path.isdir(TERA_PATCHES) else []
    errors += [f"{TERA_PATCHES}/{e}: not a variant" for e in extra]
    return errors


def make_docs(dir_, variant=I01):
    if variant not in DOCS_VARIANTS:
        sys.exit(f"unknown docs patch variant {variant!r} (one of {', '.join(DOCS_VARIANTS)})")
    copy_tree(os.path.join(ROOT, "docs"), dir_)
    for p in docs_patches()["patches"]:
        if variant not in p["variants"]:
            continue
        fn = os.path.join(dir_, *p["file"].split("/"))
        if p["op"] == "remove":
            if os.path.exists(fn):
                os.remove(fn)
        elif p["op"] == "replace":
            edit(dir_, p["file"], p["old"], p["new"])
        else:
            write(dir_, p["file"], p["content"])


# ---------------------------------------------------------------------------------------------
# hugolib/testsite (T25's cli oracle used it with a small config).

def make_testsite(dir_):
    copy_tree(repo_file("hugolib/testsite"), dir_)
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
            with open(repo_file(f["repo"]), "rb") as fh:
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


# ---------------------------------------------------------------------------------------------
# mini: the e2e oracle's small en/th site (rust/testdata/oracle/commands/e2e/mini.txtar). Its
# one GetRemote call is served from a golden getresource entry stored under the file cache key
# of the URL it requests (as tools/go-oracle/nh-commands/e2e did, frozen at 44529028).

MINI_TXTAR = os.path.join(TESTDATA, "oracle", "commands", "e2e", "mini.txtar")
MINI_CACHE = [("10426788187073209306", "17211370855584179129")]  # (golden entry, file cache key)


def make_mini(dir_):
    if os.path.basename(os.path.normpath(dir_)) != "mini":
        sys.exit("the mini site dir must be named mini (it keys the GetRemote cache)")
    for k, v in read_txtar(MINI_TXTAR).items():
        write(dir_, k, v)


def mini_cache(dir_):
    gdir = os.path.join(dir_, "mini", "filecache", "getresource")
    os.makedirs(gdir, exist_ok=True)
    for entry, key in MINI_CACHE:
        shutil.copyfile(os.path.join(GOLDEN_CACHE, entry), os.path.join(gdir, key))


# ---------------------------------------------------------------------------------------------
# images: the recipes of rust/testdata/golden/images/manifest.json (the golden images of the
# PSNR gate of T41) as a site whose home page runs every recipe with Go's image processing and
# prints `<golden name> <RelPermalink>` per line (tools/neohugo/oracle.sh, frozen at 44529028,
# copied the published files into rust/testdata/golden/images).

IMAGES_MANIFEST = os.path.join(TESTDATA, "golden", "images", "manifest.json")

# The filters of the recipes (the JSON of neohugo_images::ImageFilter) as Go template calls: the
# images.* function and the keys of its arguments.
_FILTER_ARGS = {
    "brightness": ("Brightness", ["percentage"]),
    "contrast": ("Contrast", ["percentage"]),
    "gamma": ("Gamma", ["gamma"]),
    "gaussian_blur": ("GaussianBlur", ["sigma"]),
    "grayscale": ("Grayscale", []),
    "hue": ("Hue", ["shift"]),
    "invert": ("Invert", []),
    "colorize": ("Colorize", ["hue", "saturation", "percentage"]),
    "color_balance": ("ColorBalance", ["r", "g", "b"]),
    "saturation": ("Saturation", ["percentage"]),
    "sepia": ("Sepia", ["percentage"]),
    "sigmoid": ("Sigmoid", ["midpoint", "factor"]),
    "unsharp_mask": ("UnsharpMask", ["sigma", "amount", "threshold"]),
    "pixelate": ("Pixelate", ["size"]),
    "opacity": ("Opacity", ["opacity"]),
    "auto_orient": ("AutoOrient", []),
}


def _go_value(v):
    if isinstance(v, str):
        return json.dumps(v)
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        sys.exit(f"images: unsupported filter argument {v!r}")
    return repr(v)


def make_images(dir_):
    with open(IMAGES_MANIFEST, encoding="utf-8") as fh:
        recipes = json.load(fh)
    files = {}  # repository path -> assets path

    def asset(repo_path):
        if repo_path not in files:
            files[repo_path] = f"g/{len(files):02d}{os.path.splitext(repo_path)[1].lower()}"
            with open(os.path.join(ROOT, *repo_path.split("/")), "rb") as fh:
                write(dir_, "assets/" + files[repo_path], fh.read())
        return f'(resources.Get "{files[repo_path]}")'

    def filter_call(f):
        op = f["op"]
        if op in _FILTER_ARGS:
            name, keys = _FILTER_ARGS[op]
            return " ".join([f"images.{name}"] + [_go_value(f[k]) for k in keys])
        if op == "padding":
            margin = f.get("margin") or [f.get(k, 0) for k in ("top", "right", "bottom", "left")]
            color = [_go_value(f["color"])] if "color" in f else []
            return " ".join(["images.Padding"] + [_go_value(m) for m in margin] + color)
        if op == "overlay":
            return f'images.Overlay {asset(f["image"])} {_go_value(f.get("x", 0))} {_go_value(f.get("y", 0))}'
        if op == "mask":
            return f'images.Mask {asset(f["image"])}'
        if op == "process":
            return f'images.Process {_go_value(f["spec"])}'
        sys.exit(f"images: unsupported filter {op!r}")

    lines = []
    for r in recipes:
        if r.get("imaging"):
            sys.exit(f"images: {r['golden']}: a recipe's own [imaging] is not supported (one site)")
        lines.append("{{- $r := " + asset(r["source"]) + " }}")
        for step in r["steps"]:
            if "spec" in step:
                lines.append("{{- $r = $r.Process " + _go_value(step["spec"]) + " }}")
            else:
                fs = " ".join(f"({filter_call(f)})" for f in step["filters"])
                lines.append("{{- $r = $r | images.Filter (slice " + fs + ") }}")
        lines.append(r["golden"] + " {{ $r.RelPermalink }}")
    write(dir_, "hugo.toml", 'baseURL = "https://example.org/"\n'
          'disableKinds = ["page", "section", "taxonomy", "term", "rss", "sitemap", "robotsTXT", "404"]\n'
          '[outputs]\nhome = ["html"]\n')
    write(dir_, "layouts/home.html", "\n".join(lines) + "\n")


# ---------------------------------------------------------------------------------------------
# The Rust overlay (REWRITE_PLAN.md §7.4): the site as generated above, with its layouts replaced
# by the Tera layouts of rust/sites/<site>/layouts, the Tera versions of template-processed assets
# (rust/sites/<site>/assets) copied over, and for docs the variant's Tera patch files
# (rust/sites/docs/patches/<variant>/) layered on top. Content, i18n, data, config and all other
# assets stay as generated.

def apply_overlay(dir_, overlay, variant=None):
    if not os.path.isdir(os.path.join(overlay, "layouts")):
        sys.exit(f"{overlay} has no layouts directory")
    shutil.rmtree(os.path.join(dir_, "layouts"), ignore_errors=True)
    shutil.copytree(os.path.join(overlay, "layouts"), os.path.join(dir_, "layouts"))
    if os.path.isdir(os.path.join(overlay, "assets")):
        shutil.copytree(os.path.join(overlay, "assets"), os.path.join(dir_, "assets"), dirs_exist_ok=True)
    if variant is not None:
        vdir = os.path.join(overlay, "patches", variant)
        if not os.path.isdir(vdir):
            sys.exit(f"{overlay} has no patches/{variant}")
        shutil.copytree(vdir, dir_, dirs_exist_ok=True)


SITES = {"docs": make_docs, "testsite": make_testsite, "seeksnack": make_seeksnack,
         "mini": make_mini, "images": make_images, "errors": make_errors, "probe": make_probe}
CACHES = {"seeksnack": seeksnack_cache, "mini": mini_cache}


def site_and_variant(name, variant):
    """The site and its docs patch variant: `docs-<variant>` is docs with --docs-patches."""
    base, _, v = name.partition("-")
    if base == "docs" and v in DOCS_VARIANTS:
        if variant not in (None, v):
            sys.exit(f"{name} contradicts --docs-patches {variant}")
        return base, v
    if name == "docs":
        return name, variant or I01
    if variant is not None:
        sys.exit("--docs-patches applies to the docs site only")
    return name, None


def main():
    sys.path.insert(0, HERE)
    ap = argparse.ArgumentParser(usage=__doc__)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("list")
    mk = sub.add_parser("make")
    mk.add_argument("site")
    mk.add_argument("dir")
    mk.add_argument("--overlay", metavar="DIR")
    mk.add_argument("--docs-patches", choices=DOCS_VARIANTS)
    ca = sub.add_parser("cache")
    ca.add_argument("site")
    ca.add_argument("dir")
    pa = sub.add_parser("patches")
    pa.add_argument("--check", action="store_true")
    a = ap.parse_args()

    if a.cmd == "list":
        for n in list(SITES) + ["docs-" + v for v in DOCS_VARIANTS] + ["t24-" + n for n in t24_names()]:
            print(n)
        return
    if a.cmd == "patches":
        if not a.check:
            with open(PATCHES_JSON, "w", encoding="utf-8", newline="\n") as fh:
                fh.write(patches_json_text())
        errors = check_patches()
        for e in errors:
            print(f"sites.py patches: {e}", file=sys.stderr)
        if errors:
            sys.exit(1)
        print(f"patches.json: {len(docs_patches()['patches'])} entries; the Tera patch files of "
              f"{', '.join(DOCS_VARIANTS)} correspond 1:1")
        return
    name, variant = site_and_variant(a.site, getattr(a, "docs_patches", None))
    dir_ = a.dir
    if a.cmd == "cache":
        os.makedirs(dir_, exist_ok=True)
        if name in CACHES:
            CACHES[name](dir_)
        return
    if os.path.exists(dir_):
        sys.exit(f"{dir_} exists")
    if os.path.commonpath([os.path.abspath(dir_), ROOT]) == ROOT:
        sys.exit("refusing to write a site into the repository tree")
    if name == "docs":
        make_docs(dir_, variant)
    elif name in SITES:
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
    if a.overlay:
        apply_overlay(dir_, os.path.abspath(a.overlay), variant)


if __name__ == "__main__":
    main()
