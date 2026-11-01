#!/usr/bin/env bash
# Builds the named test binaries of one cargo profile, then runs them side by side, each the way `cargo test` runs it
# (from its package's folder, the libtest arguments given). Used by the release-tests job of .github/workflows/ci.yml:
# `cargo test` runs one binary after the other, and these few long binaries keep the runner's cores busy only together.
#
# Usage: tools/run_test_binaries.sh CARGO_ARGS -- SPEC...
#   CARGO_ARGS  passed to `cargo test --no-run` (for example `--release`).
#   SPEC        `package:test[:libtest args]`, for example `al-test-cli:corpus_resolve_deps:--ignored --nocapture`.
# Each binary's output is printed in a group of its own when all are done; the exit code is 1 when any failed.
set -euo pipefail

cargo_args=()
while [ $# -gt 0 ] && [ "$1" != -- ]; do
  cargo_args+=("$1")
  shift
done
[ "${1:-}" = -- ] || { echo "usage: $0 CARGO_ARGS -- package:test[:args]..." >&2; exit 2; }
shift

build=()
for spec in "$@"; do
  IFS=: read -r pkg test _ <<<"$spec"
  build+=(-p "$pkg" --test "$test")
done
logs=$(mktemp -d)
cargo test "${cargo_args[@]}" --no-run "${build[@]}" --message-format=json-render-diagnostics > "$logs/build.json"

pids=()
names=()
for spec in "$@"; do
  IFS=: read -r pkg test args <<<"$spec"
  # The executable and package folder of this test target, from cargo's build messages.
  exe=""
  read -r exe dir < <(python3 - "$logs/build.json" "$pkg" "$test" <<'PY'
import json, os, sys
path, pkg, test = sys.argv[1:]
for line in open(path):
    try:
        m = json.loads(line)
    except ValueError:
        continue
    t = m.get("target") or {}
    # A package's folder is named after it (crates/<package>).
    if m.get("reason") == "compiler-artifact" and m.get("executable") and t.get("name") == test \
            and "test" in t.get("kind", []) and os.path.basename(os.path.dirname(m["manifest_path"])) == pkg:
        print(m["executable"], os.path.dirname(m["manifest_path"]))
        break
PY
  ) || true
  [ -n "${exe:-}" ] || { echo "::error::no test binary $pkg/$test was built"; exit 1; }
  # shellcheck disable=SC2086 # the libtest arguments are split on purpose
  (cd "$dir" && exec "$exe" $args) > "$logs/$pkg-$test.log" 2>&1 &
  pids+=($!)
  names+=("$pkg-$test")
done

status=0
for i in "${!pids[@]}"; do
  if wait "${pids[$i]}"; then result=ok; else result=FAILED; status=1; fi
  echo "::group::${names[$i]}: $result"
  cat "$logs/${names[$i]}.log"
  echo "::endgroup::"
  [ "$result" = ok ] || echo "::error::test binary ${names[$i]} failed"
done
exit $status
