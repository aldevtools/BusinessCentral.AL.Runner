# ADR-022: Compile cache: one entry per dependency set (layers since 2026-10-15)

- Status: Accepted (owner, 2026-10-14; the measurement below amended decisions 2 to 4 and the owner accepted the amendments)
- Date: 2026-10-14
- Amends: ADR-013 (decision 6: its "persistent cache only if the warm start needs it" becomes a design; its
  "lowered IR is not cached" is lifted for dependency apps), ADR-016 (the deferral of persistent dependency
  caches, PERF-2)
- Related: ADR-004 (snapshots), ADR-006 (core purity), ADR-009 (what may differ from BC), ADR-011 (IR),
  ADR-015 (version gates)

## Context

Today every run compiles the dependency apps again, demand-driven (ADR-013 decision 6): parse, check and lower
only what the tests reach. A full corpus run with Microsoft's apps takes seconds. The owner's performance goal
(owner-review, 2026-10-14) puts caching last, after the work itself is cheaper. This ADR is that last step, designed
now so the measurements can decide whether to build it. No code before the measurement section below is done and the
owner accepts.

Two facts shape the design:

- **Microsoft ships hotfixes, sometimes several per day.** An entry is valid for one exact app version only. A
  cache that is keyed by the whole stack is rebuilt after every hotfix, and Base Application is the large part.
- **AL Runner v2 restored stale rows silently.** Its install-baseline cache key missed an input (the company
  normalization rule set), so a baseline captured under old rules was restored into a run that asked for new
  ones. Its remedy is a manual rule: bump `RuleSetVersion` (`running-ms-test-buckets` skill). Here the key must
  be unable to miss an input by construction (decision 4).

How BC loads, for contrast: the NST loads the metadata of all objects and syncs the SQL schema when it starts. This
runtime has no SQL schema step, so a cache can be opened by reading an index and decoding only what a test touches.

## Decision

1. **Caching is on by default.**
   - On: the runner compiles every Microsoft dependency app completely (eager, no demand pruning) once per app
     version, stores the result, and every later run loads it.
   - Off (`--no-compile-cache`, `ALRUNNER_NO_COMPILE_CACHE=1`; `--no-cache` implies it): the demand-driven partial
     compile of ADR-013 stays as it is. It is the reference path and the fallback.
   - Only dependency apps are cached. The workspace apps (the code under test) compile on every run.

2. **One entry per dependency set** (owner, 2026-10-14, open question 5; design A of the measurement). The set is every
   non-workspace app of a run, in dependency order. Workspace apps are ordered after them in the program, so the
   position of every dependency object and every base field is the same in every run with that set and needs no
   remap. (Per-app entries were measured and do not work: the IR refers to objects and fields by position. See
   "Can lowering be done per app?".) A customer extension never invalidates the entry. A Base Application hotfix
   rebuilds the whole entry, which is 79 % of the objects under either unit. An entry holds:
   - a header: magic, format integer, the key (decision 4), a checksum of the index;
   - an object index: object type + id -> offset and length of its block;
   - the event-subscriber index, precomputed (publisher -> subscribers), by name;
   - the member and type tables the workspace is checked against (fields, enum values, procedure signatures,
     extension fields the app adds to other apps' tables);
   - one shared zstd dictionary (level 3, about 512 KB) for all blocks;
   - **one block per object**: its lowered IR (serde + postcard), compressed on its own with zstd level 3 and the
     shared dictionary, with its own checksum. The block lists the objects whose extension or implementer set it
     embeds (the "whole-program facts" of the measurement: `InterfaceCall::targets`, `InterfaceTest::objects`,
     `EnumImpl::by_ordinal`, table and page extension lists, merged page controls). **A block whose listed object is
     extended or implemented by a workspace object is not loaded; it is lowered again from source with the demand
     path.** About 4 % of the objects are affected;
   - per routine, the refusal reason if lowering refused it (decision 7).
   - No string table (the dictionary reaches nearly the same size with no format change). No call graph in the
     blocks (three quarters of the raw bytes; derived from the IR where `baseline.rs` and the explanations need it).
     No level 19 (8 % smaller, not worth the time). No source positions (dependency IR has none).
   - The entry is about 16 to 19 MB per BC build.

3. **Loading costs as little as possible.** (The first implementation keeps three parts of this for later; see
   "Not done" below: the event index and the member tables are not stored, the sources of the dependency apps are
   still read, and the blocks are decoded at the start, not on first touch.)
   - Open = read the header and the index. Nothing else.
   - The event index and the member tables are used as stored. They are not rebuilt from objects.
   - The objects the install baseline reaches are decoded in parallel at start (4 threads: 70 to 90 ms), because every
     run reaches about 3 100 of them. A block is otherwise read, checked and decoded the first time a test touches its
     object. An object no test touches is never decoded. Per-routine laziness is not needed. `Program` therefore gets lazily filled object slots (`OnceLock`, because test codeunits run on worker
     threads). This changes how the engine reaches `Program::objects` and needs an accessor (ADR-011, additive).
   - The format code is a core crate over bytes (ADR-006). The file access (read, rename, directory) sits next to
     the existing download cache in `al-deps/src/cache.rs`.

4. **The key** has five inputs. Every one must be in it, and a test proves each one misses when it changes:
   1. app id and 2. app version, for every app of the set, in order;
   3. a hash of each app's source. For an artifact the hash is made once when the artifact is stored (ADR-013
      decision 13 keeps CRC-32 and size per entry). For `.alpackages` and `--deps` files it is a hash of the zip
      central directory (name, size and CRC-32 of every entry), which needs no data read;
   4. the target BC version, with the runtime version (`Target`, ADR-015: gates change lowering);
   5. the runtime build id. Any compiler change must invalidate. Decided here: the hash of the running executable
      (it cannot miss a crate; a hash of source files can). Measured at 6 to 8 ms; there is no build-script fallback.

   The key of the entry is the ordered list of these per-app inputs plus inputs 4 and 5. Any hotfix of any app in the
   set is a miss and rebuilds the whole entry. The format integer is part of the key as well; a mismatch is a miss and rebuilds, never an error (ADR-016).

   **No input can be forgotten.** The key is one struct. The hash function destructures it field by field without
   `..`, so adding a field that is not hashed does not compile. The install-baseline key (if built) takes the key of
   the entry plus the test-data and normalization inputs, built the same way.

5. **Writing is safe and local.**
   - Write to a temporary file in the cache folder, then rename. A reader sees the whole old file or the whole new
     file. Two parallel runs may both build the same entry; the last rename wins, and the bytes are the same
     (the build is deterministic, a test checks it).
   - A damaged file is refused and rebuilt, never used: the header and index checksum are checked at open, a block
     checksum at first touch. A failed check (named reason on stderr) ignores the entry, the run
     continues from a rebuild (or the demand path), and the damaged file is replaced. A damaged cache never changes an
     outcome and never fails a run. `alrunner cache verify` reads every block, for CI.
   - **Licensing (ADR-013, "cache only"):** the cache is a local file made on the user's machine from source the
     user downloaded. Nothing compiled from Microsoft code is committed, released, uploaded as a release asset or
     shared outside the user's own CI cache.
   - Location: `<cache root>/compile/` (owner decision 3 below).

6. **CI** caches the cache files between pipeline runs with `actions/cache` (owner, 2026-10-14: short-lived key per BC build; a
   separate step builds or verifies the cache before the measured run, so the measured run is always a hit; the job
   summary reports the times with the cache on and the cache build time separately).
   - The `actions/cache` key is built from the same inputs as decision 4: the output of `--print-builds`
     (the app versions), the runtime build id and the format integer. No `restore-keys`: an entry of another version
     is never usable.
   - Entries are short-lived (hotfixes), so before saving, `alrunner cache prune` deletes every entry the current
     stack does not use. GitHub evicts caches unused for 7 days and caps a repository at 10 GB.
   - **Step order** in the pipeline:
     1. restore the cache;
     2. `alrunner cache build` builds missing entries and verifies existing ones, and prints its own time;
     3. the measured run, with a flag that turns any cache miss into an error (`--require-compile-cache`), so the
        reported run is always a cache hit;
     4. the outcome comparison against `--no-compile-cache` (decision 7);
     5. the job summary: run time with caching on, and the cache build time as a separate line;
     6. prune and save the cache.

7. **Refusals found by the eager compile** are stored per routine in the entry. They only matter when a test reaches
   that routine; then the test is refused with the stored reason, as today. A refusal in code nothing reaches does not
   change any outcome. The evidence that a full compile and a partial compile give the same outcomes is the
   existing demand-versus-eager comparison (ADR-013, "The permanent check": `tools/compare_junit.py` over a second run
   with `AL_EAGER_DEPENDENCIES=1`). That comparison stays, and gains a third leg: cached against uncached.

## Measurement (spike done 2026-10-14)

A throwaway branch (`cache-spike`, not merged) serialized the lowered program of an eager compile of the whole
dependency stack and read it back. Nothing here is a design; it is what the numbers are. Method notes:

- **Stacks.** BC 29.0.54011.56262: System, System Application, Business Foundation, Base Application,
  Application (the corpus `.alpackages`). 9 683 dependency objects, 148 860 routines, 4 201 of them refused.
  The Tests-ERM closure is BC 27.5.46862.48827 (the only artifact here with the `Tests-*` sources): 9 919
  dependency objects, 153 326 routines.
- **Machine.** 12 cores shared with other agents. Check, lowering and test threads: 4 each. Load average is
  noted per number. Run time depends strongly on load (the same all-32-buckets run took 31.5 s at load 20 and
  12.7 s at load 7), so only numbers from one session are compared. CPU time (`user`) is the steadier figure.
- **Codec.** `postcard` for the IR. Round trip checked: 0 of 9 683 objects differ after decode.

### 1. Size (BC 29 stack, dependency objects only)

| What | Size |
|---|---:|
| postcard, raw | 52.4 MB |
| bincode 1.3, raw | 113.2 MB |
| postcard, per-object zstd level 3 | 15.6 MB |
| postcard, per-object zstd level 19 | 14.3 MB |
| postcard, per-object LZ4 | 21.4 MB |
| postcard, whole stream zstd 3 (no per-object split) | 9.3 MB |
| per-object zstd 3 with one shared 512 KB dictionary | 12.3 MB (+0.25 MB dictionary) |
| call graph (`Lowered::calls`), raw / per-object zstd 3 | 151.2 MB / 6.3 MB |
| object index (`ObjectSym` of all dependency objects, with 3 375 event subscribers) | 3.09 MB raw, 0.49 MB zstd |
| member tables (44 254 fields, 97 549 procedures) | 13.4 MB raw, 2.35 MB zstd |

- **The call graph is three quarters of the raw bytes and does not belong in the blocks.** It is derivable from the
  IR. Its consumers are `baseline.rs` (static reach of the install roots) and the refusal explanations. Without it
  the blocks are 52 MB raw instead of 204 MB. The numbers above leave it out unless stated.
