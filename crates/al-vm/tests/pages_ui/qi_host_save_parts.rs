//! corpus testpage/parts/QiHostSaveParts.al (codeunit 86836): a host card with a linked ListPart and an unlinked ListPart.
//! A host SetValue whose OnValidate calls CurrPage.SaveRecord() re-reads both parts; Rec.Modify() or neither re-reads no part;
//! in OnAfterGetRecord xRec is the row the page fetched before it. The fixture objects are the corpus file's, unchanged.

use al_testkit::text;

const SRC: &str = r#"
table 86830 "LT QI SP Head"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; SaveFld; Text[30]) { }
        field(3; ModFld; Text[30]) { }
        field(4; PlainFld; Text[30]) { }
        field(5; CountFld; Text[30]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnModify()
    var
        Log: Codeunit "LT QI SP Log";
    begin
        Log.Add('T.HeadModify(' + Rec."No." + ')');
    end;
}

table 86831 "LT QI SP Line"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Doc No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Descr; Text[30]) { }
    }

    keys
    {
        key(PK; "Doc No.", "Line No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT QI SP Log";
    begin
        Log.Add('T.LineInsert(' + Rec."Doc No." + '/' + Format(Rec."Line No.") + '|' + Rec.Descr + ')');
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT QI SP Log";
    begin
        Log.Add('T.LineModify(' + Rec."Doc No." + '/' + Format(Rec."Line No.") + '|' + Rec.Descr + ')');
    end;
}

codeunit 86832 "LT QI SP Log"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Reset()
    begin
        Entries := '';
    end;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Entries;
        Entries := '';
        exit(Result);
    end;
}

page 86833 "LT QI SP Linked Part"
{
    PageType = ListPart;
    SourceTable = "LT QI SP Line";
    ApplicationArea = All;
    UsageCategory = None;
    AutoSplitKey = true;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; }
                field(DescrCtl; Rec.Descr) { ApplicationArea = All; }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('L.AGR(' + Rec."Doc No." + '/' + Format(Rec."Line No.") + '|' + Rec.Descr + ' x=' + xRec."Doc No." + '/' + Format(xRec."Line No.") + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('L.AGCR(' + Rec."Doc No." + '/' + Format(Rec."Line No.") + '|' + Rec.Descr + ' x=' + xRec."Doc No." + '/' + Format(xRec."Line No.") + ')');
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('L.NEW(' + Rec."Doc No." + ')');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    begin
        Log.Add('L.INS(' + Rec."Doc No." + '/' + Format(Rec."Line No.") + '|' + Rec.Descr + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('L.MOD(' + Rec."Doc No." + '/' + Format(Rec."Line No.") + '|' + Rec.Descr + ' x=' + xRec.Descr + ')');
    end;

    var
        Log: Codeunit "LT QI SP Log";
}

page 86834 "LT QI SP Free Part"
{
    PageType = ListPart;
    SourceTable = "LT QI SP Line";
    ApplicationArea = All;
    UsageCategory = None;
    Editable = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(DescrCtl; Rec.Descr) { ApplicationArea = All; }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('U.AGR(' + Rec."Doc No." + '/' + Format(Rec."Line No.") + ' x=' + xRec."Doc No." + '/' + Format(xRec."Line No.") + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('U.AGCR(' + Rec."Doc No." + '/' + Format(Rec."Line No.") + ' x=' + xRec."Doc No." + '/' + Format(xRec."Line No.") + ')');
    end;

    var
        Log: Codeunit "LT QI SP Log";
}

page 86835 "LT QI SP Doc"
{
    PageType = Document;
    SourceTable = "LT QI SP Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(SaveCtl; Rec.SaveFld)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('H.V(save ' + Rec.SaveFld + ' x=' + xRec.SaveFld + ')');
                        CurrPage.SaveRecord();
                        Log.Add('H.V(save done)');
                    end;
                }
                field(ModCtl; Rec.ModFld)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('H.V(mod ' + Rec.ModFld + ' x=' + xRec.ModFld + ')');
                        Rec.Modify();
                        Log.Add('H.V(mod done)');
                    end;
                }
                field(PlainCtl; Rec.PlainFld)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('H.V(plain ' + Rec.PlainFld + ' x=' + xRec.PlainFld + ')');
                    end;
                }
                field(CountCtl; Rec.CountFld)
                {
                    ApplicationArea = All;

                    trigger OnValidate()
                    begin
                        Log.Add('H.V(count ' + LinesOf(Rec."No.") + ')');
                    end;
                }
            }
            part(Lines; "LT QI SP Linked Part")
            {
                ApplicationArea = All;
                SubPageLink = "Doc No." = field("No.");
            }
            part(Free; "LT QI SP Free Part")
            {
                ApplicationArea = All;
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        Log.Add('H.AGR(' + Rec."No." + ' x=' + xRec."No." + ')');
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('H.AGCR(' + Rec."No." + ' x=' + xRec."No." + ')');
    end;

    trigger OnModifyRecord(): Boolean
    begin
        Log.Add('H.MOD(' + Rec."No." + ')');
    end;

    local procedure LinesOf(DocNo: Code[20]): Text
    var
        Line: Record "LT QI SP Line";
        Result: Text;
    begin
        Line.SetRange("Doc No.", DocNo);
        Result := Format(Line.Count());
        if Line.FindSet() then
            repeat
                Result += ' ' + Format(Line."Line No.") + '=' + Line.Descr;
            until Line.Next() = 0;
        exit(Result);
    end;

    var
        Log: Codeunit "LT QI SP Log";
}

