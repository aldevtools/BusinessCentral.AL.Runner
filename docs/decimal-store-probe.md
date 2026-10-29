# Decimal stored in a database table: real BC evidence

Measured 2026-10-29 on the existing `bcserver2` CH container, Business Central
28.5.54151.54753. This is a local service-tier measurement, **not an upstream
corpus/oracle pin**. No corpus, oracle, application object or baseline was changed.

## Synthetic probe

App `6f3c1d2e-7a1b-4c55-9d0e-2b8a51c0de01`, name `ALR Decimal Store Probe`,
versions up to `1.0.0.5`; synthetic table/codeunit 50990 with an Integer primary
key and a Decimal field (no DecimalPlaces). For each value `X` the probe ran
`R.K := K; R.D := X; R.Insert();`, then `G.Get(K)` into a second record and
reported `Format(G.D, 0, 9)` and `Format(R.D, 0, 9)` through
`Invoke-NavContainerCodeunit`; the SQL column of the field is `decimal(38,20)`. The app is removed
afterwards.

| Written `X` | Read back by Get/Find |
|---|---|
| `1000 / 3600000` | `0.00027777777777777777` |
| `-1000 / 3600000` | `-0.00027777777777777777` |
| `0.000000000000000000019` | `0.00000000000000000001` |
| `-0.000000000000000000019` | `-0.00000000000000000001` |
| `0.00012345678901234567891` | `0.00012345678901234567` |
| `0.0000123456789012345678` | `0.00001234567890123456` |
| `0.99999999999999999995` | `1` (the in-memory value is already rounded to 18 significant digits) |

The inserted record `R` keeps its own value after Insert (`R.D = 1000 / 3600000`
stays true; `R.D = G.D` is false). Hence: the column cuts toward zero after
20 decimals; the in-memory Decimal is untouched.

## Not measured

A Decimal primary key value with more than 20 decimals (how Get/Modify/Delete
find the row), temporary tables, filters on such values, BC versions other than
28.5. The runner refuses the primary key case and leaves the others as before.
