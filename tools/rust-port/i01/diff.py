#!/usr/bin/env python3
"""Byte comparison of two site builds (Go = reference, Rust = candidate).

Usage: diff.py <go-out> <rust-out> [--show N] [--json FILE] [--label NAME]

Reports the files missing from the Rust build, the extra ones, and the differing ones with the
offset of the first differing byte and some context from both sides. Exit status 0 == identical.
"""
import argparse
import difflib
import json
import os
import re
import sys


def files(root):
    out = {}
    for d, _, fs in os.walk(root):
        for f in fs:
            p = os.path.join(d, f)
            out[os.path.relpath(p, root)] = p
    return out


def first_diff(x, y):
    n = min(len(x), len(y))
    i = 0
    # Fast path in chunks.
    step = 4096
    while i + step <= n and x[i:i + step] == y[i:i + step]:
        i += step
    while i < n and x[i] == y[i]:
        i += 1
    return i


def ctx(b, i, w=80):
    s = b[max(0, i - w):i + w]
    return s.decode("utf-8", "backslashreplace")


LOG_MASKS = [
    (re.compile(r"^Total in \d+ ms$"), "Total in N ms"),
    (re.compile(r"^neohugo v\S+ .*$"), "neohugo vVERSION"),
    # Go's os.CreateTemp names (transform/chain.go writes a failing step's input there).
    (re.compile(r"hugo-transform-error\d+"), "hugo-transform-errorN"),
]


def norm_log(path):
    with open(path, encoding="utf-8", errors="backslashreplace") as fh:
        lines = fh.read().splitlines()
    out = []
    for line in lines:
        for rx, rep in LOG_MASKS:
            line = rx.sub(rep, line)
        out.append(line)
    return out


def compare_logs(go_log, rust_log):
    """Prints a unified diff of the normalised logs; returns True when they differ."""
    g, r = norm_log(go_log), norm_log(rust_log)
    if g == r:
        print("  logs: identical")
        return False
    print("  logs differ (- go, + rust):")
    for line in list(difflib.unified_diff(g, r, lineterm="", n=0))[2:42]:
        print("    " + line[:400])
    return True


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("go")
    ap.add_argument("rust")
    ap.add_argument("--show", type=int, default=20)
    ap.add_argument("--json")
    ap.add_argument("--label", default="")
    ap.add_argument("--logs", nargs=2, metavar=("GO_LOG", "RUST_LOG"),
                    help="also compare the two build logs (timings and the version line masked)")
    a = ap.parse_args()
    logs_differ = a.logs is not None and compare_logs(*a.logs)

    g, r = files(a.go), files(a.rust)
    missing = sorted(set(g) - set(r))
    extra = sorted(set(r) - set(g))
    differ = []
    same = 0
    for rel in sorted(set(g) & set(r)):
        with open(g[rel], "rb") as fh:
            x = fh.read()
        with open(r[rel], "rb") as fh:
            y = fh.read()
        if x == y:
            same += 1
        else:
            differ.append((rel, first_diff(x, y), len(x), len(y), x, y))

    label = a.label + ": " if a.label else ""
    print(f"{label}go {len(g)} files, rust {len(r)} files; identical {same}, "
          f"missing {len(missing)}, extra {len(extra)}, differing {len(differ)}")
    for rel in missing[:a.show]:
        print(f"  MISSING {rel}")
    if len(missing) > a.show:
        print(f"  ... {len(missing) - a.show} more missing")
    for rel in extra[:a.show]:
        print(f"  EXTRA   {rel}")
    if len(extra) > a.show:
        print(f"  ... {len(extra) - a.show} more extra")
    for rel, i, lx, ly, x, y in differ[:a.show]:
        print(f"  DIFFER  {rel} at byte {i} (go {lx} bytes, rust {ly} bytes)")
        print(f"    go:   {ctx(x, i)!r}")
        print(f"    rust: {ctx(y, i)!r}")
    if len(differ) > a.show:
        print(f"  ... {len(differ) - a.show} more differing")
    if a.json:
        with open(a.json, "w", encoding="utf-8") as fh:
            json.dump({"label": a.label, "go": len(g), "rust": len(r), "identical": same,
                       "missing": missing, "extra": extra,
                       "differing": [[d[0], d[1]] for d in differ]}, fh, indent=1)
    sys.exit(0 if not (missing or extra or differ or logs_differ) else 1)


if __name__ == "__main__":
    main()
