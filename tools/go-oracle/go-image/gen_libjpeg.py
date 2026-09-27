#!/usr/bin/env python3
"""Generate a JPEG corpus with libjpeg-turbo (cjpeg/jpegtran) and ImageMagick.

usage: gen_libjpeg.py <outdir> <n> <seed> [extra jpeg inputs for jpegtran...]
"""
import os
import random
import shutil
import subprocess
import sys

# ImageMagick 7 installs `magick`; ImageMagick 6 (e.g. Debian/Ubuntu) only
# `convert`, which accepts the same arguments used here.
MAGICK = "magick" if shutil.which("magick") else "convert"

out = sys.argv[1]
n = int(sys.argv[2])
seed = int(sys.argv[3])
extra = []
for a in sys.argv[4:]:
    if a.startswith("@"):
        extra += [l for l in open(a[1:]).read().split("\n") if l]
    else:
        extra.append(a)
rnd = random.Random(seed)
os.makedirs(out, exist_ok=True)
tmp = os.path.join(out, "_tmp")
os.makedirs(tmp, exist_ok=True)


def dim():
    k = rnd.randrange(20)
    if k == 0:
        return 1
    if k < 4:
        return rnd.randrange(1, 400)
    return rnd.randrange(1, 72)


def ppm(path, w, h, gray):
    mode = rnd.randrange(5)
    ch = 1 if gray else 3
    k = [rnd.randrange(-6, 7) for _ in range(6)]
    c = [rnd.randrange(256) for _ in range(3)]
    noise = rnd.randrange(0, 40)
    data = bytearray(w * h * ch)
    i = 0
    for y in range(h):
        for x in range(w):
            for j in range(ch):
                if mode == 0:
                    v = rnd.randrange(256)
                elif mode == 1:
                    v = c[j]
                elif mode == 2:
                    v = 255 if ((x // 3) + (y // 3)) % 2 else 0
                else:
                    v = c[j] + x * k[j] + y * k[j + 3]
                    if noise:
                        v += rnd.randrange(noise)
                data[i] = v & 0xFF
                i += 1
    with open(path, "wb") as f:
        f.write(b"P%d\n%d %d\n255\n" % (5 if gray else 6, w, h))
        f.write(bytes(data))


def scans_script(ncomp):
    lines = []
    dc_al = [rnd.randrange(3) for _ in range(ncomp)]
    interleave_dc = ncomp > 1 and rnd.randrange(2) == 0
    if interleave_dc:
        a = rnd.randrange(3)
        dc_al = [a] * ncomp
        lines.append("%s: 0-0, 0, %d;" % (",".join(str(c) for c in range(ncomp)), a))
    else:
        for c in range(ncomp):
            lines.append("%d: 0-0, 0, %d;" % (c, dc_al[c]))
    # Remaining work items: per component DC refinements and AC bands.
    queues = []
    if interleave_dc:
        q = []
        for a in range(dc_al[0], 0, -1):
            q.append("%s: 0-0, %d, %d;" % (",".join(str(c) for c in range(ncomp)), a, a - 1))
        queues.append(q)
    else:
        for c in range(ncomp):
            q = []
            for a in range(dc_al[c], 0, -1):
                q.append("%d: 0-0, %d, %d;" % (c, a, a - 1))
            queues.append(q)
    for c in range(ncomp):
        cuts = sorted(rnd.sample(range(2, 63), rnd.randrange(0, 3)))
        bounds = [1] + cuts + [64]
        for i in range(len(bounds) - 1):
            ss, se = bounds[i], bounds[i + 1] - 1
            if rnd.randrange(15) == 0:
                continue  # leave this band out entirely
            al = rnd.randrange(3)
            q = ["%d: %d-%d, 0, %d;" % (c, ss, se, al)]
            for a in range(al, 0, -1):
                q.append("%d: %d-%d, %d, %d;" % (c, ss, se, a, a - 1))
            queues.append(q)
    queues = [q for q in queues if q]
    while queues:
        q = rnd.choice(queues)
        lines.append(q.pop(0))
        if not q:
            queues.remove(q)
    return "\n".join(lines) + "\n"


def ncomp_of(path):
    b = open(path, "rb").read()
    i = 2
    while i + 4 <= len(b):
        if b[i] != 0xFF:
            i += 1
            continue
        m = b[i + 1]
        if m in (0xC0, 0xC1, 0xC2):
            return b[i + 9]
        if m == 0xFF or m == 0x00 or 0xD0 <= m <= 0xD9:
            i += 2
            continue
        ln = b[i + 2] << 8 | b[i + 3]
        i += 2 + ln
    return 3


def run(args, **kw):
    r = subprocess.run(args, stdout=subprocess.PIPE, stderr=subprocess.PIPE, **kw)
    return r.returncode == 0


def jpegtran(src, dst):
    args = ["jpegtran"]
    ops = rnd.randrange(12)
    if ops < 4:
        args.append("-progressive")
    elif ops < 7:
        sp = os.path.join(tmp, "scans.txt")
        with open(sp, "w") as f:
            f.write(scans_script(ncomp_of(src)))
        args += ["-scans", sp]
    if rnd.randrange(3) == 0:
        r = rnd.randrange(1, 12)
        args += ["-restart", "%d%s" % (r, "B" if rnd.randrange(2) else "")]
    if rnd.randrange(3) == 0:
        args.append("-optimize")
    k = rnd.randrange(10)
    if k == 0:
        args += ["-crop", "%dx%d+%d+%d" % (rnd.randrange(1, 60), rnd.randrange(1, 60),
                                        rnd.randrange(0, 20), rnd.randrange(0, 20))]
    elif k == 1:
        args += ["-rotate", rnd.choice(["90", "180", "270"])]
        if rnd.randrange(2):
            args.append("-trim")
    elif k == 2:
        args += ["-flip", rnd.choice(["horizontal", "vertical"])]
    elif k == 3:
        args.append("-transpose")
    elif k == 4:
        args.append("-grayscale")
    if rnd.randrange(40) == 0:
        args.append("-arithmetic")
    args += ["-copy", rnd.choice(["none", "all"]), "-outfile", dst, src]
    return run(args)


made = 0
i = 0
while made < n:
    i += 1
    kind = rnd.randrange(10)
    name = os.path.join(out, "lj-%d-%06d.jpg" % (seed, i))
    if kind < 7:
        gray = rnd.randrange(5) == 0
        w, h = dim(), dim()
        src = os.path.join(tmp, "in.p%sm" % ("g" if gray else "p"))
        ppm(src, w, h, gray)
        args = ["cjpeg", "-quality"]
        q = rnd.randrange(1, 101)
        if rnd.randrange(8) == 0:
            args.append("%d,%d" % (q, rnd.randrange(1, 101)))
        else:
            args.append(str(q))
        ncomp = 1 if gray else 3
        if not gray and rnd.randrange(8) == 0:
            args.append("-grayscale")
            ncomp = 1
        if not gray and ncomp == 3 and rnd.randrange(8) == 0:
            args.append("-rgb")
        if rnd.randrange(3) == 0:
            for _ in range(10):
                fs = []
                tot = 0
                for c in range(ncomp):
                    hh, vv = rnd.choice([1, 1, 2, 2, 3, 4]), rnd.choice([1, 1, 2, 2, 3, 4])
                    fs.append("%dx%d" % (hh, vv))
                    tot += hh * vv
                if tot <= 10:
                    break
            args += ["-sample", ",".join(fs)]
        if rnd.randrange(4) == 0:
            args.append("-progressive")
        elif rnd.randrange(8) == 0:
            sp = os.path.join(tmp, "scans.txt")
            with open(sp, "w") as f:
                f.write(scans_script(ncomp))
            args += ["-scans", sp]
        if rnd.randrange(4) == 0:
            args.append("-optimize")
        if rnd.randrange(4) == 0:
            args += ["-restart", "%d%s" % (rnd.randrange(0, 10), "B" if rnd.randrange(2) else "")]
        if rnd.randrange(4) == 0:
            args += ["-dct", rnd.choice(["int", "fast", "float"])]
        if rnd.randrange(8) == 0:
            args += ["-smooth", str(rnd.randrange(1, 100))]
        if rnd.randrange(60) == 0:
            args.append("-arithmetic")
        if rnd.randrange(80) == 0:
            args += ["-precision", "12"]
        if rnd.randrange(10) == 0:
            args.append("-baseline")
        args += ["-outfile", name, src]
        if not run(args):
            continue
        if rnd.randrange(4) == 0:
            t = name + ".t.jpg"
            if jpegtran(name, t):
                os.replace(t, name)
    elif kind < 9:
        # ImageMagick CMYK (Adobe APP14, inverted), sometimes progressive.
        w, h = dim(), dim()
        src = os.path.join(tmp, "in.ppm")
        ppm(src, w, h, False)
        args = [MAGICK, src, "-colorspace", "CMYK", "-quality", str(rnd.randrange(1, 101))]
        if rnd.randrange(2):
            args += ["-sampling-factor", rnd.choice(["1x1", "2x2", "2x1", "1x2"])]
        if rnd.randrange(3) == 0:
            args += ["-interlace", "JPEG"]
        args.append(name)
        if not run(args):
            continue
        if rnd.randrange(3) == 0:
            t = name + ".t.jpg"
            if jpegtran(name, t):
                os.replace(t, name)
    else:
        if not extra:
            continue
        src = rnd.choice(extra)
        if not jpegtran(src, name):
            continue
    made += 1

for f in os.listdir(tmp):
    os.remove(os.path.join(tmp, f))
os.rmdir(tmp)
