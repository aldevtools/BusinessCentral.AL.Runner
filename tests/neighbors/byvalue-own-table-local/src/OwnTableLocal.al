// Neighbors of issue 8596: a local record of a table handed by value to a procedure of the same table or table extension,
// called without a receiver from a procedure of that object. Each test reads the copy's view in the callee (CalcFields of a
// FlowField over a FlowFilter, Count, GetFilters, FilterGroup, CurrentKey, Ascending, MarkedOnly) and expects the default
// view BC gives it. The call shape with a view fact the probes measure runs under an assumption; each other test flips one
// fact the new rule reads (the argument, the receiver, the caller, an unmeasured view fact) and keeps main's answer.
table 59600 "NBO Acct"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal)
        {
            trigger OnValidate()
            var
                Acct: Record "NBO Acct";
            begin
                Acct.Get("No.");
                Acct.SetFilter(Amount, '<>%1', 0);
                LastSeen := Seen(Acct);
            end;
        }
        field(3; "Date Filter"; Date) { FieldClass = FlowFilter; }
        field(4; "Kind Filter"; Integer) { FieldClass = FlowFilter; }
        field(6; Kind; Integer) { }
        field(5; Net; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("NBO Entry".Amount where("Acct No." = field("No."), "Posting Date" = field("Date Filter"), Kind = field("Kind Filter")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
        key(ByAmount; Amount, "No.") { }
    }

    var
        GlobalAcct: Record "NBO Acct";
        LastSeen: Text;

    // A local record, its view, then a by-value call of a local procedure of this table.
    procedure FromLocal(View: Integer): Text
    var
        Acct: Record "NBO Acct";
    begin
        Acct.Get("No.");
        SetView(Acct, View);
        exit(Seen(Acct));
    end;

    // The same, to a procedure that is not local.
    procedure FromLocalToGlobalCallee(): Text
    var
        Acct: Record "NBO Acct";
    begin
        Acct.Get("No.");
        Acct.SetFilter(Amount, '<>%1', 0);
        exit(SeenG(Acct));
    end;

    // Rec as the argument.
    procedure FromRec(): Text
    begin
        SetFilter(Amount, '<>%1', 0);
        exit(Seen(Rec));
    end;

    // A global record of the table as the argument.
    procedure FromGlobal(): Text
    begin
        GlobalAcct.Get("No.");
        GlobalAcct.SetFilter(Amount, '<>%1', 0);
        exit(Seen(GlobalAcct));
    end;

    // A var parameter of the caller as the argument.
    procedure FromParam(var Other: Record "NBO Acct"): Text
    begin
        exit(Seen(Other));
    end;

    // A local record handed to the procedure of another record of the table.
    procedure FromOtherReceiver(): Text
    var
        Acct: Record "NBO Acct";
        Other: Record "NBO Acct";
    begin
        Acct.Get("No.");
        Acct.SetFilter(Amount, '<>%1', 0);
        exit(Other.SeenG(Acct));
    end;

    // The copy's field values (an unsaved Amount) and the Mark() of its row, read before the view.
    procedure FromLocalValues(View: Integer): Text
    var
        Acct: Record "NBO Acct";
    begin
        Acct.Get("No.");
        SetView(Acct, View);
        exit(SeenValues(Acct));
    end;

    local procedure SeenValues(Acct: Record "NBO Acct"): Text
    var
        Values: Text;
    begin
        Values := StrSubstNo('Amount=%1 Mark=%2 ', Acct.Amount, Acct.Mark());
        exit(Values + Describe(Acct));
    end;

    // SetLoadFields before the Get: main's answer stays (what the copy's load request is, the probe cannot tell).
    procedure FromLocalLoadFields(): Text
    var
        Acct: Record "NBO Acct";
    begin
        Acct.SetLoadFields(Amount);
        Acct.Get("No.");
        exit(SeenLoaded(Acct));
    end;

    local procedure SeenLoaded(Acct: Record "NBO Acct"): Text
    begin
        exit(StrSubstNo('%1 Amount loaded=%2', Describe(Acct), Acct.AreFieldsLoaded(Amount)));
    end;

    procedure GetLastSeen(): Text
    begin
        exit(LastSeen);
    end;

    procedure SetView(var Acct: Record "NBO Acct"; View: Integer)
    var
        Other: Record "NBO Acct";
    begin
        case View of
            1:
                Acct.SetFilter("Date Filter", '..%1', 20260131D);
            2:
                Acct.SetFilter(Amount, '<>%1', 0);
            3:
                begin
                    Acct.SetFilter("Date Filter", '..%1', 20260131D);
                    Acct.SetRange("Kind Filter", 7);
                    Acct.CalcFields(Net);
                end;
            4:
                begin
                    Acct.SetAutoCalcFields(Net);
                    Acct.SetFilter("Date Filter", '..%1', 20260131D);
                end;
            5:
                begin
                    Acct.SetCurrentKey(Amount, "No.");
                    Acct.SetFilter(Amount, '<>%1', 0);
                end;
            6:
                begin
                    Acct.FilterGroup(2);
                    Acct.SetFilter(Amount, '<>%1', 0);
                    Acct.FilterGroup(0);
                end;
            7:
                begin
                    Acct.FilterGroup(2);
                    Acct.SetFilter(Amount, '<>%1', 0);
                end;
            8:
                Acct.Ascending(false);
            9:
                begin
                    Acct.Mark(true);
                    Other.Get('A2');
                    Acct := Other;
                    Acct.Mark(true);
                    Acct.Get('A1');
                    Acct.MarkedOnly(true);
                end;
            10:
                begin
                    Acct.SetFilter(Amount, '<>%1', 0);
                    Acct.SetFilter(Kind, '%1', 1);
                end;
            11:
                Acct.SetCurrentKey(Amount, "No.");
            // The view facts the probes do not measure.
            13:
                begin
                    Acct.SetCurrentKey(Kind);
                    Acct.SetFilter(Amount, '<>%1', 0);
                end;
            14:
                Acct.Mark(true);
            15:
                begin
                    Acct.SetAscending(Amount, false);
                    Acct.SetFilter(Amount, '<>%1', 0);
                end;
            16:
                Acct.SetFilter(Net, '<>%1', 0);
            // A filter in group 3: no cell's caller sets one.
            20:
                begin
                    Acct.FilterGroup(3);
                    Acct.SetFilter(Amount, '<>%1', 0);
                    Acct.FilterGroup(0);
                end;
            // Field values and the combined view.
            17:
                begin
                    Acct.SetFilter(Amount, '<>%1', 0);
                    Acct.Amount := 99;
                end;
            18:
                begin
                    Acct.SetFilter(Amount, '<>%1', 0);
                    Acct.Mark(true);
                    Acct.MarkedOnly(true);
                end;
            19:
                begin
                    Acct.Mark(true);
                    Other.Get('A2');
                    Acct := Other;
                    Acct.Mark(true);
                    Acct.Get('A1');
                    Acct.MarkedOnly(true);
                    Acct.SetFilter("Date Filter", '..%1', 20260131D);
                    Acct.SetRange("Kind Filter", 7);
                    Acct.CalcFields(Net);
                    Acct.FilterGroup(2);
                    Acct.SetFilter(Amount, '<>%1', 0);
                    Acct.FilterGroup(4);
                    Acct.SetFilter(Kind, '%1', 1);
                    Acct.FilterGroup(0);
                    Acct.SetCurrentKey(Amount, "No.");
                    Acct.Ascending(false);
                    Acct.FilterGroup(2);
                end;
        end;
    end;

    local procedure Seen(Acct: Record "NBO Acct"): Text
    begin
        exit(Describe(Acct));
    end;

    procedure SeenG(Acct: Record "NBO Acct"): Text
    begin
        exit(Describe(Acct));
    end;

    procedure Describe(var Acct: Record "NBO Acct"): Text
    begin
        Acct.CalcFields(Net);
        exit(StrSubstNo('Net=%1 Count=%2 Filters=[%3] G=%4 Key=%5 Asc=%6 MO=%7', Acct.Net, Acct.Count(), Acct.GetFilters(),
            Acct.FilterGroup(), Acct.CurrentKey(), Acct.Ascending(), Acct.MarkedOnly()));
    end;
}

table 59601 "NBO Entry"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; "Acct No."; Code[20]) { }
        field(3; "Posting Date"; Date) { }
        field(4; Kind; Integer) { }
        field(5; Amount; Decimal) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

