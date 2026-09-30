#!/usr/bin/env python3
"""Regenerates the Chroma/Hugo fixtures of neohugo-highlight (see the crate README).

usage: regen.py <oracle binary> <work dir>

The work dir must hold corpus.json, written by
  NEOHUGO_HL_DUMP=<work dir>/corpus.json cargo test -p neohugo-highlight --test it dump_corpus
The oracle binary is main.go.txt of this directory built as a Go module (README).
"""
import gzip
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DATA = os.path.dirname(HERE)

# The option matrix of hugo-html.json (golden.rs).
CODES = [
    ('go-html-template', '{{ range .Pages }}\n  <a href="{{ .Permalink }}">{{ .Title }}</a>\n{{ end }}'),
    ('go-html-template', '{{/* a comment */}}\n<!DOCTYPE html>\n<p class=x>&amp; {{ $x := 1 }}</p>\n'),
    ('text', 'plain <text>\nsecond "line"\n\nfourth'),
    ('nosuchlang', 'x < y\n'),
    ('', 'no language'),
]
OPTS = [
    '',
    'noClasses=false',
    'noClasses=false,linenos=table',
    'noClasses=false,linenos=inline',
    'noClasses=false,linenos=table,hl_lines=2',
    'noClasses=false,linenos=inline,hl_lines=1 3,linenostart=5',
    'noClasses=false,hl_lines=2',
    'hl_inline=true,noClasses=true',
    'hl_inline=true,noClasses=false',
    'hl_inline=true,noClasses=false,linenos=table',
    'linenos=table',
    'linenos=inline,hl_lines=2,style=solarized-dark',
    'style=emacs,linenos=table,hl_lines=2-3',
    'noClasses=false,anchorlinenos=true,lineanchors=x,linenos=inline',
    'noClasses=false,anchorlinenos=true,linenos=table',
    'noClasses=true,anchorlinenos=true,linenos=table,style=github',
    'style=nosuchstyle',
    'tabwidth=8,style=dracula',
    'noClasses=false,wrapperClass=hl foo,linenos=true,lineNumbersInTable=false',
]


def fnv(s):
    """64-bit FNV-1a of the UTF-8 bytes, as 16 hex digits (corpus::fnv of one part)."""
    h = 0xcbf29ce484222325
    for b in s.encode():
        h = ((h ^ b) * 0x100000001b3) & 0xFFFFFFFFFFFFFFFF
    return '%016x' % h


def run(oracle, mode, items):
    out = subprocess.run([oracle, mode], input=json.dumps(items), capture_output=True,
                         text=True, check=True)
    return json.loads(out.stdout)


def write_gz(name, value):
    with gzip.GzipFile(os.path.join(DATA, name), 'wb', compresslevel=9, mtime=0) as f:
        f.write(json.dumps(value, separators=(',', ':'), ensure_ascii=False).encode())


def main():
    oracle, work = sys.argv[1], sys.argv[2]
    corpus = json.load(open(os.path.join(work, 'corpus.json')))

    tokens = {}
    for x in run(oracle, 'tokens', corpus):
        tokens.setdefault(x['key'], {'key': x['key'], 'lexer': x['lexer'], 'runs': x.get('runs') or []})
    write_gz('chroma-tokens.json.gz', sorted(tokens.values(), key=lambda x: x['key']))

    html = {}
    for x in run(oracle, 'docs', corpus):
        assert not x['html'].startswith('ERROR'), x
        html.setdefault(x['key'], {'key': x['key'], 'hash': fnv(x['html'])})
    write_gz('hugo-docs-html.json.gz', sorted(html.values(), key=lambda x: x['key']))

    items = [{'key': str(i), 'lang': lang, 'code': code, 'opts': o}
             for i, (lang, code, o) in enumerate((l, c, o) for l, c in CODES for o in OPTS)]
    cases = [{'lang': i['lang'], 'code': i['code'], 'opts': i['opts'], 'html': o['html']}
             for i, o in zip(items, run(oracle, 'html', items))]
    with open(os.path.join(DATA, 'hugo-html.json'), 'w') as f:
        json.dump(cases, f, indent=1, ensure_ascii=False)

    for style, mode, name in [('solarized-dark', 'all', 'solarized-dark.css'),
                              ('solarized-dark', 'omit', 'solarized-dark.omit-empty.css')]:
        css = subprocess.run([oracle, 'css', style, mode], capture_output=True, text=True, check=True)
        with open(os.path.join(DATA, name), 'w') as f:
            f.write(css.stdout)

    lexers = subprocess.run([oracle, 'keys'], capture_output=True, text=True, check=True).stdout
    header = ('# Chroma v2.19.0 lexer lookup (MIT, github.com/alecthomas/chroma): kind TAB key TAB lexer.\n'
              '# kind n = a name or alias (lower-case, matched case-insensitively); f = a file extension or\n'
              "# file name (matched exactly). Generated with Chroma's lexers.Get(key) (crate README).\n")
    with open(os.path.join(DATA, '..', '..', 'src', 'data', 'chroma-lexers.tsv'), 'w') as f:
        f.write(header + lexers)
    print(len(corpus), 'corpus items;', len(tokens), 'token fixtures;', len(html), 'html hashes')


if __name__ == '__main__':
    main()
