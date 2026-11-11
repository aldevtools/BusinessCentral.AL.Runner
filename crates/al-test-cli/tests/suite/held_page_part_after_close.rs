//! al-runner issue 6924 (34 `COD ERM CopyPurchSalesDoc Test` tests of an outside reporter): a SingleInstance codeunit holds
//! the sender of a host page's event in a global `Page` variable. The TestPage that opened that host closes, and later code
//! calls a procedure of the held page that reaches `CurrPage.<part>.Page`. BC raises no error: the call reaches the held
//! page's OWN part instance (its row, its globals as they were), not the part of a page that is open now. Measured on the
//! three supported BC majors (corpus probe pages/parts/HeldSenderPartAccess.al, staging branch
//! al-runtime-rs-staging-probe-train-1110b; assumption `held-page-part-after-testpage-close`). The objects are the probe's,
//! the Probe Observer replaced by a text check.
//!
//! Not here: the second test of each shape codeunit (HS07, HS09, HS11, HS13: OpenNew with a hidden or shown factbox,
//! SetValue on the key, an action). The runtime refuses the action on that new row (`new_row_action_shapes_are_refused`).

use al_test_cli::report::Outcome;
use al_testkit::outcomes_in;

const SRC: &str = r#"
table 99620 "LT HS Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }

