//! Second round of behaviours that Microsoft's tests reach (blocked set 3); each test cites the Microsoft code that needs it.

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
enum 69300 \"CV Kind\" { Extensible = true; value(0; None) { } value(1; One) { } value(2; Two) { } }
table 69300 \"CV Row\" { fields { field(1; Code; Code[20]) { } field(7; Number; Integer) { } field(2; Kind; Option) { OptionMembers = A,B,C; }
  field(3; Flag; Boolean) { } field(4; Pick; Enum \"CV Kind\") { } field(9; Total; Integer) { FieldClass = FlowField; CalcFormula = count(\"CV Row\"); } }
  keys { key(PK; Code) { Clustered = true; } } }
";

fn compile(src: String) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    )
}

fn run(tests: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = compile(format!("{OBJECTS} codeunit 69301 T {{ Subtype = Test; {tests} }}"));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

fn all_pass(tests: &str) {
    for (name, outcome) in run(tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// Gen. Jnl.-Show CT Entries: `SetFilter("Currency Code", ''''%1''|''%2''', A, B)`. The placeholders inside the quotes are
// replaced like any other, so a blank currency filters for blank (`''|''`). Assumed, not pinned (registry entry
// filter-placeholder-in-quotes); Credit Transfer Register UT.TestShowCTEntriesForGenJnlLine needs it.
#[test]
fn a_placeholder_inside_quotes_in_set_filter_is_replaced() {
    all_pass(
        "[Test] procedure QuotedPlaceholders() var Row: Record \"CV Row\"; Blank: Code[20]; begin
             Row.Code := 'A'; Row.Insert(); Row.Code := ''; Row.Number := 1; if Row.Insert() then;
             Row.Reset();
             Row.SetFilter(Code, '''%1''|''%2''', Blank, 'A');
             if Row.Count() <> 2 then Error('count %1 filters %2', Row.Count(), Row.GetFilter(Code));
         end;",
    );
}

// Item Tracking Lines publishes `[IntegrationEvent(true, false)] OnSetSourceSpecOnCollectTrackingData` from a page; the
// subscriber in Asm. Item Tracking Mgt. takes `var sender: Page "Item Tracking Lines"`. For a page the sender is the
// page itself, not its Rec (about 800 Microsoft tests stopped at "conversion of temporary Record ... to Page").
#[test]
fn the_sender_of_a_page_event_is_the_page() {
    let (ws, l) = compile(format!(
        "{OBJECTS}
page 69310 \"CV Card\" {{ PageType = Card; SourceTable = \"CV Row\"; SourceTableTemporary = true;
  trigger OnOpenPage() begin OnPing(5); end;
  [IntegrationEvent(true, false)] local procedure OnPing(Value: Integer) begin end;
  procedure Raise() begin OnPing(7); end;
  var Marked: Boolean; }}
codeunit 69311 \"CV Sub\" {{ SingleInstance = true;
  [EventSubscriber(ObjectType::Page, Page::\"CV Card\", 'OnPing', '', false, false)]
  local procedure Ping(Value: Integer; var sender: Page \"CV Card\") begin Seen += Value; end;
  procedure GetSeen(): Integer begin exit(Seen); end;
  var Seen: Integer; }}
codeunit 69301 T {{ Subtype = Test;
  [Test] procedure PageSender() var P: TestPage \"CV Card\"; S: Codeunit \"CV Sub\"; begin
     P.OpenView();
     if S.GetSeen() <> 5 then Error('seen %1', S.GetSeen());
     P.Close();
  end;
  [Test] procedure PageVariableSender() var PV: Page \"CV Card\"; S: Codeunit \"CV Sub\"; Before: Integer; begin
     Before := S.GetSeen();
     PV.Raise();
     if S.GetSeen() <> Before + 7 then Error('seen %1', S.GetSeen());
  end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    for (name, outcome) in run_codeunit(&engine, &l, oi) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// Item Reference Management RunPageReferenceListOnTempRecord: PAGE.RunModal(PAGE::"Item Reference List", TempRec) shows
// the rows of the TEMPORARY record, not those of the table (ERM Item Reference Sales tests: "Queue underflow" when the
// page showed rows the temporary record did not hold).
#[test]
fn a_page_run_with_a_temporary_record_shows_the_temporary_rows() {
    let (ws, l) = compile(format!(
        "{OBJECTS}
page 69312 \"CV List\" {{ PageType = List; InsertAllowed = false; DeleteAllowed = false; ModifyAllowed = false; SourceTable = \"CV Row\"; layout {{ area(Content) {{ repeater(R) {{ field(Code; Rec.Code) {{ }} field(Number; Rec.Number) {{ }} }} }} }} }}
codeunit 69301 T {{ Subtype = Test; var Rows: Text;
  [Test] [HandlerFunctions('ListHandler')] procedure TemporaryRows() var Perm: Record \"CV Row\"; Tmp: Record \"CV Row\" temporary; begin
     Perm.Code := 'P1'; Perm.Number := 5; Perm.Insert(); Perm.Code := 'P2'; Perm.Number := 5; Perm.Insert();
     Tmp.Code := 'T1'; Tmp.Number := 5; Tmp.Insert(); Tmp.Code := 'T2'; Tmp.Number := 6; Tmp.Insert();
     Page.RunModal(Page::\"CV List\", Tmp);
     if Rows <> 'T1;' then Error('rows %1', Rows);
  end;
  [ModalPageHandler] procedure ListHandler(var L: TestPage \"CV List\") begin
     L.FILTER.SetFilter(Number, '5');
     if L.First() then repeat Rows += L.Code.Value + ';'; until not L.Next();
     L.OK().Invoke();
  end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    for (name, outcome) in run_codeunit(&engine, &l, oi) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// A subscriber to a page field's OnAfterValidateEvent refused every page that was merely opened (about 610 Microsoft
// tests: Sales Setup / Sales Order subscribers in Base Application). The client raises the event when it writes the
// field (record/lt-loadfields-pagesource-validate/P81PageValidateEvents.al), so the write runs the subscriber.
#[test]
fn a_page_field_event_does_not_refuse_the_open_and_the_write_raises_it() {
    let (ws, l) = compile(format!(
        "{OBJECTS}
page 69313 \"CV Field Card\" {{ PageType = Card; SourceTable = \"CV Row\"; layout {{ area(Content) {{ field(Code; Rec.Code) {{ }} field(Number; Rec.Number) {{ }} }} }} }}
codeunit 69314 \"CV Field Sub\" {{
  [EventSubscriber(ObjectType::Page, Page::\"CV Field Card\", 'OnAfterValidateEvent', 'Number', false, false)]
  local procedure Validated(var Rec: Record \"CV Row\"; var xRec: Record \"CV Row\") begin end; }}
codeunit 69301 T {{ Subtype = Test;
  [Test] procedure OpenIsFine() var P: TestPage \"CV Field Card\"; begin P.OpenNew(); P.Code.SetValue('A'); P.Close(); end;
  [Test] procedure WriteIsRefused() var P: TestPage \"CV Field Card\"; begin P.OpenNew(); P.Number.SetValue(3); P.Close(); end; }}"
    ));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let o = run_codeunit(&engine, &l, oi);
    let get = |n: &str| o.iter().find(|(m, _)| m == n).map(|(_, x)| x.clone()).unwrap();
    assert_eq!(get("OpenIsFine"), Outcome::Pass);
    assert_eq!(get("WriteIsRefused"), Outcome::Pass, "{o:?}");
}

// `for Stepwise := false to true do` (Library - Sales and others): runs for false, then for true. Assumed, not pinned
// (registry entry for-boolean).
#[test]
fn a_for_loop_over_a_boolean_runs_for_false_then_true() {
    all_pass(
        "[Test] procedure BooleanLoop() var B: Boolean; seen: Text; begin
             for B := false to true do seen += Format(B) + ';';
             if seen <> 'No;Yes;' then Error('up %1', seen);
             seen := '';
             for B := true downto false do seen += Format(B) + ';';
             if seen <> 'Yes;No;' then Error('down %1', seen);
         end;",
    );
}
