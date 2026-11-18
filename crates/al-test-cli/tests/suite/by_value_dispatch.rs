//! Who calls the procedure a record is handed to by value, and what else the caller's record holds (al-runner issue 8248,
//! round 2 of its review): an interface call is not measured, nor is an event subscriber outside the shape of
//! `by_value_subscriber.rs`, a table procedure is measured only as
//! `Row.Proc(Row)` from a codeunit, and a record whose SetLoadFields after a read, SetAutoCalcFields or ReadIsolation the
//! probe did not hand on is refused. Each neighbor that main answered keeps main's result.

use crate::record_copy_reports::{PASS, VIEW_REFUSED, check, run};

const DISPATCH: &str = r##"
table 50160 "BD Row"
{
    fields
    {
        field(1; "Code"; Code[10]) { }
        field(2; Status; Option) { OptionMembers = Open,Posted; }
        field(3; Name; Text[30]) { }
        field(4; Amount; Decimal) { }
        field(5; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("BD Line".Amt where(RowCode = field("Code"))); }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure OwnFilters(Line: Record "BD Row"): Text
    begin
        exit(Line.GetFilters());
    end;

    procedure OwnFiltersOfRec(): Text
    begin
        exit(OwnFilters(Rec));
    end;
}

table 50161 "BD Line"
{
    fields { field(1; Id; Integer) { } field(2; RowCode; Code[10]) { } field(3; Amt; Decimal) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

interface "BD IFilters"
{
    procedure Filters(Line: Record "BD Row"): Text;
}

codeunit 50162 "BD Impl" implements "BD IFilters"
{
    procedure Filters(Line: Record "BD Row"): Text
    begin
        exit('temp=' + Format(Line.IsTemporary()) + ' f={' + Line.GetFilters() + '}');
    end;
}

codeunit 50163 "BD Callee"
{
    procedure Filters(Line: Record "BD Row"): Text
    begin
        exit('temp=' + Format(Line.IsTemporary()) + ' f={' + Line.GetFilters() + '}');
    end;

    procedure LoadedAfterFind(Line: Record "BD Row"): Text
    begin
        Line.FindFirst();
        exit(Format(Line.AreFieldsLoaded(Amount)));
    end;

    procedure TotalAfterFind(Line: Record "BD Row"): Text
    begin
        Line.FindFirst();
        exit(Format(Line.Total));
    end;
}

codeunit 50164 "BD Publisher"
{
    procedure Raise(var Line: Record "BD Row")
    begin
        OnThing(Line);
    end;

    [IntegrationEvent(false, false)]
    local procedure OnThing(Line: Record "BD Row")
    begin
    end;
}

codeunit 50165 "BD Subscriber"
{
    SingleInstance = true;

    var
        Seen: Text;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"BD Publisher", 'OnThing', '', false, false)]
    local procedure HandleThing(Line: Record "BD Row")
    begin
        Seen := 'temp=' + Format(Line.IsTemporary()) + ' f={' + Line.GetFilters() + '}';
    end;

    procedure Take(): Text
    begin
        exit(Seen);
    end;
}

codeunit 50166 "BD Test"
{
    Subtype = Test;

    var
        Callee: Codeunit "BD Callee";
        Impl: Codeunit "BD Impl";
        Publisher: Codeunit "BD Publisher";
        Subscriber: Codeunit "BD Subscriber";

    local procedure Seed(var Line: Record "BD Row")
    begin
        Line.DeleteAll();
        Line.Code := 'A';
        Line.Name := 'z';
        Line.Insert();
        Line.Code := 'B';
        Line.Name := 'y';
        Line.Status := Line.Status::Posted;
        Line.Insert();
    end;

    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('%1', Got);
    end;

    [Test]
    procedure Codeunit_Group0Filter_Flagged()
    var
        Line: Record "BD Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        Expect(Callee.Filters(Line), 'temp=No f={}');
    end;

    [Test]
    procedure TemporaryToSubscriber_Refused()
    var
        Line: Record "BD Row" temporary;
    begin
        Line.Code := 'A';
        Line.Insert();
        Line.SetRange(Code, 'A');
        Publisher.Raise(Line);
        Expect(Subscriber.Take(), 'temp=No f={Code: A}');
    end;

    [Test]
    procedure Interface_Group0Filter_Refused()
    var
        Line: Record "BD Row";
        Filters: Interface "BD IFilters";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        Filters := Impl;
        Expect(Filters.Filters(Line), '');
    end;

    [Test]
    procedure Interface_NoFilter_KeepsMain()
    var
        Line: Record "BD Row";
        Filters: Interface "BD IFilters";
    begin
        Seed(Line);
        Filters := Impl;
        Expect(Filters.Filters(Line), 'temp=No f={}');
    end;

    [Test]
    procedure TemporaryToInterface_Refused()
    var
        Line: Record "BD Row" temporary;
        Filters: Interface "BD IFilters";
    begin
        Line.Code := 'A';
        Line.Insert();
        Line.SetRange(Code, 'A');
        Filters := Impl;
        Expect(Filters.Filters(Line), '');
    end;

    [Test]
    procedure SetLoadFields_AfterARead_Refused()
    var
        Line: Record "BD Row";
    begin
        Seed(Line);
        Line.SetLoadFields(Name);
        Line.FindFirst();
        Line.SetRange(Status, Line.Status::Posted);
        Expect(Callee.LoadedAfterFind(Line), '');
    end;

    [Test]
    procedure SetAutoCalcFields_Refused()
    var
        Line: Record "BD Row";
    begin
        Seed(Line);
        Line.SetAutoCalcFields(Total);
        Line.SetRange(Status, Line.Status::Posted);
        Expect(Callee.TotalAfterFind(Line), '');
    end;

    [Test]
    procedure ReadIsolation_Refused()
    var
        Line: Record "BD Row";
    begin
        Seed(Line);
        Line.ReadIsolation := IsolationLevel::ReadCommitted;
        Line.SetRange(Status, Line.Status::Posted);
        Expect(Callee.Filters(Line), '');
    end;

    [Test]
    procedure TableProcedure_SameRecord_Flagged()
    var
        Line: Record "BD Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        Expect(Line.OwnFilters(Line), '');
    end;

    [Test]
    procedure TableProcedure_OtherVariable_Refused()
    var
        Line: Record "BD Row";
        Other: Record "BD Row";
    begin
        Seed(Line);
        Other.SetRange(Code, 'A');
        Expect(Line.OwnFilters(Other), '');
    end;

    [Test]
    procedure TableProcedure_RecFromTableCode_Refused()
    var
        Line: Record "BD Row";
    begin
        Seed(Line);
        Line.SetRange(Code, 'A');
        Expect(Line.OwnFiltersOfRec(), '');
    end;

    [Test]
    procedure TableProcedure_OtherVariable_NoFilter_KeepsMain()
    var
        Line: Record "BD Row";
        Other: Record "BD Row";
    begin
        Seed(Line);
        Expect(Line.OwnFilters(Other), '');
    end;
}
"##;

#[test]
fn by_value_copies_outside_the_measured_calls_and_record_state_refuse_or_keep_main() {
    let o = run(DISPATCH, "BD Test");
    let want = [
        ("Codeunit_Group0Filter_Flagged", PASS, true),
        // A subscriber's copy of a temporary record with a filter is not measured (al-runner issue 8295): its view reads refuse.
        ("TemporaryToSubscriber_Refused", VIEW_REFUSED, false),
        ("Interface_Group0Filter_Refused", VIEW_REFUSED, false),
        ("Interface_NoFilter_KeepsMain", PASS, false),
        ("TemporaryToInterface_Refused", VIEW_REFUSED, false),
        ("SetLoadFields_AfterARead_Refused", VIEW_REFUSED, false),
        ("SetAutoCalcFields_Refused", VIEW_REFUSED, false),
        ("ReadIsolation_Refused", VIEW_REFUSED, false),
        ("TableProcedure_SameRecord_Flagged", PASS, true),
        ("TableProcedure_OtherVariable_Refused", VIEW_REFUSED, false),
        ("TableProcedure_RecFromTableCode_Refused", VIEW_REFUSED, false),
        ("TableProcedure_OtherVariable_NoFilter_KeepsMain", PASS, false),
    ];
    check(&o, &want);
}
