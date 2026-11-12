# Time filter at 23:59:59.999 on a database table: real BC evidence

Measured 2026-10-29 on the existing `bcserver2` CH container, Business Central
28.5.54151.54753. This is a local service-tier measurement, **not an upstream
corpus/oracle pin**. No corpus, oracle or application object was changed.

## Synthetic probe

App `ALR Time Filter Probe` with table/codeunit 50970 (Integer PK, Time field `Tm`),
run through `Invoke-NavContainerCodeunit` and then removed. Rows: 1=000000T, 2=120000T,
3=235959.997T, 4=235959.999T, 5=000000.001T. Filtered with SetFilter/SetRange and
reported the matching PKs in order.

| Filter | Rows |
|---|---|
| `<235959.999T` | 1235 |
| `<=235959.999T` | 12345 |
| `>235959.999T` | (none) |
| `>=235959.999T` | 4 |
| `=235959.999T` | 4 |
| `000000T..235959.999T` | 12345 |
| `120000T..235959.999T` | 234 |
| `=000000T` | 15 |
| `<235959.998T` | 125 |
| `<235959.997T` | 125 |
| `>000000T` | 234 |
| `<>235959.999T` | 1235 |

Read back with `Format(Tm, 0, 9)`: 00:00:00, 12:00:00, 23:59:59.997, 00:00:00, 00:00:00.

Hence a Time of 23:59:59.999 rounds past the day in SQL, both as a filter value and as
a stored row: it sorts after every time of the day, yet a stored row reads back as 00:00:00.

## Runner

Implemented for filter values only (`sqltime.rs` `rounded_filter`). A stored row of
23:59:59.999 is still kept as 00:00:00 in the runner's table, so filters on such a stored
row (row 4 above) still differ from BC; see docs/probes-needed.md.
