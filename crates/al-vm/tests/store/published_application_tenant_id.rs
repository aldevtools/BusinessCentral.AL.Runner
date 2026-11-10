//! "Published As" (column 11) and "Tenant ID" (column 12) of Published Application: Dev and `TenantId()` for an app under
//! test and for another publisher's dependency, Global and blank for a Microsoft dependency (onprem
//! session-system/PZPublishedApplicationColumns.al:96-109, onprem metadata/platform-tables/AwOnPremTests.al:137-139,
//! record/lt-r7142-installed-app/R7169DepInstallPublishedAs.al:70); Global and blank for another publisher's dependency
//! under `--dependencies-published-global`. Against a fixture table with the platform's id and field ids.

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
        field(11; "Published As"; Option) { OptionMembers = Global,PTE,Dev; }
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
        exit('[' + R."Tenant ID" + '] ' + Format(R."Published As", 0, 2));
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

    // The filter of Microsoft's retention policy module ("Reten. Pol. Allowed Tbl. Impl.", ModuleOwnsTable).
    procedure OtherPassesTheTenantFilter(): Text
    var
        R: Record "Published Application";
    begin
        R.SetRange(Publisher, 'Partner');
        R.SetFilter("Tenant ID", '%1|%2', '', TenantId());
        exit(Format(not R.IsEmpty()));
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

fn built(global: bool) -> al_testkit::Built {
    let mut l = al_testkit::compile(SRC);
    std::sync::Arc::make_mut(&mut l.program).apps = vec![
        app("aa006438-a411-4309-b899-92353cc12b05", "Repro", false),
        app("63ca2fa4-4f03-4f2b-a480-172fef340d3f", "Microsoft", true),
        app("bb006438-a411-4309-b899-92353cc12b05", "Partner", true),
        // The Application app is in the program, so its row is an app's row (the fixture has no column for the other one).
        app("c1335042-3002-4257-bf8a-75c898ccb1b8", "Microsoft", true),
    ];
    let mut engine = al_testkit::engine(&l);
    std::sync::Arc::get_mut(&mut engine)
        .unwrap()
        .dependencies_published_global = global;
    al_testkit::Built {
        session: al_vm::Session::new(engine),
        lowered: l,
    }
}

fn run(routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    built(false).run_in("C", routine)
}

#[test]
fn an_app_under_test_is_dev_with_the_tenant_id_and_a_microsoft_dependency_global_and_blank() {
    assert_eq!(al_testkit::text(run("UnderTest")), "[default] 2|default");
    assert_eq!(al_testkit::text(run("Microsoft")), "[] 0");
}

#[test]
fn another_publishers_dependency_is_dev_with_the_tenant_id() {
    assert_eq!(al_testkit::text(run("Other")), "[default] 2");
    assert_eq!(al_testkit::text(run("OtherPassesTheTenantFilter")), "Yes");
}

/// `--dependencies-published-global`: Global and blank, a flagged answer; the row still passes Microsoft's filter.
#[test]
fn another_publishers_dependency_is_global_and_blank_under_the_option() {
    use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
    let mut b = built(true);
    clear_hits();
    assert_eq!(al_testkit::text(b.run_in("C", "Other")), "[] 0");
    assert!(take_hits().contains(&Assumption::DependencyPublishedAsGlobal));
    assert_eq!(al_testkit::text(b.run_in("C", "OtherPassesTheTenantFilter")), "Yes");
    clear_hits();
    assert_eq!(al_testkit::text(run("Other")), "[default] 2");
    assert!(!take_hits().contains(&Assumption::DependencyPublishedAsGlobal));
    // The app under test stays Dev, Microsoft's apps stay Global.
    assert_eq!(al_testkit::text(b.run_in("C", "UnderTest")), "[default] 2|default");
    assert_eq!(al_testkit::text(b.run_in("C", "Microsoft")), "[] 0");
}
