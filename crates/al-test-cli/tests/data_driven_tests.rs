//! Data-driven tests (`[TestDataSource(Codeunit, DataSet)]`, BC 29, ADR-059 decision 2): one result per test case that the
//! data source's `ListTestCases` returns, named `Procedure[identifier]`.
//!
//! Pins: corpus al-language-testlibraries/lt-bi-test-framework/BiDataDriven.al (corpus PR 861), the asserting tests
//! (passed on the corpus CI of BC 29 with these result names) and the reference results in the comments of its red part.

use al_test_cli::report::Outcome;
use al_testkit::outcomes_in;

/// The interfaces System.app declares on BC 29 (the inline app has no System.app).
const SYSTEM: &str = r#"
interface ITestContext
{
}

interface ITestDataSource
{
    procedure ListTestCases(DataSetIdentifier: Text; context: DataSourceContext): List of [Text];
    procedure GetTestCase(DataSetIdentifier: Text; TestCaseIndex: Integer; TestCaseIdentifier: Text; context: DataSourceContext): Interface ITestContext;
}

interface "DD Case Data"
{
    procedure CaseText(): Text;
}

codeunit 50100 "DD Log"
{
    SingleInstance = true;
    var
        Entries: Text;
    procedure Reset() begin Entries := ''; end;
    procedure Add(Entry: Text) begin Entries += Entry + ';'; end;
    procedure Peek(): Text begin exit(Entries); end;
}

codeunit 50101 "DD Case" implements ITestContext, "DD Case Data"
{
    var
        Value: Text;
    procedure SetText(NewValue: Text) begin Value := NewValue; end;
    procedure CaseText(): Text begin exit(Value); end;
}

// BiDataDriven.al:89-121 (Set1, Empty) and :545-581 (the red source: dup, blank, bracket, ListError, GetError).
codeunit 50102 "DD Source" implements ITestDataSource
{
    procedure ListTestCases(DataSetIdentifier: Text; context: DataSourceContext): List of [Text]
    var
        Log: Codeunit "DD Log";
        Cases: List of [Text];
    begin
        Log.Reset();
        Log.Add('list(' + DataSetIdentifier + ')');
        case DataSetIdentifier of
            'Empty':
                ;
            'dup':
                begin
                    Cases.Add('same');
                    Cases.Add('same');
                end;
            'blank':
                Cases.Add('   ');
            'bracket':
                Cases.Add('a[b');
            'ListError':
                Error('list failed');
            'Ids':
                begin
                    Cases.Add('same');
                    Cases.Add('Same');
                    Cases.Add(' padded ');
                end;
            else begin
                Cases.Add('alpha');
                Cases.Add('beta');
                Cases.Add('gamma');
            end;
        end;
        exit(Cases);
    end;

    procedure GetTestCase(DataSetIdentifier: Text; TestCaseIndex: Integer; TestCaseIdentifier: Text; context: DataSourceContext): Interface ITestContext
    var
        Log: Codeunit "DD Log";
        TestCase: Codeunit "DD Case";
    begin
        Log.Add('get(' + DataSetIdentifier + ',' + Format(TestCaseIndex) + ',' + TestCaseIdentifier + ')');
        if (DataSetIdentifier = 'GetError') and (TestCaseIndex = 2) then
            Error('get failed');
        TestCase.SetText(TestCaseIdentifier);
        exit(TestCase);
    end;
}

// BiDataDriven.al:583-605: a source that calls a DataSourceContext member.
codeunit 50103 "DD Context Source" implements ITestDataSource
{
    procedure ListTestCases(DataSetIdentifier: Text; context: DataSourceContext): List of [Text]
    var
        Cases: List of [Text];
    begin
        if DataSetIdentifier = 'CodeunitId' then
            Cases.Add(Format(context.CodeunitId()));
        Cases.Add('one');
        exit(Cases);
    end;

    procedure GetTestCase(DataSetIdentifier: Text; TestCaseIndex: Integer; TestCaseIdentifier: Text; context: DataSourceContext): Interface ITestContext
    var
        TestCase: Codeunit "DD Case";
    begin
        TestCase.SetText(TestCaseIdentifier);
        exit(TestCase);
    end;
}

table 50110 "DD Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}
"#;

fn outcomes(tests: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    outcomes_in(&format!("{SYSTEM}\n{tests}"), codeunit)
}

fn names(got: &[(String, Outcome)]) -> Vec<&str> {
    got.iter().map(|(n, _)| n.as_str()).collect()
}

fn all_pass(got: &[(String, Outcome)]) {
    for (n, o) in got {
        assert_eq!(*o, Outcome::Pass, "{n}: {got:?}");
    }
}

