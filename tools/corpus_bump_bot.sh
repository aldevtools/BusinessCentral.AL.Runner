#!/usr/bin/env bash
# The corpus bump bot (.github/workflows/corpus-bump-bot.yml): the mechanical part of a corpus bump, so the pin
# follows the corpus branch master without a person starting it. Three steps, each usable on its own:
#
#   tools/corpus_bump_bot.sh find
#       Prints `run=ID` and `sha=COMMIT` of the newest green corpus CI run whose commit is on master after the
#       pin and passes the checks of tools/corpus_bump.sh (--dry-run). Prints nothing when there is none.
#   tools/corpus_bump_bot.sh bump RUN OUT
#       In the checkout this script belongs to: the corpus outcomes on the old pin, then tools/corpus_bump.sh RUN
#       without baselines (pin, oracle, harness counts), the outcomes on the new pin (--baseline-diff), the
#       awaiting assumptions, the gate, harness and golden-file tests. Writes the pull request body to OUT/body.md
#       (tools/corpus_bump_report.py) and commits with `commit`. What needs judgment does not stop it: the
#       body lists it. It exits nonzero only when the bump itself or the commit is refused.
#   tools/corpus_bump_bot.sh commit REPO SHA RUN
#       Commits the bump in REPO: only the corpus submodule (at SHA), oracle/ and the harness counts. Any other
#       changed path is refused, a baseline (metrics/) above all: the bot never writes one (the regen bot adds
#       new passes after the merge; a removal is a human decision, CLAUDE.md hard rule 2).
#
# It never pushes and never opens a pull request; the workflow does. For tests: CORPUS_BUMP_GH,
# CORPUS_BUMP_CORPUS and CORPUS_BUMP_NO_FETCH as for tools/corpus_bump.sh, CORPUS_BUMP_BOT_PIN (the pin),
# CORPUS_BUMP_BOT_NAME and CORPUS_BUMP_BOT_EMAIL (the commit identity, default the github-actions bot).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(dirname "$here")"
gh="${CORPUS_BUMP_GH:-gh}"
corpus="${CORPUS_BUMP_CORPUS:-$root/corpus}"
repo="aldevtools/BusinessCentral.AL.Language.Tests"
workflow="AL Language Test Suite"
harness="crates/al-test-cli/tests/corpus_harness.rs"

die() { echo "corpus bump bot: refused: $*" >&2; exit 1; }
usage() { echo "usage: tools/corpus_bump_bot.sh find | bump RUN OUT | commit REPO SHA RUN" >&2; exit 2; }

pin_of() { # repository
  if [ -n "${CORPUS_BUMP_BOT_PIN:-}" ]; then echo "$CORPUS_BUMP_BOT_PIN"; return; fi
  git -C "$1" ls-tree HEAD corpus | awk '{print $3}'
}

