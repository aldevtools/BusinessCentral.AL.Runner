//! Gaps a partner app hit (np-blockers-68). Each test names the corpus pin it relies on (corpus master,
//! misc/lt-np4-probes, corpus PR 871; the pinned corpus does not hold it yet).

use al_testkit::outcomes_in;

fn outcome(o: &[(String, al_testrun::Outcome)], n: &str) -> String {
    format!("{:?}", o.iter().find(|(name, _)| name == n).map(|(_, x)| x))
}

// A read under ReadIsolation RepeatableRead starts the write transaction (misc/lt-np4-probes/Np4Misc.al:219
// NP4_ReadIsolation_RepeatableRead_Reads: `findset Yes|1|Yes` as the first read). The assignment alone opens it (corpus PR 896,
// Np4Misc.al Y9_RepeatableRead_SetAfterCommit_FirstRead `before=Yes`).
const ISOLATION: &str = r#"
table 50680 "N68 Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
codeunit 50681 T
{
    Subtype = Test;
    [Test] procedure FindSetUnderRepeatableRead()
    var Row: Record "N68 Row";
    begin
        Seed();
        Row.ReadIsolation := IsolationLevel::RepeatableRead;
        if not Row.FindSet() then Error('no rows');
        if not IsInWriteTransaction() then Error('no write transaction');
    end;
    [Test] procedure GetUnderRepeatableReadAfterFindSet()
    var Row: Record "N68 Row"; Other: Record "N68 Row";
    begin
        Seed();
        Row.ReadIsolation := IsolationLevel::RepeatableRead;
        Row.FindSet();
        Other.ReadIsolation := IsolationLevel::RepeatableRead;
        if not Other.Get(2) then Error('no row 2');
        if not IsInWriteTransaction() then Error('no write transaction');
    end;
    [Test] procedure SetWithoutReadOpensIt()
    var Row: Record "N68 Row";
    begin
        Seed();
        Row.ReadIsolation := IsolationLevel::RepeatableRead;
        if not IsInWriteTransaction() then Error('no write transaction');
    end;
    local procedure Seed()
    var Row: Record "N68 Row";
    begin
        Row.DeleteAll();
        Row.Id := 1; Row.Insert();
        Row.Id := 2; Row.Insert();
        Commit();
    end;
}
"#;

#[test]
fn a_find_set_under_repeatable_read_opens_the_write_transaction() {
    let o = outcomes_in(ISOLATION, "T");
    assert_eq!(outcome(&o, "FindSetUnderRepeatableRead"), "Some(Pass)");
    assert_eq!(outcome(&o, "GetUnderRepeatableReadAfterFindSet"), "Some(Pass)");
    assert_eq!(outcome(&o, "SetWithoutReadOpensIt"), "Some(Pass)");
}

