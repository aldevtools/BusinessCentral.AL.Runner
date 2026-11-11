#!/usr/bin/env bash
# Record the BC 28 isolation oracle (suites isolation-function and isolation-codeunit of oracle/oracle.toml) from a
# corpus "Nightly Windows (official MS BC container)" run. The isolation probe (corpus tests/al-language-isolation-probe)
# needs two custom test runners (61300, 61301), which only that Windows workflow provides, so tools/corpus_bump.sh does
# not record these suites. Run this when the CLI refuses with "the oracle is recorded for corpus commit X, but the pin Y
# changes or removes existing test files" for one of them (src/pin.rs).
#
#   tools/record_windows_oracle.sh [--dry-run] [--pin SHA] RUN_ID
#
# It refuses (exit 1, nothing changed) unless:
#   1. the run is of that workflow, completed and green;
#   2. its head commit is an ancestor of the pin (default: the commit the corpus submodule has checked out), and no file
#      under tests/al-language-isolation-probe differs between the two (the oracle then describes the pin, ADR-007);
#   3. the run has exactly one artifact nightly-windows-bc<version>-<run> of a BC 28 version, it downloads, and it holds
#      xunit-isolation-function.xml and xunit-isolation-codeunit.xml exactly once each.
# Then it copies the two files byte for byte to oracle/28/xunit-isolation-{function,codeunit}.xml and rewrites
# corpus_commit, workflow_run, artifact_id and bc_version in the two suite tables and their oracle/README.md rows
# (tools/record_windows_oracle_edit.py). --dry-run prints those edits and changes nothing.
# The comment above the suite tables in oracle.toml is prose: update it by hand.
# For tests: RECORD_WINDOWS_ORACLE_GH (the gh to call), RECORD_WINDOWS_ORACLE_ROOT (the repository root),
# RECORD_WINDOWS_ORACLE_NO_FETCH=1 (do not fetch the corpus branch).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="${RECORD_WINDOWS_ORACLE_ROOT:-$(dirname "$here")}"
gh="${RECORD_WINDOWS_ORACLE_GH:-gh}"
corpus="$root/corpus"
repo="aldevtools/BusinessCentral.AL.Language.Tests"
workflow="Nightly Windows (official MS BC container)"
probe="tests/al-language-isolation-probe"

usage() { echo "usage: tools/record_windows_oracle.sh [--dry-run] [--pin SHA] RUN_ID" >&2; exit 2; }
die() { echo "record_windows_oracle: refused: $*" >&2; exit 1; }

dry=0 run="" pin=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dry-run) dry=1 ;;
    --pin) [ $# -ge 2 ] || usage; pin="$2"; shift ;;
    -*) usage ;;
    *) [ -z "$run" ] || usage; run="$1" ;;
  esac
  shift
done
[[ "$run" =~ ^[0-9]+$ ]] || usage

# 1. The run.
info="$("$gh" run view "$run" -R "$repo" --json status,conclusion,headSha,workflowName)" \
  || die "gh could not read run $run (output above)"
read -r status conclusion sha name < <(python3 -c '
import json, sys
r = json.loads(sys.argv[1])
print(r.get("status", "") or "-", r.get("conclusion", "") or "-", r.get("headSha", "") or "-", r.get("workflowName", ""))' "$info")
[ "$name" = "$workflow" ] || die "run $run is of workflow '$name', not the workflow '$workflow'"
[ "$status" = completed ] || die "run $run is not completed (status: $status)"
[ "$conclusion" = success ] || die "run $run is not green (conclusion: $conclusion)"
[[ "$sha" =~ ^[0-9a-f]{40}$ ]] || die "run $run has no head commit"

# 2. The run's commit against the pin.
if [ "${RECORD_WINDOWS_ORACLE_NO_FETCH:-0}" != 1 ]; then git -C "$corpus" fetch -q origin master; fi
pin="$(git -C "$corpus" rev-parse "${pin:-HEAD}^{commit}")" || die "the pin ${pin:-HEAD} is not a commit of the corpus"
git -C "$corpus" cat-file -e "$sha^{commit}" 2>/dev/null || die "commit $sha of run $run is not in the corpus repository"
git -C "$corpus" merge-base --is-ancestor "$sha" "$pin" \
  || die "commit ${sha:0:12} of run $run is not an ancestor of the pin ${pin:0:12}"
changed="$(git -C "$corpus" diff --name-status --no-renames "$sha" "$pin" -- "$probe")"
[ -z "$changed" ] || die "the pin ${pin:0:12} changes the isolation probe app since run $run's commit ${sha:0:12}, so the run does not describe it:
$changed"
echo "run $run: green, commit ${sha:0:12}, an ancestor of the pin ${pin:0:12} with the same $probe"

# 3. The artifact.
arts="$("$gh" api "repos/$repo/actions/runs/$run/artifacts?per_page=100")" || die "gh could not list the artifacts of run $run (output above)"
read -r n artifact bc_version < <(python3 -c '
import json, re, sys
a = [x for x in json.loads(sys.argv[1]).get("artifacts", [])
     if re.fullmatch(r"nightly-windows-bc[0-9.]+-" + sys.argv[2], x.get("name", "")) and not x.get("expired")]
if len(a) == 1:
    print(1, a[0]["id"], re.match(r"nightly-windows-bc([0-9.]+)-", a[0]["name"]).group(1))
else:
    print(len(a), 0, 0)' "$arts" "$run")
[ "$n" = 1 ] || die "run $run has $n unexpired artifacts named nightly-windows-bc<version>-$run, expected 1"
[[ "$bc_version" =~ ^28\. ]] || die "run $run tested BC $bc_version; the isolation suites are in the BC 28 table"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
"$gh" api "repos/$repo/actions/artifacts/$artifact/zip" > "$tmp/a.zip" \
  || die "downloading artifact $artifact of run $run failed (the gh error is above)"
unzip -q "$tmp/a.zip" -d "$tmp/x" || die "artifact $artifact of run $run is not a zip file"
declare -A file
for suite in function codeunit; do
  found="$(find "$tmp/x" -type f -name "xunit-isolation-$suite.xml")"
  [ "$(printf '%s' "$found" | grep -c .)" = 1 ] \
    || die "artifact $artifact of run $run does not hold xunit-isolation-$suite.xml exactly once"
  file[$suite]="$found"
done
count() { grep -o '<test ' "$1" | grep -c . || true; }
date="$(grep -o -m1 'run-date="[0-9-]*"' "${file[function]}" | sed 's/run-date="\(.*\)"/\1/' || true)"
[ -n "$date" ] || die "xunit-isolation-function.xml of artifact $artifact has no run-date"
echo "artifact $artifact (BC $bc_version, $date): $(count "${file[function]}") cases under runner 61300, $(count "${file[codeunit]}") under 61301"

edit() { python3 -I "$here/record_windows_oracle_edit.py" "$root" "$sha" "$run" "$artifact" "$bc_version" "$date" \
  "$(count "${file[function]}")" "$(count "${file[codeunit]}")" "$1"; }
echo "edits:"
edit 1
if [ "$dry" = 1 ]; then echo "dry run: nothing changed"; exit 0; fi
for suite in function codeunit; do cp "${file[$suite]}" "$root/oracle/28/xunit-isolation-$suite.xml"; done
edit 0 >/dev/null
echo "recorded oracle/28/xunit-isolation-function.xml and -codeunit.xml from run $run; update the comment above the suite tables in oracle/oracle.toml by hand"
