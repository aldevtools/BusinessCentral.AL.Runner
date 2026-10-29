# Architecture

How the runtime is built today. The binding decisions are the ADRs in
`docs/adr/`; this page is the map.

## Pipeline

```
AL source
  → lexer → preprocessor (#if, token level)              al-syntax
  → lossless CST (rowan green trees, error tolerant)     al-syntax
  → workspace: apps, objects, namespaces, extensions     al-sema   (parallel per file)
  → member tables, type check, name/overload resolution  al-sema
  → lowering to a fully resolved typed IR                al-sema → al-ir
  → register bytecode compiled from the IR, run by al-vm   al-vm
  → in-memory store (persistent maps)                    al-data
```

Anything the pipeline cannot handle is refused. Lowering records a reason per
routine, and the runner reports a test as **not run** if anything it can reach
statically is refused. What can only be discovered while running raises
`Signal::Unsupported`, which is also reported as not run. Nothing unsupported
runs with a guessed behaviour.

The IR is compiled to register bytecode per routine on its first call (ADR-034); the bytecode is the only executor
(the tree walker was retired on 2026-10-16).

## Crates

| Crate | Responsibility |
|---|---|
| `al-syntax` | Lexer, `#if` preprocessor, error-tolerant lossless parser, typed AST views, fast green-tree builder |
| `al-sema` | Workspace of apps and objects with namespace resolution; members (fields, keys, CalcFormula, TableRelation, page controls); type checker; lowering to IR |
| `al-builtins` | Builtin catalog, generated from the corpus surface file plus evidence-backed overrides, keyed by overload id (`Record.Insert#2`); the value model; natives for text, format/evaluate, filters, JSON, XML, streams, collections, refs |
| `al-ir` | Typed IR: program, objects, routines, places, expressions, table and page schemas |
| `al-vm` | Engine (immutable, `Arc`) and Session (all mutable state). Records, triggers, events and subscribers, FlowFields, AutoIncrement, RecordRef/FieldRef, relations and rename propagation, UI dialogs and handlers, the headless page client |
| `al-data` | The `Store` trait (ADR-003), `MemStore` on `im` persistent maps, and its contract suite |
| `al-host` | Capability traits for the host (ADR-006): `Clock` and `Identity` with AL Runner's defaults (ADR-013); the session reads them for `Today`, `UserId`, the system field stamps and the like. The command line injects the system clock (`al-test-cli/src/host.rs`). Also `DotNetHost` / `DotNetSession` and the default `NoDotNet` (ADR-014) |
| `al-deps` | Dependency apps (ADR-013): version selection, the ZIP and NAVX reader (CRC checked), the `.alpackages` pool. May touch the OS |
| `al-bcdb` | Reads BC data straight from a SQL Server backup (`.bak`) or a cloud export (`.bacpac`): backup container, page map, system catalog, row and page decompression, off-page BLOBs, zip and `model.xml`, native BCP rows. A port of the owner's `bcdb` (C#). Core crate: works on a `ByteSource`, no OS access (ADR-006). Not wired into test runs yet; ADR-020 proposes how. See `docs/bcdb.md` |
| `al-bcdb-std` | The OS side of `al-bcdb`: the pread file source and `BackupProvider`, the table provider behind `al-runner test --test-data` (ADR-020). May touch the OS |
| `al-bcdb-cli` | The `al-bcdb` development command line: the bcdb read commands over `al-bcdb`, a parallel `export`, and the parity tests against the published bcdb. May touch the OS |
| `al-test-cli` | Corpus discovery, oracle reading, the BC test-runner semantics, reports (text, Markdown, JSON, JUnit), the ratchet; `SidecarHost` (`dotnet.rs`), the one place that starts a process (the .NET sidecar) |

## Runtime model

- **Engine and Session (ADR-001, 002).** The compiled program is immutable and
  shared. A Session holds the store, temporary stores, single-instance
  codeunits, event bindings, the UI handler scope and the page clients. It is
  synchronous and uses `Rc`/`RefCell` inside.
