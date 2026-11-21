//! What install code that ran in a session of its own left besides its data (al-testrun `install_once.rs`, al-runner
//! issue 8144). A test codeunit's session that starts from it must start exactly as the session in which the install code
//! ran would go on: the data ([`crate::Baseline`]) and [`InstallLeft`] are carried, and every other field of [`Session`]
//! must still hold what a new session holds ([`Session::install_left`] checks each, by name).

use crate::Session;

mod clients;

/// The state besides the data that a session starting from the install code's session takes over
/// ([`Session::install_left`], [`Session::restore_install_left`]).
#[derive(Clone)]
pub struct InstallLeft {
    /// GlobalLanguage and WorkDate.
    carried: crate::CarriedState,
    /// How far the install code moved the session's clock (`Sleep`, clock readings, stamps: `waited_ms`), and how many
    /// stamp draws it made (`clock_draws`).
    clock_ms: i64,
    clock_draws: u64,
    /// The isolated storage keys it set (`Session::isolated_written_in_codeunit`, isolated_storage_reads.rs).
    isolated_written: std::collections::HashSet<(String, String)>,
    /// Its last Commit made writes durable (`TxState::last_commit_wrote`, min_row_version.rs).
    last_commit_wrote: bool,
    /// `Database.CurrentTransactionType` after its last Commit (`TxState::transaction_type`; an AL `Commit` sets it).
    transaction_type: i32,
    /// What the SQL counters it never read noted (`SqlCounters::left_inactive`).
    sql: crate::sqlcount::SqlLeft,
    /// `EventState::caption_context_revision`: moved by a GlobalLanguage change and a binding; compared only with the value a
    /// page noted when it opened, so it is carried as it is.
    caption_context_revision: u64,
    /// How many events each subscriber handled (`EventStats::calls`, counted at dispatch.rs:322). Its one reader is the
    /// Event Subscription row's "Number of Calls" and the active-instances column next to it (platform/mod.rs:358
    /// `with_event_stats`); nothing clears it, so a session in which the install code ran reads these counts.
    subscriber_calls: std::collections::HashMap<(usize, usize), i64>,
}

