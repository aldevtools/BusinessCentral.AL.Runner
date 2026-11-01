//! Behaviour of queries and XmlPorts pinned by probe tests that are not in the corpus submodule yet
//! (corpus: query/TestQueryOuterJoinDescendingSaveAsContracts.al,
//! xmlport/TestXmlPortTriggersReplaceViewContracts.al, xmlport/TestXmlPortTextFormatsContracts.al).
//! AL source, compiled and run as a test codeunit. Each test names the probe it mirrors.

use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 64000 "DF Parent" { fields { field(1; "Entry No."; Integer) { } } keys { key(PK; "Entry No.") { } } }
table 64001 "DF Child"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Parent Entry No."; Integer) { } field(3; Amount; Decimal) { } }
    keys { key(PK; "Entry No.") { } }
}
table 64002 "DF Universal"
{
    fields
    {
        field(1; "Entry No."; Integer) { } field(2; "Integer Field"; Integer) { } field(3; "Text Field"; Text[100]) { }
        field(4; "Name Field"; Text[50]) { } field(5; "Decimal Field"; Decimal) { } field(6; "Date Field"; Date) { }
        field(7; Data; Blob) { }
    }
    keys { key(PK; "Entry No.") { } key(Text; "Text Field") { } }
}
table 64003 "DF Blob" { fields { field(1; Code; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 64004 "DF Kid"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Parent Entry No."; Integer) { } field(3; Code; Code[10]) { } }
    keys { key(PK; "Entry No.") { } }
}
codeunit 64020 "DF Log"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(Entry: Text) begin Log += Entry + ';'; end;
    procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
}
xmlport 64030 "DF XP Trig"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(U; "DF Universal")
            {
                fieldattribute(Attr; U."Name Field") { }
                fieldelement(EntryNo; U."Entry No.") { }
                fieldelement(Txt; U."Text Field")
                {
                    MinOccurs = Zero;
                    trigger OnAfterAssignField() begin XmlLog.Add('Txt.AfterAssign(' + U."Text Field" + ')'); end;
                    trigger OnBeforePassField() begin XmlLog.Add('Txt.BeforePass'); end;
                }
                fieldelement(Intg; U."Integer Field")
                {
                    MinOccurs = Zero;
                    trigger OnAfterAssignField() begin XmlLog.Add('Int.AfterAssign'); end;
                }
                trigger OnAfterInitRecord() begin XmlLog.Add('AfterInitRecord(entry=' + Format(U."Entry No.") + ',txt=' + U."Text Field" + ')'); end;
                trigger OnBeforeInsertRecord() begin XmlLog.Add('BeforeInsert(' + Format(U."Entry No.") + ')'); end;
                trigger OnAfterInsertRecord() begin XmlLog.Add('AfterInsert'); end;
            }
        }
    }
    var XmlLog: Codeunit "DF Log";
}
xmlport 64031 "DF XP Replace"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(U; "DF Universal") { AutoReplace = true; fieldelement(EntryNo; U."Entry No.") { } fieldelement(Txt; U."Text Field") { MinOccurs = Zero; } } } }
}
xmlport 64032 "DF XP Update"
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(U; "DF Universal") { AutoUpdate = true; fieldelement(EntryNo; U."Entry No.") { } fieldelement(Txt; U."Text Field") { MinOccurs = Zero; } } } }
}
xmlport 64033 "DF XP VT"
{
    Direction = Export; Format = VariableText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(U; "DF Universal") { fieldelement(EntryNo; U."Entry No.") { } fieldelement(Txt; U."Text Field") { } fieldelement(Dec; U."Decimal Field") { } } } }
}
xmlport 64034 "DF XP Nested"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(P; "DF Parent")
            {
                fieldelement(PNo; P."Entry No.") { }
                tableelement(C; "DF Kid")
                {
                    LinkFields = "Parent Entry No." = field("Entry No.");
                    LinkTable = P;
                    fieldelement(CNo; C."Entry No.") { }
                    fieldelement(CCode; C.Code) { }
                }
            }
        }
    }
}
xmlport 64035 "DF XP VT Default"
{
    Direction = Both; Format = VariableText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(U; "DF Universal") { fieldelement(EntryNo; U."Entry No.") { } fieldelement(Txt; U."Text Field") { } fieldelement(Dec; U."Decimal Field") { } fieldelement(Dt; U."Date Field") { } } } }
}
xmlport 64036 "DF XP VT Custom"
{
    Direction = Both; Format = VariableText; FieldSeparator = ';'; FieldDelimiter = '|'; RecordSeparator = '<LF>'; TextEncoding = UTF8; UseRequestPage = false;
    schema { textelement(Root) { tableelement(U; "DF Universal") { fieldelement(EntryNo; U."Entry No.") { } fieldelement(Txt; U."Text Field") { } } } }
}
xmlport 64037 "DF XP Fixed"
{
    Direction = Both; Format = FixedText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(U; "DF Universal") { fieldelement(EntryNo; U."Entry No.") { Width = 5; } fieldelement(Txt; U."Text Field") { Width = 8; } fieldelement(Dec; U."Decimal Field") { Width = 10; } } } }
}
query 64010 "DF Q Desc"
{
    QueryType = Normal;
    OrderBy = descending(IntV);
    elements { dataitem(U; "DF Universal") { column(IntV; "Integer Field") { } column(Entry; "Entry No.") { } } }
}
query 64011 "DF Q Group Desc"
{
    QueryType = Normal;
    OrderBy = descending(Grp);
    elements { dataitem(U; "DF Universal") { column(Grp; "Integer Field") { } column(Cnt) { Method = Count; } column(Mx; "Entry No.") { Method = Max; } } }
}
query 64012 "DF Q Outer"
{
    QueryType = Normal;
    OrderBy = ascending(PNo);
    elements
    {
        dataitem(P; "DF Parent")
        {
            column(PNo; "Entry No.") { }
            dataitem(C; "DF Child")
            {
                DataItemLink = "Parent Entry No." = P."Entry No.";
                SqlJoinType = LeftOuterJoin;
                column(CNo; "Entry No.") { }
                column(CAmt; Amount) { }
            }
        }
    }
}
query 64013 "DF Q Outer Sum"
{
    QueryType = Normal;
    OrderBy = ascending(PNo);
    elements
    {
        dataitem(P; "DF Parent")
        {
            column(PNo; "Entry No.") { }
            dataitem(C; "DF Child")
            {
                DataItemLink = "Parent Entry No." = P."Entry No.";
                SqlJoinType = LeftOuterJoin;
                column(SumAmt; Amount) { Method = Sum; }
                column(CntC) { Method = Count; }
            }
        }
    }
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 64100 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

fn passes(tests: &str) {
    let o = outcomes(tests);
    assert!(!o.is_empty());
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}");
    }
}

