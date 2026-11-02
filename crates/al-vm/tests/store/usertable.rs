//! The platform's triggers on the User table (`al-vm/src/usertable.rs`), against fixture tables that
//! carry the platform's ids and field ids. The corpus pins are the OnPrem app's
//! access-control/TestUser*.al files; each test names the one it mirrors.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;
use al_vm::{Engine, Session};

const TABLES: &str = r#"
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

const HELPERS: &str = r#"
procedure NewUser(var U: Record User; Email: Text) begin
    U.Init(); U."User Security ID" := CreateGuid(); U."User Name" := CopyStr(Format(CreateGuid()), 1, 50);
    U."Authentication Email" := CopyStr(Email, 1, 250); U.Insert();
end;
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!(
        "{TABLES}\ncodeunit 50100 C {{ {HELPERS} procedure F(): Text var U: Record User; V: Record User; P: Record \"User Property\"; Ac: Record \"Access Control\"; i: Integer; begin {body} end; }}"
    );
    let l = al_testkit::compile(&src);
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
    if let Some(Some(r)) = blocked.get(oi).and_then(|o| o.get(ri)) {
        panic!("F is blocked: {r}");
    }
    let engine = Engine::new(l.program, blocked);
    let mut s = Session::new(engine);
    let inst = s.instance(oi);
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn error(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Error(e)) => e.message,
        other => panic!("expected an AL error, got {other:?}"),
    }
}

fn unsupported(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(s)) => s,
        other => panic!("expected Unsupported, got {other:?}"),
    }
}

/// TestUserAuthEmailTrigger.al:44-84, :109-140.
#[test]
fn the_email_is_normalised_on_insert_and_modify() {
    let r = run("NewUser(U, '   a@t.example.com   '); V.Get(U.\"User Security ID\"); \
         exit(V.\"Authentication Email\");");
    assert_eq!(text(r), "a@t.example.com");
    let r = run(
        "NewUser(U, 'Test Person <a@t.example.com>'); V.Get(U.\"User Security ID\"); \
         exit(V.\"Authentication Email\");",
    );
    assert_eq!(text(r), "a@t.example.com");
    let r = run(
        "NewUser(U, ''); U.\"Authentication Email\" := 'a@t.example.com, d@t.example.com'; U.Modify(); \
         V.Get(U.\"User Security ID\"); exit(V.\"Authentication Email\");",
    );
    assert_eq!(text(r), "d@t.example.com");
    let r = run(
        "NewUser(U, 'a@t.example.com'); U.\"Authentication Email\" := '    '; U.Modify(); \
         V.Get(U.\"User Security ID\"); exit('[' + V.\"Authentication Email\" + ']');",
    );
    assert_eq!(text(r), "[]");
}

/// TestUserAuthEmailTrigger.al:86-106: Validate leaves the text, the Insert trims it.
#[test]
fn validate_does_not_normalise_but_insert_does() {
    let r = run(
        "U.Init(); U.\"User Security ID\" := CreateGuid(); U.\"User Name\" := 'N'; \
         U.Validate(\"Authentication Email\", '  a@t.example.com  '); \
         if U.\"Authentication Email\" <> '  a@t.example.com  ' then exit('validate changed it'); \
         U.Insert(); V.Get(U.\"User Security ID\"); exit(V.\"Authentication Email\");",
    );
    assert_eq!(text(r), "a@t.example.com");
}

/// TestUserAuthEmailTrigger.al:143-159: refused, and the row is not inserted.
#[test]
fn an_address_without_an_at_sign_is_refused_and_nothing_is_inserted() {
    let r = run(
        "U.Init(); U.\"User Security ID\" := CreateGuid(); U.\"User Name\" := 'N'; \
         U.\"Authentication Email\" := 'Abc.example.com'; U.Insert(); exit('inserted');",
    );
    assert!(error(r).contains("You must specify a valid email address"));
}

/// TestUserAuthEmailTrigger.al:162-204, :275-319: only enabled users hold an address exclusively.
#[test]
fn only_enabled_users_hold_an_address_exclusively() {
    let taken = "NewUser(U, 'a@t.example.com'); ";
    let r = run(&format!(
        "{taken} U.Init(); U.\"User Security ID\" := CreateGuid(); U.\"User Name\" := 'N2'; \
         U.\"Authentication Email\" := ' a@t.example.com'; U.Insert(); exit('inserted');"
    ));
    assert!(error(r).contains("is already being used by another user"));
    // A disabled new user may carry it.
    let r = run(&format!(
        "{taken} V.Init(); V.\"User Security ID\" := CreateGuid(); V.\"User Name\" := 'N2'; \
         V.State := V.State::Disabled; V.\"Authentication Email\" := 'a@t.example.com'; V.Insert(); exit('ok');"
    ));
    assert_eq!(text(r), "ok");
    // A disabled first user releases it.
    let r = run(
        "NewUser(U, 'a@t.example.com'); U.State := U.State::Disabled; U.Modify(); \
         NewUser(V, 'a@t.example.com'); exit('ok');",
    );
    assert_eq!(text(r), "ok");
    // Modify refuses another enabled user's address, and keeps its own.
    let r = run(
        "NewUser(U, 'a@t.example.com'); NewUser(V, 'b@t.example.com'); V.\"Authentication Email\" := 'a@t.example.com'; V.Modify(); exit('x');",
    );
    assert!(error(r).contains("is already being used by another user"));
    let r = run("NewUser(U, 'a@t.example.com'); U.\"Full Name\" := 'R'; U.Modify(); exit('ok');");
    assert_eq!(text(r), "ok");
}

