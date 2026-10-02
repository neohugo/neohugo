---
title: About fugo
description: What fugo is, where it comes from, and how it relates to Hugo.
weight: 130
---

fugo is a static site generator written in Rust. It builds sites organized the way
[Hugo](https://gohugo.io/) organizes them — content, front matter, sections, taxonomies, themes
— with [Tera](https://keats.github.io/tera/) templates, in a single binary.

## History

fugo began as **neohugo**, a fork of Hugo. Versions up to 0.148 were Hugo's Go code with
neohugo's changes. From version 0.149, fugo is a rewrite in Rust: the site model, Markdown
rendering, Chroma's highlighter, image processing and the development server were rewritten
and tested against Hugo's own output, page by page, until Hugo's documentation and other sites
built the same.

## fugo and Hugo

fugo is not affiliated with, sponsored or endorsed by the Hugo project. It uses material from
Hugo under Hugo's Apache 2.0 licence: its embedded templates rewritten in Tera, behaviour
transcribed from Hugo's source, and Hugo's test data and documentation as test fixtures. The
repository's `NOTICE` and `PROVENANCE.md` list each such file.

If you need Go templates, Hugo Modules or Hugo's other features, use Hugo. If you want Hugo's
content model with Tera templates in a Rust binary, fugo is for you; see
[Coming from Hugo](/coming-from-hugo/).
