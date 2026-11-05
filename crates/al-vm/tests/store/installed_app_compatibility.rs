//! Compatibility Major to Revision (columns 9 to 12), Hash Algorithm (14) and Extension Type (15) of NAV App Installed App
//! (al-runner issue 7142): 0, 0, 0, 0, 1 and 2 for an app under test and for a Microsoft dependency (corpus branch
//! al-runtime-rs-staging-r7142, R7142InstalledAppCompat.al); Content Hash (13) stays refused. Against a fixture table with
//! the platform's id and field ids.

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
        field(9; "Compatibility Major"; Integer) { }
        field(10; "Compatibility Minor"; Integer) { }
        field(11; "Compatibility Build"; Integer) { }
        field(12; "Compatibility Revision"; Integer) { }
        field(13; "Content Hash"; Text[250]) { }
        field(14; "Hash Algorithm"; Integer) { }
        field(15; "Extension Type"; Integer) { }
        field(16; "Published As"; Option) { OptionMembers = Global,PTE,Dev; }
    }
    keys { key(PK; "App ID") { } }
}
codeunit 50100 C
{
    // The reporter's install code builds the version text this way.
    procedure Text(PublisherName: Text): Text
    var
        R: Record "NAV App Installed App";
    begin
        R.SetRange(Publisher, PublisherName);
        R.FindFirst();
        exit(StrSubstNo('%1.%2.%3.%4|%5.%6|%7|%8', R."Version Major", R."Version Minor", R."Compatibility Build",
            R."Compatibility Revision", R."Compatibility Major", R."Compatibility Minor", R."Hash Algorithm", R."Extension Type"));
    end;

    procedure ContentHash(): Text
    var
        R: Record "NAV App Installed App";
    begin
        R.FindFirst();
        exit(R."Content Hash");
    end;
}
"#;

fn run(
    routine: &str,
    args: Vec<al_builtins::value::Value>,
) -> Result<al_builtins::value::Value, al_builtins::native::Signal> {
    let mut l = al_testkit::compile(SRC);
    let app = |id: &str, publisher: &str, version: &str, dependency: bool| al_ir::AppInfo {
        id: id.into(),
        name: format!("{publisher} app"),
        publisher: publisher.into(),
        version: version.into(),
        dependency,
        ..Default::default()
    };
    std::sync::Arc::make_mut(&mut l.program).apps = vec![
        app("aa007142-a411-4309-b899-92353cc12b05", "Repro", "2.3.4.5", false),
        app(
            "63ca2fa4-4f03-4f2b-a480-172fef340d3f",
            "Microsoft",
            "27.5.46862.48827",
            true,
        ),
    ];
    let mut b = al_testkit::Built::new(l);
    let (oi, ri) = b.routine_in("C", routine);
    b.call(oi, ri, args).map(|(v, _)| v)
}

#[test]
fn the_compatibility_columns_are_zero_and_hash_algorithm_and_extension_type_are_measured() {
    let text = |p: &str| al_testkit::text(run("Text", vec![al_builtins::value::Value::Text(p.into())]));
    assert_eq!(text("Repro"), "2.3.0.0|0.0|1|2");
    assert_eq!(text("Microsoft"), "27.5.0.0|0.0|1|2");
}

#[test]
fn the_content_hash_stays_refused() {
    assert!(matches!(
        run("ContentHash", vec![]),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}
