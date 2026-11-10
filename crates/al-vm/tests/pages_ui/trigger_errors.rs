//! An error in a page trigger while a TestPage opens, moves, starts a new row, inserts or modifies a row, or closes
//! (corpus testpage/CcPageLeads.al:453-633, the TriggerError_* tests, on the card of CcFixtures.al:336-421).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 69990 "TE Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 69991 "TE Log"
{
    SingleInstance = true;
    var FailAt: Text; Lines: Text;
    procedure SetFailAt(T: Text) begin FailAt := T; end;
    procedure GetFailAt(): Text begin exit(FailAt); end;
    procedure Add(T: Text) begin Lines += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Lines; Lines := ''; exit(R); end;
}
page 69992 "TE Card"
{
    PageType = Card; SourceTable = "TE Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } field(Name; Rec.Name) { } } }
    trigger OnInit() begin MaybeFail('OnInit'); end;
    trigger OnOpenPage() begin MaybeFail('OnOpenPage'); end;
    trigger OnFindRecord(Which: Text): Boolean begin MaybeFail('OnFindRecord'); exit(Rec.Find(Which)); end;
    trigger OnNextRecord(Steps: Integer): Integer begin MaybeFail('OnNextRecord'); exit(Rec.Next(Steps)); end;
    trigger OnAfterGetRecord() begin MaybeFail('OnAfterGetRecord'); end;
    trigger OnAfterGetCurrRecord() begin MaybeFail('OnAfterGetCurrRecord'); end;
    trigger OnNewRecord(BelowxRec: Boolean)
    var R: Record "TE Row"; L: Codeunit "TE Log";
    begin
        if L.GetFailAt() = 'DeleteThenFail' then begin
            R.Get(2);
            R.Delete();
            Error('TE failure after a delete');
        end;
        MaybeFail('OnNewRecord');
    end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin MaybeFail('OnInsertRecord'); exit(true); end;
    trigger OnModifyRecord(): Boolean begin MaybeFail('OnModifyRecord'); exit(true); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin MaybeFail('OnQueryClosePage'); exit(true); end;
    trigger OnClosePage() begin MaybeFail('OnClosePage'); end;
    local procedure MaybeFail(T: Text)
    var L: Codeunit "TE Log";
    begin
        if L.GetFailAt() = T then begin
            L.Add('hit ' + T);
            Error('TE failure in %1', T);
        end;
    end;
}
page 69993 "TE List"
{
    PageType = List; SourceTable = "TE Row"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
    trigger OnAfterGetRecord() var L: Codeunit "TE Log"; begin if L.GetFailAt() = 'OnAfterGetRecord' then Error('TE list failure'); end;
}
codeunit 69998 "TE Sub"
{
    [EventSubscriber(ObjectType::Page, Page::"TE Card", 'OnClosePageEvent', '', false, false)]
    local procedure OnClose(var Rec: Record "TE Row")
    var L: Codeunit "TE Log";
    begin
        if L.GetFailAt() = 'Subscriber' then Error('TE subscriber failure');
    end;
}
codeunit 69994 C
{
    var Log: Codeunit "TE Log";
    local procedure Outcome(): Text
    var R: Text;
    begin
        if GetLastErrorText() = 'OK' then R := 'ok' else R := GetLastErrorText() + ' [' + GetLastErrorCode() + ']';
        ClearLastError();
        exit(R);
    end;
    local procedure Seed()
    var R: Record "TE Row";
    begin
        R.DeleteAll();
        R.Id := 1; R.Name := 'a'; R.Insert();
        R.Id := 2; R.Name := 'b'; R.Insert();
        Log.SetFailAt('');
        Log.Take();
    end;
    local procedure ErrOpenEdit(var P: TestPage "TE Card") begin P.OpenEdit(); Error('OK'); end;
    local procedure ErrOpenView(var P: TestPage "TE Card") begin P.OpenView(); Error('OK'); end;
    local procedure ErrNext(var P: TestPage "TE Card") begin P.Next(); Error('OK'); end;
    local procedure ErrNew(var P: TestPage "TE Card") begin P.New(); Error('OK'); end;
    local procedure ErrInsert(var P: TestPage "TE Card") begin P.Id.SetValue(9); P.Name.SetValue('n'); P.First(); Error('OK'); end;
    local procedure ErrModify(var P: TestPage "TE Card") begin P.Name.SetValue('m'); P.Next(); Error('OK'); end;
    local procedure ErrClose(var P: TestPage "TE Card") begin P.Close(); Error('OK'); end;
    local procedure ErrRead(var P: TestPage "TE Card") var V: Text; begin V := P.Name.Value(); Error('OK'); end;

    procedure RunCase(TriggerName: Text; Step: Integer): Text
    var P: TestPage "TE Card"; R: Text;
    begin
        Seed();
        if Step = 0 then begin
            Log.SetFailAt(TriggerName);
            asserterror ErrOpenEdit(P);
        end else if Step = 6 then begin
            Log.SetFailAt(TriggerName);
            asserterror ErrOpenView(P);
        end else begin
            P.OpenEdit();
            P.First();
            Log.Take();
            if Step = 3 then
                P.New();
            Log.SetFailAt(TriggerName);
            case Step of
                1: asserterror ErrNext(P);
                2: asserterror ErrNew(P);
                3: asserterror ErrInsert(P);
                4: asserterror ErrModify(P);
                5: asserterror ErrClose(P);
            end;
        end;
        R := Outcome() + '|' + Log.Take();
        Log.SetFailAt('');
        asserterror ErrRead(P);
        R += '|' + Outcome();
        asserterror ErrClose(P);
        exit(R + '|' + Outcome());
    end;
    procedure OnFindRecord(): Text begin exit(RunCase('OnFindRecord', 0)); end;
    procedure OnAfterGetRecord(): Text begin exit(RunCase('OnAfterGetRecord', 0)); end;
    procedure OnAfterGetCurrRecord(): Text begin exit(RunCase('OnAfterGetCurrRecord', 0)); end;
    procedure OnAfterGetRecordView(): Text begin exit(RunCase('OnAfterGetRecord', 6)); end;
    procedure OnNextRecord(): Text begin exit(RunCase('OnNextRecord', 1)); end;
    procedure OnNewRecord(): Text begin exit(RunCase('OnNewRecord', 2)); end;
    procedure OnInsertRecord(): Text begin exit(RunCase('OnInsertRecord', 3)); end;
    procedure OnModifyRecord(): Text begin exit(RunCase('OnModifyRecord', 4)); end;
    procedure OnQueryClosePage(): Text begin exit(RunCase('OnQueryClosePage', 5)); end;
    procedure OnClosePage(): Text begin exit(RunCase('OnClosePage', 5)); end;

    procedure NewThenStep(): Text
    var P: TestPage "TE Card";
    begin
        Seed();
        P.OpenEdit();
        Log.SetFailAt('OnNewRecord');
        asserterror ErrNew(P);
        exit(Outcome());
    end;

    procedure ModifyThenPrevious(): Text
    var P: TestPage "TE Card";
    begin
        Seed();
        P.OpenEdit();
        P.Last();
        P.Name.SetValue('m');
        Log.SetFailAt('OnModifyRecord');
        asserterror P.Previous();
        exit(Outcome());
    end;

    procedure ModifyThenNextUsed(): Text
    var P: TestPage "TE Card"; B: Boolean;
    begin
        Seed();
        P.OpenEdit();
        P.First();
        P.Name.SetValue('m');
        Log.SetFailAt('OnModifyRecord');
        B := P.Next();
        exit(Format(B));
    end;

    procedure NewDeletesThenFails(): Text
    var P: TestPage "TE Card";
    begin
        Seed();
        P.OpenEdit();
        Log.SetFailAt('DeleteThenFail');
        asserterror ErrNew(P);
        exit(Outcome());
    end;

    procedure SubscriberCloseError(): Text
    var P: TestPage "TE Card";
    begin
        Seed();
        P.OpenEdit();
        Log.SetFailAt('Subscriber');
        asserterror P.Close();
        exit(Outcome());
    end;

    procedure OkActionCloseError(): Text
    var P: TestPage "TE Card";
    begin
        Seed();
        P.OpenEdit();
        Log.SetFailAt('OnClosePage');
        asserterror P.OK().Invoke();
        exit(Outcome());
    end;

    procedure ListOpenEdit(): Text
    var P: TestPage "TE List";
    begin
        Seed();
        Log.SetFailAt('OnAfterGetRecord');
        asserterror P.OpenEdit();
        exit(GetLastErrorText());
    end;
}
"#;

