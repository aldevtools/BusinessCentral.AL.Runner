# ADR-038: In-process stand-ins for a small closed set of .NET framework types

- Status: Accepted (owner, 2026-10-16: the closed set as proposed, the RustCrypto md-5, sha1 and sha2 crates, and migration to the sidecar for a mixed object)
- Date: 2026-10-16
- Related: ADR-014 (DotNet provider seam, section 1 "stand-ins for hot standard-library classes"), ADR-016, ADR-031

Accepted by the owner on 2026-10-16.

## Context

Every `DotNet` call goes to the sidecar process (ADR-014): one JSON request, one reply, over a pipe. A round
trip costs about 90 microseconds on the main machine (53 for a `release`, up to 250 for the first `Append`).
That is cheap for one call and expensive for a loop of calls.

Microsoft's own AL makes such a loop. System Application `Cryptography Management` turns a hash into hex text
byte by byte through `DotNet` values (`ConvertByteHashToString`: for each byte, `Array.GetValue(i)`,
`Object.ToString('x2')`, `StringBuilder.Append(text)`). Pageworks calls `GenerateHash` for every image and font
it stores.

### Measured (2026-10-16, `alrunner test Pageworks Pageworks.Test --codeunit 50356`, request trace)

| | |
|---|---|
| Requests in the run | 5,421, all of them for 39 SHA256 hashes (139 per hash; the 39 sit 139 requests apart) |
| Per byte of the digest | 3 requests: `GetValue`, `ToString`, `Append` (1,248 of each = 39 x 32) |
| Fixed per hash | `Encoding.UTF8`, `GetBytes`/`ToArray`, `SHA256.Create`, `ComputeHash`, `Length`, `Dispose`, `StringBuilder` construct, one `ToString` |
| Handle releases | 1,330 (24 %), now sent without waiting (commit "DotNet sidecar: a release is sent without waiting") |
| Time in requests | 0.25 to 0.6 s per run, of a 0.1 to 0.3 s test; BC runs the test in 0.03 s |
| The test alone | `PageworksImgHorizOverAllocTest` adds about 1,530 requests (16 hashes) to the 3,892 of the install |
| Whole `Pageworks.Test` | 121,208 requests, 15.5 s in requests, of a 132 s run (install 39.7 s, tests 82 s) |

The hex conversion is 69 % of the requests (3 x 1,248 of 5,421) and with the fixed part and the releases it is
all of them in this test. The shape is not special to Pageworks: any Microsoft code that hashes, encodes
Base64, or builds text through `StringBuilder` repeats it. Faster IPC does not remove it (139 round trips per
hash remain); batching cannot be done by the runtime, because the loop is AL code.

## Decision

Serve a closed, listed set of pure .NET framework members in process, in Rust, and keep every other `DotNet`
call in the sidecar. This is the "stand-in" ADR-014 section 1 allows when it is "measured faster" and "passes
the same tests as the sidecar path". The sidecar stays the oracle.

### 1. The set (and nothing else)

| Type | Members |
|---|---|
| `System.Text.StringBuilder` | constructors `()`, `(String)`, `(Int32)`; `Append(String)`, `Append(Char)`, `AppendLine()`, `AppendLine(String)`, `Length` (get), `Clear()`, `ToString()`, `ToString(Int32, Int32)` |
| `System.Array` / `System.Byte[]` | `GetValue(Int32)`, `SetValue(Object, Int32)`, `Length` |
| boxed `System.Byte`, `System.Int32` | `ToString()`, `ToString(String)` for the formats `d`, `D`, `x`, `X` with a width (invariant culture, as the sidecar runs) |
| `System.Text.Encoding` | `UTF8`, `ASCII`, `Unicode` (UTF-16 LE) as static properties; `GetBytes(String)`, `GetString(Byte[])`, `GetByteCount(String)` |
| `System.Security.Cryptography.HashAlgorithm` | `Create(String)` for `MD5`, `SHA1`, `SHA256`, `SHA384`, `SHA512`; `ComputeHash(Byte[])`; `Dispose()` |
| `System.Convert` | `ToBase64String(Byte[])`, `FromBase64String(String)` |

