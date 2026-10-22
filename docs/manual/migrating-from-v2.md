---
title: "Migrating from version 2"
weight: 12
---

Version 3 replaces version 2. Version 2 was written in C# on top of Business
Central's own compiler and runtime. Version 3 is written in Rust and reproduces
what AL code can observe: values, errors, data, and the order of triggers and
events. That is the public contract of the language, so it changes slowly and
visibly. The version 2 engine is frozen on the
[`v2` branch](https://github.com/aldevtools/BusinessCentral.AL.Runner/tree/v2).

The NuGet package [`MSDyn365BC.AL.Runner`](https://www.nuget.org/packages/MSDyn365BC.AL.Runner)
stays and keeps getting updates. It becomes a wrapper around the version 3 engine,
so you can keep installing with `dotnet tool` and run the new engine. The wrapper
adds some start-up overhead, which should be small. Releases of the package that
predate the wrapper still contain the version 2 engine and keep working.

If you maintain a tool that calls AL Runner, read
[Migrating tools from AL Runner v2 to v3](../integrations/migrating-tools-to-v3.md).

## What changes for you

| | Version 2 | Version 3 |
|---|---|---|
| Install | `dotnet tool install -g MSDyn365BC.AL.Runner`, needs the .NET SDK | Standalone binary with no .NET: use mise or the install script. Or keep using `dotnet tool`, now a wrapper around the version 3 engine. |
| Command | `al-runner [FLAGS] DIR...` | `al-runner test [FLAGS] APP...` |
| Input | A "bundle" folder | An app folder, a source archive or an `.app` with source. Name the app and its test app. |
| Unsupported surface | The test fails with `RunnerOutOfScopeException` | The test is **not run**, with a reason. Exit code 7. |
| Compile errors | Exit code 3 | An AL compiler error (the ones the must-not-compile corpus pins) still exits 3. Source the parser does not accept becomes "not run" rows, with exit 7. |
| Test data | `--test-data`, needs the `bcbak` reader | On by default with Microsoft's apps. `--no-test-data` turns it off. No separate reader to install. |
| Failure text | May carry a .NET prefix such as `NavNCLDialogException:` | The Business Central message only. |
| Stack trace | A .NET stack | The AL call stack. |
| Timeout | 60 seconds of wall time | 20 seconds of CPU time. |
| Business Central versions | One engine per minor version | One binary for 27, 28 and 29. |
| Dependencies | Artifact cache and dotnet tool store | `--dep`, `--package-cache`, `.alpackages`, then the artifact feed. |

## Flags

| Version 2 | Version 3 |
|---|---|
| `--test PATTERN`, `--filter PATTERN` | Same names. A pattern with a dot matches `Codeunit<ID>.<Method>`; without a dot it matches the method name. |
| `--test-exact`, `--exclude-test` | Same. |
| `--isolation codeunit\|test\|disabled` | `codeunit` and `test` work. `disabled` is not supported. |
| `--test-timeout SECONDS` | Same flag, now CPU time. Default 20. |
| `--package-cache PATH` | Same. |
| `--test-data`, `--test-data-company` | Same flags. |
| `--output-junit PATH` | `--junit FILE` |
| `--output-json` | `--output-json [FILE]`, same shape. With no FILE it is the only thing on stdout. |
| `--coverage` | Same, plus `--coverage-out FILE` and `--coverage-json FILE`. |
| `--tdd` | Same flag. Stubs missing procedures and overloads, fields, enum values, and missing codeunits, tables and enums. |
| `--server` | `al-runner serve`, same protocol. |
| `--define`, `--preprocessor-symbols` | Same. |
| `--country` | Same. |
| `--show-pass`, `--failures-only`, `--no-strict-exit` | Not accepted. Failures print by default; use `--verbose` for every test and `--allow-not-run` to relax exit codes. |
| `--cache PATH`, `--no-cache` | Gone. Your own apps are compiled on every run. The dependency apps use the compile cache (`--cache-dir`). |
| `--seed N` | Not accepted. Version 3 prints no seed. |
| `--jobs N` | Gone. Test codeunits run on worker threads automatically. |
| `--watch` | Not available. |
| `--dap [PORT]`, `--dap stdio` | `al-runner debug [--port [N] \| --stdio]`, same DAP surface. See [Editor and tool integration](editor-tools.md#debugging). |
| `--dump-csharp DIR` | Gone. There is no C# stage. |
| `--affected`, `--include-failing`, `--strict-environment` | Same flags. The strict behavior of `--strict-environment` is always on. See [Writing tests](writing-tests.md#running-only-what-a-change-affects). |
| `--per-suite` | Refused with exit 2. |
| `--auto-provision` | Accepted and ignored. Version 3 always downloads, unless `--no-download`. |
| `--count-baseline`, `--expectations` | Gone. |

The environment variables changed too. Version 3 uses `ALRUNNER_CACHE` as the one
cache root, and the `AL_RUNNER_ARTIFACTS_ROOT`, `_CACHE_ROOT`, `_SYMBOLS_ROOT` and
`_ENGINE_CACHE_ROOT` variables no longer apply. See
[the reference](cli-reference.md#environment-variables).

## Exit codes

| Code | Version 2 | Version 3 |
|---|---|---|
| 0 | All passed | All passed |
| 1 | A test failed | A test failed |
| 2 | A bundle could not execute | Bad invocation, or the run could not start |
| 3 | A bundle could not compile | The apps do not compile (an AL compiler error). No test ran. |
| 4, 5 | Baseline and expectations mismatches | Not used |
| 6 | The selection matched no test | The selection matched no test |
| 7 | Not used | Some tests were not run |
| 8 | Not used | Not run because the .NET helper could not be used |

## One difference with no workaround

Version 2 loaded compiled code. A dependency app that ships without AL source (a
compiled-only app) could still run. Version 3 compiles everything from source and
cannot run compiled C#. A test that reaches such an app is not run, and the message
names the app. Provide an `.app` that carries its source, fake the dependency behind
an interface, or keep using version 2 for that project.
