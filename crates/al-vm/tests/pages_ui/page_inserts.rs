//! New rows and inserts on a TestPage: OpenNew of a Worksheet, an API page, a card whose OnNewRecord inserts, an insert
//! into a filtered view (lane T7).

use al_builtins::native::Signal;
use al_testkit::text;

const WORKSHEET: &str = r#"
table 69889 "PI Num" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
page 69890 "PI Worksheet"
{
    PageType = Worksheet; SourceTable = "PI Num"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Number; Rec.Number) { } } } }
}
page 69891 "PI Worksheet Open"
{
    PageType = Worksheet; SourceTable = "PI Num"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Number; Rec.Number) { } } } }
    trigger OnInit() begin Error('OnInit ran'); end;
    trigger OnOpenPage() begin Error('OnOpenPage ran'); end;
}
codeunit 69892 C
{
    procedure OpenNew(): Text
    var P: TestPage "PI Worksheet";
    begin
        asserterror P.OpenNew();
        exit(GetLastErrorText() + ' [' + GetLastErrorCode() + ']');
    end;
    procedure OpenNewWithTrigger(): Text
    var P: TestPage "PI Worksheet Open";
    begin
        asserterror P.OpenNew();
        exit(GetLastErrorCode());
    end;
}
"#;

/// corpus pageext/lt-bl-implicit-creation/ImplicitCreation.al:308-318 Worksheet_OpenNew.
#[test]
fn worksheet_open_new_is_not_supported() {
    assert_eq!(
        text(al_testkit::run_in(WORKSHEET, "C", "OpenNew")),
        "The view mode 'Create' is not supported by the page ID '69890'. [NotSupportedPageViewMode]"
    );
}

/// The error comes before OnInit and OnOpenPage run (pageext/lt-bl-implicit-creation/Y8OpenNewPageTypes.al:306-317).
#[test]
fn worksheet_open_new_with_open_triggers_fails_before_them() {
    assert_eq!(
        text(al_testkit::run_in(WORKSHEET, "C", "OpenNewWithTrigger")),
        "NotSupportedPageViewMode"
    );
}

const API: &str = r#"
table 69880 "PI Api Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
    trigger OnInsert() begin Error('table OnInsert ran'); end;
}
page 69881 "PI Api"
{
    PageType = API; SourceTable = "PI Api Row";
    APIPublisher = 'pi'; APIGroup = 'pi'; APIVersion = 'v1.0'; EntityName = 'piRow'; EntitySetName = 'piRows';
    ODataKeyFields = Id; DelayedInsert = true;
    layout { area(Content) { repeater(R) { field(id; Rec.Id) { } field(name; Rec.Name) { } } } }
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Rec.Insert(true); exit(false); end;
}
page 69882 "PI Api Plain"
{
    PageType = API; SourceTable = "PI Api Row";
    APIPublisher = 'pi'; APIGroup = 'pi'; APIVersion = 'v1.0'; EntityName = 'piRow2'; EntitySetName = 'piRows2';
    ODataKeyFields = Id; DelayedInsert = true;
    layout { area(Content) { repeater(R) { field(id; Rec.Id) { } } } }
}
codeunit 69883 C
{
    procedure OpenNew(): Text
    var P: TestPage "PI Api";
    begin
        asserterror P.OpenNew();
        exit(GetLastErrorText() + ' [' + GetLastErrorCode() + ']');
    end;
    procedure SetAfter(): Text
    var P: TestPage "PI Api";
    begin
        asserterror P.OpenNew();
        asserterror P.id.SetValue(1);
        exit(GetLastErrorCode());
    end;
    procedure OpenNewPlain()
    var P: TestPage "PI Api Plain";
    begin
        P.OpenNew();
    end;
}
"#;

/// corpus pages/lt-cf-page-leads/CfPageLeads.al:144-180 ApiPage_OnInsertRecordInsertsAndReturnsFalse.
#[test]
fn api_page_open_new_is_denied() {
    let t = text(al_testkit::run_in(API, "C", "OpenNew"));
    assert!(
        t.starts_with("Sorry, the current permissions prevented the action. (Page 69881 PI Api Insert: ")
            && t.ends_with(") [DB:ClientInsertDenied]"),
        "{t}"
    );
    assert_eq!(text(al_testkit::run_in(API, "C", "SetAfter")), "TestPageNotOpened");
}

