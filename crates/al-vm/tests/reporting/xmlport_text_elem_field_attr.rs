//! An Xml export of a text element with one field attribute below a temporary table element, with its own text and an
//! OnBeforePassVariable trigger (al-runner issue 7865): the element is always written, the attribute read after the trigger.
//! Assumption `xmlport-text-elem-field-attr-export` (corpus PR 1366, xmlports/shapes-properties/XpTextElemFieldAttrExport.al).

use al_builtins::native::Signal;
use al_testkit::{Built, text};

/// `{ATTR}` the attribute line, `{ELEM}` the element's properties and trigger, `{ROW}` the table element's trigger.
const PORT: &str = r#"
xmlport 50721 "TEA Port"
{
    Direction = {DIR}; Format = Xml; FormatEvaluate = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            MaxOccurs = Once; MinOccurs = Once;
            tableelement(Line; "TEA Line")
            {
                MinOccurs = Zero; UseTemporary = true;
                fieldelement(EntryNo; Line."Entry No.") { }
                textelement(MinValue)
                {
                    MaxOccurs = Once;
                    {ELEM}
                    {ATTR}
                }
                {ROW}
            }
        }
    }
    procedure SetLines(var TempLine: Record "TEA Line" temporary)
    begin
        if TempLine.FindSet() then
            repeat
                Line := TempLine;
                Line.Insert();
            until TempLine.Next() = 0;
    end;
}
"#;

const SET_TEXT: &str = r#"
                        if Line."Default Min. Value" <> 0 then
                            MinValue := Format(Line."Default Min. Value", 0, 2)
                        else
                            Clear(MinValue);"#;

const COMMON: &str = r#"
table 50720 "TEA Line"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Default Min. Value"; Decimal) { } field(3; "Min. Value Equals Valid"; Boolean) { } field(4; Note; Text[30]) { } field(5; Num; Integer) { }
        field(6; "Has Line"; Boolean) { FieldClass = FlowField; CalcFormula = exist("TEA Line" where("Entry No." = field("Entry No."))); } }
    keys { key(PK; "Entry No.") { } }
}
table 50729 "TEA Buf" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50728 "TEA Run"
{
    local procedure AddLine(var TempLine: Record "TEA Line" temporary; No: Integer; MinValue: Decimal; EqualsValid: Boolean; Note: Text[30])
    begin
        TempLine.Init();
        TempLine."Entry No." := No;
        TempLine."Default Min. Value" := MinValue;
        TempLine."Min. Value Equals Valid" := EqualsValid;
        TempLine.Note := Note;
        TempLine.Insert();
    end;
    local procedure ExportText(var TempLine: Record "TEA Line" temporary): Text
    var B: Record "TEA Buf" temporary; Port: XmlPort "TEA Port"; OutS: OutStream; InS: InStream; Part: Text; R: Text;
    begin
        B.Data.CreateOutStream(OutS);
        Port.SetLines(TempLine);
        Port.SetDestination(OutS);
        Port.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin
            InS.ReadText(Part);
            if R <> '' then
                R += '|';
            R += Part;
        end;
        exit(R);
    end;
    procedure Measure(): Text
    var TempLine: Record "TEA Line" temporary; R: Text;
    begin
        AddLine(TempLine, 1, 5, true, '');
        R := ExportText(TempLine) + '#';
        TempLine.DeleteAll();
        AddLine(TempLine, 1, 0, true, '');
        R += ExportText(TempLine) + '#';
        TempLine.DeleteAll();
        AddLine(TempLine, 1, 5, true, '');
        AddLine(TempLine, 2, 0, true, '');
        AddLine(TempLine, 3, 7.5, false, 'n3');
        exit(R + ExportText(TempLine));
    end;
}
"#;

const ATTR: &str = r#"fieldattribute(equalsValid; Line."Min. Value Equals Valid") { }"#;

fn trigger(extra: &str) -> String {
    format!("trigger OnBeforePassVariable()\n                    begin{SET_TEXT}\n{extra}\n                    end;")
}

fn src(dir: &str, elem: &str, attr: &str, row: &str) -> String {
    let port = PORT
        .replace("{DIR}", dir)
        .replace("{ELEM}", elem)
        .replace("{ATTR}", attr)
        .replace("{ROW}", row);
    format!("{COMMON}{port}")
}

fn measure(src: &str) -> Result<al_builtins::value::Value, Signal> {
    Built::from_source(src).run_in("TEA Run", "Measure")
}

