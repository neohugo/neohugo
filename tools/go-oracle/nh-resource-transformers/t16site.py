"""Writes rust/testdata/oracle/resource-transformers/t16site, the synthetic site of the
Wave B task T16 (js-css-pipeline) oracles (jsbuild, tocss, postcss).

    python3 tools/go-oracle/nh-resource-transformers/t16site.py .

`_node_modules` is copied to `node_modules` by the oracles and the Rust tests (it is mounted as
assets/vendor, like seeksnack's)."""
import os
import sys

root = os.path.join(sys.argv[1], "rust/testdata/oracle/resource-transformers/t16site")

files = {
    "hugo.toml": '''baseURL = "https://example.org/"
title = "T16"
disableKinds = ["taxonomy", "term", "RSS", "sitemap"]

[module]
  [[module.mounts]]
    source = "assets"
    target = "assets"
  [[module.mounts]]
    source = "node_modules"
    target = "assets/vendor"
''',
    "package.json": '''{
  "name": "t16site",
  "version": "1.0.0",
  "private": true
}
''',
    "postcss.config.js": '''// A dependency-free PostCSS config: one inline plugin that shows what the child process
// sees (HUGO_ENVIRONMENT, the cwd, NODE_ENV which Hugo filters out) and rewrites a declaration.
module.exports = {
  plugins: [
    {
      postcssPlugin: "t16",
      Declaration: {
        color: (decl) => {
          if (decl.value === "red") decl.value = "#f00";
        },
      },
      OnceExit(root) {
        root.append({
          text: "env=" + process.env.HUGO_ENVIRONMENT + " node_env=" + (process.env.NODE_ENV || "-") +
            " cwd=" + require("path").basename(process.cwd()) +
            " files=" + Object.keys(process.env).filter((k) => k.startsWith("HUGO_FILE_")).sort().join(","),
        });
      },
    },
  ],
};
''',
    "postcss-alt.config.js": '''module.exports = {
  plugins: [
    {
      postcssPlugin: "t16-alt",
      Rule(rule) {
        rule.selector = rule.selector.toUpperCase();
      },
    },
  ],
};
''',
    # ---- JS / TS
    "assets/js/main.js": '''import { greet, VERSION } from './lib/util';
import helper from 'js/lib/helper';
import * as params from '@params';
import data from './data.json';

const out = [greet('world'), helper(2), VERSION, params.api, data.items.length];
export default out;
console.log(out, `x=${params.n ?? 0}`);
''',
    "assets/js/lib/util.js": '''export const VERSION = '1.2.3';
export function greet(name) {
  const f = (s) => `Hello, ${s}!`;
  return f(name);
}
''',
    "assets/js/lib/helper.js": '''export default function helper(n) {
  let sum = 0;
  for (const x of [1, 2, 3]) sum += x * n;
  return sum;
}
''',
    "assets/js/data.json": '{"items": [1, 2, 3], "name": "data"}\n',
    "assets/js/data/config.json": '{"mode": "test"}\n',
    "assets/js/json.js": '''import d from './data.json';
import cfg from 'js/data/config.json';
console.log(d.name, cfg.mode);
''',
    "assets/js/params.js": '''import * as p from '@params';
import * as cfg from '@params/config';
console.log(JSON.stringify(p), JSON.stringify(cfg));
''',
    "assets/js/define.js": '''if (process.env.NODE_ENV === 'production') {
  console.log('prod', __DEV__, VERSION_STR);
} else {
  console.log('dev');
}
''',
    "assets/js/ext.js": '''import ext from 'extpkg';
import { a } from 'extpkg/sub';
console.log(ext, a);
''',
    "assets/js/shim.js": '''import React from 'react';
console.log(React.createElement('div'));
''',
    "assets/js/shims/react.js": '''export default { createElement: (t) => ({ type: t }) };
''',
    "assets/js/pkg.js": '''import fake, { named } from 'fakepkg';
import sub from 'fakepkg/sub';
const cjs = require('cjspkg');
import inner from 'innerpkg';
console.log(fake, named, sub, cjs.value, inner);
''',
    "assets/js/drop.js": '''function f() { debugger; console.log('x'); return 1; }
console.info(f());
''',
    "assets/js/style.js": '''import './style.css';
console.log('with css');
''',
    "assets/js/style.css": '''.a { color: red; }
''',
    "assets/js/loaders.js": '''import txt from './tpl.txt';
import svg from './icon.svg';
console.log(txt, svg);
''',
    "assets/js/tpl.txt": 'hello text\n',
    "assets/js/icon.svg": '<svg xmlns="http://www.w3.org/2000/svg"><rect width="1" height="1"/></svg>\n',
    "assets/js/err.js": '''const x = ;
''',
    "assets/js/missing.js": '''import nope from 'does-not-exist';
console.log(nope);
''',
    "assets/js/inject/h.js": '''export function h(t, p, ...c) { return { t, p, c }; }
export function Frag(p) { return p; }
''',
    "assets/js/tdz.js": '''let a = 1;
{ const b = a + 1; console.log(b); }
class C { static x = 1; #p = 2; get p() { return this.#p; } }
console.log(new C().p, C.x);
''',
    "assets/js/modern.js": '''const o = { a: 1, ...{ b: 2 } };
const v = o?.a ?? 3;
async function* gen() { yield await Promise.resolve(v); }
(async () => { for await (const x of gen()) console.log(x); })();
console.log(2 ** 10, [1, [2, [3]]].flat(Infinity), 1_000_000, o?.b?.c);
''',
    "assets/js/tla.js": '''const r = await Promise.resolve(1);
console.log(r);
''',
    "assets/js/comp.jsx": '''export default function App() {
  return <div className="app"><span>{1 + 1}</span></div>;
}
''',
    "assets/ts/app.ts": '''import { Mode, area } from './mod';
enum Color { Red, Green = 'g' }
class Box<T> {
  constructor(private readonly v: T) {}
  get value(): T { return this.v; }
}
const b = new Box<number>(area(3));
console.log(Color.Green, Mode.A, b.value);
''',
    "assets/ts/mod.ts": '''export const enum Mode { A = 1, B = 2 }
export function area(r: number): number { return Math.PI * r * r; }
''',
    "assets/ts/comp.tsx": '''export function Comp(props: { name: string }) {
  return <><h1 class="t">Hi {props.name}</h1><p /></>;
}
console.log(Comp({ name: 'x' }));
''',
    "assets/ts/themeswitch.ts": '''const KEY = 'theme';
function apply(t: string): void {
  document.documentElement.setAttribute('data-theme', t);
  localStorage.setItem(KEY, t);
}
document.querySelectorAll<HTMLElement>('[data-theme-set]').forEach((el) => {
  el.addEventListener('click', () => apply(el.dataset.themeSet ?? 'light'));
});
''',
    "assets/ts/search.ts": '''const Mustache = require('mustache-lite');
const api = 'https://api.example.org';
export async function search(q: string): Promise<string> {
  const r = await fetch(`${api}/search?q=${encodeURIComponent(q)}`);
  const j: { hits: { title: string }[] } = await r.json();
  return Mustache.render('{{#hits}}<li>{{title}}</li>{{/hits}}', j);
}
(window as any).search = search;
''',
    "assets/ts/themeset.ts": '''const t = localStorage.getItem('theme') || (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
document.documentElement.setAttribute('data-theme', t);
''',
    "assets/js/jquery-lite.min.js": '''/*! jquery-lite v1 | MIT */
!function(e,t){"object"==typeof module&&"object"==typeof module.exports?module.exports=t(e):t(e)}("undefined"!=typeof window?window:this,function(e){var n=function(s){return{sel:s}};return e.$=n,n});
''',
    "assets/bad.json": '{"a": 1}\n',
    # ---- node_modules (copied to node_modules; mounted as assets/vendor)
    "_node_modules/fakepkg/package.json": '''{
  "name": "fakepkg",
  "version": "1.0.0",
  "main": "./index.cjs.js",
  "module": "./index.esm.js",
  "exports": {
    ".": { "import": "./index.esm.js", "require": "./index.cjs.js" },
    "./sub": "./lib/sub.js"
  }
}
''',
    "_node_modules/fakepkg/index.esm.js": '''export const named = 'named-esm';
export default 'fake-esm';
''',
    "_node_modules/fakepkg/index.cjs.js": '''exports.named = 'named-cjs';
exports.default = 'fake-cjs';
''',
    "_node_modules/fakepkg/lib/sub.js": '''export default 'sub';
''',
    "_node_modules/cjspkg/package.json": '{"name": "cjspkg", "main": "main.js"}\n',
    "_node_modules/cjspkg/main.js": '''/*! cjspkg license */
module.exports = { value: 42 };
''',
    "_node_modules/innerpkg/package.json": '{"name": "innerpkg", "module": "src/index.js"}\n',
    "_node_modules/innerpkg/src/index.js": '''import part from './part';
export default 'inner:' + part;
''',
    "_node_modules/innerpkg/src/part.js": '''export default 'part';
''',
    "_node_modules/mustache-lite/package.json": '{"name": "mustache-lite", "main": "mustache.js"}\n',
    "_node_modules/mustache-lite/mustache.js": '''/*!
 * mustache-lite (license comment kept at EOF)
 */
(function (global, factory) {
  typeof exports === 'object' && typeof module !== 'undefined' ? module.exports = factory() : (global.Mustache = factory());
}(this, function () {
  function render(tpl, view) { return tpl.replace(/{{(\\w+)}}/g, function (m, k) { return view[k]; }); }
  return { render: render };
}));
''',
    "_node_modules/fakejsx/package.json": '''{"name": "fakejsx", "exports": {"./jsx-runtime": "./jsx-runtime.js"}}
''',
    "_node_modules/fakejsx/jsx-runtime.js": '''export function jsx(t, p) { return { t, p }; }
export const jsxs = jsx;
export const Fragment = 'frag';
''',
    "_node_modules/fakescss/scss/_grid.scss": '''@import "variables";
.grid { display: grid; gap: $grid-gap; }
''',
    "_node_modules/fakescss/scss/_variables.scss": '''$grid-gap: 1.5rem !default;
''',
    # ---- SCSS
    "assets/scss/main.scss": '''@import "hugo:vars";
@import "partials/vars";
@import "components/button";
@import "components/cards";
@import "mixins";
@import "legacy";
@import "fakescss/scss/grid";
@import "plain.css";

// A line comment.
/* A block comment. */
.main {
  width: 100% / 3;
  margin: percentage(1 / 7);
  color: darken($brand, 12.5%);
  background: lighten($brand, 20%);
  padding: $space * 1.5 math-ish(3px);
  &:hover { color: mix($brand, #fff, 33%); }
  .inner { @include rounded(4px); content: "\\2192 →"; }
  @media (min-width: 768px) { width: 50%; }
}
%placeholder { font-weight: bold; }
.ext { @extend %placeholder; }
@each $name, $v in (small: 0.875, large: 1.25) {
  .text-#{$name} { font-size: $v * 1rem; line-height: 1 / 3 * $v; }
}
.vars { color: $primary; width: $size; font-family: $font; z-index: $n; opacity: $f; }
''',
    "assets/scss/partials/_vars.scss": '''$brand: #3a7bd5 !default;
$space: 0.75rem;
@function math-ish($x) { @return $x * 2 / 3; }
''',
    "assets/scss/components/_button.scss": '''.btn { padding: 0.375rem 0.75rem; border: 1px solid rgba($brand, 0.5); }
''',
    "assets/scss/components/cards/_index.scss": '''.card { box-shadow: 0 1px 2px rgba(0,0,0,.075); }
''',
    "assets/scss/_mixins.scss": '''@mixin rounded($r) { border-radius: $r; -webkit-border-radius: $r; }
''',
    "assets/scss/_legacy.sass": '''.legacy
  color: blue
  margin: 1px 2px
''',
    "assets/scss/simple.scss": '''$c: #123456;
.simple { color: $c; .child { margin: 0 auto; } }
@media print { .simple { display: none; } }
''',
    "assets/scss/indented.sass": '''$w: 10px
.sass-entry
  width: $w * 3
  .n
    height: 1px
''',
    "assets/scss/errors/undefined.scss": '''.x { color: $undefined-var; }
''',
    "assets/scss/errors/in-partial.scss": '''@import "broken";
.y { color: red; }
''',
    "assets/scss/errors/_broken.scss": '''.b { width: 10px + 2em; }
''',
    "assets/scss/errors/missing.scss": '''@import "does-not-exist";
''',
    "assets/scss/errors/syntax.scss": '''.z { color: red
''',
    "assets/scss/vars.scss": '''@import "hugo:vars";
.v { a: $primary; b: $size; c: $font; d: $n; e: $f; f: $calc; g: $url; h: $quoted; i: $unquoted; }
''',
    "assets/scss/vars-bool.scss": '''@import "hugo:vars";
.b { v: $bool; }
''',
    "assets/css/plain.css": '''.plain { color: red; }
/* comment */
.two { margin: 0 0 0 0; color: blue }
''',
    "assets/css/imports-comment.css": '''@import "plain.css"; /* the inliner keeps the comment in the path */
''',
    "assets/css/imports.css": '''@import "plain.css";
  @import 'sub/x.css';
@import url("remote.css");
@import "print.css" print;
@import "tailwindcss";
@import "sub/x.css";
.after { color: red; }
''',
    "assets/css/sub/x.css": '''@import "y.css";
.x { color: red; }
''',
    "assets/css/sub/y.css": '''.y { top: 0; }
''',
    "assets/css/imports-missing.css": '''.a { color: red; }
@import "nope.css";
''',
    "assets/css/broken.css": '''.ok { color: red; }
.broken { color: red;
''',
    "assets/css/imports-broken.css": '''@import "plain.css";
@import "broken.css";
''',
}

for rel, content in files.items():
    p = os.path.join(root, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w") as f:
        f.write(content)
print("wrote", len(files), "files under", root)
