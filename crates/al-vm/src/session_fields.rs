//! Every field of [`Session`], each in exactly one class (al-runner issue 8211). The table is the one place a field's
//! class is written, and each check of a class is written next to its field: [`session_classes!`] makes from it a pattern
//! that names every field without `..` (a field left out does not compile, E0027; a field in two classes does not either,
//! E0025), the check that the `refuse` fields hold a new session's value ([`Session::install_refused_field`]) and the
//! rule of a session at rest ([`Session::at_rest_why`]).
//!
//! The classes:
//! - `plain`: a value the session's own code sets, held by value; the type is written next to the name, so a wrong one
//!   does not compile.
//! - `runner`: set by the runner before the login from the run's plan (al-testrun `enter_test_codeunit`); a session
//!   image of the login keeps the target's own value.
//! - `counter`: a count read only as a difference around one call and never by AL code (the readers are cited next to
//!   each): no input or output of the login, so neither imaged nor rendered nor in the login replay key; a later
//!   difference around a call is the same.
//! - `linked`: state that holds handles or is shared between owners (the store, the transaction, the events): a copy of
//!   it is made by hand.
//! - `fresh`: caches and scratch with no AL value; a new session's value serves.
//! - `refuse`: must hold a new session's value (the predicate next to it), else a session image of the login (issue
//!   8211, `login_image.rs`) is not made ([`Session::image_refused_field`]). Install code that ran in a session of its
//!   own is held to the same predicates ([`Session::install_refused_field`], [`Session::install_left`]), except those
//!   marked `[image_only]`, which only the image requires, for the reason next to each. One marked `[image => ...]` holds
//!   install code to its predicate and the image to the stricter one in the brackets besides.
//! - `rest`: no code runs and nothing of a call is left (the predicate next to it); [`Session::at_rest_why`] names the
//!   first that does not hold.
//! - `own`: made from the engine, the host or the run's session flags before any code runs; each session keeps its own.
//!   A predicate next to one is what a session that takes over another's state requires of it.
//!
//! # Session state outside `Session`'s fields
//!
//! The table names every field, but some of what a session holds is kept elsewhere: by the store besides its rows, and
//! in thread-local or process tables (some keyed by a handle's address). Each owner is listed here with what it is:
//! **session** (state of this session that a copy of it must carry), **checked** (the check of the login replay sees it,
//! al-testrun `login_replay.rs`), **follows** (written again by a setter or a constructor from state a copy carries),
//! **run** (one value for the whole thread or process run, not of one session: an id counter, a cache of the program, a
//! diagnostic counter) or **transient** (set and taken within one call; empty at rest). Each is written `static NAME`
//! after its file. al-test-cli `tests/outside_session_state.rs` fails for a `static` of al-vm, al-builtins or al-data
//! (generated files and test code left out) that is not listed here, and for a listed one that is no longer there.
//!
//! The store, besides its rows:
//! - emptied marks (`MemStore::committed_marks`): session.
//! - company copies (`committed_copies`): follows, `restore_baseline` puts them back from `full_copies` (plain).
//! - deleted companies (`committed_dropped_backed_companies`, in `MemStore::committed_marks`): session; the image takes
//!   no login that changes them (al-data `committed_row_changes_from`), and the base shape holds them; a baseline's
//!   bytes and its frozen copy carry them (issue 9233, `MemStore::committed_deleted_companies`).
//! - held columns (`unknown_cols`, `accent_cols`, `ae_cols`): follows, `MemStore::from_exported` notes every row
//!   (test `baseline::tests::a_baseline_read_back_from_bytes_refuses_what_the_captured_one_refuses`).
//! - stamps, `moves`, `shapes`, the read caches: run (a stamp only says whether a table changed, and the facts that
//!   compare with one are carried as facts: `tx.commit_counter`, `own_read`'s "current", `baseline_origin`).
//!
//! Thread-local and process tables:
//! - `crates/al-builtins/src/errorinfo.rs` `static CALLSTACKS`: session, an ErrorInfo's call stack; checked,
//!   state_render.rs ErrorInfo line.
//! - `crates/al-builtins/src/assumptions.rs` `static HITS`: checked, the check keeps the login's marks next to its state
//!   (`LoginState::of`); a login that is not run again must raise them again.
//! - `crates/al-builtins/src/filter.rs` `static FILTER_LANGUAGE`, `crates/al-builtins/src/ops.rs` `static TURKIC_CODE`:
//!   follows, `Session::restore_carried`.
//! - `crates/al-builtins/src/value.rs` `static BORN_RESTRICTED`: follows, each instance's `born`.
//! - `crates/al-builtins/src/instance_cycles.rs` `static MADE`, `static PRUNE_AT`, `static GUARDS`, `static LIVE`:
//!   follows, an instance is made by `instance_cycles::shared`, and a thread frees its cycles at its end (`RunEnd`).
//!   `static PAGE_INSTANCES`: run (a function registered once).
//! - `crates/al-data/src/memory.rs` `static STORE_OPS`: follows, `SqlCounters::restore_left` reads it again.
//! - `crates/al-builtins/src/filter.rs` `static DATE_CONTEXT`, `crates/al-builtins/src/placeholder_output.rs`
//!   `static UNANSWERED`, `crates/al-builtins/src/stream.rs` `static OBSERVED`, `static OBSERVED_ANY`,
//!   `crates/al-builtins/src/text.rs` `static UNAUDITED`: transient (scoped by `with_date_context`; taken by the builtin
//!   call, the statement or the procedure end that set it).
//! - `crates/al-builtins/src/rendered_text.rs` `static RENDERS`, `static ACTIVE`, `static ALLOWED`,
//!   `static DOTNET_ALLOWED`: run (a marked text keeps its number in the process; a text value keeps its mark).
//! - `crates/al-builtins/src/placeholder_output.rs` `static SIZE_MADE`, `crates/al-vm/src/temp_blob_share.rs`
//!   `static STREAMS_KEPT`: run (process flags that switch a guard on, never off).
//! - `crates/al-builtins/src/permission_shadow.rs` `static ENABLED`, `static ALL`, `static LINES`: run (the shadow
//!   permission log, diagnostics).
//! - `crates/al-data/src/memory.rs` `static STAMPS`, `static NO_COMPANY`, `static ORDER_BUILDS`,
//!   `crates/al-builtins/src/text.rs` `static MARKER_SCANS`, `crates/al-vm/src/exec/mod.rs` `static ON`, `static STATS`:
//!   run (stamps and diagnostic counters).
//! - `crates/al-vm/src/lib.rs` `static NEXT_ENGINE_UID`, `crates/al-vm/src/baseline.rs` `static NEXT_ORIGIN`,
//!   `crates/al-vm/src/dotnet/blob_stream.rs` `static NEXT_VIEW`: run (ids handed out once in the process).
//! - `crates/al-vm/src/report_placeholder/files.rs` `static HEADERS`: run (the constant headers of the placeholder files,
//!   built once).
//! - `crates/al-builtins/src/catalog.rs` `static CATALOG`, `crates/al-builtins/src/collation.rs` `static ASCII_WEIGHTS`,
//!   `crates/al-builtins/src/conv/mod.rs` `static CELLS`, `crates/al-builtins/src/features.rs` `static TABLE`,
//!   `crates/al-builtins/src/native/registry.rs` `static TABLE`, `crates/al-builtins/src/page_handle.rs`
//!   `static CLOSED_PAGE`, `crates/al-builtins/src/sysenum.rs` `static LIST`, `crates/al-vm/src/auto_format_writer.rs`
//!   `static EVENTS`, `crates/al-vm/src/platform/fields.rs` `static WHY`, `static PACKAGE_WHY`,
//!   `crates/al-vm/src/platform/mod.rs` `static SHARED`, `crates/al-vm/src/refs.rs` `static C`,
//!   `crates/al-vm/src/report_save/cells.rs` `static CELLS`: run (tables of the program and the platform, the same for
//!   every session).
//!
//! Every `Session` field in one class of one table (al-runner issue 8211): `plain`, `linked`,
//! `fresh`, `refuse` (a new session's value, the predicate next to the field), `rest` (no code runs,
//! the predicate next to the field) and `own`. The table makes the pattern that names every field,
//! `Session::install_refused_field` (read by `install_left`) and `Session::at_rest_why`; its module
//! comment lists every owner of session state outside the fields (al-test-cli
//! `tests/outside_session_state.rs` keeps it complete). Entry points: `Session::at_rest_why`,
//! `Session::install_refused_field`.

