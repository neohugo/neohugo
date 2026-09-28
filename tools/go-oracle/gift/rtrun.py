#!/usr/bin/env python3
"""Crash-tolerant runner for the gift oracle's red-team cases.

Usage: rtrun.py "<oracle command>" <cases file (.gz ok)> <out file>

e.g.   rtrun.py "qemu-aarch64-static ./oracle_arm64" cases.tsv out.tsv

Runs `<oracle> rtrun <cases> <start>` (redteam.go:redteamRun), which prints
one flushed result line per case. A Go panic inside gift's parallelize
goroutines cannot be recovered and kills the oracle; the runner then records
the next case as `panic` and restarts after it. GOMAXPROCS=1 keeps the
panicking goroutine from being descheduled before the runtime exits, so the
main goroutine cannot print a partial digest first (see redteamRun).

A cases file may be a finished fixture (a 7th result field is ignored), so
running a checked-in fixture through here must reproduce it byte for byte."""
import gzip
import os
import shlex
import subprocess
import sys
import tempfile


def main():
    oracle, cases_path, out_path = shlex.split(sys.argv[1]), sys.argv[2], sys.argv[3]
    opener = gzip.open if cases_path.endswith(".gz") else open
    with opener(cases_path, "rt") as f:
        cases = [l.rstrip("\n") for l in f if l.strip()]
    with tempfile.NamedTemporaryFile("w", suffix=".tsv", delete=False) as tmp:
        tmp.write("\n".join(cases) + "\n")
    env = dict(os.environ, GOMAXPROCS="1")
    kinds = {}
    start = 0
    with open(out_path, "w") as out:
        while start < len(cases):
            p = subprocess.run(oracle + ["rtrun", tmp.name, str(start)], stdout=subprocess.PIPE,
                               stderr=subprocess.PIPE, text=True, env=env)
            lines = [l for l in p.stdout.split("\n") if l]
            for l in lines:
                out.write(l + "\n")
            start += len(lines)
            if p.returncode == 0:
                break
            if "panic:" not in p.stderr and "fatal error" not in p.stderr:
                sys.exit(f"unexpected failure at case {start}:\n{p.stderr[-2000:]}")
            msg = next((l for l in p.stderr.split("\n") if l.startswith(("panic:", "fatal error"))), "?")
            msg = msg.split("[")[0][:80]
            kinds[msg] = kinds.get(msg, 0) + 1
            out.write("\t".join(cases[start].split("\t")[:6]) + "\tpanic\n")
            out.flush()
            start += 1
    os.unlink(tmp.name)
    print(f"{len(cases)} cases, {sum(kinds.values())} panics")
    for k, v in sorted(kinds.items(), key=lambda kv: -kv[1]):
        print(f"  {v}\t{k}")


if __name__ == "__main__":
    main()
