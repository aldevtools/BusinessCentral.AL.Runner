//! TestPage SetValue into a page variable with a TableRelation (al-runner issue 9336, corpus PR 1646): the served
//! completion and the refusals of the shapes the probe does not measure (`pages/relation_text_entry.rs`).

fn src(control: &str, var: &str, typed: &str) -> String {
    format!(
        r#"
table 69960 "RT Book"
{{
    fields {{ field(1; "Code"; Code[10]) {{ }} field(2; Name; Text[30]) {{ }} }}
    keys {{ key(PK; "Code") {{ Clustered = true; }} }}
}}
table 69961 "RT Pair"
{{
    fields {{ field(1; "Group"; Code[10]) {{ }} field(2; "Code"; Code[10]) {{ }} }}
    keys {{ key(PK; "Group", "Code") {{ Clustered = true; }} }}
}}
page 69962 "RT Page"
{{
    PageType = Card;
    layout {{ area(Content) {{ field(Ctl; V) {{ {control} }} }} }}
    var V: {var};
}}
codeunit 69963 C
{{
    procedure Run(): Text
    var
        Book: Record "RT Book";
        Pair: Record "RT Pair";
        P: TestPage "RT Page";
    begin
        Book.DeleteAll();
        Book."Code" := 'MNO';
        Book.Insert();
        Pair."Group" := 'G';
        Pair."Code" := 'MNO';
        Pair.Insert();
        P.OpenEdit();
        P.Ctl.SetValue('{typed}');
        exit(P.Ctl.Value());
    end;
}}
"#
    )
}

fn refusal(src: &str) -> String {
    match al_testkit::run_in(src, "C", "Run") {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

const UNMEASURED: &str = "a shape corpus PR 1646 does not measure";

#[test]
fn a_code_variable_completes_a_lower_case_prefix_of_the_key() {
    let r = al_testkit::run_in(&src(r#"TableRelation = "RT Book";"#, "Code[10]", "mn"), "C", "Run");
    assert_eq!(al_testkit::text(r), "MNO");
}

#[test]
fn a_code_variable_without_a_row_raises_the_does_not_exist_error() {
    let r = al_testkit::run_in(&src(r#"TableRelation = "RT Book";"#, "Code[10]", "zzz"), "C", "Run");
    assert_eq!(
        al_testkit::error_message(r),
        "Validation error for Field: Ctl,  Message = 'The RT Book does not exist. Identification fields and values: Code='ZZZ''"
    );
}

#[test]
fn a_compound_key_is_refused() {
    assert!(refusal(&src(r#"TableRelation = "RT Pair";"#, "Code[10]", "MNO")).contains(UNMEASURED));
}

#[test]
fn a_value_longer_than_the_variable_is_refused() {
    assert!(refusal(&src(r#"TableRelation = "RT Book";"#, "Code[2]", "MNO")).contains(UNMEASURED));
}

#[test]
fn a_where_on_a_code_variable_is_refused() {
    let control = r#"TableRelation = "RT Book" where(Name = const(A));"#;
    assert!(refusal(&src(control, "Code[10]", "MNO")).contains(UNMEASURED));
}

#[test]
fn a_field_in_no_key_on_a_code_variable_is_refused() {
    assert!(refusal(&src(r#"TableRelation = "RT Book".Name;"#, "Code[10]", "MNO")).contains(UNMEASURED));
}
