//! A table element nested in another one, linked with LinkTable and LinkFields, with MaxOccurs = Once and no MinOccurs (al-runner
//! issue 7090; corpus xmlports/shapes-properties/XpNestedLinkedOnce.al): no MinOccurs is
//! MinOccurs = Once. The reporter's shape has a temporary child the parent's OnAfterGetRecord fills.

use al_testkit::{run_in, text};

/// The ports of XpNestedLinkedOnce.al (an export port, or an import port): `parent_occurs` on the parent, `temp` makes the child temporary and filled by the parent's
/// OnAfterGetRecord. `call` is what `Go()` gives; `Export(A, B, C)` seeds parents P1, P2, P3 (a negative count leaves one out) with that many children and gives
/// the lines joined by | and the row the child record holds after it.
fn src(direction: &str, parent_occurs: &str, temp: bool, call: &str) -> String {
    // The real-table port of the corpus reads no Count after the export.
    let count = if temp {
        " + '|count=' + Format(child.Count())"
    } else {
        ""
    };
    let (use_temp, fill) = if temp {
        (
            "UseTemporary = true;",
            "trigger OnAfterGetRecord() var i: Integer; begin child.DeleteAll(); for i := 1 to parent.Children do begin child.\"Entry no. parent\" := parent.\"Entry no.\"; child.Line := i; child.Name := 'CHILD-' + parent.\"Entry no.\" + '-' + Format(i); child.Insert(); end; end;",
        )
    } else {
        ("", "")
    };
    format!(
        r#"
table 69990 "NL Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69991 "NL Parent" {{ fields {{ field(1; "Entry no."; Code[10]) {{ }} field(2; Children; Integer) {{ }} }} keys {{ key(PK; "Entry no.") {{ }} }} }}
table 69992 "NL Child" {{ fields {{ field(1; "Entry no. parent"; Code[10]) {{ }} field(2; Line; Integer) {{ }} field(3; Name; Text[30]) {{ }} }} keys {{ key(PK; "Entry no. parent", Line) {{ }} }} }}
xmlport 69993 "NL Port"
{{
    Direction = {direction}; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            tableelement(parent; "NL Parent")
            {{
                {parent_occurs}
                XmlName = 'Transport';
                fieldattribute(no; parent."Entry no.") {{ }}
                tableelement(child; "NL Child")
                {{
                    LinkFields = "Entry no. parent" = field("Entry no.");
                    LinkTable = parent;
                    MaxOccurs = Once;
                    XmlName = 'MainInfo';
                    {use_temp}
                    fieldelement(Name; child.Name) {{ }}
                }}
                {fill}
            }}
        }}
    }}
    procedure HeldChild(): Text
    begin
        exit(child."Entry no. parent" + '/' + Format(child.Line) + '/' + child.Name + '|filters=' + child.GetFilters(){count});
    end;
}}
codeunit 69994 C
{{
    procedure Go(): Text begin exit({call}); end;

    procedure Export(A: Integer; B: Integer; C: Integer): Text
    var Blob: Record "NL Blob" temporary; X: XmlPort "NL Port"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        AddParent('P1', A, {temp}); AddParent('P2', B, {temp}); AddParent('P3', C, {temp});
        Blob.Data.CreateOutStream(OutS);
        X.SetDestination(OutS);
        X.Export();
        Blob.Data.CreateInStream(InS);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T + 'held=' + X.HeldChild());
    end;

    local procedure AddParent(No: Code[10]; Children: Integer; Temp: Boolean)
    var P: Record "NL Parent"; R: Record "NL Child"; i: Integer;
    begin
        if Children < 0 then exit;
        P."Entry no." := No; P.Children := Children; P.Insert();
        if Temp then exit;
        for i := 1 to Children do begin
            R."Entry no. parent" := No; R.Line := i; R.Name := 'R-' + No + '-' + Format(i); R.Insert();
        end;
    end;

    procedure Import(Doc: Text): Text
    var Blob: Record "NL Blob" temporary; P: Record "NL Parent"; R: Record "NL Child"; OutS: OutStream; InS: InStream; T: Text;
    begin
        Blob.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Doc);
        Blob.Data.CreateInStream(InS, TextEncoding::UTF8);
        if Xmlport.Import(Xmlport::"NL Port", InS) then T := 'ok' else T := 'error:' + GetLastErrorText();
        if P.FindSet() then repeat T += ';P:' + P."Entry no."; until P.Next() = 0;
        if R.FindSet() then repeat T += ';C:' + R."Entry no. parent" + '/' + Format(R.Line) + '/' + R.Name; until R.Next() = 0;
        exit(T);
    end;
}}
"#
    )
}

const HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>|<Root>|";

