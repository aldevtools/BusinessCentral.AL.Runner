//! An action that puts another row (another primary key) into Rec: the client saves nothing (no OnModifyRecord, no OnModify, not
//! after the action, not on Next, not on Close), reads the list again and shows the other row with its stored values; xRec of the
//! next OnAfterGetCurrRecord is the row shown before. CurrPage.Close() in that action puts the shown row back into Rec.
//! Pinned by misc/lt-probes-needed-7/PnGRefresh.al:387 QL_RecAssign_OtherChanged_ThenClose, :416 _OtherChangedUpdate_ThenClose,
//! :445 _TempChanged_ThenClose, :532 _OtherChanged_StaysOpen_Next, :608 _OtherChanged_CloseInAction (corpus bump 42).
//! Tests-ERM 134918 CustApplyAndPostPaymentInv: Apply Customer Entries' Post Application sets `Rec := TempApplyingCustLedgEntry`.

const SRC: &str = r#"
table 51490 "MR Row"
{
    fields { field(1; Id; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() var L: Codeunit "MR Log"; begin L.Add('T.MOD(' + Id + ')'); end;
}
codeunit 51493 "MR Log"
{
    SingleInstance = true;
    var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ';'; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end;
}
page 51491 "MR List"
{
    PageType = List;
    SourceTable = "MR Row";
    layout { area(Content) { repeater(R) { field(IdF; Rec.Id) { } field(NameF; Rec.Name) { } } } }
    actions
    {
        area(Processing)
        {
            action(Move)
            {
                trigger OnAction()
                var Other: Record "MR Row";
                begin
                    Other.Get('B'); Other.Name := 'changed B'; Rec := Other;
                end;
            }
            action(MoveUpdate)
            {
                trigger OnAction()
                var Other: Record "MR Row";
                begin
                    Other.Get('B'); Other.Name := 'changed B'; Rec := Other; CurrPage.Update(false);
                end;
            }
            action(ChangeOwn)
            {
                trigger OnAction() begin Rec.Name := 'own changed'; end;
            }
            action(MoveClose)
            {
                trigger OnAction()
                var Other: Record "MR Row";
                begin
                    Other.Get('B'); Other.Name := 'changed B'; Rec := Other; CurrPage.Close();
                    L.Add('ACT.end(' + Rec.Id + ',' + Rec.Name + ')');
                end;
            }
        }
    }
    var L: Codeunit "MR Log";
    trigger OnAfterGetRecord() begin L.Add('AGR(' + Rec.Id + ',' + Rec.Name + ')'); end;
    trigger OnAfterGetCurrRecord() begin L.Add('AGCR(' + Rec.Id + ',' + Rec.Name + ' x=' + xRec.Id + ',' + xRec.Name + ')'); end;
    trigger OnModifyRecord(): Boolean begin L.Add('MODR'); end;
}
codeunit 51492 "MR Test"
{
    var L: Codeunit "MR Log";

    local procedure Seed()
    var R: Record "MR Row";
    begin
        R.DeleteAll();
        R.Id := 'A'; R.Name := 'orig A'; R.Insert();
        R.Id := 'B'; R.Name := 'orig B'; R.Insert();
        R.Id := 'C'; R.Name := 'orig C'; R.Insert();
        L.Take();
    end;

    local procedure Rows(): Text
    var R: Record "MR Row"; T: Text;
    begin
        if R.FindSet() then repeat T += R.Id + ':' + R.Name + ' '; until R.Next() = 0;
        exit(T);
    end;

    procedure Move(): Text
    var P: TestPage "MR List"; T: Text;
    begin
        Seed();
        P.OpenEdit();
        L.Take();
        P.Move.Invoke();
        T := L.Take() + '|' + P.IdF.Value() + ',' + P.NameF.Value() + '|' + Rows();
        P.Next();
        T += '|' + L.Take();
        P.Close();
        exit(T + '|' + L.Take() + '|' + Rows());
    end;

    procedure MoveUpdate(): Text
    var P: TestPage "MR List"; T: Text;
    begin
        Seed();
        P.OpenEdit();
        L.Take();
        P.MoveUpdate.Invoke();
        T := L.Take() + '|' + P.IdF.Value() + ',' + P.NameF.Value();
        P.Close();
        exit(T + '|' + L.Take() + '|' + Rows());
    end;

    procedure ChangeOwn(): Text
    var P: TestPage "MR List"; T: Text;
    begin
        Seed();
        P.OpenEdit();
        L.Take();
        P.ChangeOwn.Invoke();
        T := L.Take();
        P.Close();
        exit(T + '|' + L.Take() + '|' + Rows());
    end;

    procedure MoveClose(): Text
    var P: TestPage "MR List";
    begin
        Seed();
        P.OpenEdit();
        L.Take();
        P.MoveClose.Invoke();
        exit(L.Take() + '|' + Rows());
    end;
}
"#;

const READS: &str = "AGR(A,orig A);AGR(B,changed B);AGR(B,orig B);AGR(C,orig C);AGR(B,orig B);AGR(A,orig A);AGR(B,orig B);AGR(B,orig B);AGCR(B,orig B x=A,orig A)";
const STORED: &str = "A:orig A B:orig B C:orig C ";

#[test]
fn an_action_that_leaves_another_changed_row_in_rec_saves_nothing_and_shows_that_row_as_stored() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "MR Test", "Move"));
    assert_eq!(
        got,
        format!("{READS}|B,orig B|{STORED}|AGR(C,orig C);AGR(C,orig C);AGCR(C,orig C x=B,orig B)||{STORED}")
    );
}

