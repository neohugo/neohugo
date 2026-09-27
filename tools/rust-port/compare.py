#!/usr/bin/env python3
"""Compare a site build against the golden manifest (byte parity).

Usage: compare.py <build_dir> [--manifest golden/canonical.sha256] [--golden DIR]
                  [--json OUT] [--show N]

The manifest lists the SHA-256 of every file of the golden Go build
(darwin/arm64, go1.27.1). A file matches only if its bytes are identical.
With --golden (a directory holding a golden build), the first differing
byte of each mismatching text file is also reported.
"""
import argparse, collections, hashlib, json, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))


def digest(p):
    h = hashlib.sha256()
    with open(p, 'rb') as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b''):
            h.update(chunk)
    return h.hexdigest()


def category(rel):
    ext = rel.rsplit('.', 1)[-1].lower() if '.' in os.path.basename(rel) else ''
    if ext in ('jpg', 'jpeg', 'png', 'webp', 'gif'):
        return 'image-processed' if '_hu_' in rel else 'image-copied'
    return ext or 'other'


def first_diff(a, b):
    with open(a, 'rb') as fa, open(b, 'rb') as fb:
        x, y = fa.read(), fb.read()
    n = min(len(x), len(y))
    i = next((k for k in range(n) if x[k] != y[k]), n)
    ctx = lambda s: s[max(0, i - 60):i + 60].decode('utf-8', 'replace')
    return i, ctx(x), ctx(y)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('build')
    ap.add_argument('--manifest', default=os.path.join(HERE, 'golden', 'canonical.sha256'))
    ap.add_argument('--golden', help='directory with a golden build, for diff context')
    ap.add_argument('--json')
    ap.add_argument('--show', type=int, default=25)
    a = ap.parse_args()

    want = {}
    with open(a.manifest, encoding='utf-8') as fh:
        for line in fh:
            h, rel = line.rstrip('\n').split('  ', 1)
            want[rel] = h
    got = {}
    for d, _, files in os.walk(a.build):
        for f in files:
            p = os.path.join(d, f)
            got[os.path.relpath(p, a.build)] = p

    missing = sorted(set(want) - set(got))
    extra = sorted(set(got) - set(want))
    stats = collections.defaultdict(lambda: [0, 0])
    differing = []
    for rel in sorted(want):
        cat = category(rel)
        stats[cat][1] += 1
        if rel in got:
            if digest(got[rel]) == want[rel]:
                stats[cat][0] += 1
            else:
                differing.append(rel)

    total = len(want)
    ok = sum(v[0] for v in stats.values())
    print(f'golden files: {total}  build files: {len(got)}')
    print(f'identical: {ok}/{total} ({100.0 * ok / max(total, 1):.2f}%)')
    print(f'missing: {len(missing)}  extra: {len(extra)}  differing: {len(differing)}')
    for cat, (m, t) in sorted(stats.items()):
        print(f'  {cat:16s} {m:6d}/{t:<6d}')
    for label, items in (('missing', missing), ('extra', extra), ('differing', differing)):
        if items:
            print(f'--- first {min(a.show, len(items))} {label}:')
            for rel in items[:a.show]:
                line = '    ' + rel
                if label == 'differing' and a.golden and os.path.exists(os.path.join(a.golden, rel)):
                    i, g, b = first_diff(os.path.join(a.golden, rel), got[rel])
                    line += f'\n        @{i} golden: {g!r}\n        @{i} build:  {b!r}'
                print(line)
    if a.json:
        with open(a.json, 'w') as fh:
            json.dump({'total': total, 'identical': ok, 'missing': missing, 'extra': extra,
                       'differing': differing, 'by_category': dict(stats)}, fh, indent=1)
    sys.exit(0 if ok == total and not extra else 1)


if __name__ == '__main__':
    main()
