//! A page run by `Page.Run` and answered by a `[PageHandler]`: how its close ends when a close trigger or the save of its
//! row fails (crates/al-vm/src/pages/page_run_close.rs; corpus pages/close-errors/CemPageHandlerTests.al, al-runner issue
//! 7480). The pinned cells are the corpus's. The pages here have the shape of the corpus's pages (CemFixtures.al:185-470);
//! each test flips one fact of a pinned cell (the page's code or properties, the run, the row, the handler's calls, the
//! error source) and holds the outcome to the one `origin/main` gives, or reaches a refusal of the module.

use al_test_cli::report::Outcome;

/// The pages of the pins, with places a test fills to change one fact (`Shape`).
const FIXTURE: &str = r#"
table 50100 "RV Row"
{
    /*TABLE-PROPS*/
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
    trigger OnInsert() var Cfg: Codeunit "RV Cfg"; begin Cfg.Add('T-ins ' + Rec.Name); end;
    trigger OnModify()
    var Cfg: Codeunit "RV Cfg";
    begin
        Cfg.Add('T-mod ' + Rec.Name);
        if Rec.Name = 'FAILMOD' then Error('RV table modify refused');
    end;
    /*TABLE*/
}
table 50101 "RV Other"
{
    fields { field(1; "Code"; Code[10]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}
codeunit 50102 "RV Cfg"
{
    SingleInstance = true;
    var Source: Text; Lines: Text; HMode: Integer;
    procedure Set(S: Text; M: Integer) begin Source := S; HMode := M; Lines := ''; end;
    procedure Mode(): Integer begin exit(HMode); end;
    procedure Has(T: Text): Boolean begin exit(StrPos(Source, T) > 0); end;
    procedure Add(L: Text) begin Lines += L + '; '; end;
    procedure Peek(): Text begin exit(Lines); end;
    procedure Touch() var O: Record "RV Other"; begin O.Code := CopyStr(Format(O.Count()), 1, 10); O.Insert(); end;
    procedure Query(CloseAction: Action): Boolean
    var R: Record "RV Row";
    begin
        Add('Q ' + Format(CloseAction));
        if Has('q-commit') then begin Commit(); Error('RV query after commit'); end;
        if Has('q-error') then Error('RV query error');
        if Has('q-empty') then Error('');
        if Has('q-veto') then exit(false);
        if Has('q-confirm') then if Confirm('RV query confirm?') then;
        if Has('q-write-error') then begin Touch(); Error('RV query error after write'); end;
        if Has('q-testfield') then begin R.Init(); R.TestField(Name); end;
        exit(true);
    end;
    procedure Closed()
    var R: Record "RV Row"; EI: ErrorInfo; N: Integer;
    begin
        Add('C');
        if Has('c-commit') then begin Commit(); Error('RV close after commit'); end;
        if Has('c-error') then Error('RV close error');
        if Has('c-empty') then Error('');
        if Has('c-message') then Message('RV close message');
        if Has('c-strmenu') then N := StrMenu('A,B', 1, 'RV menu');
        if Has('c-testfield') then begin R.Init(); R.TestField(Name); end;
        if Has('c-einfo') then begin EI.Message := 'RV errorinfo'; Error(EI); end;
        if Has('c-confirm') then if Confirm('RV close confirm?') then;
        if Has('c-write-only') then Touch();
        if Has('c-write-confirm') then begin Touch(); if Confirm('RV close confirm?') then; end;
        Add('C-done');
    end;
    procedure ModRec(NameValue: Text)
    var R: Record "RV Row";
    begin
        Add('M ' + NameValue);
        if Has('m-error') then Error('RV modify error');
        if Has('m-empty') then Error('');
        if Has('m-testfield') then begin R.Init(); R.TestField(Name); end;
        if Has('m-write-error') then begin Touch(); Error('RV modify error after write'); end;
        if Has('m-write-only') then Touch();
    end;
    procedure ModVar(var R: Record "RV Row")
    begin
        Add('MV ' + R.Name);
        if Has('m-var') then R.Name := 'CHANGED';
        if Has('m-commit') then Commit();
    end;
    procedure InsRec(NameValue: Text) begin Add('I ' + NameValue); end;
}
page 50103 "RV Card"
{
    PageType = Card;
    SourceTable = "RV Row";
    ApplicationArea = All;
    UsageCategory = None;
    /*CARD-PROPS*/
    layout
    {
        area(Content)
        {
            field("Code"; Rec."Code") { ApplicationArea = All; }
            field(Name; Rec.Name) { ApplicationArea = All; /*CARD-NAME*/ }
        }
    }
    /*CARD-ACTIONS*/
    var Cfg: Codeunit "RV Cfg";
    trigger OnModifyRecord(): Boolean begin /*CARD-MODIFY-CALL*/ /*CARD-MODIFY*/ exit(true); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Cfg.InsRec(Rec.Name); exit(true); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Cfg.Query(CloseAction)); end;
    trigger OnClosePage() begin Cfg.Closed(); /*CARD-CLOSE*/ end;
    /*CARD-TRIGGERS*/
}
page 50107 "RV List"
{
    PageType = List;
    SourceTable = "RV Row";
    ApplicationArea = All;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
        }
    }
    var Cfg: Codeunit "RV Cfg";
    trigger OnModifyRecord(): Boolean begin Cfg.ModRec(Rec.Name); exit(true); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Cfg.InsRec(Rec.Name); exit(true); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Cfg.Query(CloseAction)); end;
    trigger OnClosePage() begin Cfg.Closed(); end;
}
page 50108 "RV Wizard"
{
    PageType = NavigatePage;
    SourceTable = "RV Row";
    ApplicationArea = All;
    UsageCategory = None;
    layout { area(Content) { field(Name; Rec.Name) { ApplicationArea = All; } } }
    actions
    {
        area(Processing)
        {
            action(ActionNext)
            {
                ApplicationArea = All;
                InFooterBar = true;
                trigger OnAction() var Cfg: Codeunit "RV Cfg"; begin Cfg.Add('A-next'); end;
            }
        }
    }
    var Cfg: Codeunit "RV Cfg";
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Cfg.Query(CloseAction)); end;
    trigger OnClosePage() begin Cfg.Closed(); end;
}
page 50104 "RV Worksheet"
{
    PageType = Worksheet;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(R) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
    var Cfg: Codeunit "RV Cfg";
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Cfg.Query(CloseAction)); end;
    trigger OnClosePage() begin Cfg.Closed(); end;
}
page 50109 "RV Host"
{
    PageType = Card;
    ApplicationArea = All;
    UsageCategory = None;
    layout { area(Content) { } }
    actions
    {
        area(Processing)
        {
            action(RunCard)
            {
                ApplicationArea = All;
                trigger OnAction() var R: Record "RV Row"; begin R.Get('R1'); R.SetRange("Code", 'R1'); Page.Run(Page::"RV Card", R); end;
            }
        }
    }
}
/*OBJECTS*/
codeunit 50110 "RV Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var Cfg: Codeunit "RV Cfg";
    [Test] [HandlerFunctions('/*HANDLERS*/')]
    procedure T()
    var R: Record "RV Row"; O: Record "RV Other"; Res: Text;
    begin
        R.DeleteAll(); O.DeleteAll();
        R.Code := 'R1'; R.Name := 'one'; R.Insert();
        if '/*RUN*/' = 'card-empty' then R.DeleteAll();
        if '/*RUN*/' <> 'card-uncommitted' then Commit();
        if '/*RUN*/' = 'card-pretouch' then Cfg.Touch();
        Cfg.Set('/*SOURCE*/', /*MODE*/);
        R.SetRange("Code", 'R1');
        if TryRun(R) then Res := 'returned' else Res := 'error=' + GetLastErrorCode() + '|' + GetLastErrorText();
        R.Reset();
        if R.FindFirst() then;
        Error('OBS %1 log=%2 others=%3 name=%4', Res, Cfg.Peek(), O.Count(), R.Name);
    end;
    [TryFunction]
    local procedure TryRun(var R: Record "RV Row")
    var C: Page "RV Card"; L: Page "RV List"; W: Page "RV Wizard"; HP: TestPage "RV Host";
    begin
        case '/*RUN*/' of
            'card', 'card-empty', 'card-uncommitted', 'card-pretouch': Page.Run(Page::"RV Card", R);
            'card-field': Page.Run(Page::"RV Card", R, R.Name);
            'card-id0': Page.Run(0, R);
            'card-var-tableview': begin C.SetTableView(R); C.Run(); end;
            'card-var-both': begin C.SetTableView(R); C.SetRecord(R); C.Run(); end;
            'card-var-editable': begin C.SetRecord(R); C.Editable(false); C.Run(); end;
            'host-action': begin HP.OpenEdit(); HP.RunCard.Invoke(); HP.Close(); end;
            'card-no-record': Page.Run(Page::"RV Card");
            'card-no-filter': begin R.Reset(); R.Get('R1'); Page.Run(Page::"RV Card", R); end;
            'card-two-filters': begin R.SetRange(Name, 'one'); Page.Run(Page::"RV Card", R); end;
            'card-no-row': begin R.SetRange("Code", 'ZZ'); Page.Run(Page::"RV Card", R); end;
            'card-var': begin C.SetRecord(R); C.Run(); end;
            'card-var-no-record': C.Run();
            'list': Page.Run(Page::"RV List", R);
            'list-var': begin L.SetRecord(R); L.Run(); end;
            'wizard': begin W.SetRecord(R); W.Run(); end;
            'wizard-static': Page.Run(Page::"RV Wizard", R);
            'worksheet': Page.Run(Page::"RV Worksheet", R);
        end;
    end;
    [TryFunction] local procedure TryFail() begin Error('RV caught'); end;
    [PageHandler] procedure CardH(var P: TestPage "RV Card")
    begin
        Cfg.Add('H');
        case Cfg.Mode() of
            1: P.OK().Invoke();
            2: Error('RV handler error');
            3: P.Cancel().Invoke();
            5: P.Close();
            6: P.Name.SetValue('EDIT');
            7: begin P.Name.SetValue('EDIT'); P.OK().Invoke(); end;
            8: begin P.Name.SetValue('EDIT'); Error('RV handler error'); end;
            9: P.Name.SetValue('FAILMOD');
            10: begin P.New(); P."Code".SetValue('N1'); P.Name.SetValue('NEW'); end;
            11: if P.Name.Value() = '' then;
            12: begin P.Name.SetValue('A'); P.Name.SetValue('EDIT'); end;
            13: if TryFail() then;
            14: Error('');
            15: begin if TryFail() then; Error('RV handler error'); end;
            16: begin Cfg.Touch(); Error('RV handler error'); end;
            17: Cfg.Touch();
            18: begin P.First(); P.OK().Invoke(); end;
            45: Commit();
            46: begin Commit(); Error('RV handler error'); end;
            /*CARD-MODES*/
        end;
    end;
    [PageHandler] procedure ListH(var P: TestPage "RV List")
    begin
        Cfg.Add('H');
        case Cfg.Mode() of
            1: P.OK().Invoke();
            2: Error('RV handler error');
            3: P.Cancel().Invoke();
            6: P.Name.SetValue('EDIT');
        end;
    end;
    [PageHandler] procedure WizH(var P: TestPage "RV Wizard")
    begin
        Cfg.Add('H');
        case Cfg.Mode() of
            1: P.OK().Invoke();
            2: Error('RV handler error');
            3: P.Cancel().Invoke();
            5: P.Close();
            20: begin P.ActionNext.Invoke(); P.ActionNext.Invoke(); end;
            21: begin P.ActionNext.Invoke(); P.OK().Invoke(); end;
            22: begin P.ActionNext.Invoke(); Error('RV handler error'); end;
        end;
    end;
    [PageHandler] procedure WsH(var P: TestPage "RV Worksheet") begin Cfg.Add('H'); end;
    [MessageHandler] procedure MsgH(M: Text[1024]) begin Cfg.Add('MSG'); end;
    [ConfirmHandler] procedure ConfH(Q: Text[1024]; var Reply: Boolean) begin Cfg.Add('CONF'); Reply := true; end;
}
"#;