const HELPERS: &str = "procedure Check(Expected: Text; Actual: Text) begin if Expected <> Actual then Error('expected <%1> got <%2>', Expected, Actual); end;
procedure InsU(No: Integer; Int: Integer) var U: Record \"DF Universal\"; begin U.Init(); U.\"Entry No.\" := No; U.\"Integer Field\" := Int; U.Insert(); end;
procedure Seed() var P: Record \"DF Parent\"; C: Record \"DF Child\"; begin
    P.\"Entry No.\" := 1; P.Insert(); P.\"Entry No.\" := 2; P.Insert(); P.\"Entry No.\" := 3; P.Insert();
    C.\"Entry No.\" := 10; C.\"Parent Entry No.\" := 1; C.Amount := 5; C.Insert();
    C.\"Entry No.\" := 11; C.Amount := 7; C.Insert();
    C.\"Entry No.\" := 12; C.\"Parent Entry No.\" := 3; C.Amount := 1; C.Insert();
    InsU(1, 2); InsU(2, 1); InsU(3, 2); InsU(4, 1); end;";

// query/TestQueryOuterJoinDescendingSaveAsContracts.al:Query_DescendingOrderBy_TiesStayInAscendingPrimaryKeyOrder
#[test]
fn descending_order_by_keeps_ties_in_ascending_primary_key_order_and_groups_descend() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Desc() var Q: Query \"DF Q Desc\"; G: Query \"DF Q Group Desc\"; Rows: Text; begin
            Seed();
            Q.Open(); while Q.Read() do Rows += Format(Q.IntV) + '/' + Format(Q.Entry) + ' '; Q.Close();
            Check('2/1 2/3 1/2 1/4 ', Rows);
            Rows := '';
            G.Open(); while G.Read() do Rows += Format(G.Grp) + 'x' + Format(G.Cnt) + 'max' + Format(G.Mx) + ' '; G.Close();
            Check('2x2max3 1x2max4 ', Rows); end;"
    ));
}

