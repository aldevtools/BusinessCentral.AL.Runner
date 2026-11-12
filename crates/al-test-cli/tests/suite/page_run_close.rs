//! A page run by `Page.Run` and answered by a `[PageHandler]`: how its close ends when a close trigger or the save of its
//! row fails (crates/al-vm/src/pages/page_run_close.rs; corpus pages/close-errors/CemPageHandlerTests.al, al-runner issue
//! 7480). The pinned cells are the corpus's; these tests hold the neighbors (another error source, another close, another
//! page or run) to main's outcome or to a refusal, and reach each refusal of the module.

use al_test_cli::report::Outcome;

const FIXTURE: &str = r#"
table 50100 "RV Row"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Name; Text[30]) { } }
    keys { key(PK; "Code") { Clustered = true; } }
    trigger OnModify() begin if Rec.Name = 'FAILMOD' then Error('RV table modify refused'); end;
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
    procedure Add(L: Text) begin Lines += L + '; '; end;
    procedure Peek(): Text begin exit(Lines); end;
    procedure Query(CloseAction: Action): Boolean
    var O: Record "RV Other"; R: Record "RV Row";
    begin
        Add('Q ' + Format(CloseAction));
        case Source of
            'q-error': Error('RV query error');
            'q-veto': exit(false);
            'q-confirm': if Confirm('RV query confirm?') then;
            'q-write-error': begin O.Code := 'W'; O.Insert(); Error('RV query error after write'); end;
            'q-testfield': begin R.Init(); R.TestField(Name); end;
        end;
        exit(true);
    end;
    procedure Closed()
    var O: Record "RV Other"; R: Record "RV Row"; EI: ErrorInfo; N: Integer;
    begin
        Add('C');
        case Source of
            'c-error': Error('RV close error');
            'c-message': Message('RV close message');
            'c-strmenu': N := StrMenu('A,B', 1, 'RV menu');
            'c-testfield': begin R.Init(); R.TestField(Name); end;
            'c-errorinfo': begin EI.Message := 'RV errorinfo'; Error(EI); end;
            'c-confirm': if Confirm('RV close confirm?') then;
            'c-write': begin O.Code := 'C'; O.Insert(); end;
            'c-write-confirm': begin O.Code := 'C'; O.Insert(); if Confirm('RV close confirm?') then; end;
        end;
        Add('C-done');
    end;
    procedure ModRec(NameValue: Text)
    var O: Record "RV Other"; R: Record "RV Row";
    begin
        Add('M ' + NameValue);
        case Source of
            'm-error': Error('RV modify error');
            'm-testfield': begin R.Init(); R.TestField(Name); end;
            'm-write-error': begin O.Code := 'M'; O.Insert(); Error('RV modify error after write'); end;
        end;
    end;
}
page 50103 "RV Card"
{
    PageType = Card;
    SourceTable = "RV Row";
    layout { area(Content) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } }
    actions { area(Processing) { action(CloseMe) { trigger OnAction() begin CurrPage.Close(); end; } } }
    var Cfg: Codeunit "RV Cfg";
    trigger OnModifyRecord(): Boolean begin Cfg.ModRec(Rec.Name); exit(true); end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Cfg.Add('I ' + Rec.Name); if Rec.Name = 'FAILINS' then Error('RV insert error'); exit(true); end;
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
page 50105 "RV TempCard"
{
    PageType = Card;
    SourceTable = "RV Row";
    SourceTableTemporary = true;
    layout { area(Content) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } }
    var Cfg: Codeunit "RV Cfg";
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Cfg.Query(CloseAction)); end;
    trigger OnClosePage() begin Cfg.Closed(); end;
}
page 50106 "RV NoSource"
{
    PageType = Card;
    layout { area(Content) { field(X; X) { } } }
    var Cfg: Codeunit "RV Cfg"; X: Integer;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Cfg.Query(CloseAction)); end;
    trigger OnClosePage() begin Cfg.Closed(); end;
}
page 50107 "RV List"
{
    PageType = List;
    SourceTable = "RV Row";
    layout { area(Content) { repeater(R) { field("Code"; Rec."Code") { } field(Name; Rec.Name) { } } } }
    var Cfg: Codeunit "RV Cfg";
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin exit(Cfg.Query(CloseAction)); end;
    trigger OnClosePage() begin Cfg.Closed(); end;
}
"#;

