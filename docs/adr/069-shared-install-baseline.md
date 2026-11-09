# ADR-069: One install baseline for all test threads

- Status: Accepted (owner, 2026-11-09, all 11 decisions)
- Date: 2026-11-09
- Amends: ADR-013 decision 5 (install baseline), ADR-020 (test data rows per thread), ADR-066 amendments 5, 8 and 10
  (carried starts, carry chain, threads one at a time). Each decision below can be accepted or rejected on its own.

## Context

An outside report (issue 6924, a 1.5 GB BC backup with `--companies all`, 8 cores) reached about 24 GB before the
first test. ADR-066 amendment 10 now starts test threads one at a time while they fit in memory, so that run no longer
dies. But each test thread still holds a full copy of the install baseline, so with big test data a run gets few
threads, or stops at `--max-memory`.

### What the code does today (main, 230927b48)

- **The store is already copy-on-write inside one thread.** `al-data` `MemStore` keeps every table as a persistent map
  (`pmap.rs`: a spine of `Arc`-shared chunks of at most 64 rows). A clone shares everything. The first write to a table
  copies its spine (one pointer per chunk) and the one chunk it touches. A snapshot, `Session::restore_baseline`,
  commit and rollback are all clones (`memory.rs` `commit`, `rollback`, `SnapshotStore`). So every test codeunit on a
  thread already starts from the thread's baseline at the cost of a pointer copy, and its writes never reach that
  baseline.
- **Threads do not share it.** `Value` is not `Send` (its reference variants hold `Rc`), so a `Baseline` cannot cross
  threads. Each test thread makes its own copy (`corpus.rs` `thread_baseline`): the first run's baseline is written as
  bytes (`Baseline::to_bytes`, the tables the install code wrote) and each thread decodes them
  (`Baseline::from_bytes`, then `backed_by`).
- **Test data rows are built per thread.** `al-host` `TestData` reads each table of the backup once for all threads
  (the raw cells, `Arc<TableRows>`). Then each thread's `TestBacking` (`al-vm` `testdata.rs`) converts the cells of
  every table it touches into its own rows of `Value`s and keeps them in a per-thread cache (`cache: RefCell<...>`).
- **Carried starts are bytes or deltas.** A carry chain member's end state (RequiredTestIsolation = Disabled, ADR-066
  amendment 8) holds `Rc`s and stays on the walk's thread. A worker that needs it gets a `RowDelta` from the start before
  it, or the whole start as bytes (`lane_starts.rs`, `DELTA_BUDGET` 256 MB), and rebuilds it on top of its own copy.
- **The values in stored rows are already thread-safe.** A stored row holds only plain values. Text is `Txt(Arc<..>)`
  with atomic hint flags ("rows of the test data are shared between threads", `text.rs`), a Blob is
  `Arc<BlobCell>` over an `RwLock`, an Option's names are `Arc<[String]>`. A read copies a Blob before anyone can write
  to it (`copy_blob_db_row`), which the snapshots of one thread already rely on. The type does not say this, because the
  same `Value` enum also holds records, codeunits and lists.

### Measured

I added a measuring step to a local build of main 230927b48 (not committed). At the end of the run it makes four more
copies of the install baseline, the way a test thread makes one (`from_bytes`, `backed_by`), keeps all four alive,
and builds in each copy every test data table the run read. It prints the time and the growth of the resident memory
for each copy. BC 29, Microsoft's apps, release build, cores 0-2 and 5-11, `MemoryMax=12G`, one run at a time, the
default test data (the W1 CRONUS backup of the artifact, 979 MB, the largest backup on this machine).

| Run | Install rows (bytes) | Test data in one copy | One copy: memory | One copy: time | Learned cost per test thread (amendment 10) |
|---|---|---|---|---|---|
| Tests-ERM-Purchase, default test data | 1,073 (0.87 MB) | 236 tables, 25,321 rows | 6 MB + 97 MB | 4 ms + 120 to 500 ms | 1,377 MB |
| Tests-ERM-Purchase, `--companies all` | 1,074 (0.87 MB) | 236 tables, 25,321 rows | 6 MB + 97 MB | 4 ms + 120 ms | 1,440 MB |
| Every compiled table of the CRONUS company, `--companies all` | 1,074 | 388 tables, 51,404 rows | 5 MB + 143 MB | 6 ms + 250 ms | - |

The first copy grows less than the later ones because it reuses memory the allocator already held; the table uses
copies 2 to 4. All runs gave the same outcomes as main (1481 passed, 6 failed, 237 not run).

What this shows:

