# Flows

Call chains of the main flows. `docs/codemap.md` says which module owns what.

Each chain is `caller -> callee` in call order; `[a, b]` means both happen in that order. Names are
functions or methods; the file is in `docs/codemap.md`.

### Corpus run: from main to a test outcome

1. `main` (al-test-cli/main.rs) starts a thread with a large stack and calls `run`.
2. `run`: `args::parse` -> `config::OracleFile::parse` (oracle/oracle.toml) -> per BC version `run_version`.
3. `run_version`: `pin::check` -> `oracle::parse_junit` (cases of real BC) -> `corpus::discover` ->
   `select::Selection::filter` / `find_explained` -> `runtime::load_app` per app -> `deps::load_corpus`
   (Microsoft apps from `--deps` or the artifact cache of the pinned build; the corpus tree's `.alpackages` is not
   searched, ADR-013 amendment of 2026-11-10) -> `CorpusRuntime::run_suites_with`.
4. `run_suites_with`: `al_sema::lower::compile_apps_with` (compile pipeline below) -> `Engine::with_config`
   (`events::Registry::build`) -> `global_triggers_find_unavailable` -> `baseline::run` (install roots, once) ->
   for each test codeunit `run_codeunit_isolated` -> `run_codeunit_observed`. Test codeunits run on worker threads
   (`AL_TEST_THREADS`, `run_unit`); a baseline cannot cross threads, so each worker runs the install roots again
   on the engine that ran them first. `--explain` keeps one thread. `--timings` prints the compile parts and the
   slowest codeunits.
5. `run_codeunit_observed`: `Session::with_host` -> `restore_baseline` -> `run_install_codeunits` -> `OnRun` via
   `Session::call` -> per `[Test]` routine: `resolve_handlers`, `blocked_site` (refusal decided at lowering),
   `bind_handlers`, then `Session::call` -> `call_inner` -> `exec::compile` (once per routine) -> `Session::run_chunk`. The result becomes
   `Outcome::{Pass, Fail, NotRun}` (`Signal::Error` is a failure, `Signal::Unsupported` is a refusal).
6. Back in `run_version`: `report::build` (counts by area) -> `output::junit` / `markdown` -> `check_lists`
   (ADR-065): `not_passing::new_cases` with `--list-new-cases REV` (base oracle from `base_oracle::Base`), then
   `not_passing::check` and `decide` against `metrics/<major>/not-passing*.txt`, stale lines deleted with
   `--prune-not-passing`. Reading a run: `insight::top_refusals`, `not_passing::render_diff` (`--baseline-diff`),
   `explain::render`.

### `al-runner test`: from the command line to a result

