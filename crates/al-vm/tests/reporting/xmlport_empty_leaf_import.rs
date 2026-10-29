//! XmlPort import of an empty leaf text element: `<x />` runs its OnAfterAssignVariable twice, `<x></x>`, `<x> </x>` and
//! `<x>v</x>` once, below the root, below a table element and inside a text element below a table element. A field element's
//! OnAfterAssignField and a container text element's OnAfterAssignVariable run once for both forms (corpus branch
//! al-runtime-rs-staging-tr, xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TR_EmptyLeaf_Import, port in
//! TrXpReplyObjects.al).

use al_testkit::text;

const SRC: &str = r#"
table 60 "EL Row"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; Message; Text[50]) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 61 "EL Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 62 "EL Port"
{
    Format = Xml;
    FormatEvaluate = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(root)
        {
            MaxOccurs = Once;
            textelement(top)
            {
                MinOccurs = Zero;
                MaxOccurs = Once;
                trigger OnAfterAssignVariable()
                begin
                    Log += 'top=[' + top + '];';
                end;
            }
            tableelement(row; "EL Row")
            {
                MinOccurs = Zero;
                UseTemporary = true;
                fieldattribute(no; row."Entry No.") { }
                fieldelement(msg; row.Message)
                {
                    MinOccurs = Zero;
                    trigger OnAfterAssignField()
                    begin
                        Log += 'msg=[' + row.Message + '];';
                    end;
                }
                textelement(direct)
                {
                    MinOccurs = Zero;
                    MaxOccurs = Once;
                    trigger OnAfterAssignVariable()
                    begin
                        Log += 'direct=[' + direct + '];';
                    end;
                }
                textelement(box)
                {
                    MinOccurs = Zero;
                    MaxOccurs = Once;
                    textelement(inner)
                    {
                        MinOccurs = Zero;
                        MaxOccurs = Once;
                        trigger OnAfterAssignVariable()
                        begin
                            Log += 'inner=[' + inner + '];';
                        end;
                    }
                    trigger OnAfterAssignVariable()
                    begin
                        Log += 'box=[' + box + '];';
                    end;
                }
                trigger OnBeforeInsertRecord()
                begin
                    Log += 'insert ' + Format(row."Entry No.") + ' msg=[' + row.Message + '];';
                end;
            }
        }
    }
    var
        Log: Text;

    procedure GetLog(): Text
    begin
        exit(Log);
    end;
}

codeunit 63 C
{
    procedure Import(Body: Text): Text
    var
        P: XmlPort "EL Port";
        B: Record "EL Blob" temporary;
        OutS: OutStream;
        InS: InStream;
    begin
        B.Id := 1;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Body);
        B.Insert();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        P.SetSource(InS);
        P.Import();
        exit(P.GetLog());
    end;

    procedure SelfClosing(): Text
    begin
        exit(Import('<root><top /><row no="1"><msg /><direct /><box><inner /></box></row></root>'));
    end;

    procedure OpenClose(): Text
    begin
        exit(Import('<root><top></top><row no="1"><msg></msg><direct></direct><box><inner></inner></box></row></root>'));
    end;

    procedure Space(): Text
    begin
        exit(Import('<root><top> </top><row no="1"><msg> </msg><direct> </direct><box><inner> </inner></box></row></root>'));
    end;

    procedure BoxSelfClosing(): Text
    begin
        exit(Import('<root><row no="1"><box /></row></root>'));
    end;

    procedure TwoRows(): Text
    begin
        exit(Import('<root><top /><row no="1"><direct /></row><row no="2"><direct></direct><box><inner /></box></row></root>'));
    end;
}
"#;

fn run(routine: &str) -> String {
    text(al_testkit::run_in(SRC, "C", routine))
}

// TR_EmptyLeaf_Import `self-closing`.
#[test]
fn a_self_closing_leaf_text_element_runs_its_assign_trigger_twice() {
    assert_eq!(
        run("SelfClosing"),
        "top=[];top=[];msg=[];direct=[];direct=[];inner=[];inner=[];box=[];insert 1 msg=[];"
    );
}

// TR_EmptyLeaf_Import `open-close`, `space`.
#[test]
fn an_empty_leaf_text_element_with_an_end_tag_runs_its_assign_trigger_once() {
    let once = "top=[];msg=[];direct=[];inner=[];box=[];insert 1 msg=[];";
    assert_eq!(run("OpenClose"), once);
    assert_eq!(run("Space"), once);
}

// TR_EmptyLeaf_Import `box self-closing`, `two rows`.
#[test]
fn a_self_closing_container_text_element_runs_its_trigger_once() {
    assert_eq!(run("BoxSelfClosing"), "box=[];insert 1 msg=[];");
    assert_eq!(
        run("TwoRows"),
        "top=[];top=[];direct=[];direct=[];insert 1 msg=[];direct=[];inner=[];inner=[];box=[];insert 2 msg=[];"
    );
}
