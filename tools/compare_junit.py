#!/usr/bin/env python3
"""Compares the test outcomes of two JUnit files written by `al-test-cli --junit`.

Usage: compare_junit.py DEMAND.xml EAGER.xml [--allow FILE] [--reasons]

Prints every test whose outcome (pass, fail, not run) differs, with both reasons, and exits 1 if
there is any. `--allow` names a file of `classname::name` lines (comments start with `#`) whose
difference is a known and reviewed one. `--reasons`: the reason (failure or refusal message) must be
the same too. Used by CI to compare the demand-driven compile of dependency apps against the eager
one (ADR-013 decision 6, hard rule 4), and a pass that grows the main pass's compile against one that
compiles from source (ADR-063, `--reasons`).
"""
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
        out[key] = (status, why)
    return out


def main(argv):
    allow = set()
    args = argv[1:]
    reasons = "--reasons" in args
    if reasons:
        args.remove("--reasons")
    if "--allow" in args:
        i = args.index("--allow")
        with open(args[i + 1]) as f:
            allow = {l.strip() for l in f if l.strip() and not l.startswith("#")}
        del args[i : i + 2]
    if len(args) != 2:
        print(__doc__)
        return 2
    demand, eager = outcomes(args[0]), outcomes(args[1])
    bad = 0
    for key in sorted(set(demand) | set(eager)):
        d = demand.get(key, ("missing", ""))
        e = eager.get(key, ("missing", ""))
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
