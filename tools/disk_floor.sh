#!/usr/bin/env bash
# Stops a heavy step before it fills the disk. Prints the free space of the disk that holds the cache (and the
# repository, when that is another disk) and exits 1 when one has less than the floor.
#
#   tools/disk_floor.sh [GB]     the floor in GB (default 40: a cold release build plus a debug test build of the
#                                biggest crates, about 15 GB, for two agents at once, plus room for run output)
#
# Run it before `cargo build` in a new worktree and before a full Microsoft run. Exit 1: do not start; free space
# with tools/machine_cleanup.sh (dry run first) or ask the person who runs the machine.
set -uo pipefail
floor="${1:-40}"
cache="${ALRUNNER_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/al-runtime}"
root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
mkdir -p "$cache"
rc=0
for d in $(printf '%s\n' "$cache" "$root" | xargs -n1 df --output=target 2>/dev/null | grep -v '^Mounted' | sort -u); do
  free_gb=$(df -BG --output=avail "$d" | tail -1 | tr -dc '0-9')
  if [ "$free_gb" -lt "$floor" ]; then
    echo "disk_floor: $d has ${free_gb} GB free, under the floor of ${floor} GB: do not start a build or run" >&2
    rc=1
  else
    echo "disk_floor: $d has ${free_gb} GB free (floor ${floor} GB)"
  fi
done
exit $rc