// query/TestQueryOuterJoinDescendingSaveAsContracts.al:Query_LeftOuterJoin_RowsFiltersAndAggregates
#[test]
fn left_outer_join_rows_filters_and_aggregates() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Outer() var Q: Query \"DF Q Outer\"; S: Query \"DF Q Outer Sum\"; Rows: Text; begin
            Seed();
            Q.Open(); while Q.Read() do Rows += Format(Q.PNo) + '/' + Format(Q.CNo) + '/' + Format(Q.CAmt) + ' '; Q.Close();
            Check('1/10/5 1/11/7 2/0/0 3/12/1 ', Rows);
            Rows := ''; Q.SetRange(CNo, 10);
            Q.Open(); while Q.Read() do Rows += Format(Q.PNo) + '/' + Format(Q.CNo) + ' '; Q.Close();
            Check('1/10 ', Rows);
            Rows := ''; Q.SetRange(CNo); Q.SetRange(CAmt, 0);
            Q.Open(); while Q.Read() do Rows += Format(Q.PNo) + '/' + Format(Q.CNo) + ' '; Q.Close();
            Check('2/0 ', Rows);
            Rows := '';
            S.Open(); while S.Read() do Rows += Format(S.PNo) + '/sum' + Format(S.SumAmt) + '/cnt' + Format(S.CntC) + ' '; S.Close();
            Check('1/sum12/cnt2 2/sum0/cnt0 3/sum1/cnt1 ', Rows); end;"
    ));
}

// query/TestQueryOuterJoinDescendingSaveAsContracts.al:Query_SumOfAnIntegerColumn_AndARowInsertedBeforeTheFirstRead
#[test]
fn open_does_not_run_the_query() {
    passes(&format!(
        "{HELPERS}
        [Test]
        procedure Lazy() var G: Query \"DF Q Group Desc\"; Rows: Text; begin
            Seed();
            G.Open();
            InsU(99, 1);
            while G.Read() do Rows += Format(G.Grp) + 'x' + Format(G.Cnt) + ' '; G.Close();
            Check('2x2 1x3 ', Rows); end;"
    ));
}

const STREAM_TEXT: &str =
    "procedure StreamText(var B: Record \"DF Blob\"): Text var InStr: InStream; Line: Text; Result: Text; begin
    B.CalcFields(Data); B.Data.CreateInStream(InStr, TextEncoding::UTF8);
    while not InStr.EOS() do begin InStr.ReadText(Line); Result += Line.Trim(); end; exit(Result); end;";

// query/TestQueryOuterJoinDescendingSaveAsContracts.al:Query_SaveAsXml_And_SaveAsJson_Output
#[test]
fn save_as_xml_writes_a_dataset_and_save_as_json_an_array_of_objects() {
    passes(&format!(
        "{HELPERS} {STREAM_TEXT}
        [Test]
        procedure SaveAs() var Q: Query \"DF Q Desc\"; B: Record \"DF Blob\"; OutStr: OutStream; begin
            Seed();
            B.Code := 1; B.Insert(); B.Data.CreateOutStream(OutStr);
            if not Q.SaveAsXml(OutStr) then Error('SaveAsXml');
            B.Modify(); Check('<?xml version=\"1.0\" standalone=\"yes\"?><DataSet><xs:schema id=\"NewDataSet\" xmlns=\"\" xmlns:xs=\"http://www.w3.org/2001/XMLSchema\" xmlns:msdata=\"urn:schemas-microsoft-com:xml-msdata\"><xs:element name=\"NewDataSet\" msdata:IsDataSet=\"true\" msdata:MainDataTable=\"Result\" msdata:Locale=\"\"><xs:complexType><xs:choice minOccurs=\"0\" maxOccurs=\"unbounded\"><xs:element name=\"Result\" msdata:Locale=\"\"><xs:complexType><xs:sequence><xs:element name=\"IntV\" type=\"xs:int\" minOccurs=\"0\" /><xs:element name=\"Entry\" type=\"xs:int\" minOccurs=\"0\" /></xs:sequence></xs:complexType></xs:element></xs:choice></xs:complexType></xs:element></xs:schema><Result><IntV>2</IntV><Entry>1</Entry></Result><Result><IntV>2</IntV><Entry>3</Entry></Result><Result><IntV>1</IntV><Entry>2</Entry></Result><Result><IntV>1</IntV><Entry>4</Entry></Result></DataSet>', StreamText(B));
            B.Delete(); Clear(B); B.Code := 1; B.Insert(); B.Data.CreateOutStream(OutStr);
            if not Q.SaveAsJson(OutStr) then Error('SaveAsJson');
            B.Modify(); Check('[{{\"IntV\":2,\"Entry\":1}},{{\"IntV\":2,\"Entry\":3}},{{\"IntV\":1,\"Entry\":2}},{{\"IntV\":1,\"Entry\":4}}]', StreamText(B)); end;"
    ));
}

