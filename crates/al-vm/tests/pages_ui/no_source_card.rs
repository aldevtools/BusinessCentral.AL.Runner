//! A card without a source table (pages/open.rs, pages/actions.rs). Each test names the corpus test of
//! testpage/P146PageExprSetValue.al that pins what BC does.

const SRC: &str = r#"
codeunit 69970 "NS Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin Entries += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
page 69971 "NS Card"
{
    PageType = Card; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Txt; TxtVar) { trigger OnValidate() var Log: Codeunit "NS Log"; begin Log.Add('validate=' + TxtVar); end; } } }
    actions
    {
        area(Processing)
        {
            action(Noop) { trigger OnAction() var Log: Codeunit "NS Log"; begin Log.Add('action'); end; }
            action(UpdFalse) { trigger OnAction() var Log: Codeunit "NS Log"; begin Log.Add('update'); CurrPage.Update(false); Log.Add('done'); end; }
        }
    }
    var TxtVar: Text;
    trigger OnOpenPage() var Log: Codeunit "NS Log"; begin TxtVar := 'start'; Log.Add('open'); end;
    trigger OnAfterGetRecord() var Log: Codeunit "NS Log"; begin Log.Add('agr'); end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "NS Log"; begin Log.Add('agcr'); end;
    trigger OnNewRecord(BelowxRec: Boolean) var Log: Codeunit "NS Log"; begin Log.Add('new'); end;
    trigger OnClosePage() var Log: Codeunit "NS Log"; begin Log.Add('close'); end;
}
codeunit 69972 C
{
    procedure Steps(): Text
    var P: TestPage "NS Card"; Log: Codeunit "NS Log"; R: Text;
    begin
        Log.Take();
        P.OpenEdit();
        R := Log.Take();
        R += '|' + P.Txt.Value() + ':' + Log.Take();
        P.Noop.Invoke();
        R += '|' + Log.Take();
        P.UpdFalse.Invoke();
        R += '|' + Log.Take();
        P.Txt.SetValue('typed');
        R += '|' + Log.Take();
        P.Close();
        R += '|' + Log.Take();
        exit(R);
    end;

    procedure OpenNew(): Text
    var P: TestPage "NS Card"; Log: Codeunit "NS Log";
    begin
        Log.Take();
        asserterror P.OpenNew();
        exit(GetLastErrorCode() + '|' + Log.Take());
    end;
}
"#;

/// P146PageExprSetValue.al:420 QG_NoSource_OpenEdit_StepLog (open `qg open;qg agcr;`, a read nothing, each action and an action
/// with CurrPage.Update(false) end with one OnAfterGetCurrRecord, Close `qg close;`) and :478 QG_NoSource_SetValue_ThenAction.
#[test]
fn a_card_without_a_source_runs_only_on_after_get_curr_record_after_open_and_each_action() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Steps")),
        "open;agcr;|start:|action;agcr;|update;done;agcr;|validate=typed;|close;"
    );
}

/// P146PageExprSetValue.al:460 QG_NoSource_OpenNew_StepLog: OpenNew fails with DB:ClientInsertDenied after OnOpenPage.
#[test]
fn open_new_of_a_card_without_a_source_is_denied_after_on_open_page() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "OpenNew")),
        "DB:ClientInsertDenied|open;"
    );
}
