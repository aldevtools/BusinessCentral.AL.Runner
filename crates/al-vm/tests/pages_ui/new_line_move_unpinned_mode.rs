//! A move off the line `New()` started, in shapes next to the pins of testpage/NewRowThenMove.al that the corpus does not pin
//! (pages/leave_new_line.rs `off_new_line_pinned`): they are refused before anything changes.

const SRC: &str = r#"
table 69990 "NLM Row"
{
    fields { field(1; "Code"; Code[20]) { } field(2; Description; Text[50]) { } field(3; Formula; Text[100]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}
codeunit 69991 "NLM Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(Entry: Text) begin Entries += Entry + ' '; end;
    procedure Take() Result: Text begin Result := Entries; Entries := ''; end;
}
page 69992 "NLM List"
{
    PageType = List; SourceTable = "NLM Row"; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Code"; Rec."Code") { }
                field(Description; Rec.Description) { }
                field(FormulaText; FormulaText)
                {
                    trigger OnValidate()
                    begin
                        Rec.Formula := CopyStr(FormulaText, 1, MaxStrLen(Rec.Formula));
                        CurrPage.Update(true);
                    end;
                }
            }
        }
    }
    trigger OnNewRecord(BelowxRec: Boolean) begin FormulaText := ''; Log.Add('N'); end;
    trigger OnAfterGetRecord() begin FormulaText := Rec.Formula; Log.Add('R:' + Rec."Code"); end;
    trigger OnAfterGetCurrRecord() begin FormulaText := Rec.Formula; Log.Add('C:' + Rec."Code"); end;
    var Log: Codeunit "NLM Log"; FormulaText: Text;
}
codeunit 69993 C
{
    var Log: Codeunit "NLM Log";

    local procedure Seed(Count: Integer)
    var Row: Record "NLM Row"; i: Integer;
    begin
        Row.DeleteAll();
        for i := 1 to Count do begin
            Row.Init(); Row."Code" := 'S' + Format(i); Row.Formula := Format(i); Row.Insert();
        end;
    end;

    procedure ViewNewFirst(): Text
    var P: TestPage "NLM List";
    begin
        Seed(2); P.OpenView(); P.Last(); P.New(); Log.Take();
        exit(Format(P.First()) + '|' + Log.Take());
    end;

    procedure KeyTypedNewSavesFirst(): Text
    var P: TestPage "NLM List";
    begin
        Seed(0); P.OpenNew(); P."Code".SetValue('F1'); P.New(); Log.Take();
        exit(Format(P.First()) + '|' + Log.Take());
    end;

    procedure UpdateSavedNewFirst(): Text
    var P: TestPage "NLM List";
    begin
        Seed(0); P.OpenNew(); P."Code".SetValue('F1'); P.FormulaText.SetValue('2+4'); P.New(); Log.Take();
        exit(Format(P.First()) + '|' + Log.Take());
    end;
}
"#;

fn refused(routine: &str) {
    let r = al_testkit::run_in(SRC, "C", routine);
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))),
        "{routine}: {r:?}"
    );
}

/// OpenView is neither the OpenEdit of :540/:663 nor the OpenNew of :444/:581 (testpage/NewRowThenMove.al): not pinned.
#[test]
fn first_off_the_new_line_of_a_list_opened_with_open_view_is_refused() {
    refused("ViewNewFirst");
}

/// :444 List_TypedRow_New_First pins a typed row CurrPage.Update(true) saved before New(); a row New() saves itself is not pinned.
#[test]
fn first_after_new_saved_the_typed_row_itself_is_refused() {
    refused("KeyTypedNewSavesFirst");
}

/// The pinned shape itself (:444 List_TypedRow_New_First `Yes`, `R:FORMULA1 R:FORMULA1 C:FORMULA1`) still runs.
#[test]
fn first_after_update_true_saved_the_typed_row_runs_the_pinned_move() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "UpdateSavedNewFirst")),
        "Yes|R:F1 R:F1 C:F1 "
    );
}
