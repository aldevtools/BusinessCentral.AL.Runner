# ADR-054: Dependency objects decoded and linked the first time a run touches them

- Status: Accepted (owner, 2026-10-27)
- Date: 2026-10-27
- Amends: ADR-022 (decision 3 "lazy decode" and its "Not done" notes: this ADR says how), ADR-033 (the link step for
  objects a compile cache supplies), ADR-011 (how the engine reaches `Program::objects`)
- Related: ADR-009 (observable behavior), ADR-013 (demand and eager compile, the zero-difference rule), ADR-022
  "Built 2026-10-21" (routine bodies decoded on first read), ADR-043 (static reach)

## Decision in short

1. In a run that uses the compile cache, a dependency object is decoded from its block and linked the first time
   the run touches it. An object nothing touches is never decoded.
2. Everything a whole-program pass reads (the engine's table definitions, the event registry, routine facts, the
   object lists of the metadata tables, the link name space) is stored in the cache entry as an object head, and
   read from there.
3. What gets decoded is decided by what the run executes, not by a prediction. No object is ever stubbed or left
   out: a touch always gets the full object.
4. Outcomes, messages, call stacks and refusals stay identical to the eager decode, the uncached run and the demand
   compile. A new differential leg (lazy against eager decode) joins the existing ones, with zero differences allowed.

## Context

Measured 2026-10-27 (main at alpha.7, release build, perf stat and perf record):
a warm `al-runner test` of a small app on Base Application (11 tests, 10 insert a Customer, 1 reads a CRONUS
customer; dependencies System, System Application, Business Foundation, Base Application, Library Assert, Test
Runner; 3 layers, 9 887 dependency objects, 25.4 MB). Median of 5 runs, 12-thread desktop, and the owner's laptop
numbers for the same shape:

| Phase | Desktop | Laptop | What it does |
|---|---|---|---|
| reading the blocks | 0.04 s | 0.17 s | decompresses and deserializes all 9 887 blocks: 76.6 MB raw, 0.27 s of CPU on one thread, 27.7 us per object, 2.6 G instructions |
| lower | 0.07 s | 0.12 s | links all 9 887 cached objects into program positions; the workspace is 2 objects, no dependency routine is lowered from source |
| cache opened | 0.03 s | 0.05 s | decodes the facts of all 9 887 objects from the index |
| engine | 0.03 s | 0.03 s | `Engine::with_config` builds table definitions, the event registry (parsed from attribute text of 157 k routines) and routine facts from every decoded object; done twice in this run (the second engine for the test thread) |
| compile in total | 0.20 s | 0.44 s | |

What the tests reach: 29 objects have a routine body decoded (91 of 156 914 bodies). Their blocks are 1.2 % of the
raw bytes and 0.9 % of the decode time. With the tables whose definitions a record reads and the subscribers that
are only listed, under 200 objects, about 2 %.

