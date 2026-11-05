//! Rec.SetView() in the OnOpenPage of a list an action opens through RunPageLink (al-runner issue 7089, corpus issue 712). The AL
//! below is corpus testpage/RunPageLinkOpenSetView.al (branch al-runtime-rs-staging-r7089, corpus PR 1150, BC 27.5, 28.5 and 29.0
//! alike) with each observation turned into a check of the value BC gave: before SetView Rec holds the link in filter group 0
//! (a RunPageView where filter in group 3, the target's SourceTableView where filter in group 2), its linked fields are filled
//! and the target's SourceTableView key stays; SetView replaces the view of group 0, the link included, and leaves the other
//! groups; the page lists the rows of the new view.

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
table 95870 "LT RSV Host"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Code[10]) { }
        field(2; "Transport no."; Code[10]) { }
    }

    keys { key(PK; Id) { Clustered = true; } }
}

table 95871 "LT RSV Work Order"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; ID; Integer) { }
        field(2; RelationType; Option) { OptionMembers = Inbound,Outbound; }
        field(3; Transport; Code[10]) { }
        field(4; Information; Text[30]) { }
    }

    keys
    {
        key(PK; ID) { Clustered = true; }
        key(Key2; RelationType, ID) { }
    }
}

// The target, like the reporter's work order list: OnOpenPage applies a view with SetView.
page 95873 "LT RSV List"
{
    PageType = List;
    SourceTable = "LT RSV Work Order";
    SourceTableView = sorting(ID);
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(ID; Rec.ID) { ApplicationArea = All; }
                field(Information; Rec.Information) { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    var
        Log: Codeunit "LT RSV Log";
        RecRef: RecordRef;
        FieldRef: FieldRef;
    begin
        if Log.Groups() then
            Log.Add('groups before=' + GroupFilters());
        case Log.Mode() of
            'none':
                Log.Add('before ' + State());
            'same':
                Rec.SetView(Rec.GetView());
            'sameref':
                begin
                    RecRef.GetTable(Rec);
                    Rec.SetView(RecRef.GetView());
                end;
            'extra':
                Rec.SetView('SORTING(ID) WHERE(Information=FILTER(MINE|OTHER-T))');
            'empty':
                Rec.SetView('');
            'sortonly':
                Rec.SetView('SORTING(ID) ORDER(Descending)');
            'linkother':
                Rec.SetView('WHERE(Transport=FILTER(T2))');
            'saved':
                begin
                    RecRef.GetTable(Rec);
                    RecRef.SetView('SORTING(ID) ORDER(Ascending)');
                    FieldRef := RecRef.Field(1);
                    FieldRef.SetFilter('>0');
                    if RecRef.FindFirst() then;
                    Rec.SetView(RecRef.GetView());
                end;
        end;
        if Log.Mode() <> 'none' then
            Log.Add('after ' + State());
        if Log.Groups() then
            Log.Add('groups after=' + GroupFilters());
    end;

    local procedure State(): Text
    begin
        exit('filters=' + Rec.GetFilters() + ' view=' + Rec.GetView() + ' count=' + Format(Rec.Count()) + ' key=' + Rec.CurrentKey() +
          ' asc=' + Format(Rec.Ascending()) + ' rec=' + Format(Rec.ID) + '/' + Format(Rec.RelationType) + '/' + Rec.Transport + '/' + Rec.Information);
    end;

    local procedure GroupFilters(): Text
    var
        Group: Integer;
        Result: Text;
    begin
        for Group := 0 to 10 do begin
            Rec.FilterGroup(Group);
            if Rec.GetFilters() <> '' then
                Result += Format(Group) + ':' + Rec.GetFilters() + '|';
        end;
        Rec.FilterGroup(0);
        exit(Result);
    end;
}

// A target whose SourceTableView holds a where filter.
page 95874 "LT RSV List Where"
{
    PageType = List;
    SourceTable = "LT RSV Work Order";
    SourceTableView = sorting(ID) where(Information = filter(<> 'HIDDEN'));
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(ID; Rec.ID) { ApplicationArea = All; }
                field(Information; Rec.Information) { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    var
        Log: Codeunit "LT RSV Log";
    begin
        if Log.Groups() then
            Log.Add('groups before=' + GroupFilters());
        case Log.Mode() of
            'none':
                Log.Add('before ' + State());
            'same':
                Rec.SetView(Rec.GetView());
            'empty':
                Rec.SetView('');
        end;
        if Log.Mode() <> 'none' then
            Log.Add('after ' + State());
        if Log.Groups() then
            Log.Add('groups after=' + GroupFilters());
    end;

    local procedure State(): Text
    begin
        exit('filters=' + Rec.GetFilters() + ' view=' + Rec.GetView() + ' count=' + Format(Rec.Count()) + ' key=' + Rec.CurrentKey() +
          ' asc=' + Format(Rec.Ascending()) + ' rec=' + Format(Rec.ID) + '/' + Format(Rec.RelationType) + '/' + Rec.Transport + '/' + Rec.Information);
    end;

    local procedure GroupFilters(): Text
    var
        Group: Integer;
        Result: Text;
    begin
        for Group := 0 to 10 do begin
            Rec.FilterGroup(Group);
            if Rec.GetFilters() <> '' then
                Result += Format(Group) + ':' + Rec.GetFilters() + '|';
        end;
        Rec.FilterGroup(0);
        exit(Result);
    end;
}

// The host, like the reporter's card: actions open the list through RunPageLink.
page 95875 "LT RSV Host Card"
{
    PageType = Card;
    SourceTable = "LT RSV Host";
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(Id; Rec.Id) { ApplicationArea = All; }
            field("Transport no."; Rec."Transport no.") { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Processing)
        {
            action(LinkAndView)
            {
                ApplicationArea = All;
                RunObject = page "LT RSV List";
                RunPageLink = RelationType = const(Inbound), Transport = field("Transport no.");
                RunPageView = sorting(RelationType, ID);
            }
            action(LinkOnly)
            {
                ApplicationArea = All;
                RunObject = page "LT RSV List";
                RunPageLink = RelationType = const(Inbound), Transport = field("Transport no.");
            }
            action(LinkAndWhereView)
            {
                ApplicationArea = All;
                RunObject = page "LT RSV List";
                RunPageLink = RelationType = const(Inbound), Transport = field("Transport no.");
                RunPageView = where(Information = filter(<> 'HIDDEN'));
            }
            action(LinkToWhereList)
            {
                ApplicationArea = All;
                RunObject = page "LT RSV List Where";
                RunPageLink = RelationType = const(Inbound), Transport = field("Transport no.");
            }
        }
    }
}

codeunit 95872 "LT RSV Log"
{
    SingleInstance = true;

    var
        Entries: Text;
        CurrentMode: Text;
        WithGroups: Boolean;

    procedure Start(NewMode: Text; NewGroups: Boolean)
    begin
        Entries := '';
        CurrentMode := NewMode;
        WithGroups := NewGroups;
    end;

    procedure Mode(): Text
    begin
        exit(CurrentMode);
    end;

    procedure Groups(): Boolean
    begin
        exit(WithGroups);
    end;

    procedure Add(Value: Text)
    begin
        Entries += Value + ';';
    end;

    procedure Get(): Text
    begin
        exit(Entries);
    end;
}

codeunit 95876 "LT RSV RunPageLink SetView"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Log: Codeunit "LT RSV Log";

    [Test]
    procedure RSV_LinkAndView_NoSetView()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'none', false);
        if Rows <> 'MINE,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'before filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_NoSetView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'none', true);
        if Rows <> 'MINE,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|;before filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;groups after=0:RelationType: Inbound, Transport: T1|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_SameView()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'same', false);
        if Rows <> 'MINE,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_SameView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'same', true);
        if Rows <> 'MINE,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|;after filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;groups after=0:RelationType: Inbound, Transport: T1|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_SameViewOfRecordRef()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'sameref', false);
        if Rows <> 'MINE,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_OtherFilter()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'extra', false);
        if Rows <> 'MINE,OTHER-T,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters=Information: MINE|OTHER-T view=VERSION(1) SORTING(ID) WHERE(Information=FILTER(MINE|OTHER-T)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_OtherFilter_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'extra', true);
        if Rows <> 'MINE,OTHER-T,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|;after filters=Information: MINE|OTHER-T view=VERSION(1) SORTING(ID) WHERE(Information=FILTER(MINE|OTHER-T)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;groups after=0:Information: MINE|OTHER-T|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_EmptyView()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'empty', false);
        if Rows <> 'MINE,OTHER-T,OTHER-O,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters= view=VERSION(1) SORTING(ID) count=4 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_EmptyView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'empty', true);
        if Rows <> 'MINE,OTHER-T,OTHER-O,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|;after filters= view=VERSION(1) SORTING(ID) count=4 key=ID asc=Yes rec=0/Inbound/T1/;groups after=;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_SortingOnly()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'sortonly', false);
        if Rows <> 'HIDDEN,OTHER-O,OTHER-T,MINE,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters= view=VERSION(1) SORTING(ID) ORDER(Descending) count=4 key=ID asc=No rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_LinkedFieldOtherValue()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'linkother', false);
        if Rows <> 'OTHER-T,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters=Transport: T2 view=VERSION(1) SORTING(ID) WHERE(Transport=FILTER(T2)) count=1 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndView_ViewOfACopyWithItsOwnFilter()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndView', 'saved', false);
        if Rows <> 'MINE,OTHER-T,OTHER-O,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters=ID: >0 view=VERSION(1) SORTING(ID) WHERE(ID=FILTER(>0)) count=4 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkOnly_NoSetView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkOnly', 'none', true);
        if Rows <> 'MINE,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|;before filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;groups after=0:RelationType: Inbound, Transport: T1|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkOnly_SameView()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkOnly', 'same', false);
        if Rows <> 'MINE,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkOnly_OtherFilter()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkOnly', 'extra', false);
        if Rows <> 'MINE,OTHER-T,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters=Information: MINE|OTHER-T view=VERSION(1) SORTING(ID) WHERE(Information=FILTER(MINE|OTHER-T)) count=2 key=ID asc=Yes rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkOnly_EmptyView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkOnly', 'empty', true);
        if Rows <> 'MINE,OTHER-T,OTHER-O,HIDDEN,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|;after filters= view=VERSION(1) SORTING(ID) count=4 key=ID asc=Yes rec=0/Inbound/T1/;groups after=;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkOnly_SortingOnly()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkOnly', 'sortonly', false);
        if Rows <> 'HIDDEN,OTHER-O,OTHER-T,MINE,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'after filters= view=VERSION(1) SORTING(ID) ORDER(Descending) count=4 key=ID asc=No rec=0/Inbound/T1/;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndWhereView_NoSetView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndWhereView', 'none', true);
        if Rows <> 'MINE,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|3:Information: <>HIDDEN|;before filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=1 key=ID asc=Yes rec=0/Inbound/T1/;groups after=0:RelationType: Inbound, Transport: T1|3:Information: <>HIDDEN|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndWhereView_SameView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndWhereView', 'same', true);
        if Rows <> 'MINE,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|3:Information: <>HIDDEN|;after filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=1 key=ID asc=Yes rec=0/Inbound/T1/;groups after=0:RelationType: Inbound, Transport: T1|3:Information: <>HIDDEN|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkAndWhereView_EmptyView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenList('LinkAndWhereView', 'empty', true);
        if Rows <> 'MINE,OTHER-T,OTHER-O,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|3:Information: <>HIDDEN|;after filters= view=VERSION(1) SORTING(ID) count=3 key=ID asc=Yes rec=0/Inbound/T1/;groups after=3:Information: <>HIDDEN|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkToWhereList_NoSetView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenWhereList('none', true);
        if Rows <> 'MINE,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|2:Information: <>HIDDEN|;before filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=1 key=ID asc=Yes rec=0/Inbound/T1/;groups after=0:RelationType: Inbound, Transport: T1|2:Information: <>HIDDEN|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkToWhereList_SameView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenWhereList('same', true);
        if Rows <> 'MINE,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|2:Information: <>HIDDEN|;after filters=RelationType: Inbound, Transport: T1 view=VERSION(1) SORTING(ID) WHERE(RelationType=FILTER(Inbound),Transport=FILTER(T1)) count=1 key=ID asc=Yes rec=0/Inbound/T1/;groups after=0:RelationType: Inbound, Transport: T1|2:Information: <>HIDDEN|;' then Error('log <%1>', Log.Get());
    end;

    [Test]
    procedure RSV_LinkToWhereList_EmptyView_Groups()
    var
        Rows: Text;
    begin
        Rows := OpenWhereList('empty', true);
        if Rows <> 'MINE,OTHER-T,OTHER-O,' then Error('rows <%1>', Rows);
        if Log.Get() <> 'groups before=0:RelationType: Inbound, Transport: T1|2:Information: <>HIDDEN|;after filters= view=VERSION(1) SORTING(ID) count=3 key=ID asc=Yes rec=0/Inbound/T1/;groups after=2:Information: <>HIDDEN|;' then Error('log <%1>', Log.Get());
    end;

    local procedure OpenList(ActionName: Text; Mode: Text; Groups: Boolean) Rows: Text
    var
        Card: TestPage "LT RSV Host Card";
        List: TestPage "LT RSV List";
    begin
        Setup(Mode, Groups);
        Card.OpenView();
        Card.GoToKey('H1');
        List.Trap();
        case ActionName of
            'LinkAndView':
                Card.LinkAndView.Invoke();
            'LinkOnly':
                Card.LinkOnly.Invoke();
            'LinkAndWhereView':
                Card.LinkAndWhereView.Invoke();
        end;
        if List.First() then
            repeat
                Rows += List.Information.Value() + ',';
            until not List.Next();
        List.Close();
        Card.Close();
    end;

    local procedure OpenWhereList(Mode: Text; Groups: Boolean) Rows: Text
    var
        Card: TestPage "LT RSV Host Card";
        List: TestPage "LT RSV List Where";
    begin
        Setup(Mode, Groups);
        Card.OpenView();
        Card.GoToKey('H1');
        List.Trap();
        Card.LinkToWhereList.Invoke();
        if List.First() then
            repeat
                Rows += List.Information.Value() + ',';
            until not List.Next();
        List.Close();
        Card.Close();
    end;

    local procedure Setup(Mode: Text; Groups: Boolean)
    var
        Host: Record "LT RSV Host";
        WorkOrder: Record "LT RSV Work Order";
    begin
        Host.DeleteAll();
        WorkOrder.DeleteAll();
        Host.Id := 'H1';
        Host."Transport no." := 'T1';
        Host.Insert();
        AddWorkOrder(1, WorkOrder.RelationType::Inbound, 'T1', 'MINE');
        AddWorkOrder(2, WorkOrder.RelationType::Inbound, 'T2', 'OTHER-T');
        AddWorkOrder(3, WorkOrder.RelationType::Outbound, 'T1', 'OTHER-O');
        AddWorkOrder(4, WorkOrder.RelationType::Inbound, 'T1', 'HIDDEN');
        Log.Start(Mode, Groups);
    end;

    local procedure AddWorkOrder(Id: Integer; Relation: Option; Transport: Code[10]; Info: Text[30])
    var
        WorkOrder: Record "LT RSV Work Order";
    begin
        WorkOrder.ID := Id;
        WorkOrder.RelationType := Relation;
        WorkOrder.Transport := Transport;
        WorkOrder.Information := Info;
        WorkOrder.Insert();
    end;
}
"#;

#[test]
fn set_view_in_onopenpage_of_a_run_page_link_target_replaces_the_link() {
    let out = run(SRC, "LT RSV RunPageLink SetView");
    assert_eq!(out.len(), 23, "{out:?}");
    let bad: Vec<_> = out.iter().filter(|(_, o)| *o != Outcome::Pass).collect();
    assert!(bad.is_empty(), "{bad:#?}");
}
