//! The caption of a field whose CaptionClass expression gives a text with no comma, or with an area that is not a number
//! (al-runner issue 7382). The captions are pinned by corpus record/captions/CaptionExpressionPlainText.al (BC 27.5, 28.5 and
//! 29.0), which is their test; here are the two shapes that stay refused, which the corpus cannot reach.

use al_builtins::native::Signal;
use al_testkit::text;

const OBJECTS: &str = r#"
enum 50780 Kind { Extensible = true; value(0; Item) { Caption = 'Item'; } value(1; Resource) { Caption = 'Resource'; } }
table 50780 Row
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Enum Kind) { }
        field(4; PlainFormat; Code[20]) { CaptionClass = PlainFormatCaption(); }
        field(5; PlainLabel; Code[20]) { CaptionClass = PlainLabelCaption(); }
        field(6; EmptyText; Code[20]) { CaptionClass = EmptyCaption(); }
        field(7; NonNumericArea; Code[20]) { CaptionClass = NonNumericAreaCaption(); }
        field(8; WithCaption; Code[20]) { Caption = 'Static Caption'; CaptionClass = PlainFormatCaption(); }
    }
    keys { key(PK; Id) { Clustered = true; } }
    trigger OnInsert()
    var AlreadyErr: Label 'Already there. %1 = %2.';
    begin
        if Id = 99 then
            Error(AlreadyErr, FieldCaption(PlainFormat), PlainFormat);
    end;
    local procedure PlainFormatCaption(): Text[80] begin exit(Format(Kind)); end;
    local procedure PlainLabelCaption(): Text[80] var PlainLbl: Label 'Plain label text'; begin exit(PlainLbl); end;
    local procedure EmptyCaption(): Text[80] begin exit(''); end;
    local procedure NonNumericAreaCaption(): Text[80] begin exit('Zed,some text'); end;
}
page 50783 Card
{
    PageType = Card; SourceTable = Row;
    layout { area(Content) { group(General) {
        field(Id; Rec.Id) { } field(Kind; Rec.Kind) { } field(PlainFormat; Rec.PlainFormat) { } field(PlainLabel; Rec.PlainLabel) { }
        field(EmptyText; Rec.EmptyText) { } field(NonNumericArea; Rec.NonNumericArea) { } field(WithCaption; Rec.WithCaption) { }
    } } }
}
page 50784 List
{
    PageType = List; SourceTable = Row;
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { } field(Kind; Rec.Kind) { } field(PlainFormat; Rec.PlainFormat) { } field(PlainLabel; Rec.PlainLabel) { }
        field(EmptyText; Rec.EmptyText) { } field(NonNumericArea; Rec.NonNumericArea) { } field(WithCaption; Rec.WithCaption) { }
    } } }
}
"#;

const SUBSCRIBERS: &str = r#"
codeunit 42 "Caption Class"
{
    [IntegrationEvent(false, false)] procedure OnResolveCaptionClass(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end;
    [IntegrationEvent(false, false)] procedure OnAfterCaptionClassResolve(Language: Integer; CaptionExpression: Text; var Caption: Text[1024]) begin end;
}
"#;

/// What an expression in an area a subscriber resolves reads is not pinned (al-runner issue 7431): refused. An area no
/// subscriber resolves stays as written (the corpus run has such subscribers, record/captions/R6985FieldCaptionArea.al:61).
#[test]
fn an_expression_in_an_area_a_subscriber_resolves_is_refused() {
    let sub = |area: &str| {
        format!(
            "{OBJECTS}{SUBSCRIBERS}
codeunit 50788 Sub
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnResolveCaptionClass', '', true, true)]
    local procedure OnResolve(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean)
    begin
        if CaptionArea = '{area}' then begin Caption := 'claimed'; Resolved := true; end;
    end;
}}
codeunit 50789 T {{ procedure F(): Text var R: Record Row; begin exit(R.FieldCaption(NonNumericArea)); end; }}"
        )
    };
    assert_eq!(text(al_testkit::run_in(&sub("Other"), "T", "F")), "Zed,some text");
    let r = al_testkit::run_in(&sub("Zed"), "T", "F");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("which a subscriber resolves")),
        "{r:?}"
    );
}

/// A text with no comma that a subscriber of OnAfterCaptionClassResolve changes is pinned for a literal only
/// (record/captions/WcExtFieldAfterResolve.al): refused for an expression. One the subscriber leaves alone stays as written.
#[test]
fn a_text_with_no_comma_that_an_after_resolve_subscriber_changes_is_refused() {
    let src = |text: &str| {
        format!(
            "{OBJECTS}{SUBSCRIBERS}
codeunit 50788 Sub
{{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnAfterCaptionClassResolve', '', true, true)]
    local procedure OnAfter(Language: Integer; CaptionExpression: Text; var Caption: Text[1024])
    begin
        if CaptionExpression = '{text}' then Caption := 'changed';
    end;
}}
codeunit 50789 T {{ procedure F(): Text var R: Record Row; begin exit(R.FieldCaption(PlainLabel)); end; }}"
        )
    };
    assert_eq!(text(al_testkit::run_in(&src("Other"), "T", "F")), "Plain label text");
    let r = al_testkit::run_in(&src("Plain label text"), "T", "F");
    assert!(
        matches!(&r, Err(Signal::Unsupported(w)) if w.contains("which a subscriber resolves")),
        "{r:?}"
    );
}

/// `Enum.AsInteger` in the table's own CaptionClass expression leaves nothing behind (al-runner issue 7379; the corpus cases
/// CaptionExpressionPlainText.al:296-398 are the test). So does the same call inside a codeunit the expression calls, now pinned
/// (record/captions/CaptionClassStateFreeCalls.al:111-114 KindNumber, :188 HelperCc): the page open runs where the number of
/// empty-record runs is not pinned (two expressions with different texts), as a flagged pass.
#[test]
fn as_integer_in_a_codeunit_the_caption_expression_calls_runs() {
    let src = r#"
enum 50790 Kind { value(0; Item) { } value(1; Resource) { } }
codeunit 50791 Helper { procedure Number(K: Enum Kind): Integer begin exit(K.AsInteger()); end; }
table 50790 Row
{
    fields { field(1; Id; Integer) { } field(2; Kind; Enum Kind) { } field(3; Amount; Decimal) { CaptionClass = Cc(); } field(4; Other; Decimal) { CaptionClass = Cc2(); } }
    keys { key(PK; Id) { } }
    procedure Cc(): Text[80] var H: Codeunit Helper; begin if H.Number(Kind) = 0 then exit('3,All'); exit('3,Some'); end;
    procedure Cc2(): Text[80] begin exit('3,Other'); end;
}
page 50792 RowCard { PageType = Card; SourceTable = Row; layout { area(Content) { field(Amount; Rec.Amount) { } } } }
codeunit 50793 T { procedure F(): Text var P: TestPage RowCard; begin P.OpenView(); exit(P.Amount.Caption()); end; }
"#;
    assert_eq!(text(al_testkit::run_in(src, "T", "F")), "All");
}
