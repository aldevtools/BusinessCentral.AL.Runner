//! What a page control shows and how TestPage compares it: AssertEquals on a Code control, ClosingDates in the
//! control text, and the caption from CaptionClass.

use al_builtins::native::Signal;
use al_testkit::text;

const CODE_SRC: &str = r#"
table 69990 "CT Row" { fields { field(1; K; Integer) { } field(2; Cde; Code[30]) { } } keys { key(PK; K) { } } }
page 69991 "CT Card"
{
    PageType = Card; SourceTable = "CT Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Cde; Rec.Cde) { } } }
}
codeunit 69992 C
{
    procedure Lower(): Text
    var R: Record "CT Row"; P: TestPage "CT Card";
    begin
        R.K := 1; R.Cde := 'ABC'; R.Insert();
        P.OpenEdit();
        P.Cde.AssertEquals('abc');
        exit('ok');
    end;
    procedure LowerMismatch(): Text
    var R: Record "CT Row"; P: TestPage "CT Card";
    begin
        R.K := 1; R.Cde := 'ABC'; R.Insert();
        P.OpenEdit();
        P.Cde.AssertEquals('abd');
        exit('ok');
    end;
}
"#;

#[test]
fn assert_equals_on_a_code_control_ignores_the_case_of_the_expected_text() {
    // corpus pages/lt-testpage-assertequals-option/TestPageAssertEqualsOption.al:167, :179.
    assert_eq!(text(al_testkit::run_in(CODE_SRC, "C", "Lower")), "ok");
}

#[test]
fn a_mismatch_on_a_code_control_shows_the_expected_text_as_written() {
    // corpus handlers/TestRequestPageControlErrorName.al:281, :397.
    assert_eq!(
        al_testkit::error_message(al_testkit::run_in(CODE_SRC, "C", "LowerMismatch")),
        "AssertEquals for Field: Cde Expected = 'abd', Actual = 'ABC'"
    );
}

const CLOSING_SRC: &str = r#"
table 69993 "CT Dates"
{
    fields { field(1; K; Integer) { } field(2; OnField; Date) { ClosingDates = true; } field(3; Plain; Date) { } }
    keys { key(PK; K) { } }
}
page 69994 "CT Dates Card"
{
    PageType = Card; SourceTable = "CT Dates"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(OnField; Rec.OnField) { }
            field(OnControl; Rec.Plain) { ClosingDates = true; }
            field(Plain; Rec.Plain) { }
            field(OffOverField; Rec.OnField) { ClosingDates = false; }
        }
    }
}
codeunit 69995 C
{
    procedure Shown(): Text
    var R: Record "CT Dates"; P: TestPage "CT Dates Card";
    begin
        R.K := 1; R.OnField := ClosingDate(DMY2Date(31, 12, 2025)); R.Plain := R.OnField; R.Insert();
        P.OpenEdit();
        exit(P.OnField.Value() + '|' + P.OnControl.Value() + '|' + P.Plain.Value());
    end;
    procedure OffOverField(): Text
    var R: Record "CT Dates"; P: TestPage "CT Dates Card";
    begin
        R.K := 1; R.OnField := ClosingDate(DMY2Date(31, 12, 2025)); R.Insert();
        P.OpenEdit();
        exit(P.OffOverField.Value());
    end;
}
"#;

#[test]
fn a_closing_date_shows_its_mark_where_the_field_or_the_control_sets_closing_dates() {
    // corpus record/lt-x-table-props/XTablePropsTests.al:523-524, pageext/lt-z-modify-controls/ZModifyControls.al:290-291,
    // pages/lt-page-presentation-properties/P119ReportFields.al:174.
    assert_eq!(
        text(al_testkit::run_in(CLOSING_SRC, "C", "Shown")),
        "C12/31/2025|C12/31/2025|12/31/2025"
    );
}

