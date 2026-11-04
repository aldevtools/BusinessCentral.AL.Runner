//! corpus pages/parts/PartLineBeforeHostKey.al (codeunit 67582, writer QH): a new, unsaved Document host
//! whose field-linked part shows the rows of the blank key. The first read of a part field or the part's First inserts the host
//! (page OnInsertRecord, table OnInsert) and the part reads its rows again; Close, OK, the host's New() and a host field read
//! insert nothing. The fixture objects are the corpus file's.

use al_testkit::text;

const SRC: &str = r#"
table 67573 "LT DL Header"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20])
        {
            DataClassification = SystemMetadata;

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('H.OnValidate(No.):' + "No.");
            end;
        }
        field(2; Name; Text[30])
        {
            DataClassification = SystemMetadata;

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('H.OnValidate(Name):' + Name + ' key=' + "No.");
            end;
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('H.OnInsert:' + "No.");
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('H.OnModify:' + "No.");
    end;
}

table 67576 "LT DL Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Document No."; Code[20])
        {
            DataClassification = SystemMetadata;
            TableRelation = "LT DL Header";

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('L.OnValidate(Document No.):' + "Document No.");
            end;
        }
        field(2; "Line No."; Integer)
        {
            DataClassification = SystemMetadata;
        }
        field(3; Description; Text[30])
        {
            DataClassification = SystemMetadata;

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('L.OnValidate(Description):' + Description + ' key=' + "Document No." + '/' + Format("Line No."));
            end;
        }
        field(4; Quantity; Decimal)
        {
            DataClassification = SystemMetadata;

            trigger OnValidate()
            var
                Log: Codeunit "LT DL Log";
            begin
                Log.Add('L.OnValidate(Quantity):' + Format(Quantity) + ' key=' + "Document No." + '/' + Format("Line No."));
            end;
        }
    }

    keys
    {
        key(PK; "Document No.", "Line No.") { Clustered = true; }
    }

    trigger OnInsert()
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('L.OnInsert:' + "Document No." + '/' + Format("Line No."));
    end;

    trigger OnModify()
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('L.OnModify:' + "Document No." + '/' + Format("Line No."));
    end;
}

codeunit 67579 "LT DL Log"
{
    SingleInstance = true;

    var
        Lines: List of [Text];

    procedure Reset()
    begin
        Clear(Lines);
    end;

    procedure Add(Line: Text)
    begin
        Lines.Add(Line);
    end;

    procedure Dump(): Text
    var
        Line: Text;
        Result: Text;
    begin
        foreach Line in Lines do begin
            if Result <> '' then
                Result += ' | ';
            Result += Line;
        end;
        exit('[' + Result + ']');
    end;
}

page 67578 "LT DL Lines"
{
    PageType = ListPart;
    SourceTable = "LT DL Line";
    AutoSplitKey = true;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(Description; Rec.Description) { ApplicationArea = All; }
                field(Quantity; Rec.Quantity) { ApplicationArea = All; }
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('LP.OnNewRecord:' + Rec."Document No.");
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('LP.OnInsertRecord:' + Rec."Document No." + '/' + Format(Rec."Line No."));
        exit(true);
    end;
}

page 67577 "LT DL Document"
{
    PageType = Document;
    SourceTable = "LT DL Header";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field("No."; Rec."No.") { ApplicationArea = All; }
                field(Name; Rec.Name) { ApplicationArea = All; }
            }
            part(Lines; "LT DL Lines")
            {
                ApplicationArea = All;
                SubPageLink = "Document No." = field("No.");
            }
        }
    }

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('HP.OnNewRecord');
    end;

    trigger OnInsertRecord(BelowxRec: Boolean): Boolean
    var
        Log: Codeunit "LT DL Log";
    begin
        Log.Add('HP.OnInsertRecord:' + Rec."No.");
        exit(true);
    end;
}
codeunit 67582 "QH Test"
{
    local procedure Seed(Lines: Integer)
    var
        H: Record "LT DL Header";
        L: Record "LT DL Line";
        Log: Codeunit "LT DL Log";
        i: Integer;
    begin
        H.DeleteAll();
        L.DeleteAll();
        for i := 1 to Lines do begin
            L.Init();
            L."Document No." := '';
            L."Line No." := i * 10000;
            L.Description := 'seeded';
            L.Insert();
        end;
        Log.Reset();
    end;

    local procedure Stored(): Text
    var
        H: Record "LT DL Header";
        L: Record "LT DL Line";
    begin
        exit('headers=' + Format(H.Count()) + ' lines=' + Format(L.Count()));
    end;

    local procedure Take(): Text
    var
        Log: Codeunit "LT DL Log";
        T: Text;
    begin
        T := Log.Dump();
        Log.Reset();
        exit(T);
    end;

    procedure Run(Lines: Integer; Step: Integer): Text
    var
        Host: TestPage "LT DL Document";
        T: Text;
    begin
        Seed(Lines);
        Host.OpenNew();
        T := 'open=' + Take();
        case Step of
            1: T += '#read=' + Host.Lines.Description.Value();
            2: T += '#first=' + Format(Host.Lines.First());
            3: T += '#host-read=' + Host.Name.Value();
            4: Host.New();
            5: Host.OK().Invoke();
        end;
        T += '#log=' + Take();
        if Step <> 5 then
            Host.Close();
        T += '#close=' + Take() + '#' + Stored();
        exit(T);
    end;

    procedure OpenViewNoRow(Lines: Integer): Text
    var
        Host: TestPage "LT DL Document";
        T: Text;
    begin
        Seed(Lines);
        Host.OpenView();
        T := 'open=' + Take();
        T += '#no=' + Host."No.".Value();
        T += '#read=' + Host.Lines.Description.Value();
        T += '#log=' + Take();
        Host.Close();
        T += '#close=' + Take() + '#' + Stored();
        exit(T);
    end;

    procedure ViewNoRow(): Text begin exit(OpenViewNoRow(0)); end;
    procedure ViewNoRowBlankLines(): Text begin exit(OpenViewNoRow(1)); end;
    procedure ReadStored(): Text begin exit(Run(2, 1)); end;
    procedure ReadNoRows(): Text begin exit(Run(0, 1)); end;
    procedure CloseOnly(): Text begin exit(Run(2, 0)); end;
    procedure PartFirst(): Text begin exit(Run(1, 2)); end;
    procedure HostRead(): Text begin exit(Run(1, 3)); end;
    procedure HostNew(): Text begin exit(Run(1, 4)); end;
    procedure OkInvoke(): Text begin exit(Run(1, 5)); end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "QH Test", routine))
}

