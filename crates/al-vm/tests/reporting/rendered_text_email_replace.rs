//! The rgba replace of `Email Message Impl.` on rendered content (ADR-040 amendment 2026-11-12, P12; al-runner issue 6924):
//! which renders `rendered_text::DOTNET_ALLOWED` takes, from the rgba column of the save's row (al-vm `report_save`). The
//! stand-ins below make the calls `Email Message Impl.` makes: `ReplaceRgbaColorsWithRgb` (a `Regex.Replace` on a `DotNet`
//! variable that holds no object) and `SanitizeBody`. No .NET host runs here, so a call the guard lets through stops at the
//! host; the answer of the real `Regex.Replace` is checked in al-test-cli `suite/dotnet_sidecar.rs`.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::rendered_text::{self, RgbaReplace};
use al_builtins::value::Value;
use al_testkit::Built;

use super::rendered_text::FIXTURE;

const REPLACE: &str = r#"
dotnet
{
    assembly("System.Text.RegularExpressions") { type("System.Text.RegularExpressions.Regex"; "Regex") { } }
    assembly("Microsoft.Dynamics.Nav.Ncl") { type("Microsoft.Dynamics.Nav.Runtime.AppHtmlSanitizer"; "AppHtmlSanitizer") { } }
}

// A stand-in of the platform's Tenant Report Layout Selection (Base Application's objects: `report_management_stand_in`).
table 2000000233 "Tenant Report Layout Selection"
{
    fields { field(1; "Report ID"; Integer) { } field(2; "Company Name"; Text[30]) { } field(3; "Layout Name"; Text[250]) { } }
    keys { key(PK; "Report ID", "Company Name") { } }
}

report 50981 "RE Text"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } column(Short; Short) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'ReText.docx'; } }
}

report 50982 "RE Label"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'ReLabel.docx'; } }
    labels { Tint = 'rgba(1,2,3,1)'; }
}

report 50983 "RE Legacy"
{
    UsageCategory = None;
    DefaultLayout = Word;
    WordLayout = 'ReLegacy.docx';
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
}

report 50985 "RE Two"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } dataitem(U; "RT Row") { column(Id2; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'ReTwo.docx'; } }
}

report 50986 "RE Extended"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'ReExtended.docx'; } }
}

reportextension 50987 "RE Ext" extends "RE Extended"
{
    dataset { add(T) { column(Short; Short) { } } }
}

report 50988 "RE Nested"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset
    {
        dataitem(T; "RT Row")
        {
            column(Id; Id) { }
            dataitem(U; "RT Row") { DataItemLink = Id = field(Id); column(Id2; Id) { } }
        }
    }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'ReNested.docx'; } }
}

report 50989 "RE Ext Trigger Base"
{
    UsageCategory = None;
    DefaultRenderingLayout = WordOne;
    dataset { dataitem(T; "RT Row") { column(Id; Id) { } } }
    rendering { layout(WordOne) { Type = Word; LayoutFile = 'ReExtTrigger.docx'; } }
}

reportextension 50990 "RE Ext Trigger" extends "RE Ext Trigger Base"
{
    dataset { add(T) { column(Short; Short) { } } }
    trigger OnPreReport() begin end;
}

report 50996 "RE Legacy Nested"
{
    UsageCategory = None;
    DefaultLayout = Word;
    WordLayout = 'ReLegacyNested.docx';
    dataset
    {
        dataitem(T; "RT Row")
        {
            column(Id; Id) { }
            dataitem(U; "RT Row") { DataItemLink = Id = field(Id); column(Id2; Id) { } }
        }
    }
}

