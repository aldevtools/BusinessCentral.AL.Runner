//! The session image (ADR-073 decision 2): every field of [`Session`] has one class, written once, in [`image_of`].
//!
//! - *imaged*: written into the image and restored from it ([`restore`]);
//! - *must be untouched*: it holds what a new session holds, before and after a recorded run; otherwise the run is not
//!   recorded and a session in that state takes no replay;
//! - *not state*: a cache that can be rebuilt, tooling data, or plumbing of a call that is not running.
//!
//! [`image_of`] destructures `Session` (and the sub-states it images) without `..`: a new field does not compile until it
//! has a class here. The image is plain data (`Send`): a recorded run made on one test thread is replayed on another.
//! Rows are not part of it (the store is the caller's key and the recorded run's row changes, `rows.rs`), nor are the
//! SingleInstance codeunits (`globals.rs`).

use super::NotRecorded;
use crate::Session;
use std::collections::{BTreeSet, HashMap, HashSet};

/// A primary key or row as bytes (`value_codec`), so it can go to another thread.
pub(super) type RowBytes = Vec<u8>;

/// Encodes `row`; `None` when a value in it has no plain form.
pub(super) fn row_bytes(row: &[al_builtins::value::Value]) -> Option<RowBytes> {
    let mut out = Vec::new();
    al_builtins::value_codec::encode_row(row, &mut out).then_some(out)
}

/// The row [`row_bytes`] encoded.
pub(super) fn row_of(bytes: &[u8]) -> Vec<al_builtins::value::Value> {
    let mut r = al_builtins::value_codec::Reader::new(bytes);
    al_builtins::value_codec::decode_row(&mut r).expect("bytes `row_bytes` wrote")
}

/// The identity and the seeds of the session's host: what code can read of the host besides the clock, which reads
/// the same for every session of a run made with the same host values. Files are no input: a run that used one is
/// not recorded.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct HostImage {
    user_id: String,
    user_security_id: [u8; 16],
    company_name: String,
    session_id: i32,
    service_instance_id: i32,
    tenant_id: String,
    gui_allowed: bool,
    is_service_tier: bool,
    windows_language: i32,
    global_language: i32,
    locale_id: i32,
    time_zone: String,
    current_client_type: al_host::ClientType,
    default_client_type: al_host::ClientType,
    now: i64,
    random_seed: i32,
    guid_seed: Option<i32>,
    run_seed: i32,
    clock_step_ms: i64,
}

fn host_image(h: &al_host::Host) -> HostImage {
    let al_host::Host {
        clock,
        identity,
        files: _, // not an input: a run that used a file is not recorded (the impact trace, `Touches::files`)
        random_seed,
        guid_seed,
        run_seed,
        clock_step_ms,
    } = h;
    HostImage {
        user_id: identity.user_id(),
        user_security_id: identity.user_security_id(),
        company_name: identity.company_name(),
        session_id: identity.session_id(),
        service_instance_id: identity.service_instance_id(),
        tenant_id: identity.tenant_id(),
        gui_allowed: identity.gui_allowed(),
        is_service_tier: identity.is_service_tier(),
        windows_language: identity.windows_language(),
        global_language: identity.global_language(),
        locale_id: identity.locale_id(),
        time_zone: identity.time_zone(),
        current_client_type: identity.current_client_type(),
        default_client_type: identity.default_client_type(),
        now: clock.now_utc_millis(),
        random_seed: *random_seed,
        guid_seed: *guid_seed,
        run_seed: *run_seed,
        clock_step_ms: *clock_step_ms,
    }
}

/// The transaction state that lasts past a commit (the rest must be what a commit leaves).
#[derive(Clone, PartialEq, Debug)]
struct TxImage {
    commit_behavior: crate::transactions::CommitBehavior,
    tx_phase: u8,
    transaction_type: i32,
    lock_timeout: bool,
    lock_timeout_duration: i64,
    committed_row_version: i64,
    prior_committed_row_version: i64,
    closed_tx_rows: ClosedRowsImage,
    closed_tx_rows_before: ClosedRowsImage,
    last_commit_wrote: bool,
    immediate_errors: bool,
    inconsistent: Vec<u32>,
    lock_unpinned: Option<&'static str>,
}

type ClosedRows = Option<Vec<(u32, Option<String>, Vec<al_builtins::value::Value>)>>;
/// [`ClosedRows`] with each key as bytes.
type ClosedRowsImage = Option<Vec<(u32, Option<String>, RowBytes)>>;
/// The isolated storage by (scope, key): value, stored encrypted, stored from a SecretText.
type StorageImage = Vec<((String, String), (String, bool, bool))>;
/// `TrialState` without its depth (0): allowed, picked, wrote, read, guarded.
type TrialImage = (bool, Vec<(usize, usize)>, Option<u32>, BTreeSet<u32>, BTreeSet<u32>);