use crate::Session;
use std::collections::{BTreeSet, HashMap, HashSet};

/// The table, handed whole to `$with!`.
macro_rules! session_fields {
    ($with:ident) => {
        $with! {
            plain {
                // A caught error of the login stays GetLastErrorText (state_stamp.rs `last error`).
                last_error: al_builtins::native::AlError,
                // The object that raised the last error; goes with `last_error`.
                error_raised_in: Option<usize>,
                isolated_storage: HashMap<(String, String), (String, bool, bool)>,
                isolated_committed: HashMap<(String, String), (String, bool, bool)>,
                isolated_written_in_codeunit: HashSet<(String, String)>,
                // The session's baseline bookkeeping (`restore_baseline`).
                company_built: bool,
                login_held: bool,
                // What the session's own code wrote or resolved (lib.rs).
                demo_written: HashSet<u32>,
                gl_setup_written: bool,
                setup_caption_read_early: bool,
                table_connections: HashSet<(i32, String)>,
                default_table_connection: HashMap<i32, String>,
                sequences: HashMap<(String, String), crate::sessionops::Sequence>,
                sequences_changed: HashMap<(String, String), crate::sessionops::SequenceChange>,
                // Guid and SystemId draws and their key (state_stamp.rs `counters`).
                next_guid: u64,
                guid_key: u64,
                // `Random`: the sequence itself and every count next to it (state_stamp.rs `random`).
                unseeded_base: i32,
                randomized_with_seed: bool,
                rng: al_host::random::NetRandom,
                random_draws: u64,
                reseeded_in_test: bool,
                inherited_draws: u64,
                run_seeded_in_test: bool,
                carried_draws: u64,
                unseeded_randomizes: i32,
                // Row-version counters (state_stamp.rs `counters`; `install_row_version`: `mark_install_end`).
                next_row_version: i64,
                install_row_version: i64,
                autoinc: HashMap<(String, u32), i64>,
                baseline_rows: BTreeSet<u32>,
                http_policy: Option<&'static str>,
                tdd_reached: BTreeSet<String>,
                // A record's own read is current against it (state_render.rs `own_read`).
                commits: u64,
                blob_sources: crate::blob_transfer::Sources,
                application_areas: String,
                company_unknown: HashMap<(String, u32), String>,
                full_copies: BTreeSet<String>,
                // Kept until the session's data is replaced (`crate::state::RestampedRows`).
                restamped_rows: Vec<crate::state::RestampedRows>,
                // The clock: offset, last system stamp, key and draws (state_stamp.rs `counters`).
                waited_ms: i64,
                encrypted: HashMap<String, String>,
                encryption_key: crate::EncryptionKey,
                last_stamp: i64,
                clock_key: u64,
                clock_draws: u64,
                caption_translations: Vec<(String, i32, String)>,
                ui_caption_translated: Vec<(String, i32)>,
            }
            runner {
                // Set by al-testrun `enter_test_codeunit` before the login, from the run's plan: the number sequences'
                // refusal mark (al-runner issue 8152) and the codeunit's isolation settings.
                numbers_unmeasured,
                in_test_codeunit,
                isolation_from_hub,
                hub_disabled,
                // The login replay key holds it in a field of its own (al-testrun `Key::test_codeunit`), not in the
                // state lines, so the check can compare the logins of two test codeunits.
                test_object [own_key_field],
            }
            counter {
                // Read only as a difference around one call, never by AL code: `page_run_close.rs`,
                // `pages/rows.rs`, `pages/document_handler_error.rs` (written by `caught_errors.rs`).
                caught_errors,
                // `records.rs` (`SqlCounters::enter`), `isdirty.rs`, `globaltriggers.rs` (written by `lib.rs`). Real
                // logins of one key differ in it: the WorkDate step's recorded run (al-testrun `install::LoginMemo`)
                // answers without the call.
                calls_made,
                // `pages.rs` (`write_mark`, compared before and after: `page_run_close.rs`, `pages/next_held_row_read.rs`,
                // `pages/part_insert_swallowed.rs`, `pages/list_factbox_new_line.rs`, `pages/code_inserted_new_line.rs`).
                deletes,
                // `pages/empty_part_rec.rs` (`landing_store_state`).
                rollbacks,
            }
            linked {
                singletons,
                store,
                temps,
                sql,
                baseline_origin,
                media,
                media_removed,
                work_date,
                global_language,
                tx,
                ui_state,
                events,
                // The live Temp Blob records; install code is held to `as_new` (`install_left`).
                temp_blob_impl_fields,
            }
            fresh {
                spare_vecs,
                ret_used,
                exited_with_value,
                constants,
                initial_cache,
                adhoc_tables,
                resource_reads,
                // Counts and insert stamps al-testrun compares within one test only (`sequence_start.rs`; install_left.rs
                // keeps none of them either): a test's differences of the counts are the same from any start, and the
                // insert stamps are asked only for sequences inserted since the test's start, so none from the login. The
                // number sequence steps of the open transaction (`sequence_start/tx_changes.rs`) are that session's
                // transaction's.
                access_notes,
            }
            refuse {
                // `--explain`: the fault sites of the login code are that session's. Set from the caller before install
                // code runs.
                fault_trace [image_only] => |s: &Session| !s.fault_trace,
                isolated_set_in_test => |s: &Session| s.isolated_set_in_test.is_empty(),
                // Nothing is noted before a test codeunit's first test (`begin_codeunit_row_locks`).
                row_locks => |s: &Session| s.row_locks.as_new(),
                unmeasured => |s: &Session| s.unmeasured.as_new(),
                // A task holds code to run.
                tasks => |s: &Session| s.tasks.as_new(),
                sessions_started => |s: &Session| s.sessions_started == 0,
                crm_stores => |s: &Session| s.crm_stores.is_empty(),
                // Set by al-testrun after install code ran (`start_singles`), not held of it.
                si_track [image_only] => |s: &Session| s.si_track.is_none(),
                singles_unclaimed => |s: &Session| s.singles_unclaimed.is_empty(),
                seed_refusal => |s: &Session| s.seed_refusal.is_none(),
                tx_model => |s: &Session| s.tx_model == crate::TxModel::default(),
                iso_min => |s: &Session| s.iso_min.as_new(),
                runner_permission_step => |s: &Session| !s.runner_permission_step,
                runner_host => |s: &Session| s.runner_host.as_new(),
                permission_roles => |s: &Session| s.permission_roles.is_empty(),
                permissions => |s: &Session| s.permissions.is_none(),
                runner_permission_sets => |s: &Session| !s.runner_permission_sets,
                shadow_roles => |s: &Session| s.shadow_roles.is_empty(),
                shadow_active => |s: &Session| !s.shadow_active,
                shadow_lower_bound => |s: &Session| !s.shadow_lower_bound,
                report_refusal_after_denial => |s: &Session| s.report_refusal_after_denial.is_none(),
                scratch_messages => |s: &Session| s.scratch_messages.is_none(),
                // Install code leaves no table a group 1 filter (main's rule for it). The image carries none of the group 1
                // state, so it requires all of it as new: a taken-back entry and the worker's notes change later calls too.
                global_group1 [image => |s: &Session| s.global_group1.as_new()]
                    => |s: &Session| s.global_group1.is_empty(),
                capture => |s: &Session| s.capture.is_none(),
                form => |s: &Session| s.form == crate::Form::Plain,
                debug => |s: &Session| s.debug.is_none(),
                error_scopes => |s: &Session| s.error_scopes.as_new(),
                // Install code is held to `as_new` (`install_left`); a list of gone handles answers as an empty one.
                variant_refs [image_only] => |s: &Session| s.variant_refs.holds_no_live_handle(),
                // The .NET host process is the session's own.
                dotnet => |s: &Session| s.dotnet.as_new(),
                sleep_budget => |s: &Session| s.sleep_budget.is_none(),
                slept_in_test => |s: &Session| s.slept_in_test == 0,
                // Files and placeholders live outside the session.
                temp_files => |s: &Session| s.temp_files == 0,
                render_pending_files => |s: &Session| s.render_pending_files.is_empty(),
                placeholder_files => |s: &Session| s.placeholder_files.as_new(),
                // What the run saw of a RecordRef over the database Field table lives with its buffer. Not held of
                // install code, whose RecordRefs end with it.
                relation_field_no_refs [image_only] => |s: &Session| s.relation_field_no_refs.as_new(),
                // Set only by a test page call (`pages/part_update_left_view.rs`), which install code never makes.
                test_filter_steps [image_only] => |s: &Session| s.test_filter_steps.is_none(),
                pages => |s: &Session| crate::install_left::clients::pages_as_new(&s.pages),
                coverage => |s: &Session| crate::install_left::clients::coverage_as_new(&s.coverage),
                report => |s: &Session| crate::install_left::clients::report_as_new(&s.report),
            }
            rest {
                depth => |s: &Session| s.depth == 0,
                xp_pass_views => |s: &Session| s.xp_pass_views.is_empty(),
                frames => |s: &Session| s.frames.is_empty(),
                // Set and cleared around one interface call (`run_iface`): false whenever no call runs.
                next_interface_call => |s: &Session| !s.next_interface_call,
                xp_unlinked_passes => |s: &Session| s.xp_unlinked_passes.is_empty(),
                xp_link_hidden => |s: &Session| s.xp_link_hidden.is_empty(),
                call_stack => |s: &Session| s.call_stack.is_empty(),
                runner_frames => |s: &Session| s.runner_frames.is_empty(),
                fault_site => |s: &Session| s.fault_site.is_empty(),
                fault_site_lost => |s: &Session| !s.fault_site_lost,
                install_scope => |s: &Session| !s.install_scope,
                company_build => |s: &Session| !s.company_build,
                background_session => |s: &Session| !s.background_session,
                install_without_client => |s: &Session| !s.install_without_client,
                dirty_ops_open => |s: &Session| s.dirty_ops_open == 0,
                dirty_frames => |s: &Session| s.dirty_frames.is_empty(),
                live_xrecs => |s: &Session| s.live_xrecs.iter().all(|w| w.strong_count() == 0),
                session_id_override => |s: &Session| s.session_id_override.is_none(),
                truncate_undo => |s: &Session| s.truncate_undo.is_empty(),
                validating => |s: &Session| s.validating.is_empty(),
                stamp_validating => |s: &Session| s.stamp_validating.is_empty(),
                validate_events => |s: &Session| s.validate_events.is_empty(),
                validate_site => |s: &Session| s.validate_site.is_none(),
                validate_sites => |s: &Session| s.validate_sites.is_empty(),
                plain_proc_xrec => |s: &Session| s.plain_proc_xrec.is_empty(),
                ext_xrec_triggers => |s: &Session| s.ext_xrec_triggers == 0,
                ext_before_watch => |s: &Session| s.ext_before_watch.borrow().is_empty(),
                shared_proc_xrec => |s: &Session| s.shared_proc_xrec.is_empty(),
                iface_frames => |s: &Session| s.iface_frames.is_empty(),
                field_error_form => |s: &Session| s.field_error_form.is_none(),
                error_finals => |s: &Session| s.error_finals.is_none(),
                var_alias_watch => |s: &Session| s.var_alias_watch.is_empty(),
                permission_from_al => |s: &Session| !s.permission_from_al,
                permission_from_recordref => |s: &Session| !s.permission_from_recordref,
                record_calls => |s: &Session| s.record_calls.is_empty(),
                setfilter_from_al => |s: &Session| !s.setfilter_from_al,
                permission_autocalc => |s: &Session| !s.permission_autocalc,
                permission_page_calc => |s: &Session| !s.permission_page_calc,
                find_first_read => |s: &Session| !s.find_first_read,
                al_record_call => |s: &Session| s.al_record_call.is_none(),
                permission_opening_source => |s: &Session| s.permission_opening_source.is_none(),
                permission_caller => |s: &Session| s.permission_caller.is_none(),
                permission_run_object => |s: &Session| s.permission_run_object.is_none(),
                query_receiver => |s: &Session| s.query_receiver.is_none(),
                keyref_receiver_local => |s: &Session| !s.keyref_receiver_local,
                bulk_modify => |s: &Session| !s.bulk_modify,
                set_value_from_variant => |s: &Session| !s.set_value_from_variant,
                host_new_running => |s: &Session| s.host_new_running.is_none(),
                part_open_running => |s: &Session| !s.part_open_running,
                host_move_running => |s: &Session| s.host_move_running.is_none(),
                part_sync_keeps_same_link => |s: &Session| s.part_sync_keeps_same_link.is_none(),
                run_trigger_unknown => |s: &Session| s.run_trigger_unknown.is_none(),
                package_load => |s: &Session| !s.package_load,
                get_load => |s: &Session| !s.get_load,
                sysid_load => |s: &Session| !s.sysid_load,
                find_load => |s: &Session| !s.find_load,
                find_through_recordref => |s: &Session| !s.find_through_recordref,
                through_recordref => |s: &Session| !s.through_recordref,
            }
            own {
                // The same engine: al-testrun keys what it keeps by `Engine::uid`.
                engine,
                // Set by the run's session flags before any code runs (al-testrun `SessionFlags::apply`).
                no_client,
                // A temporary table's id is never observable: each session takes new ids from its own counter.
                next_temp,
                temp_release,
                // Made from the engine's test data.
                restored_row_version,
                // Made from the host's identity: code may not change the company or the user.
                cx => |s: &Session| {
                    *s.cx.company == *s.host.identity.company_name() && *s.cx.user == *s.host.identity.user_id()
                },
                ignore_permission_mock,
                permissions_off,
                // The session's own stop flag, set from another thread.
                stop,
                // Clock, identity, seeds and files, made from the run's settings; the files are the session's own.
                host => |s: &Session| !s.host.files.changed(),
                // Made from the host's identity and the engine's test data.
                companies,
                // Made from the shared test-data provider, the same in every session of the run.
                backed_companies,
            }
        }
    };
}

