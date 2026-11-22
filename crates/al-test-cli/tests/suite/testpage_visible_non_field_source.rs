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

/// Shapes the rule does not cover keep the value at open (No here): a Visible whose only field is the qualifier of an
/// Option value (`Rec.Kind::B`, a constant), and sources that index an array or call a procedure (not measured).
const NOT_COVERED: &str = r##"
table 50110 "Repro Kind Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Option) { OptionMembers = A,B; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50111 "Repro Not Covered"
{
    PageType = Card;
    SourceTable = "Repro Kind Row";
    layout
    {
        area(Content)
        {
            field(ScopeOnly; Rec.Id + 1) { Visible = Rec.Kind::B = Rec.Kind::A; }
            field(IndexSource; Values[Index]) { Visible = Rec.Id > 0; }
            field(CallSource; Twice() + 1) { Visible = Rec.Id > 0; }
        }
    }
    var
        Values: array[2] of Integer;
        Index: Integer;

    local procedure Twice(): Integer
    begin
        exit(Rec.Id * 2);
    end;

    trigger OnOpenPage()
    begin
        Index := 1;
    end;
}

codeunit 50112 "Repro Not Covered Tests"
{
    Subtype = Test;

    [Test]
    procedure NotCovered()
    var
        Row: Record "Repro Kind Row";
        Card: TestPage "Repro Not Covered";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Card.OpenEdit();
        Card.GoToRecord(Row);
        if Card.ScopeOnly.Visible() or Card.IndexSource.Visible() or Card.CallSource.Visible() then
            Error('ScopeOnly %1, IndexSource %2, CallSource %3', Card.ScopeOnly.Visible(), Card.IndexSource.Visible(), Card.CallSource.Visible());
        Card.Close();
    end;
}
"##;

#[test]
fn shapes_the_rule_does_not_cover_keep_the_value_at_open() {
    let o = outcomes_in(NOT_COVERED, "Repro Not Covered Tests");
    let got: Vec<String> = o.iter().map(|(n, x)| format!("{n}: {x:?}")).collect();
    assert_eq!(got, ["NotCovered: Pass"]);
}

