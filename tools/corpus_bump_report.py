#!/usr/bin/env python3
"""The pull request body of the corpus bump bot (tools/corpus_bump_bot.sh bump).

Usage: corpus_bump_report.py DIR

DIR holds what the bump wrote: meta.json (sha, old, run, repo), bump.log (tools/corpus_bump.sh), corpus-log.txt
(the corpus commits of the bump), NAME.txt and NAME.code for the checks diff (al-test-cli --baseline-diff),
awaiting (--awaiting-assumptions), gates (al-version gate_completeness), harness (corpus_harness) and golden
(corpus_resolve_deps, the generated pinned_columns.rs), and the
JUnit files of the old pin (before/) and the new pin (after/).

The body lists what is left for a person and decides none of it: a lost baseline case, a case that fails on the
new pin and did not on the old one, an assumption whose corpus pin landed, a corpus guard without a gate, a
harness count, a stale golden file. Each list is cut to stay under GitHub's limit of 65536 characters, with the full count.
"""
import json
import os
import re
import sys
import xml.etree.ElementTree as ET

LIST_CAP = 60  # lines per list
TEXT_CAP = 4000  # characters per quoted output


def read(d, name, default=""):
    try:
        with open(os.path.join(d, name), encoding="utf-8", errors="replace") as f:
            return f.read()
    except OSError:
        return default


def code(d, name):
    try:
        return int(read(d, name + ".code", "0").strip() or "0")
    except ValueError:
        return 1


def fails(folder):
    """classname name of each failed case, per JUnit file name."""
    out = {}
    if not os.path.isdir(folder):
        return out
    for f in sorted(os.listdir(folder)):
        if not f.endswith(".xml"):
            continue
        try:
            root = ET.parse(os.path.join(folder, f)).getroot()
        except ET.ParseError:
            continue
        out[f] = {
            f"{c.get('classname')} {c.get('name')}"
            for c in root.iter("testcase")
            if c.find("failure") is not None or c.find("error") is not None
        }
    return out


def capped(lines):
    lines = list(lines)
    shown = lines[:LIST_CAP]
    more = len(lines) - len(shown)
    if more > 0:
        shown.append(f"... and {more} more ({len(lines)} in all)")
    return shown


def quote(text):
    text = text.strip()
    if len(text) > TEXT_CAP:
        text = text[:TEXT_CAP] + "\n... (cut; the full output is in the workflow log)"
    return "```\n" + text + "\n```"


def regressions(diff):
    """The lines of each 'no longer passing' section of the baseline diff, with its version."""
    out, version, inside = [], None, False
    for line in diff.splitlines():
        m = re.match(r"Baseline diff, (BC [^:]+):", line)
        if m:
            version, inside = m.group(1), False
            continue
        if line.startswith("In the baseline but no longer passing"):
            inside = True
            continue
        if inside and line and not line.startswith(" "):
            inside = False
        if inside and line.startswith("    "):
            out.append(f"{version}: {line.strip()}")
    return out


def main(d):
    meta = json.loads(read(d, "meta.json", "{}") or "{}")
    sha, old, run, repo = (meta.get(k, "") for k in ("sha", "old", "run", "repo"))
    body = []
    body.append(f"Corpus bump to master `{sha[:12]}` from `{old[:12]}`, made by the corpus bump bot from corpus run "
                f"[{run}](https://github.com/{repo}/actions/runs/{run}).")
    body.append("")
    body.append("The bot does not merge. It changed only the corpus pin, `oracle/` and the harness counts in "
                "`crates/al-test-cli/tests/corpus_harness.rs`. It never removes a baseline line and writes no "
                "baseline: the regen bot adds the new passes after the merge.")

    left = []
    lost = regressions(read(d, "diff.txt"))
    if lost or code(d, "diff") != 0:
        left.append(("Baseline cases that no longer pass. Fix the code, or show in the commit that the case "
                     "passed by accident (CLAUDE.md hard rule 2)", capped(lost) or ["(see the baseline diff output below)"]))
    before, after = fails(os.path.join(d, "before")), fails(os.path.join(d, "after"))
    new = sorted(c for f, cases in after.items() for c in cases - before.get(f, set()))
    if new:
        left.append(("Cases that fail on the new pin and did not on the old one. Each is a pin disagreement: fix "
                     "the code or refuse its path", capped(new)))
    if code(d, "awaiting") != 0:
        left.append(("Assumptions whose awaited corpus pin landed: remove each one and cite the pin, or fix the "
                     "code where the pin disagrees (`al-test-cli --root . --awaiting-assumptions`)",
                     [quote(read(d, "awaiting.txt"))]))
    if code(d, "gates") != 0:
        left.append(("Version guards without a gate, or gate evidence without a guard "
                     "(`cargo test -p al-version --test gate_completeness`)", [quote(read(d, "gates.txt"))]))
    if code(d, "harness") != 0:
        left.append(("The corpus harness test fails: a count the bump script does not update "
                     "(`cargo test --release -p al-test-cli --test corpus_harness`)", [quote(read(d, "harness.txt"))]))
    if code(d, "golden") != 0:
        left.append(("Golden files compiled into the runtime are stale (`crates/al-builtins/src/pinned_columns.rs`): "
                     "regenerate with `UPDATE_GOLDEN=1 cargo test --release -p al-test-cli --test corpus_resolve_deps "
                     "-- --ignored` and review the diff", [quote(read(d, "golden.txt"))]))
    warnings = [l.strip() for l in read(d, "bump.log").splitlines() if "WARNING" in l]
    if warnings:
        left.append(("Warnings of the bump script", capped(warnings)))

    body.append("")
    body.append("## Left for a person")
    body.append("")
    if not left:
        body.append("Nothing is left for a person. The CI checks of this branch are the gates.")
    for title, items in left:
        body.append(f"- [ ] {title}:")
        for i in items:
            if i.startswith("```"):
                body.append("")
                body.append(i)
                body.append("")
            else:
                body.append(f"  - {i}")
    body.append("")
    body.append("Always by hand: strike the rows of `docs/probes-needed.md` that the new probes settle, and close "
                "the issues the bump fixes. The CI checks on this branch are the same gates as on main.")

    corpus_log = read(d, "corpus-log.txt").strip().splitlines()
    body.append("")
    body.append("## Corpus changes")
    body.append("")
    body.extend(f"- {l}" for l in capped(corpus_log) or ["(none listed)"])

    counts = [l.rstrip() for l in read(d, "bump.log").splitlines()
              if re.match(r"\s+\d+\.\d+: \d+ -> \d+", l) or l.startswith("docs/probes-needed.md")]
    if counts:
        body.append("")
        body.append("## Oracle cases per leg, before -> after (OnPrem in brackets)")
        body.append("")
        body.append(quote("\n".join(counts)))
    summary = [l for l in read(d, "diff.txt").splitlines() if l.startswith("Baseline diff, ")]
    if summary:
        body.append("")
        body.append("## Outcomes on the new pin")
        body.append("")
        body.extend(f"- {l}" for l in summary)
    print("\n".join(body))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: corpus_bump_report.py DIR")
    main(sys.argv[1])
