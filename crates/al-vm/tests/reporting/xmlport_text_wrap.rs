//! A text element with content inside a table element (corpus xmlport/lt-y60-xmlport-text-in-table/Y60XpTextInTable.al):
//! a text element that wraps a table element without LinkTable (import: the inner rows are stored before the outer one, a field
//! element of the outer table assigns it on every inner row; export: the inner table lists all its rows under every outer row,
//! `<Wrap />` without rows); a text element with a text attribute, leaf text elements and one with its own text attribute
//! (import); a text element with field elements of the table and a text element that wraps a table element, with triggers
//! (export).

use al_testkit::text;

const SRC: &str = r#"
table 70 "XT Hdr" { fields { field(1; "No."; Code[20]) { } field(2; Name; Text[30]) { } field(3; Note; Text[30]) { } } keys { key(PK; "No.") { } } }
table 71 "XT Line" { fields { field(1; "Hdr No."; Code[20]) { } field(2; "Line No."; Integer) { } field(3; Item; Text[30]) { } } keys { key(PK; "Hdr No.", "Line No.") { } } }
table 72 "XT Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 73 "XT Log" { SingleInstance = true; var Log: Text; procedure Add(E: Text) begin Log += E + ';'; end; procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end; }

xmlport 74 "XT Wrap Import"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Outer; "XT Hdr")
            {
                MaxOccurs = Once; UseTemporary = true;
                fieldelement(No; Outer."No.") { }
                textelement(Wrap)
                {
                    MinOccurs = Zero;
                    tableelement(Inner; "XT Line")
                    {
                        UseTemporary = true;
                        fieldelement(LineNo; Inner."Line No.") { }
                        fieldelement(Name; Outer.Name) { }
                        trigger OnBeforeInsertRecord() begin Log.Add('inner ins ' + Format(Inner."Line No.") + ' outer.name=' + Outer.Name); Inner."Hdr No." := Outer."No."; end;
                    }
                }
                trigger OnBeforeInsertRecord() begin Log.Add('outer ins ' + Outer."No." + ' name=' + Outer.Name); end;
            }
        }
    }
    trigger OnPostXmlPort()
    begin
        Inner.Reset(); if Inner.FindSet() then repeat Log.Add('[' + Inner."Hdr No." + '|' + Format(Inner."Line No.") + ']'); until Inner.Next() = 0;
    end;
    var Log: Codeunit "XT Log";
}

xmlport 75 "XT Ticket Import"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "XT Hdr")
            {
                textelement(Ticket)
                {
                    textattribute(kind) { trigger OnAfterAssignVariable() begin Log.Add('kind=' + kind); Row.Name := kind; end; }
                    textelement(Code) { MinOccurs = Zero; trigger OnAfterAssignVariable() begin Log.Add('code=' + Code); Row."No." := Code; end; }
                    textelement(Title)
                    {
                        MinOccurs = Zero;
                        textattribute(lang) { trigger OnAfterAssignVariable() begin Log.Add('lang=' + lang); end; }
                        trigger OnAfterAssignVariable() begin Log.Add('title=' + Title); Row.Note := Title; end;
                    }
                    trigger OnAfterAssignVariable() begin Log.Add('ticket=[' + Ticket + ']'); end;
                }
                trigger OnBeforeInsertRecord() begin Log.Add('ins ' + Row."No."); end;
            }
        }
    }
    var Log: Codeunit "XT Log";
}

xmlport 76 "XT Wrap Export"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "XT Hdr")
            {
                fieldattribute(No; Row."No.") { }
                textelement(Wrap)
                {
                    MinOccurs = Zero;
                    tableelement(Mid; "XT Line")
                    {
                        MinOccurs = Zero;
                        fieldattribute(LineNo; Mid."Line No.") { }
                        trigger OnAfterGetRecord() begin Log.Add('mid ' + Format(Mid."Line No.")); end;
                    }
                }
                trigger OnAfterGetRecord() begin Log.Add('row ' + Row."No."); end;
            }
        }
    }
    var Log: Codeunit "XT Log";
}

xmlport 77 "XT Info Export"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Row; "XT Hdr")
            {
                UseTemporary = true;
                textelement(A) { trigger OnBeforePassVariable() begin Log.Add('a'); A := 'a-' + Row."No."; end; }
                textelement(Info)
                {
                    MinOccurs = Zero;
                    fieldelement(Name; Row.Name) { textattribute(unit) { trigger OnBeforePassVariable() begin Log.Add('unit'); unit := 'pcs'; end; } }
                    fieldelement(Note; Row.Note) { }
                    textelement(Sub)
                    {
                        tableelement(Child; "XT Line")
                        {
                            UseTemporary = true;
                            fieldelement(LineNo; Child."Line No.") { }
                            trigger OnAfterGetRecord() begin Log.Add('child ' + Format(Child."Line No.")); end;
                        }
                        trigger OnBeforePassVariable() begin Log.Add('sub'); end;
                    }
                    trigger OnBeforePassVariable() begin Log.Add('info'); end;
                }
                trigger OnAfterGetRecord() begin Log.Add('row ' + Row."No."); end;
            }
        }
    }
    procedure Fill()
    begin
        Row."No." := 'H1'; Row.Name := 'one'; Row.Note := 'n1'; Row.Insert();
        Row."No." := 'H2'; Row.Name := 'two'; Row.Note := ''; Row.Insert();
        Child."Hdr No." := 'H1'; Child."Line No." := 1; Child.Insert();
        Child."Hdr No." := 'H2'; Child."Line No." := 2; Child.Insert();
    end;
    var Log: Codeunit "XT Log";
}

