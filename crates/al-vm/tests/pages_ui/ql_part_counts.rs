//! Corpus bump 42 (writer QL) cases of tests/al-language/pages/lt-qj-part-propagation/QjPartPropagation.al: Last, First and New on a
//! part, a new part row whose code changes the host row, and a part Update(false) that leaves the host row unchanged. The objects
//! are copied from the probe; the test procedures return their observations, and each Rust test compares them with the values the
//! probe asserts on real BC 27.5, 28.5 and 29.0.

const SRC: &str = r#"

table 86880 "LT QJ Head"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Total; Decimal) { }
        field(3; Descr; Text[30])
        {
            trigger OnValidate()
            var
                Log: Codeunit "LT QJ Log";
            begin
                Log.Add('T.H.Descr.V(' + Descr + ' x=' + xRec.Descr + ' t=' + Format(Total) + ' x.t=' + Format(xRec.Total) + ')');
            end;
        }
        field(4; Note; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnModify()
    var
        Log: Codeunit "LT QJ Log";
    begin
        Log.Add('T.H.MOD(t=' + Format(Total) + ' d=' + Descr + ' n=' + Note + ' x.t=' + Format(xRec.Total) + ' x.d=' + xRec.Descr + ')');
    end;
}

table 86881 "LT QJ Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Doc No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Qty; Decimal) { }
        field(4; Amt; Decimal) { }
    }

    keys
    {
        key(PK; "Doc No.", "Line No.") { Clustered = true; }
    }
}

codeunit 86882 "LT QJ Log"
{
    SingleInstance = true;

    var
        Lines: List of [Text];

    procedure Add(Line: Text)
    begin
        Lines.Add(Line);
    end;

    procedure Take(): Text
    var
        Line: Text;
        Result: Text;
    begin
        foreach Line in Lines do begin
            if Result <> '' then
                Result += ' ';
            Result += Line;
        end;
        Clear(Lines);
        exit(Result);
    end;

    procedure Reset()
    begin
        Clear(Lines);
    end;
}