/// An API page without OnInsertRecord: not pinned, refused.
#[test]
fn api_page_open_new_other_shape_is_refused() {
    let r = al_testkit::run_in(API, "C", "OpenNewPlain");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

const NEW_CARD: &str = r#"
codeunit 69870 "PI Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(T: Text) begin if Lines <> '' then Lines += ','; Lines += T; end;
    procedure Take(): Text var R: Text; begin R := Lines; Lines := ''; exit(R); end;
}
table 69871 "PI Doc"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[30]) { trigger OnValidate() var L: Codeunit "PI Log"; begin L.Add('Description.OnValidate'); end; }
    }
    keys { key(PK; "No.") { } }
    trigger OnInsert() var L: Codeunit "PI Log"; begin if "No." = '' then "No." := 'DOC1'; L.Add('Doc.OnInsert:' + "No."); end;
    trigger OnModify() var L: Codeunit "PI Log"; begin L.Add('Doc.OnModify'); end;
}
page 69872 "PI New Card"
{
    PageType = Card; SourceTable = "PI Doc"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field("No."; Rec."No.") { } field(Description; Rec.Description) { } } }
    trigger OnNewRecord(BelowxRec: Boolean) var L: Codeunit "PI Log"; begin L.Add('OnNewRecord'); if Rec."No." = '' then Rec.Insert(true); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean var L: Codeunit "PI Log"; begin L.Add('OnInsertRecord:' + Rec."No."); exit(true); end;
    trigger OnModifyRecord(): Boolean var L: Codeunit "PI Log"; begin L.Add('OnModifyRecord:' + Rec."No."); exit(true); end;
}
codeunit 69873 C
{
    var Log: Codeunit "PI Log";
    local procedure Rows(): Text
    var D: Record "PI Doc"; R: Text;
    begin
        if D.FindSet() then repeat R += '[' + D."No." + '/' + D.Description + ']'; until D.Next() = 0;
        exit(R);
    end;
    procedure Flow(): Text
    var D: Record "PI Doc"; P: TestPage "PI New Card"; R: Text;
    begin
        D.DeleteAll();
        Log.Take();
        P.OpenNew();
        R := Log.Take() + '|' + Rows() + '|' + P."No.".Value() + '|';
        if TrySet(P) then R += 'set ok';
        R += GetLastErrorText() + ' [' + GetLastErrorCode() + ']|' + Log.Take() + '|' + Rows() + '|';
        P.Close();
        exit(R + Log.Take() + '|' + Rows());
    end;
    [TryFunction]
    local procedure TrySet(var P: TestPage "PI New Card")
    begin
        P.Description.SetValue('Hello');
    end;
    procedure NextAfterOpen()
    var D: Record "PI Doc"; P: TestPage "PI New Card";
    begin
        D.DeleteAll();
        P.OpenNew();
        P.Next();
    end;
    procedure NextAfterSet()
    var D: Record "PI Doc"; P: TestPage "PI New Card";
    begin
        D.DeleteAll();
        P.OpenNew();
        if TrySet(P) then;
        P.Next();
    end;
}
"#;

/// A card whose OnNewRecord inserts the new row itself: the page still holds it as a new row, so the next write to a field
/// outside the key inserts it again, which fails (corpus pages/lt-cf-page-leads/CfPageLeads.al:241-289
/// Card_OnNewRecordInserts_OpenNewSetClose).
#[test]
fn card_on_new_record_insert_keeps_the_new_row() {
    assert_eq!(
        text(al_testkit::run_in(NEW_CARD, "C", "Flow")),
        "OnNewRecord,Doc.OnInsert:DOC1|[DOC1/]|DOC1|\
         Validation error for Field: Description,  Message = 'This field cannot be validated since the key fields are not valid.' [TestValidation]|\
         OnInsertRecord:DOC1,Doc.OnInsert:DOC1|[DOC1/]||[DOC1/]"
    );
}

