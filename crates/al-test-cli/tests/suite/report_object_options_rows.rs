//! al-runner issue 8250: a request page with SaveValues keeps what an OK confirmed in an "Object Options" row, so deleting the
//! report's rows makes the next run show a blank field. Corpus PR 1462 (merged on the corpus master, not pinned yet),
//! tests/al-language/reports/ReportObjectOptionsSaveValues.al, measures it; until it is pinned the row runs through assumption
//! `report-saved-values-object-options`, in the measured shapes only.
//!
//! Needs `System.app` of the pinned build in the artifact cache (`al_testfiles`).

use super::field_extension_package_ids::run_isolated;
use al_builtins::assumptions::Assumption;
use al_sema::workspace::AppKind;
use al_test_cli::report::Outcome;
use al_testrun::isolation::TestIsolation;

const APP_ID: &str = "8250a000-0000-4000-8000-000000008250";

const REPORTS: &str = r#"
report 51320 "Repro Saved Values"
{
    ProcessingOnly = true;
    UsageCategory = None;
    requestpage
    {
        SaveValues = true;
        layout { area(content) { field("Some Filter"; SomeFilter) { ApplicationArea = All; } } }
    }
    var
        SomeFilter: Text;
}

report 51322 "Repro Saved B"
{
    ProcessingOnly = true;
    UsageCategory = None;
    requestpage
    {
        SaveValues = true;
        layout { area(content) { field("Some Filter"; SomeFilter) { ApplicationArea = All; } } }
    }
    var
        SomeFilter: Text;
}

report 51323 "Repro Unsaved"
{
    ProcessingOnly = true;
    UsageCategory = None;
    requestpage
    {
        layout { area(content) { field("Some Filter"; SomeFilter) { ApplicationArea = All; } } }
    }
    var
        SomeFilter: Text;
}

report 51324 "Repro Saved Int"
{
    ProcessingOnly = true;
    UsageCategory = None;
    requestpage
    {
        SaveValues = true;
        layout { area(content) { field("Some Number"; SomeNumber) { ApplicationArea = All; } } }
    }
    var
        SomeNumber: Integer;
}
"#;

