//! A CaptionClass expression that can reach a Record write, on a page whose empty-record runs have no pinned number (al-runner
//! issue 7391). The walk lets a write through only when it sits in the expression's own routine under an `if` whose
//! condition reads only fields of Rec and constants (the shape of record/captions/CaptionClassStateFreeCalls.al:168-181
//! NotTakenCc); such a write refuses when a run of that page executes it. Every other write keeps main's result.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 50110 "RW Rec"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; NotTaken; Text[30]) { CaptionClass = NotTakenCap(); }
        field(3; Second; Text[30]) { CaptionClass = SecondCap(); }
        field(4; Flag; Code[20]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure NotTakenCap(): Text
    var
        Other: Record "RW Log";
    begin
        Other.Init();
        Other.Id := 1;
        if Rec."Code" = 'INS' then
            Other.Insert();
        if Rec."Code" = 'MOD' then
            Other.Modify();
        if Rec."Code" = 'DEL' then
            Other.Delete();
        if Rec."Code" = 'REN' then
            Other.Rename(2);
        if Rec."Code" = 'MALL' then
            Other.ModifyAll(Id, 3);
        if Rec."Code" = 'DALL' then
            Other.DeleteAll();
        if Rec.Flag = 'INS' then
            Other.Insert();
        exit('3,Not taken');
    end;

    procedure SecondCap(): Text
    begin
        exit('3,Second');
    end;
}

table 50111 "RW Taken"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Taken; Text[30]) { CaptionClass = TakenCap(); }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure TakenCap(): Text
    var
        Other: Record "RW Log";
    begin
        if Rec."Code" = '' then begin
            Other.Init();
            Other.Id := 2;
            if Other.Insert() then;
        end;
        exit('3,Taken');
    end;
}

table 50112 "RW Log"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50114 "RW Setup"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50115 "RW Setup Guard"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Guarded; Text[30]) { CaptionClass = GuardedCap(); }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure GuardedCap(): Text
    var
        Setup: Record "RW Setup";
        Other: Record "RW Log";
    begin
        if not Setup.IsEmpty() then begin
            Other.Init();
            Other.Id := 5;
            Other.Insert();
        end;
        exit('3,Guarded');
    end;
}

table 50116 "RW Callee"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Called; Text[30]) { CaptionClass = CalledCap(); }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure CalledCap(): Text
    var
        Writer: Codeunit "RW Writer";
    begin
        if Rec."Code" = 'INS' then
            Writer.Write();
        exit('3,Called');
    end;
}

codeunit 50117 "RW Writer"
{
    procedure Write()
    var
        Other: Record "RW Log";
    begin
        Other.Init();
        Other.Id := 6;
        Other.Insert();
    end;
}

page 50110 "RW Card"
{
    PageType = Card;
    SourceTable = "RW Rec";
    layout { area(Content) { field("Code"; Rec."Code") { } field(NotTaken; Rec.NotTaken) { } field(Second; Rec.Second) { } field(Flag; Rec.Flag) { } } }
}

page 50111 "RW Taken Card"
{
    PageType = Card;
    SourceTable = "RW Taken";
    layout { area(Content) { field("Code"; Rec."Code") { } field(Taken; Rec.Taken) { } } }
}

page 50115 "RW Setup Guard Card"
{
    PageType = Card;
    SourceTable = "RW Setup Guard";
    layout { area(Content) { field("Code"; Rec."Code") { } field(Guarded; Rec.Guarded) { } } }
}

page 50116 "RW Callee Card"
{
    PageType = Card;
    SourceTable = "RW Callee";
    layout { area(Content) { field("Code"; Rec."Code") { } field(Called; Rec.Called) { } } }
}