codeunit 99628 "LT HS Shape Log"
{
    SingleInstance = true;
    var Log: Text; Counter: Integer; PartVisible: Boolean; UseTry: Boolean;
    procedure Setup(NewPartVisible: Boolean; NewUseTry: Boolean) begin PartVisible := NewPartVisible; UseTry := NewUseTry; Counter := 0; Log := ''; end;
    procedure IsPartVisible(): Boolean begin exit(PartVisible); end;
    procedure IsTry(): Boolean begin exit(UseTry); end;
    procedure NextNo(): Integer begin Counter += 1; exit(Counter); end;
    procedure Add(T: Text) begin Log += T + ';'; end;
    procedure TakeLog(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
    procedure Bool(B: Boolean): Text begin if B then exit('true'); exit('false'); end;
}

page 99624 "LT HS Shape Fact"
{
    PageType = CardPart;
    SourceTable = "LT HS Row";
    layout { area(Content) { field(IdF; Rec.Id) { } } }
    var ShapeLog: Codeunit "LT HS Shape Log"; MyNo: Integer; Refreshes: Integer;
    trigger OnOpenPage() begin ShapeLog.Add(Me() + '.open'); end;
    trigger OnAfterGetCurrRecord() begin ShapeLog.Add(Me() + '.curr(' + Format(Rec.Id) + ')'); OnAfterGetRecordEventWithSender(Rec, false); end;
    procedure RefreshPageData(): Integer
    begin
        Refreshes += 1;
        ShapeLog.Add(Me() + '.refresh(id=' + Format(Rec.Id) + ',n=' + Format(Refreshes) + ')');
        exit(Refreshes);
    end;
    local procedure Me(): Text begin if MyNo = 0 then MyNo := ShapeLog.NextNo(); exit('f' + Format(MyNo)); end;
    [IntegrationEvent(true, false)] local procedure OnAfterGetRecordEventWithSender(Row: Record "LT HS Row"; ForceRefresh: Boolean) begin end;
}

page 99625 "LT HS Shape Host"
{
    PageType = Card;
    SourceTable = "LT HS Row";
    layout { area(Content) { field(IdF; Rec.Id) { } field(NameF; Rec.Name) { } } area(FactBoxes) { } }
    actions
    {
        area(Processing)
        {
            action(Touch)
            {
                trigger OnAction() var ShapeLog: Codeunit "LT HS Shape Log"; begin ShapeLog.Add('action(' + Format(Rec.Id) + ')'); end;
            }
        }
    }
}

pageextension 99626 "LT HS Shape Host Ext" extends "LT HS Shape Host"
{
    layout
    {
        addfirst(FactBoxes)
        {
            part(ShapeFact; "LT HS Shape Fact") { Caption = 'Shape'; Visible = FactVisible; SubPageLink = SystemId = field(SystemId); }
        }
    }
    var ShapeLog: Codeunit "LT HS Shape Log"; FactVisible: Boolean; MyNo: Integer;
    trigger OnOpenPage() begin FactVisible := ShapeLog.IsPartVisible(); ShapeLog.Add(Me() + '.open(vis=' + ShapeLog.Bool(FactVisible) + ')'); end;
    trigger OnAfterGetRecord() begin ShapeLog.Add(Me() + '.getrec(' + Format(Rec.Id) + ')'); OnAfterGetRecordShape(Rec); end;
    procedure UpdateShapeFactbox() begin ShapeLog.Add(Me() + '.upd'); CurrPage.ShapeFact.Page.RefreshPageData(); end;
    procedure UpdateShapeFactboxValue(): Integer begin ShapeLog.Add(Me() + '.updv'); exit(CurrPage.ShapeFact.Page.RefreshPageData()); end;
    local procedure Me(): Text begin if MyNo = 0 then MyNo := ShapeLog.NextNo(); exit('h' + Format(MyNo)); end;
    [IntegrationEvent(true, false)] local procedure OnAfterGetRecordShape(Row: Record "LT HS Row") begin end;
}

codeunit 99627 "LT HS Shape Subs"
{
    SingleInstance = true;
    var ShapeLog: Codeunit "LT HS Shape Log"; GlobalFact: Page "LT HS Shape Fact"; GlobalHost: Page "LT HS Shape Host"; Enabled: Boolean; Initialized: Boolean;
    procedure Reset() begin Clear(GlobalFact); Clear(GlobalHost); Enabled := false; Initialized := false; end;
    [EventSubscriber(ObjectType::Page, Page::"LT HS Shape Host", OnOpenPageEvent, '', false, false)]
    local procedure HostOnOpenPage() begin ShapeLog.Add('sub.open'); Enabled := true; end;
    [EventSubscriber(ObjectType::Page, Page::"LT HS Shape Host", OnAfterGetCurrRecordEvent, '', false, false)]
    local procedure HostOnAfterGetCurrRecord() begin ShapeLog.Add('sub.curr'); Enabled := true; end;
    [EventSubscriber(ObjectType::Page, Page::"LT HS Shape Host", OnQueryClosePageEvent, '', false, false)]
    local procedure HostOnQueryClosePage() begin ShapeLog.Add('sub.qclose'); Enabled := false; end;
    [EventSubscriber(ObjectType::Page, Page::"LT HS Shape Host", OnAfterGetRecordShape, '', false, false)]
    local procedure HostOnAfterGetRecordShape(sender: Page "LT HS Shape Host") begin ShapeLog.Add('sub.hold'); GlobalHost := sender; end;
    [EventSubscriber(ObjectType::Page, Page::"LT HS Shape Fact", OnAfterGetRecordEventWithSender, '', false, false)]
    local procedure FactOnAfterGetRecordEventWithSender(var sender: Page "LT HS Shape Fact"; Row: Record "LT HS Row"; ForceRefresh: Boolean)
    begin
        ShapeLog.Add('sub.fact(enabled=' + ShapeLog.Bool(Enabled) + ',init=' + ShapeLog.Bool(Initialized) + ')');
        if not Enabled and not ForceRefresh then
            exit;
        GlobalFact := sender;
        Check();
    end;
    local procedure Check()
    var Returned: Integer;
    begin
        if ShapeLog.IsTry() then begin
            if TryUpdate(Returned) then
                ShapeLog.Add('ret=' + Format(Returned))
            else
                ShapeLog.Add('err=' + GetLastErrorCode() + '|' + GetLastErrorText());
        end else
            GlobalHost.UpdateShapeFactbox();
        Initialized := true;
    end;
    [TryFunction]
    procedure TryUpdate(var Returned: Integer) begin Returned := GlobalHost.UpdateShapeFactboxValue(); end;
}

codeunit 99633 "LT HS Shape Run"
{
    procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;

    // The probe's first test (HS06, HS08, HS10, HS12): OpenEdit on an existing row, Close, then with Try one direct call.
    procedure FirstTest(Visible: Boolean; UseTry: Boolean): Text
    var ShapeLog: Codeunit "LT HS Shape Log"; Subs: Codeunit "LT HS Shape Subs"; Row: Record "LT HS Row"; Host: TestPage "LT HS Shape Host"; Returned: Integer;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Name := 'one';
        Row.Insert();
        Subs.Reset();
        ShapeLog.Setup(Visible, UseTry);
        Host.OpenEdit();
        ShapeLog.Add('T:opened');
        Host.Close();
        ShapeLog.Add('T:closed');
        if UseTry then
            if Subs.TryUpdate(Returned) then
                ShapeLog.Add('direct=' + Format(Returned))
            else
                ShapeLog.Add('direct-error');
        exit(ShapeLog.TakeLog());
    end;

    // The probe's second test (HS07, HS09, HS11, HS13): OpenNew, the key set with SetValue, an action invoked, Close. Refused at
    // the action (see `new_row_action_shapes_are_refused`).
    procedure SecondTest(UseTry: Boolean): Text
    var ShapeLog: Codeunit "LT HS Shape Log"; Subs: Codeunit "LT HS Shape Subs"; Host: TestPage "LT HS Shape Host"; Returned: Integer;
    begin
        ShapeLog.TakeLog();
        Host.OpenNew();
        ShapeLog.Add('T:opened');
        Host.IdF.SetValue(2);
        ShapeLog.Add('T:set');
        Host.Touch.Invoke();
        ShapeLog.Add('T:acted');
        Host.Close();
        ShapeLog.Add('T:closed');
        if UseTry then
            if Subs.TryUpdate(Returned) then
                ShapeLog.Add('direct=' + Format(Returned))
            else
                ShapeLog.Add('direct-error');
        exit(ShapeLog.TakeLog());
    end;
}
"#;

// The logs BC 27.5, 28.5 and 29.0 wrote for the first test of each shape codeunit (HeldSenderPartAccess.al HS06, HS08,
// HS10, HS12); `direct=N` stands for the probe's `closed.direct` observation, `V` for the part's visibility.
const FIRST_RAW: &str = "h1.open(vis=V);sub.open;f2.open;h1.getrec(1);sub.hold;sub.curr;f2.curr(1);sub.fact(enabled=true,init=false);h1.upd;f2.refresh(id=1,n=1);T:opened;sub.qclose;T:closed;";
const FIRST_TRY: &str = "h1.open(vis=V);sub.open;f2.open;h1.getrec(1);sub.hold;sub.curr;f2.curr(1);sub.fact(enabled=true,init=false);h1.updv;f2.refresh(id=1,n=1);ret=1;T:opened;sub.qclose;T:closed;h1.updv;f2.refresh(id=1,n=2);direct=2;";

/// Runs the first test of a shape codeunit of the probe and gives its outcome.
fn run_shape_first(visible: bool, use_try: bool) -> (String, Outcome) {
    let first = if use_try { FIRST_TRY } else { FIRST_RAW };
    let vis = if visible { "vis=true" } else { "vis=false" };
    let src = format!(
        "{SRC}\ncodeunit 99634 T {{ Subtype = Test; TestPermissions = Disabled;
        var Run: Codeunit \"LT HS Shape Run\";
        [Test] procedure First() begin Run.Check('{}', Run.FirstTest({visible}, {use_try})); end;
        }}",
        first.replace("vis=V", vis),
    );
    let mut o = outcomes_in(&src, "T");
    assert_eq!(o.len(), 1, "{o:?}");
    o.remove(0)
}

