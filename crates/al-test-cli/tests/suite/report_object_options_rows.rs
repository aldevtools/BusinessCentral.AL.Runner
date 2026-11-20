//! al-runner issue 8250: a request page with SaveValues keeps what an OK confirmed in an "Object Options" row, so deleting the
//! report's rows makes the next run show a blank field. Pinned by corpus tests/al-language/reports/
//! ReportObjectOptionsSaveValues.al, in the shapes it measures; the others keep main's result or refuse.
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

report 51325 "Repro Saved Del"
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

    trigger OnPreReport()
    var
        ObjectOptions: Record "Object Options";
    begin
        ObjectOptions.SetRange("Object Type", ObjectOptions."Object Type"::Report);
        ObjectOptions.SetRange("Object ID", Report::"Repro Saved Del");
        ObjectOptions.DeleteAll();
    end;
}

report 51326 "Repro Outer"
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

    trigger OnPreReport()
    begin
        Report.RunModal(Report::"Repro Saved B");
    end;
}

report 51327 "Repro Two Texts"
{
    ProcessingOnly = true;
    UsageCategory = None;
    requestpage
    {
        SaveValues = true;
        layout
        {
            area(content)
            {
                field("Some Filter"; SomeFilter) { ApplicationArea = All; }
                field("Other Filter"; OtherFilter) { ApplicationArea = All; }
            }
        }
    }
    var
        SomeFilter: Text;
        OtherFilter: Text;
}

report 51328 "Repro Data Item"
{
    ProcessingOnly = true;
    UsageCategory = None;
    dataset
    {
        dataitem(Num; Integer)
        {
            DataItemTableView = where(Number = const(1));
        }
    }
    requestpage
    {
        SaveValues = true;
        layout { area(content) { field("Some Filter"; SomeFilter) { ApplicationArea = All; } } }
    }
    var
        SomeFilter: Text;
}

report 51329 "Repro Extra Global"
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
        Other: Integer;
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
    procedure HandlerDel(var RequestPage: TestRequestPage "Repro Saved Del")
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
    procedure HandlerOuter(var RequestPage: TestRequestPage "Repro Outer")
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
    procedure HandlerTwo(var RequestPage: TestRequestPage "Repro Two Texts")
    begin
        if Mode = Mode::Set then begin
            RequestPage."Some Filter".SetValue(SetTxt);
            RequestPage."Other Filter".SetValue(SetTxt);
            RequestPage.OK().Invoke();
        end else begin
            SeenTxt := RequestPage."Some Filter".Value();
            RequestPage.Cancel().Invoke();
        end;
    end;

    [RequestPageHandler]
    procedure HandlerItem(var RequestPage: TestRequestPage "Repro Data Item")
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
    procedure HandlerE(var RequestPage: TestRequestPage "Repro Extra Global")
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

fn assert_clean_pass(o: &[(String, Outcome, Vec<Assumption>)], names: &[&str]) {
    for n in names {
        let (_, r, h) = outcome(o, n);
        assert_eq!(*r, Outcome::Pass, "{n}: {r:?}");
        assert!(h.is_empty(), "{n}: {h:?}");
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

/// The reporter's repro (issue 8250, BC 29 sandbox: both pass) and the cells of ReportObjectOptionsSaveValues.al.
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
    assert_clean_pass(
        &o,
        &[
            "A_RunSavesAValue",
            "B_DeletedObjectOptionsForgetTheValue",
            "C_SavedRunWritesOneRow",
            "D_NoDeleteNextRunShowsTheValue",
            "E_DeleteRowsNextRunShowsBlank",
            "F_DeleteRowsOfOneReport",
            "G_DeleteAndInsertAgain",
            "I_RowOfAnotherUser",
            "J_RowOfAnotherCompany",
            "K_RowUnderAnotherParameterName",
            // A Cancel writes no row; the next run finds none (the rows of the test before were cleared).
            "L_CancelledRun",
        ],
    );
}

/// The shapes ReportObjectOptionsSaveValues.al does not measure: a Report.Run that would read a deleted row, a delete still pending under
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
    assert_clean_pass(&o, &["Q1_Save", "Q2_ReadAfterIsolation"]);
}

/// Issue 8250: Option Data written over without a read (the Option Data the runtime wrote is gone from the row): the next
/// run's read refuses.
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

