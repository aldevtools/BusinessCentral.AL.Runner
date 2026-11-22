//! ADR-040 amendment 2026-11-12, Part A, review round 2: two more paths that are not a builtin call given the value. A
//! CaptionClass expression (a page control's, a table field's) that gives rendered content refuses: the class text is parsed
//! by its characters (the area before the first comma), which are not known. `CopyArray` puts each element into the
//! destination with the conversion of an assignment, and the builtin guard looks into arrays. The known text twins keep what
//! `main` answers.

use al_builtins::native::Signal;
use al_builtins::rendered_text;
use al_builtins::value::Value;
use al_testkit::Built;

const OBJECTS: &str = r#"
codeunit 42 "Caption Class"
{
    [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end;
    [IntegrationEvent(false, false)] procedure OnAfterCaptionClassResolve(Language: Integer; CaptionExpression: Text; var Caption: Text[1024]) begin end;
}

table 51030 "RC Cap Row"
{
    fields { field(1; Id; Integer) { } field(2; Capped; Integer) { CaptionClass = GetCap(); } }
    keys { key(PK; Id) { } }
    procedure GetCap(): Text var K: Codeunit "RT Keep"; begin exit(K.Get()); end;
}

page 51031 "RC Card"
{
    PageType = Card; SourceTable = "RT Row"; SourceTableTemporary = true;
    layout { area(Content) { field(CapField; Rec.Id) { ApplicationArea = All; CaptionClass = ClassVar; } } }
    trigger OnOpenPage() var K: Codeunit "RT Keep"; begin ClassVar := K.Get(); end;
    var ClassVar: Text;
}

page 51032 "RC Table Cap"
{
    PageType = Card; SourceTable = "RC Cap Row"; SourceTableTemporary = true;
    layout { area(Content) { field(Capped; Rec.Capped) { ApplicationArea = All; } } }
}

codeunit 51033 "RC Tests"
{
    local procedure Seed()
    var R: Record "RT Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
    end;

    local procedure Body(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; Out: OutStream; T: Text;
    begin
        Seed();
        Tmp.Id := 9;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, Out);
        Tmp.Data.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;

    local procedure ControlCaption(Class: Text): Text
    var P: TestPage "RC Card"; K: Codeunit "RT Keep";
    begin
        K.Put(Class);
        P.OpenEdit();
        exit(P.CapField.Caption());
    end;

    // ---- a page control's CaptionClass ---------------------------------------------------------------------------------
    procedure ControlBody(): Text begin exit(ControlCaption(Body())); end;
    procedure ControlArea3(): Text begin exit(ControlCaption('3,' + Body())); end;
    procedure ControlArea7(): Text begin exit(ControlCaption('7,' + Body())); end;
    procedure ControlKnownArea(): Text begin exit(ControlCaption('KN,x')); end;
    procedure ControlKnown(): Text begin exit(ControlCaption('3,Known caption')); end;

    // ---- a table field's CaptionClass expression -------------------------------------------------------------------------
    procedure FieldCaptionBody(): Text var R: Record "RC Cap Row"; K: Codeunit "RT Keep"; begin K.Put(Body()); exit(R.FieldCaption(Capped)); end;
    procedure FieldCaptionArea3(): Text var R: Record "RC Cap Row"; K: Codeunit "RT Keep"; begin K.Put('3,' + Body()); exit(R.FieldCaption(Capped)); end;
    procedure FieldRefCaptionBody(): Text
    var RR: RecordRef; F: FieldRef; K: Codeunit "RT Keep";
    begin
        K.Put(Body());
        RR.Open(Database::"RC Cap Row");
        F := RR.Field(2);
        exit(F.Caption());
    end;
    procedure TestPageCaptionBody(): Text
    var P: TestPage "RC Table Cap"; K: Codeunit "RT Keep";
    begin
        K.Put(Body());
        P.OpenEdit();
        exit(P.Capped.Caption());
    end;
    procedure FieldCaptionKnown(): Text var R: Record "RC Cap Row"; K: Codeunit "RT Keep"; begin K.Put('3,Known field'); exit(R.FieldCaption(Capped)); end;

    // ---- CopyArray and the other array builtins ------------------------------------------------------------------------
    procedure CopyIntoText10(): Text var A: array[3] of Text; B: array[3] of Text[10]; begin A[1] := Body(); CopyArray(B, A, 1); exit('copied'); end;
    procedure CopyIntoCode10(): Text var A: array[3] of Text; B: array[3] of Code[10]; begin A[1] := 'ab' + Body(); CopyArray(B, A, 1); exit('copied'); end;
    procedure CopyIntoText(): Text var A, B: array[3] of Text; begin A[1] := Body(); CopyArray(B, A, 1); exit(B[1]); end;
    local procedure Fill10(var A: array[3] of Text[10]) var S: array[3] of Text; begin S[2] := Body(); CopyArray(A, S, 1); end;
    procedure CopyIntoVarParameter(): Text var B: array[3] of Text[10]; begin Fill10(B); exit('copied'); end;
    procedure CopyKnownIntoText10(): Text var A: array[3] of Text; B: array[3] of Text[10]; begin A[1] := 'abcdefghijklmnopqrstuvwxyz'; CopyArray(B, A, 1); exit(B[1]); end;
    procedure CompressIt(): Integer var B: array[3] of Text; begin B[2] := Body(); exit(CompressArray(B)); end;
    procedure ArrayLenIt(): Integer var B: array[3] of Text; begin B[2] := Body(); exit(ArrayLen(B)); end;
    procedure ClearIt(): Text var B: array[3] of Text; begin B[2] := Body(); Clear(B); exit(Format(B[2] = '')); end;
}
"#;

fn src() -> String {
    format!("{}{OBJECTS}", crate::rendered_text::FIXTURE)
}

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&src()).run_in("RC Tests", proc)
}