tableextension 59602 "NBO Acct Ext" extends "NBO Acct"
{
    // A local record, its view, then a by-value call of a local procedure of this extension.
    procedure ExtFromLocal(View: Integer): Text
    var
        Acct: Record "NBO Acct";
    begin
        Acct.Get("No.");
        SetView(Acct, View);
        exit(ExtSeen(Acct));
    end;

    // A local record of the extension handed to a procedure of the base table.
    procedure ExtToBase(): Text
    var
        Acct: Record "NBO Acct";
    begin
        Acct.Get("No.");
        Acct.SetFilter(Amount, '<>%1', 0);
        exit(SeenG(Acct));
    end;

    local procedure ExtSeen(Acct: Record "NBO Acct"): Text
    begin
        exit(Describe(Acct));
    end;
}

codeunit 59603 "NBO Own Table Local Tests"
{
    Subtype = Test;

    var
        Lib: Codeunit "NBO Lib";

    [Test]
    procedure Table_Local_FlowFilterView()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(1));
    end;

    [Test]
    procedure Table_Local_NormalFieldView()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(2));
    end;

    [Test]
    procedure Table_Local_TwoFlowFiltersAndCalcFields()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(3));
    end;

    [Test]
    procedure Table_Local_SetAutoCalcFields()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(4));
    end;

    [Test]
    procedure Table_Local_SecondaryKeyOneFilter()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(5));
    end;

    [Test]
    procedure Table_Local_FilterGroup2()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(6));
    end;

    [Test]
    procedure Table_Local_CallerInGroup2()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(7));
    end;

    [Test]
    procedure Table_Local_AscendingFalse()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(8));
    end;

    [Test]
    procedure Table_Local_MarksMarkedOnly()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(9));
    end;

    [Test]
    procedure Table_Local_TwoNormalFilters()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(10));
    end;

    [Test]
    procedure Table_Local_AdHocKey()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(13));
    end;

    [Test]
    procedure Table_Local_MarksNotMarkedOnly()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(14));
    end;

    [Test]
    procedure Table_Local_SetAscendingField()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(15));
    end;

    [Test]
    procedure Table_Local_FlowFieldFilter()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(16));
    end;

    [Test]
    procedure Table_Local_FilterGroup3()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(20));
    end;

    [Test]
    procedure Table_Local_UnsavedAmount()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check2(Acct.FromLocalValues(17), 'Amount=99 Mark=No ' + Lib.Fresh());
    end;

    [Test]
    procedure Table_Local_MarkOfCurrentRow()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check2(Acct.FromLocalValues(18), 'Amount=5 Mark=No ' + Lib.Fresh());
    end;

    [Test]
    procedure Table_Local_Combined()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(19));
    end;

    [Test]
    procedure Table_Local_ToGlobalCallee()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocalToGlobalCallee());
    end;

    [Test]
    procedure Table_Rec_AsArgument()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromRec());
    end;

    [Test]
    procedure Table_GlobalRecord_AsArgument()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromGlobal());
    end;

    [Test]
    procedure Table_VarParameter_AsArgument()
    var
        Acct: Record "NBO Acct";
        Other: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Other.Get('A1');
        Other.SetFilter(Amount, '<>%1', 0);
        Lib.Check(Acct.FromParam(Other));
    end;

    [Test]
    procedure Table_Local_ToOtherReceiver()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromOtherReceiver());
    end;

    [Test]
    procedure Table_Local_FromTrigger()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Acct.Validate(Amount, 5);
        Lib.Check(Acct.GetLastSeen());
    end;

    [Test]
    procedure TableExt_Local_FlowFilterView()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.ExtFromLocal(1));
    end;

    [Test]
    procedure TableExt_Local_NormalFieldView()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.ExtFromLocal(2));
    end;

    [Test]
    procedure TableExt_Local_ToBaseProcedure()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.ExtToBase());
    end;

    [Test]
    procedure Codeunit_Local_ToTableProcedure()
    var
        Acct: Record "NBO Acct";
        Other: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Other.Get('A1');
        Other.SetFilter(Amount, '<>%1', 0);
        Lib.Check(Acct.SeenG(Other));
    end;
}

