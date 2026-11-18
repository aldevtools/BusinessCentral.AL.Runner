#!/usr/bin/env python3
"""The provenance checks of tools/corpus_bump.sh for a bump from a corpus run (ADR-077, decisions 4 to 7).

    corpus_bump_provenance.py run --repo REPO --run ID --run-json FILE --artifacts FILE --corpus DIR --state OUT
    corpus_bump_provenance.py files --state FILE --corpus DIR --sha SHA --old OLD --oracle DIR --list FILE --summary OUT

run: before any JUnit file is downloaded. RUN-JSON is gh run view --json databaseId,status,conclusion,headSha,attempt,
event,headBranch,workflowDatabaseId; ARTIFACTS the run's artifact listing; DIR the corpus, already fetched (the run's
commit and the nightly runs' commits are read). Checks that the run's results manifest belongs to its latest attempt
(decision 4), the chain cap, that the manifest's modes agree with its chains and counts, that the run is green or is the
nightly exception (decision 6), that it is a push or schedule run on master, and the nightly comparison (decision 5).
Writes the run id, its attempt, the kind and the manifest to OUT.

files: after the JUnit files of the run are downloaded. LIST has one line per file: the BC version (27.5), the path of
the downloaded file and the JUnit file name the artifact held (junit-al-language.xml). A run without a manifest is a full
run from before ADR-077 only when no suite carries provenance properties (decision 4). A run with one: every suite has its
properties and the manifest's BC build, every source commit is an ancestor of SHA, a suite of this run names SHA and an
attempt of the run, a reused suite names an older run and its codeunit's declaring files are the same at its source
commit and at SHA, the corpus's select-tests.py --check-reuse at SHA keeps every reused result, each file's counts of
fresh and reused codeunits are the manifest's, and no codeunit of the current oracle (ORACLE, the oracle folder) is
missing unless the corpus diff OLD..SHA deleted or renamed its file (decision 5). Writes the oracle.toml fields of decision 7 per
major to SUMMARY (corpus_bump_edit.py reads them); prints a warning when the CI build differs from oracle.toml bc_build.

Every refusal prints "corpus_bump: refused: ..." and exits 1. gh is CORPUS_BUMP_GH (default gh).
"""
import argparse, io, json, os, re, subprocess, sys, tarfile, tempfile, tomllib, zipfile
import xml.etree.ElementTree as ET

GH = os.environ.get("CORPUS_BUMP_GH", "gh")
BRANCH = "master"
# The manifest format the corpus writes (corpus .github/scripts/provenance.py, SCHEMA). Another schema is refused: its
# fields are not known here.
SCHEMA = 1
MANIFEST = "results-manifest"
# The properties every <testsuite> carries (decision 3; corpus provenance.py, PROPS).
PROPS = ("source_run", "source_attempt", "source_commit", "bc_build", "app")
# Decision 5: the chain is at most 20 runs.
CHAIN_MAX = 20
# Decision 6: the name of the corpus job that compares the nightly's fresh results with the reused ones. The corpus
# writes it in the step that starts reuse; until then no run has it.
COMPARISON_JOB = "Nightly comparison"
# Decision 5: the nightly comparison's answer when the fresh and the reused results agree (per BC version).
IDENTICAL = "identical"
# The run modes of a manifest: "full" (every test ran, corpus provenance.py manifest) and "subset" (a run that reuses
# results, ADR-077 context).
MODES = ("full", "subset")
# The app property of a suite (corpus provenance.py stamp: --app full or partial).
APPS = ("full", "partial")
# The jobs of corpus ci.yml a nightly run may skip: the coverage job runs only for pull_request (its "if").
NIGHTLY_SKIPPED = ("Coverage facts must not shrink",)
# The corpus writes a manifest for push runs (ci.yml, job "Results manifest") and for the nightly run (decision 6).
EVENTS = ("push", "schedule")
# Decision 5: the corpus script whose --check-reuse mode answers whether a reused result still holds at the new pin. It
# loads check-fixture-edit.py from its own folder, so the whole folder is taken from the pin.
SELECT_TESTS = ".github/scripts/select-tests.py"


def refuse(msg):
    print("corpus_bump: refused: " + msg, file=sys.stderr)
    sys.exit(1)


def gh(*args):
    p = subprocess.run([GH, *args], capture_output=True)
    if p.returncode != 0:
        refuse("gh %s failed: %s" % (" ".join(args), p.stderr.decode(errors="replace").strip()))
    return p.stdout


