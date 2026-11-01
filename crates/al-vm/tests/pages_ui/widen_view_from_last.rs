//! An action widens the view of a list that stands on its last row (pages/actions.rs `read_list_again_from_row_as`; al-runner
//! issue 6469). pages/lt-y15/Y15ActionWidensView.al:322-423 pins how many rows the refresh reads: 25 for 21 rows or more.

const SRC: &str = r#"
codeunit 69960 "WV Log"
{
    SingleInstance = true;
    var Entries: Text; Count: Integer;
    procedure Add(T: Text) begin Count += 1; Entries += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Format(Count) + ':' + Entries; Entries := ''; Count := 0; exit(R); end;
}
table 69961 "WV Bin" { fields { field(1; "Code"; Code[20]) { } field(2; Room; Integer) { } } keys { key(PK; "Code") { Clustered = true; } } }
page 69962 "WV Bins"
{
    PageType = List; SourceTable = "WV Bin"; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field("Code"; Rec."Code") { } } } }
    actions { area(Processing) { action(ShowAll) { trigger OnAction() begin Rec.SetRange(Room); CurrPage.Update(false); end; } } }
    trigger OnOpenPage() begin Rec.SetFilter(Room, '>0'); end;
    trigger OnAfterGetRecord() var Log: Codeunit "WV Log"; begin Log.Add(Rec."Code"); end;
}
codeunit 69963 C
{
    procedure Widen(Total: Integer): Text
    var P: TestPage "WV Bins"; B: Record "WV Bin"; Log: Codeunit "WV Log"; I: Integer;
    begin
        for I := 1 to Total do begin
            B.Init(); B."Code" := 'A' + Format(I, 2, '<Integer,2><Filler Character,0>');
            if I <= 2 then B.Room := 6;
            B.Insert();
        end;
        P.OpenView();
        P.Last();
        Log.Take();
        P.ShowAll.Invoke();
        exit(Log.Take());
    end;
}
"#;

fn widen(total: i32) -> String {
    let mut b = al_testkit::Built::from_source(SRC);
    let (oi, ri) = b.routine_in("C", "Widen");
    let (v, _) = b.call(oi, ri, vec![al_builtins::value::Value::Integer(total)]).unwrap();
    al_testkit::text(Ok(v))
}

/// Y15ActionWidensView.al:365-418: the action reads 25 rows for a view of 21, 25 and 40 rows, the first A02 and the last A01.
#[test]
fn the_refresh_reads_one_window_and_the_way_back() {
    for total in [21, 25, 40] {
        let log = widen(total);
        assert!(log.starts_with("25:A02;"), "{total} rows: {log}");
        assert!(log.ends_with(";A01;"), "{total} rows: {log}");
    }
}
