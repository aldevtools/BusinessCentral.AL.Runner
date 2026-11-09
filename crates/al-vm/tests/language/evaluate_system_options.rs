//! `Evaluate` into the platform's option types (format-evaluate/SoEvaluateSystemOptions.al, corpus PR 1219, BC 27.5, 28.5,
//! 29.0): a member name in any case or a prefix of one, an ordinal, blanks and TABs around the text; the value kept after a
//! rejected text; the error text of the statement form.

fn run(src: &str) -> String {
    al_testkit::text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v))
}

/// `Evaluate(V, t[, n])` for each text, from `start`: `ok/Format(V)/Format(V, 0, 2)` joined by `|`.
fn evals(ty: &str, start: &str, texts: &[&str], number: Option<u8>) -> String {
    let n = number.map_or(String::new(), |n| format!(", {n}"));
    let body: String = texts
        .iter()
        .map(|t| {
            let t = t.replace('\'', "''").replace('\t', "' + Tab + '");
            format!(
                "V := {ty}::{start}; ok := Evaluate(V, '{t}'{n}); R += Format(ok) + '/' + Format(V) + '/' + Format(V, 0, 2) + '|';"
            )
        })
        .collect();
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var V: {ty}; ok: Boolean; R: Text; Tab: Text[1]; begin Tab[1] := 9; {body} exit(R); end; }}"
    );
    run(&src)
}

/// The statement form `Evaluate(V, t)` in a try function: `passed/Format(V)/GetLastErrorText/GetLastErrorCode`.
fn statement(ty: &str, start: &str, text: &str) -> String {
    let t = text.replace('\'', "''").replace('\t', "' + Tab + '");
    let src = format!(
        "codeunit 50100 C {{ var V: {ty}; [TryFunction] local procedure TryEval(Txt: Text) begin Evaluate(V, Txt); end; \
         procedure F(): Text var Tab: Text[1]; ok: Boolean; begin Tab[1] := 9; ClearLastError(); V := {ty}::{start}; \
         ok := TryEval('{t}'); exit(Format(ok) + '/' + Format(V) + '/' + GetLastErrorText() + '/' + GetLastErrorCode()); end; }}"
    );
    run(&src)
}

#[test]
fn object_type_reads_names_prefixes_and_ordinals_with_the_members_al_cannot_name() {
    // SoEvaluateSystemOptions.al:64 Names, :88 NameForms, :114 Numbers.
    assert_eq!(
        evals(
            "ObjectType",
            "Page",
            &[
                "TableData",
                "Table",
                "query",
                " Codeunit ",
                "Tab",
                "TableDatax",
                "TableData,Query"
            ],
            None
        ),
        "Yes/TableData/0|Yes/Table/1|Yes/Query/9|Yes/Codeunit/5|Yes/TableData/0|No/Page/8|No/Page/8|"
    );
    assert_eq!(
        evals(
            "ObjectType",
            "Page",
            &["0", "2", "4", "10", "11", "+1", "01", "-1", "1.0", "2147483648"],
            None
        ),
        "Yes/TableData/0|Yes/Form/2|No/Page/8|Yes/Enum/10|No/Page/8|Yes/Table/1|Yes/Table/1|No/Page/8|No/Page/8|No/Page/8|"
    );
    // :178 BlankAndGarbage: an empty or blank text, or only a TAB, is the first member.
    assert_eq!(
        evals(
            "ObjectType",
            "Page",
            &["", "   ", "\t", "TableData\t", "\tTableData", "1\t", "zzz", "?"],
            None
        ),
        "Yes/TableData/0|Yes/TableData/0|Yes/TableData/0|Yes/TableData/0|Yes/TableData/0|Yes/Table/1|No/Page/8|No/Page/8|"
    );
}

#[test]
fn verbosity_and_telemetry_scope_count_from_one() {
    // :386 Numbers (Verbosity), :646 Numbers (TelemetryScope), :360 NameForms.
    assert_eq!(
        evals("Verbosity", "Verbose", &["0", "1", "5", "6", "", "Cri", "error"], None),
        "No/Verbose/5|Yes/Critical/1|Yes/Verbose/5|No/Verbose/5|Yes/Critical/1|Yes/Critical/1|Yes/Error/2|"
    );
    assert_eq!(
        evals("TelemetryScope", "All", &["0", "1", "2", "3", ""], None),
        "No/All/2|Yes/ExtensionPublisher/1|Yes/All/2|No/All/2|Yes/ExtensionPublisher/1|"
    );
}

