---
title: Security
description: Control which programs fugo may run, which environment variables templates may read, and which URLs get_remote may fetch.
weight: 90
---

Templates and themes are code. `[security]` limits what they can reach.

{{< code-toggle file=config >}}
[security]
  enableInlineShortcodes = false
  [security.exec]
    allow = ["^(dart-)?sass(-embedded)?$", "^go$", "^git$", "^npx$", "^postcss$", "^tailwindcss$"]
    osEnv = ["(?i)^((HTTPS?|NO)_PROXY|PATH(EXT)?|APPDATA|TE?MP|TERM|GO\\w+|(XDG_CONFIG_)?HOME|USERPROFILE|SSH_AUTH_SOCK|DISPLAY|LANG|SYSTEMDRIVE)$"]
  [security.funcs]
    getenv = ["^FUGO_", "^CI$"]
  [security.http]
    methods = ["(?i)GET|POST"]
    urls = [".*"]
{{< /code-toggle >}}

`exec.allow`
: Programs fugo may run, by name: only the [PostCSS](/asset-pipelines/postcss/),
  [Tailwind](/asset-pipelines/tailwind-css/) and [Babel](/asset-pipelines/babel/) pipelines run
  any. Add `"^babel$"` to use Babel.

`exec.osEnv`
: Environment variables passed to those programs.

`funcs.getenv`
: Variables the `get_env` function may read.

`http.urls`, `http.methods`, `http.mediaTypes`
: What `get_remote` may fetch.

`enableInlineShortcodes`
: Allow shortcodes defined inside content files.

A blocked action is an error that names the setting to change.
