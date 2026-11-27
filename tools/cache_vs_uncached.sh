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
#   tools/cache_vs_uncached.sh workspace [DIR]  the leg of the workspace layer (ADR-022 amendment 2026-11-15,
#                                            "Equality requirement", Outcomes), one BC version at a time
#
# The workspace leg: for each version of VERSIONS (default "27 28 29"), three runs with the compile cache in which
# every pass compiles from source (AL_FRESH_PASS_COMPILE=1; a pass that grows the main pass's compile does not use the
# layer): without the workspace layer (wscached-junit-<v>*.xml), with it (wsbuilt-, each pass builds its entry or reads
# one already there) and with it again (wsread-, each pass reads). The built and the read runs must give the outcomes
# and reasons of the first, with no allow list, for the main app and every suite (OnPrem, isolation), and every pass of
# the read run must say "workspace layer read". AL_TEST_CLI names the binary (default target/release/al-test-cli);
# LEG_DOTNET= (empty) leaves out --require-dotnet on a machine without the .NET runtime of the version: its DotNet tests
# then have no outcome in any of the three runs, so the leg compares fewer tests.
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
bin="${AL_TEST_CLI:-$root/target/release/al-test-cli}"

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
  local dir="$1" status=0
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

# The workspace leg (header): the runs of one version, then their comparison.
workspace_version() {
  local dir="$1" v="$2" status=0 kind f base partner passes reads
  for kind in wscached wsbuilt wsread; do
    rm -f "$dir/$kind-junit-$v"*.xml
    local env=(AL_FRESH_PASS_COMPILE=1)
    [ "$kind" = wscached ] || env+=(AL_CORPUS_WORKSPACE_CACHE=1)
    # shellcheck disable=SC2086 # LEG_DOTNET is one flag or none
    (cd "$dir" && env "${env[@]}" "$bin" --root "$root" --bc-version "$v" ${LEG_DOTNET---require-dotnet} --no-ratchet \
      --compile-cache --junit "$kind-junit-$v.xml" > "$kind-$v.log" 2>&1) ||
      { echo "the $kind run of BC $v failed, see $dir/$kind-$v.log" >&2; status=1; }
  done
  for kind in wsbuilt wsread; do
    for f in "$dir/wscached-junit-$v"*.xml; do
      [ -e "$f" ] || { echo "MISSING: the wscached run of BC $v wrote no JUnit file"; status=1; continue; }
      base="$(basename "$f")"
      partner="$dir/$kind-${base#wscached-}"
      echo "== $kind ${base#wscached-junit-}"
      if [ ! -f "$partner" ]; then
        echo "MISSING: $partner"
        status=1
        continue
      fi
      python3 "$here/compare_junit.py" --reasons "$f" "$partner" || status=1
    done
  done
  # Every pass of the read run read its entry: otherwise the leg compared a fresh compile with itself.
  passes="$(grep -c "): workspace layer " "$dir/wsread-$v.log")"
  reads="$(grep -c "): workspace layer read" "$dir/wsread-$v.log")"
  grep "): workspace layer " "$dir/wsbuilt-$v.log" "$dir/wsread-$v.log"
  if [ "$passes" = 0 ] || [ "$passes" != "$reads" ]; then
    echo "BC $v: $reads of $passes passes of the read run read the workspace layer"
    status=1
  fi
  return $status
}

workspace() {
  local dir="$1" status=0 v
  [ -x "$bin" ] || { echo "build first: cargo build --release -p al-test-cli" >&2; return 2; }
  mkdir -p "$dir"
  for v in ${VERSIONS:-$versions}; do
    echo "== BC $v"
    workspace_version "$dir" "$v" || status=1
  done
  return $status
}

case "${1:-}" in
  compare) compare "${2:?usage: cache_vs_uncached.sh compare DIR}" ;;
  run) run "${2:-$root/target/cache-vs-uncached}" ;;
  workspace) workspace "${2:-$root/target/cache-vs-uncached}" ;;
  *) sed -n '2,28p' "$0"; exit 2 ;;
esac
