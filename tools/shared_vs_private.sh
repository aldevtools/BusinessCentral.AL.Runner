#!/usr/bin/env bash
# ADR-069 differential check: the test threads share one frozen install baseline, or (AL_PRIVATE_BASELINE=1) each
# reads its own copy as before. Only time and memory may differ. This runs one command twice, once each way, and
# compares the JUnit files test by test (outcome and reason, `compare_junit.py --reasons`). It also checks that the
# shared run said its baseline was shared and unchanged at the end, and the private run that it used a copy per thread.
#
#   tools/shared_vs_private.sh OUT_DIR al-runner test ARGS...        (an al-runner run)
#   tools/shared_vs_private.sh OUT_DIR al-test-cli --root . ARGS...  (a corpus run)
#
# The command must not name `--junit` or `--timings`; this adds both. Pass an absolute `--cache-dir` to al-runner so
# both runs read the same compile cache. Each run's peak memory and wall time go to OUT_DIR/{shared,private}.time.
# Exit: 0 no difference, 1 a difference or a missing line.
set -uo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
[ $# -ge 2 ] || { sed -n 6,8p "$0"; exit 2; }
out="$1"; shift
mkdir -p "$out"

run() {  # SIDE ENV...
  local side="$1"; shift
  env "$@" python3 -I -c '
import resource, subprocess, sys, time
t = time.time()
r = subprocess.run(sys.argv[2:])
with open(sys.argv[1], "w") as f:
    f.write(f"peak_rss_kb {resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss}\nexit {r.returncode}\nwall_s {time.time() - t:.1f}\n")
' "$out/$side.time" "${cmd[@]}" --junit "$out/$side.xml" --timings > "$out/$side.log" 2>&1
  echo "$side: $(tr '\n' ' ' < "$out/$side.time")"
}

cmd=("$@")
run shared -u AL_PRIVATE_BASELINE
run private AL_PRIVATE_BASELINE=1
status=0
python3 "$here/compare_junit.py" --reasons "$out/shared.xml" "$out/private.xml" || status=1
if grep -q '^install baseline: shared by the test threads' "$out/shared.log"; then
  grep -q '^install baseline: shared by the test threads.*, unchanged at the end' "$out/shared.log" \
    || { echo "CHANGED: the shared install baseline did not hold at the end"; status=1; }
else
  echo "NOTE: the shared run did not share a baseline (one test thread, or: $(grep -m1 '^install baseline: a copy' "$out/shared.log"))"
fi
grep -q '^install baseline: shared' "$out/private.log" && { echo "WRONG: the private run shared the baseline"; status=1; }
exit $status