codeunit 50984 "RE Tests"
{
    var
        PatternTok: Label 'rgba\((\d{1,3}),\s*(\d{1,3}),\s*(\d{1,3}),\s*1(\.0{0,2})?\)', Locked = true;
        ReplacementTok: Label 'rgb($1, $2, $3)', Locked = true;

    local procedure Body(ReportId: Integer; Short: Text[30]): Text
    var R: Record "RT Row"; Tmp: Record "RT Row" temporary; O: OutStream; I: InStream; T: Text;
    begin
        R.DeleteAll();
        R.Id := 1; R.Short := Short; R.Insert();
        Tmp.Data.CreateOutStream(O);
        Report.SaveAs(ReportId, '', ReportFormat::Html, O);
        Tmp.Data.CreateInStream(I);
        I.Read(T);
        exit(T);
    end;

    // Email Message Impl. ReplaceRgbaColorsWithRgb.
    local procedure ReplaceRgba(var Body: Text)
    var RgbaRegexPattern: DotNet Regex;
    begin
        Body := RgbaRegexPattern.Replace(Body, PatternTok, ReplacementTok);
    end;

    procedure PlainBody(): Text begin exit(Body(Report::"RE Text", 'plain')); end;
    procedure RgbaBody(): Text begin exit(Body(Report::"RE Text", 'x rgba(1,2,3,1)')); end;
    procedure LabelBody(): Text begin exit(Body(Report::"RE Label", '')); end;
    procedure LegacyBody(): Text begin exit(Body(Report::"RE Legacy", '')); end;
    procedure TwoItemsBody(): Text begin exit(Body(Report::"RE Two", '')); end;
    procedure ExtendedBody(): Text begin exit(Body(Report::"RE Extended", 'plain')); end;
    procedure NestedBody(): Text begin exit(Body(Report::"RE Nested", '')); end;
    procedure ExtTriggerBody(): Text begin exit(Body(Report::"RE Ext Trigger Base", 'plain')); end;
    procedure LegacyNestedBody(): Text begin exit(Body(Report::"RE Legacy Nested", '')); end;
    procedure ReplaceNested(): Text var T: Text; begin T := NestedBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    procedure ReplaceExtTrigger(): Text var T: Text; begin T := ExtTriggerBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    procedure ReplaceLegacyNested(): Text var T: Text; begin T := LegacyNestedBody(); ReplaceRgba(T); exit(Format(T <> '')); end;

    // A stored selection row that names the report's own Word layout: the save writes the placeholder, as before.
    procedure TenantRowBody(): Text
    var S: Record "Tenant Report Layout Selection";
    begin
        S."Report ID" := Report::"RE Text"; S."Layout Name" := 'WordOne'; S.Insert();
        exit(PlainBody());
    end;
    procedure BuiltinRowBody(): Text
    var S: Record "Report Layout Selection";
    begin
        S."Report ID" := Report::"RE Text"; S."Company Name" := CompanyName(); S.Type := S.Type::"Word (built-in)"; S.Insert();
        exit(PlainBody());
    end;
    procedure ReplaceTenantRow(): Text var T: Text; begin T := TenantRowBody(); ReplaceRgba(T); exit(Format(T <> '')); end;

    procedure ReplacePlain(): Text var T: Text; begin T := PlainBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    procedure ReplaceLegacy(): Text var T: Text; begin T := LegacyBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    procedure ReplaceTwoItems(): Text var T: Text; begin T := TwoItemsBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    procedure ReplaceExtended(): Text var T: Text; begin T := ExtendedBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    // A body that joins two renders of the measured shape (a signature report after the body report).
    procedure ReplaceTwoRenders(): Text var T: Text; begin T := PlainBody() + PlainBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    procedure ReplaceRgbaData(): Text var T: Text; begin T := RgbaBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    procedure ReplaceRgbaLabel(): Text var T: Text; begin T := LabelBody(); ReplaceRgba(T); exit(Format(T <> '')); end;
    procedure ReplaceOtherPattern(): Text
    var R: DotNet Regex; T: Text;
    begin
        T := PlainBody();
        exit(R.Replace(T, 'rgba', ReplacementTok));
    end;
    // Email Message Impl. SanitizeBody, without the constructor (no .NET host here): the call is refused for its argument first.
    procedure Sanitize(): Text
    var AppHTMLSanitizer: DotNet AppHtmlSanitizer;
    begin
        exit(AppHTMLSanitizer.SanitizeEmail(PlainBody()));
    end;
}
"#;

fn run(proc: &str) -> Result<Value, Signal> {
    super::report_management_stand_in::built(&format!("{FIXTURE}{REPLACE}")).run_in("RE Tests", proc)
}

#[track_caller]
fn facts(proc: &str) -> RgbaReplace {
    let body = match run(proc) {
        Ok(Value::Text(t)) => t.as_str().to_string(),
        other => panic!("{proc}: expected a text, got {other:?}"),
    };
    let marks = rendered_text::marks(&body).expect("whole marks");
    assert_eq!(marks.len(), 1, "{proc}");
    rendered_text::render(marks[0].0).expect("registered").rgba_replace
}

/// The reason a render is not measured, or a panic.
#[track_caller]
fn not_measured(proc: &str) -> String {
    match facts(proc) {
        RgbaReplace::NotMeasured(why) => why,
        RgbaReplace::Measured => panic!("{proc}: measured"),
    }
}

#[track_caller]
fn refusal(proc: &str) -> String {
    match run(proc) {
        Err(Signal::Unsupported(why)) => why.to_string(),
        other => panic!("{proc}: expected a refusal, got {other:?}"),
    }
}