page 86883 "LT QJ Lines"
{
    PageType = ListPart;
    SourceTable = "LT QJ Line";
    ApplicationArea = All;
    AutoSplitKey = true;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(LineNoF; Rec."Line No.") { ApplicationArea = All; }
                field(QtyF; Rec.Qty)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    var
                        Head: Record "LT QJ Head";
                    begin
                        Log.Add('L.Qty.V(' + Format(Rec.Qty) + ' x=' + Format(xRec.Qty) + ')');
                        Head.Get(Rec."Doc No.");
                        Head.Total := Head.Total + Rec.Qty - xRec.Qty;
                        Head.Modify();
                        CurrPage.Update(false);
                        Log.Add('L.Qty.V.end');
                    end;
                }
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetRecord()
    begin
        Log.Add('L.AGR(' + Format(Rec."Line No.") + ' q=' + Format(Rec.Qty) + ' x.q=' + Format(xRec.Qty) + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('L.AGCR(' + Format(Rec."Line No.") + ' q=' + Format(Rec.Qty) + ' x.q=' + Format(xRec.Qty) + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('L.MODR(' + Format(Rec."Line No.") + ' q=' + Format(Rec.Qty) + ' x.q=' + Format(xRec.Qty) + ')');
    end;
}

page 86884 "LT QJ Host Both"
{
    PageType = Document;
    SourceTable = "LT QJ Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoF; Rec."No.") { ApplicationArea = All; }
                field(TotalF; Rec.Total) { ApplicationArea = All; }
                field(DescrF; Rec.Descr) { ApplicationArea = All; }
                field(NoteF; Rec.Note) { ApplicationArea = All; }
            }
            part(Lines; "LT QJ Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Doc No." = field("No.");
                UpdatePropagation = Both;
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ValidateSave)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('H.ACT(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ')');
                    Rec.Validate(Descr, 'by action');
                    CurrPage.SaveRecord();
                    Log.Add('H.ACT.end(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ')');
                end;
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetRecord()
    begin
        Log.Add('H.AGR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('H.AGCR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('H.MODR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ' x.d=' + xRec.Descr + ')');
    end;
}

page 86885 "LT QJ Host SubPart"
{
    PageType = Document;
    SourceTable = "LT QJ Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoF; Rec."No.") { ApplicationArea = All; }
                field(TotalF; Rec.Total) { ApplicationArea = All; }
                field(DescrF; Rec.Descr) { ApplicationArea = All; }
                field(NoteF; Rec.Note) { ApplicationArea = All; }
            }
            part(Lines; "LT QJ Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Doc No." = field("No.");
                UpdatePropagation = SubPart;
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ValidateSave)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('H.ACT(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ')');
                    Rec.Validate(Descr, 'by action');
                    CurrPage.SaveRecord();
                    Log.Add('H.ACT.end(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ')');
                end;
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetRecord()
    begin
        Log.Add('H.AGR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('H.AGCR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('H.MODR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ' x.d=' + xRec.Descr + ')');
    end;
}

page 86886 "LT QJ Host Default"
{
    PageType = Document;
    SourceTable = "LT QJ Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoF; Rec."No.") { ApplicationArea = All; }
                field(TotalF; Rec.Total) { ApplicationArea = All; }
                field(DescrF; Rec.Descr) { ApplicationArea = All; }
                field(NoteF; Rec.Note) { ApplicationArea = All; }
            }
            part(Lines; "LT QJ Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Doc No." = field("No.");
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ValidateSave)
            {
                ApplicationArea = All;

                trigger OnAction()
                begin
                    Log.Add('H.ACT(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ')');
                    Rec.Validate(Descr, 'by action');
                    CurrPage.SaveRecord();
                    Log.Add('H.ACT.end(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ')');
                end;
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetRecord()
    begin
        Log.Add('H.AGR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('H.AGCR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('H.MODR(t=' + Format(Rec.Total) + ' d=' + Rec.Descr + ' n=' + Rec.Note + ' x.t=' + Format(xRec.Total) + ' x.d=' + xRec.Descr + ')');
    end;
}

page 86887 "LT QJ Save Lines"
{
    PageType = ListPart;
    SourceTable = "LT QJ Line";
    ApplicationArea = All;
    AutoSplitKey = true;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(LineNoF; Rec."Line No.") { ApplicationArea = All; }
                field(AmtF; Rec.Amt)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('S.V.before(' + Format(Rec."Line No.") + ' r=' + Format(Rec.Amt) + ' x=' + Format(xRec.Amt) + ' x.ln=' + Format(xRec."Line No.") + ')');
                        CurrPage.SaveRecord();
                        Log.Add('S.V.after(' + Format(Rec."Line No.") + ' r=' + Format(Rec.Amt) + ' x=' + Format(xRec.Amt) + ' x.ln=' + Format(xRec."Line No.") + ' delta=' + Format(Rec.Amt - xRec.Amt) + ')');
                    end;
                }
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('S.AGCR(' + Format(Rec."Line No.") + ' r=' + Format(Rec.Amt) + ' x=' + Format(xRec.Amt) + ')');
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('S.NEW(x=' + Format(xRec.Amt) + ' x.ln=' + Format(xRec."Line No.") + ')');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('S.INSR(' + Format(Rec."Line No.") + ' r=' + Format(Rec.Amt) + ' x=' + Format(xRec.Amt) + ' x.ln=' + Format(xRec."Line No.") + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('S.MODR(' + Format(Rec."Line No.") + ' r=' + Format(Rec.Amt) + ' x=' + Format(xRec.Amt) + ')');
    end;
}

page 86888 "LT QJ Save Host"
{
    PageType = Document;
    SourceTable = "LT QJ Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoF; Rec."No.") { ApplicationArea = All; }
                field(DescrF; Rec.Descr) { ApplicationArea = All; }
            }
            part(Lines; "LT QJ Save Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Doc No." = field("No.");
            }
        }
    }
}

// ---- Writer QL additions (2026-10-21): a part edit that does not change the host row, a new part row, GoToKey and New counts ----

