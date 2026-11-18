//! A Label or TextConst that went into a Variant as a TextConstant (`Variant.FromTextConstant#1`): what each reader of the
//! Variant then gets. The kind tests and `Format(V)` read the TextConstant (types/variant/VaTTextConst.al:283-289,
//! types/variant/VariantIsMatrix.al:224-243); every other reader of a Label gets the Text it got before the TextConstant
//! existed; every other reader of a TextConst is refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Error(e)) => e.message.to_string(),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

/// A test codeunit whose [Test] procedure `F` runs `body` after `V := Src`, with `Src` declared `decl`.
fn run(decl: &str, body: &str) -> String {
    let src = format!(
        r#"
table 50300 "TC Row"
{{
    fields
    {{
        field(1; Name; Text[50]) {{ }}
        field(2; CodeF; Code[10]) {{ }}
    }}
    keys {{ key(PK; Name) {{ }} }}
}}
codeunit 50301 T
{{
    Subtype = Test;
    var
        Src: {decl};

    [Test]
    procedure F()
    var
        V: Variant;
        Row: Record "TC Row";
        RR: RecordRef;
        FR: FieldRef;
        T: Text;
    begin
        V := Src;
        {body}
    end;
}}
"#
    );
    out(al_testkit::run_in(&src, "T", "F"))
}

const LABEL: &str = "Label 'abc'";
const TEXTCONST: &str = "TextConst ENU = 'abc'";

#[test]
fn a_label_text_constant_reads_as_its_text_outside_the_kind_tests() {
    let cases = [
        ("if V.IsTextConstant() then Error('R=TC'); Error('R=other');", "R=TC"),
        ("Error('R=' + Format(V));", "R=abc"),
        ("Row.SetFilter(Name, V); Error('R=' + Row.GetFilter(Name));", "R=abc"),
        ("Error('R=[' + Format(V, 5) + ']');", "R=[abc  ]"),
        ("Error('R=[' + Format(V, 2) + ']');", "R=[ab]"),
        ("Row.Name := V; Error('R=' + Row.Name);", "R=abc"),
        (
            "RR.Open(Database::\"TC Row\"); FR := RR.Field(1); FR.Value := V; Error('R=' + Format(FR.Value));",
            "R=abc",
        ),
        (
            "RR.Open(Database::\"TC Row\"); FR := RR.Field(1); FR.SetFilter(V); Error('R=' + FR.GetFilter());",
            "R=abc",
        ),
        ("Error('R=' + Format(StrLen(V)));", "R=3"),
        ("T := V; Error('R=' + T);", "R=abc"),
    ];
    for (body, want) in cases {
        assert_eq!(run(LABEL, body), want, "{body}");
    }
    // A view text from a Label: the filter of the view.
    let view = "Label 'WHERE(Name=FILTER(abc))'";
    assert_eq!(run(view, "Row.SetView(V); Error('R=' + Row.GetFilter(Name));"), "R=abc");
}

#[test]
fn a_textconst_text_constant_is_read_only_by_the_pinned_readers() {
    // Pinned: the kind tests, Format, a Text and a Code variable (types/variant/VaTTextConst.al:144-151, :283-289).
    for (body, want) in [
        ("if V.IsTextConstant() then Error('R=TC'); Error('R=other');", "R=TC"),
        ("Error('R=' + Format(V));", "R=abc"),
        ("T := V; Error('R=' + T);", "R=abc"),
    ] {
        assert_eq!(run(TEXTCONST, body), want, "{body}");
    }
    for body in [
        "Error('R=' + Format(StrLen(V)));",
        "Row.SetFilter(Name, V); Error('R=' + Row.GetFilter(Name));",
        "Error('R=[' + Format(V, 5) + ']');",
        "Row.Name := V; Error('R=' + Row.Name);",
        "RR.Open(Database::\"TC Row\"); FR := RR.Field(1); FR.Value := V; Error('R=' + Format(FR.Value));",
    ] {
        let r = run(TEXTCONST, body);
        assert!(r.starts_with("refused:") && r.contains("TextConst"), "{body}: {r}");
    }
}

#[test]
fn a_text_constant_handed_by_reference_to_a_builtin_is_refused() {
    // The builtin may write the argument back; what the Variant holds then is not pinned.
    for decl in [LABEL, TEXTCONST] {
        let r = run(decl, "Evaluate(V, 'xyz'); Error('R=done');");
        assert!(r.starts_with("refused:") && r.contains("by reference"), "{decl}: {r}");
    }
}

