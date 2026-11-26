//! `login::decide` for a test codeunit when an app under test's login subscriber reads what the dependency apps' login
//! code writes: the login is raised only for a test codeunit whose reach can see what the subscribers or that code leave.

use super::app_outcomes_tests::compiled_apps;
use super::login_tests::DEP;
use crate::login::Login;

/// One login subscriber that reads the checklist setup Microsoft's login code writes (as the corpus fixture
/// al-language-xapp-a/LmXaLoginCaches.al:279-293 reads User), and writes its own note; then one test codeunit each that
/// cannot see anything of the login, reads the note, reads the checklist setup, or reads the SingleInstance state of
/// System Application's login.
const MAIN: &str = r#"
table 51900 "D Note" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51901 "D Other" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
codeunit 51900 "D Login"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var Note: Record "D Note"; Checklist: Codeunit Checklist;
    begin
        if Checklist.ShouldInitializeChecklist(false) then exit;
        Note.Code := 'AFTER';
        Note.Insert();
    end;
}
codeunit 51910 "DA Unrelated"
{
    Subtype = Test;
    [Test]
    procedure A1() var O: Record "D Other"; begin O.Code := 'X'; O.Insert(); end;
}
codeunit 51911 "DB Reads Note"
{
    Subtype = Test;
    [Test]
    procedure B1() var Note: Record "D Note"; begin Note.Get('AFTER'); end;
}
codeunit 51912 "DC Reads Checklist"
{
    Subtype = Test;
    [Test]
    procedure C1() var Checklist: Codeunit Checklist; begin if Checklist.ShouldInitializeChecklist(false) then Error('not set up'); end;
}
codeunit 51913 "DD Reads Login State"
{
    Subtype = Test;
    [Test]
    procedure D1() var SysInit: Codeunit "System Initialization"; begin if SysInit.IsInProgress() then Error('in progress'); end;
}
"#;

fn decision(main: &str, codeunit: &str) -> Login {
    let (engine, l) = compiled_apps(&[("main", &["dep"], main), ("dep", &[], DEP)], &[]);
    let test = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    crate::login::decide(&engine, &l, test, None, false)
}

fn raised(login: &Login) -> bool {
    matches!(login, Login::Raise { fallback: false })
}

// A test codeunit whose reach sees nothing the login subscribers or Microsoft's login code leave gets the apps' own
// subscribers, as for subscribers that read nothing of Microsoft's login.
#[test]
fn a_test_that_cannot_see_the_login_gets_the_own_subscribers() {
    assert!(matches!(decision(MAIN, "DA Unrelated"), Login::Own { .. }));
}

// A test codeunit that reads what the subscribers write, what Microsoft's login code writes, or a global that code sets
// gets the login raised.
#[test]
fn a_test_that_can_see_the_login_gets_it_raised() {
    for codeunit in ["DB Reads Note", "DC Reads Checklist", "DD Reads Login State"] {
        assert!(raised(&decision(MAIN, codeunit)), "{codeunit}");
    }
}

// Subscribers that change session state (here WorkDate) leave something no table or global shows: every test codeunit
// gets the login raised.
#[test]
fn subscribers_that_change_session_state_raise_the_login_for_every_test() {
    let main = MAIN.replace("Note.Insert();", "Note.Insert(); WorkDate(20300101D);");
    assert!(raised(&decision(&main, "DA Unrelated")));
}
