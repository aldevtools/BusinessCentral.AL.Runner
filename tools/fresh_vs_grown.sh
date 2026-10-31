#!/usr/bin/env bash
# ADR-063: a pass after the main one grows the main pass's compile by its own apps instead of compiling everything
# again. That must never change what the pass reports. This compares a normal corpus run (the passes grow) with runs
# that compile every pass from source (AL_FRESH_PASS_COMPILE=1), for every version and every suite: the outcome and the
# reason of every test, and the install baseline of every pass (the "baseline:" lines without their time).
#
#   tools/fresh_vs_grown.sh compare DIR   compare the files in DIR (what CI runs after its runs)
#   tools/fresh_vs_grown.sh run [DIR]     make the runs, then compare (the local recipe; about 2 min)
#
# Files in DIR:
#   normal, one `--all-versions --junit runtime-junit.xml` run, its output in normal.log:
#       runtime-junit-<v>.xml            runtime-junit-<suite>-<v>.xml
#   fresh, one `--bc-version <v> --junit fresh-junit-<v>.xml` run per version, its output in fresh-<v>.log:
#       fresh-junit-<v>.xml              fresh-junit-<v>-<suite>.xml
# A normal file with no fresh partner fails: a suite must not drop out of the comparison unseen.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
versions="27 28 29"

# The "baseline:" lines of a run's output, by the BC major of the pass that printed them (`target: BC <v>...`), without
# the time at their end.
baselines() {
  awk -v want="$2" '
    /^target: BC / { split($3, p, "."); v = p[1] }
    /^baseline: / && v == want { sub(/, [0-9.]+ s$/, ""); print }
  ' "$1"
}

compare() {
  local dir="$1" status=0 n=0 f v suite partner
  for v in $versions; do
    python3 "$here/compare_junit.py" --reasons "$dir/runtime-junit-$v.xml" "$dir/fresh-junit-$v.xml" || status=1
    n=$((n + 1))
    echo "== BC $v install baselines"
    if ! diff <(baselines "$dir/normal.log" "$v") <(baselines "$dir/fresh-$v.log" "$v"); then
      echo "DIFFERS: the install baseline of a pass of BC $v"
      status=1
    elif [ -z "$(baselines "$dir/normal.log" "$v")" ]; then
      echo "MISSING: no baseline line for BC $v in normal.log"
      status=1
    fi
  done
  for f in "$dir"/runtime-junit-*-*.xml; do
    [ -e "$f" ] || continue
    base="$(basename "$f" .xml)"
    v="${base##*-}"
    suite="${base#runtime-junit-}"
    suite="${suite%-"$v"}"
    partner="$dir/fresh-junit-$v-$suite.xml"
    echo "== BC $v suite $suite"
    if [ ! -f "$partner" ]; then
      echo "MISSING: $partner (the fresh run wrote no file for suite $suite)"
      status=1
      continue
    fi
    python3 "$here/compare_junit.py" --reasons "$f" "$partner" || status=1
    n=$((n + 1))
  done
  echo "compared $n pairs of JUnit files"
  return $status
}

run() {
  local dir="$1" bin="$root/target/release/al-test-cli" status=0 v
  [ -x "$bin" ] || { echo "build first: cargo build --release -p al-test-cli" >&2; return 2; }
  mkdir -p "$dir"
  rm -f "$dir"/runtime-junit*.xml "$dir"/fresh-junit*.xml
  # One corpus run at a time, under a memory cap (a run must stay under 3 GB).
  local cap=()
  command -v systemd-run >/dev/null && cap=(systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=3G)
  (cd "$dir" && "${cap[@]}" "$bin" --root "$root" --all-versions --require-dotnet --no-ratchet \
    --junit runtime-junit.xml > normal.log 2>&1) || { echo "normal run failed, see $dir/normal.log" >&2; status=1; }
  for v in $versions; do
    (cd "$dir" && AL_FRESH_PASS_COMPILE=1 "${cap[@]}" "$bin" --root "$root" --bc-version "$v" --require-dotnet \
      --no-ratchet --junit "fresh-junit-$v.xml" > "fresh-$v.log" 2>&1) \
      || { echo "fresh run for BC $v failed, see $dir/fresh-$v.log" >&2; status=1; }
  done
  compare "$dir" || status=1
  return $status
}

case "${1:-}" in
  compare) compare "${2:?usage: fresh_vs_grown.sh compare DIR}" ;;
  run) run "${2:-$root/target/fresh-vs-grown}" ;;
  *) sed -n '2,15p' "$0"; exit 2 ;;
esac
