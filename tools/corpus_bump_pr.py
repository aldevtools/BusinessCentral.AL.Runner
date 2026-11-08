#!/usr/bin/env python3
"""The PR-results corpus bump: tools/corpus_bump.sh --from-pr N[,N...] (owner decision 2026-11-05).

A normal bump takes the oracle from a green master push run of the corpus (about 55 minutes after a corpus pull request
merges). This bump does not wait for it: the pin moves to the merge commit of the given corpus pull requests, and the
oracle is the oracle of the current pin with the cases of the codeunits each pull request's own CI run ran (a subset
run, per BC version) replaced by that run's results. Accepted risk: a difference that only a master run shows (a flaky
test, an interaction between tests) shows up after this bump; the next normal bump corrects it.

    corpus_bump_pr.py --root ROOT --prs N[,N...] [--dry-run] [--body FILE]
    corpus_bump_pr.py merge OLD NEW...      (the codeunit replacement alone; writes OLD; for tests)

It refuses, and changes nothing, when:
  - a pull request is not merged into master, or its merge commit is not a first-parent commit of master;
  - master between the current pin and the newest merge commit has a commit that is not one of the listed merges and
    changes more than CI files, docs or must-not-compile cases;
  - a pull request changed a shared fixture (tests/al-language/_fixtures, the internals fixture, the xapp apps,
    custom.ruleset.json, an app.json), deleted a test file, or changed an app the Linux CI legs do not run;
  - the pull request's last CI run on its head is not green, or a BC version lacks a JUnit file another version has;
  - master changed, between the pull request's merge base and its merge, a file of the pull request or of a codeunit
    its run ran (the run may not have tested the merged files);
  - a codeunit declared in a changed file has cases in the current oracle that the run did not run;
  - two pull requests ran the same codeunit, or an oracle file would lose cases.
For tests: CORPUS_BUMP_GH, CORPUS_BUMP_CORPUS, CORPUS_BUMP_NO_FETCH=1 as for tools/corpus_bump.sh, and
CORPUS_BUMP_PIN (the current pin; default the corpus gitlink of ROOT's HEAD).
"""
import argparse, io, json, os, re, subprocess, sys, tempfile, zipfile
import xml.etree.ElementTree as ET

REPO = "aldevtools/BusinessCentral.AL.Language.Tests"
WORKFLOW = "AL Language Test Suite"
LEGS = [("27.5", "27"), ("28.5", "28"), ("29.0", "29")]
# The JUnit file an artifact holds -> (the test app it reports, the oracle file of a major).
ROLES = {
    "junit-al-language.xml": ("tests/al-language", "junit.xml"),
    "junit-al-language-onprem.xml": ("tests/al-language-onprem", "junit-onprem.xml"),
    "junit-al-language-onprem-dotnet.xml": ("tests/al-language-onprem-dotnet", "junit-dotnet.xml"),
    "junit-al-language-testlibraries.xml": ("tests/al-language-testlibraries", "junit-testlibraries.xml"),
    "junit-al-language-isolation-disabled.xml": ("tests/al-language-isolation-disabled", "junit-isolation-disabled.xml"),
    # The job "BC TryWrites" (server setting DisableWriteInsideTryFunctions=false; oracle.toml suite trywrite-allowed).
    "junit-al-language-trywrite-allowed.xml": ("tests/al-language-trywrite-allowed", "junit-trywrite-allowed.xml"),
}
ROLE_OF_APP = {app: role for role, (app, _) in ROLES.items()}
SECTION_START = "<!-- pr-results-bump:start -->"
SECTION_END = "<!-- pr-results-bump:end -->"


class Refused(Exception):
    pass


def run(cmd, **kw):
    r = subprocess.run(cmd, capture_output=True, text=True, **kw)
    if r.returncode != 0:
        raise Refused(f"{' '.join(cmd[:4])}... failed: {r.stderr.strip()[:300]}")
    return r.stdout


def git(corpus, *args):
    return run(["git", "-C", corpus, *args]).strip()


def git_ok(corpus, *args):
    return subprocess.run(["git", "-C", corpus, *args], capture_output=True).returncode == 0


def changed(corpus, a, b):
    """[(status, path)] from a to b, renames as a deletion and an addition."""
    out = git(corpus, "diff", "--no-renames", "--name-status", a, b)
    return [tuple(l.split("\t", 1)) for l in out.splitlines() if l]


