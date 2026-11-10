//! al-runner issue 6924 (waldo, 34 `COD ERM CopyPurchSalesDoc Test` tests): a SingleInstance codeunit holds the sender of a
//! host page's event in a global `Page` variable. The TestPage that opened that host closes, and later code calls a procedure
//! of the held page that reaches `CurrPage.<part>.Page`. BC raises no error: the call reaches the held page's OWN part
//! instance (its old row, its globals as they were), not the part of a page that is open now. Measured on BC 27.5, 28.5 and
//! 29.0 (corpus probe pages/parts/HeldSenderPartAccess.al, staging branch al-runtime-rs-staging-probe-train-1110b; assumption
//! `held-page-part-after-testpage-close`). The objects are the probe's, the Probe Observer replaced by a text check.
//!
//! Not here: the second test of each shape codeunit (HS07, HS09, HS11, HS13: OpenNew, SetValue on the key, an action), which
//! needs the save of a new card row with only its key typed on an extended page with a factbox; the runtime does not do that yet.

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

    // The probe's second test (HS07, HS09, HS11, HS13): OpenNew, the key set with SetValue, an action invoked, Close. Not run
    // by a test here (see the module comment).
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
// and its row stays id 1. In the next test BC counts 0 rows through the part's Rec with the row in the table (HS02 `read`
// `id=1 name=one count=0`): that read is refused, the rest of HS02 answers.
#[test]
fn held_page_after_testpage_close_and_in_the_next_test_reaches_its_own_part() {
    let o = run_simple(
        "[Test] procedure HS01() var Host: TestPage \"LT HS Host\"; begin
            H.Seed(); Holder.Setup(true, false); Host.OpenView();
            H.Check('touch=1;read=id=1 name=one count=1;mark=m;host=id=1 name=one', H.Reach());
            Host.Close();
            H.Check('touch=2;read=id=1 name=one count=1;mark=m;host=id=1 name=one', H.Reach());
        end;
        [Test] procedure HS02_NoRead() var Touched: Integer; T1: Text; begin
            Holder.TryTouch(Touched); H.Check('3', Format(Touched));
            Holder.TryMark(T1); H.Check('m', T1);
            Holder.TryHostRead(T1); H.Check('id=1 name=one', T1);
        end;
        [Test] procedure HS02_Read() var T1: Text; begin
            if Holder.TryRead(T1) then; H.Check('id=1 name=one count=0', T1);
        end;",
    );
    assert_eq!(o.len(), 3, "{o:?}");
    assert_all_pass(&o[..2]);
    assert!(
        matches!(&o[2].1, Outcome::NotRun(why) if why.contains("a part of a page the test client closed in an earlier test")),
        "{o:?}"
    );
}

// HS03: a second TestPage of the host opens while the holder keeps the first, closed one: the factbox event of the open page
// reaches the held page's part, not the open page's. HS04: the holder took the open page. Both read the first part's row.
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