impl Session {
    /// What the install code left besides its data, when every other field of the session holds what a new session
    /// holds; `None` otherwise (the caller then runs the install code in each test codeunit's own session). Called after
    /// the install code ran and before [`Session::capture_baseline`], which commits and clears what it captures.
    ///
    /// Every field is named: a new field of `Session` does not compile here until it is decided. Each is one of
    /// - carried: in the `Baseline` the caller captures, or in [`InstallLeft`] (the comment says which);
    /// - not written by install code, or the same in every session of the run (the comment gives the reason);
    /// - checked, a field without a comment: [`Session::at_rest`] (what runs now) or [`Session::holds_as_new`] (what a
    ///   test reads, and the owners' sub-states) require a new session's value.
    pub fn install_left(&self) -> Option<InstallLeft> {
        let Session {
            // Same engine: the caller keys what it keeps by `Engine::uid`.
            engine: _,
            last_error: _,
            // Counts read only as a difference around one call (pages/modal_handler_error.rs).
            caught_errors: _,
            depth: _,
            // Spare allocations, no AL value.
            spare_vecs: _,
            xp_pass_views: _,
            frames: _,
            next_interface_call: _,
            xp_unlinked_passes: _,
            xp_link_hidden: _,
            call_stack: _,
            error_raised_in: _,
            runner_frames: _,
            // Diagnostics of the run (`--explain`), set from the caller before any code runs.
            fault_trace: _,
            fault_site: _,
            fault_site_lost: _,
            // Carried: the Baseline (storage), committed = storage after the last Commit (checked below).
            isolated_storage: _,
            isolated_committed: _,
            isolated_set_in_test: _,
            // Carried in InstallLeft.
            isolated_written_in_codeunit: _,
            unmeasured: _,
            tasks: _,
            install_scope: _,
            company_build: _,
            // Set from the run's session flags before any code runs, the same in every session of the run.
            no_client: _,
            background_session: _,
            sessions_started: _,
            install_without_client: _,
            // Carried: `Baseline::company_built`.
            company_built: _,
            demo_written: _,
            gl_setup_written: _,
            setup_caption_read_early: _,
            table_connections: _,
            default_table_connection: _,
            crm_stores: _,
            // Carried: the Baseline.
            sequences: _,
            sequences_changed: _,
            // Counts and insert stamps al-testrun compares within one test only (`sequence_start.rs`).
            access_notes: _,
            // Set by the caller after the install code, from the run's order (al-testrun `order_hint.rs`).
            numbers_unmeasured: _,
            singletons: _,
            // Set by the caller after the install code, from the run's lane (`single_lane.rs`).
            si_track: _,
            singles_unclaimed: _,
            // Carried: the Baseline.
            store: _,
            // Tables of temporary records: no AL value of the install code outlives it (its instances end, and the
            // single instances are reset, al-testrun install.rs `run_apps_under_test`), so none is reachable.
            temps: _,
            next_temp: _,
            temp_release: _,
            // Carried: the Baseline.
            next_guid: _,
            guid_key: _,
            unseeded_base: _,
            randomized_with_seed: _,
            // The Random stream: moved only by a draw or a reseed, which `random_draws`, `randomized_with_seed` and
            // `unseeded_randomizes` count.
            rng: _,
            random_draws: _,
            // Counts read only as a difference around one call (isdirty.rs, globaltriggers.rs, records.rs).
            calls_made: _,
            sql: _,
            dirty_ops_open: _,
            dirty_frames: _,
            live_xrecs: _,
            reseeded_in_test: _,
            inherited_draws: _,
            run_seeded_in_test: _,
            carried_draws: _,
            unseeded_randomizes: _,
            // Carried: the Baseline.
            next_row_version: _,
            // Counts read only as a difference around one call (pages.rs, pages/empty_part_rec.rs).
            deletes: _,
            rollbacks: _,
            install_row_version: _,
            // Made from the engine's test data, the same in every session of the run.
            restored_row_version: _,
            session_id_override: _,
            // Carried: the Baseline.
            autoinc: _,
            truncate_undo: _,
            validating: _,
            stamp_validating: _,
            validate_events: _,
            validate_site: _,
            validate_sites: _,
            plain_proc_xrec: _,
            shared_proc_xrec: _,
            cx: _,
            // Carried: `Baseline::install_rows` (the session has no baseline, so it is empty).
            baseline_rows: _,
            baseline_origin: _,
            seed_refusal: _,
            // Set before each builtin call and each routine return that reads it.
            ret_used: _,
            exited_with_value: _,
            field_error_form: _,
            error_finals: _,
            var_alias_watch: _,
            tx_model: _,
            in_test_codeunit: _,
            isolation_from_hub: _,
            hub_disabled: _,
            iso_min: _,
            runner_permission_step: _,
            runner_host: _,
            // Set from the run's session flags before any code runs.
            ignore_permission_mock: _,
            permissions_off: _,
            permission_roles: _,
            permissions: _,
            runner_permission_sets: _,
            shadow_roles: _,
            shadow_active: _,
            shadow_lower_bound: _,
            permission_from_al: _,
            permission_from_recordref: _,
            record_calls: _,
            setfilter_from_al: _,
            permission_autocalc: _,
            permission_page_calc: _,
            find_first_read: _,
            al_record_call: _,
            permission_opening_source: _,
            report_refusal_after_denial: _,
            permission_caller: _,
            permission_run_object: _,
            query_receiver: _,
            keyref_receiver_local: _,
            test_object: _,
            http_policy: _,
            scratch_messages: _,
            tdd_reached: _,
            global_group1: _,
            // Set by the caller after the install code (al-testrun run.rs: capture, debugger).
            capture: _,
            form: _,
            debug: _,
            // Constants of the program built on first use: the same values in every session.
            constants: _,
            // The stop flag of the session's own time limit, set from another thread.
            stop: _,
            // Compared only with a count noted earlier in the same session, by code that runs then: a record's row origin
            // (stale_buffer_origin.rs:55, assigned_audit.rs:327), a blob (blob_transfer.rs:977), a query (query_reopen.rs:43),
            // a page run (pages/page_run_close.rs:631), the SQL counters (sqlcount.rs:886, reset by `left_inactive`).
            commits: _,
            // Checked (`holds_as_new`).
            blob_sources: _,
            error_scopes: _,
            // Caches of what the program defines: the same values in every session.
            initial_cache: _,
            adhoc_tables: _,
            media: _,
            media_removed: _,
            bulk_modify: _,
            variant_refs: _,
            set_value_from_variant: _,
            host_new_running: _,
            part_open_running: _,
            host_move_running: _,
            part_sync_keeps_same_link: _,
            dotnet: _,
            application_areas: _,
            // Its files are checked below (`FileSystem::changed`); its clock, identity and seeds are made from the same
            // scratch settings in every session of the run (al-testrun `install_once.rs` `Key`).
            host: _,
            // Made from the host's identity, the same in every session of the run.
            companies: _,
            // Made from the shared test-data provider, the same in every session of the run.
            backed_companies: _,
            // Carried: the Baseline.
            company_unknown: _,
            full_copies: _,
            run_trigger_unknown: _,
            package_load: _,
            // Carried in InstallLeft.
            waited_ms: _,
            // Read by the caller (`Session::resource_reads`).
            resource_reads: _,
            // Checked (`holds_as_new`).
            sleep_budget: _,
            slept_in_test: _,
            encrypted: _,
            encryption_key: _,
            temp_files: _,
            render_pending_files: _,
            placeholder_files: _,
            temp_blob_impl_fields: _,
            // Carried: the Baseline.
            last_stamp: _,
            clock_key: _,
            // Carried in InstallLeft.
            clock_draws: _,
            work_date: _,
            global_language: _,
            caption_translations: _,
            ui_caption_translated: _,
            get_load: _,
            sysid_load: _,
            find_load: _,
            find_through_recordref: _,
            through_recordref: _,
            relation_field_no_refs: _,
            tx: _,
            ui_state: _,
            pages: _,
            events: _,
            coverage: _,
            report: _,
        } = self;
        if !(self.at_rest() && self.holds_as_new()) {
            return None;
        }
        Some(InstallLeft {
            carried: crate::CarriedState {
                language: self.global_language(),
                work_date: self.work_date,
            },
            clock_ms: self.waited_ms,
            clock_draws: self.clock_draws,
            isolated_written: self.isolated_written_in_codeunit.clone(),
            last_commit_wrote: self.tx.last_commit_wrote,
            transaction_type: self.tx.transaction_type,
            sql: self.sql.left_inactive()?,
            caption_context_revision: self.events.caption_context_revision,
            subscriber_calls: self.events.event_stats.calls.borrow().clone(),
        })
    }

