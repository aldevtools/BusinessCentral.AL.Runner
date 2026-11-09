---
title: "Profiling AL code"
weight: 75
---

AL Runner can write AL CPU profiles: `.alcpuprofile` files, in the format Business
Central's AL profiler writes. The AL extension in VS Code opens them: a top-down and a
bottom-up call tree, a click on a node opens the source line, and CodeLens shows the
time and hit count of hot statements. A file written in sampling mode can also be
loaded on BC's Performance Profiler page.

## Quick start

```bash
al-runner test ./MyApp ./MyApp.Test --profile out/
```

Then open `out/50120-My Tests.alcpuprofile` in VS Code. The folder holds one file per
test codeunit, named `<codeunit id>-<codeunit name>.alcpuprofile`, and `summary.txt`,
a text version of the same data.

To get one file for the whole run, give a path that ends in `.alcpuprofile`:

```bash
al-runner test ./MyApp ./MyApp.Test --profile run.alcpuprofile
```

## Options

| Flag | Effect |
|---|---|
| `--profile PATH` | Turns the profiler on. A folder gets one file per test codeunit plus `summary.txt`. A path ending in `.alcpuprofile` gets one merged file. |
| `--profile-mode MODE` | `instrumentation` (default) or `sampling`. |
| `--profile-interval US` | Sampling only: microseconds between samples. Default 1000 (1 ms), at least 50. |

In `al-runner serve`, the `runTests` request takes a `profile` field:
`{"path": "out/", "mode": "sampling", "intervalUs": 1000}`. The summary line then has
`profile: {"files": [...]}`. See [Tool protocol](tool-protocol.md).

With the profiler off, a run pays one branch per statement, call, builtin return and
record operation. Nothing is allocated and no clock is read.

## Two modes

**Instrumentation** (the default) measures every call. Each node has its exact total
time in microseconds and its call count ("Hit count" in VS Code). Each statement has its
run count and its time, including the calls it makes. This is what VS Code's CodeLens
shows. The file is written exactly as the AL extension writes it from a BC snapshot
(`sampleExecutionTimes`, no `kind`).

**Sampling** reads no clock per call. A timer sets a flag every interval; the next
statement, builtin return or record operation takes it, and the time since the last
sample goes to the routine that is running. The file has `kind: 1` and self times in
`timeDeltas`, as BC's sampling profiler writes it. Every thread is sampled on every
tick (the old `AL_PROFILE` sampled one thread per tick). The file holds only the call
paths a sample landed in, as BC's sampling profiler builds its tree from the sampled
stacks: a path that took no sample has no time, so it is left out, and so is a record
operation no sample landed in. This keeps the profile of a long run small: one test
codeunit of Tests-ERM-Purchase entered about 700 000 call paths for about 5 000 samples.

Which to use:

- Use **instrumentation** to count calls, to see statement times in CodeLens, or to
  compare two runs exactly.
- Use **sampling** for the lowest overhead, or for a file to load on BC's
  Performance Profiler page.

On the calibration app (below), instrumentation cost about 2 % more instructions than
a run without the profiler, and sampling about 0.5 %.

### Self time in VS Code is wrong for instrumentation files