ADR-022 decision 3 already chose this ("a block is otherwise read, checked and decoded the first time a test touches
its object"), with `OnceLock` slots. It was not built because the engine reads `Program::objects` at start in about
60 whole-program loops (table definitions, event registry, seeding, platform tables), and each of those needs its
facts stored first (ADR-022, "Not done: lazy decode of the blocks of a cached run that is not pure"). The 2026-10-21
work made routine bodies lazy, which removed half of the decode. The object shells are what is left. On the desktop
12 threads hide most of it; on a laptop with fewer cores it is the largest part of the compile.

## Decision

### 1. Scope

Only a run that reads the dependency objects from compile cache entries. The demand compile (`--no-compile-cache`),
the eager compile (`AL_EAGER_DEPENDENCIES`), the cache build and the layer build are unchanged. Workspace objects and
the objects lowered again because the workspace changes them (`provided::relower_set`) are built and linked at start
as today.

### 2. Object heads in the entry

The entry gets, per object, a head: what any pass over all objects reads, and nothing more. The first list, to be
completed by the audit in step 1 of the build order:

- kind, id, name, app, `uncompiled`, the properties the platform tables and the test runner read (`Subtype`,
  `SingleInstance`, `Access`, ...);
- for a table: the field list with id, name, type, class, `stored`, `InitValue` text and value, the properties
  `table_facts`/`field_facts`/`unevaluable_init_value` read, the keys, `per_company`, the extension lists;
- per routine: name, role, attributes as parsed event declarations (publisher and subscriber facts as
  `events::Registry` needs them), the facts `routine_facts` computes, and the stored refusal reason (ADR-022
  decision 7);
- for an enum: values and implementations; for a codeunit: the interfaces it implements;
- the link facts already in `DepFacts` (keys, routine keys, field numbers, control names, extension slots).

Heads are written by the cache build from the same lowered objects as the blocks, so a head cannot disagree with its
block unless the build has a bug, and a test compares them (test plan, item 2). `al_cache::FORMAT` changes, so old
entries rebuild.

### 3. Program objects as slots

`Program::objects` becomes a list of slots. A slot holds a finished object (workspace, lowered again, or already
touched) or the block bytes plus what link needs. The first access decodes, links and keeps the object
(`OnceLock`, because test codeunits run on worker threads). Reading an object goes through one accessor. A pass over
all objects goes through the heads, or through an explicit `decode_all` that only the flagged tools call (coverage of
dependency apps, `--impact-of` index build, `--affected` hashing, `--explain` of a dependency routine) and that is
counted, so a test can fail when a plain run calls it.

This changes how the engine reaches `Program::objects` (about 600 uses in `al-vm` and `al-testrun`, about 60 loops
over all objects at start). Each loop is moved to the heads or to touched objects only. The `Index` impl keeps the
single-object uses compiling.

### 4. Link on first touch

The link name space (`UnitSpace`: object keys, routine keys, field numbers, control names, extension lists) is built
from the heads, not from decoded objects. A cached object is linked when its slot is decoded, with the same
`Resolved` tables it gets today; the routine bodies keep their own deferred fixups (`BodyFixup`). Objects linked at
start are only the workspace and the objects lowered again. The program a full `decode_all` gives must equal the
program of today's eager link, object for object.

### 5. What decides that an object is decoded

The run decides, by executing. A slot is decoded when the run:

- calls a routine of the object (a test, a trigger, a subscriber the registry dispatches to, `Codeunit.Run(id)`,
  an interface call, a method of a page, report, xmlport or query variable);
- needs more of a table than its head (a trigger, a `CalcFormula`, a `TableRelation`, a field's `OnValidate`);
- runs a page, report, xmlport or query, or opens a `TestPage` on it;
- or any other access through the accessor.

Lookups by number or name (`Codeunit.Run(id)`, `RecordRef.Open`, `Page.Run`), the event registry, the metadata
tables (`AllObj`, `Field`, `Key`, `Table Metadata`) and refusal checks read heads. A head is complete for what these
read, so the answer is the same as with the decoded object.

Static reach (ADR-043) does not choose what is decoded. It may be used to start decoding early, on other threads, the
objects the selected tests reach statically (a prefetch). A prefetch only changes when an object is decoded, never
whether a touch finds it, so it cannot change an outcome. Not part of the first build.

### 6. Integrity

ADR-022 decision 5 stays: a damaged entry never changes an outcome. Today every block is decoded and checked before
the first test. With lazy decode, the open also checks one fast checksum over the whole block region of the file
(about 21 MB; a few milliseconds), next to the header, index and dictionary checks. A failure refuses the entry before
any test runs, as today. Each block keeps its own checksum, checked when it is decoded. A block that fails that check
after the open check passed is an internal error of that test (the same rule as for a lazy body today).

### 7. Off switch and differential

`ALRUNNER_EAGER_DECODE=1` decodes and links every slot at start. It is the other side of the new differential leg
and the fallback if a problem is found after release.

## What stays observable-identical (ADR-009)

Nothing AL code or the user can see changes: values, errors and their texts, database state, trigger and event
order, call stacks, which tests are refused and why, JUnit and JSON output. Only time and memory change.

The zero-difference rule of ADR-013 and ADR-022 holds with no allow list, now over four legs: demand, eager,
cached with eager decode, cached with lazy decode. The reasons it holds by construction:

- No object is missing: every slot exists from the start and a touch gets the full object.
- Every whole-program answer comes from heads written from the same lowered objects as the blocks.
- The link result of a slot is the object today's eager link gives (checked by `decode_all` equality).
- Order cannot change: objects keep their program positions; decoding order is not observable because a slot's
  content does not depend on when it is decoded.

## Expected effect

On the measured run, desktop / laptop: blocks 0.04 / 0.17 s to about 0.002 / 0.01 s; lower 0.07 / 0.12 s to about
0.02 / 0.03 s; engine 0.03 / 0.03 s to under 0.01 s; open 0.03 / 0.05 s to about 0.02 / 0.03 s when the index facts are
decoded per object too. Compile on the laptop from 0.44 s to about 0.15 to 0.17 s. Instructions of the al-runner
process from about 5.0 G to about 1.5 G. Peak memory falls (most of the decoded program is never built). A run that
touches much (Tests-ERM) pays the same decode, moved from start into the first touch, and saves the parts it does not
reach.

## Alternatives considered

- **Make eager decode cheaper** (zstd context per thread, a zero-copy format such as rkyv for the IR). A zstd context
  saves about 1 % of the run. A zero-copy format would change every IR type and still pays for building engine
  tables from all objects. Gains of 2x at best against about 50x for this decision.
- **Store a pre-linked layer 1** (the linked form of each object for the stable positions of the first layer). Saves
  most of the lower phase, but nothing of the decode or the engine passes, and adds a second stored form per object.
- **A sparse program from static reach** (stubs for objects the selected tests cannot reach). Rejected in ADR-022: an
  object can be reached through an event, a number or a page action, so a stub would be a guess.
- **Keep a warm process** (`al-runner serve`). Exists for tools; ADR-016 rules out features that only hide start-up
  cost for the command line.
- **Per-routine laziness only.** Built 2026-10-21; this decision is the remaining part (object shells and
  whole-program passes).

## Test plan (red, then green)

1. `al-cache`: a layer file with heads round-trips; a flipped byte anywhere in the block region is refused at open;
   a flipped byte in one block is found by that block's check.
2. Heads equal what they summarize: for every object of the BC 27, 28 and 29 entries (and the layer fixtures of
   `provided.rs`), the head computed from the decoded object equals the stored head.
3. Engine from heads: `Engine` built from heads equals `Engine` built from decoded objects (table definitions, event
   registry, routine facts, table and field facts, codeunit map), compared field by field.
4. Link on touch: `decode_all` of a lazy program equals today's eagerly linked program, object for object, for the
   corpus apps and the layer fixtures.
5. Decode count: the small-app fixture of `al-testrun/tests/compile_cache.rs` decodes at most a fixed small number of
   objects (the count is printed under `--timings`); a test fails when a start-up pass decodes every object.
6. Differential runs, 0 differences each, refusal texts and failure messages included: `tools/cache_vs_uncached.sh
   run` and `tools/demand_vs_eager.sh run` (BC 27, 28, 29, OnPrem and isolation suites) with a new leg
   `ALRUNNER_EAGER_DECODE=1` against lazy; Tests-ERM (9 333 tests) and NP Retail's test app cached lazy against
   uncached.
7. Threads: test codeunits on several threads touching the same object decode it once (counter) and see the same
   object.
8. Performance: the warm small-app run measured before and after (`--timings`, instruction counts); the perf budget
   tests stay green.

## Build order

1. Audit and heads: list every pass over all objects in `al-vm` and `al-testrun`, define the head, store it, prove
   test plan items 2 and 3 with objects still decoded eagerly.
2. Slots and the accessor; engine passes read heads; `ALRUNNER_EAGER_DECODE`.
3. Link on touch (test plan item 4).
4. Index facts decoded per object, whole-region checksum.

Each step keeps the zero-difference runs at 0.

## Consequences

- The cache format and `Program`'s object access are seams that change (this ADR). The format integer changes once
  per step that changes the file.
- Every new pass over all objects must read heads, or it decodes everything and the decode-count test fails.
- Two ways to reach an object remain in the code (finished object, slot); the eager switch keeps both tested.
- Time moves from start to first touch, so `--timings` reports decode time per run (sum over touches) next to the
  start-up phases.

## Owner decisions (2026-10-27)

1. Accepted: a cached run starts with only the heads and decodes an object the first time it is touched.
2. Accepted: a damaged block found after the start check passed is an internal error of that test.
3. Prefetch by static reach (decision 5) is left out of the first build. It is built later only if a measurement shows
   that first-touch decode costs more than it saves.
4. Related, outside this ADR: the in-process .NET stand-ins may answer `TimeZoneInfo.GetUtcOffset` (ADR-038 member
   list), and the artifact cache may trust file size and modification time and run the CRC only when either changed.

## Amendment: objects stored linked (Accepted, owner 2026-10-27)

Amends decision 4 ("link on first touch"). The compile cache stores each dependency object already linked: the
extensions of the dependency set merged in, every reference a program position, the relations derived. A touch only
decodes the block; there is no link and no merge at run time. Objects that the workspace extends or implements are
lowered and linked again as today ("compiled again because the workspace extends or implements what they embed",
`provided::relower_set`), with the cached objects as placeholders in the link.

What this means in the build:

1. A block holds the linked object (`DepBlock::object`), the object's source positions and the id of its first
   position in the build's program (`DepBlock::pos_start`). The symbolic form is no longer stored: no run reads it.
2. Routine bodies are stored linked too. They stay lazy (a body is its own byte string, decoded on first read); the
   only work left for them at a touch is moving their position ids when the run has them elsewhere
   (`Routine::shift_positions`, deferred like any body fixup). Measured on the warm repro: 170 of 9 887 objects are
   decoded, 0.01 s in total, so there is nothing to gain from more laziness inside an object.
3. Per routine refusal reasons, direct calls and the types of the variables that can drive other objects
   (`DepFacts::unsupported`, `calls`, `drivers`) move from the block to the object's facts, so `Lowered::unsupported`,
   `calls` and `calls_of` never decode an object. `DepFacts::positions` holds the object's position count.
4. Position ids stay those of a complete compile: link keeps a placeholder's ids free (`SymObject::reserved_positions`)
   and the position table (`al_ir::Positions`) reads a free range from the object's block on first use
   (`PositionSource`). So `decode_all` of a cached program equals the eager compile, positions included (test plan
   item 4, the `provided.rs` fixtures).

Why it is safe: dependency objects come first in the program, so their positions are the same in the build and in a
run; an object whose linked form depends on the workspace (its extensions, implementers, or anything it embeds) is in
the relower set and never comes from a block. The zero-difference runs (uncached, eager decode, demand) check it.

## Conformance

Test plan items 1 to 7; the zero-difference runs of `tools/cache_vs_uncached.sh` and `tools/demand_vs_eager.sh` with
the lazy/eager decode leg.

## Built

- 2026-10-27, step 1: `Program::objects` is a list of slots (`al_ir::Objects`), all still built finished.
- 2026-10-27, step 2 (heads, first part): `al_ir::ObjectHead` and `Objects::heads` (given with a lazy list, otherwise
  computed from the objects once and dropped on any change of the list). `Engine::with_config` (table definitions,
  table, field and object facts, codeunit map), `events::Registry::build`, the bytecode table and the routine facts
  (both now made per object on first call), the reach index, the platform table lists and the start-up passes of the
  test runner (test selection, install plan and facts, baseline steps, observe) read heads. Measured on the warm repro
  with a program whose dependency slots start empty and heads given: start-up and a run of 11 tests make only the
  objects they run. Tests: `al-vm` `lazy_objects` (items 3 and 5 on a small program). Heads are not yet stored in the
  cache entry (next step).

- 2026-10-27, step 2 (heads, second part): the cache entry stores the head of every object (`DepSet::heads`;
  `al_cache::FORMAT` 16), made by the cache and layer build from the objects as linked. They sit after the blocks in
  chunks of 256, each with a zstd content checksum, and are decoded on the first `DepSource::head`, not at open. Tests: the `provided.rs` fixtures compare every stored head with the head of the object as the run linked it
  (item 2), `al-cache` round-trips them (item 1); `AL_CHECK_HEADS=1` makes the same check on a real entry (0 differences
  on BC 27, 28 and 29 with their suites). Nothing reads the stored heads yet.

- 2026-10-27, step 3 (decode on first touch, amendment above): a cached run starts with every cached object as an
  empty slot, its head from the entry and its positions as a hole of the position table
  (`provided::CachedObjects`); `ALRUNNER_EAGER_DECODE=1` decodes all of them at start. `--timings` prints how many
  objects were decoded on first touch and the time. Differential: `tools/cache_vs_uncached.sh` compares an
  eager-decode run with the lazy one (and CI runs it). `al_cache::FORMAT` 17. Warm repro: 170 objects decoded;
  instructions 5.08 G to 2.79 G; compile 0.22 s to 0.19 s (the index facts at open and the link of the
  placeholders are what is left, step 4).

- 2026-10-27, step 3 (rest): `field_key_rows` (a `Field.Get` by key) finds its tables through the heads; the Code
  Coverage rows find each object's position ids without reading cached positions (`Positions::ids_by_object`).
  Test plan item 5: `start_up_and_one_field_get_decode_only_a_few_objects` (al-testrun `compile_cache.rs`) decodes 2
  of 416 objects (416 before the `field_key_rows` change).

- 2026-10-27, step 4 (first part): the facts leave the index; they sit after the heads in zstd chunks of 256 and are
  decoded at open on several threads. The index holds a CRC-32 of every MiB between the header and the index, checked
  at open (decision 6): a flipped byte anywhere is refused at open (`al-cache` test
  `a_flipped_byte_anywhere_is_refused_at_open`). The placeholders of the cached objects and the program's heads are
  made on several threads. `al_cache::FORMAT` 18. Warm repro: cache open 0.05 s to 0.01 s, compile 0.18 s to 0.13 s,
  instructions about the same (2.4 G). Left: the per-routine refusal reasons and calls are still copied for every
  cached object at start (`Lowered::unsupported`, `calls`; about 11 ms), the heads are still cloned (about 7 ms) and
  link still builds its name space from all units (about 22 ms).

- 2026-10-27, step 4 (link of the cached objects): a cached object gets no piece, staged object, placeholder unit or
  default object any more. `al_link::link_items` takes `Item::Placeholder` (the facts other objects ask of it, read in
  place from the cache) and returns no object for it; `Lowered::unsupported` and `calls` read a cached object's lists
  from its facts (`lower/routine_lists.rs`); `ir::Objects` boxes its slots. Demo (`al-runner test app test`, warm,
  median of 5): compile 0.15 s to 0.12 s, instructions 3.84 G to 3.61 G; warm repro: compile 0.13 s to 0.10 s,
  2.46 G to 2.23 G.
