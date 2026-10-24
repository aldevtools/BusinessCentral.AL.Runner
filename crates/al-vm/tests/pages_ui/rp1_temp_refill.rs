//! A Worksheet over a temporary source that empties and refills Rec (Reset, DeleteAll, insert numbered from 1, FindFirst) from
//! a filter field's OnValidate and from an action (corpus pages/lt-outside-worksheet/RmTempRefill.al, al-runner issues 5779 and
//! 5780): the page then shows the new rows, from the row the refill left Rec on, wherever the client stood.

use al_testkit::text;

const SRC: &str = r#"
table 69950 "TR Row" { fields { field(1; "Entry No."; Integer) { } field(2; "Group Code"; Code[10]) { } field(3; Name; Text[50]) { } } keys { key(PK; "Entry No.") { } } }
codeunit 69951 "TR Log"
{
    SingleInstance = true;
    var E: Text;
    procedure Add(S: Text) begin E += S + ';'; end;
    procedure Take(): Text var R: Text; begin R := E; E := ''; exit(R); end;
}
page 69952 "TR Ws"
{
    PageType = Worksheet; SourceTable = "TR Row"; SourceTableTemporary = true;
    InsertAllowed = false; ModifyAllowed = false; DeleteAllowed = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) {
        group(F) { field(GroupFilter; GroupFilter) { trigger OnValidate() begin Refill(); end; } }
        repeater(Lines) { Editable = false; field(Name; NameText) { } } } }
    actions { area(Processing) {
        action(Recalculate) { trigger OnAction() begin Refill(); end; }
        action(RefillLast) { trigger OnAction() begin Refill(); Rec.FindLast(); end; }
        action(RecalculateAndUpdate) { trigger OnAction() begin Refill(); CurrPage.Update(false); end; }
    } }
    trigger OnOpenPage() begin Refill(); end;
    trigger OnAfterGetRecord() var L: Codeunit "TR Log"; begin NameText := Rec.Name; L.Add('R:' + Rec.Name); end;
    trigger OnAfterGetCurrRecord() var L: Codeunit "TR Log"; begin L.Add('C:' + Rec.Name); end;
    var GroupFilter: Text; NameText: Text;
    local procedure Refill()
    var EntryNo: Integer; RowNo: Integer;
    begin
        Rec.Reset();
        Rec.DeleteAll();
        for RowNo := 1 to 25 do
            AddRow(EntryNo, 'A', StrSubstNo('A%1', RowNo));
        AddRow(EntryNo, 'B', 'B1');
        AddRow(EntryNo, 'B', 'B2');
        if Rec.FindFirst() then;
    end;
    local procedure AddRow(var EntryNo: Integer; GroupCode: Code[10]; NewName: Text[50])
    begin
        if (GroupFilter <> '') and (GroupFilter <> GroupCode) then
            exit;
        EntryNo += 1;
        Rec.Init();
        Rec."Entry No." := EntryNo;
        Rec."Group Code" := GroupCode;
        Rec.Name := NewName;
        Rec.Insert();
    end;
}
codeunit 69953 C
{
    local procedure GoToRow(var Ws: TestPage "TR Ws"; RowNo: Integer)
    var I: Integer;
    begin
        Ws.First();
        for I := 2 to RowNo do
            Ws.Next();
    end;
    local procedure ReadRows(var Ws: TestPage "TR Ws") Rows: Text
    begin
        if Ws.First() then
            repeat
                if Rows <> '' then
                    Rows += ' ';
                Rows += Ws.Name.Value();
            until not Ws.Next();
    end;
    procedure FilterFromRow(RowNo: Integer): Text
    var Ws: TestPage "TR Ws"; L: Codeunit "TR Log"; S: Text;
    begin
        Ws.OpenEdit();
        GoToRow(Ws, RowNo);
        L.Take();
        Ws.GroupFilter.SetValue('B');
        S := L.Take() + '|' + Ws.Name.Value() + '|';
        S += Format(Ws.Next()) + Ws.Name.Value() + '|' + L.Take() + '|' + ReadRows(Ws);
        exit(S);
    end;
    procedure RefillLast()
    var Ws: TestPage "TR Ws";
    begin
        Ws.OpenEdit();
        Ws.RefillLast.Invoke();
    end;
    procedure ActionFromRow(RowNo: Integer): Text
    var Ws: TestPage "TR Ws"; L: Codeunit "TR Log"; S: Text;
    begin
        Ws.OpenEdit();
        GoToRow(Ws, RowNo);
        L.Take();
        Ws.Recalculate.Invoke();
        S := L.Take() + '|' + Ws.Name.Value() + '|';
        S += Format(Ws.Next()) + Ws.Name.Value() + '|' + L.Take();
        exit(S);
    end;
}
"#;

fn from_row(routine: &str, row: i32) -> String {
    match al_testkit::run_with(SRC, "C", routine, vec![al_builtins::value::Value::Integer(row)]) {
        Ok((v, _)) => text(Ok(v)),
        Err(e) => format!("{e:?}"),
    }
}

#[test]
fn a_filter_validate_from_the_first_row_shows_the_rows_of_the_refill() {
    // RmTempRefill.al:215 RM_Filter_B_FromOpen, :235 RM_Filter_B_OpenEdit.
    assert_eq!(
        from_row("FilterFromRow", 1),
        "R:B1;R:B2;R:B1;R:B1;C:B1;|B1|YesB2|R:B2;R:B2;C:B2;|B1 B2"
    );
}

#[test]
fn a_filter_validate_from_row_23_shows_the_rows_of_the_refill() {
    // RmTempRefill.al:254 RM_Filter_B_FromRow23.
    assert_eq!(
        from_row("FilterFromRow", 23),
        "R:B1;R:B2;R:B1;R:B1;C:B1;|B1|YesB2|R:B2;R:B2;C:B2;|B1 B2"
    );
}

#[test]
fn an_action_refill_from_row_23_shows_the_first_row() {
    // RmTempRefill.al:323 RM_Action_Recalculate_FromRow23, :344 _Next.
    let mut log = String::from("R:A23;R:A1;");
    for i in 1..=20 {
        log += &format!("R:A{i};");
    }
    log += "R:A1;R:A1;C:A1;";
    assert_eq!(from_row("ActionFromRow", 23), format!("{log}|A1|YesA2|R:A2;R:A2;C:A2;"));
}

#[test]
fn a_filter_validate_from_row_2_stays_on_the_row_with_that_key() {
    // RmTempRefill.al:279 RM_Filter_B_FromRow2_Previous: shows B2.
    assert_eq!(
        from_row("FilterFromRow", 2),
        "R:B2;R:B2;R:B1;R:B2;R:B2;C:B2;|B2|NoB2||B1 B2"
    );
}

#[test]
fn an_action_that_leaves_rec_past_the_first_row_is_refused() {
    assert!(matches!(
        al_testkit::run_in(SRC, "C", "RefillLast"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
