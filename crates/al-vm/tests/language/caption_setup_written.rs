//! CaptionClass areas 101 and 1 after the test wrote its own General Ledger Setup and Dimension rows
//! (record/captions/QdCaptionClassAreas.al:244-270 Literals_FieldCaption and :279-312 Expressions_FieldCaption, with
//! record/QkSetupRows.al WriteGLSetupCaptions: LCY Code 'GBP', Global Dimension 1 Code 'DEPARTMENT', Code Caption
//! 'Department Code'). Without test data the runtime has no demo setup rows, so a caption these areas resolve is refused unless the
//! test wrote the rows the resolver reads before any caption of those areas was resolved in the session. The subscriber stands in for
//! Base Application's codeunits 342 and 343 (SingleInstance, they keep the setup they read first).

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const OBJECTS: &str = r#"
codeunit 42 "Caption Class" { [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }
table 98 "General Ledger Setup"
{
    fields { field(1; "Primary Key"; Code[10]) { } field(71; "LCY Code"; Code[10]) { } field(79; "Global Dimension 1 Code"; Code[20]) { } }
    keys { key(PK; "Primary Key") { } }
}
table 348 Dimension
{
    fields { field(1; Code; Code[20]) { } field(3; "Code Caption"; Text[80]) { } }
    keys { key(PK; Code) { } }
}
codeunit 50411 Resolver
{
    SingleInstance = true;
    var GLSetup: Record "General Ledger Setup"; Read: Boolean;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', false, false)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    var Dim: Record Dimension;
    begin
        // The runtime's install code writes a General Ledger Setup row; a missing row reads as the blank one here.
        if not Read then begin
            if not GLSetup.Get() then
                Clear(GLSetup);
            Read := true;
        end;
        if CaptionArea = '101' then begin
            Caption := StrSubstNo(CopyStr(CaptionExpr, StrPos(CaptionExpr, ',') + 1), GLSetup."LCY Code");
            Resolved := true;
        end;
        if (CaptionArea = '1') and Dim.Get(GLSetup."Global Dimension 1 Code") then begin
            Caption := Dim."Code Caption";
            Resolved := true;
        end;
    end;
}
table 50410 CapRow
{
    fields
    {
        field(1; Id; Integer) { }
        field(14; "Lit 101-0"; Decimal) { Caption = 'Cap Lit 101-0'; CaptionClass = '101,0,Amount (%1)'; }
        field(15; "Lit 1-1"; Code[20]) { Caption = 'Cap Lit 1-1'; CaptionClass = '1,1,1'; }
        field(16; "Lit 101-2"; Decimal) { Caption = 'Cap Lit 101-2'; CaptionClass = '101,2,Amount (%1)'; }
    }
    keys { key(PK; Id) { } }
}
codeunit 50412 Setup
{
    procedure WriteGLSetup()
    var G: Record "General Ledger Setup";
    begin
        if not G.Get() then begin
            G.Init();
            G.Insert();
        end;
        G."LCY Code" := 'GBP';
        G."Global Dimension 1 Code" := 'DEPARTMENT';
        G.Modify();
    end;

    procedure WriteDimension()
    var D: Record Dimension;
    begin
        D.Init();
        D.Code := 'DEPARTMENT';
        D."Code Caption" := 'Department Code';
        D.Insert();
    end;
}
"#;

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("{OBJECTS}\ncodeunit 50419 T\n{{\n{body}\n}}\n");
    al_testkit::run_in(&src, "T", "F")
}

fn refused(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Unsupported(m)) => m.to_string(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn area_101_type_0_after_the_test_wrote_general_ledger_setup_is_resolved() {
    let got = run(r#"
    procedure F(): Text
    var R: Record CapRow; S: Codeunit Setup;
    begin
        S.WriteGLSetup();
        exit(R.FieldCaption("Lit 101-0"));
    end;"#);
    assert_eq!(text(got), "Amount (GBP)");
}

#[test]
fn area_1_after_the_test_wrote_general_ledger_setup_and_the_dimension_is_resolved() {
    let got = run(r#"
    procedure F(): Text
    var R: Record CapRow; S: Codeunit Setup;
    begin
        S.WriteGLSetup();
        S.WriteDimension();
        exit(R.FieldCaption("Lit 101-0") + '|' + R.FieldCaption("Lit 1-1"));
    end;"#);
    assert_eq!(text(got), "Amount (GBP)|Department Code");
}

#[test]
fn area_101_without_the_test_writing_the_setup_stays_refused() {
    let m = refused(run(r#"
    procedure F(): Text
    var R: Record CapRow; G: Record "General Ledger Setup";
    begin
        G."LCY Code" := 'GBP';
        exit(R.FieldCaption("Lit 101-0"));
    end;"#));
    assert!(m.contains("corpus database row"), "{m}");
}

/// Base Application's resolvers keep the setup row they read first in the session: a caption of these areas resolved before the test
/// wrote the rows leaves the install-code row in them, so later captions stay refused.
#[test]
fn a_caption_resolved_before_the_setup_was_written_keeps_later_captions_refused() {
    let m = refused(run(r#"
    procedure F(): Text
    var R: Record CapRow; S: Codeunit Setup; T: Text;
    begin
        if not TryCaption(T) then;
        S.WriteGLSetup();
        exit(R.FieldCaption("Lit 101-0"));
    end;
    [TryFunction]
    local procedure TryCaption(var T: Text)
    var R: Record CapRow;
    begin
        T := R.FieldCaption("Lit 101-0");
    end;"#));
    assert!(m.contains("corpus database row"), "{m}");
}

/// Area 101 types other than 0 read fields QkSetupRows.al does not write (Additional Reporting Currency, Currency.Description).
#[test]
fn area_101_type_2_after_the_setup_was_written_stays_refused() {
    let m = refused(run(r#"
    procedure F(): Text
    var R: Record CapRow; S: Codeunit Setup;
    begin
        S.WriteGLSetup();
        exit(R.FieldCaption("Lit 101-2"));
    end;"#));
    assert!(m.contains("corpus database row"), "{m}");
}
