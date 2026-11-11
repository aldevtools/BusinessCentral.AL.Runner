#!/usr/bin/env bash
# Bump the corpus to the commit of a green corpus CI run, in one command (issue 44). The steps are the
# ones oracle/README.md describes ("To bump"):
#   1. check the run: completed and green, and its head commit on the corpus branch master (the default
#      branch; it moves only through a pull request from al-runtime-rs-staging, merged with a merge commit);
#      when a merge commit brought the commit into master, that merge must hold exactly the commit's files;
#   2. find the artifacts: per leg (27.5, 28.5, 29.0) two to six named junit-test-results-<leg>: the oracle
#      (junit-al-language.xml), the OnPrem file, the OnPrem DotNet file, the TestLibraries file, the
#      Isolation Disabled file and the TryWrites file;
#   3. move the corpus submodule to the run's commit;
#   4. download each artifact by id and copy its XML unchanged to oracle/<major>/junit.xml,
#      oracle/<major>/junit-onprem.xml, junit-dotnet.xml, junit-testlibraries.xml, junit-isolation-disabled.xml and
#      junit-trywrite-allowed.xml;
#   5. write the new pin, run and artifact ids into oracle/oracle.toml and oracle/README.md;
#   6. build release and update the not-passing lists (ADR-065): lines for the new oracle cases that do not pass
#      (--list-new-cases HEAD), stale lines deleted (--prune-not-passing); al-test-cli exits 1 on a regression; then
#      print the assumptions whose awaited corpus pin is now in the corpus (al-test-cli --awaiting-assumptions);
#   7. write the new case counts into crates/al-test-cli/tests/corpus_harness.rs.
# It never edits a JUnit file and never edits docs/probes-needed.md: it prints the corpus files added
# since the old pin, and the number of table rows of probes-needed before and after, so the person
# running it can strike the rows the new probes settle.
#
#   tools/corpus_bump.sh [--dry-run] RUN_ID
#   tools/corpus_bump.sh [--dry-run] [--body FILE] --from-pr N[,N...]
#
# --dry-run checks the run and prints what it would do, changing nothing.
# --from-pr (owner decision 2026-11-05) does not wait for the corpus master run: the pin moves to the merge commit of
# the given corpus pull requests, and the oracle is the current oracle with the codeunits each pull request's own CI
# run ran replaced by that run's results (tools/corpus_bump_pr.py lists when it refuses). Steps 6 and 7 are the same.
# --body FILE writes the bump pull request body (which corpus runs the oracle came from) to FILE.
# For tests: CORPUS_BUMP_GH (the gh to call), CORPUS_BUMP_CORPUS (the corpus repository),
# CORPUS_BUMP_NO_FETCH=1 (do not fetch the corpus branch), CORPUS_BUMP_LISTS=0 (do not update the not-passing lists).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
gh="${CORPUS_BUMP_GH:-gh}"
corpus="${CORPUS_BUMP_CORPUS:-$root/corpus}"
repo="aldevtools/BusinessCentral.AL.Language.Tests"
branch="master"
legs="27.5 28.5 29.0"

usage() { echo "usage: tools/corpus_bump.sh [--dry-run] RUN_ID | [--dry-run] [--body FILE] --from-pr N[,N...]" >&2; exit 2; }
die() { echo "corpus_bump: refused: $*" >&2; exit 1; }
# A setup error of al-test-cli (its output in $1) that is a suite oracle recorded for an older corpus commit whose app
# the pin changed (src/pin.rs): the isolation probe's oracle comes from the corpus Windows nightly, which this script
# does not read, so point to the tool that records it. Prints nothing for any other error.
explain_setup_error() {
  printf '%s\n' "$1" | grep -q "the oracle is recorded for corpus commit" || return 0
  echo "a suite oracle is recorded for an older corpus commit whose app the pin changed (above)." >&2
  if printf '%s\n' "$1" | grep -q "tests/al-language-isolation-probe/"; then
    echo "the isolation suites take their oracle from the corpus Windows nightly: record it with" >&2
    echo "  tools/record_windows_oracle.sh [--dry-run] RUN_ID   (a green 'Nightly Windows' run whose commit is an ancestor of the pin)" >&2
    echo "then run this bump again." >&2
  fi
}