/// The handlers every test codeunit declares; `CardH` acts by the mode of `RV Cfg`.
const HANDLERS: &str = r#"
    var Cfg: Codeunit "RV Cfg";
    local procedure Seed(var R: Record "RV Row")
    var O: Record "RV Other";
    begin
        R.DeleteAll(); O.DeleteAll();
        R.Code := 'R1'; R.Name := 'one'; R.Insert();
        Commit();
    end;
    local procedure RunCard(S: Text; M: Integer)
    var R: Record "RV Row"; O: Record "RV Other"; Res: Text;
    begin
        Seed(R);
        Cfg.Set(S, M);
        if TryRunCard(R) then Res := 'returned' else Res := 'error=' + GetLastErrorCode() + '|' + GetLastErrorText();
        R.Get('R1');
        Error('OBS %1 log=%2 others=%3 name=%4', Res, Cfg.Peek(), O.Count(), R.Name);
    end;
    [TryFunction]
    local procedure TryRunCard(var R: Record "RV Row")
    begin
        Page.Run(Page::"RV Card", R);
    end;
    [PageHandler] procedure CardH(var P: TestPage "RV Card")
    begin
        Cfg.Add('H');
        case Cfg.Mode() of
            1: P.OK().Invoke();
            2: Error('RV handler error');
            3: P.CloseMe.Invoke();
            4: begin P.New(); P."Code".SetValue('N1'); P.Name.SetValue('FAILINS'); end;
            5: P.Close();
            6: P.Name.SetValue('EDIT');
            7: begin P.Name.SetValue('EDIT'); P.OK().Invoke(); end;
            8: begin P.Name.SetValue('EDIT'); Error('RV handler error'); end;
        end;
    end;
    [PageHandler] procedure WsH(var P: TestPage "RV Worksheet") begin end;
    [PageHandler] procedure TempH(var P: TestPage "RV TempCard") begin end;
    [PageHandler] procedure NoSrcH(var P: TestPage "RV NoSource") begin end;
    [PageHandler] procedure ListH(var P: TestPage "RV List") begin end;
    [MessageHandler] procedure MsgH(M: Text[1024]) begin Cfg.Add('MSG'); end;
    [ConfirmHandler] procedure ConfH(Q: Text[1024]; var Reply: Boolean) begin Cfg.Add('CONF'); Reply := true; end;
"#;

/// The outcome of a test codeunit whose one test has `handlers` and `body`.
fn outcome(handlers: &str, body: &str) -> Outcome {
    let src = format!(
        "{FIXTURE}\ncodeunit 50110 \"RV Tests\"\n{{\n    Subtype = Test;\n    TestPermissions = Disabled;\n{HANDLERS}\n    [Test] [HandlerFunctions('{handlers}')]\n    procedure T()\n    var R: Record \"RV Row\"; O: Record \"RV Other\"; L: Page \"RV List\";\n    begin\n{body}\n    end;\n}}\n"
    );
    let o = al_testkit::outcomes_in(&src, "RV Tests");
    assert_eq!(o.len(), 1, "{o:?}");
    o[0].1.clone()
}

/// `RunCard(source, mode)` with the card handler.
fn card(source: &str, mode: u32) -> Outcome {
    outcome("CardH", &format!("RunCard('{source}', {mode});"))
}

fn fail(text: &str) -> Outcome {
    Outcome::Fail(text.into())
}