fn passes_raw(tests: &str) {
    passes(&format!("{HELPERS} {IO} {tests}"));
}

const IO: &str = r#"procedure Feed(Xml: Text; var B: Record "DF Blob"; var InStr: InStream) var OutStr: OutStream; begin
    B.DeleteAll(); B.Code := 1; B.Insert(); B.Data.CreateOutStream(OutStr, TextEncoding::UTF8); OutStr.WriteText(Xml); B.Modify();
    B.CalcFields(Data); B.Data.CreateInStream(InStr, TextEncoding::UTF8); end;
procedure Dest(var B: Record "DF Blob"; var OutStr: OutStream) begin B.DeleteAll(); B.Code := 1; B.Insert(); B.Data.CreateOutStream(OutStr); end;
procedure HexOf(var B: Record "DF Blob"): Text var InStr: InStream; V: Byte; R: Text; D: Text; begin
    D := '0123456789ABCDEF'; B.Modify(); B.CalcFields(Data); B.Data.CreateInStream(InStr);
    while not InStr.EOS() do begin InStr.Read(V); R += CopyStr(D, (V div 16) + 1, 1) + CopyStr(D, (V mod 16) + 1, 1); end; exit(R); end;
procedure UniversalRows(): Text var U: Record "DF Universal"; R: Text; begin
    if U.FindSet() then repeat R += Format(U."Entry No.") + '|' + U."Text Field" + '|' + Format(U."Decimal Field", 0, 9) + '|' + Format(U."Date Field", 0, 9) + ' ; '; until U.Next() = 0; exit(R); end;"#;

// xmlport/TestXmlPortTriggersReplaceViewContracts.al:Import_TriggerOrder_AndEveryElementStartsFromAnEmptyRecord
#[test]
fn import_runs_the_triggers_in_order_and_every_element_starts_from_an_empty_record() {
    passes_raw(
        r#"[Test]
        procedure Trig() var B: Record "DF Blob"; I: InStream; X: XmlPort "DF XP Trig"; L: Codeunit "DF Log"; U: Record "DF Universal"; begin
            L.Take();
            Feed('<Root><U Attr="a"><EntryNo>1</EntryNo><Txt>t1</Txt><Intg>5</Intg></U><U Attr="b"><EntryNo>2</EntryNo></U></Root>', B, I);
            X.SetSource(I); X.Import();
            Check('AfterInitRecord(entry=0,txt=);Txt.AfterAssign(t1);Int.AfterAssign;BeforeInsert(1);AfterInsert;AfterInitRecord(entry=0,txt=);BeforeInsert(2);AfterInsert;', L.Take());
            U.Get(2); Check('', U."Text Field"); Check('b', U."Name Field"); end;"#,
    );
}

// :Import_ExistingKey_WithoutAutoReplace_RaisesTheDuplicateKeyError
#[test]
fn import_of_an_existing_key_raises_the_duplicate_key_error() {
    passes_raw(
        r#"[Test]
        procedure Dup() var B: Record "DF Blob"; I: InStream; X: XmlPort "DF XP Trig"; U: Record "DF Universal"; begin
            U."Entry No." := 1; U.Insert(); Commit();
            Feed('<Root><U Attr="c"><EntryNo>1</EntryNo><Txt>t1b</Txt></U></Root>', B, I);
            X.SetSource(I);
            asserterror X.Import();
            Check('The record in table DF Universal already exists. Identification fields and values: Entry No.=''1''', GetLastErrorText());
            Check('DB:RecordExists', GetLastErrorCode()); end;"#,
    );
}

