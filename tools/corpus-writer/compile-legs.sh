#!/usr/bin/env bash
# Compile the corpus app with the AL compiler CI pins and the preprocessor symbols and runtime of each CI leg, before every push.
# The legs differ in preprocessor symbols and app runtime (and some code compiles on one runtime and not on another, for example a
# FlowField of type Integer over a BigInteger field: AL0427 under runtime 16.1 and 17.1 only). Each leg sets the app
# runtime the values corpus CI uses (27 -> 16.1, 28 -> 17.1; 29 keeps app.json's). One compiler serves every leg: the one the "test" job
# of the corpus ci.yml pins (al_tool_version).
# usage: CORPUS=~/corpus BCLINUX=~/bc-linux tools/corpus-writer/compile-legs.sh
# The preprocessor symbols of each leg come from `al-test-cli --print-symbols VERSION` for the leg version (bc_version) of
# its major in oracle/oracle.toml: al_version::corpus_symbols is the one owner of the rule (ADR-078 point 3). AL_RUNNER_ROOT
# (default: this repository) and AL_TEST_CLI (default: its target/release/al-test-cli, so build release first) override them.
# A failing al-test-cli is exit 2 before any leg.
# Needs the Microsoft symbols in $CORPUS/tests/al-language/.alpackages (the same ones the 29 compile uses). The fixture apps the
# corpus app depends on are the app_dirs of the "test" job in $CORPUS/.github/workflows/ci.yml (in that order, except tests/al-language):
# every leg compiles them first, with its own symbols and runtime, into a scratch copy of .alpackages. Packages that are in none of
# them (Microsoft's System Application Test Library and Permissions Mock, error AL1022 when missing) come from the folders in
# EXTRA_PACKAGES (colon-separated, *.app inside). Exit 2 when ci.yml has no al_tool_version or no app_dirs for the "test" job.
# Exit 1 when any leg or fixture has an error, or the compiler is missing.
set -uo pipefail
CORPUS=${CORPUS:?set CORPUS to the corpus clone}
BCLINUX=${BCLINUX:?set BCLINUX to the bc-linux clone}
APP=$CORPUS/tests/al-language
SCRATCH=""
restore() { mv -f "$APP/app.json.legs-bak" "$APP/app.json"; (cd "$CORPUS" && { git diff --name-only -- tests/al-language | grep -E '\.(rdl|rdlc|docx|xlsx|app|log)$' | xargs -r git checkout -q --; git ls-files --others --exclude-standard -- tests/al-language | grep -E '\.(rdl|rdlc|docx|xlsx)$' | xargs -r rm -f; }); }
cleanup() { restore; [ -n "$SCRATCH" ] && rm -rf "$SCRATCH"; }
cp "$APP/app.json" "$APP/app.json.legs-bak" || exit 2
trap cleanup EXIT
status=0
# CI facts, read from the "test" job of ci.yml (one owner): the compiler pin (al_tool_version, double-quoted, single-quoted or bare)
# and the app_dirs block. The pin is the newest compiler: it accepts runtime 16.1 and 17.1 too; install-al-compiler.sh without it picks
# the AL major of the BC version, which rejects the runtime of its leg (AL1043). A missing file, pin or app_dirs is an error.
CI_YML="$CORPUS/.github/workflows/ci.yml"
ci=$(python3 - "$CI_YML" <<'PY'
import re, sys
try:
    lines = open(sys.argv[1]).read().split("\n")
except OSError:
    lines = []
job = []
inside = False
for l in lines:
    if re.match(r"^  test:\s*$", l):
        inside = True
    elif inside and re.match(r"^  \S", l):
        break
    elif inside:
        job.append(l)
pin = ""
dirs = []
i = 0
while i < len(job):
    m = re.match(r"^(\s*)(al_tool_version|app_dirs):\s*(.*)$", job[i])
    if m:
        v = re.sub(r"\s+#.*$", "", m.group(3)).strip()
        if m.group(2) == "al_tool_version":
            pin = v[1:-1] if len(v) >= 2 and v[0] == v[-1] and v[0] in "\"'" else v
        else:
            ind = len(m.group(1))
            i += 1
            while i < len(job) and (job[i].strip() == "" or len(job[i]) - len(job[i].lstrip()) > ind):
                if job[i].strip():
                    dirs.append(job[i].strip())
                i += 1
            continue
    i += 1
if not pin:
    print("no al_tool_version in %s: cannot pick CI's compiler" % sys.argv[1])
    sys.exit(2)
dirs = [d for d in dirs if d != "tests/al-language"]
if not dirs:
    print("no app_dirs for the test job in %s: cannot know the fixture apps" % sys.argv[1])
    sys.exit(2)
print(pin)
print("\n".join(dirs))
PY
) || { echo "ERRORS: $ci"; exit 2; }
PIN=$(printf '%s\n' "$ci" | head -1)
mapfile -t FIXTURES < <(printf '%s\n' "$ci" | tail -n +2)
SCRATCH=$(mktemp -d "${TMPDIR:-/tmp}/legs.XXXXXX") || exit 2
# set_app_json FILE SYMBOLS RUNTIME ("-" keeps the runtime)
set_app_json() {
  python3 - "$1" "$2" "$3" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
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
ROOT=${AL_RUNNER_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}
CLI=${AL_TEST_CLI:-$ROOT/target/release/al-test-cli}
# One line per leg: major, leg version (oracle.toml bc_version), symbols, runtime ("-" keeps app.json's).
legs=()
for spec in "27 16.1" "28 17.1" "29 -"; do
  set -- $spec
  ver=$(python3 - "$ROOT/oracle/oracle.toml" "$1" <<'PY'
import sys, tomllib
try:
    with open(sys.argv[1], "rb") as f:
        versions = tomllib.load(f).get("version", [])
except (OSError, tomllib.TOMLDecodeError) as e:
    sys.exit("%s: %s" % (sys.argv[1], e))
v = [x.get("bc_version", "") for x in versions if str(x.get("major")) == sys.argv[2]]
if len(v) != 1 or not v[0]:
    sys.exit("%s: no bc_version for BC %s" % (sys.argv[1], sys.argv[2]))
print(v[0])
PY
) || { echo "ERRORS: no leg version for BC $1"; exit 2; }
  syms=$("$CLI" --root "$ROOT" --print-symbols "$ver" 2>"$SCRATCH/cli-err") && [ -n "$syms" ] \
    || { echo "ERRORS: al-test-cli --print-symbols $ver failed ($CLI):"; tail -5 "$SCRATCH/cli-err"; exit 2; }
  legs+=("$1 $ver $syms $2")
done
for leg in "${legs[@]}"; do
  set -- $leg
  # The compiler of the leg's major (one pin serves every leg); the leg's own version names it below.
  major=$1; shift
  inst=$(bash "$BCLINUX/scripts/install-al-compiler.sh" "$major.0" "$PIN" 2>"$SCRATCH/install-err")
  dir=$(printf '%s\n' "$inst" | sed -n 's/^AL_TOOL_DIR=//p')
  if [ -x "$dir/alc" ]; then alc=("$dir/alc"); elif [ -n "$dir" ] && [ -f "$dir/alc.dll" ]; then alc=(dotnet "$dir/alc.dll"); else
    echo "== BC $1 ($2): ERRORS"; echo "no AL compiler found for BC $1 (install-al-compiler.sh gave '$dir'): not checked"; tail -5 "$SCRATCH/install-err"; status=1; continue; fi
  cp "$APP/app.json.legs-bak" "$APP/app.json"
  set_app_json "$APP/app.json" "$2" "$3"
  # Package cache of this leg: the Microsoft symbols plus the fixture apps compiled by this leg's compiler.
  pk="$SCRATCH/pk-$1"; rm -rf "$pk"; mkdir -p "$pk"; cp "$APP"/.alpackages/*.app "$pk"/ 2>/dev/null
  errs=""
  IFS=: read -ra extra <<< "${EXTRA_PACKAGES:-}"
  for pd in "${extra[@]}"; do
    [ -n "$pd" ] || continue
    if compgen -G "$pd/*.app" >/dev/null; then cp "$pd"/*.app "$pk"/; else errs="EXTRA_PACKAGES folder '$pd' does not exist or holds no *.app"; break; fi
  done
  if [ -z "$errs" ]; then
    for fxp in "${FIXTURES[@]}"; do
      fx=$(basename "$fxp"); src="$CORPUS/$fxp"
      if [ ! -f "$src/app.json" ]; then errs="fixture app $src not found"; break; fi
      cp -r "$src" "$SCRATCH/$1-$fx"; set_app_json "$SCRATCH/$1-$fx/app.json" "$2" "$3"
      # A stale copy of this fixture (same publisher and name, other file name) in .alpackages must not shadow the one built here.
      stale=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print("%s_%s_" % (d["publisher"], d["name"]))' "$SCRATCH/$1-$fx/app.json" 2>/dev/null)
      [ -n "$stale" ] && rm -f "$pk/$stale"*
      fxout=$(cd "$SCRATCH/$1-$fx" && "${alc[@]}" /project:. /packagecachepath:"$pk" /out:"$pk/$fx.app" 2>&1); fxcode=$?
      fxerrs=$(error_lines "$fxout" "$fxcode")
      if [ -n "$fxerrs" ]; then errs=$(printf 'fixture app %s:\n%s' "$fx" "$fxerrs"); break; fi
    done
  fi
  if [ -z "$errs" ]; then
    out=$(cd "$APP" && "${alc[@]}" /project:. /packagecachepath:"$pk" /out:"$SCRATCH/legs-$1.app" 2>&1); code=$?
    errs=$(error_lines "$out" "$code")
  fi
  if [ -n "$errs" ]; then echo "== BC $1 ($2): ERRORS"; printf '%s\n' "$errs" | head -20; status=1; else echo "== BC $1 ($2): ok"; fi
done
exit $status