/// PartLineBeforeHostKey.al:1967 QH_BlankRows_ReadPart_Stored.
#[test]
fn a_part_field_read_inserts_the_new_host() {
    assert_eq!(
        run("ReadStored"),
        "open=[HP.OnNewRecord]#read=seeded#log=[HP.OnInsertRecord: | H.OnInsert:]#close=[]#headers=1 lines=2"
    );
}

/// PartLineBeforeHostKey.al:2251 QH_NoRows_ReadPartControl: the empty part reads again as at open.
#[test]
fn a_part_field_read_of_an_empty_part_inserts_the_new_host() {
    assert_eq!(
        run("ReadNoRows"),
        "open=[HP.OnNewRecord | L.OnValidate(Document No.): | LP.OnNewRecord: | LP.OnNewRecord: | L.OnValidate(Document No.): | LP.OnNewRecord:]#read=#log=[HP.OnInsertRecord: | H.OnInsert: | L.OnValidate(Document No.): | LP.OnNewRecord: | LP.OnNewRecord: | L.OnValidate(Document No.): | LP.OnNewRecord:]#close=[]#headers=1 lines=0"
    );
}

/// PartLineBeforeHostKey.al:1944 QH_BlankRows_OpenNew_CloseOnly.
#[test]
fn close_inserts_no_new_host() {
    assert_eq!(
        run("CloseOnly"),
        "open=[HP.OnNewRecord]#log=[]#close=[]#headers=0 lines=2"
    );
}

/// PartLineBeforeHostKey.al:2302 QH_BlankRows_PartFirst.
#[test]
fn a_part_first_inserts_the_new_host() {
    assert_eq!(
        run("PartFirst"),
        "open=[HP.OnNewRecord]#first=Yes#log=[HP.OnInsertRecord: | H.OnInsert:]#close=[]#headers=1 lines=1"
    );
}

/// PartLineBeforeHostKey.al:2335 QH_BlankRows_ReadHostControl.
#[test]
fn a_host_field_read_inserts_nothing() {
    assert_eq!(
        run("HostRead"),
        "open=[HP.OnNewRecord]#host-read=#log=[]#close=[]#headers=0 lines=1"
    );
}

/// PartLineBeforeHostKey.al:2128 QH_BlankRows_HostNew_ThenClose.
#[test]
fn a_host_new_inserts_nothing() {
    assert_eq!(
        run("HostNew"),
        "open=[HP.OnNewRecord]#log=[HP.OnNewRecord]#close=[]#headers=0 lines=1"
    );
}

/// PartLineBeforeHostKey.al:2158 QH_BlankRows_OkInvoke.
#[test]
fn ok_inserts_no_new_host() {
    assert_eq!(
        run("OkInvoke"),
        "open=[HP.OnNewRecord]#log=[]#close=[]#headers=0 lines=1"
    );
}

/// PartLineBeforeHostKey.al:1722 QH_OpenView_NoRow_Document: the part runs OnNewRecord once, no validation, nothing is inserted.
#[test]
fn open_view_on_no_host_row_runs_the_parts_on_new_record_once() {
    assert_eq!(
        run("ViewNoRow"),
        "open=[LP.OnNewRecord:]#no=#read=#log=[]#close=[]#headers=0 lines=0"
    );
}

/// PartLineBeforeHostKey.al:1797 QH_OpenView_NoRow_BlankKeyLinesExist: the part shows no row although lines of the blank key exist.
#[test]
fn open_view_on_no_host_row_shows_no_part_row() {
    assert_eq!(
        run("ViewNoRowBlankLines"),
        "open=[LP.OnNewRecord:]#no=#read=#log=[]#close=[]#headers=0 lines=1"
    );
}
