#!/usr/bin/env bash
# The regen bot (.github/workflows/ci.yml, job `regen-bot`): copies the generated files that the
# green CI run of a push to main wrote (the artifact `regen-<run id>`) into a checkout of that
# commit and commits them. It does not push; the workflow does.
#
#   tools/regen_bot.sh REPO REGEN_DIR SHA
#
# REGEN_DIR holds files at their repository paths. Only these paths are accepted:
#   metrics/<major>/passing*.txt         the baselines (`al-test-cli --all-versions --update-baseline`)
#   docs/corpus-coverage.{md,json}       `al-test-cli --coverage-map`
#   docs/corpus-behavior-coverage.{md,json}  `al-test-cli --behavior-coverage-map`
#   docs/probes-needed.md                `al-test-cli --assumptions-doc`
#
# Baselines only grow (CLAUDE.md hard rule 2: removing a case is a human decision). When a new
# baseline lacks a case of the committed one, nothing is copied or committed, and the script exits 1.
# Any other path exits 2. Nothing changed: no commit, exit 0. The commit message carries
# `[skip ci]`, so the push does not start CI again.
#
# Identity: REGEN_BOT_NAME and REGEN_BOT_EMAIL (default: the github-actions bot).
set -euo pipefail

repo=${1:?usage: regen_bot.sh REPO REGEN_DIR SHA}
regen=${2:?usage: regen_bot.sh REPO REGEN_DIR SHA}
sha=${3:?usage: regen_bot.sh REPO REGEN_DIR SHA}
name=${REGEN_BOT_NAME:-github-actions[bot]}
email=${REGEN_BOT_EMAIL:-41898282+github-actions[bot]@users.noreply.github.com}

allowed() {
  case "$1" in
    metrics/[0-9]*/passing*.txt) return 0 ;;
    docs/corpus-coverage.md | docs/corpus-coverage.json) return 0 ;;
    docs/corpus-behavior-coverage.md | docs/corpus-behavior-coverage.json) return 0 ;;
    docs/probes-needed.md) return 0 ;;
  esac
  return 1
}

# The case lines of a baseline, sorted (comments and blank lines dropped).
cases() { grep -v -e '^[[:space:]]*#' -e '^[[:space:]]*$' "$1" | sed 's/[[:space:]]*$//' | LC_ALL=C sort -u; }

files=()
while IFS= read -r -d '' f; do
  files+=("${f#"$regen"/}")
done < <(find "$regen" -type f -print0 | LC_ALL=C sort -z)

bad=0
for f in "${files[@]}"; do
  if ! allowed "$f"; then
    echo "regen bot: $f is not a generated file the bot may write" >&2
    bad=1
  fi
done
[ "$bad" = 0 ] || exit 2

lost=0
for f in "${files[@]}"; do
  case "$f" in metrics/*) ;; *) continue ;; esac
  [ -f "$repo/$f" ] || continue
  gone=$(LC_ALL=C comm -23 <(cases "$repo/$f") <(cases "$regen/$f"))
  if [ -n "$gone" ]; then
    echo "regen bot: $f would lose $(wc -l <<<"$gone") baseline case(s); removing a case is a human decision (CLAUDE.md hard rule 2):" >&2
    sed 's/^/  /' <<<"$gone" >&2
    lost=1
  fi
done
if [ "$lost" = 1 ]; then
  echo "regen bot: nothing copied, nothing committed" >&2
  exit 1
fi

for f in "${files[@]}"; do
  mkdir -p "$(dirname "$repo/$f")"
  cp "$regen/$f" "$repo/$f"
  git -C "$repo" add -- "$f"
done

if git -C "$repo" diff --cached --quiet; then
  echo "regen bot: the generated files are current"
  exit 0
fi

changed=$(git -C "$repo" diff --cached --name-only)
git -C "$repo" -c user.name="$name" -c user.email="$email" commit -q \
  -m "Regenerate generated files for ${sha:0:8} [skip ci]" \
  -m "Written by the CI run of ${sha} on main (ci.yml, job regen-bot). Baselines only grow.

$changed"
echo "regen bot: committed"
sed 's/^/  /' <<<"$changed"
