//! Rec.Find(), FindFirst(), FindLast() and Count() in the OnOpenPage of a page an action opens through RunPageLink (al-runner
//! issue 5860). Pinned by pages/lt-rz-outside-page-moves/SjRunPageLinkOpenFind.al: Rec holds the link as a filter with only the
//! linked field filled, Find() looks for that primary key, FindFirst/FindLast move Rec, and the page opens on that row.

use al_testkit::{blocked, compile};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

fn run(src: &str, codeunit: &str) -> Vec<(String, Outcome)> {
    let l = compile(src);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut Vec::new(),
        None,
    )
}

const SRC: &str = r#"
table 50100 "Link Item" { fields { field(1; "Code"; Code[20]) { } field(2; Description; Text[30]) { } } keys { key(PK; "Code") { } } }
table 50101 "Link Entry"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Item Code"; Code[20]) { } }
    keys { key(PK; "Entry No.") { } }
}
codeunit 50102 "Link Log"
{
    SingleInstance = true;
    var Buf: Text;
    procedure Add(T: Text) begin Buf += T + ';'; end;
    procedure Get(): Text begin exit(Buf); end;
    procedure Clear() begin Buf := ''; end;
}
page 50103 "Link Items"
{
    PageType = List; SourceTable = "Link Item"; Editable = false;
    layout { area(Content) { repeater(R) { field("Code"; Rec."Code") { } } } }
    actions
    {
        area(Processing)
        {
            action(DescFind) { RunObject = page "Entries Desc Find"; RunPageLink = "Item Code" = field("Code"); }
            action(DescFindLast) { RunObject = page "Entries Desc FindLast"; RunPageLink = "Item Code" = field("Code"); }
            action(Plain) { RunObject = page "Entries Plain"; RunPageLink = "Item Code" = field("Code"); }
            action(ItemCard) { RunObject = page "Link Item Card"; RunPageLink = "Code" = field("Code"); }
        }
    }
}
page 50104 "Entries Desc Find"
{
    PageType = List; SourceTable = "Link Entry"; SourceTableView = sorting("Entry No.") order(descending); Editable = false;
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } } } }
    trigger OnOpenPage()
    var Log: Codeunit "Link Log"; Found: Boolean;
    begin
        Log.Add('count=' + Format(Rec.Count()));
        Found := Rec.Find();
        Log.Add('find=' + Format(Found) + ' row=' + Format(Rec."Entry No.") + '|' + Rec."Item Code");
        if (Rec.GetFilters() <> '') and not Found then
            Log.Add('findfirst=' + Format(Rec.FindFirst()) + ' row=' + Format(Rec."Entry No.") + '|' + Rec."Item Code");
    end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "Link Log"; begin Log.Add('agcr ' + Format(Rec."Entry No.")); end;
}
page 50108 "Entries Plain"
{
    PageType = List; SourceTable = "Link Entry"; SourceTableView = sorting("Entry No.") order(descending); Editable = false;
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } } } }
    trigger OnAfterGetCurrRecord() var Log: Codeunit "Link Log"; begin Log.Add('agcr ' + Format(Rec."Entry No.")); end;
}
page 50105 "Entries Desc FindLast"
{
    PageType = List; SourceTable = "Link Entry"; SourceTableView = sorting("Entry No.") order(descending); Editable = false;
    layout { area(Content) { repeater(R) { field("Entry No."; Rec."Entry No.") { } } } }
    trigger OnOpenPage()
    var Log: Codeunit "Link Log";
    begin
        Log.Add('findlast=' + Format(Rec.FindLast()) + ' row=' + Format(Rec."Entry No."));
    end;
}
page 50106 "Link Item Card"
{
    PageType = Card; SourceTable = "Link Item"; Editable = false;
    layout { area(Content) { field("Code"; Rec."Code") { } field(Description; Rec.Description) { } } }
    trigger OnOpenPage()
    var Log: Codeunit "Link Log";
    begin
        Log.Add('find=' + Format(Rec.Find()) + ' row=' + Rec."Code" + '|' + Rec.Description);
    end;
}
codeunit 50107 "Open Find Tests"
{
    Subtype = Test;
    var Log: Codeunit "Link Log";

    local procedure OpenOn(var Items: TestPage "Link Items"; ItemCode: Code[20])
    var Item: Record "Link Item"; Entry: Record "Link Entry";
    begin
        Entry.DeleteAll(); Item.DeleteAll();
        Item."Code" := 'I1'; Item.Description := 'one'; Item.Insert();
        Item."Code" := 'I2'; Item.Description := 'two'; Item.Insert();
        Item."Code" := 'I3'; Item.Description := 'three'; Item.Insert();
        Entry."Entry No." := 1; Entry."Item Code" := 'I1'; Entry.Insert();
        Entry."Entry No." := 2; Entry."Item Code" := 'I2'; Entry.Insert();
        Entry."Entry No." := 3; Entry."Item Code" := 'I1'; Entry.Insert();
        Entry."Entry No." := 4; Entry."Item Code" := 'I1'; Entry.Insert();
        Items.OpenView();
        Items.GoToKey(ItemCode);
        Log.Clear();
    end;

    local procedure Rows(var Entries: TestPage "Entries Desc Find") Result: Text
    begin
        if Entries.First() then
            repeat Result += Entries."Entry No.".Value() + ','; until not Entries.Next();
    end;

    local procedure Check(Want: Text; Got: Text)
    begin
        if Want <> Got then Error('want <%1> got <%2>', Want, Got);
    end;

    [Test]
    [HandlerFunctions('DescHandler')]
    procedure DescFindHandler()
    var Items: TestPage "Link Items";
    begin
        OpenOn(Items, 'I1');
        Items.DescFind.Invoke();
        Check('count=3;find=No row=0|I1;findfirst=Yes row=4|I1;agcr 4;agcr 3;agcr 1;rows=4,3,1,;', Log.Get());
    end;

    [Test]
    [HandlerFunctions('PlainHandler')]
    procedure PlainHandlerT()
    var Items: TestPage "Link Items";
    begin
        OpenOn(Items, 'I1');
        Items.Plain.Invoke();
        Check('agcr 4;agcr 3;agcr 1;rows=4,3,1,;', Log.Get());
    end;

    [PageHandler]
    procedure DescHandler(var Entries: TestPage "Entries Desc Find")
    begin
        Log.Add('rows=' + Rows(Entries));
    end;

    [PageHandler]
    procedure PlainHandler(var Entries: TestPage "Entries Plain")
    var R: Text;
    begin
        if Entries.First() then
            repeat R += Entries."Entry No.".Value() + ','; until not Entries.Next();
        Log.Add('rows=' + R);
    end;

    [Test]
    procedure DescFindTrap()
    var Items: TestPage "Link Items"; Entries: TestPage "Entries Desc Find";
    begin
        OpenOn(Items, 'I1');
        Entries.Trap();
        Items.DescFind.Invoke();
        Check('count=3;find=No row=0|I1;findfirst=Yes row=4|I1;agcr 4;', Log.Get());
        Check('4', Entries."Entry No.".Value());
        Check('4,3,1,', Rows(Entries));
    end;

    [Test]
    procedure DescFindNoRows()
    var Items: TestPage "Link Items"; Entries: TestPage "Entries Desc Find";
    begin
        OpenOn(Items, 'I3');
        Entries.Trap();
        Items.DescFind.Invoke();
        Check('count=0;find=No row=0|I3;findfirst=No row=0|I3;', Log.Get());
        Check('', Rows(Entries));
    end;

    [Test]
    procedure DescFindLastTrap()
    var Items: TestPage "Link Items"; Entries: TestPage "Entries Desc FindLast";
    begin
        OpenOn(Items, 'I1');
        Entries.Trap();
        Items.DescFindLast.Invoke();
        Check('findlast=Yes row=1;', Log.Get());
        Check('1', Entries."Entry No.".Value());
    end;

    [Test]
    procedure CardFindTrap()
    var Items: TestPage "Link Items"; Card: TestPage "Link Item Card";
    begin
        OpenOn(Items, 'I2');
        Card.Trap();
        Items.ItemCard.Invoke();
        Check('find=Yes row=I2|two;', Log.Get());
        Check('I2|two', Card."Code".Value() + '|' + Card.Description.Value());
    end;
}
"#;

