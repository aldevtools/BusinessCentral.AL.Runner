//! An XmlPort field element bound to a FlowField exports the calculated value (Base Application's "Import / Export Workflow"
//! exports "Workflow Step Argument"."Response Option Group", a Lookup FlowField; 76 Tests-Workflow tests were refused for the
//! binding). Not pinned by the corpus: assumption xmlport-exports-flowfield-calculated. A FlowFilter stays refused.

use al_sema::workspace::AppSource;
use al_testrun::runtime::{CorpusRuntime, RunOptions, SuiteRun};
use al_testrun::{CaseId, Outcome};

const SRC: &str = r#"
table 50530 "XFF Child" { fields { field(1; Id; Integer) { } field(2; PId; Integer) { } field(3; Amount; Decimal) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50531 "XFF Parent" { fields { field(1; Id; Integer) { } field(2; Total; Decimal) { FieldClass = FlowField; CalcFormula = sum("XFF Child".Amount where(PId = field(Id))); } field(3; Sink; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
xmlport 50530 "XFF Port" { Direction = Export; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(P; "XFF Parent") { fieldelement(Id; P.Id) { } fieldelement(Total; P.Total) { } } } } }
codeunit 50530 "T"
{
    Subtype = Test;
    [Test]
    procedure ExportsTheCalculatedValue()
    var Parent: Record "XFF Parent"; Child: Record "XFF Child"; Holder: Record "XFF Parent"; OutS: OutStream; InS: InStream; Line: Text; All: Text;
    begin
        Parent.Id := 1; Parent.Insert();
        Child.Id := 1; Child.PId := 1; Child.Amount := 3; Child.Insert();
        Child.Id := 2; Child.PId := 1; Child.Amount := 4; Child.Insert();
        Holder.Id := 99; Holder.Insert();
        Holder.Sink.CreateOutStream(OutS, TextEncoding::UTF16);
        Xmlport.Export(Xmlport::"XFF Port", OutS);
        Holder.Modify();
        Holder.CalcFields(Sink);
        Holder.Sink.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS do begin InS.ReadText(Line); All += Line; end;
        if StrPos(All, '<Total>7.00</Total>') = 0 then Error('export was %1', All);
    end;
}
"#;

#[test]
fn an_xmlport_exports_the_calculated_value_of_a_flowfield() {
    let rt = CorpusRuntime::run_suites_with(
        vec![AppSource {
            id: "main".into(),
            name: "main".into(),
            dependencies: vec![],
            files: vec![("main.al".into(), SRC.into())],
        }],
        vec![al_sema::AppKind::Workspace],
        &[],
        &Default::default(),
        None,
        Default::default(),
        &[SuiteRun::everything()],
        &RunOptions::default(),
    );
    let o = rt.first_outcome(&CaseId {
        codeunit: 50530,
        name: "ExportsTheCalculatedValue".into(),
    });
    assert_eq!(o, Outcome::Pass, "{o:?}");
}