    /// No code runs and nothing of a call is left: the fields that hold what runs now.
    fn at_rest(&self) -> bool {
        self.depth == 0
            && self.frames.is_empty()
            && self.call_stack.is_empty()
            && self.xp_pass_views.is_empty()
            && self.xp_unlinked_passes.is_empty()
            && self.xp_link_hidden.is_empty()
            && self.error_raised_in.is_none()
            && self.runner_frames.is_empty()
            && self.fault_site.is_empty()
            && !self.fault_site_lost
            && self.dirty_ops_open == 0
            && self.dirty_frames.is_empty()
            && self.live_xrecs.iter().all(|w| w.strong_count() == 0)
            && self.truncate_undo.is_empty()
            && self.validating.is_empty()
            && self.stamp_validating.is_empty()
            && self.validate_events.is_empty()
            && self.validate_site.is_none()
            && self.validate_sites.is_empty()
            && self.plain_proc_xrec.is_empty()
            && self.shared_proc_xrec.is_empty()
            && self.field_error_form.is_none()
            && self.error_finals.is_none()
            && self.var_alias_watch.is_empty()
            && !self.install_scope
            && !self.company_build
            && !self.background_session
            && !self.install_without_client
            && self.session_id_override.is_none()
            && self.run_trigger_unknown.is_none()
            && !self.package_load
            && !self.bulk_modify
            && !self.set_value_from_variant
            && self.host_new_running.is_none()
            && !self.part_open_running
            && self.host_move_running.is_none()
            && self.part_sync_keeps_same_link.is_none()
            && !self.get_load
            && !self.sysid_load
            && !self.find_load
            && !self.find_through_recordref
            && !self.through_recordref
            && !self.permission_from_al
            && !self.permission_from_recordref
            && self.record_calls.is_empty()
            && !self.setfilter_from_al
            && !self.permission_autocalc
            && !self.permission_page_calc
            && !self.find_first_read
            && self.al_record_call.is_none()
            && self.permission_opening_source.is_none()
            && self.permission_caller.is_none()
            && self.permission_run_object.is_none()
            && self.query_receiver.is_none()
            && !self.keyref_receiver_local
    }