Everything else is out: `MemoryStream`, `Stream` hashing, `HMAC`, `Rfc2898DeriveBytes`, `RSA`, `Aes`, `Regex`,
`DateTime`, collections, XML, JSON, `Math`, all server classes (they stay in the registry of ADR-014).
`SHA384` is in because the `Hash Algorithm` option of Cryptography Management lists it; it costs no more than
`SHA512`.

### 2. Object identity and mixing with the sidecar

A value created in process is an in-process object (`DotNetKind::Local`, a new variant; additive). When AL code
hands it to a member that the set does not serve (a `MemoryStream.Write(byte[])`, an XML call), the object
**migrates**: its state is sent to the sidecar once, it becomes a sidecar handle, and every later call on it goes
to the sidecar. One side owns the object at a time, so identity and mutation behave as in .NET. The reverse
(a sidecar `byte[]` handed to `ComputeHash`) is read by value through one `get_array` request and the hash is
then computed in process; the handle stays a handle.

### 3. Same outcomes, shown by differential tests

- A new test suite `crates/al-vm/tests/dotnet_diff.rs`, run where `dotnet` is found (CI has it; it is skipped,
  and reported as skipped, elsewhere). Each member of section 1 runs twice, once through the in-process stand-in
  and once forced through the sidecar (`ALRUNNER_NO_INPROC=1`), and the results are compared: the value, or the
  exception CLR type and message.
- Inputs are generated, not hand-picked: random byte arrays (0 to 70,000 bytes, covering the block boundaries
  of MD5, SHA1, SHA256 and SHA512), random strings with lone surrogates, non-BMP characters and NUL, random
  indices (valid and out of range: `IndexOutOfRangeException` text), invalid Base64 (`FormatException` text),
  invalid UTF-8 for `GetString` (the U+FFFD replacement rules).
- The set is a file, `crates/al-vm/src/dotnet/inproc.txt`, one `Type.Member` per line. A unit test fails when a
  line has no differential test, and when the router serves a member the file does not list.
- The Pageworks suite and the Microsoft buckets that reach Cryptography Management (Tests-ERM, Tests-SCM) are
  run with and without the stand-ins; `tools/compare_junit.py` must show 0 differences.
- ADR-014 policy (`policy.txt`) is applied before the stand-in is chosen, so a denied member stays denied.

### 4. What stays in the sidecar

Every type and member not in the table, and every call after a migration. The sidecar protocol does not
change. With `ALRUNNER_NO_INPROC=1` the whole set goes to the sidecar, which is how the differential tests and a
user who suspects a stand-in run the old path.

## Expected effect

- A SHA256 digest: 139 round trips (about 12 ms) to none (microseconds). The Pageworks image test loses about
  1,530 of 1,530 hash requests; its remaining time is the AL work, expected near BC's 0.03 s class.
- The measurement to confirm on the first build: requests and request time of `Pageworks.Test` before and after
  (121,208 and 15.5 s today).
- No change for code outside the set.

## Alternatives considered

1. **Faster IPC** (shared memory, a spin wait). Cuts 90 microseconds to perhaps 20, keeps 139 round trips per hash,
   adds platform-specific code. Rejected as the main fix.
2. **Batch or pipeline requests in the host.** The host cannot see the next call: the loop is AL code. The one
   safe case (a `release` needs no reply) is done.
3. **Replace Microsoft's `Cryptography Management` with native code.** Rejected by hard rule 7 (no fakes of
   dependent-app objects).
4. **Do nothing.** Keeps every hash at 12 ms and every Pageworks image test at several times BC's time.
5. **A wider stand-in layer** (streams, `Regex`, `DateTime`). Larger payoff and a larger risk of silent
   difference. Not proposed; a later ADR can extend the closed set with the same test rule.

## Consequences

- New Rust dependencies for the digests: the RustCrypto `md-5`, `sha1`, `sha2` crates (pure Rust, no unsafe in
  the API we use). The owner approves them with this ADR.