fn closed_rows_image(rows: &ClosedRows) -> Result<ClosedRowsImage, NotRecorded> {
    let Some(rows) = rows else { return Ok(None) };
    rows.iter()
        .map(|(t, c, pk)| {
            Ok((
                *t,
                c.clone(),
                row_bytes(pk).ok_or(NotRecorded::Value("tx.closed_tx_rows"))?,
            ))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn closed_rows_of(rows: &ClosedRowsImage) -> ClosedRows {
    rows.as_ref()
        .map(|v| v.iter().map(|(t, c, pk)| (*t, c.clone(), row_of(pk))).collect())
}

/// `tx`: what lasts past a commit is imaged; every part a commit clears, and every part that only exists while code
/// runs, must be as a commit left it. `commit_counter` is the store's counter at the commit: equal to the store's
/// counter now (the caller replays the rows, which moves it).
fn tx_image(tx: &crate::state::TxState, store_counter: u64) -> Result<TxImage, NotRecorded> {
    let crate::state::TxState {
        tx_world,
        commit_behavior,
        tx_phase,
        transaction_type,
        lock_timeout,
        lock_timeout_duration,
        committed_row_version,
        prior_committed_row_version,
        closed_tx_rows,
        closed_tx_rows_before,
        rows_touched,
        tables_inserted,
        locked_tables,
        commit_counter,
        writes,
        find_set_tables,
        isolation_lock,
        rolled_back,
        last_commit_wrote,
        deferred_insert,
        insert_buffer,
        insert_from_al,
        deferred_by,
        deferred_raised,
        immediate_errors,
        try_depth,
        used_tries,
        guarded_runs,
        row_versions,
        xp_export_try_world,
        inconsistent,
        lock_tx,
        lock_unpinned,
        recref_lock_tx,
        isolation_lock_tables,
        locking_read_tx_unpinned,
    } = tx;
    let committed = *commit_counter == store_counter
        && *tx_world == 0
        && rows_touched.is_empty()
        && tables_inserted.is_empty()
        && locked_tables.is_empty()
        && writes.is_empty()
        && find_set_tables.is_empty()
        && !isolation_lock
        && !rolled_back
        && deferred_insert.is_none()
        && insert_buffer.is_empty()
        && !insert_from_al
        && deferred_by.is_none()
        && *deferred_raised == 0
        && *try_depth == 0
        && used_tries.is_empty()
        && guarded_runs.is_empty()
        && row_versions.pending_inserts.is_empty()
        && row_versions.sent == 0
        && row_versions.handed_out == row_versions.accounted
        && row_versions.unpinned.is_none()
        && xp_export_try_world.is_none()
        && !lock_tx
        && !recref_lock_tx
        && isolation_lock_tables.is_empty()
        && !locking_read_tx_unpinned;
    if !committed {
        return Err(NotRecorded::Untouched("tx (a transaction is open)"));
    }
    Ok(TxImage {
        commit_behavior: *commit_behavior,
        tx_phase: *tx_phase,
        transaction_type: *transaction_type,
        lock_timeout: *lock_timeout,
        lock_timeout_duration: *lock_timeout_duration,
        committed_row_version: *committed_row_version,
        prior_committed_row_version: *prior_committed_row_version,
        closed_tx_rows: closed_rows_image(closed_tx_rows)?,
        closed_tx_rows_before: closed_rows_image(closed_tx_rows_before)?,
        last_commit_wrote: *last_commit_wrote,
        immediate_errors: *immediate_errors,
        inconsistent: inconsistent.clone(),
        lock_unpinned: *lock_unpinned,
    })
}

/// The event state: the bindings and every part that only exists while an event is raised must be empty; the setup
/// answers and the install marks are imaged; the statistics the Event Subscription table reads are imaged by value.
#[derive(Clone, PartialEq, Debug)]
struct EventImage {
    calls: Vec<((usize, usize), i64)>,
    active: Vec<(usize, i64)>,
    stats_language: i32,
    install_view: (BTreeSet<u32>, Option<u32>, BTreeSet<u32>),
    skip_dependency_subscribers: bool,
    not_installed_apps: Option<BTreeSet<u32>>,
    order_unknown_apps: Option<BTreeSet<u32>>,
    order_unknown_hits: BTreeSet<(u32, u32)>,
    install_written: Option<BTreeSet<u32>>,
    install_rows: Option<usize>,
    setup_no_ops: BTreeSet<(usize, usize)>,
    trial: TrialImage,
    global_setups: Vec<(usize, crate::globaltriggers::GlobalSetup)>,
}

fn sorted<K: Ord + Clone, V: Clone>(m: &HashMap<K, V>) -> Vec<(K, V)> {
    let mut v: Vec<(K, V)> = m.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn event_image(e: &crate::state::EventState) -> Result<EventImage, NotRecorded> {
    let crate::state::EventState {
        bindings,
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
        subscriber_stack,
        next_var_lens,
        publisher_var_lens,
        unpinned_event_depth,
        skip_dependency_subscribers,
        not_installed_apps,
        order_unknown_apps,
        order_unknown_hits,
        install_written,
        install_rows,
        setup_no_ops,
        trial,
        global_setups,
    } = e;
    if !bindings.is_empty() || !page_held.is_empty() {
        return Err(NotRecorded::Untouched("events.bindings (a manual subscriber is bound)"));
    }
    let quiet = pages_with_instance.iter().all(|w| w.strong_count() == 0)
        && page_var_runs.iter().all(|w| w.strong_count() == 0)
        && !publish_frame
        && only_targets.is_none()
        && sender_refused.is_none()
        && var_origins.is_empty()
        && next_aliases.is_empty()
        && !next_is_subscriber
        && subscriber_stack.is_empty()
        && next_var_lens.is_empty()
        && publisher_var_lens.is_empty()
        && *unpinned_event_depth == 0
        && trial.depth == 0;
    if !quiet {
        return Err(NotRecorded::Untouched("events (an event is being raised)"));
    }
    let crate::platform::EventStats {
        calls,
        active,
        language,
        install_view,
    } = &**event_stats;
    let view = install_view.borrow();
    let crate::order_trial::TrialState {
        allowed,
        picked,
        depth: _, // 0, checked above
        wrote,
        read,
        guarded,
    } = trial;
    Ok(EventImage {
        calls: sorted(&calls.borrow()),
        active: sorted(&active.borrow()),
        stats_language: language.get(),
        install_view: (view.not_installed.clone(), view.installing, view.order_unknown.clone()),
        skip_dependency_subscribers: *skip_dependency_subscribers,
        not_installed_apps: not_installed_apps.clone(),
        order_unknown_apps: order_unknown_apps.clone(),
        order_unknown_hits: order_unknown_hits.borrow().clone(),
        install_written: install_written.clone(),
        // The shared install rows of the engine: the same `Arc` or none.
        install_rows: install_rows.as_ref().map(|a| std::sync::Arc::as_ptr(a) as usize),
        setup_no_ops: setup_no_ops.clone(),
        trial: (*allowed, picked.clone(), *wrote, read.clone(), guarded.clone()),
        global_setups: sorted(global_setups),
    })
}

/// The state of a session outside the database rows and the SingleInstance codeunits, as plain data (see the module
/// comment). Two sessions with equal images, the same rows and no SingleInstance codeunit run the same code the same way.
#[derive(Clone, PartialEq, Debug)]
pub struct SessionImage {
    last_error: al_builtins::native::AlError,
    runner_frames: String,
    fault_trace: bool,
    isolated_storage: StorageImage,
    isolated_committed: StorageImage,
    isolated_set_in_test: BTreeSet<(String, String)>,
    install_scope: bool,
    company_build: bool,
    no_client: bool,
    background_session: bool,
    sessions_started: i32,
    install_without_client: bool,
    company_built: bool,
    demo_written: BTreeSet<u32>,
    gl_setup_written: bool,
    setup_caption_read_early: bool,
    table_connections: BTreeSet<(i32, String)>,
    default_table_connection: Vec<(i32, String)>,
    sequences: Vec<((String, String), crate::sessionops::Sequence)>,
    sequences_changed: Vec<((String, String), crate::sessionops::SequenceChange)>,
    next_temp: u64,
    next_guid: u64,
    guid_key: u64,
    unseeded_base: i32,
    randomized_with_seed: bool,
    rng: al_host::random::NetRandom,
    random_draws: u64,
    calls_made: u64,
    sql: crate::sqlcount::SqlImage,
    reseeded_in_test: bool,
    inherited_draws: u64,
    run_seeded_in_test: bool,
    carried_draws: u64,
    unseeded_randomizes: i32,
    next_row_version: i64,
    deletes: u64,
    install_row_version: i64,
    restored_row_version: i64,
    session_id_override: Option<i32>,
    autoinc: Vec<((String, u32), i64)>,
    truncate_undo: Vec<((String, u32), i64)>,
    cx: al_data::AccessContext,
    baseline_rows: BTreeSet<u32>,
    tx_model: crate::TxModel,
    in_test_codeunit: bool,
    isolation_from_hub: bool,
    hub_disabled: bool,
    iso_min: crate::min_row_version::IsoMin,
    runner_permission_step: bool,
    ignore_permission_mock: bool,
    permissions_off: bool,
    permission_roles: Vec<String>,
    runner_permission_sets: bool,
    shadow_roles: Vec<String>,
    shadow_active: bool,
    shadow_lower_bound: bool,
    report_refusal_after_denial: Option<String>,
    query_object: usize,
    test_object: Option<usize>,
    http_policy: Option<&'static str>,
    scratch_messages: Option<Vec<String>>,
    tdd_reached: BTreeSet<String>,
    global_group1: Vec<(u32, String)>,
    commits: u64,
    application_areas: String,
    host: HostImage,
    companies: Vec<String>,
    company_unknown: Vec<((String, u32), String)>,
    full_copies: BTreeSet<String>,
    waited_ms: i64,
    sleep_budget: Option<i64>,
    slept_in_test: i64,
    encrypted: Vec<(String, String)>,
    encryption_key: crate::EncryptionKey,
    temp_files: u32,
    last_stamp: i64,
    clock_key: u64,
    clock_draws: u64,
    work_date: Option<i32>,
    global_language: i32,
    caption_translations: Vec<(String, i32, String)>,
    ui_caption_translated: Vec<(String, i32)>,
    tx: TxImage,
    sent_notifications: Vec<[u8; 16]>,
    dev_endpoint: crate::dev_endpoint_ui::DevEndpointUi,
    clientless: bool,
    clientless_login_skipped: bool,
    events: EventImage,
}

impl SessionImage {
    /// The test codeunit the image holds.
    pub(super) fn test_object(&self) -> Option<usize> {
        self.test_object
    }

    /// This image keyed without the test codeunit (a run that did not read it, `Touches::test_object`).
    pub(super) fn forget_test_object(&mut self) {
        self.test_object = None;
    }

    /// The WorkDate the image holds (`None`: today).
    pub(crate) fn work_date(&self) -> Option<i32> {
        self.work_date
    }
}

/// Fails with the name of a must-be-untouched field that does not hold what a new session holds.
fn untouched(fresh: bool, field: &'static str) -> Result<(), NotRecorded> {
    if fresh {
        Ok(())
    } else {
        Err(NotRecorded::Untouched(field))
    }
}

/// The session's image, or why the session is in a state no recorded run covers (a must-be-untouched field holds
/// something). Called before a recorded run (its key) and after it (its result), and before a replay (the key).
pub(super) fn image_of(s: &Session) -> Result<SessionImage, NotRecorded> {
    let Session {
        engine: _,                   // not state: the program and the engine's own data are immutable
        last_error,                  // imaged
        depth,                       // untouched
        spare_vecs: _,               // not state: emptied vectors kept for reuse
        xp_pass_views,               // untouched
        frames,                      // untouched
        xp_unlinked_passes,          // untouched
        xp_link_hidden,              // untouched
        call_stack,                  // untouched
        runner_frames,               // imaged
        fault_trace,                 // imaged
        fault_site: _,               // not state: tooling (where the last error left from, for `--explain`)
        fault_site_lost: _,          // not state: tooling, with `fault_site`
        isolated_storage,            // imaged
        isolated_committed,          // imaged
        isolated_set_in_test,        // imaged
        tasks,                       // untouched
        install_scope,               // imaged
        company_build,               // imaged
        no_client,                   // imaged
        background_session,          // imaged
        sessions_started,            // imaged
        install_without_client,      // imaged
        company_built,               // imaged
        demo_written,                // imaged
        gl_setup_written,            // imaged
        setup_caption_read_early,    // imaged
        table_connections,           // imaged
        default_table_connection,    // imaged
        crm_stores,                  // untouched
        sequences,                   // imaged
        sequences_changed,           // imaged
        singletons: _,               // imaged by `globals.rs` (the SingleInstance codeunits and their globals)
        si_track,                    // untouched
        singles_unclaimed,           // untouched
        store,                       // the rows: the caller's key and the recorded run's row changes (`rows.rs`)
        temps,                       // untouched (after the dead temporary tables are released)
        next_temp,                   // imaged
        temp_release,                // untouched
        next_guid,                   // imaged
        guid_key,                    // imaged
        unseeded_base,               // imaged
        randomized_with_seed,        // imaged
        rng,                         // imaged
        random_draws,                // imaged
        calls_made,                  // imaged
        sql,                         // imaged
        dirty_ops_open,              // untouched
        dirty_frames,                // untouched
        live_xrecs,                  // untouched
        reseeded_in_test,            // imaged
        inherited_draws,             // imaged
        run_seeded_in_test,          // imaged
        carried_draws,               // imaged
        unseeded_randomizes,         // imaged
        next_row_version,            // imaged
        deletes,                     // imaged
        install_row_version,         // imaged
        restored_row_version,        // imaged
        session_id_override,         // imaged
        autoinc,                     // imaged
        truncate_undo,               // imaged
        validating,                  // untouched
        stamp_validating,            // untouched
        plain_proc_xrec,             // untouched
        cx,                          // imaged
        baseline_rows,               // imaged
        baseline_origin: _, // the rows: the caller's key (`Session::baseline_origin`); a run does not change it
        ret_used: _,        // not state: set before each builtin call
        field_error_form,   // untouched
        error_finals: _,    // not state: plumbing of a failed call, taken by its caller
        var_alias_watch,    // untouched
        tx_model,           // imaged
        in_test_codeunit,   // imaged
        isolation_from_hub, // imaged
        hub_disabled,       // imaged
        iso_min,            // imaged
        runner_permission_step, // imaged
        runner_host,        // untouched
        ignore_permission_mock, // imaged
        permissions_off,    // imaged
        permission_roles,   // imaged
        permissions,        // untouched
        runner_permission_sets, // imaged
        shadow_roles,       // imaged
        shadow_active,      // imaged
        shadow_lower_bound, // imaged
        permission_from_al, // untouched
        permission_autocalc, // untouched
        permission_page_calc, // untouched
        permission_opening_source, // untouched
        report_refusal_after_denial, // imaged
        permission_caller,  // untouched
        permission_run_object, // untouched
        query_object,       // imaged
        test_object,        // imaged
        http_policy,        // imaged
        scratch_messages,   // imaged
        tdd_reached,        // imaged
        global_group1,      // imaged
        capture,            // untouched (decision 6: a session that notes the code it runs takes no replay)
        form,               // untouched
        debug,              // untouched (decision 6)
        constants: _,       // not state: a cache by node address
        stop: _,            // not state: the runner's stop flag for this session
        commits,            // imaged
        error_scopes,       // untouched
        initial_cache: _,   // not state: a cache by table
        adhoc_tables: _,    // not state: a cache of table definitions
        media,              // untouched
        media_removed,      // untouched
        bulk_modify,        // untouched
        variant_refs,       // untouched
        set_value_from_variant, // untouched
        host_new_running,   // untouched
        part_open_running,  // untouched
        part_sync_keeps_same_link, // untouched
        dotnet,             // untouched
        application_areas,  // imaged
        host,               // imaged (identity, seeds, clock); its files untouched
        companies,          // imaged
        company_unknown,    // imaged
        full_copies,        // imaged
        run_trigger_unknown, // untouched
        package_load,       // untouched
        waited_ms,          // imaged
        sleep_budget,       // imaged
        slept_in_test,      // imaged
        encrypted,          // imaged
        encryption_key,     // imaged
        temp_files,         // imaged
        render_pending_files, // untouched
        last_stamp,         // imaged
        clock_key,          // imaged
        clock_draws,        // imaged
        work_date,          // imaged
        global_language,    // imaged
        caption_translations, // imaged
        ui_caption_translated, // imaged
        get_load,           // untouched
        sysid_load,         // untouched
        find_load,          // untouched
        tx,                 // imaged: what lasts past a commit; the rest untouched (`tx_image`)
        ui_state,           // imaged: the notifications and client marks; the handlers untouched
        pages,              // untouched
        events,             // imaged: setup answers and marks; bindings untouched (`event_image`)
        coverage,           // untouched (decision 6)
        report,             // untouched
    } = s;
    untouched(
        *depth == 0 && frames.is_empty() && call_stack.is_empty(),
        "depth (code is running)",
    )?;
    untouched(
        xp_pass_views.is_empty() && xp_unlinked_passes.is_empty() && xp_link_hidden.is_empty(),
        "xmlport passes",
    )?;
    untouched(tasks.holds_nothing(), "tasks (background tasks)")?;
    untouched(crm_stores.is_empty(), "crm_stores")?;
    untouched(
        si_track.is_none() && singles_unclaimed.is_empty(),
        "single instances moved in or watched",
    )?;
    untouched(
        temps.is_empty() && temp_release.borrow().is_empty(),
        "temps (temporary tables held outside globals)",
    )?;
    untouched(
        *dirty_ops_open == 0 && dirty_frames.is_empty() && live_xrecs.iter().all(|w| w.strong_count() == 0),
        "record writes running",
    )?;
    untouched(
        validating.is_empty() && stamp_validating.is_empty() && plain_proc_xrec.is_empty(),
        "a validate or table procedure running",
    )?;
    untouched(
        field_error_form.is_none() && var_alias_watch.is_empty(),
        "a call running",
    )?;
    untouched(runner_host.holds_nothing(), "runner_host (a test runner codeunit runs)")?;
    untouched(permissions.is_none(), "permissions (a permission set is assigned)")?;
    untouched(
        !permission_from_al
            && !permission_autocalc
            && !permission_page_calc
            && permission_opening_source.is_none()
            && permission_caller.is_none()
            && permission_run_object.is_none(),
        "a permission check running",
    )?;
    untouched(
        capture.is_none() && debug.is_none() && *form == crate::Form::Plain,
        "value capture or debugger",
    )?;
    untouched(error_scopes.holds_nothing(), "error_scopes (collected errors)")?;
    let al_builtins::media::MediaStore {
        objects,
        sets,
        removed,
        outside_tenant,
    } = media;
    let no_media = objects.borrow().is_empty()
        && sets.borrow().is_empty()
        && removed.borrow().is_empty()
        && outside_tenant.borrow().is_empty();
    untouched(
        no_media && media_removed.0.is_empty() && media_removed.1.is_empty(),
        "media",
    )?;
    untouched(
        !bulk_modify
            && !variant_refs.get_table
            && !set_value_from_variant
            && !host_new_running
            && !part_open_running
            && part_sync_keeps_same_link.is_none(),
        "a record or page operation running",
    )?;
    untouched(dotnet.holds_nothing(), "dotnet (.NET objects)")?;
    untouched(
        run_trigger_unknown.is_none() && !package_load,
        "a company copy or package load running",
    )?;
    untouched(render_pending_files.is_empty(), "render_pending_files")?;
    untouched(!get_load && !sysid_load && !find_load, "a record load running")?;
    untouched(pages.holds_nothing(), "pages (open pages)")?;
    untouched(coverage.holds_nothing(), "coverage, impact trace or profiler")?;
    untouched(report.holds_nothing(), "report (a report runs)")?;
    let crate::state::UiState {
        scope,
        page_handler_depth,
        sent_notifications,
        dev_endpoint,
        clientless,
        clientless_login_skipped,
    } = ui_state;
    untouched(scope.is_none() && *page_handler_depth == 0, "ui_state (UI handlers)")?;
    Ok(SessionImage {
        last_error: last_error.clone(),
        runner_frames: runner_frames.clone(),
        fault_trace: *fault_trace,
        isolated_storage: sorted(isolated_storage),
        isolated_committed: sorted(isolated_committed),
        isolated_set_in_test: isolated_set_in_test.iter().cloned().collect(),
        install_scope: *install_scope,
        company_build: *company_build,
        no_client: *no_client,
        background_session: *background_session,
        sessions_started: *sessions_started,
        install_without_client: *install_without_client,
        company_built: *company_built,
        demo_written: demo_written.iter().copied().collect(),
        gl_setup_written: *gl_setup_written,
        setup_caption_read_early: *setup_caption_read_early,
        table_connections: table_connections.iter().cloned().collect(),
        default_table_connection: sorted(default_table_connection),
        sequences: sorted(sequences),
        sequences_changed: sorted(sequences_changed),
        next_temp: *next_temp,
        next_guid: *next_guid,
        guid_key: *guid_key,
        unseeded_base: *unseeded_base,
        randomized_with_seed: *randomized_with_seed,
        rng: rng.clone(),
        random_draws: *random_draws,
        calls_made: *calls_made,
        sql: sql
            .image()
            .ok_or(NotRecorded::Untouched("sql (a measurement of the SQL counters runs)"))?,
        reseeded_in_test: *reseeded_in_test,
        inherited_draws: *inherited_draws,
        run_seeded_in_test: *run_seeded_in_test,
        carried_draws: *carried_draws,
        unseeded_randomizes: *unseeded_randomizes,
        next_row_version: *next_row_version,
        deletes: *deletes,
        install_row_version: *install_row_version,
        restored_row_version: *restored_row_version,
        session_id_override: *session_id_override,
        autoinc: sorted(autoinc),
        truncate_undo: sorted(truncate_undo),
        cx: cx.clone(),
        baseline_rows: baseline_rows.clone(),
        tx_model: *tx_model,
        in_test_codeunit: *in_test_codeunit,
        isolation_from_hub: *isolation_from_hub,
        hub_disabled: *hub_disabled,
        iso_min: iso_min.clone(),
        runner_permission_step: *runner_permission_step,
        ignore_permission_mock: *ignore_permission_mock,
        permissions_off: *permissions_off,
        permission_roles: permission_roles.clone(),
        runner_permission_sets: *runner_permission_sets,
        shadow_roles: shadow_roles.clone(),
        shadow_active: *shadow_active,
        shadow_lower_bound: *shadow_lower_bound,
        report_refusal_after_denial: report_refusal_after_denial.clone(),
        query_object: *query_object,
        test_object: *test_object,
        http_policy: *http_policy,
        scratch_messages: scratch_messages.clone(),
        tdd_reached: tdd_reached.clone(),
        global_group1: sorted(global_group1),
        commits: *commits,
        application_areas: application_areas.clone(),
        host: host_image(host),
        companies: companies.clone(),
        company_unknown: sorted(company_unknown),
        full_copies: full_copies.clone(),
        waited_ms: *waited_ms,
        sleep_budget: *sleep_budget,
        slept_in_test: *slept_in_test,
        encrypted: sorted(encrypted),
        encryption_key: *encryption_key,
        temp_files: *temp_files,
        last_stamp: *last_stamp,
        clock_key: *clock_key,
        clock_draws: *clock_draws,
        work_date: *work_date,
        global_language: *global_language,
        caption_translations: caption_translations.clone(),
        ui_caption_translated: ui_caption_translated.clone(),
        tx: tx_image(tx, store.change_counter())?,
        sent_notifications: sent_notifications.clone(),
        dev_endpoint: dev_endpoint.clone(),
        clientless: *clientless,
        clientless_login_skipped: *clientless_login_skipped,
        events: event_image(events)?,
    })
}

/// Puts the imaged fields of `image` into `s`, whose image before was the recorded run's image before (the caller
/// checked): every must-be-untouched field already holds what it holds after the run. The host is not changed: its
/// image was part of the key and a run cannot change it.
pub(super) fn restore(s: &mut Session, image: &SessionImage) {
    let SessionImage {
        last_error,
        runner_frames,
        fault_trace,
        isolated_storage,
        isolated_committed,
        isolated_set_in_test,
        install_scope,
        company_build,
        no_client,
        background_session,
        sessions_started,
        install_without_client,
        company_built,
        demo_written,
        gl_setup_written,
        setup_caption_read_early,
        table_connections,
        default_table_connection,
        sequences,
        sequences_changed,
        next_temp,
        next_guid,
        guid_key,
        unseeded_base,
        randomized_with_seed,
        rng,
        random_draws,
        calls_made,
        sql,
        reseeded_in_test,
        inherited_draws,
        run_seeded_in_test,
        carried_draws,
        unseeded_randomizes,
        next_row_version,
        deletes,
        install_row_version,
        restored_row_version,
        session_id_override,
        autoinc,
        truncate_undo,
        cx,
        baseline_rows,
        tx_model,
        in_test_codeunit,
        isolation_from_hub,
        hub_disabled,
        iso_min,
        runner_permission_step,
        ignore_permission_mock,
        permissions_off,
        permission_roles,
        runner_permission_sets,
        shadow_roles,
        shadow_active,
        shadow_lower_bound,
        report_refusal_after_denial,
        query_object,
        test_object,
        http_policy,
        scratch_messages,
        tdd_reached,
        global_group1,
        commits,
        application_areas,
        host: _, // part of the key: a run cannot change the host's identity or seeds
        companies,
        company_unknown,
        full_copies,
        waited_ms,
        sleep_budget,
        slept_in_test,
        encrypted,
        encryption_key,
        temp_files,
        last_stamp,
        clock_key,
        clock_draws,
        work_date,
        global_language,
        caption_translations,
        ui_caption_translated,
        tx,
        sent_notifications,
        dev_endpoint,
        clientless,
        clientless_login_skipped,
        events,
    } = image;
    s.last_error = last_error.clone();
    s.runner_frames = runner_frames.clone();
    s.fault_trace = *fault_trace;
    s.isolated_storage = isolated_storage.iter().cloned().collect();
    s.isolated_committed = isolated_committed.iter().cloned().collect();
    s.isolated_set_in_test = isolated_set_in_test.iter().cloned().collect::<HashSet<_>>();
    s.install_scope = *install_scope;
    s.company_build = *company_build;
    s.no_client = *no_client;
    s.background_session = *background_session;
    s.sessions_started = *sessions_started;
    s.install_without_client = *install_without_client;
    s.company_built = *company_built;
    s.demo_written = demo_written.iter().copied().collect();
    s.gl_setup_written = *gl_setup_written;
    s.setup_caption_read_early = *setup_caption_read_early;
    s.table_connections = table_connections.iter().cloned().collect();
    s.default_table_connection = default_table_connection.iter().cloned().collect();
    s.sequences = sequences.iter().cloned().collect();
    s.sequences_changed = sequences_changed.iter().cloned().collect();
    s.next_temp = *next_temp;
    s.next_guid = *next_guid;
    s.guid_key = *guid_key;
    s.unseeded_base = *unseeded_base;
    s.randomized_with_seed = *randomized_with_seed;
    s.rng = rng.clone();
    s.random_draws = *random_draws;
    s.calls_made = *calls_made;
    s.sql = crate::sqlcount::SqlCounters::from_image(sql);
    s.reseeded_in_test = *reseeded_in_test;
    s.inherited_draws = *inherited_draws;
    s.run_seeded_in_test = *run_seeded_in_test;
    s.carried_draws = *carried_draws;
    s.unseeded_randomizes = *unseeded_randomizes;
    s.next_row_version = *next_row_version;
    s.deletes = *deletes;
    s.install_row_version = *install_row_version;
    s.restored_row_version = *restored_row_version;
    s.session_id_override = *session_id_override;
    s.autoinc = autoinc.iter().cloned().collect();
    s.truncate_undo = truncate_undo.iter().cloned().collect();
    s.cx = cx.clone();
    s.baseline_rows = baseline_rows.clone();
    s.tx_model = *tx_model;
    s.in_test_codeunit = *in_test_codeunit;
    s.isolation_from_hub = *isolation_from_hub;
    s.hub_disabled = *hub_disabled;
    s.iso_min = iso_min.clone();
    s.runner_permission_step = *runner_permission_step;
    s.ignore_permission_mock = *ignore_permission_mock;
    s.permissions_off = *permissions_off;
    s.permission_roles = permission_roles.clone();
    s.runner_permission_sets = *runner_permission_sets;
    s.shadow_roles = shadow_roles.clone();
    s.shadow_active = *shadow_active;
    s.shadow_lower_bound = *shadow_lower_bound;
    s.report_refusal_after_denial = report_refusal_after_denial.clone();
    s.query_object = *query_object;
    // A run does not change it (`finish_recording`); a run that did not read it is replayed in any test codeunit.
    let _ = test_object;
    s.http_policy = *http_policy;
    s.scratch_messages = scratch_messages.clone();
    s.tdd_reached = tdd_reached.clone();
    s.global_group1 = global_group1.iter().cloned().collect();
    s.commits = *commits;
    s.application_areas = application_areas.clone();
    s.companies = companies.clone();
    s.company_unknown = company_unknown.iter().cloned().collect();
    s.full_copies = full_copies.clone();
    s.waited_ms = *waited_ms;
    s.sleep_budget = *sleep_budget;
    s.slept_in_test = *slept_in_test;
    s.encrypted = encrypted.iter().cloned().collect();
    s.encryption_key = *encryption_key;
    s.temp_files = *temp_files;
    s.last_stamp = *last_stamp;
    s.clock_key = *clock_key;
    s.clock_draws = *clock_draws;
    s.work_date = *work_date;
    s.global_language = *global_language;
    // What `GlobalLanguage(Id)` sets beside the field (`hostvalues.rs`).
    al_builtins::ops::set_turkic_code(al_builtins::native::is_turkic_language(*global_language));
    al_builtins::filter::set_filter_language(*global_language);
    s.caption_translations = caption_translations.clone();
    s.ui_caption_translated = ui_caption_translated.clone();
    let TxImage {
        commit_behavior,
        tx_phase,
        transaction_type,
        lock_timeout,
        lock_timeout_duration,
        committed_row_version,
        prior_committed_row_version,
        closed_tx_rows,
        closed_tx_rows_before,
        last_commit_wrote,
        immediate_errors,
        inconsistent,
        lock_unpinned,
    } = tx;
    s.tx.commit_behavior = *commit_behavior;
    s.tx.tx_phase = *tx_phase;
    s.tx.transaction_type = *transaction_type;
    s.tx.lock_timeout = *lock_timeout;
    s.tx.lock_timeout_duration = *lock_timeout_duration;
    s.tx.committed_row_version = *committed_row_version;
    s.tx.prior_committed_row_version = *prior_committed_row_version;
    s.tx.closed_tx_rows = closed_rows_of(closed_tx_rows);
    s.tx.closed_tx_rows_before = closed_rows_of(closed_tx_rows_before);
    s.tx.last_commit_wrote = *last_commit_wrote;
    s.tx.immediate_errors = *immediate_errors;
    s.tx.inconsistent = inconsistent.clone();
    s.tx.lock_unpinned = *lock_unpinned;
    s.tx.commit_counter = s.store.change_counter();
    s.ui_state.sent_notifications = sent_notifications.clone();
    s.ui_state.dev_endpoint = dev_endpoint.clone();
    s.ui_state.clientless = *clientless;
    s.ui_state.clientless_login_skipped = *clientless_login_skipped;
    let EventImage {
        calls,
        active,
        stats_language,
        install_view,
        skip_dependency_subscribers,
        not_installed_apps,
        order_unknown_apps,
        order_unknown_hits,
        install_written,
        install_rows: _, // the engine's: the same `Arc` (part of the key)
        setup_no_ops,
        trial,
        global_setups,
    } = events;
    let stats = &s.events.event_stats;
    *stats.calls.borrow_mut() = calls.iter().cloned().collect();
    *stats.active.borrow_mut() = active.iter().cloned().collect();
    stats.language.set(*stats_language);
    {
        let mut view = stats.install_view.borrow_mut();
        view.not_installed = install_view.0.clone();
        view.installing = install_view.1;
        view.order_unknown = install_view.2.clone();
    }
    s.events.skip_dependency_subscribers = *skip_dependency_subscribers;
    s.events.not_installed_apps = not_installed_apps.clone();
    s.events.order_unknown_apps = order_unknown_apps.clone();
    *s.events.order_unknown_hits.borrow_mut() = order_unknown_hits.clone();
    s.events.install_written = install_written.clone();
    s.events.setup_no_ops = setup_no_ops.clone();
    s.events.trial.allowed = trial.0;
    s.events.trial.picked = trial.1.clone();
    s.events.trial.wrote = trial.2;
    s.events.trial.read = trial.3.clone();
    s.events.trial.guarded = trial.4.clone();
    s.events.global_setups = global_setups.iter().cloned().collect();
}
