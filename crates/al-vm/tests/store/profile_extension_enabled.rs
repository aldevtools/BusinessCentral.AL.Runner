//! The Enabled column of "All Profile" when a profile extension declares `Enabled`: a profile that is
//! disabled and has an extension with `Enabled = true` shows as enabled, and the reverse (pageext/lt-u-page-props/UPageProps.al:340-365); an extension that declares
//! only a Caption leaves the profile's own Enabled (corpus pages/lt-y88-page-probes/Y88PageProbes.al:
//! 239-247, Y88_Profile_EnabledForms, objects in Y88PageObjects.al:323-358).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::AppSource;
use al_vm::{Engine, Session};

const SRC: &str = r#"
table 2000000178 "All Profile" {
    fields { field(1; Scope; Option) { OptionMembers = System,Tenant; } field(2; "App ID"; Guid) { } field(3; "Profile ID"; Code[30]) { }
        field(4; Description; Text[250]) { } field(5; "Role Center ID"; Integer) { } field(6; "Default Role Center"; Boolean) { }
        field(13; "Disable Personalization"; Boolean) { } field(14; "App Name"; Text[250]) { } field(15; Enabled; Boolean) { }
        field(16; Caption; Text[100]) { } field(17; Promoted; Boolean) { } }
    keys { key(PK; Scope, "App ID", "Profile ID") { } }
}
profile "P Off" { Caption = 'P Off'; Enabled = false; }
profile "P Base Off" { Caption = 'P Base Off'; Enabled = false; }
profileextension "P Base Off Ext" extends "P Base Off" { Enabled = true; }
profile "P Base On Off" { Caption = 'P Base On Off'; Enabled = true; }
profileextension "P Base On Off Ext" extends "P Base On Off" { Enabled = false; }
profile "P Base On" { Caption = 'P Base On'; Enabled = true; }
profileextension "P Base On Ext" extends "P Base On" { Caption = 'Ext caption'; }
codeunit 50100 C {
    procedure F(): Text
    var P: Record "All Profile";
    begin
        P.SetRange("Profile ID", 'P OFF');
        P.FindFirst();
        exit(Format(P.Enabled) + ';' + One('P BASE OFF') + ';' + One('P BASE ON') + ';' + One('P BASE ON OFF') + ';');
    end;
    local procedure One(Id: Code[30]): Text
    var P: Record "All Profile";
    begin
        P.SetRange("Profile ID", Id);
        P.FindFirst();
        exit(Format(P.Enabled));
    end;
}
"#;

fn run() -> Result<Value, Signal> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), SRC.into())],
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
fn an_extension_enabled_true_overrides_a_disabled_profile() {
    let r = run();
    assert!(matches!(&r, Ok(Value::Text(t)) if t == "No;Yes;Yes;No;"), "{r:?}");
}