// :Import_AutoReplace_ReplacesTheWholeBuffer_KeepsSystemId and :Import_AutoUpdate_ChangesOnlyTheCarriedFields_KeepsSystemId
#[test]
fn auto_replace_resets_the_other_fields_and_auto_update_keeps_them_and_both_keep_the_system_id() {
    passes_raw(
        r#"procedure SeedOne() var U: Record "DF Universal"; begin
            U.DeleteAll(); U."Entry No." := 1; U."Text Field" := 'orig'; U."Integer Field" := 42; U."Name Field" := 'nm'; U.Insert(); Commit(); end;
        [Test]
        procedure Replace() var B: Record "DF Blob"; I: InStream; X: XmlPort "DF XP Replace"; U: Record "DF Universal"; Old: Guid; begin
            SeedOne(); U.Get(1); Old := U.SystemId;
            Feed('<Root><U><EntryNo>1</EntryNo><Txt>replaced</Txt></U></Root>', B, I);
            X.SetSource(I); X.Import(); Commit();
            U.Get(1); Check('replaced', U."Text Field"); Check('0', Format(U."Integer Field")); Check('', U."Name Field");
            if U.SystemId <> Old then Error('SystemId must be kept'); end;
        [Test]
        procedure Update() var B: Record "DF Blob"; I: InStream; X: XmlPort "DF XP Update"; U: Record "DF Universal"; Old: Guid; begin
            SeedOne(); U.Get(1); Old := U.SystemId;
            Feed('<Root><U><EntryNo>1</EntryNo><Txt>updated</Txt></U></Root>', B, I);
            X.SetSource(I); X.Import(); Commit();
            U.Get(1); Check('updated', U."Text Field"); Check('42', Format(U."Integer Field")); Check('nm', U."Name Field");
            if U.SystemId <> Old then Error('SystemId must be kept'); end;"#,
    );
}

// :Export_OnBeforePassField_Runs_AndSetTableViewFiltersAndOrders
#[test]
fn export_honours_the_table_view_filter_key_and_direction() {
    passes_raw(
        r#"procedure EntryNos(var B: Record "DF Blob"): Text var I: InStream; Line: Text; R: Text; begin
            B.Modify(); B.CalcFields(Data); B.Data.CreateInStream(I, TextEncoding::UTF16);
            while not I.EOS() do begin I.ReadText(Line); if Line.Contains('EntryNo') then R += Line.Trim() + ' '; end; exit(R); end;
        [Test]
        procedure View() var B: Record "DF Blob"; O: OutStream; X: XmlPort "DF XP Trig"; U: Record "DF Universal"; L: Codeunit "DF Log"; begin
            L.Take();
            U."Entry No." := 30; U.Insert(); U."Entry No." := 10; U.Insert(); U."Entry No." := 20; U."Text Field" := 'k'; U.Insert();
            Dest(B, O); X.SetDestination(O); U.SetFilter("Entry No.", '>=20'); X.SetTableView(U); X.Export();
            Check('<EntryNo>20</EntryNo> <EntryNo>30</EntryNo> ', EntryNos(B));
            Check('Txt.BeforePass;Txt.BeforePass;', L.Take());
            Clear(X); Dest(B, O); X.SetDestination(O); Clear(U); U.SetCurrentKey("Text Field"); U.Ascending(false); X.SetTableView(U); X.Export();
            Check('<EntryNo>20</EntryNo> <EntryNo>30</EntryNo> <EntryNo>10</EntryNo> ', EntryNos(B)); end;"#,
    );
}

// :Export_VariableText_Bytes
#[test]
fn variable_text_export_quotes_every_value_and_separates_records_with_cr_lf() {
    passes_raw(
        r#"[Test]
        procedure VT() var B: Record "DF Blob"; O: OutStream; X: XmlPort "DF XP VT"; U: Record "DF Universal"; I: InStream; Line: Text; C: Text; begin
            U."Entry No." := 1; U."Text Field" := 'a,b'; U."Decimal Field" := 1234.5; U.Insert();
            U."Entry No." := 2; U."Text Field" := 'q"x'; U."Decimal Field" := 0; U.Insert();
            Dest(B, O); X.SetDestination(O); X.Export(); B.Modify(); B.CalcFields(Data); B.Data.CreateInStream(I);
            while not I.EOS() do begin I.ReadText(Line); C += Line + '|'; end;
            Check('"1","a,b","1,234.50"|"2","q"x","0.00"|', C); end;"#,
    );
}