/// What a test changes in the pinned card and its table; empty: the shape of the pins.
#[derive(Default, Clone, Copy)]
struct Shape {
    /// Page properties of the card.
    props: &'static str,
    /// Properties and triggers of the card's Name control.
    name: &'static str,
    /// An `actions` section of the card.
    actions: &'static str,
    /// Statements of OnModifyRecord before its `exit(true)`.
    modify: &'static str,
    /// Statements of OnClosePage after its call.
    close: &'static str,
    /// More triggers of the card.
    triggers: &'static str,
    /// More code of the table.
    table: &'static str,
    /// More modes of the card's handler.
    modes: &'static str,
    /// More objects.
    objects: &'static str,
    /// The call of OnModifyRecord; empty: the pins' `Cfg.ModRec(Rec.Name);`.
    modify_call: &'static str,
    /// Properties of the table.
    table_props: &'static str,
}

const PINNED: Shape = Shape {
    props: "",
    name: "",
    actions: "",
    modify: "",
    close: "",
    triggers: "",
    table: "",
    modes: "",
    objects: "",
    modify_call: "",
    table_props: "",
};

/// The outcome of the test: `run` the page (a case of `TryRun`), with the error `source` switched on and the handler in `mode`.
fn outcome(shape: Shape, handlers: &str, run: &str, source: &str, mode: u32) -> Outcome {
    let modify_call = match shape.modify_call {
        "" => "Cfg.ModRec(Rec.Name);",
        call => call,
    };
    let src = FIXTURE
        .replace("/*TABLE-PROPS*/", shape.table_props)
        .replace("/*CARD-MODIFY-CALL*/", modify_call)
        .replace("/*CARD-PROPS*/", shape.props)
        .replace("/*CARD-NAME*/", shape.name)
        .replace("/*CARD-ACTIONS*/", shape.actions)
        .replace("/*CARD-MODIFY*/", shape.modify)
        .replace("/*CARD-CLOSE*/", shape.close)
        .replace("/*CARD-TRIGGERS*/", shape.triggers)
        .replace("/*TABLE*/", shape.table)
        .replace("/*CARD-MODES*/", shape.modes)
        .replace("/*OBJECTS*/", shape.objects)
        .replace("/*HANDLERS*/", handlers)
        .replace("/*RUN*/", run)
        .replace("/*SOURCE*/", source)
        .replace("/*MODE*/", &mode.to_string());
    let o = al_testkit::outcomes_in(&src, "RV Tests");
    assert_eq!(o.len(), 1, "{o:?}");
    o[0].1.clone()
}

