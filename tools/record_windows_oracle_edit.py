"""Rewrite the isolation suites' oracle metadata (tools/record_windows_oracle.sh).

  record_windows_oracle_edit.py ROOT SHA RUN ARTIFACT BC_VERSION DATE FUNCTION_CASES CODEUNIT_CASES DRY

In oracle/oracle.toml it rewrites corpus_commit, workflow_run, artifact_id and bc_version in exactly the two
[[version.suite]] tables named isolation-function and isolation-codeunit. In oracle/README.md it rewrites the "From"
and "Cases" cells of their table rows and the commit in the sentence "come from the nightly on corpus master at `...`".
It prints every changed line. DRY=1 prints and writes nothing. Exit 1 when a table, key or row is not found once.
"""
import re
import sys

SUITES = ("isolation-function", "isolation-codeunit")


def die(msg):
    print(f"record_windows_oracle: refused: {msg}", file=sys.stderr)
    sys.exit(1)


def edit_toml(text, sha, run, artifact, bc_version):
    lines = text.split("\n")
    starts = [i for i, l in enumerate(lines) if l.startswith("[[")] + [len(lines)]
    new = {
        "corpus_commit": f'corpus_commit = "{sha}"',
        "workflow_run": f"workflow_run = {run}",
        "artifact_id": f"artifact_id = {artifact}",
        "bc_version": f'bc_version = "{bc_version}"',
    }
    changes = []
    for name in SUITES:
        blocks = [
            (a, b)
            for a, b in zip(starts, starts[1:])
            if lines[a] == "[[version.suite]]" and f'name = "{name}"' in lines[a:b]
        ]
        if len(blocks) != 1:
            die(f"oracle/oracle.toml has {len(blocks)} [[version.suite]] tables named {name}, expected 1")
        a, b = blocks[0]
        for key, line in new.items():
            at = [i for i in range(a, b) if re.match(rf"{key} = ", lines[i])]
            if len(at) != 1:
                die(f"the {name} table of oracle/oracle.toml has {len(at)} '{key}' lines, expected 1")
            if lines[at[0]] != line:
                changes.append(f"oracle/oracle.toml [{name}]: {lines[at[0]]} -> {line}")
                lines[at[0]] = line
    return "\n".join(lines), changes


def set_cases(cell, n):
    if not re.match(r" \d+", cell):
        die(f"oracle/README.md: the Cases cell '{cell.strip()}' does not start with a number")
    return re.sub(r"^ \d+", f" {n}", cell)


def edit_readme(text, sha, run, artifact, bc_version, date, cases):
    lines = text.split("\n")
    changes = []
    for name in SUITES:
        at = [i for i, l in enumerate(lines) if l.startswith(f"| `{name}`")]
        if len(at) != 1:
            die(f"oracle/README.md has {len(at)} table rows for {name}, expected 1")
        cells = lines[at[0]].split("|")
        # ['', name, app, oracle file, from, cases, '']
        if len(cells) != 7:
            die(f"oracle/README.md: the {name} row does not have five cells")
        if name == "isolation-function":
            cells[4] = (
                f" the Windows nightly of {date}, BC {bc_version}, run {run}, artifact {artifact},"
                f" corpus master {sha[:8]} "
            )
        cells[5] = set_cases(cells[5], cases[name])
        row = "|".join(cells)
        if row != lines[at[0]]:
            changes.append(f"oracle/README.md: {lines[at[0]]}\n  -> {row}")
            lines[at[0]] = row
    out = "\n".join(lines)
    pat = r"(come from the nightly on corpus master at `)([0-9a-f]+)(`)"
    if len(re.findall(pat, out)) != 1:
        die("oracle/README.md does not say once 'come from the nightly on corpus master at `<commit>`'")
    old = re.search(pat, out).group(2)
    if old != sha[:8]:
        out = re.sub(pat, rf"\g<1>{sha[:8]}\g<3>", out)
        changes.append(f"oracle/README.md: corpus master at `{old}` -> `{sha[:8]}`")
    return out, changes


def main():
    root, sha, run, artifact, bc_version, date, fcases, ccases, dry = sys.argv[1:10]
    tpath, rpath = f"{root}/oracle/oracle.toml", f"{root}/oracle/README.md"
    with open(tpath, encoding="utf-8", newline="") as f:
        toml = f.read()
    with open(rpath, encoding="utf-8", newline="") as f:
        readme = f.read()
    toml, c1 = edit_toml(toml, sha, run, artifact, bc_version)
    cases = {"isolation-function": fcases, "isolation-codeunit": ccases}
    readme, c2 = edit_readme(readme, sha, run, artifact, bc_version, date, cases)
    for c in c1 + c2:
        print(f"  {c}")
    if dry == "1":
        return
    with open(tpath, "w", encoding="utf-8", newline="") as f:
        f.write(toml)
    with open(rpath, "w", encoding="utf-8", newline="") as f:
        f.write(readme)


main()
