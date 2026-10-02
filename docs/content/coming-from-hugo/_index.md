---
title: Coming from Hugo
description: What carries over from a Hugo site, what changes, and how to move a site to fugo step by step.
weight: 100
---

fugo began as a fork of Hugo and keeps Hugo's content model, so most of a Hugo site carries
over unchanged. The big difference is the template language: fugo renders
[Tera](https://keats.github.io/tera/), not Go templates.

## What carries over

- **Content**: Markdown and HTML files, front matter in TOML, YAML or JSON, page bundles,
  sections, taxonomies, menus, `cascade`, summaries, shortcode calls in content, multilingual
  content.
- **Configuration**: Hugo's keys, the configuration directory, environments, `_merge`.
- **Layout names**: Hugo v0.146's — `home.html`, `single.html`, `_partials/`, `_shortcodes/`,
  `_markup/` — and its lookup order.
- **Data, i18n, assets and static files**, mounts and themes from `themes/` or `_vendor/`.
- **Output**: URLs, aliases, pagination, feeds, sitemaps and highlighted code as Hugo writes them.

## What changes

| Hugo | fugo |
|---|---|
| Go templates | Tera templates: see [Converting templates](/coming-from-hugo/templates/) |
| `hugo.toml` | `config.toml` (`hugo.*` files are not read) |
| `HUGO_*` environment variables | `FUGO_*` |
| `hugo` template object (`hugo.Version`, `hugo.IsProduction`) | `build` (`build.version`, `build.is_production`) |
| `hugo_stats.json` | `build_stats.json` |
| `hugo` / `hugo server` | `fugo` (or `fugo build`) / `fugo server` |
| Layout names before v0.146 (`_default/`, `partials/`) | refused, with the new name |

The other differences — unsupported features and commands — are listed in
[Differences](/coming-from-hugo/differences/).

## Moving a site

1. **Rename the configuration**: `hugo.toml` → `config.toml` (or `config/_default/hugo.toml` →
   `config.toml`), and `HUGO_*` variables in your CI to `FUGO_*`.
2. **Use v0.146 layout names.** If your layouts still use `_default/` or `partials/`, rename
   them; fugo's error message names the new path of each file.
3. **Convert the templates** to Tera, file by file, with the
   [conversion guide](/coming-from-hugo/templates/). A theme needs the same.
4. **Check them**: `fugo templates check` reports syntax errors, unknown functions, missing
   context names and pages no template renders, without building.
5. **Build and compare**: `fugo build -d public-fugo` and compare with Hugo's `public/`
   (`diff -r`). Differences in minified bytes and Sass output are expected; see
   [Differences](/coming-from-hugo/differences/).

Content does not change, so content writers can keep working while the templates move.
