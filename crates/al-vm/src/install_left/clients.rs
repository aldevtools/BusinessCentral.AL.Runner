//! The session's page, UI, event, coverage and report state after install code ran ([`crate::Session::install_left`]):
//! each holds what a new session holds. Every field is named: a new one does not compile here until it is decided.

use crate::state::{CoverageState, EventState, PageClients, ReportState, UiState};

/// The handler functions and notifications of a test method: install code runs outside one.
pub(super) fn ui_as_new(ui: &UiState) -> bool {
    let UiState {
        scope,
        page_handler_depth,
        sent_notifications,
        dev_endpoint,
        // Set from the run's session flags before any code runs (`--no-client`), the same in every session of the run.
        clientless: _,
        clientless_login_skipped,
    } = ui;
    // `on`: set from the run's session flags, the same in every session of the run.
    let crate::dev_endpoint_ui::DevEndpointUi { on: _, answered } = dev_endpoint;
    scope.is_none()
        && *page_handler_depth == 0
        && sent_notifications.is_empty()
        && answered.is_empty()
        && !clientless_login_skipped
}

/// What the TestPage client keeps between calls: install code opened no page whose state lasts.
pub(super) fn pages_as_new(p: &PageClients) -> bool {
    let PageClients {
        action_tally,
        saved_values,
        saved_values_committed,
        saved_rows,
        saved_row_pending,
        traps,
        scope_unpinned,
        bg_worker,
        bg_reads,
        // Emptied when the session enters a test codeunit (`Session::reset_query_cache`), so what install code read is not
        // carried.
        query_cache: _,
        caption_dialogs,
        caption_write_guard,
        bg_drain_after_call,
        page_xrec,
        modify_record_xrec,
        curr_field_no,
        lenient_bind,
        failed_trigger,
        ext_answers,
        ext_vetoed,
        own_exited,
        named_false_insert,
        test_open,
        run_page_link_open_moved,
        run_page_link_opening,
        test_close,
        close_save_error,
        close_save_raised_in,
        shown_frames,
        shown_controls,
        trigger_kinds,
        client_depth,
        code_close_vetoes,
        open_vetoed,
        test_methods_started,
        onvalidate_pages,
    } = p;
    action_tally.is_none()
        && saved_values.is_empty()
        && saved_values_committed.is_empty()
        && saved_rows.is_empty()
        && saved_row_pending.is_empty()
        && traps.is_empty()
        && scope_unpinned.is_empty()
        && bg_worker.is_none()
        && bg_reads.is_empty()
        && *caption_dialogs == crate::pages::CaptionDialogs::default()
        && caption_write_guard.is_none()
        && bg_drain_after_call.is_none()
        && page_xrec.is_none()
        && modify_record_xrec.is_empty()
        && curr_field_no.is_empty()
        && !lenient_bind
        && failed_trigger.is_none()
        && ext_answers.is_empty()
        && !ext_vetoed
        && !own_exited
        && !named_false_insert
        && !test_open
        && !run_page_link_open_moved
        && *run_page_link_opening == 0
        && !test_close
        && close_save_error.is_none()
        && close_save_raised_in.is_none()
        && shown_frames.is_empty()
        && shown_controls.is_empty()
        && trigger_kinds.is_empty()
        && *client_depth == 0
        && code_close_vetoes.is_empty()
        && open_vetoed.is_empty()
        && *test_methods_started == 0
        && onvalidate_pages.is_empty()
}