// BiDataDriven.al:140-177 (codeunit 81603), header items 2 and 3 (:13-20): ListTestCases once, then GetTestCase and the run
// per case, the index from 1; one result per case named `Rows[alpha]`; the plain test first.
#[test]
fn each_case_is_a_result_named_after_procedure_and_identifier() {
    let got = outcomes(
        r#"
codeunit 50120 "DD Calls"
{
    Subtype = Test;

    [TestDataSource(50102, 'Set1')]
    procedure Rows(Context: Interface ITestContext)
    var
        Log: Codeunit "DD Log";
        CaseData: Interface "DD Case Data";
        Id: Text;
    begin
        CaseData := Context as "DD Case Data";
        Id := CaseData.CaseText();
        case Id of
            'alpha':
                if Log.Peek() <> 'list(Set1);get(Set1,1,alpha);' then Error('alpha saw %1', Log.Peek());
            'beta':
                if Log.Peek() <> 'list(Set1);get(Set1,1,alpha);run(alpha);get(Set1,2,beta);' then Error('beta saw %1', Log.Peek());
            'gamma':
                if Log.Peek() <> 'list(Set1);get(Set1,1,alpha);run(alpha);get(Set1,2,beta);run(beta);get(Set1,3,gamma);' then Error('gamma saw %1', Log.Peek());
            else
                Error('unexpected case %1', Id);
        end;
        Log.Add('run(' + Id + ')');
    end;

    [Test]
    procedure Plain()
    begin
    end;
}
"#,
        "DD Calls",
    );
    assert_eq!(
        names(&got),
        ["Plain", "Rows[alpha]", "Rows[beta]", "Rows[gamma]"],
        "{got:?}"
    );
    all_pass(&got);
}

// BiDataDriven.al:179-195 (codeunit 81604), header item 3: an empty data set gives no result and the body does not run.
#[test]
fn an_empty_data_set_gives_no_result() {
    let got = outcomes(
        r#"
codeunit 50121 "DD Empty"
{
    Subtype = Test;

    [TestDataSource(50102, 'Empty')]
    procedure NoCases(Context: Interface ITestContext)
    begin
        Error('a data set without cases must not run the test procedure');
    end;

    [Test]
    procedure PlainRuns()
    begin
    end;
}
"#,
        "DD Empty",
    );
    assert_eq!(got, [("PlainRuns".to_string(), Outcome::Pass)]);
}

// BiDataDriven.al:197-219 (codeunit 81605): identifiers pass through unchanged; 'same' and 'Same' are distinct (header item 5).
#[test]
fn identifiers_pass_through_unchanged() {
    let got = outcomes(
        r#"
codeunit 50122 "DD Ids"
{
    Subtype = Test;
    var
        Seen: Text;

    [TestDataSource(50102, 'Ids')]
    procedure Id(Context: Interface ITestContext)
    var
        CaseData: Interface "DD Case Data";
    begin
        CaseData := Context as "DD Case Data";
        Seen += '<' + CaseData.CaseText() + '>';
        if CaseData.CaseText() = ' padded ' then
            if Seen <> '<same><Same>< padded >' then Error('seen %1', Seen);
    end;
}
"#,
        "DD Ids",
    );
    assert_eq!(names(&got), ["Id[same]", "Id[Same]", "Id[ padded ]"], "{got:?}");
    all_pass(&got);
}

// BiDataDriven.al:244-306 (codeunit 81607), header item 1: plain tests in declaration order, then the data-driven procedures
// in alphabetical order of their names.
#[test]
fn plain_tests_first_then_data_driven_procedures_alphabetically() {
    let got = outcomes(
        r#"
codeunit 50123 "DD Order"
{
    Subtype = Test;

    [TestDataSource(50102, 'Set1')]
    procedure Zeta(Context: Interface ITestContext)
    begin
    end;

    [Test]
    procedure PlainZ()
    begin
    end;

    [TestDataSource(50102, 'Set1')]
    procedure Alpha(Context: Interface ITestContext)
    begin
    end;

    [Test]
    procedure PlainA()
    begin
    end;

    [TestDataSource(50102, 'Set1')]
    procedure Mid(Context: Interface ITestContext)
    begin
    end;
}
"#,
        "DD Order",
    );
    assert_eq!(
        names(&got),
        [
            "PlainZ",
            "PlainA",
            "Alpha[alpha]",
            "Alpha[beta]",
            "Alpha[gamma]",
            "Mid[alpha]",
            "Mid[beta]",
            "Mid[gamma]",
            "Zeta[alpha]",
            "Zeta[beta]",
            "Zeta[gamma]"
        ],
        "{got:?}"
    );
    all_pass(&got);
}

