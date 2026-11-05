#!/usr/bin/env bash
# ADR-066: Microsoft's test runner codeunit (130450, run as BcContainerHelper runs it) against al-runner's own runner,
# which stays the differential oracle until the replaced code is deleted. Every difference must be explained (a test
# that subscribes to the events of "Test Runner - Mgt", or a refusal of the runner's own code); list reviewed ones in
# tools/runner-builtin-known-differences.txt.
#
#   tools/runner_vs_builtin.sh corpus [DIR]          the corpus BC 27 leg (no TestRunnerHub: every codeunit goes through
#                                                    130450), with al-test-cli --test-runner microsoft and builtin
#   tools/runner_vs_builtin.sh app APP [DIR] [ARGS]  one app or Microsoft bucket (*.Source.zip) with al-runner, with
#                                                    the default runner and --test-runner builtin; ARGS go to both runs
#
# Release binaries needed. One run at a time, each under a memory cap. Prints the differences (tools/compare_junit.py)
# and the wall time of each run; exits 1 on an unlisted difference.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
allow="$here/runner-builtin-known-differences.txt"
cap=()
command -v systemd-run >/dev/null && cap=(systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=8G)

timed() {
  local label="$1"
  shift
  local start ms
  start=$(date +%s%N)
  "$@" >"$dir/$label.log" 2>&1
  local status=$?
  ms=$((($(date +%s%N) - start) / 1000000))
  printf '%s: %d.%03d s (exit %s; a run with failing tests exits 1)\n' "$label" $((ms / 1000)) $((ms % 1000)) "$status"
  grep -E '^timings' "$dir/$label.log" || true
}

compare() {
  local args=()
  [ -f "$allow" ] && args=(--allow "$allow")
  python3 "$here/compare_junit.py" "$dir/builtin.xml" "$dir/runner.xml" "${args[@]}"
}

case "${1:-}" in
  corpus)
    dir="${2:-$HOME/.cache/al-runtime/runner-vs-builtin/corpus}"
    mkdir -p "$dir"
    bin="$root/target/release/al-test-cli"
    [ -x "$bin" ] || { echo "build first: cargo build --release -p al-test-cli" >&2; exit 2; }
    timed builtin "${cap[@]}" "$bin" --root "$root" --bc-version 27 --no-ratchet --timings --test-runner builtin --junit "$dir/builtin.xml"
    timed runner "${cap[@]}" "$bin" --root "$root" --bc-version 27 --no-ratchet --timings --test-runner microsoft --junit "$dir/runner.xml"
    compare
    ;;
  app)
    app="${2:?an app folder or *.Source.zip}"
    dir="${3:-$HOME/.cache/al-runtime/runner-vs-builtin/$(basename "$app" .zip)}"
    shift 3 2>/dev/null || shift $#
    mkdir -p "$dir"
    bin="$root/target/release/al-runner"
    [ -x "$bin" ] || { echo "build first: cargo build --release -p alrunner-cli" >&2; exit 2; }
    timed builtin "${cap[@]}" "$bin" test "$app" --test-runner builtin --junit "$dir/builtin.xml" "$@"
    timed runner "${cap[@]}" "$bin" test "$app" --junit "$dir/runner.xml" "$@"
    compare
    ;;
  *)
    sed -n '2,14p' "$0"
    exit 2
    ;;
esac
