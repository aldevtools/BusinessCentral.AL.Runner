//! The close of a Document with hidden parts after its ModalPageHandler raised an error (al-runner issue 7473) runs only in
//! the shape of pages/close-errors/CemDocumentTests.al (crates/al-vm/src/pages/document_handler_error.rs
//! `document_close_pinned`). "D Doc" is page CEM Doc of CemFixtures.al:348-424 with other names: it closes as the corpus
//! pins it. Every page that differs from it in one fact (no part, one part, `Editable = false`, `ModifyAllowed = false`,
//! `CurrPage.Update(false)`, an OnValidate that raises, one more trigger) and every handler outside the pinned modes
//! (al-runner PR 7816 review, round 3) keeps main's behavior: main's result where main ran it, else a refusal.

use al_testrun::Outcome;

const HEAD: &str = r#"
table 50500 "D Code" { fields { field(1; "Code"; Code[10]) { } } keys { key(PK; "Code") { } } }
table 50501 "D Row"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } field(3; Note; Text[30]) { } field(4; Ref; Code[10]) { TableRelation = "D Code"; } }
    keys { key(PK; "Code") { } }
    trigger OnModify() var L: Codeunit "D Log"; begin L.Add('T-mod ' + Rec.Name); if Rec.Name = 'FAILMOD' then Error('table modify refused'); end;
}
table 50502 "D Line"
{
    fields { field(1; DocCode; Code[10]) { } field(2; LineNo; Integer) { } field(3; Qty; Decimal) { } }
    keys { key(PK; DocCode, LineNo) { } }
    trigger OnModify() var L: Codeunit "D Log"; begin L.Add('L-mod'); if Rec.Qty = -1 then Error('line modify refused'); end;
}
codeunit 50503 "D Log"
{
    SingleInstance = true;
    var L: Text; F: Text;
    procedure Set(NF: Text) begin F := NF; L := ''; end;
    procedure Add(T: Text) begin L += T + '; '; end;
    procedure Take(): Text var T: Text; begin T := L; L := ''; exit(T); end;
    procedure Query(A: Action): Boolean begin Add('Q ' + Format(A)); if F = 'query' then Error('query error'); if F = 'veto' then exit(false); exit(true); end;
    procedure Closed() begin Add('C'); if F = 'close' then Error('close error'); end;
}
page 50504 "D Lines" { PageType = ListPart; SourceTable = "D Line"; layout { area(Content) { repeater(R) { field(LineNo; Rec.LineNo) { } field(Qty; Rec.Qty) { } } } } }
page 50505 "D Info" { PageType = CardPart; SourceTable = "D Row"; layout { area(Content) { field(Note; Rec.Note) { } } } }
"#;

const LINES: &str = r#"part(Lines; "D Lines") { ApplicationArea = All; SubPageLink = DocCode = field("Code"); }"#;
const FACTBOXES: &str = r#"area(FactBoxes) { part(Info; "D Info") { ApplicationArea = All; SubPageLink = "Code" = field("Code"); } part(Hidden; "D Info") { ApplicationArea = All; Visible = false; SubPageLink = "Code" = field("Code"); } part(Extra; "D Info") { ApplicationArea = All; Visible = ExtraVisible; } }"#;
const UPDATE: &str = "CurrPage.Update();";

/// The pages: name, page properties, the body of Name's OnValidate, the content part, the factboxes, more triggers.
const PAGES: [(&str, &str, &str, &str, &str, &str); 8] = [
    ("Doc", "", UPDATE, LINES, FACTBOXES, ""),
    ("NoParts", "", UPDATE, "", "", ""),
    ("OneVis", "", UPDATE, LINES, "", ""),
    ("RO", "Editable = false;", UPDATE, LINES, FACTBOXES, ""),
    ("UpdFalse", "", "CurrPage.Update(false);", LINES, FACTBOXES, ""),
    ("NoMod", "ModifyAllowed = false;", UPDATE, LINES, FACTBOXES, ""),
    (
        "ValErr",
        "",
        "if Rec.Name = 'BAD' then Error('validate refused');",
        LINES,
        FACTBOXES,
        "",
    ),
    (
        "Agcr",
        "",
        UPDATE,
        LINES,
        FACTBOXES,
        "trigger OnAfterGetCurrRecord() begin end;",
    ),
];

fn page(id: usize, (name, props, validate, lines, factboxes, more): (&str, &str, &str, &str, &str, &str)) -> String {
    let line_mode = if lines.is_empty() {
        ""
    } else {
        "12: begin P.Lines.First(); P.Lines.Qty.SetValue(-1); P.OK().Invoke(); end;"
    };
    format!(
        r#"
page {id} "D {name}"
{{
    PageType = Document; SourceTable = "D Row"; ApplicationArea = All; UsageCategory = None; {props}
    layout
    {{
        area(Content)
        {{
            group(General)
            {{
                field("Code"; Rec."Code") {{ ApplicationArea = All; }}
                field(Name; Rec.Name) {{ ApplicationArea = All; trigger OnValidate() begin {validate} end; }}
                field(Ref; Rec.Ref) {{ ApplicationArea = All; }}
            }}
            {lines}
        }}
        {factboxes}
    }}
    var Log: Codeunit "D Log"; ExtraVisible: Boolean;
    trigger OnModifyRecord(): Boolean begin Log.Add('M'); exit(true); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Log.Query(CloseAction)); end;
    trigger OnClosePage() begin Log.Closed(); end;
    {more}
}}
[ModalPageHandler] procedure H{name}(var P: TestPage "D {name}")
begin
    case Mode of
        7: Error('handler error');
        8: P.Name.SetValue('FAILMOD');
        {line_mode}
        13: begin P.Next(); Error('handler error'); end;
        20: P.Ref.SetValue('NOPE');
        21: P.Name.SetValue('BAD');
        22: begin P.Name.SetValue('X'); Error('handler error'); end;
        30: begin asserterror P.Name.SetValue('FAILMOD'); Error('handler error'); end;
    end;
end;
"#
    )
}