- **Level 19 saves 8 % over level 3 and is not worth it.** LZ4 is 37 % bigger than zstd 3.
- **Per-object compression costs 67 % against one stream** (median object 2 KB, p90 13 KB, largest 246 KB). A
  shared dictionary takes back most of it. The cost is an index entry per block (0.52 MB for 9 683 blocks).
- **Strings are two thirds of the raw bytes.** 1.94 million strings, 35.2 MB, in the blocks; 252 469 distinct
  strings, 10.5 MB (2.0 MB zstd 3). A shared string table would give about 20 MB of blocks plus the 10.5 MB table
  before compression. It would not make decode faster (decode time is allocation, see 2). The dictionary reaches
  nearly the same size with no format change. Recommendation: dictionary, no string table.
- **Dependency IR carries no source positions** (0 of 53 376 positions in the program belong to a dependency
  object). `Program::positions` and `sources` are workspace-only and stay out of the entries. (Superseded by ADR-041,
  accepted 2026-10-17: the entries now hold the positions of dependency routines, and the key records `AL_NO_DEP_POSITIONS`.)
- **One entry (blocks with dictionary, object index, member tables, block index) is about 16 to 19 MB per BC build.**
  The Tests-ERM closure is the same size (15.9 MB blocks).

### 2. Decode time (one thread, warm file cache, cold process)

| What decoded | Blocks | Postcard bytes | Time | Of which zstd | Of which postcard decode |
|---|---:|---:|---:|---:|---:|
| Open: block index (9 683 entries) | | 0.52 MB | 1.2 to 1.3 ms | | |
| Corpus run: objects the demand compile activates | 3 601 | 28.9 MB | 269 to 283 ms | 62 to 65 ms | 201 to 213 ms |
| One corpus test file (`--file`) | 3 176 | 26.3 MB | 239 to 248 ms | 54 to 56 ms | 181 to 187 ms |
| Small app, one test (install baseline reach) | 3 105 | 26.1 MB | 239 ms | 54 ms | 180 ms |
| Tests-ERM (27.5) | 5 415 | 39.0 MB | 361 ms | 89 ms | 264 ms |
| Everything, one thread | 9 683 | 52.4 MB | 471 to 488 ms | 129 to 133 ms | 330 to 343 ms |
| Everything, 4 threads | 9 683 | 52.4 MB | 188 to 190 ms | | |

- The earlier estimate (load 50 to 150 ms for about 35 000 routines) was low by a factor of two to three on one
  thread. The 35 298 routines of a corpus run sit in 3 601 objects, and an object decodes whole.
- **Decode is allocation work, not decompression.** zstd is a quarter of the time. About 28.9 MB of postcard becomes
  280 MB of heap (the uncached corpus run peaks at 1 394 MB in total).
- **A single test of a small app reaches 3 105 objects**, the same as a whole corpus run, because the install
  baseline (`Company-Initialize` and the install codeunits) is a root of every run. So the cost of a small run is
  the baseline's reach: 0.24 s decode on one thread, then 0.2 s to run the baseline.
- The "reached" objects are the demand compile's active set (`SCHEMA`, `READ` and `FULL` levels). A cached run
  that decodes at first touch decodes at most this many.
- Object index decode: 15 to 22 ms; rebuilding `by_name`, `by_id` and `subscribers_of` from it: 6 to 12 ms.
  Member tables decode: 42 to 69 ms for all objects, against 400 to 785 ms to compute them today (measured
  warm, after the files were parsed; cold it includes the parse). The event index is a by-name list per object
  (0.21 MB); the global map is rebuilt in a few milliseconds and is not stored.

### 3. Open time

Index 1.3 ms, object index 15 to 22 ms plus maps 6 to 12 ms, member tables 42 to 69 ms (or lazily per object):
**25 to 35 ms with the member tables lazy, 70 to 100 ms with everything decoded.** Today a run with dependencies
reads and inflates the dependency sources ("load apps and dependencies", 0.11 to 0.18 s) and scans 108 MB of source
(62 to 180 ms wall, 9 746 files). A cache hit skips both.
The member tables and the event index can be stored, but not "as is": `FieldInfo::declared_in`, `ProcSig::declared_in`
and `EnumImpl::declared_in` are object indices, and `on_validate`, `on_lookup` and `ProcSig::offset` are byte offsets
into source files. Types (`Ty`) refer to objects by name, so they store as they are. The offsets are used only
when an object is lowered from source.

### 4. Eager compile (the one-time cost)

| Run | Compile wall | Peak memory | Load |
|---|---:|---:|---:|
| Corpus stack, BC 29, 4 threads | 8.1 to 8.8 s | 3.3 GB | 8 |
| Corpus stack, BC 29, 1 thread (`AL_CHECK_THREADS=1 AL_LOWER_THREADS=1 RAYON_NUM_THREADS=1`) | 9.5 s | 3.2 GB | 6 |
| Corpus stack, BC 29, 4 threads | 13.4 s | 3.3 GB | 18 |
| Tests-ERM closure, 27.5, 4 threads | 9.6 s | 3.8 GB | 11 |
| Tests-SMB closure, 27.5, 4 threads | 9.2 s | 3.4 GB | about 8 |

The estimate was 10 to 30 s per Base Application version; it is **about 9 s on a quiet machine and 13 s on a busy
one**, and it is almost the same on one thread (the eager check is serial; lowering is parallel). Serializing and
compressing the blocks adds roughly 2 s (the spike's pass, which does five codecs, takes 3.3 s on 4 threads). Memory:
3.3 to 3.8 GB, so the builder needs a cap of at least 5 GB.

### 5. Peak memory of a cached run

Not measured as one run (no cached run exists). From the parts: decoded IR of the objects a corpus run reaches
280 MB (measured in the reader), plus the workspace compile (197 MB for the corpus alone, measured with `--no-deps`),
plus the object index and member tables (about 50 MB decoded). About 0.5 to 0.6 GB against 1.4 GB uncached
(corpus), 1.2 GB for the small app. Estimate, not a measurement.

### 6. Hash costs

| Hash | Time |
|---|---:|
| sha256 of the running executable (14.8 MB, file cache warm) | 6.4 to 8.4 ms (read 2.6 to 12 ms) |
| blake3 of the same | 2.5 to 5.1 ms |
| central directory of `Base Application.app` (38.4 MB file, 8 350 entries, 0.83 MB directory): read, parse, sha256 | 3.7 ms + 1.1 ms |

Decision 4.5 stands: **hash the executable; the build-script fallback is not needed.** Total key cost for the
five apps is under 20 ms.

### 7. Can lowering be done per app? (the go / no-go question)

**No, not as decision 2 is written.** The IR refers to other objects by position in the one global
`Program::objects` list, and to table fields by position in a list that other apps' extensions grow. It also holds
facts about the whole program. Findings, each measured or shown by a test:

1. **Object references are positions in `Program::objects`.** `Expr::Call::object`, `InterfaceCall::targets`,
   `RtType::{Record, Codeunit, Page, TestPage, Query, Interface, Enum}`, `Const::Enum`, `Table::extensions`,
   `Field::on_validate` and friends, `Page::extensions`, `Control::owner`, `Control::part_page`,
   `RunObject::object`, `SrcPos::object`, report, query and xmlport `table`/`base`/`extension` fields: about 35 places
   in `al-ir`. The workspace apps come first in the list today, so **every dependency object moves by the size of the
   workspace**: Tests-ERM against Tests-SMB on the same artifact moves all 9 919 dependency objects by 245, and only
   1 628 of them (16 %: enums, permission sets, profiles, `object` placeholders) have byte-identical IR. A table's
   IR differed in 936 places for Customer (table 18), a codeunit's in 4 717 (Sales-Post). In the 26 objects
   diffed field by field (codeunits, tables, pages, reports, queries, an xmlport), **every difference was a shift by
   245**; nothing else changed.
2. **Field references are positions in the table's merged field list.** `Place::Field(place, usize)`,
   `Const::FieldPos`, `Table::keys`, relation arms, `DataItemLink`, page control bindings. The order is: the table's
   own fields, the six system fields, then the fields of extensions in workspace object order
   (`members.rs`, `merged_members`). A test on the spike branch compiles app B (a table extension that adds `BF`,
   plus a codeunit that assigns `Rec.BF`) alone and next to app X, which extends the same table first: `BF` is
   field 7 in one and field 8 in the other, and the codeunit's IR differs
   (`Assign(Field(Local(0), 7), ..)` against `Field(Local(0), 8)`). `Place::Field` does not name the table, so a remap
   has to type the place from the slot it starts at.
3. **Other apps' contributions are merged into an object's IR**, so an object block is not a function of its own
   app: `Table::fields` and `keys` (extension fields), `Table::extensions`, `Field::ext_before_validate` and
   `ext_after_validate`, `Page::controls` (page extension controls are inserted into the base page's list, which
   shifts the control indices that `Expr::Control` uses), `Page::extensions` and `ext_triggers`, enum values,
   report extensions. And some **dependency routines embed the fan-out over all apps**: `InterfaceCall::targets`
   (every implementer), `InterfaceTest::objects`, `EnumImpl::by_ordinal` (every enum value, extensions included).
4. **How much of this exists in the stack.** In the Tests-ERM closure there are 67 extension objects that cross an
   app boundary (11 page and 4 table extensions of Base Application on Business Foundation objects, 17 page, 3 table,
   10 enum and 2 permission set extensions on System Application, plus the test apps), and 315 extensions inside their
   own app (these are fine). Dependency IR contains 537 `InterfaceCall`, 13 `InterfaceTest` and 156 `EnumImpl` sites in
   about 200 objects, and 210 tables and pages list extensions. **About 400 of 9 919 objects (4 %) carry whole-program
   facts.** Every other block depends only on object positions and on field positions of base fields.

What it would take:

| Design | What changes | Estimate (mine) |
|---|---|---|
| **A. One entry per dependency set** (all non-workspace apps of a run, in dependency order). Workspace apps are ordered after them. | Dependency object indices and base field positions are then the same in every run with that set, so **no remap**. Needs the app order changed (workspace last; `runs[0].app = Some(1)` and the app index logic move), blocks that list the objects whose extension or implementer set they embed, and a load step that re-lowers (demand path, from source) the blocks whose listed objects the workspace extends or implements. Fallback to lazy parse of the source for the few objects the workspace checker reads directly. | 2 weeks |
| **B. One entry per app** (decision 2 as written). | All of A, plus: stable references (a typed remap visitor over about 80 IR enum variants for object indices and field positions, with a field-id table per table), a canonical extension order, and a merge step for fields, controls, enum values and subscribers contributed by other entries. | 4 to 5 weeks, with a risk of subtle wrong-field bugs that the corpus would catch only partly |

**Per app buys little here.** Microsoft's apps move together per BC build. Base Application is 7 853 of 9 919
objects (79 %), System Application 1 244 (13 %). A hotfix of Base Application rebuilds 79 % of the set under A as
well. The only gain of B is keeping System Application and Business Foundation (14 %) across a Base Application hotfix.

### 8. Projected run times

"Today" is origin/main (3c35b4e) in one session at load 6 to 7, two runs each, `Time:` line of
`--timings`; the spike's cached figures are built from measured parts as shown. Cached = workspace compile
(measured as the same run without dependencies, or the workspace share of check and lowering) + open + decode +
install baseline + tests. The workspace share is the measured share of check and lowering thread-seconds spent on
workspace objects (corpus: not separable, taken from `--no-deps`; Tests-ERM: 24 %; 32 buckets: 53 %).

| Run | Today (total, user CPU) | Cached, projected | Factor |
|---|---|---|---:|
| Small app, one test (`alrunner test`, BC 29) | 2.10, 2.12 s; 6.0 s CPU | 0.35 to 0.55 s: open 0.03 + decode 0.07 (4 threads) to 0.24 (1 thread) + baseline 0.20 + test | 4 to 6 |
| Corpus, BC 29, with Microsoft's apps | 2.92, 3.88 s; 9.3 and 11.9 s CPU | 1.7 to 2.1 s: workspace compile 0.35 to 0.5 + open 0.03 + decode 0.1 to 0.25 + baseline 0.2 + tests 0.86 to 1.09 | 1.4 to 2.3 |
| Tests-ERM (27.5) | 6.13, 5.01 s; 17.6 and 14.3 s CPU | 2.3 to 2.8 s | about 2 |
| All 32 buckets, one process | 13.17, 12.69 s; 38.3 and 34.8 s CPU; 5.9 GB | 8.3 to 9.5 s (test apps are 91 MB of source and stay) | 1.4 |

The earlier estimates (corpus 1 s, single test 0.3 s, 32 buckets 6 to 8 s) are not reached. The floors are the
tests that run only with Microsoft's apps (0.86 to 1.09 s in the corpus), the workspace compile, and the baseline run.
The corpus without Microsoft's apps takes 0.45 to 0.53 s in total. The break-even is 6 to 12 runs: the build is
about 12 s, the saving 1 to 2 s per corpus run or about 1.6 s per small run.

Uncached numbers will drop when `perf-3` merges (-18 % instructions at 4 threads, -29 % at 12), which narrows the
corpus gap more than the small-app gap, because the small-app cost is the dependency work perf-3 does not remove.

### Go / no-go

The rule was: build only if a warm cached run is clearly faster than the best uncached run, on the corpus and one
bucket. Read against the numbers:

- **Clearly faster:** a small app with one test (4 to 6 times) and Tests-ERM (about 2 times).
- **Faster, less clearly:** the corpus (1.4 to 2.3 times, the two runs differ by 1 s; closer after perf-3 merges) and all 32 buckets (1.4 times).
- **Per-app entries (decision 2): no-go as written.** The unit has to be the dependency set (design A) or the work
  grows to design B.
- Not tried here: making the uncached path cheaper for a small app. Its cost is the install baseline's reach (3 105
  objects, about 6 CPU-seconds), so a cheaper baseline would help the uncached path and the cached one alike.