fn refused_with(o: &Outcome, reason: &str) {
    match o {
        Outcome::NotRun(why) => assert!(why.contains(reason), "{why}"),
        other => panic!("expected a refusal with <{reason}>, got {other:?}"),
    }
}

const ON_CLOSE_PAGE_REFUSED: &str =
    "an error in OnClosePage at a close by an action or a handler while the page closes";
const VETO_REFUSED: &str = "a page whose OnQueryClosePage refused the platform's close";

// The pinned cells on a card of this shape (CemPageHandlerTests.al:474 CardStaticRun_HandlerReturns_ClosePageError, :583
// _HandlerReturns_ClosePageConfirmUnhandled, :255 _HandlerReturns_QueryCloseError, :611 _HandlerOk_ClosePageConfirmUnhandled,
// :227 _HandlerTableRefusesReturns_NoError, :144 _HandlerError_NoError).
#[test]
fn the_pinned_cells_run_on_a_card_of_the_pinned_shape() {
    assert_eq!(
        card("c-error", 0),
        fail("OBS returned log=H; Q OK; C;  others=0 name=one")
    );
    assert_eq!(
        card("c-confirm", 0),
        fail("OBS returned log=H; Q OK; C;  others=0 name=one")
    );
    assert_eq!(card("q-error", 0), fail("OBS returned log=H; Q OK;  others=0 name=one"));
    assert_eq!(
        card("c-confirm", 1),
        fail(
            "OBS error=TestWrapped|One or more errors occurred while closing the page.\\\\Unhandled UI: Confirm RV close confirm? log=H; Q OK; C;  others=0 name=one"
        )
    );
    assert_eq!(
        card("none", 2),
        fail("OBS error=Dialog|RV handler error log=H; Q OK; C; C-done;  others=0 name=one")
    );
}

// Review of PR 7840, HIGH: OnQueryClosePage at the platform's close. Only an AL Error with a literal text, before any write,
// is pinned (CemFixtures.al:150-155 `Error('CEM query close error')`); a veto, an unhandled Confirm, a TestField error and an
// error after a write keep main's outcome.
#[test]
fn an_on_query_close_page_veto_after_the_handler_returned_keeps_the_refusal() {
    refused_with(&card("q-veto", 0), VETO_REFUSED);
    refused_with(&card("q-veto", 1), VETO_REFUSED);
}

#[test]
fn an_unhandled_confirm_in_on_query_close_page_keeps_mains_message_error() {
    let shown = "OBS error=MissingUIHandler|Unhandled UI: Message Unhandled UI: Confirm RV query confirm? log=H; Q OK;  others=0 name=one";
    assert_eq!(card("q-confirm", 0), fail(shown));
    assert_eq!(card("q-confirm", 1), fail(shown));
}

#[test]
fn a_test_field_error_in_on_query_close_page_keeps_mains_message_error() {
    assert_eq!(
        card("q-testfield", 0),
        fail(
            "OBS error=MissingUIHandler|Unhandled UI: Message Name must have a value in RV Row: Code=. It cannot be zero or empty. log=H; Q OK;  others=0 name=one"
        )
    );
}

#[test]
fn an_on_query_close_page_error_after_a_write_keeps_mains_message_error() {
    assert_eq!(
        card("q-write-error", 0),
        fail(
            "OBS error=MissingUIHandler|Unhandled UI: Message RV query error after write log=H; Q OK;  others=1 name=one"
        )
    );
}

// Review of PR 7840, MED 1: only an unhandled Confirm is pinned among the UI calls of OnClosePage (CemFixtures.al:165).
#[test]
fn an_unhandled_message_or_str_menu_in_on_close_page_keeps_the_refusal() {
    refused_with(&card("c-message", 0), ON_CLOSE_PAGE_REFUSED);
    refused_with(&card("c-message", 1), ON_CLOSE_PAGE_REFUSED);
    refused_with(&card("c-strmenu", 0), ON_CLOSE_PAGE_REFUSED);
}