/// The pinned card run by `Page.Run(Id, Rec)`.
fn card(source: &str, mode: u32) -> Outcome {
    outcome(PINNED, "CardH", "card", source, mode)
}

fn obs(text: &str) -> Outcome {
    Outcome::Fail(format!("OBS {text}"))
}

fn refused_with(o: &Outcome, reason: &str) {
    match o {
        Outcome::NotRun(why) => assert!(why.contains(reason), "{why}"),
        other => panic!("expected a refusal with <{reason}>, got {other:?}"),
    }
}

/// main's refusal of an error of OnClosePage at a close by an action or the platform.
const ON_CLOSE_PAGE_REFUSED: &str =
    "an error in OnClosePage at a close by an action or a handler while the page closes";
/// main's refusal of a page that stays open at the platform's close.
const VETO_REFUSED: &str = "a page whose OnQueryClosePage refused the platform's close";
/// main's refusal of a failed save at the platform's close of a page that is not modal.
const SAVE_REFUSED: &str =
    "an error raised while the page saved its edited row as the platform closed it after the handler returned";
/// The module's refusal after a handler error.
const HANDLER_ERROR_CLOSE_REFUSED: &str = "a page left open by a PageHandler that raised an error, whose close then";

// The pinned cells on the pages of the pinned shape (CemPageHandlerTests.al:469 CardStaticRun_HandlerReturns_ClosePageError,
// :578 _HandlerReturns_ClosePageConfirmUnhandled, :250 _HandlerReturns_QueryCloseError, :386 _HandlerEditReturns_QueryCloseError,
// :606 _HandlerOk_ClosePageConfirmUnhandled, :278 _HandlerOk_QueryCloseError, :551 _HandlerEditOk_ClosePageError, :687 _HandlerEditReturns_PageModifyRecordError,
// :222 _HandlerTableRefusesReturns_NoError, :139 _HandlerError_NoError, :84 _HandlerCancel_NoError, :359
// _HandlerError_QueryCloseError, :633 _HandlerClose_ClosePageConfirmUnhandled).
#[test]
fn the_pinned_cells_run_on_a_card_of_the_pinned_shape() {
    assert_eq!(card("c-error", 0), obs("returned log=H; Q OK; C;  others=0 name=one"));
    assert_eq!(card("c-confirm", 0), obs("returned log=H; Q OK; C;  others=0 name=one"));
    assert_eq!(card("q-error", 0), obs("returned log=H; Q OK;  others=0 name=one"));
    assert_eq!(
        card("q-error", 6),
        obs("returned log=H; M EDIT; T-mod EDIT; Q OK;  others=0 name=EDIT")
    );
    assert_eq!(
        card("c-confirm", 1),
        obs(
            "error=TestWrapped|One or more errors occurred while closing the page.\\\\Unhandled UI: Confirm RV close confirm? log=H; Q OK; C;  others=0 name=one"
        )
    );
    assert_eq!(
        card("q-error", 1),
        obs("error=TestWrapped:MissingUIHandler|Unhandled UI: Message RV query error log=H; Q OK;  others=0 name=one")
    );
    assert_eq!(
        card("c-error", 7),
        obs(
            "error=TestWrapped|One or more errors occurred while closing the page.\\\\RV close error log=H; M EDIT; T-mod EDIT; Q OK; C;  others=0 name=EDIT"
        )
    );
    assert_eq!(card("m-error", 6), obs("returned log=H; M EDIT;  others=0 name=one"));
    assert_eq!(
        card("none", 9),
        obs("returned log=H; M FAILMOD; T-mod FAILMOD;  others=0 name=one")
    );
    assert_eq!(
        card("none", 2),
        obs("error=Dialog|RV handler error log=H; Q OK; C; C-done;  others=0 name=one")
    );
    assert_eq!(
        card("none", 3),
        obs(
            "error=TestActionNotFound|The built-in action = Cancel is not found on the page. log=H; Q OK; C; C-done;  others=0 name=one"
        )
    );
    assert_eq!(
        card("q-error", 2),
        obs("error=Dialog|RV handler error log=H; Q OK;  others=0 name=one")
    );
    assert_eq!(
        card("c-confirm", 5),
        obs(
            "error=TestWrapped|One or more errors occurred while closing the page.\\\\Unhandled UI: Confirm RV close confirm? log=H; Q OK; C;  others=0 name=one"
        )
    );
}