### How to reproduce

Branch `cache-spike` (commit "SPIKE (do not merge)", worktree `wt-cache-spike`). Environment variables, all read by
`al-testrun/src/runtime.rs` and `spike.rs`: `AL_EAGER_DEPENDENCIES=1` with `AL_SPIKE_WRITE=FILE` runs the eager
compile, prints every size above, writes the blocks, and exits; `AL_SPIKE_DEMAND=FILE` writes the demand compile's
active objects of a normal run; `AL_SPIKE_NS=1` prints thread-seconds of check and lowering split by workspace and
dependency; `AL_SPIKE_LOADSPLIT=1` times the workspace load per app kind; `AL_SPIKE_HASHES=FILE` writes a hash per
dependency object (to compare two closures). `cargo run --release -p al-test-cli --example cache_spike_read -- FILE
DEMAND.json...` opens a written file and times the decode. The corpus run is
`al-test-cli --root . --bc-version 29`; the bucket runs are the recipe in `docs/alrunner-buckets.md`.

## Amendments adopted (owner, 2026-10-14)

The owner accepted all seven amendments the measurement proposed. They are folded into the decisions above:
(1) the entry is per dependency set; (2) blocks list the objects whose extension or implementer set they embed and are
lowered again when the workspace extends or implements one; (3) no call graph in the blocks; (4) per-object zstd
level 3 with one shared dictionary, no string table, no level 19; (5) the objects the baseline reaches are decoded in
parallel at start; (6) the executable is hashed, no build-script fallback; (7) the first-run cost is 9 to 13 s and
5 GB of memory in the builder, and an entry is 16 to 19 MB per BC build.

## Open questions (all answered, see "Owner decisions")

Each was one decision. The recommendation came first.

1. **How are blocks serialized?**
   - Recommended: **serde + postcard (or bincode) + per-object zstd.** The IR already derives serde. Small change,
     small risk, format versioned by the integer. Zero-copy gives nothing here: a compressed block must be
     decompressed into memory before it can be read.
   - Alternative: rkyv-style zero-copy. Needs derives and alignment on every IR type, and works only for
     uncompressed data (index and tables, not the blocks). Gain is a faster open of the tables. Pick it only if the
     measurement shows the tables dominate.
2. **Is the install-baseline snapshot in version 1?**
   - Recommended: **no, later.** Version 1 caches compiled code. The baseline is the part where the v2 failure
     happened, and it also depends on test data and normalization inputs. Add it after the code cache is proven,
     with its own key test.
   - Alternative: yes. Saves the install run (`Company-Initialize` and install triggers) on each process start,
     at the price of the wider key now.
3. **Where does the cache live?**
   - Recommended: **`<cache root>/compile/` with the root the download cache already uses** (`--cache-dir`,
     `ALRUNNER_CACHE`, default `~/.cache/al-runtime`). No new variable; one folder to delete or save in CI.
   - Alternative: its own variable (`ALRUNNER_COMPILE_CACHE`). Lets CI keep it apart from the artifacts. Costs a
     second setting to explain.
4. **WebAssembly: cache or none?**
   - Recommended: **none in version 1.** The zstd crate needs a C compiler and is not a clean WebAssembly build;
     the host has no agreed storage. WebAssembly keeps the demand-driven compile.
   - Alternative: the host hands the format code a byte source (`ByteSource`, as in `al-bcdb`), with a pure Rust
     decoder (`ruzstd`, slower, or LZ4). The format code is already core and OS-free, so this stays possible.

5. **What is the unit of an entry?** (added after the measurement)
   - Recommended: **the dependency set** (all non-workspace apps of a run, in dependency order, workspace apps ordered
     after them). No remap of object or field positions, about 2 weeks, and a Base Application hotfix rebuilds 79 % of the
     set under either unit.
   - Alternative: **one entry per app** with stable references and a merge of other apps' contributions. About 4 to 5
     weeks, riskier, and it keeps only System Application and Business Foundation (14 % of the objects) across a Base
     Application hotfix.
   - Alternative: **do not build.** Pick this if a corpus run 1.4 to 2.3 times faster and a bucket run 1.4 to 2 times
     faster is not worth the second path (decision "Two paths stay"). The small-app run is 4 to 6 times faster.

## Owner decisions (2026-10-14)

The owner accepted every recommendation above and answered question 5:

1. Blocks are serialized with serde + postcard (or bincode) and compressed per object with zstd.
2. The install-baseline snapshot is not in version 1.
3. The cache lives in `<cache root>/compile/`, under the root the download cache already uses.
4. WebAssembly has no compile cache in version 1.
5. **One cache entry per dependency set**, not per app (design A). Workspace apps are ordered last.
6. CI caches the cache files with `actions/cache`; a separate step builds or verifies the cache; the measured run is a
   cache hit; the job summary reports the run time with the cache on and the cache build time apart.

Decided without owner review (2026-10-14): see `docs/owner-review.md` (the file layout of an entry, the way a block is
lowered again from source, the corruption handling, and the CLI spelling).

## What was built (2026-10-14, branch `compile-cache`)

- **Crates.** `al-cache` (core, no OS access): `Key`, the entry file, `build` and `Entry`. `al-sema/provided.rs`:
  `DepFacts`, `DepBlock`, `DepSource`, `relower_set`. `al-testrun/compile_cache.rs`: the key of a run, reading, writing
  and building the file. `lower::lower_for_cache` builds an entry (the complete compile of the dependency apps alone);
  `lower::compile_loaded_cached` is a compile that takes those objects from it.
- **The file.** Header (magic, format, key digest, where the index is, hashes), the zstd dictionary, one block per
  dependency object (postcard, zstd level 3 with the dictionary and a content checksum), the index last (the facts of
  every object and where each block sits). Opening reads the header, the index and the dictionary and checks their
  hashes and the length; a block is checked when it is read.
- **What a block is.** The lowered `ir::Object`, why each routine cannot run, and the direct calls of each routine.
  What a variable of a page, report or xmlport type drives is added when the call graph is asked for
  (`Lowered::calls_of`), from the page extensions and parts of the run. The facts beside the blocks are what the
  lowering of other objects reads: the table behind a page, why a page cannot be driven, its parts, the numbering of
  routines, the layout of a query.
- **What makes a block unusable.** A block records the objects whose extensions or implementers it embeds: itself,
  the interface of an interface call or test, the enum of an enum cast, a page or report a variable names (with the
  parts of a page), a page whose control index it uses. The workspace "touches" the objects its own objects extend
  or implement (and the interfaces they extend). Every block that lists a touched object is lowered again from source,
  with the objects that extend each other. A test per case compares the whole `Program`, the refusals and the call
  graph of the cached compile with the complete compile (`provided.rs`), and `demand_audit.rs` runs its 78 probes
  in the cached mode against the complete compile.
- **Order.** `CorpusRuntime::try_run_suites_with` puts the dependency apps first in the program and maps the app
  indices of the caller (`SuiteRun::app`, `RunOptions::only`, `TestResult::app`) both ways.
- **The key** holds the format integer, the sha256 of the executable, the target, the preprocessor symbols, and per
  dependency app its id, name, dependency ids, publisher, version and a hash of every path and byte of its source. For
  an app from a package the source hash is made from the files already read (about 30 ms for Business Central); the
  hash of the zip directory that the first version of this ADR planned needs a change in `al-deps` and is left for
  when the load of the sources is skipped.
- **Outcomes.** The cached compile is the complete compile for the dependency apps. It gives the same outcome as the
  eager run for every corpus test and the same as the demand-driven run except in `tools/demand-eager-known-differences.txt`
  (a test the complete compile refuses before it starts because a page variable's call graph holds every routine of
  the page). Those five tests pass in the uncached run and are refused with the cache. `al-test-cli` therefore uses the
  cache only with `--compile-cache`; the ratchet runs without it. `alrunner test` uses it unless
  `--no-compile-cache` or `ALRUNNER_NO_COMPILE_CACHE=1`.

### Measured with the implementation (2026-10-14, release build, 12 cores shared with other agents)