/// TestUserAuthEmailTrigger.al:321-351.
#[test]
fn modify_refuses_the_name_of_another_user_and_accepts_a_fresh_one() {
    let r = run("NewUser(U, ''); NewUser(V, ''); V.\"User Name\" := U.\"User Name\"; V.Modify(); exit('x');");
    assert!(error(r).contains("The user name must be unique"));
    let r = run("NewUser(U, ''); U.\"User Name\" := 'FRESH'; U.Modify(); exit('ok');");
    assert_eq!(text(r), "ok");
}

/// TestUserAuthEmailTrigger.al:241-272.
#[test]
fn changing_the_email_clears_the_authentication_object_id() {
    let set = "NewUser(U, 'a@t.example.com'); P.Get(U.\"User Security ID\"); P.\"Authentication Object ID\" := 'OBJ'; P.Modify(); ";
    let r = run(&format!(
        "{set} U.\"Authentication Email\" := 'b@t.example.com'; U.Modify(); P.Get(U.\"User Security ID\"); exit('[' + P.\"Authentication Object ID\" + ']');"
    ));
    assert_eq!(text(r), "[]");
    let r = run(&format!(
        "{set} U.\"Full Name\" := 'R'; U.Modify(); P.Get(U.\"User Security ID\"); exit(P.\"Authentication Object ID\");"
    ));
    assert_eq!(text(r), "OBJ");
}

/// TestUserPropertyPageInsert.al:219-237: every inserted User has a User Property row.
#[test]
fn every_inserted_user_has_a_user_property_row_and_deleting_it_removes_it() {
    let r = run(
        "U.Init(); U.\"User Security ID\" := CreateGuid(); U.\"User Name\" := 'N'; U.Insert(false); \
         if not P.Get(U.\"User Security ID\") then exit('no row'); \
         if P.Get(CreateGuid()) then exit('row for nobody'); \
         U.Delete(); if P.Get(U.\"User Security ID\") then exit('row stayed'); exit('ok');",
    );
    assert_eq!(text(r), "ok");
    // TestUserDeleteCascadeOrder.al:62-84: DeleteAll goes through the same cascade.
    let r = run(
        "NewUser(U, ''); V.SetRange(\"User Security ID\", U.\"User Security ID\"); V.DeleteAll(); \
         if P.Get(U.\"User Security ID\") then exit('row stayed'); exit('ok');",
    );
    assert_eq!(text(r), "ok");
}

/// Hard rule 4: what nothing pins is refused, not run.
#[test]
fn what_the_corpus_does_not_pin_is_refused() {
    let r = run("NewUser(U, ''); U.ModifyAll(\"Full Name\", 'x'); exit('x');");
    assert!(unsupported(r).contains("ModifyAll on the User table"));
    let r = run(
        "U.Init(); U.\"User Security ID\" := CreateGuid(); U.\"User Name\" := 'N'; U.\"Windows Security ID\" := 'S-1'; U.Insert(); exit('x');",
    );
    assert!(unsupported(r).contains("Windows Security ID"));
}

/// Two '@' is an invalid form, pinned by Y66AuthEmailLocalPart.al:25-125 (BC 27.5, 28.5, 29.0).
#[test]
fn two_at_signs_in_the_authentication_email_are_an_error() {
    let r = run(
        "U.Init(); U.\"User Security ID\" := CreateGuid(); U.\"User Name\" := 'N'; U.\"Authentication Email\" := 'a@b@c.d'; U.Insert(); exit('x');",
    );
    assert_eq!(error(r), "You must specify a valid email address.");
}

