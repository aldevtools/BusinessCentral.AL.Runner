//! Caption Class.OnAfterCaptionClassResolve: raised after every caption class translation, with the caption the resolvers gave,
//! and its subscribers may change that caption (types/caption-class-events/Y71CaptionClassEvents.al; al-runner issue 6924).
//! The stub codeunit 42 stands in for the System Application's Caption Class codeunit.

use al_testkit::text;

const OBJECTS: &str = r#"
codeunit 42 "Caption Class"
{
    [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end;
    [IntegrationEvent(false, false)] procedure OnAfterCaptionClassResolve(Language: Integer; CaptionExpression: Text; var Caption: Text[1024]) begin end;
}
codeunit 50451 Log
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(L: Text) begin if Lines <> '' then Lines += ' | '; Lines += L; end;
    procedure Take(): Text var T: Text; begin T := Lines; Lines := ''; exit(T); end;
}
codeunit 50452 Subs
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnResolveCaptionClass', '', false, false)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    var L: Codeunit Log;
    begin
        if CaptionArea <> 'LTY71' then exit;
        L.Add('resolve area=' + CaptionArea + ' expr=' + CaptionExpr);
        Caption := 'res';
        Resolved := true;
    end;

    // The shape of al-runner issue 6924: parameters in another order than the publisher's.
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Caption Class", 'OnAfterCaptionClassResolve', '', false, false)]
    local procedure OnAfter(CaptionExpression: Text; Language: Integer; var Caption: Text[1024])
    var L: Codeunit Log; Parts: List of [Text];
    begin
        L.Add('after expr=' + CaptionExpression + ' caption=' + Caption + ' session=' + Format(Language = GlobalLanguage()));
        Parts := CaptionExpression.Split(',');
        if Parts.Get(1) = 'LTWC' then
            Caption := 'WC table ' + Parts.Get(2) + ' field ' + Parts.Get(3);
    end;
}
table 50450 WcBase
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys { key(PK; Id) { } }
}
tableextension 50453 WcExt extends WcBase
{
    fields
    {
        field(50450; "WC Ext FC"; Text[100]) { Caption = 'WC Ext FC Cap'; CaptionClass = 'LTWC,27,1'; }
        field(50451; "WC No Comma"; Text[100]) { Caption = 'WC No Comma Cap'; CaptionClass = 'WCNOCOMMA'; }
        field(50452; "WC Empty Area"; Text[100]) { Caption = 'WC Empty Area Cap'; CaptionClass = ',WC empty area'; }
        field(50453; "WC Three Alone"; Text[100]) { Caption = 'WC Three Alone Cap'; CaptionClass = '3,'; }
    }
}
page 50454 WcCard
{
    PageType = Card;
    SourceTable = WcBase;
    layout { area(Content) { field(CtlThree; Rec.Id) { CaptionClass = '3,WC ctl three'; } } }
}
"#;

fn run(body: &str) -> String {
    let src = format!("{OBJECTS}\ncodeunit 50459 T\n{{\n{body}\n}}\n");
    text(al_testkit::run_in(&src, "T", "F"))
}

