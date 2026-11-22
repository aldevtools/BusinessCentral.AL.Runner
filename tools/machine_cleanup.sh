#!/usr/bin/env bash
# Lists, and with --apply deletes, what agents leave behind on a shared machine: worktrees of finished pull requests,
# idle cargo target folders, old compile cache entries and old scratch folders. Dry run by default: it prints one line
# per item ("would delete" or "keep" with the reason) and the total it would free.
#
#   tools/machine_cleanup.sh [--apply] [--keep-hours N] [--compile-days N] [--scratch-days N] [--no-gh | --prs FILE]
#
#   --apply           delete; without it nothing changes.
#   --keep-hours N    anything written in the last N hours is kept (default 6).
#   --compile-days N  compile cache entries unused for N x 24 hours go (default 3). A cache hit sets the entry's
#                     modification time, and a missing entry is a miss (crates/al-testrun/src/cache_gc.rs), so a
#                     deleted entry costs at most one rebuild of it.
#   --scratch-days N  folders under <cache>/scratch/ untouched for N days go (default 7).
#   --no-gh           do not ask GitHub; then no worktree is removed (target folders and caches still are).
#   --prs FILE        read the pull requests from FILE, one "BRANCH STATE" line each (STATE as gh prints it:
#                     OPEN, MERGED, CLOSED), instead of asking GitHub.
#
# What it never touches: a worktree with uncommitted or unpushed work (also in a submodule: uncommitted files, commits
# on no remote branch, a stash; commits made on a detached HEAD and then left behind sit in the submodule's reflog
# only, and are not seen), a locked worktree, a worktree whose branch has an open or no pull request, the target/
# of a worktree whose pull request state was not read or is open (a reviewer may run that head binary; only MERGED,
# CLOSED or no pull request lets a target/ go), a folder a running process uses
# (working folder, executable or CARGO_TARGET_DIR; a process in a worktree's root counts as using that worktree's
# target/), the main checkout's own files, the artifact cache (<cache>/artifacts), and every folder it does not name
# above. Local branches go only with `git branch -d` (fully merged), never -D. It deletes no remote branch.
# Two runs at once: the second prints "already running" and exits 0.
#
# <cache> is chosen by tools/cache_root.sh (the order of crates/al-deps/src/cache.rs); it refuses / and $HOME.
# Sizes are what `du` counts; on a compressing file system `df` gains less.
set -uo pipefail
apply=0 keep_hours=6 compile_days=3 scratch_days=7 use_gh=1 prs_file=""
while [ $# -gt 0 ]; do
  case "$1" in
    --apply) apply=1; shift ;;
    --keep-hours) keep_hours="${2:?--keep-hours N}"; shift 2 ;;
    --compile-days) compile_days="${2:?--compile-days N}"; shift 2 ;;
    --scratch-days) scratch_days="${2:?--scratch-days N}"; shift 2 ;;
    --no-gh) use_gh=0; shift ;;
    --prs) prs_file="${2:?--prs FILE}"; shift 2 ;;
    -h | --help) sed -n '2,29p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "machine_cleanup: unknown argument $1" >&2; exit 2 ;;
  esac
done
# The main checkout, also when this runs from a worktree.
common="$(git rev-parse --path-format=absolute --git-common-dir)" || exit 2
repo="$(dirname "$common")"
# shellcheck source=tools/cache_root.sh
. "$(dirname "${BASH_SOURCE[0]}")/cache_root.sh"
if [ "$apply" = 1 ]; then
  mkdir -p "$cache" && exec 9>"$cache/.machine_cleanup.lock" || exit 2
  flock -n 9 || { echo "machine_cleanup: already running (lock $cache/.machine_cleanup.lock)"; exit 0; }
fi
total_kb=0
since="$(date -d "-${keep_hours} hours" +%Y-%m-%dT%H:%M:%S)"

# Every folder a running process uses: its working folder, its executable's folder, its CARGO_TARGET_DIR.
busy=()
for p in /proc/[0-9]*; do
  for l in "$p/cwd" "$p/exe"; do
    t="$(readlink "$l" 2>/dev/null)" && [ -n "$t" ] && busy+=("$t")
  done
  t="$(tr '\0' '\n' 2>/dev/null < "$p/environ" | sed -n 's/^CARGO_TARGET_DIR=//p' | head -1)"
  [ -n "$t" ] && busy+=("$t")
