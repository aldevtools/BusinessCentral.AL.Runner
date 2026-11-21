#!/usr/bin/env bash
# Stops a heavy step before it fills the disk. Prints the free space of the disk that holds the cache (and the
# repository, when that is another disk) and exits 1 when one has less than the floor, when the floor is not a
# number, or when the free space cannot be read. It exits 2 when tools/cache_root.sh refuses the cache root.
#
#   tools/disk_floor.sh [GB]     the floor in GB (default 40: a cold release build plus a debug test build of the
#                                biggest crates, about 15 GB, for two agents at once, plus room for run output)
#
# Run it before `cargo build` in a new worktree and before a full Microsoft run. Exit 1 or 2: do not start; free space
# with tools/machine_cleanup.sh (dry run first) or ask the person who runs the machine.
set -uo pipefail
floor="${1:-40}"
case "$floor" in '' | *[!0-9]*) echo "disk_floor: the floor must be a number of GB, got '$floor'" >&2; exit 1 ;; esac
# shellcheck source=tools/cache_root.sh
. "$(dirname "${BASH_SOURCE[0]}")/cache_root.sh"
root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
mkdir -p "$cache"
rc=0
mounts=""
for p in "$cache" "$root"; do
  m="$(df --output=target -- "$p" 2>/dev/null | tail -n +2)"
  if [ -n "$m" ]; then mounts+="$m"$'\n'; else echo "disk_floor: cannot read the disk of $p" >&2; rc=1; fi
done
while IFS= read -r d; do
  [ -n "$d" ] || continue
  free_gb=$(df -BG --output=avail -- "$d" 2>/dev/null | tail -n 1 | tr -dc '0-9')
  if [ -z "$free_gb" ]; then
    echo "disk_floor: cannot read the free space of $d" >&2
    rc=1
  elif [ "$free_gb" -lt "$floor" ]; then
    echo "disk_floor: $d has ${free_gb} GB free, under the floor of ${floor} GB: do not start a build or run" >&2
    rc=1
  else
    echo "disk_floor: $d has ${free_gb} GB free (floor ${floor} GB)"
  fi
done < <(printf '%s' "$mounts" | sort -u)
exit $rc