Two sessions under `flock /tmp/al-perf.lock`, each back to back: uncached, cold (the entry is deleted, so the run
builds it), hit, hit, uncached again. Wall seconds, CPU seconds (user + system), and the peak resident size of the
largest process. The load average is in `docs/adr/022-measurements.txt` (4 to 9 for the first three rows, 8 to 15 for
the last), so compare the CPU column and the pairs, not rows. The first three rows are from the session at the final
commit (`measure2`), the 32 buckets from the session before the last two merges of `main` (see the note below the table).

| Run | Uncached (two runs) | Cold: build + run | Of which the build | Hit (two runs) | Memory: uncached / cold / hit |
|---|---|---|---|---|---|
| Small app, one test (`alrunner`, BC 29, no test data) | 1.60 s, 1.51 s; CPU 7.1, 7.0 s | 5.33 s; CPU 23.6 s | 4.51 s | 0.74 s, 0.75 s; CPU 2.6 s | 1259 / 3922 / 896 MB |
| Corpus, BC 29, Microsoft's apps (`al-test-cli`) | 2.24 s, 2.48 s; CPU 12.2, 12.5 s | 6.49 s; CPU 28.3 s | 5.00 s | 1.46 s, 1.51 s; CPU 7.1, 7.4 s | 1445 / 3766 / 1232 MB |
| Tests-ERM, BC 27.5, test data (`alrunner`) | 12.02 s, 11.72 s; CPU 39.1, 40.7 s | 13.99 s; CPU 50.7 s | 5.76 s | 8.09 s, 8.34 s; CPU 27.8, 28.2 s | 3107 / 4183 / 2112 MB |
| All 32 buckets, one process, test data (earlier session) | 33.4 s, 30.4 s; CPU 130.8, 125.5 s | 29.2 s; CPU 124.9 s | 4.71 s | 35.0 s, 34.8 s; CPU 118.5, 116.1 s | 6310 / 4963 / 4710 MB |

Note on the 32 buckets: `main` as of the last merge needs more than 18 GB for the combined run with test data and is
killed, with or without the cache (the binary built from `main` in the main checkout at 18:41 too), so the row is from
the session before. It is a regression on `main`, not an effect of the cache.

- **Build.** 4.5 to 5.8 s and 3.8 to 4.2 GB, for an entry of 12.9 to 13.3 MB. The complete check of the dependency apps
  runs on all cores now (`Checker::check_objects_parallel`); before, it was serial and the build took 10 to 19 s.
- **Hit.** The file opens in 0.03 to 0.05 s (key, read, header and index). Reading the blocks takes 0.05 to 0.17 s
  on all cores (more when the machine is loaded). A small app runs 2 times faster than uncached, uses a third of
  the CPU and 30 % less memory. The corpus runs 1.6 times faster with 40 % less CPU. Tests-ERM runs 1.45 times faster
  (the 5 to 6 s of tests with test data do not change) with 30 % less CPU and a third less memory.
- **All 32 buckets: no gain in wall time.** The workspace holds 91 MB of test sources, which load, check and lower the
  same with and without the cache (load 2.5 s, check 1.3 s, lowering 1.1 to 1.5 s), and 19 s of the run is the
  tests with test data. The cache replaces the demand compile of the dependency apps (about 3 s on a quiet machine)
  by reading the blocks (0.15 s). The CPU time is 8 % lower, the peak memory 25 % lower.
- **Not reached: the projections of the measurement section** (small app 4 to 6 times, corpus 1.4 to 2.3 times). The
  sources of the dependency apps are still read, inflated, hashed and scanned on every run (0.2 to 0.35 s), every
  block of the objects the workspace does not change is decoded at the start (more objects than a run reaches), and
  the whole program is built (the engine reads every object). Skipping these needs the object index and the member
  tables in the entry, a workspace without dependency sources, and a lazily filled `Program::objects`; see "Not done".
- **Outcomes.** The program of the cached compile equals the program of the complete compile object for object
  (`provided.rs` tests, the 78 probes of `demand_audit.rs`), every corpus test of BC 27, 28 and 29 and of the OnPrem
  and isolation suites has the outcome it has in the eager run, and Tests-ERM with test data and all 40300 tests of the
  32 buckets have the outcome of the eager run (0 differences). Against the uncached run (demand-driven compile): the
  corpus differs in the 5 tests of `tools/demand-eager-known-differences.txt` (on all three versions), Tests-ERM in
  none, and the 32 buckets in 7 of 40300 tests (a test that passes or fails uncached is not run with the cache). The
  cause is the same as in the corpus: a routine that only the complete compile has is in the static call graph of a
  page variable, for example `Navigate.OnAction` through a `RunObject` action of a dependency page.

### Not done

- **Skipping the load of the dependency sources, and lazy objects.** The object index and the member tables are not in
  the entry; the sources of the dependency apps are read and scanned on every run; every needed block is decoded at
  the start. `Program::objects` is a plain list and the engine reads all of it (tables, the event registry), so lazy
  decode needs those whole-program passes stored in the entry first.
- **The same outcome as the uncached run for every test: done (2026-10-14, branch `zero-diff`, owner decision "there
  should be zero differences").** The static call graph of a page, report or xmlport variable (`lower.rs`,
  `driven_edges`) is a function of the sources (routine roles), not of what was compiled. A `TestPage` variable adds the
  page, its extensions and its parts; the `RunObject` targets of a workspace page, and of the page the variable names,
  are compiled but add an edge only for workspace pages. A `Page` variable of a dependency page adds no edge, and a
  platform method called on it (`Run`) reaches the page. A report or xmlport variable adds its triggers, and the demand
  compile reaches them whenever the variable is declared. `CurrPage` adds nothing. The corpus on BC 27, 28 and 29 and the
  suites have no difference between demand, eager and cached, and `tools/demand-eager-known-differences.txt` is empty.
- **`alrunner cache` (list, verify, prune)**; CI prunes with a shell step and verifies with
  `al-test-cli --build-compile-cache`.
- **The install-baseline snapshot** (owner decision: not in version 1).

Decided without owner review (2026-10-14):

- A damaged, cut or foreign file is ignored with a note, removed, and the run builds a new one (or, if the entry was
  read but a block fails, compiles without it). The first version of this ADR said the run stops. Ignoring cannot
  change an outcome and needs no manual step. `--require-compile-cache` makes any of that an error.
- The corpus tool is opt-in (`--compile-cache`), because its baseline is recorded under the demand-driven compile.
- A run that uses an entry sets its modification time; a run that builds an entry removes the entries nothing used for
  30 days (an app that changes on every run would otherwise leave 13 MB behind each time).
- `al-test-cli --build-compile-cache` builds or checks the entries and prints the build time; CI uses it. There is no
  `alrunner cache` command yet.
- The facts and blocks of an entry hold the dependency objects only; the sources of the dependency apps are still read
  and scanned on every run, and the blocks are all decoded at the start (in parallel).

## A pure test in an app that depends on Base Application (issue 20, 2026-10-14, branch `fast-pure`)

The owner's request (issue 20): a test that touches nothing from Base Application, in an app that depends on it,
should cost what it costs in an app without dependencies (0.16 s, 78 MB in the owner's measurement). The fixture is
the owner's setup: a demo app (`"application": "28.0.0.0"`, an enum and a codeunit of three procedures that use no
table) and a test app of 6 tests, run on BC 28.5 (`alrunner test demo test --no-test-data --filter
SmallCustomerIsBronze`).

### Where the time went before (1 pure test, release build)

| Part | Compile cache hit | Without the cache |
|---|---|---|
| Load the apps: read the four dependency `.app` files, inflate every source file | 0.14 to 0.26 s | 0.09 to 0.2 s |
| Key of the cache entry (sha256 of every source byte), read the 13 MB file, open | 0.04 s | not run |
| Scan of the 9 752 dependency objects (lexer, preprocessor, declaration scan) | 0.09 s | 0.09 s (in "load") |
| Decode and check every block (zstd, postcard, 9 752 objects, all cores) | 0.17 s | not run |
| Check and lower the workspace (7 routines), build the engine | 0.06 s | 0.04 s |
| Demand compile of what the install baseline reaches (421 routines of Base Application) | not run | 1.8 to 2.5 s check, 1.0 to 1.2 s lower |
| Install baseline run (17 roots) | 0.19 to 0.25 s | 0.3 to 0.4 s |
| Tests (one test, 0.00 s), two .NET helper processes started ahead, exit | about 0.1 s | about 0.1 s |
| Total | 0.95 s wall, 2.6 s CPU, 887 MB | 4.5 to 5.1 s wall, 16 s CPU, 1 905 MB |

CPU profile of a cache hit (`perf`, share of samples): lexer, preprocessor and declaration scan of the dependency
sources 28 %, inflate of the sources 9 %, sha256 of the sources 4 %, zstd and postcard decode 8 %, the rest spread.
After the baseline skip, with the cache off, the same profile reads: lexer, preprocessor, scan 60 %, inflate 21 %.
So once nothing else is left, the cost of a pure run is reading the dependency sources to learn what objects they
hold.

### What changed

1. The install baseline does not run when no selected test can observe it, and its roots are not compiled
   (ADR-013, "The baseline runs only when a selected test can observe it"). That removes the two largest rows.
2. A selection whose test codeunits look free of data (a text check) is compiled first the demand-driven way,
   without the cache. The cache decodes all 9 752 dependency objects (0.17 s, 440 MB) to serve a test that reaches
   7 routines; the demand-driven compile touches only those. If the rule then finds the tests may observe the
   baseline, the run goes on with the cache as before. `--require-compile-cache` keeps the cache path. A cold cache is
   not built for a pure selection (the build takes 5 to 6 s).
3. The two .NET helper processes are not started ahead of time for such a selection (they start on first use).

### Measured after (same fixture; wall seconds, CPU seconds, peak memory; medians of 7 back to back runs under
`flock /tmp/al-perf.lock`, load average 19 to 22 on a 12-core machine shared with other agents, so compare the CPU
column and the pairs, not the wall column)

| Run | Before (`AL_FORCE_BASELINE=1`, and the cache path) | After | 
|---|---|---|
| Cache on (default), 1 pure test | 1.04 s, CPU 2.57 s, 891 MB | 0.39 s, CPU 1.12 s, 399 MB |
| Cache on, 6 pure tests | 1.22 s, CPU 2.71 s, 891 MB | 0.58 s, CPU 1.37 s, 402 MB |
| `--no-compile-cache`, 1 pure test | 3.86 s, CPU 15.7 s, 1 909 MB | 0.44 s, CPU 1.14 s, 400 MB |
| `--no-compile-cache`, 6 pure tests | 3.89 s, CPU 16.0 s, 1 917 MB | 0.36 s, CPU 1.25 s, 400 MB |
| Same 6 tests, app without `application` | 0.02 s, CPU 0.02 s, 14 MB | (the owner measured 0.16 s, 78 MB) |

(The "before" rows are the same binary with `AL_FORCE_BASELINE=1`; they match the owner's numbers within the load of
the machine: 1.9 to 2.2 s and 910 MB cached, 6.4 to 7.0 s and 1 900 MB uncached.)