/// Before BC 29, a TestPage's open of such a Card raises "The identifier '<field>' could not be found." (code
/// Expression:Argument) for the first such control's field, after OnOpenPage and before OnAfterGetRecord (corpus PR 1504,
/// the BC 27.5 and 28.5 CI of VisibleExpressionSource.al). `Plain`, bound to a field, comes first and does not count: on
/// page 75708 a field-bound control with `Visible = Rec.Flag` comes first, and BC names the next control's field.
const BEFORE_29: &str = r##"
table 50120 "Repro Old Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Integer) { }
        field(3; Flag; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50121 "Repro Old Card"
{
    PageType = Card;
    SourceTable = "Repro Old Row";
    layout
    {
        area(Content)
        {
            field(Plain; Rec.Id) { Visible = Rec.Amount > 0; }
            field(FlagShown; Rec.Amount + 1) { Visible = Rec.Flag; }
            field(AmountShown; Rec.Amount + 1) { Visible = Rec.Amount > 0; }
        }
    }
}

page 50122 "Repro Old On Open"
{
    PageType = Card;
    SourceTable = "Repro Old Row";
    layout
    {
        area(Content)
        {
            field(AmountShown; Rec.Amount + 1) { Visible = Rec.Amount > 0; }
        }
    }
    trigger OnOpenPage()
    begin
        Error('OnOpenPage ran');
    end;
}

page 50123 "Repro Old On Get"
{
    PageType = Card;
    SourceTable = "Repro Old Row";
    layout
    {
        area(Content)
        {
            field(AmountShown; Rec.Amount + 1) { Visible = Rec.Amount > 0; }
        }
    }
    trigger OnAfterGetRecord()
    begin
        Error('OnAfterGetRecord ran');
    end;
}

codeunit 50124 "Repro Old Tests"
{
    Subtype = Test;

    [Test]
    procedure OpenModes()
    var
        Row: Record "Repro Old Row";
        EditCard: TestPage "Repro Old Card";
        ViewCard: TestPage "Repro Old Card";
        NewCard: TestPage "Repro Old Card";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        asserterror EditCard.OpenEdit();
        Check();
        asserterror ViewCard.OpenView();
        Check();
        asserterror NewCard.OpenNew();
        Check();
    end;

    [Test]
    procedure OnOpenPageFirst()
    var
        Card: TestPage "Repro Old On Open";
    begin
        asserterror Card.OpenEdit();
        if GetLastErrorText() <> 'OnOpenPage ran' then
            Error('got %1', GetLastErrorText());
    end;

    [Test]
    procedure BeforeOnAfterGetRecord()
    var
        Row: Record "Repro Old Row";
        Card: TestPage "Repro Old On Get";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        asserterror Card.OpenEdit();
        if GetLastErrorText() <> 'The identifier ''Amount'' could not be found.' then
            Error('got %1', GetLastErrorText());
    end;

    local procedure Check()
    begin
        if (GetLastErrorText() <> 'The identifier ''Flag'' could not be found.') or (GetLastErrorCode() <> 'Expression:Argument') then
            Error('got %1 (%2)', GetLastErrorText(), GetLastErrorCode());
    end;
}
"##;

#[test]
fn before_29_a_testpage_open_raises_the_client_compile_error() {
    for v in ["27.5", "28.5"] {
        let target = al_version::Target::for_bc(al_version::BcVersion::parse(v).unwrap());
        let o = al_testkit::outcomes_in_with(BEFORE_29, "Repro Old Tests", |e| e.target = Some(target));
        let got: Vec<String> = o.iter().map(|(n, x)| format!("{n}: {x:?}")).collect();
        assert_eq!(
            got,
            [
                "OpenModes: Pass",
                "OnOpenPageFirst: Pass",
                "BeforeOnAfterGetRecord: Pass"
            ],
            "{v}"
        );
    }
}

/// The routes to such a Card and the shapes next to the rule, as corpus PR 1504 measured them (al-runner PR 8667 review,
/// VisibleExpressionSource.al pages 75707-75710 and its route tests): another control binding the named field (`BoundElsewhere`,
/// `OneBound`) and the primary key (`NamesKey`) change nothing; a field-bound control's own Visible (`IdCtl`) keeps its value
/// at open; every route the probe measured reads Yes on BC 29 and raises at the open before it.
const ROUTES: &str = r##"
table 50130 "Repro Route Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Amount; Integer) { }
        field(3; Flag; Boolean) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50131 "Repro Route Card"
{
    PageType = Card;
    SourceTable = "Repro Route Row";
    layout { area(Content) { field(AmountPlusOne; Rec.Amount + 1) { Visible = Rec.Amount > 0; } } }
}

page 50132 "Repro Bound Elsewhere"
{
    PageType = Card;
    SourceTable = "Repro Route Row";
    layout
    {
        area(Content)
        {
            field(AmountCtl; Rec.Amount) { }
            field(Total; Rec.Amount + 1) { Visible = Rec.Amount > 0; }
        }
    }
}

page 50133 "Repro Bound First"
{
    PageType = Card;
    SourceTable = "Repro Route Row";
    layout
    {
        area(Content)
        {
            field(IdCtl; Rec.Id) { Visible = Rec.Flag; }
            field(Total; Rec.Amount + 1) { Visible = Rec.Amount > 0; }
        }
    }
}

page 50134 "Repro Names Key"
{
    PageType = Card;
    SourceTable = "Repro Route Row";
    layout { area(Content) { field(Total; Rec.Amount + 1) { Visible = Rec.Id > 0; } } }
}

page 50135 "Repro One Bound"
{
    PageType = Card;
    SourceTable = "Repro Route Row";
    layout
    {
        area(Content)
        {
            field(AmountCtl; Rec.Amount) { }
            field(Total; Rec.Amount + 1) { Visible = (Rec.Amount > 0) and Rec.Flag; }
        }
    }
}

codeunit 50136 "Repro Route Tests"
{
    Subtype = Test;

    var
        ModalVisible: Text;

    [Test]
    procedure OpenView()
    var
        Row: Record "Repro Route Row";
        Card: TestPage "Repro Route Card";
    begin
        InsertRow(Row);
        Card.OpenView();
        Card.GoToRecord(Row);
        Check(Card.AmountPlusOne.Visible(), true);
        Card.Close();
    end;

    [Test]
    procedure OpenNew()
    var
        Row: Record "Repro Route Row";
        Card: TestPage "Repro Route Card";
    begin
        InsertRow(Row);
        Card.OpenNew();
        Check(Card.AmountPlusOne.Visible(), true);
        Card.Close();
    end;

    [Test]
    procedure TrapRun()
    var
        Row: Record "Repro Route Row";
        Card: TestPage "Repro Route Card";
    begin
        InsertRow(Row);
        Card.Trap();
        Page.Run(Page::"Repro Route Card", Row);
        Check(Card.AmountPlusOne.Visible(), true);
        Card.Close();
    end;

    [Test]
    [HandlerFunctions('ModalHandler')]
    procedure Modal()
    var
        Row: Record "Repro Route Row";
    begin
        InsertRow(Row);
        ModalVisible := 'not run';
        Page.RunModal(Page::"Repro Route Card", Row);
        if ModalVisible <> 'Yes' then
            Error('in handler: %1', ModalVisible);
    end;

    [Test]
    [HandlerFunctions('ModalHandler')]
    procedure ModalLookup()
    var
        Row: Record "Repro Route Row";
    begin
        InsertRow(Row);
        if Page.RunModal(Page::"Repro Route Card", Row) = Action::LookupOK then;
    end;

    [Test]
    procedure BoundElsewhere()
    var
        Row: Record "Repro Route Row";
        Card: TestPage "Repro Bound Elsewhere";
    begin
        InsertRow(Row);
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Check(Card.Total.Visible(), true);
        Card.Close();
    end;

    [Test]
    procedure BoundFirst()
    var
        Row: Record "Repro Route Row";
        Card: TestPage "Repro Bound First";
    begin
        InsertRow(Row);
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Check(Card.IdCtl.Visible(), false);
        Check(Card.Total.Visible(), true);
        Card.Close();
    end;

    [Test]
    procedure NamesKey()
    var
        Row: Record "Repro Route Row";
        Card: TestPage "Repro Names Key";
    begin
        InsertRow(Row);
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Check(Card.Total.Visible(), true);
        Card.Close();
    end;

    [Test]
    procedure OneBound()
    var
        Row: Record "Repro Route Row";
        Card: TestPage "Repro One Bound";
    begin
        InsertRow(Row);
        Card.OpenEdit();
        Card.GoToRecord(Row);
        Check(Card.Total.Visible(), true);
        Card.Close();
    end;

    [ModalPageHandler]
    procedure ModalHandler(var Card: TestPage "Repro Route Card")
    begin
        ModalVisible := Format(Card.AmountPlusOne.Visible());
    end;

    local procedure InsertRow(var Row: Record "Repro Route Row")
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Amount := 1;
        Row.Flag := false;
        Row.Insert();
    end;

    local procedure Check(Actual: Boolean; Expected: Boolean)
    begin
        if Actual <> Expected then
            Error('visible: %1, expected %2', Actual, Expected);
    end;
}

codeunit 50137 "Repro Route Old Tests"
{
    Subtype = Test;

    [Test]
    procedure TrapRun()
    var
        Row: Record "Repro Route Row";
        Card: TestPage "Repro Route Card";
    begin
        InsertRow(Row);
        Card.Trap();
        asserterror Page.Run(Page::"Repro Route Card", Row);
        Check('Amount');
    end;

    [Test]
    [HandlerFunctions('ModalHandler')]
    procedure Modal()
    var
        Row: Record "Repro Route Row";
    begin
        InsertRow(Row);
        Page.RunModal(Page::"Repro Route Card", Row);
    end;

    [Test]
    procedure BoundElsewhere()
    var
        Card: TestPage "Repro Bound Elsewhere";
    begin
        asserterror Card.OpenEdit();
        Check('Amount');
    end;

    [Test]
    procedure BoundFirst()
    var
        Card: TestPage "Repro Bound First";
    begin
        asserterror Card.OpenEdit();
        Check('Amount');
    end;

    [Test]
    procedure NamesKey()
    var
        Card: TestPage "Repro Names Key";
    begin
        asserterror Card.OpenEdit();
        Check('Id');
    end;

    [Test]
    procedure OneBound()
    var
        Card: TestPage "Repro One Bound";
    begin
        asserterror Card.OpenEdit();
        Check('Amount');
    end;

    [ModalPageHandler]
    procedure ModalHandler(var Card: TestPage "Repro Route Card")
    begin
    end;

    local procedure InsertRow(var Row: Record "Repro Route Row")
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
    end;

    local procedure Check(Field: Text)
    begin
        if (GetLastErrorText() <> StrSubstNo('The identifier ''%1'' could not be found.', Field)) or (GetLastErrorCode() <> 'Expression:Argument') then
            Error('got %1 (%2)', GetLastErrorText(), GetLastErrorCode());
    end;
}
"##;

const REFUSED: &str = "opened in a way the probe did not measure";

/// Each test's outcome, as `Pass` (flagged or not) or `NotRun` when refused for an unmeasured route, on BC version `v`.
fn route_outcomes(codeunit: &str, v: &str) -> Vec<(String, &'static str, bool)> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(v).unwrap());
    crate::record_copy_reports::run_with(ROUTES, codeunit, |e| e.target = Some(target))
        .into_iter()
        .map(|(name, outcome, flagged)| {
            let kind = if outcome.starts_with("Pass") {
                "Pass"
            } else if outcome.starts_with("NotRun") && outcome.contains(REFUSED) {
                "Refused"
            } else {
                panic!("{name} on {v}: {outcome}")
            };
            (name, kind, flagged)
        })
        .collect()
}

#[test]
fn from_bc_29_every_measured_route_and_shape_reads_yes_flagged() {
    let got = route_outcomes("Repro Route Tests", "29.0");
    let want: Vec<(String, &str, bool)> = [
        ("OpenView", "Pass", true),
        ("OpenNew", "Pass", true),
        ("TrapRun", "Pass", true),
        ("Modal", "Pass", true),
        ("ModalLookup", "Refused", false),
        ("BoundElsewhere", "Pass", true),
        ("BoundFirst", "Pass", true),
        ("NamesKey", "Pass", true),
        ("OneBound", "Pass", true),
    ]
    .into_iter()
    .map(|(n, k, f)| (n.to_string(), k, f))
    .collect();
    assert_eq!(got, want);
}

#[test]
fn before_29_every_measured_route_and_shape_raises_at_the_open_flagged() {
    for v in ["27.5", "28.5"] {
        let got = route_outcomes("Repro Route Old Tests", v);
        let want: Vec<(String, &str, bool)> = [
            ("TrapRun", "Pass", true),
            ("Modal", "Refused", false),
            ("BoundElsewhere", "Pass", true),
            ("BoundFirst", "Pass", true),
            ("NamesKey", "Pass", true),
            ("OneBound", "Pass", true),
        ]
        .into_iter()
        .map(|(n, k, f)| (n.to_string(), k, f))
        .collect();
        assert_eq!(got, want, "{v}");
    }
}