page 86970 "LT QL Note Lines"
{
    PageType = ListPart;
    SourceTable = "LT QJ Line";
    ApplicationArea = All;
    AutoSplitKey = true;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(LineNoF; Rec."Line No.") { ApplicationArea = All; }
                field(AmtF; Rec.Amt)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('N.Amt.V(' + Format(Rec.Amt) + ' x=' + Format(xRec.Amt) + ')');
                        CurrPage.Update(false);
                        Log.Add('N.Amt.V.end');
                    end;
                }
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetRecord()
    begin
        Log.Add('N.AGR(' + Format(Rec."Line No.") + ' a=' + Format(Rec.Amt) + ' x.a=' + Format(xRec.Amt) + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('N.AGCR(' + Format(Rec."Line No.") + ' a=' + Format(Rec.Amt) + ' x.a=' + Format(xRec.Amt) + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('N.MODR(' + Format(Rec."Line No.") + ' a=' + Format(Rec.Amt) + ' x.a=' + Format(xRec.Amt) + ')');
    end;
}

page 86971 "LT QL Note Host Both"
{
    PageType = Document;
    SourceTable = "LT QJ Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoF; Rec."No.") { ApplicationArea = All; }
                field(TotalF; Rec.Total) { ApplicationArea = All; }
            }
            part(Lines; "LT QL Note Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Doc No." = field("No.");
                UpdatePropagation = Both;
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetRecord()
    begin
        Log.Add('H.AGR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('H.AGCR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('H.MODR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;
}

page 86972 "LT QL Note Host SubPart"
{
    PageType = Document;
    SourceTable = "LT QJ Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoF; Rec."No.") { ApplicationArea = All; }
                field(TotalF; Rec.Total) { ApplicationArea = All; }
            }
            part(Lines; "LT QL Note Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Doc No." = field("No.");
                UpdatePropagation = SubPart;
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetRecord()
    begin
        Log.Add('H.AGR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('H.AGCR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('H.MODR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;
}

page 86973 "LT QL Note Host Default"
{
    PageType = Document;
    SourceTable = "LT QJ Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoF; Rec."No.") { ApplicationArea = All; }
                field(TotalF; Rec.Total) { ApplicationArea = All; }
            }
            part(Lines; "LT QL Note Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Doc No." = field("No.");
            }
        }
    }

    var
        Log: Codeunit "LT QJ Log";

    trigger OnAfterGetRecord()
    begin
        Log.Add('H.AGR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('H.AGCR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('H.MODR(t=' + Format(Rec.Total) + ' x.t=' + Format(xRec.Total) + ')');
    end;
}

codeunit 86989 "LT QL Count Tests"
{
    var
        Log: Codeunit "LT QJ Log";

    local procedure Seed()
    var
        Head: Record "LT QJ Head";
        Line: Record "LT QJ Line";
    begin
        Line.DeleteAll();
        Head.DeleteAll();
        Head."No." := 'H1';
        Head.Total := 1;
        Head.Descr := 'orig';
        Head.Note := 'orig';
        Head.Insert();
        Line."Doc No." := 'H1';
        Line."Line No." := 10000;
        Line.Qty := 1;
        Line.Amt := 1;
        Line.Insert();
        Log.Reset();
    end;

    local procedure Stored(): Text
    var
        Head: Record "LT QJ Head";
    begin
        if not Head.Get('H1') then
            exit('<no header>');
        exit('t=' + Format(Head.Total) + ' d=' + Head.Descr + ' n=' + Head.Note);
    end;

    local procedure StoredLines(): Text
    var
        Line: Record "LT QJ Line";
        Result: Text;
    begin
        Line.SetRange("Doc No.", 'H1');
        if Line.FindSet() then
            repeat
                if Result <> '' then
                    Result += ' ';
                Result += Format(Line."Line No.") + ':q=' + Format(Line.Qty) + ',a=' + Format(Line.Amt);
            until Line.Next() = 0;
        exit(Result);
    end;

    // ---- (1) part OnValidate modifies the host row, then the host action validates and saves ----

    local procedure SeedQl()
    var
        Head: Record "LT QJ Head";
        Line: Record "LT QJ Line";
    begin
        Seed();
        Head."No." := 'H2';
        Head.Total := 2;
        Head.Descr := 'two';
        Head.Note := 'two';
        Head.Insert();
        Head."No." := 'H3';
        Head.Total := 3;
        Head.Descr := 'three';
        Head.Note := 'three';
        Head.Insert();
        Line."Doc No." := 'H2';
        Line."Line No." := 30000;
        Line.Qty := 3;
        Line.Amt := 3;
        Line.Insert();
        Line."Line No." := 40000;
        Line.Qty := 4;
        Line.Amt := 4;
        Line.Insert();
        Log.Reset();
    end;

    local procedure StoredLinesOf(DocNo: Code[20]): Text
    var
        Line: Record "LT QJ Line";
        Result: Text;
    begin
        Line.SetRange("Doc No.", DocNo);
        if Line.FindSet() then
            repeat
                if Result <> '' then
                    Result += ' ';
                Result += Format(Line."Line No.") + ':q=' + Format(Line.Qty) + ',a=' + Format(Line.Amt);
            until Line.Next() = 0;
        exit(Result);
    end;
    procedure QL_Both_NewPartRow_PartModifiesHost(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Both";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        Log.Take();
        TP.Lines.Last();
        TP.Lines.New();
        R += '|log-new=' + Log.Take();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|part-line-after-part=' + TP.Lines.LineNoF.Value();
        R += '|part-qty-after-part=' + TP.Lines.QtyF.Value();
        R += '|stored-after-part=' + Stored();
        R += '|lines-after-part=' + StoredLines();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QL_SubPart_NewPartRow_PartModifiesHost(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host SubPart";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        Log.Take();
        TP.Lines.Last();
        TP.Lines.New();
        R += '|log-new=' + Log.Take();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|part-line-after-part=' + TP.Lines.LineNoF.Value();
        R += '|part-qty-after-part=' + TP.Lines.QtyF.Value();
        R += '|stored-after-part=' + Stored();
        R += '|lines-after-part=' + StoredLines();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QL_Default_NewPartRow_PartModifiesHost(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Default";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        Log.Take();
        TP.Lines.Last();
        TP.Lines.New();
        R += '|log-new=' + Log.Take();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|part-line-after-part=' + TP.Lines.LineNoF.Value();
        R += '|part-qty-after-part=' + TP.Lines.QtyF.Value();
        R += '|stored-after-part=' + Stored();
        R += '|lines-after-part=' + StoredLines();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QL_Both_PartUpdate_HostRowUnchanged(): Text
    var
        R: Text;
        TP: TestPage "LT QL Note Host Both";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        TP.Lines.AmtF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|stored-after-part=' + Stored();
        R += '|lines-after-part=' + StoredLines();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QL_SubPart_PartUpdate_HostRowUnchanged(): Text
    var
        R: Text;
        TP: TestPage "LT QL Note Host SubPart";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        TP.Lines.AmtF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|stored-after-part=' + Stored();
        R += '|lines-after-part=' + StoredLines();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QL_Default_PartUpdate_HostRowUnchanged(): Text
    var
        R: Text;
        TP: TestPage "LT QL Note Host Default";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        TP.Lines.AmtF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|stored-after-part=' + Stored();
        R += '|lines-after-part=' + StoredLines();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QL_Both_GoToKey_Counts(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Both";
    begin
        SeedQl();
        TP.OpenEdit();
        R += '|log-open=' + Log.Take();
        TP.GoToKey('H2');
        R += '|log-gotokey-h2=' + Log.Take();
        TP.GoToKey('H2');
        R += '|log-gotokey-h2-again=' + Log.Take();
        TP.GoToKey('H1');
        R += '|log-gotokey-h1=' + Log.Take();
        R += '|part-line-at-h1=' + TP.Lines.LineNoF.Value();
        TP.Close();
        R += '|log-close=' + Log.Take();
        exit(R);
    end;

    procedure QL_Default_GoToKey_Counts(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Default";
    begin
        SeedQl();
        TP.OpenEdit();
        R += '|log-open=' + Log.Take();
        TP.GoToKey('H2');
        R += '|log-gotokey-h2=' + Log.Take();
        TP.GoToKey('H2');
        R += '|log-gotokey-h2-again=' + Log.Take();
        TP.GoToKey('H1');
        R += '|log-gotokey-h1=' + Log.Take();
        R += '|part-line-at-h1=' + TP.Lines.LineNoF.Value();
        TP.Close();
        R += '|log-close=' + Log.Take();
        exit(R);
    end;

    procedure QL_OpenView_GoToKey_Counts(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Default";
    begin
        SeedQl();
        TP.OpenView();
        R += '|log-open=' + Log.Take();
        TP.GoToKey('H2');
        R += '|log-gotokey-h2=' + Log.Take();
        TP.GoToKey('H1');
        R += '|log-gotokey-h1=' + Log.Take();
        TP.Close();
        R += '|log-close=' + Log.Take();
        exit(R);
    end;

    procedure QL_PartNew_WithoutLast(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        SeedQl();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        TP.Lines.New();
        R += '|log-1-new=' + Log.Take();
        R += '|part-line=' + TP.Lines.LineNoF.Value();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLinesOf('H1');
        exit(R);
    end;

    procedure QL_PartLastNew_TwoLines(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        SeedQl();
        TP.OpenEdit();
        TP.GoToKey('H2');
        R += '|log-open=' + Log.Take();
        R += '|last-result=' + Format(TP.Lines.Last());
        R += '|log-1-last=' + Log.Take();
        TP.Lines.New();
        R += '|log-2-new=' + Log.Take();
        R += '|part-line=' + TP.Lines.LineNoF.Value();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLinesOf('H2');
        exit(R);
    end;

    procedure QL_PartFirstNew_TwoLines(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        SeedQl();
        TP.OpenEdit();
        TP.GoToKey('H2');
        R += '|log-open=' + Log.Take();
        R += '|first-result=' + Format(TP.Lines.First());
        R += '|log-1-first=' + Log.Take();
        TP.Lines.New();
        R += '|log-2-new=' + Log.Take();
        R += '|part-line=' + TP.Lines.LineNoF.Value();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLinesOf('H2');
        exit(R);
    end;

    procedure QL_PartLastNew_NoLines(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        SeedQl();
        TP.OpenEdit();
        TP.GoToKey('H3');
        R += '|log-open=' + Log.Take();
        R += '|last-result=' + Format(TP.Lines.Last());
        R += '|log-1-last=' + Log.Take();
        TP.Lines.New();
        R += '|log-2-new=' + Log.Take();
        R += '|part-line=' + TP.Lines.LineNoF.Value();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLinesOf('H3');
        exit(R);
    end;

    procedure QL_PartLastNewNew(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        SeedQl();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        R += '|last-result=' + Format(TP.Lines.Last());
        R += '|log-1-last=' + Log.Take();
        TP.Lines.New();
        R += '|log-2-new=' + Log.Take();
        TP.Lines.New();
        R += '|log-3-new=' + Log.Take();
        R += '|part-line=' + TP.Lines.LineNoF.Value();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLinesOf('H1');
        exit(R);
    end;
    procedure QN_EmptyPart_HostNext(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        SeedQl();
        TP.OpenEdit();
        TP.Next();
        Log.Take();
        R += '|next-result=' + Format(TP.Next());
        R += '|host-no=' + TP.NoF.Value();
        R += '|log-move=' + Log.Take();
        R += '|last-result=' + Format(TP.Lines.Last());
        R += '|log-last=' + Log.Take();
        TP.Lines.New();
        R += '|log-new=' + Log.Take();
        TP.Close();
        R += '|log-close=' + Log.Take();
        exit(R);
    end;

    procedure QN_EmptyPart_HostLast(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        SeedQl();
        TP.OpenEdit();
        Log.Take();
        R += '|last-host-result=' + Format(TP.Last());
        R += '|log-move=' + Log.Take();
        R += '|last-result=' + Format(TP.Lines.Last());
        R += '|log-last=' + Log.Take();
        TP.Lines.New();
        R += '|log-new=' + Log.Take();
        exit(R);
    end;
}
"#;

fn run(proc_name: &str) -> Vec<String> {
    let t = al_testkit::text(al_testkit::run_in(SRC, "LT QL Count Tests", proc_name));
    t.split('|').skip(1).map(str::to_string).collect()
}

fn expected(pairs: &[(&str, &str)]) -> Vec<String> {
    pairs.iter().map(|(k, v)| format!("{k}={v}")).collect()
}

/// QjPartPropagation.al:995 QL_Both_NewPartRow_PartModifiesHost.
#[test]
fn ql_both_new_part_row_part_modifies_host() {
    let want = expected(&[
        (
            "log-new",
            "L.AGCR(0 q=0 x.q=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1) L.AGCR(0 q=0 x.q=0)",
        ),
        (
            "log-part-edit",
            "L.Qty.V(5 x=0) L.Qty.V.end H.AGR(t=6 d=orig n=orig x.t=1) H.AGCR(t=6 d=orig n=orig x.t=1)",
        ),
        ("host-total-after-part", "6.00"),
        ("part-line-after-part", "0"),
        ("part-qty-after-part", "5.00"),
        ("stored-after-part", "t=6 d=orig n=orig"),
        ("lines-after-part", "10000:q=1,a=1"),
        ("log-close", ""),
        ("stored-after-close", "t=6 d=orig n=orig"),
        ("lines-after-close", "10000:q=1,a=1 20000:q=5,a=0"),
    ]);
    assert_eq!(run("QL_Both_NewPartRow_PartModifiesHost"), want);
}

/// QjPartPropagation.al:1032 QL_SubPart_NewPartRow_PartModifiesHost.
#[test]
fn ql_subpart_new_part_row_part_modifies_host() {
    let want = expected(&[
        (
            "log-new",
            "L.AGCR(0 q=0 x.q=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1) L.AGCR(0 q=0 x.q=0)",
        ),
        (
            "log-part-edit",
            "L.Qty.V(5 x=0) L.Qty.V.end L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1) L.AGCR(0 q=0 x.q=0) H.AGR(t=6 d=orig n=orig x.t=1) H.AGCR(t=6 d=orig n=orig x.t=1)",
        ),
        ("host-total-after-part", "6.00"),
        ("part-line-after-part", "0"),
        ("part-qty-after-part", "0.00"),
        ("stored-after-part", "t=6 d=orig n=orig"),
        ("lines-after-part", "10000:q=1,a=1"),
        ("log-close", ""),
        ("stored-after-close", "t=6 d=orig n=orig"),
        ("lines-after-close", "10000:q=1,a=1"),
    ]);
    assert_eq!(run("QL_SubPart_NewPartRow_PartModifiesHost"), want);
}

/// QjPartPropagation.al:1069 QL_Default_NewPartRow_PartModifiesHost.
#[test]
fn ql_default_new_part_row_part_modifies_host() {
    let want = expected(&[
        (
            "log-new",
            "L.AGCR(0 q=0 x.q=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1) L.AGCR(0 q=0 x.q=0)",
        ),
        (
            "log-part-edit",
            "L.Qty.V(5 x=0) L.Qty.V.end L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1) L.AGCR(0 q=0 x.q=0) H.AGR(t=6 d=orig n=orig x.t=1) H.AGCR(t=6 d=orig n=orig x.t=1)",
        ),
        ("host-total-after-part", "6.00"),
        ("part-line-after-part", "0"),
        ("part-qty-after-part", "0.00"),
        ("stored-after-part", "t=6 d=orig n=orig"),
        ("lines-after-part", "10000:q=1,a=1"),
        ("log-close", ""),
        ("stored-after-close", "t=6 d=orig n=orig"),
        ("lines-after-close", "10000:q=1,a=1"),
    ]);
    assert_eq!(run("QL_Default_NewPartRow_PartModifiesHost"), want);
}

/// QjPartPropagation.al:1106 QL_Both_PartUpdate_HostRowUnchanged.
#[test]
fn ql_both_part_update_host_row_unchanged() {
    let want = expected(&[
        (
            "log-open",
            "H.AGR(t=1 x.t=0) H.AGCR(t=1 x.t=0) N.AGR(10000 a=1 x.a=0) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=0) H.AGR(t=1 x.t=1) H.AGCR(t=1 x.t=1)",
        ),
        (
            "log-part-edit",
            "N.Amt.V(5 x=1) N.Amt.V.end H.AGR(t=1 x.t=1) H.AGCR(t=1 x.t=1) N.AGR(10000 a=1 x.a=1) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=5)",
        ),
        ("host-total-after-part", "1.00"),
        ("stored-after-part", "t=1 d=orig n=orig"),
        ("lines-after-part", "10000:q=1,a=1"),
        ("log-close", ""),
        ("lines-after-close", "10000:q=1,a=1"),
    ]);
    assert_eq!(run("QL_Both_PartUpdate_HostRowUnchanged"), want);
}

/// QjPartPropagation.al:1134 QL_SubPart_PartUpdate_HostRowUnchanged.
#[test]
fn ql_subpart_part_update_host_row_unchanged() {
    let want = expected(&[
        (
            "log-open",
            "H.AGR(t=1 x.t=0) H.AGCR(t=1 x.t=0) N.AGR(10000 a=1 x.a=0) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=0) H.AGR(t=1 x.t=1) H.AGCR(t=1 x.t=1)",
        ),
        (
            "log-part-edit",
            "N.Amt.V(5 x=1) N.Amt.V.end N.AGR(10000 a=1 x.a=1) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=5)",
        ),
        ("host-total-after-part", "1.00"),
        ("stored-after-part", "t=1 d=orig n=orig"),
        ("lines-after-part", "10000:q=1,a=1"),
        ("log-close", ""),
        ("lines-after-close", "10000:q=1,a=1"),
    ]);
    assert_eq!(run("QL_SubPart_PartUpdate_HostRowUnchanged"), want);
}

/// QjPartPropagation.al:1162 QL_Default_PartUpdate_HostRowUnchanged.
#[test]
fn ql_default_part_update_host_row_unchanged() {
    let want = expected(&[
        (
            "log-open",
            "H.AGR(t=1 x.t=0) H.AGCR(t=1 x.t=0) N.AGR(10000 a=1 x.a=0) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=0) H.AGR(t=1 x.t=1) H.AGCR(t=1 x.t=1)",
        ),
        (
            "log-part-edit",
            "N.Amt.V(5 x=1) N.Amt.V.end N.AGR(10000 a=1 x.a=1) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=5)",
        ),
        ("host-total-after-part", "1.00"),
        ("stored-after-part", "t=1 d=orig n=orig"),
        ("lines-after-part", "10000:q=1,a=1"),
        ("log-close", ""),
        ("lines-after-close", "10000:q=1,a=1"),
    ]);
    assert_eq!(run("QL_Default_PartUpdate_HostRowUnchanged"), want);
}

/// QjPartPropagation.al:1190 QL_Both_GoToKey_Counts.
#[test]
fn ql_both_gotokey_counts() {
    let want = expected(&[
        (
            "log-open",
            "H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0)",
        ),
        (
            "log-gotokey-h2",
            "H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1) H.AGR(t=2 d=two n=two x.t=1) H.AGCR(t=2 d=two n=two x.t=1) L.AGR(30000 q=3 x.q=1) L.AGR(40000 q=4 x.q=3) L.AGR(30000 q=3 x.q=4) L.AGR(30000 q=3 x.q=3) L.AGCR(30000 q=3 x.q=1)",
        ),
        (
            "log-gotokey-h2-again",
            "H.AGR(t=1 d=orig n=orig x.t=2) H.AGCR(t=1 d=orig n=orig x.t=2) L.AGR(10000 q=1 x.q=3) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=3) H.AGR(t=2 d=two n=two x.t=1) H.AGCR(t=2 d=two n=two x.t=1) L.AGR(30000 q=3 x.q=1) L.AGR(40000 q=4 x.q=3) L.AGR(30000 q=3 x.q=4) L.AGR(30000 q=3 x.q=3) L.AGCR(30000 q=3 x.q=1)",
        ),
        (
            "log-gotokey-h1",
            "H.AGR(t=1 d=orig n=orig x.t=2) H.AGCR(t=1 d=orig n=orig x.t=2) L.AGR(10000 q=1 x.q=3) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=3)",
        ),
        ("part-line-at-h1", "10000"),
        ("log-close", ""),
    ]);
    assert_eq!(run("QL_Both_GoToKey_Counts"), want);
}

/// QjPartPropagation.al:1217 QL_Default_GoToKey_Counts.
#[test]
fn ql_default_gotokey_counts() {
    let want = expected(&[
        (
            "log-open",
            "H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0)",
        ),
        (
            "log-gotokey-h2",
            "H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1) H.AGR(t=2 d=two n=two x.t=1) H.AGCR(t=2 d=two n=two x.t=1) L.AGR(30000 q=3 x.q=1) L.AGR(40000 q=4 x.q=3) L.AGR(30000 q=3 x.q=4) L.AGR(30000 q=3 x.q=3) L.AGCR(30000 q=3 x.q=1)",
        ),
        (
            "log-gotokey-h2-again",
            "H.AGR(t=1 d=orig n=orig x.t=2) H.AGCR(t=1 d=orig n=orig x.t=2) L.AGR(10000 q=1 x.q=3) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=3) H.AGR(t=2 d=two n=two x.t=1) H.AGCR(t=2 d=two n=two x.t=1) L.AGR(30000 q=3 x.q=1) L.AGR(40000 q=4 x.q=3) L.AGR(30000 q=3 x.q=4) L.AGR(30000 q=3 x.q=3) L.AGCR(30000 q=3 x.q=1)",
        ),
        (
            "log-gotokey-h1",
            "H.AGR(t=1 d=orig n=orig x.t=2) H.AGCR(t=1 d=orig n=orig x.t=2) L.AGR(10000 q=1 x.q=3) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=3)",
        ),
        ("part-line-at-h1", "10000"),
        ("log-close", ""),
    ]);
    assert_eq!(run("QL_Default_GoToKey_Counts"), want);
}

/// QjPartPropagation.al:1244 QL_OpenView_GoToKey_Counts.
#[test]
fn ql_open_view_gotokey_counts() {
    let want = expected(&[
        (
            "log-open",
            "H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0)",
        ),
        (
            "log-gotokey-h2",
            "H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1) H.AGR(t=2 d=two n=two x.t=1) H.AGCR(t=2 d=two n=two x.t=1) L.AGR(30000 q=3 x.q=1) L.AGR(40000 q=4 x.q=3) L.AGR(30000 q=3 x.q=4) L.AGR(30000 q=3 x.q=3) L.AGCR(30000 q=3 x.q=1)",
        ),
        (
            "log-gotokey-h1",
            "H.AGR(t=1 d=orig n=orig x.t=2) H.AGCR(t=1 d=orig n=orig x.t=2) L.AGR(10000 q=1 x.q=3) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=3)",
        ),
        ("log-close", ""),
    ]);
    assert_eq!(run("QL_OpenView_GoToKey_Counts"), want);
}

/// QjPartPropagation.al:1266 QL_PartNew_WithoutLast.
#[test]
fn ql_part_new_without_last() {
    let want = expected(&[
        ("log-open", "S.AGCR(10000 r=1 x=0)"),
        ("log-1-new", "S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0)"),
        ("part-line", "0"),
        ("log-close", ""),
        ("lines-after-close", "10000:q=1,a=1"),
    ]);
    assert_eq!(run("QL_PartNew_WithoutLast"), want);
}

/// QjPartPropagation.al:1290 QL_PartLastNew_TwoLines.
#[test]
fn ql_part_last_new_two_lines() {
    let want = expected(&[
        ("log-open", "S.AGCR(10000 r=1 x=0) S.AGCR(30000 r=3 x=1)"),
        ("last-result", "Yes"),
        (
            "log-1-last",
            "S.NEW(x=4 x.ln=40000) S.AGCR(0 r=0 x=0) S.AGCR(40000 r=4 x=3)",
        ),
        ("log-2-new", "S.NEW(x=4 x.ln=40000) S.AGCR(0 r=0 x=0)"),
        ("part-line", "0"),
        ("log-close", ""),
        ("lines-after-close", "30000:q=3,a=3 40000:q=4,a=4"),
    ]);
    assert_eq!(run("QL_PartLastNew_TwoLines"), want);
}

/// QjPartPropagation.al:1318 QL_PartFirstNew_TwoLines.
#[test]
fn ql_part_first_new_two_lines() {
    let want = expected(&[
        ("log-open", "S.AGCR(10000 r=1 x=0) S.AGCR(30000 r=3 x=1)"),
        ("first-result", "Yes"),
        ("log-1-first", ""),
        ("log-2-new", "S.NEW(x=4 x.ln=40000) S.AGCR(0 r=0 x=0)"),
        ("part-line", "0"),
        ("log-close", ""),
        ("lines-after-close", "30000:q=3,a=3 40000:q=4,a=4"),
    ]);
    assert_eq!(run("QL_PartFirstNew_TwoLines"), want);
}

/// QjPartPropagation.al:1346 QL_PartLastNew_NoLines.
#[test]
fn ql_part_last_new_no_lines() {
    let want = expected(&[
        (
            "log-open",
            "S.AGCR(10000 r=1 x=0) S.AGCR(30000 r=3 x=1) S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0) S.NEW(x=0 x.ln=0) S.AGCR(0 r=0 x=0) S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0)",
        ),
        ("last-result", "No"),
        (
            "log-1-last",
            "S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0) S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0)",
        ),
        ("log-2-new", "S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0)"),
        ("part-line", "0"),
        ("log-close", ""),
        ("lines-after-close", ""),
    ]);
    assert_eq!(run("QL_PartLastNew_NoLines"), want);
}

/// QjPartPropagation.al:1374 QL_PartLastNewNew.
#[test]
fn ql_part_last_new_new() {
    let want = expected(&[
        ("log-open", "S.AGCR(10000 r=1 x=0)"),
        ("last-result", "Yes"),
        (
            "log-1-last",
            "S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0) S.AGCR(10000 r=1 x=1)",
        ),
        ("log-2-new", "S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0)"),
        ("log-3-new", "S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0)"),
        ("part-line", "0"),
        ("log-close", ""),
        ("lines-after-close", "10000:q=1,a=1"),
    ]);
    assert_eq!(run("QL_PartLastNewNew"), want);
}

/// QjPartPropagation.al:2468 QN_EmptyPart_HostNext (corpus bump 44): an empty part after a host Next runs OnNewRecord three
/// times, the first and third with the part row shown before as xRec.
#[test]
fn qn_empty_part_host_next() {
    let three = "S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0) S.NEW(x=0 x.ln=0) S.AGCR(0 r=0 x=0) S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0)";
    let want = expected(&[
        ("next-result", "Yes"),
        ("host-no", "H3"),
        ("log-move", three),
        ("last-result", "No"),
        (
            "log-last",
            "S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0) S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0)",
        ),
        ("log-new", "S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0)"),
        ("log-close", ""),
    ]);
    assert_eq!(run("QN_EmptyPart_HostNext"), want);
}

/// QjPartPropagation.al:2561 QN_EmptyPart_HostLast.
#[test]
fn qn_empty_part_host_last() {
    let want = expected(&[
        ("last-host-result", "Yes"),
        (
            "log-move",
            "S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0) S.NEW(x=0 x.ln=0) S.AGCR(0 r=0 x=0) S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0)",
        ),
        ("last-result", "No"),
        (
            "log-last",
            "S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0) S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0)",
        ),
        ("log-new", "S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0)"),
    ]);
    assert_eq!(run("QN_EmptyPart_HostLast"), want);
}