codeunit 78 C
{
    local procedure Import(Id: Integer; Content: Text): Text
    var B: Record "XT Blob" temporary; OutS: OutStream; InS: InStream; L: Codeunit "XT Log";
    begin
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS, TextEncoding::UTF8); OutS.WriteText(Content);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Id, InS);
        exit(L.Take());
    end;

    local procedure Read(var B: Record "XT Blob" temporary): Text
    var InS: InStream; L: Text; R: Text;
    begin
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(L); R += L + '|'; end;
        exit(R);
    end;

    procedure WrapImport(): Text
    begin
        exit(Import(74, '<Root><Outer><No>H1</No><Wrap><Inner><LineNo>1</LineNo><Name>n1</Name></Inner><Inner><LineNo>2</LineNo><Name>n2</Name></Inner></Wrap></Outer></Root>')
            + '#' + Import(74, '<Root><Outer><No>H1</No></Outer></Root>'));
    end;

    procedure TicketImport(): Text
    var H: Record "XT Hdr"; R: Text;
    begin
        R := Import(75, '<Root><Row><Ticket kind="k"><Code>c</Code><Title lang="en">t</Title></Ticket></Row><Row><Ticket kind="k2"><Code>c2</Code></Ticket></Row></Root>');
        if H.FindSet() then repeat R += '[' + H."No." + '|' + H.Name + '|' + H.Note + ']'; until H.Next() = 0;
        exit(R);
    end;

    procedure WrapExport(): Text
    var H: Record "XT Hdr"; Ln: Record "XT Line"; B: Record "XT Blob" temporary; OutS: OutStream; L: Codeunit "XT Log"; R: Text;
    begin
        H."No." := 'H1'; H.Insert(); H."No." := 'H2'; H.Insert();
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS); Xmlport.Export(76, OutS);
        R := Read(B) + L.Take();
        Ln."Hdr No." := 'H1'; Ln."Line No." := 10; Ln.Insert(); Ln."Hdr No." := 'H1'; Ln."Line No." := 20; Ln.Insert();
        B.Delete(); B.Insert(); B.Data.CreateOutStream(OutS); Xmlport.Export(76, OutS);
        exit(R + '#' + Read(B) + L.Take());
    end;

    procedure InfoExport(): Text
    var XP: XmlPort "XT Info Export"; B: Record "XT Blob" temporary; OutS: OutStream; L: Codeunit "XT Log";
    begin
        XP.Fill();
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS); XP.SetDestination(OutS); XP.Export();
        exit(Read(B) + L.Take());
    end;
}
"#;

const HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Root>|"#;

#[test]
fn a_wrapped_inner_table_is_stored_before_the_outer_row() {
    // Y60XpTextInTable.al TextWrap_TempTable_Inner_Import ('two inner', 'no wrap').
    assert_eq!(
        text(al_testkit::run(SRC, "WrapImport")),
        "inner ins 1 outer.name=n1;inner ins 2 outer.name=n2;outer ins H1 name=n2;[H1|1];[H1|2];#outer ins H1 name=;"
    );
}

#[test]
fn a_text_element_with_an_attribute_leaves_and_an_attributed_leaf_imports_in_document_order() {
    // TextWrap_Attr_Leaves_Import ('full', 'two rows').
    assert_eq!(
        text(al_testkit::run(SRC, "TicketImport")),
        "kind=k;code=c;lang=en;title=t;ticket=[];ins C;kind=k2;code=c2;ticket=[];ins C2;[C|k|t][C2|k2|]"
    );
}

#[test]
fn a_wrapped_unlinked_table_lists_all_its_rows_under_every_row() {
    // TextWrap_Table_Nested_Export, TextWrap_Q3_NoRows ('no lines').
    assert_eq!(
        text(al_testkit::run(SRC, "WrapExport")),
        format!(
            "{HEAD}  <Row No=\"H1\">|    <Wrap />|  </Row>|  <Row No=\"H2\">|    <Wrap />|  </Row>|</Root>|row H1;row H2;#{HEAD}  <Row No=\"H1\">|    <Wrap>|      <Mid LineNo=\"10\" />|      <Mid LineNo=\"20\" />|    </Wrap>|  </Row>|  <Row No=\"H2\">|    <Wrap>|      <Mid LineNo=\"10\" />|      <Mid LineNo=\"20\" />|    </Wrap>|  </Row>|</Root>|row H1;mid 10;mid 20;row H2;mid 10;mid 20;"
        )
    );
}

#[test]
fn a_text_element_of_fields_and_a_wrapped_table_runs_its_triggers() {
    // TextWrap_Info_Fields_Sub_Export.
    let row = |no: &str, name: &str, note: &str| {
        format!(
            "  <Row>|    <A>a-{no}</A>|    <Info>|      <Name unit=\"pcs\">{name}</Name>|      {note}|      <Sub>|        <Child>|          <LineNo>1</LineNo>|        </Child>|        <Child>|          <LineNo>2</LineNo>|        </Child>|      </Sub>|    </Info>|  </Row>|"
        )
    };
    assert_eq!(
        text(al_testkit::run(SRC, "InfoExport")),
        format!(
            "{HEAD}{}{}</Root>|row H1;a;info;unit;sub;child 1;child 2;row H2;a;info;unit;sub;child 1;child 2;",
            row("H1", "one", "<Note>n1</Note>"),
            row("H2", "two", "<Note />")
        )
    );
}
