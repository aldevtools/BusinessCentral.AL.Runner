//! The edges of the measured blank-text shapes of `Evaluate` into Option fields and Enums with blank captions (al-runner issues
//! 8354, 8356): the error text of a rejected blank text, the format number, and the kinds of blank members measured together.

use al_testkit::text;

const SRC: &str = r#"
enum 69831 "NB G Last" { Extensible = false; value(0; Alpha) { Caption = 'Alpha cap'; } value(5; Beta) { Caption = 'Beta cap'; } value(10; Blank) { Caption = ''; } }
enum 69832 "NB G Mid" { value(0; Alpha) { Caption = 'Alpha cap'; } value(1; None) { Caption = ' '; } value(2; Beta) { Caption = 'Beta cap'; } }
enum 69833 "NB G Two" { value(0; Alpha) { Caption = 'Alpha cap'; } value(1; N1) { Caption = ''; } value(2; N2) { Caption = ' '; } value(3; Beta) { Caption = 'Beta cap'; } }
table 69830 "NB G Row"
{
    TableType = Temporary;
    fields
    {
        field(1; Id; Integer) { }
        field(2; NamedEmptyCap; Option) { OptionMembers = None,A,B; OptionCaption = ',A cap,B cap'; }
        field(3; FirstEmpty; Option) { OptionMembers = "",A,B; OptionCaption = ',A cap,B cap'; }
        field(4; MixNoneEmpty; Option) { OptionMembers = None,"",B; OptionCaption = ',Blank1,B cap'; }
        field(5; Mid; Enum "NB G Mid") { }
        field(6; Two; Enum "NB G Two") { }
        field(7; Short; Option) { OptionMembers = A,"",B; OptionCaption = 'A cap,'; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69834 C
{
    procedure NamedEmptyCapStatement(): Text
    var Row: Record "NB G Row";
    begin
        Evaluate(Row.NamedEmptyCap, '');
        exit('no error');
    end;

    procedure FirstEmptyFormat(N: Integer): Text
    var Row: Record "NB G Row"; R: Boolean;
    begin
        R := Evaluate(Row.FirstEmpty, '', N);
        exit(Format(R));
    end;

    procedure LastFormat(N: Integer): Text
    var E: Enum "NB G Last"; R: Boolean;
    begin
        R := Evaluate(E, '', N);
        exit(Format(R));
    end;

    procedure MixNoneEmpty(): Text
    var Row: Record "NB G Row"; R: Boolean;
    begin
        R := Evaluate(Row.MixNoneEmpty, '');
        exit(Format(R));
    end;

    procedure MidField(): Text
    var Row: Record "NB G Row"; R: Boolean;
    begin
        R := Evaluate(Row.Mid, '');
        exit(Format(R));
    end;

    procedure TwoField(): Text
    var Row: Record "NB G Row"; R: Boolean;
    begin
        R := Evaluate(Row.Two, '');
        exit(Format(R));
    end;

    procedure Short(): Text
    var Row: Record "NB G Row"; R: Boolean;
    begin
        R := Evaluate(Row.Short, '');
        exit(Format(R));
    end;
}
"#;

fn run(routine: &str, args: Vec<al_builtins::value::Value>) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_with(SRC, "C", routine, args).map(|(v, _)| v)
}

fn refused(routine: &str, args: Vec<al_builtins::value::Value>) -> String {
    match run(routine, args) {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.into_reason(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// A member with a name and the caption `''`: the statement error lists its name in place of the empty caption (corpus PR 1577,
// format-evaluate/EvalOptionBlankCaptions.al:344 OBC_Option_NamedEmptyCap), on the assumption `option-blank-text-blank-captions`.
#[test]
fn the_statement_error_lists_the_member_name_for_an_empty_caption() {
    match run("NamedEmptyCapStatement", vec![]) {
        Err(al_builtins::native::Signal::Error(e)) => assert_eq!(
            (e.message.as_str(), e.code.as_str()),
            ("'' is not an option. The existing options are: None,A cap,B cap", "InvalidOptionString")
        ),
        other => panic!("expected the error, got {other:?}"),
    }
}

// Corpus PR 1577 measured the blank texts without a format number only: a format number refuses, for an Option field and an
// Enum.
#[test]
fn a_blank_text_with_a_format_number_is_refused() {
    use al_builtins::value::Value;
    for n in [2, 9] {
        assert!(refused("FirstEmptyFormat", vec![Value::Integer(n)]).contains("not pinned"), "{n}");
        assert!(refused("LastFormat", vec![Value::Integer(n)]).contains("blank caption"), "{n}");
    }
    assert_eq!(text(run("FirstEmptyFormat", vec![Value::Integer(0)])), "No");
    assert_eq!(text(run("LastFormat", vec![Value::Integer(0)])), "Yes");
}

// Kinds of blank members the probe measured only one at a time: a field that mixes them, and Enum fields of enums with a middle
// Caption `' '` or two blank captions, refuse.
#[test]
fn unmeasured_mixes_of_blank_members_are_refused() {
    assert!(refused("MixNoneEmpty", vec![]).contains("mix of blank member names and captions"));
    assert!(refused("MidField", vec![]).contains("blank caption"));
    assert!(refused("TwoField", vec![]).contains("blank caption"));
}

// An OptionCaption with fewer entries than the field has members is not measured: refused.
#[test]
fn a_blank_text_into_a_field_with_fewer_captions_than_members_is_refused() {
    assert!(refused("Short", vec![]).contains("not one per member"));
}
