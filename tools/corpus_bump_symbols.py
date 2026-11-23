#!/usr/bin/env python3
"""The preprocessor symbol check of tools/corpus_bump.sh (ADR-078 point 3).

    corpus_bump_symbols.py --state FILE --oracle DIR --out FILE

The corpus keeps one list of released minors and derives each leg's preprocessor symbols from it. The results
manifest of the run (in STATE, written by tools/corpus_bump_provenance.py run) records the list (top-level
`released_minors`) and each version's symbols (`versions.<v>.preprocessor_symbols`). The runner derives the same
symbols in `al_version::corpus_symbols`; this check asks `al-test-cli --print-symbols V --released-minors LIST` for
each version and refuses when a set differs, so a bump never pins a corpus whose symbols the runner does not define.

When the manifest records the list, OUT gets it as a JSON list (tools/corpus_bump_edit.py writes it into oracle.toml).
A list in oracle.toml (DIR/oracle.toml) never goes away by a bump: a run without one is refused. Neither recording a
list keeps the major symbols, as before ADR-078.

The CLI is $CORPUS_BUMP_CLI, else target/release/al-test-cli, built only when the manifest records a list.
"""
import argparse, json, os, subprocess, sys, tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)


def refuse(msg):
    print("corpus_bump: refused: " + msg, file=sys.stderr)
    sys.exit(1)


def cli():
    """The al-test-cli that prints the runner's symbols."""
    if os.environ.get("CORPUS_BUMP_CLI"):
        return os.environ["CORPUS_BUMP_CLI"]
    r = subprocess.run(["cargo", "build", "--release", "-q", "-p", "al-test-cli"], cwd=ROOT)
    if r.returncode != 0:
        refuse("building al-test-cli for the preprocessor symbol check failed (output above)")
    return os.path.join(ROOT, "target", "release", "al-test-cli")


def runner_symbols(exe, version, released):
    r = subprocess.run([exe, "--root", ROOT, "--print-symbols", version, "--released-minors", ",".join(released)],
                       capture_output=True, text=True)
    if r.returncode != 0:
        refuse("al-test-cli --print-symbols %s --released-minors %s failed: %s"
               % (version, ",".join(released), (r.stderr or r.stdout).strip()))
    return sorted(s for s in r.stdout.strip().split(",") if s)


def main():
    ap = argparse.ArgumentParser()
    for x in ("--state", "--oracle", "--out"):
        ap.add_argument(x, required=True)
    a = ap.parse_args()
    state = json.load(open(a.state))
    run_id, manifest = state["run_id"], state["manifest"]
    with open(os.path.join(a.oracle, "oracle.toml"), "rb") as f:
        recorded = tomllib.load(f).get("released_minors")
    if manifest is None:
        if recorded is not None:
            refuse("oracle.toml records released minors, but run %s has no results manifest" % run_id)
        print("no released minors recorded: the preprocessor symbols stay BC<major>PLUS")
        return
    released = manifest.get("released_minors")
    versions = manifest["versions"]
    named = sorted(v for v, e in versions.items() if "preprocessor_symbols" in e)
    if released is None:
        if named:
            refuse("BC %s: the results manifest names preprocessor symbols but no released minors" % ", ".join(named))
        if recorded is not None:
            refuse("oracle.toml records released minors, but the results manifest of run %s has none" % run_id)
        print("no released minors recorded: the preprocessor symbols stay BC<major>PLUS")
        return
    if not isinstance(released, list) or not all(isinstance(m, str) for m in released) or not released:
        refuse("run %s: the results manifest's released minors %r are not a list of versions" % (run_id, released))
    exe = cli()
    for v in sorted(versions):
        given = versions[v].get("preprocessor_symbols")
        if given is None:
            refuse("BC %s: the results manifest records released minors but no preprocessor symbols" % v)
        if not isinstance(given, list) or not all(isinstance(s, str) for s in given):
            refuse("BC %s: the results manifest's preprocessor symbols %r are not a list of names" % (v, given))
        ours = runner_symbols(exe, v, released)
        if sorted(given) != ours:
            refuse("BC %s: the results manifest names the preprocessor symbols %s, al-test-cli --print-symbols %s gives "
                   "%s: the runner's al_version::corpus_symbols and the corpus's list of released minors disagree"
                   % (v, ",".join(sorted(given)), v, ",".join(ours)))
    print("preprocessor symbols of BC %s agree with al-test-cli --print-symbols" % ", ".join(sorted(versions)))
    print("released minors: %s" % ", ".join(released))
    json.dump(released, open(a.out, "w"))


if __name__ == "__main__":
    main()