/// Runs, in a program of its own, a test that opens page "D `name`" by the static `Page.RunModal` with failure `fail` of
/// the log codeunit switched on and handler mode `mode`, and observes the error text, its code and the trigger log.
fn run(name: &str, fail: &str, mode: i32) -> Outcome {
    let mut pages = String::new();
    let mut handlers = String::new();
    for (i, p) in PAGES.into_iter().enumerate() {
        let src = page(50510 + i, p);
        let (pg, handler) = src.split_once("[ModalPageHandler]").expect("page and handler");
        pages += pg;
        handlers += "[ModalPageHandler]";
        handlers += handler;
    }
    let src = format!(
        r#"{HEAD}{pages}
codeunit 50590 "D Tests"
{{
    Subtype = Test;
    var Log: Codeunit "D Log"; Mode: Integer;
    [Test] [HandlerFunctions('H{name}')]
    procedure T()
    var Row: Record "D Row"; C: Record "D Code"; Line: Record "D Line"; Res: Action;
    begin
        Row.DeleteAll(); C.DeleteAll(); Line.DeleteAll();
        Row."Code" := 'R1'; Row.Name := 'start'; Row.Insert(); C."Code" := 'C1'; C.Insert();
        Line.DocCode := 'R1'; Line.LineNo := 1; Line.Qty := 1; Line.Insert(); Commit();
        Log.Set('{fail}'); Mode := {mode}; ClearLastError();
        asserterror Res := Page.RunModal(Page::"D {name}", Row);
        Error('OBS %1 | %2 | %3', GetLastErrorText(), GetLastErrorCode(), Log.Take());
    end;
    {handlers}
}}
"#
    );
    let o = al_testkit::outcomes_in(&src, "D Tests");
    o.into_iter().next().expect("one test").1
}

fn assert_refused(name: &str, fail: &str, mode: i32, reason: &str) {
    match run(name, fail, mode) {
        Outcome::NotRun(why) => assert!(why.contains(reason), "{name} {fail} {mode}: {why}"),
        other => panic!("{name} {fail} {mode}: {other:?}"),
    }
}

const MODIFY_REFUSED: &str =
    "Validation error for Field: Name,  Message = 'table modify refused (Select Refresh to discard errors)'";

#[test]
fn the_pinned_shape_closes_as_the_corpus_pins() {
    assert_eq!(
        run("Doc", "none", 7),
        Outcome::Fail("OBS handler error | Dialog | Q LookupCancel; C; ".into())
    );
    assert_eq!(
        run("Doc", "none", 8),
        Outcome::Fail(format!("OBS {MODIFY_REFUSED} | TestValidation | M; T-mod FAILMOD; "))
    );
    assert_eq!(
        run("Doc", "query", 7),
        Outcome::Fail("OBS Unhandled UI: Message query error | MissingUIHandler | Q LookupCancel; ".into())
    );
}

/// A page that differs from CEM Doc in one fact keeps main's behavior (al-runner PR 7816 review, round 3: main's result
/// where main ran the test, else main's refusal).
#[test]
fn a_page_outside_the_pinned_shape_keeps_mains_behavior() {
    assert_eq!(
        run("NoParts", "none", 7),
        Outcome::Fail("OBS handler error | Dialog | ".into())
    );
    assert_refused("NoParts", "close", 0, "an error in OnClosePage");
    assert_eq!(
        run("OneVis", "none", 8),
        Outcome::Fail(format!(
            "OBS {MODIFY_REFUSED} | TestValidation | M; T-mod FAILMOD; Q LookupCancel; C; "
        ))
    );
    for name in ["RO", "UpdFalse", "NoMod"] {
        assert_refused(name, "none", 7, "not pinned");
    }
    assert_refused("ValErr", "none", 21, "not pinned");
    assert_refused("Agcr", "none", 8, "with an unsaved row or a hidden part");
}

/// A handler outside the pinned modes of `CemDocModal` keeps main's refusal: a value typed before the handler's own error,
/// a value of another field that its relation refused, a refused save the handler caught before its own error, and a move.
#[test]
fn a_handler_outside_the_pinned_modes_keeps_mains_refusal() {
    for mode in [13, 20, 22, 30] {
        assert_refused("Doc", "none", mode, "with an unsaved row or a hidden part");
    }
}

/// The two refusals of the pinned shape: OnQueryClosePage vetoes the close the platform asks after the handler's error
/// (document_handler_error.rs `close_document_after_handler_error`), and OK in the handler after the ListPart line's save
/// failed (`document_part_save_error`, CemDocumentTests.al:293 HandlerLineRefusesOk_NoError answers None without an error).
#[test]
fn the_pinned_shape_refuses_what_the_corpus_does_not_pin() {
    assert_refused("Doc", "veto", 7, "whose close then failed or was refused");
    assert_refused("Doc", "none", 12, "inside the handler");
}
