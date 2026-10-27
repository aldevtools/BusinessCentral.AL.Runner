# ADR-045: The test clock moves forward on every read

- Status: Accepted (owner, 2026-10-18)
- Date: 2026-10-18
- Amends: ADR-013 decision 9, item 2 (the clock is frozen by default)

## Context

The test clock stood still. `Today`, `Time` and `CurrentDateTime` answered one instant for the whole run, and only `Sleep(n)` moved it. The reason
was repeatability: a clock that follows the wall makes an outcome depend on machine load (a Base Application subscriber compares
`Format(xRec) <> Format(Rec)` after `Item.OnModify` sets "Last Time Modified" to `Time`).

Real BC never gives a test a predictable time between two reads, so a test can only rely on "later than". A frozen clock breaks
code that waits for time to pass without calling `Sleep` (retry loops that stop when `CurrentDateTime` passes a deadline; a partner
app's return-retry test expected three retries and got one). Owner decision: the fake clock moves forward by a fixed small step on
every read, and by n milliseconds on `Sleep(n)`.

## Decision

1. **One source.** The session keeps the clock: the host's instant (`Clock::now_utc_millis`, fixed at the first read of a run) plus a
   counter of milliseconds the session has moved it. `Today`, `Time`, `CurrentDateTime`, the .NET `DateTime.Now/UtcNow/Today` and
   `DateTimeOffset.Now/UtcNow` all read that one value.
2. **Step: 1 millisecond.** `DateTime` and `Time` have millisecond resolution, so 1 ms is the smallest step that makes two reads
   differ. A read answers the current instant, then moves the clock on by 1 ms. The first read of a session is therefore the host
   instant unchanged (the deterministic host still starts at 2024-01-15T12:00:00Z).
3. **`Sleep(n)`** adds n ms to the same counter and does not wait. `Sleep(0)` moves nothing. A negative n stays refused (not pinned).
   Between two reads around `Sleep(n)` the difference is n + 1 ms.
4. **What does not move the clock.** Internal readers: system stamps (`SystemCreatedAt` and friends, still strictly increasing by
   themselves), the default `WorkDate`, record link creation time, report and page header times. They read the current instant
   without stepping, so a stamp written between two AL reads lies between them.
5. **`ALRUNNER_REAL_CLOCK=1` stays.** It makes the host answer the machine clock; the session step and `Sleep` offset still apply on
   top (a 1 ms step per read, negligible against a moving wall clock).
6. **Repeatability is kept.** The result depends only on the number of reads and the sleeps, not on load. The date can still roll
   over during a run, as before.

## Consequences

- Code that loops until time has passed now ends. Two reads never compare equal; `A = B` between two reads is no longer true.
- `Format(Time)` changes at a second boundary after 1000 reads. The Item workflow case above compares `Format` at seconds; it can
  now see a change that the frozen clock hid. The Microsoft buckets Tests-ERM and Tests-SCM were run before and after (see the pull
  request); changed outcomes are listed there.
- Corpus tests that pin only "later" or "at least n ms" pass unchanged.

## Conformance

- Corpus pins that must still pass (all rely on order, not on equality or exact differences):
  `session/W5SessionFacts.al:37-40` (and :52, Sleep moves the clock by at least n), `session/TestProbeSessionMiscContracts.al:784-786`,
  `session/TestSessionFunctions.al:39-65`, `record/TestSystemFieldStampsContracts.al:60-97`, `record/TestRecordLinkColumns.al:104-106`,
  `record/TestRecordSystemId.al:510-764`, `record/TestXRecStampFields.al:168-179`, `system-fields/W6SystemFieldStamps.al`.
- `crates/al-vm/tests/store/hostvalues.rs`: `every_read_of_the_clock_is_one_millisecond_later_than_the_last`,
  `sleep_moves_the_clock_by_the_time_waited`, `sleep_of_zero_is_allowed_and_a_negative_wait_is_refused`.
- `crates/al-vm/tests/dotnet.rs`: `datetime_now_is_the_host_clock_and_needs_no_dotnet_host`.

## Amendment: the run clock starts at noon UTC of the day (2026-10-19)

### Decided without owner review (2026-10-19)

1. **The host instant is 12:00:00.000 UTC of the UTC day the run first reads the clock**, not the wall time of that read
   (`al-testrun/src/host.rs`, `run_clock_start`). The date stays the wall clock's, so `Today` is today; only the time of
   day is fixed. A run that reads a kept install baseline still starts where that baseline's run started
   (`SystemClock::begin_at`); the baseline cache key holds the day, so that is the same noon. `ALRUNNER_REAL_CLOCK=1`
   is unchanged.
2. **Why.** Item 6 above held only within a run. The instant came from the wall clock, so `Time` differed between runs,
   and between selections: the kept baseline is keyed by the selection, so each selection replayed the instant of its
   own first run. Microsoft's test libraries seed the session's random generator from the clock
   (`Any.SetDefaultSeed` and `LibraryRandom.Init` call `Randomize(Time - 000000T)`), and the seed stays in force for the
   rest of the codeunit. `Job Posting.PurchaseOrderFCYWithJob` (Tests-Job, codeunit 136309) failed with a rounding
   difference or passed depending on which other codeunits were selected: an earlier test of its codeunit
   (`PostingPurchaseOrderWithItemTracking`, through `CreateNoSeries`) calls `Any.SetDefaultSeed`. The ms phase of the
   instant also decided at which read `Format(Time)` crosses a second. Both broke ADR-043 condition 1.
3. **Why noon.** Any fixed time of day works; noon is the deterministic host's time of day (2024-01-15T12:00:00Z), and it
   keeps the UTC date for every time zone between UTC-12 and UTC+12.
4. **What BC does.** BC's `Time` is the wall clock, so a test that seeds from it draws other numbers in each run on BC
   too. Microsoft's runner limits that to one test method: `ALTestRunner Reset Environment` (130453) runs
   `Reset State Before Test Run` (130301), which calls `LibraryRandom.SetSeed(1)`, before every test method. This runner
   does not do that yet (branch per-test-reset); until it does, the seed of one test reaches the later tests of its
   codeunit, but now in the same way in every run.

## Amendment: the clock catches up with the last system stamp (2026-10-27)

### Status: Accepted (owner, 2026-10-27)

1. **A stamp later than the clock moves the clock to the stamp.** Stamps are strictly increasing (item 4), so more writes than
   milliseconds between two reads put the stamps ahead of the clock: a partner app's test inserted a row and read
   `CurrentDateTime() - SystemCreatedAt` as negative. Item 4 says a stamp written between two reads lies between them, and
   `SystemCreatedAt` is the current time (record/TestSystemFieldStampsContracts.al:67). The clock now answers at least the last
   stamp (`Session::stamp_millis`). A stamp still never steps the clock past itself, so a run with few writes is unchanged.
2. **A stamp is a value the SQL datetime column holds.** The column keeps 1/300 s (P130DateFilters.al:162-183), so a stamp's
   millisecond part is one of 0, 3, 7, 10, 13, 17, ... (the next such value at or after the instant above). Whether a record's
   buffer holds a stamp rounded is not pinned; with such a value it reads the same either way, so a stamp copied into a
   DateTime field and read back is still equal.
3. **Repeatability is kept.** The clock depends on the number of reads, sleeps and writes, not on load.