const USE: &str = "a use of the rendered content of report";

#[test]
fn rendered_text_email_replace_the_save_keeps_the_rgba_column_of_its_row() {
    // The measured base and the forms with one fact changed (RxTenantLayoutBodyTests.al:319, :346, :373).
    for proc in ["PlainBody", "LegacyBody", "NestedBody", "ExtendedBody"] {
        assert_eq!(facts(proc), RgbaReplace::Measured, "{proc}");
    }
    for (proc, want) in [
        ("RgbaBody", "dataset holds rgba( yes (row: no)"),
        ("LabelBody", "dataset holds rgba( yes (row: no)"),
        // Two items side by side and an extension with a trigger are no measured form.
        ("TwoItemsBody", "data items another tree (row: one)"),
        ("ExtTriggerBody", "report extensions another set (row: none)"),
    ] {
        let why = not_measured(proc);
        assert!(why.contains(want), "{proc}: {why}");
    }
}

/// The forms measured with one fact changed pass the guard (corpus pull request 1371,
/// reports/html-email-body/RxTenantLayoutBodyTests.al:319, :346, :373): the call stops at the .NET host this test has none
/// of, not at the rendered content, and marks the assumption.
#[test]
fn rendered_text_email_replace_takes_each_form_measured_alone() {
    for proc in ["ReplaceLegacy", "ReplaceNested", "ReplaceExtended"] {
        assumptions::clear_hits();
        let why = refusal(proc);
        assert!(!why.contains(USE), "{proc}: {why}");
        assert!(
            assumptions::take_hits().contains(&Assumption::EmailBodyRgbaReplaceKeepsContent),
            "{proc}"
        );
    }
}

/// One refusal reason per fact no cell measured, and one for two measured forms together; none marks the assumption.
#[test]
fn rendered_text_email_replace_refuses_each_unmeasured_report_shape() {
    assumptions::clear_hits();
    for (proc, want) in [
        ("ReplaceTwoItems", "data items another tree (row: one)"),
        ("ReplaceExtTrigger", "report extensions another set (row: none)"),
        // Two forms measured one at a time.
        ("ReplaceLegacyNested", "the nearest measured renders differ in: "),
        ("ReplaceTwoRenders", "the body joins 2 renders"),
    ] {
        let why = refusal(proc);
        assert!(why.contains(USE) && why.contains(want), "{proc}: {why}");
    }
    assert!(!assumptions::take_hits().contains(&Assumption::EmailBodyRgbaReplaceKeepsContent));
}

/// An Html saved while a stored selection row names the report, even its own Word layout, is not the measured render: which
/// layout BC renders behind a row is not measured. The refusal names the row.
#[test]
fn rendered_text_email_replace_a_render_behind_a_selection_row_is_not_measured() {
    for proc in ["TenantRowBody", "BuiltinRowBody"] {
        let why = not_measured(proc);
        assert!(
            why.contains("a selection row names the report yes (row: no)"),
            "{proc}: {why}"
        );
    }
    let why = refusal("ReplaceTenantRow");
    assert!(
        why.contains(USE) && why.contains("a selection row names the report yes (row: no)"),
        "{why}"
    );
}

#[test]
fn rendered_text_email_replace_takes_the_default_word_layout_and_marks_its_assumption() {
    assumptions::clear_hits();
    // The guard lets the call through: it stops at the .NET host this test has none of, not at the rendered content.
    let why = refusal("ReplacePlain");
    assert!(!why.contains(USE), "{why}");
    assert!(
        assumptions::take_hits().contains(&Assumption::EmailBodyRgbaReplaceKeepsContent),
        "{why}"
    );
}

#[test]
fn rendered_text_email_replace_refuses_a_dataset_that_holds_rgba() {
    assumptions::clear_hits();
    for proc in ["ReplaceRgbaData", "ReplaceRgbaLabel"] {
        let why = refusal(proc);
        assert!(
            why.contains(USE) && why.contains("dataset holds rgba( yes (row: no)"),
            "{proc}: {why}"
        );
    }
    assert!(!assumptions::take_hits().contains(&Assumption::EmailBodyRgbaReplaceKeepsContent));
}

#[test]
fn rendered_text_email_replace_another_pattern_and_the_sanitizer_stay_refused() {
    let why = refusal("ReplaceOtherPattern");
    assert!(
        why.contains(USE) && why.contains("System.Text.RegularExpressions.Regex.Replace given the value"),
        "{why}"
    );
    let why = refusal("Sanitize");
    assert!(
        why.contains(USE) && why.contains("SanitizeEmail given the value"),
        "{why}"
    );
}