// The other pinned runs (:993 CardVariableRun_HandlerReturns_ClosePageError, :1323 ListStaticRun_HandlerReturns_ClosePageError,
// :1296 ListStaticRun_HandlerError_QueryCloseError, :1681 WizardRun_HandlerReturns_ClosePageConfirmUnhandled, :1790
// WizardRun_HandlerError_ClosePageConfirmUnhandled, :1736 WizardRun_HandlerCancel_ClosePageConfirmUnhandled).
#[test]
fn the_pinned_cells_run_on_the_other_pinned_runs() {
    let returned = obs("returned log=H; Q OK; C;  others=0 name=one");
    assert_eq!(outcome(PINNED, "CardH", "card-var", "c-error", 0), returned);
    assert_eq!(outcome(PINNED, "ListH", "list", "c-error", 0), returned);
    assert_eq!(
        outcome(PINNED, "ListH", "list", "q-error", 2),
        obs("error=Dialog|RV handler error log=H; Q OK;  others=0 name=one")
    );
    assert_eq!(outcome(PINNED, "WizH", "wizard", "c-confirm", 0), returned);
    assert_eq!(
        outcome(PINNED, "WizH", "wizard", "c-confirm", 2),
        obs("error=Dialog|RV handler error log=H; Q OK; C;  others=0 name=one")
    );
    assert_eq!(
        outcome(PINNED, "WizH", "wizard", "c-confirm", 3),
        obs(
            "error=TestActionNotFound|The built-in action = Cancel is not found on the page. log=H; Q OK; C;  others=0 name=one"
        )
    );
}

