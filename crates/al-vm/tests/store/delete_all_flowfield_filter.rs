//! DeleteAll on a record filtered on a FlowField (al-runner issue 6961). Measured on BC 29 by corpus branch
//! al-runtime-rs-staging-r6961, record/crud/R6961DeleteAllFlowFilter.al: the FlowField filter selects the rows as FindSet does;
//! DeleteAll() runs no OnDelete and deletes the rows chosen before the first delete; DeleteAll(true) runs OnDelete per row and
//! tests each row against the filters when it reaches it, after the earlier deletes.

use al_testkit::text;

const SRC: &str = r#"
table 50851 "DF Head"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Parent; Integer) { }
        field(3; Kind; Code[10]) { }
        field(10; HasLines; Boolean) { FieldClass = FlowField; CalcFormula = exist("DF Line" where(HeadId = field(Id))); }
        field(11; LineSum; Decimal) { FieldClass = FlowField; CalcFormula = sum("DF Line".Amount where(HeadId = field(Id), PostDate = field(DateFilter))); }
        field(12; LineCount; Integer) { FieldClass = FlowField; CalcFormula = count("DF Line" where(HeadId = field(Id))); }
        field(13; ParentExists; Boolean) { FieldClass = FlowField; CalcFormula = exist("DF Head" where(Id = field(Parent))); }
        field(14; DateFilter; Date) { FieldClass = FlowFilter; }
        field(15; LastLineKind; Code[10]) { FieldClass = FlowField; CalcFormula = lookup("DF Line".Kind where(HeadId = field(Id))); }
    }
    keys { key(PK; Id) { Clustered = true; } }
    trigger OnDelete()
    var Log: Record "DF Log";
    begin
        Log.Seq := 0; Log.Msg := 'del ' + Format(Id); Log.Insert();
    end;
}
table 50852 "DF Line"
{
    fields { field(1; HeadId; Integer) { } field(2; LineNo; Integer) { } field(3; Amount; Decimal) { } field(4; PostDate; Date) { } field(5; Kind; Code[10]) { } }
    keys { key(PK; HeadId, LineNo) { Clustered = true; } }
}
table 50853 "DF Log"
{
    fields { field(1; Seq; Integer) { AutoIncrement = true; } field(2; Msg; Text[100]) { } }
    keys { key(PK; Seq) { Clustered = true; } }
}
codeunit 50854 C
{
    local procedure AddHead(Id: Integer; Parent: Integer; Kind: Code[10])
    var H: Record "DF Head";
    begin
        H.Id := Id; H.Parent := Parent; H.Kind := Kind; H.Insert();
    end;

    local procedure AddLine(HeadId: Integer; LineNo: Integer; Amount: Decimal; PostDate: Date; Kind: Code[10])
    var L: Record "DF Line";
    begin
        L.HeadId := HeadId; L.LineNo := LineNo; L.Amount := Amount; L.PostDate := PostDate; L.Kind := Kind; L.Insert();
    end;

    local procedure Seed()
    begin
        AddHead(1, 0, 'A'); AddHead(2, 0, 'B'); AddHead(3, 0, 'A'); AddHead(4, 0, 'B');
        AddLine(1, 10000, 5, DMY2Date(10, 1, 2030), 'X');
        AddLine(3, 10000, 20, DMY2Date(10, 1, 2030), 'Y');
        AddLine(3, 20000, 30, DMY2Date(10, 2, 2030), 'Z');
    end;

    local procedure SeedChain()
    begin
        AddHead(1, 0, 'A'); AddHead(2, 1, 'A'); AddHead(3, 2, 'A'); AddHead(4, 3, 'A'); AddHead(5, 9, 'A');
    end;

    local procedure Result(): Text
    var H: Record "DF Head"; L: Record "DF Log"; R: Text;
    begin
        if H.FindSet() then repeat R += Format(H.Id) + ','; until H.Next() = 0;
        R += '|';
        if L.FindSet() then repeat R += L.Msg + ';'; until L.Next() = 0;
        exit(R);
    end;

    local procedure ExistFalse(Run: Boolean): Text
    var H: Record "DF Head";
    begin
        Seed();
        H.SetRange(HasLines, false);
        H.DeleteAll(Run);
        exit(Result());
    end;

    procedure ExistTrueDescending(): Text
    var H: Record "DF Head";
    begin
        Seed();
        H.Ascending(false);
        H.SetRange(HasLines, true);
        H.DeleteAll(true);
        exit(Result());
    end;

    procedure SumWithFlowFilter(): Text
    var H: Record "DF Head";
    begin
        Seed();
        H.SetRange(DateFilter, DMY2Date(1, 1, 2030), DMY2Date(31, 1, 2030));
        H.SetFilter(LineSum, '>%1&<%2', 1, 10);
        H.DeleteAll();
        exit(Result());
    end;

    procedure CountAndNormal(): Text
    var H: Record "DF Head";
    begin
        Seed();
        H.SetFilter(LineCount, '>=%1', 1);
        H.SetRange(Kind, 'A');
        H.SetRange(Id, 2, 4);
        H.DeleteAll(true);
        exit(Result());
    end;

    procedure LookupNotEqual(): Text
    var H: Record "DF Head";
    begin
        Seed();
        H.SetFilter(LastLineKind, '<>%1', 'Y');
        H.DeleteAll(true);
        exit(Result());
    end;

    local procedure SelfReference(Run: Boolean; Exists: Boolean): Text
    var H: Record "DF Head";
    begin
        SeedChain();
        H.SetRange(ParentExists, Exists);
        H.DeleteAll(Run);
        exit(Result());
    end;

    procedure Temporary(): Text
    var H: Record "DF Head"; T: Record "DF Head" temporary; R: Text;
    begin
        Seed();
        H.FindSet();
        repeat T := H; T.Insert(); until H.Next() = 0;
        T.SetRange(HasLines, false);
        T.DeleteAll(true);
        T.Reset();
        T.FindSet();
        repeat R += Format(T.Id) + ','; until T.Next() = 0;
        exit(R + '|' + Result());
    end;

    procedure ExistFalseNoTrigger(): Text begin exit(ExistFalse(false)); end;
    procedure ExistFalseRunTrigger(): Text begin exit(ExistFalse(true)); end;
    procedure ChainNoTrigger(): Text begin exit(SelfReference(false, false)); end;
    procedure ChainRunTrigger(): Text begin exit(SelfReference(true, false)); end;
    procedure ChainExistsRunTrigger(): Text begin exit(SelfReference(true, true)); end;

    procedure ViaRecordRef(): Text
    var RecRef: RecordRef; FldRef: FieldRef;
    begin
        Seed();
        RecRef.Open(Database::"DF Head");
        FldRef := RecRef.Field(10);
        FldRef.SetRange(false);
        RecRef.DeleteAll(true);
        exit(Result());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

// R6961DeleteAllFlowFilter.al DeleteAll_ExistFalse_NoTrigger, DeleteAll_ExistFalse_RunTrigger.
#[test]
fn delete_all_with_an_exist_filter() {
    assert_eq!(run("ExistFalseNoTrigger"), "1,3,|");
    assert_eq!(run("ExistFalseRunTrigger"), "1,3,|del 2;del 4;");
}

// DeleteAll_ExistTrue_RunTrigger_Descending, DeleteAll_SumFilterWithFlowFilter_NoTrigger, DeleteAll_CountAndNormalFilter_RunTrigger,
// DeleteAll_LookupNotEqual_RunTrigger.
#[test]
fn delete_all_with_sum_count_and_lookup_filters() {
    assert_eq!(run("ExistTrueDescending"), "2,4,|del 3;del 1;");
    assert_eq!(run("SumWithFlowFilter"), "2,3,4,|");
    assert_eq!(run("CountAndNormal"), "1,2,4,|del 3;");
    assert_eq!(run("LookupNotEqual"), "3,|del 1;del 2;del 4;");
}

// DeleteAll_SelfReferenceExist_NoTrigger (the rows chosen before the first delete), DeleteAll_SelfReferenceExist_RunTrigger and
// DeleteAll_SelfReferenceExistTrue_RunTrigger (each row tested when it is reached).
#[test]
fn delete_all_with_a_filter_on_a_flowfield_over_the_same_table() {
    assert_eq!(run("ChainNoTrigger"), "2,3,4,|");
    assert_eq!(run("ChainRunTrigger"), "|del 1;del 2;del 3;del 4;del 5;");
    assert_eq!(run("ChainExistsRunTrigger"), "1,3,5,|del 2;del 4;");
}

// DeleteAll_Temporary_ExistFalse_RunTrigger, RecordRef_DeleteAll_ExistFalse_RunTrigger.
#[test]
fn delete_all_with_a_flowfield_filter_on_a_temporary_record_and_through_a_recordref() {
    assert_eq!(run("Temporary"), "1,3,|1,2,3,4,|del 2;del 4;");
    assert_eq!(run("ViaRecordRef"), "1,3,|del 2;del 4;");
}