/// SjRunPageLinkOpenFind.al:314-334 (Trap, descending view), :337-350 (PageHandler: First() on the row the page opened on
/// runs no OnAfterGetCurrRecord), :353-365 (no linked row), :413-425 (FindLast), :427-450 (Card).
#[test]
fn find_findfirst_findlast_and_count_in_onopenpage_of_a_run_page_link_target() {
    let out = run(SRC, "Open Find Tests");
    for t in [
        "DescFindHandler",
        "DescFindTrap",
        "DescFindNoRows",
        "DescFindLastTrap",
        "CardFindTrap",
    ] {
        let o = out.iter().find(|(n, _)| n == t).map(|(_, o)| o.clone()).unwrap();
        assert_eq!(o, Outcome::Pass, "{t}");
    }
}

/// A RunPageLink list with no OnOpenPage: whether First() on the row it opened on runs OnAfterGetCurrRecord is not pinned (the
/// corpus pins it only where OnOpenPage moved Rec, SjRunPageLinkOpenFind.al:337-410), so the case is refused.
#[test]
fn first_on_a_run_page_link_list_whose_onopenpage_did_not_move_rec_is_refused() {
    let out = run(SRC, "Open Find Tests");
    let o = out
        .iter()
        .find(|(n, _)| n == "PlainHandlerT")
        .map(|(_, o)| o.clone())
        .unwrap();
    match o {
        Outcome::NotRun(why) => assert!(why.contains("did not move Rec"), "{why}"),
        other => panic!("PlainHandlerT: {other:?}"),
    }
}
