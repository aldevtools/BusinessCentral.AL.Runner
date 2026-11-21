#!/usr/bin/env python3
"""Compares the test outcomes of two JUnit files written by `al-test-cli --junit` or `al-runner --junit`.

Usage: compare_junit.py DEMAND.xml EAGER.xml [--allow FILE] [--reasons]
       compare_junit.py --entries FILE [--title TEXT] [--major N] [--root DIR] MAIN.xml HEAD.xml

Prints every test whose outcome (pass, fail, not run) differs, with both reasons, and exits 1 if
there is any. `--allow` names a file of `classname::name` lines (comments start with `#`) whose
difference is a known and reviewed one. `--reasons`: the reason (failure or refusal message) must be
the same too. Used by CI to compare the demand-driven compile of dependency apps against the eager
one (ADR-013 decision 6, hard rule 4), and a pass that grows the main pass's compile against one that
compiles from source (ADR-063, `--reasons`).

`--entries` is the table `tools/neighbor-diff.sh` prints: one markdown row per test whose outcome differs
between a main binary and the head binary. A difference is another status, another set of assumption ids
(a clean pass that became flagged), or another failure message; a refusal's text is not compared. FILE
holds `classname::name<TAB>entry` lines (tests/neighbors/README.md); a key ending in ` @N` applies only with
`--major N`. An entry counts only when it states the head result and its evidence exists under `--root`
(default: this repository), see `states_head`. Exit 1 while a difference has no such entry, while an entry
has no difference (stale), when neither file has a test, when a side has a test stopped by a resource limit,
or when no test ran on either side.
"""
import os
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


ID = re.compile(r"[a-z0-9]+(-[a-z0-9]+)*")


def states_head(entry, head, root):
    """Whether `entry` states the head result `head` (status, message, ids) and cites evidence that exists.

    REFUSED text: the head does not run the test, with a message that starts with text.
    ASSUMED id: the head ran through id, and id is an assumption registry entry (al-builtins build.rs: the file
    name is the id).
    PINNED file:line: the head passes or fails clean, and corpus/<file> has that line.
    """
    status, why, ids = head
    kind, _, rest = entry.partition(" ")
    if kind == "REFUSED":
        return status == "not run" and rest.strip() != "" and why.startswith(rest)
    if kind == "ASSUMED":
        aid = rest.split(" ", 1)[0]
        flags = [i.strip() for i in ids.split(",")]
        registry = os.path.join(root, "crates", "al-builtins", "src", "assumptions", aid + ".rs")
        return ID.fullmatch(aid) is not None and aid in flags and os.path.isfile(registry)
    if kind == "PINNED":
        m = re.fullmatch(r"([\w./-]+):([1-9]\d*)(\s.*)?", rest)
        if not m or ids or status not in ("pass", "fail") or ".." in m.group(1).split("/"):
            return False
        path = os.path.join(root, "corpus", m.group(1))
        if not os.path.isfile(path):
            return False
        with open(path, "rb") as f:
            return sum(1 for _ in f) >= int(m.group(2))
    return False


def cell(o):
    status, why, ids = o
    text = status + (f", flagged ({ids})" if ids else "") + (f": {why[:160]}" if why else "")
    return text.replace("|", "\\|").replace("\n", " ")


# How the reason of a test starts that al-runner did not run because the run hit a resource limit, not because of the
# test: crates/al-testrun/src/runtime/memory.rs (REASON) and runtime/run.rs (TIME_LIMIT_REASON). The neighbor_diff
# tests of al-test-cli check that both texts are still the runtime's.
RESOURCE_LIMITS = ("memory limit (", "the test used more than the time limit of ")


def not_comparable(main, head):
    """Why the two runs cannot be compared, or None: a side has a test stopped by a resource limit, or no test ran."""
    for side, out in (("main", main), ("head", head)):
        hit = [k for k in sorted(out) if out[k][0] == "not run" and out[k][1].startswith(RESOURCE_LIMITS)]
        if hit:
            n = len(hit)
            return f"{side}: {n} test{'s' if n > 1 else ''} not run for a resource limit, first {hit[0]}: {out[hit[0]][1]}"
    both = {**main, **head}
    if all(o[0] == "not run" for o in both.values()):
        first = sorted(both)[0]
        return f"no test ran on either side, first {first}: {both[first][1]}"
    return None


def read_entries(path, major):
    """The entries of FILE that apply on BC major `major`: key -> entry."""
    with open(path) as f:
        rows = [l.rstrip("\n").split("\t", 1) for l in f if l.strip() and not l.startswith("#")]
    entries = {}
    for r in rows:
        if len(r) != 2:
            continue
        key = r[0].strip()
        m = re.fullmatch(r"(.*) @(\d+)", key)
        if m and m.group(2) != major:
            continue
        entries[m.group(1) if m else key] = (key, r[1].strip())
    return entries


def entries_table(path, title, main, head, root, major):
    """The neighbor table (see the module text); returns the exit code."""
    entries = read_entries(path, major)
    if title:
        print(f"### {title}\n")
    if not main and not head:
        print("neither file has a test: the app did not compile or ran nothing (see the logs)\n")
        return 1
    why = not_comparable(main, head)
    if why:
        print(f"not compared, {why} (see the logs)\n")
        return 1
    print("| test | main | head | entry |\n|---|---|---|---|")
    keys = sorted(set(main) | set(head))
    differ = missing = 0
    used = set()
    for key in keys:
        m, h = main.get(key, ("missing", "", "")), head.get(key, ("missing", "", ""))
        if m[0] == h[0] and m[2] == h[2] and (m[0] != "fail" or m[1] == h[1]):
            continue
        differ += 1
        used.add(key)
        entry = entries.get(key, ("", ""))[1]
        if not states_head(entry, h, root):
            missing += 1
            entry = "MISSING"
        print(f"| {key} | {cell(m)} | {cell(h)} | {entry.replace('|', chr(92) + '|')} |")
    print(f"\n{differ} of {len(keys)} tests differ, {missing} without an entry\n")
    stale = sorted(written for key, (written, _) in entries.items() if key not in used)
    for written in stale:
        print(f"stale entry, no difference: {written}")
    if stale:
        print()
    return 1 if missing or stale else 0


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
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    if "--root" in args:
        i = args.index("--root")
        root = args[i + 1]
        del args[i : i + 2]
    major = ""
    if "--major" in args:
        i = args.index("--major")
        major = args[i + 1]
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
        return entries_table(entries, title, demand, eager, root, major)
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
