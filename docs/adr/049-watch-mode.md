# ADR-049: `alrunner test --watch`: rerun the tests when AL source changes

- Status: Proposed
- Date: 2026-10-21
- Related: ADR-016 (release gates, owner decision 2026-10-21 that 3.0.0 needs v2's features), ADR-028 (warm
  server), ADR-029 and ADR-033 (incremental compile), ADR-050 (`--affected`), ADR-051 (`--tdd`), ADR-006 (core purity)
- Parity rows: PERF-4 (`--watch`), PERF-5 (dashboard), ENV-4 (`AL_RUNNER_WATCH_*`)

## Context

The owner decided on 2026-10-21 that 3.0.0 replaces v2 fully. So AL Runner needs `--watch`. Today `alrunner test
--watch` exits 2 with "deferred" (`args.rs`).

### What v2 does

All references are to the `v2` branch.

- **Flag.** `--watch` keeps the process alive and reruns after a change. It is refused together with `--server`
  (exit 2). `Program.cs` around lines 544, 690-712, 2648-2745 and 4991-5115.
- **What is watched.** A `FileSystemWatcher` per app root: the folder of the nearest `app.json` above each source
  path. Recursive. Only `.al` events count. `.git` is skipped. A folder event is only a hint and is checked against a
  snapshot of the `.al` files (length and modified time). The snapshot is taken when a cycle starts, so an edit made
  during a run starts the next cycle. `WatchSource.cs`.
- **Debounce.** Wait for quiet: each new event resets the wait. A cycle starts after 250 ms with no event, and at the
  latest 10 s after the first event. `AL_RUNNER_WATCH_QUIET_MS` and `AL_RUNNER_WATCH_MAX_WAIT_MS` set both. A missing,
  invalid or non-positive value uses the default. `WatchSource.cs` lines 38 and 49.
- **What reruns.** All tests, unless `--affected` is set (ADR-050). The compile is incremental. When it is not, v2
  prints `[watch] <module>: FULL REBUILD this cycle — <reason>`. `Program.cs` around 4118-4170.
- **Output.** On an interactive ANSI terminal, a live dashboard (`WatchDashboard.cs`): header with app, state, time
  and duration; a banner for a full rebuild; the `--affected` and `--tdd` lines; a tree of codeunits and tests with
  failure messages and AL call stacks; a pass/fail/error footer. Arrow keys, Page Up/Down, Home and End scroll. `Q`
  or Ctrl+C quits. Otherwise plain lines: one per test, the summary, `[watch] waiting for AL source changes… (Ctrl+C
  to quit)`, then `[watch] change detected — re-running…`.
- **JSON.** No `--output-json` output per cycle.
- **End.** Ctrl+C or `Q`: exit 0.
- **Tests.** `WatchTests`, `WatchSourceTests`, `WatchDashboardTests`, `WatchMidCycleEditTests`,
  `WatchNewFolderTests`, `WatchFullRebuildReasonTests`, `WatchMovedAppTests`, `TddWatchTests` and others in
  `AlRunner.Tests`.
- **Tools.** Neither ALchemist nor LethAL uses `--watch`. They use the server. So the watch surface is for people,
  and its text does not have to match v2 byte for byte.

### What AL Runner has already

- `alrunner serve` keeps a warm slot (`al_testrun::runtime::Warm`) with the compiled dependency apps, the lowered
  workspace and the post-install snapshot (ADR-028).
- `Compiled::update_files` compiles only the changed objects when the change is in code (ADR-029, ADR-033), and
  compiles everything again otherwise.
- `events::Hooks` streams one `TestEvent` per test, and `cancel` stops a run after the current test.

So `--watch` is a loop in the CLI around what `serve` already does. Only the file watching is new.

## Decision

1. **Surface.** `alrunner test --watch PATH...`. Every other `alrunner test` option keeps its meaning for every
   cycle. Only `alrunner test` takes `--watch`.
2. **Where it lives.** All of it lives in `alrunner-cli` (`watch.rs`, new). The watcher, the clock for the debounce,
   the terminal and Ctrl+C are OS access, so none of it goes into a core crate (ADR-006).
3. **Detecting a change: polling a snapshot.** Every 100 ms the CLI lists the `.al` files under each app root (the
   folder of the nearest `app.json`, as v2) and keeps length, modified time and a content hash for each. The content
   hash is computed only when length or time changed. A cycle starts when the set of files or a hash changed. Polling
   needs no new dependency and behaves the same on Linux, macOS and Windows. Saving a file without changes starts no
   cycle (an improvement on v2). `.git`, `.alpackages`, `.worktrees`, `node_modules` and the output folders of the
   run (`--junit`, `--coverage` paths) are skipped. An `app.json` change is watched too and starts a full compile.
   See open question 1 for native file events.
4. **Debounce as v2.** Quiet window 250 ms, at most 10 s after the first change. `ALRUNNER_WATCH_QUIET_MS` and
   `ALRUNNER_WATCH_MAX_WAIT_MS` set them. The v2 names are still read. The new name wins. A value that is not a
   positive number is an error at start-up (exit 2), as for `ALRUNNER_TEST_TIMEOUT` (migration guide).
