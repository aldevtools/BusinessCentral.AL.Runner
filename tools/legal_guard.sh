#!/usr/bin/env bash
# Fails when a tracked file holds content that must not be committed (CLAUDE.md hard rule 7):
#
#   binary     a *.dll, *.app, *.nupkg or *.Source.zip file
#   copyright  a Microsoft copyright or license header
#   marker     the output of a .NET decompiler
#   phrase     a licensing claim (what is true: it runs without a BC service tier,
#              Docker or SQL Server)
#
# Usage: tools/legal_guard.sh [ROOT]   (default: the repository that holds this script)
#
# Only tracked files are looked at; submodules (the corpus) are not. A path that must stay is
# listed in ROOT/tools/legal_guard_allowlist.txt as "<check> <path>", one per line, with a reason
# in a comment above it. An entry excuses its path from that one check only.
# The tests are in crates/al-test-cli/tests/legal_guard.rs. The patterns are written so that this
# file does not match them itself.
set -euo pipefail

root=${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"}
allowlist="$root/tools/legal_guard_allowlist.txt"

allowed() { # check path
    [[ -f $allowlist ]] && grep -qxF -- "$1 $2" <(sed -e 's/#.*//' -e 's/[[:space:]]*$//' "$allowlist")
}

fail=0
report() { # check path what
    if ! allowed "$1" "$2"; then
        echo "legal_guard: $1: $2: $3" >&2
        fail=1
    fi
}

# Binaries and packages, by name.
while IFS= read -r -d '' path; do
    report binary "$path" "binary or package file"
done < <(git -C "$root" ls-files -z | grep -z -i -E '\.(dll|app|nupkg)$|\.source\.zip$' || true)

# Content checks. git grep looks at the tracked files of the work tree and skips submodules.
scan() { # check what pattern
    local path
    while IFS= read -r -d '' path; do
        report "$1" "$path" "$2"
    done < <(git -C "$root" grep -z -l -I -i -E -e "$3" || true)
}

scan copyright "Microsoft copyright or license header" \
    'Copyright (\(c\)|©) Micro[s]oft Corporation|Licensed under the MIT License\. See Lic[e]nse\.txt in the project root'
scan marker "decompiler marker" \
    'Decompiled[ ]with|ICSharpCode[.]Decompiler|(^|[^A-Za-z0-9+/])(IL[S]py|dn[S]py|dot[P]eek)'
scan phrase "license claim (say: without a BC service tier, Docker or SQL Server)" \
    'without (a |any )?((BC|Business Central) )?licen[cs]e([^a-z]|$)|no (BC|Business Central) licen[cs]e|(and|needs?|or) (no|a) licen[cs]e([^a-z]|$)'

exit $fail
