#!/usr/bin/env bash
# Runs neighbor probe apps on the head binary and on a main binary, and prints per app and BC version a table of every
# test whose outcome differs, with the entry the author wrote for it (tools/compare_junit.py --entries). Implementers,
# fix agents, reviewers and the CI job `neighbors` all run this script. The layout of a neighbor app and its
# differences.tsv: tests/neighbors/README.md.
#
#   tools/neighbor-diff.sh [OPTIONS] APP...    the given app folders (each may hold a differences.tsv)
#   tools/neighbor-diff.sh [OPTIONS] --changed the folders under tests/neighbors/ that differ from BASE, and the rule
#                                              that a change of a binary input adds one
#   tools/neighbor-diff.sh [OPTIONS] --plan    only the rule and the list of apps (no build, no run)
#   tools/neighbor-diff.sh [OPTIONS] --stale-mode APP...  "note" or "strict" per app: how its stale entries count
#   tools/neighbor-diff.sh [OPTIONS] --main-sidecar  "own" or "shared": whether the main run builds its own sidecar
#   tools/neighbor-diff.sh [OPTIONS] --same-inputs  the main commits, newest first from BASE, whose binary is BASE's
#
#   --bc-version N   a BC major to run on, repeatable (default 29); the build is the one oracle/oracle.toml pins
#   --base REF       the main commit to compare with (default: the merge base of HEAD and origin/main)
#
# The head binary is built from the working tree. The main binary of BASE comes from the cache
# (<cache>/main-bin/<commit>), else from the CI artifact `al-runner-main-<commit>` of BASE or of an older main commit
# with the same binary inputs (tools/binary-inputs.txt), else it is built (one build at a time, under a lock).
# Each binary runs with its own .NET sidecar: ALRT_DOTNET_SIDECAR (a sidecar built from the working tree, as CI sets it)
# serves the head run, and the main run too when tools/dotnet-sidecar equals BASE's; otherwise the main run gets no
# ALRT_DOTNET_SIDECAR and builds the sidecar sources built into it (once per source hash, in the sidecar cache).
# Exit 0: every difference has an entry. 1: a difference has none, an entry is stale or malformed, the rule fails, or
# the runs are not compared (a resource limit, no test ran, or nothing passed). 2: the tool could not finish.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$(git rev-parse --show-toplevel)" && pwd -P)" || exit 2
cache="${ALRUNNER_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/al-runtime}"
repo="${NEIGHBOR_DIFF_REPO:-aldevtools/BusinessCentral.AL.Runner}"
jobs="${CARGO_BUILD_JOBS:-4}"
# What the al-runner binary is built from (one list, read by the rule and by the lookback).
mapfile -t inputs < <(awk '!/^#/ && NF { print $1 }' "$here/binary-inputs.txt")
[ ${#inputs[@]} -gt 0 ] || exit 2
versions=() base="" mode=apps apps=()
while [ $# -gt 0 ]; do
  case "$1" in
    --bc-version) versions+=("${2:?--bc-version N}"); shift 2 ;;
    --base) base="${2:?--base REF}"; shift 2 ;;
    --changed) mode=changed; shift ;;
    --plan) mode=plan; shift ;;
    --same-inputs) mode=same; shift ;;
    --stale-mode) mode=stale; shift ;;
    --main-sidecar) mode=sidecar; shift ;;
    -h | --help) sed -n '2,25p' "$0"; exit 0 ;;
    -*) echo "neighbor-diff: unknown option $1" >&2; exit 2 ;;
    *) apps+=("$(cd "$1" && pwd -P)") || exit 2; shift ;;
  esac
done
[ ${#versions[@]} -gt 0 ] || versions=(29)
git() { command git -C "$root" "$@"; }
if [ -z "$base" ]; then
  base="$(git merge-base HEAD origin/main)" || exit 2
  [ "$base" = "$(git rev-parse origin/main)" ] ||
    echo "note: comparing with the merge base ${base:0:9}; origin/main is newer. Merge it first to compare with today's main." >&2
fi
base="$(git rev-parse --verify "$base^{commit}")" || exit 2

# Every memory-heavy step runs in the shared slice under a cap, as the other tools do.
cap() {
  local mem="$1"
  shift
  if command -v systemd-run > /dev/null; then
    systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax="$mem" "$@"
  else
    "$@"
  fi
}

# The rule (tests/neighbors/README.md): a change of a binary input, other than a pure rename, comes with a changed
# folder tests/neighbors/<topic>/ that holds an app (app.json) or a none.txt with the reason. Sets `apps`.
plan() {
  local runtime folders f status=0
  runtime="$(git diff --name-status -M100% "$base" -- "${inputs[@]}" | awk '$1 != "R100"' | head -1)"
  folders="$({
    git diff --name-only --diff-filter=d "$base" -- tests/neighbors
    git ls-files --others --exclude-standard -- tests/neighbors
  } | awk -F/ 'NF >= 4 { print $1 "/" $2 "/" $3 }' | sort -u)"
  for f in $folders; do
    if [ -f "$root/$f/none.txt" ] && [ -f "$root/$f/app.json" ]; then
      echo "$f: holds both app.json and none.txt; keep one." >&2
      status=1
    elif [ -f "$root/$f/none.txt" ]; then
      if [ -s "$root/$f/none.txt" ]; then
        echo "$f: no neighbor app. Reason: $(tr '\n' ' ' < "$root/$f/none.txt")"
      else
        echo "$f: none.txt is empty; it must give the reason why no neighbor app applies." >&2
        status=1
      fi
    elif [ -f "$root/$f/app.json" ]; then
      apps+=("$root/$f")
    else
      echo "$f: holds neither app.json nor none.txt." >&2
      status=1
    fi
  done
  if [ -n "$runtime" ] && [ -z "$folders" ]; then
    echo "A binary input changed (${runtime##*[[:space:]]} and maybe more), but no folder under tests/neighbors/ did." >&2
    echo "Add a neighbor app with its differences.tsv, or a none.txt with the reason (tests/neighbors/README.md)." >&2
    status=1
  fi
  echo "neighbor apps: ${#apps[@]}"
  [ -z "${GITHUB_OUTPUT:-}" ] || echo "apps=$([ ${#apps[@]} -gt 0 ] && echo true || echo false)" >> "$GITHUB_OUTPUT"
  return $status
}

