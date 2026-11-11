//! "Published As" (column 16) of NAV App Installed App: Dev (2) for an app under test, Global (0) for a Microsoft dependency
//! (record/table-properties/Y12PublishedAs.al:31-80, al-runner issue 6438), and
//! unknown for another publisher's dependency; against a fixture table with the platform's id and field ids.

const SRC: &str = r#"
table 2000000153 "NAV App Installed App"
{
    fields
    {
        field(1; "App ID"; Guid) { }
        field(2; "Package ID"; Guid) { }
        field(3; Name; Text[250]) { }
        field(4; Publisher; Text[250]) { }
        field(5; "Version Major"; Integer) { }
        field(6; "Version Minor"; Integer) { }
        field(7; "Version Build"; Integer) { }
        field(8; "Version Revision"; Integer) { }
        field(16; "Published As"; Option) { OptionMembers = Global,PTE,Dev; }
    }
    keys { key(PK; "App ID") { } }
}
codeunit 50100 C
{
    procedure F(): Text
    var
        R: Record "NAV App Installed App";
        S: Text;
    begin
        R.SetRange(Publisher, 'Repro');
        R.FindFirst();
        S := Format(R."Published As", 0, 2);
        R.SetRange("Published As", R."Published As"::Global);
        S += '|' + Format(R.IsEmpty());
        R.SetRange("Published As", R."Published As"::Dev);
        S += '|' + Format(R.IsEmpty());
        R.Reset();
        R.SetRange(Publisher, 'Microsoft');
        R.FindFirst();
        exit(S + '|' + Format(R."Published As", 0, 2));
    end;

    procedure Other(): Text
    var
        R: Record "NAV App Installed App";
    begin
        R.SetRange(Publisher, 'Partner');
        R.FindFirst();
        exit(Format(R."Published As", 0, 2));
    end;

    procedure Total(): Integer
    var
        R: Record "NAV App Installed App";
    begin
        exit(R.Count());
    end;

    procedure OwnRow(): Text
    var
        R: Record "NAV App Installed App";
        Id: Guid;
    begin
        Evaluate(Id, '{aa006438-a411-4309-b899-92353cc12b05}');
        exit(Format(R.Get(Id)));
    end;

    procedure PlatformRow(): Text
    var
        R: Record "NAV App Installed App";
        Id: Guid;
    begin
        Evaluate(Id, '{8874ed3a-0643-4247-9ced-7a7002f7135d}');
        exit(Format(R.Get(Id)));
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
    built_without(global, &[])
}

/// [`built`] on an engine that leaves the apps `uninstalled` (indexes) uninstalled.
fn built_without(global: bool, uninstalled: &[u32]) -> al_testkit::Built {
    let mut l = al_testkit::compile(SRC);
    std::sync::Arc::make_mut(&mut l.program).apps = vec![
        app("aa006438-a411-4309-b899-92353cc12b05", "Repro", false),
        app("63ca2fa4-4f03-4f2b-a480-172fef340d3f", "Microsoft", true),
        app("bb006438-a411-4309-b899-92353cc12b05", "Partner", true),
        app("8874ed3a-0643-4247-9ced-7a7002f7135d", "Microsoft", true),
    ];
    let mut engine = al_testkit::engine(&l);
    let e = std::sync::Arc::get_mut(&mut engine).unwrap();
    e.dependencies_published_global = global;
    e.uninstalled_apps = uninstalled.iter().copied().collect();
    al_testkit::Built {
        session: al_vm::Session::new(engine),
        lowered: l,
    }
}

fn run(routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    built(false).run_in("C", routine)
}

#[test]
fn an_app_under_test_is_dev_and_a_microsoft_dependency_global() {
    assert_eq!(al_testkit::text(run("F")), "2|Yes|No|0");
}

/// R7169DepInstallPublishedAs.al:70 (`AL Language| Dev|2` for a dependency published through the development endpoint).
#[test]
fn another_publishers_dependency_is_dev() {
    assert_eq!(al_testkit::text(run("Other")), "2");
}

/// `--dependencies-published-global`: a documented setting for a pipeline that publishes dependencies as compiled apps.
#[test]
fn another_publishers_dependency_is_global_under_the_option() {
    use al_builtins::assumptions::{Assumption, clear_hits, take_hits};
    let mut b = built(true);
    clear_hits();
    assert_eq!(al_testkit::text(b.run_in("C", "Other")), "0");
    assert!(take_hits().contains(&Assumption::DependencyPublishedAsGlobal));
    clear_hits();
    assert_eq!(al_testkit::text(run("Other")), "2");
    assert!(!take_hits().contains(&Assumption::DependencyPublishedAsGlobal));
    // The app under test stays Dev, Microsoft's apps stay Global.
    assert_eq!(al_testkit::text(b.run_in("C", "F")), "2|Yes|No|0");
}

/// During an app's install its own row does not exist and the apps its install has not reached are not listed
/// (R7169DepInstallPublishedAs.al:36-45: `install_own` = `no row`; the filter counts of the install time).
#[test]
fn during_an_install_the_own_app_and_the_apps_after_it_have_no_row() {
    let mut b = built(false);
    let all = al_testkit::int(b.run_in("C", "Total"));
    assert_eq!(all, 3);
    b.session.set_apps_not_installed(Some([2].into()));
    b.session.set_installing_app(Some(0));
    assert_eq!(al_testkit::int(b.run_in("C", "Total")), 1);
    assert_eq!(al_testkit::text(b.run_in("C", "OwnRow")), "No");
    b.session.set_apps_install_order_unknown(None);
    b.session.set_apps_not_installed(None);
    assert_eq!(al_testkit::int(b.run_in("C", "Total")), 3);
}

/// The platform's System app (8874ed3a-...) has no NAV App Installed App row (Y12PublishedAs.al:41, :46 `no row`).
#[test]
fn the_platform_system_app_has_no_row() {
    assert_eq!(al_testkit::text(run("PlatformRow")), "No");
}

/// ADR-073 Phase 2: an app the engine leaves uninstalled (`Engine::uninstalled_apps`, the try-write suite's app on the main
/// suites' engine) is not on the tenant and has no row; BC's main job does not have that app, and the main test
/// R7169DepInstallPublishedAs.al counts the rows of the apps by its publisher.
#[test]
fn an_uninstalled_app_has_no_row() {
    assert_eq!(al_testkit::int(built(false).run_in("C", "Total")), 3);
    let mut b = built_without(false, &[2]);
    assert_eq!(al_testkit::int(b.run_in("C", "Total")), 2);
    assert!(b.run_in("C", "Other").is_err(), "the Partner app (index 2) has no row");
}
