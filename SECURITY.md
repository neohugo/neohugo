## Security Policy

### Reporting a Vulnerability

Please report (suspected) security vulnerabilities to **[bjorn.erik.pedersen@gmail.com](mailto:bjorn.erik.pedersen@gmail.com)**. You will receive a response from us within 48 hours. If we can confirm the issue, we will release a patch as soon as possible depending on the complexity of the issue but historically within days.

Also see [Hugo's Security Model](https://gohugo.io/about/security/). neohugo applies Hugo's `security` configuration (`exec.allow`, `exec.osEnv`, `funcs.getenv`, `http.urls`, `http.methods`), but its layouts are Tera templates, not Go's `html/template`: output is escaped by output format (HTML and XML), not by context, so a value in `<script>` needs `jsonify | safe` and one in a query string `urlencode` ([docs/rust-port/template-api.md](docs/rust-port/template-api.md)).
