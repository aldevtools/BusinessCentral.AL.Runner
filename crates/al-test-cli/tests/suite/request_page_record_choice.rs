//! A request page control bound to an Option or Enum field of a global record of the report (al-runner issue 7376) takes the
//! entries reports/request-page/RequestPageOptionFieldOfGlobalRecord.al pins; the entries it does not pin are refused
//! (`pages/choice_entry.rs`).

use al_testrun::Outcome;

const SRC: &str = r#"
enum 69890 "RC Kind" { Extensible = false; value(0; None) { } value(1; First) { Caption = 'First one'; } }
table 69891 "RC Setup"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Pos; Option) { OptionMembers = "No Logo",Left; }
        field(3; Kind; Enum "RC Kind") { }
        field(4; "Long Name"; Text[30]) { }
    }
    keys { key(PK; "Code") { } }
}
table 69892 "RC Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 69893 "RC Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RC Row") { } dataitem(Item; "RC Setup") { } }
    requestpage
    {
        layout
        {
            area(Content)
            {
                field(PosCtl; Setup.Pos) { ApplicationArea = All; }
                field(KindCtl; Setup.Kind) { ApplicationArea = All; }
                field(ItemCtl; Item.Pos) { ApplicationArea = All; }
                field(TempCtl; TempSetup.Pos) { ApplicationArea = All; }
                field(ArrKindCtl; Arr[2].Kind) { ApplicationArea = All; }
                field(NeCtl; Setup.Pos) { ApplicationArea = All; Editable = false; }
                field(CapCtl; Setup.Pos) { ApplicationArea = All; Caption = 'Position'; }
                field(OptCapCtl; Setup.Pos) { ApplicationArea = All; OptionCaption = 'A,B'; }
                field(ValCtl; Setup.Pos)
                {
                    ApplicationArea = All;
                    trigger OnValidate() begin end;
                }
                group(Grp)
                {
                    field(GrpCtl; Setup.Pos) { ApplicationArea = All; }
                }
            }
        }
    }
    var
        Setup: Record "RC Setup";
        TempSetup: Record "RC Setup" temporary;
        Arr: array[2] of Record "RC Setup";
}
report 69896 "RC Text Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RC Row") { } }
    requestpage
    {
        layout
        {
            area(Content)
            {
                field(NameCtl; Setup."Long Name") { ApplicationArea = All; }
            }
        }
    }
    var
        Setup: Record "RC Setup";
}
codeunit 69894 "RC Mode"
{
    SingleInstance = true;
    var
        M: Integer;
    procedure SetMode(N: Integer) begin M := N; end;
    procedure Mode(): Integer begin exit(M); end;
}
codeunit 69895 "RC Tests"
{
    Subtype = Test;

    local procedure RunWith(N: Integer)
    var
        M: Codeunit "RC Mode";
    begin
        M.SetMode(N);
        Report.Run(Report::"RC Report", true);
    end;

    local procedure SetPos(var RP: TestRequestPage "RC Report"; V: Variant)
    begin
        RP.PosCtl.SetValue(V);
    end;

    local procedure SetKind(var RP: TestRequestPage "RC Report"; V: Variant)
    begin
        RP.KindCtl.SetValue(V);
    end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure BooleanFalse() begin RunWith(1); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure DigitsOnEnum() begin RunWith(2); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure TextNotInVariant() begin RunWith(3); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure IntegerSelects() begin RunWith(4); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure DataItemField() begin RunWith(5); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure TemporaryRecordField() begin RunWith(6); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure ArrayElementEnumField() begin RunWith(7); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure NotEditableControl() begin RunWith(8); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure ControlWithCaption() begin RunWith(9); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure ControlWithOptionCaption() begin RunWith(10); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure ControlWithOnValidate() begin RunWith(11); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure ControlInGroup() begin RunWith(12); end;

    [Test]
    [HandlerFunctions('Handler')]
    procedure PunctuationText() begin RunWith(13); end;

    [Test]
    [HandlerFunctions('TextHandler')]
    procedure QuotedTextFieldParameters()
    begin
        Report.RunRequestPage(Report::"RC Text Report");
    end;

    [RequestPageHandler]
    procedure TextHandler(var RP: TestRequestPage "RC Text Report")
    begin
        RP.OK().Invoke();
    end;

    [RequestPageHandler]
    procedure Handler(var RP: TestRequestPage "RC Report")
    var
        M: Codeunit "RC Mode";
    begin
        case M.Mode() of
            1:
                SetPos(RP, false);
            2:
                SetKind(RP, '1');
            3:
                RP.PosCtl.SetValue('Left');
            4:
                begin
                    RP.PosCtl.SetValue(1);
                    if RP.PosCtl.Value <> 'Left' then
                        Error('shows %1', RP.PosCtl.Value);
                end;
            5:
                RP.ItemCtl.SetValue(1);
            6:
                RP.TempCtl.SetValue(1);
            7:
                RP.ArrKindCtl.SetValue(1);
            8:
                RP.NeCtl.SetValue(1);
            9:
                RP.CapCtl.SetValue(1);
            10:
                RP.OptCapCtl.SetValue(1);
            11:
                RP.ValCtl.SetValue(1);
            12:
                RP.GrpCtl.SetValue(1);
            13:
                SetPos(RP, '?-');
        end;
        RP.OK().Invoke();
    end;
}
"#;

