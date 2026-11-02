//! A move (Next) after a save the page refused with SaveAbort (ModifyAllowed is false) does not move. The corpus discards the
//! answer (pages/lt-y80-modify-denied/Y80ModifyDenied.al Y80_MD_List_Set_Next_Next: `P.Next();`), so the answer is not pinned:
//! a discarded Next runs, a Next whose answer the code uses is refused.

const SRC: &str = r#"
table 69970 "SA Row"
{
    fields { field(1; Id; Integer) { } field(2; Txt; Text[30]) { } }
    keys { key(PK; Id) { } }
}
page 69971 "SA List"
{
    PageType = List; SourceTable = "SA Row"; ModifyAllowed = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(TxtCtl; Rec.Txt) { } } } }
}
codeunit 69972 C
{
    local procedure Seed()
    var R: Record "SA Row";
    begin
        R.Id := 1; R.Txt := 'a'; R.Insert();
        R.Id := 2; R.Txt := 'b'; R.Insert();
    end;

    procedure Discarded(): Text
    var P: TestPage "SA List";
    begin
        Seed();
        P.OpenEdit();
        P.First();
        P.TxtCtl.SetValue('x');
        P.Next();
        exit(P.IdCtl.Value() + '/' + P.TxtCtl.Value());
    end;

    procedure Used(): Boolean
    var P: TestPage "SA List"; Moved: Boolean;
    begin
        Seed();
        P.OpenEdit();
        P.First();
        P.TxtCtl.SetValue('x');
        Moved := P.Next();
        exit(Moved);
    end;
}
"#;

#[test]
fn a_next_after_a_refused_save_that_discards_its_answer_stays_on_the_row() {
    let mut built = al_testkit::Built::from_source(SRC);
    assert_eq!(al_testkit::text(built.run_in("C", "Discarded")), "1/x");
}

#[test]
fn a_next_after_a_refused_save_whose_answer_is_used_is_refused() {
    let mut built = al_testkit::Built::from_source(SRC);
    let r = built.run_in("C", "Used");
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