# The al-runner of the working tree, copied to $1 so a later main build in the same target folder cannot replace it.
build_head() {
  (cd "$root" && cap 6G env CARGO_BUILD_JOBS="$jobs" "${CARGO:-cargo}" build --release -q -p alrunner-cli) >&2 || return 2
  cp "${CARGO_TARGET_DIR:-$root/target}/release/al-runner" "$1"
}

# The CI artifact of main commit $1 into folder $2.
fetch_artifact() {
  local id
  command -v gh > /dev/null || return 1
  id="$(gh api "repos/$repo/actions/artifacts?name=al-runner-main-$1&per_page=5" \
    --jq '[.artifacts[] | select(.expired | not)][0].id // empty' 2> /dev/null)"
  [ -n "$id" ] || return 1
  rm -rf "$2.tmp" && mkdir -p "$2.tmp" &&
    gh api "repos/$repo/actions/artifacts/$id/zip" > "$2.tmp/a.zip" &&
    python3 -m zipfile -e "$2.tmp/a.zip" "$2.tmp" && rm "$2.tmp/a.zip" &&
    chmod +x "$2.tmp/al-runner" && mv "$2.tmp" "$2"
}

# Builds main commit $1 into folder $2 in a worktree of its own. NEIGHBOR_MAIN_TARGET: the cargo target folder (CI
# passes its own so the dependencies are not built twice).
build_main() {
  local wt target="${NEIGHBOR_MAIN_TARGET:-$cache/main-bin/target}"
  wt="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")/.worktrees/neighbor-diff-main"
  if [ -d "$wt" ]; then
    command git -C "$wt" checkout -q --force --detach "$1" || return 2
  else
    git worktree add -q --force --detach "$wt" "$1" || return 2
  fi
  echo "building the main binary of ${1:0:9} in $wt" >&2
  (cd "$wt" && cap 6G env CARGO_TARGET_DIR="$target" CARGO_BUILD_JOBS="$jobs" "${CARGO:-cargo}" build --release -q \
    -p alrunner-cli) >&2 || return 2
  mkdir -p "$2.tmp" && cp "$target/release/al-runner" "$2.tmp/" && mv "$2.tmp" "$2"
}

# Prints the main commits, newest first from BASE (at most 30), whose binary inputs equal BASE's.
same_inputs() {
  local c
  for c in $(git rev-list --first-parent -n 30 "$base"); do
    git diff --quiet "$c" "$base" -- "${inputs[@]}" || break
    echo "$c"
  done
}

# Prints the path of a main binary for BASE: cached, from an artifact, or built.
main_binary() {
  local dir="$cache/main-bin" c
  mkdir -p "$dir"
  exec 9> "$dir/lock"
  flock 9
  for c in $(same_inputs); do
    if [ -x "$dir/$c/al-runner" ] || fetch_artifact "$c" "$dir/$c"; then
      echo "main binary: ${c:0:9} (same binary inputs as ${base:0:9})" >&2
      echo "$dir/$c/al-runner"
      return 0
    fi
  done
  build_main "$base" "$dir/$base" || return 2
  # Keep the eight newest binaries.
  ls -1dt "$dir"/[0-9a-f]*[0-9a-f] 2> /dev/null | tail -n +9 | xargs -r rm -rf
  echo "$dir/$base/al-runner"
}

