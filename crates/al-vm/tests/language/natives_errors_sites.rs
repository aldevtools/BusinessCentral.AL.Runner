//! What the natives-and-errors package answers only at the sites the corpus measured, and refuses everywhere else as `main`
//! did: the `GetLastErrorText(true)` text by raise site (errors/last-error/LastErrorFlagRuntimeCodes.al), the unassigned
//! Variant and the Variant record slots at `:=` only (types/variant/VaTConvert.al, VariantRecordIdConversions.al), a Variant
//! as a condition at the AL condition only (VariantRecordIdConversions.al:238-265), and the streams that were never created
//! (streams/instream-outstream/StreamUncreatedUse.al).

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69890 "NE Row" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 69891 "NE C"
{
    local procedure Flag(): Text begin exit(GetLastErrorText(true)); end;

    // The sites LastErrorFlagRuntimeCodes.al measured.
    procedure ListGet(): Text var L: List of [Integer]; I: Integer; begin asserterror I := L.Get(3); exit(Flag()); end;
    procedure Substring(): Text var T: Text; R: Text; begin T := 'abc'; asserterror R := T.Substring(9); exit(Flag()); end;
    procedure PadLeft(): Text var T: Text; R: Text; begin T := 'abc'; asserterror R := T.PadLeft(-1); exit(Flag()); end;
    procedure ArrayWrite(): Text var A: array[3] of Integer; I: Integer; begin I := 4; asserterror A[I] := 1; exit(Flag()); end;
    procedure JsonRemoveAt(): Text var JA: JsonArray; begin JA.Add(1); asserterror JA.RemoveAt(5); exit(Flag()); end;
    procedure DictGet(): Text var D: Dictionary of [Text, Integer]; I: Integer; begin asserterror I := D.Get('missing key'); exit(Flag()); end;
    procedure DictAddTwice(): Text var D: Dictionary of [Text, Integer]; begin D.Add('k', 1); asserterror D.Add('k', 2); exit(Flag()); end;
    procedure VariantTextInt(): Text var V: Variant; I: Integer; begin V := 'abc'; asserterror I := V; exit(Flag()); end;
    procedure Dmy2DateMonth(): Text var D: Date; begin asserterror D := DMY2Date(40, 13, 2020); exit(Flag()); end;
    procedure EvalGuid(): Text var G: Guid; begin asserterror Evaluate(G, 'not a guid'); exit(Flag()); end;

    // Sites nothing measured, which share a code and a text with a measured one.
    procedure ListRemoveAt(): Text var L: List of [Integer]; begin L.Add(1); asserterror L.RemoveAt(9); exit(Flag()); end;
    procedure ListSet(): Text var L: List of [Integer]; begin L.Add(1); asserterror L.Set(9, 1); exit(Flag()); end;
    procedure ListGetVar(): Text var L: List of [Integer]; I: Integer; begin asserterror L.Get(3, I); exit(Flag()); end;
    procedure PadRight(): Text var T: Text; R: Text; begin T := 'abc'; asserterror R := T.PadRight(-1); exit(Flag()); end;
    procedure SubstringCount(): Text var T: Text; R: Text; begin T := 'abc'; asserterror R := T.Substring(9, 1); exit(Flag()); end;
    procedure Dmy2DateDay(): Text var D: Date; begin asserterror D := DMY2Date(40, 1, 2020); exit(Flag()); end;
    procedure JsonGet(): Text var JA: JsonArray; T: JsonToken; begin JA.Add(1); asserterror JA.Get(5, T); exit(Flag()); end;
    procedure ArrayRead(): Text var A: array[3] of Integer; I: Integer; J: Integer; begin I := 4; asserterror J := A[I]; exit(Flag()); end;
    procedure VariantEmptyInt(): Text var V: Variant; I: Integer; begin V := ''; asserterror I := V; exit(Flag()); end;
    procedure DictGetVar(): Text var D: Dictionary of [Text, Integer]; I: Integer; begin asserterror D.Get('missing key', I); exit(Flag()); end;

    // Finds on an empty set: through a Record (measured) and through a RecordRef (not measured, al-runner issue 7806).
    procedure RecFindFirstFiltered(): Text var Row: Record "NE Row"; begin Row.SetRange(Id, 42); asserterror Row.FindFirst(); exit(Flag()); end;
    procedure RecFindFirstEmpty(): Text var Row: Record "NE Row"; begin asserterror Row.FindFirst(); exit(Flag()); end;
    procedure RefFindFirstFiltered(): Text var Ref: RecordRef; begin Ref.Open(69890); Ref.Field(1).SetRange(42); asserterror Ref.FindFirst(); exit(Flag()); end;
    procedure RefFindSetFiltered(): Text var Ref: RecordRef; begin Ref.Open(69890); Ref.Field(1).SetRange(42); asserterror Ref.FindSet(); exit(Flag()); end;
    procedure RefFindMinusFiltered(): Text var Ref: RecordRef; begin Ref.Open(69890); Ref.Field(1).SetRange(42); asserterror Ref.Find('-'); exit(Flag()); end;
    procedure RefFindFirstEmpty(): Text var Ref: RecordRef; begin Ref.Open(69890); asserterror Ref.FindFirst(); exit(Flag()); end;
    procedure RefFindLastEmpty(): Text var Ref: RecordRef; begin Ref.Open(69890); asserterror Ref.FindLast(); exit(Flag()); end;
    procedure RefFindEqualEmpty(): Text var Ref: RecordRef; begin Ref.Open(69890); asserterror Ref.Find('='); exit(Flag()); end;
    procedure RefFindFirstFilteredPlain(): Text var Ref: RecordRef; begin Ref.Open(69890); Ref.Field(1).SetRange(42); asserterror Ref.FindFirst(); exit(GetLastErrorText(false)); end;

    // A Variant record slot outside `:=`.
    [TryFunction]
    local procedure TakeRow(Row: Record "NE Row") begin end;
    procedure TextVariantToRecordParameter(): Text var V: Variant; begin V := 'abc'; if TakeRow(V) then exit('ok'); exit('failed'); end;
    local procedure GiveRow(V: Variant) Row: Record "NE Row" begin exit(V); end;
    procedure TextVariantExitAsRecord(): Text var V: Variant; Row: Record "NE Row"; begin V := 'abc'; Row := GiveRow(V); exit('ok'); end;

    // An unassigned Variant outside `:=`.
    local procedure TakeDecimal(D: Decimal): Decimal begin exit(D); end;
    procedure UnassignedToDecimalParameter(): Text var V: Variant; begin exit(Format(TakeDecimal(V))); end;

    // A Variant as a condition.
    procedure IfUnassigned(): Text var V: Variant; begin if V then exit('then'); exit('else'); end;
    procedure IfInteger1(): Text var V: Variant; begin V := 1; if V then exit('then'); exit('else'); end;
    procedure IfInteger2(): Text var V: Variant; begin V := 2; if V then exit('then'); exit('else'); end;
    procedure IfDate(): Text var V: Variant; begin V := Today(); if V then exit('then'); exit('else'); end;

    // Streams: never created, and cleared.
    procedure WriteTextNoArgUsed(): Text var O: OutStream; N: Integer; begin N := O.WriteText(); exit(Format(N)); end;
    procedure WriteTextLenUsed(): Text var O: OutStream; N: Integer; begin N := O.WriteText('abc', 2); exit(Format(N)); end;
    procedure WriteLenUsed(): Text var O: OutStream; N: Integer; begin N := O.Write('abc', 2); exit(Format(N)); end;
    procedure WriteRecordUsed(): Text var O: OutStream; Row: Record "NE Row"; N: Integer; begin N := O.Write(Row); exit(Format(N)); end;
    procedure WriteTextOneUsed(): Text var O: OutStream; N: Integer; begin N := O.WriteText('x'); exit(Format(N)); end;
    procedure WriteIntUsed(): Text var O: OutStream; N: Integer; begin N := O.Write(5); exit(Format(N)); end;
    procedure WriteTextCleared(): Text var Row: Record "NE Row"; O: OutStream; N: Integer; begin Row.Data.CreateOutStream(O); Clear(O); N := O.WriteText('x'); exit(Format(N)); end;
    procedure EosCleared(): Text var Row: Record "NE Row"; I: InStream; begin Row.Data.CreateInStream(I); Clear(I); exit(Format(I.EOS())); end;
    procedure LengthCleared(): Text var Row: Record "NE Row"; I: InStream; begin Row.Data.CreateInStream(I); Clear(I); asserterror I.Length(); exit(GetLastErrorCode()); end;
    procedure ReadCodeBigLength(): Text var Row: Record "NE Row"; O: OutStream; I: InStream; C: Code[10]; Big: BigInteger; N: Integer; begin
        Row.Data.CreateOutStream(O); O.WriteText('abcdef'); Row.Data.CreateInStream(I); Big := 3; N := I.Read(C, Big); exit(Format(N) + C); end;
    procedure ReadUtf16BigLength(): Text var Row: Record "NE Row"; O: OutStream; I: InStream; T: Text; Big: BigInteger; N: Integer; begin
        Row.Data.CreateOutStream(O, TextEncoding::UTF16); O.WriteText('abcdef'); Row.Data.CreateInStream(I, TextEncoding::UTF16); Big := 3; N := I.Read(T, Big); exit(Format(N) + T); end;
    procedure ReadTextBigLength(): Text var Row: Record "NE Row"; O: OutStream; I: InStream; T: Text; Big: BigInteger; N: Integer; begin
        Row.Data.CreateOutStream(O); O.WriteText('abcdef'); Row.Data.CreateInStream(I); Big := 3; N := I.Read(T, Big); exit(Format(N) + T); end;
}
"#;

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    run_in(SRC, "NE C", routine)
}

