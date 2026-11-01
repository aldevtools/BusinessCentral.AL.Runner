//! The integration tests of al-test-cli in one test binary, one module per file: linking one binary per file took
//! most of the debug test build. A new test file goes here as a module. A test stays a binary of its own
//! (tests/<name>.rs) when CI, a doc or a message runs it by name (`--test NAME`), when it needs a feature or a
//! process of its own, or when it declares modules of its own.
mod action_error_codes;
mod action_filter_drops_last_row;
mod action_filter_goto_key;
mod action_widens_view_first;
mod action_widens_view_window;
mod array_size_mismatch;
mod autoformat_units;
mod awaiting_assumptions;
mod bare_quoted_ext_control;
mod bigtext_write;
mod blocked_set_2;
mod blocked_set_3;
mod bucket_lowering;
mod bucket_misc;
mod bucket_service_misc;
mod bump21_unbuilt;
mod caller_modules_test_runner;
mod caption_class_empty_count;
mod caption_class_goto;
mod card_onfindrecord;
mod ci_compile_cache;
mod codemap;
mod control_add_in_calls;
mod conversions;
mod corpus_bump_bot;
mod corpus_bump_script;
mod data_caption_expression;
mod data_driven_tests;
mod data_fixes;
mod deferred_insert_at_test_end;
mod dependency_positions;
mod dotnet_diff;
mod dotnet_sidecar;
mod dotnet_typed_values;
mod drilldown_lookup;
mod duration_times_decimal;
mod enum_conversions;
mod erm_builtins_3;
mod erm_builtins_4;
mod erm_builtins_5;
mod erm_builtins_6;
mod erm_builtins_7;
mod erm_builtins_8;
mod erm_lowering_gaps;
mod explain_site;
mod filtergroup_or;
mod find_list_descending;
mod fixes2;
mod fixpages;
mod global_filter_group_page;
mod http_handlers;
mod inconsistent_at_test_end;
mod instream_scope;
mod isolation_probe;
mod language_catalog_current;
mod legal_guard;
mod list_read_order;
mod list_refused_validate_close;
mod list_update_without_current;
mod long_list_moved_rec_open;
mod lookup_deferred_refresh_part_move;
mod lookup_global_record_field;
mod lookup_ok_no_row;
mod lookup_three_field_link_row_triggers;
mod lookup_two_field_link;
mod lookup_two_field_link_row_triggers;
mod main_regressions;
mod min_active_row_version;
mod misc_marketing_2;
mod missing_codeunit_caller;
mod modal_close_veto;
mod modal_gotokey_desc_key;
mod modal_handler_close;
mod modal_handler_error_close;
mod modal_parts_veto;
mod ms_top_refusals_3;
mod native_not_reached;
mod no_microsoft_source_in_repo;
mod np_blockers_5;
mod np_blockers_68;
mod np_blockers_69;
mod numeric_extends_target;
mod option_enum_bigtext;
mod or_group_page_rec;
mod outside_fb_repros;
mod outside_rt_pages;
mod outside_sc_pins;
mod page_bare_run_modal;
mod page_object_id;
mod page_open_record;
mod page_run_field_arg;
mod page_trigger_errors;
mod page_views;
mod pageext_implicit_promoted;
mod pageext_move_actions;
mod pageext_worksheet_factbox;
mod pagemisc;
mod part_after_modal_close;
mod part_array_element_control;
mod prepush_script;
mod provider_factbox_edit;
mod quoted_ident;
mod r1b_page_moves;
mod readable_messages;
mod reapply_marks_other_key;
mod regen_bot;
mod report_handler;
mod report_run_after_write;
mod report_run_system_printer_true;
mod reportext_request_page_run;
mod request_view_stale_address;
mod requestpage;
mod requestpage_close_and_parameters;
mod rp1_worksheet_modal;
mod run_page_link_list_reads;
mod run_page_link_open_find;
mod runmodal_lookup_shape;
mod runmodal_record_first_row;
mod runobject;
mod save_row_gone;
mod single_instance_test_codeunit;
mod stale_delete_triggers;
mod stale_updlock_read;
mod step_time_guard;
mod subpagelink_field_filter;
mod temp_list_key_open;
mod test_framework_refusals;
mod testpage_action_without_parentheses;
mod this_in_objects;
mod unpinned_error_result;
mod update_draft_row;
mod user_reports;
mod values_allowed_setvalue;
mod with_statement;
mod worksheet_validate_temp_part;
mod xmlport_request_page;
mod y14_pins;
mod y17_expr_refresh;
mod y17_page_pins;
mod y41_page_probes;
mod y48_platform_probes;

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
