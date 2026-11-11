//! The one-xRec-per-validation-step rule of `pages/validate_xrec.rs` runs only on the pages the corpus pins it on; everywhere
//! else, and for what no probe shows on those pages, the step keeps the former behavior (review of al-runner issue 7403):
//! - an error out of the step leaves the page's xRec as it was before the step, so the earlier unsaved edit is still saved
//!   at Close (the review's repro: an earlier SetValue without a save, then an OnValidate that calls `Error('')`);
//! - a save inside the step on a row with an earlier unsaved edit hands the table's OnModify the row the page read as xRec;
//! - a List with OnAfterGetCurrRecord, and a Card that hosts a part, are not the pinned pages;
//! - a page extension's OnAfterValidate after a save inside the step is refused only when it reads xRec.

const SRC: &str = r#"
table 51960 "VX Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; F0; Integer) { }
        field(3; F1; Integer) { }
        field(4; F2; Integer) { }
        field(5; F3; Integer) { }
        field(6; F6; Integer) { }
        field(7; F7; Integer) { }
    }
    keys { key(PK; "No.") { } }
    trigger OnModify() var L: Codeunit "VX Log"; begin L.Add('TMOD r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); end;
}
codeunit 51961 "VX Log"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(Line: Text) begin Log += Line + '; '; end;
    procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
    procedure Row(R: Record "VX Row"): Text begin exit(StrSubstNo('%1%2%3%4%5%6', R.F0, R.F1, R.F2, R.F3, R.F6, R.F7)); end;
}
page 51962 "VX List"
{
    PageType = List;
    SourceTable = "VX Row";
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { }
                field(F0; Rec.F0) { trigger OnValidate() begin L.Add('F0 r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); end; }
                field(F1; Rec.F1) { trigger OnValidate() begin L.Add('F1 r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); CurrPage.Update(); end; }
                field(F6; Rec.F6) { trigger OnValidate() begin L.Add('F6 r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); Error('boom'); end; }
                field(F7; Rec.F7) { trigger OnValidate() begin L.Add('F7 r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); Error(''); end; }
            }
        }
    }
    var L: Codeunit "VX Log";
}
page 51963 "VX List AGCR"
{
    PageType = List;
    SourceTable = "VX Row";
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("No."; Rec."No.") { }
                field(F3; Rec.F3) { trigger OnValidate() begin L.Add('F3 r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); CurrPage.Update(false); end; }
            }
        }
    }
    trigger OnAfterGetCurrRecord() begin L.Add('AGCR r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); end;
    var L: Codeunit "VX Log";
}
page 51964 "VX Part"
{
    PageType = ListPart;
    SourceTable = "VX Row";
    layout { area(Content) { repeater(Rows) { field("No."; Rec."No.") { } } } }
}
page 51965 "VX Card Host"
{
    PageType = Card;
    SourceTable = "VX Row";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(F0; Rec.F0) { trigger OnValidate() begin L.Add('F0 r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); end; }
            part(Lines; "VX Part") { }
        }
    }
    var L: Codeunit "VX Log";
}
page 51966 "VX Doc"
{
    PageType = Document;
    SourceTable = "VX Row";
    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { }
            field(F1; Rec.F1) { trigger OnValidate() begin L.Add('F1 r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); CurrPage.Update(); end; }
            field(F2; Rec.F2) { trigger OnValidate() begin L.Add('F2 r=' + L.Row(Rec) + ' x=' + L.Row(xRec)); CurrPage.Update(); end; }
        }
    }
    var L: Codeunit "VX Log";
}
pageextension 51967 "VX Doc X" extends "VX Doc"
{
    layout
    {
        modify(F1) { trigger OnAfterValidate() begin L.Add('e1 r=' + Format(Rec.F1)); end; }
        modify(F2) { trigger OnAfterValidate() begin Note(); end; }
    }
    local procedure Note() begin L.Add('e2 x=' + Format(xRec.F2)); end;
    var L: Codeunit "VX Log";
}
codeunit 51968 "VX Test"
{
    var L: Codeunit "VX Log";

    local procedure Seed()
    var R: Record "VX Row";
    begin
        R.DeleteAll();
        R."No." := 'R1'; R.F0 := 1; R.F1 := 1; R.F2 := 1; R.F3 := 1; R.F6 := 1; R.F7 := 1; R.Insert();
        Commit();
        L.Take();
    end;

    local procedure Stored(): Text var R: Record "VX Row"; begin R.Get('R1'); exit(L.Row(R)); end;

    procedure QuietErrorThenClose(): Text
    var P: TestPage "VX List";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('R1');
        P.F0.SetValue(5); L.Add('|');
        P.F7.SetValue(9); L.Add('|');
        P.Close();
        exit(L.Take() + '|' + Stored());
    end;

    procedure ErrorThenSameValueUpdate(): Text
    var P: TestPage "VX List";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('R1');
        P.F0.SetValue(5); L.Add('|');
        asserterror P.F6.SetValue(9); L.Add('|');
        P.F1.SetValue(1); L.Add('|');
        P.Close();
        exit(L.Take() + '|' + Stored());
    end;

    procedure UnsavedEditThenSave(): Text
    var P: TestPage "VX List";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('R1');
        P.F0.SetValue(4); L.Add('|');
        P.F1.SetValue(5); L.Add('|');
        P.Close();
        exit(L.Take() + '|' + Stored());
    end;

    procedure ListWithAgcrUpdateFalse(): Text
    var P: TestPage "VX List AGCR";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('R1'); L.Take();
        P.F3.SetValue(5); L.Add('|');
        P.Close();
        exit(L.Take() + '|' + Stored());
    end;

    procedure CardWithPartSetTwice(): Text
    var P: TestPage "VX Card Host";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('R1');
        P.F0.SetValue(5); L.Add('|');
        P.F0.SetValue(6); L.Add('|');
        P.Close();
        exit(L.Take() + '|' + Stored());
    end;

    procedure DocExtNotReadingXRec(): Text
    var P: TestPage "VX Doc";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('R1');
        P.F1.SetValue(5); L.Add('|');
        P.Close();
        exit(L.Take() + '|' + Stored());
    end;

    procedure DocExtReadingXRec(): Text
    var P: TestPage "VX Doc";
    begin
        Seed(); P.OpenEdit(); P.GoToKey('R1');
        P.F2.SetValue(5); L.Add('|');
        P.Close();
        exit(L.Take() + '|' + Stored());
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "VX Test", routine))
}

#[test]
fn an_error_out_of_the_step_keeps_the_earlier_unsaved_edit_for_close() {
    // Inside the step the page trigger sees Rec before the step (Y7403XRecAfterCurrPageUpdate.al:529-543); after the error the
    // page holds the row it read as xRec again, so Close still saves the earlier edit.
    assert_eq!(
        run("QuietErrorThenClose"),
        "F0 r=511111 x=111111; |; F7 r=511119 x=511111; |; TMOD r=511111 x=111111; |511111"
    );
    assert_eq!(
        run("ErrorThenSameValueUpdate"),
        "F0 r=511111 x=111111; |; F6 r=511191 x=511111; |; F1 r=511111 x=111111; TMOD r=511111 x=111111; |; |511111"
    );
}

#[test]
fn a_save_inside_the_step_hands_the_table_the_row_the_page_read_as_xrec() {
    // The page trigger sees Rec before the step (Y7403XRecAfterCurrPageUpdate.al:529-543); the save keeps the former xRec.
    assert_eq!(
        run("UnsavedEditThenSave"),
        "F0 r=411111 x=111111; |; F1 r=451111 x=411111; TMOD r=451111 x=111111; |; |451111"
    );
}

#[test]
fn a_list_with_on_after_get_curr_record_keeps_the_former_update_false() {
    assert_eq!(
        run("ListWithAgcrUpdateFalse"),
        "F3 r=111511 x=111111; AGCR r=111511 x=111511; |; |111111"
    );
}

#[test]
fn a_card_that_hosts_a_part_keeps_the_former_xrec() {
    assert_eq!(
        run("CardWithPartSetTwice"),
        "F0 r=511111 x=111111; |; F0 r=611111 x=111111; |; TMOD r=611111 x=111111; |611111"
    );
}

#[test]
fn an_extension_on_after_validate_that_does_not_read_xrec_runs_after_a_save_in_the_step() {
    assert_eq!(
        run("DocExtNotReadingXRec"),
        "F1 r=151111 x=111111; TMOD r=151111 x=111111; e1 r=5; |; |151111"
    );
}

#[test]
fn an_extension_on_after_validate_that_reads_xrec_after_a_save_in_the_step_is_refused() {
    let r = al_testkit::run_in(SRC, "VX Test", "DocExtReadingXRec");
    assert!(matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