codeunit 50113 C
{
    procedure NotTakenOpen(): Text
    var
        Card: TestPage "RW Card";
    begin
        Card.OpenNew();
        exit(Card.NotTaken.Caption);
    end;

    procedure NotTakenOpenEdit(): Text
    var
        R: Record "RW Rec";
        L: Record "RW Log";
        Card: TestPage "RW Card";
    begin
        R."Code" := 'A';
        R.Insert();
        Card.OpenEdit();
        exit(Card.NotTaken.Caption + '|' + Format(L.Count()));
    end;

    local procedure OpenEditOn(C: Code[20]): Text
    var
        R: Record "RW Rec";
        Card: TestPage "RW Card";
    begin
        R."Code" := C;
        R.Insert();
        Card.OpenEdit();
        exit(Card.NotTaken.Caption);
    end;

    procedure RowIns(): Text begin exit(OpenEditOn('INS')); end;
    procedure RowMod(): Text begin exit(OpenEditOn('MOD')); end;
    procedure RowDel(): Text begin exit(OpenEditOn('DEL')); end;
    procedure RowRen(): Text begin exit(OpenEditOn('REN')); end;
    procedure RowMall(): Text begin exit(OpenEditOn('MALL')); end;
    procedure RowDall(): Text begin exit(OpenEditOn('DALL')); end;

    procedure LaterRow(): Text
    var
        R: Record "RW Rec";
        Card: TestPage "RW Card";
    begin
        R."Code" := 'A';
        R.Insert();
        R."Code" := 'B';
        R.Insert();
        Card.OpenEdit();
        Card.Flag.SetValue('INS');
        Card.GoToKey('B');
        exit(Card.NotTaken.Caption);
    end;

    procedure TakenOpen(): Text
    var
        Card: TestPage "RW Taken Card";
    begin
        Card.OpenNew();
        exit(Card.Taken.Caption);
    end;

    procedure TakenFieldCaption(): Text
    var
        R: Record "RW Taken";
        L: Record "RW Log";
    begin
        exit(R.FieldCaption(Taken) + '|' + Format(L.Count()));
    end;

    procedure SetupGuardOpen(): Text
    var
        Card: TestPage "RW Setup Guard Card";
    begin
        Card.OpenNew();
        exit(Card.Guarded.Caption);
    end;

    procedure CalleeOpen(): Text
    var
        Card: TestPage "RW Callee Card";
    begin
        Card.OpenNew();
        exit(Card.Called.Caption);
    end;
}
"#;

