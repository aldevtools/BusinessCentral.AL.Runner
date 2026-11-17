# Hand-off: area-test-isolation-state-1117

## Done
- #8239 (partner login changes a SingleInstance codeunit outside the pinned shape): the refusal now applies only to a
  test codeunit whose static reach reads a global of that codeunit, or whose reach is not fully known
  (`partner_login.rs` `unmeasured_single`, `unmeasured_seen`). Tests in `runtime/partner_login_tests.rs`.
  The reporter's `ReadsNothing` runs; `IsSaaSAfterLogin` stays refused until the probe below pins the cross-app shape.

## Next, in order
1. #8238 (promised to the outside reporter). Design:
   - Owner: `runtime/blocked.rs` `subscribers_reached_from`. Today it adds `events.subscribers_of(t)` for every table t
     of any Record slot (locals + globals) of each visited routine.
   - New allowlist predicate `reads_only(objs, o, r, t)`: object o is a codeunit without a TableNo property; every
     occurrence of every Record(t) slot in routine r is the receiver of a read builtin (Record.IsEmpty, Get, Find,
     FindFirst, FindLast, FindSet, Next, Count, SetRange, SetFilter, Reset, SetCurrentKey) or a field load. Count all
     place occurrences: Expr::Load, Arg::Ref in Builtin/Call/InterfaceCall/DotNet args and receivers (use
     `al_ir::types::for_each_expr_post`), plus Stmt::Assign targets and both For vars. Any other use: main's result.
   - When it holds, skip only the subscribers of the ten table events (on(before|after)(insert|modify|delete|rename|
     validate)event) of t for that routine; keep the table's other subscribers.
   - Unpinned: no corpus case pins that a read raises no table event (TestTableEventDispatch.al only pins that writes
     do). Needs an assumption entry `table-event-not-raised-by-read` (awaiting the probe below) and a mark: compute
     `install::touched` with and without the narrowing; a test that reads a table only the old set holds marks it.
     Wire it next to the skipped-tables refusal check (`install.rs` `refuse` / `skipped`).
   - Red test: the reporter's repro as a Rust test (stopped install routine + `Record Company` IsEmpty in a helper).
2. #8152 follow-ups promised in the owner's comment: (a) the default run's `number order:` report should list a codeunit
   that only reads `NumberSequence.Current` after one that takes numbers (`runtime/order_hint.rs` `numbers_order`);
   (b) `--strict-order --isolation test`: the reader gets 100; refuse that cell until a probe measures it.
3. The rest of the area: 8218, 8192, 8115, 8104, 8063, 8062, 8056, 7640, 7622, 7620, 7619, 7611, 7609 (none started).

## Probes needed (spec)
- P1 (#8238): table events on reads. A table with subscribers to all ten table events that log to a log table. Run
  IsEmpty, Get, Find('-'), FindFirst, FindLast, FindSet+Next, Count, SetRange/SetFilter/Reset/SetCurrentKey and a field
  read; assert the log is empty. Same on the Company table (OnBefore/AfterDeleteEvent subscribers) with IsEmpty and Count.
- P2 (#8239): a partner dependency app's OnAfterLogin subscriber calls EnvironmentInformation.IsSaaS(); the test calls
  EnvironmentInformation.SetTestabilitySandbox(true) then IsSaaS(): false means the login's memo
  (IsSaasInitialized/IsSaaSConfig) stayed in the test session. Also a partner SingleInstance codeunit of another
  partner app with a Boolean global set by the login, read by the first test.

## Traps
- `run_codeunit_inner` and several al-testrun files are near their length caps.
- The worktree builds into its own target/ (CARGO_TARGET_DIR=<worktree>/target).