/// Event subscriptions, what the Event Subscription table reports and the install's own event state. Not checked:
/// `caption_context_revision`, the language and the calls of `event_stats` (all carried, `InstallLeft`).
pub(super) fn events_as_new(e: &EventState) -> bool {
    let EventState {
        bindings,
        caption_context_revision: _,
        page_held,
        pages_with_instance,
        page_var_runs,
        event_stats,
        publish_frame,
        only_targets,
        sender_refused,
        var_origins,
        next_aliases,
        next_is_subscriber,
        by_value_publisher,
        raise_arg_var_param,
        next_event_args,
        blob_publisher,
        next_blob_event,
        subscriber_stack,
        next_var_lens,
        next_option_hops,
        publisher_var_lens,
        unpinned_event_depth,
        skip_dependency_subscribers,
        not_installed_apps,
        order_unknown_apps,
        order_unknown_hits,
        install_written,
        install_rows,
        setup_no_ops,
        db_setup_table,
        trial,
        global_setups,
    } = e;
    let crate::platform::EventStats {
        calls: _,
        active,
        language: _,
        install_view,
        // `Session::no_client_test_session`, kept in step by `Session::sync_no_client_test_session`: once install code ended it
        // is what the run's session flags make it in a new session.
        no_client_test_session: _,
    } = &**event_stats;
    let crate::order_trial::TrialState {
        allowed,
        picked,
        depth,
        wrote,
        read,
        guarded,
    } = trial;
    bindings.is_empty()
        && page_held.is_empty()
        && pages_with_instance.is_empty()
        && page_var_runs.is_empty()
        && active.borrow().is_empty()
        && *install_view.borrow() == crate::platform::InstallView::default()
        && !publish_frame
        && only_targets.is_none()
        && sender_refused.is_none()
        && var_origins.is_empty()
        && next_aliases.is_empty()
        && !next_is_subscriber
        && by_value_publisher.is_none()
        && !raise_arg_var_param
        && next_event_args.is_empty()
        && blob_publisher.is_none()
        && !next_blob_event
        && subscriber_stack.is_empty()
        && next_var_lens.is_empty()
        && next_option_hops.is_empty()
        && publisher_var_lens.is_empty()
        && *unpinned_event_depth == 0
        && !skip_dependency_subscribers
        && not_installed_apps.is_none()
        && order_unknown_apps.is_none()
        && order_unknown_hits.borrow().is_empty()
        && install_written.is_none()
        && install_rows.is_none()
        && setup_no_ops.is_empty()
        && db_setup_table.is_none()
        && !allowed
        && picked.is_empty()
        && *depth == 0
        && wrote.is_none()
        && read.is_empty()
        && guarded.is_empty()
        && global_setups.is_empty()
}

/// Code coverage, the impact trace, the cell tap and the profiler: all off and nothing recorded. A run with one of them on
/// keeps the install code in each codeunit's session (al-testrun `install_once.rs`); the cell tap is set from the session
/// flags, so it fails here.
pub(super) fn coverage_as_new(c: &CoverageState) -> bool {
    let CoverageState {
        code_coverage,
        coverage_counts,
        recorder,
        impact,
        impact_on,
        cells,
        profiler,
    } = c;
    !code_coverage
        && coverage_counts.is_none()
        && recorder.as_new()
        && impact.borrow().is_none()
        && !impact_on
        && cells.is_none()
        && profiler.is_none()
}

/// The report state: no report ran and nothing it noted lasts.
pub(super) fn report_as_new(r: &ReportState) -> bool {
    let ReportState {
        rebind_after_run,
        curr_report_language,
        format_region,
        variable_culture,
        run_language,
        variable_region,
        region_property,
        region_in_init,
        report_init_depth,
        report_quit,
        report_trigger,
        report_flow,
        flow_unwind,
        request_only_page,
        zero_date_view_failed,
        report_executed,
        execute_parameters,
        report_run_modal_done,
        excel_sheets,
        report_running,
        request_captions,
        print_only_if_detail,
        csv_written,
        report_save_tried,
        prepared,
        caller_writes,
        rp_init_done,
        rp_init_by_handler,
        report_preview_pinned,
        run_modal,
        run_bare,
        system_printer,
        set_view_filters,
        dataset,
    } = r;
    !rebind_after_run
        && curr_report_language.is_none()
        && format_region.is_none()
        && variable_culture.is_empty()
        && run_language.is_none()
        && !variable_region
        && !region_property
        && region_in_init.is_none()
        && report_init_depth.is_none()
        && !report_quit
        && report_trigger.is_none()
        && report_flow.is_none()
        && flow_unwind.is_none()
        && !request_only_page
        && !zero_date_view_failed
        && report_executed.is_empty()
        && execute_parameters.is_none()
        && report_run_modal_done.is_empty()
        && excel_sheets.is_idle()
        && report_running.is_empty()
        && request_captions.is_empty()
        && !print_only_if_detail
        && csv_written.is_none()
        && report_save_tried.is_empty()
        && prepared.is_empty()
        && caller_writes.is_empty()
        && rp_init_done.is_empty()
        && rp_init_by_handler.is_empty()
        && report_preview_pinned.is_empty()
        && !run_modal
        && !run_bare
        && !system_printer
        && set_view_filters.is_empty()
        && dataset.is_none()
}
