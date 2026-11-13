//! ADR-040 amendment 2026-11-12, Part A: the paths that are not a builtin call given the value. A character assignment,
//! `foreach`, the write-back of a `var` argument, a write over the bytes of a buffer, a report dataset column, an XmlPort
//! element and a page control each either keep the mark or refuse; none gives known text for rendered content. The known
//! text twins keep what `main` answers.

use al_builtins::native::Signal;
use al_builtins::rendered_text::{self, Form};
use al_builtins::value::Value;
use al_testkit::Built;

const LEAKS: &str = r#"
report 51012 "RL Carrier"
{
    UsageCategory = None;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } column(BodyCol; BodyText) { } } }
    var BodyText: Text;
    trigger OnPreReport() var K: Codeunit "RT Keep"; begin BodyText := K.Get(); end;
}

xmlport 51013 "RL Port"
{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { textelement(BodyEl) { trigger OnBeforePassVariable() begin BodyEl := Body; end; } } }
    var Body: Text;
    procedure SetBody(B: Text) begin Body := B; end;
}

xmlport 51014 "RL Big Port"
{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { textelement(BodyEl) { TextType = BigText; trigger OnBeforePassVariable() begin BodyEl.AddText(Body); end; } } }
    var Body: Text;
    procedure SetBody(B: Text) begin Body := B; end;
}

page 51015 "RL Card"
{
    PageType = Card;
    layout { area(Content) { field(BodyField; Body) { ApplicationArea = All; } } }
    var Body: Text;
    trigger OnOpenPage() var K: Codeunit "RT Keep"; begin Body := K.Get(); end;
}