const HELPERS: &str = r#"
    var
        Mode: Option Set,SetCancel,Read,ReadOk;
        SetTxt: Text;
        SeenTxt: Text;
        SetInt: Integer;
        SeenInt: Integer;

    local procedure ClearRows(ReportId: Integer)
    var
        ObjectOptions: Record "Object Options";
    begin
        ObjectOptions.SetRange("Object Type", ObjectOptions."Object Type"::Report);
        ObjectOptions.SetRange("Object ID", ReportId);
        ObjectOptions.DeleteAll();
        Commit();
    end;

    local procedure RunSet(ReportId: Integer; Value: Text)
    begin
        Mode := Mode::Set;
        SetTxt := Value;
        Commit();
        Report.RunModal(ReportId);
    end;

    local procedure RunRead(ReportId: Integer): Text
    begin
        Mode := Mode::Read;
        SeenTxt := 'not read';
        Commit();
        Report.RunModal(ReportId);
        exit(SeenTxt);
    end;

    local procedure RowCount(ReportId: Integer): Integer
    var
        ObjectOptions: Record "Object Options";
    begin
        ObjectOptions.SetRange("Object Type", ObjectOptions."Object Type"::Report);
        ObjectOptions.SetRange("Object ID", ReportId);
        exit(ObjectOptions.Count());
    end;

    local procedure FindRow(ReportId: Integer; var ObjectOptions: Record "Object Options"): Boolean
    begin
        ObjectOptions.SetRange("Object Type", ObjectOptions."Object Type"::Report);
        ObjectOptions.SetRange("Object ID", ReportId);
        exit(ObjectOptions.FindFirst());
    end;

    local procedure OptionDataText(var ObjectOptions: Record "Object Options"): Text
    var
        InStr: InStream;
        Result: Text;
    begin
        ObjectOptions.CalcFields("Option Data");
        if not ObjectOptions."Option Data".HasValue() then
            exit('');
        ObjectOptions."Option Data".CreateInStream(InStr, TextEncoding::UTF8);
        InStr.Read(Result);
        exit(Result);
    end;

    local procedure MoveRow(ReportId: Integer; What: Integer; NewValue: Text)
    var
        ObjectOptions: Record "Object Options";
        Copy: Record "Object Options";
    begin
        if not FindRow(ReportId, ObjectOptions) then
            Error('no row to move');
        ObjectOptions.CalcFields("Option Data");
        Copy := ObjectOptions;
        case What of
            1:
                Copy."User Name" := CopyStr(NewValue, 1, MaxStrLen(Copy."User Name"));
            2:
                Copy."Company Name" := CopyStr(NewValue, 1, MaxStrLen(Copy."Company Name"));
            3:
                Copy."Parameter Name" := CopyStr(NewValue, 1, MaxStrLen(Copy."Parameter Name"));
        end;
        ClearRows(ReportId);
        Copy.Insert();
        Commit();
    end;

    local procedure Expect(What: Text; Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('%1: %2, expected %3', What, Actual, Expected);
    end;

    [RequestPageHandler]
    procedure HandlerA(var RequestPage: TestRequestPage "Repro Saved Values")
    begin
        case Mode of
            Mode::Set:
                begin
                    RequestPage."Some Filter".SetValue(SetTxt);
                    RequestPage.OK().Invoke();
                end;
            Mode::SetCancel:
                begin
                    RequestPage."Some Filter".SetValue(SetTxt);
                    RequestPage.Cancel().Invoke();
                end;
            Mode::Read:
                begin
                    SeenTxt := RequestPage."Some Filter".Value();
                    RequestPage.Cancel().Invoke();
                end;
            Mode::ReadOk:
                begin
                    SeenTxt := RequestPage."Some Filter".Value();
                    RequestPage.OK().Invoke();
                end;
        end;
    end;

    [RequestPageHandler]
    procedure HandlerB(var RequestPage: TestRequestPage "Repro Saved B")
    begin
        if Mode = Mode::Set then begin
            RequestPage."Some Filter".SetValue(SetTxt);
            RequestPage.OK().Invoke();
        end else begin
            SeenTxt := RequestPage."Some Filter".Value();
            RequestPage.Cancel().Invoke();
        end;
    end;

    [RequestPageHandler]
    procedure HandlerU(var RequestPage: TestRequestPage "Repro Unsaved")
    begin
        RequestPage."Some Filter".SetValue(SetTxt);
        RequestPage.OK().Invoke();
    end;

    [RequestPageHandler]
    procedure HandlerI(var RequestPage: TestRequestPage "Repro Saved Int")
    begin
        if Mode = Mode::Set then begin
            RequestPage."Some Number".SetValue(SetInt);
            RequestPage.OK().Invoke();
        end else begin
            Evaluate(SeenInt, RequestPage."Some Number".Value());
            RequestPage.Cancel().Invoke();
        end;
    end;
"#;

fn run_tests(tests: &str) -> Vec<(String, Outcome, Vec<Assumption>)> {
    run_tests_isolated(tests, TestIsolation::Codeunit)
}

fn run_tests_isolated(tests: &str, isolation: TestIsolation) -> Vec<(String, Outcome, Vec<Assumption>)> {
    let src = format!(
        "{REPORTS}\ncodeunit 51321 T\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n{HELPERS}\n{tests}\n}}\n"
    );
    run_isolated(&[(APP_ID, AppKind::Workspace, &src)], isolation)
}

fn outcome<'a>(o: &'a [(String, Outcome, Vec<Assumption>)], n: &str) -> &'a (String, Outcome, Vec<Assumption>) {
    o.iter()
        .find(|(name, ..)| name == n)
        .unwrap_or_else(|| panic!("no test {n}: {o:?}"))
}

fn assert_flagged_pass(o: &[(String, Outcome, Vec<Assumption>)], names: &[&str]) {
    for n in names {
        let (_, r, h) = outcome(o, n);
        assert_eq!(*r, Outcome::Pass, "{n}: {r:?}");
        assert!(h.contains(&Assumption::ReportSavedValuesObjectOptions), "{n}: {h:?}");
    }
}

