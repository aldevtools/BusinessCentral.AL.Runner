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
    /// The language the Event Subscription and Report Layout lists follow (`EventStats::language`, 0 for 1033 until set).
    stats_language: i32,
}

impl Session {
    /// What the install code left besides its data, when every other field of the session holds what a new session
    /// holds; `None` otherwise (the caller then runs the install code in each test codeunit's own session). Called after
    /// the install code ran and before [`Session::capture_baseline`], which commits and clears what it captures.
    ///
    /// Every field is named: a new field of `Session` does not compile here until it is decided. Each is one of
    /// - carried: in the `Baseline` the caller captures, or in [`InstallLeft`];
    /// - checked: it must equal a new session's value;
    /// - not written by install code, with the reason.
    pub fn install_left(&self) -> Option<InstallLeft> {
        let Session {
            // Same engine: the caller keys what it keeps by `Engine::uid`.
            engine: _,
            last_error,
            // Counts read only as a difference around one call (pages/modal_handler_error.rs).
            caught_errors: _,
            depth,
            // Spare allocations, no AL value.
            spare_vecs: _,
            xp_pass_views,
            frames,
            xp_unlinked_passes,
            xp_link_hidden,
            call_stack,
            error_raised_in,
            runner_frames,
            // Diagnostics of the run (`--explain`), set from the caller before any code runs.
            fault_trace: _,
            fault_site,
            fault_site_lost,
            // Carried: the Baseline (storage), committed = storage after the last Commit (checked below).
            isolated_storage,
            isolated_committed,
            isolated_set_in_test,
            // Carried in InstallLeft.
            isolated_written_in_codeunit,
            unmeasured,
            tasks,
            install_scope,
            company_build,
            // Set from the run's session flags before any code runs, the same in every session of the run.
            no_client: _,
            background_session,
            sessions_started,
            install_without_client,
            // Carried: `Baseline::company_built`.
            company_built: _,
            demo_written,
            gl_setup_written,
            setup_caption_read_early,
            table_connections,
            default_table_connection,
            crm_stores,
            // Carried: the Baseline.
            sequences: _,
            sequences_changed,
            singletons,
            // Set by the caller after the install code, from the run's lane (`single_lane.rs`).
            si_track: _,
            singles_unclaimed,
            // Carried: the Baseline.
            store: _,
            // Tables of temporary records: no AL value of the install code outlives it (its instances end, and the
            // single instances are reset, al-testrun install.rs `run_apps_under_test`), so none is reachable.
            temps: _,
            next_temp: _,
            temp_release: _,
            // Carried: the Baseline.
            next_guid: _,
            guid_key,
            unseeded_base,
            randomized_with_seed,
            // The Random stream: moved only by a draw or a reseed, which `random_draws`, `randomized_with_seed` and
            // `unseeded_randomizes` count.
            rng: _,
            random_draws,
            // Counts read only as a difference around one call (isdirty.rs, globaltriggers.rs, records.rs).
            calls_made: _,
            sql,
            dirty_ops_open,
            dirty_frames,
            live_xrecs,
            reseeded_in_test,
            inherited_draws,
            run_seeded_in_test,
            carried_draws,
            unseeded_randomizes,
            // Carried: the Baseline.
            next_row_version: _,
            // Counts read only as a difference around one call (pages.rs, pages/empty_part_rec.rs).
            deletes: _,
            rollbacks: _,
            install_row_version,
            // Made from the engine's test data, the same in every session of the run.
            restored_row_version,
            session_id_override,
            // Carried: the Baseline.
            autoinc: _,
            truncate_undo,
            validating,
            stamp_validating,
            validate_site,
            validate_sites,
            plain_proc_xrec,
            cx,
            // Carried: `Baseline::install_rows` (the session has no baseline, so it is empty).
            baseline_rows: _,
            baseline_origin,
            seed_refusal,
            // Set before each builtin call and each routine return that reads it.
            ret_used: _,
            exited_with_value: _,
            field_error_form,
            error_finals,
            var_alias_watch,
            tx_model,
            in_test_codeunit,
            isolation_from_hub,
            hub_disabled,
            iso_min,
            runner_permission_step,
            runner_host,
            // Set from the run's session flags before any code runs.
            ignore_permission_mock: _,
            permissions_off: _,
            permission_roles,
            permissions,
            runner_permission_sets,
            shadow_roles,
            shadow_active,
            shadow_lower_bound,
            permission_from_al,
            permission_from_recordref,
            record_calls,
            setfilter_from_al,
            permission_autocalc,
            permission_page_calc,
            find_first_read,
            al_record_call,
            permission_opening_source,
            report_refusal_after_denial,
            permission_caller,
            permission_run_object,
            // Set before each query method call that reads it.
            query_object: _,
            query_receiver,
            keyref_receiver_local,
            test_object,
            http_policy,
            scratch_messages,
            tdd_reached,
            global_group1,
            // Set by the caller after the install code (al-testrun run.rs: capture, debugger).
            capture,
            form,
            debug,
            // Constants of the program built on first use: the same values in every session.
            constants: _,
            // The stop flag of the session's own time limit, set from another thread.
            stop: _,
            // Counts read only as a difference around one test method (al-testrun isolation.rs).
            commits: _,
            blob_sources,
            error_scopes,
            // Caches of what the program defines: the same values in every session.
            initial_cache: _,
            adhoc_tables: _,
            media,
            media_removed,
            bulk_modify,
            variant_refs,
            set_value_from_variant,
            host_new_running,
            part_open_running,
            host_move_running,
            part_sync_keeps_same_link,
            dotnet,
            application_areas,
            // Its files are checked below (`FileSystem::changed`); its clock, identity and seeds are made from the same
            // scratch settings in every session of the run (al-testrun `install_once.rs` `Key`).
            host,
            // Made from the host's identity, the same in every session of the run.
            companies: _,
            // Carried: the Baseline.
            company_unknown: _,
            full_copies: _,
            run_trigger_unknown,
            package_load,
            // Carried in InstallLeft.
            waited_ms,
            // Read by the caller (`Session::read_resources`).
            read_resources: _,
            sleep_budget,
            slept_in_test,
            encrypted,
            encryption_key,
            temp_files,
            render_pending_files,
            placeholder_files,
            temp_blob_impl_fields,
            // Carried: the Baseline.
            last_stamp: _,
            clock_key,
            // Carried in InstallLeft.
            clock_draws,
            work_date,
            global_language,
            caption_translations,
            ui_caption_translated,
            get_load,
            sysid_load,
            find_load,
            find_through_recordref,
            through_recordref,
            tx,
            ui_state,
            pages,
            events,
            coverage,
            report,
        } = self;
        let fresh_guid_key = al_host::seed::fixed_guid_key(host.guid_seed.unwrap_or(host.random_seed));
        let at_rest = *depth == 0
            && frames.is_empty()
            && call_stack.is_empty()
            && xp_pass_views.is_empty()
            && xp_unlinked_passes.is_empty()
            && xp_link_hidden.is_empty()
            && error_raised_in.is_none()
            && runner_frames.is_empty()
            && fault_site.is_empty()
            && !fault_site_lost
            && *dirty_ops_open == 0
            && dirty_frames.is_empty()
            && live_xrecs.iter().all(|w| w.strong_count() == 0)
            && truncate_undo.is_empty()
            && validating.is_empty()
            && stamp_validating.is_empty()
            && validate_site.is_none()
            && validate_sites.is_empty()
            && plain_proc_xrec.is_empty()
            && field_error_form.is_none()
            && error_finals.is_none()
            && var_alias_watch.is_empty()
            && !install_scope
            && !company_build
            && !background_session
            && !install_without_client
            && session_id_override.is_none()
            && run_trigger_unknown.is_none()
            && !package_load
            && !bulk_modify
            && !set_value_from_variant
            && host_new_running.is_none()
            && !part_open_running
            && host_move_running.is_none()
            && part_sync_keeps_same_link.is_none()
            && !get_load
            && !sysid_load
            && !find_load
            && !find_through_recordref
            && !through_recordref
            && !permission_from_al
            && !permission_from_recordref
            && record_calls.is_empty()
            && !setfilter_from_al
            && !permission_autocalc
            && !permission_page_calc
            && !find_first_read
            && al_record_call.is_none()
            && permission_opening_source.is_none()
            && permission_caller.is_none()
            && permission_run_object.is_none()
            && query_receiver.is_none()
            && !keyref_receiver_local;
        // State a test reads that a new session holds, by the reviewer's table of PR 8153 and the field list above.
        let as_new = *last_error == al_builtins::native::AlError::default()
            && isolated_committed == isolated_storage
            && isolated_set_in_test.is_empty()
            && unmeasured.as_new()
            && tasks.as_new()
            && *sessions_started == 0
            && demo_written.is_empty()
            && !gl_setup_written
            && !setup_caption_read_early
            && table_connections.is_empty()
            && default_table_connection.is_empty()
            && crm_stores.is_empty()
            && sequences_changed.is_empty()
            && singletons.is_empty()
            && singles_unclaimed.is_empty()
            && *guid_key == fresh_guid_key
            && *unseeded_base == host.random_seed
            && !randomized_with_seed
            && *random_draws == 0
            && !reseeded_in_test
            && *inherited_draws == 0
            && !run_seeded_in_test
            && *carried_draws == 0
            && *unseeded_randomizes == 0
            && install_row_version == restored_row_version
            && *cx.company == *host.identity.company_name()
            && *cx.user == *host.identity.user_id()
            && baseline_origin.is_none()
            && seed_refusal.is_none()
            && *tx_model == crate::TxModel::default()
            && !in_test_codeunit
            && !isolation_from_hub
            && !hub_disabled
            && !runner_permission_step
            && permission_roles.is_empty()
            && permissions.is_none()
            && !runner_permission_sets
            && shadow_roles.is_empty()
            && !shadow_active
            && !shadow_lower_bound
            && report_refusal_after_denial.is_none()
            && test_object.is_none()
            && http_policy.is_none()
            && scratch_messages.is_none()
            && tdd_reached.is_empty()
            && global_group1.is_empty()
            && capture.is_none()
            && *form == crate::Form::Plain
            && debug.is_none()
            && media.objects.borrow().is_empty()
            && media.sets.borrow().is_empty()
            && media.removed.borrow().is_empty()
            && media.outside_tenant.borrow().is_empty()
            && media_removed.0.is_empty()
            && media_removed.1.is_empty()
            && application_areas.is_empty()
            && sleep_budget.is_none()
            && *slept_in_test == 0
            && encrypted.is_empty()
            && *encryption_key == crate::EncryptionKey::Present
            && *temp_files == 0
            && render_pending_files.is_empty()
            && *clock_key == al_host::seed::clock_key(host.run_seed, 0, "")
            && caption_translations.is_empty()
            && ui_caption_translated.is_empty()
            && self.tx_left_as_after_commit();
        let owners_as_new = iso_min.as_new()
            && runner_host.as_new()
            && blob_sources.as_new()
            && error_scopes.as_new()
            && variant_refs.as_new()
            && dotnet.as_new()
            && !host.files.changed()
            && placeholder_files.as_new()
            && temp_blob_impl_fields.as_new()
            && clients::ui_as_new(ui_state)
            && clients::pages_as_new(pages)
            && clients::events_as_new(events)
            && clients::coverage_as_new(coverage)
            && clients::report_as_new(report);
        if !(at_rest && as_new && owners_as_new) {
            return None;
        }
        Some(InstallLeft {
            carried: crate::CarriedState {
                language: *global_language,
                work_date: *work_date,
            },
            clock_ms: *waited_ms,
            clock_draws: *clock_draws,
            isolated_written: isolated_written_in_codeunit.clone(),
            last_commit_wrote: tx.last_commit_wrote,
            transaction_type: tx.transaction_type,
            sql: sql.left_inactive()?,
            caption_context_revision: events.caption_context_revision,
            stats_language: events.event_stats.language.get(),
        })
    }

