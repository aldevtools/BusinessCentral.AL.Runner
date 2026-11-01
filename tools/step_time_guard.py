#!/usr/bin/env python3
"""The loose time guard of the corpus job (.github/workflows/ci.yml, job `corpus`).

The work ratchet (al-test-cli, metrics/<major>/work.json) is the exact check: it counts what a run
compiles and runs. This guard catches what the counts cannot see (a slower routine, a new step of
the same work) and stays loose, because a shared runner's wall time varies by half from one run to
the next: the step fails only when it took more than FACTOR times the median of the same step in
main's last SAMPLES successful CI runs. With fewer than MIN_SAMPLES of them it prints a note and
passes.

Usage:
  step_time_guard.py SECONDS --step NAME [--job corpus] [--samples FILE]

SECONDS is how long the step took in this run. Without --samples the earlier times are read with
`gh api` (GH_TOKEN and GITHUB_REPOSITORY must be set; the job needs `actions: read`). With
--samples FILE they are read from FILE, one number of seconds per line (for tests).
Exit code: 0 within the limit or too few samples, 1 over it, 2 bad arguments.
"""

import argparse
import datetime
import json
import os
import statistics
import subprocess
import sys

FACTOR = 1.5
SAMPLES = 10
MIN_SAMPLES = 5


def decide(current, samples):
    """(passes, message) for a step that took `current` seconds, against the earlier `samples`."""
    if len(samples) < MIN_SAMPLES:
        return True, (
            f"step time guard: {current:.0f} s; only {len(samples)} earlier runs of main to compare with "
            f"(needs {MIN_SAMPLES}), not checked"
        )
    median = statistics.median(samples)
    limit = FACTOR * median
    if current > limit:
        return False, (
            f"step time guard: the step took {current:.0f} s, more than {FACTOR}x the median {median:.0f} s "
            f"of main's last {len(samples)} successful runs (limit {limit:.0f} s). Look at the work counts "
            f"(work ratchet lines above) and the timings tables of the job summary"
        )
    return True, (
        f"step time guard: {current:.0f} s, within {FACTOR}x the median {median:.0f} s "
        f"of main's last {len(samples)} successful runs"
    )


def gh_json(path):
    out = subprocess.run(["gh", "api", path], check=True, capture_output=True, text=True).stdout
    return json.loads(out)


def seconds_between(start, end):
    parse = lambda s: datetime.datetime.fromisoformat(s.replace("Z", "+00:00"))
    return (parse(end) - parse(start)).total_seconds()


def main_samples(job, step):
    """The times of `step` of `job` in main's last SAMPLES successful pushes to ci.yml, newest first."""
    repo = os.environ["GITHUB_REPOSITORY"]
    runs = gh_json(
        f"repos/{repo}/actions/workflows/ci.yml/runs?branch=main&event=push&status=success&per_page={SAMPLES}"
    )["workflow_runs"]
    out = []
    for run in runs[:SAMPLES]:
        jobs = gh_json(f"repos/{repo}/actions/runs/{run['id']}/jobs?per_page=100")["jobs"]
        for j in jobs:
            if j.get("name") != job:
                continue
            for s in j.get("steps") or []:
                if s.get("name") == step and s.get("conclusion") == "success" and s.get("completed_at"):
                    out.append(seconds_between(s["started_at"], s["completed_at"]))
    return out


def main(argv):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("seconds", type=float)
    p.add_argument("--step", required=True)
    p.add_argument("--job", default="corpus")
    p.add_argument("--samples")
    a = p.parse_args(argv)
    if a.samples:
        with open(a.samples) as f:
            samples = [float(line) for line in f if line.strip()]
    else:
        try:
            samples = main_samples(a.job, a.step)
        except (OSError, KeyError, ValueError, subprocess.CalledProcessError) as e:
            print(f"step time guard: could not read main's earlier runs ({e}), not checked")
            return 0
    ok, message = decide(a.seconds, samples)
    if not ok and os.environ.get("GITHUB_ACTIONS"):
        print(f"::error::{message}")
    else:
        print(message)
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
