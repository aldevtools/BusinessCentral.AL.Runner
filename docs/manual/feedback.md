---
title: "Reporting a problem"
weight: 135
---

Reports from users decide what is fixed first. Open a GitHub issue with the "Alpha
feedback" template at
[aldevtools/BusinessCentral.AL.Runner](https://github.com/aldevtools/BusinessCentral.AL.Runner/issues/new/choose).
Before you open one, search the open issues. If your problem is already there,
comment on that issue and say what you need it for.

Do not paste secrets or customer data into an issue.

## Which kind of problem

### A wrong result

A test passes or fails in AL Runner, but behaves differently on real Business
Central. This is the most useful report. Include:

- the output of `al-runner --version`,
- the Business Central version of the run,
- the full command line,
- the test name, what AL Runner did, and what real Business Central does,
- the output of `al-runner test ... --explain TestName`,
- if you can, a small app that shows the problem.

### A refused feature you need

A test is reported as **not run**, and you need it to run. Include:

- the output of `al-runner --version` and the Business Central version,
- the reason text. `--explain TestName` prints it for one test, and `--top-refusals`
  ranks the reasons of a run,
- what the test does, or a small app that reaches the same reason.

If the [Roadmap](roadmap.md) already lists the feature, comment on its issue instead.

### A crash or a hang

AL Runner stops with an internal error, or does not finish. Include:

- the output of `al-runner --version` and your operating system,
- the full command line,
- the complete output on stderr,
- if you can, the app that triggers it.

## What happens next

A wrong result or a crash can be fixed once it can be reproduced. A small app helps
most.

A refused feature often takes longer. AL Runner implements a behavior only after it
has been measured on real Business Central. The first step is a new test in the
[AL language test corpus](https://github.com/aldevtools/BusinessCentral.AL.Language.Tests)
that runs on real Business Central 27.5, 28.5 and 29.0. When that result is in, the
behavior is built to match it. [How results are checked](how-it-works.md) explains
why.
