//! A list whose control's OnValidate changes the page's filters so that the row it stands on leaves the view, then calls
//! CurrPage.Update: which row the list shows then is pinned only after an action (pages/lt-ai-list-order/AiListOrderTests.al:335
//! DropCurrent_RowsAfter_Plain). Refused. Tests-SCM 137831 BinContentsPage_CanSwitchLocationFilterWithoutRenamingRecord (Bin
//! Contents, page 7374) reached it: First was served from the rows read before the filter changed.

const SRC: &str = r#"
table 69960 "UpdAw Line" {
    fields { field(1; Loc; Code[10]) { } field(2; No; Integer) { } }
    keys { key(PK; Loc, No) { Clustered = true; } }
}
page 69961 "UpdAw List"
{
    PageType = Worksheet; SourceTable = "UpdAw Line"; ApplicationArea = All; UsageCategory = None; InsertAllowed = false;
    layout {
        area(Content) {
            field(LocCtl; LocFilter) {
                trigger OnValidate()
                begin
                    Rec.FilterGroup := 2;
                    Rec.SetRange(Loc, LocFilter);
                    Rec.FilterGroup := 0;
                    CurrPage.Update(not IsNullGuid(Rec.SystemId));
                end;
            }
            repeater(G) { field(NoCtl; Rec.No) { } field(RowLocCtl; Rec.Loc) { } }
        }
    }
    var LocFilter: Code[10]; Reads: Integer;
    trigger OnAfterGetRecord()
    begin
        Reads += 1;
    end;
    trigger OnOpenPage()
    begin
        Rec.FilterGroup := 2;
        Rec.SetFilter(Loc, 'A|B');
        Rec.FilterGroup := 0;
    end;
}
codeunit 69962 C
{
    procedure Switch(): Text
    var
        R: Record "UpdAw Line";
        P: TestPage "UpdAw List";
        T: Text;
    begin
        R.Loc := 'A';
        R.No := 1;
        R.Insert();
        P.OpenView();
        P.LocCtl.SetValue('B');
        T := Format(P.First());
        R.FindFirst();
        exit(T + '/' + R.Loc + '/' + Format(R.Count()));
    end;
}
"#;

#[test]
fn update_after_a_validate_whose_filter_left_the_row_is_refused() {
    match al_testkit::run_in(SRC, "C", "Switch") {
        Err(al_builtins::native::Signal::Unsupported(m)) => assert!(m.contains("filters left the row"), "{m}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