fn assert_all_pass(o: &[(String, Outcome)]) {
    assert!(!o.is_empty());
    for (name, outcome) in o {
        assert!(matches!(outcome, Outcome::Pass), "{name}: {outcome:?}");
    }
}

// HS06 and HS10: part hidden or shown, the held page reached while its TestPage is open; the close runs OnQueryClosePage.
#[test]
fn held_sender_reaches_its_factbox_while_open_hidden_or_shown() {
    for visible in [false, true] {
        let (name, o) = run_shape_first(visible, false);
        assert!(matches!(o, Outcome::Pass), "visible={visible} {name}: {o:?}");
    }
}

// HS08 and HS12: after TestPage.Close the test calls the held page, which reaches its own factbox: the factbox's counter goes
// on (2), for a hidden part and a shown one.
#[test]
fn after_testpage_close_the_held_page_reaches_its_own_factbox_hidden_or_shown() {
    for visible in [false, true] {
        let (name, o) = run_shape_first(visible, true);
        assert!(matches!(o, Outcome::Pass), "visible={visible} {name}: {o:?}");
    }
}

// The probe's first objects (HeldSenderPartAccess.al:16-232, objects 99620-99623): a host with a ListPart `Lines` and a
// factbox; a SingleInstance holder keeps the host's sender from its OnOpenPage event and reaches the part through it.
const SIMPLE: &str = r#"
table 99620 "LT HS Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }

codeunit 99620 "LT HS Holder"
{
    SingleInstance = true;
    var Held: Page "LT HS Host"; Assign: Boolean; Reach: Boolean; Log: Text;
    procedure Setup(AssignSender: Boolean; ReachFromFactbox: Boolean) begin Assign := AssignSender; Reach := ReachFromFactbox; Log := ''; end;
    procedure TakeLog(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
    procedure Add(T: Text) begin Log += T + ';'; end;
    [EventSubscriber(ObjectType::Page, Page::"LT HS Host", 'OnOpened', '', false, false)]
    local procedure HostOpened(var sender: Page "LT HS Host") begin if Assign then Held := sender; end;
    [EventSubscriber(ObjectType::Page, Page::"LT HS Fact", 'OnFactRow', '', false, false)]
    local procedure FactRow(var sender: Page "LT HS Fact"; RowId: Integer)
    var Touched: Integer; Seen: Text;
    begin
        Add('fact(' + Format(RowId) + ')');
        if not Reach then
            exit;
        if TryTouch(Touched) then
            Add('touch=' + Format(Touched))
        else
            Add('touch-error=' + GetLastErrorCode() + '|' + GetLastErrorText());
        if TryRead(Seen) then
            Add('read=' + Seen)
        else
            Add('read-error=' + GetLastErrorCode() + '|' + GetLastErrorText());
    end;
    [TryFunction] procedure TryTouch(var Result: Integer) begin Result := Held.PartTouch(); end;
    [TryFunction] procedure TryRead(var Result: Text) begin Result := Held.PartRead(); end;
    [TryFunction] procedure TryMark(var Result: Text) begin Held.PartSetMark('m'); Result := Held.PartGetMark(); end;
    [TryFunction] procedure TryHostRead(var Result: Text) begin Result := Held.HostRead(); end;
    procedure PartUpdate() begin Held.PartUpdate(); end;
    procedure PartFind(): Boolean begin exit(Held.PartFind()); end;
}

page 99621 "LT HS Part"
{
    PageType = ListPart;
    SourceTable = "LT HS Row";
    layout { area(Content) { repeater(Rows) { field(IdF; Rec.Id) { } field(NameF; Rec.Name) { } } } }
    var Touches: Integer; MarkText: Text;
    procedure ReadRec(): Text begin exit('id=' + Format(Rec.Id) + ' name=' + Rec.Name + ' count=' + Format(Rec.Count())); end;
    procedure Touch(): Integer begin Touches += 1; exit(Touches); end;
    procedure SetMark(NewMark: Text) begin MarkText := NewMark; end;
    procedure GetMark(): Text begin exit(MarkText); end;
    procedure UpdateMe() begin CurrPage.Update(false); end;
    procedure FindIt(): Boolean begin exit(Rec.FindFirst()); end;
}

page 99622 "LT HS Fact"
{
    PageType = CardPart;
    SourceTable = "LT HS Row";
    layout { area(Content) { field(IdF; Rec.Id) { } } }
    trigger OnAfterGetCurrRecord() begin OnFactRow(Rec.Id); end;
    [IntegrationEvent(true, false)] local procedure OnFactRow(RowId: Integer) begin end;
}

page 99623 "LT HS Host"
{
    PageType = Card;
    SourceTable = "LT HS Row";
    layout
    {
        area(Content) { field(IdF; Rec.Id) { } part(Lines; "LT HS Part") { SubPageLink = Id = field(Id); } }
        area(FactBoxes) { part(Fact; "LT HS Fact") { SubPageLink = Id = field(Id); } }
    }
    trigger OnOpenPage() begin OnOpened(); end;
    procedure PartRead(): Text begin exit(CurrPage.Lines.Page.ReadRec()); end;
    procedure PartTouch(): Integer begin exit(CurrPage.Lines.Page.Touch()); end;
    procedure PartSetMark(NewMark: Text) begin CurrPage.Lines.Page.SetMark(NewMark); end;
    procedure PartGetMark(): Text begin exit(CurrPage.Lines.Page.GetMark()); end;
    procedure PartUpdate() begin CurrPage.Lines.Page.UpdateMe(); end;
    procedure PartFind(): Boolean begin exit(CurrPage.Lines.Page.FindIt()); end;
    procedure HostRead(): Text begin exit('id=' + Format(Rec.Id) + ' name=' + Rec.Name); end;
    [IntegrationEvent(true, false)] local procedure OnOpened() begin end;
}

codeunit 99624 "LT HS Simple Help"
{
    procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
    procedure Seed() var Row: Record "LT HS Row"; begin Row.DeleteAll(); Row.Id := 1; Row.Name := 'one'; Row.Insert(); end;
    // The probe's ObserveReach, as one line: touch, read, mark, host.
    procedure Reach(): Text
    var Holder: Codeunit "LT HS Holder"; Touched: Integer; T1: Text; R: Text;
    begin
        if Holder.TryTouch(Touched) then R += 'touch=' + Format(Touched) else R += 'touch-error';
        if Holder.TryRead(T1) then R += ';read=' + T1 else R += ';read-error';
        if Holder.TryMark(T1) then R += ';mark=' + T1 else R += ';mark-error';
        if Holder.TryHostRead(T1) then R += ';host=' + T1 else R += ';host-error';
        exit(R);
    end;
}
"#;

/// Runs `tests` (the body of a test codeunit) against [`SIMPLE`].
fn run_simple(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!(
        "{SIMPLE}\ncodeunit 99625 T {{ Subtype = Test; TestPermissions = Disabled;
        var H: Codeunit \"LT HS Simple Help\"; Holder: Codeunit \"LT HS Holder\";
        {tests}
        }}"
    );
    outcomes_in(&src, "T")
}

