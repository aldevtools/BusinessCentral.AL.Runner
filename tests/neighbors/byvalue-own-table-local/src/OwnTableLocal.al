// Neighbors of issue 8596: a local record of a table handed by value to a procedure of the same table or table extension,
// called without a receiver from a procedure of that object. Each test reads the copy's view in the callee (CalcFields of a
// FlowField over a FlowFilter, Count, GetFilters, FilterGroup). The measured shape (one caller filter on a FlowFilter or a
// Normal field, table and table extension) runs under an assumption; each other test flips one fact the new rule reads and
// keeps main's refusal.
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
        field(5; Net; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("NBO Entry".Amount where("Acct No." = field("No."), "Posting Date" = field("Date Filter"), Kind = field("Kind Filter")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
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

    procedure GetLastSeen(): Text
    begin
        exit(LastSeen);
    end;

    procedure SetView(var Acct: Record "NBO Acct"; View: Integer)
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
        exit(StrSubstNo('Net=%1 Count=%2 Filters=[%3] G=%4', Acct.Net, Acct.Count(), Acct.GetFilters(), Acct.FilterGroup()));
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
        Fresh: Label 'Net=60 Count=2 Filters=[] G=0', Locked = true;

    [Test]
    procedure Table_Local_NoView()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromLocal(0));
    end;

    [Test]
    procedure Table_Local_FlowFilterView()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromLocal(1));
    end;

    [Test]
    procedure Table_Local_NormalFieldView()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromLocal(2));
    end;

    [Test]
    procedure Table_Local_TwoFlowFiltersAndCalcFields()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromLocal(3));
    end;

    [Test]
    procedure Table_Local_SetAutoCalcFields()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromLocal(4));
    end;

    [Test]
    procedure Table_Local_ToGlobalCallee()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromLocalToGlobalCallee());
    end;

    [Test]
    procedure Table_Rec_AsArgument()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromRec());
    end;

    [Test]
    procedure Table_GlobalRecord_AsArgument()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromGlobal());
    end;

    [Test]
    procedure Table_VarParameter_AsArgument()
    var
        Acct: Record "NBO Acct";
        Other: Record "NBO Acct";
    begin
        Setup(Acct);
        Other.Get('A1');
        Other.SetFilter(Amount, '<>%1', 0);
        Check(Acct.FromParam(Other));
    end;

    [Test]
    procedure Table_Local_ToOtherReceiver()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.FromOtherReceiver());
    end;

    [Test]
    procedure Table_Local_FromTrigger()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Acct.Validate(Amount, 5);
        Check(Acct.GetLastSeen());
    end;

    [Test]
    procedure TableExt_Local_FlowFilterView()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.ExtFromLocal(1));
    end;

    [Test]
    procedure TableExt_Local_NormalFieldView()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.ExtFromLocal(2));
    end;

    [Test]
    procedure TableExt_Local_ToBaseProcedure()
    var
        Acct: Record "NBO Acct";
    begin
        Setup(Acct);
        Check(Acct.ExtToBase());
    end;

    [Test]
    procedure Codeunit_Local_ToTableProcedure()
    var
        Acct: Record "NBO Acct";
        Other: Record "NBO Acct";
    begin
        Setup(Acct);
        Other.Get('A1');
        Other.SetFilter(Amount, '<>%1', 0);
        Check(Acct.SeenG(Other));
    end;

    local procedure Check(Actual: Text)
    begin
        if Actual <> Fresh then
            Error('expected %1, got %2', Fresh, Actual);
    end;

    // Inserts the rows when they are missing, then gets A1.
    local procedure Setup(var Acct: Record "NBO Acct")
    var
        Acct2: Record "NBO Acct";
        Entry: Record "NBO Entry";
    begin
        if not Acct.Get('A1') then begin
            Acct."No." := 'A1';
            Acct.Amount := 5;
            Acct.Insert();
            Acct2."No." := 'A2';
            Acct2.Amount := 0;
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
