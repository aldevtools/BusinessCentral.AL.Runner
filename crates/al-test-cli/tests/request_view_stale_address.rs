//! A request page shows the filters of a `SetTableView` only for the record that got them. The views were keyed by the record's
//! address, which a later record can reuse once the first is gone: a later run without `SetTableView` then read a stale view
//! (its handler's `GetFilter` gave the old filter, or was refused). The outcome hung on the allocator, so the corpus case
//! report/lt-report-requestpage/RptFilters.al:510 RequestFilterFields_Handler_SetsFilters passed on one CI machine and not on
//! another. The two tests are the corpus's (RptFilters.al:510 and :594), the SetTableView one first so that its record is
//! freed before the plain run allocates its own.

use al_test_cli::report::Outcome;

const SRC: &str = r#"
table 91380 "LT ZV Row"
{
    fields { field(1; Id; Integer) { } field(2; Grp; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
report 91381 "LT ZV ReqFilters"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Row; "LT ZV Row")
        {
            RequestFilterFields = Grp, Id;
            DataItemTableView = sorting(Id);
            trigger OnPreDataItem()
            begin
                L.Add('pre filters=[' + Row.GetFilters() + '] grp=[' + Row.GetFilter(Grp) + '] id=[' + Row.GetFilter(Id) + ']');
            end;
            trigger OnAfterGetRecord()
            begin
                L.Add('get ' + Format(Row.Id));
            end;
        }
    }
    requestpage { layout { area(Content) { field(NoteCtl; NoteVal) { ApplicationArea = All; } } } }
    var
        NoteVal: Text[30];
        L: Codeunit "LT ZV Log";
}
codeunit 91382 "LT ZV Log"
{
    SingleInstance = true;
    var
        Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
codeunit 91383 T
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        L: Codeunit "LT ZV Log";

    local procedure Seed()
    var
        Row: Record "LT ZV Row";
        I: Integer;
    begin
        Row.DeleteAll();
        for I := 1 to 4 do begin
            Row.Id := I;
            Row.Grp := 1 + ((I - 1) div 2);
            Row.Insert();
        end;
        L.Take();
        Commit();
    end;

    local procedure Check(Expected: Text)
    var
        Got: Text;
    begin
        Got := L.Take();
        if Got <> Expected then
            Error('log: %1', Got);
    end;

    [Test]
    [HandlerFunctions('ReqFiltersOk')]
    procedure A_SetTableView_AndRequestFilter_Combined()
    var
        Rpt: Report "LT ZV ReqFilters";
        Row: Record "LT ZV Row";
    begin
        Seed();
        Row.SetRange(Id, 1, 3);
        Rpt.SetTableView(Row);
        Rpt.Run();
        Check('handler grp=[] id=[1..3];handler after grp=[2] id=[1..3];pre filters=[Id: >=3, Grp: 2] grp=[2] id=[>=3];get 3;get 4;');
    end;

    [Test]
    [HandlerFunctions('ReqFiltersOk')]
    procedure B_RequestFilterFields_Handler_SetsFilters()
    begin
        Seed();
        Report.Run(Report::"LT ZV ReqFilters");
        Check('handler grp=[] id=[];handler after grp=[2] id=[>=3];pre filters=[Id: >=3, Grp: 2] grp=[2] id=[>=3];get 3;get 4;');
    end;

    // The pair many times over: each run frees its data item record, and a later run's record is likely to get a freed address.
    [Test]
    [HandlerFunctions('ReqFiltersOk')]
    procedure C_Interleaved_ManyTimes()
    var
        I: Integer;
    begin
        for I := 1 to 200 do begin
            A_SetTableView_AndRequestFilter_Combined();
            B_RequestFilterFields_Handler_SetsFilters();
        end;
    end;

    [RequestPageHandler]
    procedure ReqFiltersOk(var RP: TestRequestPage "LT ZV ReqFilters")
    begin
        L.Add('handler grp=[' + RP.Row.GetFilter(Grp) + '] id=[' + RP.Row.GetFilter(Id) + ']');
        RP.Row.SetFilter(Grp, '2');
        RP.Row.SetFilter(Id, '>=3');
        L.Add('handler after grp=[' + RP.Row.GetFilter(Grp) + '] id=[' + RP.Row.GetFilter(Id) + ']');
        RP.OK().Invoke();
    end;
}
"#;

#[test]
fn a_request_page_run_without_set_table_view_reads_no_earlier_view() {
    let o = al_testkit::outcomes_in(SRC, "T");
    assert_eq!(o.len(), 3, "{o:?}");
    for (name, outcome) in &o {
        assert_eq!(*outcome, Outcome::Pass, "{name}: {o:?}");
    }
}

/// The cause, shown without depending on the allocator: a run's SetTableView view must not outlive the run's record. Before the
/// fix every run left its entry behind (one per run, never removed), so a later record at a reused address found it.
#[test]
fn a_set_table_view_entry_does_not_outlive_its_record() {
    const LOOP: &str = r#"
table 91390 "LT ZW Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
report 91391 "LT ZW Rpt"
{
    ProcessingOnly = true;
    UseRequestPage = false;
    dataset { dataitem(Row; "LT ZW Row") { } }
}
codeunit 91392 C
{
    procedure RunOnce()
    var
        Rpt: Report "LT ZW Rpt";
        Row: Record "LT ZW Row";
    begin
        Row.SetRange(Id, 1, 3);
        Rpt.SetTableView(Row);
        Rpt.Run();
    end;
}
"#;
    let mut b = al_testkit::Built::from_source(LOOP);
    for _ in 0..50 {
        b.run_in("C", "RunOnce").unwrap();
    }
    let n = b.session.set_view_filter_entries();
    assert!(
        n <= 1,
        "{n} SetTableView entries remain after 50 runs whose records are gone"
    );
}
