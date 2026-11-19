//! `--dev-endpoint-ui` and the session's application areas (al-runner issue 8333). Microsoft's Test Runner app empties
//! them before every test method (ALTestRunnerResetEnvironment.Codeunit.al:27; corpus permissions/testpermissions/BaTpEnv.al:53-55
//! reads the empty text at the start of a test), which the default mode models. The dev-endpoint test runner skips
//! that step, and what the areas hold there is not measured: under the flag, `ApplicationArea()` before the test set
//! them is refused, and so is a page reader whose answer they decide. Off, every test keeps main's result.

use al_sema::workspace::AppSource;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun, TestResult};

const SRC: &str = r#"
page 50972 "Dev Endpoint Areas Card"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(Info; InfoText) { ApplicationArea = All; }
        }
    }
    var
        InfoText: Text;
}

page 50974 "Dev Endpoint Mixed Card"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(InAll; AllText) { ApplicationArea = All; }
            field(InAllAndBasic; BothText) { ApplicationArea = Basic, All; }
            field(InArea; AreaText) { ApplicationArea = Manufacturing; }
            field(NoArea; NoAreaText) { }
        }
    }
    var
        AllText: Text;
        BothText: Text;
        AreaText: Text;
        NoAreaText: Text;
}

page 50975 "Dev Endpoint Init Sets Card"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(InBasic; BasicText) { ApplicationArea = Basic; }
        }
    }
    var
        BasicText: Text;

    trigger OnInit()
    begin
        ApplicationArea('#Basic');
    end;
}

page 50976 "Dev Endpoint Group Card"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            group(InBasicGroup)
            {
                ApplicationArea = Basic;
                field(InGroup; GroupText) { ApplicationArea = All; }
            }
        }
    }
    var
        GroupText: Text;
}

page 50977 "Dev Endpoint Part"
{
    PageType = CardPart;
    layout
    {
        area(Content)
        {
            field(PartInfo; PartText) { ApplicationArea = All; }
        }
    }
    var
        PartText: Text;
}

page 50978 "Dev Endpoint All Part Host"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(HostInfo; HostText) { ApplicationArea = All; }
            part(Sub; "Dev Endpoint Part") { ApplicationArea = All; }
        }
    }
    var
        HostText: Text;
}

page 50979 "Dev Endpoint Basic Part Host"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(HostInfo; HostText) { ApplicationArea = All; }
            part(Sub; "Dev Endpoint Part") { ApplicationArea = Basic; }
        }
    }
    var
        HostText: Text;
}

page 50980 "Dev Endpoint Group Part Host"
{
    PageType = Card;
    layout
    {
        area(Content)
        {
            field(HostInfo; HostText) { ApplicationArea = All; }
            group(BasicGroup)
            {
                ApplicationArea = Basic;
                part(Sub; "Dev Endpoint Part") { ApplicationArea = All; }
            }
        }
    }
    var
        HostText: Text;
}

