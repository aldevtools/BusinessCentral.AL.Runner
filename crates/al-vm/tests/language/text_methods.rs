//! Text methods on a Label or TextConst, `Text.Remove`, and `IndexOfAny` with a start index
//! (corpus types/variant/LTP89TextConst.al, text/textbuilder-bigtext).

use al_testkit::text;

fn run(body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ var Src: Label 'Hello, World'; procedure F(): Text var T: Text; Chars: List of [Char]; begin T := Src; {body} end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn a_label_has_the_text_methods() {
    // LTP89TextConst.al:Search_1, Edit_1, Edit_2, Trim_Family: a Label answers as the Text it holds.
    assert_eq!(
        run(
            "exit(Format(Src.Contains('World')) + '|' + Format(Src.LastIndexOf('o')) + '|' + Src.ToUpper() + '|' + Src.PadLeft(14, '*') + '|' + Src.Replace('l', 'L') + '|' + Src.Substring(8, 3) + '|' + Src.ToLower());"
        ),
        "Yes|9|HELLO, WORLD|**Hello, World|HeLLo, WorLd|Wor|hello, world"
    );
}

#[test]
fn text_remove_is_one_based_with_an_optional_count() {
    // LTP89TextConst.al:Edit_1: remove_t `Hello`, remove_count_t `Helloorld`.
    assert_eq!(
        run("exit(T.Remove(6) + '|' + T.Remove(6, 3) + '|' + Src.Remove(6, 3));"),
        "Hello|Helloorld|Helloorld"
    );
}

#[test]
fn index_of_any_takes_a_one_based_start_index() {
    // LTP89TextConst.al:IndexOfAny_Overloads: `[o, ,]` finds 5 from the start and 6 from position 6.
    assert_eq!(
        run(
            "Chars.Add('o'); Chars.Add(','); exit(Format(T.IndexOfAny(Chars)) + '|' + Format(Src.IndexOfAny(Chars, 6)) + '|' + Format(T.IndexOfAny(Chars, 7)));"
        ),
        "5|6|9"
    );
}

#[test]
fn variant_knows_in_streams_out_streams_and_never_binary() {
    // types/variant/VaTHelper.al Flags (InStream, OutStream) and lt-variant-is-text-streams/LTP89IsMisc.al:Binary_OnManyVariants
    // (IsBinary is No for every value, a stream and a Blob field's value included).
    let src = "table 50101 T { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
        codeunit 50100 C { procedure F(): Text var R: Record T; Ins: InStream; Outs: OutStream; V: Variant; S: Text; begin
          R.Data.CreateInStream(Ins); R.Data.CreateOutStream(Outs);
          V := Ins; S += Format(V.IsInStream()) + Format(V.IsOutStream()) + Format(V.IsBinary()) + '|';
          V := Outs; S += Format(V.IsInStream()) + Format(V.IsOutStream()) + Format(V.IsBinary()) + '|';
          V := 5; S += Format(V.IsInStream()) + Format(V.IsOutStream()) + Format(V.IsBinary());
          exit(S); end; }";
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "YesNoNo|NoYesNo|NoNoNo"
    );
}

#[test]
fn a_placeholder_without_an_argument_stays_as_written() {
    // types/variant/VaTTextConst.al:TextConst_StrSubstNo.
    let src = "codeunit 50100 C { procedure F(): Text var W: Text; begin W := 'Value %1 and %2';
        exit(StrSubstNo(W, 'a') + '|' + StrSubstNo(W) + '|' + StrSubstNo('%10 and %1', 'a') + '|' + StrSubstNo('%0 and %1', 'a') + '|' + StrSubstNo('%5', 'a') + '|' + StrSubstNo('end%', 'a') + '|' + StrSubstNo(W, '%2', 'x') + '|' + StrSubstNo('%01', 'a') + '|' + StrSubstNo('%%1', 'a')); end; }";
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "Value a and %2|Value %1 and %2|%10 and a|%0 and a|%5|end%|Value %2 and x|%01|%a"
    );
}