#[test]
fn a_control_that_turns_closing_dates_off_over_its_field_is_refused() {
    let r = al_testkit::run_in(CLOSING_SRC, "C", "OffOverField");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

const CAPTION_SRC: &str = r#"
table 69996 "CT Cap Row"
{
    fields
    {
        field(1; K; Integer) { }
        field(2; Descr; Text[30]) { }
        field(3; Classed; Integer) { Caption = 'Plain caption'; CaptionClass = '3,Class caption'; }
    }
    keys { key(PK; K) { } }
}
codeunit 69997 "CT Cap Log"
{
    SingleInstance = true;
    var Calls: Text;
    procedure Take(): Text var R: Text; begin R := Calls; Calls := ''; exit(R); end;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', true, true)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        if CaptionArea <> 'CT' then exit;
        Calls += 'resolve(' + CaptionExpr + ')';
        Caption := 'resolved:' + CaptionExpr;
        Resolved := true;
    end;
}
page 69998 "CT Cap Card"
{
    PageType = Card; SourceTable = "CT Cap Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(Claimed; Rec.Descr) { CaptionClass = 'CT,Hello'; }
            field(FromVariable; Extra) { CaptionClass = 'CT,' + Suffix; }
            field(SetSuffix; Suffix) { }
            field(Classed; Rec.Classed) { }
            field(CaptionOverClass; Rec.Classed) { Caption = 'Own'; }
        }
    }
    var Extra: Text; Suffix: Text;
}
codeunit 69999 C
{
    procedure ReadTwice(): Text
    var R: Record "CT Cap Row"; P: TestPage "CT Cap Card"; L: Codeunit "CT Cap Log"; Out: Text;
    begin
        R.K := 1; R.Insert();
        L.Take();
        P.OpenEdit();
        Out := L.Take() + '|';
        Out += P.Claimed.Caption() + '|' + L.Take() + '|' + P.Claimed.Caption() + '|' + L.Take();
        exit(Out);
    end;
    procedure FollowsTheVariable(): Text
    var R: Record "CT Cap Row"; P: TestPage "CT Cap Card"; L: Codeunit "CT Cap Log"; Out: Text;
    begin
        R.K := 1; R.Insert();
        P.OpenEdit();
        L.Take();
        Out := P.FromVariable.Caption() + '|' + L.Take() + '|';
        P.SetSuffix.SetValue('World');
        Out += L.Take() + '|' + P.FromVariable.Caption() + '|' + L.Take();
        exit(Out);
    end;
    procedure FieldClass(): Text
    var R: Record "CT Cap Row"; P: TestPage "CT Cap Card";
    begin
        R.K := 1; R.Insert();
        P.OpenEdit();
        exit(P.Classed.Caption());
    end;
    procedure CaptionOverClass(): Text
    var R: Record "CT Cap Row"; P: TestPage "CT Cap Card";
    begin
        R.K := 1; R.Insert();
        P.OpenEdit();
        exit(P.CaptionOverClass.Caption());
    end;
}
"#;

#[test]
fn a_control_caption_class_is_resolved_when_the_page_fills_and_read_from_there() {
    // corpus misc/W2CaptionClass.al:92 (at open), :113-116 (no event at either read).
    assert_eq!(
        text(al_testkit::run_in(CAPTION_SRC, "C", "ReadTwice")),
        "resolve(Hello)resolve()|resolved:Hello||resolved:Hello|"
    );
}

#[test]
fn a_caption_class_expression_is_resolved_again_only_when_its_text_changes() {
    // corpus misc/W2CaptionClass.al:174-178: after SetValue only the changed expression raises the event.
    assert_eq!(
        text(al_testkit::run_in(CAPTION_SRC, "C", "FollowsTheVariable")),
        "resolved:||resolve(World)|resolved:World|"
    );
}

#[test]
fn a_control_without_caption_shows_its_field_caption_class() {
    // corpus pageext/lt-page-ext-properties/TableExtCaptionClass.al:69.
    assert_eq!(
        text(al_testkit::run_in(CAPTION_SRC, "C", "FieldClass")),
        "Class caption"
    );
}