codeunit 50971 "Dev Endpoint Areas"
{
    Subtype = Test;

    [Test]
    procedure ReadAreas_NotSet()
    begin
        if ApplicationArea() <> '' then
            Error('areas %1', ApplicationArea());
    end;

    [Test]
    procedure OpenTestPage_NotSet()
    var
        Card: TestPage "Dev Endpoint Areas Card";
    begin
        Card.OpenView();
        if Card.Info.Value <> '' then
            Error('info %1', Card.Info.Value);
        Card.Close();
    end;

    [Test]
    procedure SetsAreas()
    begin
        ApplicationArea('#Basic');
    end;

    [Test]
    procedure ReadAreas_SetInEarlierTest()
    begin
        if ApplicationArea() <> '' then
            Error('areas %1', ApplicationArea());
    end;

    [Test]
    procedure ReadAreas_AfterSet()
    begin
        ApplicationArea('#Basic');
        if ApplicationArea() <> '#Basic' then
            Error('areas %1', ApplicationArea());
    end;

    [Test]
    procedure ReadAreas_AfterSetEmpty()
    begin
        ApplicationArea('');
        if ApplicationArea() <> '' then
            Error('areas %1', ApplicationArea());
    end;

    [Test]
    procedure OpenTestPage_AfterSet()
    var
        Card: TestPage "Dev Endpoint Areas Card";
    begin
        ApplicationArea('#Basic');
        Card.OpenView();
        Card.Close();
    end;

    [Test]
    procedure NoRead()
    begin
    end;

    [Test]
    procedure Mixed_ControlsWithAll_NotSet()
    var
        Card: TestPage "Dev Endpoint Mixed Card";
    begin
        Card.OpenView();
        Card.InAll.SetValue('a');
        Card.InAllAndBasic.SetValue('b');
        Card.Close();
    end;

    [Test]
    procedure Mixed_ControlInOtherArea_NotSet()
    var
        Card: TestPage "Dev Endpoint Mixed Card";
    begin
        Card.OpenView();
        Card.InArea.SetValue('c');
        Card.Close();
    end;

    [Test]
    procedure Mixed_ControlWithoutArea_NotSet()
    var
        Card: TestPage "Dev Endpoint Mixed Card";
    begin
        Card.OpenView();
        Card.NoArea.SetValue('d');
        Card.Close();
    end;

    [Test]
    procedure Mixed_ControlInOtherArea_AfterSet()
    var
        Card: TestPage "Dev Endpoint Mixed Card";
    begin
        ApplicationArea('#Manufacturing');
        Card.OpenView();
        Card.InArea.SetValue('c');
        Card.Close();
    end;

    [Test]
    procedure OnInitSets_ControlRead()
    var
        Card: TestPage "Dev Endpoint Init Sets Card";
    begin
        Card.OpenView();
        Card.InBasic.SetValue('e');
        Card.Close();
    end;

    [Test]
    procedure GroupInOtherArea_NotSet()
    var
        Card: TestPage "Dev Endpoint Group Card";
    begin
        Card.OpenView();
        Card.InGroup.SetValue('f');
        Card.Close();
    end;

    [Test]
    procedure PartWithAll_NotSet()
    var
        Host: TestPage "Dev Endpoint All Part Host";
    begin
        Host.OpenView();
        Host.Sub.PartInfo.SetValue('g');
        Host.Close();
    end;

    [Test]
    procedure PartInOtherArea_NotSet()
    var
        Host: TestPage "Dev Endpoint Basic Part Host";
    begin
        Host.OpenView();
        Host.HostInfo.SetValue('h');
        Host.Close();
    end;

    [Test]
    procedure PartWithAllInBasicGroup_TouchPart_NotSet()
    var
        Host: TestPage "Dev Endpoint Group Part Host";
    begin
        Host.OpenView();
        Host.Sub.PartInfo.SetValue('m');
        Host.Close();
    end;

    [Test]
    procedure PartWithAllInBasicGroup_OpenOnly_NotSet()
    var
        Host: TestPage "Dev Endpoint Group Part Host";
    begin
        Host.OpenView();
        Host.HostInfo.SetValue('n');
        Host.Close();
    end;

    [Test]
    procedure PartWithAllInBasicGroup_TouchPart_AfterSet()
    var
        Host: TestPage "Dev Endpoint Group Part Host";
    begin
        ApplicationArea('#Basic');
        Host.OpenView();
        Host.Sub.PartInfo.SetValue('o');
        Host.Close();
    end;

    [Test]
    [HandlerFunctions('ModalReadsAll')]
    procedure RunModalHandled_ControlWithAll_NotSet()
    begin
        Page.RunModal(Page::"Dev Endpoint Mixed Card");
    end;

    [Test]
    [HandlerFunctions('ModalReadsOtherArea')]
    procedure RunModalHandled_ControlInOtherArea_NotSet()
    begin
        Page.RunModal(Page::"Dev Endpoint Mixed Card");
    end;

    [ModalPageHandler]
    procedure ModalReadsAll(var Card: TestPage "Dev Endpoint Mixed Card")
    begin
        Card.InAll.SetValue('i');
    end;

    [ModalPageHandler]
    procedure ModalReadsOtherArea(var Card: TestPage "Dev Endpoint Mixed Card")
    begin
        Card.InArea.SetValue('j');
    end;

    [Test]
    procedure TrapRun_ControlWithAll_NotSet()
    var
        Card: TestPage "Dev Endpoint Mixed Card";
    begin
        Card.Trap();
        Page.Run(Page::"Dev Endpoint Mixed Card");
        Card.InAll.SetValue('k');
        Card.Close();
    end;

    [Test]
    procedure TrapRun_ControlInOtherArea_NotSet()
    var
        Card: TestPage "Dev Endpoint Mixed Card";
    begin
        Card.Trap();
        Page.Run(Page::"Dev Endpoint Mixed Card");
        Card.InArea.SetValue('l');
        Card.Close();
    end;

    [Test]
    procedure RunUnhandled_NotSet()
    begin
        Page.Run(Page::"Dev Endpoint Mixed Card");
    end;
}