const NOT_OPEN: &str = "The TestPage is not open. [TestPageNotOpened]";

fn case(name: &str) -> String {
    text(al_testkit::run_in(SRC, "C", name))
}

fn refused(name: &str) -> String {
    match al_testkit::run_in(SRC, "C", name) {
        Err(Signal::Unsupported(why)) => why.into_reason(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

// CcPageLeads.al:520-557: an error in OnFindRecord, OnAfterGetRecord or OnAfterGetCurrRecord at OpenEdit of a card.
#[test]
fn an_error_while_a_card_opens_for_editing_says_the_page_has_to_close() {
    for t in ["OnFindRecord", "OnAfterGetRecord", "OnAfterGetCurrRecord"] {
        assert_eq!(
            case(t),
            format!("Page TE Card has to close. [FormAbort:Dialog]|hit {t};|{NOT_OPEN}|{NOT_OPEN}"),
            "{t}"
        );
    }
}

// testpage/lifecycle/TestPageTeardown.al:1130-1141 (CardTrigger_FailedOpen_Value_RaisesNotOpen): OpenView raises the error itself.
#[test]
fn an_error_while_a_card_opens_for_viewing_is_raised_as_it_is() {
    assert_eq!(
        case("OnAfterGetRecordView"),
        format!("TE failure in OnAfterGetRecord [Dialog]|hit OnAfterGetRecord;|{NOT_OPEN}|{NOT_OPEN}")
    );
}

// The same error while a list opens for editing is not pinned: its text cannot be read.
#[test]
fn an_error_while_a_list_opens_for_editing_has_no_pinned_text() {
    let why = refused("ListOpenEdit");
    assert!(why.contains("not pinned"), "{why}");
}

// CcPageLeads.al:559-569: OnNextRecord on Next() names the caption with the row's key.
#[test]
fn an_error_in_on_next_record_says_the_page_has_to_close_with_the_key() {
    assert_eq!(
        case("OnNextRecord"),
        format!("Page TE Card - 1 has to close. [FormAbort:Dialog]|hit OnNextRecord;|{NOT_OPEN}|{NOT_OPEN}")
    );
}

// CcPageLeads.al:572-582: New() swallows the error of OnNewRecord.
#[test]
fn new_swallows_an_error_in_on_new_record() {
    assert_eq!(case("NewThenStep"), "ok");
}

// What the page then shows is not pinned (docs/probes-needed.md): the next call on it is refused.
#[test]
fn the_page_after_a_swallowed_on_new_record_error_is_refused() {
    let why = refused("OnNewRecord");
    assert!(why.contains("OnNewRecord"), "{why}");
}

// CcPageLeads.al:585-595: the insert SetValue starts raises a TestValidation error with no field name; the page after it is not
// pinned and is refused.
#[test]
fn an_error_in_on_insert_record_is_a_validation_error_then_refused() {
    let why = refused("OnInsertRecord");
    assert!(why.contains("OnInsertRecord"), "{why}");
}

// CcPageLeads.al:598-608: Next() swallows the error of OnModifyRecord; the page after it is refused.
#[test]
fn the_page_after_a_swallowed_on_modify_record_error_is_refused() {
    let why = refused("OnModifyRecord");
    assert!(why.contains("OnModifyRecord"), "{why}");
}

// CcPageLeads.al:611-621 (OnQueryClosePage) needs a test method for the unhandled message: al-test-cli tests/page_trigger_errors.rs.

// CcPageLeads.al:624-633: wrapped, code TestWrapped, then the page is closed.
#[test]
fn an_error_in_on_close_page_is_wrapped() {
    assert_eq!(
        case("OnClosePage"),
        format!(
            "One or more errors occurred while closing the page.\n\nTE failure in OnClosePage [TestWrapped]|hit OnClosePage;|{NOT_OPEN}|{NOT_OPEN}"
        )
    );
}

// Only Next() is pinned for an OnModifyRecord error, and only where its answer is not used.
#[test]
fn an_on_modify_record_error_on_another_move_is_refused() {
    assert!(refused("ModifyThenPrevious").contains("OnModifyRecord on Previous()"));
}

#[test]
fn the_used_answer_of_next_after_an_on_modify_record_error_is_refused() {
    assert!(refused("ModifyThenNextUsed").contains("answer of Next()"));
}

// Whether a delete the failing OnNewRecord made survives the swallowed error is not pinned: refused at once.
#[test]
fn a_swallowed_on_new_record_error_after_a_delete_is_refused() {
    assert!(refused("NewDeletesThenFails").contains("wrote nothing"));
}

// Only the page's own OnClosePage at Close() is pinned: a subscriber's error, and an OnClosePage error at a close by an action,
// are refused.
#[test]
fn other_close_errors_are_refused() {
    assert!(refused("SubscriberCloseError").contains("OnClosePageEvent subscriber"));
    assert!(refused("OkActionCloseError").contains("close by an action"));
}
