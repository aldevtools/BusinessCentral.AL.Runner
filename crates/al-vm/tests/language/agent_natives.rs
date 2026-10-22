//! The `[Native]` procedures behind the System Application agent API, in a test session that is not an agent session
//! (al-runner issues 5685 and 5686). Codeunits 2000000020 "Feature Access Management" and 2000000022 "Agent Utilities" are
//! declared as System.app declares them; a second Microsoft app ("sysapp") wraps the access check the way "Agent Task Builder" does.

use al_builtins::native::Signal;
use al_testkit::{Built, text};
use al_sema::workspace::AppSource;

const SYS: &str = r#"
codeunit 2000000020 "Feature Access Management"
{
    procedure AgentManagementAllowed(ThrowError: Boolean): Boolean begin exit(AgentManagementAllowedInternal(ThrowError)); end;
    procedure IsSandboxEnvironmentOrFirstPartyCode(ThrowError: Boolean): Boolean begin exit(IsSandboxEnvironmentOrFirstPartyCodeInternal(ThrowError)); end;
    procedure IsEnvironmentPositiveListed(): Boolean begin exit(IsEnvironmentPositiveListedInternal()); end;
    procedure IsSandboxEnvironment(): Boolean begin exit(IsSandboxEnvironmentInternal()); end;
    procedure IsTestSession(): Boolean begin exit(IsTestSessionInternal()); end;
    [Native] local procedure AgentManagementAllowedInternal(ThrowError: Boolean): Boolean begin end;
    [Native] local procedure IsSandboxEnvironmentOrFirstPartyCodeInternal(ThrowError: Boolean): Boolean begin end;
    [Native] local procedure IsSandboxEnvironmentInternal(): Boolean begin end;
    [Native] local procedure IsEnvironmentPositiveListedInternal(): Boolean begin end;
    [Native] local procedure IsTestSessionInternal(): Boolean begin end;
}
codeunit 2000000022 "Agent Utilities"
{
    [Native] procedure GetCurrentSessionAgentTaskId(): Integer begin end;
    [Native] procedure GetCurrentSessionAgentMetadataProviderType(): Integer begin end;
}
"#;

// System Application: a Microsoft app of its own, as in BC.
const SYSAPP: &str = r#"
codeunit 4315 "Builder"
{
    procedure Initialize(): Text
    var Fam: Codeunit "Feature Access Management";
    begin
        Fam.AgentManagementAllowed(true);
        exit('returned');
    end;
    procedure NoThrow(): Boolean
    var Fam: Codeunit "Feature Access Management";
    begin
        exit(Fam.AgentManagementAllowed(false));
    end;
    procedure Sandbox(): Boolean
    var Fam: Codeunit "Feature Access Management";
    begin
        exit(Fam.IsSandboxEnvironment());
    end;
}
"#;

const PARTNER: &str = r#"
codeunit 50100 P
{
    procedure Ids(): Text
    var A: Codeunit "Agent Utilities";
    begin
        exit(Format(A.GetCurrentSessionAgentTaskId()) + '|' + Format(A.GetCurrentSessionAgentMetadataProviderType()));
    end;
    procedure Checks(): Text
    var Fam: Codeunit "Feature Access Management"; Out: Text; i: Integer;
    begin
        for i := 1 to 6 do
            Out += Format(Try(i)) + ':' + GetLastErrorText() + ':' + GetLastErrorCode() + '|';
        exit(Out);
    end;
    [TryFunction]
    local procedure Try(Which: Integer)
    var Fam: Codeunit "Feature Access Management"; B: Boolean;
    begin
        ClearLastError();
        case Which of
            1: B := Fam.AgentManagementAllowed(false);
            2: B := Fam.AgentManagementAllowed(true);
            3: B := Fam.IsSandboxEnvironmentOrFirstPartyCode(true);
            4: B := Fam.IsEnvironmentPositiveListed();
            5: B := Fam.IsTestSession();
            6: if not Fam.IsSandboxEnvironment() then Error('not a sandbox');
        end;
    end;
    procedure ViaMicrosoft(): Text
    var B: Codeunit Builder;
    begin
        exit(B.Initialize());
    end;
    procedure ViaMicrosoftNoThrow(): Boolean
    var B: Codeunit Builder;
    begin
        exit(B.NoThrow());
    end;
    procedure SandboxViaMicrosoft(): Boolean
    var B: Codeunit Builder;
    begin
        exit(B.Sandbox());
    end;
}
"#;

fn app(id: &str, deps: &[&str], src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: vec![("f.al".into(), src.into())],
    }
}

fn built() -> Built {
    let mut l = al_testkit::compile_apps(vec![
        app("sys", &[], SYS),
        app("sysapp", &["sys"], SYSAPP),
        app("partner", &["sys", "sysapp"], PARTNER),
    ]);
    for a in &mut std::sync::Arc::make_mut(&mut l.program).apps {
        a.publisher = if a.id.starts_with("sys") { "Microsoft".into() } else { "Partner".into() };
    }
    Built::new(l)
}

// Agent Utilities outside an agent session: task id 0 (platform/lt-rh-agent-session/RhAgentSession.al:47 through
// "Agent Session"; al-language-onprem/platform/lt-rh-agent-natives/RhAgentNatives.al:82), provider type -1 (RhAgentNatives.al:83).
#[test]
fn agent_utilities_outside_an_agent_session() {
    assert_eq!(text(built().run_in("P", "Ids")), "0|-1");
}

// Called from a non-Microsoft app, every Feature Access Management check but IsSandboxEnvironment raises the pinned error
// whatever ThrowError says; IsSandboxEnvironment answers Yes (RhAgentNatives.al:104, :126, :156-178).
#[test]
fn feature_access_checks_from_a_partner_app() {
    let e = "No:Feature availability can only be checked by Microsoft applications.:InvalidOperation|";
    assert_eq!(text(built().run_in("P", "Checks")), format!("{e}{e}{e}{e}{e}Yes::|"));
}

// Called by the System Application, AgentManagementAllowed(true) returns without an error (RhAgentSession.al:66-69,
// Agent Task Builder.Initialize). ThrowError false from Microsoft code and IsSandboxEnvironment from Microsoft code are not
// pinned: refused.
#[test]
fn feature_access_from_microsoft_code() {
    assert_eq!(text(built().run_in("P", "ViaMicrosoft")), "returned");
    for r in ["ViaMicrosoftNoThrow", "SandboxViaMicrosoft"] {
        match built().run_in("P", r) {
            Err(Signal::Unsupported(_)) => {}
            other => panic!("{r}: {other:?}"),
        }
    }
}