done
# True when a running process uses DIR or a path below it.
in_use() {
  local d="${1%/}" b
  for b in "${busy[@]}"; do
    case "$b" in "$d" | "$d"/*) return 0 ;; esac
  done
  return 1
}
# True when anything below DIR (at most DEPTH levels down) was written after $since.
recent() { [ -n "$(find "$1" -maxdepth "${2:-3}" -newermt "$since" -print -quit 2>/dev/null)" ]; }
kb() { du -sk "$1" 2>/dev/null | cut -f1; }
dropped=()
finished_wts=()
# True when PATH is, or lies below, a folder dropped before (so its size is not counted twice).
under_dropped() {
  local d
  for d in "${dropped[@]}"; do
    case "$1" in "$d" | "$d"/*) return 0 ;; esac
  done
  return 1
}
# drop KIND PATH: delete (or list) PATH and add its size to the total.
drop() {
  under_dropped "$2" && return
  dropped+=("$2")
  local k; k="$(kb "$2")"; k="${k:-0}"
  total_kb=$((total_kb + k))
  if [ "$apply" = 1 ]; then
    rm -rf -- "$2" && echo "deleted       $1 $2 ($((k / 1024)) MB)"
  else
    echo "would delete  $1 $2 ($((k / 1024)) MB)"
  fi
}
keep() { echo "keep          $1 $2: $3"; }

echo "== worktrees of finished pull requests (${repo}/.worktrees)"
prs=""
if [ -n "$prs_file" ]; then
  git -C "$repo" fetch -q origin
  prs="$(cat "$prs_file")"
elif [ "$use_gh" = 1 ]; then
  git -C "$repo" fetch -q origin
  prs="$(gh pr list --state all -L 2000 --json headRefName,state,number \
    --jq 'group_by(.headRefName)[] | max_by(.number) | "\(.headRefName) \(.state)"' 2>/dev/null)" || prs=""
fi
[ "$use_gh" = 0 ] || [ -n "$prs" ] || echo "  no pull requests read: no worktree is removed"
while read -r w lk b; do
  case "$w" in "$repo"/.worktrees/?*) ;; *) continue ;; esac
  name="$(basename "$w")"
  [ "$lk" = 0 ] || { keep worktree "$name" "locked"; continue; }
  [ -n "$prs" ] || { keep worktree "$name" "pull requests not read"; continue; }
  state="$(awk -v b="$b" '$1 == b { print $2 }' <<<"$prs")"
  # Only a worktree whose pull request state was read, and is MERGED, CLOSED or absent, may lose its target/.
  case "$state" in MERGED | CLOSED | "") finished_wts+=("$w") ;; esac
  head="$(git -C "$w" rev-parse HEAD 2>/dev/null)" || { keep worktree "$name" "no HEAD"; continue; }
  case "$state" in
    MERGED | CLOSED) ;;
    # No pull request: only a worktree without commits of its own (HEAD on main) may go.
    "") git -C "$repo" merge-base --is-ancestor "$head" origin/main 2>/dev/null ||
          { keep worktree "$name" "no pull request and commits not on main"; continue; } ;;
    *) keep worktree "$name" "pull request $state"; continue ;;
  esac
  # Every commit must exist elsewhere: on main, or on the branch's remote branch.
  if ! git -C "$repo" merge-base --is-ancestor "$head" origin/main 2>/dev/null; then
    up="$(git -C "$w" rev-parse '@{u}' 2>/dev/null)"
    if [ -z "$up" ] || ! git -C "$repo" merge-base --is-ancestor "$head" "$up" 2>/dev/null; then
      keep worktree "$name" "commits that are on neither main nor its remote branch"; continue
    fi
  fi
  st="$(GIT_OPTIONAL_LOCKS=0 git -C "$w" status --porcelain --untracked-files=all --ignore-submodules=untracked 2>/dev/null)"
  [ -z "$st" ] || { keep worktree "$name" "uncommitted work, e.g. $(head -1 <<<"$st")"; continue; }
  # `git worktree remove` deletes the submodule's git folder: commits on no remote branch and stashes go with it.
  # A failing `git submodule foreach` keeps the worktree (checked through PIPESTATUS), nested submodules included.
  sub="$(git -C "$w" submodule foreach --quiet --recursive \
    'git status --porcelain --untracked-files=all; git log --branches --not --remotes -1 --format="unpushed commit %h"; git stash list' \
    2>/dev/null | head -1; exit "${PIPESTATUS[0]}")" || { keep worktree "$name" "submodules could not be checked"; continue; }
  [ -z "$sub" ] || { keep worktree "$name" "uncommitted or unpushed work in a submodule, e.g. $sub"; continue; }
  in_use "$w" && { keep worktree "$name" "a running process uses it"; continue; }
  recent "$w" 1 && { keep worktree "$name" "written in the last ${keep_hours} h"; continue; }
  k="$(kb "$w")"; total_kb=$((total_kb + ${k:-0}))
  if [ "$apply" = 1 ]; then
    git -C "$w" submodule deinit -f --all >/dev/null 2>&1
    if git -C "$repo" worktree remove --force "$w"; then
      echo "deleted       worktree $name (${state:-no PR}, $(( ${k:-0} / 1024 )) MB)"
      [ -n "$b" ] && git -C "$repo" branch -d "$b" >/dev/null 2>&1 && echo "              merged branch $b deleted"
    fi
  else
    echo "would delete  worktree $name (${state:-no PR}, $(( ${k:-0} / 1024 )) MB)"
  fi
done < <(git -C "$repo" worktree list --porcelain |
  awk '/^worktree /{w=$2; b=""; l=0} /^locked/{l=1} /^branch /{b=$2; sub("refs/heads/","",b)} /^$/{if(w!="") print w" "l" "b; w=""} END{if(w!="") print w" "l" "b}')
[ "$apply" = 1 ] && git -C "$repo" worktree prune

echo "== scratch folders untouched for ${scratch_days} days ($cache/scratch)"
if [ -d "$cache/scratch" ]; then
  while read -r d; do
    in_use "$d" && { keep scratch "$d" "a running process uses it"; continue; }
    [ -n "$(find "$d" -newermt "$(date -d "-${scratch_days} days" +%Y-%m-%dT%H:%M:%S)" -print -quit 2>/dev/null)" ] && continue
    drop scratch "$d"
  done < <(find "$cache/scratch" -mindepth 1 -maxdepth 1 -type d)
fi

echo "== idle cargo target folders"
# A cargo target folder holds CACHEDIR.TAG. The main checkout's own target/ is left to its user.
while read -r tag; do
  d="$(dirname "$tag")"
  [ "$d" = "$repo/target" ] && continue
  [ -d "$d" ] || continue
  in_use "$d" && { keep target "$d" "a running process uses it"; continue; }
  # A worktree's target/ goes only when its pull request state was read and is MERGED, CLOSED or absent (an open
  # pull request's head binary may be run there), and no process runs in the worktree (a cargo build there has the
  # worktree root as working folder).
  case "$d" in "$repo"/.worktrees/?*/target)
    wt="${d%/target}" fin=0
    for o in "${finished_wts[@]}"; do [ "$o" = "$wt" ] && fin=1; done
    [ "$fin" = 1 ] || { keep target "$d" "its pull request is not known to be finished"; continue; }
    in_use "$wt" && { keep target "$d" "a running process uses its worktree"; continue; } ;;
  esac
  recent "$d" 2 && { keep target "$d" "written in the last ${keep_hours} h"; continue; }
  drop target "$d"