// HS01 + HS02: the held host reached from the test while its TestPage is open, after TestPage.Close in the same test, and in
// the next test of the codeunit before any open: the same part instance answers (its counter goes on 1, 2, 3; its mark stays),
// its row stays id 1, and the row the first test inserted is still in the table (count 1; corpus commit 75ec185a).
#[test]
fn held_page_after_testpage_close_and_in_the_next_test_reaches_its_own_part() {
    let o = run_simple(
        "[Test] procedure HS01() var Host: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(true, false); Host.OpenView();
            H.Check('touch=1;read=id=1 name=one count=1;mark=m;host=id=1 name=one', H.Reach());
            Host.Close();
            H.Check('touch=2;read=id=1 name=one count=1;mark=m;host=id=1 name=one', H.Reach());
        end;
        [Test] procedure HS02() begin
            H.Check('touch=3;read=id=1 name=one count=1;mark=m;host=id=1 name=one', H.Reach());
        end;",
    );
    assert_eq!(o.len(), 2, "{o:?}");
    assert_all_pass(&o);
}

// HS03: the same TestPage variable opens again while the holder keeps the first, closed page: the variable gets a new page
// and the factbox event of the open page reaches the held first page's part (its counter starts at 1, it was never touched).
// HS04: the holder took the open page. Both read the part's row.
#[test]
fn factbox_event_of_a_second_open_reaches_the_held_closed_pages_part() {
    let o = run_simple(
        "[Test] procedure HS03() var Host: TestPage \"LT HS Host\"; L: Text; begin
            H.Seed(); Holder.Setup(true, false); Host.OpenView(); Host.Close();
            Holder.Setup(false, true); Host.OpenView(); L := Holder.TakeLog(); Host.Close();
            H.Check('fact(1);touch=1;read=id=1 name=one count=1;', L);
        end;
        [Test] procedure HS04() var Host: TestPage \"LT HS Host\"; L: Text; begin
            H.Seed(); Holder.Setup(true, true); Host.OpenView(); L := Holder.TakeLog(); Host.Close();
            H.Check('fact(1);touch=1;read=id=1 name=one count=1;', L);
        end;",
    );
    assert_all_pass(&o);
}

