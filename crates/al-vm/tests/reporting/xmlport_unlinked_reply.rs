//! A reply port: a temporary table element (MaxOccurs = Once, MinOccurs = Zero) holding an optional text element over an
//! unlinked temporary table element (MinOccurs = Zero) with text attributes and a second optional text element over another
//! unlinked temporary table element (MinOccurs = Once, MaxOccurs = Unbounded, SourceTableView) that the outer row's
//! OnAfterGetRecord empties and refills (assumption `xmlport-unlinked-reply`; corpus branch al-runtime-rs-staging-tr,
//! xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TR_Booking_*, xmlport 90064 in TrXpReplyObjects.al).

use al_testkit::text;

const SRC: &str = r#"
table 70 "RP Result"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Ref; Integer) { }
        field(3; Token; Text[30]) { }
        field(4; Message; Text[50]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 71 "RP Part"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Ref; Integer) { }
        field(3; "Code"; Code[20]) { }
        field(4; Kind; Integer) { }
        field(5; Description; Text[50]) { }
    }
    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
        key(ByKind; Kind) { }
    }
}

table 72 "RP Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 73 "RP Reply"
{
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(root)
        {
            MaxOccurs = Once;
            tableelement(result; "RP Result")
            {
                MaxOccurs = Once;
                MinOccurs = Zero;
                UseTemporary = true;
                textelement(state)
                {
                    MaxOccurs = Once;
                    MinOccurs = Zero;
                    trigger OnBeforePassVariable()
                    begin
                        Log += 'pass state;';
                        state := result.Token;
                    end;
                }
                fieldelement(message; result.Message) { MinOccurs = Zero; }
                textelement(details)
                {
                    MaxOccurs = Once;
                    MinOccurs = Zero;
                    tableelement(line; "RP Result")
                    {
                        MinOccurs = Zero;
                        UseTemporary = true;
                        fieldattribute(no; line.Ref) { }
                        textattribute(count)
                        {
                            trigger OnBeforePassVariable()
                            begin
                                Log += 'pass count;';
                                count := Format(PartCount);
                            end;
                        }
                        textelement(parts)
                        {
                            MaxOccurs = Once;
                            MinOccurs = Zero;
                            tableelement(part; "RP Part")
                            {
                                UseTemporary = true;
                                MinOccurs = Once;
                                MaxOccurs = Unbounded;
                                SourceTableView = sorting(Kind);
                                fieldattribute(code; part.Code) { }
                                textelement(desc)
                                {
                                    MinOccurs = Once;
                                    MaxOccurs = Once;
                                    trigger OnBeforePassVariable()
                                    begin
                                        Log += 'pass desc;';
                                    end;
                                }
                                trigger OnAfterGetRecord()
                                begin
                                    Log += 'get part ' + part.Code + ';';
                                    desc := part.Description;
                                end;
                            }
                        }
                        trigger OnAfterGetRecord()
                        begin
                            Log += 'get line ' + Format(line.Ref) + ';';
                            part.DeleteAll();
                            PartCount := 0;
                            Source.SetRange(Ref, line.Ref);
                            if Source.FindSet() then
                                repeat
                                    part.TransferFields(Source, true);
                                    part.Insert();
                                    PartCount += 1;
                                until Source.Next() = 0;
                            Source.Reset();
                        end;
                    }
                }
                trigger OnAfterGetRecord()
                begin
                    Log += 'get result;';
                end;
            }
        }
    }
    var
        Source: Record "RP Part" temporary;
        Log: Text;
        PartCount: Integer;

    procedure AddPart(No: Integer; LineRef: Integer; NewCode: Code[20]; NewKind: Integer; NewDescription: Text[50])
    begin
        Source.Init();
        Source."Entry No." := No;
        Source.Ref := LineRef;
        Source.Code := NewCode;
        Source.Kind := NewKind;
        Source.Description := NewDescription;
        Source.Insert();
    end;

    procedure AddLine(No: Integer; LineRef: Integer)
    begin
        line.Init();
        line."Entry No." := No;
        line.Ref := LineRef;
        line.Insert();
    end;

    procedure SetResult(No: Integer; NewToken: Text[30]; NewMessage: Text[50])
    begin
        result.Init();
        result."Entry No." := No;
        result.Token := NewToken;
        result.Message := NewMessage;
        result.Insert();
        result.Reset();
    end;

    procedure After(): Text
    begin
        exit('token=' + result.Token + ' state=' + state);
    end;

    procedure GetLog(): Text
    begin
        exit(Log);
    end;
}