    /// The fields a test reads, and the owners' sub-states, hold what a new session holds.
    fn holds_as_new(&self) -> bool {
        let fresh_guid_key = al_host::seed::fixed_guid_key(self.host.guid_seed.unwrap_or(self.host.random_seed));
        // State a test reads that a new session holds, by the reviewer's table of PR 8153 and the field list above.
        self.last_error == al_builtins::native::AlError::default()
            && self.isolated_committed == self.isolated_storage
            && self.isolated_set_in_test.is_empty()
            && self.unmeasured.as_new()
            && self.tasks.as_new()
            && self.sessions_started == 0
            && self.demo_written.is_empty()
            && !self.gl_setup_written
            && !self.setup_caption_read_early
            && self.table_connections.is_empty()
            && self.default_table_connection.is_empty()
            && self.crm_stores.is_empty()
            && self.sequences_changed.is_empty()
            && self.singletons.is_empty()
            && self.singles_unclaimed.is_empty()
            && self.guid_key == fresh_guid_key
            && self.unseeded_base == self.host.random_seed
            && !self.randomized_with_seed
            && self.random_draws == 0
            && !self.reseeded_in_test
            && self.inherited_draws == 0
            && !self.run_seeded_in_test
            && self.carried_draws == 0
            && self.unseeded_randomizes == 0
            && self.install_row_version == self.restored_row_version
            && *self.cx.company == *self.host.identity.company_name()
            && *self.cx.user == *self.host.identity.user_id()
            && self.baseline_origin.is_none()
            && self.seed_refusal.is_none()
            && self.tx_model == crate::TxModel::default()
            && !self.in_test_codeunit
            && !self.isolation_from_hub
            && !self.hub_disabled
            && !self.runner_permission_step
            && self.permission_roles.is_empty()
            && self.permissions.is_none()
            && !self.runner_permission_sets
            && self.shadow_roles.is_empty()
            && !self.shadow_active
            && !self.shadow_lower_bound
            && self.report_refusal_after_denial.is_none()
            && self.test_object.is_none()
            && self.http_policy.is_none()
            && self.scratch_messages.is_none()
            && self.tdd_reached.is_empty()
            && self.global_group1.is_empty()
            && self.capture.is_none()
            && self.form == crate::Form::Plain
            && self.debug.is_none()
            && self.media.objects.borrow().is_empty()
            && self.media.sets.borrow().is_empty()
            && self.media.removed.borrow().is_empty()
            && self.media.outside_tenant.borrow().is_empty()
            && self.media_removed.0.is_empty()
            && self.media_removed.1.is_empty()
            && self.application_areas.is_empty()
            && self.sleep_budget.is_none()
            && self.slept_in_test == 0
            && self.encrypted.is_empty()
            && self.encryption_key == crate::EncryptionKey::Present
            && self.temp_files == 0
            && self.render_pending_files.is_empty()
            && self.clock_key == al_host::seed::clock_key(self.host.run_seed, 0, "")
            && self.caption_translations.is_empty()
            && self.ui_caption_translated.is_empty()
            && self.tx_left_as_after_commit()
            && self.iso_min.as_new()
            && self.runner_host.as_new()
            && self.blob_sources.as_new()
            && self.error_scopes.as_new()
            && self.variant_refs.as_new()
            && self.dotnet.as_new()
            && !self.host.files.changed()
            && self.placeholder_files.as_new()
            && self.temp_blob_impl_fields.as_new()
            && clients::ui_as_new(&self.ui_state)
            && clients::pages_as_new(&self.pages)
            && clients::events_as_new(&self.events)
            && clients::coverage_as_new(&self.coverage)
            && clients::report_as_new(&self.report)
    }

