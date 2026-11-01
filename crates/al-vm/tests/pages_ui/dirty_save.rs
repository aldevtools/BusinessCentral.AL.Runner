//! A TestPage saves the row it holds an edit of before the next step runs (corpus probe
//! pages/lt-y45-save-before-action/Y45SaveBeforeAction.al, file header lines 7-26): the modify chain (OnModifyRecord,
//! OnBeforeModifyEvent, OnModify, OnAfterModifyEvent) runs first, also for the same value set again, then the step.

const SRC: &str = include_str!("dirty_save_fixture.al");

fn step(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

const SAVE: &str = "pM new/old db=old/1;eBM new t=Yes db=old/1;tM new/old db=old/1;eAM db=new/1";
const LINE_SAVE: &str = "lpM 10000 new/a db=a;eBML new;lM 10000 new/a db=a";

// Y45SaveBeforeAction.al:98 Card_SetName_RunObjectCodeunit, :431 List_SetName_RunObjectCodeunit.
#[test]
fn a_runobject_codeunit_runs_after_the_save() {
    assert_eq!(step("CardRunCu"), format!("{SAVE};cu H1 rec=new/1 db=new/1 l1=a;R H1;C H1"));
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