dry=0
run=""
prs=""
body=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry=1 ;;
    --from-pr) [ $# -ge 2 ] || usage; prs="$2"; shift ;;
    --from-pr=*) prs="${1#--from-pr=}" ;;
    --body) [ $# -ge 2 ] || usage; body="$2"; shift ;;
    -*) usage ;;
    *) [ -z "$run" ] || usage; run="$1" ;;
  esac
  shift
done
if [ -n "$prs" ]; then
  [ -z "$run" ] || usage
  [[ "$prs" =~ ^[0-9]+(,[0-9]+)*$ ]] || usage
else
  [ -z "$body" ] || usage
  [ -n "$run" ] || usage
  [[ "$run" =~ ^[0-9]+$ ]] || usage
fi

probe_rows() { grep -c '^| ' "$root/docs/probes-needed.md" || true; }
# The runtime's harness totals leave out the cases real BC skipped (BC 27 lists O_Database_CopyCompany as <skipped/>):
# 14975 JUnit cases were 14974 runtime cases in bump 27. The oracle README keeps the JUnit count (cases()).
run_cases() { python3 - "$1" <<'PY'
import sys, xml.etree.ElementTree as ET
n = sum(1 for c in ET.parse(sys.argv[1]).iter("testcase")
        if c.get("name") != "OnRun" and c.find("skipped") is None)
print(n)
PY
}
# Steps 6 and 7, the same for both modes. Argument: where the oracle came from (for the last line).
finish() {
# 6. The not-passing lists (ADR-065). The base is HEAD, the commit before the bump: a line is added only for a case its
# oracle did not have, and a case it had that does not pass is a regression (exit 1).
cd "$root"
cargo build --release -q
# Problems that need a person are collected and printed at the end as a to-do list; the script exits 1 after the
# last step then, never in the middle (a bump that stops at the first one leaves the rest undone).
todo=()
# One pass over the three BC versions. Arguments: extra al-test-cli flags. Prints the lines of the lists; when the
# check exits 1 (a regression or a case without an outcome) it prints the whole output first, so the cases are named.
# An exit above 1 is a setup error and stops the bump. Sets list_rc to 1 when a version still has a problem.
list_pass() {
  local major out rc
  list_rc=0
  for major in 27 28 29; do
    rc=0
    out="$(./target/release/al-test-cli --root . --baseline-diff "$@" --bc-version "$major" 2>&1)" || rc=$?
    printf '%s\n' "$out" | grep -E "^BC [0-9.]+[: ]" || true
    if [ "$rc" -gt 1 ]; then printf '%s\n' "$out" >&2; explain_setup_error "$out"; die "not-passing lists for BC $major (exit $rc)"; fi
    if [ "$rc" = 1 ]; then
      printf '%s\n' "$out" | grep -vE "^BC [0-9.]+[: ]" || true
      list_rc=1
    fi
  done
}
# CORPUS_BUMP_LISTS=0 leaves the lists, the golden files and the gate check alone: the bump bot
# (tools/corpus_bump_bot.sh) does them in its own run.
if [ "${CORPUS_BUMP_LISTS:-1}" != 0 ]; then
  # Microsoft's apps come only from the builds oracle/oracle.toml pins, read from the artifact cache (ADR-013 amendment
  # of 2026-11-10: the corpus tree holds none and its .alpackages is not searched). Fill the cache before the lists and
  # the golden files: a list pass on a cache without them would record every case that needs a Microsoft app.
  echo "fetching Microsoft's apps of the pinned builds into the artifact cache:"
  fetch_log="$(mktemp)"
  ./target/release/al-test-cli --root . --all-versions --fetch-deps 2>&1 | tee "$fetch_log" \
    || { explain_setup_error "$(cat "$fetch_log")"; rm -f "$fetch_log"; die "./target/release/al-test-cli --root . --all-versions --fetch-deps failed (output above: the artifact feed, or the oracle and the pin disagree). Nothing was listed and no golden file was written. Run again when it passes"; }
  rm -f "$fetch_log"

  # Lines for the new cases that do not pass, stale lines deleted. A pass that exits 1 is not final: the golden files
  # below change what the runtime compiles, so the lists are pruned again after them.
  list_pass --list-new-cases HEAD --prune-not-passing

  # 6a. The golden files that depend on the corpus (pinned_columns.rs, expected-unresolved.txt): regenerated here, so
  # the pull request does not fail on stale ones. They belong in the bump commit.
  # The tests read Microsoft's apps of the pinned BC 27 build from the cache and fail before they write when the
  # System app or Base Application is not loaded: a golden file written without them would list every Microsoft
  # reference as unresolved and then agree with itself.
  golden_build="$(python3 - "$root/oracle/oracle.toml" <<'PY'
import sys, tomllib
with open(sys.argv[1], "rb") as f:
    versions = tomllib.load(f)["version"]
print(min(versions, key=lambda v: v["major"]).get("bc_build", "no pinned build"))
PY
)"
  echo "regenerating the golden files (corpus_resolve_deps) against Microsoft's apps of BC build $golden_build:"
  UPDATE_GOLDEN=1 cargo test --release -q -p al-test-cli --test corpus_resolve_deps -- --ignored \
    || die "regenerating the golden files failed (output above)"
  echo "golden files generated against BC build $golden_build"
  golden="crates/al-builtins/src/pinned_columns.rs crates/al-sema/tests/expected-unresolved.txt"
  # shellcheck disable=SC2086
  git -C "$root" diff --stat -- $golden
  echo "add to the bump commit: $golden"
  cargo build --release -q
  list_pass --prune-not-passing
  if [ "$list_rc" = 1 ]; then
    todo+=("the not-passing lists: fix each regression and each case without an outcome listed above (the bump added no line for them)")
  fi

  # 6c. The version gates: a new `#if BC28PLUS` file needs a gate that cites it. Never extended here.
  gates_out="$(cargo test -q -p al-version 2>&1)" || {
    printf '%s\n' "$gates_out"
    echo "files with a guard above the floor and no gate:"
    printf '%s\n' "$gates_out" | sed -n '/corpus guards with no gate/,/^$/p' | grep -E "\(BC[0-9]+PLUS\)" | sed 's/^/  /' || true
    todo+=("version gates: extend a Gate in crates/al-version/src/gates.rs (its evidence list) for each file above, then cargo test -p al-version")
  }
