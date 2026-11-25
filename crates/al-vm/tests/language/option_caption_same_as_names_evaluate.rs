//! Evaluate into an Option field whose OptionCaption repeats its member names (`Alpha,Beta` for `Alpha,Beta`), the shape of
//! al-runner issue 8355: a number past the members.

use al_testkit::text;

const SRC: &str = r#"
table 69820 "NB Pair Row"
{
    TableType = Temporary;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Pair; Option) { OptionMembers = Alpha,Beta; OptionCaption = 'Alpha,Beta'; }
        field(3; Trio; Option) { OptionMembers = Alpha,Beta,Gamma; OptionCaption = 'Alpha,Beta,Gamma'; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69821 C
{
    procedure Pair(Start: Integer; T: Text): Text
    var Row: Record "NB Pair Row"; R: Boolean; Ord: Integer;
    begin
        Row.Pair := Start;
        R := Evaluate(Row.Pair, T);
        Ord := Row.Pair;
        exit(Format(R) + '|' + Format(Ord));
    end;

    procedure PairStatement(Start: Integer; T: Text): Text
    var Row: Record "NB Pair Row"; Ord: Integer;
    begin
        Row.Pair := Start;
        Evaluate(Row.Pair, T);
        Ord := Row.Pair;
        exit(Format(Ord));
    end;

    procedure Trio(T: Text): Text
    var Row: Record "NB Pair Row"; R: Boolean; Ord: Integer;
    begin
        R := Evaluate(Row.Trio, T);
        Ord := Row.Trio;
        exit(Format(R) + '|' + Format(Ord));
    end;
}
"#;

fn run(routine: &str, args: Vec<al_builtins::value::Value>) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    al_testkit::run_with(SRC, "C", routine, args).map(|(v, _)| v)
}

fn pair(start: i32, t: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    use al_builtins::value::Value;
    run("Pair", vec![Value::Integer(start), Value::Text(t.into())])
}

fn statement(start: i32, t: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    use al_builtins::value::Value;
    run("PairStatement", vec![Value::Integer(start), Value::Text(t.into())])
}

fn refusal(r: Result<al_builtins::value::Value, al_builtins::native::Signal>) -> String {
    match r {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.into_reason(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// The reporter's repro of al-runner issue 8355 (BC 28.4 and 29.0: `2` and `99` true and stored, `-1` false and the value
// kept) and the other texts of corpus PR 1577 OBC_Option_PairOrdinals, on the assumption `option-caption-same-as-names-ordinal`.
#[test]
fn a_number_past_the_members_of_a_field_whose_captions_are_its_names_is_stored() {
    for (start, t, want) in [
        (0, "2", "Yes|2"),
        (0, "99", "Yes|99"),
        (0, "-1", "No|0"),
        (1, "-1", "No|1"),
        (0, "+2", "Yes|2"),
        (0, "2147483647", "Yes|2147483647"),
        (0, "2147483648", "No|0"),
    ] {
        assert_eq!(text(pair(start, t)), want, "{start} [{t}]");
    }
    assert_eq!(text(statement(0, "99")), "99");
    match statement(1, "-1") {
        Err(al_builtins::native::Signal::Error(e)) => {
            assert_eq!(e.message, "'-1' is not an option. The existing options are: Alpha,Beta");
        }
        other => panic!("expected the Option error, got {other:?}"),
    }
}

// The texts main answered keep main's answer: an ordinal of a member, a member name in any case, the empty text.
#[test]
fn a_member_of_a_field_whose_captions_are_its_names_reads_as_before() {
    assert_eq!(text(pair(0, "1")), "Yes|1");
    assert_eq!(text(pair(0, "beta")), "Yes|1");
    assert_eq!(text(pair(1, "")), "No|1");
}

// Texts, forms and fields corpus PR 1577 did not measure keep main's refusal.
#[test]
fn an_unmeasured_number_past_the_members_is_refused() {
    for r in [pair(0, "3"), pair(0, " 2"), statement(0, "2"), run("Trio", vec![al_builtins::value::Value::Text("3".into())])] {
        assert!(refusal(r).contains("neither an ordinal nor a member name"));
    }
}