const READ_ONLY: Shape = Shape {
    props: "Editable = false;",
    ..PINNED
};
const OPEN_TRIGGER: Shape = Shape {
    triggers: "trigger OnOpenPage() begin Cfg.Add('O'); end;",
    ..PINNED
};
const CLOSE_BODY: Shape = Shape {
    close: "Cfg.Add('C2');",
    ..PINNED
};
const CONTROL_TRIGGER: Shape = Shape {
    name: "trigger OnValidate() begin Cfg.Add('V'); end;",
    ..PINNED
};
const ACTION: Shape = Shape {
    actions: "actions { area(Processing) { action(Noop) { ApplicationArea = All; trigger OnAction() begin Cfg.Add('A-noop'); end; } } }",
    modes: "30: P.Noop.Invoke(); 31: begin P.Noop.Invoke(); Error('RV handler error'); end;",
    ..PINNED
};
const TABLE_PROCEDURE: Shape = Shape {
    table: "procedure Extra() begin end;",
    ..PINNED
};
const SUBSCRIBER: Shape = Shape {
    objects: "codeunit 50120 \"RV Sub\" { [EventSubscriber(ObjectType::Page, Page::\"RV Card\", OnClosePageEvent, '', false, false)] local procedure S(var Rec: Record \"RV Row\") begin end; }",
    ..PINNED
};
const MODIFY_FALSE: Shape = Shape {
    modify: "if Cfg.Has('m-false') then exit(false);",
    ..PINNED
};

/// An expected refusal that holds `reason`, for `main_gives`.
fn refused(reason: &str) -> String {
    format!("!{reason}")
}

/// The scenario ends as it does on `origin/main` (measured with the main binary 2bd87774f, BC 27): the observation
/// `expected`, or with a leading `!` a refusal that holds the rest.
fn main_gives(shape: Shape, handlers: &str, run: &str, source: &str, mode: u32, expected: &str) {
    let o = outcome(shape, handlers, run, source, mode);
    match expected.strip_prefix('!') {
        Some(reason) => refused_with(&o, reason),
        None => assert_eq!(o, obs(expected), "{run} {source} {mode}"),
    }
}

// Review round 2, MED 1: the row. Every pin runs the page on a stored row under the cursor (CemPageHandlerTests.al:28-34
// `Row.Get('R1')`). An empty table, a record filtered to no row and a new row keep main's outcome.
#[test]
fn a_page_with_no_stored_row_under_the_cursor_keeps_mains_outcome() {
    main_gives(
        PINNED,
        "CardH",
        "card-empty",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card-empty",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card-empty",
        "q-error",
        0,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; Q OK;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card-empty",
        "c-error",
        1,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card-no-row",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card-no-row",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "c-error",
        10,
        &refused(
            "a page inserted a new row outside its filters on SetValue of a field with validation code, or on a page other than a list",
        ),
    );
}