Nothing else got slower. Corpus BC 29 with Microsoft's apps, `al-test-cli`: 4.2 s and 4.7 s with the rule on, 4.4 s and
4.5 s with `AL_FORCE_BASELINE=1` (load 33; CPU 20.8 and 21.5 s against 21.3 and 21.0 s; 2.0 GB both). Cached
(`--compile-cache`), warm: 1.80 s and 1.86 s, 1 246 MB both. Tests-ERM 27.5 (`alrunner test` on the zip, with the
cache, load 35 to 47): outcomes identical (9 333 tests, 0 differences), CPU 75 s against 70 to 72 s, peak 2.1 GB
both; none of its 277 test codeunits passes the rule, so the baseline runs as before.

### Slice 2: a pure run reads no dependency source (2026-10-14, branch `fast-pure`)

The profile above ended with "the cost of a pure run is reading the dependency sources to learn what objects they
hold". Three parts remove that:

1. **A key from the zip directory.** `alrunner` loads the dependency packages lazily (`deps::load_project_with`,
   `Request::lazy_sources`): `al-deps/src/lazy.rs` reads the end record, the central directory (5 000 entries for Base
   Application) and the manifest, with positional reads on the open file, and nothing else. The loaded app has the
   paths of its files and empty texts. The hash of the package is made from the name, size and CRC-32 of every entry
   (`LazySource::identity`), which is what the first version of this ADR planned. The key of the full entry
   (`AppKey::source`) uses it for such an app, so a hit no longer hashes 150 MB of text (30 ms CPU). The CRC-32 of an
   entry is checked when the entry is read. `al_cache::FORMAT` is 4. A package that cannot be read this way (a folder,
   a wrapper that holds an inner `.app`, an unusual file) is read as before and hashed over its bytes.
2. **A small index entry.** `al-cache/src/index.rs`, file `index-<hash>.alci` in the cache folder: the scan of every
   dependency file (`al_sema::FileIndex`: namespace, usings, the objects with their ids, names, extends, implements,
   subscribers, scanned properties and layouts, the `dotnet { }` aliases), `zstd(postcard(..))`, with the key digest and
   a payload hash. Base Application, System Application and the rest are about 3 MB. Its key is the key of the apps
   without the target (the scan does not depend on the BC version). It is made by a scan, not a compile, so a cold run
   makes it in the time of the scan it would have done anyway (0.1 s CPU, once) and no cold run builds the 5 s full
   entry for a pure selection. `Workspace::load_apps_lazy` builds the workspace from it: the object table, the
   extension and subscriber indexes and name resolution come out as for a scanned load (a test compares them object
   for object, and the programs of an eager and a lazy load).
3. **Lazy text.** `Workspace::text(file)` asks the loader (`SourceLoader`) the first time a file is parsed or its
   lines are needed, and keeps the text. The only two readers of a dependency file's text in `al-sema` are the lazy
   parse and the line table of `lower.rs`. A workspace that names `Record Customer` reads the file that declares it
   and no other.

The full compile entry uses the same machinery: on a hit, a run with lazily read apps reads no source and scans none
(the index replaces the scan, the key needs no text); a miss reads them for the build, as before. A run that falls
back to the demand-driven compile with the baseline reads them (`LazySource::fill`, on all cores, once) before it
starts. `--no-compile-cache` reads them at once, as before: the index is a cache too. `ALRUNNER_EAGER_SOURCES=1`
loads every package eagerly (the other side of the differential check; Tests-ERM gives the same 9 333 outcomes both
ways).

Also: `scan_path` reads a package's manifest from its directory instead of reading the whole file (Base Application's
source archive is 31 MB), and the hash of the executable that starts every key (10 ms) is made on another thread while
the packages load.

**Measured** (same fixture and method as above, medians of 9 back to back runs under `flock`, load average 17 to 20 on
12 cores shared with other agents; the CPU column is the one to compare; wall and memory of the quiet-machine runs in
brackets):

| Run | Slice 1 (`ALRUNNER_EAGER_SOURCES=1`) | Slice 2 |
|---|---|---|
| 1 pure test, cache on, index warm | 0.44 s wall, CPU 1.13 s, 387 MB | 0.14 s wall (load 7), CPU 0.18 to 0.26 s, 121 MB |
| 6 pure tests, cache on | (slice 1: 0.58 s, CPU 1.37 s, 402 MB) | CPU 0.39 s, 123 MB, 0.13 s wall (load 7) |
| 1 pure test, `--no-compile-cache` | 0.44 s, CPU 1.14 s, 400 MB | 0.23 s, CPU 1.01 s, 260 MB (sources read at once) |
| 1 pure test, cold index | 0.44 s, CPU 1.13 s | 0.27 s, CPU 1.02 s, 250 MB (one scan, index saved) |
| 1 impure test (inserts a Customer), cache on | 1.09 s, CPU 2.73 s, 3 055 MB at most (entry rebuilt once) | 0.79 s, CPU 1.75 s, 673 MB |
| same, `--no-compile-cache` | | 4.63 s, CPU 15.8 s, 1 802 MB |
| Same 6 tests, app without `application` | 0.02 s, 24 MB | 0.02 s, 24 MB |

So the owner's pure test costs 0.14 s and 121 MB (the owner measured 0.16 s and 78 MB for the app without
dependencies on their machine; here that case is 0.02 s). Tests-ERM 27.5 (cache on, warm): 10.2 s and 1.96 GB against
12.0 s and 2.13 GB with eager sources, CPU 66 s against 71 s.

What is left in a pure run (perf, 0.12 s CPU): the sha-256 of the executable 11 %, the zip directories 3 %, zstd and
postcard decode of the index 7 %, the workspace load from the index (about 10 000 objects, their name maps, the
extension and subscriber indexes) 10 %, the `lower` pass over every object to make shells 5 %, thread start-up and
the rest in small pieces.

### Not done: lazy decode of the blocks of a cached run that is not pure

Measured on a run that inserts a Customer (baseline runs): reading the blocks is 0.13 s wall of 0.70 s, and the
baseline needs about 3 100 of the 9 752 objects anyway, so decoding only what is touched would save 60 % of that, 40 to
60 ms and about 200 MB. Getting it needs `Program::objects` to fill slots on first access (172 uses of `.objects`; the
`Index` impl keeps most of them compiling) and, before that is worth anything, the whole-program passes to read stored
data: `Engine::with_config` builds a table definition for every table and the event registry from every routine's
attributes, `platform/seed.rs` and seven more passes iterate every object (and every iteration decodes everything).
Those facts (tables, publishers and subscribers by routine) would be stored in the entry beside the blocks. A sparse
program (stubs for the objects a rule says nothing reaches) is sound only for a run the rule of ADR-013 calls pure,
which slice 1 and 2 already serve without any block; for any other run an object can be reached through an event, a
number (`Codeunit.Run(id)`, `RecordRef.Open`) or a page action, so a stub would be a guess. Recommendation: not now.
The larger costs of a non-pure cached run are the install baseline (0.2 s, which also needs the snapshot the owner
left out of version 1) and the 0.9 s of system time that 670 MB of allocation and its release cost.

## Owner decision 2026-10-15: layered entries (branch `layered-cache`)

**Decision (owner).** Day to day a developer edits their implementation app and their test app all the time. A change
in their own code must never recompile Base Application or the rest of Microsoft's stack. The implementation app comes
from `.alpackages` as a dependency of the test app, so under decision 2 (one entry per dependency set) one changed line
rebuilt the whole set (10.9 s measured in ADR-026). That is replaced by layers. Large partner implementation apps
(thousands of objects) are a second concern (ADR-026, "NP Retail").

This amends decision 2 and the first amendment of 2026-10-14. The cache format changes (`al_cache::FORMAT` 4 to 5);
every old file misses and is built again.

