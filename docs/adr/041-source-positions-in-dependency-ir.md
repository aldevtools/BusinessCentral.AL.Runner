# ADR-041: Source positions in the IR of dependency apps

- Status: Accepted (owner, 2026-10-17)
- Date: 2026-10-17
- Related: ADR-013 (external objects), ADR-022 (compile cache, section "Dependency IR carries no source positions"), CLAUDE.md hard rules 3 and 6

## Context

`SessionInformation.Callstack` and `GetLastErrorCallStack` name, for each frame, the object, the routine and the line of the
current statement relative to the routine (corpus session/TestSessionInformationCallstackContracts.al:130-143,
session/TestSessionCallstackSourcesContracts.al:Callstack_FromTablePageReportAndExtensionCode). The lowering gave only the
routines of the workspace such positions: for a dependency app they cost a third of the lowering and "no corpus test reads
them" (ADR-013, measured; ADR-022 keeps them out of the cache entries).

Microsoft's own tests read them. `Error Message Management.GetCurrCallStack` reads `SessionInformation.Callstack`, so every
test that logs an error message through it reaches a call stack whose frames are routines of Base Application. Without
positions the call is refused ("a frame whose format is not pinned"): 197 Tests-Misc and 17 Tests-SCM-Service tests.

### Measured (BC 27.5, Tests-Misc, release build, same machine)

| | Without | With positions in dependency IR |
|---|---:|---:|
| Tests passed | 1279 | 1451 (+172) |
| Cold compile (cache entry built) | 19.4 s | 30.8 s |
| Warm compile (entry read) | 2.6 s | 2.9 s |
| Compile cache entry for BC 27.5 | about 21 MB | about 29 MB |
| Run time of the tests | 13.4 s | 10.9 s to 12.2 s (noise) |

### Measured again on main with bump 15 (2026-10-17)

| | Without | With |
|---|---:|---:|
| Tests-Misc passed (BC 27.5) | 1353 | 1527 |
| Tests-SCM-Service passed | 1083 | 1099 |
| Compile cache entry, BC 27.5 | 19.0 MB | 25.3 MB (+33 %) |
| Full corpus run with the apps (BC 29) | 137.1 G instructions | 143.9 G (+5 %); limit 155 G |
| Cold compile of the corpus | 662 399 per KiB | 664 306 per KiB (the corpus has no dependency code); limit 695 000 |

Both budgets hold; no limit is raised.

## Decision

The routines of dependency apps carry source positions like those of the workspace. `AL_NO_DEP_POSITIONS=1` turns it off
(and the compile cache key records the choice, as it does for `AL_UNPINNED_BASE_ACCESS`).

The text of a frame of a dependency routine is the one of a workspace routine, with the app of the object, its publisher and
its version appended (pinned for the apps the corpus defines). That the line of a Microsoft routine is counted from its
signature as BC counts it follows from the same rule; no corpus test measures a Microsoft routine.

## Consequences

- A cold compile of the dependency apps takes about 60 % longer and its cache entry grows by about 40 %. A warm run is not
  slower. The perf budgets are checked against this (docs/architecture.md).
- ADR-022's statement "Dependency IR carries no source positions" no longer holds; the entries now hold them.

## Conformance

`crates/al-test-cli/tests/suite/dependency_positions.rs` (a dependency frame names its line; with `AL_NO_DEP_POSITIONS` the call
is refused again).