#[test]
fn an_unhandled_message_in_on_close_page_at_test_page_close_keeps_the_unpinned_error() {
    refused_with(&card("c-message", 5), "an error whose text is not pinned");
}

// Review of PR 7840, MED 2 and the error sources of OnClosePage: a TestField error, Error(ErrorInfo), a write before the
// Confirm, CurrPage.Close() of a page action.
#[test]
fn other_on_close_page_errors_keep_the_refusal() {
    refused_with(&card("c-testfield", 0), ON_CLOSE_PAGE_REFUSED);
    refused_with(&card("c-errorinfo", 0), ON_CLOSE_PAGE_REFUSED);
    refused_with(&card("c-write-confirm", 0), ON_CLOSE_PAGE_REFUSED);
    refused_with(&card("c-error", 3), ON_CLOSE_PAGE_REFUSED);
}

#[test]
fn other_pages_and_runs_keep_the_refusal() {
    let worksheet = outcome(
        "WsH",
        "Seed(R); Cfg.Set('c-error', 0); Page.Run(Page::\"RV Worksheet\", R); Error('OBS returned');",
    );
    refused_with(&worksheet, ON_CLOSE_PAGE_REFUSED);
    let temp = outcome(
        "TempH",
        "Seed(R); Cfg.Set('c-error', 0); Page.Run(Page::\"RV TempCard\"); Error('OBS returned');",
    );
    refused_with(&temp, ON_CLOSE_PAGE_REFUSED);
    let no_source = outcome(
        "NoSrcH",
        "Seed(R); Cfg.Set('c-error', 0); Page.Run(Page::\"RV NoSource\"); Error('OBS returned');",
    );
    refused_with(&no_source, ON_CLOSE_PAGE_REFUSED);
    let list_variable = outcome(
        "ListH",
        "Seed(R); Cfg.Set('c-error', 0); L.SetTableView(R); L.Run(); Error('OBS returned');",
    );
    refused_with(&list_variable, ON_CLOSE_PAGE_REFUSED);
    let list_veto = outcome(
        "ListH",
        "Seed(R); Cfg.Set('q-veto', 0); Page.Run(Page::\"RV List\", R); Error('OBS returned');",
    );
    refused_with(&list_veto, VETO_REFUSED);
}

// The handler raised an error: the platform closes the page as after a return only in the pinned cells
// (CemPageHandlerTests.al:144 _HandlerError_NoError, :364 _HandlerError_QueryCloseError). After an error of OnClosePage, a
// veto or another error of OnQueryClosePage the close is not pinned.
#[test]
fn a_handler_error_then_an_unpinned_close_failure_is_refused() {
    let reason = "a page left open by a PageHandler that raised an error, whose close then failed or was refused";
    refused_with(&card("c-error", 2), ON_CLOSE_PAGE_REFUSED);
    refused_with(&card("q-veto", 2), reason);
    refused_with(&card("q-confirm", 2), reason);
}

#[test]
fn a_handler_error_with_an_unsaved_row_is_refused() {
    refused_with(
        &card("none", 8),
        "a page left open by a PageHandler that raised an error, with an unsaved row (not pinned)",
    );
}

#[test]
fn a_handler_error_whose_close_writes_is_refused() {
    refused_with(
        &card("c-write", 2),
        "a page left open by a PageHandler that raised an error, whose close wrote to the database (not pinned)",
    );
}

#[test]
fn an_on_query_close_page_error_at_ok_taken_by_a_message_handler_is_refused() {
    refused_with(
        &outcome("CardH,MsgH", "RunCard('q-error', 1);"),
        "an error of OnQueryClosePage at OK in a PageHandler of Page.Run, taken by a MessageHandler (not pinned)",
    );
}

#[test]
fn a_save_error_at_ok_in_the_handler_is_refused() {
    refused_with(
        &card("m-error", 7),
        "an error raised while the page saved its edited row at an action of a PageHandler of Page.Run (not pinned)",
    );
}