def git(corpus, *args, check=True):
    p = subprocess.run(["git", "-C", corpus, *args], capture_output=True, text=True)
    if check and p.returncode != 0:
        refuse("git %s failed: %s" % (" ".join(args), p.stderr.strip()))
    return p


# ---------- the manifest ----------

def download_manifest(repo, artifacts):
    """The newest results-manifest artifact of an artifact listing, parsed; None when the run has none."""
    found = [a for a in artifacts.get("artifacts", []) if a.get("name") == MANIFEST]
    if not found:
        return None
    newest = max(found, key=lambda a: a.get("created_at", ""))
    data = gh("api", "repos/%s/actions/artifacts/%s/zip" % (repo, newest["id"]))
    try:
        with zipfile.ZipFile(io.BytesIO(data)) as z:
            names = [n for n in z.namelist() if n.endswith(".json")]
            if len(names) != 1:
                refuse("artifact %s (%s) does not hold exactly one JSON file" % (newest["id"], MANIFEST))
            return json.loads(z.read(names[0]))
    except (zipfile.BadZipFile, json.JSONDecodeError) as e:
        refuse("artifact %s (%s) is not a zipped JSON manifest: %s" % (newest["id"], MANIFEST, e))


def check_manifest_belongs(run_id, run, manifest):
    """Decision 4: the manifest counts only for the run's latest attempt (and its schema is the known one). RUN needs
    headSha and attempt. The one check for the manifest of the run and of its nightly run."""
    if manifest.get("schema") != SCHEMA:
        refuse("run %s: results manifest schema %r, this check knows schema %d" % (run_id, manifest.get("schema"), SCHEMA))
    if manifest.get("run_id") != run_id:
        refuse("run %s: its results manifest names run %r" % (run_id, manifest.get("run_id")))
    if manifest.get("commit") != run.get("headSha"):
        refuse("run %s: its results manifest names commit %r, the run commit %r" % (run_id, manifest.get("commit"), run.get("headSha")))
    if manifest.get("run_attempt") != run.get("attempt"):
        refuse("run %s: its results manifest is of attempt %r, the run's latest attempt is %r"
               % (run_id, manifest.get("run_attempt"), run.get("attempt")))
    if not isinstance(manifest.get("versions"), dict) or not manifest["versions"]:
        refuse("run %s: its results manifest names no BC version" % run_id)


def version_is_full(v):
    return v.get("mode") == "full" and v.get("chain") == 0 and all(
        l.get("reused") == 0 for l in (v.get("legs") or {}).values())


def check_manifest_shape(run_id, manifest):
    """A version's mode is one of MODES and says full exactly when its chain is 0 and no leg reused a codeunit; the base
    run is null exactly when every version is full. The files stage then checks the leg counts against the files."""
    for v, e in sorted(manifest["versions"].items()):
        if e.get("mode") not in MODES:
            refuse("run %s, BC %s: mode %r, expected one of %s" % (run_id, v, e.get("mode"), ", ".join(MODES)))
        if not isinstance(e.get("legs"), dict) or not e["legs"]:
            refuse("run %s, BC %s: its results manifest names no leg" % (run_id, v))
        for name, leg in sorted(e["legs"].items()):
            if not all(isinstance(leg.get(k), int) and leg[k] >= 0 for k in ("fresh", "reused")):
                refuse("run %s, BC %s, %s: fresh %r and reused %r are not counts"
                       % (run_id, v, name, leg.get("fresh"), leg.get("reused")))
        reused = sum(l["reused"] for l in e["legs"].values())
        if (e["mode"] == "full") != (e["chain"] == 0 and reused == 0):
            refuse("run %s, BC %s: mode %s with chain %d and %d reused codeunits" % (run_id, v, e["mode"], e["chain"], reused))
    if (manifest.get("base_run") is None) != is_full(manifest):
        refuse("run %s: base run %r, but %s" % (run_id, manifest.get("base_run"),
                                               "every version is full" if is_full(manifest) else "a version reuses results"))


def is_full(manifest):
    """Every version of the manifest ran every test: mode full, chain 0, no reused codeunit (check_manifest_shape and
    the files stage tie this to the files)."""
    return all(version_is_full(v) for v in manifest["versions"].values())


# ---------- run ----------

