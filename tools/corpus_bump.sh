#!/usr/bin/env bash
# Bump the corpus to the commit of a green corpus CI run, in one command (issue 44). The steps are the
# ones oracle/README.md describes ("To bump"):
#   1. check the run: completed and green, and its head commit on the corpus branch master (the default
#      branch; it moves only through a pull request from al-runtime-rs-staging, merged with a merge commit);
#      when a merge commit brought the commit into master, that merge must hold exactly the commit's files;
#   2. find the artifacts: per leg (27.5, 28.5, 29.0) exactly two named junit-test-results-<leg>, the
#      larger is the oracle (junit-al-language.xml), the smaller the OnPrem file;
#   3. move the corpus submodule to the run's commit;
#   4. download each artifact by id and copy its XML unchanged to oracle/<major>/junit.xml and
#      oracle/<major>/junit-onprem.xml;
#   5. write the new pin, run and artifact ids into oracle/oracle.toml and oracle/README.md;
#   6. build release and regenerate the baselines (al-test-cli refuses when a baseline case regresses);
#   7. write the new case counts into crates/al-test-cli/tests/corpus_harness.rs.
# It never edits a JUnit file and never edits docs/probes-needed.md: it prints the corpus files added
# since the old pin, and the number of table rows of probes-needed before and after, so the person
# running it can strike the rows the new probes settle.
#
#   tools/corpus_bump.sh [--dry-run] RUN_ID
#
# --dry-run checks the run and prints what it would do, changing nothing.
# For tests: CORPUS_BUMP_GH (the gh to call), CORPUS_BUMP_CORPUS (the corpus repository),
# CORPUS_BUMP_NO_FETCH=1 (do not fetch the corpus branch).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
gh="${CORPUS_BUMP_GH:-gh}"
corpus="${CORPUS_BUMP_CORPUS:-$root/corpus}"
repo="aldevtools/BusinessCentral.AL.Language.Tests"
branch="master"
legs="27.5 28.5 29.0"

usage() { echo "usage: tools/corpus_bump.sh [--dry-run] RUN_ID" >&2; exit 2; }
die() { echo "corpus_bump: refused: $*" >&2; exit 1; }

dry=0
run=""
for a in "$@"; do
  case "$a" in
    --dry-run) dry=1 ;;
    -*) usage ;;
    *) [ -z "$run" ] || usage; run="$a" ;;
  esac
