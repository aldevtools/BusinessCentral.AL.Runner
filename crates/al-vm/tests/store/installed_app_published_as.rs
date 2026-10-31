//! "Published As" (column 16) of NAV App Installed App: Dev (2) for an app under test, Global (0) for a Microsoft dependency
//! (record/lt-y12-published-as/Y12PublishedAs.al:31-80, al-runner issue 6438), and
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

fn run(routine: &str) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let mut l = al_testkit::compile(SRC);
    std::sync::Arc::make_mut(&mut l.program).apps = vec![
        app("aa006438-a411-4309-b899-92353cc12b05", "Repro", false),
        app("63ca2fa4-4f03-4f2b-a480-172fef340d3f", "Microsoft", true),
        app("bb006438-a411-4309-b899-92353cc12b05", "Partner", true),
        app("8874ed3a-0643-4247-9ced-7a7002f7135d", "Microsoft", true),
    ];
    al_testkit::Built::new(l).run_in("C", routine)
}

#[test]
fn an_app_under_test_is_dev_and_a_microsoft_dependency_global() {
    assert_eq!(al_testkit::text(run("F")), "2|Yes|No|0");
}

#[test]
fn another_publishers_dependency_is_refused() {
    assert!(matches!(run("Other"), Err(al_builtins::native::Signal::Unsupported(_))));
}

/// The platform's System app (8874ed3a-...) has no NAV App Installed App row (Y12PublishedAs.al:41, :46 `no row`).
#[test]
fn the_platform_system_app_has_no_row() {
    assert_eq!(al_testkit::text(run("PlatformRow")), "No");
}
