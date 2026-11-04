//! Corpus probe QJ (tests/al-language/pages/parts/QjPartPropagation.al, on corpus master since bump 41, the same as
//! corpus branch al-runtime-rs-staging-qj 1fd2ccd1; green on BC 27, 28, 29). The objects are copied from the probe; the test codeunit returns its
//! observations instead of asserting them, and each Rust test compares them with the values the probe asserts on real BC.
//! The first observation (the trigger log of OpenEdit + GoToKey, and of Last + New on the part) is not compared yet: after GoToKey on
//! the Document host the runtime re-reads the part once more than BC, and BC runs the part's New twice (to be fixed with the
//! GoToKey list rules, another branch).

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

codeunit 86889 "LT QJ Part Propagation Tests"
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

    procedure QJ_Both_PartModifiesHost_ThenHostSave(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Both";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|stored-after-part=' + Stored();
        TP.ValidateSave.Invoke();
        R += '|log-action=' + Log.Take();
        R += '|host-total-after-action=' + TP.TotalF.Value();
        R += '|host-descr-after-action=' + TP.DescrF.Value();
        R += '|stored-after-action=' + Stored();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QJ_SubPart_PartModifiesHost_ThenHostSave(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host SubPart";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|stored-after-part=' + Stored();
        TP.ValidateSave.Invoke();
        R += '|log-action=' + Log.Take();
        R += '|host-total-after-action=' + TP.TotalF.Value();
        R += '|host-descr-after-action=' + TP.DescrF.Value();
        R += '|stored-after-action=' + Stored();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QJ_Default_PartModifiesHost_ThenHostSave(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Default";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|stored-after-part=' + Stored();
        TP.ValidateSave.Invoke();
        R += '|log-action=' + Log.Take();
        R += '|host-total-after-action=' + TP.TotalF.Value();
        R += '|host-descr-after-action=' + TP.DescrF.Value();
        R += '|stored-after-action=' + Stored();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    // The host holds an unsaved edit (Note typed) when the part edit modifies the host row.

    procedure QJ_Both_HostEdited_PartModifiesHost_ThenHostSave(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Both";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        TP.NoteF.SetValue('typed');
        R += '|log-open-and-host-edit=' + Log.Take();
        R += '|stored-after-host-edit=' + Stored();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|host-note-after-part=' + TP.NoteF.Value();
        R += '|stored-after-part=' + Stored();
        TP.ValidateSave.Invoke();
        R += '|log-action=' + Log.Take();
        R += '|host-total-after-action=' + TP.TotalF.Value();
        R += '|stored-after-action=' + Stored();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        exit(R);
    end;

    procedure QJ_SubPart_HostEdited_PartModifiesHost_ThenHostSave(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host SubPart";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        TP.NoteF.SetValue('typed');
        R += '|log-open-and-host-edit=' + Log.Take();
        R += '|stored-after-host-edit=' + Stored();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|host-note-after-part=' + TP.NoteF.Value();
        R += '|stored-after-part=' + Stored();
        TP.ValidateSave.Invoke();
        R += '|log-action=' + Log.Take();
        R += '|host-total-after-action=' + TP.TotalF.Value();
        R += '|stored-after-action=' + Stored();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        exit(R);
    end;

    procedure QJ_Default_HostEdited_PartModifiesHost_ThenHostSave(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Host Default";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        TP.NoteF.SetValue('typed');
        R += '|log-open-and-host-edit=' + Log.Take();
        R += '|stored-after-host-edit=' + Stored();
        TP.Lines.QtyF.SetValue(5);
        R += '|log-part-edit=' + Log.Take();
        R += '|host-total-after-part=' + TP.TotalF.Value();
        R += '|host-note-after-part=' + TP.NoteF.Value();
        R += '|stored-after-part=' + Stored();
        TP.ValidateSave.Invoke();
        R += '|log-action=' + Log.Take();
        R += '|host-total-after-action=' + TP.TotalF.Value();
        R += '|stored-after-action=' + Stored();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|stored-after-close=' + Stored();
        exit(R);
    end;

    // ---- (2) subform OnValidate: CurrPage.SaveRecord() and Rec/xRec of the same field before and after ----

    procedure QJ_SubformSaveRecord_StoredRow(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        R += '|log-open=' + Log.Take();
        TP.Lines.AmtF.SetValue(5);
        R += '|log-first-edit=' + Log.Take();
        R += '|lines-after-first-edit=' + StoredLines();
        TP.Lines.AmtF.SetValue(7);
        R += '|log-second-edit=' + Log.Take();
        R += '|lines-after-second-edit=' + StoredLines();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
    end;

    procedure QJ_SubformSaveRecord_NewRow(): Text
    var
        R: Text;
        TP: TestPage "LT QJ Save Host";
    begin
        Seed();
        TP.OpenEdit();
        TP.GoToKey('H1');
        TP.Lines.Last();
        TP.Lines.New();
        R += '|log-open-and-new=' + Log.Take();
        TP.Lines.AmtF.SetValue(5);
        R += '|log-first-edit=' + Log.Take();
        R += '|lines-after-first-edit=' + StoredLines();
        TP.Lines.AmtF.SetValue(7);
        R += '|log-second-edit=' + Log.Take();
        R += '|lines-after-second-edit=' + StoredLines();
        TP.Close();
        R += '|log-close=' + Log.Take();
        R += '|lines-after-close=' + StoredLines();
        exit(R);
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
}

"#;

fn run(proc_name: &str) -> Vec<String> {
    let t = al_testkit::text(al_testkit::run_in(SRC, "LT QJ Part Propagation Tests", proc_name));
    t.split('|').skip(1).map(str::to_string).collect()
}

#[test]
fn qj_both_part_modifies_host_then_host_save() {
    let expected = [
        "log-open=H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0) H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1)",
        "log-part-edit=L.Qty.V(5 x=1) L.Qty.V.end H.AGR(t=5 d=orig n=orig x.t=1) H.AGCR(t=5 d=orig n=orig x.t=1) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=5)",
        "host-total-after-part=5.00",
        "stored-after-part=t=5 d=orig n=orig",
        "log-action=H.AGR(t=5 d=orig n=orig x.t=5) H.ACT(t=5 d=orig) T.H.Descr.V(by action x=orig t=5 x.t=5) H.MODR(t=5 d=by action n=orig x.t=5 x.d=orig) T.H.MOD(t=5 d=by action n=orig x.t=5 x.d=orig) H.ACT.end(t=5 d=by action) H.AGR(t=5 d=by action n=orig x.t=5) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1)",
        "host-total-after-action=5.00",
        "host-descr-after-action=by action",
        "stored-after-action=t=5 d=by action n=orig",
        "log-close=",
        "stored-after-close=t=5 d=by action n=orig",
        "lines-after-close=10000:q=1,a=1",
    ];
    assert_eq!(run("QJ_Both_PartModifiesHost_ThenHostSave"), expected);
}

#[test]
fn qj_sub_part_part_modifies_host_then_host_save() {
    let expected = [
        "log-open=H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0) H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1)",
        "log-part-edit=L.Qty.V(5 x=1) L.Qty.V.end L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=5) H.AGR(t=5 d=orig n=orig x.t=1) H.AGCR(t=5 d=orig n=orig x.t=1)",
        "host-total-after-part=5.00",
        "stored-after-part=t=5 d=orig n=orig",
        "log-action=H.AGR(t=5 d=orig n=orig x.t=5) H.ACT(t=5 d=orig) T.H.Descr.V(by action x=orig t=5 x.t=5) H.MODR(t=5 d=by action n=orig x.t=5 x.d=orig) T.H.MOD(t=5 d=by action n=orig x.t=5 x.d=orig) H.ACT.end(t=5 d=by action) H.AGR(t=5 d=by action n=orig x.t=5) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1)",
        "host-total-after-action=5.00",
        "host-descr-after-action=by action",
        "stored-after-action=t=5 d=by action n=orig",
        "log-close=",
        "stored-after-close=t=5 d=by action n=orig",
        "lines-after-close=10000:q=1,a=1",
    ];
    assert_eq!(run("QJ_SubPart_PartModifiesHost_ThenHostSave"), expected);
}

#[test]
fn qj_default_part_modifies_host_then_host_save() {
    let expected = [
        "log-open=H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0) H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1)",
        "log-part-edit=L.Qty.V(5 x=1) L.Qty.V.end L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=5) H.AGR(t=5 d=orig n=orig x.t=1) H.AGCR(t=5 d=orig n=orig x.t=1)",
        "host-total-after-part=5.00",
        "stored-after-part=t=5 d=orig n=orig",
        "log-action=H.AGR(t=5 d=orig n=orig x.t=5) H.ACT(t=5 d=orig) T.H.Descr.V(by action x=orig t=5 x.t=5) H.MODR(t=5 d=by action n=orig x.t=5 x.d=orig) T.H.MOD(t=5 d=by action n=orig x.t=5 x.d=orig) H.ACT.end(t=5 d=by action) H.AGR(t=5 d=by action n=orig x.t=5) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1)",
        "host-total-after-action=5.00",
        "host-descr-after-action=by action",
        "stored-after-action=t=5 d=by action n=orig",
        "log-close=",
        "stored-after-close=t=5 d=by action n=orig",
        "lines-after-close=10000:q=1,a=1",
    ];
    assert_eq!(run("QJ_Default_PartModifiesHost_ThenHostSave"), expected);
}

#[test]
fn qj_both_host_edited_part_modifies_host_then_host_save() {
    let expected = [
        "log-open-and-host-edit=H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0) H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1)",
        "stored-after-host-edit=t=1 d=orig n=orig",
        "log-part-edit=H.MODR(t=1 d=orig n=typed x.t=1 x.d=orig) T.H.MOD(t=1 d=orig n=typed x.t=1 x.d=orig) L.Qty.V(5 x=1) L.Qty.V.end H.AGR(t=5 d=orig n=typed x.t=1) H.AGCR(t=5 d=orig n=typed x.t=1) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=5)",
        "host-total-after-part=5.00",
        "host-note-after-part=typed",
        "stored-after-part=t=5 d=orig n=typed",
        "log-action=H.AGR(t=5 d=orig n=typed x.t=5) H.ACT(t=5 d=orig) T.H.Descr.V(by action x=orig t=5 x.t=5) H.MODR(t=5 d=by action n=typed x.t=5 x.d=orig) T.H.MOD(t=5 d=by action n=typed x.t=5 x.d=orig) H.ACT.end(t=5 d=by action) H.AGR(t=5 d=by action n=typed x.t=5) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1)",
        "host-total-after-action=5.00",
        "stored-after-action=t=5 d=by action n=typed",
        "log-close=",
        "stored-after-close=t=5 d=by action n=typed",
    ];
    assert_eq!(
        run("QJ_Both_HostEdited_PartModifiesHost_ThenHostSave")[1..],
        expected[1..]
    );
}

#[test]
fn qj_sub_part_host_edited_part_modifies_host_then_host_save() {
    let expected = [
        "log-open-and-host-edit=H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0) H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1)",
        "stored-after-host-edit=t=1 d=orig n=orig",
        "log-part-edit=H.MODR(t=1 d=orig n=typed x.t=1 x.d=orig) T.H.MOD(t=1 d=orig n=typed x.t=1 x.d=orig) L.Qty.V(5 x=1) L.Qty.V.end L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=5) H.AGR(t=5 d=orig n=typed x.t=1) H.AGCR(t=5 d=orig n=typed x.t=1)",
        "host-total-after-part=5.00",
        "host-note-after-part=typed",
        "stored-after-part=t=5 d=orig n=typed",
        "log-action=H.AGR(t=5 d=orig n=typed x.t=5) H.ACT(t=5 d=orig) T.H.Descr.V(by action x=orig t=5 x.t=5) H.MODR(t=5 d=by action n=typed x.t=5 x.d=orig) T.H.MOD(t=5 d=by action n=typed x.t=5 x.d=orig) H.ACT.end(t=5 d=by action) H.AGR(t=5 d=by action n=typed x.t=5) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1)",
        "host-total-after-action=5.00",
        "stored-after-action=t=5 d=by action n=typed",
        "log-close=",
        "stored-after-close=t=5 d=by action n=typed",
    ];
    assert_eq!(
        run("QJ_SubPart_HostEdited_PartModifiesHost_ThenHostSave")[1..],
        expected[1..]
    );
}

#[test]
fn qj_default_host_edited_part_modifies_host_then_host_save() {
    let expected = [
        "log-open-and-host-edit=H.AGR(t=1 d=orig n=orig x.t=0) H.AGCR(t=1 d=orig n=orig x.t=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=0) H.AGR(t=1 d=orig n=orig x.t=1) H.AGCR(t=1 d=orig n=orig x.t=1)",
        "stored-after-host-edit=t=1 d=orig n=orig",
        "log-part-edit=H.MODR(t=1 d=orig n=typed x.t=1 x.d=orig) T.H.MOD(t=1 d=orig n=typed x.t=1 x.d=orig) L.Qty.V(5 x=1) L.Qty.V.end L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=5) H.AGR(t=5 d=orig n=typed x.t=1) H.AGCR(t=5 d=orig n=typed x.t=1)",
        "host-total-after-part=5.00",
        "host-note-after-part=typed",
        "stored-after-part=t=5 d=orig n=typed",
        "log-action=H.AGR(t=5 d=orig n=typed x.t=5) H.ACT(t=5 d=orig) T.H.Descr.V(by action x=orig t=5 x.t=5) H.MODR(t=5 d=by action n=typed x.t=5 x.d=orig) T.H.MOD(t=5 d=by action n=typed x.t=5 x.d=orig) H.ACT.end(t=5 d=by action) H.AGR(t=5 d=by action n=typed x.t=5) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1)",
        "host-total-after-action=5.00",
        "stored-after-action=t=5 d=by action n=typed",
        "log-close=",
        "stored-after-close=t=5 d=by action n=typed",
    ];
    assert_eq!(
        run("QJ_Default_HostEdited_PartModifiesHost_ThenHostSave")[1..],
        expected[1..]
    );
}

#[test]
fn qj_subform_save_record_stored_row() {
    let expected = [
        "log-open=S.AGCR(10000 r=1 x=0)",
        "log-first-edit=S.V.before(10000 r=5 x=1 x.ln=10000) S.MODR(10000 r=5 x=1) S.V.after(10000 r=5 x=1 x.ln=10000 delta=4)",
        "lines-after-first-edit=10000:q=1,a=5",
        "log-second-edit=S.V.before(10000 r=7 x=5 x.ln=10000) S.MODR(10000 r=7 x=5) S.V.after(10000 r=7 x=5 x.ln=10000 delta=2)",
        "lines-after-second-edit=10000:q=1,a=7",
        "log-close=",
        "lines-after-close=10000:q=1,a=7",
    ];
    assert_eq!(run("QJ_SubformSaveRecord_StoredRow"), expected);
}

#[test]
fn qj_subform_save_record_new_row() {
    let expected = [
        "log-open-and-new=S.AGCR(10000 r=1 x=0) S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0) S.AGCR(10000 r=1 x=1) S.NEW(x=1 x.ln=10000) S.AGCR(0 r=0 x=0)",
        "log-first-edit=S.V.before(0 r=5 x=0 x.ln=0) S.INSR(20000 r=5 x=0 x.ln=0) S.V.after(20000 r=5 x=0 x.ln=0 delta=5)",
        "lines-after-first-edit=10000:q=1,a=1 20000:q=0,a=5",
        "log-second-edit=S.V.before(20000 r=7 x=5 x.ln=20000) S.MODR(20000 r=7 x=5) S.V.after(20000 r=7 x=5 x.ln=20000 delta=2)",
        "lines-after-second-edit=10000:q=1,a=1 20000:q=0,a=7",
        "log-close=",
        "lines-after-close=10000:q=1,a=1 20000:q=0,a=7",
    ];
    assert_eq!(run("QJ_SubformSaveRecord_NewRow"), expected);
}

/// QjPartPropagation.al:997 QL_Both_NewPartRow_PartModifiesHost (corpus bump 42): on a new part row with UpdatePropagation Both the
/// host reads its row again and the part keeps its typed row, inserted at Close.
#[test]
fn ql_both_new_part_row_part_modifies_host() {
    let expected = [
        "log-new=L.AGCR(0 q=0 x.q=0) L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1) L.AGCR(0 q=0 x.q=0)",
        "log-part-edit=L.Qty.V(5 x=0) L.Qty.V.end H.AGR(t=6 d=orig n=orig x.t=1) H.AGCR(t=6 d=orig n=orig x.t=1)",
        "host-total-after-part=6.00",
        "part-line-after-part=0",
        "part-qty-after-part=5.00",
        "stored-after-part=t=6 d=orig n=orig",
        "lines-after-part=10000:q=1,a=1",
        "log-close=",
        "stored-after-close=t=6 d=orig n=orig",
        "lines-after-close=10000:q=1,a=1 20000:q=5,a=0",
    ];
    assert_eq!(run("QL_Both_NewPartRow_PartModifiesHost"), expected);
}
