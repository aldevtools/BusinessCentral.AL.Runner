//! A second Export on the same xmlport instance, and the state a table element record keeps after an export under a
//! SourceTableView (assumptions `xmlport-export-again` and `xmlport-view-filter-unlinked`; corpus branch
//! al-runtime-rs-staging-pub-ts, xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Again_* and TS_View_*,
//! xmlports 90081 and 90088 in TsXpObjects.al).

use al_testkit::text;

const SRC: &str = r#"
table 80 "EA Result"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Ref; Integer) { }
        field(3; Token; Text[30]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 81 "EA Part"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Ref; Integer) { }
        field(3; "Code"; Code[20]) { }
        field(4; Kind; Integer) { }
        field(5; Qty; Integer) { }
    }
    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
        key(ByKind; Kind) { }
    }
}

table 82 "EA Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 83 "EA Again"
{
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(bookings)
        {
            MaxOccurs = Once;
            tableelement(result; "EA Result")
            {
                MaxOccurs = Once;
                MinOccurs = Zero;
                UseTemporary = true;
                fieldelement(token; result.Token) { MinOccurs = Zero; MaxOccurs = Once; }
                textelement(details)
                {
                    MaxOccurs = Once;
                    MinOccurs = Zero;
                    tableelement(line; "EA Result")
                    {
                        MinOccurs = Zero;
                        UseTemporary = true;
                        fieldattribute(no; line.Ref) { }
                        textelement(parts)
                        {
                            MaxOccurs = Once;
                            MinOccurs = Zero;
                            tableelement(part; "EA Part")
                            {
                                UseTemporary = true;
                                MinOccurs = Once;
                                MaxOccurs = Unbounded;
                                SourceTableView = sorting(Kind);
                                fieldattribute(no; part."Entry No.") { }
                                fieldattribute(code; part.Code) { }
                                textattribute(sid)
                                {
                                    trigger OnBeforePassVariable()
                                    begin
                                        sid := Sid(part);
                                    end;
                                }
                                trigger OnAfterGetRecord()
                                begin
                                    Log += 'get part ' + part.Code + ';';
                                end;
                            }
                        }
                        trigger OnAfterGetRecord()
                        begin
                            Log += 'get line ' + Format(line.Ref) + ';';
                            part.DeleteAll();
                            Source.SetRange(Ref, line.Ref);
                            if Source.FindSet() then
                                repeat
                                    part := Source;
                                    part.Insert();
                                until Source.Next() = 0;
                            Source.Reset();
                        end;
                    }
                }
            }
        }
    }
    var
        Source: Record "EA Part" temporary;
        Log: Text;

    local procedure Sid(var Row: Record "EA Part" temporary): Text
    begin
        if IsNullGuid(Row.SystemId) then
            exit('null');
        exit(CopyStr(Format(Row.SystemId), 36, 2));
    end;

    procedure Setup()
    begin
        result.Init();
        result."Entry No." := 1;
        result.Token := 'T9';
        result.Insert();
        AddLine(10, 1);
        AddLine(20, 2);
        AddLine(30, 3);
        AddPart(1, 2, 'ZOO', 2);
        AddPart(2, 2, 'SHOW', 1);
        AddPart(9, 9, 'SPARE', 3);
    end;

    local procedure AddLine(No: Integer; LineRef: Integer)
    begin
        line.Init();
        line."Entry No." := No;
        line.Ref := LineRef;
        line.Insert();
    end;

    local procedure AddPart(No: Integer; LineRef: Integer; NewCode: Code[20]; NewKind: Integer)
    begin
        Source.Init();
        Source."Entry No." := No;
        Source.Ref := LineRef;
        Source.Code := NewCode;
        Source.Kind := NewKind;
        Evaluate(Source.SystemId, '{00000000-0000-0000-0000-0000000000' + Format(No, 0, '<Integer,2><Filler Character,0>') + '}');
        Source.Insert(false, true);
    end;

    procedure AssignPart(No: Integer)
    begin
        Source.Get(No);
        part := Source;
    end;

    procedure TransferPart(No: Integer)
    begin
        Source.Get(No);
        part.TransferFields(Source, true);
    end;

    procedure InitPart()
    begin
        part.Init();
    end;

    procedure SetPartKey()
    begin
        part.SetCurrentKey("Entry No.");
    end;

    procedure SetLineFilter(LineRef: Integer)
    begin
        line.SetRange(Ref, LineRef);
    end;

    procedure State(): Text
    begin
        exit('line=' + Format(line.Ref) + ' part=' + Format(part."Entry No.") + ':' + part.Code + '/' + Sid(part) +
            ' parts=' + Format(part.Count()) + ' partview=' + part.GetView(false) + ' lineview=' + line.GetView(false));
    end;

    procedure TakeLog(): Text
    var
        L: Text;
    begin
        L := Log;
        Log := '';
        exit(L);
    end;
}

