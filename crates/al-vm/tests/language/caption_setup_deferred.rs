//! A page open resolves a control's CaptionClass of area 1 or 101 even when the text depends on General Ledger Setup, which the
//! runtime's database does not hold as BC's. What is unknown is the caption text, so the refusal belongs where the text is read
//! (the control's Caption()), not at the open (runner issue 6794). Base Application's resolvers (codeunits 342 and 343) only assign
//! Caption and Resolved, so they cannot change state the test sees; a resolver of another app could, and keeps the open refused.
//! The Base Application stand-in here has Base Application's app id; the codeunit shape follows `caption_setup_written.rs`.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{AppSource, text};

const BASE_ID: &str = "437dbf0e-84ff-417a-965d-ed2bb9650972";

const BASE: &str = r#"
codeunit 42 "Caption Class" { [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }
table 98 "General Ledger Setup"
{
    fields { field(1; "Primary Key"; Code[10]) { } field(79; "Global Dimension 1 Code"; Code[20]) { } }
    keys { key(PK; "Primary Key") { } }
}
table 348 Dimension
{
    fields { field(1; Code; Code[20]) { } field(3; "Code Caption"; Text[80]) { } }
    keys { key(PK; Code) { } }
}
codeunit 50411 Resolver
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', false, false)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    var GLSetup: Record "General Ledger Setup"; Dim: Record Dimension;
    begin
        if not GLSetup.Get() then
            Clear(GLSetup);
        if CaptionArea = '1' then begin
            Caption := 'Dimension ' + GLSetup."Global Dimension 1 Code";
            if Dim.Get(GLSetup."Global Dimension 1 Code") then
                Caption := Dim."Code Caption";
            Resolved := true;
        end;
    end;
}
table 50410 CapRow
{
    fields { field(1; Id; Integer) { } field(2; Dim; Code[20]) { } }
    keys { key(PK; Id) { } }
}
page 50413 CapCard
{
    PageType = Card;
    SourceTable = CapRow;
    layout { area(Content) { field(Id; Rec.Id) { } field(Dim; Rec.Dim) { CaptionClass = '1,1,1'; } } }
}
codeunit 50419 T
{
    procedure Opens(): Text
    var P: TestPage CapCard;
    begin
        P.OpenView();
        if not P.Id.Visible() then
            Error('Id not visible');
        P.Close();
        exit('opened');
    end;

    procedure ReadsCaption(): Text
    var P: TestPage CapCard; S: Text;
    begin
        P.OpenView();
        S := P.Dim.Caption();
        P.Close();
        exit(S);
    end;
}
"#;

const OTHER_APP: &str = r#"
codeunit 60001 OtherResolver
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', false, false)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    var G: Record "General Ledger Setup";
    begin
        if G.Get() then
            G.Modify();
    end;
}
"#;

fn app(id: &str, name: &str, deps: Vec<String>, src: &str) -> AppSource {
    AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: deps,
        files: vec![("f.al".into(), src.into())],
    }
}

fn run(extra: Option<&str>, routine: &str) -> Result<Value, Signal> {
    let mut apps = vec![app(BASE_ID, "Base Application", vec![], BASE)];
    if let Some(src) = extra {
        apps.push(app("other", "Other", vec![BASE_ID.into()], src));
    }
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps));
    b.run_in("T", routine)
}

fn refused(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(m)) => m.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_open_runs_when_only_base_application_resolves_the_caption() {
    assert_eq!(text(run(None, "Opens")), "opened");
}

#[test]
fn reading_the_unresolved_caption_is_refused() {
    let m = refused(run(None, "ReadsCaption"));
    assert!(m.contains("corpus database row"), "{m}");
}

#[test]
fn a_resolver_of_another_app_keeps_the_open_refused() {
    let m = refused(run(Some(OTHER_APP), "Opens"));
    assert!(m.contains("corpus database row"), "{m}");
}