#[test]
fn a_control_caption_over_a_field_caption_class_is_refused() {
    let r = al_testkit::run_in(CAPTION_SRC, "C", "CaptionOverClass");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

const UNPINNED_SRC: &str = r#"
table 69960 "CT Un Row"
{
    fields { field(1; K; Integer) { } field(2; Ev; Integer) { CaptionClass = 'CT,Field'; } }
    keys { key(PK; K) { } }
}
codeunit 69961 "CT Un Log"
{
    SingleInstance = true;
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', true, true)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        if CaptionArea <> 'CT' then exit;
        Caption := 'resolved:' + CaptionExpr;
        Resolved := true;
    end;
}
page 69962 "CT Un List"
{
    PageType = List; SourceTable = "CT Un Row"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(R)
            {
                field(K; Rec.K) { }
                field(FromVariable; Rec.K) { CaptionClass = 'CT,' + Suffix; }
                field(Ev; Rec.Ev) { }
            }
        }
    }
    actions { area(Processing) { action(Change) { ApplicationArea = All; trigger OnAction() begin Suffix := 'Changed'; end; } } }
    var Suffix: Text;
}
codeunit 69963 C
{
    local procedure Seed()
    var R: Record "CT Un Row";
    begin
        R.K := 1; R.Insert(); R.K := 2; R.Insert();
    end;
    procedure SameTextAgain(): Text
    var P: TestPage "CT Un List";
    begin
        Seed();
        P.OpenView();
        P.Next();
        exit(P.FromVariable.Caption());
    end;
    procedure ChangedWithoutFill(): Text
    var P: TestPage "CT Un List";
    begin
        Seed();
        P.OpenView();
        P.Change.Invoke();
        exit(P.FromVariable.Caption());
    end;
    procedure FieldClassTwice(): Text
    var P: TestPage "CT Un List";
    begin
        Seed();
        P.OpenView();
        exit(P.Ev.Caption() + '|' + P.Ev.Caption());
    end;
}
"#;

fn refused(routine: &str) {
    let r = al_testkit::run_in(UNPINNED_SRC, "C", routine);
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
}

#[test]
fn a_caption_class_expression_that_gives_the_same_text_at_a_later_fill_keeps_its_caption() {
    // The session keeps what it translated (types/PSCaptionClass.al:94-115): the next row's fill raises no event again.
    assert_eq!(
        text(al_testkit::run_in(UNPINNED_SRC, "C", "SameTextAgain")),
        "resolved:"
    );
}

#[test]
fn a_caption_class_expression_read_after_it_changed_without_a_fill_is_refused() {
    refused("ChangedWithoutFill");
}

#[test]
fn a_second_read_of_a_field_caption_class_that_raises_the_event_is_refused() {
    // XTablePropsTests.al:483-487 pins one read only.
    refused("FieldClassTwice");
}

/// A page field bound to a bare field name of the source table (no `Rec.`, warning AL0604) is that field: its caption is the
/// field's (pages/lt-page-controls-aa/AaControls.al:684-692, `SystemId` gives 'System ID'), its value the record's, and
/// SystemId is not editable.
#[test]
fn a_control_bound_to_a_bare_field_name_is_the_field() {
    let src = r#"
table 69990 "CT Row" { fields { field(1; K; Integer) { } field(2; Cde; Code[30]) { Caption = 'Code Caption'; } } keys { key(PK; K) { } } }
page 69991 "CT Card" { PageType = Card; SourceTable = "CT Row";
  layout { area(Content) { field(SystemId; SystemId) { } field(Cde; Cde) { } } } }
codeunit 69992 C {
  procedure F(): Text var R: Record "CT Row"; P: TestPage "CT Card"; T: Text; begin
    R.K := 1; R.Cde := 'A1'; R.Insert(); P.OpenView();
    T := P.SystemId.Caption() + '|' + Format(P.SystemId.Editable()) + '|' + P.Cde.Caption() + '|' + P.Cde.Value();
    T += '|' + Format(P.SystemId.Value() = LowerCase(Format(R.SystemId)));
    P.Close(); exit(T); end; }"#;
    assert_eq!(
        text(al_testkit::run_in(src, "C", "F")),
        "System ID|No|Code Caption|A1|Yes"
    );
}