#[test]
fn a_hash_field_is_filled_by_strsubstno_and_refused_in_error() {
    // BC reads `#1####` as a field of width six (text/text-methods/StrSubstNo.Codeunit.al:SubstNoHash,
    // P86StrSubstNo.al:Sub_FieldWidth). Only StrSubstNo is pinned: Error with such a field stays refused.
    let src = "codeunit 50100 C { procedure F(): Text begin exit(StrSubstNo('[#1####]', 'a')); end; }";
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "[a     ]"
    );
    let src = "codeunit 50100 C { procedure F(): Text begin Error('#1####', 'a'); end; }";
    assert!(matches!(
        al_testkit::run_with(src, "C", "F", vec![]),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}

#[test]
fn data_transfer_update_audit_fields_is_accepted_after_set_tables_outside_install_code() {
    // install-upgrade/W5DataTransferOutside.al:SetTables_Twice_AndUpdateAuditFields (`DT.UpdateAuditFields := false`),
    // errors/PnBErrorsCollections.al:DataTransfer_OutsideInstall op6 (`UpdateAuditFields(true)`).
    let src = "table 50101 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 50100 C { procedure F(): Text var DT: DataTransfer; begin DT.SetTables(Database::T, Database::T); DT.UpdateAuditFields := false; DT.UpdateAuditFields(true); exit('ok'); end; }";
    assert_eq!(text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)), "ok");
}

#[test]
fn a_one_character_literal_in_a_variant_is_a_char_and_clear_empties_a_variant() {
    // types/lt-variant/VaTIs*.al:Is_Text_Code_Char_Byte (`V := 'a'` is Char; `'ab'`, `''` and a Text variable are Text),
    // :Is_Empty_Variant (`V := 5; Clear(V)` leaves no type); LTP89IsMisc.al:WideChar_Variants and NearTypes (Abs keeps its type).
    let src = "codeunit 50100 C { procedure F(): Text var V: Variant; T: Text; I: Integer; S: Text; begin
        V := 'a'; S += Format(V.IsChar()) + Format(V.IsText()) + Format(V.IsWideChar()) + '|';
        V := 'ab'; S += Format(V.IsChar()) + Format(V.IsText()) + '|';
        T := 'a'; V := T; S += Format(V.IsChar()) + Format(V.IsText()) + '|';
        V := 5; Clear(V); S += Format(V.IsInteger()) + '|';
        I := -3; V := Abs(I); S += Format(V.IsInteger()) + Format(V.IsDecimal());
        exit(S); end; }";
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "YesNoYes|NoYes|NoYes|No|YesNo"
    );
}

#[test]
fn a_text_builder_has_a_capacity_of_16_that_can_be_set_and_that_clear_keeps() {
    // text/textbuilder-bigtext/TxCTextBuilderBasics.al:Capacity_Defaults, Capacity_Setter, Clear_Basics.
    let src = "codeunit 50100 C { procedure F(): Text var TB: TextBuilder; S: Text; begin
        S += Format(TB.Capacity()); TB.Append('abc'); S += '|' + Format(TB.Capacity());
        TB.Capacity(100); S += '|' + Format(TB.Capacity()); TB.Clear(); S += '|' + Format(TB.Capacity()) + '|' + TB.ToText() + '|' + Format(TB.Length());
        exit(S); end; }";
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "16|16|100|100||0"
    );
}

#[test]
fn data_transfer_errors_carry_their_code_and_ensure_capacity_grows() {
    // install-upgrade/W5DataTransferOutside.al (DB:Ok), text/textbuilder-bigtext/TxCTextBuilderBasics.al:EnsureCapacity_Basics.
    let src = "table 50101 T { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
        codeunit 50100 C { procedure F(): Text var DT: DataTransfer; TB: TextBuilder; S: Text; begin
        DT.SetTables(Database::T, Database::T); asserterror DT.CopyRows(); S := GetLastErrorCode();
        TB.EnsureCapacity(50); S += '|' + Format(TB.Capacity() >= 50); asserterror TB.EnsureCapacity(2147483647 + 1);
        exit(S); end; }";
    let r = al_testkit::run_with(src, "C", "F", vec![]);
    let s = match r {
        Ok((al_builtins::value::Value::Text(s), _)) => s.to_string(),
        other => panic!("{other:?}"),
    };
    assert_eq!(s, "DB:Ok|Yes");
}

#[test]
fn ensure_capacity_of_a_negative_number_answers_false_when_the_answer_is_used() {
    // format-evaluate/AxFillerBuilderAutoFormat.al:156-185; P144TextBuilderCapacity.al:160-173 (a statement on a
    // builder that holds text raises).
    let src = "codeunit 50100 C { procedure F(): Text var Empty: TextBuilder; Abc: TextBuilder; B1: Boolean; B2: Boolean; S: Text; begin
        Abc.Append('abc'); B1 := Empty.EnsureCapacity(-1); B2 := Abc.EnsureCapacity(-1);
        S := Format(B1) + '|' + Format(B2) + '|' + Format(Abc.Capacity());
        asserterror Abc.EnsureCapacity(-1); exit(S + '|' + GetLastErrorCode()); end; }";
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "No|No|16|ArgumentOutOfRange"
    );
}

