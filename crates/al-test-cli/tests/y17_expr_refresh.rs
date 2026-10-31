//! Pins of pages/lt-y17-expr-refresh/Y17ExprRefresh.al (corpus writer Y17): what an expression control over a field of a
//! page-global record shows in a linked ListPart after TestField.Visible, TestAction.Visible, TestPage.GoToKey, TestPart.Last,
//! Next and First, and an action of the Document host that calls a procedure of the part.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69740 "YR Doc" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; "No.") { } } }
table 69741 "YR Line"
{
    fields { field(1; "Doc No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Amount; Decimal) { } }
    keys { key(PK; "Doc No.", "Line No.") { } }
}
page 69742 "YR Lines"
{
    PageType = ListPart; SourceTable = "YR Line"; UsageCategory = None; ApplicationArea = All;
    layout
    {
        area(Content)
        {
            repeater(Rows) { field("Line No."; Rec."Line No.") { } field(Amount; Rec.Amount) { } }
            group(Totals) { field(TotalAmt; TotalLine.Amount) { Editable = false; } field(TotalCount; LineCount) { Editable = false; } }
        }
    }
    trigger OnFindRecord(Which: Text): Boolean begin Log.Add('find'); Clear(TotalLine); exit(Rec.Find(Which)); end;
    trigger OnAfterGetRecord() begin Log.Add('agr ' + Format(Rec."Line No.")); end;
    trigger OnAfterGetCurrRecord()
    var SumLine: Record "YR Line";
    begin
        Log.Add('agcr ' + Format(Rec."Line No."));
        SumLine.SetRange("Doc No.", Rec."Doc No.");
        SumLine.CalcSums(Amount);
        TotalLine.Amount := SumLine.Amount;
        LineCount := SumLine.Count();
    end;
    trigger OnNewRecord(BelowxRec: Boolean)
    var Last: Record "YR Line";
    begin
        Last.SetRange("Doc No.", Rec."Doc No.");
        if Last.FindLast() then Rec."Line No." := Last."Line No." + 10000 else Rec."Line No." := 10000;
        Log.Add('new ' + Format(Rec."Line No."));
    end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Log.Add('ins ' + Format(Rec."Line No.")); exit(true); end;
    trigger OnModifyRecord(): Boolean begin Log.Add('mod ' + Format(Rec."Line No.")); exit(true); end;
    procedure AddDiscount()
    var Upd: Record "YR Line";
    begin
        Log.Add('proc');
        Upd.SetRange("Doc No.", Rec."Doc No.");
        if Upd.FindSet() then repeat Upd.Amount := Upd.Amount - 1; Upd.Modify(); until Upd.Next() = 0;
    end;
    var TotalLine: Record "YR Line"; Log: Codeunit "YR Log"; LineCount: Integer;
}
page 69743 "YR Card"
{
    PageType = Document; SourceTable = "YR Doc"; UsageCategory = None; ApplicationArea = All;
    layout
    {
        area(Content)
        {
            group(General) { field("No."; Rec."No.") { } field(Name; Rec.Name) { } }
            part(Lines; "YR Lines") { SubPageLink = "Doc No." = field("No."); }
        }
    }
    actions
    {
        area(Processing)
        {
            action(CalcDisc) { trigger OnAction() begin CurrPage.Lines.Page.AddDiscount(); end; }
            action(CalcDiscUpd) { trigger OnAction() begin CurrPage.Lines.Page.AddDiscount(); CurrPage.Update(false); end; }
        }
    }
    trigger OnAfterGetCurrRecord() begin Log.Add('card agcr'); end;
    var Log: Codeunit "YR Log";
}
codeunit 69744 "YR Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(E: Text) begin if T <> '' then T += ';'; T += E; end;
    procedure Take() R: Text begin R := T; T := ''; end;
}
codeunit 69745 T
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Log: Codeunit "YR Log";
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;
    local procedure Seed()
    var Doc: Record "YR Doc"; Line: Record "YR Line";
    begin
        Line.DeleteAll(); Doc.DeleteAll();
        Doc."No." := 'D1'; Doc.Name := 'Doc one'; Doc.Insert();
        Line."Doc No." := 'D1'; Line."Line No." := 10000; Line.Amount := 10; Line.Insert();
        Line."Line No." := 20000; Line.Amount := 5; Line.Insert();
        Log.Take();
    end;
    local procedure SetAmount(LineNo: Integer; Amount: Decimal)
    var Line: Record "YR Line";
    begin
        Line.Get('D1', LineNo); Line.Amount := Amount; Line.Modify();
    end;
    local procedure Stored() R: Text
    var Line: Record "YR Line";
    begin
        if Line.FindSet() then repeat R += Format(Line."Line No.") + '/' + Format(Line.Amount, 0, 9) + ';'; until Line.Next() = 0;
    end;
    local procedure Totals(var Card: TestPage "YR Card"): Text
    begin
        exit(Card.Lines.TotalAmt.Value() + '/' + Card.Lines.TotalCount.Value());
    end;

    [Test]
    procedure Visible_PartControl()
    var Card: TestPage "YR Card";
    begin
        Seed();
        Card.OpenEdit();
        Check('card agcr;find;agr 10000;agr 20000;agr 10000;find;agr 10000;agcr 10000', Log.Take());
        Card.GoToKey('D1');
        Check('card agcr', Log.Take());
        Check('15.00/2', Totals(Card));
        SetAmount(10000, 100);
        Check('Yes', Format(Card.Lines.Amount.Visible()));
        Check('', Log.Take());
        Check('15.00/2', Totals(Card));
        Check('10000/10.00', Card.Lines."Line No.".Value() + '/' + Card.Lines.Amount.Value());
        Check('', Log.Take());
    end;

    [Test]
    procedure Visible_CardControlAndAction()
    var Card: TestPage "YR Card";
    begin
        Seed();
        Card.OpenEdit();
        Card.GoToKey('D1');
        Check('15.00/2', Totals(Card));
        Log.Take();
        SetAmount(10000, 100);
        Check('Yes', Format(Card.Name.Visible()));
        Check('15.00/2', Totals(Card));
        SetAmount(20000, 50);
        Check('Yes', Format(Card.CalcDisc.Visible()));
        Check('15.00/2', Totals(Card));
        Check('', Log.Take());
    end;

    [Test]
    procedure PartNext_NewLineSaved()
    var Card: TestPage "YR Card";
    begin
        Seed();
        Card.OpenEdit();
        Card.GoToKey('D1');
        Check('15.00/2', Totals(Card));
        Log.Take();
        Check('Yes/20000', Format(Card.Lines.Last()) + '/' + Card.Lines."Line No.".Value());
        Check('new 30000;agcr 30000;agr 20000;find;agr 20000;agcr 20000', Log.Take());
        Check('Yes/30000', Format(Card.Lines.Next()) + '/' + Card.Lines."Line No.".Value());
        Check('new 30000;agcr 30000', Log.Take());
        Card.Lines.Amount.SetValue(7);
        Check('', Log.Take());
        Check('Yes/40000', Format(Card.Lines.Next()) + '/' + Card.Lines."Line No.".Value());
        Check('ins 30000;new 40000;agcr 40000', Log.Take());
        Check('22.00/3', Totals(Card));
        Check('', Log.Take());
        Check('10000/10;20000/5;30000/7;', Stored());
        Card.Close();
        Check('', Log.Take());
        Check('10000/10;20000/5;30000/7;', Stored());
    end;

    [Test]
    procedure PartNext_ExistingRows()
    var Card: TestPage "YR Card";
    begin
        Seed();
        Card.OpenEdit();
        Card.GoToKey('D1');
        Check('Yes/10000', Format(Card.Lines.First()) + '/' + Card.Lines."Line No.".Value());
        Check('15.00/2', Totals(Card));
        Log.Take();
        SetAmount(20000, 50);
        Check('Yes/20000/50.00', Format(Card.Lines.Next()) + '/' + Card.Lines."Line No.".Value() + '/' + Card.Lines.Amount.Value());
        Check('agr 20000;find;agr 20000;agcr 20000', Log.Take());
        Check('60.00/2', Totals(Card));
        Check('', Log.Take());
    end;

    [Test]
    procedure PartFirst_Linked()
    var Card: TestPage "YR Card";
    begin
        Seed();
        Card.OpenEdit();
        Card.GoToKey('D1');
        Check('15.00/2', Totals(Card));
        Log.Take();
        SetAmount(10000, 100);
        Check('Yes/10000/10.00', Format(Card.Lines.First()) + '/' + Card.Lines."Line No.".Value() + '/' + Card.Lines.Amount.Value());
        Check('', Log.Take());
        Check('15.00/2', Totals(Card));
        Check('', Log.Take());
    end;

    [Test]
    procedure CardAction_CallsPartProcedure()
    var Card: TestPage "YR Card";
    begin
        Seed();
        Card.OpenEdit();
        Card.GoToKey('D1');
        Check('15.00/2', Totals(Card));
        Check('10000/10.00', Card.Lines."Line No.".Value() + '/' + Card.Lines.Amount.Value());
        Log.Take();
        Card.CalcDisc.Invoke();
        Check('proc;card agcr;find;agr 10000;agr 20000;agr 10000;find;agr 10000;agcr 10000', Log.Take());
        Check('13.00/2', Totals(Card));
        Check('10000/9.00', Card.Lines."Line No.".Value() + '/' + Card.Lines.Amount.Value());
        Check('', Log.Take());
        Check('10000/9;20000/4;', Stored());
    end;

    [Test]
    procedure CardAction_CallsPartProcedure_Update()
    var Card: TestPage "YR Card";
    begin
        Seed();
        Card.OpenEdit();
        Card.GoToKey('D1');
        Check('15.00/2', Totals(Card));
        Log.Take();
        Card.CalcDiscUpd.Invoke();
        Check('proc;card agcr;find;agr 10000;agr 20000;agr 10000;find;agr 10000;agcr 10000', Log.Take());
        Check('13.00/2', Totals(Card));
        Check('10000/9.00', Card.Lines."Line No.".Value() + '/' + Card.Lines.Amount.Value());
        Check('10000/9;20000/4;', Stored());
    end;
}
"#;

/// Y17ExprRefresh.al:517 (Visible of a part control), :549 (of a card control and an action), :583 (part Last and Next to a
/// new row that Next saves), :626 (part Next over stored rows), :711 (part First on the current row), :653 and :682 (an
/// action of the host that calls a procedure of the part, without and with CurrPage.Update(false)).
#[test]
fn y17_expression_control_refresh() {
    let out = outcomes_in(SRC, "T");
    let bad: Vec<_> = out.iter().filter(|(_, o)| !matches!(o, Outcome::Pass)).collect();
    assert!(bad.is_empty(), "{bad:#?}");
    assert_eq!(out.len(), 7);
}