// Error('') out of a control's own OnValidate during SetValue on a List and a Worksheet page: the test goes on, the last error has
// the empty text and code NSValidateField:Dialog, the control shows the old value and the row keeps it
// (misc/lt-np4-probes/Np4EmptyError.al:200 NP4_EE_ControlValidate_List, :224 _Worksheet).
const EMPTY_ERROR: &str = r#"
table 50682 "N68 Line" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
page 50683 "N68 List"
{
    PageType = List; SourceTable = "N68 Line"; UsageCategory = None;
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { trigger OnValidate() begin if Rec.Name = 'empty' then Error(''); end; }
    } } }
}
page 50684 "N68 Sheet"
{
    PageType = Worksheet; SourceTable = "N68 Line"; UsageCategory = None;
    layout { area(Content) { repeater(Rows) {
        field(Id; Rec.Id) { }
        field(Name; Rec.Name) { trigger OnValidate() begin if Rec.Name = 'empty' then Error(''); end; }
    } } }
}
page 50685 "N68 Part"
{
    PageType = ListPart; SourceTable = "N68 Line";
    layout { area(Content) { repeater(Rows) {
        field(Name; Rec.Name) { trigger OnValidate() begin if Rec.Name = 'empty' then Error(''); end; }
    } } }
}
page 50686 "N68 Host"
{
    PageType = Card; SourceTable = "N68 Line"; UsageCategory = None;
    layout { area(Content) { field(Id; Rec.Id) { } part(Sub; "N68 Part") { } } }
}
codeunit 50687 T
{
    Subtype = Test;
    [Test] procedure OnList()
    var Row: Record "N68 Line"; P: TestPage "N68 List";
    begin
        Row := Seed();
        P.OpenEdit(); P.GoToRecord(Row);
        P.Name.SetValue('empty');
        Check(P.Name.Value());
        P.Close();
        Stored();
    end;
    [Test] procedure OnWorksheet()
    var Row: Record "N68 Line"; P: TestPage "N68 Sheet";
    begin
        Row := Seed();
        P.OpenEdit(); P.GoToRecord(Row);
        P.Name.SetValue('empty');
        Check(P.Name.Value());
        P.Close();
        Stored();
    end;
    [Test] procedure OnListPartInCard()
    var Row: Record "N68 Line"; P: TestPage "N68 Host";
    begin
        Row := Seed();
        P.OpenEdit(); P.GoToRecord(Row);
        P.Sub.Name.SetValue('empty');
    end;
    local procedure Seed(): Record "N68 Line"
    var Row: Record "N68 Line";
    begin
        Row.DeleteAll();
        Row.Id := 1; Row.Name := 'seed'; Row.Insert();
        Commit();
        exit(Row);
    end;
    local procedure Check(Shown: Text)
    begin
        if GetLastErrorText() <> '' then Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'NSValidateField:Dialog' then Error('code %1', GetLastErrorCode());
        if Shown <> 'seed' then Error('shown %1', Shown);
    end;
    local procedure Stored()
    var Row: Record "N68 Line";
    begin
        Row.Get(1);
        if Row.Name <> 'seed' then Error('stored %1', Row.Name);
    end;
}
"#;

#[test]
fn an_empty_error_out_of_a_list_or_worksheet_control_validate_ends_set_value_quietly() {
    let o = outcomes_in(EMPTY_ERROR, "T");
    assert_eq!(outcome(&o, "OnList"), "Some(Pass)");
    assert_eq!(outcome(&o, "OnWorksheet"), "Some(Pass)");
    // A ListPart inside a Card is pinned too (corpus PR 896, Np4EmptyError.al Y9_EE_ControlValidate_ListPartInCard).
    assert_eq!(outcome(&o, "OnListPartInCard"), "Some(Pass)");
}