codeunit 50973 "Dev Endpoint Areas OnRun"
{
    Subtype = Test;

    var
        AtRun: Text;

    trigger OnRun()
    begin
        AtRun := ApplicationArea();
    end;

    [Test]
    procedure AfterOnRunRead()
    begin
        if AtRun <> '' then
            Error('areas %1', AtRun);
    end;
}
"#;

fn run(dev_endpoint_ui: bool) -> CorpusRuntime {
    CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions {
            dev_endpoint_ui,
            ..Default::default()
        },
    )
}

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a TestResult {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
}

/// What the default mode and the flag both answer: every test that opens no page, or reads only controls whose area list
/// has `All` (on the page for every area text, pages/application-area/W2AppAreas.al:42 AllCtl, 'found' at :194, :246),
/// or sets the areas before the page copies them.
const KEPT: [&str; 12] = [
    "OpenTestPage_NotSet",
    "SetsAreas",
    "ReadAreas_AfterSet",
    "ReadAreas_AfterSetEmpty",
    "OpenTestPage_AfterSet",
    "NoRead",
    "Mixed_ControlsWithAll_NotSet",
    "Mixed_ControlInOtherArea_AfterSet",
    "PartWithAll_NotSet",
    "PartWithAllInBasicGroup_TouchPart_AfterSet",
    "RunModalHandled_ControlWithAll_NotSet",
    "TrapRun_ControlWithAll_NotSet",
];

/// Under the flag, refused: `ApplicationArea()` before the test set the areas (also in a test codeunit's OnRun), and a
/// page reader whose answer the unknown areas decide (a control or part whose area list has no `All`, or that sits in a
/// group that states an area without `All`: a part there refuses when the page opens, before its triggers run), also on a
/// page whose OnInit sets the areas after the page copied them.
const REFUSED: [&str; 12] = [
    "ReadAreas_NotSet",
    "ReadAreas_SetInEarlierTest",
    "Mixed_ControlInOtherArea_NotSet",
    "Mixed_ControlWithoutArea_NotSet",
    "OnInitSets_ControlRead",
    "GroupInOtherArea_NotSet",
    "PartInOtherArea_NotSet",
    "PartWithAllInBasicGroup_TouchPart_NotSet",
    "PartWithAllInBasicGroup_OpenOnly_NotSet",
    "RunModalHandled_ControlInOtherArea_NotSet",
    "TrapRun_ControlInOtherArea_NotSet",
    "AfterOnRunRead",
];

/// Off, the runner models the Test Runner app: every test starts with the empty text, a test after one that set the
/// areas included (BaTpEnv.al:53-55; session-system/TestSessionApplicationAreaTestBoundary.al:53-59), so every page shows
/// every control and every test passes but the page run that no handler takes.
#[test]
fn off_every_test_starts_with_no_areas() {
    let rt = run(false);
    for r in rt.results().iter().filter(|r| r.test != "RunUnhandled_NotSet") {
        assert_eq!(r.outcome, Outcome::Pass, "{}", r.test);
    }
    assert_eq!(rt.results().len(), 25);
    assert_eq!(
        result(&rt, "RunUnhandled_NotSet").outcome,
        Outcome::Fail("Unhandled UI: Page 50974".into())
    );
}

/// On, a page run that no handler takes shows no control: it keeps the dev-endpoint answer it has on main.
#[test]
fn on_an_unhandled_page_run_keeps_the_dev_endpoint_answer() {
    assert_eq!(result(&run(true), "RunUnhandled_NotSet").outcome, Outcome::Pass);
}

#[test]
fn on_a_read_the_unknown_areas_decide_is_refused() {
    let rt = run(true);
    for test in REFUSED.into_iter().filter(|t| *t != "AfterOnRunRead") {
        let r = result(&rt, test);
        let site = if test.starts_with("ReadAreas") {
            "ApplicationArea() under --dev-endpoint-ui"
        } else {
            "a page control whose ApplicationArea list has no All"
        };
        assert!(
            matches!(&r.outcome, Outcome::NotRun(m) if m.contains(site)),
            "{test}: {:?}",
            r.outcome
        );
    }
    // The read in OnRun refuses the test codeunit's run.
    assert!(matches!(&result(&rt, "AfterOnRunRead").outcome, Outcome::NotRun(_)));
}

#[test]
fn on_a_read_the_unknown_areas_cannot_change_keeps_the_result_off() {
    let (off, on) = (run(false), run(true));
    for test in KEPT {
        assert_eq!(result(&on, test).outcome, result(&off, test).outcome, "{test}");
    }
}