codeunit 51016 "RL Tests"
{
    Subtype = Test;

    local procedure Seed()
    var R: Record "RT Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Insert();
    end;

    local procedure Save(var Tmp: Record "RT Row" temporary)
    var Out: OutStream;
    begin
        Seed();
        Tmp.Id := 9;
        Tmp.Data.CreateOutStream(Out);
        Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, Out);
    end;

    local procedure Body(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; T: Text;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;

    local procedure ReadBack(var Tmp: Record "RT Row" temporary): Text
    var I: InStream; T: Text;
    begin
        Tmp.Other.CreateInStream(I, TextEncoding::UTF8);
        I.Read(T);
        exit(T);
    end;

    // ---- a character assignment --------------------------------------------------------------------------------------
    procedure CharOnTheMark(): Text var T: Text; begin T := Body(); T[1] := 'x'; exit(Format(T = '')); end;
    procedure CharOnTheNumber(): Text var T: Text; begin T := Body(); T[2] := 'x'; exit(Format(T = '')); end;
    procedure CharBeforeTheMark(): Text var T: Text; begin T := 'ab' + Body(); T[1] := 'x'; exit(Format(T = '')); end;
    procedure CharAfterTheMark(): Text var T: Text; begin T := Body() + 'cd'; T[4] := 'x'; exit(Format(T = '')); end;
    procedure CharAtTheEnd(): Text var T: Text; begin T := 'ab' + Body(); T[5] := 'x'; exit(Format(T = '')); end;
    procedure CharOfKnownText(): Text var T: Text; begin T := 'abc'; T[1] := 'x'; T[4] := 'd'; exit(T); end;

    // ---- BigText.GetSubText ------------------------------------------------------------------------------------------
    procedure SubTextFromOne(): Text
    var B: BigText; T: Text;
    begin
        B.AddText('Sig ');
        B.AddText(Body());
        B.GetSubText(T, 1);
        exit(T);
    end;
    procedure SubTextFromTwo(): Text var B: BigText; T: Text; begin B.AddText(Body()); B.AddText('tail'); B.GetSubText(T, 2); exit(T); end;
    procedure SubTextFromFour(): Text var B: BigText; T: Text; begin B.AddText(Body()); B.AddText('tail'); B.GetSubText(T, 4); exit(T); end;
    procedure SubTextFromFive(): Text var B: BigText; T: Text; begin B.AddText(Body()); B.AddText('tail'); B.GetSubText(T, 5); exit(T); end;
    procedure SubTextWithALength(): Text var B: BigText; T: Text; begin B.AddText(Body()); B.GetSubText(T, 1, 5); exit(T); end;
    procedure SubTextCountUsed(): Integer var B: BigText; T: Text; begin B.AddText(Body()); exit(B.GetSubText(T, 1)); end;
    procedure SubTextIntoBigTextFromOne(): Text var B, B2: BigText; begin B.AddText(Body()); B.AddText('tail'); B.GetSubText(B2, 1); exit(Format(B2)); end;
    procedure SubTextIntoBigTextFromFour(): Text var B, B2: BigText; begin B.AddText(Body()); B.AddText('tail'); B.GetSubText(B2, 4); exit(Format(B2)); end;
    procedure SubTextIntoText10(): Text var B: BigText; S: Text[10]; begin B.AddText(Body()); B.GetSubText(S, 1); exit('ok'); end;
    procedure SubTextOfKnownText(): Text var B: BigText; T: Text; begin B.AddText('abcdef'); B.GetSubText(T, 4); exit(T); end;

    // ---- a write over the bytes of a buffer -----------------------------------------------------------------------------
    local procedure Written(var Tmp: Record "RT Row" temporary)
    var O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(Body());
    end;
    procedure KnownOverTheMark(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Written(Tmp);
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('1234567');
        exit(ReadBack(Tmp));
    end;
    procedure KnownOverAndPastTheMark(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Written(Tmp);
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('123456789012');
        exit(ReadBack(Tmp));
    end;
    procedure KnownOverThenTheBodyAgain(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Written(Tmp);
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('12');
        O.WriteText(Body());
        exit(ReadBack(Tmp));
    end;
    procedure TheBodyOverKnownText(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('abcdefghijklmnop');
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(Body());
        exit(ReadBack(Tmp));
    end;
    procedure TheBodyOverTheHtml(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream; I: InStream; T, B: Text;
    begin
        B := Body();
        Save(Tmp);
        Tmp.Data.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(B);
        Tmp.Data.CreateInStream(I, TextEncoding::UTF8);
        I.Read(T);
        exit(T);
    end;
    procedure ACopyOverKnownText(): Text
    var Tmp, Tmp2: Record "RT Row" temporary; O: OutStream; I: InStream;
    begin
        Written(Tmp);
        Tmp2.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('abcdefghijklmnop');
        Tmp2.Other.CreateOutStream(O, TextEncoding::UTF8);
        Tmp.Other.CreateInStream(I, TextEncoding::UTF8);
        CopyStream(O, I);
        exit(ReadBack(Tmp2));
    end;
    procedure KnownAfterTheBody(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(Body());
        O.WriteText('tail');
        exit(ReadBack(Tmp));
    end;
    procedure KnownThenTheBody(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.Write('known');
        O.Write(Body());
        exit(ReadBack(Tmp));
    end;
    procedure ACopyAfterKnownText(): Text
    var Tmp, Tmp2: Record "RT Row" temporary; O: OutStream; I: InStream;
    begin
        Written(Tmp);
        Tmp2.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('pre');
        Tmp.Other.CreateInStream(I, TextEncoding::UTF8);
        CopyStream(O, I);
        exit(ReadBack(Tmp2));
    end;
    procedure KnownOverKnown(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('abcdefgh');
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('XY');
        exit(ReadBack(Tmp));
    end;

    // ---- a report dataset column and an XmlPort element ----------------------------------------------------------------
    procedure DatasetColumn(): Text
    var Tmp: Record "RT Row" temporary; K: Codeunit "RT Keep"; O: OutStream;
    begin
        K.Put('Sig' + Body());
        Seed();
        Tmp.Other.CreateOutStream(O);
        Report.SaveAs(Report::"RL Carrier", '', ReportFormat::Xml, O);
        exit('written');
    end;
    procedure DatasetColumnOfKnownText(): Text
    var Tmp: Record "RT Row" temporary; K: Codeunit "RT Keep"; O: OutStream;
    begin
        K.Put('Sig');
        Seed();
        Tmp.Other.CreateOutStream(O);
        Report.SaveAs(Report::"RL Carrier", '', ReportFormat::Xml, O);
        exit(Format(Tmp.Other.HasValue()));
    end;
    procedure XmlPortElement(): Text
    var P: XmlPort "RL Port"; Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        P.SetBody('Sig' + Body());
        Tmp.Other.CreateOutStream(O);
        P.SetDestination(O);
        P.Export();
        exit('written');
    end;
    procedure XmlPortBigTextElement(): Text
    var P: XmlPort "RL Big Port"; Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        P.SetBody('Sig' + Body());
        Tmp.Other.CreateOutStream(O);
        P.SetDestination(O);
        P.Export();
        exit('written');
    end;
    procedure XmlPortElementOfKnownText(): Text
    var P: XmlPort "RL Port"; Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        P.SetBody('Sig');
        Tmp.Other.CreateOutStream(O);
        P.SetDestination(O);
        P.Export();
        exit(Format(Tmp.Other.HasValue()));
    end;

    // ---- a Text[N] variable reached through a var argument or foreach --------------------------------------------------
    procedure ReadInto10(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; S: Text[10];
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I);
        I.Read(S);
        exit('ok');
    end;
    procedure BigTextSubTextInto10(): Text var B: BigText; S: Text[10]; begin B.AddText(Body()); B.GetSubText(S, 1); exit('ok'); end;
    procedure DictGetInto10(): Text
    var D: Dictionary of [Integer, Text]; S: Text[10];
    begin
        D.Add(1, Body());
        D.Get(1, S);
        exit('ok');
    end;
    procedure ListGetInto10(): Text
    var L: List of [Text]; S: Text[10];
    begin
        L.Add(Body());
        L.Get(1, S);
        exit('ok');
    end;
    procedure ForeachInto10(): Text
    var L: List of [Text]; S: Text[10];
    begin
        L.Add(Body());
        foreach S in L do;
        exit('ok');
    end;
    procedure ForeachIntoCode(): Text
    var L: List of [Text]; C: Code[20];
    begin
        L.Add(Body());
        foreach C in L do;
        exit('ok');
    end;
    procedure ForeachIntoText(): Text
    var L: List of [Text]; T, Last: Text;
    begin
        L.Add(Body());
        foreach T in L do Last := T;
        exit(Last);
    end;
    procedure VarParameterInto10(): Text var S: Text[10]; begin Fill(S); exit('ok'); end;
    local procedure Fill(var T: Text) begin T := Body(); end;
    procedure ForeachOfKnownTextInto10(): Text
    var L: List of [Text]; S: Text[10];
    begin
        L.Add('abcdefghijklmnopqrstuvwxyz');
        foreach S in L do;
        exit(S);
    end;
    procedure ReadOfKnownTextInto10(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream; I: InStream; S: Text[10];
    begin
        Tmp.Other.CreateOutStream(O);
        O.WriteText('abcdefghijklmnopqrstuvwxyz');
        Tmp.Other.CreateInStream(I);
        I.Read(S);
        exit(S);
    end;

    // ---- a page control ------------------------------------------------------------------------------------------------
    procedure FieldValue(): Text
    var P: TestPage "RL Card"; K: Codeunit "RT Keep"; T: Text;
    begin
        K.Put(Body());
        P.OpenEdit();
        T := P.BodyField.Value();
        exit(T);
    end;
    procedure FieldAssertEquals(): Text
    var P: TestPage "RL Card"; K: Codeunit "RT Keep";
    begin
        K.Put(Body());
        P.OpenEdit();
        P.BodyField.AssertEquals('x');
        exit('equal');
    end;
    procedure FieldAssertEqualsCaught(): Text
    var P: TestPage "RL Card"; K: Codeunit "RT Keep";
    begin
        K.Put(Body());
        P.OpenEdit();
        asserterror P.BodyField.AssertEquals('x');
        exit(GetLastErrorText());
    end;
    procedure FieldAsInteger(): Integer
    var P: TestPage "RL Card"; K: Codeunit "RT Keep";
    begin
        K.Put(Body());
        P.OpenEdit();
        exit(P.BodyField.AsInteger());
    end;
    procedure FieldAssertEqualsOfKnownText(): Text
    var P: TestPage "RL Card"; K: Codeunit "RT Keep";
    begin
        K.Put('known');
        P.OpenEdit();
        asserterror P.BodyField.AssertEquals('x');
        exit(GetLastErrorText());
    end;
}
"#;

fn src() -> String {
    format!("{}{LEAKS}", crate::rendered_text::FIXTURE)
}

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&src()).run_in("RL Tests", proc)
}

fn text(proc: &str) -> String {
    al_testkit::text(run(proc))
}

fn raw(proc: &str) -> String {
    match run(proc) {
        Ok(Value::Text(t)) => t.as_str().to_string(),
        other => panic!("{proc}: expected a text, got {other:?}"),
    }
}

/// The run refuses, the reason holds `want` and neither the mark nor the placeholder's markup.
#[track_caller]
fn refused(proc: &str, want: &str) {
    match run(proc) {
        Err(Signal::Unsupported(why)) => {
            assert!(why.contains(want), "{proc}: {why}");
            assert!(
                !why.contains(rendered_text::RENDERED_TEXT_MARK) && !why.contains("<!DOCTYPE"),
                "{proc}: the reason prints the mark or the placeholder: {why}"
            );
        }
        other => panic!("{proc}: expected a refusal naming {want:?}, got {other:?}"),
    }
}

const USE: &str = "a use of the rendered content of report RT Word (Html, layout WordOne)";

#[test]
fn rendered_text_leaks_a_character_assignment_refuses_at_every_position() {
    for p in [
        "CharOnTheMark",
        "CharOnTheNumber",
        "CharBeforeTheMark",
        "CharAfterTheMark",
        "CharAtTheEnd",
    ] {
        refused(p, USE);
    }
    // A known text keeps what main answers.
    assert_eq!(text("CharOfKnownText"), "xbcd");
}

#[test]
fn rendered_text_leaks_getsubtext_moves_the_whole_text_from_position_one_into_a_text_only() {
    let t = raw("SubTextFromOne");
    assert!(t.starts_with("Sig "), "{t:?}");
    assert_eq!(rendered_text::marks(&t).expect("whole marks")[0].1, Form::Whole);
    assert_eq!(t.chars().count(), 6, "{t:?}");
    for p in [
        "SubTextFromTwo",
        "SubTextFromFour",
        "SubTextFromFive",
        "SubTextWithALength",
        "SubTextCountUsed",
        "SubTextIntoBigTextFromOne",
        "SubTextIntoBigTextFromFour",
        "SubTextIntoText10",
    ] {
        refused(p, USE);
    }
    assert_eq!(text("SubTextOfKnownText"), "def");
}

#[test]
fn rendered_text_leaks_a_write_over_a_buffer_that_holds_rendered_content_makes_every_read_refuse() {
    for p in [
        "KnownOverTheMark",
        "KnownOverAndPastTheMark",
        "KnownOverThenTheBodyAgain",
        "TheBodyOverKnownText",
        "ACopyOverKnownText",
    ] {
        refused(
            p,
            "a read of the content of a placeholder text with rendered content that was written over",
        );
    }
    refused("TheBodyOverTheHtml", "after AL wrote over it");
}

#[test]
fn rendered_text_leaks_an_append_keeps_the_written_text_and_known_text_keeps_its_answer() {
    let t = raw("KnownAfterTheBody");
    assert!(t.ends_with("tail"), "{t:?}");
    assert_eq!(rendered_text::marks(&t).expect("whole marks").len(), 1);
    // `Write` ends a text with a zero byte and a text read stops there: the known text written first is all of it.
    assert_eq!(text("KnownThenTheBody"), "known");
    let t = raw("ACopyAfterKnownText");
    assert!(t.starts_with("pre"), "{t:?}");
    assert_eq!(rendered_text::marks(&t).expect("whole marks").len(), 1);
    assert_eq!(text("KnownOverKnown"), "XYcdefgh");
}

#[test]
fn rendered_text_leaks_a_report_dataset_column_and_an_xmlport_element_refuse() {
    for p in ["DatasetColumn", "XmlPortElement", "XmlPortBigTextElement"] {
        refused(p, USE);
    }
    assert_eq!(text("DatasetColumnOfKnownText"), "Yes");
    assert_eq!(text("XmlPortElementOfKnownText"), "Yes");
}

#[test]
fn rendered_text_leaks_a_text_of_a_declared_length_refuses_behind_a_var_argument_and_foreach() {
    for p in [
        "ReadInto10",
        "BigTextSubTextInto10",
        "DictGetInto10",
        "ListGetInto10",
        "ForeachInto10",
        "ForeachIntoCode",
        "VarParameterInto10",
    ] {
        refused(p, USE);
    }
    let t = raw("ForeachIntoText");
    assert_eq!(rendered_text::marks(&t).expect("whole marks").len(), 1);
    // Known text keeps what main answers on both paths.
    assert_eq!(text("ForeachOfKnownTextInto10"), "abcdefghijklmnopqrstuvwxyz");
    match run("ReadOfKnownTextInto10") {
        Err(Signal::Error(e)) => assert_eq!(
            e.message,
            "The length of the string is 26, but it must be less than or equal to 10 characters. Value: abcdefghijklmnopqrstuvwxyz"
        ),
        other => panic!("expected the length error, got {other:?}"),
    }
}

#[test]
fn rendered_text_leaks_a_page_control_gives_its_text_to_value_only() {
    let t = raw("FieldValue");
    assert_eq!(rendered_text::marks(&t).expect("whole marks").len(), 1);
    for p in ["FieldAssertEquals", "FieldAssertEqualsCaught", "FieldAsInteger"] {
        refused(p, USE);
    }
    assert_eq!(
        text("FieldAssertEqualsOfKnownText"),
        "AssertEquals for Field: BodyField Expected = 'x', Actual = 'known'"
    );
}