- The behavior of a stand-in is a second implementation to keep equal to .NET. The differential suite and the
  closed list are the guard; a mismatch fails CI, and the sidecar answer wins by definition.
- The stand-ins are an internal speed-up, not an AL-visible change (ADR-009: internals are free to be better).
  No new AL behavior needs a corpus pin; the sidecar path is the oracle.
- ADR-014 section 1 is read as allowing this set. Nothing in ADR-014 is changed.

## Owner decisions needed

1. Accept the closed set of section 1, or cut it (for example hashes only: `HashAlgorithm`, `Encoding`,
   `Array.GetValue`, boxed `ToString`, `StringBuilder.Append/ToString`).
2. Accept the three RustCrypto crates as workspace dependencies.
3. Accept migration to the sidecar as the rule for a mixed object (section 2), or require a refusal instead.

## Conformance

`crates/al-vm/tests/dotnet_diff.rs` (differential, with the sidecar as oracle), the `inproc.txt` list test, the
Pageworks and Microsoft-bucket junit comparisons, and the request-count line of `alrunner test --timings`.

## Amendment 2026-10-17: TimeZoneInfo and DateTimeOffset

- Status: Accepted (owner, 2026-10-17, in conversation). The library choice was made by the orchestrator; the owner may still object.
- Reason: measured on six Microsoft buckets (request trace of 2026-10-16), these two types are 87.2 % of all
  sidecar member calls (210,751 of 241,697) and, with their handle releases, about 87 % of all requests. A request costs
  about 0.14 ms and a full Microsoft run sends about 500,000. The goal is run time, not replacing the sidecar.

### Members added to the closed set (exactly what Microsoft's code calls, and nothing more)

Source of the list: the traced calls of the six buckets (7 members of `DateTimeOffset`, 2 of `TimeZoneInfo`; the counts
add up to the 210,751) and the AL that makes them (`TypeHelper.GetUserTimezoneOffset` and `GetTimezoneOffset`,
`DotNet_DateTimeOffset.ConvertToUtcDateTime` and `GetOffset`, `Unix Timestamp Impl`).

| Type | Member | Calls traced | Served in process |
|---|---|---|---|
| `System.TimeZoneInfo` | `FindSystemTimeZoneById(String)` | 81,751 | yes, for `UTC` and the Windows ids of the bundled CLDR table (below); any other id goes to the sidecar |
| `System.TimeZoneInfo` | `BaseUtcOffset` (get) | 81,751 | yes |
| `System.DateTimeOffset` | constructor `(DateTime)` | 11,808 | yes |
| `System.DateTimeOffset` | constructor `(Int64 ticks, TimeSpan offset)` (what `DateTimeOffset.Now` builds, ADR-024) | 11,808 | yes |
| `System.DateTimeOffset` | `Offset` (get) | 11,812 | yes |
| `System.DateTimeOffset` | `LocalDateTime` (get) | 11,807 | yes |
| `System.DateTimeOffset` | `DateTime` (get) | 4 | yes |
| `System.DateTimeOffset` | `ToLocalTime()` | 5 | yes |
| `System.DateTimeOffset` | `Parse(String, IFormatProvider, DateTimeStyles)` | 4 | no: culture dependent, goes to the sidecar |
| `System.DateTimeOffset` | `ToString(String)` | 1 | no: format dependent, goes to the sidecar |
| `System.TimeZoneInfo` | `GetUtcOffset(DateTime)` | (added 2026-10-27) | yes, on the UTC zone only (amendment 2026-10-27) |

Everything else on both types (`GetUtcOffset` except on the UTC zone, see the amendment of 2026-10-27, `ConvertTime`, `IsDaylightSavingTime`, `SupportsDaylightSavingTime`,
`DisplayName`, `StandardName`, `GetAdjustmentRules`, `FromUnixTimeSeconds`, `ToUnixTimeSeconds`, `UtcDateTime`, arithmetic,
`TimeZoneInfo.Local`, ...) goes to the sidecar. They are not in the traces; `TimeZoneImpl` calls some of them, and they run
once per call of that codeunit, not in the hot loop. A later amendment can add one when a measurement asks for it.

