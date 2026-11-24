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
//!   start (`login_start::why_not`, `NotTaken::NotesCode`).
//! - **key**: a serve request sets it and the login can read it: a field of the login key holds it by content, so a start
//!   is never taken under another value (the field is named).
//! - **run**: one value for the whole process or thread run, set once before any session, not of one session or request
//!   (a cache of the program, an id counter, a diagnostic, a setting of the process start).
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
//!   also reaches `Key::company`). `static RUN_START`, `static FILES_DIR`, `static ENVIRONMENT`, `static ON`: run (`OnceLock`s,
//!   set once per process whatever a later request asks: the run clock's start, `--files-dir`, `--environment`,
//!   `ALRUNNER_REAL_CLOCK`; a run with a files directory or the real clock takes no start, `NotTaken::RealFiles`,
//!   `NotTaken::RealClock`).
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