fi

# 6b. The assumptions that waited for a corpus pin now in the corpus (Info::awaiting in
# crates/al-builtins/src/assumptions.rs): each must go (the pin agrees) or its code be fixed (the pin disagrees).
# The bump goes on; ./check.sh fails until they are dealt with.
echo "assumptions waiting for a corpus pin:"
./target/release/al-test-cli --root . --awaiting-assumptions 2>&1 | grep -E "^(assumption |warning: assumption |awaiting assumptions: )" || echo "  the check failed (lines above): remove each assumption whose pin landed, or fix its code, before ./check.sh"

# 7. The harness counts: totals per major and the BC 29 area totals the harness asserts.
dir="$(mktemp -d)"
./target/release/al-test-cli --root . --all-versions --no-deps --json "$dir/pass-rate.json" >/dev/null 2>&1 || true
hargs=()
for leg in $legs; do
  major="${leg%%.*}"
  hargs+=("$major" 0 0 "$(run_cases "$root/oracle/$major/junit.xml")" "$(run_cases "$root/oracle/$major/junit-onprem.xml")")
done
python3 -I "$here/corpus_bump_harness.py" "$root" "$dir" "$old" "$sha" "${hargs[@]}"
rm -rf "$dir"
echo "docs/probes-needed.md table rows after: $(probe_rows) (strike the rows the new probes settle by hand)"
echo "bumped to $sha from $1; review git diff and open the pull request (its CI is the gate)"
if [ "${#todo[@]}" -gt 0 ]; then
  echo "TO DO before the pull request (the bump is not finished):" >&2
  printf '  - %s\n' "${todo[@]}" >&2
  exit 1
fi
}

# The PR-results bump (--from-pr): tools/corpus_bump_pr.py checks, merges the oracle, moves the pin; then steps 6 and 7.
if [ -n "$prs" ]; then
  echo "docs/probes-needed.md table rows before: $(probe_rows)"
  pargs=(--root "$root" --prs "$prs")
  [ "$dry" = 0 ] || pargs+=(--dry-run)
  [ -z "$body" ] || pargs+=(--body "$body")
  python3 -I "$here/corpus_bump_pr.py" "${pargs[@]}" || exit $?
  [ "$dry" = 0 ] || exit 0
  old="$(git -C "$root" ls-tree HEAD corpus | awk '{print $3}')"
  sha="$(git -C "$corpus" rev-parse HEAD)"
  finish "corpus PR(s) $prs (a PR-results bump)"
  exit 0