### Semantics served

- The session's local time zone is UTC (`TZ=UTC`, ADR-014 section 9). So `DateTimeOffset(DateTime)` has offset zero and
  `LocalDateTime` is the UTC ticks as a `DateTime` of kind Local. The stand-in does not read the machine time zone.
- `FindSystemTimeZoneById` answers a `TimeZoneInfo` object holding the id and the base offset. `BaseUtcOffset` is the
  standard (non-DST) offset of the zone as .NET on Linux computes it from the tz data.
- Windows ids come from a table generated from Unicode CLDR `windowsZones.xml` (territory 001 rows, the mapping .NET reads
  from ICU), committed with its generator (`crates/al-host/examples/gen_windows_zones.rs`). Version: CLDR 48
  (`release-48`). Zone rules come from the `jiff` crate with its bundled tz database (`jiff-tzdb`, no file is read from
  `/usr/share/zoneinfo`, ADR-006). Both crates are pinned to an exact version.
- Where jiff could differ from the .NET on the oracle machine (a zone whose standard offset is ambiguous, such as zones with
  negative DST, or a tz database release that differs), the differential test lists the id and the stand-in forwards that
  id to the sidecar (an exclusion list in the code). The sidecar answer wins by definition.
- Exceptions (unknown id, out-of-range ticks, an offset that is not whole minutes or beyond 14 hours) are not produced
  here: the stand-in forwards the call and the sidecar throws, so the CLR type and message are .NET's own.

### Mixing with sidecar objects

As in section 2. A `TimeZoneInfo` or `DateTimeOffset` made in process migrates when AL code hands it to a member that is
not in the list: a `DateTimeOffset` is rebuilt in the sidecar with `DateTimeOffset(Int64, TimeSpan)`, a `TimeZoneInfo`
with `FindSystemTimeZoneById(id)`. A sidecar `DateTimeOffset` or `TimeZoneInfo` (from `Parse`, or an id outside the table)
stays a handle and every call on it goes to the sidecar.

### Licenses

`jiff` and `jiff-tzdb`: Unlicense OR MIT (the tz data is public domain). CLDR `windowsZones.xml`: Unicode License v3
(permissive, needs the notice); the license text is committed next to the generated table
(`crates/al-host/THIRD-PARTY-UNICODE.txt`).

### Conformance

The differential test (`crates/al-test-cli/tests/suite/dotnet_diff.rs`) covers every served member on every Windows id of the
table, on UTC, and on edge ticks (year 1, year 9999, leap days, DST switch instants of several zones); `inproc.txt`
lists the members as before.

## Amendment 2026-10-27: TimeZoneInfo.GetUtcOffset for the UTC zone

- Status: Accepted (owner, 2026-10-27: add `GetUtcOffset` to the in-process member list).
- Reason: a warm `al-runner test` of a small app on Base Application (profile of 2026-10-27) made only two kinds of .NET
  calls: `FindSystemTimeZoneById("UTC")` once per session (already served here) and `GetUtcOffset(DateTime)` on that
  zone. The second one moved the zone to a sidecar, so the run started a sidecar process and waited about 0.05 s for it.

### Member added

| Type | Member | Served in process |
|---|---|---|
| `System.TimeZoneInfo` | `GetUtcOffset(DateTime)` | yes, only on a zone found by the id `UTC` (any case): the answer is a zero `TimeSpan` for every `DateTime` |

Every other zone, and every other overload (`GetUtcOffset(DateTimeOffset)`), goes to the sidecar: the zone moves there on
its first call, as in section 2. The offset of a zone with daylight saving time or a changed history depends on the tz data
of the host, which the sidecar has and the stand-in does not claim to match.

### Conformance

`crates/al-test-cli/tests/suite/dotnet_diff.rs` calls `GetUtcOffset` on every id of the zone list (the stand-in must forward all
but `UTC`) and on `UTC` and `utc` for every tick of the edge list, and compares each answer with the sidecar's.