// :Nested_Export_GroupsTheChildrenUnderTheirParent_ImportLinksOnlyTheFirstChild
#[test]
fn nested_import_links_only_the_first_child() {
    passes_raw(
        r#"[Test]
        procedure Nested() var B: Record "DF Blob"; I: InStream; X: XmlPort "DF XP Nested"; K: Record "DF Kid"; R: Text; begin
            Feed('<Root><P><PNo>5</PNo><C><CNo>50</CNo><CCode>x</CCode></C><C><CNo>51</CNo><CCode>y</CCode></C></P></Root>', B, I);
            X.SetSource(I); X.Import(); Commit();
            K.FindSet(); repeat R += Format(K."Entry No.") + '/parent=' + Format(K."Parent Entry No.") + '/' + K.Code + ' '; until K.Next() = 0;
            Check('50/parent=5/X 51/parent=0/Y ', R); end;"#,
    );
}

// xmlport/TestXmlPortTextFormatsContracts.al:VariableText_Import_QuotedUnquotedAndBlankValues,
// :VariableText_Import_DoubledQuoteInsideAQuotedValue_IsNotAnEscape
#[test]
fn variable_text_import_reads_quoted_unquoted_and_blank_values() {
    passes_raw(
        r#"[Test]
        procedure Import() var B: Record "DF Blob"; I: InStream; X: XmlPort "DF XP VT Default"; Cr: Text[1]; Lf: Text[1]; begin
            Cr[1] := 13; Lf[1] := 10;
            Feed('"1","a,b","1,234.50","03/05/24"' + Cr + Lf + '"2","plain","0",""' + Cr + Lf + '3,unquoted,5.5,01/02/24', B, I);
            X.SetSource(I); X.Import(); Commit();
            Check('1|a,b|1234.5|2024-03-05 ; 2|plain|0| ; 3|unquoted|5.5|2024-01-02 ; ', UniversalRows()); end;
        [Test]
        procedure DoubledQuote() var B: Record "DF Blob"; I: InStream; X: XmlPort "DF XP VT Default"; begin
            Feed('"2","say ""hi""","0",""', B, I);
            X.SetSource(I);
            asserterror X.Import();
            Check('The value "hi" can''t be evaluated into type Decimal.', GetLastErrorText()); end;"#,
    );
}

// :VariableText_CustomSeparators_ImportAndExportBytes
#[test]
fn variable_text_with_custom_separators_and_utf8_writes_a_byte_order_mark() {
    passes_raw(
        r#"[Test]
        procedure Custom() var B: Record "DF Blob"; I: InStream; O: OutStream; X: XmlPort "DF XP VT Custom"; E: XmlPort "DF XP VT Custom"; Lf: Text[1]; begin
            Lf[1] := 10;
            Feed('|1|;|a;b|' + Lf + '|2|;|x|', B, I);
            X.SetSource(I); X.Import(); Commit();
            Check('1|a;b|0| ; 2|x|0| ; ', UniversalRows());
            Dest(B, O); E.SetDestination(O); E.Export();
            Check('EFBBBF7C317C3B7C613B627C0A7C327C3B7C787C', HexOf(B)); end;"#,
    );
}

// :FixedText_Export_PadsAndCutsToWidth_Import_ReadsByWidth
#[test]
fn fixed_text_pads_and_cuts_to_the_width_and_reads_by_width() {
    passes_raw(
        r#"[Test]
        procedure Fixed() var B: Record "DF Blob"; I: InStream; O: OutStream; E: XmlPort "DF XP Fixed"; X: XmlPort "DF XP Fixed"; U: Record "DF Universal"; Cr: Text[1]; Lf: Text[1]; begin
            Cr[1] := 13; Lf[1] := 10;
            U."Entry No." := 1; U."Text Field" := 'abc'; U."Decimal Field" := 12.5; U.Insert();
            U."Entry No." := 123456; U."Text Field" := 'toolongvalue'; U."Decimal Field" := -1; U.Insert();
            Dest(B, O); E.SetDestination(O); E.Export();
            Check('3120202020616263202020202031322E353020202020200D0A3132333435746F6F6C6F6E67762D312E30302020202020', HexOf(B));
            U.DeleteAll();
            Feed('    1abc        12.50' + Cr + Lf + '    2xyz         3.00', B, I);
            X.SetSource(I); X.Import(); Commit();
            Check('1|abc|12.5| ; 2|xyz|3| ; ', UniversalRows()); end;"#,
    );
}
