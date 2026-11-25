# Cache row commits: review round 2

The first four Truncate cells in review round 1 measure a database `Record`
filled by the worker, a temporary `Record` filled by the worker, a database
`Record` filled by the test session, and a database `FieldRef` filled by the
worker. The next three neighboring shapes remain main refusals.

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
| Direct row-table filter plus `RecordRef.Truncate` | REFUSED, not measured | `BCE Review R2 Tests::FilteredRecordRefTruncateRefused` |

## Fact table

The allowlist stays beside `Session::cached_calc_fields`. Every fact is an
equality:

| Positive fact | Value |
|---|---|
| AL read | `RecordOp::CalcFields` |
| receiver | direct Record, database or temporary; or database FieldRef |
| receiver filters | zero |
| initial reader | worker for temporary Record and database FieldRef; worker or test session for database Record |
| row-table Truncate receiver | direct Record |
| row-table filters at Truncate | zero |
| entry delta | the number of all Truncate commits equals the number of measured Truncate commits added |

The three tests deliberately flip the next reader, receiver-storage and commit
facts. Main refuses every one before the cache can replay a stale answer; head
must keep that refusal. No difference row is expected for these three tests.
`WithoutTruncateKeepsMainAnswer` keeps the app runnable when both sides refuse
the three Truncate cells.

The corpus body needed for a clean run is not available for these three
neighbors. They are refusal tests, not claims that the cache answer is measured.