// XmlPort export: a table element without LinkTable below another one, MinOccurs = Once and MaxOccurs = Once, writes the first
// row of each pass; a pass without rows writes the record as it stands, which after a pass of two rows is the second row
// (misc/lt-np4-probes/Np4XmlPort.al:364 NP4_XmlPort_OnceOnce_ZeroAndOneRows, :381 _TwoRows). PreserveWhiteSpace changes nothing an
// export writes, with and without DefaultNamespace (:280 NP4_XmlPort_PreserveWhiteSpace_WithNamespace, :300 _DefaultWhiteSpace_WithNamespace, :320 _PreserveWhiteSpace_NoNamespace).
const XMLPORT: &str = r#"
table 50688 "N68 Head" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
table 50689 "N68 Child" { fields { field(1; Id; Integer) { } field(2; Txt; Text[100]) { } } keys { key(PK; Id) { } } }
table 50690 "N68 Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 50691 "N68 Once"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema { textelement(Root) {
        tableelement(Head; "N68 Head") {
            MinOccurs = Zero; MaxOccurs = Unbounded;
            fieldattribute(id; Head.Id) { }
            tableelement(Child; "N68 Child") {
                MinOccurs = Once; MaxOccurs = Once;
                fieldattribute(cid; Child.Id) { }
                fieldelement(Txt; Child.Txt) { }
                trigger OnPreXmlItem() begin Child.SetRange(Id, Head.Id * 10, Head.Id * 10 + 9); end;
            }
        }
    } }
}
xmlport 50692 "N68 Ns Ws"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    DefaultNamespace = 'urn:n68'; UseDefaultNamespace = true; PreserveWhiteSpace = true;
    schema { textelement(Root) {
        tableelement(Row; "N68 Child") { UseTemporary = true; fieldelement(Txt; Row.Txt) { } }
    } }
    procedure Seed(T: Text) begin Row.Id := 1; Row.Txt := T; Row.Insert(); end;
}
xmlport 50693 "N68 Ns"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    DefaultNamespace = 'urn:n68'; UseDefaultNamespace = true;
    schema { textelement(Root) {
        tableelement(Row; "N68 Child") { UseTemporary = true; fieldelement(Txt; Row.Txt) { } }
    } }
    procedure Seed(T: Text) begin Row.Id := 1; Row.Txt := T; Row.Insert(); end;
}
codeunit 50694 T
{
    Subtype = Test;
    [Test] procedure OnceOnceTwoThenNone()
    var B: Record "N68 Blob" temporary; P: XmlPort "N68 Once"; O: OutStream;
    begin
        Seed(2, 0);
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(O, TextEncoding::UTF8);
        P.SetDestination(O); P.Export();
        Expect(ToText(B), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Root>|  <Head id="1">|    <Child cid="11">|      <Txt>c11</Txt>|    </Child>|  </Head>|  <Head id="2">|    <Child cid="12">|      <Txt>c12</Txt>|    </Child>|  </Head>|</Root>|');
    end;
    [Test] procedure OnceOnceNoneThenOne()
    var B: Record "N68 Blob" temporary; P: XmlPort "N68 Once"; O: OutStream;
    begin
        Seed(0, 1);
        B.Id := 1; B.Insert(); B.Data.CreateOutStream(O, TextEncoding::UTF8);
        P.SetDestination(O); P.Export();
        Expect(ToText(B), '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Root>|  <Head id="1">|    <Child cid="0">|      <Txt />|    </Child>|  </Head>|  <Head id="2">|    <Child cid="21">|      <Txt>c21</Txt>|    </Child>|  </Head>|</Root>|');
    end;
    [Test] procedure PreserveWhiteSpaceWithNamespace()
    var B: Record "N68 Blob" temporary; P: XmlPort "N68 Ns Ws"; Q: XmlPort "N68 Ns"; O: OutStream; Plain: Text; Lf: Text[1];
    begin
        Lf[1] := 10;
        Q.Seed(' a' + Lf + 'b ');
        B.Id := 2; B.Insert(); B.Data.CreateOutStream(O, TextEncoding::UTF8);
        Q.SetDestination(O); Q.Export();
        Plain := ToText(B);
        B.Delete();
        P.Seed(' a' + Lf + 'b ');
        B.Id := 2; B.Insert(); B.Data.CreateOutStream(O, TextEncoding::UTF8);
        P.SetDestination(O); P.Export();
        Expect(ToText(B), Plain);
        // The line feed of the value is written as CR LF: 129 bytes.
        Expect(Plain, '<?xml version="1.0" encoding="UTF-8" standalone="no"?>|<Root xmlns="urn:n68">|  <Row>|    <Txt> a|b </Txt>|  </Row>|</Root>|len=129');
    end;
    local procedure Seed(One: Integer; Two: Integer)
    var H: Record "N68 Head"; C: Record "N68 Child"; i: Integer;
    begin
        H.DeleteAll(); C.DeleteAll();
        H.Id := 1; H.Insert(); H.Id := 2; H.Insert();
        for i := 1 to One do begin C.Id := 10 + i; C.Txt := 'c' + Format(C.Id); C.Insert(); end;
        for i := 1 to Two do begin C.Id := 20 + i; C.Txt := 'c' + Format(C.Id); C.Insert(); end;
    end;
    local procedure ToText(var B: Record "N68 Blob" temporary): Text
    var I: InStream; Line: Text; X: Text;
    begin
        B.Modify(); B.CalcFields(Data); B.Data.CreateInStream(I, TextEncoding::UTF8);
        while not I.EOS() do begin I.ReadText(Line); X += Line + '|'; end;
        if B.Id = 2 then exit(X + 'len=' + Format(B.Data.Length));
        exit(X);
    end;
    local procedure Expect(Got: Text; Want: Text)
    begin
        if Got <> Want then Error('got %1', Got);
    end;
}
"#;

#[test]
fn xmlport_nested_once_once_and_preserve_white_space_with_a_namespace_export() {
    let o = outcomes_in(XMLPORT, "T");
    for t in [
        "OnceOnceTwoThenNone",
        "OnceOnceNoneThenOne",
        "PreserveWhiteSpaceWithNamespace",
    ] {
        assert_eq!(outcome(&o, t), "Some(Pass)", "{t}");
    }
}
