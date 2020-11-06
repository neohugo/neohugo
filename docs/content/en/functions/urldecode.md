---
title: urldecode
# linktitle: urldecode
description: The string so it can be safely placed inside a URL query.
date: 2017-02-01
publishdate: 2017-02-01
lastmod: 2017-02-01
categories: [functions]
menu:
  docs:
    parent: "functions"
keywords: [urls,strings]
godocref:
signature: ["urldecode INPUT"]
hugoversion:
deprecated: false
workson: []
relatedfuncs: []
---

The following examples pull from a content file with the following front matter:

{{< code file="content/blog/neohugo.md" copy="false">}}
+++
title = "A Forked of Hugo"
location = "กรุงเทพ ไทย"
+++
{{< /code >}}

The following might be used as a partial within a [single page template][singletemplate]:

{{< code file="layouts/partials/content-header.html" download="content-header.html" >}}
<header>
    <h1>{{.Title}}</h1>
    {{ with .Params.location }}
        <div><a href="/locations/{{ urldecode ( . | urlize) }}">{{.}}</a></div>
    {{ end }}
</header>
{{< /code >}}

The preceding partial would then output to the rendered page as follows, assuming the page is being built with Hugo's default pretty URLs.

{{< output file="/blog/neohugo/index.html" >}}
<header>
    <h1>A Forked of Hugo</h1>
    <div><a href="/locations/กรุงเทพ-ไทย">กรุงเทพ ไทย</a></div>
</header>
{{< /output >}}



[singletemplate]: /templates/single-page-templates/