- **Values (ADR-010).** One `Value` enum. Value types copy. List, Dictionary,
  Json, Xml and stream types share. A Record is a handle onto a field buffer,
  filters, key and position; `:=` copies fields.
- **Store (ADR-003, 004).** AL-shaped operations with typed outcomes.
  Snapshots are root pointers, so commit, rollback and per-test reset are O(1).
  Filter text quotes literal comparison characters, including `>`, and doubles embedded
  apostrophes. `SetView` at run time reads those quoted Text/Code values as literals; in an object
  property (`SourceTableView`, `SubPageView`, ...) a quoted `'>B'` still compares (corpus
  `filters/W4Filters.al`, `record/TestFilterQuotedLiterals.al` and
  `filters/lt-uc-quoted-values/UcQuotedValues.al`).
- **Test runner semantics** (pinned by the corpus):
  - One session and one fresh store per test codeunit; install codeunits run first. A test
    codeunit is the **isolation unit** (`al-test-cli/src/isolation.rs`): its methods share the
    instance (AL globals, single-instance codeunits, one `OnRun`) under every `TestIsolation`, so
    methods are never scheduled apart. Each unit forks the install baseline (O(1)) and shares
    nothing mutable with another.
  - `TestIsolation = Codeunit` (the default, and what every corpus oracle ran under) keeps the
    database between methods; `Function` restores the state after `OnRun` when each method ends
    (O(1) snapshot restore); `Disabled` is refused. A runner codeunit's property selects it
    (`oracle.toml` `runner`).
  - A passing test commits at the test boundary.
  - `asserterror` rolls back to the last commit.
  - `[HandlerFunctions]` are bound per test method; "Unhandled UI" and unexecuted handlers fail the test.
- **Pages (ADR-012).** Pages lower to a control tree, and their triggers are
  ordinary routines. A headless client in `al-vm/src/pages.rs` plays the BC
  test client: open/close, navigation, SetValue and validation, actions, and
  modal and page handlers.
- **XmlPorts.** An xmlport lowers to an object whose first globals are its
  element variables (text and table elements, in schema order), two hidden
  stream slots, then its declared variables; the schema is `Object::xmlport`.
  A variable is a codeunit-like instance. `Import` and `Export` run in
  `al-vm/src/xmlports.rs` over the document tree of `al-builtins/src/xmlport.rs`.
- **Reports.** A report lowers to an object whose first globals are its data item
  records (source order), then a hidden request-page flag, then its declared
  variables; the dataset is `Object::report`. A variable is a codeunit-like
  instance and its first use runs `OnInitReport`. `Run`, `RunModal`, `Execute`,
  `RunRequestPage` and `SetTableView` run in `al-vm/src/reports.rs`: processing
  only, with no layout or output (a run that gets as far as output is refused).
- **Request pages (ADR-012).** The request page of a report is `Object.page` of the
  report: the same test client, over the report's own instance. Its controls bind to
  the report's variables, each data item is a `DataItem` control over the item's
  record, and a reportextension's request page merges in like a page extension. A run
  that shows the page opens it in `al-vm/src/pages/request.rs`, hands it to the
  `[RequestPageHandler]` (`UiKind::RequestPage`, bound by the report) and goes on if the
  handler confirmed with OK. `RunRequestPage` returns the parameters text.
- **FilterPageBuilder.** `LibraryValue::FilterPageBuilder` holds named controls, each
  a record buffer; `al-vm/src/filterpage.rs` runs it. `:=` copies the control list and
  shares the records. `RunModal` is refused.
- **Culture.** Fixed en-US, which matches the oracle.
- **DotNet (ADR-014).** A `dotnet { assembly(..) { type(..; Alias) } }` block declares
  aliases; `X: DotNet Alias` lowers to `RtType::DotNet` and member uses to `Expr::DotNet`
  (constructor, call, property get and set), whose results have no static type and are coerced
  exactly where they are stored. In `al-vm/src/dotnet/` a call goes to the registry of
  runtime-provided classes (`registry.rs`, empty until probes pin members), then through the
  policy (`policy.txt`, section 8 of the ADR), then to the host (`al-host::dotnet`), else it is
  refused. `al-test-cli/src/dotnet.rs` implements the host with a .NET 8 helper process
  (`tools/dotnet-sidecar`), one per Session, started at the first call that reaches it. A
  .NET exception is `Signal::Unsupported` with its CLR type and message until a probe pins
  BC's text. A machine without .NET refuses with the `dotnet-unavailable` marker; the ratchet
  skips such baseline cases locally and `--require-dotnet` (CI) makes them an error.