/// Deleting a User deletes its Access Control rows, and no one else's (staging
/// tests/al-language-onprem/session/PZUserDelete.al:DeleteUser_WithRunTrigger_RemovesItsAccessControlRows and
/// DeleteUser_WithoutRunTrigger_AccessControlRows, BC 29.0; also what Microsoft's codeunit 134614 needs).
#[test]
fn deleting_a_user_deletes_its_access_control_rows_and_only_those() {
    let r = run("NewUser(U, ''); NewUser(V, ''); \
         Ac.\"User Security ID\" := U.\"User Security ID\"; Ac.\"Role ID\" := 'R'; Ac.Insert(); \
         Ac.\"Role ID\" := 'S'; Ac.Insert(); \
         Ac.\"User Security ID\" := V.\"User Security ID\"; Ac.\"Role ID\" := 'R'; Ac.Insert(); \
         U.Delete(); \
         Ac.SetRange(\"User Security ID\", U.\"User Security ID\"); i := Ac.Count(); \
         Ac.SetRange(\"User Security ID\", V.\"User Security ID\"); exit(Format(i) + '/' + Format(Ac.Count()));");
    assert_eq!(text(r), "0/1");
    // DeleteAll goes through the same cascade.
    let r = run(
        "NewUser(U, ''); Ac.\"User Security ID\" := U.\"User Security ID\"; Ac.\"Role ID\" := 'R'; Ac.Insert(); \
         V.SetRange(\"User Security ID\", U.\"User Security ID\"); V.DeleteAll(); \
         Ac.SetRange(\"User Security ID\", U.\"User Security ID\"); exit(Format(Ac.Count()));",
    );
    assert_eq!(text(r), "0");
}

/// The first full corpus run of this change never finished: the scan of the User rows did not
/// advance and collected the first row until memory ran out. Many users must be read in a bounded
/// loop.
#[test]
fn writing_many_users_terminates() {
    let r = run("for i := 1 to 50 do NewUser(U, ''); U.Reset(); exit(Format(U.Count()));");
    // 50 new users and the session user the platform seeds.
    assert_eq!(text(r), "51");
}

/// With IsSaaS true (the default environment) deleting the only enabled SUPER user raises
/// "There should be at least one enabled 'SUPER' user." with code Dialog (staging
/// PZUserDelete.al:DeleteAllUsers_AsTheOnlySuperUser_IsRefused, BC 29.0). Other users go.
#[test]
fn deleting_the_only_super_user_is_refused_on_saas() {
    let r = run(
        "NewUser(U, ''); Ac.\"User Security ID\" := U.\"User Security ID\"; Ac.\"Role ID\" := 'D365 BASIC'; Ac.Insert(); \
         U.Delete(true); exit('ok');",
    );
    assert_eq!(text(r), "ok");
    let r = run("U.SetRange(\"User Security ID\", UserSecurityId()); U.FindFirst(); U.Delete(true); exit('ok');");
    match r {
        Err(Signal::Error(e)) => {
            assert_eq!(e.message, "There should be at least one enabled 'SUPER' user.");
            assert_eq!(e.code, "Dialog");
        }
        other => panic!("{other:?}"),
    }
    // A second enabled SUPER user lets the first go.
    let r = run(
        "NewUser(V, ''); Ac.\"User Security ID\" := V.\"User Security ID\"; Ac.\"Role ID\" := 'SUPER'; Ac.Insert(); \
         U.SetRange(\"User Security ID\", UserSecurityId()); U.FindFirst(); U.Delete(true); exit('ok');",
    );
    assert_eq!(text(r), "ok");
}

fn error_with_code(r: Result<Value, Signal>) -> (String, String) {
    match r {
        Err(Signal::Error(e)) => (e.message, e.code),
        other => panic!("expected an AL error, got {other:?}"),
    }
}

/// access-control/lt-av-user-insert/AvUserInsertErrors.al:45-120: the full texts and codes. A used Insert result still
/// raises the name error.
#[test]
fn the_user_trigger_errors_carry_their_full_texts_and_codes() {
    let r = run(
        "U.Init(); U.\"User Security ID\" := CreateGuid(); U.\"User Name\" := 'N'; \
         U.\"Authentication Email\" := 'ltav.no.at.example.com'; U.Insert(true); exit('inserted');",
    );
    assert_eq!(
        error_with_code(r),
        (
            "You must specify a valid email address.".into(),
            "CannotSpecifyAuthenticationEmail".into()
        )
    );
    let r = run(
        "NewUser(U, 'ltav.taken@example.com'); V.Init(); V.\"User Security ID\" := CreateGuid(); V.\"User Name\" := 'N2'; \
         V.\"Authentication Email\" := 'ltav.taken@example.com'; V.Insert(true); exit('inserted');",
    );
    assert_eq!(
        error_with_code(r),
        (
            "The specified authentication email address 'ltav.taken@example.com' is already being used by another user. You must specify a unique email address.".into(),
            "UserTableAuthEmailMustBeUnique".into()
        )
    );
    let r = run("NewUser(U, ''); NewUser(V, ''); V.\"User Name\" := U.\"User Name\"; V.Modify(true); exit('x');");
    assert_eq!(
        error_with_code(r),
        (
            "The user name must be unique.".into(),
            "UserTableUserNameMustBeUnique".into()
        )
    );
    let r = run(
        "NewUser(U, ''); V.Init(); V.\"User Security ID\" := CreateGuid(); V.\"User Name\" := U.\"User Name\"; \
         if V.Insert(true) then exit('true'); exit('false');",
    );
    assert_eq!(
        error_with_code(r),
        (
            "The user name must be unique.".into(),
            "UserTableUserNameMustBeUnique".into()
        )
    );
}