/// The table, for a module that makes more of it (`login_image/fields.rs` makes the image's copy of the `plain` fields).
pub(crate) use session_fields;

/// Makes, from the table: the pattern that names every field, the type check of the `plain` fields,
/// [`Session::image_refused_field`], [`Session::install_refused_field`] and [`Session::at_rest_why`].
macro_rules! session_classes {
    (
        plain { $($p:ident: $pt:ty,)* }
        runner { $($n:ident $([$nk:ident])?,)* }
        counter { $($c:ident,)* }
        linked { $($l:ident,)* }
        fresh { $($f:ident,)* }
        refuse { $($r:ident $([$tag:ident $(=> $ip:expr)?])? => $rp:expr,)* }
        rest { $($s:ident => $sp:expr,)* }
        own { $($o:ident $(=> $op:expr)?,)* }
    ) => {
        // Every field in exactly one class, and each `plain` field of the type written next to it.
        const _: fn(&Session) = |session| {
            let Session { $($p: _,)* $($n: _,)* $($c: _,)* $($l: _,)* $($f: _,)* $($r: _,)* $($s: _,)* $($o: _,)* } =
                session;
            $(let _: &$pt = &session.$p;)*
        };

        impl Session {
            /// The first field of the `refuse` class, or of the `own` class with a requirement, that does not hold what
            /// the session image of the login requires (`login_image.rs`); `None` when each does. Each predicate of the
            /// class, those marked `[image_only]` and the stricter ones of `[image => ...]` included.
            pub(crate) fn image_refused_field(&self) -> Option<&'static str> {
                $(image_check!(self, $r $([$tag $(=> $ip)?])? => $rp);)*
                $($(
                    let holds: fn(&Session) -> bool = $op;
                    if !holds(self) {
                        return Some(stringify!($o));
                    }
                )?)*
                None
            }

            /// The first field of the `refuse` class (those marked `[image_only]` left out), or of the `own` class with
            /// a requirement, that does not hold what a new session holds; `None` when each does. What install code
            /// that ran in a session of its own must leave as a new session holds it (`Session::install_left`).
            pub(crate) fn install_refused_field(&self) -> Option<&'static str> {
                $(install_check!(self, $r $([$tag $(=> $ip)?])? => $rp);)*
                $($(
                    let holds: fn(&Session) -> bool = $op;
                    if !holds(self) {
                        return Some(stringify!($o));
                    }
                )?)*
                None
            }

            /// The first field of the `rest` class that does not hold a session at rest, by its name; `None` at rest:
            /// no code runs and nothing of a call is left.
            pub(crate) fn at_rest_why(&self) -> Option<&'static str> {
                $(
                    let holds: fn(&Session) -> bool = $sp;
                    if !holds(self) {
                        return Some(stringify!($s));
                    }
                )*
                None
            }
        }
    };
}