codeunit 86836 "LT QI SP Tests"
{
    local procedure Seed()
    var
        Head: Record "LT QI SP Head";
        Line: Record "LT QI SP Line";
        Log: Codeunit "LT QI SP Log";
    begin
        Line.DeleteAll();
        Head.DeleteAll();
        Head.Init();
        Head."No." := 'D1';
        Head.SaveFld := 'a';
        Head.ModFld := 'a';
        Head.PlainFld := 'a';
        Head.Insert();
        Head."No." := 'D2';
        Head.Insert();
        AddLine('D1', 10000, 'd1 l1');
        AddLine('D1', 20000, 'd1 l2');
        AddLine('D2', 10000, 'd2 l1');
        Log.Reset();
    end;

    local procedure AddLine(DocNo: Code[20]; LineNo: Integer; Descr: Text[30])
    var
        Line: Record "LT QI SP Line";
    begin
        Line.Init();
        Line."Doc No." := DocNo;
        Line."Line No." := LineNo;
        Line.Descr := Descr;
        Line.Insert();
    end;

    local procedure Stored(DocNo: Code[20]): Text
    var
        Line: Record "LT QI SP Line";
        Result: Text;
    begin
        Line.SetRange("Doc No.", DocNo);
        Result := Format(Line.Count());
        if Line.FindSet() then
            repeat
                Result += ' ' + Format(Line."Line No.") + '=' + Line.Descr;
            until Line.Next() = 0;
        exit(Result);
    end;

    local procedure HeadValues(): Text
    var
        Head: Record "LT QI SP Head";
    begin
        Head.Get('D1');
        exit(Head.SaveFld + '|' + Head.ModFld + '|' + Head.PlainFld + '|' + Head.CountFld);
    end;

    procedure HostSetValue(Which: Integer; NewValue: Text): Text
    var
        Log: Codeunit "LT QI SP Log";
        Doc: TestPage "LT QI SP Doc";
        T: Text;
    begin
        Seed();
        Doc.OpenEdit();
        T := 'open=' + Log.Take();
        case Which of
            1: Doc.SaveCtl.SetValue(NewValue);
            2: Doc.ModCtl.SetValue(NewValue);
            3: Doc.PlainCtl.SetValue(NewValue);
        end;
        T += '#setvalue=' + Log.Take();
        T += '#stored=' + HeadValues();
        T += '#part-descr=' + Doc.Lines.DescrCtl.Value();
        T += '#after-part-read=' + Log.Take();
        Doc.Close();
        T += '#close=' + Log.Take();
        T += '#stored-after-close=' + HeadValues();
        exit(T);
    end;

    procedure PartMoved(): Text
    var
        Log: Codeunit "LT QI SP Log";
        Doc: TestPage "LT QI SP Doc";
        T: Text;
    begin
        Seed();
        Doc.OpenEdit();
        T := 'part-next=' + Format(Doc.Lines.Next());
        T += '#part-row=' + Doc.Lines.LineNoCtl.Value();
        T += '#open-and-next=' + Log.Take();
        Doc.SaveCtl.SetValue('b');
        T += '#setvalue=' + Log.Take();
        T += '#part-row-after=' + Doc.Lines.LineNoCtl.Value();
        T += '#after-part-read=' + Log.Take();
        Doc.Close();
        T += '#close=' + Log.Take();
        exit(T);
    end;

    procedure PartRow(Kind: Integer): Text
    var
        Log: Codeunit "LT QI SP Log";
        Doc: TestPage "LT QI SP Doc";
        T: Text;
    begin
        Seed();
        Doc.OpenEdit();
        case Kind of
            1, 4:
                begin
                    Doc.Lines.Last();
                    Doc.Lines.New();
                    Doc.Lines.DescrCtl.SetValue('typed');
                end;
            2:
                begin
                    Doc.Lines.Last();
                    Doc.Lines.New();
                end;
            3:
                begin
                    Doc.Lines.First();
                    Doc.Lines.DescrCtl.SetValue('edited');
                end;
        end;
        T := 'stored-before=' + Stored('D1');
        Log.Take();
        if Kind = 4 then
            Doc.SaveCtl.SetValue('b')
        else
            Doc.CountCtl.SetValue('x');
        T += '#setvalue=' + Log.Take();
        T += '#stored-after=' + Stored('D1');
        if Kind = 1 then begin
            T += '#part-descr=' + Doc.Lines.DescrCtl.Value();
            T += '#after-part-read=' + Log.Take();
        end;
        Doc.Close();
        T += '#close=' + Log.Take();
        T += '#stored-after-close=' + Stored('D1');
        exit(T);
    end;
    procedure SaveChanged(): Text begin exit(HostSetValue(1, 'b')); end;
    procedure SaveSame(): Text begin exit(HostSetValue(1, 'a')); end;
    procedure ModChanged(): Text begin exit(HostSetValue(2, 'b')); end;
    procedure PlainChanged(): Text begin exit(HostSetValue(3, 'b')); end;
    procedure PartNew(): Text begin exit(PartRow(1)); end;
    procedure PartUntouched(): Text begin exit(PartRow(2)); end;
    procedure PartEdited(): Text begin exit(PartRow(3)); end;
    procedure PartNewSave(): Text begin exit(PartRow(4)); end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "LT QI SP Tests", routine))
}

