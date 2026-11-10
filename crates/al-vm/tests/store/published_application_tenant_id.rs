//! "Tenant ID" (column 12) of Published Application: `TenantId()` for an app under test, blank for a Microsoft dependency
//! (onprem session-system/PZPublishedApplicationColumns.al:96-109, onprem metadata/platform-tables/AwOnPremTests.al:137-139),
//! and unknown for another publisher's dependency: a read is refused. Against a fixture table with the platform's id and
//! field ids.

const SRC: &str = r#"
table 2000000206 "Published Application"
{
    fields
    {
        field(1; "Runtime Package ID"; Guid) { }
        field(2; "Package ID"; Guid) { }
        field(3; ID; Guid) { }
        field(4; Name; Text[250]) { }
        field(5; Publisher; Text[250]) { }
        field(6; "Version Major"; Integer) { }
        field(7; "Version Minor"; Integer) { }
        field(8; "Version Build"; Integer) { }
        field(9; "Version Revision"; Integer) { }
        field(12; "Tenant ID"; Text[128]) { }
        field(15; Description; Blob) { }
        field(20; Logo; Media) { }
    }
    keys { key(PK; "Runtime Package ID") { } }
}
codeunit 50100 C
{
    procedure TenantOf(Publisher: Text): Text
    var
        R: Record "Published Application";
    begin
        R.SetRange(Publisher, Publisher);
        R.FindFirst();
        exit('[' + R."Tenant ID" + ']');
    end;

    procedure UnderTest(): Text
    begin
        exit(TenantOf('Repro') + '|' + TenantId());
    end;

    procedure Microsoft(): Text
    begin
        exit(TenantOf('Microsoft'));
    end;

    procedure Other(): Text
    begin
        exit(TenantOf('Partner'));
    end;

    procedure OtherName(): Text
    var
        R: Record "Published Application";
    begin
        R.SetRange(Publisher, 'Partner');
        R.FindFirst();
        exit(R.Name);
    end;
}
"#;

fn app(id: &str, publisher: &str, dependency: bool) -> al_ir::AppInfo {
    al_ir::AppInfo {
        id: id.into(),
        name: format!("{publisher} app"),
        publisher: publisher.into(),
        version: "1.0.0.0".into(),
        dependency,
        ..Default::default()
    }
}

fn run(routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let mut l = al_testkit::compile(SRC);
    std::sync::Arc::make_mut(&mut l.program).apps = vec![
        app("aa006438-a411-4309-b899-92353cc12b05", "Repro", false),
        app("63ca2fa4-4f03-4f2b-a480-172fef340d3f", "Microsoft", true),
        app("bb006438-a411-4309-b899-92353cc12b05", "Partner", true),
        // The Application app is in the program, so its row is an app's row (the fixture has no column for the other one).
        app("c1335042-3002-4257-bf8a-75c898ccb1b8", "Microsoft", true),
    ];
    let engine = al_testkit::engine(&l);
    al_testkit::Built {
        session: al_vm::Session::new(engine),
        lowered: l,
    }
    .run_in("C", routine)
}

#[test]
fn an_app_under_test_carries_the_tenant_id_and_a_microsoft_dependency_a_blank() {
    assert_eq!(al_testkit::text(run("UnderTest")), "[default]|default");
    assert_eq!(al_testkit::text(run("Microsoft")), "[]");
}

#[test]
fn the_tenant_id_of_another_publishers_dependency_is_refused_and_its_other_columns_read() {
    match run("Other") {
        Err(al_builtins::native::Signal::Unsupported(why)) => {
            assert!(why.contains("Tenant ID") && why.contains("Partner app"), "{why}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(al_testkit::text(run("OtherName")), "Partner app");
}