### Text values

`Value::Text` and `Value::Code` hold a `Txt` (`al-builtins/src/text.rs`): a string behind an `Arc`, copy on write.
A variable read, an argument, a return value and `text_of` clone the handle (a counter bump), not the text. A write
(`make_mut`, `push_str`) copies only when another value shares the string. `S += x` and `S := S + x` on a Text
variable take the text out of the variable and append in place (`Session::append_text`), so a loop is linear.
`Txt` remembers whether it is ASCII (then a UTF-16 position is a byte position) and whether it holds the call-stack
marker U+FDD0 (a noncharacter: private-use characters are real text); both are computed once per string and dropped on a write. The counter and flags are atomic because
the rows of the test data are shared between threads (`SharedMat`). This is an internal representation (ADR-009):
nothing AL code can see changed. Still linear per call: `Text.Contains`, `IndexOf`, `Substring` on a long text
(they scan it, as BC does), and any write to a text that is also held elsewhere (it copies once).

## Source positions

Lowering records where statements and routines sit in the source. Coverage, call
stacks, value capture, loop tracking and a debugger all start from this table.

**The data** (`al-ir`):

- `Program::positions` is a table of `SrcPos { file, line, col, end_line, object, routine }`.
  A position is named by its index, a `u32`. Lines and columns count from 1; the
  column is a byte column. Lines are the author's lines: the compiler's added page
  expression routines do not move them.
- `Program::sources[pos.file]` gives `{ app, path }`. The path is relative to the app.
  `pos.object` and `pos.routine` index `Program::objects` and `Object::routines`, so a
  position names the routine or trigger it is in without anything else. The app is
  `Object::app`.
- `Routine::pos` is the position of the `procedure` or `trigger` keyword (after
  attributes and modifiers). It is `NO_POS` for a routine the compiler made up.
- `Stmt::Pos(id)` is a marker statement. It sits directly before every statement it
  describes, in the same list, nested statements included. Blocks (`begin ... end`) have
  none, because lowering flattens them. A statement that fails to lower leaves no marker.
- Call expressions have no position of their own. A call is found through the statement
  that contains it, so a call on a later line of a multi-line statement reports the
  statement's first line. Giving `Expr::Call` and `Expr::Builtin` an optional position
  later is additive; nothing needs it yet.

**From an executing statement to its position:**

1. `Session::call_stack` (`al-vm/src/lib.rs`) holds one `CallFrame { routine, pos }` per
   routine on the call stack, parallel to `Session::frames` (which holds the object).
2. Running `Stmt::Pos(id)` sets `pos` of the top frame. After a call returns, the caller's
   frame still holds its own statement. `while` and `repeat` set it back to the loop's own
   position after each pass, so the loop condition is attributed to the loop line.
3. `program.pos(frame.pos)` gives the `SrcPos`; `program.sources[pos.file]` gives the file.
   `program.pos(routine.pos)` gives the routine header.
4. The code that wants to see every statement as it starts (coverage, value capture,
   a debugger) hooks the `S::Pos` arm of `Session::exec`. The hit is "position `id` ran".
   Nothing else has to change.

**Where the positions go wrong, so nobody trusts them too far:** the table has no entry
for page expression routines (`pos` is `NO_POS`, the call stack marks such a frame
unreadable), and the table is only built when lowering is asked for positions
(`lower` does; `lower_with_positions(.., false)` does not, which keeps unit tests of
lowered bodies free of markers).