// The same TestPage variable opens again after the held page's part was touched: the holder keeps the first page and its
// part (touch 3, mark m), the variable shows a new page. Not measured in this exact shape; HS07 and HS09 measure the rule with
// another TestPage variable (`h1.upd;f2.refresh(id=1,n=2)` while `h3` is open).
#[test]
fn reopening_the_same_testpage_variable_leaves_the_held_page_its_part() {
    let o = run_simple(
        "[Test] procedure B01() var Host: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(true, false); Host.OpenView();
            H.Check('touch=1;read=id=1 name=one count=1;mark=m;host=id=1 name=one', H.Reach());
            Host.Close();
            Holder.Setup(false, false);
            Host.OpenView();
            H.Check('touch=2;read=id=1 name=one count=1;mark=m;host=id=1 name=one', H.Reach());
            Host.Close();
        end;",
    );
    assert_all_pass(&o);
}

fn refused(o: &(String, Outcome), why: &str) -> bool {
    matches!(&o.1, Outcome::NotRun(r) if r.contains(why))
}

const PART_REFUSED: &str = "a part of a page that is not open, reached by the code of that page";

// Closes the probe does not measure stay refused: the built-in OK action, the close at the end of the test (the next test
// reaches the page), a modal page closed after its handler raised an error. A modal run closed with OK answers (pinned:
// WdPartAfterRunModal.al).
#[test]
fn closes_the_probe_does_not_measure_stay_refused() {
    let o = run_simple(
        "[Test] procedure D1_OkAction() var Host: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(true, false); Host.OpenView(); Host.OK().Invoke(); H.Reach();
        end;
        [Test] procedure D5a_LeftOpen() var Host: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(true, false); Host.OpenEdit();
        end;
        [Test] procedure D5b_NextTest() begin H.Reach(); end;
        [Test] [HandlerFunctions('FailingHandler')] procedure F1_HandlerError() var P: Page \"LT HS Host\"; begin
            H.Seed(); asserterror P.RunModal(); P.PartTouch();
        end;
        [Test] [HandlerFunctions('OkHandler')] procedure F2_HandlerOk() var P: Page \"LT HS Host\"; begin
            H.Seed(); P.RunModal(); H.Check('1', Format(P.PartTouch()));
        end;
        [ModalPageHandler] procedure FailingHandler(var Pg: TestPage \"LT HS Host\") begin Error('handler failed'); end;
        [ModalPageHandler] procedure OkHandler(var Pg: TestPage \"LT HS Host\") begin Pg.OK().Invoke(); end;",
    );
    assert_eq!(o.len(), 5, "{o:?}");
    assert!(refused(&o[0], PART_REFUSED), "{o:?}");
    assert!(matches!(o[1].1, Outcome::Pass), "{o:?}");
    assert!(refused(&o[2], PART_REFUSED), "{o:?}");
    assert!(refused(&o[3], PART_REFUSED), "{o:?}");
    assert!(matches!(o[4].1, Outcome::Pass), "{o:?}");
}

// Inside the held part only what the probe measured runs: a page method of the part (CurrPage.Update) and a method of its Rec
// other than Count (FindFirst) refuse.
#[test]
fn unmeasured_calls_inside_the_held_part_refuse() {
    let o = run_simple(
        "[Test] procedure E1_Update() var Host: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(true, false); Host.OpenView(); Host.Close(); Holder.PartUpdate();
        end;
        [Test] procedure E2_FindFirst() var Host: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(true, false); Host.OpenView(); Host.Close(); Holder.PartFind();
        end;",
    );
    assert_eq!(o.len(), 2, "{o:?}");
    assert!(refused(&o[0], "on a part of a page the test client closed"), "{o:?}");
    assert!(
        refused(&o[1], "only its fields and Count() are measured there"),
        "{o:?}"
    );
}

// HS07, HS09, HS11 and HS13: OpenNew, the key set with SetValue, an action, hidden or shown factbox. BC stores the row before
// the action and reads the host twice after it (HeldSenderPartAccess.al:750); the runtime does not model that and refuses the
// action. The not-passing lists cannot tell a refusal from a failure, so this keeps the shape refused.
#[test]
fn new_row_action_shapes_are_refused() {
    for visible in [false, true] {
        for use_try in [false, true] {
            let src = format!(
                "{SRC}\ncodeunit 99634 T {{ Subtype = Test; TestPermissions = Disabled;
                var Run: Codeunit \"LT HS Shape Run\";
                [Test] procedure First() begin Run.FirstTest({visible}, {use_try}); end;
                [Test] procedure Second() begin Run.SecondTest({use_try}); end;
                }}"
            );
            let o = outcomes_in(&src, "T");
            assert_eq!(o.len(), 2, "{o:?}");
            assert!(matches!(o[0].1, Outcome::Pass), "{o:?}");
            assert!(
                refused(
                    &o[1],
                    "an action on a new row whose key was typed, on a page with parts"
                ),
                "{o:?}"
            );
        }
    }
}