fn out(src: &str, routine: &str) -> String {
    match al_testkit::Built::from_source(src).run_in("C", routine) {
        Ok(Value::Text(t)) => t.to_string(),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

/// The run-time refusal of a write in the expression of field `field` of table `table`: main's static text, raised when the write runs.
fn refused_at_run_time(got: &str, field: &str, table: &str, method: &str) -> bool {
    got.starts_with("refused:")
        && got.contains(&format!(
            "the CaptionClass expression of field {field} of table {table} calls Record.{method}, and how many times BC runs it"
        ))
        && got.contains("5870")
}

/// The write is reachable but the empty record never takes its branch: the open runs.
#[test]
fn a_write_the_runs_do_not_take_lets_the_open_run() {
    assert_eq!(out(SRC, "NotTakenOpen"), "Not taken");
}

/// The open's row does not take the branch either: the open runs and writes nothing.
#[test]
fn a_write_the_row_does_not_take_lets_the_open_run() {
    assert_eq!(out(SRC, "NotTakenOpenEdit"), "Not taken|0");
}

/// A write that runs during the open's empty-record runs refuses the open, with main's text.
#[test]
fn a_write_the_empty_runs_take_refuses_the_open() {
    let got = out(SRC, "TakenOpen");
    assert!(refused_at_run_time(&got, "Taken", "RW Taken", "Insert"), "{got}");
}

/// A write that the open's row run takes refuses, one test per write method the walk lets through.
#[test]
fn a_write_the_open_row_takes_refuses_for_every_write_method() {
    for (routine, method) in [
        ("RowIns", "Insert"),
        ("RowMod", "Modify"),
        ("RowDel", "Delete"),
        ("RowRen", "Rename"),
        ("RowMall", "ModifyAll"),
        ("RowDall", "DeleteAll"),
    ] {
        let got = out(SRC, routine);
        assert!(
            refused_at_run_time(&got, "NotTaken", "RW Rec", method),
            "{routine}: {got}"
        );
    }
}

/// A write that a later row run of the page takes refuses: GoToKey after a SetValue runs the expressions with the saved row
/// (record/captions/QdCaptionClassAreas.al:1463-1895, `Session::page_go_to_key`).
#[test]
fn a_write_a_later_row_run_takes_refuses() {
    let got = out(SRC, "LaterRow");
    assert!(refused_at_run_time(&got, "NotTaken", "RW Rec", "Insert"), "{got}");
}

/// Main's result is kept: a FieldCaption read outside a page open runs the write.
#[test]
fn a_field_caption_read_outside_a_page_open_keeps_mains_result() {
    assert_eq!(out(SRC, "TakenFieldCaption"), "Taken|1");
}

/// A write whose guard reads a table (state a page trigger can write) keeps main's static refusal.
#[test]
fn a_write_guarded_by_a_table_read_keeps_mains_refusal() {
    let got = out(SRC, "SetupGuardOpen");
    assert!(
        got.starts_with(
            "refused:the CaptionClass expression of field Guarded of table RW Setup Guard calls Record.Insert"
        ) && got.contains("5870"),
        "{got}"
    );
}

/// A write in a procedure the expression calls keeps main's static refusal, also under a guard on Rec.
#[test]
fn a_write_in_a_called_procedure_keeps_mains_refusal() {
    let got = out(SRC, "CalleeOpen");
    assert!(
        got.starts_with("refused:the CaptionClass expression of field Called of table RW Callee calls Record.Insert")
            && got.contains("5870"),
        "{got}"
    );
}

/// The cells of `caption_state_free_pin` (record/captions/CaptionClassStateFreeCalls.al:218-232, :444-476): a table of six fields
/// with four different expressions, each of which inserts a log row here.
const STATE_FREE: &str = r#"
table 50120 "SF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; A; Text[30]) { CaptionClass = ACap(); }
        field(4; B; Text[30]) { CaptionClass = BCap(); }
        field(5; Cc; Text[30]) { CaptionClass = CCap(); }
        field(6; D; Text[30]) { CaptionClass = DCap(); }
    }
    keys { key(PK; Id) { Clustered = true; } }

    local procedure Log(): Text
    var
        L: Record "SF Log";
    begin
        L.Init();
        L.Id := L.Count() + 1;
        L.Insert();
    end;

    procedure ACap(): Text begin Log(); exit('3,A'); end;
    procedure BCap(): Text begin Log(); exit('3,B'); end;
    procedure CCap(): Text begin Log(); exit('3,C'); end;
    procedure DCap(): Text begin Log(); exit('3,D'); end;
}

table 50121 "SF Log"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50120 "SF Card"
{
    PageType = Card;
    SourceTable = "SF Row";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { ApplicationArea = All; }
            field(A; Rec.A) { ApplicationArea = All; }
            field(B; Rec.B) { ApplicationArea = All; }
            field(Cc; Rec.Cc) { ApplicationArea = All; }
            field(D; Rec.D) { ApplicationArea = All; }
        }
    }
}

page 50121 "SF Cues"
{
    PageType = CardPart;
    SourceTable = "SF Row";
    layout
    {
        area(Content)
        {
            cuegroup(G)
            {
                field(A; Rec.A) { ApplicationArea = All; }
            }
        }
    }
}

codeunit 50113 C
{
    procedure CardOpenEdit(): Text
    var
        R: Record "SF Row";
        L: Record "SF Log";
        Card: TestPage "SF Card";
    begin
        R.Id := 1;
        R.Insert();
        Card.OpenEdit();
        exit(Format(L.Count()));
    end;

    procedure CuesOpenView(): Text
    var
        R: Record "SF Row";
        L: Record "SF Log";
        Cues: TestPage "SF Cues";
    begin
        R.Id := 1;
        R.Insert();
        Cues.OpenView();
        exit(Format(L.Count()));
    end;
}
"#;