/// Issue 8250: only that Option Data holds the value is pinned (ReportObjectOptionsSaveValues.al:197), not its text, so every
/// look at the Option Data the runtime wrote refuses: a read, and the edit of :294, which reads it first.
#[test]
fn issue_8250_option_data_read_refuses() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure C_OptionDataRead()
    var
        ObjectOptions: Record "Object Options";
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-ABC');
        FindRow(Report::"Repro Saved Values", ObjectOptions);
        Expect('Option Data', Format(OptionDataText(ObjectOptions).Contains('OBO-ABC')), 'Yes');
    end;

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

"#,
    );
    assert_refused(&o, &["C_OptionDataRead"]);
    assert_refused(&o, &["H_EditOptionData"]);
}

/// Review of PR 8511, HIGH-1: the row the runtime writes for one report's save does not hide a change AL made before to the
/// rows of another report. Deleting A's rows then saving B: the next run of A finds no row and shows blank (:232, :259).
#[test]
fn issue_8250_save_of_another_report_keeps_a_delete() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA,HandlerB')]
    procedure C2_DeleteThenSaveOther()
    begin
        ClearRows(Report::"Repro Saved Values");
        ClearRows(Report::"Repro Saved B");
        RunSet(Report::"Repro Saved Values", 'OBO-A1');
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved B", 'OBO-B1');
        Expect('A', RunRead(Report::"Repro Saved Values"), '');
        Expect('B', RunRead(Report::"Repro Saved B"), 'OBO-B1');
    end;
"#,
    );
    assert_clean_pass(&o, &["C2_DeleteThenSaveOther"]);
}

/// Review of PR 8511, HIGH-1: a change to the rows after a save kept outside them still refuses after a later save of another
/// report writes its row.
#[test]
fn issue_8250_save_of_another_report_keeps_a_refusal() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerI,HandlerB')]
    procedure C4_UnmirroredThenChangeThenSaveOther()
    var
        ObjectOptions: Record "Object Options";
    begin
        Mode := Mode::Set;
        SetInt := 5;
        Commit();
        Report.RunModal(Report::"Repro Saved Int");
        ObjectOptions."Object Type" := ObjectOptions."Object Type"::Report;
        ObjectOptions."Object ID" := 99999;
        ObjectOptions."Parameter Name" := 'X';
        ObjectOptions.Insert();
        ObjectOptions.Delete();
        Commit();
        RunSet(Report::"Repro Saved B", 'OBO-B');
        Mode := Mode::Read;
        Commit();
        Report.RunModal(Report::"Repro Saved Int");
    end;
"#,
    );
    assert_refused(&o, &["C4_UnmirroredThenChangeThenSaveOther"]);
}

/// Review of PR 8511, HIGH-2: the row has a SystemId, as every stored row; the stamps the probe did not read refuse.
#[test]
fn issue_8250_written_row_has_a_system_id() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure S01_SystemId()
    var
        ObjectOptions: Record "Object Options";
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-S');
        FindRow(Report::"Repro Saved Values", ObjectOptions);
        Expect('SystemId null', Format(IsNullGuid(ObjectOptions.SystemId)), 'No');
    end;
"#,
    );
    assert_clean_pass(&o, &["S01_SystemId"]);
}

/// Review of PR 8511, HIGH-2: SystemCreatedAt of the row the runtime wrote is not pinned.
#[test]
fn issue_8250_written_row_stamps_refuse() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure S01_CreatedAt()
    var
        ObjectOptions: Record "Object Options";
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-S');
        FindRow(Report::"Repro Saved Values", ObjectOptions);
        Expect('SystemCreatedAt', Format(ObjectOptions.SystemCreatedAt = 0DT), 'No');
    end;
"#,
    );
    assert_refused(&o, &["S01_CreatedAt"]);
}

/// Review of PR 8511, MED-3: a request page of a report run inside the run of another does not lose the outer save. The
/// outer report has a trigger, so its save is kept outside the rows; once the inner report's row is deleted, its next read
/// refuses instead of answering the old value.
#[test]
fn issue_8250_nested_run_keeps_the_outer_save() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerOuter,HandlerB')]
    procedure S08_Nested()
    begin
        ClearRows(Report::"Repro Outer");
        ClearRows(Report::"Repro Saved B");
        RunSet(Report::"Repro Outer", 'OBO-OUT');
        ClearRows(Report::"Repro Outer");
        ClearRows(Report::"Repro Saved B");
        Expect('outer', RunRead(Report::"Repro Outer"), '');
    end;