/// Y71CaptionClassEvents.al:235-246 CaptionClassTranslate_Statement_AfterLanguageChange: a claimed area raises OnResolveCaptionClass,
/// then OnAfterCaptionClassResolve with the resolved caption (`x1_1033|log`); area 3 raises only OnAfterCaptionClassResolve
/// (`area3_x3|log`); a second translation of the same text raises nothing (`x1_1030_again|log`, `area3_x3_again|log`).
#[test]
fn caption_class_translate_raises_on_after_resolve_once_per_text() {
    let got = run(r#"
    procedure F(): Text
    var L: Codeunit Log; S: Text;
    begin
        S := CaptionClassTranslate('LTY71,x1') + ' / ' + L.Take() + ' || ';
        S += CaptionClassTranslate('LTY71,x1') + ' / ' + L.Take() + ' || ';
        S += CaptionClassTranslate('3,Lit x3') + ' / ' + L.Take() + ' || ';
        S += CaptionClassTranslate('3,Lit x3') + ' / ' + L.Take();
        exit(S);
    end;"#);
    assert_eq!(
        got,
        "res / resolve area=LTY71 expr=x1 | after expr=LTY71,x1 caption=res session=Yes || res /  || \
         Lit x3 / after expr=3,Lit x3 caption=Lit x3 session=Yes || Lit x3 / "
    );
}

/// The caption an OnAfterCaptionClassResolve subscriber sets is the translation (al-runner issue 6924).
#[test]
fn an_on_after_resolve_subscriber_sets_the_caption_of_an_unclaimed_area() {
    let got = run(r#"
    procedure F(): Text
    var L: Codeunit Log;
    begin
        exit(CaptionClassTranslate('LTWC,27,9') + ' / ' + L.Take());
    end;"#);
    assert_eq!(
        got,
        "WC table 27 field 9 / after expr=LTWC,27,9 caption=LTWC,27,9 session=Yes"
    );
}

/// al-runner issue 6924: FieldCaption of a table extension field whose CaptionClass is an area nobody claims reads the caption
/// an OnAfterCaptionClassResolve subscriber sets (record/captions/WcExtFieldAfterResolve.al WcExtField_FieldCaption_AfterResolveSets).
#[test]
fn field_caption_of_a_table_extension_field_takes_the_on_after_resolve_caption() {
    let got = run(r#"
    procedure F(): Text
    var L: Codeunit Log; R: Record WcBase;
    begin
        exit(R.FieldCaption("WC Ext FC") + ' / ' + L.Take());
    end;"#);
    assert_eq!(
        got,
        "WC table 27 field 1 / after expr=LTWC,27,1 caption=LTWC,27,1 session=Yes"
    );
}

/// A caption class with no comma through CaptionClassTranslate is served like a table field's (an assumption for the function,
/// docs/probes-needed.md): the event once, the text as the caption.
#[test]
fn a_caption_class_without_a_comma_raises_the_event_once() {
    let got = run(r#"
    procedure F(): Text
    var L: Codeunit Log; S: Text;
    begin
        S := CaptionClassTranslate('no comma') + ' / ' + L.Take() + ' || ';
        S += CaptionClassTranslate('no comma') + ' / ' + L.Take();
        exit(S);
    end;"#);
    assert_eq!(
        got,
        "no comma / after expr=no comma caption=no comma session=Yes || no comma / "
    );
}

/// record/captions/WcExtFieldAfterResolve.al WcExtField_NoComma_FieldCaption, WcExtField_EmptyArea_FieldCaption (BC 27.5, 28.5,
/// 29.0): the event is raised once with the expression as both expression and caption; the subscriber left it, so it is the
/// caption. WcExtField_ThreeAlone_FieldCaption: `3,` alone raises nothing and gives the Caption.
#[test]
fn field_caption_of_the_unclaimed_shapes_raises_the_event_as_pinned() {
    let got = run(r#"
    procedure F(): Text
    var L: Codeunit Log; R: Record WcBase; S: Text;
    begin
        S := R.FieldCaption("WC No Comma") + ' / ' + L.Take() + ' || ';
        S += R.FieldCaption("WC Empty Area") + ' / ' + L.Take() + ' || ';
        S += R.FieldCaption("WC Three Alone") + ' / ' + L.Take();
        exit(S);
    end;"#);
    assert_eq!(
        got,
        "WCNOCOMMA / after expr=WCNOCOMMA caption=WCNOCOMMA session=Yes || \
         ,WC empty area / after expr=,WC empty area caption=,WC empty area session=Yes || WC Three Alone Cap / "
    );
}

/// WcControl_OwnArea3_TestPageCaption: the page open raises the event once for a control's own `3,<text>`, the control reads the
/// text after the comma, and reading it raises nothing more.
#[test]
fn a_controls_own_area_3_caption_class_raises_the_event_at_page_open() {
    let got = run(r#"
    procedure F(): Text
    var L: Codeunit Log; R: Record WcBase; P: TestPage WcCard; S: Text;
    begin
        R.DeleteAll(); R.Id := 1; R.Insert();
        P.OpenView();
        S := L.Take() + ' || ' + P.CtlThree.Caption() + ' || ' + L.Take();
        P.Close();
        exit(S);
    end;"#);
    assert_eq!(
        got,
        "after expr=3,WC ctl three caption=WC ctl three session=Yes || WC ctl three || "
    );
}