5. **A cycle.**
   1. Take the snapshot (an edit during the cycle starts the next one, as v2).
   2. Hand the changed files to the warm slot. `Compiled::update_files` decides between the incremental and the
      complete compile. A complete compile prints `[watch] full compile this cycle: <reason>`. The reason is the
      text `update_files` already returns.
   3. Run the selection: all selected tests, or the `--affected` selection (ADR-050).
   4. Print the result, then `[watch] waiting for AL source changes (Ctrl+C to quit)`.
6. **A change during a run.** The run is cancelled after the current test (the `cancel` flag of `Hooks`). The
   finished tests are printed. The next cycle starts after the quiet window. This is faster than v2, which finishes
   the stale run first. See open question 3.
7. **A compile error.** The diagnostics are printed and the loop waits for the next change. The process does not exit.
   The last good compile is kept in the warm slot.
8. **Output.**
   - Not a terminal (or `NO_COLOR`, or `--no-dashboard`): plain lines. Each cycle prints the same text a one-shot
     `alrunner test` prints, then the `[watch]` lines. CI and editors can read it.
   - An interactive terminal: the screen is cleared and redrawn each cycle with the same content as v2's dashboard
     (header, rebuild reason, `--affected` and `--tdd` lines, failures with message and AL call stack, footer).
     Scrolling keys need raw terminal mode; see open question 2.
9. **Files per cycle.** `--junit FILE` and `--coverage FILE` are rewritten after each cycle (atomic replace), so a
   coverage viewer follows the latest run.
10. **End.** Ctrl+C (and `q` when the dashboard is on) stops the watcher, cancels a running cycle after the current
    test, and exits 0. Exit 0 also when the last cycle failed: the watch itself did not fail.
11. **Memory.** One warm slot, as in `serve`. The ADR-028 test of 50 changed requests with bounded memory is repeated
    for 50 cycles.

## What is refused

- `--watch` with `--output-json` (exit 2): a stream of JSON documents on stdout is not a format any tool reads. Tools
  use `alrunner serve`. v2 has no JSON per cycle either.
- `--watch` with `--impact-of` and with `impact-index build` (exit 2): those are batch jobs.
- `--watch` in the WebAssembly hosts: there is no file watching there (ADR-016 already lists it).
- `--watch` with sources that are only `.app` packages: nothing to watch (exit 2, with the reason).
- A change to a dependency `.app` under `.alpackages` is not watched in 3.0. The message says to restart. See open
  question 4.

## Test plan (red, then green)

All tests are in `crates/alrunner-cli/tests/watch.rs`. They start the binary with a short quiet window and drive it
by writing files. A test hook (`ALRUNNER_WATCH_TEST_CYCLES=N`) ends the process after N cycles, so no test waits on
a terminal.

1. Start, first cycle, the waiting line. Red: today exit 2 "deferred".
2. Edit a test to fail: the next cycle reports the failure. Edit it back: it passes.
3. A burst of five writes within the quiet window gives one cycle.
4. Save without a change gives no cycle.
5. An edit during a run cancels it after the current test and starts a new cycle with the new text.
6. A new folder with a new `.al` file is picked up. A deleted file is picked up (complete compile, with the reason).
7. A syntax error prints diagnostics and keeps watching; the fix runs again.
8. The incremental compile is used for a code change (`incremental` in the plain output) and a full compile for a
   declaration change, with the reason line.
9. `--junit` and `--coverage` files are rewritten each cycle.
10. Environment variables: the new name, the v2 name, the new name winning, a bad value giving exit 2.
11. Refusals: `--output-json`, `--impact-of`.
12. Plain-line output when stdout is not a terminal; dashboard output checked by a unit test of the renderer on a
    fixed result (no terminal needed).
13. 50 cycles with a changed source: resident memory stays bounded (as ADR-028).

## Size

About 3 agent-days: 1 for the snapshot, debounce and loop on top of the warm slot; 1 for the output (plain and
redraw); 1 for the tests and docs (migration guide, parity rows PERF-4, PERF-5, ENV-4). Native file events or a
scrolling dashboard add about 1 day each.

## Consequences

- No new seam: the loop uses the warm slot, `update_files` and `Hooks` as they are.
- `--affected` (ADR-050) and `--tdd` (ADR-051) plug into step 3 of a cycle.
- Polling costs a directory walk every 100 ms. For a project of 300 files that is well under 1 ms of CPU per poll.

## Open questions for the owner

1. Change detection: polling every 100 ms (recommended: no dependency, same on every OS) or native file events (the
   `notify` crate, license to check against the allow-list).
2. Dashboard: clear-and-redraw without scrolling in 3.0 (recommended) or v2's scrolling dashboard (needs raw terminal
   mode, about 1 more day).
3. An edit during a run: cancel after the current test and restart (recommended) or finish the stale run first (v2).
4. Dependency `.app` changes: restart needed in 3.0 (recommended) or watch `.alpackages` too.

## Conformance

`crates/alrunner-cli/tests/watch.rs` (test plan above) and the renderer unit tests in `crates/alrunner-cli/src/watch.rs`.
