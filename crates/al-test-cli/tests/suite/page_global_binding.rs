//! A manual binding made from a page trigger on a codeunit held by a global of the page (the shape of al-runner issue
//! 6924: a licensed page's `OnOpenPage` calls a controller codeunit that binds a manual subscriber on its own global, and
//! the page's instance is what holds the binding). Pinned by corpus codeunits/TestEventManualBinding.al Contract 11
//! (:277-376): the binding fires while the page is open, ends when a TestPage closes or a page no variable holds closes,
//! and lives on after a Page variable's RunModal while the variable is in scope. A local TestPage left open drops the
//! binding once its variable is gone (:379-421 Test11i-k). A Page variable after its non-modal Run is not pinned: an
//! event that would reach its binding is refused.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50510 "LP Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 50511 "LP Block Sub"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Table, Database::"LP Row", 'OnBeforeInsertEvent', '', false, false)]
    local procedure OnBeforeInsert(var Rec: Record "LP Row"; RunTrigger: Boolean)
    begin
        Error('blocked by the page binding');
    end;
}

codeunit 50512 "LP Controller"
{
    var
        BlockSub: Codeunit "LP Block Sub";

    procedure CheckIfInLicense()
    begin
        BindSubscription(BlockSub);
    end;
}

page 50510 "LP Card"
{
    PageType = Card;
    SourceTable = "LP Row";

    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { }
        }
    }

    var
        Controller: Codeunit "LP Controller";

    trigger OnOpenPage()
    begin
        Controller.CheckIfInLicense();
    end;
}

codeunit 50513 "LP Tests"
{
    Subtype = Test;

    [Test]
    procedure T01_ControllerLocal()
    var
        Controller: Codeunit "LP Controller";
    begin
        Controller.CheckIfInLicense();
    end;

    [Test]
    procedure T02_InsertAfterControllerLocal()
    var
        Row: Record "LP Row";
    begin
        Row.Id := 1;
        Row.Insert();
    end;

    [Test]
    procedure T03_InsertWhileTestPageOpen()
    var
        Card: TestPage "LP Card";
        Row: Record "LP Row";
    begin
        Card.OpenView();
        Row.Id := 2;
        asserterror Row.Insert();
        if GetLastErrorText() <> 'blocked by the page binding' then
            Error('unexpected: %1', GetLastErrorText());
        Card.Close();
    end;

    [Test]
    procedure T04_InsertAfterTestPageClose()
    var
        Card: TestPage "LP Card";
        Row: Record "LP Row";
    begin
        Card.OpenView();
        Card.Close();
        Row.Id := 3;
        Row.Insert();
    end;

    [Test]
    procedure T05_InsertInNextTest()
    var
        Row: Record "LP Row";
    begin
        Row.Id := 4;
        Row.Insert();
    end;

    [Test]
    procedure T06_TestPageLeftOpen()
    var
        Card: TestPage "LP Card";
    begin
        Card.OpenView();
    end;

    [Test]
    procedure T07_InsertAfterTestPageLeftOpen()
    var
        Row: Record "LP Row";
    begin
        Row.Id := 5;
        Row.Insert();
    end;

    [Test]
    procedure T08_InsertAfterHelperLeftTestPageOpen()
    var
        Row: Record "LP Row";
    begin
        OpenCardAndLeaveItOpen();
        Row.Id := 6;
        Row.Insert();
    end;

    local procedure OpenCardAndLeaveItOpen()
    var
        Card: TestPage "LP Card";
    begin
        Card.OpenView();
    end;
}

codeunit 50514 "LP Modal Tests"
{
    Subtype = Test;

    [Test]
    [HandlerFunctions('CardModalHandler')]
    procedure M01_InsertAfterRunModalById()
    var
        Row: Record "LP Row";
    begin
        Page.RunModal(Page::"LP Card");
        Row.Id := 11;
        Row.Insert();
    end;

    [Test]
    [HandlerFunctions('CardPageHandler')]
    procedure M02_InsertAfterRunById()
    var
        Row: Record "LP Row";
    begin
        Page.Run(Page::"LP Card");
        Row.Id := 12;
        Row.Insert();
    end;

    [Test]
    [HandlerFunctions('CardModalHandler')]
    procedure M03_InsertAfterRunModalOfPageVariable()
    var
        Card: Page "LP Card";
        Row: Record "LP Row";
    begin
        Card.RunModal();
        Row.Id := 13;
        asserterror Row.Insert();
        if GetLastErrorText() <> 'blocked by the page binding' then
            Error('unexpected: %1', GetLastErrorText());
    end;

    [Test]
    procedure M04_InsertAfterPageVariableGone()
    var
        Row: Record "LP Row";
    begin
        Row.Id := 14;
        Row.Insert();
    end;

    [Test]
    [HandlerFunctions('CardPageHandler')]
    procedure M05_InsertAfterRunOfPageVariable()
    var
        Card: Page "LP Card";
        Row: Record "LP Row";
    begin
        Card.Run();
        Row.Id := 15;
        Row.Insert();
    end;

    [Test]
    procedure M06_InsertAfterModelessPageVariableGone()
    var
        Row: Record "LP Row";
    begin
        Row.Id := 16;
        Row.Insert();
    end;

    [ModalPageHandler]
    procedure CardModalHandler(var Card: TestPage "LP Card")
    begin
        Card.OK().Invoke();
    end;

    [PageHandler]
    procedure CardPageHandler(var Card: TestPage "LP Card")
    begin
        Card.OK().Invoke();
    end;
}
"#;

#[test]
fn a_binding_a_page_holds_follows_the_page_instance() {
    let mut all = outcomes_in(SRC, "LP Tests");
    all.extend(outcomes_in(SRC, "LP Modal Tests"));
    // TestEventManualBinding.al:277 Test11a, :291 Test11b, :302 Test11c, :317 Test11d, :330 Test11e, :343 Test11f,
    // :355 Test11g, :367 Test11h, :379 Test11i, :393 Test11j, :404 Test11k.
    for pass in [
        "T01_ControllerLocal",
        "T02_InsertAfterControllerLocal",
        "T03_InsertWhileTestPageOpen",
        "T04_InsertAfterTestPageClose",
        "T05_InsertInNextTest",
        "T06_TestPageLeftOpen",
        "T07_InsertAfterTestPageLeftOpen",
        "T08_InsertAfterHelperLeftTestPageOpen",
        "M01_InsertAfterRunModalById",
        "M02_InsertAfterRunById",
        "M03_InsertAfterRunModalOfPageVariable",
        "M04_InsertAfterPageVariableGone",
        "M06_InsertAfterModelessPageVariableGone",
    ] {
        assert!(matches!(outcome(&all, pass), Outcome::Pass), "{pass}: {all:#?}");
    }
}

#[test]
fn a_binding_of_a_page_whose_fate_is_not_pinned_is_refused_where_it_would_fire() {
    let mut all = outcomes_in(SRC, "LP Tests");
    all.extend(outcomes_in(SRC, "LP Modal Tests"));
    let refused = "M05_InsertAfterRunOfPageVariable";
    match outcome(&all, refused) {
        Outcome::NotRun(why) => assert!(
            why.contains("whether that page still holds it is not pinned"),
            "{refused}: {why}"
        ),
        other => panic!("{refused}: {other:?}"),
    }
}

fn outcome<'a>(all: &'a [(String, Outcome)], name: &str) -> &'a Outcome {
    &all.iter()
        .find(|(n, _)| n == name)
        .unwrap_or_else(|| panic!("no test {name}: {all:?}"))
        .1
}
