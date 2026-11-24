//! The refusals next to the XmlPort import shapes of al-runner issue 8877 (corpus PR 1550, XpAutoUpdateAssign.al): the
//! linked table element whose SourceTableView is one `const` filter (`xmlport-linked-view-const`), the parent stored again
//! after its linked child rows (`xmlport-stored-again-assign-triggers`) and the AutoUpdate save whose triggers assign fields
//! (`xmlport-autoupdate-assign-triggers`). The measured view shape runs, flagged; each neighbor below is refused with its own
//! text. The same shapes run on BC in the neighbor app `tests/neighbors/fix-8877-xmlport-autoupdate`.

use al_builtins::assumptions::Assumption;
use al_testrun::Outcome;

const SRC: &str = r#"
table 50100 "RX Src" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50101 "RX Suite" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 50102 "RX Eval" { fields { field(1; SuiteCode; Code[20]) { } field(2; LineNo; Integer) { } field(3; Name; Text[30]) { } } keys { key(PK; SuiteCode, LineNo, Name) { Clustered = true; } } }
table 50103 "RX EvalIns" { fields { field(1; SuiteCode; Code[20]) { } field(2; LineNo; Integer) { } field(3; Name; Text[30]) { } } keys { key(PK; SuiteCode, LineNo, Name) { Clustered = true; } }
    trigger OnInsert() begin LineNo := 7; end; }
table 50104 "RX Par" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Stamp; Text[30]) { } field(4; Extra; Text[30]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 50105 "RX Chi" { fields { field(1; Id; Integer) { } field(2; ParentId; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }

xmlport 50110 "RX View"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Suite; "RX Suite") { fieldattribute(Code; Suite.Code) { }
        tableelement(SuiteEval; "RX Eval") { LinkTable = Suite; LinkFields = SuiteCode = field(Code); MinOccurs = Zero; XmlName = 'Evaluator';
            SourceTableView = where(LineNo = const(0)); fieldattribute(Name; SuiteEval.Name) { } } } } }
}
xmlport 50111 "RX View Other"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Suite; "RX Suite") { fieldattribute(Code; Suite.Code) { }
        tableelement(SuiteEval; "RX Eval") { LinkTable = Suite; LinkFields = SuiteCode = field(Code); MinOccurs = Zero; XmlName = 'Evaluator';
            SourceTableView = where(LineNo = const(5)); fieldattribute(Name; SuiteEval.Name) { } } } } }
}
xmlport 50112 "RX View OnInsert"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Suite; "RX Suite") { fieldattribute(Code; Suite.Code) { }
        tableelement(SuiteEval; "RX EvalIns") { LinkTable = Suite; LinkFields = SuiteCode = field(Code); MinOccurs = Zero; XmlName = 'Evaluator';
            SourceTableView = where(LineNo = const(0)); fieldattribute(Name; SuiteEval.Name) { } } } } }
}
xmlport 50113 "RX View Skip"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Suite; "RX Suite") { fieldattribute(Code; Suite.Code) { }
        tableelement(SuiteEval; "RX Eval") { LinkTable = Suite; LinkFields = SuiteCode = field(Code); MinOccurs = Zero; XmlName = 'Evaluator';
            SourceTableView = where(LineNo = const(0)); fieldattribute(Name; SuiteEval.Name) { }
            trigger OnAfterInitRecord() begin currXMLport.Skip(); end; } } } }
}
xmlport 50114 "RX Stale"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Par; "RX Par") { XmlName = 'P';
        fieldelement(Id; Par.Id) { trigger OnAfterAssignField() begin Par.Stamp := 'trig'; end; }
        fieldelement(Name; Par.Name) { }
        tableelement(Chi; "RX Chi") { XmlName = 'C'; LinkTable = Par; LinkFields = ParentId = field(Id); MinOccurs = Zero;
            fieldelement(CId; Chi.Id) { }
            trigger OnAfterInitRecord() begin Par.Extra := 'child'; end; } } } }
}
xmlport 50115 "RX UpdBm"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Par; "RX Par") { XmlName = 'P'; AutoUpdate = true;
        fieldelement(Id; Par.Id) { trigger OnAfterAssignField() begin Par.Stamp := 'trig'; end; }
        fieldelement(Name; Par.Name) { MinOccurs = Zero; }
        trigger OnBeforeModifyRecord() begin Par.Extra := 'bm'; end; } } }
}

xmlport 50116 "RX Skipped"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Par; "RX Par") { XmlName = 'P';
        fieldelement(Id; Par.Id) { trigger OnAfterAssignField() begin Par.Stamp := 'trig'; end; }
        fieldelement(Name; Par.Name) { }
        tableelement(Chi; "RX Chi") { XmlName = 'C'; LinkTable = Par; LinkFields = ParentId = field(Id); MinOccurs = Zero;
            fieldelement(CId; Chi.Id) { } }
        trigger OnBeforeInsertRecord() var Other: Record "RX Par"; begin if Other.Get(Par.Id) then currXMLport.Skip(); end; } } }
}
xmlport 50117 "RX View Read"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Suite; "RX Suite") { fieldattribute(Code; Suite.Code) { }
        tableelement(SuiteEval; "RX Eval") { LinkTable = Suite; LinkFields = SuiteCode = field(Code); MinOccurs = Zero; XmlName = 'Evaluator';
            SourceTableView = where(LineNo = const(0)); fieldattribute(Name; SuiteEval.Name) { } } } } }
    trigger OnPostXmlPort() begin Seen := SuiteEval.GetFilters(); end;
    var Seen: Text;
}