def neutral(path):
    """A path whose change no oracle can see: CI files, docs, scripts, must-not-compile cases, corpus bookkeeping."""
    return ("/" not in path or path.startswith((".github/", "docs/", "scripts/", "tests/must-not-compile/"))
            or path in ("tests/rename-map.tsv", "tests/facts-changes.tsv", "tests/al.code-workspace"))


def shared(path):
    """A path that can change the outcome of codeunits a subset run does not run: a full master run is needed."""
    return ("/_fixtures/" in path or path.startswith(("tests/al-language-internals-fixture/", "tests/al-language-xapp-"))
            or path == "tests/custom.ruleset.json" or os.path.basename(path) == "app.json")


def app_of(path):
    for app in ROLE_OF_APP:
        if path.startswith(app + "/"):
            return app
    return None


def suites(root):
    return {s.get("name"): s for s in root.findall("testsuite")}


def cases(root):
    return sum(1 for c in root.iter("testcase") if c.get("name") != "OnRun")


def merge(old_root, new_roots):
    """The old JUnit with each codeunit (testsuite) a new file has replaced in place; new codeunits are appended.
    The same rule as the corpus's own shard merge (.github/scripts/ci-shards.py merge)."""
    order, by = [], {}
    for r in [old_root, *new_roots]:
        for s in r.findall("testsuite"):
            k = s.get("name")
            if k not in by:
                order.append(k)
            by[k] = s
    kept = [by[k] for k in order]

    def isum(attr):
        return str(sum(int(s.get(attr, "0")) for s in kept))

    out = ET.Element("testsuites", {
        "name": old_root.get("name", "merged"), "tests": isum("tests"), "failures": isum("failures"),
        "errors": isum("errors"), "skipped": isum("skipped"),
        "time": "%.3f" % sum(float(s.get("time") or 0) for s in kept),
        "timestamp": max([r.get("timestamp") or "" for r in [old_root, *new_roots]])})
    for s in kept:
        out.append(s)
    return out


def write_junit(root, path):
    ET.ElementTree(root).write(path, encoding="unicode", xml_declaration=True)


def codeunits_in(corpus, rev, paths):
    """{codeunit id: path} of the codeunits declared in the given .al files at rev."""
    found = {}
    for p in paths:
        if not p.endswith(".al") or not git_ok(corpus, "cat-file", "-e", f"{rev}:{p}"):
            continue
        text = run(["git", "-C", corpus, "show", f"{rev}:{p}"])
        for m in re.finditer(r"(?im)^\s*codeunit\s+(\d+)\b", text):
            found[m.group(1)] = p
    return found


def files_of_codeunits(corpus, rev, ids):
    """The .al files under tests/ at rev that declare one of the codeunit ids."""
    if not ids:
        return set()
    r = subprocess.run(["git", "-C", corpus, "grep", "-I", "-i", "-E", r"^[[:space:]]*codeunit[[:space:]]+[0-9]+",
                        rev, "--", "tests/"], capture_output=True, text=True)
    files = set()
    for line in r.stdout.splitlines():
        _, path, text = line.split(":", 2)
        m = re.match(r"(?i)\s*codeunit\s+(\d+)", text)
        if m and m.group(1) in ids:
            files.add(path)
    return files


class Gh:
    def __init__(self, gh):
        self.gh = gh

    def json(self, *args):
        return json.loads(run([self.gh, *args]) or "null")

    def lines(self, *args):
        return [json.loads(l) for l in run([self.gh, *args]).splitlines() if l.strip()]

    def raw(self, *args):
        r = subprocess.run([self.gh, *args], capture_output=True)
        if r.returncode != 0:
            raise Refused(f"gh {' '.join(args)} failed: {r.stderr.decode(errors='replace')[:300]}")
        return r.stdout


def read_artifact(gh, aid):
    """(xml file name, parsed root) of an artifact holding one JUnit file."""
    z = zipfile.ZipFile(io.BytesIO(gh.raw("api", f"repos/{REPO}/actions/artifacts/{aid}/zip")))
    xml = [n for n in z.namelist() if n.endswith(".xml")]
    if len(xml) != 1:
        raise Refused(f"artifact {aid} does not hold exactly one XML file")
    return os.path.basename(xml[0]), ET.fromstring(z.read(xml[0]))


