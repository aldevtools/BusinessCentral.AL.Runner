#!/usr/bin/env bash
# Builds test binaries of one cargo profile, then runs them side by side, each the way `cargo test` runs it (from its
# package's folder, the libtest arguments given). `cargo test` runs one binary after the other, and a few long binaries
# keep the runner's cores busy only together.
#
# Usage:
#   tools/run_test_binaries.sh CARGO_ARGS -- SPEC...
#     Builds and runs the named test targets, all at once (the release-tests job of .github/workflows/ci.yml).
#     SPEC  `package:test[:libtest args]`, for example `al-test-cli:corpus_resolve_deps:--ignored --nocapture`.
#   tools/run_test_binaries.sh CARGO_ARGS -- --all [--except package:test]...
#     Builds every test binary CARGO_ARGS select (for example `--workspace --no-default-features`: the lib, bin and
#     integration test targets, not doc tests) and runs them in cargo's order, at most `nproc` at a time. `--except` leaves a test target out (the test job of ci.yml runs
#     `must_not_compile` in release instead).
#   CARGO_ARGS  passed to `cargo test --no-run`.
# Each binary's output is printed in a group of its own when all are done; the exit code is 1 when any failed.
set -euo pipefail

cargo_args=()
while [ $# -gt 0 ] && [ "$1" != -- ]; do
  cargo_args+=("$1")
  shift
done
[ "${1:-}" = -- ] || { echo "usage: $0 CARGO_ARGS -- (package:test[:args]... | --all [--except package:test]...)" >&2; exit 2; }
shift

all=false
except=()
specs=()
if [ "${1:-}" = --all ]; then
  all=true
  shift
  while [ $# -gt 0 ]; do
    [ "$1" = --except ] && [ $# -ge 2 ] || { echo "usage: --all [--except package:test]..." >&2; exit 2; }
    except+=("$2")
    shift 2
  done
else
  specs=("$@")
fi

build=()
for spec in "${specs[@]}"; do
  IFS=: read -r pkg test _ <<<"$spec"
  build+=(-p "$pkg" --test "$test")
done
logs=$(mktemp -d)
cargo test "${cargo_args[@]}" --no-run "${build[@]}" --message-format=json-render-diagnostics > "$logs/build.json"

# One line per binary to run: name, package folder, executable, libtest args (tab-separated), from cargo's build
# messages. A package's folder is named after it (crates/<package>).
python3 - "$logs/build.json" "$all" "${except[@]}" -- "${specs[@]}" > "$logs/plan.tsv" <<'PY'
import json, os, sys
args = sys.argv[1:]
path, all_mode = args[0], args[1] == "true"
sep = args.index("--")
except_ = set(args[2:sep])
specs = args[sep + 1:]
arts = []
for line in open(path):
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m.get("executable") and (m.get("profile") or {}).get("test"):
        arts.append(m)
def pkg_of(m):
    return os.path.basename(os.path.dirname(m["manifest_path"]))
out = []
if all_mode:
    for m in arts:
        t = m["target"]
        if "test" in t["kind"] and f"{pkg_of(m)}:{t['name']}" in except_:
            continue
        kind = "test" if "test" in t["kind"] else t["kind"][0]
        out.append((f"{pkg_of(m)}-{kind}-{t['name']}", os.path.dirname(m["manifest_path"]), m["executable"], ""))
else:
    for spec in specs:
        pkg, test, *rest = spec.split(":", 2)
        found = [m for m in arts if m["target"]["name"] == test and "test" in m["target"]["kind"] and pkg_of(m) == pkg]
        if not found:
            print(f"::error::no test binary {pkg}/{test} was built", file=sys.stderr)
            sys.exit(1)
        m = found[0]
        out.append((f"{pkg}-{test}", os.path.dirname(m["manifest_path"]), m["executable"], rest[0] if rest else ""))
for row in out:
    print("\t".join(row))
PY

# Named specs all start at once (a handful of long binaries); --all keeps at most nproc running.
if $all; then max=$(nproc); else max=1000000; fi
names=()
while IFS=$'\t' read -r name dir exe args; do
  while [ "$(jobs -rp | wc -l)" -ge "$max" ]; do wait -n || true; done
  # shellcheck disable=SC2086 # the libtest arguments are split on purpose
  ( cd "$dir" && "$exe" $args > "$logs/$name.log" 2>&1; echo $? > "$logs/$name.rc" ) &
  names+=("$name")
done < "$logs/plan.tsv"
wait || true
[ ${#names[@]} -gt 0 ] || { echo "::error::no test binary was built"; exit 1; }

status=0
for name in "${names[@]}"; do
  if [ "$(cat "$logs/$name.rc" 2>/dev/null)" = 0 ]; then result=ok; else result=FAILED; status=1; fi
  echo "::group::$name: $result"
  cat "$logs/$name.log"
  echo "::endgroup::"
  [ "$result" = ok ] || echo "::error::test binary $name failed"
done
exit $status