/// Any other call on such a page is not pinned: refused.
#[test]
fn card_on_new_record_insert_other_calls_are_refused() {
    for routine in ["NextAfterOpen", "NextAfterSet"] {
        let r = al_testkit::run_in(NEW_CARD, "C", routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
}

const CLEAR_ALL: &str = r#"
table 69860 "PI Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69861 "PI CA Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Add(T: Text) begin Lines += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Lines; Lines := ''; exit(R); end;
}
page 69862 "PI ClearAll Card"
{
    PageType = Card; SourceTable = "PI Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } } }
    var BaseFlag: Boolean;
    trigger OnOpenPage() begin BaseFlag := true; end;
    trigger OnAfterGetRecord() var L: Codeunit "PI CA Log";
    begin
        L.Add('base before=' + Format(BaseFlag));
        ClearAll();
        L.Add('base after=' + Format(BaseFlag) + ' id=' + Format(Rec.Id));
    end;
}
pageextension 69863 "PI ClearAll Ext" extends "PI ClearAll Card"
{
    var ExtFlag: Boolean;
    trigger OnOpenPage() begin ExtFlag := true; end;
    trigger OnAfterGetRecord() var L: Codeunit "PI CA Log"; begin L.Add('ext flag=' + Format(ExtFlag) + ' id=' + Format(Rec.Id)); end;
}
codeunit 69864 C
{
    procedure Run(): Text
    var R: Record "PI Row"; L: Codeunit "PI CA Log"; P: TestPage "PI ClearAll Card";
    begin
        R.Id := 7; R.Insert();
        L.Take();
        P.OpenView();
        P.Close();
        exit(L.Take());
    end;
}
"#;

/// corpus pages/lt-ce-page-run/CePageExt.al:27-41 ClearAll_InOwnBasePage_ExtensionGlobal.
#[test]
fn clear_all_in_a_page_clears_its_extension_globals() {
    assert_eq!(
        text(al_testkit::run_in(CLEAR_ALL, "C", "Run")),
        "base before=Yes;base after=No id=7;ext flag=No id=7;"
    );
}

const FILTERED: &str = r#"
table 69850 "PI F Row"
{
    fields { field(1; Id; Integer) { } field(2; Amount; Decimal) { } field(3; Checked; Decimal) { trigger OnValidate() begin end; } }
    keys { key(PK; Id) { } }
}
page 69851 "PI F List"
{
    PageType = List; SourceTable = "PI F Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(Id; Rec.Id) { } field(Amount; Rec.Amount) { } field(Checked; Rec.Checked) { } } } }
}
codeunit 69852 C
{
    var Kept: TestPage "PI F List";
    [TryFunction]
    local procedure TryNewRow(Filter: Text; Id: Integer)
    var List: TestPage "PI F List";
    begin
        List.OpenEdit();
        if Filter <> '' then
            List.Filter.SetFilter(Id, Filter);
        List.New();
        List.Id.SetValue(Id);
        List.Amount.SetValue(1);
        List.Close();
    end;
    procedure Inside(): Text
    var R: Record "PI F Row";
    begin
        if not TryNewRow('1..5', 3) then exit(GetLastErrorText());
        exit(Format(R.Get(3)));
    end;
    procedure OutsideRaises(): Text
    begin
        // A global variable: its end of scope is not reached here.
        Kept.OpenEdit();
        Kept.Filter.SetFilter(Id, '1..5');
        Kept.New();
        Kept.Id.SetValue(7);
        asserterror Kept.Amount.SetValue(1);
        exit(GetLastErrorText() + ' [' + GetLastErrorCode() + ']');
    end;
    procedure OutsideScopeExit()
    begin
        if TryNewRow('1..5', 7) then;
    end;
    procedure OutsideValidatedField()
    var List: TestPage "PI F List";
    begin
        List.OpenEdit();
        List.Filter.SetFilter(Id, '1..5');
        List.New();
        List.Id.SetValue(7);
        List.Checked.SetValue(1);
    end;
}
"#;

/// corpus events/lt-cm-trigger-leads/CmTriggerLeads.al:405-440 Item40_PageInsert_FilteredView, and pages/lt-ao-testpage/
/// AoTests.al:467-487 FilterGroup0_NewRowField: a new list row the page inserts outside its filter.
#[test]
fn list_insert_outside_the_filter_raises_on_set_value() {
    assert_eq!(text(al_testkit::run_in(FILTERED, "C", "Inside")), "Yes");
    assert_eq!(
        text(al_testkit::run_in(FILTERED, "C", "OutsideRaises")),
        "Validation error for Field: ,  Message = 'The view is filtered, and the entry is outside the filter. Some actions may not work.' [TestValidation]"
    );
}

/// What the page saves when its variable then goes out of scope, and whether a field with validation code is validated
/// first, are not pinned: refused.
#[test]
fn list_insert_outside_the_filter_unpinned_paths_are_refused() {
    for routine in ["OutsideScopeExit", "OutsideValidatedField"] {
        let r = al_testkit::run_in(FILTERED, "C", routine);
        assert!(matches!(r, Err(Signal::Unsupported(_))), "{routine}: {r:?}");
    }
}
