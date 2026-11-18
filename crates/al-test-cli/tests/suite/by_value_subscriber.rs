//! An event subscriber's by-value Record parameter (al-runner issue 8295). Corpus PR 1430 measured one shape
//! (record/filters/ByValueRecordView.al:286-308, :964-978): a codeunit's IntegrationEvent(false, false) with a by-value
//! Record parameter, raised by a call of the publisher, to an automatic subscriber codeunit whose parameter is by value, the
//! caller's record a stored one with a filter in filter group 0 and the plain view. That subscriber's copy has no filters
//! (`sub f={} n=3`), flagged by assumption `by-value-record-no-filters`. Every neighbor whose publisher parameter is by value,
//! with a filter or another view, refuses a read of the copy's view; a copy with the view of a fresh record, a read that is
//! not of the view, and a copy whose publisher parameter is `var` (also Rec of a platform table event) keep main.

use crate::record_copy_reports::{PASS, VIEW_REFUSED, check, run};

const SUBSCRIBER: &str = r##"
table 50170 "BS Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[20]) { }
        field(3; Status; Option) { OptionMembers = Open,Posted; }
        field(4; "Location Filter"; Code[10]) { FieldClass = FlowFilter; }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
        key(ByName; Name) { }
    }

    procedure RaiseOwn(var R: Record "BS Row")
    begin
        OnTableThing(R);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnTableThing(R: Record "BS Row")
    begin
    end;

    procedure RaiseOwnVar(var R: Record "BS Row")
    begin
        OnTableVar(R);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnTableVar(var R: Record "BS Row")
    begin
    end;
}

codeunit 50171 "BS Log"
{
    SingleInstance = true;

    var
        Log: Text;

    procedure Add(Line: Text)
    begin
        Log += Line + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Log;
        Log := '';
        exit(Result);
    end;
}

codeunit 50172 "BS Publisher"
{
    var
        Shared: Integer;

    procedure Raise(var R: Record "BS Row")
    begin
        OnThing(R);
    end;

    procedure RaiseBusiness(var R: Record "BS Row")
    begin
        OnBusiness(R);
    end;

    procedure RaiseInternal(var R: Record "BS Row")
    begin
        OnInternal(R);
    end;

    procedure RaiseIsolated(var R: Record "BS Row")
    begin
        OnIsolated(R);
    end;

    procedure RaiseGlobal(var R: Record "BS Row")
    begin
        OnGlobal(R);
    end;

    procedure RaiseSender(var R: Record "BS Row")
    begin
        OnSender(R);
    end;

    procedure RaiseVar(var R: Record "BS Row")
    begin
        OnVar(R);
    end;

    procedure RaiseManual(var R: Record "BS Row")
    begin
        OnManual(R);
    end;

    procedure RaiseSingle(var R: Record "BS Row")
    begin
        OnSingle(R);
    end;

    procedure RaiseCommitBehavior(var R: Record "BS Row")
    begin
        OnCommitBehavior(R);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnThing(R: Record "BS Row")
    begin
    end;

    [BusinessEvent(false)]
    local procedure OnBusiness(R: Record "BS Row")
    begin
    end;

    [InternalEvent(false)]
    local procedure OnInternal(R: Record "BS Row")
    begin
    end;

    [IntegrationEvent(false, false, true)]
    local procedure OnIsolated(R: Record "BS Row")
    begin
    end;

    [IntegrationEvent(false, true)]
    local procedure OnGlobal(R: Record "BS Row")
    begin
    end;

    [IntegrationEvent(true, false)]
    local procedure OnSender(R: Record "BS Row")
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnVar(var R: Record "BS Row")
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnManual(R: Record "BS Row")
    begin
    end;

    [IntegrationEvent(false, false)]
    local procedure OnSingle(R: Record "BS Row")
    begin
    end;

    [CommitBehavior(CommitBehavior::Ignore)]
    [IntegrationEvent(false, false)]
    local procedure OnCommitBehavior(R: Record "BS Row")
    begin
    end;
}

codeunit 50177 "BS Single Publisher"
{
    SingleInstance = true;

    procedure Raise(var R: Record "BS Row")
    begin
        OnSinglePublisher(R);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnSinglePublisher(R: Record "BS Row")
    begin
    end;
}