fn obs(s: &str, name: &str) -> String {
    s.split('#')
        .find_map(|p| p.strip_prefix(&format!("{name}=")))
        .unwrap_or("<missing>")
        .to_string()
}

/// testpage/parts/QiHostSaveParts.al QI_HostSaveRecord_ChangedValue (:361-367).
#[test]
fn qi_hostsaverecord_changedvalue() {
    let s = run("SaveChanged");
    assert_eq!(
        obs(&s, "open"),
        "H.AGR(D1 x=);H.AGCR(D1 x=);L.AGR(D1/10000|d1 l1 x=/0);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGCR(D1/10000|d1 l1 x=D1/0);U.AGR(D1/10000 x=/0);U.AGR(D1/20000 x=D1/10000);U.AGR(D2/10000 x=D1/20000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=/0);",
        "open"
    );
    assert_eq!(
        obs(&s, "setvalue"),
        "H.V(save b x=a);H.MOD(D1);T.HeadModify(D1);H.V(save done);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGCR(D1/10000|d1 l1 x=D1/10000);U.AGR(D1/10000 x=D1/10000);U.AGR(D1/20000 x=D1/10000);U.AGR(D2/10000 x=D1/20000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=D1/10000);",
        "setvalue"
    );
    assert_eq!(obs(&s, "stored"), "b|a|a|", "stored");
    assert_eq!(obs(&s, "part-descr"), "d1 l1", "part-descr");
    assert_eq!(obs(&s, "after-part-read"), "", "after-part-read");
    assert_eq!(obs(&s, "close"), "", "close");
    assert_eq!(obs(&s, "stored-after-close"), "b|a|a|", "stored-after-close");
}

/// testpage/parts/QiHostSaveParts.al QI_HostSaveRecord_SameValue (:385-387).
#[test]
fn qi_hostsaverecord_samevalue() {
    let s = run("SaveSame");
    assert_eq!(
        obs(&s, "open"),
        "H.AGR(D1 x=);H.AGCR(D1 x=);L.AGR(D1/10000|d1 l1 x=/0);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGCR(D1/10000|d1 l1 x=D1/0);U.AGR(D1/10000 x=/0);U.AGR(D1/20000 x=D1/10000);U.AGR(D2/10000 x=D1/20000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=/0);",
        "open"
    );
    assert_eq!(
        obs(&s, "setvalue"),
        "H.V(save a x=a);H.V(save done);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGCR(D1/10000|d1 l1 x=D1/10000);U.AGR(D1/10000 x=D1/10000);U.AGR(D1/20000 x=D1/10000);U.AGR(D2/10000 x=D1/20000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=D1/10000);",
        "setvalue"
    );
    assert_eq!(obs(&s, "close"), "", "close");
}