#[test]
fn the_ordinals_of_default_layout_test_permissions_and_execution_context() {
    // :1888 DefaultLayout (None 0, RDLC 1, Word 2, Excel 3; Custom is no member), :2154 TestPermissions, :2678 ExecutionContext.
    assert_eq!(
        evals(
            "DefaultLayout",
            "Word",
            &["None", "RDLC", "Word", "Excel", "Custom", "4", ""],
            None
        ),
        "Yes/None/0|Yes/RDLC/1|Yes/Word/2|Yes/Excel/3|No/Word/2|No/Word/2|Yes/None/0|"
    );
    assert_eq!(
        evals(
            "TestPermissions",
            "InheritFromTestCodeunit",
            &["Disabled", "Restrictive", "NonRestrictive", "0", "Dis", "4"],
            None
        ),
        "Yes/Disabled/3|Yes/Restrictive/1|Yes/NonRestrictive/2|Yes/InheritFromTestCodeunit/0|Yes/Disabled/3|No/InheritFromTestCodeunit/0|"
    );
    assert_eq!(
        evals(
            "ExecutionContext",
            "Upgrade",
            &["Normal", "Install", "Upgrade", "2", "3", "4"],
            None
        ),
        "Yes/Normal/0|Yes/Install/1|Yes/Upgrade/3|Yes/Uninstall/2|Yes/Upgrade/3|No/Upgrade/3|"
    );
}

#[test]
fn report_format_has_members_past_html_and_none_at_zero() {
    // :1368 ReportFormat: the empty text and 0 are None, 6 to 8 Jpeg, Xps, Meta; the prefix Exc is Excel.
    assert_eq!(
        evals("ReportFormat", "Html", &["", "0", "Exc", "Html", "6", "7", "8"], None),
        "Yes/None/0|Yes/None/0|Yes/Excel/1|Yes/Html/5|Yes/Jpeg/6|Yes/Xps/7|Yes/Meta/8|"
    );
}

#[test]
fn web_service_action_result_codes_read_by_name_and_zero() {
    // :3190 WebServiceActionResultCode: Updated is the code 200 of Get; no number from 1 to 8 is a member.
    assert_eq!(
        evals(
            "WebServiceActionResultCode",
            "Deleted",
            &["None", "Get", "Created", "Updated", "0", "1", "+1", ""],
            None
        ),
        "Yes/None/0|Yes/Get/200|Yes/Created/201|Yes/Get/200|Yes/None/0|No/Deleted/204|No/Deleted/204|Yes/None/0|"
    );
}

#[test]
fn field_class_formats_its_name_for_number_two_and_has_key_field_reference_from_bc_28() {
    // :3546 Numbers, guarded `#if BC28PLUS` at :3560; a context with no target has every gate.
    assert_eq!(
        evals("FieldClass", "FlowFilter", &["Normal", "1", "3", "4"], None),
        "Yes/Normal/Normal|Yes/FlowField/FlowField|Yes/KeyFieldReference/KeyFieldReference|No/FlowFilter/FlowFilter|"
    );
}

#[test]
fn data_classification_rejects_seven() {
    // :910 Numbers: 7 is no member (AnonymousData is not one Evaluate reads).
    assert_eq!(
        evals("DataClassification", "SystemMetadata", &["6", "7", "Cus"], None),
        "Yes/SystemMetadata/6|No/SystemMetadata/6|Yes/CustomerContent/0|"
    );
}

#[test]
fn format_numbers_zero_two_and_nine_read_alike() {
    // :530 Format9 (Verbosity).
    assert_eq!(
        evals("Verbosity", "Verbose", &["Critical", "0", ""], Some(9)),
        "Yes/Critical/1|No/Verbose/5|Yes/Critical/1|"
    );
    assert_eq!(
        evals("Verbosity", "Verbose", &["1", "zzz"], Some(2)),
        "Yes/Critical/1|No/Verbose/5|"
    );
}

#[test]
fn the_statement_form_raises_the_list_of_every_member() {
    // :208 StatementErrors (ObjectType), :474 (Verbosity), :3634 (FieldClass, BC 28 and later).
    assert_eq!(
        statement("ObjectType", "Page", "zzz"),
        "No/Page/'zzz' is not an option. The existing options are: TableData, Table, Form, Report, Codeunit, XmlPort, MenuSuite, \
         Page, Query, Enum/InvalidOptionString"
    );
    assert_eq!(
        statement("Verbosity", "Verbose", "a\tb"),
        "No/Verbose/'a\\tb' is not an option. The existing options are: Critical, Error, Warning, Normal, Verbose/InvalidOptionString"
    );
    assert_eq!(statement("Verbosity", "Verbose", "5"), "Yes/Verbose//");
    assert_eq!(
        statement("FieldClass", "FlowFilter", "it's"),
        "No/FlowFilter/'it's' is not an option. The existing options are: Normal, FlowField, FlowFilter, KeyFieldReference/InvalidOptionString"
    );
}

#[test]
fn web_service_action_result_codes_read_their_http_numbers() {
    // SoEvaluateSystemOptionsHidden.al:1519 HttpNumbers and :1538 HttpNumberForms: 200, 201 and 204 read with a plus sign,
    // blanks, a leading zero or a TAB; the numbers between and around them, a decimal point and a minus are rejected.
    assert_eq!(
        evals(
            "WebServiceActionResultCode",
            "Deleted",
            &[
                "200", "201", "202", "203", "204", "205", "400", "+200", " 200 ", "0200", "204\t", "200.0", "-200"
            ],
            None
        ),
        "Yes/Get/200|Yes/Created/201|No/Deleted/204|No/Deleted/204|Yes/Deleted/204|No/Deleted/204|No/Deleted/204|\
         Yes/Get/200|Yes/Get/200|Yes/Get/200|Yes/Deleted/204|No/Deleted/204|No/Deleted/204|"
    );
    assert_eq!(statement("WebServiceActionResultCode", "Deleted", "200"), "Yes/Get//");
}

