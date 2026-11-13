//! An action on a list with OnAfterGetRecord and OnAfterGetCurrRecord that filters the view down to no row, then
//! CurrPage.Update (al-runner issue 6945; corpus pages/actions/ActionFilterEmptyView.al, measured on BC 27.5, 28.5 and 29.0).

const SRC: &str = r#"
codeunit 69960 "AE Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin if Entries <> '' then Entries += ';'; Entries += T; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
table 69961 "AE Row" { fields { field(1; Id; Integer) { } field(2; Included; Boolean) { } } keys { key(PK; Id) { } } }
page 69962 "AE List"
{
    PageType = List; SourceTable = "AE Row"; ApplicationArea = All; UsageCategory = None; Editable = false;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Shown; Shown) { } } } }
    actions
    {
        area(Processing)
        {
            action(NoneFalse) { trigger OnAction() begin Log.Add('a' + Format(Rec.Id)); Rec.SetRange(Included, true); CurrPage.Update(false); Log.Add('u' + Format(Rec.Id)); end; }
            action(NoneTrue) { trigger OnAction() begin Log.Add('a' + Format(Rec.Id)); Rec.SetRange(Included, true); CurrPage.Update(true); Log.Add('u' + Format(Rec.Id)); end; }
        }
    }
    trigger OnAfterGetRecord() begin Shown := Rec.Id; Log.Add('R' + Format(Rec.Id)); end;
    trigger OnAfterGetCurrRecord() begin Log.Add('C' + Format(Rec.Id)); end;
    var Log: Codeunit "AE Log"; Shown: Integer;
}
codeunit 69963 C
{
    local procedure Shows(var P: TestPage "AE List"): Text
    var Log: Codeunit "AE Log";
    begin
        exit('[' + P.Id.Value() + '|' + P.Shown.Value() + ']:' + Log.Take());
    end;
    procedure Run(StartRow: Integer; UpdateTrue: Boolean): Text
    var P: TestPage "AE List"; Row: Record "AE Row"; Log: Codeunit "AE Log"; i: Integer; T: Text;
    begin
        for i := 1 to 4 do begin Row.Init(); Row.Id := i; Row.Insert(); end;
        P.OpenView();
        P.First();
        for i := 2 to StartRow do P.Next();
        Log.Take();
        if UpdateTrue then P.NoneTrue.Invoke() else P.NoneFalse.Invoke();
        T := Shows(P);
        T += '|' + Format(P.First()) + Shows(P);
        T += '|' + Format(P.Next()) + Shows(P);
        T += '|' + Format(P.Last()) + Shows(P);
        Row.Get(2);
        T += '|' + Format(P.GoToRecord(Row)) + Shows(P);
        T += '|' + Format(P.GoToKey(3)) + Shows(P);
        P.Close();
        exit(T);
    end;
    procedure FromFirstFalse(): Text begin exit(Run(1, false)); end;
    procedure FromFirstTrue(): Text begin exit(Run(1, true)); end;
    procedure FromLaterFalse(): Text begin exit(Run(3, false)); end;
    procedure FilteredBefore()
    var P: TestPage "AE List"; Row: Record "AE Row"; i: Integer;
    begin
        for i := 1 to 4 do begin Row.Init(); Row.Id := i; Row.Insert(); end;
        P.OpenView();
        P.Filter.SetFilter(Id, '1..3');
        P.First();
        P.NoneFalse.Invoke();
    end;
}
"#;

// ActionFilterEmptyView.al Action_NoRowsLeft_UpdateFalse_Moves and _GoTo (27.5 and 29.0): `[|]:R1;a1;u1;R2`, then every move
// answers false on the blank row.
#[test]
fn an_action_that_leaves_no_row_shows_a_blank_row() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "FromFirstFalse")),
        "[|]:R1;a1;u1;R2|No[|]:|No[|]:|No[|]:|No[|]:|No[|]:"
    );
}

// Action_NoRowsLeft_UpdateTrue_Moves: the same with CurrPage.Update(true).
#[test]
fn the_same_with_update_true() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "FromFirstTrue")),
        "[|]:R1;a1;u1;R2|No[|]:|No[|]:|No[|]:|No[|]:|No[|]:"
    );
}

// Action_NoRowsLeft_FromLaterRow_UpdateFalse: from row 3 the row read after the update is the row before it.
#[test]
fn from_a_later_row_the_row_before_it_is_read() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "FromLaterFalse")),
        "[|]:R3;a3;u3;R2|No[|]:|No[|]:|No[|]:|No[|]:|No[|]:"
    );
}

// A view that had a filter before the action (the journal shape of al-runner issue 7922) is not pinned: refused, and the reason
// names that condition.
#[test]
fn a_view_filtered_before_the_action_is_refused_by_name() {
    let r = al_testkit::run_in(SRC, "C", "FilteredBefore");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(m)) if m.contains("the view had filters before the action")),
        "{r:?}"
    );
}
