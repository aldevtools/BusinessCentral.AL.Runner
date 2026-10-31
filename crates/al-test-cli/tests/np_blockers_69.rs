//! Gaps a partner app hit (np-blockers-69). Each test names the corpus pin it relies on (corpus PR 896, writer Y9, branch
//! al-runtime-rs-staging-y9; the pinned corpus does not hold it yet).

use al_testkit::outcomes_in;

fn outcome(o: &[(String, al_testrun::Outcome)], n: &str) -> String {
    format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x))
}

// The ASSIGNMENT of UpdLock or RepeatableRead opens the write transaction, the read does not
// (misc/lt-np4-probes/Np4Misc.al Y9_UpdLock_SetBeforeCommit_FirstRead, Y9_RepeatableRead_SetAfterCommit_FirstRead,
// Y9_ReadIsolation_ValueAcrossCommit, Y9_ReadIsolation_SetAgainAfterCommit).
const ISOLATION: &str = r#"
table 50690 "N69 Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } }
    keys { key(PK; Id) { } }
}
codeunit 50691 Noop { trigger OnRun() begin end; }
codeunit 50692 T
{
    Subtype = Test;
    [Test] procedure UpdLockSetBeforeCommitThenGet()
    var Row: Record "N69 Row";
    begin
        Seed();
        Row.ReadIsolation := IsolationLevel::UpdLock;
        Commit();
        if not Row.Get(2) then Error('no row');
        if IsInWriteTransaction() then Error('write transaction after the read');
        if not Codeunit.Run(Codeunit::Noop) then Error('guarded run failed: %1', GetLastErrorText());
    end;
    [Test] procedure RepeatableReadSetBeforeCommitThenCount()
    var Row: Record "N69 Row";
    begin
        Seed();
        Row.ReadIsolation := IsolationLevel::RepeatableRead;
        Commit();
        if Row.Count() <> 2 then Error('count');
        if IsInWriteTransaction() then Error('write transaction after the read');
    end;
    [Test] procedure RepeatableReadSetAfterCommitOpensTransaction()
    var Row: Record "N69 Row";
    begin
        Seed();
        Row.ReadIsolation := IsolationLevel::RepeatableRead;
        if not IsInWriteTransaction() then Error('no write transaction after the assignment');
        asserterror if Codeunit.Run(Codeunit::Noop) then Error('ran');
        if GetLastErrorCode() <> 'DB:Ok' then Error('code %1', GetLastErrorCode());
    end;
    [Test] procedure SameLevelAgainDifferentLevelOtherRecord()
    var Locked: Record "N69 Row"; Fresh: Record "N69 Row";
    begin
        Seed();
        Locked.ReadIsolation := IsolationLevel::UpdLock;
        Commit();
        Locked.ReadIsolation := IsolationLevel::UpdLock;
        if IsInWriteTransaction() then Error('same level opened it');
        Locked.ReadIsolation := IsolationLevel::RepeatableRead;
        if not IsInWriteTransaction() then Error('different level did not open it');
        Commit();
        Fresh.ReadIsolation := IsolationLevel::RepeatableRead;
        if not IsInWriteTransaction() then Error('other record did not open it');
        Commit();
        Fresh.ReadIsolation := IsolationLevel::ReadCommitted;
        if IsInWriteTransaction() then Error('ReadCommitted opened it');
    end;
    [Test] procedure FindFirstUnderIsolationFromEarlierTransactionIsRefused()
    var Row: Record "N69 Row";
    begin
        Seed();
        Row.ReadIsolation := IsolationLevel::UpdLock;
        Commit();
        Row.FindFirst();
        if IsInWriteTransaction() then Error('write transaction');
    end;
    local procedure Seed()
    var Row: Record "N69 Row";
    begin
        Row.DeleteAll();
        Row.Id := 1; Row.Insert();
        Row.Id := 2; Row.Insert();
        Commit();
    end;
}
"#;

#[test]
fn the_assignment_of_a_locking_read_isolation_opens_the_write_transaction_not_the_read() {
    let o = outcomes_in(ISOLATION, "T");
    for t in [
        "UpdLockSetBeforeCommitThenGet",
        "RepeatableReadSetBeforeCommitThenCount",
        "RepeatableReadSetAfterCommitOpensTransaction",
        "SameLevelAgainDifferentLevelOtherRecord",
    ] {
        assert_eq!(outcome(&o, t), "Some(Pass)", "{t}");
    }
    let g = outcome(&o, "FindFirstUnderIsolationFromEarlierTransactionIsRefused");
    assert!(g.contains("NotRun"), "{g}");
}