fn assert_clean_pass(o: &[(String, Outcome, Vec<Assumption>)], names: &[&str]) {
    for n in names {
        let (_, r, h) = outcome(o, n);
        assert_eq!(*r, Outcome::Pass, "{n}: {r:?}");
        assert!(!h.contains(&Assumption::ReportSavedValuesObjectOptions), "{n}: {h:?}");
    }
}

fn assert_refused(o: &[(String, Outcome, Vec<Assumption>)], names: &[&str]) {
    for n in names {
        let (_, r, _) = outcome(o, n);
        assert!(
            matches!(r, Outcome::NotRun(why) if why.contains("Object Options")),
            "{n}: {r:?}"
        );
    }
}

/// The reporter's repro (issue 8250, BC 29 sandbox: both pass) and the cells of corpus PR 1462.
#[test]
fn issue_8250_saved_values_live_in_object_options_rows() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure A_RunSavesAValue()
    begin
        Commit();
        Mode := Mode::Set;
        SetTxt := 'ABC';
        Report.RunModal(Report::"Repro Saved Values");
    end;

    [Test]
    [HandlerFunctions('HandlerA')]
    procedure B_DeletedObjectOptionsForgetTheValue()
    var
        ObjectOptions: Record "Object Options";
        HadRow: Boolean;
    begin
        ObjectOptions.SetRange("Object Type", ObjectOptions."Object Type"::Report);
        ObjectOptions.SetRange("Object ID", Report::"Repro Saved Values");
        HadRow := not ObjectOptions.IsEmpty();
        ObjectOptions.DeleteAll();
        Commit();
        Mode := Mode::ReadOk;
        SeenTxt := 'not read';
        Report.RunModal(Report::"Repro Saved Values");
        if SeenTxt <> '' then
            Error('The request page showed "%1" after the report''s Object Options were deleted (a saved row existed: %2).', SeenTxt, HadRow);
    end;

    // ReportObjectOptionsSaveValues.al:178
    [Test]
    [HandlerFunctions('HandlerA,HandlerU')]
    procedure C_SavedRunWritesOneRow()
    var
        ObjectOptions: Record "Object Options";
    begin
        ClearRows(Report::"Repro Saved Values");
        ClearRows(Report::"Repro Unsaved");
        Expect('rows before', Format(RowCount(Report::"Repro Saved Values")), '0');
        RunSet(Report::"Repro Saved Values", 'OBO-ABC');
        Expect('rows after OK', Format(RowCount(Report::"Repro Saved Values")), '1');
        FindRow(Report::"Repro Saved Values", ObjectOptions);
        Expect('Parameter Name', ObjectOptions."Parameter Name", 'Last used options and filters');
        Expect('User Name', Format(ObjectOptions."User Name" = UserId()), 'Yes');
        Expect('Company Name', Format(ObjectOptions."Company Name" = CompanyName()), 'Yes');
        Expect('Created By', Format(ObjectOptions."Created By" = UserId()), 'Yes');
        Expect('Public Visible', Format(ObjectOptions."Public Visible"), 'No');
        Expect('Temporary', Format(ObjectOptions.Temporary), 'No');
        Expect('Option Data', Format(OptionDataText(ObjectOptions).Contains('OBO-ABC')), 'Yes');
        RunSet(Report::"Repro Unsaved", 'OBO-UNSAVED');
        Expect('rows without SaveValues', Format(RowCount(Report::"Repro Unsaved")), '0');
    end;

    // :218
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure D_NoDeleteNextRunShowsTheValue()
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-KEEP');
        Expect('next run', RunRead(Report::"Repro Saved Values"), 'OBO-KEEP');
    end;

    // :232
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure E_DeleteRowsNextRunShowsBlank()
    var
        ObjectOptions: Record "Object Options";
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-ABC');
        ObjectOptions.SetRange("Object Type", ObjectOptions."Object Type"::Report);
        ObjectOptions.SetRange("Object ID", Report::"Repro Saved Values");
        ObjectOptions.DeleteAll();
        Commit();
        Mode := Mode::ReadOk;
        SeenTxt := 'not read';
        Report.RunModal(Report::"Repro Saved Values");
        Expect('next run', SeenTxt, '');
        Expect('rows after the OK', Format(RowCount(Report::"Repro Saved Values")), '1');
        Expect('run after', RunRead(Report::"Repro Saved Values"), '');
    end;

    // :259
    [Test]
    [HandlerFunctions('HandlerA,HandlerB')]
    procedure F_DeleteRowsOfOneReport()
    begin
        ClearRows(Report::"Repro Saved Values");
        ClearRows(Report::"Repro Saved B");
        RunSet(Report::"Repro Saved Values", 'OBO-A1');
        RunSet(Report::"Repro Saved B", 'OBO-B1');
        ClearRows(Report::"Repro Saved Values");
        Expect('A', RunRead(Report::"Repro Saved Values"), '');
        Expect('B', RunRead(Report::"Repro Saved B"), 'OBO-B1');
    end;

    // :277
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure G_DeleteAndInsertAgain()
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-AGAIN');
        MoveRow(Report::"Repro Saved Values", 0, '');
        Expect('next run', RunRead(Report::"Repro Saved Values"), 'OBO-AGAIN');
    end;

    // :294
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure H_EditOptionData()
    var
        ObjectOptions: Record "Object Options";
        OutStr: OutStream;
        Data: Text;
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-OLD');
        FindRow(Report::"Repro Saved Values", ObjectOptions);
        Data := OptionDataText(ObjectOptions);
        ObjectOptions."Option Data".CreateOutStream(OutStr, TextEncoding::UTF8);
        OutStr.WriteText(Data.Replace('OBO-OLD', 'OBO-NEW'));
        ObjectOptions.Modify();
        Commit();
        Expect('next run', RunRead(Report::"Repro Saved Values"), 'OBO-NEW');
    end;

    // :321, :339, :357
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure I_RowOfAnotherUser()
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-USER');
        MoveRow(Report::"Repro Saved Values", 1, 'OBO OTHER USER');
        Expect('next run', RunRead(Report::"Repro Saved Values"), '');
        Expect('rows after', Format(RowCount(Report::"Repro Saved Values")), '1');
    end;

    [Test]
    [HandlerFunctions('HandlerA')]
    procedure J_RowOfAnotherCompany()
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-COMPANY');
        MoveRow(Report::"Repro Saved Values", 2, 'OBO Other Company');
        Expect('next run', RunRead(Report::"Repro Saved Values"), '');
    end;

    [Test]
    [HandlerFunctions('HandlerA')]
    procedure K_RowUnderAnotherParameterName()
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-NAME');
        MoveRow(Report::"Repro Saved Values", 3, 'OBO Other Name');
        Expect('next run', RunRead(Report::"Repro Saved Values"), '');
    end;

    // :374
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure L_CancelledRun()
    begin
        ClearRows(Report::"Repro Saved Values");
        Mode := Mode::SetCancel;
        SetTxt := 'OBO-CANCEL';
        Commit();
        Report.RunModal(Report::"Repro Saved Values");
        Expect('rows after Cancel', Format(RowCount(Report::"Repro Saved Values")), '0');
        Expect('next run', RunRead(Report::"Repro Saved Values"), '');
    end;
