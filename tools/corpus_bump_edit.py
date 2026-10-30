#!/usr/bin/env python3
"""The oracle.toml and oracle/README.md edit of tools/corpus_bump.sh (issue 44).

    corpus_bump_edit.py ROOT SHA RUN [MAJOR ORACLE_ID ONPREM_ID CASES ONPREM_CASES]...

Writes the new pin, the run and the artifact ids of each leg (the version's own oracle and its onprem suite;
other suites, such as the isolation probe from another run, are left alone), and the README's pin and leg rows.
"""
import os, re, sys
root, sha, run, rest = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
# DOTNET_IDS="27=ID 28=ID 29=ID": the artifact ids of the OnPrem DotNet files (suite onprem-dotnet).
dotnet = {k: v for k, v in (x.split("=") for x in os.environ.get("DOTNET_IDS", "").split()) if v != "0"}
# TESTLIB_IDS="27=ID 28=ID 29=ID": the artifact ids of the TestLibraries files (suite testlibraries).
testlib = {k: v for k, v in (x.split("=") for x in os.environ.get("TESTLIB_IDS", "").split()) if v != "0"}
legs = [rest[i:i + 5] for i in range(0, len(rest), 5)]
# oracle.toml: the pin, then per [[version]] table its run and artifact, and its onprem suite's.
p = root + "/oracle/oracle.toml"
t = open(p).read()
blocks = re.split(r'(?m)^(?=\[\[version\]\])', t)
# The pin is the top-level corpus_commit only; a suite with its own corpus_commit (the isolation probe,
# from another run) keeps it.
blocks[0] = re.sub(r'^corpus_commit = "[0-9a-f]+"', f'corpus_commit = "{sha}"', blocks[0], count=1, flags=re.M)
out = [blocks[0]]

def leg_fields(part, artifact):
    part = re.sub(r'^workflow_run = \d+', f'workflow_run = {run}', part, count=1, flags=re.M)
    return re.sub(r'^artifact_id = \d+', f'artifact_id = {artifact}', part, count=1, flags=re.M)

for b in blocks[1:]:
    major = re.search(r'^major = (\d+)', b, re.M).group(1)
    leg = next((l for l in legs if l[0] == major), None)
    if leg:
        _, oracle, onprem, _, _ = leg
        parts = re.split(r'(?m)^(?=\[\[version\.suite\]\])', b)
        # The version's own oracle, then its onprem suite; other suites come from other runs.
        parts = [leg_fields(parts[0], oracle)] + [
            leg_fields(s, onprem) if re.search(r'^name = "onprem"', s, re.M)
            else leg_fields(s, dotnet[major]) if re.search(r'^name = "onprem-dotnet"', s, re.M) and major in dotnet
            else leg_fields(s, testlib[major]) if re.search(r'^name = "testlibraries"', s, re.M) and major in testlib
            else s
            for s in parts[1:]
        ]
        b = "".join(parts)
    out.append(b)
open(p, "w").write("".join(out))
# README.md: the pin, and the leg table rows (run, artifact id, cases).
p = root + "/oracle/README.md"
t = open(p).read()
t = re.sub(r'(\| Corpus pin \| `)[0-9a-f]{40}(`)', rf'\g<1>{sha}\2', t)
for major, oracle, onprem, n, n_onprem in legs:
    t = re.sub(
        rf'^\| {major} \| ([0-9.]+) \| `[0-9a-f]+` \(the pin\) \| \d+ \| (`[^`]+`) \| \d+ \| \d+',
        rf'| {major} | \1 | `{sha[:7]}` (the pin) | {run} | \2 | {oracle} | {n}',
        t, flags=re.M)
open(p, "w").write(t)
