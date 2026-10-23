//! Behaviour pinned by the corpus staging branch (al-runtime-rs-staging 8217373) for the blockers a
//! real project met (Pageworks): `continue`, `Label.Split`, `foreach` over a JsonArray and an
//! XmlAttributeCollection, `Database.LockTimeoutDuration`. Each test names the staging file and the
//! literal it takes from the real service tier. Written in my own minimal AL.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::{run_with, text};

fn run(src: &str, routine: &str) -> Result<Value, Signal> {
    al_testkit::run(src, routine)
}

fn cu(body: &str) -> String {
    format!("codeunit 50100 C {{ {body} }}")
}

// types/PUContinue.al, Continue_InForToDownto
#[test]
fn continue_in_for_keeps_the_counter_moving() {
    let r = text(run(
        &cu("procedure F(): Text var i: Integer; Seen: Text; begin
            for i := 1 to 6 do begin if i mod 2 = 0 then continue; Seen += Format(i); end;
            Seen += '/' + Format(i) + '/';
            for i := 6 downto 1 do begin if i = 3 then continue; Seen += Format(i); end;
            exit(Seen); end;"),
        "F",
    ));
    assert_eq!(r, "135/6/65421");
}

// Continue_InWhileAndRepeat: in repeat, continue jumps to the until check
#[test]
fn continue_in_while_and_repeat() {
    let r = text(run(
        &cu("procedure F(): Text var i: Integer; Seen: Text; begin
            while i < 6 do begin i += 1; if i mod 2 = 1 then continue; Seen += Format(i); end;
            Seen += '|'; i := 0;
            repeat i += 1; if i = 2 then continue; Seen += Format(i); until i >= 4;
            Seen += '|'; i := 0;
            repeat i += 1; if i < 10 then continue; Seen += Format(i); until i >= 4;
            exit(Seen + '/' + Format(i)); end;"),
        "F",
    ));
    assert_eq!(r, "246|134|/4");
}

// Continue_InForeach_AndNestedLoops
#[test]
fn continue_in_foreach_nested_loops_and_before_break() {
    let r = text(run(
        &cu("procedure F(): Text var L: List of [Integer]; N: Integer; i: Integer; j: Integer; Seen: Text; begin
            L.Add(1); L.Add(2); L.Add(3); L.Add(4);
            foreach N in L do begin if N = 2 then continue; Seen += Format(N); end;
            Seen += '|';
            for i := 1 to 3 do for j := 1 to 3 do begin if j = 2 then continue; Seen += Format(i) + Format(j) + ' '; end;
            Seen += '|';
            for i := 1 to 5 do begin if i = 2 then continue; if i = 4 then break; Seen += Format(i); end;
            exit(Seen + '/' + Format(i)); end;"),
        "F",
    ));
    assert_eq!(r, "134|11 13 21 23 31 33 |13/4");
}

// Continue_InRecordLoop_AndWithCase
#[test]
fn continue_inside_a_case_branch_of_a_loop() {
    let r = text(run(
        &cu("procedure F(): Text var i: Integer; Seen: Text; begin
            i := 0;
            repeat
                i += 1;
                case i of
                    1: continue;
                    else Seen += Format(i);
                end;
            until i >= 3;
            exit(Seen); end;"),
        "F",
    ));
    assert_eq!(r, "23");
}

/// A variable called `Continue` is still a variable (older code uses the name).
#[test]
fn a_variable_called_continue_still_works() {
    let r = text(run(
        &cu("procedure F(): Text var Continue: Boolean; n: Integer; begin
            Continue := true;
            while Continue do begin n += 1; if n >= 3 then Continue := false; end;
            exit(Format(n)); end;"),
        "F",
    ));
    assert_eq!(r, "3");
}

// types/PVLabelSplitForeach.al, Label_Split_OneAndSeveralSeparators
#[test]
fn label_split_is_text_split() {
    let r = text(run(
        "codeunit 50100 C {
            var CsvLbl: Label 'a,b;c,,d', Locked = true; EmptyLbl: Label '', Locked = true;
            local procedure Join(List: List of [Text]): Text var Part: Text; Result: Text; begin
                foreach Part in List do Result += '[' + Part + ']'; exit(Result); end;
            procedure F(): Text begin
                exit(Join(CsvLbl.Split(',')) + '|' + Join(CsvLbl.Split(',', ';')) + '|' + Join(EmptyLbl.Split(',')) + '|' + Format(CsvLbl.Split(',').Count())); end;
        }",
        "F",
    ));
    assert_eq!(r, "[a][b;c][][d]|[a][b][c][][d]|[]|4");
}

// Foreach_JsonArray_ElementsAndOrder, Foreach_JsonArray_Empty_AndNestedModification
#[test]
fn foreach_over_a_json_array_in_order_and_as_it_was_at_the_start() {
    let r = text(run(
        &cu(
            "procedure F(): Text var Arr: JsonArray; Token: JsonToken; Seen: Text; Count: Integer; begin
            Arr.ReadFrom('[1,\"two\",true,null,{\"k\":4},[5]]');
            foreach Token in Arr do begin
                if Token.IsValue() then Seen += 'V:' + Format(Token.AsValue().IsNull()) + ' '
                else if Token.IsObject() then Seen += 'O ' else Seen += 'A ';
            end;
            Arr.ReadFrom('[]'); foreach Token in Arr do Count += 1; Seen += Format(Count) + ' ';
            Arr.ReadFrom('[1,2,3]'); Count := 0;
            foreach Token in Arr do begin Count += 1; if Count = 1 then Arr.Add(4); end;
            exit(Seen + Format(Count)); end;",
        ),
        "F",
    ));
    assert_eq!(r, "V:No V:No V:No V:Yes O A 0 3");
}

// Foreach_XmlAttributeCollection
#[test]
fn foreach_over_xml_attributes_in_document_order() {
    let r = text(run(
        &cu("procedure F(): Text var Doc: XmlDocument; Root: XmlElement; Attrs: XmlAttributeCollection; Attr: XmlAttribute; Seen: Text; Count: Integer; begin
            XmlDocument.ReadFrom('<root a=\"1\" b=\"two\" c=\"\"/>', Doc); Doc.GetRoot(Root);
            Attrs := Root.Attributes();
            foreach Attr in Attrs do begin Seen += Attr.Name() + '=' + Attr.Value() + ';'; Count += 1; end;
            Seen += Format(Count) + Format(Attrs.Count());
            XmlDocument.ReadFrom('<root/>', Doc); Doc.GetRoot(Root); Count := 0;
            foreach Attr in Root.Attributes() do Count += 1;
            exit(Seen + Format(Count)); end;"),
        "F",
    ));
    assert_eq!(r, "a=1;b=two;c=;330");
}

// Database_LockTimeoutDuration_Default: positive, LockTimeout is Yes
#[test]
fn lock_timeout_duration_is_positive() {
    let r = text(run(
        &cu(
            "procedure F(): Text var D: Duration; begin D := Database.LockTimeoutDuration(); exit(Format(D > 0) + Format(Database.LockTimeout())); end;",
        ),
        "F",
    ));
    assert_eq!(r, "YesYes");
}

// ---- Base64Convert, the platform's [Native] codeunit (System.app 2000000024) ----------------------
// types/PVBase64.al on the staging branch. The test declares the codeunit as System.app does.

const BASE64: &str = r#"
codeunit 50101 Base64Convert
{
    [Native] procedure ToBase64(String: Text; InsertLineBreaks: Boolean; TextEncoding: TextEncoding; Codepage: Integer): Text begin end;
    [Native] procedure ToBase64(InStream: InStream; InsertLineBreaks: Boolean): Text begin end;
    [Native] procedure FromBase64(Base64String: Text; TextEncoding: TextEncoding; CodePage: Integer): Text begin end;
    [Native] procedure FromBase64(Base64String: Text; OutStream: OutStream) begin end;
    [Native] procedure Unlisted(String: Text): Text begin end;
}
table 50102 "B64 Row" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
codeunit 50100 C
{
    procedure Encoded(): Text
    var B: Codeunit Base64Convert; Long: Text; i: Integer; Wb: Text;
    begin
        for i := 1 to 120 do Long += 'a';
        Wb := B.ToBase64(Long, true, TextEncoding::UTF8, 0);
        exit(B.ToBase64('Hello', false, TextEncoding::UTF8, 0) + '|' + B.ToBase64('äöü', false, TextEncoding::UTF16, 0)
            + '|' + B.ToBase64('äöü', false, TextEncoding::Windows, 1252) + '|' + B.ToBase64('äöü', false, TextEncoding::Windows, 0)
            + '|' + Format(StrLen(Wb)) + '|' + B.FromBase64('w6TDtsO8', TextEncoding::UTF8, 0) + '|' + B.FromBase64('5Pb8', TextEncoding::Windows, 1252));
    end;
    procedure Invalid(): Text
    var B: Codeunit Base64Convert; Code: Text; Msg: Text;
    begin
        asserterror B.FromBase64('not base64!', TextEncoding::UTF8, 0);
        exit(GetLastErrorCode() + '|' + GetLastErrorText());
    end;
    procedure Streams(): Text
    var B: Codeunit Base64Convert; R: Record "B64 Row" temporary; OutS: OutStream; InS: InStream; T: Text;
    begin
        R.Id := 1;
        R.Data.CreateOutStream(OutS);
        B.FromBase64('SGVsbG8=', OutS);
        R.Insert();
        R.CalcFields(Data);
        R.Data.CreateInStream(InS);
        exit(B.ToBase64(InS, false));
    end;
    procedure NotPinned(): Text
    var B: Codeunit Base64Convert;
    begin
        exit(B.Unlisted('x'));
    end;
    procedure NotReached(Call: Boolean): Text
    var B: Codeunit Base64Convert;
    begin
        if Call then
            exit(B.Unlisted('x'));
        exit('not called');
    end;
}
"#;

#[test]
fn base64_convert_natives_answer_as_the_probe_did() {
    assert_eq!(text(run(BASE64, "Encoded")), "SGVsbG8=|5AD2APwA|5Pb8|5Pb8|164|äöü|äöü");
    assert_eq!(
        text(run(BASE64, "Invalid")),
        "AL:Format|The input is not a valid Base-64 string as it contains a non-base 64 character, more than two padding characters, or an illegal character among the padding characters."
    );
    assert_eq!(text(run(BASE64, "Streams")), "SGVsbG8=");
}

#[test]
fn a_native_that_has_no_implementation_stays_refused() {
    match run(BASE64, "NotPinned") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("Native") && why.contains("Unlisted"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// al-runner issue 5684: an install routine that can reach a native it never calls runs on BC (corpus-writer RH:
// nothing to pin, the reached native stops BC with a CLR exception). The native refuses when it is called, not when it
// is merely reachable.
#[test]
fn a_native_without_implementation_refuses_only_when_called() {
    let not_called = run_with(BASE64, "C", "NotReached", vec![Value::Boolean(false)]).map(|(v, _)| v);
    assert_eq!(text(not_called), "not called");
    match run_with(BASE64, "C", "NotReached", vec![Value::Boolean(true)]) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("Native") && why.contains("Unlisted"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// ---- XPath predicates joined by `and` (a Pageworks column lookup) --------------------------------
// `.//*[local-name()="Column" and @name="Col"]` was read as local-name() = 'Column" and @name="Col' (the
// quotes around the whole rest taken as one literal) and found nothing, so every column lookup of the
// project came back empty. XPath 1.0 `and` of the pinned predicate forms (corpus xml/TestXmlRuntimeContracts.al:283
// local-name(), @name='v'); the conjunction itself is the W3C operator, recorded as an assumption in
// docs/probes-needed.md.
#[test]
fn xpath_predicates_joined_by_and_select_what_both_say() {
    let r = text(run(
        &cu("procedure F(): Text var Doc: XmlDocument; Root: XmlElement; N: XmlNode; L: XmlNodeList; Seen: Text; begin
            XmlDocument.ReadFrom('<d><Column name=\"A\">1</Column><Column name=\"B\">2</Column><Other name=\"A\">3</Other></d>', Doc);
            Doc.GetRoot(Root);
            Root.AsXmlNode().SelectSingleNode('.//*[local-name()=\"Column\" and @name=\"B\"]', N); Seen += N.AsXmlElement().InnerText();
            Root.SelectNodes('.//*[local-name()=\"Column\" and @name=\"A\"]', L); Seen += '/' + Format(L.Count());
            Root.SelectNodes('.//*[@name=\"A\" and local-name()=\"Other\"]', L); Seen += '/' + Format(L.Count());
            Root.SelectNodes('.//*[local-name()=\"Column\" and @name=\"Z\"]', L); Seen += '/' + Format(L.Count());
            exit(Seen); end;"),
        "F",
    ));
    assert_eq!(r, "2/1/1/0");
}

/// A literal with a quote of its own kind inside is not a literal: refused, not matched against nothing.
#[test]
fn an_xpath_literal_with_an_inner_quote_is_refused() {
    match run(
        &cu(
            "procedure F() var Doc: XmlDocument; Root: XmlElement; L: XmlNodeList; begin
            XmlDocument.ReadFrom('<d/>', Doc); Doc.GetRoot(Root);
            Root.SelectNodes('.//*[local-name()=\"a\" b\"]', L); end;",
        ),
        "F",
    ) {
        Err(Signal::Unsupported(why)) => assert!(why.contains("XPath"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// ---- overloads that differ in the element type of a `var List` (NPR Json Parser.GetValues) -------
// `GetValues(var List of [JsonValue])` and `GetValues(var List of [JsonObject])`: a var parameter takes
// exactly its own type, so the argument's element type picks the overload. The first one declared was
// always taken, and its `AsValue()` failed on an object ("Unable to convert from NavJsonToken to NavJsonValue").
#[test]
fn a_var_list_argument_picks_the_overload_with_its_own_element_type() {
    let r = text(run(
        "codeunit 50100 C {
            procedure Pick(var L: List of [Text]): Text begin exit('text'); end;
            procedure Pick(var L: List of [Integer]): Text begin exit('integer'); end;
            procedure Pick(var L: List of [JsonObject]): Text begin exit('object'); end;
            procedure F(): Text var A: List of [Text]; B: List of [Integer]; C: List of [JsonObject]; begin
                exit(Pick(A) + '|' + Pick(B) + '|' + Pick(C)); end;
        }",
        "F",
    ));
    assert_eq!(r, "text|integer|object");
}

// ---- Report.WordXmlPart (staging types/PZMisc.al:WordXmlPart_*, BC 29.0) -------------------------------
// The XML of the data set as text for a report that has only an RDLC layout, with either flag: it starts with
// `<`, holds the data item and column names and the Word data namespace marker `urn:microsoft-dynamics-nav`.
// An unknown report id fails with the code AL:MetadataNotFound.
const WORD_REPORT: &str = "table 50130 \"WX Src\" { fields { field(1; Id; Integer) { } field(2; \"Long Name\"; Text[20]) { } } keys { key(PK; Id) { } } }
report 50131 \"WX Report\" { DefaultLayout = RDLC; dataset { dataitem(Src; \"WX Src\") { column(IdCol; Id) { } dataitem(\"Child Item\"; \"WX Src\") { column(NameCol; \"Long Name\") { } } } } }
codeunit 50100 C { procedure F(): Text var R: Text; Seen: Text; begin
    R := Report.WordXmlPart(50131);
    Seen := Format(R.StartsWith('<')) + '/' + Format(StrPos(R, 'IdCol') > 0) + '/' + Format(StrPos(R, 'Src') > 0) + '/' + Format(StrPos(R, 'urn:microsoft-dynamics-nav') > 0);
    Seen += '/' + Format(StrPos(R, 'ElementType') > 0);
    R := Report.WordXmlPart(50131, true);
    Seen += '/' + Format(StrPos(R, 'ElementType=\"Column\"') > 0) + '/' + Format(StrPos(R, 'Child_Item') > 0);
    R := Report.WordXmlPart(50131, false);
    Seen += '/' + Format(StrPos(R, 'ElementType') > 0);
    exit(Seen);
end;
[TryFunction] procedure Missing() var R: Text; begin R := Report.WordXmlPart(99999999); end;
procedure G(): Text var Ok: Boolean; begin Ok := Missing(); exit(Format(Ok) + '|' + GetLastErrorCode()); end; }";

#[test]
fn word_xml_part_answers_the_data_set_as_xml_text() {
    assert_eq!(text(run(WORD_REPORT, "F")), "Yes/Yes/Yes/Yes/No/Yes/Yes/No");
}

#[test]
fn word_xml_part_of_an_unknown_report_fails_with_metadata_not_found() {
    assert_eq!(text(run(WORD_REPORT, "G")), "No|AL:MetadataNotFound");
}

// A dataset fixture of Pageworks' picker source: `.//*[local-name()="DataItem"]` over `DataSet/DataItems/DataItem`, then the
// `name` attribute of each, read through `foreach .. in Element.Attributes()`.
#[test]
fn data_items_are_listed_by_their_name_attribute() {
    let r = text(run(
        &cu("procedure F(): Text var Doc: XmlDocument; Root: XmlElement; L: XmlNodeList; N: XmlNode; A: XmlAttribute; Seen: Text; begin
            XmlDocument.ReadFrom('<DataSet><DataItems><DataItem name=\"Customer\"><Columns><Column name=\"No.\">C001</Column></Columns><DataItems /></DataItem><DataItem name=\"Customer\" /></DataItems></DataSet>', Doc);
            Doc.GetRoot(Root);
            if not Root.SelectNodes('.//*[local-name()=\"DataItem\"]', L) then exit('no select');
            Seen := Format(L.Count());
            foreach N in L do foreach A in N.AsXmlElement().Attributes() do Seen += '/' + A.Name() + '=' + A.Value();
            foreach N in L do Seen += '|' + NameOf(N.AsXmlElement());
            exit(Seen);
        end;
        procedure NameOf(E: XmlElement): Text var A: XmlAttribute; begin foreach A in E.Attributes() do if A.Name() = 'name' then exit(A.Value()); end;"),
        "F",
    ));
    assert_eq!(r, "2/name=Customer/name=Customer|Customer|Customer");
}

// Pageworks' `DataItemXPath`: `(.//*[local-name()="DataItem" and @name="X"])[1]` and steps after it. The corpus pins the
// parenthesised path with steps after it (xml/TestXmlRuntimeContracts.al), not a predicate after the parenthesis: the
// W3C XPath 1.0 filter expression (docs/probes-needed.md "XPath predicates joined by `and`").
#[test]
fn a_predicate_after_a_parenthesised_path_picks_by_position_then_steps_follow() {
    let r = text(run(
        &cu("procedure F(): Text var Doc: XmlDocument; Root: XmlElement; L: XmlNodeList; Seen: Text; begin
            XmlDocument.ReadFrom('<d><i name=\"A\"><c>1</c><c>2</c></i><i name=\"A\"><c>3</c></i><i name=\"B\"><c>4</c></i></d>', Doc);
            Doc.GetRoot(Root);
            Root.SelectNodes('(.//*[local-name()=\"i\" and @name=\"A\"])[1]/*[local-name()=\"c\"]', L); Seen := Format(L.Count());
            Root.SelectNodes('(.//i)[last()]/c', L); Seen += '/' + Format(L.Count());
            Root.SelectNodes('(.//i)[2]//c', L); Seen += '/' + Format(L.Count());
            exit(Seen);
        end;"),
        "F",
    ));
    assert_eq!(r, "2/1/1");
}
