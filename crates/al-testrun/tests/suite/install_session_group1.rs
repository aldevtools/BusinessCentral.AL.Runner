//! Al-runner issue 8071: a filter in filter group 1 (the global group) stays on the table for the rest of the session
//! (types/text-code-char/P145PageRunView.al:232-235), and each app under test installs in a session of its own
//! (record/BfInstallTests.al:142-144, ADR-046 amendment 1, A4 item 1). The filter one app's install session set is not
//! there in the next app's install session, nor in the test session. Within one install session it stays (main's refusal).

use al_sema::workspace::AppSource;
use al_testrun::runtime::CorpusRuntime;
use al_testrun::{CaseId, Outcome};

const FIRST_ID: &str = "11111111-1111-1111-1111-111111118071";
const SECOND_ID: &str = "22222222-2222-2222-2222-222222228071";

/// The table, and an install routine that sets a group 1 filter on it through a local record and leaves it set.
const FIRST: &str = r#"
table 50100 "G1 Item"
{
    fields { field(1; "No."; Code[20]) { } field(2; Blocked; Boolean) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
codeunit 50101 "G1 Setter"
{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var
        I: Record "G1 Item";
    begin
        I.FilterGroup(1);
        I.SetRange(Blocked, false);
        I.FilterGroup(0);
    end;
}
"#;

/// An install routine that filters the table through another record and logs that it ran.
fn reader(id: u32) -> String {
    format!(
        r#"
table {id} "G1 Log {id}"
{{
    fields {{ field(1; "No."; Integer) {{ }} }}
    keys {{ key(PK; "No.") {{ Clustered = true; }} }}
}}
codeunit {id} "G1 Reader {id}"
{{
    Subtype = Install;
    trigger OnInstallAppPerCompany()
    var
        I: Record "G1 Item";
        L: Record "G1 Log {id}";
    begin
        I.SetRange("No.", 'A');
        if I.IsEmpty() then begin
            L."No." := 1;
            L.Insert();
        end;
    end;
}}
"#
    )
}

fn tests(log: u32) -> String {
    format!(
        r#"
codeunit 50400 "G1 Tests"
{{
    Subtype = Test;

    [Test]
    procedure ReaderRan()
    var
        L: Record "G1 Log {log}";
    begin
        L.Get(1);
    end;

    [Test]
    procedure TestSessionFilters()
    var
        I: Record "G1 Item";
    begin
        I.SetRange("No.", 'A');
        if not I.IsEmpty() then Error('a row');
    end;
}}
"#
    )
}

/// The test session's own filter on the table, with no install routine that reads it.
fn tests_no_reader() -> String {
    tests(0).replace(
        "    [Test]\n    procedure ReaderRan()\n    var\n        L: Record \"G1 Log 0\";\n    begin\n        L.Get(1);\n    end;\n\n",
        "",
    )
}

fn app(id: &str, deps: &[&str], src: String) -> AppSource {
    AppSource {
        id: id.into(),
        name: format!("G1 {}", &id[..1]),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![("f.al".into(), src)],
    }
}

/// Where the reader is.
enum Reader {
    /// In a second app under test (another install session).
    OtherApp,
    /// In the setter's app (the same install session).
    SameApp,
    /// Nowhere: only the test session filters the table.
    None,
}

fn outcome(reader_at: Reader, name: &str) -> Outcome {
    let apps = match reader_at {
        Reader::OtherApp => vec![
            app(FIRST_ID, &[], FIRST.into()),
            app(SECOND_ID, &[FIRST_ID], format!("{}{}", reader(50200), tests(50200))),
        ],
        Reader::SameApp => vec![app(FIRST_ID, &[], format!("{FIRST}{}{}", reader(50200), tests(50200)))],
        Reader::None => vec![app(FIRST_ID, &[], format!("{FIRST}{}", tests_no_reader()))],
    };
    let kinds = vec![al_sema::AppKind::Workspace; apps.len()];
    let rt = CorpusRuntime::run_for_apps(apps, kinds, &[], &Default::default(), None, Default::default());
    rt.first_outcome(&CaseId {
        codeunit: 50400,
        name: name.into(),
    })
}

#[test]
fn the_next_apps_install_session_starts_without_the_group_1_filter() {
    assert_eq!(outcome(Reader::OtherApp, "ReaderRan"), Outcome::Pass);
}

#[test]
fn the_test_session_starts_without_the_install_sessions_group_1_filter() {
    assert_eq!(outcome(Reader::None, "TestSessionFilters"), Outcome::Pass);
    assert_eq!(outcome(Reader::OtherApp, "TestSessionFilters"), Outcome::Pass);
}

/// Main's refusal: in the same install session the filter is still there, and the runtime does not apply it to another record.
#[test]
fn a_later_routine_of_the_same_install_session_keeps_mains_refusal() {
    match outcome(Reader::SameApp, "ReaderRan") {
        Outcome::NotRun(why) => assert!(why.contains("filter group 1"), "{why}"),
        other => panic!("expected not run, got {other:?}"),
    }
}
