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
