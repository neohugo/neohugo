---
title: Introduction
description: How fugo finds and merges its configuration — the file, the configuration directory, environments, command-line flags and FUGO_ environment variables.
weight: 10
---

## The configuration file

fugo reads the first of `config.toml`, `config.yaml`, `config.yml` and `config.json` in the
project directory (`--source`). If there are several, it warns about the others. `--config`
names other files, comma-separated; the first wins where they disagree.

```toml {title="config.toml"}
baseURL = "https://example.org/"
title = "My site"
languageCode = "en-US"

[params]
  description = "Notes on Rust and the web"
```

Keys are case-insensitive: `baseURL` and `baseurl` are the same. Hugo's `hugo.toml` is not
read; rename it to `config.toml`.

## Configuration directory

Settings can be split into files in `config/` (or `--config-dir`):

```text
config/
├── _default/
│   ├── config.toml       root settings
│   ├── params.toml       [params]
│   ├── menus.en.toml     [languages.en.menus]
│   └── languages.toml    [languages]
├── production/
│   └── config.toml       only in production
└── development/
    └── params.toml       only in development
```

A file's name says where its settings go: `params.toml` is `[params]`, `menus.en.toml` the
English menus, `config.toml` the root.

## Environments

The environment is `production` for `fugo build` and `development` for `fugo server`;
`--environment staging` (or `FUGO_ENVIRONMENT=staging`) chooses another. `config/<environment>/`
is merged over `config/_default/`, and templates read it as `build.environment`,
`build.is_production`, `build.is_development`.

## Precedence

From lowest to highest: themes (see [Themes](/configuration/themes/#configuration)), the configuration
file, `config/_default/`, `config/<environment>/`, command-line flags (`--base-url`,
`--destination`, …), then environment variables.

A table with `_merge = "none"` replaces the table below it instead of merging into it.

## Environment variables

`FUGO_` followed by a key sets it: `FUGO_BASEURL=https://staging.example.org/`,
`FUGO_PARAMS_ANALYTICS=false`. Underscores separate nested keys; to keep an underscore in a key,
write another delimiter right after `FUGO`: `FUGOxPARAMSxAPI_KEY=secret` sets `params.api_key`.
Values take the type of the setting they replace (a boolean, a number, a list as `["a", "b"]`,
a table as JSON or TOML). Hugo's `HUGO_` variables are not read.