fn export(parent_occurs: &str, temp: bool, a: i32, b: i32, c: i32) -> String {
    text(run_in(
        &src("Export", parent_occurs, temp, &format!("Export({a}, {b}, {c})")),
        "C",
        "Go",
    ))
}

fn element(no: &str, name: &str) -> String {
    let name = if name.is_empty() {
        "      <Name />|".to_string()
    } else {
        format!("      <Name>{name}</Name>|")
    };
    format!("  <Transport no=\"{no}\">|    <MainInfo>|{name}    </MainInfo>|  </Transport>|")
}

// XpNestedLinkedOnce.al XOM_TempOnce_OneParent_ZeroChildren, _OneChild, _TwoChildren, _TwoParents.
#[test]
fn the_reporters_shape_writes_the_first_temporary_child_and_holds_the_last() {
    let once = "MaxOccurs = Once;";
    assert_eq!(
        export(once, true, 0, -1, -1),
        format!("{HEAD}{}</Root>|held=/0/|filters=|count=0", element("P1", ""))
    );
    assert_eq!(
        export(once, true, 1, -1, -1),
        format!(
            "{HEAD}{}</Root>|held=P1/1/CHILD-P1-1|filters=|count=1",
            element("P1", "CHILD-P1-1")
        )
    );
    assert_eq!(
        export(once, true, 2, -1, -1),
        format!(
            "{HEAD}{}</Root>|held=P1/2/CHILD-P1-2|filters=|count=2",
            element("P1", "CHILD-P1-1")
        )
    );
    assert_eq!(
        export(once, true, 1, 2, -1),
        format!(
            "{HEAD}{}</Root>|held=P1/1/CHILD-P1-1|filters=|count=1",
            element("P1", "CHILD-P1-1")
        )
    );
}

// XpNestedLinkedOnce.al XOM_TempOnceUnbounded_ZeroOneTwoChildren, _TwoOneZeroChildren: a pass whose trigger inserts no row writes
// the row the record held before the DeleteAll.
#[test]
fn a_temporary_child_below_an_unbounded_parent_writes_one_row_per_pass() {
    assert_eq!(
        export("", true, 0, 1, 2),
        format!(
            "{HEAD}{}{}{}</Root>|held=P3/2/CHILD-P3-2|filters=|count=2",
            element("P1", ""),
            element("P2", "CHILD-P2-1"),
            element("P3", "CHILD-P3-1")
        )
    );
    assert_eq!(
        export("", true, 2, 1, 0),
        format!(
            "{HEAD}{}{}{}</Root>|held=P2/1/CHILD-P2-1|filters=|count=0",
            element("P1", "CHILD-P1-1"),
            element("P2", "CHILD-P2-1"),
            element("P3", "CHILD-P2-1")
        )
    );
}

// XpNestedLinkedOnce.al XOM_RealOnce_ZeroOneTwoChildren, _TwoOneZeroChildren (a real child table; the corpus port also writes the
// line as an attribute, the held text has no count).
#[test]
fn a_real_child_without_min_occurs_writes_one_row_per_pass() {
    assert_eq!(
        export("", false, 0, 1, 2),
        format!(
            "{HEAD}{}{}{}</Root>|held=P3/2/R-P3-2|filters=",
            element("P1", ""),
            element("P2", "R-P2-1"),
            element("P3", "R-P3-1")
        )
    );
    assert_eq!(
        export("", false, 2, 1, 0),
        format!(
            "{HEAD}{}{}{}</Root>|held=P2/1/R-P2-1|filters=",
            element("P1", "R-P1-1"),
            element("P2", "R-P2-1"),
            element("P3", "R-P2-1")
        )
    );
}

// XpNestedLinkedOnce.al XOM_ImportOnce_OneChild, _NoChild, _TwoChildren.
#[test]
fn import_takes_one_child_element_per_parent() {
    let import = |doc: &str| {
        text(run_in(
            &src("Import", "", false, &format!("Import('{doc}')")),
            "C",
            "Go",
        ))
    };
    assert_eq!(
        import("<Root><Transport no=\"P1\"><MainInfo><Name>a</Name></MainInfo></Transport></Root>"),
        "ok;P:P1;C:P1/0/a"
    );
    assert_eq!(
        import(
            "<Root><Transport no=\"P1\"></Transport><Transport no=\"P2\"><MainInfo><Name>b</Name></MainInfo></Transport></Root>"
        ),
        "error:Imported XML cannot validate with the schema: The element 'Transport' has incomplete content. List of possible elements expected: 'MainInfo'."
    );
    assert_eq!(
        import(
            "<Root><Transport no=\"P1\"><MainInfo><Name>a</Name></MainInfo><MainInfo><Name>b</Name></MainInfo></Transport></Root>"
        ),
        "error:Imported XML cannot validate with the schema: The element 'Transport' has invalid child element 'MainInfo'."
    );
}