1. `main` (alrunner-cli/main.rs) starts a thread with a large stack; `args::parse` returns `Command::Test`.
2. `run::test`: `apps::load` per APP (`al_deps::load::read_package_at_with`: folder, `*.Source.zip` or `.app`) ->
   `discover::test_codeunits` over each APP's files (which test codeunits exist, which the selection picks) ->
   `al_version::choose_target` -> `apps::combined_project` (the APPs' dependencies, plus the `Application` and `System`
   floors as declared Microsoft dependencies) -> `deps::load_project` (search list: `--dep`, `--package-cache`, each folder
   APP's `.alpackages`, then the artifact feed; `al_deps::load::scan_path` reads a directory, a package file or an app folder;
   a Microsoft package in `.alpackages` older than the run's BC version gives way to the feed, `artifact::load_with_artifacts`).
3. An APP with an `application` or `platform` floor gets the dependencies it implies (`Dependencies::implied_dependencies`),
   so it sees Base Application without declaring it.
4. `CorpusRuntime::run_suites_with` (the same compile and run as the corpus job; see "Corpus run" step 4), with
   `RunOptions::only` set to the selected codeunits. `CorpusRuntime::results` lists every test with its codeunit.
   Test data is on unless `--no-test-data`: `run::choose_test_data` finds the backup (`testdata::locate`: the build of the
   Microsoft apps on disk, in the artifact cache, downloaded by `al_deps::demo::ensure_demo_backup`, or another build of
   the version) and opens it (`al_bcdb_std::BackupProvider`); without a backup the run says why test data is off. The
   build of the tests (`apps::own_build`) is `BuildQuery::preferred`, so the apps, `System.app` and the data share it.
   The backup is passed as `RunOptions::test_data`; `run_suites_with` puts it on
   each `Engine`. `Session::with_host` gives the store a `TestBacking` (al-vm/testdata.rs): a table the store holds no
   entry for reads `provider.read` once for all threads (`TestData::read`), maps columns to fields, and builds the
   table once per thread. `run::test` prints `LoadStats::summary` before the test summary.
5. `run::test` filters to the selected tests, prints failures, `refusals::rank` / `render_top`, the summary line and the
   timing line, writes `output::junit`, and returns `output::exit_code`.

### Compile pipeline: parse -> workspace -> demand -> check -> lower

1. `lower::compile_apps_with` -> `compile_with_roots`.
2. `Workspace::load_apps` (al-sema/workspace.rs): workspace apps are parsed (`parse_file`, in parallel,
   `pages::expand_or_parse` first for page expressions, which splices the generated routines into the first parse); dependency apps are only scanned (`scanned_file` ->
   `al_syntax::scan::scan`) and parsed later on demand (`Workspace::green`). Then `index_extensions_and_subscribers`
   and `resolve_references`.
3. `members::build`: fields (extensions merged), enum values, procedure signatures.
4. `check::check_with`: `check_object` for every workspace object (a test selection leaves unselected test codeunits
   out: `Roots::only_tests`), then `reach_baseline` (install roots), `reach_dependencies_on_workspace` (a dependency app that stands on a workspace app is reached whole, as the compile cache compiles it; al-runner issue 6044) and `run_demand` (check/demand.rs):
   `activate` / `activate_table` / `activate_ops` / `activate_global` queue routines, `check_reached` checks them;
   files of newly reached dependency objects are parsed ahead on worker threads. Result: `CheckResult`
   (types and resolutions by node position, `Demand`). The demand loop runs in rounds of tasks (check a workspace
   object or a reached routine, activate a raised object); worker threads (`AL_CHECK_THREADS`) take chunks of a
   round, each with its own `Checker`, and report deltas that the calling thread merges (`demand.rs`: `Task`,
   `Pool`, `Delta`). Types and resolutions are a position map in 16 shards merged one thread per shard.
5. `lower::lower_with_positions`: `Workspace::restrict_to` (what demand did not reach stops resolving), then per object
   `Lowerer::routine` -> `stmt` / `expr` / `call` for reached routines and `stub` for the rest. Anything inexpressible
   becomes a refusal reason in `Lowered::unsupported`. Output: `al_ir::Program` (an `Arc`, shared with the engine) +
   call graph. `lower_object` is a function of the plain `Shared` data plus per-thread `Local`/`Seq` (syntax nodes
   and layouts belong to the thread that built them): worker threads (`AL_LOWER_THREADS`) and the calling thread take all active objects in turn,
   each lowered with positions numbered from 0 that `Routine::shift_positions` moves into place in object order.

### Compile cache: a run that finds the dependency apps already lowered (ADR-022)

1. `CorpusRuntime::try_run_suites_with` (testrun/runtime/corpus.rs), with `RunOptions::compile_cache`: the apps are put in the
   program order a cache needs, dependency apps first, Microsoft's before the others and every app after the apps it
   depends on (`compile_cache::dependency_order`); a dependency app that stands on a workspace app is not cached and comes
   after the cached ones (`dependency_apps_on_workspace`, al-runner issue 6044; the caller's order is kept in `order` for `TestResult::app`,
   `SuiteRun::app` and `RunOptions::only`). No cache with `AL_EAGER_DEPENDENCIES` or without dependency apps.
2. `compile_with_cache` -> `compile_cache::open_or_build_with`: the dependency set is cached in layers (owner decision
   2026-10-15): `layer_ranges` (layer 1 = the Microsoft apps, then one layer per other app), `layer_keys` (each layer's key
   holds its own apps and the digest of the key below: executable hash, target, symbols, manifest and source hash) ->
   `read_entry` per layer (`al_cache::Entry::open` checks header, index, dictionary; a refused file is removed and
   reported). Each missing layer is built, lowest first: layer 1 = `lower::lower_for_cache` (the complete compile of the
   Microsoft apps), a higher layer = `lower::LayerBuilder::lower` (compiles the layer's apps with the layers below taken
   from a `provided::Stack`, keeps its own objects and the lower objects it changes) -> `al_cache::build_layer` ->
   `write_entry`. All missing layers above the first share one `LayerBuilder` workspace: it loads the apps up to the
   lowest of them once and grows by `Workspace::append_apps` for each next one (ADR-022 amendment 2026-10-27);
   `ALRUNNER_LAYER_BY_LAYER=1` loads the apps below again per layer (`lower::lower_layer`, the same bytes).
3. `lower::compile_loaded_cached` on the `Stack` of all layers (an object comes from the highest layer that holds it):
   `Workspace::load_apps` for all apps; `provided::check_identity`; `relower_set` (the
   dependency objects whose `DepFacts::embeds` the workspace extends or implements, with the objects that extend each
   other); the blocks of the others are read in parallel (`DepSource::block`); the checker skips them
   (`Roots::provided`); `lower_core` takes their pieces from the blocks and their facts (`Shared`: page reasons, parts,
   routine numbers, ...) from `DepFacts`, and lowers the rest as the eager compile does. An error gives the workspace
   back and the run compiles without the cache.
4. `Lowered::calls_of` / `calls_into` give the call graph: the direct calls stored in the block plus what a page,
   report or xmlport variable drives (`driven_edges`).

### Record.Insert(true): trigger, events, Global Triggers, store

1. `Session::eval` (al-vm/lib.rs) on `Expr::Builtin("Record.Insert#2")` -> `al_builtins::native::lookup` -> the entry
   in `native.rs` calls `Ctx::record("Insert", ..)` -> `Session::record` (lib.rs) -> `records::record_op`, arm `"Insert"`.
2. `record_op`, in order: `auto_increment_field` -> `table_event("onbeforeinsertevent")` (`listened`, then
   `dispatch::raise`) -> `assign_system_id` -> with RunTrigger: `run_ext_trigger("OnBeforeInsert")`,
   `run_trigger(OnInsert)` (`Session::call`), `run_ext_trigger("OnInsert")` -> `globaltriggers::global_trigger(Insert)`
   (`global_setup`, then `raise` on the platform codeunit `Global Triggers`, event `ondatabaseinsert`) ->
   `check_system_id_unique` (Insert(_, true) keeps the buffer's SystemId) -> `bump_row_version`, `stamp_insert` ->
   `take_auto_increment` -> `unique_violation` (duplicate key fails here, `fail_unique`) ->
   `store_for(rec).insert` (al-data `Store::insert`, `MemStore`) -> `run_ext_trigger("OnAfterInsert")` ->
   `table_event("onafterinsertevent")`.

### TestPage open

1. `Session::eval` on `Expr::Builtin("TestPage.OpenEdit#1")` -> native entry -> `Ctx::page` -> `pages::page_op`
   (wraps a page transaction: `in_page_transaction`) -> `page_op_inner` -> `test_page_op` -> `test_page_op_inner`
   (`"OpenEdit"`, `"OpenView"`, `"OpenNew"`) -> `page_open` -> `open_page` -> `open_page_as`.
2. `open_page_as`: `page_instance` / `construct_page` (instance of the page object) -> `page_trigger("oninit")` ->
   UI handler lookup (`ui.rs`, `resolve_handlers` in the runner) -> `check_page_events` ->
   `page_trigger("onopenpage")` and `page_event("onopenpageevent")` -> `load_first_row` -> `populate_current` /
   `populate_window` (control expressions, FlowFields). Controls are then read and written through `test_page_op_inner`.
3. Closing: `page_close` -> `close_with` (OnQueryClosePage, OnClosePage, saving values).

### Event raise

1. A call to a publisher routine: `Session::call` sees `engine.events.publishers` holds `(object, routine)` ->
   `dispatch::publish` (converts parameters, builds `EventCall`) -> `raise`.
   Table events build an `EventCall` themselves (`records::table_event`), `Global Triggers` events too
   (`globaltriggers::global_trigger`), page events via `pages::page_event`.
2. `raise`: `event_targets` (subscribers from `engine.events.subscribers[(object, event)]`, static or bound instances
   via `static_instance` / `bound_instances`; a Global Triggers subscriber that cannot run refuses here) -> per
   subscriber map parameters by name -> `Session::call(subscriber)` -> `var` values are written back into the arguments.
3. The registry is built once per program: `events::Registry::build` reads the `EventSubscriber` attributes of the IR.
   At compile time the same relation is `Workspace::subscribers_of`, which demand uses to reach subscribers.

### Guarded Codeunit.Run

1. `if Codeunit.Run(..) then` lowers to `Expr::Builtin("Codeunit.Run#1" | "#3")` with `ret_used = true`.
   `Session::eval` (lib.rs, the `Codeunit.Run#` branch) evaluates receiver and arguments, then
   `transactions::codeunit_run(id, recv, args, guarded = ret_used)`.
2. `codeunit_run`: unguarded -> `bg_refuse_transaction_call`; resolve the codeunit (`shared_instance` / `instance`);
   refuse a test codeunit started from a test (`nested_test_codeunit`); TableNo codeunits take the record as `Rec`;
   guarded and `in_write_transaction` -> `transaction_stopped()` (BC refuses a guarded run over an uncommitted write).
3. `Session::call(object, OnRun)` inside `tx_world += 1`. `Ok` + guarded -> `commit_transaction`, returns true.
   `Err(Signal::Error)` + guarded -> `add_top_frame` (call stack text), `rollback_transaction`, `last_error` set,
   returns false. Unguarded errors propagate. `Signal::Unsupported` always propagates (a refusal, not a failure).

## Reading a failure or a refusal

`al-test-cli --explain TEST` (explain.rs) shows the outcome, the reason and, with `Session::trace_faults`
(al-vm/faultsite.rs), the call stack the error or refusal left from, each frame with `file:line:col`
(`Program::positions`, docs/architecture.md "Source positions").