def pr_run_results(gh, pr, head):
    """The last CI run on the pull request's head and its JUnit per (major, role): {major: {role: (artifact id, root)}}."""
    runs = gh.json("run", "list", "-R", REPO, "--workflow", WORKFLOW, "--commit", head, "--event", "pull_request",
                   "-L", "20", "--json", "databaseId,status,conclusion,createdAt,headSha")
    runs = [r for r in runs or [] if r.get("headSha") == head]
    if not runs:
        raise Refused(f"corpus PR {pr}: no CI run on its head {head[:12]}")
    last = max(runs, key=lambda r: r.get("createdAt", ""))
    rid = last["databaseId"]
    if last.get("status") != "completed" or last.get("conclusion") != "success":
        raise Refused(f"corpus PR {pr}: its last CI run {rid} is not green "
                      f"(status {last.get('status')}, conclusion {last.get('conclusion')})")
    arts = gh.lines("api", "--paginate", f"repos/{REPO}/actions/runs/{rid}/artifacts?per_page=100",
                    "--jq", ".artifacts[]")
    out = {}
    for leg, major in LEGS:
        mine = sorted((a for a in arts if a.get("name") == f"junit-test-results-{leg}"),
                      key=lambda a: a.get("created_at", ""), reverse=True)
        out[major] = {}
        for a in mine:
            role, root = read_artifact(gh, a["id"])
            if role not in ROLES:
                raise Refused(f"run {rid}: artifact {a['id']} of junit-test-results-{leg} holds no known JUnit file ({role})")
            if role in out[major]:
                continue  # a re-run shard or job: the newest artifact of a role wins
            bad = [c for c in root.iter("testcase") if c.find("failure") is not None or c.find("error") is not None]
            if bad:
                raise Refused(f"run {rid}: {role} of BC {leg} has {len(bad)} failed case(s)")
            out[major][role] = (a["id"], root)
    roles = {m: sorted(r) for m, r in out.items()}
    if len({tuple(r) for r in roles.values()}) != 1:
        raise Refused(f"corpus PR {pr}, run {rid}: the BC versions do not have the same JUnit files ({roles}); "
                      "a BC version is missing")
    return rid, out