    /// The transaction state after the install's last Commit, field by field: what a new session holds, except what the
    /// Commit sets (`committed_row_version`, carried in the Baseline; `commit_counter`, the store's change counter, which
    /// [`Session::restore_install_left`] sets again; `last_commit_wrote` and `transaction_type`, carried).
    fn tx_left_as_after_commit(&self) -> bool {
        let crate::state::TxState {
            tx_world,
            commit_behavior,
            phase,
            // Carried in InstallLeft.
            transaction_type: _,
            lock_timeout,
            lock_timeout_duration,
            committed_row_version: _,
            commit_counter,
            rolled_back,
            rollback_unpinned,
            rollback_kind,
            last_commit_wrote: _,
            open,
            failed_sql_rename,
            insert_from_al,
            rename_from_al,
            get_from_al,
            write_of_al,
            deferred_by,
            deferred_raised,
            immediate_errors,
            try_depth,
            assert_depth,
            used_tries,
            guarded_runs,
            guarded_bodies,
            xp_export_try_world,
        } = &self.tx;
        let fresh = crate::state::TxState::fresh();
        *tx_world == fresh.tx_world
            && *commit_behavior == fresh.commit_behavior
            && *phase == fresh.phase
            && *lock_timeout == fresh.lock_timeout
            && *lock_timeout_duration == fresh.lock_timeout_duration
            && *commit_counter == self.store.change_counter()
            && !rolled_back
            && rollback_unpinned.is_none()
            && rollback_kind.is_none()
            && failed_sql_rename.is_none()
            && !insert_from_al
            && !rename_from_al
            && !get_from_al
            && write_of_al.is_none()
            && deferred_by.is_none()
            && *deferred_raised == 0
            && !immediate_errors
            && *try_depth == 0
            && *assert_depth == 0
            && used_tries.is_empty()
            && guarded_runs.is_empty()
            && guarded_bodies.is_empty()
            && xp_export_try_world.is_none()
            && open.is_clear()
    }

    /// Starts this new session from what install code left in a session of its own: the clock first (the restore keeps
    /// no row stamp ahead of it), the data, then [`InstallLeft`]. Like the session in which the install code ran, it has
    /// no baseline origin (`Session::baseline_origin`).
    pub fn restore_install_left(&mut self, data: &crate::Baseline, left: &InstallLeft) {
        self.waited_ms += left.clock_ms;
        self.restore_baseline(data);
        self.baseline_origin = None;
        self.restore_carried(left.carried);
        self.clock_draws = left.clock_draws;
        self.isolated_written_in_codeunit = left.isolated_written.clone();
        self.tx.last_commit_wrote = left.last_commit_wrote;
        self.tx.transaction_type = left.transaction_type;
        self.tx.commit_counter = self.store.change_counter();
        self.sql.restore_left(&left.sql);
        self.events.caption_context_revision = left.caption_context_revision;
        self.events
            .event_stats
            .calls
            .borrow_mut()
            .clone_from(&left.subscriber_calls);
    }
}
