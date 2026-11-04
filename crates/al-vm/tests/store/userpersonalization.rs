//! "User Personalization" (2000000073): the platform creates the session user's row when the user
//! signs in to a client, and the test runner's session is that sign-in: one row, Scope Tenant, App ID
//! Base Application, language and locale 1033, a profile id that is not blank
//! (session-system/TestUserPersonalizationRowContracts.al). Rows a test
//! writes itself (profiles/TestTenantProfileRenameCascade.al:39-80) are ordinary rows.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_vm::{Engine, Session};

const TABLES: &str = r#"
table 2000000120 "User" {
    fields { field(1; "User Security ID"; Guid) { } field(2; "User Name"; Code[50]) { } field(3; "Full Name"; Text[80]) { } }
    keys { key(PK; "User Security ID") { } }
}
table 2000000073 "User Personalization" {
    fields { field(3; "User SID"; Guid) { } field(9; "Profile ID"; Code[30]) { } field(10; "App ID"; Guid) { } field(11; Scope; Option) { OptionMembers = System,Tenant; } field(12; "Language ID"; Integer) { } field(27; "Locale ID"; Integer) { } }
    keys { key(PK; "User SID") { } }
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!(
        "{TABLES}\ncodeunit 50100 C {{ procedure F(): Text var P: Record \"User Personalization\"; Other: Guid; begin {body} end; }}"
    );
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    let mut s = Session::new(Engine::new(l.program, blocked));
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

#[test]
fn the_session_users_row_exists_with_the_pinned_columns() {
    let r = run(
        "if not P.Get(UserSecurityId()) then exit('missing'); if P.Count() <> 1 then exit('count'); \
         exit(Format(P.Scope) + '|' + Format(P.\"App ID\") + '|' + Format(P.\"Language ID\") + '|' + Format(P.\"Locale ID\") + '|' + Format(P.\"Profile ID\" <> ''));",
    );
    assert!(
        matches!(&r, Ok(Value::Text(t)) if t == "Tenant|{437DBF0E-84FF-417A-965D-ED2BB9650972}|1033|1033|Yes"),
        "{r:?}"
    );
}

#[test]
fn rows_a_test_wrote_and_other_users_are_ordinary() {
    // The row of the session user is found and changed.
    let r = run("P.Get(UserSecurityId()); P.\"Profile ID\" := 'X'; P.Modify(); \
         P.Get(UserSecurityId()); P.\"Profile ID\" := 'Y'; P.Modify(); P.Get(UserSecurityId()); exit(P.\"Profile ID\");");
    assert!(matches!(&r, Ok(Value::Text(t)) if t == "Y"), "{r:?}");
    // Another user's missing row is the ordinary answer.
    let r = run("Other := CreateGuid(); exit(Format(P.Get(Other)));");
    assert!(matches!(&r, Ok(Value::Text(t)) if t == "No"), "{r:?}");
}
