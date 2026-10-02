## Security Policy

### Reporting a Vulnerability

Please report (suspected) security vulnerabilities in neohugo privately through GitHub's
[private vulnerability reporting](https://github.com/neohugo/neohugo/security/advisories/new)
for this repository, not in a public issue or discussion. If we can confirm the issue, we will
release a patch as soon as possible depending on the complexity of the issue.

neohugo is not maintained by the Hugo project: please do not send reports about neohugo to Hugo's
maintainers. A vulnerability that also affects Hugo itself should be reported to Hugo as well,
following [Hugo's security policy](https://github.com/gohugoio/hugo/security/policy).

### Security model

neohugo follows [Hugo's Security Model](https://gohugo.io/about/security/) and applies Hugo's
`security` configuration (`exec.allow`, `exec.osEnv`, `funcs.getenv`, `http.urls`,
`http.methods`), but its layouts are Tera templates, not Go's `html/template`: output is escaped
by output format (HTML and XML), not by context, so a value in `<script>` needs `jsonify | safe`
and one in a query string `urlencode` ([docs/rust-port/template-api.md](docs/rust-port/template-api.md)).
