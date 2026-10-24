//! TestPage.Close() on a page whose own OnQueryClosePage answers false: Close returns, and the last error is
//! 'The page cannot be closed at this time.' with the code FormCloseNotAllowed, also inside a TryFunction and over an
//! earlier error. An allowed close leaves the last error as it was (corpus handlers/TestPageCloseVeto_Tests.al:244-373;
//! al-runner issue 5769).

use al_testkit::{run_in, text};

const SRC: &str = r#"
codeunit 69980 "CV Mode"
{
    SingleInstance = true;
    var Veto: Boolean;
    procedure Set(V: Boolean) begin Veto := V; end;
    procedure Get(): Boolean begin exit(Veto); end;
}
page 69981 "CV Card"
{
    PageType = Card; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(F; X) { } } }
    trigger OnQueryClosePage(CloseAction: Action): Boolean var M: Codeunit "CV Mode"; begin exit(not M.Get()); end;
    var X: Integer;
}
codeunit 69982 C
{
    procedure Direct(Veto: Boolean): Text
    var P: TestPage "CV Card"; M: Codeunit "CV Mode";
    begin
        M.Set(Veto);
        P.OpenEdit();
        ClearLastError();
        P.Close();
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure VetoDirect(): Text begin exit(Direct(true)); end;
    procedure AllowDirect(): Text begin exit(Direct(false)); end;
    procedure VetoInTry(): Text
    var P: TestPage "CV Card"; M: Codeunit "CV Mode";
    begin
        M.Set(true);
        P.OpenEdit();
        ClearLastError();
        exit(Format(TryClose(P)) + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    [TryFunction]
    local procedure TryClose(var P: TestPage "CV Card")
    begin
        P.Close();
    end;
    procedure Earlier(Veto: Boolean): Text
    var P: TestPage "CV Card"; M: Codeunit "CV Mode";
    begin
        M.Set(Veto);
        P.OpenEdit();
        asserterror Error('earlier');
        P.Close();
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure VetoEarlier(): Text begin exit(Earlier(true)); end;
    procedure AllowEarlier(): Text begin exit(Earlier(false)); end;
}
"#;

#[test]
fn a_vetoed_close_leaves_the_veto_as_the_last_error() {
    assert_eq!(
        text(run_in(SRC, "C", "VetoDirect")),
        "The page cannot be closed at this time.|FormCloseNotAllowed"
    );
}

#[test]
fn a_vetoed_close_in_a_try_function_returns_true_and_leaves_the_veto() {
    assert_eq!(
        text(run_in(SRC, "C", "VetoInTry")),
        "Yes|The page cannot be closed at this time.|FormCloseNotAllowed"
    );
}

#[test]
fn a_vetoed_close_replaces_an_earlier_last_error() {
    assert_eq!(
        text(run_in(SRC, "C", "VetoEarlier")),
        "The page cannot be closed at this time.|FormCloseNotAllowed"
    );
}

#[test]
fn an_allowed_close_leaves_the_last_error_as_it_was() {
    assert_eq!(text(run_in(SRC, "C", "AllowDirect")), "|");
    assert_eq!(text(run_in(SRC, "C", "AllowEarlier")), "earlier|Dialog");
}