- **A copy is mostly test data rows.** The install code's own rows are small (6 MB). The rows built from the backup
  cost about 2.8 to 3.8 KB per row in each thread (`Value` per field, plus the map), although the raw cells of the
  backup are already read once for all threads.
- **On Microsoft's suites with CRONUS the copy is a small part of a thread.** About 100 MB of the 1.4 GB a thread
  costs on Tests-ERM-Purchase. The rest is session state and the carried state of the carry chain (ADR-066
  amendment 9, "Limits"). This ADR does not change that part.
- **`--companies all` does not change a Microsoft suite.** Microsoft's per-company tables stay in the session company
  (ADR-046), so the copy is the same.
- **The copy grows with the rows a run touches.** The W1 company has about 52,000 rows in compiled tables. A partner
  backup with several companies and the partner apps' own tables filled holds many more. At about 3 KB per row, one
  million touched rows cost about 3 GB per thread. That would explain a peak like the outside report's (24 GB on 8
  threads).
  I could not measure that backup; it is not on this machine.

Saving, estimated from these numbers: with one shared copy, each test thread after the first saves its copy. With 12
threads on Tests-ERM-Purchase that is 11 x 103 MB, about 1.1 GB, and 11 x 120 to 500 ms of setup. On a backup the
size of the outside report it is about 3 GB per thread after the first (about 21 GB on 8 threads), if the estimate per
row holds there. The thread count of amendment 10 then depends on the session state, not on the size of the backup.

## Decision

1. **One install baseline per run, shared by every test thread, read-only.** After the install code runs, the
   baseline is frozen once (`Baseline::freeze` gives a `SharedBaseline`). Its tables are the same persistent maps as
   today, so a frozen baseline costs no copy. Every session on every thread starts from it with
   `Session::restore_baseline`, as today. Nothing ever writes to it until the run ends.
2. **The change layer of a session is the persistent map it already has; no separate overlay.** A session's store is a
   clone of the shared maps. Its first write to a table copies that table's spine and the one chunk of 64 rows it
   changes, as within one thread today. Reads, `Count`, `IsEmpty`, `FindSet` and every key order read the session's
   own map, which shares unchanged chunks with the base. Rollback drops the session's maps. I rejected an overlay
   (shared base plus a per-session map of changed rows and deletions, merged on every read): every read path,
   `Count`, `IsEmpty`, key order and the order caches would need a merge step, and one path that forgets it leaks
   writes between sessions.
3. **How the shared maps cross threads: a checked wrapper over today's types.** `Value` is not `Send`, although the
   plain values a stored row holds are thread-safe (see Context). `SharedBaseline` holds the tables in a type with
   `unsafe impl Send + Sync`. Only `freeze` makes one, and it checks every key and row value with an exhaustive
   `match` on `Value` (no `_` arm), so a new variant does not compile until someone classifies it. A baseline with a
   value that is not plain (the same rule `Baseline::to_bytes` uses) is not frozen: the run keeps today's path, a
   copy per thread, and `--timings` says why. I rejected a separate `StoredValue` type for rows for now: it is the
   safe form, but it changes every record read and write in `al-vm` and `al-data`.
4. **Test data rows are built once per process.** The per-thread cache of `TestBacking` becomes one cache per test
   data and engine (keyed by `Backing::identity`, which already names "the same rows for every table"), one
   `OnceLock` per table, holding frozen `Rows`. The first thread that touches a table builds it; the others wait for
   it and share it. The rows are a pure function of the backup and the program, so which thread builds them does not
   matter. `LoadStats` already records each table once.
5. **The raw cells of a table are dropped once its rows are built,** when only one engine of the run reads the test
   data. When another engine may build the table again (the extra pass of ADR-063 gets a fresh engine), the cells
   stay. I will measure this step on its own; it can be rejected without touching the others.
6. **Carried starts are frozen states on the same base.** The end state of a carry chain member (ADR-066 amendment
   8) and every start the lane or a run ahead needs is frozen the same way. It shares every unchanged chunk with the
   install baseline and the carry before it, and a worker takes it as an `Arc`. The `RowDelta` cache, the whole-bytes
   starts and the start a worker holds (`lane_starts.rs`: `Deltas`, `Starts`, `Held`, `DELTA_BUDGET`) stay only for
   a state that `freeze` refuses.
7. **Commit is unchanged.** `Commit` copies the session's maps into the session's own committed maps
   (`MemStore::commit`); it never reaches the shared baseline. Under TestIsolation Codeunit or Function the session,
   or the snapshot of one test, is dropped at the end, so committed work is gone as on BC. Under Disabled the
   codeunit's committed state becomes the next carried start (decision 6), as today.