/// A codeunit with `Subtype = Test` (`subtype`) whose procedure `F` (with `attrs`) runs `body`; `Shape(V)` says what a
/// Variant holds.
fn sites(subtype: &str, attrs: &str, decl: &str, body: &str) -> String {
    let src = format!(
        "codeunit 50200 C {{ {subtype} var Simple: {decl}; G: Variant; \
         local procedure Take(V: Variant): Text begin exit(Shape(V)); end; \
         [TryFunction] local procedure TakeTry(V: Variant) begin Error(Shape(V)); end; \
         procedure TakePublic(V: Variant): Text begin exit(Shape(V)); end; \
         procedure Give(): Variant begin exit(Simple); end; \
         local procedure Shape(V: Variant): Text begin if V.IsTextConstant() then exit('TC:' + Format(V)); \
         if V.IsText() then exit('T:' + Format(V)); exit('?'); end; \
         {attrs} procedure F() var V: Variant; begin {body} end; }}"
    );
    out(al_testkit::run_in(&src, "C", "F"))
}

#[test]
fn a_text_constant_goes_into_a_variant_only_at_the_pinned_sites() {
    // Pinned (types/variant/VaTTextConst.al:137-149, :274-289): in a [Test] procedure of a test codeunit, a plain Label or ENU
    // TextConst assigned to a local Variant, and a global TextConst handed to a bare `local procedure` with a by-value Variant
    // parameter, make the Variant a TextConstant. Every other site keeps the earlier answer: a Label is a Text there, a
    // TextConst is refused.
    let run = |decl: &str, body: &str| sites("Subtype = Test;", "[Test]", decl, body);
    for decl in [TEXTCONST, LABEL] {
        assert_eq!(run(decl, "V := Simple; Error(Shape(V));"), "TC:abc", "{decl}");
    }
    assert_eq!(run(TEXTCONST, "Error(Take(Simple));"), "TC:abc");
    // A Label at an unpinned site or of an unpinned form stays a Text.
    for body in [
        "Error(Take(Simple));",
        "Error(TakePublic(Simple));",
        "TakeTry(Simple);",
        "V := Give(); Error(Shape(V));",
        "G := Simple; Error(Shape(G));",
    ] {
        assert_eq!(run(LABEL, body), "T:abc", "{body}");
    }
    for decl in [
        "Label 'abc', Locked = true",
        "Label 'abc', Comment = 'c'",
        "Label 'abc', MaxLength = 10",
    ] {
        assert_eq!(run(decl, "V := Simple; Error(Shape(V));"), "T:abc", "{decl}");
    }
    assert_eq!(run("Label ''", "V := Simple; Error(Shape(V));"), "T:");
    // The routine is not a [Test] procedure of a test codeunit: a Label stays a Text, a TextConst is refused.
    for (subtype, attrs) in [("", ""), ("Subtype = Test;", ""), ("", "[TryFunction]")] {
        let r = sites(subtype, attrs, LABEL, "V := Simple; Error(Shape(V));");
        assert_eq!(r, "T:abc", "{subtype} {attrs}");
        let r = sites(subtype, attrs, TEXTCONST, "V := Simple; Error(Shape(V));");
        assert!(
            r.starts_with("refused:") && r.contains("TextConst"),
            "{subtype} {attrs}: {r}"
        );
    }
    // A TextConst at an unpinned site or of an unpinned form is refused.
    for (decl, body) in [
        (TEXTCONST, "Error(TakePublic(Simple));"),
        (TEXTCONST, "TakeTry(Simple);"),
        (TEXTCONST, "V := Give(); Error(Shape(V));"),
        (TEXTCONST, "G := Simple; Error(Shape(G));"),
        ("TextConst ENU = 'abc', DEU = 'Hallo'", "V := Simple; Error(Shape(V));"),
        ("TextConst ENU = 'abc', Comment = 'c'", "V := Simple; Error(Shape(V));"),
        ("TextConst ENU = ''", "V := Simple; Error(Shape(V));"),
    ] {
        let r = run(decl, body);
        assert!(
            r.starts_with("refused:") && r.contains("TextConst"),
            "{decl} {body}: {r}"
        );
    }
    // A TextConst read as a text stays supported.
    assert_eq!(run(TEXTCONST, "Error(Simple);"), "abc");
}

#[test]
fn a_table_procedure_keeps_a_label_a_text() {
    let src = "table 50202 R { fields { field(1; K; Integer) { } } \
               procedure F() var V: Variant; L: Label 'abc'; begin V := L; if V.IsText() then Error('T'); Error('other'); end; }";
    assert_eq!(out(al_testkit::run_in(src, "R", "F")), "T");
}
