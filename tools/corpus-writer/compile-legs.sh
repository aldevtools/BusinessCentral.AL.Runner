#!/usr/bin/env bash
# Compile the corpus app with the AL compiler and preprocessor symbols of each CI leg group, before every push.
# The legs differ in preprocessor symbols and app runtime (and some code compiles on one runtime and not on another, for example a
# FlowField of type Integer over a BigInteger field: AL0427 under runtime 16.1 and 17.1 only). Each leg sets the app
# runtime the values corpus CI uses (27 -> 16.1, 28 -> 17.1; 29 keeps app.json's), with the compiler CI pins (al_tool_version).
# usage: CORPUS=~/corpus BCLINUX=~/bc-linux tools/corpus-writer/compile-legs.sh
# Needs the Microsoft symbols in $CORPUS/tests/al-language/.alpackages (the same ones the 29 compile uses). The fixture apps the
# corpus app depends on (internals fixture, cross-app A, D, B, C) are not in there: every leg compiles them first, with its own
# compiler and runtime, into a scratch copy of .alpackages. Packages that are in none of them (Microsoft's System Application Test
# Library and Permissions Mock, error AL1022 when missing) come from the folders in EXTRA_PACKAGES (colon-separated, *.app inside). Exit 1 when any leg or fixture has an error, or a compiler is missing.
set -uo pipefail
CORPUS=${CORPUS:?set CORPUS to the corpus clone}
BCLINUX=${BCLINUX:?set BCLINUX to the bc-linux clone}
APP=$CORPUS/tests/al-language
cp "$APP/app.json" "$APP/app.json.legs-bak"
restore() { mv -f "$APP/app.json.legs-bak" "$APP/app.json"; (cd "$CORPUS" && { git diff --name-only -- tests/al-language | grep -E '\.(rdl|rdlc|docx|xlsx|app|log)$' | xargs -r git checkout -q --; git ls-files --others --exclude-standard -- tests/al-language | grep -E '\.(rdl|rdlc|docx|xlsx)$' | xargs -r rm -f; }); }
status=0
FIXTURES="al-language-internals-fixture al-language-xapp-a al-language-xapp-d al-language-xapp-b al-language-xapp-c"  # publish order of ci.yml
# CI pins one compiler for every leg (al_tool_version in ci.yml, the newest one: it accepts runtime 16.1 and 17.1 too). Without the
# pin, install-al-compiler.sh picks the AL major of the BC version (16.2 for 27, 17.0 for 28), which rejects the runtime of its leg (AL1043).
PIN=$(sed -n 's/^[[:space:]]*al_tool_version:[[:space:]]*"\([^"]*\)".*/\1/p' "$CORPUS/.github/workflows/ci.yml" 2>/dev/null | head -1)
[ -n "$PIN" ] || echo "warning: no al_tool_version in $CORPUS/.github/workflows/ci.yml; each leg uses the compiler of its BC major, which may reject the leg's runtime (AL1043)" >&2
SCRATCH=$(mktemp -d "${TMPDIR:-/tmp}/legs.XXXXXX")
cleanup() { restore; rm -rf "$SCRATCH"; }
trap cleanup EXIT
# set_app_json FILE SYMBOLS RUNTIME ("-" keeps the runtime; SYMBOLS "-" keeps the symbols)
set_app_json() {
  python3 - "$1" "$2" "$3" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
if sys.argv[2] != "-":
    d["preprocessorSymbols"] = sys.argv[2].split(",")
if sys.argv[3] != "-":
    d["runtime"] = sys.argv[3]
json.dump(d, open(sys.argv[1], "w"), indent=2)
PY
}
# error_lines OUTPUT CODE: every error line, also one without a source location (`error AL1022`: a package is missing).
error_lines() {
  local errs
  errs=$(printf '%s\n' "$1" | grep -E '(^|[: ])error [A-Z]+[0-9]+' || true)
  if [ -z "$errs" ] && [ "$2" -ne 0 ]; then errs=$(printf 'alc exited with %s:\n%s' "$2" "$(printf '%s\n' "$1" | tail -5)"); fi
  printf '%s' "$errs"
}
for leg in "27.0 BC27PLUS 16.1" "28.0 BC27PLUS,BC28PLUS 17.1" "29.0 BC27PLUS,BC28PLUS,BC29PLUS -"; do
  set -- $leg
  dir=$(bash "$BCLINUX/scripts/install-al-compiler.sh" "$1" $PIN 2>/dev/null | sed -n 's/^AL_TOOL_DIR=//p')
  if [ -x "$dir/alc" ]; then alc=("$dir/alc"); elif [ -n "$dir" ] && [ -f "$dir/alc.dll" ]; then alc=(dotnet "$dir/alc.dll"); else
    echo "== BC $1 ($2): ERRORS"; echo "no AL compiler found for BC $1 (install-al-compiler.sh gave '$dir'): not checked"; status=1; continue; fi
  cp "$APP/app.json.legs-bak" "$APP/app.json"
  set_app_json "$APP/app.json" "$2" "$3"
  # Package cache of this leg: the Microsoft symbols plus the fixture apps compiled by this leg's compiler.
  pk="$SCRATCH/pk-$1"; rm -rf "$pk"; mkdir -p "$pk"; cp "$APP"/.alpackages/*.app "$pk"/ 2>/dev/null
  for pd in $(printf %s "${EXTRA_PACKAGES:-}" | tr : " "); do cp "$pd"/*.app "$pk"/ 2>/dev/null; done
  errs=""
  for fx in $FIXTURES; do
    src="$CORPUS/tests/$fx"
    if [ ! -f "$src/app.json" ]; then errs="fixture app $src not found"; break; fi
    cp -r "$src" "$SCRATCH/$1-$fx"; set_app_json "$SCRATCH/$1-$fx/app.json" - "$3"
    fxout=$(cd "$SCRATCH/$1-$fx" && "${alc[@]}" /project:. /packagecachepath:"$pk" /out:"$pk/$fx.app" 2>&1); fxcode=$?
    fxerrs=$(error_lines "$fxout" "$fxcode")
    if [ -n "$fxerrs" ]; then errs=$(printf 'fixture app %s:\n%s' "$fx" "$fxerrs"); break; fi
  done
  if [ -z "$errs" ]; then
    out=$(cd "$APP" && "${alc[@]}" /project:. /packagecachepath:"$pk" /out:"$SCRATCH/legs-$1.app" 2>&1); code=$?
    errs=$(error_lines "$out" "$code")
  fi
  if [ -n "$errs" ]; then echo "== BC $1 ($2): ERRORS"; printf '%s\n' "$errs" | head -20; status=1; else echo "== BC $1 ($2): ok"; fi
done
exit $status