"#,
    );
    assert_flagged_pass(
        &o,
        &[
            "A_RunSavesAValue",
            "B_DeletedObjectOptionsForgetTheValue",
            "C_SavedRunWritesOneRow",
            "E_DeleteRowsNextRunShowsBlank",
            "F_DeleteRowsOfOneReport",
            "H_EditOptionData",
            "I_RowOfAnotherUser",
            "J_RowOfAnotherCompany",
            "K_RowUnderAnotherParameterName",
            // A Cancel writes no row; the next run finds none (the rows of the test before were cleared).
            "L_CancelledRun",
        ],
    );
    // The saved row stands as written: the value comes back as on main, the write itself is assumed.
    assert_flagged_pass(&o, &["D_NoDeleteNextRunShowsTheValue", "G_DeleteAndInsertAgain"]);
}

/// The shapes corpus PR 1462 did not measure: a Report.Run that would read a deleted row, a delete still pending under
/// AutoRollback, and a save the runtime keeps outside the rows (a SaveValues page with a non-Text field) whose report's
/// rows AL changed after. A save outside the measured shapes without any later change to the rows keeps main's result.
#[test]
fn issue_8250_unmeasured_shapes_refuse_and_others_keep_mains_result() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure M_RunAfterDelete()
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-RUN');
        ClearRows(Report::"Repro Saved Values");
        Mode := Mode::Read;
        Commit();
        Report.Run(Report::"Repro Saved Values");
    end;

    // Runs after M, in the same session: report A's values stand in a row M deleted.
    [Test]
    [HandlerFunctions('HandlerA')]
    [TransactionModel(TransactionModel::AutoRollback)]
    procedure N_PendingDeleteUnderAutoRollback()
    var
        ObjectOptions: Record "Object Options";
    begin
        ObjectOptions.SetRange("Object Type", ObjectOptions."Object Type"::Report);
        ObjectOptions.SetRange("Object ID", Report::"Repro Saved Values");
        ObjectOptions.DeleteAll();
        Mode := Mode::Read;
        Report.RunModal(Report::"Repro Saved Values");
    end;

    [Test]
    [HandlerFunctions('HandlerI')]
    procedure P_IntegerSaveKeepsMainsResult()
    begin
        Mode := Mode::Set;
        SetInt := 9;
        Commit();
        Report.RunModal(Report::"Repro Saved Int");
        Mode := Mode::Read;
        SeenInt := 0;
        Commit();
        Report.RunModal(Report::"Repro Saved Int");
        if SeenInt <> 9 then
            Error('shown %1', SeenInt);
    end;
