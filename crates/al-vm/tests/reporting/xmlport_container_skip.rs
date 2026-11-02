//! Triggers of a container text element (one with child elements) and `currXMLport.Skip()` in them (corpus
//! xmlport/lt-y60-xmlport-container-skip/Y60XpContainerSkip.al): Skip in a container's OnBeforePassVariable leaves out the
//! container with all its children, whose triggers do not run; a container whose children are all skipped is written empty;
//! Skip of an element with text attributes leaves out the attributes too; Skip of a text attribute leaves out only it. On
//! import a container's OnAfterAssignVariable runs after its children, with its own text.

use al_testkit::text;

const SRC: &str = r#"
table 60 "XC Src" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Ref; Text[30]) { } } keys { key(PK; Id) { } } }
table 61 "XC Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 62 "XC Log" { SingleInstance = true; var Log: Text; procedure Add(E: Text) begin Log += E + ';'; end; procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end; }

xmlport 63 "XC Billing"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Doc)
        {
            textelement(Billing)
            {
                MinOccurs = Zero;
                textelement(InvDoc)
                {
                    MaxOccurs = Once; MinOccurs = Zero;
                    textelement(RefId) { MaxOccurs = Once; }
                    trigger OnBeforePassVariable() begin Log.Add('invdoc'); RefId := InvRef; if InvRef = '' then currXMLport.Skip(); end;
                }
                trigger OnBeforePassVariable() begin Log.Add('billing'); if BillRef = '' then currXMLport.Skip(); end;
            }
            textelement(Tail) { trigger OnBeforePassVariable() begin Log.Add('tail'); Tail := 't'; end; }
        }
    }
    procedure SetRefs(B: Text; I: Text) begin BillRef := B; InvRef := I; end;
    var Log: Codeunit "XC Log"; BillRef: Text; InvRef: Text;
}

xmlport 64 "XC Attach"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Doc)
        {
            tableelement(Row; "XC Src")
            {
                XmlName = 'Line';
                textelement(Attachment)
                {
                    textelement(Blob)
                    {
                        MinOccurs = Zero;
                        textattribute(fname) { trigger OnBeforePassVariable() begin Log.Add('fname=' + fname); if fname = '' then currXMLport.Skip(); end; }
                        trigger OnBeforePassVariable() begin Log.Add('blob=' + Blob); if Blob = '' then currXMLport.Skip(); end;
                    }
                    textelement(Ext)
                    {
                        textelement(Uri) { trigger OnBeforePassVariable() begin Log.Add('uri=' + Uri); if Uri = '' then currXMLport.Skip(); end; }
                        trigger OnBeforePassVariable() begin Log.Add('ext'); if Uri = '' then currXMLport.Skip(); end;
                    }
                }
                trigger OnAfterGetRecord() begin Log.Add('row ' + Format(Row.Id)); Blob := Bl; fname := Row.Ref; Uri := Row.Ref; end;
            }
        }
    }
    procedure SetBlob(B: Text) begin Bl := B; end;
    var Log: Codeunit "XC Log"; Bl: Text;
}

xmlport 65 "XC Billing Import"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Doc)
        {
            textelement(Billing)
            {
                MinOccurs = Zero;
                textelement(InvDoc)
                {
                    MaxOccurs = Once; MinOccurs = Zero;
                    textelement(RefId) { MaxOccurs = Once; trigger OnAfterAssignVariable() begin Log.Add('refid=' + RefId); end; }
                    trigger OnAfterAssignVariable() begin Log.Add('invdoc=[' + InvDoc + ']'); end;
                }
                trigger OnAfterAssignVariable() begin Log.Add('billing=[' + Billing + ']'); end;
            }
        }
    }
    var Log: Codeunit "XC Log";
}

