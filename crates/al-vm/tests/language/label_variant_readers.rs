//! A Label put into a Variant now holds a TextConstant (lower/label_variant.rs). Where that Variant is read as text, it gives
//! the text, as main did when the Variant held a Text (`conv::variant::text_const_as_text`; types/Y81LabelVariant.al).

const SRC: &str = r#"
table 60906 Row
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[50]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 60907 C
{
    var
        GLbl: Label 'Global text';

    procedure FieldAssign(): Text
    var V: Variant; Row: Record Row temporary; Lbl: Label 'Local';
    begin
        V := Lbl;
        Row.Name := V;
        exit(Row.Name);
    end;

    procedure GlobalRecordUses(): Text
    var V: Variant; Row: Record Row temporary;
    begin
        V := GLbl;
        Row.Id := 1;
        Row.Name := V;
        Row.Insert();
        Row.SetFilter(Name, '%1', V);
        exit(Row.Name + '|' + Format(Row.Count()) + '|' + Row.GetFilter(Name));
    end;

    procedure ValidateAndEvaluate(): Text
    var V: Variant; Row: Record Row temporary; I: Integer; L: Label '42';
    begin
        V := L;
        Evaluate(I, V);
        Row.Validate(Name, V);
        exit(Format(I) + '|' + Row.Name);
    end;

    procedure FieldRefUses(): Text
    var V: Variant; RR: RecordRef; FR: FieldRef; L: Label 'Local';
    begin
        V := L;
        RR.Open(Database::Row, true);
        FR := RR.Field(2);
        FR.Value := V;
        RR.Field(1).Value := 1;
        RR.Insert();
        FR.SetRange(V);
        exit(Format(FR.Value) + '|' + Format(RR.Count()) + '|' + FR.GetFilter());
    end;

    procedure ListAdd(): Text
    var V: Variant; L: List of [Text]; Lbl: Label 'Local';
    begin
        V := Lbl;
        L.Add(V);
        exit(L.Get(1));
    end;

    procedure StrSubstNoFormat(): Text
    var V: Variant; Lbl: Label 'Local %1';
    begin
        V := Lbl;
        exit(StrSubstNo(V, 5) + '|' + StrSubstNo(Format(V), 6));
    end;

    procedure VariantIntoVariant(): Text
    var V: Variant; W: Variant; Lbl: Label 'Local';
    begin
        V := Lbl;
        W := V;
        exit(Format(W.IsTextConstant()) + '|' + Format(W.IsText()));
    end;
}
"#;

fn run(routine: &str) -> String {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
}

// Main's value for each shape (main's Variant held a Text there).
#[test]
fn a_label_in_a_variant_reads_as_its_text() {
    assert_eq!(run("FieldAssign"), "Local");
    assert_eq!(run("GlobalRecordUses"), "Global text|1|Global text");
    assert_eq!(run("ValidateAndEvaluate"), "42|42");
    assert_eq!(run("FieldRefUses"), "Local|1|Local");
    assert_eq!(run("ListAdd"), "Local");
    assert_eq!(run("StrSubstNoFormat"), "Local 5|Local 6");
}

#[test]
fn a_variant_into_a_variant_keeps_the_text_constant() {
    assert_eq!(run("VariantIntoVariant"), "Yes|No");
}
