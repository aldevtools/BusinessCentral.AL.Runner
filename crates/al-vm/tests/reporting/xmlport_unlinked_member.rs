//! A reply row's optional text element holding field elements of the row and an unlinked temporary table element
//! (MaxOccurs = Once, MinOccurs = Zero) (assumption `xmlport-unlinked-reply`; corpus branch al-runtime-rs-staging-tr,
//! xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TR_Member_*, xmlport 90065 in TrXpReplyObjects.al).

use al_testkit::text;

const SRC: &str = r#"
table 80 "MB Member"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; MemberNo; Code[20]) { }
        field(3; Email; Text[80]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 81 "MB Card"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; CardNo; Text[50]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 82 "MB Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 83 "MB Reply"
{
    Format = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(root)
        {
            MaxOccurs = Once;
            tableelement(reply; "MB Member")
            {
                MaxOccurs = Once;
                MinOccurs = Zero;
                UseTemporary = true;
                textelement(status) { MaxOccurs = Once; }
                textelement(member)
                {
                    MaxOccurs = Once;
                    MinOccurs = Zero;
                    fieldelement(memberno; reply.MemberNo) { }
                    fieldelement(email; reply.Email) { }
                    tableelement(card; "MB Card")
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

    procedure AddResponse(No: Integer; NewMemberNo: Code[20]; NewEmail: Text[80])
    begin
        status := '1';
        reply.Init();
        reply."Entry No." := No;
        reply.MemberNo := NewMemberNo;
        reply.Email := NewEmail;
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

    procedure CardNo(): Text
    begin
        exit(card.CardNo);
    end;
}

codeunit 84 C
{
    procedure Export(var P: XmlPort "MB Reply"): Text
    var
        B: Record "MB Blob" temporary;
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

    procedure NoCard(): Text
    var
        P: XmlPort "MB Reply";
    begin
        P.AddResponse(5, 'M-100', 'ann@example.com');
        exit(Export(P));
    end;

    procedure TwoCards(): Text
    var
        P: XmlPort "MB Reply";
    begin
        P.AddResponse(5, 'M-100', 'ann@example.com');
        P.AddCard(1, 'CARD-1');
        P.AddCard(2, 'CARD-2');
        exit(Export(P));
    end;

    procedure ThreeCards(): Text
    var
        P: XmlPort "MB Reply";
        First: Text;
    begin
        P.AddResponse(5, 'M-100', 'ann@example.com');
        P.AddCard(1, 'CARD-1');
        P.AddCard(2, 'CARD-2');
        First := Export(P);
        P.AddCard(0, 'CARD-0');
        exit(First + ' # ' + Export(P));
    end;

    procedure ThreeCardsThenRead(): Text
    var
        P: XmlPort "MB Reply";
    begin
        P.AddResponse(5, 'M-100', 'ann@example.com');
        P.AddCard(1, 'CARD-1');
        P.AddCard(2, 'CARD-2');
        P.AddCard(3, 'CARD-3');
        Export(P);
        exit(P.CardNo());
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<root>|<reply>|<status>1</status>|<member>|<memberno>M-100</memberno>|<email>ann@example.com</email>|";

// TR_Member_Reply_Cards `no card`: the optional table element without rows is left out, the text element keeps the fields.
#[test]
fn a_reply_without_a_card_writes_the_member_fields_only() {
    assert_eq!(
        run("NoCard"),
        format!("{HEAD}</member>|</reply>|</root>|get reply M-100;")
    );
}

// TR_Member_Reply_Cards `two cards`: MaxOccurs = Once writes the first card only, with one OnAfterGetRecord.
#[test]
fn a_reply_with_two_cards_writes_the_first() {
    assert_eq!(
        run("TwoCards"),
        format!(
            "{HEAD}<card>|<cardnumber>CARD-1</cardnumber>|</card>|</member>|</reply>|</root>|get reply M-100;get card CARD-1;"
        )
    );
}

// TS_Again_TrMember (branch al-runtime-rs-staging-pub-ts): a second export reads the cards again from the start; with three
// cards, a card added in between that sorts first is the one written, with one OnAfterGetRecord.
#[test]
fn a_later_export_with_three_cards_writes_the_first() {
    let card = |no: &str, log: &str| {
        format!(
            "{HEAD}<card>|<cardnumber>{no}</cardnumber>|</card>|</member>|</reply>|</root>|get reply M-100;get card {log};"
        )
    };
    assert_eq!(
        run("ThreeCards"),
        format!(
            "{} # {}",
            card("CARD-1", "CARD-1"),
            card("CARD-0", "CARD-1;get reply M-100;get card CARD-0")
        )
    );
}

// Which card the record holds after an export of three cards is not pinned: reading it refuses.
#[test]
fn the_card_after_an_export_of_three_cards_is_refused() {
    let r = al_testkit::run_in(SRC, "C", "ThreeCardsThenRead");
    assert!(
        matches!(&r, Err(al_builtins::native::Signal::Unsupported(m)) if m.contains("three or more rows")),
        "{r:?}"
    );
}
