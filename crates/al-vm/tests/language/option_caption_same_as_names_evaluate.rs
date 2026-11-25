//! Evaluate into an Option field whose OptionCaption repeats its member names (`Alpha,Beta` for `Alpha,Beta`), the shape of
//! al-runner issue 8355: a number past the members, for two members named by letters only.

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
        field(4; Hue; Option) { OptionMembers = Red,Green; OptionCaption = 'Red,Green'; }
        field(5; Coded; Option) { OptionMembers = A1,B2; OptionCaption = 'A1,B2'; }
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

    procedure Hue(T: Text): Text
    var Row: Record "NB Pair Row"; R: Boolean; Ord: Integer;
    begin
        R := Evaluate(Row.Hue, T);
        Ord := Row.Hue;
        exit(Format(R) + '|' + Format(Ord));
    end;

    procedure Coded(T: Text): Text
    var Row: Record "NB Pair Row"; R: Boolean; Ord: Integer;
    begin
        R := Evaluate(Row.Coded, T);
        Ord := Row.Coded;
        exit(Format(R) + '|' + Format(Ord));
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

fn run(
    routine: &str,
    args: Vec<al_builtins::value::Value>,
) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
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

// Only the members the evidence declares (`Alpha,Beta`) are measured: other members whose captions are their names read as a
// field without captions, as on main, and a number past them refuses.
#[test]
fn other_letter_names_whose_captions_are_the_names_read_as_a_field_without_captions() {
    use al_builtins::value::Value;
    for t in ["2", "99", "-1"] {
        let why = refusal(run("Hue", vec![Value::Text(t.into())]));
        assert!(why.contains("neither an ordinal nor a member name"), "[{t}] {why}");
    }
}

// The reporter's repro of al-runner issue 8355, unchanged: each test ends in `Error` with the result (BC 28.4 and 29.0:
// `Yes 2|2`, `Yes 99|99`, `No Alpha|0`).
const REPORTER: &str = r#"
table 71810 "Eval Opt Two"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Pair; Option)
        {
            OptionMembers = Alpha,Beta;
            OptionCaption = 'Alpha,Beta';
        }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 71861 "Eval Opt Two Test"
{
    Subtype = Test;

    local procedure Try(T: Text)
    var
        R: Record "Eval Opt Two";
        Ok: Boolean;
    begin
        Ok := Evaluate(R.Pair, T);
        Error('%1 %2|%3', Ok, R.Pair, Format(R.Pair, 0, 2));
    end;

    [Test]
    procedure Two()
    begin
        Try('2');
    end;

    [Test]
    procedure NinetyNine()
    begin
        Try('99');
    end;

    [Test]
    procedure MinusOne()
    begin
        Try('-1');
    end;
}
"#;

#[test]
fn the_reporters_repro_of_issue_8355_gives_bcs_answers() {
    for (routine, want) in [
        ("Two", "Yes 2|2"),
        ("NinetyNine", "Yes 99|99"),
        ("MinusOne", "No Alpha|0"),
    ] {
        match al_testkit::run_with(REPORTER, "Eval Opt Two Test", routine, vec![]) {
            Err(al_builtins::native::Signal::Error(e)) => assert_eq!(e.message, want, "{routine}"),
            other => panic!("{routine}: expected the test's Error, got {other:?}"),
        }
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
    use al_builtins::value::Value;
    for r in [
        pair(0, "3"),
        pair(0, " 2"),
        statement(0, "2"),
        run("Trio", vec![Value::Text("3".into())]),
        // A member name with a character other than a letter: no evidence measured one.
        run("Coded", vec![Value::Text("2".into())]),
    ] {
        assert!(refusal(r).contains("neither an ordinal nor a member name"));
    }
}
