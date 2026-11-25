# Cache row commits: review round 2

The first four Truncate cells in review round 1 measure a database `Record`
filled by the worker, a temporary `Record` filled by the worker, a database
`Record` filled by the test session, and a database `FieldRef` filled by the
worker. The neighboring shapes below remain main refusals (round 2, and
al-runner issue 9097).

All line numbers below name
`tests/al-language/pages/background-tasks/BgTaskCacheEdges.al` in the merged
evidence from corpus pull requests 1558 and 1581, with the source copied in
review round 1.

## Area table

| Shape | State | Neighbor |
|---|---|---|
| Database `Record.CalcFields`, no record filters, worker first, direct `Record.Truncate` | ASSUMED; corpus PR 1558 line 958 | existing review-r1 cell |
| Temporary `Record.CalcFields`, no record filters, worker first, direct `Record.Truncate` | ASSUMED; corpus PR 1581 line 1178 | existing review-r1 cell |
| Database `Record.CalcFields`, no record filters, test session first, direct `Record.Truncate` | ASSUMED; corpus PR 1581 line 1184 | existing review-r1 cell |
| Database `FieldRef.CalcField`, no record filters, worker first, direct `Record.Truncate` | ASSUMED; corpus PR 1581 line 1190 | existing review-r1 cell |
| `CalcFields` with a FlowFilter set | REFUSED, not measured | `BCE Review R2 Tests::CalcFieldsWithFlowFilterRefused` |
| Temporary `RecordRef`, `FieldRef.CalcField` after Truncate | REFUSED, not measured | `BCE Review R2 Tests::TemporaryFieldRefCalcFieldRefused` |
| Filtered `Record.Truncate` | REFUSED, not measured | `BCE Review R2 Tests::FilteredRecordTruncateRefused` |
| Unfiltered `RecordRef.Truncate` | REFUSED, not measured | `BCE Review R2 Tests::UnfilteredRecordRefTruncateRefused` |
| `Record.Truncate(true)` | REFUSED, not measured | `BCE Review R2 Tests::TruncateResetRefused` |
| `MarkedOnly(true)`, then `Record.Truncate` | REFUSED, not measured | `BCE Review R2 Tests::MarkedOnlyTruncateRefused` |
| Worker `CalcFields` of a record `Get` loaded | REFUSED, not measured | `BCE Review R2 Tests::CalcFieldsRecordLoadedByGetRefused` |
| `Record.Truncate(false)` (explicit argument) | REFUSED, not measured | `BCE Review R2 Tests::ExplicitFalseTruncateRefused` |
| Worker `CalcFields` of a temporary record after its own Insert | REFUSED, not measured | `BCE Review R2 Tests::TemporaryCalcFieldsAfterOwnInsertRefused` |
| Worker `CalcFields` of a record `Find` loaded | main's behavior: never cached, answers from the table; does not reach the Truncate gate (al-runner issue 9103) | `BCE Review R2 Tests::CalcFieldsRecordLoadedByFindRefused` |
| Worker `CalcFields` of a temporary record `Get` loaded | REFUSED, not measured | `BCE Review R2 Tests::TemporaryCalcFieldsRecordLoadedByGetRefused` |

## Fact table

The allowlist stays beside `Session::cached_calc_fields`. Every fact is an
equality:

| Positive fact | Value |
|---|---|
| AL read | `RecordOp::CalcFields` |
| receiver | direct Record, database or temporary; or database FieldRef |
| receiver filters | zero |
| receiver load | read no row: a new variable (:782, :1039, :1045, :1053), or the test session's database record after its own Insert (:1124-:1125, :1137); `position` and `row_origin` equal those, and for the own-Insert shape temporary, FieldRef and worker are all false |
| initial reader | worker for temporary Record and database FieldRef; worker or test session for database Record |
| row-table Truncate receiver | direct Record |
| row-table filters at Truncate | zero |
| row-table MarkedOnly at Truncate | false (:883, :1142-:1144) |
| Truncate argument | none given: `Truncate()` (:883, :1142-:1144); `Truncate(false)` and `Truncate(true)` refuse |
| entry delta | the number of all Truncate commits equals the number of measured Truncate commits added |

Each test flips exactly one reader, receiver-storage or commit fact. Six of them (Get-loaded database and temporary record, `Truncate(true)`, `Truncate(false)`, `MarkedOnly`, temporary record after its own Insert) were flagged passes on main after PR 8932 and are refused on head: they are listed in `differences.tsv`. The rest are refused on both.
`WithoutTruncateKeepsMainAnswer` keeps the app runnable when both sides refuse
the Truncate cells.

The corpus body needed for a clean run is not available for these
neighbors. They are refusal tests, not claims that the cache answer is measured.