#[test]
fn remove_and_pad_outside_the_text_raise_the_dotnet_argument_errors() {
    // text/text-methods/Edit1.Codeunit.al:41-63, 216-330.
    let src = r#"codeunit 50100 C {
        procedure E(T: Text; S: Integer; N: Integer; Two: Boolean): Text var R: Text; begin
            if Two then R := T.Remove(S, N) else R := T.Remove(S); exit(R); end;
        procedure Err(T: Text; S: Integer; N: Integer; Two: Boolean): Text begin
            asserterror E(T, S, N, Two); exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;
        procedure F(): Text var T: Text; begin
            T := 'abcdef';
            exit(E(T, 7, 0, false) + '#' + Err(T, 8, 0, false) + '#' + Err(T, 0, 0, false) + '#' + E(T, 7, 0, true) + '#'
                + Err(T, 4, 4, true) + '#' + Err(T, 0, 1, true) + '#' + Err(T, 2, -1, true));
        end;
        procedure P(): Text var T: Text; begin T := 'ab'; asserterror T := T.PadLeft(-5);
            exit(GetLastErrorText() + '|' + GetLastErrorCode()); end; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "abcdef#startIndex cannot be larger than length of string. (Parameter 'startIndex')|ArgumentOutOfRange#\
         StartIndex cannot be less than 1. (Parameter 'startIndex')|ArgumentOutOfRange#abcdef#\
         count ('4') must be less than or equal to '3'. (Parameter 'count')\\Actual value was 4.|ArgumentOutOfRange#\
         startIndex ('-1') must be a non-negative value. (Parameter 'startIndex')\\Actual value was -1.|ArgumentOutOfRange#\
         count ('-1') must be a non-negative value. (Parameter 'count')\\Actual value was -1.|ArgumentOutOfRange"
    );
    assert_eq!(
        text(al_testkit::run_with(src, "C", "P", vec![]).map(|(v, _)| v)),
        "totalWidth ('-5') must be a non-negative value. (Parameter 'totalWidth')\\Actual value was -5.|ArgumentOutOfRange"
    );
}

#[test]
fn text_builder_length_setter_replace_in_a_range_and_to_text_from_zero() {
    // text/textbuilder-bigtext/TxCTextBuilderBasics.al:230-257; TxCTextBuilderEdit.al:235-330.
    let src = r#"codeunit 50100 C { procedure F(): Text var TB: TextBuilder; S: Text; Z: Text; begin
        TB.Append('abcdef'); TB.Length(3); S := TB.ToText(); TB.Length(5); Z := TB.ToText();
        S += '|' + Format(StrLen(Z)) + '|' + Format(Z[4] = 0);
        TB.Clear(); TB.Append('aXaXaXaX'); TB.Replace('X', '-', 2, 4); S += '|' + TB.ToText();
        TB.Clear(); TB.Append('aXaXaXaX'); TB.Replace('aX', '-', 1, 4); S += '|' + TB.ToText();
        asserterror TB.Replace('X', '-', 0, 2); S += '|' + GetLastErrorCode() + '|' + TB.ToText();
        TB.Clear(); TB.Append('0123456789'); S += '|' + TB.ToText(0, 10);
        exit(S); end; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "abc|5|Yes|a-a-aXaX|--aXaX|ArgumentOutOfRange|--aXaX|0123456789"
    );
}

#[test]
fn incstr_below_zero_and_padstr_ranges_raise() {
    // text/text-methods/IncStr.Codeunit.al:141-174; PadStr.Codeunit.al:37-73.
    let src = r#"codeunit 50100 C { procedure F(): Text var S: Text; begin
        asserterror S := IncStr('5', -6); S := GetLastErrorText() + '|' + GetLastErrorCode();
        asserterror S += PadStr('abc', 6, 'xy'); S += '#' + GetLastErrorText();
        asserterror S += PadStr('abc', -1); S += '#' + GetLastErrorText();
        exit(S); end; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "The result of incrementing cannot be negative.|InvalidOperation#\
         The value of PADSTR parameter 3 is outside of the permitted range. The current value is: 2. The permitted range is: from 1 to 1.#\
         The value of PADSTR parameter 2 is outside of the permitted range. The current value is: -1. The permitted range is: from 0 to 2,147,483,647."
    );
}