/// testpage/parts/QiHostSaveParts.al QI_HostModify_ChangedValue (:437-443).
#[test]
fn qi_hostmodify_changedvalue() {
    let s = run("ModChanged");
    assert_eq!(
        obs(&s, "open"),
        "H.AGR(D1 x=);H.AGCR(D1 x=);L.AGR(D1/10000|d1 l1 x=/0);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGCR(D1/10000|d1 l1 x=D1/0);U.AGR(D1/10000 x=/0);U.AGR(D1/20000 x=D1/10000);U.AGR(D2/10000 x=D1/20000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=/0);",
        "open"
    );
    assert_eq!(obs(&s, "setvalue"), "H.V(mod b x=a);H.V(mod done);", "setvalue");
    assert_eq!(obs(&s, "stored"), "a|b|a|", "stored");
    assert_eq!(obs(&s, "part-descr"), "d1 l1", "part-descr");
    assert_eq!(
        obs(&s, "after-part-read"),
        "H.MOD(D1);T.HeadModify(D1);",
        "after-part-read"
    );
    assert_eq!(obs(&s, "close"), "", "close");
    assert_eq!(obs(&s, "stored-after-close"), "a|b|a|", "stored-after-close");
}

/// testpage/parts/QiHostSaveParts.al QI_HostPlain_ChangedValue (:465-471).
#[test]
fn qi_hostplain_changedvalue() {
    let s = run("PlainChanged");
    assert_eq!(
        obs(&s, "open"),
        "H.AGR(D1 x=);H.AGCR(D1 x=);L.AGR(D1/10000|d1 l1 x=/0);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGCR(D1/10000|d1 l1 x=D1/0);U.AGR(D1/10000 x=/0);U.AGR(D1/20000 x=D1/10000);U.AGR(D2/10000 x=D1/20000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=/0);",
        "open"
    );
    assert_eq!(obs(&s, "setvalue"), "H.V(plain b x=a);", "setvalue");
    assert_eq!(obs(&s, "stored"), "a|a|a|", "stored");
    assert_eq!(obs(&s, "part-descr"), "d1 l1", "part-descr");
    assert_eq!(
        obs(&s, "after-part-read"),
        "H.MOD(D1);T.HeadModify(D1);",
        "after-part-read"
    );
    assert_eq!(obs(&s, "close"), "", "close");
    assert_eq!(obs(&s, "stored-after-close"), "a|a|b|", "stored-after-close");
}

/// testpage/parts/QiHostSaveParts.al QI_PartNewRow_ThenHostSetValue (:499-505).
#[test]
fn qi_partnewrow_thenhostsetvalue() {
    let s = run("PartNew");
    assert_eq!(obs(&s, "stored-before"), "2 10000=d1 l1 20000=d1 l2", "stored-before");
    assert_eq!(
        obs(&s, "setvalue"),
        "L.INS(D1/30000|typed);T.LineInsert(D1/30000|typed);U.AGR(D1/30000 x=D1/10000);U.AGR(D1/30000 x=D1/30000);U.AGCR(D1/30000 x=/0);H.V(count 3 10000=d1 l1 20000=d1 l2 30000=typed);",
        "setvalue"
    );
    assert_eq!(
        obs(&s, "stored-after"),
        "3 10000=d1 l1 20000=d1 l2 30000=typed",
        "stored-after"
    );
    assert_eq!(obs(&s, "part-descr"), "typed", "part-descr");
    assert_eq!(obs(&s, "after-part-read"), "", "after-part-read");
    assert_eq!(
        obs(&s, "close"),
        "H.MOD(D1);T.HeadModify(D1);U.AGR(D1/10000 x=D1/30000);",
        "close"
    );
    assert_eq!(
        obs(&s, "stored-after-close"),
        "3 10000=d1 l1 20000=d1 l2 30000=typed",
        "stored-after-close"
    );
}

/// testpage/parts/QiHostSaveParts.al QI_PartNewRowUntouched_ThenHostSetValue (:527-530).
#[test]
fn qi_partnewrowuntouched_thenhostsetvalue() {
    let s = run("PartUntouched");
    assert_eq!(obs(&s, "setvalue"), "H.V(count 2 10000=d1 l1 20000=d1 l2);", "setvalue");
    assert_eq!(obs(&s, "stored-after"), "2 10000=d1 l1 20000=d1 l2", "stored-after");
    assert_eq!(obs(&s, "close"), "H.MOD(D1);T.HeadModify(D1);", "close");
    assert_eq!(
        obs(&s, "stored-after-close"),
        "2 10000=d1 l1 20000=d1 l2",
        "stored-after-close"
    );
}

