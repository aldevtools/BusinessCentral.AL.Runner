#!/usr/bin/env bash
# ADR-022: a run that finds the dependency apps in the compile cache must give the outcome of the
# uncached run for every test. This compares the JUnit files of a normal corpus run (demand-driven
# compile of the dependency apps) with those of a run that uses the cache, for the main app and for
# every extra suite (OnPrem, the isolation probes). Any difference not listed in
# tools/demand-eager-known-differences.txt fails: the cache holds the complete compile, so it
# differs from the demand-driven run exactly where the eager run does, and from the eager run
# nowhere (tests/demand_audit.rs and the Program comparison of al-sema check that part).
#
#   tools/cache_vs_uncached.sh compare DIR   compare the files in DIR (what CI runs after its runs)
#   tools/cache_vs_uncached.sh run [DIR]     make the runs, then compare (the local recipe)
#
# File names in DIR, which is what `al-test-cli --all-versions --junit` writes:
#   uncached:  runtime-junit-<v>.xml          runtime-junit-<suite>-<v>.xml
#   cached:    cached-junit-<v>.xml           cached-junit-<suite>-<v>.xml
#   eager decode (ADR-054, ALRUNNER_EAGER_DECODE=1 with the cache): eagerdecode-junit-<v>.xml, ...; when they
#   are there, each must equal its cached partner (the lazy decode), with no allow list.
# A normal file with no cached partner fails: a suite must not drop out of the comparison unseen.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
versions="27 28 29"

compare() {
  local dir="$1" status=0 n=0 f v partner
  for v in $versions; do
    echo "== BC $v"
    python3 "$here/compare_junit.py" --allow "$here/demand-eager-known-differences.txt" \
      "$dir/runtime-junit-$v.xml" "$dir/cached-junit-$v.xml" || status=1
    n=$((n + 1))
  done
  # runtime-junit-<suite>-<v>.xml: the suites are the files whose name has more than the version.
  for f in "$dir"/runtime-junit-*-*.xml; do
    [ -e "$f" ] || continue
    base="$(basename "$f" .xml)"
    partner="$dir/cached-junit-${base#runtime-junit-}.xml"
    echo "== ${base#runtime-junit-}"
    if [ ! -f "$partner" ]; then
      echo "MISSING: $partner (the cached run wrote no file for ${base#runtime-junit-})"
      status=1
      continue
    fi
    python3 "$here/compare_junit.py" --allow "$here/demand-eager-known-differences.txt" \
      "$f" "$partner" || status=1
    n=$((n + 1))
  done
  # ADR-054: lazy against eager decode of the cached objects.
  for f in "$dir"/eagerdecode-junit-*.xml; do
    [ -e "$f" ] || continue
    base="$(basename "$f" .xml)"
    partner="$dir/cached-junit-${base#eagerdecode-junit-}.xml"
    echo "== eager decode ${base#eagerdecode-junit-}"
    if [ ! -f "$partner" ]; then
      echo "MISSING: $partner"
      status=1
      continue
    fi
    python3 "$here/compare_junit.py" "$f" "$partner" || status=1
    n=$((n + 1))
  done
  echo "compared $n pairs of JUnit files"
  return $status
}

run() {
  local dir="$1" bin="$root/target/release/al-test-cli" status=0
  [ -x "$bin" ] || { echo "build first: cargo build --release -p al-test-cli" >&2; return 2; }
  mkdir -p "$dir"
  rm -f "$dir"/runtime-junit*.xml "$dir"/cached-junit*.xml "$dir"/eagerdecode-junit*.xml
  # One corpus run at a time, under a memory cap (a run must stay under 3 GB).
  local cap=()
  command -v systemd-run >/dev/null && cap=(systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=3G)
  (cd "$dir" && "${cap[@]}" "$bin" --root "$root" --all-versions --require-dotnet --no-ratchet \
    --junit runtime-junit.xml > normal.log 2>&1) || { echo "uncached run failed, see $dir/normal.log" >&2; status=1; }
  (cd "$dir" && "${cap[@]}" "$bin" --root "$root" --all-versions --require-dotnet --no-ratchet --compile-cache \
    --junit cached-junit.xml > cached.log 2>&1) || { echo "cached run failed, see $dir/cached.log" >&2; status=1; }
  (cd "$dir" && ALRUNNER_EAGER_DECODE=1 "${cap[@]}" "$bin" --root "$root" --all-versions --require-dotnet --no-ratchet \
    --compile-cache --junit eagerdecode-junit.xml > eagerdecode.log 2>&1) ||
    { echo "eager decode run failed, see $dir/eagerdecode.log" >&2; status=1; }
  compare "$dir" || status=1
  return $status
}

case "${1:-}" in
  compare) compare "${2:?usage: cache_vs_uncached.sh compare DIR}" ;;
  run) run "${2:-$root/target/cache-vs-uncached}" ;;
  *) sed -n '2,17p' "$0"; exit 2 ;;
esac