#[test]
fn the_same_with_curr_page_update_in_the_action() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "MR Test", "MoveUpdate"));
    assert_eq!(got, format!("{READS}|B,orig B||{STORED}"));
}

#[test]
fn curr_page_close_in_that_action_puts_the_shown_row_back_into_rec() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "MR Test", "MoveClose"));
    assert_eq!(
        got,
        format!("AGR(A,orig A);AGR(A,orig A);ACT.end(A,orig A);AGR(A,orig A)|{STORED}")
    );
}

#[test]
fn an_action_that_changes_the_shown_row_reads_it_first_then_saves_it_then_reads_the_list() {
    let got = al_testkit::text(al_testkit::run_in(SRC, "MR Test", "ChangeOwn"));
    assert_eq!(
        got,
        "AGR(A,orig A);AGR(A,own changed);MODR;T.MOD(A);AGR(A,own changed);AGR(B,orig B);AGR(C,orig C);AGR(A,own changed);\
         AGR(A,own changed);AGCR(A,own changed x=A,own changed)||A:own changed B:orig B C:orig C "
    );
}

// The same list with OnQueryClosePage and OnClosePage (corpus bump 44, misc/lt-probes-needed-7/PnGRefresh.al:865
// QN_CloseQ_Sheet_OtherChanged_CloseInAction, :931 _OwnRowChanged_CloseInAction, :953 _NoChange_CloseInAction, and the List page at
// :1026, :1048, :1070): CurrPage.Close() reads the shown row back first, then the close triggers see it as Rec and xRec; nothing is
// stored; one more OnAfterGetRecord follows the action.
const CLOSE_SRC: &str = r#"
table 51494 "MC Row"
{
    fields { field(1; Id; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
    trigger OnModify() var L: Codeunit "MC Log"; begin L.Add('T.MOD(' + Id + ')'); end;
}
codeunit 51495 "MC Log"
{
    SingleInstance = true;
    var Buf: Text;
    procedure Add(T: Text) begin if Buf <> '' then Buf += ';'; Buf += T; end;
    procedure Take(): Text var R: Text; begin R := Buf; Buf := ''; exit(R); end;
}
page 51496 "MC List"
{
    PageType = List;
    SourceTable = "MC Row";
    layout { area(Content) { repeater(R) { field(IdF; Rec.Id) { } field(NameF; Rec.Name) { } } } }
    actions
    {
        area(Processing)
        {
            action(MoveClose)
            {
                trigger OnAction()
                var Other: Record "MC Row";
                begin
                    Other.Get('B'); Other.Name := 'changed B'; Rec := Other; CurrPage.Close();
                    L.Add('ACT.end(' + Rec.Id + ',' + Rec.Name + ' x=' + xRec.Id + ',' + xRec.Name + ')');
                end;
            }
            action(ChangeOwnClose)
            {
                trigger OnAction()
                begin
                    Rec.Name := 'own changed'; CurrPage.Close();
                    L.Add('ACT.end(' + Rec.Id + ',' + Rec.Name + ' x=' + xRec.Id + ',' + xRec.Name + ')');
                end;
            }
            action(CloseOnly)
            {
                trigger OnAction()
                begin
                    CurrPage.Close();
                    L.Add('ACT.end(' + Rec.Id + ',' + Rec.Name + ' x=' + xRec.Id + ',' + xRec.Name + ')');
                end;
            }
        }
    }
    var L: Codeunit "MC Log";
    trigger OnAfterGetRecord() begin L.Add('AGR(' + Rec.Id + ',' + Rec.Name + ')'); end;
    trigger OnAfterGetCurrRecord() begin L.Add('AGCR(' + Rec.Id + ',' + Rec.Name + ' x=' + xRec.Id + ',' + xRec.Name + ')'); end;
    trigger OnModifyRecord(): Boolean begin L.Add('MODR'); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        L.Add('QCP(' + Rec.Id + ',' + Rec.Name + ' x=' + xRec.Id + ',' + xRec.Name + ' a=' + Format(CloseAction) + ')');
        exit(true);
    end;
    trigger OnClosePage() begin L.Add('CP(' + Rec.Id + ',' + Rec.Name + ' x=' + xRec.Id + ',' + xRec.Name + ')'); end;
}
codeunit 51497 "MC Test"
{
    var L: Codeunit "MC Log";

    local procedure Seed()
    var R: Record "MC Row";
    begin
        R.DeleteAll();
        R.Id := 'A'; R.Name := 'orig A'; R.Insert();
        R.Id := 'B'; R.Name := 'orig B'; R.Insert();
        R.Id := 'C'; R.Name := 'orig C'; R.Insert();
        L.Take();
    end;

    local procedure Rows(): Text
    var R: Record "MC Row"; T: Text;
    begin
        if R.FindSet() then repeat T += R.Id + ':' + R.Name + ' '; until R.Next() = 0;
        exit(T);
    end;

    procedure MoveClose(): Text
    var P: TestPage "MC List";
    begin
        Seed(); P.OpenEdit(); L.Take();
        P.MoveClose.Invoke();
        exit(L.Take() + '|' + Rows());
    end;

    procedure ChangeOwnClose(): Text
    var P: TestPage "MC List";
    begin
        Seed(); P.OpenEdit(); L.Take();
        P.ChangeOwnClose.Invoke();
        exit(L.Take() + '|' + Rows());
    end;

    procedure CloseOnly(): Text
    var P: TestPage "MC List";
    begin
        Seed(); P.OpenEdit(); L.Take();
        P.CloseOnly.Invoke();
        exit(L.Take() + '|' + Rows());
    end;
}
"#;

const CLOSED: &str = "QCP(A,orig A x=A,orig A a=OK);CP(A,orig A x=A,orig A);ACT.end(A,orig A x=A,orig A);AGR(A,orig A)";

#[test]
fn with_close_triggers_curr_page_close_after_another_row_reads_the_shown_row_back_before_them() {
    let got = al_testkit::text(al_testkit::run_in(CLOSE_SRC, "MC Test", "MoveClose"));
    assert_eq!(got, format!("AGR(A,orig A);AGR(A,orig A);{CLOSED}|{STORED}"));
}

#[test]
fn with_close_triggers_curr_page_close_drops_a_change_to_the_own_row() {
    let got = al_testkit::text(al_testkit::run_in(CLOSE_SRC, "MC Test", "ChangeOwnClose"));
    assert_eq!(got, format!("AGR(A,orig A);{CLOSED}|{STORED}"));
}

#[test]
fn with_close_triggers_curr_page_close_alone_reads_the_row_once_more_after_the_action() {
    let got = al_testkit::text(al_testkit::run_in(CLOSE_SRC, "MC Test", "CloseOnly"));
    assert_eq!(got, format!("AGR(A,orig A);{CLOSED}|{STORED}"));
}
