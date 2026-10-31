#!/usr/bin/env bash
# Shows that two branches that each change the assumption registry merge without a conflict.
#
# Copies the registry (crates/al-builtins/src/assumptions.rs and crates/al-builtins/src/assumptions/) of the
# checkout ROOT into a throwaway git repository, then makes two branches from it and edits each the way
# docs/agents/brief.md says an agent adds or removes an entry:
#   1. both branches add an entry in the same area (ids next to each other);
#   2. one branch removes the last entry, the other adds one.
# Each pair is merged with plain git (no merge drivers: GitHub's merges ignore .gitattributes). The table in
# docs/probes-needed.md is not part of it: feature branches leave it to the regen bot.
# Exit 0 when every merge is clean, 1 otherwise.
#
# Usage: tools/assumption_merge_check.sh [ROOT]   (ROOT defaults to the checkout this script is in)
set -euo pipefail

root=$(cd "${1:-$(dirname "$0")/..}" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
repo="$work/repo"
mkdir -p "$repo"

rs=crates/al-builtins/src/assumptions.rs
dir=crates/al-builtins/src/assumptions

for p in "$rs" "$dir"; do
    if [ -e "$root/$p" ]; then
        mkdir -p "$repo/$(dirname "$p")"
        cp -r "$root/$p" "$repo/$p"
    fi
done

g() { git -C "$repo" -c user.name=t -c user.email=t@t -c commit.gpgsign=false "$@"; }
g init -q -b main
g add -A
g commit -qm base

# The registry's layout: one file per entry, or every entry in assumptions.rs.
per_file() { [ -d "$repo/$dir" ]; }

# add ID: adds an entry as an agent does.
add() {
    local id=$1 variant
    variant=$(echo "$id" | sed -E 's/(^|-)([a-z])/\U\2/g')
    local entry="$variant => {
    id: \"$id\",
    assumed: \"Assumed behavior of $id.\",
    tests: \"Microsoft tests that need $id.\",
    probe: \"The probe $id wants.\",
}"
    if per_file; then
        printf '%s\n' "$entry" >"$repo/$dir/$id.rs"
    else
        # At the end of `registry! { ... }`, before its closing `}`.
        python3 - "$repo/$rs" "$entry," <<'PY2'
import sys
rs, entry = sys.argv[1:3]
t = open(rs).read()
start = t.index("registry! {")
end = t.index("\n}\n", start)
t = t[:end] + "\n" + "\n".join("    " + l if l else l for l in entry.split("\n")) + t[end:]
open(rs, "w").write(t)
PY2
    fi
}

# remove_last: removes the last entry of the registry.
remove_last() {
    if per_file; then
        local last
        last=$(cd "$repo/$dir" && ls ./*.rs | sort | tail -n 1)
        rm "$repo/$dir/${last#./}"
    else
        python3 - "$repo/$rs" <<'PY2'
import sys
rs = sys.argv[1]
t = open(rs).read()
start = t.index("registry! {")
end = t.index("\n}\n", start)
body = t[start:end]
# The last entry starts at the line that opens its variant (`    Name => {`).
cut = t.rindex("\n", 0, start + body.rindex(" => {", 0, body.rindex("id: \"")))
open(rs, "w").write(t[:cut] + t[end:])
PY2
    fi
}

fails=0
# scenario NAME EDIT_A EDIT_B: two branches from main, merged into a third.
scenario() {
    local name=$1 a=$2 b=$3
    g checkout -q -B "a-$name" main
    eval "$a"
    g add -A
    g commit -qm "a $name"
    g checkout -q -B "b-$name" main
    eval "$b"
    g add -A
    g commit -qm "b $name"
    if g merge -q --no-edit "a-$name" >/dev/null 2>&1; then
        echo "clean: $name"
    else
        echo "CONFLICT: $name"
        g diff --name-only --diff-filter=U | sed 's/^/  conflicted: /'
        g merge --abort
        fails=1
    fi
}

scenario two-adds-same-area "add page-zz-first" "add page-zz-second"
scenario remove-and-add "remove_last" "add page-zz-third"

exit "$fails"