/// The three exports of XpTextElemFieldAttrExport.al:452-462 with `a1`, `a2`, `a3` as the MinValue elements.
fn expect(a1: &str, a2: &str, a3: [&str; 3]) -> String {
    let head = r#"<?xml version="1.0" encoding="UTF-16" standalone="no"?>|<Root>|"#;
    let line = |no: u32, e: &str| format!("  <Line>|    <EntryNo>{no}</EntryNo>|    {e}|  </Line>|");
    format!(
        "{head}{}</Root>#{head}{}</Root>#{head}{}{}{}</Root>",
        line(1, a1),
        line(1, a2),
        line(1, a3[0]),
        line(2, a3[1]),
        line(3, a3[2])
    )
}

fn flagged(src: &str) -> String {
    al_builtins::assumptions::take_hits();
    let got = text(measure(src));
    let hits = al_builtins::assumptions::take_hits();
    assert!(
        hits.contains(&al_builtins::assumptions::Assumption::XmlportTextElemFieldAttrExport),
        "{hits:?}"
    );
    got
}

fn standard() -> String {
    expect(
        r#"<MinValue equalsValid="true">5</MinValue>"#,
        r#"<MinValue equalsValid="true" />"#,
        [
            r#"<MinValue equalsValid="true">5</MinValue>"#,
            r#"<MinValue equalsValid="true" />"#,
            r#"<MinValue equalsValid="false">7.5</MinValue>"#,
        ],
    )
}

/// XpTextElemFieldAttrExport.al:468 (the attribute and the element's own trigger, MinOccurs = Zero) and :498 (MinOccurs not set).
#[test]
fn the_element_with_its_attribute_and_trigger_is_always_written() {
    let zero = format!("MinOccurs = Zero;\n{}", trigger(""));
    assert_eq!(flagged(&src("Export", &zero, ATTR, "")), standard());
    assert_eq!(flagged(&src("Export", &trigger(""), ATTR, "")), standard());
}

/// XpTextElemFieldAttrExport.al:478: the text set in the table element's OnAfterGetRecord.
#[test]
fn the_text_set_in_on_after_get_record_is_written_beside_the_attribute() {
    let row = format!("trigger OnAfterGetRecord()\n                begin{SET_TEXT}\n                end;");
    assert_eq!(flagged(&src("Export", "MinOccurs = Zero;", ATTR, &row)), standard());
}

