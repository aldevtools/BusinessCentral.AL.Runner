#!/usr/bin/env python3
"""Compares the test outcomes of two JUnit files written by `al-test-cli --junit` or `al-runner --junit`.

Usage: compare_junit.py DEMAND.xml EAGER.xml [--allow FILE] [--reasons]
       compare_junit.py --entries FILE [--title TEXT] MAIN.xml HEAD.xml

Prints every test whose outcome (pass, fail, not run) differs, with both reasons, and exits 1 if
there is any. `--allow` names a file of `classname::name` lines (comments start with `#`) whose
difference is a known and reviewed one. `--reasons`: the reason (failure or refusal message) must be
the same too. Used by CI to compare the demand-driven compile of dependency apps against the eager
one (ADR-013 decision 6, hard rule 4), and a pass that grows the main pass's compile against one that
compiles from source (ADR-063, `--reasons`).

`--entries` is the table `tools/neighbor-diff.sh` prints: one markdown row per test whose outcome differs
between a main binary and the head binary. A difference is another status, another set of assumption ids
(a clean pass that became flagged), or another failure message; a refusal's text is not compared. FILE
holds `classname::name<TAB>entry` lines; an entry is `PINNED file:line`, `ASSUMED id ...` or
`REFUSED text`. Exit 1 while a difference has no such entry, or when neither file has a test.
"""
import re
import sys
import xml.etree.ElementTree as ET


def outcomes(path):
    out = {}
    for case in ET.parse(path).getroot().iter("testcase"):
        key = f"{case.get('classname')}::{case.get('name')}"
        if case.find("failure") is not None or case.find("error") is not None:
            status, why = "fail", (case.find("failure") if case.find("failure") is not None else case.find("error")).get("message", "")
        elif case.find("skipped") is not None:
            status, why = "not run", case.find("skipped").get("message", "")
        else:
            status, why = "pass", ""
        ids = [p.get("value", "") for p in case.iter("property") if p.get("name") == "assumptions"]
        out[key] = (status, why, ",".join(ids))
    return out


ENTRY = re.compile(r"^(PINNED \S+:\d+|ASSUMED \S+|REFUSED \S)")


def cell(o):
    status, why, ids = o
    text = status + (f", flagged ({ids})" if ids else "") + (f": {why[:160]}" if why else "")
    return text.replace("|", "\\|").replace("\n", " ")


def entries_table(path, title, main, head):
    """The neighbor table (see the module text); returns the exit code."""
    with open(path) as f:
        rows = [l.rstrip("\n").split("\t", 1) for l in f if l.strip() and not l.startswith("#")]
    entries = {r[0].strip(): r[1].strip() for r in rows if len(r) == 2}
    if title:
        print(f"### {title}\n")
    if not main and not head:
        print("neither file has a test: the app did not compile or ran nothing (see the logs)\n")
        return 1
    print("| test | main | head | entry |\n|---|---|---|---|")
    keys = sorted(set(main) | set(head))
    differ = missing = 0
    for key in keys:
        m, h = main.get(key, ("missing", "", "")), head.get(key, ("missing", "", ""))
        if m[0] == h[0] and m[2] == h[2] and (m[0] != "fail" or m[1] == h[1]):
            continue
        differ += 1
        entry = entries.get(key, "")
        if not ENTRY.match(entry):
            missing += 1
            entry = "MISSING"
        print(f"| {key} | {cell(m)} | {cell(h)} | {entry.replace('|', chr(92) + '|')} |")
    print(f"\n{differ} of {len(keys)} tests differ, {missing} without an entry\n")
    return 1 if missing else 0


def main(argv):
    allow = set()
    args = argv[1:]
    reasons = "--reasons" in args
    if reasons:
        args.remove("--reasons")
    title = ""
    if "--title" in args:
        i = args.index("--title")
        title = args[i + 1]
        del args[i : i + 2]
    entries = None
    if "--entries" in args:
        i = args.index("--entries")
        entries = args[i + 1]
        del args[i : i + 2]
    if "--allow" in args:
        i = args.index("--allow")
        with open(args[i + 1]) as f:
            allow = {l.strip() for l in f if l.strip() and not l.startswith("#")}
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__)
        return 2
    demand, eager = outcomes(args[0]), outcomes(args[1])
    if entries is not None:
        return entries_table(entries, title, demand, eager)
    bad = 0
    for key in sorted(set(demand) | set(eager)):
        d = demand.get(key, ("missing", "", ""))
        e = eager.get(key, ("missing", "", ""))
        if d[0] == e[0] and (not reasons or d[1] == e[1]):
            continue
        mark = "allowed" if key in allow else "DIFFERS"
        if key not in allow:
            bad += 1
        print(f"{mark}: {key}\n  demand: {d[0]} {d[1][:300]}\n  eager:  {e[0]} {e[1][:300]}")
    what = "outcome or reason" if reasons else "outcome"
    print(f"{len(demand)} tests compared, {bad} {what} difference(s)")
    return 1 if bad else 0


sys.exit(main(sys.argv))
