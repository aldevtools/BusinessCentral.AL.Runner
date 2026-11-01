//! A TestPage saves the row it holds an edit of before the next step runs (corpus probe
//! pages/lt-y45-save-before-action/Y45SaveBeforeAction.al, file header lines 7-26): the modify chain (OnModifyRecord,
//! OnBeforeModifyEvent, OnModify, OnAfterModifyEvent) runs first, also for the same value set again, then the step.

const SRC: &str = include_str!("dirty_save_fixture.al");

fn step(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

const SAVE: &str = "pM new/old db=old/1;eBM new t=Yes db=old/1;tM new/old db=old/1;eAM db=new/1";
const LINE_SAVE: &str = "lpM 10000 new/a db=a;eBML new;lM 10000 new/a db=a";

// Y45SaveBeforeAction.al:98 Card_SetName_RunObjectCodeunit, :431 List_SetName_RunObjectCodeunit, :670 on a Document.
#[test]
fn a_runobject_codeunit_runs_after_the_save() {
    assert_eq!(step("CardRunCu"), format!("{SAVE};cu H1 rec=new/1 db=new/1 l1=a;R H1;C H1"));
    // :670 Doc_SetName_RunObjectCodeunit: the part reads its row after the host's.
    assert_eq!(step("DocRunCu"), format!("{SAVE};cu H1 rec=new/1 db=new/1 l1=a;R H1;C H1;lC 10000"));
    assert_eq!(step("ListRunCu"), format!("{SAVE};cu H1 rec=new/1 db=new/1 l1=a;R H1;R H2;R H1;R H1;C H1"));
}

// Y45SaveBeforeAction.al:128 Card_SetName_RunObjectPage.
#[test]
fn a_runobject_page_opens_after_the_save() {
    assert_eq!(step("CardRunPg"), format!("new|{SAVE};peek open f=H1 db=new/1;peek C new"));
}

// Y45SaveBeforeAction.al:371 Card_SetSameName_Action.
#[test]
fn the_same_value_set_again_still_runs_the_modify_chain() {
    assert_eq!(
        step("CardSameName"),
        "pM old/old db=old/1;eBM old t=Yes db=old/1;tM old/old db=old/1;eAM db=old/1;R H1;act H1 rec=old/1 db=old/1 l1=a;R H1;R H1;C H1"
    );
}

// Y45SaveBeforeAction.al:700 Doc_SetName_LineAction, :817 Doc_SetLine_Action, :847 Doc_SetLine_LineAction.
#[test]
fn a_document_saves_the_edited_host_or_line_before_an_action_of_the_other() {
    assert_eq!(step("DocSetNameLineAction"), format!("{SAVE};lact 10000 rec=a db=a head=new/1;lC 10000"));
    assert_eq!(
        step("DocSetLineAction"),
        format!("{LINE_SAVE};R H1;act H1 rec=old/1 db=old/1 l1=new;R H1;R H1;C H1;lC 10000")
    );
    assert_eq!(step("DocSetLineLineAction"), format!("{LINE_SAVE};lact 10000 rec=new db=new head=old/1;lC 10000"));
}

// Y45SaveBeforeAction.al:1748 Doc_Existing_NewLine_Action.
#[test]
fn a_new_part_line_is_inserted_before_a_host_action() {
    assert_eq!(
        step("DocNewLineAction"),
        "lpI 30000 nl;eBIL nl;lI H1 30000 nl head=old/1;R H1;act H1 rec=old/1 db=old/1 l1=a;R H1;R H1;C H1;lC 30000"
    );
}

const FAILED_SAVE: &str = "pM boom/old db=old/1;eBM boom t=Yes db=old/1;tM boom/old db=old/1";
const KEY_NOT_VALID: &str = "Validation error for Field: Name,  Message = 'This field cannot be validated since the key fields are not valid.";

// Y45SaveBeforeAction.al:1808 Card_ModifyError_Action_PageState, :1190 Card_ModifyError_Action: no error reaches the test, the
// action does not run, the page holds the edit and one error on no control; the next SetValue fails, an action and Close run
// nothing.
#[test]
fn a_failed_save_before_an_action_holds_the_page() {
    assert_eq!(
        step("CardModifyErrorState"),
        format!("{FAILED_SAVE}|Modify refused for boom.|1|0|old/1|boom|{KEY_NOT_VALID} (Select Refresh to discard errors)'||")
    );
}

// :1228 Card_ModifyError_Next: Next answers No.
#[test]
fn a_failed_save_before_next_answers_no() {
    assert_eq!(step("CardModifyErrorNext"), format!("No|{FAILED_SAVE}|"));
}

// :1956 Doc_ModifyError_LineAction_PageState: the part action still runs; :1907 List_ModifyError_Action_PageState: no hint.
#[test]
fn a_part_action_runs_after_its_host_failed_to_save() {
    assert_eq!(
        step("DocModifyErrorLineAction"),
        format!("{FAILED_SAVE};lact 10000 rec=a db=a head=old/1;lC 10000|1|{KEY_NOT_VALID} (Select Refresh to discard errors)'")
    );
    assert_eq!(step("ListModifyErrorSetAgain"), format!("{KEY_NOT_VALID}'"));
}

// :1296 Card_ValidateError_Action, :1342 Card_ValidateError_Next, :1427 List_ValidateError_Action: after a refused entry an
// action and Next run nothing, Next answers No; Close runs nothing and stores nothing.
#[test]
fn a_refused_entry_holds_the_next_step() {
    assert_eq!(
        step("CardValidateErrorSteps"),
        "Validation error for Field: Qty,  Message = 'Qty must not be negative. (Select Refresh to discard errors)'|1|1||No|||old/1"
    );
    assert_eq!(
        step("ListValidateErrorAction"),
        "Validation error for Field: Qty,  Message = 'Qty must not be negative.'|"
    );
}