codeunit 50173 "BS Subscriber"
{
    local procedure Describe(Tag: Text; R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add(Tag + ' f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnThing', '', false, false)]
    local procedure HandleThing(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('sub f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnBusiness', '', false, false)]
    local procedure HandleBusiness(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        if R.Name = 'fields only' then
            Log.Add('name=' + R.Name)
        else
            Log.Add('business f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnInternal', '', false, false)]
    local procedure HandleInternal(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('internal f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnIsolated', '', false, false)]
    local procedure HandleIsolated(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('isolated f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnGlobal', '', false, false)]
    local procedure HandleGlobal(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('global f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnSender', '', false, false)]
    local procedure HandleSender(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('sender f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnVar', '', false, false)]
    local procedure HandleVar(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('var f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Table, Database::"BS Row", 'OnTableThing', '', false, false)]
    local procedure HandleTableThing(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('table f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Table, Database::"BS Row", 'OnTableVar', '', false, false)]
    local procedure HandleTableVar(R: Record "BS Row")
    var
        Other: Record "BS Row";
        Log: Codeunit "BS Log";
    begin
        R.CopyFilter("Location Filter", Other."Location Filter");
        Log.Add('tablevar f=' + R.GetFilter("Location Filter") + ' copied=' + Other.GetFilter("Location Filter") + ' n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnCommitBehavior', '', false, false)]
    local procedure HandleCommitBehavior(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('commit f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Single Publisher", 'OnSinglePublisher', '', false, false)]
    local procedure HandleSinglePublisher(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('singlepub f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;

    [EventSubscriber(ObjectType::Table, Database::"BS Row", 'OnAfterModifyEvent', '', false, false)]
    local procedure HandleModify(Rec: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('modify f={' + Rec.GetFilters() + '} n=' + Format(Rec.Count()));
    end;
}

codeunit 50174 "BS Manual Subscriber"
{
    EventSubscriberInstance = Manual;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnManual', '', false, false)]
    local procedure HandleManual(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('manual f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;
}

codeunit 50175 "BS Single Subscriber"
{
    SingleInstance = true;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BS Publisher", 'OnSingle', '', false, false)]
    local procedure HandleSingle(R: Record "BS Row")
    var
        Log: Codeunit "BS Log";
    begin
        Log.Add('single f={' + R.GetFilters() + '} n=' + Format(R.Count()));
    end;
}

codeunit 50176 "BS Test"
{
    Subtype = Test;

    var
        Publisher: Codeunit "BS Publisher";
        Log: Codeunit "BS Log";

    local procedure Seed(var Row: Record "BS Row")
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Name := 'c';
        Row.Status := Row.Status::Open;
        Row.Insert();
        Row.Id := 2;
        Row.Name := 'b';
        Row.Status := Row.Status::Posted;
        Row.Insert();
        Row.Id := 3;
        Row.Name := 'a';
        Row.Status := Row.Status::Open;
        Row.Insert();
        Row.Reset();
        Log.Take();
    end;

    local procedure Expect(Want: Text)
    var
        Got: Text;
    begin
        Got := Log.Take();
        if Got <> Want then
            Error('%1', Got);
    end;

    [Test]
    procedure Integration_Group0Filter_Flagged()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.Raise(Row);
        Expect('sub f={} n=3;');
        if Row.GetFilters() <> 'Status: Posted' then
            Error('caller %1', Row.GetFilters());
    end;

    [Test]
    procedure Integration_NoFilter_KeepsMain()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Publisher.Raise(Row);
        Expect('sub f={} n=3;');
    end;

    [Test]
    procedure Integration_Group2Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.FilterGroup(2);
        Row.SetRange(Status, Row.Status::Posted);
        Row.FilterGroup(0);
        Publisher.Raise(Row);
        Expect('');
    end;

    [Test]
    procedure Integration_CallerInGroup2_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Row.FilterGroup(2);
        Publisher.Raise(Row);
        Expect('');
    end;

    [Test]
    procedure Integration_SecondaryKey_NoFilter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetCurrentKey(Name);
        Publisher.Raise(Row);
        Expect('');
    end;

    [Test]
    procedure Integration_Marks_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.Get(2);
        Row.Mark(true);
        Publisher.Raise(Row);
        Expect('');
    end;

    [Test]
    procedure Business_Group0Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.RaiseBusiness(Row);
        Expect('');
    end;

    [Test]
    procedure Business_Group0Filter_FieldRead_Runs()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Row.Name := 'fields only';
        Publisher.RaiseBusiness(Row);
        Expect('name=fields only;');
    end;

    [Test]
    procedure Business_NoFilter_KeepsMain()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Publisher.RaiseBusiness(Row);
        Expect('business f={} n=3;');
    end;

    [Test]
    procedure Internal_Group0Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.RaiseInternal(Row);
        Expect('');
    end;

    [Test]
    procedure Isolated_Group0Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.RaiseIsolated(Row);
        Expect('');
    end;

    [Test]
    procedure GlobalVarAccess_Group0Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.RaiseGlobal(Row);
        Expect('');
    end;

    [Test]
    procedure IncludeSender_Group0Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.RaiseSender(Row);
        Expect('');
    end;

    [Test]
    procedure VarPublisherParameter_Group0Filter_KeepsMain()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.RaiseVar(Row);
        Expect('var f={Status: Posted} n=1;');
    end;

    [Test]
    procedure VarTablePublisher_FlowFilter_KeepsMain()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetFilter("Location Filter", 'BLUE');
        Row.SetRange(Status, Row.Status::Posted);
        Row.RaiseOwnVar(Row);
        Expect('tablevar f=BLUE copied=BLUE n=1;');
    end;

    [Test]
    procedure CommitBehaviorPublisher_Group0Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.RaiseCommitBehavior(Row);
        Expect('');
    end;

    [Test]
    procedure SingleInstancePublisher_Group0Filter_Refused()
    var
        Row: Record "BS Row";
        SinglePublisher: Codeunit "BS Single Publisher";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        SinglePublisher.Raise(Row);
        Expect('');
    end;

    [Test]
    procedure ManualSubscriber_Group0Filter_Refused()
    var
        Row: Record "BS Row";
        Manual: Codeunit "BS Manual Subscriber";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        BindSubscription(Manual);
        Publisher.RaiseManual(Row);
        Expect('');
    end;

    [Test]
    procedure SingleInstanceSubscriber_Group0Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Publisher.RaiseSingle(Row);
        Expect('');
    end;

    [Test]
    procedure TablePublisher_Group0Filter_Refused()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Row.RaiseOwn(Row);
        Expect('');
    end;

    [Test]
    procedure TableEvent_Group0Filter_KeepsMain()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.SetRange(Status, Row.Status::Posted);
        Row.FindFirst();
        Row.Modify();
        Expect('modify f={Status: Posted} n=1;');
    end;

    [Test]
    procedure TableEvent_NoFilter_KeepsMain()
    var
        Row: Record "BS Row";
    begin
        Seed(Row);
        Row.Get(2);
        Row.Modify();
        Expect('modify f={} n=3;');
    end;

    [Test]
    procedure TemporaryVarPublisher_KeepsMain()
    var
        Temp: Record "BS Row" temporary;
    begin
        Temp.Id := 9;
        Temp.Insert();
        Temp.SetRange(Id, 9);
        Publisher.RaiseVar(Temp);
        Expect('var f={Id: 9} n=0;');
    end;

    [Test]
    procedure Temporary_Group0Filter_Refused()
    var
        Temp: Record "BS Row" temporary;
    begin
        Temp.Id := 9;
        Temp.Insert();
        Temp.SetRange(Id, 9);
        Publisher.Raise(Temp);
        Expect('');
    end;
}
"##;

#[test]
fn issue_8295_subscriber_by_value_record_gets_no_filters_in_the_measured_shape_only() {
    let o = run(SUBSCRIBER, "BS Test");
    let want = [
        // record/filters/ByValueRecordView.al:964-978 (corpus PR 1430): `sub f={} n=3`, the caller keeps its filter.
        ("Integration_Group0Filter_Flagged", PASS, true),
        // The copy holds the view of a fresh record already: nothing to drop, as on main.
        ("Integration_NoFilter_KeepsMain", PASS, false),
        // Filter groups, standing group, key and marks the event cell did not measure.
        ("Integration_Group2Filter_Refused", VIEW_REFUSED, false),
        ("Integration_CallerInGroup2_Refused", VIEW_REFUSED, false),
        ("Integration_SecondaryKey_NoFilter_Refused", VIEW_REFUSED, false),
        ("Integration_Marks_Refused", VIEW_REFUSED, false),
        // Publisher kinds and attributes the probe did not measure.
        ("Business_Group0Filter_Refused", VIEW_REFUSED, false),
        ("Business_Group0Filter_FieldRead_Runs", PASS, false),
        ("Business_NoFilter_KeepsMain", PASS, false),
        ("Internal_Group0Filter_Refused", VIEW_REFUSED, false),
        ("Isolated_Group0Filter_Refused", VIEW_REFUSED, false),
        ("GlobalVarAccess_Group0Filter_Refused", VIEW_REFUSED, false),
        ("IncludeSender_Group0Filter_Refused", VIEW_REFUSED, false),
        // Publisher facts the probe did not measure: a SingleInstance publisher codeunit, a CommitBehavior attribute.
        ("CommitBehaviorPublisher_Group0Filter_Refused", VIEW_REFUSED, false),
        ("SingleInstancePublisher_Group0Filter_Refused", VIEW_REFUSED, false),
        // A by-value parameter of a table's own event.
        ("TablePublisher_Group0Filter_Refused", VIEW_REFUSED, false),
        // The publisher's parameter is `var`, or the platform raises the event (Rec of OnAfterModifyEvent is `var`): the
        // probe measured only a by-value publisher parameter, so the subscriber's copy keeps main's view.
        ("VarPublisherParameter_Group0Filter_KeepsMain", PASS, false),
        ("VarTablePublisher_FlowFilter_KeepsMain", PASS, false),
        ("TableEvent_Group0Filter_KeepsMain", PASS, false),
        ("TableEvent_NoFilter_KeepsMain", PASS, false),
        // Subscriber instances the probe did not measure.
        ("ManualSubscriber_Group0Filter_Refused", VIEW_REFUSED, false),
        ("SingleInstanceSubscriber_Group0Filter_Refused", VIEW_REFUSED, false),
        // A temporary record: not measured for a subscriber. Through a `var` publisher parameter it keeps main's view.
        ("Temporary_Group0Filter_Refused", VIEW_REFUSED, false),
        ("TemporaryVarPublisher_KeepsMain", PASS, false),
    ];
    check(&o, &want);
}