def nightly_jobs_ok(jobs):
    """Decision 6: every test leg and the manifest job succeeded and the comparison job is the one failure. A job may be
    skipped only when it is one of NIGHTLY_SKIPPED."""
    jobs = [(j.get("name") or "", j.get("conclusion")) for j in jobs]
    ok = all(c == "success" or (n == COMPARISON_JOB and c == "failure") or (n in NIGHTLY_SKIPPED and c == "skipped")
             for n, c in jobs)
    return (ok and [n for n, _ in jobs].count(COMPARISON_JOB) == 1 and ("Results manifest", "success") in jobs
            and any(n.startswith("BC ") and c == "success" for n, c in jobs))


def check_green(repo, run_id, run, manifest):
    """Decision 5, first bullet, with the exception of decision 6. Returns "green" or "nightly"."""
    status, conclusion = run.get("status"), run.get("conclusion")
    if status != "completed":
        refuse("run %s is not completed (status: %s)" % (run_id, status or "none"))
    if conclusion == "success":
        return "green"
    if manifest is not None and run.get("event") == "schedule" and conclusion == "failure" and is_full(manifest):
        jobs = json.loads(gh("run", "view", str(run_id), "-R", repo, "--attempt", str(run.get("attempt")), "--json", "jobs"))
        if nightly_jobs_ok(jobs.get("jobs", [])):
            print("run %s: the nightly full run, only its job %r failed (ADR-077 decision 6)" % (run_id, COMPARISON_JOB))
            return "nightly"
    refuse("run %s is not green (conclusion: %s)" % (run_id, conclusion or "none"))


def check_chain(run_id, manifest):
    """Decision 5: the chain is at most CHAIN_MAX runs, per BC version."""
    for v, e in sorted(manifest["versions"].items()):
        chain = e.get("chain")
        if not isinstance(chain, int) or chain < 0 or chain > CHAIN_MAX:
            refuse("run %s, BC %s: chain %r, at most %d runs" % (run_id, v, chain, CHAIN_MAX))


def is_ancestor(corpus, a, b):
    """git merge-base --is-ancestor: exit 0 is yes, exit 1 is no; any other exit (a commit git cannot read) refuses."""
    p = git(corpus, "merge-base", "--is-ancestor", a, b, check=False)
    if p.returncode not in (0, 1):
        refuse("git merge-base --is-ancestor %s %s failed: %s" % (a, b, p.stderr.strip()))
    return p.returncode == 0


def check_nightly_comparison(repo, corpus, run_id, run, manifest):
    """Decision 5: the newest nightly run at or before the run's commit does not report a difference.

    Its manifest must be of its latest attempt and say IDENTICAL for every BC version of the run's manifest, and the
    nightly must have ended green. A nightly without that answer is refused like a difference: nothing shows that the
    reused results agree. With no nightly at or before the commit, a
    full run passes (no comparison is open) and a run that reuses results is refused (nothing compared them)."""
    wf = run.get("workflowDatabaseId")
    if not wf:
        refuse("run %s: gh named no workflow, so its nightly runs cannot be listed" % run_id)
    nightlies = json.loads(gh("run", "list", "-R", repo, "--workflow", str(wf), "--event", "schedule", "--branch", BRANCH,
                              "--status", "completed", "-L", "50", "--json", "databaseId,headSha,attempt,conclusion,createdAt"))
    before = [n for n in nightlies if is_ancestor(corpus, n["headSha"], run["headSha"])]
    if not before:
        if is_full(manifest):
            print("run %s: no nightly run at or before its commit, a full run (no comparison open)" % run_id)
            return
        refuse("run %s reuses results and no nightly run at or before its commit compared them" % run_id)
    n = max(before, key=lambda r: r.get("createdAt", ""))
    nid = n["databaseId"]
    arts = json.loads(gh("api", "repos/%s/actions/runs/%s/artifacts?per_page=100" % (repo, nid)))
    nm = download_manifest(repo, arts)
    if nm is None:
        refuse("run %s: the newest nightly run at or before its commit, %s, has no results manifest" % (run_id, nid))
    check_manifest_belongs(nid, n, nm)
    for v in sorted(manifest["versions"]):
        cmp = (nm["versions"].get(v) or {}).get("comparison")
        if cmp != IDENTICAL:
            refuse("run %s: the nightly run %s reports comparison %r for BC %s; a difference blocks the bump until it is "
                   "classified (ADR-077 decision 5)" % (run_id, nid, cmp, v))
    if n.get("conclusion") != "success":
        refuse("run %s: the nightly run %s ended with conclusion %s, so its comparison does not count (ADR-077 decision 4)"
               % (run_id, nid, n.get("conclusion")))
    print("run %s: nightly run %s at or before its commit reports no difference" % (run_id, nid))


