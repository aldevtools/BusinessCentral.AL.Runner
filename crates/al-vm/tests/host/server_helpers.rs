//! Three server classes the System Application calls through DotNet, as pinned by corpus
//! tests/al-language-onprem-dotnet/lt-y58-server-helpers/Y58ServerHelpers.al (corpus master 7330f181; real BC 27.5, 28.5,
//! 29.0): `PasswordGenerator.GeneratePassword`, `VSCodeRequestHelper.GetLaunchInformationQueryPart` and
//! `ALSearch.IsItemSearchReady`. The AL below is my own minimal version of those calls.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{boolean, text};

const SRC: &str = r#"
dotnet {
    assembly("Microsoft.Dynamics.Nav.PasswordGenerator") {
        type("Microsoft.Dynamics.Nav.PasswordGenerator.PasswordGenerator"; "Gen") { }
    }
    assembly("Microsoft.Dynamics.Nav.Ncl") { type("Microsoft.Dynamics.Nav.Runtime.VSCodeRequestHelper"; "VsHelper") { } }
    assembly("Microsoft.Dynamics.Nav.DataSearch") { type("Microsoft.Dynamics.Nav.DataSearch.AL.ALSearch"; "Search") { } }
}
codeunit 50100 C
{
    procedure T(): Text
    var G: DotNet Gen; V: DotNet VsHelper; S: DotNet Search; P: Text; I: Integer; J: Integer; N: Integer;
    begin
        __BODY__
    end;
    procedure B(): Boolean
    var S: DotNet Search;
    begin
        exit(S.IsItemSearchReady());
    end;
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    al_testkit::run_in(&SRC.replace("__BODY__", body), "C", "T")
}

fn error(r: Result<Value, Signal>) -> (String, String) {
    match r {
        Err(Signal::Error(e)) => (e.message.clone(), e.code.clone()),
        other => panic!("expected an error, got {other:?}"),
    }
}

const GEN: &str =
    "A call to Microsoft.Dynamics.Nav.PasswordGenerator.PasswordGenerator.GeneratePassword failed with this message: ";

#[test]
fn a_password_has_the_length_asked_and_at_least_the_non_alphanumerics_asked() {
    // PasswordGenerator_Shape_16_10 (:183): length exact, the minimum of non-alphanumerics honored, from 25 characters.
    let r = run(
        "for I := 1 to 50 do begin P := G.GeneratePassword(16, 10); if StrLen(P) <> 16 then exit('length'); \
         N := 0; for J := 1 to 16 do if StrPos('!#$%&()*+-./:;=>?@[]^_{|}', Format(P[J])) > 0 then N += 1 else \
         if StrPos('abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789', Format(P[J])) = 0 then exit('char ' + P); \
         if N < 10 then exit('few ' + P); end; exit('ok');",
    );
    assert_eq!(text(r), "ok");
}

#[test]
fn consecutive_passwords_differ() {
    assert_eq!(
        text(run(
            "P := G.GeneratePassword(8, 1); if P = G.GeneratePassword(8, 1) then exit('same'); exit('ok');"
        )),
        "ok"
    );
}

#[test]
fn out_of_range_arguments_are_argument_errors() {
    // PasswordGenerator_Arguments (:207).
    for (len, min, msg) in [
        (0, 0, "Password length incorrect"),
        (129, 1, "Password length incorrect"),
        (-1, 0, "Password length incorrect"),
        (4, 6, "Number of non alphanumeric characters is incorrect"),
        (8, -1, "Number of non alphanumeric characters is incorrect"),
    ] {
        let (m, code) = error(run(&format!("exit(G.GeneratePassword({len}, {min}));")));
        assert_eq!(m, format!("{GEN}{msg}"), "{len}/{min}");
        assert_eq!(code, "DotNetInvoke:Argument");
    }
    assert_eq!(
        text(run(
            "exit(Format(StrLen(G.GeneratePassword(1, 0))) + '/' + Format(StrLen(G.GeneratePassword(128, 1))));"
        )),
        "1/128"
    );
}

#[test]
fn the_launch_query_part_names_six_parameters_with_values() {
    // VSCodeRequestHelper_LaunchQueryPart_Names (:250): no leading ? or &, names in this order, no empty value.
    let q = text(run("exit(V.GetLaunchInformationQueryPart());"));
    assert!(!q.starts_with('?') && !q.starts_with('&'), "{q}");
    let names: Vec<&str> = q.split('&').map(|p| p.split_once('=').expect(p).0).collect();
    assert_eq!(
        names,
        [
            "server",
            "serverInstance",
            "port",
            "authentication",
            "runtime",
            "appVersion"
        ]
    );
    assert!(q.split('&').all(|p| !p.ends_with('=')), "{q}");
}

#[test]
fn item_search_is_not_ready_in_a_test_session() {
    // ALSearch_IsItemSearchReady (:322).
    assert!(!boolean(al_testkit::run_in(SRC, "C", "B")));
}

#[test]
fn other_item_search_members_are_refused() {
    match run("S.EnableItemSearch(); exit('');") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("EnableItemSearch"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