/// XpTextElemFieldAttrExport.al:508: the attribute is read after OnBeforePassVariable.
#[test]
fn the_attribute_is_read_after_the_trigger() {
    let flip =
        trigger(r#"                        Line."Min. Value Equals Valid" := not Line."Min. Value Equals Valid";"#);
    assert_eq!(
        flagged(&src("Export", &format!("MinOccurs = Zero;\n{flip}"), ATTR, "")),
        expect(
            r#"<MinValue equalsValid="false">5</MinValue>"#,
            r#"<MinValue equalsValid="false" />"#,
            [
                r#"<MinValue equalsValid="false">5</MinValue>"#,
                r#"<MinValue equalsValid="false" />"#,
                r#"<MinValue equalsValid="true">7.5</MinValue>"#,
            ],
        )
    );
}

/// XpTextElemFieldAttrExport.al:518: a blank Text attribute is written as `note=""`.
#[test]
fn a_blank_text_attribute_is_written_empty() {
    let attr = r#"fieldattribute(note; Line.Note) { }"#;
    assert_eq!(
        flagged(&src("Export", &format!("MinOccurs = Zero;\n{}", trigger("")), attr, "")),
        expect(
            r#"<MinValue note="">5</MinValue>"#,
            r#"<MinValue note="" />"#,
            [
                r#"<MinValue note="">5</MinValue>"#,
                r#"<MinValue note="" />"#,
                r#"<MinValue note="n3">7.5</MinValue>"#
            ],
        )
    );
}

/// XpTextElemFieldAttrExport.al:528: the text never assigned (pinned on main through the attribute container).
#[test]
fn the_text_never_assigned_writes_the_attribute_only() {
    let e = r#"<MinValue equalsValid="true" />"#;
    assert_eq!(
        text(measure(&src("Export", "MinOccurs = Zero;", ATTR, ""))),
        expect(e, e, [e, e, r#"<MinValue equalsValid="false" />"#])
    );
}

fn refused(src: &str) -> String {
    match measure(src) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{other:?}"),
    }
}

/// Not measured, so refused: the attribute's own OnBeforePassField, a second attribute, an attribute of another type, a
/// port that also imports, Skip in the element's trigger.
#[test]
fn unmeasured_neighbors_refuse() {
    let elem = format!("MinOccurs = Zero;\n{}", trigger(""));
    let own =
        r#"fieldattribute(equalsValid; Line."Min. Value Equals Valid") { trigger OnBeforePassField() begin end; }"#;
    let two = format!("{ATTR}\nfieldattribute(note; Line.Note) {{ }}");
    let num = r#"fieldattribute(num; Line.Num) { }"#;
    let skip = format!("MinOccurs = Zero;\n{}", trigger("currXMLport.Skip();"));
    let sema = "triggers of the container text element MinValue";
    for (s, want) in [
        (src("Export", &elem, own, ""), sema),
        (src("Export", &elem, &two, ""), sema),
        (src("Both", &elem, ATTR, ""), sema),
        (src("Export", &elem, ATTR, "").replace("UseTemporary = true;", ""), sema),
        (src("Export", &elem, num, ""), sema),
        (src("Export", &skip, ATTR, ""), "Skip, Break or Quit"),
    ] {
        let why = refused(&s);
        assert!(why.contains(want), "{want}: {why}");
    }
}

/// Without a trigger, an attribute of a type the probe does not measure keeps main's refusal of the element's own text.
#[test]
fn an_unmeasured_attribute_type_without_a_trigger_keeps_mains_refusal() {
    let row = format!("trigger OnAfterGetRecord()\n                begin{SET_TEXT}\n                end;");
    let why = refused(&src(
        "Export",
        "MinOccurs = Zero;",
        r#"fieldattribute(num; Line.Num) { }"#,
        &row,
    ));
    assert!(
        why.contains("with a field attribute and its own text below a table element"),
        "{why}"
    );
}

/// Each fact of the probe's ports that the shape requires, flipped one at a time with the element's trigger: main's static
/// refusal stays (XpTextElemFieldAttrExport.al:44-370 declares FormatEvaluate = Xml, no Encoding and no namespace, a text
/// element without XmlName, a Normal field, one such element per table element).
#[test]
fn unmeasured_port_and_element_facts_keep_mains_refusal() {
    let elem = format!("MinOccurs = Zero;\n{}", trigger(""));
    let base = src("Export", &elem, ATTR, "");
    let flow = r#"fieldattribute(hasLine; Line."Has Line") { }"#;
    let entry = r#"fieldelement(EntryNo; Line."Entry No.") { }"#;
    let max_value = format!(
        "{entry}\n                textelement(MaxValue) {{ MaxOccurs = Once; {ATTR} trigger OnBeforePassVariable() begin MaxValue := 'x'; end; }}"
    );
    let sema = "triggers of the container text element";
    for (what, s) in [
        (
            "FormatEvaluate = Legacy",
            base.replace("FormatEvaluate = Xml;", "FormatEvaluate = Legacy;"),
        ),
        ("no FormatEvaluate", base.replace("FormatEvaluate = Xml;", "")),
        (
            "Encoding = UTF8 with a default namespace",
            base.replace(
                "UseRequestPage = false;",
                "UseRequestPage = false; Encoding = UTF8; DefaultNamespace = 'urn:x'; UseDefaultNamespace = true;",
            ),
        ),
        ("a FlowField attribute", src("Export", &elem, flow, "")),
        (
            "XmlName",
            src("Export", &format!("XmlName = 'MinV';\n{elem}"), ATTR, ""),
        ),
        ("two such elements", base.replace(entry, &max_value)),
    ] {
        al_builtins::assumptions::take_hits();
        let why = refused(&s);
        assert!(why.contains(sema), "{what}: {why}");
        assert!(
            !al_builtins::assumptions::take_hits()
                .contains(&al_builtins::assumptions::Assumption::XmlportTextElemFieldAttrExport),
            "{what}"
        );
    }
}

/// A Text attribute is measured with the element's trigger only (XpTextElemFieldAttrExport.al:300); without it main's refusal
/// stays.
#[test]
fn a_text_attribute_without_a_trigger_keeps_mains_refusal() {
    let row = format!("trigger OnAfterGetRecord()\n                begin{SET_TEXT}\n                end;");
    let why = refused(&src(
        "Export",
        "MinOccurs = Zero;",
        r#"fieldattribute(note; Line.Note) { }"#,
        &row,
    ));
    assert!(
        why.contains("with a field attribute and its own text below a table element"),
        "{why}"
    );
}