xmlport 84 "EA View"
{
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(rows)
        {
            MaxOccurs = Once;
            tableelement(row; "EA Part")
            {
                MinOccurs = Zero;
                UseTemporary = true;
                SourceTableView = sorting(Kind) where(Qty = filter(> 0));
                fieldattribute(no; row."Entry No.") { }
                fieldattribute(code; row.Code) { }
            }
        }
    }

    procedure Setup()
    begin
        AddRow(1, 'A', 2, 1);
        AddRow(2, 'B', 1, 2);
        AddRow(3, 'C', 3, 3);
        AddRow(4, 'Z', 1, 0);
    end;

    local procedure AddRow(No: Integer; NewCode: Code[20]; NewKind: Integer; NewQty: Integer)
    begin
        row.Init();
        row."Entry No." := No;
        row.Code := NewCode;
        row.Kind := NewKind;
        row.Qty := NewQty;
        row.Insert();
    end;

    procedure SetKey()
    begin
        row.SetCurrentKey("Entry No.");
    end;

    procedure SetCodeFilter(F: Text)
    begin
        row.SetFilter(Code, F);
    end;

    procedure ResetRow()
    begin
        row.Reset();
    end;

    procedure View(): Text
    begin
        exit(row.GetView(false) + ' count=' + Format(row.Count()));
    end;
}