1. **The unit is a layer.** The dependency apps are put in a canonical order: a topological order (every app after the
   apps it depends on) that takes the apps whose publisher is `Microsoft` first, then the others in the order they came
   (`compile_cache::dependency_order`; a cycle or a repeated app id keeps the caller's order). Layer 1 is all the leading
   Microsoft apps, the platform `System.app` included. **Every other dependency app is a layer of its own.** The workspace
   apps still come after all layers. (Finer granularity for Microsoft apps was not taken: a hotfix of Base Application
   rebuilds the layers above it anyway, and the apps under it, System Application and Business Foundation, are 14 % of
   the objects, ADR-022 section 7.)
2. **The key of a layer** is the key of decision 4 for the apps of that layer, plus the digest of the key of the layer
   below (all zeros for layer 1). So a change in an app (source, version, manifest) changes the key of its layer and of
   every layer above it, and of no layer below. The executable hash, the target and the preprocessor symbols are in every
   layer's key. A test lists the fields of `Key` and fails when one has no miss test (`below` has one).
3. **Positions stay stable because an upper layer only appends.** The program holds the layers in order and the
   workspace last. An object of layer k has the position it has when all layers up to k are present, whatever comes
   above; the base fields of a table and the extension fields of layers below keep their positions, and an extension
   field of layer k is appended after them (extension fields are numbered in object order). Nothing above a layer can
   move anything in it, which is why a change in a late app cannot disturb an early layer and the IR needs no remap.
4. **What a layer holds.** Its own objects (blocks and facts, positions from the end of the layers below), and a block
   and facts for each lower object that it changes: an object its apps extend or implement, and the objects that embed
   that, the same set the workspace re-lowers (`provided::relower_set`, ADR-022 "What makes a block unusable"). These are
   lowered again with the layer present, so for every object the block of the highest layer that holds it is the block
   of the complete compile of all layers up to it. The file is the entry format with two more index fields (`base_len`,
   and the program position of every slot). Layers above the first compress with the dictionary of the first (training
   one took about 1.4 s of 6 for NP Retail's layer, for blocks of the same kind).
5. **Build.** Layers are built lowest first. Layer 1 is `lower_for_cache` (unchanged). A higher layer is
   `lower::lower_layer`: all apps up to the layer are loaded, the objects below are taken from a `provided::Stack` of the
   layers under it, the layer's apps are compiled completely (the eager rule of decision 7), and the result keeps only
   the objects that were not taken from below. A run finds the layers of its dependency set, builds the missing ones in
   order (a layer that is present is kept even when one under it was rebuilt, because its key says which layers it was
   made on), and compiles the workspace on the `Stack` exactly as it did on one entry.
6. **Writing and pruning** stay as in decision 5. A run that builds a layer keeps the files of all its layers and
   removes the ones nothing used for 30 days. `--build-compile-cache` and `--require-compile-cache` work per layer; a
   miss in any layer is a miss for `--require-compile-cache`, and the error names the files.
7. **The zero-difference rule holds, with no allow list.** Evidence:
   - `provided.rs` tests build layers on top of each other and compare, object by object, facts and blocks of the
     stack with `lower_for_cache` of all apps (a layer that changes nothing, a table extension, extensions and
     implementers of every kind, a third layer that extends what the second added, query, page-part and report cases),
     and the `Program`, refusals and call graph of a workspace compiled on a stack with the eager compile.
   - `al-cache` tests: a layer file round trip, a layer file refused when the layer below changed.
   - `al-testrun/tests/compile_cache.rs`: a change in the implementation app rebuilds only its layer, a change in a library
     app rebuilds its layer and the ones above, a change in a Microsoft app rebuilds all, a deleted or damaged lower or
     upper file is rebuilt and never changes an outcome, and the caller's app order does not matter.
   - `tools/cache_vs_uncached.sh run` and `tools/demand_vs_eager.sh run`: 0 differences on BC 27, 28 and 29 and the
     OnPrem and isolation suites. `tools/baseline_rule_check.sh run`: 0. Not in the corpus (all its dependencies are
     Microsoft's, one layer): Tests-ERM cached against `--no-compile-cache` (9333 tests, 0 differences) and NP Retail's
     test app (4813 tests) with a two-layer set built cold, built again after a one-file change, and uncached
     (0 differences each).

**Measured (2026-10-15, release build, 12 cores shared with other agents; the 1-minute load average is given with
each figure, and was 14 to 25 throughout).** Rebuild after one changed file in a non-Microsoft dependency app:

| Set | Before (one entry per set) | After (layers) |
|---|---|---|
| 4 Microsoft apps + a 1-file partner library (BC 27.5), test app on top | 5.9, 6.0, 6.0, 6.6 s (load 14 to 16) | layer 2 only: 1.4, 1.6, 2.3 s (load 14 to 15); the Microsoft layer is a hit |
| The same 5 apps cold | 6.0 s | 9.2 s (8.4 for layer 1, 0.75 for layer 2) |
| 12 Microsoft apps + NP Retail (7 234 files, 7 234 objects, 42 % of the set) | 14.8, 24.5, 25.6, 29.0 s (load 17 to 25) | layer 2 only: 11.2, 11.9, 12.4 s (load 22 to 23); 7.7 and 8.7 s at load 16 and 23 |

The fixed cost of a layer rebuild is about 1.3 s (load and scan of all apps, the blocks of the layers below, drop of the
workspace); the rest is 1.0 to 1.5 ms per object of the layer. ADR-026 had estimated 0.3 s for a 300 object library on
the ratio of 0.9 ms per object; the fixed cost was not in that estimate. A big partner app is not helped much (see
ADR-026): its own layer is most of the old cost.

**Not done.** The fixed 1.3 s: the sources of the layers below are read and scanned again for each layer built (the
`.alci` source index is not used for the layer build); the blocks of the layers below are decoded though only their
facts are read. A partner app that Microsoft's apps depend on (never seen) makes the order fall back to the caller's.

## Owner request 2026-10-15 (second): a base layer shared by projects (branch `shared-base-layer`)

The Microsoft layer was keyed by every Microsoft app in a run's closure, so two projects shared it only when their
Microsoft app sets were identical. Test apps differ often (Library Assert only, or Library Assert + Any +
Tests-TestLibraries, or one more Microsoft app such as Shopify Connector), and each different set rebuilt Base
Application (8 to 11 s). This amends "Owner decision 2026-10-15: layered entries", points 1 and 2.

1. **The base layer** is the Microsoft apps `System`, `System Application`, `Business Foundation` and `Base Application`
   that are in the run's set, and every app of the set they depend on (by the `dependencies` of each app). Any two
   projects on one BC build have the same base layer. Without any of those four (a small fixture) the base layer is the
   leading Microsoft apps, as before.
2. **Every other app is a layer of its own, above the base layer, in a canonical order** that does not depend on the
   order the apps came in: a topological order that, among the apps whose dependencies are placed, takes the other
   Microsoft apps before the rest and then goes by app name (case-insensitive) and id
   (`compile_cache::dependency_order`). Two projects therefore share the base layer, and share further layers when their
   apps agree up to that point. A cycle or a repeated app id keeps the caller's order.
3. **The target in the key is the runtime only.** A compile of the dependency apps reads the target only through the
   builtins catalog, which depends on the runtime version (`Catalog::for_target`). The BC version and build were in the
   key too, so a project whose app.json says `application: 27.0` and one on `27.5` of the same build got different
   layers (the first attempt at sharing missed for that reason). The apps' own versions and source hashes, which are in
   the key, carry the build.
4. **The preprocessor symbols** were left in the key by this section's first version and are superseded by "Owner decision 2026-10-15 (third)" below.
5. **The fixed cost of a layer** went from 0.8 to 2 s to 0.3 s. A layer build used to decode the blocks of all layers
   below it only to hand them to `lower_core`, which put them in a program nobody reads; they are now placeholders
   (`DepBlock::placeholder`, `ir::Object` derives `Default`), and the stack/complete equality tests still pass. The
   remaining 0.1 to 0.3 s is the scan of the apps below.

**Measured (2026-10-15, release build, load 20 to 24).** A fresh cache, project A (4 Microsoft apps + a 1-file partner
library, BC 27.5.46862.48827) built first, then project B (Tests-ERM's closure: 11 Microsoft apps), cold:

| Project B | One Microsoft layer (before) | Base layer shared (after) |
|---|---:|---:|
| Cache build, wall | 9.06 s (load 23) | 2.90 s (load 21): base hit, seven test-library layers built in 0.2 to 1.1 s each |
| Whole run, compile | 11.2 s | 5.4 s |
| Peak memory | 3139 MB | 1502 MB |

A one-file change in a partner library layer: 0.27 to 0.43 s (load 25), against 0.75 to 2.3 s with the first layers.
The cold build of one project alone is slower with many layers: project B on an empty cache took 15.8 and 12.5 s as one
layer (load 17 and 22) and 17.0 and 15.4 s as eight (load 21), about 3 s more, because each layer has its own fixed
cost and the test libraries are compiled one at a time. Every project after the first on the same BC build saves the
base layer.

## Owner decision 2026-10-15 (third): preprocessor symbols belong to the app that has them (branch `dep-symbols`)

**Decision (coordinator, for the owner).** In Business Central a dependency is compiled by its publisher with the
symbols of its own `app.json`. A project that defines `CLEAN25` must not change Base Application's code (hard rule 6: this
is observable). Until now the project's `preprocessorSymbols` (the union over its app.json files) were applied to every
app, dependencies included. Microsoft's sources use them (`#if not CLEAN25` 1 837 times in Base Application 27.5, also
`CLEAN26`, `CLEAN27`, `CLEANSCHEMA27`), so a project with `CLEAN25` got a different Base Application, in the uncached run
and the cached one.

1. **Every app is compiled with the symbols of its own app.json** (none for a Microsoft app, whose app.json has none). The
   project's symbols apply to the workspace apps only. `Workspace::load_apps_symbols` takes a symbol set per app;
   `load_apps_lazy` and `load_apps` are the same with no per-app sets. In a run, a dependency app gets its set from
   `Manifest::symbols`, which comes from `package::Manifest::preprocessor_symbols` (read from the root `app.json` of a source
   archive or folder; a compiled `.app` has none: its publisher compiled it) through `DependencyApp`. The parse after load
   (a file parsed on first use) and the scan of a lazily read app use the same per-app set.
2. **`symbols` left the key.** `Key` has no `symbols` field. `AppKey` has the app's own symbols (sorted), so two projects
   share a layer whatever their own symbols are, and an app whose app.json symbols change gets a new layer. The source
   index (`.alci`) key is built the same way. The cache format is 6.
3. **Outcomes.** Compared before this change and after, uncached (`--no-compile-cache`), no test data:
   - Corpus on BC 27, 28 and 29 and the OnPrem and isolation suites: 5 595, 5 593, 5 593, 6, 7, 72, 72 and 72 tests,
     0 outcome differences. Tests-ERM: 9 333 tests, 0 differences. NP Retail's test app: 4 813 tests, 0 differences.
     None of these dependencies carries symbols that differ from the project's (NP Retail's are `BC27` and `BC2700`, the
     same in both of its app.json files, and no source of Microsoft's tests them).
   - Where it does change an outcome: a partner library whose own app.json says `preprocessorSymbols: ["LIBSYM"]` and
     has `#if not LIBSYM codeunit ... #endif` is now compiled with `LIBSYM` (the gated codeunit is gone, as in BC); before,
     the project's symbols (none) were applied and the codeunit was there. In the same way a project with `CLEAN25` no
     longer removes code from Base Application. Tests: `the_projects_symbols_do_not_reach_a_dependency_app` and
     `a_dependency_is_compiled_with_the_symbols_of_its_own_app_json` (cached and uncached).
4. **Layers built in parallel: not done, and not sound with this format.** Two layers that do not depend on each other
   (for example Any and Library Assert) could be built at the same time only if they did not need each other's positions.
   They do: a layer's objects take the positions after all the layers below it in the order, so the second layer's blocks
   say object numbers that depend on how many objects the first has, and `lower_core` reads the facts of every lower
   object. Compiling both on the same lower stack would put them at the same positions. The measurements show what is left
   to win anyway: each layer's check, lower and encode already use all cores, and the serial part of a small layer is
   0.1 to 0.3 s (scan of the apps below 0.1 to 0.3 s, drops 0.02 s, the rest under 0.05 s). The seven test-library layers of
   Tests-ERM took 0.2 to 1.1 s each, 2.8 s together. Overlapping the scan of the next layer with the compile of this one
   would save an estimated 0.6 s of that and costs a second workspace in memory; not built.

## Amendment (Accepted by the owner, 2026-10-16): the install baseline is an entry of this cache (branch `baseline-in-compile-cache`)

Status of this amendment: **Proposed**. It needs the owner's acceptance. The code is on the branch and works; nothing
here is merged.

### Why

Every run first runs the install code of the dependency apps and `Company-Initialize` on a fresh session, then
captures the data it left (the install baseline, ADR-013 decision 5). The result is the same for the same program
and the same world around it. A first version of keeping it between runs (the pull request of 2026-10-16) put it in
a folder of its own. The owner allows one persistent cache, so this is the same cache, not a second one.

### Decision

1. **One more entry type in `<cache dir>/compile/`.** The baseline entry is a file `<key>.albr` next to the layer files
   (`.alcc`) and the source indexes (`.alci`). The same folder, the same write method (a temporary file, then a rename),
   the same cleanup (`compile_cache::prune`: an entry nothing used for 30 days goes; a read marks the file as used).
2. **Same build pin.** The key starts with the hash of the running executable (`runtime_build`), as the layers do.
   Another build never reads the entry.
3. **The key** is a SHA-256 over named parts, so a part that is missing changes the key as much as one that differs:
   - the build pin and the file name of the top dependency layer (that name holds every dependency app, its version
     and source hash, and the target);
   - for each non-dependency app: id, name, dependencies, preprocessor symbols and the hash of its source files;
   - the selected test codeunits (the install code that runs depends on what the selection reaches);
   - the BC target, `--environment`, and `AL_UNPINNED_BASE_ACCESS`;
   - whether a .NET host is usable (see 5);
   - the identity of the test data: for the Cronus demo backup, the file's full path, length and modification time
     plus the company. The backup comes from the version-pinned artifact of the same build, so the path and the
     modification time follow the artifact. A source that cannot give an identity gets no entry;
   - the UTC day.
4. **The run clock.** The rows of a baseline carry the instant the run that made it began (`SystemClock`). The entry
   stores that instant. A run that reads the entry starts its run clock there, as if it had begun then. If its clock
   has started already at another instant, the entry is not used and the install code runs. So every value a test can
   see equals what an uncached run started at that instant would show. The day in the key bounds how far back a
   run's clock can start.
5. **.NET makes no difference to whether the entry is kept.** A run with a usable .NET host keeps and reads the entry
   like any other. The installed state is a valid result whatever .NET calls happened. The key does hold whether a
   host was usable, because a baseline made without one lacks what the refused calls would have written, and a run
   with one must not start from it. The key does not hold the assembly folders; a change there that changes install
   results needs `alrunner cache clear`.
6. **Test data.** The entry holds the tables the install code wrote to (with the restored rows copied in and the
   changes made), the note lines about changes to restored rows, and the counters. Every other table is the restored
   database and is read through the run's own engine, as in an uncached run (`Baseline::backed_by`).
   A table the install code emptied stays empty.
7. **The multi-thread snapshot** (test threads start from the bytes of the baseline instead of running the install
   code again) is the same format and now also works with test data.
8. **Flush and rebuild.**
   - `alrunner cache clear [--cache-dir DIR]` removes every entry of the compile cache (layers, indexes, baselines,
     temporary files) and prints how many files and bytes. It does not touch the downloaded artifacts or the backup
     page maps.
   - `alrunner test --rebuild-compile-cache` reads no entry: it builds the entries the run needs again and writes them
     over the old ones. Entries of other app sets stay. It cannot be combined with `--no-compile-cache` or
     `--require-compile-cache`.
   - Owner, 2026-10-16: with a way to flush, a cached result in doubt is not a problem.

### Measured

Tests-ERM with test data, BC 27.5.46862.48827, `--filter Reminder` (198 tests), `AL_FORCE_BASELINE=1`, 5 runs each,
alternating (cold: the entry removed; cached: read). The machine had a load average of 17 to 23 on 12 cores.

| | install baseline phase |
|---|---|
| cold (install code runs) | 0.09, 0.12, 0.08, 0.14, 0.14 s; median 0.12 s |
| cached | 0.08, 0.04, 0.04, 0.09, 0.06 s; median 0.06 s |

The entry is 164 KB. The saving is about 0.06 s per run here, because the install code of this set is cheap. The
corpus run with Microsoft's apps (no test data) spends 0.2 to 0.5 s on the baseline, which the entry also removes.
Outcomes: the 198 tests, and the whole of Tests-ERM (9333 tests, 100 s), give 0 differences between `origin/main`, a cold run
and a cached run (JUnit compared with `tools/compare_junit.py`). The cached full run printed "baseline: read from the cache".

### Not changed

The layer entries, their keys and the format number are not touched. A baseline entry has its own magic and layout
number; a change of the layout makes old files unreadable, and an unreadable file is removed.

### Open for the owner

- Whether the small saving justifies another entry type. The cost is one more thing that can be stale; the flush
  command and the build pin are the guard.
- Whether the key should hold the assembly folders of `--dotnet-assembly-dir`.

## Amendment (2026-10-17): cleanup of old builds (branch `cache-prune`)

Status: additive; no seam changes. Decided by the orchestrator, for the owner's request that old entries go by themselves.

### Why

An entry can only be loaded by the alrunner build that wrote it (`runtime_build` is in every key). Every new build
therefore makes new entries and leaves the old ones. The only cleanup was the 30 day rule of `compile_cache::prune`,
which ran after a write and never fired on a folder where everything is younger than 30 days: on 2026-10-17 the folder
held 2958 files, 7.1 GB, all younger than 2 days.

### Decision

1. **The build is in the file name.** Every entry file starts with the first 16 hex digits of its `runtime_build`:
   `<tag>-deps-<32 hex>.alcc`, `<tag>-index-<32 hex>.alci`, `<tag>-<key>.albr` (`al_cache::build_tag`). The cleanup
   reads no header and no body, only the directory and the modification time. The file contents, the keys and `FORMAT`
   are unchanged. A name without a tag (from before this amendment) can not be loaded by any build now, so it counts
   as another build.
2. **A hit renews the entry.** Layer, index and baseline reads already set the modification time to now
   (`compile_cache::touch`, errors ignored).
3. **The rules** (`al_testrun::cache_gc::prune`):
   - an entry of another build, unused for 7 days, goes. Not at once: several builds run side by side on one machine
     (CI, worktrees, concurrent agents), and deleting at once would make them evict each other;
   - any entry unused for 30 days goes;
   - a temporary file (`.<name>.tmp-<pid>-<n>`) older than 1 day goes.
   Files of other names (including the stamp `.last-prune`) are never touched. A file that is already gone is skipped;
   a reader that finds a file gone treats it as a miss and rebuilds.
4. **Automatic.** At the end of `alrunner test` (after the result is printed, so never before tests), at most once in
   24 hours per folder (the modification time of `.last-prune`), for at most 200 ms. A cleanup that runs out of time
   does not update the stamp, so the next run continues. `ALRUNNER_NO_CACHE_PRUNE=1` turns it off. The folder is the
   one `--cache-dir` / `$ALRUNNER_CACHE` names. Not run with `--no-compile-cache`.
5. **By hand.** `alrunner cache prune [--dry-run] [--all-other-builds]` runs the rules with no time limit and prints
   count and bytes (and the split by rule). `--all-other-builds` removes entries of other builds whatever their age.
   `alrunner cache info` prints entries and bytes, this build and the other builds by tag.
6. **Scope.** Only `<cache dir>/compile/`. The downloaded artifacts and demo backups are per BC build and stay.

## Measured 2026-10-21: the fixed cost of a cached run of a small partner app (branch `lane-perf-small-app`)

The case: a small partner app (about 1 test codeunit, extends Base Application), one test, BC 27.5, 13 dependency apps in
10 layers (9 981 dependency objects, 30.1 MB of entry), the install baseline read from its entry. Release build, 12
cores shared with other agents (load 10 to 25), so the CPU and instruction columns are the ones to compare.

| Part | Wall, all cores | CPU, one thread (`RAYON_NUM_THREADS=1 AL_LOWER_THREADS=1`) |
|---|---|---|
| Open the 10 layer files (read, index hash, facts decode) | 0.06 to 0.23 s | 0.18 s |
| Load the workspace from the source index | 0.04 to 0.13 s | 0.15 s |
| Read the blocks (zstd, postcard, every object) | 0.17 to 1.0 s | 1.0 to 1.65 s |
| Lower: link of every object, the workspace's own lowering | 0.13 to 0.52 s | 0.38 s |
| Engine | 0.05 to 0.23 s | 0.15 s |
| Install baseline (read from its entry), the test | 0.06 to 0.2 s | |
| Two .NET standbys started ahead (now on first use, ADR-014) | (in the background) | 0.2 to 0.3 s |

Total before: 2.35 to 2.56 s of CPU and 7.4 G instructions for 0.7 to 1.5 s of wall (the owner's 4.75 s was the same
work on a busier machine). After this branch: 6.45 G instructions, CPU 2.2 to 2.36 s.

The blocks are the largest part. Layer 1 alone (9 506 objects, 72 MB raw) decodes in 0.11 s of zstd and 0.30 to
0.42 s of postcard on one thread when its memory is reused, and in 0.6 s when every byte is fresh memory, as in a run
(207 000 page faults, 1 GB peak). Large pages for mimalloc change neither. A zstd context per thread instead of per
block would save about 12 % of the zstd part (1 % of the run). Of the raw bytes, routines are 29 MB (bodies 14, locals
4.3, attributes 2.4, expression types 1.7), pages 11.6, references 9.5, source positions 8.4, tables 6.2.

**What would cut it, and what it takes.** Every object is decoded and linked because the program must be complete
(the eager compile). Two forms of laziness were weighed:

1. *Lazy objects* (decision 3, `OnceLock` slots): the engine reads `Program::objects` in 482 places in `al-vm` and
   `al-testrun`, 60 of them loops over every object at start (table definitions, the event registry, seeding, the
   platform tables). Each loop needs its facts stored beside the blocks first. Not started; the "Not done" note above
   still holds.
2. *Lazy routine bodies* (safe by construction: a body is decoded and linked the first time it is read, with the same
   result): bodies with their expression types are 50 to 55 % of the postcard decode and its allocation (0.29 of 0.53 s
   for layer 1, fresh memory), and most of the link walk. Expected saving about 0.3 to 0.4 s of CPU and a few hundred
   MB per run. It needs `Routine::body` to become a wrapper that decodes on first access (a body stored as its own
   byte string in the block, a new `al_cache::FORMAT`), link to keep each object's resolved references for the
   rewrite at first access, and the walkers to leave an unread body alone. The readers of a body are few (the
   bytecode compile of a routine, the reach walks of `observe.rs`, `written.rs`, coverage, impact), and none of the
   start-up loops reads one. This is the recommended next step.

### Built 2026-10-21: routine bodies decoded and linked on first read (branch `lane-perf-small-app-2`)

`Routine::body` is an `al_ir::Body`. In the compact form (postcard, the cache entry) a body is a byte string of its own;
read back, it keeps the bytes and decodes them the first time the body is read. Link does not walk an unread body: it
hands the body its reference rewrite (`BodyFixup`, with the object's resolved tables behind an `Arc`), and
`Routine::shift_positions` hands it the position shift; both run, in order, right after the decode. A reader sees the
statements an eager decode gives. JSON keeps the plain statement list. `al_cache::FORMAT` is 13, so older entries
rebuild. Whole objects are still decoded at cache read (the "not now" above holds).

One thing differs: a body that does not fit its object's reference tables was a link error at start; now it panics as an
internal error when the body is read. Such a body cannot come from a build: the tables and the body are lowered
together, and a damaged block is refused by its checksum before it is decoded.

Measured (12 shared cores, load 12 to 24, so instructions and CPU are the columns to compare):

| Run | Before | After |
|---|---|---|
| The small partner app, 1 test, cache hit | 6.45 G instructions, CPU 2.21 to 2.42 s, wall 0.64 to 0.94 s, peak 1 003 MB | 5.49 G instructions, CPU 1.49 to 1.65 s, wall 0.52 to 0.84 s, peak 625 MB |
| Tests-ERM BC 27.5, cache hit | 2 907.5 G instructions (with the .NET helpers), compile 1.30 to 1.68 s, peak 4 478 MB | 2 907.2 G instructions, compile 1.16 to 1.24 s, peak 4 299 MB; 0 outcome differences, the same failure messages |
| Corpus BC 29, full run with Microsoft's apps | 386.0 G instructions | 386.0 to 386.1 G instructions; 0 baseline cases lost |

In the partner run 23 of 159 528 stored bodies are decoded. Tests-ERM reaches many more, so it gains little, and pays
no more: the decode moves from the start into the first call. The corpus run does not read the compile cache.

Not done: a zstd context per thread (about 1 % of the run, measured above); lazy objects.

## Amendment (Accepted, owner 2026-10-27): build the missing layers of one run in one load (branch `adr-022-layers`)

Status of this amendment: **Accepted** (owner, 2026-10-27). See "Owner decisions 2026-10-27" at its end.

It changes how layers are built ("Owner decision 2026-10-15: layered entries", point 5). It does not change what a
layer file holds, its key, or the format number.

### Why: measured on main 2957fcd4f

Microsoft's "Enforced Digital Vouchers Tests" (codeunit 139516), BC 29.0.54011.56262: 19 dependency apps in 16 layers,
10 916 dependency objects. `al-runner test-microsoft --app "Enforced Digital Vouchers Tests" --codeunit 139516
--bc-version 29 --timings`, release build, with timers added to `lower_layer_with` and `open_or_build_with` (not
committed). Every layer of the run missed (a fresh build has a fresh executable hash). Three cold runs, 12 cores, load
average 4 to 7:

| | Run 1 | Run 2 | Run 3 |
|---|---:|---:|---:|
| Cache build, all 16 layers | 9.58 s | 9.59 s | 9.65 s |
| Layer 1 (System, System Application, Business Foundation, Base Application) | 5.08 s | 5.09 s | 5.13 s |
| Layers 2 to 16 | 4.50 s | 4.50 s | 4.52 s |
| Whole run, wall / instructions | not measured | 11.1 s / 231 G | 11.1 s / 230 G |

Where the 4.5 s of layers 2 to 16 go (sum over the 15 layers, run 1; runs 2 and 3 within 0.06 s per row):

| Phase | Sum | Per layer | Depends on |
|---|---:|---:|---|
| Copy of the source text of all apps up to the layer (`apps[..range.end].to_vec()`) and other glue | 0.55 s | 0.03 to 0.05 s | the apps below |
| `Workspace::load_apps_symbols`: read and scan of every file of every app up to the layer (9 968 to 11 048 files), no source index | 1.00 s | 0.063 to 0.071 s | the apps below |
| `provided_blocks`, catalog | 0.12 s | 0.008 s | the apps below |
| Check (`check_with`) | 0.97 s | 0.001 to 0.38 s | the layer's own objects |
| `lower_core` | 1.17 s | 0.03 s for a 1-object layer, up to 0.26 s | both: about 0.03 s fixed |
| Drop of the workspace, members and check | 0.34 s | 0.005 to 0.09 s | both |
| Encode and write the file, open it again | 0.33 s | 0.002 to 0.09 s | the layer's own objects |

A layer with one or two own objects (Library Assert, Library Variable Storage, Permissions Mock) takes 0.12 to 0.14 s.
That is the fixed cost of a layer: about 0.13 s. Fifteen layers pay it fifteen times, about 1.9 to 2.0 s. The rest,
about 2.5 s, is the check, lowering and encoding of the 1 077 objects the upper layers own. That work is needed in any
design.

The cause is confirmed: each layer loads and scans the apps of every layer below it again. It copies their text, builds
a new workspace of about 10 000 objects, lowers over all of them, and drops it all. The blocks of the layers below are
not decoded (placeholders since "Owner request 2026-10-15 (second)", point 5), so that is not part of the cost.

The measurement of the same day ("Many dependencies", main 0b3cbc871) gave 6.2 s for the upper layers of an 11.2 s
build, 0.2 to 0.9 s per layer. That run was on a busier machine. The split above says the fixed part is about 45 % of
the upper layers, not all of it. **The expected saving is about 2 s, not 4 to 5 s.**

### Decision

1. **One workspace for all missing layers of a run.** `open_or_build_with` loads the apps of the lowest missing layer
   and everything below it once. Then for each missing layer, lowest first, it appends that layer's apps to the same
   workspace and builds the layer. The apps below are not copied, read or scanned again.
2. **The workspace grows only by appending.** This is the same rule that keeps positions stable (layered entries,
   point 3): objects, files and extension fields of a new app come after those already there. The derived tables
   (names, ids, `ext_of`, `subscribers_of`, `implementers_of`, interface families, visibility) are updated for the new
   app only. A new `Workspace::append_apps` does this. After each append the workspace equals, field by field, a
   workspace loaded from scratch with the same apps (a test checks it).
3. **The lower walk of a layer visits only what it lowers.** These are the layer's own objects and the lower objects
   it changes (`relower_set`). The objects taken from below are not visited one by one. This removes the fixed 0.03 s
   of `lower_core`.
4. **What stays identical.** Each layer file has the same key, the same format number and the same bytes as a layer
   built alone. Positions, blocks, facts, heads and parse problems are unchanged. The run that uses the layers does not
   change. Zero outcome differences between cached and uncached runs, as before.
5. **Memory.** One workspace is held across the layer builds instead of one per layer. The peak is the workspace of
   the top layer plus one layer's check and lowering, the same as building the top layer alone today.

### Alternatives

- **Read the source index for the layers below, still one workspace per layer.** A small change, but it saves little.
  Loading a workspace of about 10 000 objects from the index measured 0.04 to 0.13 s ("Measured 2026-10-21"), about
  the same as the scan (0.067 s). The copy, the fixed lower walk and the drop stay. Expected saving under 0.5 s.
- **Compile all missing upper layers in one pass, then split the result into layer files.** This saves the fixed cost
  too. Small layers would also share the cores (most use few of the 12), about 0.5 to 1 s more. But an object of layer k
  must hold what the compile of the apps up to k gives. Where a higher layer extends or implements what it embeds, the
  one-pass result differs, and those objects must be lowered again per layer. That is harder to prove equal. It can
  come after this decision, measured on its own.
- **Build independent layers in parallel.** Not sound with this format ("Owner decision 2026-10-15 (third)", point 4):
  a layer's positions depend on every layer below it.
- **Fewer layers (merge Microsoft's test libraries into one layer).** Saves the fixed cost too. But a change in one of
  them would rebuild the others, and layers would no longer be shared between projects with different test library
  sets. That goes against the owner decisions of 2026-10-15.

### Expected saving

On "Enforced Digital Vouchers Tests", cold: about 1.9 to 2.0 s of the 4.5 s of upper layers (cache build 9.6 s to about
7.6 s). This is about 0.13 s for every layer built after the first, so a run with fewer missing layers saves less. A
warm run (all layers hit) does not change. Layer 1 (5.1 s) does not change; it is shared between projects on one build.

### Test plan

1. `al-sema`: `Workspace::append_apps` gives a workspace equal to `load_apps_symbols` of the same apps (objects, files,
   every derived table), for an app that extends, implements and subscribes to what is below it.
2. `al-testrun/tests/compile_cache.rs`: the files of all missing layers built in one load are byte-equal to the files
   built one layer at a time (the current path, kept behind an environment variable while the new path is proven).
   Cases: all layers missing, only upper layers missing, a gap (layer 3 missing, 4 present, 5 missing).
3. The existing tests of "layered entries", point 7, stay green unchanged.
4. `tools/cache_vs_uncached.sh run` and `tools/demand_vs_eager.sh run`: 0 differences on BC 27, 28 and 29.
5. The measurement above repeated, before and after, with the same command. The result is recorded in this ADR.

### Owner decisions 2026-10-27

1. **Accepted as proposed.** All missing layers of a run are built in one append-only workspace. Each layer file stays
   byte-identical to a layer built alone; the key and the format number do not change. The saving of about 2 s is
   worth the change.
2. **The one-pass compile of the upper layers is not part of this amendment.** It is not planned with it.
3. **The old one-layer-at-a-time build stays behind an environment variable** as an off switch (orchestrator's call,
   like `ALRUNNER_EAGER_DECODE`).

## Consequences

- **IR serialization becomes a seam.** The format is versioned by an integer in the header and the key. Changing an
  IR type changes the runtime build id and so invalidates all entries; no migration code is needed.
- **The program order changes, `Program` does not.** The dependency apps come first in the program of a cached run, so
  an object index means the same in the entry and in the run; `CorpusRuntime` maps the caller's app indices both ways.
  Lazy object slots are not built yet (see "Not done").
- **First run is slow.** Building the entry is the complete compile of the dependency set: measured 4.6 to 5.3 s and
  3.8 to 4.1 GB for Business Foundation, System Application and Base Application of BC 27.5 to 29 (it was 10 to 19 s
  while the complete check ran on one thread). The runner prints one line to stderr saying it is building the cache
  once. Later runs, and every CI run after the cache-build step, do not pay it.
- **Memory:** the builder holds the complete compile in memory once (3.8 to 4.1 GB measured, so it needs a cap of at
  least 5 GB). A cached run holds the decoded program: 0.9 GB for a small app, 1.2 GB for the corpus, 2.1 GB for
  Tests-ERM, against 1.3, 1.4 and 3.1 GB uncached.
- **Disk:** an entry per set of dependency apps (13 MB), left behind after each hotfix. A run that builds an entry
  removes the ones nothing used for 30 days. `alrunner cache` and `alrunner doctor` (ADR-016) are not built.
- **Two paths stay.** The demand-driven compile remains (cache off, WebAssembly, differential), so both are
  maintained. The cached path adds no new behaviour: it gives the outcomes of the complete compile, which are those of
  the uncached path except in a few tests (see "Outcomes" above and the open question in `docs/owner-review.md`).
- **ADR-016's rule** (no feature that only hides start-up or run cost) is not broken: this is the owner's explicit
  request, placed after the performance work, with a go/no-go gate.

## Conformance

- **Key misses (one test per input).** Build an entry, change exactly one input, assert a miss and a rebuild: app
  id, version, source (one changed byte in one source file, and one changed entry in a central directory), target BC
  version, runtime build id, the key of a dependency entry, the format integer. The same inputs hit. A test lists the
  key struct's fields and fails if one has no miss test.
- **Corrupted entry.** Flip a byte in the header, in the index and in a block: each is refused with a named error,
  the entry is deleted and rebuilt, and the corrupted bytes are never executed. A truncated file does the same.
- **Atomic and parallel.** Several processes build the same entry at once; every reader sees a whole file. Two
  builds produce identical bytes.
- **Sharing.** Two projects with different own extensions load the same dependency-set entry without rewriting it.
  A customer table extension on a Base table and a customer event subscriber give the same outcomes as an uncached
  run.
- **Outcome identity.** The corpus with caching on and with `--no-compile-cache` gives the same outcome and the
  same refusal text for every test, for each BC major (the JUnit files, with the duration attributes removed, are
  byte-identical). Extends the demand-versus-eager check.
- **CI step order.** The workflow file is checked by a test: restore, build/verify, measured run with
  `--require-compile-cache`, comparison, summary, prune and save, in that order; the summary has the cache build
  time on its own line.
- **Licensing.** The existing `no_microsoft_source_in_repo` test also fails if a cache file is tracked by git, and
  the release job's asset list has no cache file.
