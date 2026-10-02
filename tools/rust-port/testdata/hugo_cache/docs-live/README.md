# GetRemote responses of the published docs build (docs-live)

`sites.py cache docs-live <dir>` copies these into `<dir>/docs-live/filecache/getresource/`
(a `.gz` file decompressed), the `NEOHUGO_CACHEDIR` of gate A-D3 (`compare.sh docs-live`), whose
build runs without network access. Each file is a cached response as neohugo stores it (an HTTP
response: status line, headers, blank line, body) under its cache key, the hash neohugo derives
from the URL and the request options. They are the responses the published site
(neohugo/neohugo.github.io at a1928152, built 2025-10-13; `testdata/golden/README.md`) was built
with, recorded on 2026-10-01 and normalised: only the status line, `Date` (set to the build's
day) and `Content-Type` headers are kept.

| Key | Request | Body |
|---|---|---|
| `7291f7c1c512575b176010dff53c7816` | `https://api.github.com/repos/gohugoio/hugo/releases` (the news content adapter `content/en/news/_content.html`, through `helpers/funcs/get-remote-data.html`) | the API's first page (30 releases) as of the build: the releases created before 2025-10-13T15:00:00Z, newest first, as the API returned them on 2026-10-01, without the fields the site does not read (`assets`, `body`, `reactions`, `mentions_count`; `author` reduced to `login`, `id`, `html_url`, `type`). Its first 24 non-draft, non-prerelease entries are the 24 releases the published /news/ lists |
| `410dd1e5b3e8bc93c03900dd593fe676` | `https://api.github.com/repos/gohugoio/hugo` (`helpers/funcs/get-github-info.html`, cache key option `<url>2025-10-13`) | the repository as the API returned it, with `stargazers_count` and `watchers_count` set to 84125, the number the published pages show |
| `509cf07231e0bfc2e603ab922586c877` (`.gz`) | `https://github.com/google/fonts/raw/refs/heads/main/ofl/lato/Lato-Regular.ttf` (`images.Text` example, `_shortcodes/img.html`) | the font as served (`application/octet-stream`: its media type is sniffed) |
| `31351302268d25e87acb4162832b3688` | `https://publish.x.com/oembed?…` for x.com/sandiegozoo/status/1453110110599868418 (`x` shortcode) | the oEmbed JSON as served |
| `02ae90cea3f273d6af224cf7802c58ff` | `https://publish.x.com/oembed?…` for x.com/letsencrypt/status/971755920639307777 (`x` shortcode) | the oEmbed JSON as served |
