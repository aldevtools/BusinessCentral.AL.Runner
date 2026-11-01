# Oracle

`oracle.toml` lists one `[[version]]` table per BC major (ADR-015). Each table
points to the JUnit report the corpus CI produced against a real BC service
tier, and to the ratchet for that major. The JUnit is the ground truth the
pass rate is measured against: every `<testcase>` in it passed on real BC.

| Field | Value |
|---|---|
| Corpus repo | aldevtools/BusinessCentral.AL.Language.Tests (until 2026-10-21 StefanMaron/BusinessCentral.AL.Language.Tests), branch `master` (the default branch; until 2026-10-19 the side branch `al-runtime-rs`, whose commits are all on `master`) |
| Corpus pin | `92031e20d86eeba7bda6bbb944a99e8bd0bedf1a` (the `corpus` submodule, master merge of corpus PR 706; must equal `corpus_commit` in `oracle.toml`) |

| Major | Oracle leg | Corpus commit of the run | Workflow run | Artifact | Artifact id | Cases |
|---|---|---|---|---|---|---|
| 27 | 27.5 | `92031e2` (the pin) | 41270764562 | `junit-test-results-27.5` | 13070681217 | 20792 |
| 28 | 28.5 | `92031e2` (the pin) | 41270764562 | `junit-test-results-28.5` | 13070169466 | 20887 (+26 `OnRun`) |
| 29 | 29.0 | `92031e2` (the pin) | 41270764562 | `junit-test-results-29.0` | 13071460439 | 21014 (+26 `OnRun`) |

The files come from the green full run of corpus PR 706 ("full run: the pull request has the label full-run"), whose head `f64dfd8` has the same tree
(`cff97a3c`) as the pin `2be1140`.
Every file has 0 failures. Each run published two artifacts with the same name; the
oracle is the large one (the main suite, `junit-al-language.xml`). The small one is
the OnPrem suite (`junit-al-language-onprem.xml`, 72 cases): it is the oracle of the
`onprem` test app below.

## Other test apps

`[[version.suite]]` tables in `oracle.toml` add a test app with its own oracle file, its own
ratchet (`metrics/<major>/passing-<name>.txt`) and its own report line. They are not part of the
overall count.

| Name | App | Oracle file | From | Cases |
|---|---|---|---|---|
| `onprem` (27, 28, 29) | `tests/al-language-onprem` | `<major>/junit-onprem.xml` | the small artifact of the same CI run as the main oracle (ids 11957600568, 11957362118, 11956862735), copied unchanged | 72 |
| `isolation-function` (28 only) | `tests/al-language-isolation-probe` | `28/xunit-isolation-function.xml` | the Windows nightly of 2026-10-18, BC 28.5, run 39127745894, artifact 12137159749 | 7 |
| `isolation-codeunit` (28 only) | the same app | `28/xunit-isolation-codeunit.xml` | the same artifact | 7 (T5 fails on BC, on purpose) |
| `testlibraries` (27, 28, 29) | `tests/al-language-testlibraries` | `<major>/junit-testlibraries.xml` | the TestLibraries artifact (`junit-al-language-testlibraries.xml`) of the same CI run as the main oracle, job "BC TestLibraries <ver> / test", copied unchanged | 37 (BC 29: 40, one skipped on purpose) |

The `isolation-disabled` suite (`tests/al-language-isolation-disabled`, test runner 130451, `TestIsolation = Disabled`;
oracle `<major>/junit-isolation-disabled.xml`, artifact `junit-al-language-isolation-disabled.xml`) is not in the table
yet: `tools/corpus_bump.sh` adds it to `oracle.toml` with the first bump whose corpus pin holds the app (the app is not
there before, and the runner reads every suite's app). `metrics/<major>/passing-isolation-disabled.txt` exist and are
empty; the regen bot fills them. It runs in the main environment under its own runner, so it adds no compile pass.

The `testlibraries` suite names `extra_apps` in `oracle.toml`: Microsoft's Tests-TestLibraries and Data
Archive, which the corpus job installs although no test app declares them (corpus `.github/workflows/ci.yml`,
job `test-testlibraries`). They change what AL code sees (the test runner runs codeunits 130301 and 130302 of
Tests-TestLibraries around every test, and Backup Management sets the database trigger flags), so a suite with
`extra_apps` runs in an environment of its own: a second compile and run of that version, with those apps loaded
from the artifact (compiled from their source, ADR-013) and without the main app's tests. The main app and the other
suites keep the main job's environment. Its ratchet is `metrics/<major>/passing-testlibraries.txt`; the regen bot
fills it from main.

The isolation probe never runs in the Linux CI legs: it needs two custom test runners
(`-testRunnerCodeunitId`), which only the Windows nightly provides. Its files are BcContainerHelper
XUnit, not JUnit, and come from the nightly on corpus staging at `92aeaa7`, whose tests/ are
identical to the pin's (the CLI checks the corpus diff, ADR-007). There is no isolation oracle for
BC 27 and 29. The control suite fails `T5` on real BC; the report
shows that the runtime fails it too.

All three legs come from one corpus commit. The corpus workflow now defines `BC29PLUS` on
dispatch, so the 29.0 run on the pin is usable. The CLI still accepts a `corpus_commit` inside a
`[[version]]` table for an older commit, when the corpus diff since then adds files under
`tests/` and changes none (ADR-007). No table uses it today.

Layout:

```text
oracle/oracle.toml        the pin and the [[version]] tables
oracle/<major>/junit.xml  JUnit of the newest minor of that major (the oracle leg)
metrics/<major>/passing.txt  the ratchet for that major
```

The oracle leg of a major is its newest minor: 27.5, 28.5, 29.0. Every JUnit
file must come from the CI run of the pinned corpus commit, or of the older commit its
`[[version]]` table names (ADR-007). The runner of the
28.x and 29.0 legs adds an `OnRun` pseudo-case per test codeunit; the reader
skips them (ADR-015 1.6).

To bump: move the submodule, download the matching run's
`junit-test-results-<version>` artifact **by artifact id** (two artifacts share
each name), copy it unchanged to `oracle/<major>/junit.xml` (the large one) and
`oracle/<major>/junit-onprem.xml` (the small one), and update `oracle.toml`. Never edit a
JUnit or XUnit file.

The OnPrem DotNet app (`corpus/tests/al-language-onprem-dotnet`) is the suite `onprem-dotnet`. Its oracle is
`oracle/<major>/junit-dotnet.xml` (the smallest of the three `junit-test-results-<leg>` artifacts of a run) and its
ratchet is `metrics/<major>/passing-dotnet.txt`.
