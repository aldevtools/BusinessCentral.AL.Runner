//! A container text element right below a table element, without a trigger of its own, whose nodes are leaf text elements (one
//! with an OnBeforePassVariable that skips when empty) and a container of leaves: the TaxCategory of a TaxSubtotal and of an
//! AllowanceCharge in Microsoft's PEPPOL 3.0 ports (al-runner issue 6924). Not pinned yet: the export marks assumption
//! `xmlport-leaf-triggers-tree-in-table`.

use al_builtins::assumptions::{Assumption, take_hits};
use al_testkit::{run, text};

const SRC: &str = r#"
table 69980 "XQ Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69981 "XQ Num" { fields { field(1; Number; Integer) { } } keys { key(PK; Number) { } } }
codeunit 69982 "XQ Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
xmlport 69983 "XQ Tax"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    Namespaces = "" = 'urn:p:inv';
    schema
    {
        tableelement(hdr; "XQ Num")
        {
            MaxOccurs = Once; SourceTableView = sorting(Number) where(Number = filter(1 ..)); XmlName = 'Invoice';
            tableelement(sub; "XQ Num")
            {
                XmlName = 'TaxSubtotal'; SourceTableView = sorting(Number) where(Number = filter(1 ..));
                textelement(Category)
                {
                    XmlName = 'TaxCategory';
                    textelement(catid) { XmlName = 'ID'; }
                    textelement(pct) { XmlName = 'Percent'; }
                    textelement(reason)
                    {
                        XmlName = 'TaxExemptionReason';
                        trigger OnBeforePassVariable() var L: Codeunit "XQ Log"; begin L.Add('reason ' + reason); if reason = '' then currXMLport.Skip(); end;
                    }
                    textelement(Scheme) { XmlName = 'TaxScheme'; textelement(schemeid) { XmlName = 'ID'; } }
                }
                trigger OnAfterGetRecord()
                var L: Codeunit "XQ Log";
                begin
                    L.Add('agr sub ' + Format(sub.Number));
                    if sub.Number > 2 then currXMLport.Break();
                    catid := 'S' + Format(sub.Number);
                    pct := '25';
                    reason := '';
                    if sub.Number = 2 then reason := Why;
                    schemeid := 'VAT';
                end;
            }
            textelement(After) { }
            trigger OnAfterGetRecord() begin After := 'a'; end;
        }
    }
    var Why: Text;
    procedure SetWhy(W: Text) begin Why := W; end;
}
codeunit 69984 C
{
    local procedure Export(W: Text): Text
    var B: Record "XQ Blob" temporary; X: Xmlport "XQ Tax"; OutS: OutStream; InS: InStream; T: Text; Line: Text; L: Codeunit "XQ Log"; N: Record "XQ Num"; i: Integer;
    begin
        for i := 1 to 3 do begin N.Number := i; if N.Insert() then; end;
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        X.SetWhy(W);
        X.SetDestination(OutS);
        X.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line.Trim(); end;
        exit(T + '|' + L.Take());
    end;
    procedure WithReason(): Text begin exit(Export('exempt')); end;
    procedure NoReason(): Text begin exit(Export('')); end;
}
"#;

#[test]
fn a_container_without_a_trigger_below_a_table_element_runs_its_leaf_triggers_and_marks_its_assumption() {
    take_hits();
    let out = text(run(SRC, "WithReason"));
    assert!(
        out.contains(
            "<TaxSubtotal><TaxCategory><ID>S1</ID><Percent>25</Percent><TaxScheme><ID>VAT</ID></TaxScheme></TaxCategory></TaxSubtotal>\
             <TaxSubtotal><TaxCategory><ID>S2</ID><Percent>25</Percent><TaxExemptionReason>exempt</TaxExemptionReason>\
             <TaxScheme><ID>VAT</ID></TaxScheme></TaxCategory></TaxSubtotal><After>a</After></Invoice>"
        ),
        "{out}"
    );
    assert!(
        out.ends_with("|agr sub 1;reason ;agr sub 2;reason exempt;agr sub 3;"),
        "{out}"
    );
    assert!(take_hits().contains(&Assumption::XmlportLeafTriggersTreeInTable));
    let out = text(run(SRC, "NoReason"));
    assert!(!out.contains("TaxExemptionReason"), "{out}");
}