/// The same with a quoted field name that holds a space (`field(Ctl; "Profile Name")`, al-runner issue 6314): the quotes
/// only spell the name, so the control is the field, on a page and on a control a page extension adds.
#[test]
fn a_control_bound_to_a_bare_quoted_field_name_is_the_field() {
    let src = r#"
table 69990 "CT Row" { fields { field(1; K; Integer) { } field(2; "Profile Name"; Code[30]) { Caption = 'Profile Caption'; } } keys { key(PK; K) { } } }
page 69991 "CT Card" { PageType = Card; SourceTable = "CT Row";
  layout { area(Content) { group(G) { field(Prof; "Profile Name") { Editable = Ed; } } } }
  var Ed: Boolean; }
pageextension 69993 "CT Card Ext" extends "CT Card" {
  layout { addlast(G) { field(ExtProf; "Profile Name") { } } } }
codeunit 69992 C {
  procedure F(): Text var R: Record "CT Row"; P: TestPage "CT Card"; T: Text; begin
    R.K := 1; R."Profile Name" := 'A1'; R.Insert(); P.OpenEdit();
    T := P.Prof.Caption() + '|' + P.Prof.Value() + '|' + Format(P.Prof.Editable()) + '|' + P.ExtProf.Caption() + '|' + P.ExtProf.Value();
    P.Close(); exit(T); end; }"#;
    assert_eq!(
        text(al_testkit::run_in(src, "C", "F")),
        "Profile Caption|A1|No|Profile Caption|A1"
    );
}

/// A bare name that is also a procedure of the page, or of the source table, or a page variable, could mean either; which one
/// BC binds is not pinned: the page is refused.
#[test]
fn a_bare_control_source_that_names_two_things_is_refused() {
    let shapes = [
        ("", "procedure Cde(): Code[30] begin exit('P'); end;", ""),
        ("procedure Cde(): Code[30] begin exit('T'); end;", "", ""),
        ("", "", "var Cde: Code[30];"),
    ];
    for (table_proc, page_proc, page_var) in shapes {
        let src = format!(
            r#"
table 69990 "CT Row" {{ fields {{ field(1; K; Integer) {{ }} field(2; Cde; Code[30]) {{ }} }} keys {{ key(PK; K) {{ }} }} {table_proc} }}
page 69991 "CT Card" {{ PageType = Card; SourceTable = "CT Row";
  layout {{ area(Content) {{ field(CdeCtl; Cde) {{ }} }} }} {page_var} {page_proc} }}
codeunit 69992 C {{ procedure F(): Text var P: TestPage "CT Card"; begin P.OpenView(); exit(P.CdeCtl.Value()); end; }}"#
        );
        match al_testkit::run_in(&src, "C", "F") {
            Err(Signal::Unsupported(why)) => assert!(why.contains("not pinned"), "{why}"),
            other => panic!("expected a refusal for {table_proc}{page_proc}{page_var}, got {other:?}"),
        }
    }
}

/// The session keeps what it translated (types/PSCaptionClass.al:94-115): a page fill after SetValue whose CaptionClass expression
/// gives the text it gave before raises no OnResolveCaptionClass and keeps the caption (pageext/lt-z-modify-controls/
/// ZModifyControls.al:424-440 runs through it).
#[test]
fn a_caption_class_that_gives_the_same_text_at_a_later_fill_is_kept() {
    let src = r#"
table 69990 "CT Row" { fields { field(1; K; Integer) { } field(2; Cde; Code[30]) { } } keys { key(PK; K) { } } }
page 69991 "CT Card" { PageType = Card; SourceTable = "CT Row";
  layout { area(Content) { field(CdeCtl; Rec.Cde) { CaptionClass = Cc(); } } }
  local procedure Cc(): Text begin exit('PS,same'); end; }
codeunit 69993 Sub { SingleInstance = true; var Log: Text;
  [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', false, false)]
  local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
  begin Log += 'r'; Caption := 'resolved:' + CaptionExpr; Resolved := true; end;
  procedure Take(): Text var T: Text; begin T := Log; Log := ''; exit(T); end; }
codeunit 42 "Caption Class" { [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }
codeunit 69992 C {
  procedure F(): Text var R: Record "CT Row"; P: TestPage "CT Card"; S: Codeunit Sub; T: Text; begin
    R.K := 1; R.Insert(); S.Take(); P.OpenEdit(); T := S.Take() + '|';
    P.CdeCtl.SetValue('X'); T += S.Take() + '|' + P.CdeCtl.Caption(); P.Close(); exit(T); end; }"#;
    assert_eq!(text(al_testkit::run_in(src, "C", "F")), "r||resolved:same");
}