"#,
    );
    assert_refused(&o, &["M_RunAfterDelete", "N_PendingDeleteUnderAutoRollback"]);
    assert_clean_pass(&o, &["P_IntegerSaveKeepsMainsResult"]);
}

/// Issue 8250: a save outside the measured shapes (an Integer variable) keeps its values outside the rows; when AL then
/// changes the Object Options rows, which of them BC's next run shows is not measured, so the run refuses. Its own run:
/// a refused test before it would block it.
#[test]
fn issue_8250_rows_changed_after_an_unmeasured_save_refuse() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerI,HandlerA')]
    procedure O_IntegerSaveThenRowsChanged()
    begin
        Mode := Mode::Set;
        SetInt := 7;
        Commit();
        Report.RunModal(Report::"Repro Saved Int");
        // The runtime's own row for report A is no change by AL; deleting it is.
        RunSet(Report::"Repro Saved Values", 'OBO-O');
        ClearRows(Report::"Repro Saved Values");
        Mode := Mode::Read;
        Report.RunModal(Report::"Repro Saved Int");
    end;
"#,
    );
    assert_refused(&o, &["O_IntegerSaveThenRowsChanged"]);
}

/// The rollback that ends a test under TestIsolation = Function puts the store back, without the row a save of the test
/// wrote: no change by AL, so the next test keeps main's result (the stored value).
#[test]
fn issue_8250_isolation_rollback_keeps_mains_result() {
    let o = run_tests_isolated(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure Q1_Save()
    begin
        RunSet(Report::"Repro Saved Values", 'OBO-Q');
    end;

    [Test]
    [HandlerFunctions('HandlerA')]
    procedure Q2_ReadAfterIsolation()
    begin
        Expect('next run', RunRead(Report::"Repro Saved Values"), 'OBO-Q');
    end;
"#,
        TestIsolation::Function,
    );
    assert_flagged_pass(&o, &["Q1_Save"]);
    assert_clean_pass(&o, &["Q2_ReadAfterIsolation"]);
}

/// Issue 8250: an Option Data edit that changes its form is not measured and refuses.
#[test]
fn issue_8250_option_data_edit_of_another_form_refuses() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure R1_EditChangesTheForm()
    var
        ObjectOptions: Record "Object Options";
        OutStr: OutStream;
    begin
        RunSet(Report::"Repro Saved Values", 'OBO-R1');
        if not FindRow(Report::"Repro Saved Values", ObjectOptions) then
            Error('no row');
        ObjectOptions."Option Data".CreateOutStream(OutStr, TextEncoding::UTF8);
        OutStr.WriteText('not the form');
        ObjectOptions.Modify();
        Commit();
        RunRead(Report::"Repro Saved Values");
    end;
"#,
    );
    assert_refused(&o, &["R1_EditChangesTheForm"]);
}