/// One `refuse` predicate in [`Session::install_refused_field`]: a predicate marked `[image_only]`, and the image's one
/// of `[image => ...]`, is only type checked.
macro_rules! install_check {
    ($s:expr, $r:ident [image_only] => $rp:expr) => {
        let _: fn(&Session) -> bool = $rp;
    };
    ($s:expr, $r:ident [image => $ip:expr] => $rp:expr) => {
        let _: fn(&Session) -> bool = $ip;
        install_check!($s, $r => $rp);
    };
    ($s:expr, $r:ident => $rp:expr) => {
        let holds: fn(&Session) -> bool = $rp;
        if !holds($s) {
            return Some(stringify!($r));
        }
    };
}

/// One `refuse` entry in [`Session::image_refused_field`]: its predicate, and the stricter one of `[image => ...]` after
/// it.
macro_rules! image_check {
    ($s:expr, $r:ident [image_only] => $rp:expr) => {
        image_check!($s, $r => $rp);
    };
    ($s:expr, $r:ident [image => $ip:expr] => $rp:expr) => {
        image_check!($s, $r => $rp);
        image_check!($s, $r => $ip);
    };
    ($s:expr, $r:ident => $rp:expr) => {
        let holds: fn(&Session) -> bool = $rp;
        if !holds($s) {
            return Some(stringify!($r));
        }
    };
}

session_fields!(session_classes);

#[cfg(test)]
mod tests;