#[test]
fn the_names_of_members_al_cannot_name_are_read_in_any_case_and_as_a_prefix() {
    // SoEvaluateSystemOptionsHidden.al:120 ObjectType_UnnamableNames1, :144 (Enum), :640-806 (ReportFormat), :1286
    // (ExecutionContext), :1722 (FieldClass, BC 28 and later; a context with no target has every gate).
    assert_eq!(
        evals(
            "ObjectType",
            "Page",
            &["Form", "form", "FORM", "  Form  ", "Fo", "En"],
            None
        ),
        "Yes/Form/2|Yes/Form/2|Yes/Form/2|Yes/Form/2|Yes/Form/2|Yes/Enum/10|"
    );
    assert_eq!(
        evals(
            "ReportFormat",
            "Html",
            &["None", "No", "Jp", "X", "Me", "ExcelDatas", "Cust", "Z"],
            Some(9)
        ),
        "Yes/None/0|Yes/None/0|Yes/Jpeg/6|Yes/Xml/4|Yes/Meta/8|Yes/ExcelDataset/9|Yes/Custom/10|Yes/Zip/11|"
    );
    assert_eq!(
        evals("ExecutionContext", "Normal", &["Uninsta"], None),
        "Yes/Uninstall/2|"
    );
    assert_eq!(
        evals("FieldClass", "FlowFilter", &["  keyfieldreference  "], None),
        "Yes/KeyFieldReference/KeyFieldReference|"
    );
    assert_eq!(statement("ObjectType", "Page", "Form"), "Yes/Form//");
}

#[test]
fn a_rejected_text_with_blanks_around_it_names_the_text_without_them() {
    // SoEvaluateSystemOptionsHidden.al:68 ObjectType_StatementBlanksText, :96 _StatementBlanksNumber (all 14 types alike).
    for text in [" zzz", "zzz ", " zzz ", "\tzzz", "zzz\t"] {
        assert_eq!(
            statement("Verbosity", "Verbose", text),
            "No/Verbose/'zzz' is not an option. The existing options are: Critical, Error, Warning, Normal, Verbose/InvalidOptionString",
            "{text:?}"
        );
    }
    assert_eq!(
        statement("Verbosity", "Verbose", " -1 "),
        "No/Verbose/'-1' is not an option. The existing options are: Critical, Error, Warning, Normal, Verbose/InvalidOptionString"
    );
}

#[test]
fn web_service_action_result_code_updated_formats_as_get() {
    // SoEvaluateSystemOptionsHidden.al:1576 WebServiceActionResultCode_FormatOfMembers `Updated` `Get/Get/200/200`.
    let src = "codeunit 50100 C { procedure F(): Text var V: WebServiceActionResultCode; begin V := WebServiceActionResultCode::Updated; \
               exit(Format(V) + '/' + Format(V, 0, 1) + '/' + Format(V, 0, 2) + '/' + Format(V, 0, 9)); end; }";
    assert_eq!(run(src), "Get/Get/200/200");
}

#[test]
fn a_field_class_variable_formats_its_name_in_every_standard_format() {
    // format-evaluate/SoEvaluateSystemOptions.al:3769 FormatOfVariable_Members, :3786 _Default, :3806 _FromEvaluate (BC 28 and
    // later: KeyFieldReference; a context with no target has every gate).
    let src = "codeunit 50100 C { local procedure S(V: FieldClass): Text begin exit(Format(V) + '|' + Format(V, 0, 0) + '|' + \
               Format(V, 0, 1) + '|' + Format(V, 0, 2) + '|' + Format(V, 0, 9)); end; \
               procedure F(): Text var V: FieldClass; R: Text; begin R := S(V) + '/' + Format(V, 3) + '/'; \
               V := FieldClass::FlowField; R += S(V) + '/'; V := FieldClass::FlowFilter; R += S(V) + '/'; \
               Evaluate(V, '3'); R += S(V); exit(R); end; }";
    assert_eq!(
        run(src),
        "Normal|Normal|Normal|Normal|Normal/Nor/FlowField|FlowField|FlowField|FlowField|FlowField/\
         FlowFilter|FlowFilter|FlowFilter|FlowFilter|FlowFilter/\
         KeyFieldReference|KeyFieldReference|KeyFieldReference|KeyFieldReference|KeyFieldReference"
    );
    let src = "codeunit 50100 C { procedure F(): Text var V: FieldClass; begin asserterror Message(Format(V, 0, '<Text>')); \
               exit(GetLastErrorText() + '/' + GetLastErrorCode()); end; }";
    assert_eq!(
        run(src),
        "An invalid field or attribute has been specified for the 'Format' property.\\\\<Text>/InvalidFormatString"
    );
}
