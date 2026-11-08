//! `Evaluate` into an Option field that declares no OptionCaption: record/filters/P68I_TransferEnumOption.al:303-332,
//! errors/P137OptionEvaluate.al:155-231 (field Three) and format-evaluate/AxFormatOption.al:187-212 (fields OptSpace, OptEmpty).

fn run(src: &str) -> String {
    al_testkit::text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v))
}

const TABLE: &str = r#"
table 50103 R
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Three; Option) { OptionMembers = A,B,C; }
        field(3; OptSpace; Option) { OptionMembers = " ",A; }
        field(4; OptEmpty; Option) { OptionMembers = ,A; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
"#;

fn eval_three(texts: &[&str]) -> String {
    let calls: String = texts
        .iter()
        .map(|t| {
            format!("Row.Three := Row.Three::B; Ok := Evaluate(Row.Three, '{t}'); M += Format(Ok) + '|' + Format(Row.Three) + '|' + Format(Row.Three, 0, 2) + ';';\n")
        })
        .collect();
    run(&format!(
        r#"{TABLE}
codeunit 50103 C
{{
    procedure F(): Text
    var Row: Record R; Ok: Boolean; M: Text;
    begin
        {calls}
        exit(M);
    end;
}}
"#
    ))
}

// P137OptionEvaluate.al:171-179: a member name in any case, an ordinal of any size from 0 up; a negative number, an unknown name
// and the empty text are rejected and keep the value. P68I_TransferEnumOption.al:326-329: `16` and `6` of six members.
#[test]
fn a_field_without_captions_takes_names_and_ordinals_of_any_size() {
    assert_eq!(
        eval_three(&["A", "c", "0", "2", "3", "4", "16", "-1", "D", ""]),
        "Yes|A|0;Yes|C|2;Yes|A|0;Yes|C|2;Yes|3|3;Yes|4|4;Yes|16|16;No|B|1;No|B|1;No|B|1;"
    );
}

// P137OptionEvaluate.al:228 and P68I_TransferEnumOption.al:330: with the result unused the error names the members.
#[test]
fn a_rejected_text_with_the_result_unused_names_the_members() {
    let r = al_testkit::run_with(
        &format!(
            r#"{TABLE}
codeunit 50103 C
{{
    procedure F(): Text
    var Row: Record R;
    begin
        Evaluate(Row.Three, '9x');
    end;
}}
"#
        ),
        "C",
        "F",
        vec![],
    )
    .map(|(v, _)| v);
    assert_eq!(
        al_testkit::error_message(r),
        "'9x' is not an option. The existing options are: A,B,C"
    );
}

// AxFormatOption.al:208-211: the empty text and a blank name a member declared as " ", not one declared as "".
#[test]
fn the_empty_text_names_a_blank_member_but_not_an_empty_one() {
    let got = run(&format!(
        r#"{TABLE}
codeunit 50103 C
{{
    procedure F(): Text
    var R: Record R; Ok: Boolean; M: Text;
    begin
        R.OptEmpty := 1; Ok := Evaluate(R.OptEmpty, ''); M += Format(Ok) + '|' + Format(R.OptEmpty, 0, 9) + ';';
        R.OptEmpty := 1; Ok := Evaluate(R.OptEmpty, ' '); M += Format(Ok) + '|' + Format(R.OptEmpty, 0, 9) + ';';
        R.OptSpace := 1; Ok := Evaluate(R.OptSpace, ''); M += Format(Ok) + '|' + Format(R.OptSpace, 0, 9) + ';';
        R.OptSpace := 1; Ok := Evaluate(R.OptSpace, ' '); M += Format(Ok) + '|' + Format(R.OptSpace, 0, 9) + ';';
        exit(M);
    end;
}}
"#
    ));
    assert_eq!(got, "No|1;No|1;Yes|0;Yes|0;");
}
