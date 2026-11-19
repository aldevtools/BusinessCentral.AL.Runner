//! Every field of [`Session`], each in exactly one class (PERF-01 step 2 design, section 1). The table is the one place a
//! field's class is written: [`image_fields!`] makes from it a pattern that names every field without `..` (a field left
//! out does not compile, E0027; a field in two classes does not either, E0025), the plain fields' image, and the check
//! that the refused fields hold a new session's value.
//!
//! The classes:
//! - `plain`: imaged by value ([`PlainFields`]); the type is written next to the name, so a wrong one does not compile.
//! - `linked`: imaged by hand, in `session_parts.rs` and the arena.
//! - `fresh`: caches and scratch with no AL value; the thawn session keeps its new value.
//! - `refuse`: must hold a new session's value (the predicate next to it), else the session has no image. Install code
//!   that ran in a session of its own is held to the same predicates (`Session::install_left`), except those marked
//!   `[image_only]`.
//! - `rest`: must be at rest ([`Session::at_rest`], the one owner of that rule), else no image.
//! - `own`: made from the engine, the host or the run's session flags before any code runs; the thawn session keeps its
//!   own. A predicate next to one is what the image requires of it.

use crate::Session;
use std::collections::{BTreeSet, HashMap, HashSet};

/// The table, handed whole to `$with!`.
macro_rules! session_fields {
    ($with:ident) => {
        $with! {
            plain {
                // A caught error of the login stays GetLastErrorText (state_stamp.rs `last error`).
                last_error: al_builtins::native::AlError,
                // Counts read as a difference around one call (install_left.rs): by value, so a later difference is the same.
                caught_errors: u64,
                // The object that raised the last error; goes with `last_error`.
                error_raised_in: Option<usize>,
                isolated_storage: HashMap<(String, String), (String, bool, bool)>,
                isolated_committed: HashMap<(String, String), (String, bool, bool)>,
                isolated_written_in_codeunit: HashSet<(String, String)>,
                // The session's baseline bookkeeping (`restore_baseline`): by value, so the thaw needs no baseline of its own.
                company_built: bool,
                // What the session's own code wrote or resolved (lib.rs): the login is such code.
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
                calls_made: u64,
                reseeded_in_test: bool,
                inherited_draws: u64,
                run_seeded_in_test: bool,
                carried_draws: u64,
                unseeded_randomizes: i32,
                // Row-version counters (state_stamp.rs `counters`; `install_row_version`: `mark_install_end`).
                next_row_version: i64,
                deletes: u64,
                rollbacks: u64,
                install_row_version: i64,
                autoinc: HashMap<(String, u32), i64>,
                baseline_rows: BTreeSet<u32>,
                // Set by al-testrun `enter_test_codeunit` before the login.
                in_test_codeunit: bool,
                isolation_from_hub: bool,
                hub_disabled: bool,
                test_object: Option<usize>,
                http_policy: Option<&'static str>,
                tdd_reached: BTreeSet<String>,
                // A record's own read is current against it (state_render.rs `own_read`).
                commits: u64,
                blob_sources: crate::blob_transfer::Sources,
                application_areas: String,
                company_unknown: HashMap<(String, u32), String>,
                full_copies: BTreeSet<String>,
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
            linked {
                numbers_unmeasured,
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
                // keeps none of them either): a test's differences are the same from any start.
                access_notes,
            }
            refuse {
                // `--explain`: the fault sites of the login code are that session's.
                fault_trace [image_only] => |s: &Session| !s.fault_trace,
                isolated_set_in_test => |s: &Session| s.isolated_set_in_test.is_empty(),
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
                // Install code may leave an entry another record took back (`install_left` asks `is_empty`).
                global_group1 [image_only] => |s: &Session| s.global_group1.as_new(),
                capture => |s: &Session| s.capture.is_none(),
                form => |s: &Session| s.form == crate::Form::Plain,
                debug => |s: &Session| s.debug.is_none(),
                error_scopes => |s: &Session| s.error_scopes.as_new(),
                // Install code is held to `as_new` (`install_left`); a list of gone handles answers as an empty one.
                variant_refs [image_only] => |s: &Session| s.variant_refs.holds_no_live_handle(),
                // The .NET host process is not in the image.
                dotnet => |s: &Session| s.dotnet.as_new(),
                sleep_budget => |s: &Session| s.sleep_budget.is_none(),
                slept_in_test => |s: &Session| s.slept_in_test == 0,
                // Files and placeholders live outside the image.
                temp_files => |s: &Session| s.temp_files == 0,
                render_pending_files => |s: &Session| s.render_pending_files.is_empty(),
                placeholder_files => |s: &Session| s.placeholder_files.as_new(),
                // Install code is held to `as_new` (`install_left`); a list of gone records answers as an empty one.
                temp_blob_impl_fields [image_only] => |s: &Session| s.temp_blob_impl_fields.holds_no_live_record(),
                // What the run saw of a RecordRef over the database Field table lives with its buffer. Not held of
                // install code: main's install_left does not ask it.
                relation_field_no_refs [image_only] => |s: &Session| s.relation_field_no_refs.as_new(),
                pages => |s: &Session| crate::install_left::clients::pages_as_new(&s.pages),
                coverage => |s: &Session| crate::install_left::clients::coverage_as_new(&s.coverage),
                report => |s: &Session| crate::install_left::clients::report_as_new(&s.report),
            }
            rest {
                depth,
                xp_pass_views,
                frames,
                next_interface_call,
                xp_unlinked_passes,
                xp_link_hidden,
                call_stack,
                runner_frames,
                fault_site,
                fault_site_lost,
                install_scope,
                company_build,
                background_session,
                install_without_client,
                dirty_ops_open,
                dirty_frames,
                live_xrecs,
                session_id_override,
                truncate_undo,
                validating,
                stamp_validating,
                validate_site,
                validate_sites,
                plain_proc_xrec,
                field_error_form,
                error_finals,
                var_alias_watch,
                permission_from_al,
                permission_from_recordref,
                record_calls,
                setfilter_from_al,
                permission_autocalc,
                permission_page_calc,
                find_first_read,
                al_record_call,
                permission_opening_source,
                permission_caller,
                permission_run_object,
                query_receiver,
                keyref_receiver_local,
                bulk_modify,
                set_value_from_variant,
                host_new_running,
                part_open_running,
                host_move_running,
                part_sync_keeps_same_link,
                run_trigger_unknown,
                package_load,
                get_load,
                sysid_load,
                find_load,
                find_through_recordref,
                through_recordref,
            }
            own {
                // The image is taken and thawn on one engine.
                engine,
                // Set by the run's session flags before any code runs (al-testrun `SessionFlags::apply`); in the key.
                no_client,
                // The thaw takes new ids from the thawn session's counter: a temporary table's id is never observable.
                next_temp,
                temp_release,
                // Made from the engine's test data.
                restored_row_version,
                // Made from the host's identity: the login may not change the company or the user.
                cx => |s: &Session| {
                    *s.cx.company == *s.host.identity.company_name() && *s.cx.user == *s.host.identity.user_id()
                },
                ignore_permission_mock,
                permissions_off,
                // The session's own stop flag, set from another thread.
                stop,
                // Clock, identity, seeds and files, made from the run's settings; the files are not in the image.
                host => |s: &Session| !s.host.files.changed(),
                // Made from the host's identity and the engine's test data.
                companies,
                // Made from the shared test-data provider, the same in every session of the run.
                backed_companies,
            }
        }
    };
}

/// Makes, from the table: [`PlainFields`] with `take` and `give`, and [`Session::refused_field`] with the pattern that
/// names every field.
macro_rules! image_fields {
    (
        plain { $($p:ident: $pt:ty,)* }
        linked { $($l:ident,)* }
        fresh { $($f:ident,)* }
        refuse { $($r:ident $([$only:ident])? => $rp:expr,)* }
        rest { $($s:ident,)* }
        own { $($o:ident $(=> $op:expr)?,)* }
    ) => {
        /// The `plain` fields of a session, by value.
        #[derive(Clone)]
        pub(crate) struct PlainFields {
            $(pub(crate) $p: $pt,)*
        }

        impl PlainFields {
            /// The plain fields of `s`.
            pub(crate) fn take(s: &Session) -> PlainFields {
                PlainFields { $($p: s.$p.clone(),)* }
            }

            /// Writes the plain fields into `s`.
            pub(crate) fn give(&self, s: &mut Session) {
                $(s.$p = self.$p.clone();)*
            }
        }

        impl Session {
            /// The first field of the `refuse` class, or of the `own` class with a requirement, that does not hold what
            /// the image requires; `None` when each does. The `rest` class is [`Session::at_rest`].
            pub(crate) fn refused_field(&self) -> Option<&'static str> {
                let Session { $($p: _,)* $($l: _,)* $($f: _,)* $($r: _,)* $($s: _,)* $($o: _,)* } = self;
                $(
                    let holds: fn(&Session) -> bool = $rp;
                    if !holds(self) {
                        return Some(stringify!($r));
                    }
                )*
                $($(
                    let holds: fn(&Session) -> bool = $op;
                    if !holds(self) {
                        return Some(stringify!($o));
                    }
                )?)*
                None
            }

            /// [`Session::refused_field`] without the predicates marked `[image_only]`: what install code that ran in a
            /// session of its own must leave as a new session holds it, too (`Session::install_left`).
            pub(crate) fn install_refused_field(&self) -> Option<&'static str> {
                $(install_check!(self, $r $([$only])? => $rp);)*
                $($(
                    let holds: fn(&Session) -> bool = $op;
                    if !holds(self) {
                        return Some(stringify!($o));
                    }
                )?)*
                None
            }
        }
    };
}

/// One `refuse` predicate in [`Session::install_refused_field`]: none for a predicate marked `[image_only]`.
macro_rules! install_check {
    ($s:expr, $r:ident [image_only] => $rp:expr) => {};
    ($s:expr, $r:ident => $rp:expr) => {
        let holds: fn(&Session) -> bool = $rp;
        if !holds($s) {
            return Some(stringify!($r));
        }
    };
}

session_fields!(image_fields);

// `LoginImage` holds the plain fields and is `Send + Sync`: a field that holds an `Rc` cannot be put in this class.
const _: fn() = || {
    fn both<T: Send + Sync>() {}
    both::<PlainFields>();
};