def cmd_run(a):
    run_id = int(a.run)
    run = json.load(open(a.run_json))
    manifest = download_manifest(a.repo, json.load(open(a.artifacts)))
    if manifest is None:
        kind = check_green(a.repo, run_id, run, None)
        print("run %s: no results manifest, a full run from before ADR-077 only if no suite carries provenance" % run_id)
    else:
        check_manifest_belongs(run_id, run, manifest)
        check_chain(run_id, manifest)
        check_manifest_shape(run_id, manifest)
        kind = check_green(a.repo, run_id, run, manifest)
        if run.get("event") not in EVENTS:
            refuse("run %s: event %r, a results manifest comes only from %s" % (run_id, run.get("event"), " and ".join(EVENTS)))
        if run.get("headBranch") != BRANCH:
            refuse("run %s ran on branch %r, not on %s" % (run_id, run.get("headBranch"), BRANCH))
        if kind != "nightly":
            check_nightly_comparison(a.repo, a.corpus, run_id, run, manifest)
    json.dump({"run_id": run_id, "attempt": run.get("attempt"), "kind": kind, "manifest": manifest}, open(a.state, "w"))


# ---------- files ----------

def suites(path):
    try:
        return ET.parse(path).getroot().iter("testsuite")
    except ET.ParseError as e:
        refuse("%s is not a JUnit file: %s" % (path, e))


def props_of(suite):
    p = suite.find("properties")
    if p is None:
        return {}
    return {x.get("name"): x.get("value") for x in p.findall("property") if x.get("name") in PROPS}


def codeunit_of(suite):
    m = re.fullmatch(r"Codeunit (\d+)", suite.get("name") or "")
    return int(m.group(1)) if m else None


DECLARATION = re.compile(r"\s*codeunit\s+(\d+)(\s|$)", re.I)
_trees = {}


def tree(corpus, commit, app_dir):
    """({codeunit: sorted files under app_dir that declare it}, {file: blob id}) at commit: one git grep and one git
    ls-tree per commit and folder, kept for the whole check."""
    key = (commit, app_dir)
    if key not in _trees:
        p = git(corpus, "grep", "-I", "-i", "-E", "--null", "-e", r"^[[:space:]]*codeunit[[:space:]]+[0-9]+([[:space:]]|$)",
                commit, "--", app_dir, check=False)
        if p.returncode not in (0, 1):
            refuse("git grep at %s failed: %s" % (commit[:12], p.stderr.strip()))
        decl = {}
        for line in p.stdout.splitlines():
            name, _, text = line.partition("\0")
            m = DECLARATION.match(text)
            if m:
                decl.setdefault(int(m.group(1)), set()).add(name.split(":", 1)[1])
        blobs = {}
        for entry in git(corpus, "ls-tree", "-r", "-z", commit, "--", app_dir).stdout.split("\0"):
            if "\t" in entry:
                meta, path = entry.split("\t", 1)
                blobs[path] = meta.split()[2]
        _trees[key] = ({cu: sorted(fs) for cu, fs in decl.items()}, blobs)
    return _trees[key]


def declaring_files(corpus, commit, app_dir, cu):
    """The files under app_dir at commit that declare codeunit cu."""
    return tree(corpus, commit, app_dir)[0].get(cu, [])


def check_reused_unchanged(corpus, sha, app_dir, cu, source_commit, where):
    """Decision 5: the files that declare a reused codeunit are identical at its source commit and at the new pin."""
    then, now = declaring_files(corpus, source_commit, app_dir, cu), declaring_files(corpus, sha, app_dir, cu)
    blobs_then, blobs_now = tree(corpus, source_commit, app_dir)[1], tree(corpus, sha, app_dir)[1]
    if not then or then != now or any(blobs_then.get(f) != blobs_now.get(f) for f in then):
        refuse("%s: codeunit %d is reused from commit %s, but the files that declare it differ at the new pin (%s then, %s now)"
               % (where, cu, source_commit[:12], ", ".join(then) or "none", ", ".join(now) or "none"))


