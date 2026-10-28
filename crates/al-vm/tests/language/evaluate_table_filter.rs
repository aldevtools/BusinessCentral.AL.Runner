//! `Evaluate` into a TableFilter field (al-runner issue 5998). The form `<table name>: <field>=<value>` with a Code field is
//! pinned (permissions/lt-av-permission-filter/AvSetPermissionFilter.al:125,131,157-158: Evaluate answers true and Format gives
//! the text back); an Option field with a member name is the registered assumption `table-filter-evaluate-option-member`.

use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
use al_builtins::native::Signal;

const TABLES: &str = "table 50100 \"Repro Planning Unit\" { fields { field(1; Code; Code[20]) { } field(2; \"Certificate Filter\"; TableFilter) { } }
        keys { key(PK; Code) { Clustered = true; } } }
    table 50102 \"Repro Certificate\" { fields { field(1; Code; Code[20]) { } field(2; \"Report Type\"; Option) { OptionMembers = DIAM,LGDIAM; } }
        keys { key(PK; Code) { Clustered = true; } } }";

fn src(body: &str) -> String {
    format!("{TABLES} codeunit 50101 C {{ {body} }}")
}

#[test]
fn a_code_field_condition_is_stored_as_written() {
    let s = src(
        "procedure F(): Text var P: Record \"Repro Planning Unit\"; Ok: Boolean; begin
        Ok := Evaluate(P.\"Certificate Filter\", 'Repro Certificate: Code=A');
        exit(Format(Ok) + '|' + Format(P.\"Certificate Filter\")); end;",
    );
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&s, "C", "F")),
        "Yes|Repro Certificate: Code=A"
    );
    assert!(!take_hits().contains(&Assumption::TableFilterEvaluateOptionMember));
}

#[test]
fn the_reporters_option_member_condition_is_a_flagged_pass() {
    let s = src(
        "procedure F(): Text var P: Record \"Repro Planning Unit\"; Got: Record \"Repro Planning Unit\"; begin
        Evaluate(P.\"Certificate Filter\", 'Repro Certificate: Report Type=LGDIAM');
        P.Code := 'U1'; P.Insert(); Got.Get('U1');
        exit(Format(Got.\"Certificate Filter\")); end;",
    );
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&s, "C", "F")),
        "Repro Certificate: Report Type=LGDIAM"
    );
    assert!(take_hits().contains(&Assumption::TableFilterEvaluateOptionMember));
}

#[test]
fn texts_no_test_pins_are_refused() {
    for text in [
        "Repro Certificate: Code=A|B",
        "Repro Certificate: Code=A,Report Type=DIAM",
        "Repro Certificate: Report Type=OTHER",
        "No Such Table: Code=A",
        "Repro Certificate: No Field=A",
        "Code=A",
        "",
    ] {
        let s = src(&format!(
            "procedure F(): Boolean var P: Record \"Repro Planning Unit\"; begin exit(Evaluate(P.\"Certificate Filter\", '{text}')); end;"
        ));
        match al_testkit::run_in(&s, "C", "F") {
            Err(Signal::Unsupported(why)) => assert!(why.contains("TableFilter"), "{text}: {why}"),
            other => panic!("{text}: expected a refusal, got {other:?}"),
        }
    }
}

// al-runner issue 6006: several `Field=Value` conditions joined by commas, and Code values with spaces, in the forms the reporter
// measured on real BC (Bin: `Location Code=MAIN,Code=BIN01`, `Code=BIN WITH SPACE`, both together): Evaluate answers true,
// Format and Format(..., 0, 9) give the text back, no error. The registered assumption `table-filter-evaluate-several-code-fields`.