**Error call stacks** (`al-vm/src/callstack.rs`). An error does not carry its stack from
the point where it is raised. Each routine it leaves adds one line (innermost first),
and the routine that catches it (`asserterror`, a TryFunction call, a guarded
`Codeunit.Run`) adds the last. The line format, and which parts the corpus pins, are in
that file's header and in `docs/probes-needed.md`.

## Apps in the corpus

| App | Role |
|---|---|
| `tests/al-language` (Target = Cloud) | The main test app. |
| `tests/al-language-internals-fixture` | A dependency of the test app (CI publishes it first). Loaded as a second app: some `codeunit/` tests call its objects across the app boundary (`ALT Internal Codeunit`, `ALT Cross Compute`). |
| `tests/al-language-xapp-a`, `-b`, `-c` | Cross-app fixtures (no tests) the test app depends on: B depends on the internals fixture and A, C on A and B. Used by `events/lt-bw-subscriber-order` and `tableextension/lt-bw-cross-app`. |
| `tests/al-language-onprem` (Target = OnPrem) | Not in the main oracle (a separate CI job and artifact). Run as the `onprem` suite against its own oracle (`oracle/README.md`). |
| `tests/al-language-isolation-probe` | Only in the Windows nightly, under two custom test runners. Run as the `isolation-function` and `isolation-codeunit` suites (BC 28 only). |
| `tests/must-not-compile` | AL files the real compiler rejects, each with the expected AL codes. |

The runner loads every corpus app the test app depends on (directly or through another of them) from source, in
dependency order, before the test app: `al_testrun::deps::corpus_source_dirs` reads the `app.json` files of the folders
next to the test app. No list of folders is kept in the runtime. Language coverage per area is in
`docs/corpus-coverage.md`. The first inventory of the corpus (2026-10-13) is in `docs/history/corpus-inventory.md`.

## Budgets (tests, run in CI in release)

| Budget | Limit | Today |
|---|---|---|
| Cold compile of the corpus (`al-test-cli/tests/perf_budget.rs`) | ≤ 695 000 instructions per KiB of corpus source (the day's value plus 5 %; the per-object value changes when the corpus objects grow); CPU-time fallback 695 000 ns per KiB. Wall time is printed, not budgeted: on shared CI runners the same code ranged from 0.8 to 1.0 s and from 2.4 to 3.0 s of CPU between runs | 664 306 instructions per KiB (corpus bump 15 with ADR-041, 19.2 MB of source; 662 399 before) |
| Interpreter, seven programs of `al-test-cli/tests/vm-bench` (`perf_vm.rs`, ADR-031) | instructions per iteration, on the bytecode (the default since ADR-034 step 4): Integer loop 534, Decimal loop 1821, call 2525, text 5876, builtins 6267, records 16261, `RecordRef.FieldIndex` 4764 (call, text and builtins: the value of 2026-10-19 plus 10 %, FieldIndex that of 2026-10-27, the rule of `perf_vm.rs`; lowered by each fast path) | 507 / 1 803 / 2 295 / 5 342 / 5 697 / 14 916 / 4 331 |
| Full corpus run with Microsoft's apps (`perf_budget.rs`, BC 29, whole process tree) | ≤ 155 G instructions (the day's value plus 5 %); fallback without the counter: 45 s of child CPU time (CI measured 25.8 s and 34.7 s, owner-review 2026-10-16). Seconds are printed, not budgeted: with the same code 12 s was exceeded at 21.5 s on a machine with load 25 | 143.9 G with ADR-041 positions in the dependency IR (137.1 G before; stable to 0.02 % between runs) |
| Serial parse throughput (`al-syntax/tests/perf_parse_throughput.rs`) | ≥ 50 MB/s | ~65–75 MB/s |

`cargo run --release -p al-test-cli --example phases` prints timings per
phase. `--example compile_dump` dumps every compile output so a performance
change can be shown to change nothing.

## Dependencies outside the workspace

Today only the corpus apps are compiled. Objects from Microsoft apps (System
Application, Base Application, platform system tables) are refused as
"outside the workspace".

ADR-013 (Proposed) is the plan for them: compile Microsoft's own AL source,
fetched from the BC artifact feed into a local cache, and implement natively
only the platform objects that have no AL body.