def check_reuse_selection(corpus, sha, reused):
    """Decision 5: the corpus's selection rule, as the new pin has it, keeps every reused result. reused: {source commit:
    {test app dir: set of codeunits}}. select-tests.py --check-reuse refuses a codeunit of source commit S that the
    selection of S..SHA selects, and every codeunit of S when that span is a full run for a reason other than the file
    cap or an empty selection; a pin whose script lacks the mode, or has no script, is refused too."""
    if not reused:
        return
    folder = os.path.dirname(SELECT_TESTS)
    p = subprocess.run(["git", "-C", corpus, "archive", "--format=tar", sha, folder], capture_output=True)
    if p.returncode != 0:
        refuse("%d source commits hold reused results, but the corpus at the new pin %s has no %s to run "
               "select-tests.py --check-reuse: %s" % (len(reused), sha[:12], folder, p.stderr.decode(errors="replace").strip()))
    given = {src: {app: ",".join(map(str, sorted(cus))) for app, cus in sorted(apps.items())}
             for src, apps in sorted(reused.items())}
    with tempfile.TemporaryDirectory() as d:
        tarfile.open(fileobj=io.BytesIO(p.stdout)).extractall(d, filter="data")
        q = subprocess.run([sys.executable, "-I", os.path.join(d, SELECT_TESTS), "--check-reuse", json.dumps(given),
                            "--head", sha, "--repo", corpus], capture_output=True, text=True)
    if q.returncode != 0:
        refuse("select-tests.py --check-reuse of the corpus at the new pin %s exited %d, so reused results may be stale:\n%s"
               % (sha[:12], q.returncode, (q.stdout + q.stderr).strip()))


def check_missing(corpus, old, sha, app_dir, old_file, new_ids, where):
    """Decision 5: no codeunit of the current oracle is missing unless the corpus diff deleted or renamed its file."""
    if not os.path.isfile(old_file):
        return
    missing = sorted({c for c in map(codeunit_of, suites(old_file)) if c is not None} - new_ids)
    if not missing:
        return
    if git(corpus, "cat-file", "-e", old + "^{commit}", check=False).returncode != 0:
        refuse("%s: codeunits %s of the current oracle are missing and the old pin %s is not in the corpus repository"
               % (where, missing[:5], old[:12]))
    gone = set()
    for line in git(corpus, "diff", "--name-status", "-M", old, sha, "--", app_dir).stdout.splitlines():
        f = line.split("\t")
        if f[0] == "D" or f[0].startswith("R"):
            gone.add(f[1])
    for cu in missing:
        files = declaring_files(corpus, old, app_dir, cu)
        if not files or any(f not in gone for f in files):
            refuse("%s: codeunit %d of the current oracle is missing, and the corpus diff %s..%s did not delete or rename "
                   "its file (%s)" % (where, cu, old[:12], sha[:12], ", ".join(files) or "no file declares it"))


