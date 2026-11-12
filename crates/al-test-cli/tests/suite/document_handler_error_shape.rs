//! The close of a Document with hidden parts after its ModalPageHandler raised an error (al-runner issue 7473) runs only in
//! the shape of pages/close-errors/CemDocumentTests.al (crates/al-vm/src/pages/document_handler_error.rs
//! `document_close_pinned`). The pinned shape here closes as the corpus pins it; a page with one more trigger (the reporter's
//! OnAfterGetCurrRecord) and a handler with an unpinned call keep main's refusal.

use al_testrun::Outcome;

const SRC: &str = r#"
table 50500 "D Row"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "Code") { } }
    trigger OnModify() begin if Rec.Name = 'FAILMOD' then Error('table modify refused'); end;
}
table 50501 "D Line" { fields { field(1; DocCode; Code[10]) { } field(2; LineNo; Integer) { } } keys { key(PK; DocCode, LineNo) { } } }
codeunit 50502 "D Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + '; '; end;
    procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end;
}
page 50503 "D Lines" { PageType = ListPart; SourceTable = "D Line"; layout { area(Content) { repeater(R) { field(LineNo; Rec.LineNo) { } } } } }
page 50504 "D Info" { PageType = CardPart; SourceTable = "D Row"; layout { area(Content) { field(Name; Rec.Name) { } } } }
page 50505 "D Doc"
{
    PageType = Document; SourceTable = "D Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(General)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; trigger OnValidate() begin CurrPage.Update(); end; }
            }
            part(Lines; "D Lines") { ApplicationArea = All; SubPageLink = DocCode = field("Code"); }
        }
        area(FactBoxes) { part(Hidden; "D Info") { ApplicationArea = All; Visible = false; SubPageLink = "Code" = field("Code"); } part(Extra; "D Info") { ApplicationArea = All; Visible = ExtraVisible; } }
    }
    var Log: Codeunit "D Log"; ExtraVisible: Boolean;
    trigger OnModifyRecord(): Boolean begin Log.Add('M'); exit(true); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin Log.Add('Q ' + Format(CloseAction)); exit(true); end;
    trigger OnClosePage() begin Log.Add('C'); end;
}
page 50506 "D Doc AGCR"
{
    PageType = Document; SourceTable = "D Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            group(General)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; trigger OnValidate() begin CurrPage.Update(); end; }
            }
            part(Lines; "D Lines") { ApplicationArea = All; SubPageLink = DocCode = field("Code"); }
        }
        area(FactBoxes) { part(Hidden; "D Info") { ApplicationArea = All; Visible = false; SubPageLink = "Code" = field("Code"); } part(Extra; "D Info") { ApplicationArea = All; Visible = ExtraVisible; } }
    }
    var Log: Codeunit "D Log"; ExtraVisible: Boolean;
    trigger OnModifyRecord(): Boolean begin Log.Add('M'); exit(true); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin Log.Add('Q ' + Format(CloseAction)); exit(true); end;
    trigger OnClosePage() begin Log.Add('C'); end;
    trigger OnAfterGetCurrRecord() begin end;
}
codeunit 50507 "D Tests"
{
    Subtype = Test;
    var Log: Codeunit "D Log"; Mode: Integer;
    local procedure Setup(var Row: Record "D Row")
    begin
        Row.DeleteAll(); Row."Code" := 'R1'; Row.Name := 'start'; Row.Insert(); Commit(); Log.Take();
    end;
    {TEST}
    [ModalPageHandler] procedure DocModal(var P: TestPage "D Doc")
    begin
        case Mode of
            7: Error('handler error');
            8: P.Name.SetValue('FAILMOD');
            13: begin P.Next(); Error('handler error'); end;
        end;
    end;
    [ModalPageHandler] procedure AgcrModal(var P: TestPage "D Doc AGCR") begin P.Name.SetValue('FAILMOD'); end;
}
"#;

fn run(test: &str) -> Outcome {
    let o = al_testkit::outcomes_in(&SRC.replace("{TEST}", test), "D Tests");
    o.into_iter().next().expect("one test").1
}

const REFUSAL: &str = "with an unsaved row or a hidden part";

#[test]
fn the_pinned_shape_closes_as_the_corpus_pins() {
    let error = run(
        r#"[Test] [HandlerFunctions('DocModal')] procedure HandlerError() var Row: Record "D Row"; Res: Action; begin Setup(Row); Mode := 7; asserterror Res := Page.RunModal(Page::"D Doc", Row); Error('OBS %1 | %2 | %3', GetLastErrorText(), GetLastErrorCode(), Log.Take()); end;"#,
    );
    assert_eq!(
        error,
        Outcome::Fail("OBS handler error | Dialog | Q LookupCancel; C; ".into())
    );
    let refused = run(
        r#"[Test] [HandlerFunctions('DocModal')] procedure TableRefuses() var Row: Record "D Row"; Res: Action; begin Setup(Row); Mode := 8; asserterror Res := Page.RunModal(Page::"D Doc", Row); Error('OBS %1 | %2', GetLastErrorCode(), Log.Take()); end;"#,
    );
    assert_eq!(refused, Outcome::Fail("OBS TestValidation | M; ".into()));
}

#[test]
fn a_page_with_another_trigger_keeps_mains_refusal() {
    match run(
        r#"[Test] [HandlerFunctions('AgcrModal')] procedure Agcr() var Row: Record "D Row"; Res: Action; begin Setup(Row); asserterror Res := Page.RunModal(Page::"D Doc AGCR", Row); Error('OBS %1', Log.Take()); end;"#,
    ) {
        Outcome::NotRun(why) => assert!(why.contains(REFUSAL), "{why}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_handler_with_an_unpinned_call_keeps_mains_refusal() {
    match run(
        r#"[Test] [HandlerFunctions('DocModal')] procedure Moves() var Row: Record "D Row"; Res: Action; begin Setup(Row); Mode := 13; asserterror Res := Page.RunModal(Page::"D Doc", Row); Error('OBS %1', Log.Take()); end;"#,
    ) {
        Outcome::NotRun(why) => assert!(why.contains(REFUSAL), "{why}"),
        other => panic!("{other:?}"),
    }
}
