//! A subscriber of AutoFormatTranslate outside Microsoft's apps that writes its result, inside a test run (al-runner issue 8011,
//! review of PR 8015). What the test can see decides (`Unraised::seen_by`):
//! - a writer the test can see refuses, except the two cells events/platform-triggers/PtObserve.al:482-497 and :558-570 measure
//!   (a List opened with OpenView, control type 1, and a Card opened with OpenEdit, control type 2; blank expression, Value read);
//! - a writer the test cannot see that logs to no SingleInstance codeunit refuses (the reporter's shape);
//! - a writer the test cannot see that holds a SingleInstance codeunit runs as on main (the shape of the corpus fixture
//!   events/platform-triggers/PtFixtures.al:148-158; a known hole until the event is raised).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

/// `SUB` is the subscriber codeunit, `TESTS` the bodies of the test codeunit.
fn src(sub: &str, tests: &str) -> String {
    format!(
        r#"
table 69490 "AfT Row"
{{
    fields {{ field(1; Id; Integer) {{ }} field(2; Amount; Decimal) {{ }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
}}
page 69491 "AfT List"
{{
    PageType = List; SourceTable = "AfT Row"; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ repeater(Rows) {{ field(AmountCtl; Rec.Amount) {{ ApplicationArea = All; AutoFormatType = 1; AutoFormatExpression = ''; }} }} }} }}
}}
page 69492 "AfT Card"
{{
    PageType = Card; SourceTable = "AfT Row"; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ field(AmountCtl; Rec.Amount) {{ ApplicationArea = All; AutoFormatType = 2; AutoFormatExpression = ''; }} }} }}
}}
codeunit 69493 "AfT Log"
{{
    SingleInstance = true;
    var Mode: Integer;
    procedure SetMode(M: Integer) begin Mode := M; end;
    procedure GetMode(): Integer begin exit(Mode); end;
}}
{sub}
codeunit 69495 T
{{
    Subtype = Test;
    local procedure Seed()
    var R: Record "AfT Row";
    begin
        R.DeleteAll();
        R.Id := 1;
        R.Amount := 1234.5;
        R.Insert();
    end;
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
{tests}
}}
"#
    )
}

/// Writes Translation only while the log's mode is 2, as the corpus fixture does.
const LOGGING_WRITER: &str = r#"
codeunit 69494 "AfT Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", AutoFormatTranslate, '', false, false)]
    local procedure Write(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    var Log: Codeunit "AfT Log";
    begin
        if Log.GetMode() = 2 then
            Translation := '<Precision,1:1><Standard Format,0>';
    end;
}
"#;

/// The reporter's shape: no SingleInstance codeunit.
const PLAIN_WRITER: &str = r#"
codeunit 69494 "AfT Sub"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", AutoFormatTranslate, '', false, false)]
    local procedure Write(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    begin
        Translation := '<Precision,1:1><Standard Format,0>';
    end;
}
"#;

const UNSEEN_TESTS: &str = r#"
    [Test] procedure ListValue()
    var L: TestPage "AfT List";
    begin
        Seed();
        L.OpenView();
        Check('1,234.50', L.AmountCtl.Value());
    end;
"#;

/// The test holds the SingleInstance codeunit the writer holds: it can see the writer.
const SEEN_TESTS: &str = r#"
    [Test] procedure ListOpenViewValue()
    var Log: Codeunit "AfT Log"; L: TestPage "AfT List";
    begin
        Seed();
        Log.SetMode(2);
        L.OpenView();
        Check('1,234.50', L.AmountCtl.Value());
        Log.SetMode(0);
    end;
    [Test] procedure CardOpenEditValue()
    var Log: Codeunit "AfT Log"; C: TestPage "AfT Card";
    begin
        Seed();
        Log.SetMode(2);
        C.OpenEdit();
        Check('1,234.50', C.AmountCtl.Value());
        Log.SetMode(0);
    end;
    [Test] procedure CardOpenViewValue()
    var Log: Codeunit "AfT Log"; C: TestPage "AfT Card";
    begin
        Seed();
        C.OpenView();
        Check('1,234.50', C.AmountCtl.Value());
    end;
    [Test] procedure ListOpenEditValue()
    var Log: Codeunit "AfT Log"; L: TestPage "AfT List";
    begin
        Seed();
        L.OpenEdit();
        Check('1,234.50', L.AmountCtl.Value());
    end;
    [Test] procedure ListAssertEquals()
    var Log: Codeunit "AfT Log"; L: TestPage "AfT List";
    begin
        Seed();
        L.OpenView();
        L.AmountCtl.AssertEquals(1234.5);
    end;
"#;

fn get(out: &[(String, Outcome)], name: &str) -> Outcome {
    out.iter().find(|(t, _)| t == name).map(|(_, o)| o.clone()).unwrap()
}

fn refused(o: Outcome) {
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("\"AfT Sub\"") && why.contains("AutoFormatTranslate")),
        "{o:?}"
    );
}

#[test]
fn an_unseen_writer_with_a_single_instance_witness_runs_as_main() {
    let out = outcomes_in(&src(LOGGING_WRITER, UNSEEN_TESTS), "T");
    assert_eq!(get(&out, "ListValue"), Outcome::Pass);
}

#[test]
fn an_unseen_writer_without_a_single_instance_witness_refuses() {
    let out = outcomes_in(&src(PLAIN_WRITER, UNSEEN_TESTS), "T");
    refused(get(&out, "ListValue"));
}

#[test]
fn a_seen_writer_refuses_except_the_two_measured_cells() {
    let out = outcomes_in(&src(LOGGING_WRITER, SEEN_TESTS), "T");
    assert_eq!(get(&out, "ListOpenViewValue"), Outcome::Pass);
    assert_eq!(get(&out, "CardOpenEditValue"), Outcome::Pass);
    for neighbor in ["CardOpenViewValue", "ListOpenEditValue", "ListAssertEquals"] {
        refused(get(&out, neighbor));
    }
}
