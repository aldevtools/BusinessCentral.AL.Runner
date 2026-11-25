//! Session state outside al-vm's `Session` that al-testrun keeps (al-runner issue 8211): every `static` of al-testrun (a
//! thread-local or a process table, test code left out), each listed with what a login start does with it, as
//! `crates/al-vm/src/session_fields.rs` lists those of al-vm, al-builtins and al-data.
//! al-test-cli `tests/outside_session_state.rs` fails for a `static` of al-testrun that is not listed here, and for a listed
//! one that is no longer there.
//!
//! A start is taken in `runtime/run.rs` `open_company`, on the session's own thread, in place of the login; the open's
//! writes and assumption marks come with it (`login_start::Start`). Each owner is one of:
//! - **untouched**: a replay neither reads nor writes it. The login ran no test and no lane step, so nothing here changes
//!   between the recording session's login and a replay (the owner says why).
//! - **gated**: the state exists only in a run that follows the code it runs (coverage, the profiler); such a run takes no
//!   start (`login_start::run_gate`, `NotTaken::NotesCode`).
//! - **key**: a serve request sets it and the login can read it: a field of the login key holds it by content, so a start
//!   is never taken under another value (the field is named).
//! - **run**: one value for the whole process or thread run, set once before any session, not of one session or request
//!   (a cache of the program, an id counter, a diagnostic, a setting of the process start).
//!
//! The inputs of a login that are neither a `Session` field nor a `static` are listed here too, under the same test:
//! each field of the run's plan for one test codeunit (`runtime::Scratch`) and each engine field al-vm's
//! `Engine::reset_run_state` lists as "Set by a run". Besides the owners above, each is one of
//! - **engine**: a warm server keeps an engine for a later request only when the request has the same value
//!   (`runtime/corpus.rs` `kept_engine_key`), and the starts live on the engine (`Engine::derived`; a sibling engine
//!   has its own), so every start of an engine was made under the value a session of it has.
//! - **after**: read or written only after the session opened the company, or not read by the login at all (what the
//!   tests report, how they are chosen and run).
//!
//! Fields a run sets:
//! - `crates/al-testrun/src/runtime/mod.rs` (`Scratch`): `field flags`: key (`Key::flags`, the whole value).
//!   `field run_seed`: key (`Key::seed`, the session's run seed). `field hub`: key (`Key::isolation`).
//!   `field installs`: key (what the install code left for the session, `install_once.rs`, is in its rows and state
//!   before the login: `Key::database`, `Key::session`). `field numbers_unmeasured`: key (the runner sets it on the
//!   session before the login, `enter_test_codeunit`, and its line is in `Key::session`). `field clock`: key (begun
//!   per run, `host::RunClock`; `Key::clock`, the session's clock before the login). `field clock_set`,
//!   `field perturb`: gated (`NotTaken::Perturbed`). `field trace`, `field driver`: gated (they turn the impact trace
//!   on, which `Session::notes_code_run` reads: `NotTaken::NotesCode`). `field capture`, `field order`,
//!   `field messages`, `field captured`, `field traces`, `field stubbed`, `field ui_answered`, `field cells`,
//!   `field slept`, `field skipped`, `field entered`, `field run_only`, `field hub_leg`, `field leak`,
//!   `field leak_hits`, `field onrun_carry`, `field read_tables`, `field discovery`, `field on_run_rows`,
//!   `field report_unpinned_errors`, `field skip_after_timeout`, `field stop_after_first_failure`, `field runner`,
//!   `field watch`, `field singles_past_stop`, `field singles_out`, `field unfinished_after_start`,
//!   `field refused_mid_run`, `field test_panicked`, `field unfinished_at`, `field order_hints`: after (what to
//!   capture, the runner codeunit, the single instance watch, the carries and the leak are taken after the open; the
//!   rest chooses, limits or reports the tests).
//! - `crates/al-vm/src/lib.rs` (`Engine`, "Set by a run"): `field emit_version`: key (`Key::emit_version`).
//!   `field unseeded`, `field demo_tables`: key (`Key::refused_reads`). `field resource_overrides`: key (the texts of
//!   the files the login read, `StartKey`). `field test_data`: key (`Key::backing`) and engine. `field license`: key
//!   (`Key::license`) and engine; `field license_caps`: engine (worked out from the license on first use).
//!   `field skipped_apps`,
//!   `field write_in_try_allowed`, `field dependencies_published_global`: engine. `field company_refusal`: after (with
//!   it no session of the run opens the company, `run_codeunit_inner`). `field resources_read`, `field dotnet_called`:
//!   after (written by any session that reads a resource file or makes such a `DotNet` call, and read only when the
//!   install baseline is kept, before a test session opens the company; a replayed login writes neither).
//!
//! Thread-local and process tables:
//! - `crates/al-testrun/src/events.rs` `static CURRENT`: untouched (the hooks and reported names of the running test; the
//!   login reports no test and reads only the stop flag, and the debugger attaches after the open).
//! - `crates/al-testrun/src/runtime/run.rs` `static LANE_SINGLES`: untouched (the single instances an earlier codeunit left;
//!   a session that had some moved in takes no start, `NotTaken::MovedIn`). `static CARRIED`: untouched (what the last
//!   codeunit with RequiredTestIsolation = Disabled left; a session that starts from a carry has other rows, so another
//!   key; it is read after the open, from the session's store).
//! - `crates/al-testrun/src/install.rs` `static LOGIN_RECORDS`: untouched (the records that shorten a later real login; a
//!   replay skips the login code, so it neither records nor takes a memo run).
//! - `crates/al-testrun/src/baseline_cache.rs` `static KEPT`, `static CARRIED`: run (the install baseline a thread keeps for
//!   its next run, and a carried baseline; read before any session opens a company).
//! - `crates/al-testrun/src/runtime/discovery.rs` `static FOUND`: untouched (what the discovery run found in test codeunits'
//!   `OnRun`; the login is not a test codeunit).
//! - `crates/al-testrun/src/coverage.rs` `static SINK`: gated (coverage: `NotTaken::NotesCode`).
//! - `crates/al-testrun/src/profiling/mod.rs` `static INTERNER`, `static ON`, `static STATE`, `static GENERATION`,
//!   `static NEXT`, `static THREAD_ID`, `static LEGACY`: gated (the profiler: `profiling::active()` is in the gate).
//! - `crates/al-testrun/src/host.rs` `static SERVER_URL`, `static COMPANIES`, `static COMPANY_NAME`,
//!   `static PUBLISH_ORDER_KNOWN`: key (a serve request sets them: `Key::run`, `RunSettings`, by content; the company name
//!   also reaches `Key::company`). `static FILES_DIR`, `static ENVIRONMENT`, `static ON`: run (`OnceLock`s, set once
//!   per process whatever a later request asks: `--files-dir`, `--environment`, `ALRUNNER_REAL_CLOCK`; a run with a
//!   files directory or the real clock takes no start, `host::unkeyed`, `NotTaken::Unkeyed`). The run clock is begun
//!   per run (`RunClock`, `Scratch::clock`), not kept here: `Key::clock` holds it.
//! - `crates/al-testrun/src/dotnet.rs` `static BUILD`, `static PROCESSES`, `static START_ON_THIS_THREAD`: run (the sidecar
//!   build lock, a process counter, the start-up time read after each test codeunit; an image needs the DotNet state of a
//!   new session, so a login that started a sidecar has no start).
//! - `crates/al-testrun/src/compile_cache.rs` `static COUNTER`, `static HASH`, `static PINNED_EXECUTABLE`: run (the compile
//!   cache's file counter, the hash of the runner's own build and the executable it is read from).
//! - `crates/al-testrun/src/crash.rs` `static RUNNING`, `static PREVIOUS`, `static INSTALL`, `static TOLD`: run (the crash
//!   handler's note of the codeunit that runs and its signal state).
//! - `crates/al-testrun/src/login_replay.rs` `static NEXT`: run (the numbers a backing that does not say who it is gets,
//!   one per session, so its key matches no other key).
//! - `crates/al-testrun/src/report_render.rs` `static STARTS`: run (a counter for the folder name of a render sidecar).
