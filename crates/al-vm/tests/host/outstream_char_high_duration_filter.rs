//! OutStream.Write of a Char above 127 with a length and Write(Text, Length) of non-ASCII text (al-runner issue 7387:
//! streams/instream-outstream/StreamLengthOthers.al Char_WriteReadLength_High_1, _2, Text_WriteLength_NonAscii), FieldRef.SetFilter
//! of Duration texts (issue 7388: recordref-fieldref/fieldref/TestFieldRefFilter.al FieldRef_SetFilter_Zero_Duration, _DurationTexts),
//! all corpus PR 1248; Decimal fields with an AutoFormat and DecimalPlaces (issue 7404: format-evaluate/format-matrix/FMFields.al
//! StrSubstNo_Format_DecimalField_*, corpus PR 1256).

use al_testkit::{run, text};

const SRC: &str = r#"
table 60019 "OCH Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
        field(3; Dur; Duration) { }
        field(4; DecAf2Places; Decimal) { AutoFormatType = 2; DecimalPlaces = 0 : 5; }
        field(5; DecAf2Expr; Decimal) { AutoFormatType = 2; AutoFormatExpression = ''; DecimalPlaces = 2 : 2; }
    }
    keys { key(PK; Id) { } }
}
codeunit 50100 C
{
    local procedure Hex(var B: Record "OCH Row" temporary): Text
    var InS: InStream; Byte1: Byte; Code: Integer; R: Text;
    begin
        B.Data.CreateInStream(InS);
        while not InS.EOS() do begin
            InS.Read(Byte1);
            Code := Byte1;
            R += Format(Code) + ' ';
        end;
        exit(R);
    end;

    local procedure CharBytes(Code: Integer): Text
    var B: Record "OCH Row" temporary; OutS: OutStream; C: Char; Ret: Integer;
    begin
        C := Code;
        B.Data.CreateOutStream(OutS);
        Ret := OutS.Write(C, 1);
        exit(Format(Ret) + ':' + Hex(B));
    end;

    local procedure TextBytes(Value: Text; Len: Integer): Text
    var B: Record "OCH Row" temporary; OutS: OutStream; Ret: Integer;
    begin
        B.Data.CreateOutStream(OutS);
        Ret := OutS.Write(Value, Len);
        exit(Format(Ret) + ':' + Hex(B));
    end;

    procedure Chars(): Text
    begin
        exit(CharBytes(182) + '|' + CharBytes(128) + '|' + CharBytes(255) + '|' + CharBytes(256) + '|' + CharBytes(8364) + '|' + CharBytes(127));
    end;

    procedure Texts(): Text
    var P: Text;
    begin
        P[1] := 182;
        exit(TextBytes(P, 1) + '|' + TextBytes(P, 2) + '|' + TextBytes('a' + P + 'b', 3) + '|' + TextBytes('é', 1) + '|' + TextBytes('€', 1));
    end;

    local procedure Filter(Txt: Text): Text
    var R: Record "OCH Row" temporary; RecRef: RecordRef; F: FieldRef; Ids: Text;
    begin
        R.Id := 1; R.Dur := 0; R.Insert();
        R.Id := 2; R.Dur := 1000; R.Insert();
        R.Id := 3; R.Dur := 60000; R.Insert();
        RecRef.GetTable(R);
        F := RecRef.Field(3);
        F.SetFilter(Txt);
        if RecRef.FindSet() then
            repeat
                Ids += Format(RecRef.Field(1).Value) + ',';
            until RecRef.Next() = 0;
        exit(F.GetFilter() + '=' + Ids);
    end;

    procedure Filters(): Text
    begin
        exit(Filter('0') + '|' + Filter('<>0') + '|' + Filter('<>''''') + '|' + Filter('1000') + '|' + Filter('0..1000') + '|' + Filter('1S') + '|' +
             Filter('1M') + '|' + Filter('1 minute') + '|' + Filter(' 0 ') + '|' + Filter('-1') + '|' + Filter('0|1000'));
    end;

    procedure AutoFormats(): Text
    var R: Record "OCH Row";
    begin
        R.DecAf2Expr := 1234.5;
        exit(StrSubstNo('%1', R.DecAf2Expr) + '|' + Format(R.DecAf2Expr));
    end;
}
"#;

#[test]
fn a_char_above_127_with_a_length_writes_its_code_page_byte() {
    assert_eq!(text(run(SRC, "Chars")), "1:20 |1:63 |1:152 |1:65 |1:177 |1:127 ");
}

#[test]
fn write_text_with_a_length_of_non_ascii_text_writes_the_code_page_bytes() {
    assert_eq!(
        text(run(SRC, "Texts")),
        "2:20 0 |2:20 0 |4:97 20 98 0 |2:130 0 |2:177 0 "
    );
}

#[test]
fn a_duration_filter_reads_numbers_as_hours_and_s_and_m_as_seconds_and_minutes() {
    assert_eq!(
        text(run(SRC, "Filters")),
        "''=1,|<>''=2,3,|<>''=2,3,|41 days 16 hours=|''..41 days 16 hours=1,2,3,|1 second=2,|1 minute=3,|1 minute=3,|''=1,|-1 hour=|''|41 days 16 hours=1,"
    );
}

#[test]
fn a_unit_amount_with_decimal_places_takes_the_auto_format() {
    // DecAf2Places takes General Ledger Setup's places, which needs Base Application: the corpus run covers it.
    assert_eq!(text(run(SRC, "AutoFormats")), "1,234.50|1,234.50");
}