// Error('') in a control's OnValidate on a ListPart inside a Card ends SetValue quietly (Np4EmptyError.al
// Y9_EE_ControlValidate_ListPartInCard, corpus PR 896): the test goes on, the control shows the old value.
const EMPTY_ERROR: &str = r#"
table 50693 "N69 Head" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 50694 "N69 Line" { fields { field(1; HeadId; Integer) { } field(2; LineNo; Integer) { } field(3; Name; Text[30]) { } } keys { key(PK; HeadId, LineNo) { } } }
page 50695 "N69 Lines"
{
    PageType = ListPart; SourceTable = "N69 Line"; UsageCategory = None;
    layout { area(Content) { repeater(Rows) {
        field(LineNo; Rec.LineNo) { }
        field(Name; Rec.Name) { trigger OnValidate() begin if Rec.Name = 'empty' then Error(''); end; }
    } } }
}
page 50696 "N69 Card"
{
    PageType = Card; SourceTable = "N69 Head"; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } part(Lines; "N69 Lines") { SubPageLink = HeadId = field(Id); } } }
}
codeunit 50697 T
{
    Subtype = Test;
    [Test] procedure ListPartInCard()
    var Head: Record "N69 Head"; Line: Record "N69 Line"; Host: TestPage "N69 Card";
    begin
        Head.Id := 1; Head.Insert();
        Line.HeadId := 1; Line.LineNo := 10000; Line.Name := 'seed'; Line.Insert();
        Commit();
        Host.OpenEdit();
        Host.GoToRecord(Head);
        Host.Lines.First();
        Host.Lines.Name.SetValue('empty');
        if GetLastErrorCode() <> 'NSValidateField:Dialog' then Error('code %1', GetLastErrorCode());
        if Host.Lines.Name.Value() <> 'seed' then Error('value %1', Host.Lines.Name.Value());
        Host.Close();
        Line.Get(1, 10000);
        if Line.Name <> 'seed' then Error('stored %1', Line.Name);
    end;
}
"#;

#[test]
fn an_empty_error_out_of_a_list_part_control_validate_in_a_card_ends_set_value_quietly() {
    let o = outcomes_in(EMPTY_ERROR, "T");
    assert_eq!(outcome(&o, "ListPartInCard"), "Some(Pass)");
}

// OK on the lookup list of a static Page.RunModal(Id, Rec) that shows no row fails with TestActionNotFound, and the record
// comes back without its filters (errors/lt-ms-refusals-129/Y9LookupNoRow.al Y9_Lookup_RunModalIdRec_FilteredToNoRow_Ok,
// corpus PR 896).
const LOOKUP: &str = r#"
table 50698 "N69 Item" { fields { field(1; Id; Integer) { } field(2; Grp; Code[10]) { } } keys { key(PK; Id) { } } }
page 50699 "N69 Item List"
{
    PageType = List; SourceTable = "N69 Item"; UsageCategory = None;
    layout { area(Content) { repeater(G) { field(Id; Rec.Id) { } field(Grp; Rec.Grp) { } } } }
}
codeunit 50700 T
{
    Subtype = Test;
    [ModalPageHandler] procedure OkHandler(var P: TestPage "N69 Item List") begin P.OK().Invoke(); end;
    [Test] [HandlerFunctions('OkHandler')] procedure FilteredToNoRow()
    var Item: Record "N69 Item"; Row: Record "N69 Item"; Result: Action;
    begin
        Item.Id := 1; Item.Grp := 'A'; Item.Insert(); Commit();
        Row.SetRange(Grp, 'NONE');
        asserterror Result := Page.RunModal(Page::"N69 Item List", Row);
        if GetLastErrorText() <> 'The built-in action = OK is not found on the page.' then Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'TestActionNotFound' then Error('code %1', GetLastErrorCode());
        if Row.GetFilters() <> '' then Error('filters %1', Row.GetFilters());
    end;
}
"#;

#[test]
fn ok_on_a_static_lookup_that_shows_no_row_is_not_found() {
    let o = outcomes_in(LOOKUP, "T");
    assert_eq!(outcome(&o, "FilteredToNoRow"), "Some(Pass)");
}

