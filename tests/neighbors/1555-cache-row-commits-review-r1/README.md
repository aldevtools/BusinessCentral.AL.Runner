# Cache row commits: review round 1

`src/EdgesProbe.al` is the complete 38-cell corpus file at merged corpus PR 1581,
commit `852fb2b9`, with two extra header lines. The first 33 cells merged in corpus
PR 1558. The five appended cells are evidence from the merged follow-up, not corpus
pins yet.

The full file passed three consecutive runs on local BC 29.0. It compiled against
BC 27, 28 and 29 System symbols; the three existing Enum-to-Integer warnings are
intentional measured inputs.

## Area table

All line numbers name `tests/al-language/pages/background-tasks/BgTaskCacheEdges.al`
in the corpus, not the two-line-prefixed copy here.

| Shape | Corpus line | State |
|---|---|---|
| Database Record, key assigned, worker CalcFields first, row table Truncate | 958 | ASSUMED background-task-query-cache; merged PR 1558, awaiting pin |
| Temporary Record, key assigned, worker CalcFields first, database row table Truncate | 1178 | ASSUMED background-task-query-cache; merged corpus PR 1581 at 852fb2b9 |
| Database Record, key assigned, test CalcFields first, row table Truncate | 1184 | ASSUMED background-task-query-cache; merged corpus PR 1581 at 852fb2b9 |
| Database RecordRef, key assigned, worker FieldRef.CalcField first, row table Truncate | 1190 | ASSUMED background-task-query-cache; merged corpus PR 1581 at 852fb2b9 |
| Integer 0 and Integer 2 naming no table, SelectLatestVersion | 1196, 1202 | Main's behavior; merged corpus PR 1581 at 852fb2b9 |
| ModifyAll, DeleteAll and Delete commits of the calculated row | 915, 921, 927 | Main's behavior; merged PR 1558, awaiting pin |
| Rename; Delete followed by reinsert | Not measured by these cells | Main's behavior; no new claim |
| Char, Byte or Option naming a table in a worker | No cell | REFUSED as on main; existing Rust refusal test |

## Fact table

No runtime condition changes in this review round. The existing branch removes
the CalcFields-only Truncate veto from `cached_group`. Its owner remains
`query_cache.rs::cached_calc_fields`, using `Ends::Sources`.

| Fact | Positive equality in the existing owner | Evidence/neighbor |
|---|---|---|
| AL read kind | `al_record_read == Some(RecordOp::CalcFields)` | All four Truncate cells |
| Receiver storage | Database or temporary, with a separate temporary key component | 958, 1178 |
| Initial reader | Worker or test session before further writes | 958, 1184 |
| Calculation entry point | Record.CalcFields or database FieldRef.CalcField | 958, 1190 |
| Formula source | Count of BCE Line, Parent equals assigned primary key | All four Truncate cells |
| Open mode | Database RecordRef open without the temporary flag | 1190 |
| Entry lifetime | Source commit stamps equal; worker select stamp equal | All four Truncate cells |
| Row-table commit kind | Truncate | All four Truncate cells; row-table commit intentionally is not a source stamp |
| No-table selects | Value kind equals Integer, value equals 0 or 2, resolution equals None | 1196, 1202 |

The cells compare outcomes through the public background-task/Record/FieldRef
seams. They also check every newly admitted reader of the existing assumption
mark. No mark, flag, budget, or refusal is added in this review round.

## Copy and pin retirement

The previous full corpus copy moved from
`8677-bg-cache-calcfields/src/EdgesProbe.al`; no second copy remains.
`outside_fb_8637::issue_1555_cells` now includes this copy and checks all four test
codeunits (16 + 9 + 8 + 5), including their assumption marks.

After both merged follow-ups are pinned,
remove this copy and the Rust copied-cell test. Keep the independently authored
neighbor apps. Do not remove the follow-up copy merely because a bump pins the
original 33 cells.