"#,
    );
    let (_, r, h) = outcome(&o, "S08_Nested");
    assert!(
        matches!(r, Outcome::Pass) && h.is_empty()
            || matches!(r, Outcome::NotRun(why) if why.contains("Object Options")),
        "{r:?} {h:?}"
    );
}

/// Saves outside the measured shapes keep main's result: no row, and the next run shows the stored value. A report variable's
/// RunModal (the probe runs `Report.RunModal(Id)`, :111), two saved Text controls (the probe report has one, :33), a report
/// with a data item, and one whose OnPreReport deletes its rows (review of PR 8511, MED-4; the probe reports have neither,
/// :20-40).
#[test]
fn issue_8250_saves_outside_the_measured_shapes_keep_mains_result() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure V_ReportVariable()
    var
        R: Report "Repro Saved Values";
    begin
        ClearRows(Report::"Repro Saved Values");
        Mode := Mode::Set;
        SetTxt := 'OBO-VAR';
        Commit();
        R.RunModal();
        Expect('rows', Format(RowCount(Report::"Repro Saved Values")), '0');
        Expect('next run', RunRead(Report::"Repro Saved Values"), 'OBO-VAR');
    end;

    [Test]
    [HandlerFunctions('HandlerTwo')]
    procedure W_TwoTexts()
    begin
        ClearRows(Report::"Repro Two Texts");
        RunSet(Report::"Repro Two Texts", 'OBO-TWO');
        Expect('rows', Format(RowCount(Report::"Repro Two Texts")), '0');
        Expect('next run', RunRead(Report::"Repro Two Texts"), 'OBO-TWO');
    end;

    [Test]
    [HandlerFunctions('HandlerItem')]
    procedure X_DataItem()
    begin
        ClearRows(Report::"Repro Data Item");
        RunSet(Report::"Repro Data Item", 'OBO-ITEM');
        Expect('rows', Format(RowCount(Report::"Repro Data Item")), '0');
        Expect('next run', RunRead(Report::"Repro Data Item"), 'OBO-ITEM');
    end;

    [Test]
    [HandlerFunctions('HandlerDel')]
    procedure Y_BodyDeletes()
    begin
        ClearRows(Report::"Repro Saved Del");
        RunSet(Report::"Repro Saved Del", 'OBO-DEL');
        Expect('rows', Format(RowCount(Report::"Repro Saved Del")), '0');
        Expect('next run', RunRead(Report::"Repro Saved Del"), 'OBO-DEL');
    end;
"#,
    );
    assert_clean_pass(&o, &["V_ReportVariable", "W_TwoTexts", "X_DataItem", "Y_BodyDeletes"]);
}

/// Review of PR 8511 round 2, A: the probe saves through `Report.RunModal(Id)` only (:111). A save through another overload of
/// the static RunModal keeps main's result: no row, and the next run shows the stored value.
#[test]
fn issue_8250_other_run_modal_overloads_keep_mains_result() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure N01_RunModalIdTrue()
    begin
        ClearRows(Report::"Repro Saved Values");
        Mode := Mode::Set;
        SetTxt := 'OBO-N01';
        Commit();
        Report.RunModal(Report::"Repro Saved Values", true);
        Expect('rows', Format(RowCount(Report::"Repro Saved Values")), '0');
        ClearRows(Report::"Repro Saved Values");
        Expect('next run', RunRead(Report::"Repro Saved Values"), 'OBO-N01');
    end;

    [Test]
    [HandlerFunctions('HandlerA')]
    procedure N02_RunModalIdTrueFalse()
    begin
        ClearRows(Report::"Repro Saved Values");
        Mode := Mode::Set;
        SetTxt := 'OBO-N02';
        Commit();
        Report.RunModal(Report::"Repro Saved Values", true, false);
        Expect('rows', Format(RowCount(Report::"Repro Saved Values")), '0');
        ClearRows(Report::"Repro Saved Values");
        Expect('next run', RunRead(Report::"Repro Saved Values"), 'OBO-N02');
    end;
"#,
    );
    assert_clean_pass(&o, &["N01_RunModalIdTrue", "N02_RunModalIdTrueFalse"]);
}

