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
    }
    keys { key(PK; "Code") { } }
}
table 69892 "RC Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
report 69893 "RC Report"
{
    ProcessingOnly = true;
    dataset { dataitem(Row; "RC Row") { } }
    requestpage
    {
        layout
        {
            area(Content)
            {
                field(PosCtl; Setup.Pos) { ApplicationArea = All; }
                field(KindCtl; Setup.Kind) { ApplicationArea = All; }
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
