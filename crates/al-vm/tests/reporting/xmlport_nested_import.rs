//! Xml import of nested linked table elements whose parent has no AutoReplace: the trigger order BC runs
//! (corpus xmlport/lt-xmlport-trigger-flow/RcXmlPortNestedImport.al, tests XmlPortTriggerFlow.al:452-566).

use al_testkit::run;

const SRC: &str = r#"
table 69970 "NI Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69971 "NI Par"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } }
    trigger OnInsert() begin L.Add('tP+' + Format(Id)); end;
    trigger OnModify() begin L.Add('tP~' + Format(Id)); end;
    var L: Codeunit "NI Log";
}
table 69972 "NI Chi"
{
    fields { field(1; Id; Integer) { } field(2; ParentId; Integer) { } } keys { key(PK; Id) { } }
    trigger OnInsert() begin L.Add('tC+' + Format(Id) + '/' + Format(ParentId)); end;
    trigger OnModify() begin L.Add('tC~' + Format(Id) + '/' + Format(ParentId)); end;
    var L: Codeunit "NI Log";
}
table 69973 "NI GChi"
{
    fields { field(1; Id; Integer) { } field(2; ChildId; Integer) { } } keys { key(PK; Id) { } }
    trigger OnInsert() begin L.Add('tG+' + Format(Id) + '/' + Format(ChildId)); end;
    var L: Codeunit "NI Log";
}
codeunit 69974 "NI Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(T: Text) begin Entries += T + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
xmlport 69975 "NI Two"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NI Par")
            {
                fieldelement(Id; Par.Id) { }
                fieldelement(Name; Par.Name) { }
                tableelement(Chi; "NI Chi")
                {
                    LinkTable = Par; LinkFields = ParentId = field(Id); MinOccurs = Zero;
                    fieldelement(CId; Chi.Id) { }
                    trigger OnAfterGetRecord() begin L.Add('C.agr'); end;
                    trigger OnBeforeInsertRecord() begin L.Add('C.bi=' + Format(Chi.Id)); end;
                    trigger OnBeforeModifyRecord() begin L.Add('C.bm=' + Format(Chi.Id)); end;
                    trigger OnAfterModifyRecord() begin L.Add('C.am=' + Format(Chi.Id)); end;
                    tableelement(G; "NI GChi")
                    {
                        LinkTable = Chi; LinkFields = ChildId = field(Id); MinOccurs = Zero;
                        fieldelement(GId; G.Id) { }
                        trigger OnBeforeInsertRecord() begin L.Add('G.bi=' + Format(G.Id)); end;
                    }
                }
                trigger OnAfterGetRecord() begin L.Add('P.agr'); end;
                trigger OnBeforeInsertRecord() begin L.Add('P.bi=' + Format(Par.Id)); end;
                trigger OnAfterInsertRecord() begin L.Add('P.ai=' + Format(Par.Id)); end;
                trigger OnBeforeModifyRecord() begin L.Add('P.bm=' + Format(Par.Id)); end;
                trigger OnAfterModifyRecord() begin L.Add('P.am=' + Format(Par.Id)); end;
            }
        }
    }
    var L: Codeunit "NI Log";
}
xmlport 69976 "NI Temp"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Par; "NI Par")
            {
                fieldelement(Id; Par.Id) { }
                tableelement(Chi; "NI Chi")
                {
                    LinkTable = Par; LinkFields = ParentId = field(Id); MinOccurs = Zero; UseTemporary = true; AutoReplace = true;
                    fieldelement(CId; Chi.Id) { }
                    trigger OnBeforeInsertRecord() begin L.Add('C.bi=' + Format(Chi.Id)); end;
                    trigger OnBeforeModifyRecord() begin L.Add('C.bm=' + Format(Chi.Id)); end;
                }
                trigger OnBeforeModifyRecord() begin L.Add('P.bm=' + Format(Par.Id)); end;
            }
        }
        trigger OnPostXmlPort() begin if Chi.FindSet() then repeat L.Add('tmp=' + Format(Chi.Id) + '/' + Format(Chi.ParentId)); until Chi.Next() = 0; end;
    }
    var L: Codeunit "NI Log";
}
codeunit 69977 C
{
    local procedure Import(Port: Integer; Doc: Text)
    var B: Record "NI Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText(Doc);
        B.Data.CreateInStream(InS);
        Xmlport.Import(Port, InS);
    end;

    procedure NewParents(): Text
    var L: Codeunit "NI Log";
    begin
        Import(Xmlport::"NI Two", '<Root><Par><Id>1</Id><Name>one</Name><Chi><CId>10</CId></Chi><Chi><CId>11</CId></Chi></Par><Par><Id>2</Id><Name>two</Name></Par></Root>');
        exit(L.Take());
    end;

    procedure ThreeLevels(): Text
    var L: Codeunit "NI Log";
    begin
        Import(Xmlport::"NI Two", '<Root><Par><Id>1</Id><Name>one</Name><Chi><CId>10</CId><G><GId>100</GId></G><G><GId>101</GId></G></Chi><Chi><CId>11</CId></Chi></Par></Root>');
        exit(L.Take());
    end;

    procedure ExistingParent(): Text
    var L: Codeunit "NI Log"; P: Record "NI Par";
    begin
        P.Id := 1; P.Insert();
        L.Take();
        asserterror Import(Xmlport::"NI Two", '<Root><Par><Id>1</Id><Name>new</Name><Chi><CId>10</CId></Chi></Par></Root>');
        exit(L.Take() + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure TempChild(): Text
    var L: Codeunit "NI Log";
    begin
        Import(Xmlport::"NI Temp", '<Root><Par><Id>1</Id><Chi><CId>10</CId></Chi><Chi><CId>10</CId></Chi><Chi><CId>11</CId></Chi></Par><Par><Id>2</Id><Chi><CId>11</CId></Chi></Par></Root>');
        exit(L.Take());
    end;
}
"#;

#[test]
fn a_parent_without_auto_replace_is_modified_again_after_its_child_rows() {
    // NoAutoReplace_NewParentTwoChildren (XmlPortTriggerFlow.al:452, trace :462): P.bm, table OnModify, P.am after the child
    // rows; the second child gets ParentId 0; a parent without child rows is not modified again; OnAfterGetRecord never runs.
    assert_eq!(
        al_testkit::text(run(SRC, "NewParents")),
        "P.bi=1;tP+1;P.ai=1;C.bi=10;tC+10/1;C.bi=11;tC+11/0;P.bm=1;tP~1;P.am=1;P.bi=2;tP+2;P.ai=2;"
    );
}

#[test]
fn each_row_with_child_rows_is_modified_after_its_own_children() {
    // ThreeLevels_NoAutoReplace (:535, trace :545).
    assert_eq!(
        al_testkit::text(run(SRC, "ThreeLevels")),
        "P.bi=1;tP+1;P.ai=1;C.bi=10;tC+10/1;G.bi=100;tG+100/10;G.bi=101;tG+101/0;C.bm=10;tC~10/1;C.am=10;C.bi=11;tC+11/0;P.bm=1;tP~1;P.am=1;"
    );
}

#[test]
fn an_existing_parent_fails_after_its_child_rows_before_its_second_modify() {
    // NoAutoReplace_ExistingParentWithChild (:468): the insert runs its triggers, the child rows are inserted, then the import
    // fails with the table's "already exists" before the parent's second OnBeforeModifyRecord.
    assert_eq!(
        al_testkit::text(run(SRC, "ExistingParent")),
        "P.bi=1;tP+1;P.ai=1;C.bi=10;tC+10/1;|The record in table NI Par already exists. Identification fields and values: Id='1'|DB:RecordExists"
    );
}

#[test]
fn a_temporary_auto_replace_child_runs_no_table_triggers() {
    // TempChild_AutoReplace (:551, trace :561): no table triggers for the temporary child; a key already in the temporary table
    // is modified, also under another parent; the replaced row 10 gets ParentId 0.
    assert_eq!(
        al_testkit::text(run(SRC, "TempChild")),
        "tP+1;C.bi=10;C.bm=10;C.bi=11;P.bm=1;tP~1;tP+2;C.bm=11;P.bm=2;tP~2;tmp=10/0;tmp=11/2;"
    );
}