/// The state-free Card keeps main's result: 11 empty-record runs of four expressions and one row run each write 48 rows.
#[test]
fn the_state_free_card_keeps_mains_writes() {
    assert_eq!(out(STATE_FREE, "CardOpenEdit"), "48");
}

/// The state-free CardPart keeps main's result: 6 empty-record runs of four expressions and the row run of its one control.
#[test]
fn the_state_free_card_part_keeps_mains_writes() {
    assert_eq!(out(STATE_FREE, "CuesOpenView"), "25");
}

const CAPTION_CLASS: &str = r#"
codeunit 42 "Caption Class"
{
    [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end;
    [IntegrationEvent(false, false)] procedure OnAfterCaptionClassResolve(Language: Integer; CaptionExpression: Text; var Caption: Text[1024]) begin end;
}

table 50130 "TS Log"
{
    fields { field(1; Id; Integer) { } field(2; Txt; Text[100]) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50131 "TS Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', true, true)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    var
        L: Record "TS Log";
    begin
        L.Init();
        L.Id := L.Count() + 1;
        L.Txt := CaptionArea + '|' + CaptionExpr;
        L.Insert();
    end;
}
"#;

/// The literal own-area Card (record/captions/CaptionClassOwnArea.al:144-156, :210-224): a subscriber of the caption translation
/// that writes keeps main's result; no expression runs on this page.
const LITERAL_CARD: &str = r#"
table 50132 "TS Literal Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Dimension 1"; Decimal) { CaptionClass = 'ZZL,2,1'; }
        field(3; "Not Shown"; Decimal) { CaptionClass = 'ZZM,2,1'; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50132 "TS Literal Card"
{
    PageType = Card;
    SourceTable = "TS Literal Row";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { ApplicationArea = All; }
            field("Dimension 1"; Rec."Dimension 1") { ApplicationArea = All; }
        }
    }
}

codeunit 50113 C
{
    procedure Open(): Text
    var
        R: Record "TS Literal Row";
        L: Record "TS Log";
        Card: TestPage "TS Literal Card";
        Got: Text;
    begin
        R.Id := 1;
        R.Insert();
        Card.OpenEdit();
        Got := Card."Dimension 1".Caption();
        exit(Got + '|' + Format(L.Count()));
    end;
}
"#;

/// A Card with two expression fields (no pinned count): a subscriber of the translation of the controls' captions that writes
/// runs outside the expressions' own calls and keeps main's result.
const TWO_EXPRESSIONS: &str = r#"
table 50133 "TS Expr Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; P; Text[30]) { CaptionClass = PCap(); }
        field(3; Q; Text[30]) { CaptionClass = QCap(); }
    }
    keys { key(PK; Id) { Clustered = true; } }

    procedure PCap(): Text begin exit('ZZP,one'); end;
    procedure QCap(): Text begin exit('ZZQ,two'); end;
}

page 50133 "TS Expr Card"
{
    PageType = Card;
    SourceTable = "TS Expr Row";
    layout { area(Content) { field(Id; Rec.Id) { } field(P; Rec.P) { } field(Q; Rec.Q) { } } }
}

codeunit 50113 C
{
    procedure Open(): Text
    var
        R: Record "TS Expr Row";
        L: Record "TS Log";
        Card: TestPage "TS Expr Card";
    begin
        R.Id := 1;
        R.Insert();
        Card.OpenEdit();
        Card.Close();
        exit('closed|' + Format(L.Count()));
    end;
}
"#;

#[test]
fn a_writing_translation_subscriber_on_the_literal_card_keeps_mains_result() {
    let got = out(&format!("{CAPTION_CLASS}{LITERAL_CARD}"), "Open");
    assert!(!got.starts_with("refused:"), "{got}");
    assert!(got.ends_with("|2"), "{got}");
}

#[test]
fn a_writing_translation_subscriber_on_a_two_expression_card_keeps_mains_result() {
    assert_eq!(out(&format!("{CAPTION_CLASS}{TWO_EXPRESSIONS}"), "Open"), "closed|2");
}