// Review round 2, MED 2: the page's own code and properties (CemFixtures.al:185-225 page CEM Card). A property, another
// trigger, another trigger body, a control trigger, an action, a table procedure or a subscriber keeps main's outcome.
#[test]
fn a_page_with_other_code_or_properties_keeps_mains_outcome() {
    main_gives(
        READ_ONLY,
        "CardH",
        "card",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        READ_ONLY,
        "CardH",
        "card",
        "q-error",
        1,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; Q OK;  others=0 name=one",
    );
    main_gives(
        READ_ONLY,
        "CardH",
        "card",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
    main_gives(
        OPEN_TRIGGER,
        "CardH",
        "card",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        OPEN_TRIGGER,
        "CardH",
        "card",
        "none",
        2,
        "error=Dialog|RV handler error log=O; H;  others=0 name=one",
    );
    main_gives(
        CLOSE_BODY,
        "CardH",
        "card",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        CLOSE_BODY,
        "CardH",
        "card",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
    main_gives(
        CONTROL_TRIGGER,
        "CardH",
        "card",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(ACTION, "CardH", "card", "c-error", 0, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        ACTION,
        "CardH",
        "card",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
    main_gives(
        TABLE_PROCEDURE,
        "CardH",
        "card",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        SUBSCRIBER,
        "CardH",
        "card",
        "c-confirm",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        SUBSCRIBER,
        "CardH",
        "card",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
}

// Review round 2, MED 3: the error. Only `Error` with a text is pinned (CemFixtures.al:153, :163, :174); `Error('')` in a
// close trigger, in the save or in the handler keeps main's outcome.
#[test]
fn an_error_without_a_text_keeps_mains_outcome() {
    main_gives(
        PINNED,
        "CardH",
        "card",
        "q-empty",
        0,
        "error=MissingUIHandler|Unhandled UI: Message  log=H; Q OK;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "q-empty",
        1,
        "error=MissingUIHandler|Unhandled UI: Message  log=H; Q OK;  others=0 name=one",
    );
    main_gives(PINNED, "CardH", "card", "c-empty", 0, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(PINNED, "CardH", "card", "c-empty", 1, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(PINNED, "CardH", "card", "m-empty", 6, &refused(SAVE_REFUSED));
    main_gives(
        PINNED,
        "CardH",
        "card",
        "none",
        14,
        "error=Dialog| log=H;  others=0 name=one",
    );
}

// Review round 2, MED 4: the save. In every pinned cell with a typed value and a close-trigger error the close saved the row
// first (CemPageHandlerTests.al:386 `M-card EDIT; T-mod R1 EDIT; Q-card OK;`). A save OnModifyRecord skipped, and a save that
// wrote more than the row, keep main's outcome.
#[test]
fn a_save_that_did_not_write_exactly_the_row_keeps_mains_outcome() {
    main_gives(
        MODIFY_FALSE,
        "CardH",
        "card",
        "m-false q-error",
        6,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; M EDIT; Q OK;  others=0 name=one",
    );
    main_gives(
        MODIFY_FALSE,
        "CardH",
        "card",
        "m-false c-error",
        7,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "m-write-only q-error",
        6,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; M EDIT; T-mod EDIT; Q OK;  others=1 name=EDIT",
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "m-write-only c-error",
        7,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
}

// Review round 2, MED 5: the handler's calls and the record. The pinned handlers do nothing, type Name, invoke OK or Cancel,
// call Close or raise an error (CemPageHandlerTests.al:1814-1941), and every run gets the record (CemFixtures.al:476-500).
// A page action invoked first (the wizard of al-runner issue 7480 included), a read, two typed values, a caught error, a write
// by the handler, another call before OK and a run without the record keep main's outcome.
#[test]
fn other_handler_calls_and_a_run_without_the_record_keep_mains_outcome() {
    main_gives(
        PINNED,
        "WizH",
        "wizard",
        "c-confirm",
        20,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(PINNED, "WizH", "wizard", "c-error", 21, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        PINNED,
        "WizH",
        "wizard",
        "q-error",
        22,
        "error=Dialog|RV handler error log=H; A-next;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "WizH",
        "wizard",
        "none",
        22,
        "error=Dialog|RV handler error log=H; A-next;  others=0 name=one",
    );
    main_gives(PINNED, "CardH", "card", "c-error", 11, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(PINNED, "CardH", "card", "c-error", 12, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(PINNED, "CardH", "card", "c-error", 13, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        PINNED,
        "CardH",
        "card",
        "none",
        15,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "none",
        16,
        "error=Dialog|RV handler error log=H;  others=1 name=one",
    );
    main_gives(PINNED, "CardH", "card", "c-error", 17, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(PINNED, "CardH", "card", "c-error", 18, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        PINNED,
        "CardH",
        "card-no-record",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card-var-no-record",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(ACTION, "CardH", "card", "c-error", 30, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        ACTION,
        "CardH",
        "card",
        "q-error",
        31,
        "error=Dialog|RV handler error log=H; A-noop;  others=0 name=one",
    );
}

// Other error sources than the pinned ones (an `Error` with a text, a Confirm no handler takes in OnClosePage): a veto, an
// unhandled Confirm in OnQueryClosePage, a TestField error, `Error(ErrorInfo)`, an unhandled Message or StrMenu and an error
// after a write keep main's outcome.
#[test]
fn other_error_sources_keep_mains_outcome() {
    main_gives(PINNED, "CardH", "card", "q-veto", 0, &refused(VETO_REFUSED));
    main_gives(PINNED, "CardH", "card", "q-veto", 1, &refused(VETO_REFUSED));
    main_gives(
        PINNED,
        "CardH",
        "card",
        "q-confirm",
        0,
        "error=MissingUIHandler|Unhandled UI: Message Unhandled UI: Confirm RV query confirm? log=H; Q OK;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "q-confirm",
        1,
        "error=MissingUIHandler|Unhandled UI: Message Unhandled UI: Confirm RV query confirm? log=H; Q OK;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "q-testfield",
        0,
        "error=MissingUIHandler|Unhandled UI: Message Name must have a value in RV Row: Code=. It cannot be zero or empty. log=H; Q OK;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "q-write-error",
        0,
        "error=MissingUIHandler|Unhandled UI: Message RV query error after write log=H; Q OK;  others=1 name=one",
    );
    main_gives(PINNED, "CardH", "card", "c-message", 0, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(PINNED, "CardH", "card", "c-message", 1, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        PINNED,
        "CardH",
        "card",
        "c-message",
        5,
        &refused("GetLastErrorCode of an error whose text is not pinned"),
    );
    main_gives(PINNED, "CardH", "card", "c-strmenu", 0, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        PINNED,
        "CardH",
        "card",
        "c-testfield",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(PINNED, "CardH", "card", "c-einfo", 0, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        PINNED,
        "CardH",
        "card",
        "c-write-confirm",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
}

// Other pages, runs and rows than the cells measure keep main's outcome: a Worksheet, a List by a Page variable, a NavigatePage
// by `Page.Run(Id, Rec)`, a handler error on a Card by a Page variable, a failed save on a Card by a Page variable or a List,
// a typed value on a List, Cancel on a List, a typed value before the handler's error.
#[test]
fn other_pages_runs_and_rows_keep_mains_outcome() {
    main_gives(
        PINNED,
        "WsH",
        "worksheet",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "ListH",
        "list-var",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "WizH",
        "wizard-static",
        "c-confirm",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card-var",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
    main_gives(PINNED, "CardH", "card-var", "m-error", 6, &refused(SAVE_REFUSED));
    main_gives(PINNED, "ListH", "list", "m-error", 6, &refused(SAVE_REFUSED));
    main_gives(
        PINNED,
        "ListH",
        "list",
        "q-error",
        6,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; M EDIT; T-mod EDIT; Q OK;  others=0 name=EDIT",
    );
    main_gives(
        PINNED,
        "ListH",
        "list",
        "none",
        3,
        "error=TestActionNotFound|The built-in action = Cancel is not found on the page. log=H;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card",
        "none",
        8,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
}

// No pin binds a MessageHandler or a ConfirmHandler (every test of CemPageHandlerTests.al binds only its PageHandler): with
// one bound, main's outcome stays.
#[test]
fn a_bound_message_or_confirm_handler_keeps_mains_outcome() {
    main_gives(PINNED, "CardH,MsgH", "card", "q-error", 0, &refused(VETO_REFUSED));
    main_gives(PINNED, "CardH,MsgH", "card", "q-error", 1, &refused(VETO_REFUSED));
    main_gives(
        PINNED,
        "CardH,MsgH",
        "card",
        "c-message",
        0,
        "returned log=H; Q OK; C; MSG; C-done;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH,ConfH",
        "card",
        "c-confirm",
        0,
        "returned log=H; Q OK; C; CONF; C-done;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH,ConfH",
        "card",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH,MsgH",
        "card",
        "c-error",
        1,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH,MsgH",
        "card",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
}

// The record of the run: every pin gives the row with one filter, on its key (CemPageHandlerTests.al:33-34 `Row.Get('R1');
// Row.SetRange("Code", 'R1')`). A record without a filter or with a second one keeps main's outcome.
#[test]
fn a_record_with_another_view_keeps_mains_outcome() {
    main_gives(
        PINNED,
        "CardH",
        "card-no-filter",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card-no-filter",
        "none",
        2,
        "error=Dialog|RV handler error log=H;  others=0 name=one",
    );
    main_gives(
        PINNED,
        "CardH",
        "card-two-filters",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card-two-filters",
        "q-error",
        1,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; Q OK;  others=0 name=one",
    );
}

// Other errors of the save at the platform's close than an `Error` with a text before any write keep main's refusal.
#[test]
fn other_save_errors_keep_mains_refusal() {
    main_gives(PINNED, "CardH", "card", "m-testfield", 6, &refused(SAVE_REFUSED));
    main_gives(PINNED, "CardH", "card", "m-write-error", 6, &refused(SAVE_REFUSED));
}
// The handler raised its error on a page and run of the pins: the platform asks the close, as BC does
// (CemPageHandlerTests.al:139 CardStaticRun_HandlerError_NoError `log` `Q-card OK; C-card;`). main asked no close there. A
// close that then fails outside the pinned cells, is refused by OnQueryClosePage or writes is not pinned: refused.
#[test]
fn a_handler_error_then_an_unpinned_close_on_a_pinned_page_is_refused() {
    refused_with(&card("c-error", 2), ON_CLOSE_PAGE_REFUSED);
    refused_with(&card("c-message", 2), ON_CLOSE_PAGE_REFUSED);
    refused_with(&card("q-veto", 2), HANDLER_ERROR_CLOSE_REFUSED);
    refused_with(&card("q-confirm", 2), HANDLER_ERROR_CLOSE_REFUSED);
    refused_with(&card("c-write-only", 2), HANDLER_ERROR_CLOSE_REFUSED);
}

// The save of the typed value fails at OK in the handler of the pinned card: BC logs neither close trigger and `Page.Run`
// returns (CemPageHandlerTests.al:715 CardStaticRun_HandlerEditOk_PageModifyRecordError `returned` `M-card EDIT;`). main
// runs both close triggers after it; the cell is refused until that close is built.
#[test]
fn a_save_error_at_ok_in_the_handler_of_the_pinned_card_is_refused() {
    refused_with(
        &card("m-error", 7),
        "an error raised while the page saved its edited row at OK in a PageHandler of Page.Run",
    );
}

// The shape of al-runner issue 7480 as the reporter wrote it: a NavigatePage run by its Page variable, the handler clicks
// Next twice and returns, and OnClosePage asks a Confirm no handler takes. No pinned handler invokes a page action
// (CemPageHandlerTests.al:1918-1941 `CemWizardRun`), and the page's own code is not the pinned page's: main's refusal stays
// (probe wanted: docs/probes-needed.md).
#[test]
fn the_reporters_wizard_keeps_mains_refusal() {
    let src = r#"
table 50100 "RV Row"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Finished; Boolean) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}
page 50108 "RV Wizard"
{
    PageType = NavigatePage;
    SourceTable = "RV Row";
    layout { area(Content) { field("Code"; Rec."Code") { } } }
    actions { area(Processing) { action(ActionNext) { InFooterBar = true; trigger OnAction() begin Steps += 1; end; } } }
    var Steps: Integer; SaveRecord: Boolean;
    trigger OnClosePage()
    begin
        if not SaveRecord and not Rec.Finished then
            if not Confirm('Do you want to finish this simulation later?') then
                Rec.Delete(true);
    end;
}
codeunit 50111 "RV Wizard Tests"
{
    Subtype = Test;
    [Test] [HandlerFunctions('WizH')]
    procedure T()
    var R: Record "RV Row"; W: Page "RV Wizard";
    begin
        R.DeleteAll(); R.Code := 'R1'; R.Insert();
        W.Run();
    end;
    [PageHandler] procedure WizH(var P: TestPage "RV Wizard") begin P.ActionNext.Invoke(); P.ActionNext.Invoke(); end;
}
"#;
    let o = al_testkit::outcomes_in(src, "RV Wizard Tests");
    assert_eq!(o.len(), 1, "{o:?}");
    refused_with(&o[0].1, ON_CLOSE_PAGE_REFUSED);
}

const MOD_VAR: Shape = Shape {
    modify_call: "Cfg.ModVar(Rec);",
    ..PINNED
};

// Review round 3, MED 1: the transaction. Every pin commits before the run (CemFixtures.al:549 `Commit()` in `Setup`,
// CemPageHandlerTests.al:33-37) and nothing commits after it. An uncommitted write before the run and a Commit in the
// handler, in a close trigger or in the codeunit the save calls keep main's outcome (measured with the main binary
// 2d19bd631, BC 27).
#[test]
fn an_uncommitted_write_before_the_run_or_a_commit_in_it_keeps_mains_outcome() {
    let query_edit = "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; M EDIT; T-mod EDIT; Q OK;  others=0 name=EDIT";
    let handler_error = "error=Dialog|RV handler error log=H;  others=0 name=one";
    main_gives(
        PINNED,
        "CardH",
        "card-uncommitted",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(PINNED, "CardH", "card-uncommitted", "q-error", 6, query_edit);
    main_gives(PINNED, "CardH", "card-uncommitted", "none", 2, handler_error);
    main_gives(
        PINNED,
        "CardH",
        "card-pretouch",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card-pretouch",
        "q-error",
        6,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; M EDIT; T-mod EDIT; Q OK;  others=1 name=EDIT",
    );
    main_gives(PINNED, "CardH", "card", "c-error", 45, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(PINNED, "CardH", "card", "none", 46, handler_error);
    main_gives(PINNED, "CardH", "card", "c-commit", 0, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(PINNED, "CardH", "card", "c-commit", 1, &refused(ON_CLOSE_PAGE_REFUSED));
    main_gives(
        PINNED,
        "CardH",
        "card",
        "q-commit",
        0,
        "error=MissingUIHandler|Unhandled UI: Message RV query after commit log=H; Q OK;  others=0 name=one",
    );
    // After a handler error the close triggers run before the Commit in OnQueryClosePage is seen: main's outcome (no close)
    // cannot be given any more, so it is refused as every close outside the cells after a handler error.
    refused_with(&card("q-commit", 2), HANDLER_ERROR_CLOSE_REFUSED);
    main_gives(
        MOD_VAR,
        "CardH",
        "card",
        "m-commit q-error",
        6,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; MV EDIT; T-mod EDIT; Q OK;  others=0 name=EDIT",
    );
}

// Review round 3, MED 2: how the run was made. The pins run the page by `Page.Run(Page::"CEM Card", Row)` with no field
// argument, or by `SetRecord` then `Run()` on a Page variable (CemFixtures.al:476-500), from a codeunit the test called
// (CemPageHandlerTests.al:36). SetTableView, Editable(false) on the variable, a field argument, page id 0 and a run from an
// action of a page the test opened keep main's outcome (main binary 2d19bd631, BC 27).
#[test]
fn a_run_made_another_way_keeps_mains_outcome() {
    let lookup_id = Shape {
        table_props: "LookupPageId = \"RV Card\";",
        ..PINNED
    };
    for run in [
        "card-var-tableview",
        "card-var-both",
        "card-var-editable",
        "card-field",
        "host-action",
    ] {
        main_gives(PINNED, "CardH", run, "c-error", 0, &refused(ON_CLOSE_PAGE_REFUSED));
    }
    main_gives(
        lookup_id,
        "CardH",
        "card-id0",
        "c-error",
        0,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        PINNED,
        "CardH",
        "card-field",
        "q-error",
        6,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; M EDIT; T-mod EDIT; Q OK;  others=0 name=EDIT",
    );
    main_gives(
        PINNED,
        "CardH",
        "host-action",
        "none",
        2,
        "error=TestWrapped:Dialog|RV handler error log=H;  others=0 name=one",
    );
}

// Review round 3, MED 3: what the save hands on. The pinned OnModifyRecord passes values only (CemFixtures.al:203
// `Cfg.ModRec('card', Rec.Name)`), so the saved value is the typed one. A call that passes Rec by var keeps main's outcome,
// with and without a change of the row in the codeunit (main binary 2d19bd631, BC 27).
#[test]
fn a_save_that_passes_rec_by_var_keeps_mains_outcome() {
    main_gives(
        MOD_VAR,
        "CardH",
        "card",
        "m-var q-error",
        6,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; MV EDIT; T-mod CHANGED; Q OK;  others=0 name=CHANGED",
    );
    main_gives(
        MOD_VAR,
        "CardH",
        "card",
        "m-var c-error",
        7,
        &refused(ON_CLOSE_PAGE_REFUSED),
    );
    main_gives(
        MOD_VAR,
        "CardH",
        "card",
        "q-error",
        6,
        "error=MissingUIHandler|Unhandled UI: Message RV query error log=H; MV EDIT; T-mod EDIT; Q OK;  others=0 name=EDIT",
    );
}
