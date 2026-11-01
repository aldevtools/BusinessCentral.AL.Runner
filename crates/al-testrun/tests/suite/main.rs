//! The integration tests of al-testrun in one test binary, one module per file: linking one binary per file took
//! most of the debug test build. A new test file goes here as a module. A test stays a binary of its own
//! (tests/<name>.rs) when CI, a doc or a message runs it by name (`--test NAME`), when it re-runs its own
//! executable, or when it declares modules of its own. `baseline_cache_files_dir` stays apart because it
//! changes what the kept baseline of other tests in the same process reads (`baseline_cache`, `warm_kept`,
//! `warm_baseline_edit` then fail).
mod action_trigger_lazy_refusal;
mod affected_run;
mod baseline_cache;
mod baseline_skip;
mod capture;
mod code_coverage_refresh;
mod companies_opt_in;
mod compile_cache;
mod datatransfer_install;
mod demo_row_tables;
mod dev_endpoint_ui;
mod entries;
mod execute;
mod ext_validate_across_apps;
mod failure_after_refused_test;
mod failure_after_unfinished_test;
mod fieldref_calcfield_blob;
mod filter_page_caption_assign;
mod impact_static;
mod impact_trace;
mod incremental;
mod install_hold_back;
mod lean_selection;
mod login_work_date_lean;
mod media_export_file_lazy;
mod modify_without_change;
mod page_insert_by_code;
mod page_run_view;
mod page_variable_control_no_insert;
mod permission_writes;
mod query_after_last_row;
mod query_close_fall_through;
mod query_record_id_column;
mod recordref_into_record;
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
mod request_page_source_rec;
mod request_page_source_table_props;
mod required_isolation;
mod resource_stream_encoding;
mod run_request_page_static_view;
mod runner_isolation;
mod runner_permission_sets;
mod runner_reset;
mod single_instance_test_codeunit;
mod start_session_record;
mod testfilter_qualified_field;
mod testpage_close_at_test_end;
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
