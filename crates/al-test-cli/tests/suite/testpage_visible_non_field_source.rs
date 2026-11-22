//! The Visible state a TestPage reads for a field control whose Visible expression names a field of the source record
//! (al-runner issue 8660, corpus PR 1504 pages/controls/VisibleExpressionSource.al). A control whose source is an expression,
//! a page variable or a constant reads Yes, whatever the row holds; a control bound to a table field, or whose Visible names
//! no field, reads the value the expression had at open (a BC 29.1 container ran the probe; assumption
//! `visible-names-row-field-reads-yes`).

use al_testkit::outcomes_in;

const SRC: &str = r##"
table 50100 "Repro Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Integer) { }
        field(3; Flag; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50101 "Repro Expr Source"
{
    PageType = Card;
    SourceTable = "Repro Row";
    layout
    {
        area(Content)
        {
            field(AmountPlusOne; Rec.Amount + 1) { Visible = Rec.Amount > 0; }
        }
    }
}

page 50102 "Repro Other Sources"
{
    PageType = Card;
    SourceTable = "Repro Row";
    layout
    {
        area(Content)
        {
            field(ExprNegative; Rec.Amount + 1) { Visible = Rec.Amount < 0; }
            field(ExprFlag; Rec.Amount + 1) { Visible = Rec.Flag; }
            field(ExprMixed; Rec.Amount + 1) { Visible = ShowIt and (Rec.Amount >= 0); }
            field(ExprBareField; Rec.Amount + 1) { Visible = Amount > 0; }
            field(VarSource; Doubled) { Visible = Rec.Amount > 0; }
            field(ConstSource; 'Fixed') { Visible = Rec.Amount > 0; }
        }
    }
    var
        ShowIt: Boolean;
        Doubled: Integer;

    trigger OnAfterGetRecord()
    begin
        ShowIt := Rec.Amount > 0;
        Doubled := Rec.Amount * 2;
    end;
}

page 50103 "Repro Open Time"
{
    PageType = Card;
    SourceTable = "Repro Row";
    layout
    {
        area(Content)
        {
            field(FieldOwn; Rec.Amount) { Visible = Rec.Amount > 0; }
            field(FieldOther; Rec.Id) { Visible = Rec.Amount > 0; }
            field(ExprVariable; Rec.Amount + 1) { Visible = ShowIt; }
            field(ExprConstant; Rec.Amount + 1) { Visible = 1 = 0; }
        }
    }
    var
        ShowIt: Boolean;

    trigger OnAfterGetRecord()
    begin
        ShowIt := Rec.Amount > 0;
    end;
}

codeunit 50104 "Repro Visible Tests"
{
    Subtype = Test;

    [Test]
    procedure ExprSource()
    var
        Row: Record "Repro Row";
        Card: TestPage "Repro Expr Source";
    begin
        InsertRow(Row);
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Check('AmountPlusOne', Card.AmountPlusOne.Visible(), true);
        Card.Close();
    end;

    [Test]
    procedure OtherSources()
    var
        Row: Record "Repro Row";
        Card: TestPage "Repro Other Sources";
    begin
        InsertRow(Row);
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Check('ExprNegative', Card.ExprNegative.Visible(), true);
        Check('ExprFlag', Card.ExprFlag.Visible(), true);
        Check('ExprMixed', Card.ExprMixed.Visible(), true);
        Check('ExprBareField', Card.ExprBareField.Visible(), true);
        Check('VarSource', Card.VarSource.Visible(), true);
        Check('ConstSource', Card.ConstSource.Visible(), true);
        Card.Close();
    end;

    [Test]
    procedure OpenTime()
    var
        Row: Record "Repro Row";
        Card: TestPage "Repro Open Time";
    begin
        InsertRow(Row);
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Check('FieldOwn', Card.FieldOwn.Visible(), false);
        Check('FieldOther', Card.FieldOther.Visible(), false);
        Check('ExprVariable', Card.ExprVariable.Visible(), false);
        Check('ExprConstant', Card.ExprConstant.Visible(), false);
        Card.Close();
    end;

    local procedure InsertRow(var Row: Record "Repro Row")
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Amount := 1;
        Row.Flag := false;
        Row.Insert();
    end;

    local procedure Check(Name: Text; Actual: Boolean; Expected: Boolean)
    begin
        if Actual <> Expected then
            Error('%1 visible: %2, expected %3', Name, Actual, Expected);
    end;
}
"##;

#[test]
fn a_non_field_control_whose_visible_names_a_row_field_reads_yes() {
    let o = outcomes_in(SRC, "Repro Visible Tests");
    let got: Vec<String> = o.iter().map(|(n, x)| format!("{n}: {x:?}")).collect();
    assert_eq!(got, ["ExprSource: Pass", "OtherSources: Pass", "OpenTime: Pass"]);
}