// The save at the platform's close: pinned for an AL Error with a literal text of the page's OnModifyRecord (raised in a codeunit
// it calls) and of the table's OnModify, the row as it was (CemPageHandlerTests.al:692 _HandlerEditReturns_PageModifyRecordError,
// :227 _HandlerTableRefusesReturns_NoError). A TestField error or an error after a write keeps the refusal.
#[test]
fn a_save_error_at_the_platform_close_runs_only_for_the_pinned_sources() {
    assert_eq!(
        card("m-error", 6),
        fail("OBS returned log=H; M EDIT;  others=0 name=one")
    );
    let reason =
        "an error raised while the page saved its edited row as the platform closed it after the handler returned";
    refused_with(&card("m-testfield", 6), reason);
    refused_with(&card("m-write-error", 6), reason);
}

// Shapes main ran keep main's outcome.
#[test]
fn closes_main_ran_keep_mains_outcome() {
    assert_eq!(
        card("none", 4),
        fail("OBS returned log=H; I ; M FAILINS; Q OK; C; C-done;  others=0 name=one")
    );
    assert_eq!(
        card("none", 3),
        fail("OBS returned log=H; Q OK; C; C-done;  others=0 name=one")
    );
    assert_eq!(
        outcome("CardH,MsgH", "RunCard('c-message', 0);"),
        fail("OBS returned log=H; Q OK; C; MSG; C-done;  others=0 name=one")
    );
    assert_eq!(
        outcome("CardH,ConfH", "RunCard('c-confirm', 0);"),
        fail("OBS returned log=H; Q OK; C; CONF; C-done;  others=0 name=one")
    );
}

// No pin binds a MessageHandler: an error of OnQueryClosePage at the platform's close with one bound keeps main's refusal.
#[test]
fn an_on_query_close_page_error_with_a_message_handler_bound_keeps_the_refusal() {
    refused_with(&outcome("CardH,MsgH", "RunCard('q-error', 0);"), VETO_REFUSED);
}

// The shape of al-runner issue 7480: a NavigatePage run by its Page variable, the handler clicks Next twice and returns, and
// OnClosePage asks a Confirm no handler takes before it would delete the row (CemPageHandlerTests.al:1686
// WizardRun_HandlerReturns_ClosePageConfirmUnhandled: `Page.Run` returns).
#[test]
fn the_reporters_wizard_returns_after_an_unhandled_confirm_in_on_close_page() {
    let src = format!(
        r#"{FIXTURE}
page 50108 "RV Wizard"
{{
    PageType = NavigatePage;
    SourceTable = "RV Row";
    layout {{ area(Content) {{ field(Name; Rec.Name) {{ }} }} }}
    actions {{ area(Processing) {{ action(ActionNext) {{ InFooterBar = true; trigger OnAction() begin Steps += 1; end; }} }} }}
    var Steps: Integer; SaveRecord: Boolean;
    trigger OnClosePage()
    begin
        if not SaveRecord then
            if not Confirm('Do you want to finish later?') then
                Rec.Delete(true);
    end;
}}
codeunit 50111 "RV Wizard Tests"
{{
    Subtype = Test;
    [Test] [HandlerFunctions('WizH')]
    procedure T()
    var R: Record "RV Row"; W: Page "RV Wizard";
    begin
        R.DeleteAll(); R.Code := 'R1'; R.Insert();
        W.SetRecord(R);
        W.Run();
        if not R.Get('R1') then Error('row gone');
    end;
    [PageHandler] procedure WizH(var P: TestPage "RV Wizard") begin P.ActionNext.Invoke(); P.ActionNext.Invoke(); end;
}}
"#
    );
    let o = al_testkit::outcomes_in(&src, "RV Wizard Tests");
    assert_eq!(o, vec![("T".to_string(), Outcome::Pass)]);
}