codeunit 66 C
{
    local procedure Read(var B: Record "XC Blob" temporary): Text
    var InS: InStream; L: Text; R: Text;
    begin
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(L); R += L + '|'; end;
        exit(R);
    end;

    procedure Billing(BillRef: Text; InvRef: Text): Text
    var XP: XmlPort "XC Billing"; B: Record "XC Blob" temporary; OutS: OutStream; L: Codeunit "XC Log";
    begin
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS);
        XP.SetRefs(BillRef, InvRef); XP.SetDestination(OutS); XP.Export();
        exit(Read(B) + L.Take());
    end;

    procedure Attach(Bl: Text): Text
    var XP: XmlPort "XC Attach"; S: Record "XC Src"; B: Record "XC Blob" temporary; OutS: OutStream; L: Codeunit "XC Log";
    begin
        S.Id := 1; S.Ref := 'r1'; S.Insert(); S.Id := 2; S.Ref := ''; S.Insert();
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS);
        XP.SetBlob(Bl); XP.SetDestination(OutS); XP.Export();
        exit(Read(B) + L.Take());
    end;

    procedure Import(Content: Text): Text
    var XP: XmlPort "XC Billing Import"; B: Record "XC Blob" temporary; OutS: OutStream; InS: InStream; L: Codeunit "XC Log";
    begin
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS, TextEncoding::UTF8); OutS.WriteText(Content);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        XP.SetSource(InS); XP.Import();
        exit(L.Take());
    end;

    procedure BillingAll(): Text begin exit(Billing('', '') + '#' + Billing('B1', '') + '#' + Billing('B1', 'I1')); end;
    procedure AttachFilled(): Text begin exit(Attach('b')); end;
    procedure AttachEmpty(): Text begin exit(Attach('')); end;
    procedure ImportAll(): Text
    begin
        exit(Import('<Doc><Billing><InvDoc><RefId>x</RefId></InvDoc></Billing></Doc>') + '#' + Import('<Doc><Billing/></Doc>') + '#' + Import('<Doc></Doc>'));
    end;
}
"#;

const HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Doc>|"#;

#[test]
fn skip_in_a_container_leaves_it_out_with_its_children() {
    // Y60XpContainerSkip.al Container_Nested_BothTriggers_Export.
    let want = [
        format!("{HEAD}  <Tail>t</Tail>|</Doc>|billing;tail;"),
        format!("{HEAD}  <Billing />|  <Tail>t</Tail>|</Doc>|billing;invdoc;tail;"),
        format!(
            "{HEAD}  <Billing>|    <InvDoc>|      <RefId>I1</RefId>|    </InvDoc>|  </Billing>|  <Tail>t</Tail>|</Doc>|billing;invdoc;tail;"
        ),
    ]
    .join("#");
    assert_eq!(text(al_testkit::run(SRC, "BillingAll")), want);
}

#[test]
fn skip_two_levels_below_a_table_element() {
    // Container_TwoLevelsBelowTable_Export ('uri empty on two'), Container_TwoLevelsBelowTable_RowSkip_Export ('blob empty').
    assert_eq!(
        text(al_testkit::run(SRC, "AttachFilled")),
        format!(
            "{HEAD}  <Line>|    <Attachment>|      <Blob fname=\"r1\">b</Blob>|      <Ext>|        <Uri>r1</Uri>|      </Ext>|    </Attachment>|  </Line>|  <Line>|    <Attachment>|      <Blob>b</Blob>|    </Attachment>|  </Line>|</Doc>|row 1;blob=b;fname=r1;ext;uri=r1;row 2;blob=b;fname=;ext;"
        )
    );
    assert_eq!(
        text(al_testkit::run(SRC, "AttachEmpty")),
        format!(
            "{HEAD}  <Line>|    <Attachment>|      <Ext>|        <Uri>r1</Uri>|      </Ext>|    </Attachment>|  </Line>|  <Line>|    <Attachment />|  </Line>|</Doc>|row 1;blob=;ext;uri=r1;row 2;blob=;ext;"
        )
    );
}

#[test]
fn a_container_on_import_runs_its_trigger_after_its_children() {
    // Container_NestedTriggers_Import.
    assert_eq!(
        text(al_testkit::run(SRC, "ImportAll")),
        "refid=x;invdoc=[];billing=[];#billing=[];#"
    );
}