8. **Counters stay per session.** The next SystemId, row version (`SystemRowVersion`, timestamps), AutoIncrement
   values, number sequences, isolated storage and the last stamp are plain values in the shared baseline and are
   copied into each session at restore, as today. The stamps that name a table state (`next_stamp`) already come from
   one counter for the whole process, so two threads never give the same stamp to different rows.
9. **What stays per thread or per session:** the virtual tables provider, the order cache and the scan cache of the
   store, the access watch, the restored-changes log, temporary tables and all execution state. Locking is not
   modeled across sessions today (each session has its own database), and this does not change. Sharing the order
   cache between threads is a possible later change, not part of this ADR.
10. **Memory budget.** The fixed part of amendment 9 then includes the shared baseline and test data once; the cost a
   thread learns no longer includes a copy. A thread that ends for memory (amendment 10, decision 3) drops only its
   sessions, and a new thread starts without a decode.
11. **A private copy stays available for the differential test.** A run option (`RunOptions::private_baseline`,
   `AL_PRIVATE_BASELINE=1` read in the front end, ADR-006) makes each thread build its own copy as today. It exists
   for the conformance check below and is not documented for users.

## Consequences

- Only time and memory change (ADR-009). The rows, their order, the counters and every outcome are the same as with
  a copy per thread.
- A test thread starts in microseconds instead of 0.1 to 0.5 s, and its memory is its sessions and its share of the
  carried state.
- The serve warm slot keeps one shared baseline instead of one per thread (`baseline_cache::read_kept`).
- Code that goes away when built: the per-thread decode in `corpus.rs` `thread_baseline`, most of `lane_starts.rs`,
  the per-thread `TestBacking` cache.
- This does not shrink the session state or the carried state of a long carry chain, which is most of a thread's cost
  on Microsoft's suites.

### Risks

1. **A write to a shared row in place leaks to other threads.** With no overlay there is no read path that can skip a
   layer, but code that changes a value inside a shared row through interior mutability would. Today that is a
   Blob's `BlobCell`: a read copies the Blob first (`copy_blob_db_row`), which the snapshots of one thread already
   rely on, but a new path that skips the copy would change the base. Across threads such a write would also panic
   (`BlobCell::borrow_mut` uses `try_write`) or change another session's data. Mitigation: `freeze` can store Blobs
   in a form that cannot be written, and the differential runs compare a fingerprint of the base at the start and
   the end of every run (debug builds and the CI tool below).
2. **Soundness of the `unsafe` wrapper** rests on the plain-value check of decision 3. A new `Value` variant that
   holds `Rc` and that the check accepts would be undefined behavior. Mitigation: the exhaustive `match`, a unit test
   per variant, and a review rule: any change to `freeze` needs the owner.
3. **The cache of decision 4 must stay keyed by the test data and the engine.** A run with two engines (ADR-063) or
   a warm server with a changed program must not reuse rows built for another program. `Backing::identity` already
   includes the engine id.
4. **Lock contention.** The first touch of a table by several threads waits on one `OnceLock`, and `Arc` reference
   counts on shared chunks are atomic. I expect this to cost less than the copies it removes, and the build measures
   it (`--timings`).
5. **The saving on a large partner backup is an estimate** (about 3 KB per row touched). The first build is measured
   on the stress suite with a large backup before it is merged.

## Conformance

- **Differential conformance (the main check).** `tools/shared_vs_private.sh` runs each suite twice, with the shared
  baseline and with `AL_PRIVATE_BASELINE=1`, and compares the JUnit files test by test (outcome and message): the
  corpus on every BC major, Tests-ERM-Purchase, Tests-ERM-Sales, Tests-SCM, the stress suite, NP Retail, and one run
  with `--test-data` and `--companies all`. Zero differences is the gate, as for `tools/fresh_vs_grown.sh`
  (ADR-063). The known exception stays: a test that runs into its time limit depends on load. CI runs it nightly and
  on a push to main.
- **Base unchanged.** Debug builds and the tool check that the fingerprint of the shared baseline
  (`Baseline::fingerprint`) is the same at the end of the run as after the install code.
- **Unit tests** (red first): a write, delete, rename, `DeleteAll`, `Commit` and Blob write in a session on one
  thread are not seen by a session on another thread started from the same baseline; `freeze` refuses a row with a
  value that is not plain, and the run then uses a copy per thread; a carried start shared by two workers gives each
  the same rows; two threads that touch the same test data table build it once; a static assertion that
  `SharedBaseline` is `Send + Sync`.
- **Memory.** `--timings` prints the size of the shared baseline and of the test data rows once, and the per-thread
  cost without them. The measurement above is repeated on the first build.