find_run() {
  local pin ref runs id sha
  pin="$(pin_of "$root")"
  [[ "$pin" =~ ^[0-9a-f]{40}$ ]] || die "no corpus pin"
  if [ "${CORPUS_BUMP_NO_FETCH:-0}" != 1 ]; then
    git -C "$corpus" fetch -q origin master
    ref=origin/master
  else
    ref=master
  fi
  if [ "$(git -C "$corpus" rev-parse "$ref")" = "$pin" ]; then
    echo "the pin ${pin:0:12} is corpus master" >&2
    return 0
  fi
  # Only push runs on master: a pull request run tests only the changed files (run 41182315237 had 27 cases where
  # the oracle had 19831), and its commit can still pass the checks of corpus_bump.sh.
  runs="$("$gh" run list -R "$repo" --workflow "$workflow" --event push --branch master --status success -L 40 \
    --json databaseId,headSha)"
  # Newest first, as gh lists them.
  while read -r id sha; do
    [ -n "$id" ] || continue
    if [ "$sha" = "$pin" ] || ! git -C "$corpus" cat-file -e "$sha^{commit}" 2>/dev/null \
      || ! git -C "$corpus" merge-base --is-ancestor "$pin" "$sha" \
      || ! git -C "$corpus" merge-base --is-ancestor "$sha" "$ref"; then
      echo "run $id: commit ${sha:0:12} is not on master after the pin, skipped" >&2
      continue
    fi
    if CORPUS_BUMP_NO_FETCH=1 CORPUS_BUMP_CORPUS="$corpus" CORPUS_BUMP_GH="$gh" \
      bash "$here/corpus_bump.sh" --dry-run "$id" >&2; then
      printf 'run=%s\nsha=%s\n' "$id" "$sha"
      return 0
    fi
    echo "run $id: refused by the bump checks, skipped" >&2
  done < <(python3 -c '
import json, sys
for r in json.loads(sys.argv[1]):
    print(r["databaseId"], r["headSha"])' "$runs")
  echo "no green corpus run after the pin ${pin:0:12}" >&2
}

# The number of test cases of a JUnit file (`-` reads standard input).
junit_cases() {
  python3 -c '
import sys, xml.etree.ElementTree as ET
src = sys.stdin if sys.argv[1] == "-" else sys.argv[1]
print(sum(1 for _ in ET.parse(src).iter("testcase")))' "$1"
}

# The paths a bump may change, and why any other is refused.
allowed() {
  case "$1" in
    corpus | oracle/* | "$harness") return 0 ;;
  esac
  return 1
}

commit_bump() { # repository, commit, run
  local r="$1" sha="$2" run="$3" bad=0 path
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || die "commit $sha is not a full commit id"
  while IFS= read -r path; do
    [ -n "$path" ] || continue
    case "$path" in
      metrics/*) echo "corpus bump bot: $path changed: the bot never writes a baseline (the regen bot adds new passes after the merge; a removal is a human decision)" >&2; bad=1 ;;
      *) allowed "$path" || { echo "corpus bump bot: $path is not part of a corpus bump" >&2; bad=1; } ;;
    esac
  done < <(git -C "$r" status --porcelain=v1 -uall --ignore-submodules=none | cut -c4-)
  [ "$bad" = 0 ] || die "paths outside the bump changed; nothing committed"
  # Never weaken an oracle (CLAUDE.md hard rule 2): no JUnit file of oracle/ may lose cases.
  local f was is
  while IFS= read -r f; do
    [ -n "$f" ] || continue
    was="$(git -C "$r" show "HEAD:$f" 2>/dev/null | junit_cases -)" || was=0
    is="$(junit_cases "$r/$f" 2>/dev/null)" || is=0
    if [ "$is" -lt "$was" ]; then
      echo "corpus bump bot: $f has $is cases, $was before: an oracle never shrinks in a bump" >&2
      bad=1
    fi
  done < <(git -C "$r" ls-files -- 'oracle/*.xml')
  [ "$bad" = 0 ] || die "an oracle file lost cases; nothing committed"
  local now
  now="$(git -C "$r/corpus" rev-parse HEAD)"
  [ "$now" = "$sha" ] || die "the corpus submodule is at ${now:0:12}, not at the target ${sha:0:12}; nothing committed"
  grep -q "^corpus_commit = \"$sha\"" "$r/oracle/oracle.toml" || die "oracle/oracle.toml does not pin ${sha:0:12}; nothing committed"
  git -C "$r" add -- corpus oracle
  [ ! -e "$r/$harness" ] || git -C "$r" add -- "$harness"
  if git -C "$r" diff --cached --quiet; then
    echo "corpus bump bot: nothing to commit" >&2
    return 0
  fi
  git -C "$r" -c user.name="${CORPUS_BUMP_BOT_NAME:-github-actions[bot]}" \
    -c user.email="${CORPUS_BUMP_BOT_EMAIL:-41898282+github-actions[bot]@users.noreply.github.com}" \
    commit -q -m "Corpus bump to master ${sha:0:12} (bump bot)" \
    -m "Pin, oracle and harness counts from corpus run $run, made by tools/corpus_bump_bot.sh. No baseline was written: the regen bot adds the new passes after the merge."
  echo "committed $(git -C "$r" rev-parse --short HEAD)"
}

# Runs a command, keeps its output in OUT/NAME.txt and its exit code in OUT/NAME.code; never fails.
keep() { # name, command...
  local name="$1" code=0
  shift
  "$@" > "$out/$name.txt" 2>&1 || code=$?
  echo "$code" > "$out/$name.code"
  echo "$name: exit $code"
}

bump() { # run, out
  local run="$1" sha old
  out="$2"
  mkdir -p "$out/before" "$out/after"
  cd "$root"
  [ -z "$(git status --porcelain=v1 -uall --ignore-submodules=none)" ] || die "the checkout has changes"
  old="$(pin_of "$root")"
  cargo build --release -q -p al-test-cli
  # The outcomes on the old pin, to tell new failures from old ones.
  keep before ./target/release/al-test-cli --root . --all-versions --require-dotnet --no-ratchet --junit "$out/before/runtime-junit.xml"
  CORPUS_BUMP_BASELINES=0 bash "$here/corpus_bump.sh" "$run" 2>&1 | tee "$out/bump.log"
  sha="$(git -C corpus rev-parse HEAD)"
  python3 -c '
import json, sys
json.dump({"sha": sys.argv[1], "old": sys.argv[2], "run": sys.argv[3], "repo": sys.argv[4]}, open(sys.argv[5], "w"))' \
    "$sha" "$old" "$run" "$repo" "$out/meta.json"
  git -C corpus log --first-parent --format='%h %s' "$old..$sha" > "$out/corpus-log.txt" || true
  cargo build --release -q -p al-test-cli
  # The outcomes on the new pin against the baselines (a lost case exits 1 and stays in the list).
  keep diff ./target/release/al-test-cli --root . --all-versions --require-dotnet --baseline-diff --junit "$out/after/runtime-junit.xml"
  keep awaiting ./target/release/al-test-cli --root . --awaiting-assumptions
  keep gates cargo test -q -p al-version --test gate_completeness
  keep harness cargo test -q --release -p al-test-cli --test corpus_harness
  keep golden cargo test -q --release -p al-test-cli --test corpus_resolve_deps -- --ignored
  python3 -I "$here/corpus_bump_report.py" "$out" > "$out/body.md"
  commit_bump "$root" "$sha" "$run"
}

case "${1:-}" in
  find) [ $# = 1 ] || usage; find_run ;;
  bump) [ $# = 3 ] || usage; [[ "$2" =~ ^[0-9]+$ ]] || usage; bump "$2" "$3" ;;
  commit) [ $# = 4 ] || usage; commit_bump "$2" "$3" "$4" ;;
  *) usage ;;
esac