codeunit 85 C
{
    local procedure Rows(var B: Record "EA Blob" temporary): Text
    var
        InS: InStream;
        Line: Text;
        Xml: Text;
    begin
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        InS.ReadText(Line);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Xml += Line.Trim() + '|';
        end;
        exit(Xml);
    end;

    procedure Export(var P: XmlPort "EA Again"): Text
    var
        B: Record "EA Blob" temporary;
        OutS: OutStream;
    begin
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        P.SetDestination(OutS);
        P.Export();
        exit(Rows(B) + P.TakeLog());
    end;

    procedure ExportView(var P: XmlPort "EA View"): Text
    var
        B: Record "EA Blob" temporary;
        OutS: OutStream;
    begin
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        P.SetDestination(OutS);
        P.Export();
        exit(Rows(B));
    end;

    procedure SameData(): Text
    var
        P: XmlPort "EA Again";
        R: Text;
    begin
        P.Setup();
        R := Export(P) + ' # ' + P.State();
        exit(R + ' # ' + Export(P) + ' # ' + P.State());
    end;

    procedure AssignBetween(): Text
    var
        P: XmlPort "EA Again";
        R: Text;
    begin
        P.Setup();
        R := Export(P);
        P.AssignPart(9);
        exit(P.State() + ' # ' + Export(P));
    end;

    procedure TransferBetween(): Text
    var
        P: XmlPort "EA Again";
        R: Text;
    begin
        P.Setup();
        R := Export(P);
        P.TransferPart(9);
        exit(P.State() + ' # ' + Export(P));
    end;

    procedure InitBetween(): Text
    var
        P: XmlPort "EA Again";
        R: Text;
    begin
        P.Setup();
        R := Export(P);
        P.InitPart();
        exit(P.State() + ' # ' + Export(P));
    end;

    procedure KeyBetween(): Text
    var
        P: XmlPort "EA Again";
        R: Text;
    begin
        P.Setup();
        R := Export(P);
        P.SetPartKey();
        exit(P.State() + ' # ' + Export(P) + ' # ' + P.State());
    end;

    procedure LineFilterBetween(): Text
    var
        P: XmlPort "EA Again";
        R: Text;
    begin
        P.Setup();
        R := Export(P);
        P.SetLineFilter(2);
        exit(Export(P) + ' # ' + P.State());
    end;

    procedure ViewFirst(): Text
    var
        Plain: XmlPort "EA View";
        Filtered: XmlPort "EA View";
        R: Text;
    begin
        Plain.Setup();
        R := ExportView(Plain) + ' ' + Plain.View();
        Filtered.Setup();
        Filtered.SetKey();
        Filtered.SetCodeFilter('A|C|Z');
        exit(R + ' # ' + ExportView(Filtered) + ' ' + Filtered.View());
    end;

    procedure ViewSecond(): Text
    var
        KeySet: XmlPort "EA View";
        Filtered: XmlPort "EA View";
        ResetRow: XmlPort "EA View";
        R: Text;
    begin
        KeySet.Setup();
        R := ExportView(KeySet);
        KeySet.SetKey();
        R := ExportView(KeySet) + ' ' + KeySet.View();
        Filtered.Setup();
        R += ' # ' + ExportView(Filtered);
        Filtered.SetCodeFilter('A|C|Z');
        R += ' # ' + ExportView(Filtered) + ' ' + Filtered.View();
        ResetRow.Setup();
        R += ' # ' + ExportView(ResetRow);
        ResetRow.ResetRow();
        R += ' # ' + ResetRow.View();
        exit(R + ' # ' + ExportView(ResetRow) + ' ' + ResetRow.View());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

fn bookings(line1: &str, line2: &str, line3: &str) -> String {
    format!(
        "<bookings>|<result>|<token>T9</token>|<details>|<line no=\"1\">|<parts>|{line1}</parts>|</line>|<line no=\"2\">|<parts>|{line2}</parts>|</line>|<line no=\"3\">|<parts>|{line3}</parts>|</line>|</details>|</result>|</bookings>|"
    )
}

const ZOO: &str = "<part no=\"1\" code=\"ZOO\" sid=\"01\" />|";
const SHOW: &str = "<part no=\"2\" code=\"SHOW\" sid=\"02\" />|";
const LOG: &str = "get line 1;get line 2;get part SHOW;get part ZOO;get line 3;";

// TS_Again_SameData: each element reads its table again; the MinOccurs = Once part element without rows writes the record
// the first export left (the last row read, with its SystemId). The record keeps the view's key, the view's filters are not
// shown by GetView.
#[test]
fn a_second_export_writes_the_row_the_first_export_left() {
    let first = bookings(
        "<part no=\"0\" code=\"\" sid=\"null\" />|",
        &format!("{SHOW}{ZOO}"),
        ZOO,
    );
    let second = bookings(ZOO, &format!("{SHOW}{ZOO}"), ZOO);
    let state = "line=3 part=1:ZOO/01 parts=0 partview=VERSION(1) SORTING(Field4) lineview=VERSION(1) SORTING(Field1)";
    assert_eq!(
        run("SameData"),
        format!("{first}{LOG} # {state} # {second}{LOG} # {state}")
    );
}

// TS_Again_AssignBetween: `part := Source` between the exports copies the SystemId; the next export writes that row.
#[test]
fn an_assigned_row_is_written_by_the_next_export() {
    let state =
        "line=3 part=9:SPARE/09 parts=0 partview=VERSION(1) SORTING(Field4) lineview=VERSION(1) SORTING(Field1)";
    let second = bookings(
        "<part no=\"9\" code=\"SPARE\" sid=\"09\" />|",
        &format!("{SHOW}{ZOO}"),
        ZOO,
    );
    assert_eq!(run("AssignBetween"), format!("{state} # {second}{LOG}"));
}

// TS_Again_TransferFieldsBetween: TransferFields(From, true) copies the fields but not the SystemId.
#[test]
fn transfer_fields_between_exports_keeps_the_old_system_id() {
    let state =
        "line=3 part=9:SPARE/01 parts=0 partview=VERSION(1) SORTING(Field4) lineview=VERSION(1) SORTING(Field1)";
    let second = bookings(
        "<part no=\"9\" code=\"SPARE\" sid=\"01\" />|",
        &format!("{SHOW}{ZOO}"),
        ZOO,
    );
    assert_eq!(run("TransferBetween"), format!("{state} # {second}{LOG}"));
}

// TS_Again_InitBetween: Init keeps the primary key and nulls the SystemId; the next export still ends on the last row read.
#[test]
fn init_between_exports_keeps_the_primary_key_only() {
    let state = "line=3 part=1:/null parts=0 partview=VERSION(1) SORTING(Field4) lineview=VERSION(1) SORTING(Field1)";
    let second = bookings(
        "<part no=\"1\" code=\"\" sid=\"null\" />|",
        &format!("{SHOW}{ZOO}"),
        ZOO,
    );
    assert_eq!(run("InitBetween"), format!("{state} # {second}{LOG}"));
}

// TS_Again_KeyBetween: a later export does not set the SourceTableView's key again.
#[test]
fn a_key_set_between_exports_decides_the_order_of_the_next() {
    let set = "line=3 part=1:ZOO/01 parts=0 partview=VERSION(1) SORTING(Field1) lineview=VERSION(1) SORTING(Field1)";
    let after = "line=3 part=2:SHOW/02 parts=0 partview=VERSION(1) SORTING(Field1) lineview=VERSION(1) SORTING(Field1)";
    let second = bookings(ZOO, &format!("{ZOO}{SHOW}"), SHOW);
    let log = "get line 1;get line 2;get part ZOO;get part SHOW;get line 3;";
    assert_eq!(run("KeyBetween"), format!("{set} # {second}{log} # {after}"));
}

// TS_Again_LineFilter: a filter set on an element without SourceTableView between the exports is kept.
#[test]
fn a_filter_set_between_exports_is_kept() {
    let xml = format!(
        "<bookings>|<result>|<token>T9</token>|<details>|<line no=\"2\">|<parts>|{SHOW}{ZOO}</parts>|</line>|</details>|</result>|</bookings>|"
    );
    let state = "line=2 part=1:ZOO/01 parts=2 partview=VERSION(1) SORTING(Field4) lineview=VERSION(1) SORTING(Field1) WHERE(Field2=1(2))";
    assert_eq!(
        run("LineFilterBetween"),
        format!("{xml}get line 2;get part SHOW;get part ZOO; # {state}")
    );
}

const BAC: &str = "<rows>|<row no=\"2\" code=\"B\" />|<row no=\"1\" code=\"A\" />|<row no=\"3\" code=\"C\" />|</rows>|";

// TS_View_FirstExport: a SourceTableView filter on an element without LinkTable; its key replaces a key set before, filters set
// before combine with it, and GetView after the export shows the key and the code's filters only.
#[test]
fn a_source_table_view_filter_on_an_unlinked_element() {
    let ac = "<rows>|<row no=\"1\" code=\"A\" />|<row no=\"3\" code=\"C\" />|</rows>|";
    assert_eq!(
        run("ViewFirst"),
        format!(
            "{BAC} VERSION(1) SORTING(Field4) count=3 # {ac} VERSION(1) SORTING(Field4) WHERE(Field3=1(A|C|Z)) count=2"
        )
    );
}

// TS_View_SecondExport: a later export keeps the record's key (one set in between, the primary key after Reset) and applies the
// view's filters again.
#[test]
fn a_second_export_keeps_the_key_and_applies_the_filters_again() {
    let abc = "<rows>|<row no=\"1\" code=\"A\" />|<row no=\"2\" code=\"B\" />|<row no=\"3\" code=\"C\" />|</rows>|";
    let ac = "<rows>|<row no=\"1\" code=\"A\" />|<row no=\"3\" code=\"C\" />|</rows>|";
    assert_eq!(
        run("ViewSecond"),
        format!(
            "{abc} VERSION(1) SORTING(Field1) count=3 # {BAC} # {ac} VERSION(1) SORTING(Field4) WHERE(Field3=1(A|C|Z)) count=2 # {BAC} # VERSION(1) SORTING(Field1) count=4 # {abc} VERSION(1) SORTING(Field1) count=3"
        )
    );
}