def plan(root, corpus, gh, prs, old, fetch):
    ref = "master"
    if fetch:
        git(corpus, "fetch", "-q", "origin", "master")
        ref = "origin/master"
    if not git_ok(corpus, "cat-file", "-e", f"{old}^{{commit}}"):
        raise Refused(f"the current pin {old[:12]} is not in the corpus repository")
    first_parent = git(corpus, "rev-list", "--first-parent", ref).splitlines()
    infos = []
    for n in prs:
        p = gh.json("pr", "view", str(n), "-R", REPO, "--json", "number,state,mergeCommit,headRefOid,baseRefName,title")
        if p.get("state") != "MERGED" or not p.get("mergeCommit"):
            raise Refused(f"corpus PR {n} is not merged (state {p.get('state')})")
        if p.get("baseRefName") != "master":
            raise Refused(f"corpus PR {n} was merged into {p.get('baseRefName')}, not master")
        m = p["mergeCommit"]["oid"]
        if m not in first_parent:
            raise Refused(f"the merge commit {m[:12]} of corpus PR {n} is not a first-parent commit of master")
        if m == old or not git_ok(corpus, "merge-base", "--is-ancestor", old, m):
            raise Refused(f"the merge commit {m[:12]} of corpus PR {n} is not after the current pin {old[:12]}")
        infos.append({"pr": n, "title": p.get("title", ""), "merge": m, "head": p["headRefOid"]})
    # Merge order: master's first-parent order, oldest first; the pin moves to the newest merge.
    infos.sort(key=lambda i: -first_parent.index(i["merge"]))
    target = infos[-1]["merge"]
    merges = {i["merge"] for i in infos}
    for c in git(corpus, "rev-list", "--first-parent", f"{old}..{target}").splitlines():
        if c in merges:
            continue
        seen = [p for _, p in changed(corpus, f"{c}^1", c) if not neutral(p)]
        if seen:
            subj = git(corpus, "log", "-1", "--format=%s", c)
            raise Refused(f"master commit {c[:12]} ({subj}) between the pin and {target[:12]} is not one of the listed "
                          f"pull requests and changes {seen[0]}: add its pull request to --from-pr, or bump from a master run")
    for i in infos:
        m, head = i["merge"], i["head"]
        parent = git(corpus, "rev-parse", f"{m}^1")
        files = changed(corpus, parent, m)
        i["paths"] = [p for _, p in files]
        for st, p in files:
            if neutral(p):
                continue
            if shared(p):
                raise Refused(f"corpus PR {i['pr']} changed the shared fixture {p}: a full master run is needed")
            if app_of(p) is None:
                raise Refused(f"corpus PR {i['pr']} changed {p}, which no Linux CI leg reports: bump from a master run")
            if st.startswith("D") and p.endswith(".al"):
                raise Refused(f"corpus PR {i['pr']} deleted or moved {p}: a full master run is needed")
        i["run"], i["results"] = pr_run_results(gh, i["pr"], head)
        apps_run = {ROLES[r][0] for r in i["results"]["27"]}
        for p in i["paths"]:
            a = app_of(p)
            if a and p.endswith(".al") and a not in apps_run:
                raise Refused(f"corpus PR {i['pr']} changed {p} but its run {i['run']} reports no codeunit of {a}")
        i["selected"] = {mj: {r: set(suites(rt)) for r, (_, rt) in rs.items()} for mj, rs in i["results"].items()}
        # The run tested the pull request's head merged with master as it was then. When the merge commit's files are
        # not the head's, master moved on: none of what it changed since the merge base may touch the pull request's
        # files or the files of the codeunits its run ran.
        if git(corpus, "rev-parse", f"{m}^{{tree}}") != git(corpus, "rev-parse", f"{head}^{{tree}}"):
            base = git(corpus, "merge-base", head, parent)
            moved = {p for _, p in changed(corpus, base, parent)}
            ids = {s.split()[-1] for rs in i["selected"].values() for ss in rs.values() for s in ss}
            touched = moved & (set(i["paths"]) | files_of_codeunits(corpus, m, ids))
            touched = {p for p in touched if not neutral(p)}
            if touched:
                raise Refused(f"corpus PR {i['pr']}: master changed {sorted(touched)[0]} after the pull request's merge "
                              f"base {base[:12]}, so its run may not have tested the merged files: bump from a master run")
    # Every codeunit a changed file declares, with cases in the current oracle, must have been run.
    oracle = {}
    for _, mj in LEGS:
        for role, (_, f) in ROLES.items():
            path = os.path.join(root, "oracle", mj, f)
            if os.path.exists(path):
                oracle[(mj, role)] = ET.parse(path).getroot()
    for i in infos:
        for cid, p in codeunits_in(corpus, i["merge"], i["paths"]).items():
            role = ROLE_OF_APP.get(app_of(p))
            for _, mj in LEGS:
                o = oracle.get((mj, role))
                name = f"Codeunit {cid}"
                if o is not None and name in suites(o) and name not in i["selected"][mj].get(role, set()):
                    raise Refused(f"codeunit {cid} ({p}) has cases in the BC {mj} oracle but the run {i['run']} of "
                                  f"corpus PR {i['pr']} did not run it")
        for mj, rs in i["selected"].items():
            for role in rs:
                if (mj, role) not in oracle:
                    raise Refused(f"run {i['run']} reports {role} for BC {mj}, which has no oracle file yet: "
                                  "bump from a master run")
    for _, mj in LEGS:
        for role in ROLES:
            seen = {}
            for i in infos:
                for s in i["selected"][mj].get(role, ()):
                    if s in seen:
                        raise Refused(f"{s} ran in the runs of corpus PRs {seen[s]} and {i['pr']} (BC {mj}): "
                                      "bump from a master run")
                    seen[s] = i["pr"]
    merged, report = {}, []
    for (mj, role), o in oracle.items():
        news = [i["results"][mj][role][1] for i in infos if role in i["results"][mj]]
        if not news:
            continue
        m = merge(o, news)
        before, after = cases(o), cases(m)
        if after < before:
            raise Refused(f"oracle/{mj}/{ROLES[role][1]} would go from {before} to {after} cases: "
                          "an oracle never shrinks in a bump; bump from a master run")
        merged[(mj, role)] = m
        report.append((mj, ROLES[role][1], before, after))
    return target, infos, merged, sorted(report)


