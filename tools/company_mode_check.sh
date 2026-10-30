#!/usr/bin/env bash
# ADR-046 step B and amendment 4: the default mode models a database with one company (owner 2026-10-30: decision 4c
# no longer refuses there). This check runs the corpus twice, with the opt-in and with one company (`--one-company`), and
# compares the JUnit files. A case may differ only by being not run in the default mode, or by failing there where the
# opt-in passes: the corpus database has two companies, and a case that counts them fails in a one-company database. A
# case that passes in the default mode but not under the opt-in, or runs in the default mode but is not run under the
# opt-in, is an error.
#
# The opt-in is `companies` of oracle/oracle.toml; while that key is absent, the companies of the corpus CI database
# (install/lt-bf-install-probes/BfInstallTests.al:47-64), or COMPANIES="A,B" from the environment.
#
#   tools/company_mode_check.sh compare DIR   compare the files in DIR
#   tools/company_mode_check.sh run [DIR]     make the runs, then compare
#
# File names in DIR, which is what `al-test-cli --all-versions --junit` writes:
#   opt-in:  companies-junit-<v>.xml   companies-junit-<suite>-<v>.xml
#   default: one-junit-<v>.xml         one-junit-<suite>-<v>.xml
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"

compare() {
  local dir="$1" status=0 n=0 f base partner
  for f in "$dir"/companies-junit-*.xml; do
    [ -e "$f" ] || continue
    base="$(basename "$f" .xml)"
    partner="$dir/one-junit-${base#companies-junit-}.xml"
    echo "== ${base#companies-junit-}"
    if [ ! -f "$partner" ]; then
      echo "MISSING: $partner"
      status=1
      continue
    fi
    python3 - "$f" "$partner" <<'PY' || status=1
import sys
import xml.etree.ElementTree as ET
def outcomes(path):
    out = {}
    for case in ET.parse(path).getroot().iter("testcase"):
        key = f"{case.get('classname')}::{case.get('name')}"
        if case.find("failure") is not None or case.find("error") is not None:
            e = case.find("failure") if case.find("failure") is not None else case.find("error")
            out[key] = ("fail", e.get("message", ""))
        elif case.find("skipped") is not None:
            out[key] = ("not run", case.find("skipped").get("message", ""))
        else:
            out[key] = ("pass", "")
    return out
opt, one = outcomes(sys.argv[1]), outcomes(sys.argv[2])
bad = allowed = 0
for key in sorted(set(opt) | set(one)):
    o, d = opt.get(key, ("missing", "")), one.get(key, ("missing", ""))
    if o[0] == d[0]:
        continue
    if d[0] == "not run" or (d[0] == "fail" and o[0] == "pass"):
        allowed += 1
        continue
    bad += 1
    print(f"DIFFERS: {key}\n  --companies: {o[0]} {o[1][:300]}\n  one company: {d[0]} {d[1][:300]}")
print(f"{len(opt)} tests compared, {allowed} not run or failing in the default mode only, {bad} error(s)")
sys.exit(1 if bad else 0)
PY
    n=$((n + 1))
  done
  echo "compared $n pairs of JUnit files"
  [ "$n" -gt 0 ] || status=1
  return $status
}

run() {
  local dir="$1" bin="$root/target/release/al-test-cli" status=0 companies=()
  [ -x "$bin" ] || { echo "build first: cargo build --release -p al-test-cli" >&2; return 2; }
  mkdir -p "$dir"
  rm -f "$dir"/companies-junit*.xml "$dir"/one-junit*.xml
  if ! grep -q '^companies *=' "$root/oracle/oracle.toml"; then
    companies=(--companies "${COMPANIES:-CRONUS International Ltd.,My Company}")
  fi
  local cap=()
  command -v systemd-run >/dev/null && cap=(systemd-run --user --scope -q --slice=al-work.slice -p MemoryMax=3G)
  (cd "$dir" && "${cap[@]}" "$bin" --root "$root" --all-versions --require-dotnet --no-ratchet "${companies[@]}" \
    --junit companies-junit.xml > companies.log 2>&1) || { echo "opt-in run failed, see $dir/companies.log" >&2; status=1; }
  (cd "$dir" && "${cap[@]}" "$bin" --root "$root" --all-versions --require-dotnet --no-ratchet --one-company \
    --junit one-junit.xml > one.log 2>&1) || { echo "one-company run failed, see $dir/one.log" >&2; status=1; }
  compare "$dir" || status=1
  return $status
}

case "${1:-}" in
  compare) compare "${2:?usage: company_mode_check.sh compare DIR}" ;;
  run) run "${2:-$root/target/company-mode-check}" ;;
  *) sed -n '2,18p' "$0"; exit 2 ;;
esac