The AL extension computes a node's self time as its total minus the **self** times of
its children. It should subtract their **totals**. Example: A takes 10 ms and calls B
(8 ms), which calls C (5 ms). The true self time of A is 2 ms. VS Code shows 7 ms
(10 minus B's self time of 3 ms). The total times are right.

BC's own instrumentation files show the same error, so AL Runner writes its files the
same way. For exact self times:

- use sampling mode (VS Code reads self times straight from the file), or
- read `summary.txt`, which computes self time as total minus the children's totals.

## Memory

The profiler keeps the tree of every test codeunit until the run ends, and that memory
counts toward `--max-memory` like the rest of the run. An instrumentation profile holds
every call path the tests entered, so on a large suite it can need several GB; sampling
holds only the sampled paths. The files are written to disk node by node, so writing
them needs only a few MB more. Under `--max-memory` (or a cgroup's `memory.max`) the
write also stops before the process would pass the limit: the files already complete
stay, a file that was not finished is removed, and a line says how many test codeunits
were not written. The results files (`--junit`, `--output-json`) are written before the
profile.

## Threads and test codeunits

AL Runner runs test codeunits on several worker threads at once. Each test codeunit
runs on its own session, and each session has its own profile.

- **One file per codeunit** (folder): the root node is the thread, for example
  `Thread 3 (test worker)`. Below it are the frames as they ran: the test runner
  codeunits, `OnRun`, each test method, and what they called. The file explains that
  codeunit's own time.
- **Merged file**: the root is `al-runner run`. Below it is one node per thread, then
  one node per test codeunit (`Codeunit 50120 "My Tests"`), then the frames. A thread's
  total is the time it was busy. All threads together add up to more than the wall
  time of the run.

Every file has an `alRunner` block, which VS Code ignores:

```json
"alRunner": {
  "mode": "instrumentation", "unit": "microseconds", "intervalUs": null, "samples": 0,
  "threads": [{"id": 1, "label": "Thread 1 (test worker)", "busyUs": 7339632}],
  "codeunits": [{"id": 50120, "name": "Calib Tests", "thread": 1, "totalUs": 7339632,
                 "tests": [{"name": "Cpu", "totalUs": 523714}, "..."]}]
}
```

All times in the file are in microseconds. `startTime` and `endTime` count from
0001-01-01, as BC's do.

## Database operations

AL Runner keeps the database in memory. A record operation that is a SQL round trip on
BC costs microseconds here. To show this work anyway, the profile counts the record
operations of each routine: Get, Find, FindFirst, FindLast, FindSet, Next, Insert,
Modify, ModifyAll, Delete, DeleteAll, Rename, CalcFields, CalcSums, Count, IsEmpty and
LockTable.

In the file they are laid out like the `SQL` nodes of BC's sampling profiler. Under the
routine's node is one node `Database (in memory)`. Under it is one node per operation
and table, for example `FindSet "Sales Line"`. Its hit count is the number of calls and
its time is the runner's in-memory time. A temporary record is listed apart, with
`(temporary)`: it never reaches SQL on BC either. The time of a trigger an operation
runs (OnInsert, a subscriber) is not in the operation's node: the trigger is a node of
its own.

`summary.txt` has the same counts in its last list, "record operations per routine".

## How representative are the times?

A profile from AL Runner tells you where AL Runner spends its time. Much of that
carries over to BC, but not all. These are the differences that change the picture:

| | Business Central | AL Runner | effect on the profile |
|---|---|---|---|
| record reads and writes | a SQL round trip each (Insert, Modify, Delete, Get on a cache miss, FindSet per batch, CalcFields one query), about 0.1 to 1 ms | in memory, about 2.5 µs per operation | record-heavy code looks 100 to 1000 times cheaper |
| CalcFields, CalcSums | a SQL aggregate, often over a SIFT index: close to constant cost | a scan of the matching rows | cost grows with the number of rows; can be weighted too high or too low |
| AL statements | C# compiled from AL | a bytecode interpreter | same order of magnitude, not equal |
| builtins | .NET implementations | Rust implementations | the cost of one builtin can grow differently with its input |
| metadata, permissions, locks, network, client | real costs (metadata on first use, lock waits, UI round trips) | none or near zero | first-use and UI costs are missing |

Rules of thumb:

1. **Hot spots in AL-only code carry over.** Loops, string building, arithmetic and
   calls between your own procedures keep about the same ratios.
2. **For record-heavy routines, read the counts, not the time.** A routine with
   20,000 Modify calls is cheap here and slow on BC. The `Database (in memory)` nodes
   show the counts.
3. **A builtin can behave differently.** For example, `CopyStr(Text, 1, 10)` on a
   1 MB text takes about 0.3 ms in AL Runner today, because it walks the whole text.
   On BC it does not. Check a hot builtin before you rewrite code around it.
4. **Costs AL Runner does not have do not show.** Metadata loading, lock waits and
   page round trips are absent.

### Calibration

The calibration app runs four workloads, one test each. Measured with AL Runner on one
thread:

| test | workload | AL Runner | share | what the profile shows |
|---|---|---:|---:|---|
| Cpu | 400,000 loop iterations, Decimal arithmetic, one local call each | 199 ms | 8 % | `CpuWork` and `Step` (400,000 calls) |
| Records | 20,000 Insert, a FindSet, 20,000 Modify and Next | 147 ms | 6 % | `Database (in memory)`: Insert, Modify, Next 20,500 each |
| Flow | 500 rows, 5,000 CalcFields over a growing range | 693 ms | 29 % | `CalcFields "Calib Header"` 5,000 calls, almost all of the time |
| Strings | 200,000 TextBuilder.Append, 4,000 `CopyStr(TB.ToText(), 1, 10)` | 1,355 ms | 56 % | the `CopyStr` statement, 1,174 ms |

On BC, the Records and Flow tests are expected to take the largest share, and Cpu and
Strings to shrink. The same app has not been timed on BC yet. When it has been, this
table gets a BC column and a factor per record operation.

## The text summary

`summary.txt` (and the file of the old `AL_PROFILE` variable) has:

1. A header that names the mode and the unit, for example
   `samples 3810 of 1 ms on 1 threads (each thread sampled every tick); times in ms`.
2. The wall time of each test codeunit and each of its tests, with the thread.
3. The routines by self time and by inclusive time, with calls (instrumentation) or
   samples (sampling).
4. The statements by time (`file:line`), with runs or samples.
5. The record operations per routine, by count.

## The old `AL_PROFILE` variable

`AL_PROFILE=FILE` still works in this release. It will be removed in the next one.
It now does the same as `--profile FILE.alcpuprofile --profile-mode sampling`, and
also writes the text summary to `FILE`:

- `AL_PROFILE=profile.txt` writes `profile.txt` (text) and
  `profile.txt.alcpuprofile` (merged sampling profile) when the run ends.
- `AL_PROFILE_US=N` sets the interval, as before (default 5,000 µs).
- `AL_PROFILE_ONLY` is gone. Select the test with `--test` or `--codeunit` instead,
  or open the codeunit's own file.

A run with `AL_PROFILE` set prints a line that says it is deprecated.

## Limits

- Only test codeunits are profiled: the install code that runs before them, and
  compiling, are not. `--timings` shows those.
- Frames of dependency apps (System Application, Base Application) link to the AL
  extension's symbol preview (`al-preview://`). The line number counts from the start
  of the file, not of the object, so the cursor may land a few lines off. Frames of the
  APPs you give as folders link to the `.al` file and line.
- Columns count characters as bytes. A line with non-ASCII characters before the
  statement puts the cursor a little to the right.
