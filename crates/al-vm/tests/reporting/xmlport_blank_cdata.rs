//! Xml import of a value that is white space only (corpus xmlport/lt-y91-xmlport-probes/Y91XpTests.al): with
//! PreserveWhiteSpace a CDATA section of white space is kept in a text element (:149 `cdata_space`) and a Text field element
//! (:214 `n_cdata_space`), plain white space is not; a text attribute of white space only is kept with it (:184 `att_space`)
//! and is empty without it (:82 `att_space`).

use al_testkit::text;

const SRC: &str = r#"
table 74 "BC Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 70 "BC Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
xmlport 71 "BC Keep"
{
    Direction = Import; Format = Xml; PreserveWhiteSpace = true; UseRequestPage = false;
    schema { textelement(Root) { textelement(A) { } textelement(B) { textattribute(Att) { } } tableelement(Item; "BC Row") { fieldelement(Id; Item.Id) { } fieldelement(Name; Item.Name) { } } } }
    procedure Seen(): Text begin exit('A=[' + A + '] Att=[' + Att + ']'); end;
}
xmlport 72 "BC Default"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { textelement(A) { } textelement(B) { textattribute(Att) { } } tableelement(Item; "BC Row") { fieldelement(Id; Item.Id) { } fieldelement(Name; Item.Name) { } } } }
    procedure Seen(): Text begin exit('A=[' + A + '] Att=[' + Att + ']'); end;
}
codeunit 73 C
{
    procedure Run(Doc: Text; Keep: Boolean): Text
    var K: XmlPort "BC Keep"; D: XmlPort "BC Default"; B: Record "BC Row"; Tb: Record "BC Blob" temporary; OutS: OutStream; InS: InStream; R: Text;
    begin
        B.DeleteAll();
        Tb.Data.CreateOutStream(OutS);
        OutS.WriteText(Doc);
        Tb.Data.CreateInStream(InS);
        if Keep then begin K.SetSource(InS); K.Import(); R := K.Seen(); end else begin D.SetSource(InS); D.Import(); R := D.Seen(); end;
        B.FindFirst();
        exit(R + ' Name=[' + B.Name + ']');
    end;
    procedure CDataKept(): Text
    begin
        exit(Run('<Root><A><![CDATA[ ]]></A><B Att="y">z</B><Item><Id>5</Id><Name><![CDATA[ ]]></Name></Item></Root>', true));
    end;
    procedure CDataDefault(): Text
    begin
        exit(Run('<Root><A><![CDATA[ ]]></A><B Att="y">z</B><Item><Id>5</Id><Name><![CDATA[ ]]></Name></Item></Root>', false));
    end;
    procedure PlainKept(): Text
    begin
        exit(Run('<Root><A> </A><B Att="y">z</B><Item><Id>5</Id><Name> </Name></Item></Root>', true));
    end;
    procedure AttKept(): Text
    begin
        exit(Run('<Root><A>x</A><B Att=" ">z</B><Item><Id>5</Id><Name>n</Name></Item></Root>', true));
    end;
    procedure AttDefault(): Text
    begin
        exit(Run('<Root><A>x</A><B Att=" ">z</B><Item><Id>5</Id><Name>n</Name></Item></Root>', false));
    end;
    procedure AttPaddedDefault(): Text
    begin
        exit(Run('<Root><A>x</A><B Att="  p  ">z</B><Item><Id>5</Id><Name>n</Name></Item></Root>', false));
    end;
}
"#;

#[test]
fn preserve_keeps_a_cdata_section_of_white_space() {
    assert_eq!(text(al_testkit::run(SRC, "CDataKept")), "A=[ ] Att=[y] Name=[ ]");
}

#[test]
fn preserve_drops_plain_white_space() {
    assert_eq!(text(al_testkit::run(SRC, "PlainKept")), "A=[] Att=[y] Name=[]");
}

#[test]
fn without_preserve_a_cdata_section_of_white_space_is_empty() {
    assert_eq!(text(al_testkit::run(SRC, "CDataDefault")), "A=[] Att=[y] Name=[]");
}

#[test]
fn preserve_keeps_a_text_attribute_of_white_space() {
    assert_eq!(text(al_testkit::run(SRC, "AttKept")), "A=[x] Att=[ ] Name=[n]");
}

#[test]
fn without_preserve_a_text_attribute_is_trimmed() {
    assert_eq!(text(al_testkit::run(SRC, "AttDefault")), "A=[x] Att=[] Name=[n]");
    assert_eq!(text(al_testkit::run(SRC, "AttPaddedDefault")), "A=[x] Att=[p] Name=[n]");
}