fn refused(routine: &str) {
    match run(routine) {
        Err(Signal::Unsupported(_)) => {}
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

const NOT_CLASSIFIED: &str =
    "Message not shown because the NavBaseException constructor was used without privacy classification";

#[test]
fn the_flag_answers_at_the_measured_raise_sites() {
    // LastErrorFlagRuntimeCodes.al:161-176 (List.Get, Substring, PadLeft) and :210-217 (Dictionary.Get).
    for routine in ["ListGet", "Substring", "PadLeft", "DictGet"] {
        assert_eq!(text(run(routine)), NOT_CLASSIFIED, "{routine}");
    }
    // :188-199 (an array write, JsonArray.RemoveAt), :213-220 (Dictionary.Add twice), :233-244 (a Variant holding 'abc' into
    // an Integer), :238-250 (Evaluate into a Guid): the text as it is.
    assert_eq!(text(run("ArrayWrite")), "Index out of bounds.");
    assert_eq!(text(run("JsonRemoveAt")), "Index out of bounds.");
    assert_eq!(
        text(run("DictAddTwice")),
        "An item with the same key has already been added."
    );
    assert_eq!(
        text(run("VariantTextInt")),
        "The expression Variant cannot be type-converted to a Integer value."
    );
    assert!(text(run("EvalGuid")).starts_with("Invalid format of GUID string."));
    // :236-247: DMY2Date hides the value of parameter 2 and keeps its range.
    assert_eq!(
        text(run("Dmy2DateMonth")),
        "The value of DMY2Date parameter 2 is outside of the permitted range. The current value is: <redacted>. The permitted range is: from 1 to 12."
    );
}

#[test]
fn the_flag_refuses_unmeasured_sites_that_share_a_measured_text() {
    for routine in [
        "ListRemoveAt",
        "ListSet",
        "ListGetVar",
        "PadRight",
        "SubstringCount",
        "Dmy2DateDay",
        "JsonGet",
        "ArrayRead",
        "VariantEmptyInt",
        "DictGetVar",
    ] {
        refused(routine);
    }
}

#[test]
fn the_flag_refuses_recordref_finds_on_an_empty_set() {
    // The Record cells are measured (LastErrorFlagRuntimeCodes.al:NothingInsideFilter_EmptyTable_ExcludeFlag, P86LastError.al:339-353).
    assert_eq!(
        text(run("RecFindFirstFiltered")),
        "There is no NE Row within the filter.\\\\Filters: <redacted>"
    );
    assert_eq!(text(run("RecFindFirstEmpty")), "The NE Row table is empty.");
    // The same finds through a RecordRef are not measured: the flagged text refuses (al-runner issue 7806).
    for routine in [
        "RefFindFirstFiltered",
        "RefFindSetFiltered",
        "RefFindMinusFiltered",
        "RefFindFirstEmpty",
        "RefFindLastEmpty",
        "RefFindEqualEmpty",
    ] {
        refused(routine);
    }
    // The error itself and its text without the flag stay as on main.
    assert_eq!(
        text(run("RefFindFirstFilteredPlain")),
        "There is no NE Row within the filter.\\\\Filters: Id: 42"
    );
}

#[test]
fn a_variant_record_slot_outside_an_assignment_is_not_converted_here() {
    // Only `:=` is measured (types/variant/VariantRecordIdConversions.al, VaTConvert.al:76-101): an argument and `exit` do not
    // raise the record-slot errors.
    for routine in ["TextVariantToRecordParameter", "TextVariantExitAsRecord"] {
        if let Err(Signal::Error(e)) = run(routine) {
            assert!(
                !e.message.contains("INavRecordHandle"),
                "{routine}: the assignment's error outside an assignment: {}",
                e.message
            );
        }
    }
}

#[test]
fn an_unassigned_variant_outside_an_assignment_keeps_mains_path() {
    // VaTConvert.al:290-301 pins NotInitialized for `Dec := V`, not for a Decimal parameter.
    assert!(
        !matches!(run("UnassignedToDecimalParameter"), Err(Signal::Error(e)) if e.code == "NotInitialized"),
        "an argument took the assignment's rule"
    );
}

#[test]
fn a_variant_as_a_condition_answers_the_measured_values_and_refuses_the_others() {
    // VariantRecordIdConversions.al:238-265.
    match run("IfUnassigned") {
        Err(Signal::Error(e)) => assert_eq!(e.code, "NotInitialized"),
        other => panic!("{other:?}"),
    }
    assert_eq!(text(run("IfInteger1")), "then");
    refused("IfInteger2");
    refused("IfDate");
}

#[test]
fn a_null_in_a_shared_truth_test_is_refused_as_on_main() {
    // `ops::truthy` serves page properties, MarkedOnly, RecordRef.Open and record results too: no Variant rule there.
    assert!(matches!(
        al_builtins::ops::truthy(&al_builtins::value::Value::Null),
        Err(Signal::Unsupported(_))
    ));
    assert!(matches!(
        al_builtins::ops::truthy(&al_builtins::value::Value::Integer(1)),
        Err(Signal::Unsupported(_))
    ));
}

#[test]
fn uncreated_streams_answer_only_the_measured_forms() {
    // StreamUncreatedUse.al:57-60, :123-124: WriteText(Text) and Write(Integer) answer 0.
    assert_eq!(text(run("WriteTextOneUsed")), "0");
    assert_eq!(text(run("WriteIntUsed")), "0");
    for routine in [
        "WriteTextNoArgUsed",
        "WriteTextLenUsed",
        "WriteLenUsed",
        "WriteRecordUsed",
        "WriteTextCleared",
        "EosCleared",
        "ReadCodeBigLength",
        "ReadUtf16BigLength",
    ] {
        refused(routine);
    }
    // Y62GoneInStream.al:GoneStream_Clear: Length of a cleared InStream raises NotInitialized, as before.
    assert_eq!(text(run("LengthCleared")), "NotInitialized");
    // StreamUncreatedUse.al:162-179: a BigInteger length into a Text reads as an Integer length.
    assert_eq!(text(run("ReadTextBigLength")), "3abc");
}