codeunit 50149 H
{
    procedure Run(PortId: Integer; Xml: Text)
    var Src: Record "RX Src"; InS: InStream; OutS: OutStream;
    begin
        Src.DeleteAll(); Src.Id := 1; Src.Data.CreateOutStream(OutS, TextEncoding::UTF8); OutS.WriteText(Xml); Src.Insert();
        Src.CalcFields(Data); Src.Data.CreateInStream(InS, TextEncoding::UTF8); Xmlport.Import(PortId, InS);
    end;

    procedure CheckEval()
    var E: Record "RX Eval";
    begin if not E.Get('A', 0, 'e0') then Error('no evaluator row'); end;

    procedure SeedPar()
    var Par: Record "RX Par";
    begin Par.DeleteAll(); Par.Id := 1; Par.Name := 'old'; Par.Stamp := 's0'; Par.Extra := 'e0'; Par.Insert(); end;

    // The row a first import of "RX Skipped" stores: every field the document does not carry as the record holds it.
    procedure SeedSkipped()
    var Par: Record "RX Par";
    begin Par.DeleteAll(); Par.Id := 1; Par.Name := 'old'; Par.Stamp := 'trig'; Par.Insert(); end;
}
"#;

const SUITE: &str = r#"'<Root><Suite Code="A"><Evaluator Name="e0"/></Suite></Root>'"#;

/// Each case runs in a codeunit of its own: a case refused while it ran makes the later cases of its codeunit untrusted.
fn outcome(body: &str) -> (Outcome, Vec<Assumption>) {
    let src = format!(
        "{SRC}\ncodeunit 50151 TR {{ Subtype = Test; var H: Codeunit H; [Test] procedure T() begin {body} end; }}\n"
    );
    let mut got = al_testkit::outcomes_and_marks_in(&src, "TR");
    assert_eq!(got.len(), 1, "{got:#?}");
    let (_, o, marks) = got.remove(0);
    (o, marks)
}

fn refused_with(body: &str, text: &str) {
    match outcome(body) {
        (Outcome::NotRun(m), _) if m.starts_with(text) => {}
        other => panic!("expected a refusal starting with {text:?}, got {other:#?}"),
    }
}

#[test]
fn linked_view_const_measured_shape_runs_flagged() {
    let (o, marks) = outcome(&format!("H.Run(Xmlport::\"RX View\", {SUITE}); H.CheckEval();"));
    assert_eq!(o, Outcome::Pass);
    assert!(marks.contains(&Assumption::XmlportLinkedViewConst), "{marks:?}");
}

#[test]
fn linked_view_const_other_value_at_the_save_refuses() {
    refused_with(
        &format!("H.Run(Xmlport::\"RX View Other\", {SUITE});"),
        "XmlPort import of a linked table element whose SourceTableView const filter's field holds another value",
    );
}

#[test]
fn linked_view_const_on_insert_changing_the_field_refuses() {
    refused_with(
        &format!("H.Run(Xmlport::\"RX View OnInsert\", {SUITE});"),
        "XmlPort import of a linked table element whose SourceTableView const filter's field holds another value",
    );
}

#[test]
fn linked_view_const_skipped_by_its_trigger_refuses() {
    refused_with(
        &format!("H.Run(Xmlport::\"RX View Skip\", {SUITE});"),
        "XmlPort import of a linked table element whose SourceTableView has a const filter, skipped by its trigger",
    );
}

#[test]
fn stored_again_record_differs_from_the_stored_row_refuses() {
    refused_with(
        "H.SeedPar(); H.Run(Xmlport::\"RX Stale\", '<Root><P><Id>2</Id><Name>two</Name><C><CId>10</CId></C></P></Root>');",
        "XmlPort import of a table element stored again after its linked child rows, whose record differs",
    );
}

#[test]
fn autoupdate_before_modify_changing_the_row_refuses() {
    refused_with(
        "H.SeedPar(); H.Run(Xmlport::\"RX UpdBm\", '<Root><P><Id>1</Id><Name>new</Name></P></Root>');",
        "AutoUpdate with triggers that assign fields and an OnBeforeModifyRecord that changes the row",
    );
}

#[test]
fn stored_again_after_a_first_save_that_stored_no_row_refuses() {
    refused_with(
        "H.SeedSkipped(); H.Run(Xmlport::\"RX Skipped\", '<Root><P><Id>1</Id><Name>new</Name><C><CId>10</CId></C></P></Root>');",
        "XmlPort import of a table element stored again after its linked child rows, whose first save stored no row",
    );
}

#[test]
fn linked_view_const_read_by_another_routine_of_the_port_refuses() {
    refused_with(
        &format!("H.Run(Xmlport::\"RX View Read\", {SUITE});"),
        "XmlPort import: sourcetableview with a filter on the linked tableelement",
    );
}