def edit_files(root, old, target, infos, merged):
    for (mj, role), m in merged.items():
        write_junit(m, os.path.join(root, "oracle", mj, ROLES[role][1]))
    p = os.path.join(root, "oracle", "oracle.toml")
    t = open(p).read()
    t = re.sub(r'^corpus_commit = "[0-9a-f]+"', f'corpus_commit = "{target}"', t, count=1, flags=re.M)
    open(p, "w").write(t)
    p = os.path.join(root, "oracle", "README.md")
    t = open(p).read()
    t = re.sub(r'(\| Corpus pin \| `)[0-9a-f]{40}(`)', rf"\g<1>{target}\2", t)
    for (mj, role), m in merged.items():
        if role != "junit-al-language.xml":
            continue
        t = re.sub(rf"^(\| {mj} \| [0-9.]+ \| `[0-9a-f]+` )\(the pin\)", r"\1(full run; PR results on top, see below)",
                   t, flags=re.M)
        t = re.sub(rf"^(\| {mj} \| [0-9.]+ \| `[0-9a-f]+` \([^)]*\) \| \d+ \| `[^`]+` \| \d+ \| )\d+",
                   rf"\g<1>{cases(m)}", t, flags=re.M)
    lines = [f"- pin `{target[:7]}` (was `{old[:7]}`): " + "; ".join(
        f"corpus PR {i['pr']} (head `{i['head'][:7]}`, merge `{i['merge'][:7]}`, run {i['run']}: "
        + ", ".join(f"BC {mj} {len(ss)} codeunit(s) of {ROLES[r][1]}"
                    for mj, rs in sorted(i["selected"].items()) for r, ss in sorted(rs.items())) + ")"
        for i in infos)]
    if SECTION_START not in t:
        t = t.rstrip("\n") + "\n\n" + "\n".join([
            SECTION_START,
            "## PR-results bumps since the last full run",
            "",
            "These pins came from `tools/corpus_bump.sh --from-pr` (owner decision 2026-11-05), not from a master run. Each",
            "replaced, in the JUnit files above, the codeunits that a corpus pull request's own CI run ran with that run's",
            "results. The table above still names the last full run. The next bump from a green master run removes this",
            "section.",
            "",
            SECTION_END]) + "\n"
    t = t.replace(SECTION_END, "\n".join(lines) + "\n" + SECTION_END)
    open(p, "w").write(t)


def body(old, target, infos, report):
    out = [f"PR-results corpus bump to master `{target[:12]}` (was `{old[:12]}`).", "",
           "This bump did not wait for the corpus master run (owner decision 2026-11-05). The oracle is the oracle of the",
           "previous pin, with the cases of the codeunits each corpus pull request's own CI run ran replaced by that run's",
           "results. A difference only a master run shows (a flaky test, an interaction between tests) shows up after",
           "this bump; the next bump from a green master run corrects it.", "", "Corpus runs the oracle came from:", ""]
    for i in infos:
        out.append(f"- corpus PR {i['pr']} ({i['title']}): head `{i['head'][:12]}`, merge `{i['merge'][:12]}`, run "
                   f"https://github.com/{REPO}/actions/runs/{i['run']}")
        for mj, rs in sorted(i["results"].items()):
            for role, (aid, rt) in sorted(rs.items()):
                out.append(f"  - BC {mj} {ROLES[role][1]}: artifact {aid}, {len(suites(rt))} codeunit(s), {cases(rt)} case(s)")
    out += ["", "Oracle cases, before -> after:", ""]
    out += [f"- oracle/{mj}/{f}: {b} -> {a}" for mj, f, b, a in report]
    out += ["", "All other cases are the previous oracle's, unchanged."]
    return "\n".join(out) + "\n"


def main(argv):
    if argv[:1] == ["merge"]:
        old = ET.parse(argv[1]).getroot()
        write_junit(merge(old, [ET.parse(p).getroot() for p in argv[2:]]), argv[1])
        return 0
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", required=True)
    ap.add_argument("--prs", required=True)
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--body")
    a = ap.parse_args(argv)
    if not re.fullmatch(r"\d+(,\d+)*", a.prs):
        print("usage: --from-pr N[,N...]", file=sys.stderr)
        return 2
    prs = sorted({int(x) for x in a.prs.split(",")})
    corpus = os.environ.get("CORPUS_BUMP_CORPUS") or os.path.join(a.root, "corpus")
    gh = Gh(os.environ.get("CORPUS_BUMP_GH", "gh"))
    old = os.environ.get("CORPUS_BUMP_PIN") or git(a.root, "ls-tree", "HEAD", "corpus").split()[2]
    try:
        target, infos, merged, report = plan(a.root, corpus, gh, prs, old, os.environ.get("CORPUS_BUMP_NO_FETCH") != "1")
    except Refused as e:
        print(f"corpus_bump: refused: {e}", file=sys.stderr)
        return 1
    for i in infos:
        print(f"corpus PR {i['pr']}: merge {i['merge'][:12]}, last run {i['run']} green")
    for mj, f, b, af in report:
        print(f"  oracle/{mj}/{f}: {b} -> {af} cases")
    text = body(old, target, infos, report)
    if a.dry_run:
        print(f"would move corpus to {target} and write the oracle files above")
        print("dry run: nothing changed")
        return 0
    edit_files(a.root, old, target, infos, merged)
    git(corpus, "checkout", "-q", target)
    if a.body:
        open(a.body, "w").write(text)
        print(f"pull request body: {a.body}")
    else:
        print(text)
    print(f"pin={target}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