// A reopen through a helper that takes the TestPage by value or by var, or a plain reopen, with no Page variable holding the
// page: the copies share one page and the caller's variable is open afterwards (as Microsoft's helpers do,
// SalesStockout OpenSalesOrderPageByNo).
#[test]
fn reopen_through_a_helper_copy_keeps_the_callers_variable_open() {
    let o = run_simple(
        "local procedure OpenByValue(TP: TestPage \"LT HS Host\") begin TP.OpenEdit(); end;
        local procedure OpenByVar(var TP: TestPage \"LT HS Host\") begin TP.OpenEdit(); end;
        [Test] procedure R1_ByValue() var TP: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(false, false); TP.OpenEdit(); TP.Close(); OpenByValue(TP); H.Check('1', TP.IdF.Value); TP.Close();
        end;
        [Test] procedure R2_ByVar() var TP: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(false, false); TP.OpenEdit(); TP.Close(); OpenByVar(TP); H.Check('1', TP.IdF.Value); TP.Close();
        end;
        [Test] procedure R3_FirstOpenByValue() var TP: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(false, false); OpenByValue(TP); H.Check('1', TP.IdF.Value); TP.Close();
        end;
        [Test] procedure R4_Plain() var TP: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(false, false); TP.OpenEdit(); TP.Close(); TP.OpenEdit(); H.Check('1', TP.IdF.Value); TP.Close();
        end;
",
    );
    assert_eq!(o.len(), 4, "{o:?}");
    assert_all_pass(&o);
}