def cmd_files(a):
    state = json.load(open(a.state))
    run_id, attempt, manifest = state["run_id"], state["attempt"], state["manifest"]
    files = [line.split() for line in open(a.list) if line.strip()]
    if manifest is None:
        for leg, path, name in files:
            for s in suites(path):
                if props_of(s):
                    refuse("run %s has no results manifest, but %s of BC %s carries provenance properties (suite %s): "
                           "only a full run from before ADR-077 may come without one" % (run_id, name, leg, s.get("name")))
        json.dump({}, open(a.summary, "w"))
        return
    ancestors = {}
    per_version = {}
    reused_by_source = {}
    for leg, path, name in files:
        ver = manifest["versions"].get(leg)
        if ver is None:
            refuse("run %s: its results manifest has no BC version %s" % (run_id, leg))
        entry = ver.get("legs", {}).get(name)
        if entry is None or not entry.get("test_app_dir"):
            refuse("run %s: its results manifest has no leg %s for BC %s" % (run_id, name, leg))
        app_dir = entry["test_app_dir"]
        acc = per_version.setdefault(leg, {"fresh": 0, "reused": 0, "runs": set()})
        ids = set()
        fresh = reused = 0
        for s in suites(path):
            where = "run %s, BC %s, %s, suite %s" % (run_id, leg, name, s.get("name"))
            p = props_of(s)
            if set(p) != set(PROPS) or not all(p.values()):
                refuse("%s: provenance properties %s, expected all of %s" % (where, sorted(p), ", ".join(PROPS)))
            if p["bc_build"] != ver.get("bc_build"):
                refuse("%s: BC build %s, the manifest names %s for BC %s" % (where, p["bc_build"], ver.get("bc_build"), leg))
            c = p["source_commit"]
            if c not in ancestors:
                ancestors[c] = is_ancestor(a.corpus, c, a.sha)
            if not ancestors[c]:
                refuse("%s: source commit %s is not an ancestor of the run's commit %s" % (where, c[:12], a.sha[:12]))
            cu = codeunit_of(s)
            if cu is None:
                refuse("%s: not a codeunit suite" % where)
            ids.add(cu)
            if not p["source_run"].isdigit() or not p["source_attempt"].isdigit():
                refuse("%s: source run %r, source attempt %r" % (where, p["source_run"], p["source_attempt"]))
            if p["app"] not in APPS:
                refuse("%s: app %r, expected one of %s" % (where, p["app"], ", ".join(APPS)))
            src = int(p["source_run"])
            acc["runs"].add(src)
            if src == run_id:
                # A suite of this run ran at the run's commit (corpus provenance.py manifest) in one of its attempts.
                if c != a.sha:
                    refuse("%s: names this run but source commit %s, the run's commit is %s" % (where, c[:12], a.sha[:12]))
                if int(p["source_attempt"]) > attempt:
                    refuse("%s: source attempt %s, the run is at attempt %s" % (where, p["source_attempt"], attempt))
                fresh += 1
            else:
                if src >= run_id:
                    refuse("%s: reused from run %d, which is not older than run %d" % (where, src, run_id))
                reused += 1
                check_reused_unchanged(a.corpus, a.sha, app_dir, cu, c, where)
                reused_by_source.setdefault(c, {}).setdefault(app_dir, set()).add(cu)
        if (fresh, reused) != (entry.get("fresh"), entry.get("reused")):
            refuse("run %s, BC %s, %s: %d fresh and %d reused codeunits, the manifest names %r and %r"
                   % (run_id, leg, name, fresh, reused, entry.get("fresh"), entry.get("reused")))
        acc["fresh"] += fresh
        acc["reused"] += reused
        old_file = os.path.join(a.oracle, leg.split(".")[0], os.path.basename(path))
        check_missing(a.corpus, a.old, a.sha, app_dir, old_file, ids, "run %s, BC %s, %s" % (run_id, leg, name))
    check_reuse_selection(a.corpus, a.sha, reused_by_source)
    builds = {}
    toml_path = os.path.join(a.oracle, "oracle.toml")
    if os.path.isfile(toml_path):
        with open(toml_path, "rb") as f:
            builds = {str(v["major"]): v.get("bc_build") for v in tomllib.load(f).get("version", [])}
    summary = {}
    for leg, acc in sorted(per_version.items()):
        ver = manifest["versions"][leg]
        if len(acc["runs"]) > ver["chain"] + 1:
            refuse("run %s, BC %s: results from %d runs, more than its chain %d allows"
                   % (run_id, leg, len(acc["runs"]), ver["chain"]))
        major = leg.split(".")[0]
        summary[major] = {"mode": ver.get("mode"), "fresh": acc["fresh"], "reused": acc["reused"],
                          "oldest_source_run": min(acc["runs"]), "ci_bc_build": ver.get("bc_build")}
        print("BC %s: mode %s, %d fresh and %d reused codeunits, oldest source run %d, CI build %s"
              % (leg, ver.get("mode"), acc["fresh"], acc["reused"], min(acc["runs"]), ver.get("bc_build")))
        if builds.get(major) and builds[major] != ver.get("bc_build"):
            print("WARNING: BC %s: CI ran build %s, oracle.toml bc_build (the build the runner loads) is %s"
                  % (leg, ver.get("bc_build"), builds[major]))
    json.dump(summary, open(a.summary, "w"))


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("run")
    for x in ("--repo", "--run", "--run-json", "--artifacts", "--corpus", "--state"):
        r.add_argument(x, required=True)
    f = sub.add_parser("files")
    for x in ("--state", "--corpus", "--sha", "--old", "--oracle", "--list", "--summary"):
        f.add_argument(x, required=True)
    a = ap.parse_args()
    cmd_run(a) if a.cmd == "run" else cmd_files(a)


if __name__ == "__main__":
    main()
