#!/usr/bin/env python3
"""Stratified selection of small corpus files for the checked-in fixtures.

usage: select_fixtures.py <lj.tsv> <mk.tsv> <mk-dir> > sel.list
"""
import random, os, sys, collections
rnd = random.Random(7)
rows = []
for tsv, root in [(sys.argv[1], "."), (sys.argv[2], sys.argv[3])]:
    for line in open(tsv):
        f = line.rstrip("\n").split("\t")
        name = f[0]
        path = os.path.join(root, name) if root != "." else name
        size = os.path.getsize(path)
        data = open(path, "rb").read()
        prog = b"\xff\xc2" in data[:4000]
        dri = b"\xff\xdd" in data[:4000]
        dec = f[4]
        if dec.startswith("YCbCr"):
            cat = "YCbCr" + dec.split(" ")[2]
        else:
            cat = dec.split(" ")[0] if not dec.startswith("err") else dec
        rows.append((cat, prog, dri, size, path))
bycat = collections.defaultdict(list)
for r in rows:
    if r[3] <= 2600:
        bycat[(r[0], r[1], r[2])].append(r)
sel = []
for k, v in sorted(bycat.items()):
    rnd.shuffle(v)
    sel += v[:8]
rnd.shuffle(sel)
sel = sel[:260]
tot = sum(r[3] for r in sel)
print(len(sel), tot, file=sys.stderr)
cats = collections.Counter((r[0], r[1], r[2]) for r in sel)
for k, v in sorted(cats.items()):
    print(k, v, file=sys.stderr)
for r in sel:
    print(r[4])