/// A subscriber to OnResolveCaptionClass that claims every area: with it the event would see the area read from the mark.
const SUBSCRIBER: &str = r#"
codeunit 51034 "RC Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', true, true)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        Caption := 'claimed [' + CaptionArea + ']'; Resolved := true;
    end;
}
"#;

fn run_subscribed(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&format!("{}{SUBSCRIBER}", src())).run_in("RC Tests", proc)
}

fn text(proc: &str) -> String {
    al_testkit::text(run(proc))
}

/// The run refuses, the reason holds `want` and not the mark.
#[track_caller]
fn refused(proc: &str, want: &str) {
    match run(proc) {
        Err(Signal::Unsupported(why)) => {
            assert!(why.contains(want), "{proc}: {why}");
            assert!(
                !why.contains(rendered_text::RENDERED_TEXT_MARK),
                "{proc}: the reason prints the mark: {why}"
            );
        }
        other => panic!("{proc}: expected a refusal naming {want:?}, got {other:?}"),
    }
}

#[test]
fn rendered_text_caption_class_of_a_page_control_refuses() {
    for p in ["ControlBody", "ControlArea3", "ControlArea7"] {
        refused(p, "a CaptionClass text");
    }
    assert_eq!(text("ControlKnown"), "Known caption");
    for p in ["ControlBody", "ControlArea7", "FieldCaptionBody"] {
        match run_subscribed(p) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("a CaptionClass text"), "{p}: {why}"),
            other => panic!("{p}: expected a refusal, got {other:?}"),
        }
    }
    // A known class text the subscriber claims keeps what main answers.
    assert_eq!(al_testkit::text(run_subscribed("ControlKnownArea")), "claimed [KN]");
}

#[test]
fn rendered_text_caption_class_expression_of_a_table_field_refuses() {
    for p in [
        "FieldCaptionBody",
        "FieldCaptionArea3",
        "FieldRefCaptionBody",
        "TestPageCaptionBody",
    ] {
        refused(p, "a CaptionClass text");
    }
    assert_eq!(text("FieldCaptionKnown"), "Known field");
}

#[test]
fn rendered_text_copyarray_converts_each_element_as_an_assignment() {
    for p in ["CopyIntoText10", "CopyIntoCode10", "CopyIntoVarParameter"] {
        refused(p, "a conversion to a Text of a declared length or another type");
    }
    match run("CopyIntoText") {
        Ok(Value::Text(t)) => assert_eq!(rendered_text::marks(t.as_str()).expect("whole marks").len(), 1),
        other => panic!("expected the marked text, got {other:?}"),
    }
    // A known text keeps what main answers (no length check on a copy).
    assert_eq!(text("CopyKnownIntoText10"), "abcdefghijklmnopqrstuvwxyz");
}

#[test]
fn rendered_text_array_builtins_keep_their_answers() {
    assert!(matches!(run("CompressIt"), Ok(Value::Integer(1))));
    assert!(matches!(run("ArrayLenIt"), Ok(Value::Integer(3))));
    assert_eq!(text("ClearIt"), "Yes");
}