fi

# 1. The run.
info="$("$gh" run view "$run" -R "$repo" --json status,conclusion,headSha)"
read -r status conclusion sha < <(python3 -c '
import json, sys
r = json.loads(sys.argv[1])
print(r.get("status", ""), r.get("conclusion", ""), r.get("headSha", ""))' "$info")
[ "$status" = completed ] || die "run $run is not completed (status: ${status:-none})"
[ "$conclusion" = success ] || die "run $run is not green (conclusion: ${conclusion:-none})"
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || die "run $run has no head commit"
if [ "${CORPUS_BUMP_NO_FETCH:-0}" != 1 ]; then
  git -C "$corpus" fetch -q origin "$branch"
  ref="origin/$branch"
else
  ref="$branch"
fi
git -C "$corpus" cat-file -e "$sha^{commit}" 2>/dev/null || die "commit $sha of run $run is not in the corpus repository"
git -C "$corpus" merge-base --is-ancestor "$sha" "$ref" || die "commit $sha of run $run is not on the corpus branch $branch"
# A bump lands through a pull request: the run tested the commit merged with master. When master had commits
# the commit did not have, the run's JUnit covers files the pinned commit lacks. The merge commit that brought
# the commit into master (the oldest first-parent commit of master that contains it) must have its files.
if ! git -C "$corpus" rev-list --first-parent "$ref" | grep -x "$sha" >/dev/null; then
  merge="$(git -C "$corpus" rev-list --first-parent --ancestry-path "$sha..$ref" | tail -n 1)"
  [ "$(git -C "$corpus" rev-parse "$merge^{tree}")" = "$(git -C "$corpus" rev-parse "$sha^{tree}")" ] \
    || die "commit $sha of run $run came into $branch with merge ${merge:0:7}, whose files differ from it ($branch had commits the run's commit lacks): merge origin/$branch into the bump branch, run CI on the pull request again and bump from that run"
fi
echo "run $run: green, commit $sha on $branch"

# 2. The artifacts: per leg the larger is the oracle, the smaller the OnPrem file.
arts="$("$gh" api "repos/$repo/actions/runs/$run/artifacts?per_page=100")"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
# The XML file name an artifact holds (junit-al-language.xml, -onprem.xml, -onprem-dotnet.xml).
xml_of() {
  local d="$tmp/role-$1"
  mkdir -p "$d"
  "$gh" api "repos/$repo/actions/artifacts/$1/zip" > "$d/a.zip" || return 1
  unzip -Z1 "$d/a.zip" | grep '\.xml$' | head -n 1
}
declare -A oracle_id onprem_id dotnet_id testlib_id isol_id tryw_id
for leg in $legs; do
  # Two artifacts: the oracle (larger) and the OnPrem file. Three (since the OnPrem DotNet app runs in CI): the DotNet file is the smallest.
  # Four (since the TestLibraries job): the roles are read from the XML file names below.
  tl=0 isol=0 tw=0
  read -r n big mid small < <(python3 -c '
import json, sys
a = [x for x in json.loads(sys.argv[1]).get("artifacts", []) if x.get("name") == "junit-test-results-" + sys.argv[2]]
a.sort(key=lambda x: x.get("size_in_bytes", 0))
n = len(a)
if n == 2:
    print(n, a[1]["id"], a[0]["id"], 0)
elif n == 3:
    print(n, a[2]["id"], a[1]["id"], a[0]["id"])
else:
    print(n, 0, 0, 0)' "$arts" "$leg")
  if [ "$n" -gt 3 ]; then
    # A job rerun (corpus run 39303760055, attempt 2) keeps the first attempt's artifact under the same name.
    # The role is the XML file each artifact holds; per role the newest artifact is the rerun's.
    big=0 mid=0 small=0 tl=0 isol=0 tw=0
    while read -r id; do
      # A command substitution cannot stop the script: a failed download is caught here (gh printed its error).
      xml="$(xml_of "$id")" || die "downloading artifact $id of junit-test-results-$leg failed (the gh error is above)"
      case "$xml" in
        junit-al-language.xml) [ "$big" != 0 ] || big=$id ;;
        junit-al-language-onprem.xml) [ "$mid" != 0 ] || mid=$id ;;
        junit-al-language-onprem-dotnet.xml) [ "$small" != 0 ] || small=$id ;;
        # The TestLibraries job runs with Microsoft's Tests-TestLibraries and Data Archive installed: the oracle of the
        # suite testlibraries, which runs in an environment of its own (oracle.toml `extra_apps`).
        junit-al-language-testlibraries.xml) [ "$tl" != 0 ] || tl=$id ;;
        # The Isolation Disabled job (corpus tests/al-language-isolation-disabled, test runner 130451): the oracle of the
        # suite isolation-disabled. It runs in the main environment under its own runner, so it needs no pass of its own.
        junit-al-language-isolation-disabled.xml) [ "$isol" != 0 ] || isol=$id ;;
        # The TryWrites job (corpus tests/al-language-trywrite-allowed) runs BC with the server setting
        # DisableWriteInsideTryFunctions=false: the oracle of the suite trywrite-allowed, which runs in a pass of its own
        # with that setting (oracle.toml `server_settings`).
        junit-al-language-trywrite-allowed.xml) [ "$tw" != 0 ] || tw=$id ;;
        *) die "artifact $id of junit-test-results-$leg holds no known JUnit file" ;;
      esac
    done < <(python3 -c '
import json, sys
a = [x for x in json.loads(sys.argv[1]).get("artifacts", []) if x.get("name") == "junit-test-results-" + sys.argv[2]]
a.sort(key=lambda x: x.get("created_at", ""), reverse=True)
print("\n".join(str(x["id"]) for x in a))' "$arts" "$leg")
    { [ "$big" != 0 ] && [ "$mid" != 0 ]; } || die "run $run: junit-test-results-$leg lacks the oracle or the OnPrem file"
  fi
  { [ "$n" -ge 2 ]; } || die "run $run has $n artifact(s) named junit-test-results-$leg, expected 2 to 6 (the oracle, the OnPrem file, the OnPrem DotNet file, the TestLibraries file, the Isolation Disabled file and the TryWrites file)"
  oracle_id[$leg]=$big
  onprem_id[$leg]=$mid
  dotnet_id[$leg]=$small
  testlib_id[$leg]=$tl
  isol_id[$leg]=$isol
  tryw_id[$leg]=$tw
  line="$leg: oracle artifact $big, onprem artifact $mid"
  [ "$small" = 0 ] || line="$line, dotnet artifact $small"
  [ "$tl" = 0 ] || line="$line, testlibraries artifact $tl"
  [ "$isol" = 0 ] || line="$line, isolation-disabled artifact $isol"
  [ "$tw" = 0 ] || line="$line, trywrite-allowed artifact $tw"
  echo "$line"
done

old="$(git -C "$root" ls-tree HEAD corpus 2>/dev/null | awk '{print $3}')"
old="${old:-$(git -C "$corpus" rev-parse HEAD)}"
if git -C "$corpus" cat-file -e "$old^{commit}" 2>/dev/null; then
  echo "corpus files added since the old pin ${old:0:7}:"
  git -C "$corpus" diff --name-only --diff-filter=A "$old" "$sha" -- tests | sed 's/^/  /'
else
  echo "the old pin ${old:0:7} is not in the corpus repository: no list of added files"
fi
echo "docs/probes-needed.md table rows before: $(probe_rows)"

if [ "$dry" = 1 ]; then
  echo "would move corpus to $sha, copy the artifacts above into oracle/27, 28, 29,"
  echo "update oracle/oracle.toml and oracle/README.md, the not-passing lists and the harness counts"
  echo "dry run: nothing changed"
  exit 0
fi

cases() { python3 - "$1" <<'PY'
import sys, xml.etree.ElementTree as ET
n = sum(1 for c in ET.parse(sys.argv[1]).iter("testcase") if c.get("name") != "OnRun")
print(n)
PY
}
declare -A before_cases before_onprem
for leg in $legs; do
  major="${leg%%.*}"
  before_cases[$leg]="$(cases "$root/oracle/$major/junit.xml")"
  before_onprem[$leg]="$(cases "$root/oracle/$major/junit-onprem.xml")"
done

# 3. The submodule.
git -C "$corpus" checkout -q "$sha"

# 4. The oracle files, copied unchanged.
fetch_xml() { # artifact id, destination
  local id="$1" dest="$2" d="$tmp/$1"
  mkdir -p "$d"
  "$gh" api "repos/$repo/actions/artifacts/$id/zip" > "$d/a.zip" || die "downloading artifact $id failed (the gh error is above)"
  (cd "$d" && unzip -q a.zip)
  local xml
  xml="$(find "$d" -name '*.xml' | head -n 2)"
  [ "$(printf '%s\n' "$xml" | grep -c .)" = 1 ] || die "artifact $id does not hold exactly one XML file"
  cp "$xml" "$dest"
}
for leg in $legs; do
  major="${leg%%.*}"
  fetch_xml "${oracle_id[$leg]}" "$root/oracle/$major/junit.xml"
  fetch_xml "${onprem_id[$leg]}" "$root/oracle/$major/junit-onprem.xml"
  # The DotNet file is the oracle of the onprem-dotnet suite of oracle.toml.
  if [ "${dotnet_id[$leg]}" != 0 ]; then fetch_xml "${dotnet_id[$leg]}" "$root/oracle/$major/junit-dotnet.xml"; fi
  # The TestLibraries file is the oracle of the testlibraries suite.
  if [ "${testlib_id[$leg]}" != 0 ]; then fetch_xml "${testlib_id[$leg]}" "$root/oracle/$major/junit-testlibraries.xml"; fi
  # The Isolation Disabled file is the oracle of the isolation-disabled suite (corpus_bump_edit.py adds the suite).
  if [ "${isol_id[$leg]}" != 0 ]; then fetch_xml "${isol_id[$leg]}" "$root/oracle/$major/junit-isolation-disabled.xml"; fi
  # The TryWrites file is the oracle of the trywrite-allowed suite (corpus_bump_edit.py adds the suite).
  if [ "${tryw_id[$leg]}" != 0 ]; then fetch_xml "${tryw_id[$leg]}" "$root/oracle/$major/junit-trywrite-allowed.xml"; fi
done

# 5. oracle.toml and README.md.
args=()
added_files="$(git -C "$corpus" diff --name-only --diff-filter=A "$old" "$sha" -- tests 2>/dev/null | grep -c . || true)"
echo "oracle cases per leg, before -> after (OnPrem in brackets):"
for leg in $legs; do
  major="${leg%%.*}"
  now="$(cases "$root/oracle/$major/junit.xml")"
  now_onprem="$(cases "$root/oracle/$major/junit-onprem.xml")"
  echo "  $leg: ${before_cases[$leg]} -> $now ($(( now - ${before_cases[$leg]} )) more) [${before_onprem[$leg]} -> $now_onprem]"
  if [ "${added_files:-0}" -gt 0 ] && [ "$now" = "${before_cases[$leg]}" ] && [ "$now_onprem" = "${before_onprem[$leg]}" ]; then
    echo "  WARNING: $added_files corpus file(s) were added but the oracle of $leg has no new case: did the CI run the new tests?"
  fi
done
for leg in $legs; do
  major="${leg%%.*}"
  args+=("$major" "${oracle_id[$leg]}" "${onprem_id[$leg]}" "$(cases "$root/oracle/$major/junit.xml")" "$(cases "$root/oracle/$major/junit-onprem.xml")")
done
DOTNET_IDS="27=${dotnet_id[27.5]} 28=${dotnet_id[28.5]} 29=${dotnet_id[29.0]}" \
  TESTLIB_IDS="27=${testlib_id[27.5]} 28=${testlib_id[28.5]} 29=${testlib_id[29.0]}" \
  ISOL_IDS="27=${isol_id[27.5]} 28=${isol_id[28.5]} 29=${isol_id[29.0]}" \
  TRYW_IDS="27=${tryw_id[27.5]} 28=${tryw_id[28.5]} 29=${tryw_id[29.0]}" python3 "$here/corpus_bump_edit.py" "$root" "$sha" "$run" "${args[@]}"

finish "run $run"