// The no-filter controls, in a codeunit of their own so that no refusal of the other codeunit masks their answers: the
// default key (BC drops the caller's secondary key), and a SetLoadFields before the Get, which keeps main's answer.
codeunit 59604 "NBO Own Table Local Controls"
{
    Subtype = Test;

    var
        Lib: Codeunit "NBO Lib";

    [Test]
    procedure Table_Local_NoView()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(0));
    end;

    [Test]
    procedure Table_Local_SecondaryKeyNoFilter()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check(Acct.FromLocal(11));
    end;

    [Test]
    procedure Table_Local_SetLoadFieldsBeforeRead()
    var
        Acct: Record "NBO Acct";
    begin
        Lib.Setup(Acct);
        Lib.Check2(Acct.FromLocalLoadFields(), Lib.Fresh() + ' Amount loaded=Yes');
    end;
}

codeunit 59605 "NBO Lib"
{
    procedure Fresh(): Text
    begin
        exit('Net=60 Count=2 Filters=[] G=0 Key=No. Asc=Yes MO=No');
    end;

    procedure Check(Actual: Text)
    begin
        Check2(Actual, Fresh());
    end;

    procedure Check2(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('expected %1, got %2', Expected, Actual);
    end;

    // Inserts the rows when they are missing, then gets A1.
    procedure Setup(var Acct: Record "NBO Acct")
    var
        Acct2: Record "NBO Acct";
        Entry: Record "NBO Entry";
    begin
        if not Acct.Get('A1') then begin
            Acct."No." := 'A1';
            Acct.Amount := 5;
            Acct.Kind := 1;
            Acct.Insert();
            Acct2."No." := 'A2';
            Acct2.Amount := 0;
            Acct2.Kind := 2;
            Acct2.Insert();
            Entry."Entry No." := 1;
            Entry."Acct No." := 'A1';
            Entry."Posting Date" := 20260110D;
            Entry.Kind := 7;
            Entry.Amount := 100;
            Entry.Insert();
            Entry."Entry No." := 2;
            Entry."Posting Date" := 20260310D;
            Entry.Kind := 7;
            Entry.Amount := -40;
            Entry.Insert();
        end;
        Acct.Get('A1');
    end;
}