// XmlPort import white space: without PreserveWhiteSpace each run is one space and the ends are trimmed, with it the value is
// kept; a value of spaces only is empty (Np4XmlPort.al Y9_XmlPort_Import_*, corpus PR 896). Export: LF, CR LF and a lone CR
// in a value are written as CR LF (Y9_XmlPort_Export_LineFeed_Plain).
const XMLPORT: &str = r#"
table 50701 "N69 XRow" { fields { field(1; Id; Integer) { } field(2; Txt; Text[50]) { } } keys { key(PK; Id) { } } }
table 50702 "N69 Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 50703 "N69 Plain"
{
    Direction = Import; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema { textelement(Root) { textelement(Note) { } tableelement(Row; "N69 XRow") { UseTemporary = true; fieldelement(Txt; Row.Txt) { }
        trigger OnBeforeInsertRecord() begin NextId += 1; Row.Id := NextId; end; } } }
    var NextId: Integer;
    procedure Result(): Text var S: Text;
    begin S := '[' + Note + ']'; if Row.FindSet() then repeat S += '[' + Row.Txt + ']'; until Row.Next() = 0; exit(S); end;
}
xmlport 50704 "N69 Keep"
{
    Direction = Import; Format = Xml; Encoding = UTF8; UseRequestPage = false; PreserveWhiteSpace = true;
    DefaultNamespace = 'urn:n69'; UseDefaultNamespace = true;
    schema { textelement(Root) { textelement(Note) { } tableelement(Row; "N69 XRow") { UseTemporary = true; fieldelement(Txt; Row.Txt) { }
        trigger OnBeforeInsertRecord() begin NextId += 1; Row.Id := NextId; end; } } }
    var NextId: Integer;
    procedure Result(): Text var S: Text;
    begin S := '[' + Note + ']'; if Row.FindSet() then repeat S += '[' + Row.Txt + ']'; until Row.Next() = 0; exit(S); end;
}
xmlport 50705 "N69 Lf"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema { textelement(Root) { textelement(Note) { trigger OnBeforePassVariable() begin Note := NoteText; end; } } }
    var NoteText: Text;
    procedure Seed(T: Text) begin NoteText := T; end;
}
codeunit 50706 T
{
    Subtype = Test;
    local procedure Source(var B: Record "N69 Blob"; Xml: Text; var InS: InStream)
    var OutS: OutStream;
    begin
        B.Id := 1; B.Data.CreateOutStream(OutS, TextEncoding::UTF8); OutS.WriteText(Xml); B.Insert();
        B.CalcFields(Data); B.Data.CreateInStream(InS, TextEncoding::UTF8);
    end;
    local procedure Doc(Ns: Text): Text
    var Lf: Text[1];
    begin
        Lf[1] := 10;
        exit('<Root' + Ns + '>' + Lf + '  <Note>  a  b' + Lf + ' c </Note>' + Lf + '  <Row>' + Lf + '    <Txt> x' + Lf + '  y </Txt>' + Lf + '  </Row>' + Lf + '  <Row><Txt>   </Txt></Row>' + Lf + '</Root>');
    end;
    [Test] procedure ImportCollapses()
    var B: Record "N69 Blob" temporary; Port: XmlPort "N69 Plain"; InS: InStream;
    begin
        Source(B, Doc(''), InS); Port.SetSource(InS); Port.Import();
        if Port.Result() <> '[a b c][x y][]' then Error('got %1', Port.Result());
    end;
    [Test] procedure ImportPreserves()
    var B: Record "N69 Blob" temporary; Port: XmlPort "N69 Keep"; InS: InStream; Lf: Text[1];
    begin
        Lf[1] := 10;
        Source(B, Doc(' xmlns="urn:n69"'), InS); Port.SetSource(InS); Port.Import();
        if Port.Result() <> '[  a  b' + Lf + ' c ][ x' + Lf + '  y ][]' then Error('got %1', Port.Result());
    end;
    [Test] procedure ExportLineEnds()
    var B: Record "N69 Blob" temporary; Port: XmlPort "N69 Lf"; OutS: OutStream; InS: InStream; Got: Text; Line: Text; Cr: Text[1]; Lf: Text[1];
    begin
        Cr[1] := 13; Lf[1] := 10;
        Port.Seed('n1' + Lf + 'n2' + Cr + Lf + 'n3' + Cr + 'n4');
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(OutS); Port.SetDestination(OutS); Port.Export(); B.Modify();
        B.CalcFields(Data); B.Data.CreateInStream(InS);
        InS.Read(Got);
        if StrPos(Got, '<Note>n1' + Cr + Lf + 'n2' + Cr + Lf + 'n3' + Cr + Lf + 'n4</Note>') = 0 then Error('got %1', Got);
    end;
}
"#;

#[test]
fn xmlport_import_white_space_and_export_line_ends() {
    let o = outcomes_in(XMLPORT, "T");
    for t in ["ImportCollapses", "ImportPreserves", "ExportLineEnds"] {
        assert_eq!(outcome(&o, t), "Some(Pass)", "{t}");
    }
}