fn outcome(name: &str) -> Outcome {
    al_testkit::outcomes_in(SRC, "RC Tests")
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, o)| o)
        .expect("the test ran")
}

fn refused(name: &str, needle: &str) {
    match outcome(name) {
        Outcome::NotRun(why) => assert!(why.contains(needle), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn an_integer_selects_the_option_value() {
    assert_eq!(outcome("IntegerSelects"), Outcome::Pass);
}

#[test]
fn false_on_an_option_field_of_a_global_record_is_refused() {
    refused("BooleanFalse", "SetValue of");
}

#[test]
fn digits_in_a_variant_on_an_enum_field_of_a_global_record_are_refused() {
    refused("DigitsOnEnum", "holding digits on a request page Enum control");
}

#[test]
fn a_text_not_in_a_variant_on_an_option_field_of_a_global_record_is_refused() {
    refused("TextNotInVariant", "SetValue of");
}

/// Only a control of the shape the corpus file pins takes the field's type: directly in the content area, with no property but
/// ApplicationArea and no trigger, bound to a field of a global record of the report that is not temporary, or to an Option field
/// of an element of a global record array (RequestPageOptionFieldOfGlobalRecord.al:93-101). Every other control keeps the base
/// refusal.
#[test]
fn only_the_pinned_control_shape_takes_the_field_type() {
    for name in [
        "DataItemField",
        "TemporaryRecordField",
        "ArrayElementEnumField",
        "NotEditableControl",
        "ControlWithCaption",
        "ControlWithOptionCaption",
        "ControlWithOnValidate",
        "ControlInGroup",
    ] {
        match outcome(name) {
            Outcome::NotRun(why) => assert!(
                why.contains("SetValue on a control whose type is not known"),
                "{name}: {why}"
            ),
            other => panic!("{name}: expected the base refusal, got {other:?}"),
        }
    }
}

/// A text in a Variant fails to evaluate only in the forms the corpus file pins (member names, captions, a prefix, the empty
/// text); punctuation is refused.
#[test]
fn a_punctuation_text_is_refused() {
    refused("PunctuationText", "neither digits, blank nor letters");
}

/// A quoted field name in a parameters text is pinned for an Option field only (RequestPageOptionFieldOfGlobalRecord.al:514).
#[test]
fn a_quoted_text_field_in_the_parameters_text_is_refused() {
    refused(
        "QuotedTextFieldParameters",
        "the parameters text for request page control NameCtl",
    );
}