const BINS: &str =
    "table 50110 \"Repro Bin\" { fields { field(1; \"Location Code\"; Code[10]) { } field(2; Code; Code[20]) { }
        field(3; Description; Text[50]) { } field(4; \"Bin Type\"; Option) { OptionMembers = PICK,PUT; }
        field(5; \"Zone Code\"; Code[10]) { Caption = 'Zone'; } }
        keys { key(PK; \"Location Code\", Code) { Clustered = true; } } }
    table 50111 \"Repro Holder\" { fields { field(1; Id; Integer) { } field(2; \"Filter Value\"; TableFilter) { } }
        keys { key(PK; Id) { Clustered = true; } } }";

fn bin_src(body: &str) -> String {
    format!("{BINS} codeunit 50112 C {{ {body} }}")
}

#[test]
fn the_reporters_measured_forms_give_the_text_back_and_are_flagged() {
    for text in [
        "Repro Bin: Location Code=MAIN,Code=BIN01",
        "Repro Bin: Code=BIN WITH SPACE",
        "Repro Bin: Location Code=MAIN,Code=BIN WITH SPACE",
    ] {
        let s = bin_src(&format!(
            "procedure F(): Text var H: Record \"Repro Holder\"; Got: Record \"Repro Holder\"; Ok: Boolean; begin
            ClearLastError();
            Ok := Evaluate(H.\"Filter Value\", '{text}');
            H.Id := 1; H.Insert(); Got.Get(1);
            exit(Format(Ok) + '|' + Format(Got.\"Filter Value\") + '|' + Format(Got.\"Filter Value\", 0, 9) + '|' + GetLastErrorText() + '|' + GetLastErrorCode()); end;"
        ));
        clear_hits();
        assert_eq!(
            al_testkit::text(al_testkit::run_in(&s, "C", "F")),
            format!("Yes|{text}|{text}||"),
            "{text}"
        );
        assert!(
            take_hits().contains(&Assumption::TableFilterEvaluateSeveralCodeFields),
            "{text}"
        );
    }
}

#[test]
fn the_single_plain_code_condition_stays_a_clean_pass() {
    let s = bin_src(
        "procedure F(): Text var H: Record \"Repro Holder\"; begin
        Evaluate(H.\"Filter Value\", 'Repro Bin: Code=BIN01');
        exit(Format(H.\"Filter Value\")); end;",
    );
    clear_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(&s, "C", "F")),
        "Repro Bin: Code=BIN01"
    );
    assert!(!take_hits().contains(&Assumption::TableFilterEvaluateSeveralCodeFields));
}

#[test]
fn unmeasured_several_field_and_space_forms_are_refused() {
    for text in [
        // a filter expression, a wildcard or a quoted value
        "Repro Bin: Location Code=MAIN,Code=A..B",
        "Repro Bin: Location Code=MAIN,Code=A|B",
        "Repro Bin: Location Code=MAIN,Code='BIN01'",
        "Repro Bin: Code=BIN*",
        // an unknown field, a field given twice, a field in another case
        "Repro Bin: Location Code=MAIN,Nope=BIN01",
        "Repro Bin: Code=A,Code=B",
        "Repro Bin: location code=MAIN,Code=BIN01",
        // a field that is no Code field, or whose caption is not its name
        "Repro Bin: Location Code=MAIN,Description=X",
        "Repro Bin: Location Code=MAIN,Bin Type=PICK",
        "Repro Bin: Location Code=MAIN,Zone Code=Z1",
        "Repro Bin: Zone Code=Z 1",
        // fields not in the order of their numbers, more than two, a space after the comma, an empty condition
        "Repro Bin: Code=BIN01,Location Code=MAIN",
        "Repro Bin: Location Code=MAIN,Code=BIN01,Zone Code=Z1",
        "Repro Bin: Location Code=MAIN, Code=BIN01",
        "Repro Bin: Location Code=MAIN,",
        // spaces at the ends, two in a row, lower case, too long
        "Repro Bin: Code= BIN",
        "Repro Bin: Code=BIN ",
        "Repro Bin: Code=BIN  TWO",
        "Repro Bin: Code=bin with space",
        "Repro Bin: Location Code=main,Code=BIN01",
        "Repro Bin: Location Code=MAINLOCATION1,Code=BIN01",
        "Repro Bin: Code=A VERY LONG BIN CODE OF MORE",
    ] {
        let s = bin_src(&format!(
            "procedure F(): Boolean var H: Record \"Repro Holder\"; begin exit(Evaluate(H.\"Filter Value\", '{}')); end;",
            text.replace('\'', "''")
        ));
        match al_testkit::run_in(&s, "C", "F") {
            Err(Signal::Unsupported(why)) => assert!(why.contains("TableFilter"), "{text}: {why}"),
            other => panic!("{text}: expected a refusal, got {other:?}"),
        }
    }
}