// BiDataDriven.al:316-376 (codeunit 81608), header item 4: globals and database rows persist from case to case; handlers
// work per case.
#[test]
fn cases_share_globals_rows_and_handlers() {
    let got = outcomes(
        r#"
codeunit 50124 "DD State"
{
    Subtype = Test;
    var
        HandlerHits: Integer;

    [TestDataSource(50102, 'Set1')]
    [HandlerFunctions('MsgHandler')]
    procedure Rows(Context: Interface ITestContext)
    var
        CaseData: Interface "DD Case Data";
        Row: Record "DD Row";
        Position: Integer;
    begin
        CaseData := Context as "DD Case Data";
        case CaseData.CaseText() of
            'alpha': Position := 1;
            'beta': Position := 2;
            'gamma': Position := 3;
        end;
        if HandlerHits <> Position - 1 then Error('hits %1 before case %2', HandlerHits, Position);
        if Row.Count() <> Position - 1 then Error('rows %1 before case %2', Row.Count(), Position);
        Row.Id := Position;
        Row.Insert();
        Message('hello');
        if HandlerHits <> Position then Error('hits after');
    end;

    [MessageHandler]
    procedure MsgHandler(Msg: Text[1024])
    begin
        HandlerHits += 1;
    end;
}
"#,
        "DD State",
    );
    assert_eq!(names(&got), ["Rows[alpha]", "Rows[beta]", "Rows[gamma]"], "{got:?}");
    all_pass(&got);
}

// BiDataDriven.al:612-629 (reference results of codeunit 81631), header item 6: a failing case fails only itself; an error
// in GetTestCase fails that case and the body does not run for it.
#[test]
fn a_failing_case_or_get_test_case_fails_only_that_case() {
    let got = outcomes(
        r#"
codeunit 50125 "DD Red"
{
    Subtype = Test;

    [TestDataSource(50102, 'Set')]
    procedure Failing(Context: Interface ITestContext)
    var
        CaseData: Interface "DD Case Data";
    begin
        CaseData := Context as "DD Case Data";
        if CaseData.CaseText() = 'beta' then
            Error('case failed: %1', CaseData.CaseText());
    end;

    [TestDataSource(50102, 'GetError')]
    procedure GetError(Context: Interface ITestContext)
    begin
        Error('the body must not run for beta');
    end;
}
"#,
        "DD Red",
    );
    assert_eq!(
        names(&got),
        [
            "Failing[alpha]",
            "Failing[beta]",
            "Failing[gamma]",
            "GetError[alpha]",
            "GetError[beta]",
            "GetError[gamma]"
        ],
        "{got:?}"
    );
    assert_eq!(got[0].1, Outcome::Pass);
    assert_eq!(got[1].1, Outcome::Fail("case failed: beta".into()));
    assert_eq!(got[2].1, Outcome::Pass);
    assert!(
        matches!(&got[3].1, Outcome::Fail(m) if m == "the body must not run for beta"),
        "{got:?}"
    );
    assert_eq!(got[4].1, Outcome::Fail("get failed".into()));
}

// BiDataDriven.al:631-668 (codeunits 81635-81637), header item 5: an invalid list ends the procedure with one failed result
// named after it, and none of its cases run.
#[test]
fn an_invalid_list_is_one_failed_result_under_the_procedure_name() {
    for (set, message) in [
        (
            "dup",
            "Data-driven test 'Bad': ListTestCases returned the duplicate identifier 'same'. Every test case must have a distinct identifier.",
        ),
        (
            "blank",
            "Data-driven test 'Bad': ListTestCases returned a blank identifier. Every test case must have a non-blank, distinct identifier that contains no square brackets.",
        ),
        (
            "bracket",
            "Data-driven test 'Bad': ListTestCases returned the identifier 'a[b', which contains a square bracket ('[' or ']'). Test case identifiers must not contain square brackets; they delimit the case name in 'Method[identifier]'.",
        ),
    ] {
        let got = outcomes(
            &format!(
                r#"
codeunit 50126 "DD Bad"
{{
    Subtype = Test;

    [TestDataSource(50102, '{set}')]
    procedure Bad(Context: Interface ITestContext)
    begin
        Error('no case may run');
    end;
}}
"#
            ),
            "DD Bad",
        );
        assert_eq!(got, [("Bad".to_string(), Outcome::Fail(message.into()))], "{set}");
    }
}

// BiDataDriven.al:669-680 and :681-702, header item 7: an error in ListTestCases, or a DataSourceContext member called in a
// source, ends the session on BC and no data-driven result of the codeunit is reported. The runner does not model a
// session that ends: the data-driven procedures are not run; the plain tests keep their results.
#[test]
fn a_session_that_bc_ends_leaves_the_data_driven_procedures_not_run() {
    for (source, set) in [("50102", "ListError"), ("50103", "CodeunitId"), ("50103", "Other")] {
        let got = outcomes(
            &format!(
                r#"
codeunit 50127 "DD Ends"
{{
    Subtype = Test;

    [TestDataSource(50102, 'Set1')]
    procedure Before(Context: Interface ITestContext)
    begin
    end;

    [TestDataSource({source}, '{set}')]
    procedure Ends(Context: Interface ITestContext)
    begin
    end;

    [Test]
    procedure Plain()
    begin
    end;
}}
"#
            ),
            "DD Ends",
        );
        assert_eq!(got[0], ("Plain".to_string(), Outcome::Pass), "{set}: {got:?}");
        assert!(got.len() > 1, "{set}: {got:?}");
        for (n, o) in &got[1..] {
            assert!(
                matches!(o, Outcome::NotRun(why) if why.contains("ends the test session")),
                "{set}: {n} {o:?}"
            );
        }
    }
}
