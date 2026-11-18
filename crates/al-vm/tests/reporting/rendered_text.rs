//! ADR-040 amendment 2026-11-12, Part A: rendered content in Text values.
//!
//! A whole text read of the Html placeholder of a Word layout report gives a text that holds only the mark of rendered content
//! (`al_builtins::rendered_text`). It may move (table A3) and is known not to be `''`; every other use refuses (A3b). No corpus
//! case reaches it (the corpus's Html read, AMReportOutputTests.al WordLayout_SaveAsHtml, reads characters and stays refused), so
//! these tests cover the allow-list, the refusals and the loss modes of A8.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::rendered_text::{self, Form};
use al_builtins::value::Value;
use al_testkit::Built;

pub(crate) const FIXTURE: &str = r#"
table 50990 "RT Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } field(3; Other; Blob) { } field(4; Short; Text[30]) { } }
    keys { key(PK; Id) { } }
}

report 50991 "RT Word"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'RtWord.docx'; } }
}

codeunit 50992 "RT Keep"
{
    SingleInstance = true;
    var Kept: Text;
    procedure Put(T: Text) begin Kept := T; end;
    procedure Get(): Text begin exit(Kept); end;
}
"#;

const TESTS: &str = r#"
codeunit 50993 "RT Tests"
{
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
        Tmp.Insert();
    end;

    local procedure Body(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; T: Text;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;

    procedure ReturnBody(): Text begin exit(Body()); end;

    // ---- allowed ---------------------------------------------------------------------------------------------------
    procedure NotEmpty(): Text
    var T: Text;
    begin
        T := Body();
        exit(Format(T <> '') + Format(T = '') + Format('' <> T) + Format('' = T));
    end;
    procedure Joined(): Text var T: Text; begin T := 'Sig ' + Body(); T += ' end'; exit(Format(T <> '')); end;
    procedure DifferingPrefix(): Text var T: Text; begin T := 'Sig ' + Body(); exit(Format(T = 'Other') + Format(T <> 'Other')); end;
    procedure Builder(): Text
    var TB: TextBuilder; T: Text;
    begin
        TB.Append('a');
        TB.Append(Body());
        TB.AppendLine('b');
        T := TB.ToText();
        exit(Format(T <> '') + Format(Format(TB) <> ''));
    end;
    procedure FormatIdentity(): Text begin exit(Format(Format(Body()) <> '')); end;
    procedure Substituted(): Text begin exit(Format(StrSubstNo('<p>%1</p>', Body()) <> '')); end;
    procedure InAVariant(): Text var V: Variant; T: Text; begin V := Body(); T := V; exit(Format(V.IsText()) + Format(T <> '')); end;
    procedure InAList(): Text
    var L: List of [Text]; D: Dictionary of [Integer, Text]; T: Text;
    begin
        L.Add(Body());
        D.Add(1, Body());
        D.Get(1, T);
        exit(Format(L.Get(1) <> '') + Format(T <> ''));
    end;
    procedure StreamRoundTrip(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; O: OutStream; T: Text;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('Sig ' + Body());
        Tmp.Other.CreateInStream(I, TextEncoding::UTF8);
        I.Read(T);
        exit(Format(T <> '') + Format(Tmp.Other.HasValue()) + Format(Tmp.Other.Length() > 0));
    end;
    procedure StreamRoundTripText(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; O: OutStream; T: Text;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText('Sig ' + Body());
        Tmp.Other.CreateInStream(I, TextEncoding::UTF8);
        I.Read(T);
        exit(T);
    end;
    procedure BigTextRead(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; B: BigText;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I);
        B.Read(I);
        exit(Format(B));
    end;
    procedure FormatOfBigText(): Text var B: BigText; begin B.AddText('Sig '); B.AddText(Body()); exit(Format(B)); end;
    procedure KeptInASingleInstance() var K: Codeunit "RT Keep"; begin K.Put(Body()); end;
    procedure AskKept(): Text var K: Codeunit "RT Keep"; begin exit(Format(K.Get() <> '')); end;
    procedure ContainsKept(): Boolean var K: Codeunit "RT Keep"; begin exit(K.Get().Contains('x')); end;

    // ---- refused ---------------------------------------------------------------------------------------------------
    procedure Contains(): Boolean begin exit(Body().Contains('NLB')); end;
    procedure StrPosOf(): Integer begin exit(StrPos(Body(), 'a')); end;
    procedure Length(): Integer begin exit(StrLen(Body())); end;
    procedure Less(): Boolean begin exit(('Body ' + Body()) < 'body'); end;
    procedure LessThanEmpty(): Boolean begin exit('' < Body()); end;
    procedure InRange(): Boolean begin exit(Body() in ['a' .. 'z']); end;
    procedure EqualsKnown(): Boolean begin exit(Body() = 'x'); end;
    procedure CaseOf(): Integer begin case Body() of 'x': exit(1); end; exit(0); end;
    procedure Part(): Text begin exit(CopyStr(Body(), 1, 3)); end;
    procedure Upper(): Text begin exit(UpperCase(Body())); end;
    procedure Index(): Char var T: Text; begin T := Body(); exit(T[1]); end;
    procedure IntoText10() var Short: Text[10]; begin Short := Body(); end;
    procedure IntoCode() var C: Code[20]; begin C := Body(); end;
    procedure IntoAField() var R: Record "RT Row" temporary; begin R.Short := Body(); end;
    procedure IntoACodeParameter() begin TakesCode(Body()); end;
    local procedure TakesCode(C: Code[20]) begin end;
    procedure Parsed() var N: Integer; begin Evaluate(N, Body()); end;
    procedure AsAKey() var D: Dictionary of [Text, Integer]; begin D.Add(Body(), 1); end;
    procedure ListContains(): Boolean var L: List of [Text]; begin L.Add(Body()); exit(L.Contains('x')); end;
    procedure ShowIt() begin Message(Body()); end;
    procedure RaiseIt() begin Error(Body()); end;
    procedure Padded(): Text begin exit(Format(Body(), 10)); end;
    procedure FieldSubstituted(): Text begin exit(StrSubstNo('#1####', Body())); end;
    procedure AsTemplate(): Text begin exit(StrSubstNo(Body(), 'x')); end;
    procedure BuilderLength(): Integer var TB: TextBuilder; begin TB.Append(Body()); exit(TB.Length()); end;
    procedure BigTextLength(): Integer var B: BigText; begin B.AddText(Body()); exit(B.Length()); end;
    procedure WriteCount(): Integer
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        exit(O.WriteText(Body()));
    end;
    procedure WriteMsDos()
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O);
        O.WriteText(Body());
    end;
    procedure WriteWindows()
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::Windows);
        O.WriteText(Body());
    end;
    procedure WriteUtf16()
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF16);
        O.WriteText(Body());
    end;
    procedure WrittenLengthAsANumber(): Text
    var Tmp: Record "RT Row" temporary; O: OutStream;
    begin
        Tmp.Other.CreateOutStream(O, TextEncoding::UTF8);
        O.WriteText(Body());
        exit(Format(Tmp.Other.Length()));
    end;
    procedure ReadTextLine(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; T: Text;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I);
        I.ReadText(T);
        exit(T);
    end;
    procedure ReadWithALength(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; T: Text;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I);
        I.Read(T, 10);
        exit(T);
    end;
    procedure ReadCountUsed(): Integer
    var Tmp: Record "RT Row" temporary; I: InStream; T: Text;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I);
        exit(I.Read(T));
    end;
    procedure ReadTwice(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; T: Text;
    begin
        Save(Tmp);
        Tmp.Data.CreateInStream(I);
        I.Read(T);
        I.Read(T);
        exit(T);
    end;
    procedure ReadAfterAWriteOverIt(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; O: OutStream; T: Text;
    begin
        Save(Tmp);
        Tmp.Data.CreateOutStream(O);
        O.WriteText('SIG');
        Tmp.Data.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;
    procedure ReadWithNoRows(): Text
    var Tmp: Record "RT Row" temporary; R: Record "RT Row"; I: InStream; O: OutStream; T: Text;
    begin
        R.DeleteAll();
        Tmp.Data.CreateOutStream(O);
        Report.SaveAs(Report::"RT Word", '', ReportFormat::Html, O);
        Tmp.Data.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;
    procedure ReadOfAWordPlaceholder(): Text
    var Tmp: Record "RT Row" temporary; I: InStream; O: OutStream; T: Text;
    begin
        Seed();
        Tmp.Data.CreateOutStream(O);
        Report.SaveAs(Report::"RT Word", '', ReportFormat::Word, O);
        Tmp.Data.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;
}
"#;

fn src() -> String {
    format!("{FIXTURE}{TESTS}")
}

fn run(proc: &str) -> Result<Value, Signal> {
    Built::from_source(&src()).run_in("RT Tests", proc)
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

const USE: &str = "a use of the rendered content of report RT Word (Html, layout WordOne)";

#[test]
fn rendered_text_a_whole_read_of_the_html_gives_the_mark_alone_and_marks_its_assumption() {
    assumptions::clear_hits();
    let body = raw("ReturnBody");
    assert!(assumptions::take_hits().contains(&Assumption::ReportHtmlRenderedText));
    let marks = rendered_text::marks(&body).expect("whole marks");
    assert_eq!(marks.len(), 1);
    assert_eq!(marks[0].1, Form::Whole);
    // L8: no character of the placeholder's markup reaches AL.
    assert_eq!(body.chars().count(), 2, "{body:?}");
    assert!(!body.contains('<'));
    let r = rendered_text::render(marks[0].0).expect("registered");
    assert_eq!(
        (r.report.as_str(), r.format.as_str(), r.layout.as_str()),
        ("RT Word", "Html", "WordOne")
    );
}

#[test]
fn rendered_text_two_reads_are_two_renders() {
    let a = rendered_text::marks(&raw("ReturnBody")).unwrap();
    let b = rendered_text::marks(&raw("ReturnBody")).unwrap();
    assert_ne!(a[0].0, b[0].0, "a render number is never reused");
}

#[test]
fn rendered_text_the_allowed_uses_move_the_value() {
    assert_eq!(text("NotEmpty"), "YesNoYesNo");
    assert_eq!(text("Joined"), "Yes");
    assert_eq!(text("DifferingPrefix"), "NoYes");
    assert_eq!(text("Builder"), "YesYes");
    assert_eq!(text("FormatIdentity"), "Yes");
    assert_eq!(text("Substituted"), "Yes");
    assert_eq!(text("InAVariant"), "YesYes");
    assert_eq!(text("InAList"), "YesYes");
    assert_eq!(text("StreamRoundTrip"), "YesYesYes");
}

#[test]
fn rendered_text_a_utf8_stream_round_trip_keeps_the_known_parts_and_the_mark() {
    let t = raw("StreamRoundTripText");
    assert!(t.starts_with("Sig "), "{t:?}");
    assert_eq!(rendered_text::marks(&t).unwrap()[0].1, Form::Whole);
}

#[test]
fn rendered_text_bigtext_read_gives_the_mark_and_format_of_a_bigtext_gives_the_changed_form() {
    let read = raw("BigTextRead");
    assert_eq!(rendered_text::marks(&read).unwrap()[0].1, Form::Changed);
    let t = raw("FormatOfBigText");
    assert!(t.starts_with("Sig "), "{t:?}");
    assert_eq!(rendered_text::marks(&t).unwrap()[0].1, Form::Changed);
}

#[test]
fn rendered_text_every_other_use_refuses() {
    for p in [
        "Contains",
        "StrPosOf",
        "Length",
        "Part",
        "Upper",
        "Parsed",
        "AsAKey",
        "ShowIt",
        "RaiseIt",
        "Padded",
        "FieldSubstituted",
        "AsTemplate",
        "BuilderLength",
        "BigTextLength",
        "WriteCount",
        "WriteMsDos",
        "WriteWindows",
        "WriteUtf16",
        "IntoText10",
        "IntoCode",
        "IntoAField",
        "IntoACodeParameter",
        "EqualsKnown",
        "CaseOf",
    ] {
        refused(p, USE);
    }
}

#[test]
fn rendered_text_an_ordered_comparison_refuses_also_against_empty_and_after_a_differing_prefix() {
    for p in ["Less", "LessThanEmpty", "InRange"] {
        refused(p, "an ordered comparison");
    }
}

#[test]
fn rendered_text_a_list_searched_by_value_and_an_index_refuse() {
    for p in ["ListContains", "Index"] {
        match run(p) {
            Err(Signal::Unsupported(_)) => {}
            other => panic!("{p}: expected a refusal, got {other:?}"),
        }
    }
}

#[test]
fn rendered_text_the_length_of_a_written_buffer_answers_only_against_0() {
    refused("WrittenLengthAsANumber", "byte size");
}

#[test]
fn rendered_text_every_other_read_of_the_html_refuses() {
    refused("ReadTwice", "from a position after its start");
    for p in ["ReadTextLine", "ReadWithALength", "ReadOfAWordPlaceholder"] {
        refused(p, "a read of the content of a placeholder");
    }
    refused("ReadCountUsed", "return value used");
    refused("ReadAfterAWriteOverIt", "after AL wrote over it");
    // The statement form of a save with no dataset row raises ReportResultEmpty before the read (corpus pull request 1371,
    // reports/html-email-body/RxLayoutSelectionTests.al:915 NoRow_WordLayout_Html `statement err|code`); it refused before
    // that was measured.
    match run("ReadWithNoRows") {
        Err(Signal::Error(e)) => assert_eq!(e.code.as_str(), "ReportResultEmpty"),
        other => panic!("ReadWithNoRows: expected ReportResultEmpty, got {other:?}"),
    }
}

#[test]
fn rendered_text_a_body_kept_in_a_single_instance_codeunit_stays_marked_in_a_later_session() {
    let mut built = Built::from_source(&src());
    built.run_in("RT Tests", "KeptInASingleInstance").expect("kept");
    // The render list and the guard are process-wide: the kept body still answers '' and still refuses Contains.
    assert_eq!(al_testkit::text(built.run_in("RT Tests", "AskKept")), "Yes");
    match built.run_in("RT Tests", "ContainsKept") {
        Err(Signal::Unsupported(why)) => assert!(why.contains(USE), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
