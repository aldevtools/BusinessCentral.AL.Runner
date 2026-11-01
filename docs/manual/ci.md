---
title: "Continuous integration"
weight: 80
---

AL Runner runs without a BC service tier, Docker or SQL Server, so it runs on an ordinary
hosted runner. AL Runner is an alpha. Use `--allow-not-run` so refused tests do not
fail the job, and drop it when you want every test to run.

Use AL Runner in a pipeline for fast feedback, not as the only check before a
release. Before you deploy to production, run your tests at least once in a way
Microsoft supports, for example in a Business Central container on Windows
(BcContainerHelper) or with AL-Go for GitHub.

## GitHub Actions

A minimal job for a repository that holds `MyApp` and `MyApp.Test`:

```yaml
name: AL tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    env:
      ALRUNNER_CACHE: ${{ github.workspace }}/.alrunner-cache
    steps:
      - uses: actions/checkout@v4
      - name: Install al-runner
        run: |
          curl -fsSL https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.sh | sh -s -- v3.0.0-alpha.11
          echo "$HOME/.local/bin" >> "$GITHUB_PATH"
      - uses: actions/cache@v4
        with:
          path: .alrunner-cache
          key: alrunner-${{ runner.os }}-${{ hashFiles('**/app.json') }}
      - name: Run tests
        run: al-runner test ./MyApp ./MyApp.Test --junit test-results.xml --allow-not-run
      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: test-results
          path: test-results.xml
```

## Advice

- **Pin the version** so the result is repeatable. The example installs one numbered
  alpha. Use `latest-alpha` for the newest numbered alpha, or `edge` for the nightly
  build. The compile cache is kept per `al-runner` build, so a new build fills it again
  once.
- **Cache the cache folder.** Set `ALRUNNER_CACHE` to a folder inside the workspace
  and cache it. It holds the downloads, the demo backup and the compile cache. The
  first run builds it, which takes about 10 seconds of compile plus the downloads.
  Add `--no-test-data` if your tests need no CRONUS data; the cache is then much
  smaller.
- **Require the cache** with `--require-compile-cache` when a missing cache should
  fail the job rather than silently take longer.
- **Go offline** with `--no-download` once the cache is warm and the feed is not needed.
- **Report files.** `--junit FILE` gives test results where each not-run test is
  `<skipped>` with its reason. `--output-json FILE` gives a machine-readable
  document. `--coverage-out FILE` gives Cobertura XML for coverage tools.
- **Exit codes.** See [Reading results](results.md#exit-codes). Code 7 means some
  tests were refused, code 8 means the .NET helper was needed and missing.
- **See why tests did not run.** Add `--top-refusals` to the command.

## Microsoft's own test apps

```bash
al-runner test-microsoft --app Tests-ERM --junit erm --allow-not-run
al-runner test ./MyApp ./MyApp.Test --affected-from microsoft --junit results.xml
```

See [Running Microsoft's tests](microsoft-tests.md). To skip tests your change
cannot affect, restore and save the stored result of `--affected` between builds
with `--affected-store FILE`; see [Writing tests](writing-tests.md#running-only-what-a-change-affects).

## AL-Go

The [AL-Go fork](https://github.com/aldevtools/AL-Go) adds a Linux fast lane: pull
request builds compile from source and run AL unit tests on Ubuntu, with no Windows
container. See its repository for the settings.
