//! Pages without a source table: the shapes next to the ones events/page-events/NoSourcePageEvents.al pins keep the earlier
//! behavior or are refused (al-runner PR 7787 review). A ModalPageHandler error on a Card or a List closes nothing (the close
//! triggers run only on a NavigatePage, a CardPart and a ListPart, :1149, :1349, :1549); Next() after OpenView is refused (every
//! navigation pin opens with OpenEdit, :3084-:3348), and so are Next() and New() on a Card a Page variable ran and the test
//! trapped, and Next() in a ModalPageHandler; a Card over no table whose parts have a source table closes them in the
//! declared order (the reversed order is pinned for parts over no table, :2964); an action after a SetValue on an expression
//! control of a List or a Worksheet is refused (pinned on a Card and a NavigatePage only).

use al_testkit::outcomes_in;
use al_testrun::Outcome;

const SRC: &str = r#"
table 69960 "NSN Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 69961 "NSN Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Add(T: Text) begin L += T + ';'; end;
    procedure Get(): Text begin exit(L); end;
    procedure Clear() begin L := ''; end;
}
page 69962 "NSN Card"
{
    PageType = Card;
    layout { area(Content) { field(F; V) { } } }
    trigger OnOpenPage() begin Log.Add('open'); end;
    trigger OnQueryClosePage(A: Action): Boolean begin Log.Add('qcp'); exit(true); end;
    trigger OnClosePage() begin Log.Add('close'); end;
    var Log: Codeunit "NSN Log"; V: Integer;
}
page 69963 "NSN List"
{
    PageType = List;
    layout { area(Content) { field(A1; Arr[1]) { } field(Shown; Other) { } } }
    actions { area(Processing) { action(Go) { trigger OnAction() begin Other := Arr[1] + '!'; end; } } }
    trigger OnOpenPage() begin Log.Add('open'); end;
    trigger OnQueryClosePage(A: Action): Boolean begin Log.Add('qcp'); exit(true); end;
    trigger OnClosePage() begin Log.Add('close'); end;
    var Log: Codeunit "NSN Log"; Arr: array[2] of Text; Other: Text;
}
page 69964 "NSN Worksheet"
{
    PageType = Worksheet;
    layout { area(Content) { field(A1; Arr[1]) { } field(Shown; Other) { } } }
    actions { area(Processing) { action(Go) { trigger OnAction() begin Other := Arr[1] + '!'; end; } } }
    var Arr: array[2] of Text; Other: Text;
}
page 69965 "NSN PartA"
{
    PageType = ListPart; SourceTable = "NSN Row";
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
    trigger OnQueryClosePage(A: Action): Boolean begin Log.Add('A.qcp'); exit(true); end;
    trigger OnClosePage() begin Log.Add('A.close'); end;
    var Log: Codeunit "NSN Log";
}
page 69966 "NSN PartB"
{
    PageType = ListPart; SourceTable = "NSN Row";
    layout { area(Content) { repeater(R) { field(Id; Rec.Id) { } } } }
    trigger OnQueryClosePage(A: Action): Boolean begin Log.Add('B.qcp'); exit(true); end;
    trigger OnClosePage() begin Log.Add('B.close'); end;
    var Log: Codeunit "NSN Log";
}
page 69967 "NSN Host"
{
    PageType = Card;
    layout { area(Content) { part(A; "NSN PartA") { } part(B; "NSN PartB") { } } }
    trigger OnClosePage() begin Log.Add('H.close'); end;
    var Log: Codeunit "NSN Log";
}
codeunit 69968 T
{
    Subtype = Test;
    var Log: Codeunit "NSN Log";
    local procedure Check(Want: Text; Got: Text) begin if Want <> Got then Error('want [%1] got [%2]', Want, Got); end;

    [Test] [HandlerFunctions('CardErr')] procedure CardHandlerErrorClosesNothing()
    begin
        Log.Clear();
        asserterror Page.RunModal(Page::"NSN Card");
        Check('boom', GetLastErrorText());
        Check('open;handler;', Log.Get());
    end;
    [ModalPageHandler] procedure CardErr(var Pg: TestPage "NSN Card") begin Log.Add('handler'); Error('boom'); end;

    [Test] [HandlerFunctions('ListErr')] procedure ListHandlerErrorClosesNothing()
    begin
        Log.Clear();
        asserterror Page.RunModal(Page::"NSN List");
        Check('boom', GetLastErrorText());
        Check('open;handler;', Log.Get());
    end;
    [ModalPageHandler] procedure ListErr(var Pg: TestPage "NSN List") begin Log.Add('handler'); Error('boom'); end;

    [Test] procedure CardNextAfterOpenViewRefused()
    var Pg: TestPage "NSN Card";
    begin
        Pg.OpenView();
        Pg.Next();
        Pg.Close();
    end;

    [Test] procedure CardNextOnTrappedRunRefused()
    var Pg: TestPage "NSN Card"; PgVar: Page "NSN Card";
    begin
        Pg.Trap(); PgVar.Run();
        Pg.Next();
        Pg.Close();
    end;

    [Test] procedure CardNewOnTrappedRunRefused()
    var Pg: TestPage "NSN Card"; PgVar: Page "NSN Card";
    begin
        Pg.Trap(); PgVar.Run();
        asserterror Pg.New();
        Pg.Close();
    end;

    [Test] [HandlerFunctions('CardNext')] procedure CardNextInModalHandlerRefused()
    begin
        Page.RunModal(Page::"NSN Card");
    end;
    [ModalPageHandler] procedure CardNext(var Pg: TestPage "NSN Card") begin Pg.Next(); Pg.OK().Invoke(); end;

    [Test] procedure HostWithSourcePartsKeepsDeclaredOrder()
    var Pg: TestPage "NSN Host";
    begin
        Pg.OpenView();
        Log.Clear();
        Pg.Close();
        Check('A.qcp;B.qcp;A.close;B.close;H.close;', Log.Get());
    end;

    [Test] procedure ListSetValueThenActionRefused()
    var Pg: TestPage "NSN List";
    begin
        Pg.OpenEdit();
        Pg.A1.SetValue('x');
        Pg.Go.Invoke();
        Check('x!', Pg.Shown.Value);
    end;

    [Test] procedure WorksheetSetValueThenActionRefused()
    var Pg: TestPage "NSN Worksheet";
    begin
        Pg.OpenEdit();
        Pg.A1.SetValue('x');
        Pg.Go.Invoke();
        Check('x!', Pg.Shown.Value);
    end;
}
"#;

#[test]
fn neighbors_of_the_pinned_no_source_shapes_keep_the_earlier_behavior_or_refuse() {
    let out = outcomes_in(SRC, "T");
    let get = |n: &str| out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap();
    let mut wrong = Vec::new();
    for pass in [
        "CardHandlerErrorClosesNothing",
        "ListHandlerErrorClosesNothing",
        "HostWithSourcePartsKeepsDeclaredOrder",
    ] {
        if get(pass) != Outcome::Pass {
            wrong.push(format!("{pass}: {:?}", get(pass)));
        }
    }
    for refused in [
        "CardNextAfterOpenViewRefused",
        "CardNextOnTrappedRunRefused",
        "CardNewOnTrappedRunRefused",
        "CardNextInModalHandlerRefused",
        "ListSetValueThenActionRefused",
        "WorksheetSetValueThenActionRefused",
    ] {
        if !matches!(get(refused), Outcome::NotRun(_)) {
            wrong.push(format!("{refused}: {:?}", get(refused)));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
