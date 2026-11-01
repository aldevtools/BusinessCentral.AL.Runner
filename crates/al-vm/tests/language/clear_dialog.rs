//! Clear of a Dialog, opened or not, raises nothing and leaves it closed: Update and Close then raise "The operation failed
//! because the dialog box is not open." (code Dialog) and Open works again; ClearAll and Clear of a `var` parameter do the same
//! (al-runner issue 6656; corpus writer Y64, branch al-runtime-rs-staging-y64 85d86e7b, pages/lt-page-runtime/P82Dialog.al:
//! 442-547 Y64_Clear_UnopenedDialog, Y64_Clear_OpenedDialog, Y64_Clear_DialogInSingleInstanceCodeunit).

use al_testkit::text;

const SRC: &str = r#"
codeunit 69801 "Status Dialog"
{
    SingleInstance = true;
    var IsOpen: Boolean; Window: Dialog;
    procedure Open() begin IsOpen := true; Window.Open('#1####'); end;
    procedure Reset() begin Clear(IsOpen); Clear(Window); end;
    procedure ResetAll() begin ClearAll(); end;
    procedure ResetVar() begin ClearDialog(Window); Clear(IsOpen); end;
    local procedure ClearDialog(var Dlg: Dialog) begin Clear(Dlg); end;
    procedure State(): Text begin exit('IsOpen=' + Format(IsOpen)); end;
    procedure TryUpdate(): Text
    begin
        ClearLastError();
        if DoUpdate() then exit('ok');
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    [TryFunction] local procedure DoUpdate() begin Window.Update(1, 'x'); end;
}
codeunit 69802 C
{
    [TryFunction] local procedure TryClose(var D: Dialog) begin D.Close(); end;
    [TryFunction] local procedure TryOpen(var D: Dialog) begin D.Open('Value #1####'); end;
    [TryFunction] local procedure TryUpdate(var D: Dialog) begin D.Update(1, 5); end;

    procedure Local(): Text
    var D: Dialog; T: Text;
    begin
        Clear(D);
        T := Format(TryUpdate(D)) + ':' + GetLastErrorText() + ';';
        T += Format(TryClose(D)) + ';' + Format(TryOpen(D)) + ';' + Format(TryUpdate(D)) + ';';
        Clear(D);
        T += Format(TryUpdate(D)) + ';' + Format(TryOpen(D)) + ';' + Format(TryClose(D)) + ';';
        Clear(D);
        Clear(D);
        exit(T);
    end;

    procedure Single(): Text
    var S: Codeunit "Status Dialog"; Again: Codeunit "Status Dialog"; T: Text;
    begin
        S.Reset();
        T := S.State() + ';';
        S.Open();
        S.Reset();
        T += S.State() + ';' + S.TryUpdate() + ';';
        S.Open();
        S.ResetAll();
        T += Again.State() + ';' + Again.TryUpdate() + ';';
        Again.Open();
        S.ResetVar();
        T += S.State() + ';' + S.TryUpdate() + ';';
        exit(T);
    end;
}
"#;

#[test]
fn clear_closes_a_local_dialog() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Local")),
        "No:The operation failed because the dialog box is not open.;No;Yes;Yes;No;Yes;Yes;"
    );
}

#[test]
fn clear_clearall_and_a_var_parameter_close_a_single_instance_dialog() {
    let closed = "error:The operation failed because the dialog box is not open.|Dialog";
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Single")),
        format!("IsOpen=No;IsOpen=No;{closed};IsOpen=No;{closed};IsOpen=No;{closed};")
    );
}
