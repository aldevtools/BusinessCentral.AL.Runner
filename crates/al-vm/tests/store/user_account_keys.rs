//! The `NavUserAccountHelper` members the User Card reads (`al-vm/src/user_password.rs` `user_password_is_set`,
//! `al-vm/src/user_account_keys.rs`), against fixture tables with the platform's ids and field names. The measured answers
//! are pinned by corpus PR 1330 (al-language-onprem session-system/database-and-blob-operations/
//! SetUserPasswordStatement.al:284, :323); these tests hold the refusals of the states the probe did not measure.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;
use al_vm::{Engine, Session};

const SOURCE: &str = r#"
dotnet { assembly("Microsoft.Dynamics.Nav.NavUserAccount") {
    type("Microsoft.Dynamics.Nav.NavUserAccount.NavUserAccountHelper"; "NavUserAccountHelper") { }
} }
table 2000000120 "User" {
    fields {
        field(1; "User Security ID"; Guid) { }
        field(2; "User Name"; Code[50]) { }
        field(3; "Full Name"; Text[80]) { }
        field(4; State; Option) { OptionMembers = Enabled,Disabled; }
        field(7; "Windows Security ID"; Text[119]) { }
        field(11; "Authentication Email"; Text[250]) { }
        field(16; "Application ID"; Guid) { }
    }
    keys { key(PK; "User Security ID") { } }
}
table 2000000121 "User Property" {
    fields {
        field(1; "User Security ID"; Guid) { }
        field(2; Password; Text[80]) { }
        field(3; "Name Identifier"; Text[250]) { }
        field(4; "Authentication Key"; Text[80]) { }
        field(5; "WebServices Key"; Text[80]) { }
        field(6; "WebServices Key Expiry Date"; DateTime) { }
        field(7; "Authentication Object ID"; Text[80]) { }
        field(10; "Telemetry User ID"; Guid) { }
    }
    keys { key(PK; "User Security ID") { } }
}
table 2000000053 "Access Control" {
    fields { field(1; "User Security ID"; Guid) { } field(2; "Role ID"; Code[20]) { } }
    keys { key(PK; "User Security ID", "Role ID") { } }
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!(
        "{SOURCE}\ncodeunit 50100 C {{ \
         procedure NewUser(var U: Record User) begin U.Init(); U.\"User Security ID\" := CreateGuid(); \
           U.\"User Name\" := 'LT-KEYS'; U.Insert(); end; \
         procedure F(): Text var U: Record User; P: Record \"User Property\"; H: DotNet NavUserAccountHelper; \
           K: Text[80]; N: Text[250]; D: DateTime; Unknown: Guid; begin {body} end; }}"
    );
    let l = al_testkit::compile(&src);
    let blocked: Vec<Vec<Option<String>>> = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().map(|r| r.to_string())).collect())
        .collect();
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == "F")
        .unwrap();
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        panic!("F is blocked: {r}");
    }
    let mut s = Session::new(Engine::new(l.program, blocked));
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn unsupported(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(s)) => s.into_reason(),
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

/// The measured answers for a user row AL inserted (SetUserPasswordStatement.al:284, :323).
#[test]
fn a_new_user_has_no_password_and_blank_keys() {
    let r = run("NewUser(U); \
         exit(Format(H.IsPasswordSet(U.\"User Security ID\")) + ' ' + \
           Format(H.TryGetAuthenticationKey(U.\"User Security ID\", K)) + '[' + K + '] ' + \
           Format(H.TryGetNameIdentifier(U.\"User Security ID\", N)) + '[' + N + '] ' + \
           Format(H.TryGetWebServicesKey(U.\"User Security ID\", K, D)) + '[' + K + '][' + Format(D) + '] ' + \
           Format(H.GetAuthenticationStatus(U.\"User Security ID\")));");
    assert_eq!(text(r), "No Yes[] Yes[] Yes[][] 0");
}

/// IsPasswordSet for an unknown user raises the NothingInsideFilter error of User Property (SetUserPasswordStatement.al:284).
#[test]
fn is_password_set_for_an_unknown_user_raises_nothing_inside_filter() {
    match run("Evaluate(Unknown, '{7E57C0DE-0000-4000-8000-00000000A0A2}'); exit(Format(H.IsPasswordSet(Unknown)));") {
        Err(Signal::Error(e)) => {
            assert_eq!(e.code, "DB:NothingInsideFilter");
            assert!(
                e.message.starts_with("There is no User Property within the filter."),
                "{}",
                e.message
            );
        }
        other => panic!("expected an AL error, got {other:?}"),
    }
}

/// The key members for an unknown user: not measured, refused.
#[test]
fn the_keys_of_an_unknown_user_are_refused() {
    for call in [
        "H.TryGetAuthenticationKey(Unknown, K)",
        "H.TryGetNameIdentifier(Unknown, N)",
        "H.TryGetWebServicesKey(Unknown, K, D)",
        "H.GetAuthenticationStatus(Unknown)",
    ] {
        let why = unsupported(run(&format!(
            "Evaluate(Unknown, '{{7E57C0DE-0000-4000-8000-00000000A0A2}}'); exit(Format({call}));"
        )));
        assert!(why.contains("without a User and a User Property row"), "{call}: {why}");
    }
}

/// A key AL wrote into User Property itself: not measured, refused.
#[test]
fn a_filled_key_field_is_refused() {
    for (field, call) in [
        (
            "Authentication Key",
            "H.TryGetAuthenticationKey(U.\"User Security ID\", K)",
        ),
        ("Name Identifier", "H.TryGetNameIdentifier(U.\"User Security ID\", N)"),
        (
            "WebServices Key",
            "H.TryGetWebServicesKey(U.\"User Security ID\", K, D)",
        ),
        (
            "Authentication Object ID",
            "H.GetAuthenticationStatus(U.\"User Security ID\")",
        ),
    ] {
        let why = unsupported(run(&format!(
            "NewUser(U); P.Get(U.\"User Security ID\"); P.\"{field}\" := 'x'; P.Modify(); exit(Format({call}));"
        )));
        assert!(
            why.contains(&format!("User Property {field} is not blank")),
            "{field}: {why}"
        );
    }
    let why = unsupported(run("NewUser(U); P.Get(U.\"User Security ID\"); \
         P.\"WebServices Key Expiry Date\" := CreateDateTime(20260101D, 0T); P.Modify(); \
         exit(Format(H.TryGetWebServicesKey(U.\"User Security ID\", K, D)));"));
    assert!(why.contains("WebServices Key Expiry Date is not blank"), "{why}");
}

/// GetAuthenticationStatus for a user with an Authentication Email: not measured, refused.
#[test]
fn the_authentication_status_of_a_user_with_an_email_is_refused() {
    let why = unsupported(run(
        "NewUser(U); U.\"Authentication Email\" := 'a@t.example.com'; U.Modify(); \
         exit(Format(H.GetAuthenticationStatus(U.\"User Security ID\")));",
    ));
    assert!(why.contains("Authentication Email"), "{why}");
}
