//! A report that carries ObsoleteState, ObsoleteReason and ObsoleteTag runs as one without them (Base Application's
//! "Calculate Subcontracts"; Tests-SCM-Manufacturing refused 34 tests with "report property obsoletereason is not supported yet").

use al_testkit::text;

const SRC: &str = r#"
table 69960 "ROP Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 69961 "ROP Rpt"
{
    ProcessingOnly = true; UseRequestPage = false;
    ObsoleteState = Pending; ObsoleteReason = 'Replaced'; ObsoleteTag = '27.0';
    dataset { dataitem(Item; "ROP Row") { trigger OnAfterGetRecord() begin Hits += 1; end; } }
    var Hits: Integer;
    procedure GetHits(): Integer begin exit(Hits); end;
}
codeunit 69962 C
{
    procedure F(): Text
    var R: Report "ROP Rpt"; Row: Record "ROP Row";
    begin
        Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert();
        R.RunModal();
        exit(Format(R.GetHits()));
    end;
}
"#;

#[test]
fn a_report_with_obsolete_properties_runs() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "F")), "2");
}

/// A data item, a request page, a query column and a query filter that carry ObsoleteState = Pending, ObsoleteReason and
/// ObsoleteTag still run (al-runner issue 6652; corpus pages/lt-x-obsolete-pending/XObsoletePending.al:255-311, :589-612,
/// the tests at :478-566).
const ELEMENTS: &str = r#"
table 69963 "ROP Src" { fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } } keys { key(PK; Id) { } } }
report 69964 "ROP Item Rpt"
{
    ProcessingOnly = true;
    dataset
    {
        dataitem(Item; "ROP Src")
        {
            ObsoleteState = Pending; ObsoleteReason = 'Data item replaced.'; ObsoleteTag = '1.0';
            RequestFilterFields = Id;
            trigger OnAfterGetRecord() begin Log += 'row' + Format(Item.Id) + '/' + Format(Mode) + ';'; end;
        }
    }
    requestpage
    {
        ObsoleteState = Pending; ObsoleteReason = 'Request page replaced.'; ObsoleteTag = '1.0';
        layout { area(Content) { field(ModeField; Mode) { } } }
    }
    var Log: Text; Mode: Integer;
    procedure GetLog(): Text begin exit(Log); end;
}
query 69965 "ROP Query"
{
    elements
    {
        dataitem(Source; "ROP Src")
        {
            column(Id; Id) { ObsoleteState = Pending; ObsoleteReason = 'Column replaced.'; ObsoleteTag = '1.0'; }
            filter(AmountFilter; Amount) { ObsoleteState = Pending; ObsoleteReason = 'Filter replaced.'; ObsoleteTag = '1.0'; }
        }
    }
}
codeunit 69966 C
{
    local procedure Seed()
    var Row: Record "ROP Src";
    begin
        Row.Id := 1; Row.Amount := 1; Row.Insert(); Row.Id := 2; Row.Amount := 2; Row.Insert();
    end;

    procedure Report(): Text
    var R: Report "ROP Item Rpt";
    begin
        Seed();
        R.UseRequestPage(false);
        R.RunModal();
        exit(R.GetLog());
    end;

    procedure Query(): Text
    var Q: Query "ROP Query"; Rows: Text;
    begin
        Seed();
        Q.SetRange(AmountFilter, 2);
        Q.Open();
        while Q.Read() do Rows += Format(Q.Id) + ';';
        exit(Rows);
    end;
}
"#;

#[test]
fn a_report_with_an_obsolete_data_item_and_request_page_runs() {
    assert_eq!(text(al_testkit::run_in(ELEMENTS, "C", "Report")), "row1/0;row2/0;");
}

#[test]
fn a_query_with_an_obsolete_column_and_filter_runs() {
    assert_eq!(text(al_testkit::run_in(ELEMENTS, "C", "Query")), "2;");
}

/// The same on a request page group, field and action, and on a data item, column and request page field a report extension
/// adds: alc compiles them (al-runner issue 6652, corpus writer Y64's compile facts, alc 30 with BC 29 symbols).
const EXT: &str = r#"
table 69967 "ROP Ext Src" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 69968 "ROP Ext Child" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 69969 "ROP Ext Rpt"
{
    ProcessingOnly = true;
    dataset { dataitem(Item; "ROP Ext Src") { trigger OnAfterGetRecord() begin Log += 'row' + Format(Item.Id) + ';'; end; } }
    requestpage
    {
        layout
        {
            area(Content)
            {
                group(Options)
                {
                    ObsoleteState = Pending; ObsoleteReason = 'Group replaced.'; ObsoleteTag = '1.0';
                    field(ModeField; Mode) { ObsoleteState = Pending; ObsoleteReason = 'Field replaced.'; ObsoleteTag = '1.0'; }
                }
            }
        }
        actions
        {
            area(Processing)
            {
                action(Act) { ObsoleteState = Pending; ObsoleteReason = 'Action replaced.'; ObsoleteTag = '1.0'; trigger OnAction() begin end; }
            }
        }
    }
    var Log: Text; Mode: Integer;
    procedure GetLog(): Text begin exit(Log); end;
    procedure AddLog(T: Text) begin Log += T; end;
}
reportextension 69970 "ROP Ext" extends "ROP Ext Rpt"
{
    dataset
    {
        add(Item) { column(ExtCol; Item.Id) { ObsoleteState = Pending; ObsoleteReason = 'Column replaced.'; ObsoleteTag = '1.0'; } }
        addlast(Item)
        {
            dataitem(Child; "ROP Ext Child")
            {
                ObsoleteState = Pending; ObsoleteReason = 'Data item replaced.'; ObsoleteTag = '1.0';
                DataItemLink = Id = field(Id);
                trigger OnAfterGetRecord() begin AddLog('child' + Format(Child.Id) + ';'); end;
            }
        }
    }
    requestpage
    {
        layout { addlast(Options) { field(ExtField; ExtMode) { ObsoleteState = Pending; ObsoleteReason = 'Field replaced.'; ObsoleteTag = '1.0'; } } }
    }
    var ExtMode: Integer;
}
codeunit 69971 C
{
    procedure Report(): Text
    var R: Report "ROP Ext Rpt"; Row: Record "ROP Ext Src"; Child: Record "ROP Ext Child";
    begin
        Row.Id := 1; Row.Insert(); Row.Id := 2; Row.Insert(); Child.Id := 2; Child.Insert();
        R.UseRequestPage(false);
        R.RunModal();
        exit(R.GetLog());
    end;
}
"#;

#[test]
fn obsolete_request_page_controls_and_report_extension_elements_run() {
    assert_eq!(text(al_testkit::run_in(EXT, "C", "Report")), "row1;row2;child2;");
}