codeunit 74 C
{
    procedure Export(var P: XmlPort "RP Reply"): Text
    var
        B: Record "RP Blob" temporary;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Xml: Text;
    begin
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        P.SetDestination(OutS);
        P.Export();
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Xml += Line.Trim() + '|';
        end;
        exit(Xml + P.GetLog());
    end;

    procedure NoLines(): Text
    var
        P: XmlPort "RP Reply";
    begin
        P.SetResult(1, 'T9', 'OK');
        exit(Export(P));
    end;

    procedure TwoLines(): Text
    var
        P: XmlPort "RP Reply";
    begin
        P.SetResult(1, 'T9', 'OK');
        P.AddLine(10, 1);
        P.AddLine(20, 2);
        P.AddPart(1, 1, 'ZOO', 2, 'Zoo');
        P.AddPart(2, 1, 'SHOW', 1, 'Show');
        P.AddPart(3, 2, 'PARK', 3, '');
        exit(Export(P));
    end;

    procedure LinesWithoutParts(): Text
    var
        P: XmlPort "RP Reply";
    begin
        P.SetResult(1, 'T9', 'OK');
        P.AddLine(10, 1);
        P.AddLine(20, 2);
        P.AddLine(30, 3);
        P.AddPart(1, 2, 'ZOO', 2, 'Zoo');
        exit(Export(P));
    end;

    procedure TwoExports(): Text
    var
        P: XmlPort "RP Reply";
        First: Text;
    begin
        P.SetResult(1, 'T9', 'OK');
        P.AddLine(10, 1);
        P.AddPart(1, 1, 'ZOO', 2, 'Zoo');
        First := Export(P);
        exit(Export(P));
    end;

    procedure TwoResults(): Text
    var
        P: XmlPort "RP Reply";
    begin
        P.SetResult(1, 'T1', 'first');
        P.SetResult(2, 'T2', 'second');
        exit(Export(P) + ' ' + P.After());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<root>|<result>|";

// TR_Booking_Reply_NoLines: the optional text element over an unlinked table without rows is written empty.
#[test]
fn a_reply_without_lines_writes_the_optional_text_element_empty() {
    assert_eq!(
        run("NoLines"),
        format!("{HEAD}<state>T9</state>|<message>OK</message>|<details />|</result>|</root>|get result;pass state;")
    );
}

// TR_Booking_Reply_TwoLinesTwoParts: each line's OnAfterGetRecord refills the unlinked part table; parts follow SourceTableView.
#[test]
fn each_outer_row_writes_the_rows_the_unlinked_table_holds_when_reached() {
    assert_eq!(
        run("TwoLines"),
        format!(
            "{HEAD}<state>T9</state>|<message>OK</message>|<details>|<line no=\"1\" count=\"2\">|<parts>|<part code=\"SHOW\">|<desc>Show</desc>|</part>|<part code=\"ZOO\">|<desc>Zoo</desc>|</part>|</parts>|</line>|<line no=\"2\" count=\"1\">|<parts>|<part code=\"PARK\">|<desc />|</part>|</parts>|</line>|</details>|</result>|</root>|get result;pass state;get line 1;pass count;get part SHOW;pass desc;get part ZOO;pass desc;get line 2;pass count;get part PARK;pass desc;"
        )
    );
}

// TR_Booking_Reply_LinesWithoutParts: a MinOccurs = Once table element without rows writes one element from the record as it
// is (empty before any read, else the last row read), without its OnAfterGetRecord; its children's triggers run.
#[test]
fn a_required_unlinked_table_element_without_rows_writes_the_record_as_it_is() {
    assert_eq!(
        run("LinesWithoutParts"),
        format!(
            "{HEAD}<state>T9</state>|<message>OK</message>|<details>|<line no=\"1\" count=\"0\">|<parts>|<part code=\"\">|<desc />|</part>|</parts>|</line>|<line no=\"2\" count=\"1\">|<parts>|<part code=\"ZOO\">|<desc>Zoo</desc>|</part>|</parts>|</line>|<line no=\"3\" count=\"0\">|<parts>|<part code=\"ZOO\">|<desc>Zoo</desc>|</part>|</parts>|</line>|</details>|</result>|</root>|get result;pass state;get line 1;pass count;pass desc;get line 2;pass count;get part ZOO;pass desc;get line 3;pass count;pass desc;"
        )
    );
}

// TR_Booking_TwoReplyRows: MaxOccurs = Once with two rows writes the first only, with one OnAfterGetRecord; the record holds the
// second row afterwards.
#[test]
fn a_max_once_table_element_with_two_rows_writes_the_first_and_ends_on_the_second() {
    assert_eq!(
        run("TwoResults"),
        format!(
            "{HEAD}<state>T1</state>|<message>first</message>|<details />|</result>|</root>|get result;pass state; token=T2 state=T1"
        )
    );
}

// The partner shape of TR_Booking_Reply_TwoLinesTwoParts: a second export on the same instance, whose line OnAfterGetRecord runs
// DeleteAll() on the part table after the first export ran under its SourceTableView (only a sorting, no filters).
#[test]
fn a_second_export_may_empty_the_unlinked_table_again() {
    let once = "get result;pass state;get line 1;pass count;get part ZOO;pass desc;";
    assert_eq!(
        run("TwoExports"),
        format!(
            "{HEAD}<state>T9</state>|<message>OK</message>|<details>|<line no=\"1\" count=\"1\">|<parts>|<part code=\"ZOO\">|<desc>Zoo</desc>|</part>|</parts>|</line>|</details>|</result>|</root>|{once}{once}"
        )
    );
}
