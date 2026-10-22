---
title: "Continuous integration"
weight: 7
---

AL Runner needs no Docker, no SQL Server and no license, so it runs on an ordinary
hosted runner. Version 3 is an alpha. Use `--allow-not-run` so refused tests do not
fail the job, and drop it when you want every test to run.

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
          curl -fsSL https://github.com/aldevtools/BusinessCentral.AL.Runner/releases/download/edge/install.sh | sh -s -- edge
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

- **Pin the version** when the result must be repeatable. Replace `edge` with a tag
  such as `v3.0.0-alpha.4`.
- **Cache the cache folder.** Set `ALRUNNER_CACHE` to a folder inside the workspace
  and cache it. It holds the downloads, the demo backup and the compile cache. The
  first run builds it, which takes about 10 seconds of compile plus the downloads.
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
al-runner test Tests-ERM.Source.zip --junit erm.xml --allow-not-run
```

See [Test data](test-data.md) for the recipe that matches Microsoft's CI.

## AL-Go

The [AL-Go fork](https://github.com/aldevtools/AL-Go) adds a Linux fast lane: pull
request builds compile from source and run AL unit tests on Ubuntu, with no Windows
container. See its repository for the settings.
