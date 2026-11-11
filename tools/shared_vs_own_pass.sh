#!/usr/bin/env bash
# ADR-073 Phase 2: a suite that differs from a pass only in its server settings (the try-write suite) runs in that pass,
# on an engine of its own. That must never change what any suite reports. This compares a normal corpus run with runs
# that give such a suite a pass of its own again (AL_SERVER_SETTINGS_OWN_PASS=1), for every version and every suite: the
# outcome and the reason (failure or refusal message) of every test.
#
#   tools/shared_vs_own_pass.sh compare DIR   compare the files in DIR (what CI runs after its runs)
#   tools/shared_vs_own_pass.sh run [DIR]     make the runs, then compare (the local recipe; about 4 min)
#
# Files in DIR:
#   normal, one `--all-versions --junit runtime-junit.xml` run:
#       runtime-junit-<v>.xml            runtime-junit-<suite>-<v>.xml
#   own pass, one `--bc-version <v> --junit own-junit-<v>.xml` run per version:
#       own-junit-<v>.xml                own-junit-<v>-<suite>.xml
# A normal file with no own-pass partner fails: a suite must not drop out of the comparison unseen.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
versions="${VERSIONS:-27 28 29}"

compare() {
  local dir="$1" status=0 n=0 f v suite partner
  for v in $versions; do
    echo "== BC $v main"
    python3 "$here/compare_junit.py" --reasons "$dir/runtime-junit-$v.xml" "$dir/own-junit-$v.xml" || status=1
    n=$((n + 1))
  done
  for f in "$dir"/runtime-junit-*-*.xml; do
    [ -e "$f" ] || continue
    base="$(basename "$f" .xml)"
    v="${base##*-}"
    case " $versions " in *" $v "*) ;; *) continue ;; esac
    suite="${base#runtime-junit-}"
    suite="${suite%-"$v"}"
    partner="$dir/own-junit-$v-$suite.xml"
    echo "== BC $v suite $suite"
    if [ ! -f "$partner" ]; then
      echo "MISSING: $partner (the own-pass run wrote no file for suite $suite)"
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
  rm -f "$dir"/runtime-junit*.xml "$dir"/own-junit*.xml
  # One corpus run at a time, under a memory cap (a run must stay under 3 GB).
  local cap=()
  command -v systemd-run >/dev/null && cap=(systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=3G)
  (cd "$dir" && "${cap[@]}" "$bin" --root "$root" --all-versions --require-dotnet --no-ratchet \
    --junit runtime-junit.xml > normal.log 2>&1) || { echo "normal run failed, see $dir/normal.log" >&2; status=1; }
  for v in $versions; do
    (cd "$dir" && AL_SERVER_SETTINGS_OWN_PASS=1 "${cap[@]}" "$bin" --root "$root" --bc-version "$v" --require-dotnet \
      --no-ratchet --junit "own-junit-$v.xml" > "own-$v.log" 2>&1) \
      || { echo "own-pass run for BC $v failed, see $dir/own-$v.log" >&2; status=1; }
  done
  compare "$dir" || status=1
  return $status
}

case "${1:-}" in
  compare) compare "${2:?usage: shared_vs_own_pass.sh compare DIR}" ;;
  run) run "${2:-$root/target/shared-vs-own-pass}" ;;
  *) sed -n '2,15p' "$0"; exit 2 ;;
esac