/// Review of PR 8511 round 2, A: after a measured save and a delete, a read through `Report.RunModal(Id, true)` is not the
/// probe's read (:114-121): it refuses.
#[test]
fn issue_8250_read_by_another_run_modal_overload_refuses() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure N04_ReadByRunModalIdTrue()
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-N04');
        ClearRows(Report::"Repro Saved Values");
        Mode := Mode::Read;
        SeenTxt := 'not read';
        Commit();
        Report.RunModal(Report::"Repro Saved Values", true);
        Expect('next run', SeenTxt, '');
    end;
"#,
    );
    assert_refused(&o, &["N04_ReadByRunModalIdTrue"]);
}

/// Review of PR 8511 round 2, A: the probe's run is not substituted. A run that SubstituteReport swaps for another report keeps
/// main's result: no row for either report.
#[test]
fn issue_8250_substituted_run_keeps_mains_result() {
    let o = run_tests(
        r#"
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Reporting Triggers", 'SubstituteReport', '', false, false)]
    local procedure OnSubstitute(ReportId: Integer; RunMode: Option Normal,ParametersOnly,Execute,Print,SaveAs,RunModal; RequestPageXml: Text; RecordRef: RecordRef; var NewReportId: Integer)
    begin
        if ReportId = Report::"Repro Saved Values" then
            NewReportId := Report::"Repro Saved B";
    end;

    [Test]
    [HandlerFunctions('HandlerB')]
    procedure N10_Substituted()
    begin
        ClearRows(Report::"Repro Saved Values");
        ClearRows(Report::"Repro Saved B");
        RunSet(Report::"Repro Saved Values", 'OBO-N10');
        Expect('rows', Format(RowCount(Report::"Repro Saved Values")), '0');
        Expect('rows of the substitute', Format(RowCount(Report::"Repro Saved B")), '0');
    end;
"#,
    );
    assert_clean_pass(&o, &["N10_Substituted"]);
}

/// Review of PR 8511 round 2, B: the probe never confirms a second OK while a row stands (every test deletes the rows first).
/// The SystemId of the row after such a re-save is not pinned: its read refuses.
#[test]
fn issue_8250_second_save_over_a_row_refuses_its_system_id() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure N08_SecondSaveSystemId()
    var
        ObjectOptions: Record "Object Options";
        First: Guid;
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-N08a');
        FindRow(Report::"Repro Saved Values", ObjectOptions);
        First := ObjectOptions.SystemId;
        RunSet(Report::"Repro Saved Values", 'OBO-N08b');
        FindRow(Report::"Repro Saved Values", ObjectOptions);
        Expect('same SystemId', Format(First = ObjectOptions.SystemId), 'No');
    end;
"#,
    );
    assert_refused(&o, &["N08_SecondSaveSystemId"]);
}

/// Review of PR 8511 round 2, C: the probe reads the row as written (:218) and an exact copy inserted again (:277). A row
/// whose Public Visible AL changed refuses the next read.
#[test]
fn issue_8250_edit_of_another_column_refuses() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerA')]
    procedure N09_PublicVisible()
    var
        ObjectOptions: Record "Object Options";
    begin
        ClearRows(Report::"Repro Saved Values");
        RunSet(Report::"Repro Saved Values", 'OBO-N09');
        FindRow(Report::"Repro Saved Values", ObjectOptions);
        ObjectOptions."Public Visible" := true;
        ObjectOptions.Modify();
        Commit();
        Expect('next run', RunRead(Report::"Repro Saved Values"), 'OBO-N09');
    end;
"#,
    );
    assert_refused(&o, &["N09_PublicVisible"]);
}

/// Review of PR 8511 round 2, L1: the probe report has one global (:37-38). A report with a second global keeps main's result.
#[test]
fn issue_8250_report_with_another_global_keeps_mains_result() {
    let o = run_tests(
        r#"
    [Test]
    [HandlerFunctions('HandlerE')]
    procedure N07_ExtraGlobal()
    begin
        ClearRows(Report::"Repro Extra Global");
        RunSet(Report::"Repro Extra Global", 'OBO-N07');
        Expect('rows', Format(RowCount(Report::"Repro Extra Global")), '0');
        ClearRows(Report::"Repro Extra Global");
        Expect('next run', RunRead(Report::"Repro Extra Global"), 'OBO-N07');
    end;
"#,
    );
    assert_clean_pass(&o, &["N07_ExtraGlobal"]);
}
