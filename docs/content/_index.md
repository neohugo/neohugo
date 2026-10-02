---
title: fugo
description: fugo is a fast static site generator written in Rust. It reads Hugo's content model and renders Tera templates, in a single binary with no runtime dependencies.
lead: Write Markdown, design with Tera templates, publish a fast static site. fugo builds Hugo-style projects from a single Rust binary, with asset pipelines, image processing and a live-reloading server built in.
features:
  - title: Fast by default
    text: Written in Rust and parallel throughout. This documentation, about 300 pages with processed images and a Sass stylesheet, builds in well under two seconds.
  - title: Hugo's content model
    text: Page bundles, sections, taxonomies, menus, multilingual sites, front matter cascades, shortcodes and render hooks work the way Hugo users expect.
  - title: Tera templates
    text: Layouts are [Tera](https://keats.github.io/tera/) templates with Hugo's layout names, checked by `fugo templates check` before you build.
  - title: Pipelines built in
    text: Sass, JavaScript bundling, image processing, fingerprinting and minification run in process. No Node.js needed unless you choose PostCSS or Tailwind.
  - title: Live reload
    text: "`fugo server` builds into memory, watches your project, and reloads the browser when something changes."
  - title: One binary
    text: Download a release for Linux, macOS or Windows, or build it with Cargo. Nothing else to install.
---