done
[ -n "$run" ] || usage
[[ "$run" =~ ^[0-9]+$ ]] || usage

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
  "$gh" api "repos/$repo/actions/artifacts/$1/zip" > "$d/a.zip"
  unzip -Z1 "$d/a.zip" | grep '\.xml$' | head -n 1
}
declare -A oracle_id onprem_id dotnet_id
for leg in $legs; do
  # Two artifacts: the oracle (larger) and the OnPrem file. Three (since the OnPrem DotNet app runs in CI): the DotNet file is the smallest.
  # Four (since the TestLibraries job): the roles are read from the XML file names below.
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
    big=0 mid=0 small=0
    while read -r id; do
      case "$(xml_of "$id")" in
        junit-al-language.xml) [ "$big" != 0 ] || big=$id ;;
        junit-al-language-onprem.xml) [ "$mid" != 0 ] || mid=$id ;;
        junit-al-language-onprem-dotnet.xml) [ "$small" != 0 ] || small=$id ;;
        # The TestLibraries job runs with Microsoft's Tests-TestLibraries installed, an environment the runtime does not
        # model (orchestrator, 2026-10-22): its results are no oracle.
        junit-al-language-testlibraries.xml) ;;
        *) die "artifact $id of junit-test-results-$leg holds no known JUnit file" ;;
      esac
    done < <(python3 -c '
import json, sys
a = [x for x in json.loads(sys.argv[1]).get("artifacts", []) if x.get("name") == "junit-test-results-" + sys.argv[2]]
a.sort(key=lambda x: x.get("created_at", ""), reverse=True)
print("\n".join(str(x["id"]) for x in a))' "$arts" "$leg")
    { [ "$big" != 0 ] && [ "$mid" != 0 ]; } || die "run $run: junit-test-results-$leg lacks the oracle or the OnPrem file"
  fi
  { [ "$n" -ge 2 ]; } || die "run $run has $n artifact(s) named junit-test-results-$leg, expected 2 to 4 (the oracle, the OnPrem file, the OnPrem DotNet file and the TestLibraries file)"
  oracle_id[$leg]=$big
  onprem_id[$leg]=$mid
  dotnet_id[$leg]=$small
  echo "$leg: oracle artifact $big, onprem artifact $mid${small:+, dotnet artifact $small}"
done

old="$(git -C "$root" ls-tree HEAD corpus 2>/dev/null | awk '{print $3}')"
old="${old:-$(git -C "$corpus" rev-parse HEAD)}"
if git -C "$corpus" cat-file -e "$old^{commit}" 2>/dev/null; then
  echo "corpus files added since the old pin ${old:0:7}:"
  git -C "$corpus" diff --name-only --diff-filter=A "$old" "$sha" -- tests | sed 's/^/  /'
else
  echo "the old pin ${old:0:7} is not in the corpus repository: no list of added files"
fi
probe_rows() { grep -c '^| ' "$root/docs/probes-needed.md" || true; }
echo "docs/probes-needed.md table rows before: $(probe_rows)"

if [ "$dry" = 1 ]; then
  echo "would move corpus to $sha, copy the six artifacts above into oracle/27, 28, 29,"
  echo "update oracle/oracle.toml and oracle/README.md, regenerate the baselines and the harness counts"
  echo "dry run: nothing changed"
  exit 0
fi

cases() { python3 - "$1" <<'PY'
import sys, xml.etree.ElementTree as ET
n = sum(1 for c in ET.parse(sys.argv[1]).iter("testcase") if c.get("name") != "OnRun")
print(n)
PY
}
# The runtime's harness totals leave out the cases real BC skipped (BC 27 lists O_Database_CopyCompany as <skipped/>):
# 14975 JUnit cases were 14974 runtime cases in bump 27. The oracle README keeps the JUnit count (cases()).
run_cases() { python3 - "$1" <<'PY'
import sys, xml.etree.ElementTree as ET
n = sum(1 for c in ET.parse(sys.argv[1]).iter("testcase")
        if c.get("name") != "OnRun" and c.find("skipped") is None)
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
  "$gh" api "repos/$repo/actions/artifacts/$id/zip" > "$d/a.zip"
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
DOTNET_IDS="27=${dotnet_id[27.5]} 28=${dotnet_id[28.5]} 29=${dotnet_id[29.0]}" python3 "$here/corpus_bump_edit.py" "$root" "$sha" "$run" "${args[@]}"

# 6. Baselines (al-test-cli refuses while a baseline case regresses).
cd "$root"
cargo build --release -q
for major in 27 28 29; do
  ./target/release/al-test-cli --root . --update-baseline --bc-version "$major" | grep -E "^BC [0-9.]+: " || die "baseline for BC $major"
done

# 7. The harness counts: totals per major and the BC 29 area totals the harness asserts.
dir="$(mktemp -d)"
./target/release/al-test-cli --root . --all-versions --no-deps --json "$dir/pass-rate.json" >/dev/null 2>&1 || true
hargs=()
for leg in $legs; do
  major="${leg%%.*}"
  hargs+=("$major" "${oracle_id[$leg]}" "${onprem_id[$leg]}" "$(run_cases "$root/oracle/$major/junit.xml")" "$(run_cases "$root/oracle/$major/junit-onprem.xml")")
done
python3 - "$root" "$dir" "$old" "$sha" "${hargs[@]}" <<'PY'
import json, os, re, sys
root, out, old, new, rest = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4], sys.argv[5:]
legs = {rest[i]: rest[i + 3] for i in range(0, len(rest), 5)}
onprem = {rest[i]: rest[i + 4] for i in range(0, len(rest), 5)}
p = root + "/crates/al-test-cli/tests/corpus_harness.rs"
t = open(p).read()
t = re.sub(r'\[\("27", \d+\), \("28", \d+\), \("29", \d+\)\]',
           f'[("27", {legs["27"]}), ("28", {legs["28"]}), ("29", {legs["29"]})]', t)
# The BC 29 totals, and the BC 28 total of the test that reads the other test apps; the OnPrem total of BC 28.
t = re.sub(r'(json\("29"\);\n\s*assert_eq!\(r\["overall"\]\["total"\], )\d+\);', rf'\g<1>{legs["29"]});', t)
t = re.sub(r'(json\("28"\);\n\s*assert_eq!\(r\["overall"\]\["total"\], )\d+\);', rf'\g<1>{legs["28"]});', t)
t = re.sub(r'(get\("onprem"\)\["counts"\]\["total"\], )\d+\);', rf'\g<1>{onprem["28"]});', t)
if old != new:
    t = t.replace(old, new)
t = re.sub(r'( of )\d+( \()', rf'\g<1>{legs["29"]}\2', t)
f = os.path.join(out, "pass-rate-29.json")
if os.path.exists(f):
    areas = json.load(open(f))["areas"]
    for area in ("json", "record"):
        t = re.sub(rf'(assert_eq!\(r\["areas"\]\["{area}"\]\["total"\], )\d+\);',
                   rf'\g<1>{areas[area]["total"]});', t)
open(p, "w").write(t)
PY
rm -rf "$dir"
echo "docs/probes-needed.md table rows after: $(probe_rows) (strike the rows the new probes settle by hand)"
echo "bumped to $sha from run $run; review git diff, then run ./check.sh"
