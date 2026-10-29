//! A text element with the default MinOccurs and MaxOccurs right below a temporary table element, holding one unlinked
//! temporary table element (MaxOccurs = Once, MinOccurs = Zero) (assumption `xmlport-default-container-unlinked`; corpus branch
//! al-runtime-rs-staging-pub-ts, xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Cards_DefaultOccurs_Export,
//! xmlport 90084 in TsXpObjects.al).

use al_testkit::text;

const SRC: &str = r#"
table 90 "DC Member"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; MemberNo; Code[20]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 91 "DC Card"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; CardNo; Text[50]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 92 "DC Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 93 "DC Reply"
{
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(members)
        {
            MaxOccurs = Once;
            tableelement(reply; "DC Member")
            {
                MaxOccurs = Once;
                MinOccurs = Zero;
                UseTemporary = true;
                fieldelement(memberno; reply.MemberNo) { }
                textelement(cards)
                {
                    tableelement(card; "DC Card")
                    {
                        MaxOccurs = Once;
                        MinOccurs = Zero;
                        UseTemporary = true;
                        fieldelement(cardnumber; card.CardNo) { }
                        trigger OnAfterGetRecord()
                        begin
                            Log += 'get card ' + card.CardNo + ';';
                        end;
                    }
                }
                trigger OnAfterGetRecord()
                begin
                    Log += 'get reply ' + reply.MemberNo + ';';
                end;
            }
        }
    }
    var
        Log: Text;

    procedure AddResponse(No: Integer; NewMemberNo: Code[20])
    begin
        reply.Init();
        reply."Entry No." := No;
        reply.MemberNo := NewMemberNo;
        reply.Insert();
    end;

    procedure AddCard(No: Integer; NewCardNo: Text[50])
    begin
        card.Init();
        card."Entry No." := No;
        card.CardNo := NewCardNo;
        card.Insert();
    end;

    procedure GetLog(): Text
    begin
        exit(Log);
    end;
}

codeunit 94 C
{
    procedure Export(var P: XmlPort "DC Reply"): Text
    var
        B: Record "DC Blob" temporary;
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
        InS.ReadText(Line);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Xml += Line.Trim() + '|';
        end;
        exit(Xml + P.GetLog());
    end;

    procedure NoCard(): Text
    var
        P: XmlPort "DC Reply";
    begin
        P.AddResponse(5, 'M-100');
        exit(Export(P));
    end;

    procedure TwoCards(): Text
    var
        P: XmlPort "DC Reply";
    begin
        P.AddResponse(5, 'M-100');
        P.AddCard(1, 'CARD-1');
        P.AddCard(2, 'CARD-2');
        exit(Export(P));
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

// TS_Cards_DefaultOccurs_Export `no card`: the text element is written empty.
#[test]
fn a_default_container_without_rows_is_written_empty() {
    assert_eq!(
        run("NoCard"),
        "<members>|<reply>|<memberno>M-100</memberno>|<cards />|</reply>|</members>|get reply M-100;"
    );
}

// TS_Cards_DefaultOccurs_Export `two cards`: only the first card, with one OnAfterGetRecord.
#[test]
fn a_default_container_with_two_rows_holds_the_first() {
    assert_eq!(
        run("TwoCards"),
        "<members>|<reply>|<memberno>M-100</memberno>|<cards>|<card>|<cardnumber>CARD-1</cardnumber>|</card>|</cards>|</reply>|</members>|get reply M-100;get card CARD-1;"
    );
}