pinned_build() {
  awk -v m="$1" '/^\[\[version\]\]/ { maj = "" } /^major = / { maj = $3 }
    maj == m && /^bc_build = / { gsub(/"/, "", $3); print $3; exit }' "$root/oracle/oracle.toml"
}

# Prints 'own' when the working tree's tools/dotnet-sidecar differs from BASE's (the main run then must not take the
# sidecar ALRT_DOTNET_SIDECAR names), else 'shared'. A failing git command counts as a difference.
main_sidecar() {
  local d=tools/dotnet-sidecar
  git diff --quiet "$base" -- "$d" 2> /dev/null && [ -z "$(git ls-files --others --exclude-standard -- "$d")" ] &&
    echo shared || echo own
}

# A folder the pull request does not change already has its change on main: its stale entries are a note. Anything else
# is strict: the app is outside the repository (physical path), or a git command fails or prints a change.
stale_mode() {
  local rel changed
  case "$1" in "$root"/*) rel="${1#"$root"/}" ;; *) echo strict; return ;; esac
  changed="$(git diff --name-only "$base" -- "$rel")" || { echo strict; return; }
  [ -z "$changed" ] || { echo strict; return; }
  changed="$(git ls-files --others --exclude-standard -- "$rel")" || { echo strict; return; }
  [ -z "$changed" ] && echo note || echo strict
}

[ "$mode" = same ] && { same_inputs; exit 0; }
[ "$mode" = sidecar ] && { main_sidecar; exit 0; }
if [ "$mode" = stale ]; then
  for app in "${apps[@]}"; do stale_mode "$app"; done
  exit 0
fi
status=0
if [ "$mode" != apps ]; then
  plan || status=1
  [ "$mode" = plan ] && exit $status
fi
[ ${#apps[@]} -gt 0 ] || exit $status
out="${NEIGHBOR_DIFF_OUT:-$cache/neighbor-diff/$(git rev-parse --short HEAD)}"
mkdir -p "$out"
# Absolute: each run changes into its own folder, so a relative $out would no longer name this one.
out="$(cd "$out" && pwd -P)"
head_bin="$out/al-runner-head"
build_head "$head_bin" || { echo "neighbor-diff: the head build failed" >&2; exit 2; }
main_bin="$(main_binary)" || { echo "neighbor-diff: no main binary for $base" >&2; exit 2; }
main_env=()
if [ "$(main_sidecar)" = own ] && [ -n "${ALRT_DOTNET_SIDECAR:-}" ]; then
  echo "main run: tools/dotnet-sidecar differs from ${base:0:9}; the main binary builds its own sidecar" >&2
  main_env=(-u ALRT_DOTNET_SIDECAR)
fi
for app in "${apps[@]}"; do
  name="$(basename "$app")"
  entries="$app/differences.tsv"
  [ -f "$entries" ] || entries=/dev/null
  # deps/<name>/ folders (app.json + src) are packed into source packages at run time; no binary is committed.
  dep_args=()
  # flags.txt (optional): run options both sides get, one per line, such as --strict-order.
  flag_args=()
  [ -f "$app/flags.txt" ] && mapfile -t flag_args < <(grep -v '^[[:space:]]*\(#\|$\)' "$app/flags.txt")
  run_app="$app"
  if [ -d "$app/deps" ]; then
    python3 "$here/pack_deps.py" "$app" "$out/$name.packages" > /dev/null || exit 2
    dep_args=(--dep "$out/$name.packages")
    # The runner compiles the app folder recursively: stage the app without deps/, or the dependency sources are
    # compiled into the app as well (AL0264 duplicate objects).
    run_app="$out/$name.src"
    rm -rf "$run_app" && mkdir -p "$run_app" &&
      tar -C "$app" --exclude=./deps --exclude=./differences.tsv --exclude=./flags.txt -cf - . | tar -C "$run_app" -xf - || exit 2
  fi
  for v in "${versions[@]}"; do
    build="$(pinned_build "$v")"
    for side in main head; do
      bin="$main_bin" side_env=("${main_env[@]}")
      [ $side = head ] && bin="$head_bin" side_env=()
      rm -f "$out/$name-$v.$side.xml"
      # Each run starts in a fresh copy of the app folder: a relative path in flags.txt (such as --files-dir) resolves
      # against the app, wherever the script runs from, and files a run writes there touch neither the repo nor the other run.
      cwd="$out/$name-$v.$side.cwd"
      rm -rf "$cwd" && cp -r "$app" "$cwd" || exit 2
      (cd "$cwd" && cap 3G env "${side_env[@]}" timeout 900 "$bin" test "$run_app" "${dep_args[@]}" "${flag_args[@]}" --bc-version "${build:-$v}" \
        --seed 1 --junit "$out/$name-$v.$side.xml") > "$out/$name-$v.$side.log" 2>&1
      [ -s "$out/$name-$v.$side.xml" ] || { echo "neighbor-diff: no JUnit file, see $out/$name-$v.$side.log" >&2; exit 2; }
    done
    note=()
    [ "$(stale_mode "$app")" = note ] && note=(--stale-note)
    python3 "$here/compare_junit.py" "${note[@]}" --entries "$entries" --build "${build:-$v}" --title "$name, BC ${build:-$v}" \
      "$out/$name-$v.main.xml" "$out/$name-$v.head.xml" | tee -a "${GITHUB_STEP_SUMMARY:-/dev/null}"
    [ "${PIPESTATUS[0]}" -eq 0 ] || status=1
  done
done
echo "logs and JUnit files: $out"
exit $status
