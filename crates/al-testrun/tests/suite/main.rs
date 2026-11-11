//! The integration tests of al-testrun in one test binary, one module per file: linking one binary per file took
//! most of the debug test build. A new test file goes here as a module. A test stays a binary of its own
//! (tests/<name>.rs) when CI, a doc or a message runs it by name (`--test NAME`), when it re-runs its own
//! executable, or when it declares modules of its own. `baseline_cache_files_dir` stays apart because it
//! changes what the kept baseline of other tests in the same process reads (`baseline_cache`, `warm_kept`,
//! `warm_baseline_edit` then fail). `companies_opt_in` stays apart because it sets the company list of the process
//! (`host::set_companies`), which is part of the kept baseline's key: a run of another test between its own two runs
//! got another key (`baseline_cache` then failed now and then).
mod action_trigger_lazy_refusal;
mod affected_run;
mod baseline_cache;
mod baseline_rule_carry;
mod baseline_skip;
mod capture;
mod chain_numbers;
mod chain_numbers_handed_on;
mod chain_singles;
mod code_coverage_refresh;
mod compile_cache;
mod datatransfer_clear;
mod datatransfer_install;
mod datatransfer_other_app_ext;
mod demo_row_tables;
mod dev_endpoint_ui;
mod entries;
mod execute;
mod ext_validate_across_apps;
mod failure_after_refused_test;
mod failure_after_unfinished_test;
mod fieldref_calcfield_blob;
mod filter_page_caption_assign;
mod hidden_part_turns;
mod impact_static;
mod impact_trace;
mod incremental;
mod install_db_event_order;
mod install_dep_package_data;
mod install_hold_back;
mod install_load_package_data;
mod install_order_independent;
mod install_own_table_guard;
mod install_own_table_rows;
mod install_partner_order;
mod install_refused_package_data_rows;
mod install_stopped_rows;
mod install_trigger_setup_order;
mod install_try_write_allowed;
mod lean_selection;
mod login_work_date_lean;
mod media_export_file_lazy;
mod memory_threads;
mod modify_without_change;
mod no_client_session;
mod observer_cells;
mod order_default;
mod page_insert_by_code;
mod page_run_view;
mod page_variable_control_no_insert;
mod permission_abp;
mod permission_abp_actionref;
mod permission_abp_shapes;
mod permission_execute;
mod permission_execute_triggers;
mod permission_license;
mod permission_open_page_named_grant;
mod permission_read_write;
mod permission_report_after_denial;
mod permission_set_name_scope;
mod permission_unguarded_part;
mod permission_wildcard_scope;
mod permission_writes;
mod query_after_last_row;
mod query_close_fall_through;
mod query_record_id_column;
mod recordref_into_record;
mod refused_chain_link;
mod report_default_layout;
mod report_render_discarded;
mod report_request_values;
mod report_saved_values_isolation;
mod report_trigger_lazy_refusal;
mod report_unpinned_errors;
mod report_view_over_stored_filters;
mod request_page_array_source;
mod request_page_expression_setvalue;
mod request_page_filter_blank;
mod request_page_seed_bare_root;
mod request_page_source_rec;
mod request_page_source_table_props;
mod required_isolation;
mod resource_stream_encoding;
mod run_request_page_static_view;
mod runner_isolation;
mod runner_permission_sets;
mod runner_reset;
mod selection_install_roots;
mod single_instance_lane;
mod single_instance_test_codeunit;
mod skip_after_timeout;
mod start_session_record;
mod strict_sequential;
mod testfilter_qualified_field;
mod testpage_close_at_test_end;
mod testpage_close_query_close_unhandled;
mod testpage_save_left_open_at_test_end;
mod timeout_after_refused_test;
mod translated_reads;
mod unknown_reach_reason_stable;
mod variant_number_into_text;
mod warm_baseline_edit;
mod warm_kept;
mod xmlport_encoding;
mod xmlport_flowfield_export;
mod xmlport_inlineschema;
mod xmlport_namespaces;

/// A file in this folder that no `mod` line names is never compiled, so its tests never run (a merge can move a new
/// test file here).
#[test]
fn every_file_of_the_suite_is_a_module() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/suite");
    let main = std::fs::read_to_string(dir.join("main.rs")).unwrap();
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if let Some(module) = name.strip_suffix(".rs") {
            if module != "main" && !main.lines().any(|l| l == format!("mod {module};")) {
                missing.push(name);
            }
        }
    }
    missing.sort();
    assert!(
        missing.is_empty(),
        "add `mod NAME;` to tests/suite/main.rs for: {missing:?}"
    );
}

/// A module of this binary may not change a setting of the process (`al_testrun::host::set_*`): the tests of the
/// binary run side by side, and a setting such as the company list is part of the kept baseline's key
/// (`runtime::baseline_key::baseline_key`), so a test that ran between two runs of another one gave the second run another
/// key and another baseline (`baseline_cache` then failed now and then). Such a test is a binary of its own.
#[test]
fn no_module_of_the_suite_changes_a_setting_of_the_process() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/suite");
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|n| n == "main.rs") || path.extension().is_none_or(|x| x != "rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        for (i, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            if code.contains("host::set_") {
                found.push(format!("{}:{}", path.file_name().unwrap().to_string_lossy(), i + 1));
            }
        }
    }
    found.sort();
    assert!(
        found.is_empty(),
        "move these tests to a binary of their own (tests/<name>.rs): {found:?}"
    );
}
mod enum_caption_ml_readers;
