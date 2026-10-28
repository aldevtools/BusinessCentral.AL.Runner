//! An action refills a temporary Worksheet of 3 or 25 rows with two rows and leaves Rec on the first (al-runner issue 6029, the
//! reporter's repro green on real BC: a First/Next walk lists only `R1 R2`). The reads are the registered assumption
//! `temp-refill-to-fewer-rows-by-action`: each row of the view once, then the first row twice and OnAfterGetCurrRecord, or once
//! without that trigger. Rec left on another row, and a refill to more rows than a window, are refused.

const PAGE: &str = r#"
page 51495 "TF Ws"
{
    PageType = Worksheet; SourceTable = "TF Row"; SourceTableTemporary = true;
    InsertAllowed = false; ModifyAllowed = false; DeleteAllowed = false;
    layout { area(Content) { repeater(Rows) { Editable = false; field(Name; Rec.Name) { } } } }
    actions
    {
        area(Processing)
        {
            action(Refill) { trigger OnAction() begin Fill(1, 2); end; }
            action(RefillOnLast) { trigger OnAction() begin Fill(1, 2); Rec.FindLast(); end; }
            action(RefillMany) { trigger OnAction() begin Fill(101, 22); end; }
        }
    }
    trigger OnOpenPage() begin Fill(1, RowsAtOpen); end;
    trigger OnAfterGetRecord() begin L.Add('R:' + Rec.Name); end;
    CURRENT
    var L: Codeunit "TF Log"; RowsAtOpen: Integer;
    procedure SetRows(N: Integer) begin RowsAtOpen := N; end;
    local procedure Fill(From: Integer; N: Integer) var I: Integer;
    begin
        Rec.Reset(); Rec.DeleteAll();
        for I := From to From + N - 1 do begin Rec.Init(); Rec."Entry No." := I; Rec.Name := StrSubstNo('R%1', I); Rec.Insert(); end;
        if Rec.FindFirst() then;
    end;
}
table 51494 "TF Row" { fields { field(1; "Entry No."; Integer) { } field(2; Name; Text[50]) { } } keys { key(PK; "Entry No.") { } } }
codeunit 51496 "TF Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(T: Text) begin E += T + ';'; end;
    procedure Take() R: Text begin R := E; E := ''; end;
}
codeunit 51497 C
{
    procedure Run(N: Integer; Which: Integer): Text
    var Ws: TestPage "TF Ws"; P: Page "TF Ws"; L: Codeunit "TF Log"; s: Text; Rows: Text;
    begin
        Ws.Trap(); P.SetRows(N); P.Run();
        Ws.First();
        L.Take();
        case Which of
            0: Ws.Refill.Invoke();
            1: Ws.RefillOnLast.Invoke();
            2: Ws.RefillMany.Invoke();
        end;
        s := L.Take() + '|' + Ws.Name.Value() + '|';
        if Ws.First() then repeat Rows += Ws.Name.Value() + ' '; until not Ws.Next();
        Ws.Close();
        exit(s + Rows);
    end;
    procedure Three(): Text begin exit(Run(3, 0)); end;
    procedure TwentyFive(): Text begin exit(Run(25, 0)); end;
    procedure OnLast(): Text begin exit(Run(3, 1)); end;
    procedure Many(): Text begin exit(Run(25, 2)); end;
}
"#;

fn src(current: bool) -> String {
    PAGE.replace(
        "CURRENT",
        if current {
            "trigger OnAfterGetCurrRecord() begin L.Add('C:' + Rec.Name); end;"
        } else {
            ""
        },
    )
}

fn run(current: bool, routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_in(&src(current), "C", routine)
}

#[test]
fn with_on_after_get_curr_record_the_client_lists_only_the_two_rows() {
    assert_eq!(
        al_testkit::text(run(true, "Three")),
        "R:R1;R:R1;R:R2;R:R1;R:R1;C:R1;|R1|R1 R2 "
    );
    assert_eq!(
        al_testkit::text(run(true, "TwentyFive")),
        "R:R1;R:R1;R:R2;R:R1;R:R1;C:R1;|R1|R1 R2 "
    );
}

#[test]
fn without_it_the_client_lists_only_the_two_rows() {
    assert_eq!(al_testkit::text(run(false, "Three")), "R:R1;R:R1;R:R2;R:R1;|R1|R1 R2 ");
    assert_eq!(
        al_testkit::text(run(false, "TwentyFive")),
        "R:R1;R:R1;R:R2;R:R1;|R1|R1 R2 "
    );
}

#[test]
fn rec_left_on_another_row_or_more_rows_than_a_window_are_refused() {
    for current in [true, false] {
        for routine in ["OnLast", "Many"] {
            let r = run(current, routine);
            assert!(
                matches!(r, Err(al_builtins::native::Signal::Unsupported(_))),
                "{routine} current={current}: {r:?}"
            );
        }
    }
}