#[test]
fn to_text_of_integers_guids_and_bytes() {
    // format-evaluate/P93ScalarToText.al:180-300, P93Guid.al:28-102.
    let src = r#"codeunit 50100 C { procedure F(): Text var I: Integer; B: BigInteger; G: Guid; Y: Byte; begin
        I := 1000; B := -9223372036854775807L; B := B - 1; Y := 65;
        Evaluate(G, 'e6c0bda3-5b0a-4a7e-9a5e-0123456789ab');
        exit(I.ToText() + '|' + B.ToText() + '|' + G.ToText() + '|' + G.ToText(false) + '|' + Y.ToText(true)); end; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "1000|-9223372036854775808|{E6C0BDA3-5B0A-4A7E-9A5E-0123456789AB}|{E6C0BDA3-5B0A-4A7E-9A5E-0123456789AB}|A"
    );
}

#[test]
fn evaluate_reads_a_guid_source_as_its_text() {
    // types/duration/E_Guid.al:224-311.
    let src = r#"codeunit 50100 C { procedure F(): Text var G: Guid; G2: Guid; T: Text; Ok: Boolean; begin
        Evaluate(G, '{ab12cd34-ef56-4a78-9b01-23456789abcd}'); Ok := Evaluate(T, G);
        exit(Format(Ok) + '|' + T + '|' + Format(Evaluate(G2, G)) + '|' + Format(G2 = G)); end; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "Yes|{AB12CD34-EF56-4A78-9B01-23456789ABCD}|Yes|Yes"
    );
}

#[test]
fn format_of_an_option_field_whose_caption_list_is_shorter_or_longer() {
    // format-evaluate/FormatOptionFieldCaptions.al:82-177.
    let src = r#"table 50101 T { fields { field(1; Id; Integer) { }
            field(4; Short; Option) { OptionMembers = One,Two,Three; OptionCaption = 'Uno,Dos'; }
            field(5; Long; Option) { OptionMembers = One,Two; OptionCaption = 'Uno,Dos,Tres,Quatro'; } } keys { key(PK; Id) { } } }
        codeunit 50100 C { procedure F(): Text var R: Record T; begin
            R.Short := R.Short::Two; R.Long := R.Long::Two; S := Format(R.Short) + '|' + Format(R.Long);
            R.Short := R.Short::Three; R.Long := 2; exit(S + '|' + Format(R.Short) + '|' + Format(R.Long)); end;
        var S: Text; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "Dos|Dos|Three|2"
    );
}

#[test]
fn big_text_add_text_at_a_position_and_get_sub_text_bounds() {
    // format-evaluate/P93BigText.al:95-388.
    let src = r#"codeunit 50100 C { procedure F(): Text var B: BigText; O: BigText; T: Text; S: Text; N: Integer; begin
        B.AddText('abcdef'); B.AddText('XY', 3); B.GetSubText(T, 1); S := T;
        B.AddText(B, 100); B.GetSubText(T, 1); S += '|' + T;
        N := B.GetSubText(T, 17, 1); S += '|' + Format(N) + '[' + T + ']';
        N := B.GetSubText(O, 2, 3); O.GetSubText(T, 1); S += '|' + Format(N) + T;
        asserterror B.AddText('Z', 0); S += '|' + GetLastErrorCode();
        asserterror N := B.GetSubText(T, 0); S += '|' + GetLastErrorText();
        S += '|' + Format(B.TextPos('')); exit(S); end; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "abXYcdef|abXYcdefabXYcdef|0[]|3bXY|OutsidePermittedRange|The value of COPYSTR parameter 2 is outside of the permitted range. The current value is: 0. The permitted range is: from 1 to 2,147,483,647.|0"
    );
}

#[test]
fn last_index_of_the_empty_text_and_index_of_any_one_past_the_end() {
    // text/text-methods/Search1.Codeunit.al:418-487.
    let src = r#"codeunit 50100 C { procedure F(): Text var T: Text; E: Text; L: List of [Char]; C: Char; begin
        T := 'hello'; C := 'o'; L.Add(C);
        exit(Format(E.LastIndexOf('', 0)) + Format(T.LastIndexOf('', 5)) + Format(T.LastIndexOf('', 6)) + Format(T.IndexOfAny(L, 6))); end; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "1660"
    );
}

#[test]
fn a_big_integer_in_a_set_of_integers_and_time_plus_a_long_duration() {
    // types/operators-expressions/P79BigOption.al:401, 511-529; D_Duration.al:463-474.
    let src = r#"codeunit 50100 C { procedure F(): Text var B: BigInteger; T: Time; D: Duration; S: Text; begin
        B := 2; S := Format(B in [0, 1, 2]);
        B := 4294967296L; asserterror S += Format(B in [0, 1, 2]); S += '|' + GetLastErrorText();
        T := 100000T; D := 5000000000L; asserterror T := T + D; exit(S + '|' + GetLastErrorCode()); end; }"#;
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "Yes|Overflow under conversion of System.Int64 value 4294967296 to System.Int32.|ConversionCast"
    );
}