    /// The transaction state after the install's last Commit, field by field: what a new session holds, except what the
    /// Commit sets (`committed_row_version`, carried in the Baseline; `commit_counter`, the store's change counter, which
    /// [`Session::restore_install_left`] sets again; `last_commit_wrote` and `transaction_type`, carried).
    fn tx_left_as_after_commit(&self) -> bool {
        let crate::state::TxState {
            tx_world,
            commit_behavior,
            tx_phase,
            // Carried in InstallLeft.
            transaction_type: _,
            lock_timeout,
            lock_timeout_duration,
            committed_row_version: _,
            rows_touched,
            tables_inserted,
            locked_tables,
            commit_counter,
            writes,
            isolation_lock,
            rolled_back,
            rollback_unpinned,
            rollback_kind,
            reads_after_rollback,
            last_commit_wrote: _,
            deferred_insert,
            failed_sql_rename,
            insert_buffer,
            insert_from_al,
            rename_from_al,
            write_of_al,
            deferred_by,
            deferred_raised,
            immediate_errors,
            try_depth,
            used_tries,
            guarded_runs,
            guarded_bodies,
            row_versions,
            xp_export_try_world,
            inconsistent,
            lock_tx,
            lock_unpinned,
            recref_lock_tx,
            isolation_lock_tables,
            locking_reads,
            db_reads,
            locking_read_tx_unpinned,
        } = &self.tx;
        let fresh = crate::state::TxState::fresh();
        *tx_world == fresh.tx_world
            && *commit_behavior == fresh.commit_behavior
            && *tx_phase == fresh.tx_phase
            && *lock_timeout == fresh.lock_timeout
            && *lock_timeout_duration == fresh.lock_timeout_duration
            && rows_touched.is_empty()
            && tables_inserted.is_empty()
            && locked_tables.is_empty()
            && *commit_counter == self.store.change_counter()
            && writes.is_empty()
            && !isolation_lock
            && !rolled_back
            && rollback_unpinned.is_none()
            && rollback_kind.is_none()
            && reads_after_rollback.is_empty()
            && deferred_insert.is_none()
            && failed_sql_rename.is_none()
            && insert_buffer.is_empty()
            && !insert_from_al
            && !rename_from_al
            && write_of_al.is_none()
            && deferred_by.is_none()
            && *deferred_raised == 0
            && !immediate_errors
            && *try_depth == 0
            && used_tries.is_empty()
            && guarded_runs.is_empty()
            && guarded_bodies.is_empty()
            && row_versions.pending_inserts.is_empty()
            && row_versions.sent == 0
            && row_versions.handed_out == 0
            && row_versions.accounted == 0
            && row_versions.last == 0
            && row_versions.unpinned.is_none()
            && xp_export_try_world.is_none()
            && inconsistent.is_empty()
            && !lock_tx
            && lock_unpinned.is_none()
            && !recref_lock_tx
            && isolation_lock_tables.is_empty()
            && locking_reads.is_empty()
            && *db_reads == 0
            && !locking_read_tx_unpinned
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
        self.events.event_stats.language.set(left.stats_language);
    }
}