done < <({ find "$repo"/.worktrees -mindepth 3 -maxdepth 3 -name CACHEDIR.TAG -path '*/target/*' 2>/dev/null
  find "$cache" -mindepth 2 -maxdepth 4 -name CACHEDIR.TAG -not -path "$cache/artifacts/*" 2>/dev/null; } | sort -u)

echo "== compile cache entries unused for ${compile_days} days ($cache/compile)"
if [ -d "$cache/compile" ]; then
  n=0 k=0
  while IFS=$'\t' read -r size f; do
    n=$((n + 1)); k=$((k + size / 1024))
    [ "$apply" = 1 ] && rm -f -- "$f"
  done < <(find "$cache/compile" -maxdepth 1 -type f ! -name '.last-prune' -mmin +"$((compile_days * 1440))" -printf '%s\t%p\n')
  total_kb=$((total_kb + k))
  echo "$([ "$apply" = 1 ] && echo "deleted     " || echo "would delete") $n entries ($((k / 1024)) MB)"
fi

echo "== total: $([ "$apply" = 1 ] && echo freed || echo would free) $((total_kb / 1048576)) GB" \
  "($((total_kb / 1024)) MB); $(df -h "$cache" | awk 'NR==2 {print $4" free on the cache disk"}')"
[ "$apply" = 1 ] || echo "dry run: nothing deleted; run again with --apply"
