#!/usr/bin/env bash
# The fast checks to run before every push: the ones that kept failing in CI after a green local
# test run. With a warm target folder it takes under a minute (a cold one: about 4 minutes); it does
# not replace check.sh before a commit that is meant to merge.
#
#   1. cargo fmt --check over the workspace.
#   2. al-version: no BC version literal outside the gates (no_version_literals), and every
#      `#if BCnnPLUS` corpus file cited by a gate (gate_completeness).
#   3. al-builtins: the assumption registry tests, and builtin_ids.rs up to date.
#   4. file_size_ratchet: no source file past 3,000 lines or its cap in metrics/file-sizes.toml.
#   5. clippy -D warnings on the crates the branch changed (all targets). A change to a workspace
#      file (Cargo.toml, Cargo.lock, rustfmt.toml, clippy.toml, .cargo/) runs it on the workspace.
#
# The changed files are those of `git diff <base>` (committed and uncommitted) plus untracked files;
# <base> is the merge base with origin/main unless --base REF is given.
#
# Usage: tools/prepush.sh [--base REF] [--dry-run] [--files FILE|-]
#   --dry-run   print the commands, one per line, and run nothing.
#   --files F   read the changed files from F (- for stdin) instead of git.
# It installs no git hook. To run it on every push yourself:
#   printf '#!/bin/sh\nexec tools/prepush.sh\n' > .git/hooks/pre-push && chmod +x .git/hooks/pre-push
set -euo pipefail
cd "$(dirname "$0")/.."

base=""
dry=0
files=""
while [ $# -gt 0 ]; do
    case "$1" in
        --base) base="$2"; shift 2 ;;
        --dry-run) dry=1; shift ;;
        --files) files="$2"; shift 2 ;;
        -h|--help) sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "prepush.sh: unknown argument $1" >&2; exit 2 ;;
    esac
done

changed_files() {
    if [ -n "$files" ]; then
        if [ "$files" = "-" ]; then cat; else cat "$files"; fi
        return
    fi
    if [ -z "$base" ]; then
        base=$(git merge-base origin/main HEAD)
    fi
    git diff --name-only "$base"
    git ls-files --others --exclude-standard
}

# The clippy selection: "--workspace", the -p list of changed crates, or nothing.
clippy_selection() {
    local whole=0
    local -a pkgs=()
    local f dir name
    while IFS= read -r f; do
        [ -n "$f" ] || continue
        case "$f" in
            Cargo.toml|Cargo.lock|rustfmt.toml|clippy.toml|.cargo/*) whole=1 ;;
            crates/*/*)
                dir=${f#crates/}
                dir=crates/${dir%%/*}
                [ -f "$dir/Cargo.toml" ] || continue
                name=$(sed -n 's/^name *= *"\(.*\)"/\1/p' "$dir/Cargo.toml" | head -n 1)
                pkgs+=("$name")
                ;;
        esac
    done
    if [ "$whole" = 1 ]; then
        echo "--workspace"
    elif [ ${#pkgs[@]} -gt 0 ]; then
        printf '%s\n' "${pkgs[@]}" | sort -u | sed 's/^/-p /' | tr '\n' ' ' | sed 's/ $//'
        echo
    fi
}

sel=$(changed_files | clippy_selection)

cmds=(
    "cargo fmt --all --check"
    "cargo test -p al-version --test no_version_literals --test gate_completeness -q"
    "cargo test -p al-builtins --lib -q assumptions"
    "cargo test -p al-builtins --test builtin_ids_current -q"
    "cargo test -p al-test-cli --test file_size_ratchet -q"
)
if [ -n "$sel" ]; then
    cmds+=("cargo clippy $sel --all-targets -- -D warnings")
fi

if [ "$dry" = 1 ]; then
    printf '%s\n' "${cmds[@]}"
    exit 0
fi

for c in "${cmds[@]}"; do
    echo "prepush: $c" >&2
    start=$SECONDS
    if ! $c; then
        echo "prepush: FAILED: $c" >&2
        exit 1
    fi
    echo "prepush: ok in $((SECONDS - start)) s" >&2
done
echo "prepush: all checks passed (run ./check.sh before a commit that is meant to merge)" >&2
