//! A TestPage filter change refills a list in a key other than the primary key (SourceTableView descending, the Customer Ledger
//! Entries shape) to one row, or to more rows than a window holds: the client reads the window from the first row of the view,
//! then that row three more times, then OnAfterGetCurrRecord (corpus master 97c29f86, pages/lt-y17-refill-key/Y17RefillKey.al:341
//! Refill_Desc_OneRow `10:R10;R10;R10;C10;`, :364 Refill_Desc_MoreThanWindow `29:R29;R28;...;R7;R29;R29;R29;C29;`, :311
//! Refill_Desc_ManyThenOne).

const SRC: &str = r#"
table 51520 "RK Entry"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Cust No."; Code[20]) { } field(3; "Doc Type"; Integer) { } }
    keys { key(PK; "Entry No.") { } key(ByCust; "Cust No.") { } }
}
codeunit 51521 "RK Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
page 51522 "RK List"
{
    PageType = List; SourceTable = "RK Entry"; Editable = false;
    SourceTableView = sorting("Cust No.") order(descending);
    layout { area(Content) { repeater(Rows) { field("Entry No."; Rec."Entry No.") { } field("Cust No."; Rec."Cust No.") { } } } }
    trigger OnAfterGetRecord() begin L.Add('R' + Format(Rec."Entry No.")); end;
    trigger OnAfterGetCurrRecord() begin L.Add('C' + Format(Rec."Entry No.")); end;
    var L: Codeunit "RK Log";
}
codeunit 51523 C
{
    local procedure Seed()
    var E: Record "RK Entry"; N: Integer;
    begin
        E.DeleteAll();
        for N := 1 to 30 do begin
            E."Entry No." := N; E."Cust No." := 'C' + Format(100 + N); E."Doc Type" := 2;
            if (N mod 5 = 0) and (N <> 10) then E."Doc Type" := 1;
            E.Insert();
        end;
    end;
    procedure OneRow(): Text
    var P: TestPage "RK List"; L: Codeunit "RK Log"; s: Text;
    begin
        Seed(); P.OpenView(); L.Take();
        P.Filter.SetFilter("Cust No.", 'C110');
        s := P."Entry No.".Value() + ':' + L.Take();
        s += '|' + Format(P.Next()) + '/' + P."Entry No.".Value() + ':' + L.Take();
        s += '|' + Format(P.First()) + '/' + P."Entry No.".Value() + ':' + L.Take();
        P.Close();
        exit(s);
    end;
    procedure Many(): Text
    var P: TestPage "RK List"; L: Codeunit "RK Log";
    begin
        Seed(); P.OpenView(); L.Take();
        P.Filter.SetFilter("Doc Type", '2');
        exit(P."Entry No.".Value() + ':' + L.Take());
    end;
    procedure ManyThenOne(): Text
    var P: TestPage "RK List"; L: Codeunit "RK Log"; s: Text;
    begin
        Seed(); P.OpenView();
        P.Filter.SetFilter("Doc Type", '2'); L.Take();
        P.Filter.SetFilter("Cust No.", 'C110');
        s := P."Entry No.".Value() + ':' + L.Take();
        s += '|' + Format(P.Last()) + '/' + P."Entry No.".Value() + ':' + L.Take();
        exit(s);
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

#[test]
fn a_refill_to_one_row_reads_it_three_times() {
    assert_eq!(run("OneRow"), "10:R10;R10;R10;C10;|No/10:|Yes/10:");
    // Last on the one row reads nothing (Y17RefillKey.al:311 `last` `Yes/10:`).
    assert_eq!(run("ManyThenOne"), "10:R10;R10;R10;C10;|Yes/10:");
}

#[test]
fn a_refill_to_more_rows_than_a_window_reads_the_window_then_the_first_row_three_times() {
    // 25 rows with Doc Type 2, descending: 29, 28, 27, 26, 24, ... The window holds the first 20 of them.
    let window: Vec<i32> = (1..=30).rev().filter(|n| n % 5 != 0 || *n == 10).take(20).collect();
    let mut want = String::from("29:");
    for n in window {
        want += &format!("R{n};");
    }
    want += "R29;R29;R29;C29;";
    assert_eq!(run("Many"), want);
}