/// testpage/parts/QiHostSaveParts.al QI_PartEditedRow_ThenHostSetValue (:553-557).
#[test]
fn qi_parteditedrow_thenhostsetvalue() {
    let s = run("PartEdited");
    assert_eq!(obs(&s, "stored-before"), "2 10000=d1 l1 20000=d1 l2", "stored-before");
    assert_eq!(
        obs(&s, "setvalue"),
        "L.MOD(D1/10000|edited x=d1 l1);T.LineModify(D1/10000|edited);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=/0);H.V(count 2 10000=edited 20000=d1 l2);",
        "setvalue"
    );
    assert_eq!(obs(&s, "stored-after"), "2 10000=edited 20000=d1 l2", "stored-after");
    assert_eq!(obs(&s, "close"), "H.MOD(D1);T.HeadModify(D1);", "close");
    assert_eq!(
        obs(&s, "stored-after-close"),
        "2 10000=edited 20000=d1 l2",
        "stored-after-close"
    );
}

/// testpage/parts/QiHostSaveParts.al QI_PartNewRow_ThenHostSaveRecord (:580-583).
#[test]
fn qi_partnewrow_thenhostsaverecord() {
    let s = run("PartNewSave");
    assert_eq!(
        obs(&s, "setvalue"),
        "L.INS(D1/30000|typed);T.LineInsert(D1/30000|typed);U.AGR(D1/30000 x=D1/10000);U.AGR(D1/30000 x=D1/30000);U.AGCR(D1/30000 x=/0);H.V(save b x=a);H.MOD(D1);T.HeadModify(D1);H.V(save done);L.AGR(D1/30000|typed x=D1/20000);L.AGR(D1/20000|d1 l2 x=D1/30000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/30000|typed x=D1/10000);L.AGR(D1/30000|typed x=D1/30000);L.AGCR(D1/30000|typed x=D1/30000);U.AGR(D1/10000 x=D1/30000);U.AGR(D1/10000 x=D1/10000);U.AGR(D1/20000 x=D1/10000);U.AGR(D1/30000 x=D1/20000);U.AGR(D2/10000 x=D1/30000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=D1/10000);",
        "setvalue"
    );
    assert_eq!(
        obs(&s, "stored-after"),
        "3 10000=d1 l1 20000=d1 l2 30000=typed",
        "stored-after"
    );
    assert_eq!(obs(&s, "close"), "", "close");
    assert_eq!(
        obs(&s, "stored-after-close"),
        "3 10000=d1 l1 20000=d1 l2 30000=typed",
        "stored-after-close"
    );
}

/// testpage/parts/QiHostSaveParts.al QI_HostSaveRecord_PartMovedToSecondRow (:409-415).
#[test]
fn qi_hostsaverecord_partmovedtosecondrow() {
    let s = run("PartMoved");
    assert_eq!(obs(&s, "part-next"), "Yes", "part-next");
    assert_eq!(obs(&s, "part-row"), "20000", "part-row");
    assert_eq!(
        obs(&s, "open-and-next"),
        "H.AGR(D1 x=);H.AGCR(D1 x=);L.AGR(D1/10000|d1 l1 x=/0);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/10000|d1 l1 x=D1/10000);L.AGCR(D1/10000|d1 l1 x=D1/0);U.AGR(D1/10000 x=/0);U.AGR(D1/20000 x=D1/10000);U.AGR(D2/10000 x=D1/20000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=/0);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/20000|d1 l2 x=D1/20000);L.AGCR(D1/20000|d1 l2 x=D1/10000);",
        "open-and-next"
    );
    assert_eq!(
        obs(&s, "setvalue"),
        "H.V(save b x=a);H.MOD(D1);T.HeadModify(D1);H.V(save done);L.AGR(D1/20000|d1 l2 x=D1/20000);L.AGR(D1/10000|d1 l1 x=D1/20000);L.AGR(D1/20000|d1 l2 x=D1/10000);L.AGR(D1/20000|d1 l2 x=D1/20000);L.AGCR(D1/20000|d1 l2 x=D1/20000);U.AGR(D1/10000 x=D1/10000);U.AGR(D1/20000 x=D1/10000);U.AGR(D2/10000 x=D1/20000);U.AGR(D1/10000 x=D2/10000);U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=D1/10000);",
        "setvalue"
    );
    assert_eq!(obs(&s, "part-row-after"), "20000", "part-row-after");
    assert_eq!(obs(&s, "after-part-read"), "", "after-part-read");
    assert_eq!(obs(&s, "close"), "", "close");
}
