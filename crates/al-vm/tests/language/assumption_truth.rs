//! Paths that ran under an assumption although the corpus pins them (or pins them otherwise). Each test checks the
//! pinned value and that the run marks no assumption of that path.

use al_testkit::text;

/// Runs `F` of codeunit `T` and returns its text and the ids of the assumptions the run marked.
fn run(src: &str) -> (String, Vec<&'static str>) {
    al_builtins::assumptions::clear_hits();
    let got = text(al_testkit::run_in(src, "T", "F"));
    let hits = al_builtins::assumptions::take_hits()
        .iter()
        .map(|a| a.info().id)
        .collect();
    (got, hits)
}

const FOR_SRC: &str = r#"
enum 69860 "AT Kind" { Extensible = true; value(0; Red) { } value(1; Green) { } value(5; Blue) { } }
table 69861 "AT Row"
{
    fields { field(1; Id; Integer) { } field(2; Opt; Option) { OptionMembers = A,B,C,D; } field(3; Kind; Enum "AT Kind") { } }
    keys { key(PK; Id) { } }
}
codeunit 69862 T
{
    procedure F(): Text
    var R: Record "AT Row"; Opt: Option A,B,C,D; K: Enum "AT Kind"; S: Text;
    begin
        for R.Opt := R.Opt::A to R.Opt::C do S += Format(R.Opt) + Format(R.Opt, 0, 2) + ',';
        S += Format(R.Opt) + ';';
        for Opt := Opt::D downto Opt::A do S += Format(Opt) + ',';
        S += Format(Opt) + ';';
        for K := K::Red to K::Blue do S += Format(K) + ',';
        S += Format(K) + ';';
        for K := K::Blue downto K::Red do S += Format(K) + ',';
        S += Format(K) + ';';
        for R.Kind := R.Kind::Red to R.Kind::Green do S += Format(R.Kind) + ',';
        S += Format(R.Kind);
        exit(S);
    end;
}
"#;

/// types/lt-for-option-time-decimal/ForLoopOptionVariable.al:31-54 (field A to C, ordinals), :221-237 (variable
/// downto); ForOptionEnum_risky.al:39-60 (enum with a gap: Red,Green,2,3,4,Blue), :83-104 (downto), :62-81 (field).
#[test]
fn a_for_loop_over_an_option_or_enum_counter_is_pinned() {
    let (got, hits) = run(FOR_SRC);
    assert_eq!(
        got,
        "A0,B1,C2,C;D,C,B,A,A;Red,Green,2,3,4,Blue,Blue;Blue,4,3,2,Green,Red,Red;Red,Green,Green"
    );
    assert!(!hits.contains(&"enum-option-integer-ordinal"), "{hits:?}");
}

const BIG_SRC: &str = r#"
enum 69863 "AT BE" { Extensible = false; value(0; Zero) { } value(1; One) { } value(2; Two) { } value(7; Seven) { } }
table 69864 "AT Big"
{
    fields { field(1; Id; Integer) { } field(2; Opt; Option) { OptionMembers = A,B,C; } }
    keys { key(PK; Id) { } }
}
codeunit 69865 T
{
    var Opt: Option Zero,One,Two; E: Enum "AT BE";
    [TryFunction]
    local procedure SetOpt(Big: BigInteger) begin Opt := Big; end;
    [TryFunction]
    local procedure SetEnum(Big: BigInteger) begin E := Big; end;
    local procedure Take(O: Option A,B,C): Text begin exit(Format(O) + '/' + Format(O, 0, 2)); end;
    procedure F(): Text
    var Big: BigInteger; S: Text; R: Record "AT Big";
    begin
        Big := 3;
        Opt := Big;
        S += Format(Opt) + ',';
        Big := -1;
        Opt := Big;
        S += Format(Opt) + ',';
        Big := 7;
        E := Big;
        S += Format(E) + ',';
        Big := 3;
        E := Big;
        S += Format(E) + ',';
        Big := 1;
        S += Take(Big) + ',';
        R.Opt := Big;
        S += Format(R.Opt) + ',';
        Big := 99;
        R.Validate(Opt, Big);
        S += Format(R.Opt) + ';';
        Evaluate(Big, '2147483648');
        if not SetOpt(Big) then
            S += GetLastErrorText() + '|' + GetLastErrorCode() + ';';
        Evaluate(Big, '-2147483649');
        if not SetEnum(Big) then
            S += GetLastErrorText() + '|' + GetLastErrorCode();
        exit(S);
    end;
}
"#;

/// types/lt-bigint-enum-option/BigIntToEnumOption.al:95-121 (Option: lacking and negative ordinals kept), :123-145
/// (beyond Integer: ConversionCast), :148-171 and :173-195 (Enum on BC 29); types/lt-implicit-conversions/P79BigOption.al:194-218
/// (argument, field, Validate).
#[test]
fn a_biginteger_into_an_option_or_enum_is_pinned() {
    let (got, hits) = run(BIG_SRC);
    assert_eq!(
        got,
        "3,-1,Seven,3,B/1,B,99;\
         Overflow under conversion of System.Int64 value 2147483648 to System.Int32.|ConversionCast;\
         Overflow under conversion of System.Int64 value -2147483649 to System.Int32.|ConversionCast"
    );
    assert!(!hits.contains(&"enum-option-integer-ordinal"), "{hits:?}");
}

const REL_SRC: &str = r#"
enum 69866 "AT TQ Kind" { Extensible = false; value(0; A) { } value(1; "B C") { } value(2; D) { } }
table 69867 "AT TQ Ref"
{
    fields { field(1; "Code"; Code[10]) { } field(2; Kind; Enum "AT TQ Kind") { } field(3; Opt; Option) { OptionMembers = A,"B C",D; } }
    keys { key(PK; "Code") { } }
}
table 69868 "AT TQ Src"
{
    fields
    {
        field(1; "No."; Code[10]) { }
        field(2; "By Kind"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter(A | "B C")); }
        field(3; "By Opt"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Opt = filter(A | "B C")); }
        field(4; "By Quoted Only"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter("B C")); }
        field(6; "By Const"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = const("B C")); }
        field(7; "By Not"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter(<> "B C")); }
        field(8; "By Range"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter(A .. "B C")); }
    }
    keys { key(PK; "No.") { } }
}
codeunit 69869 T
{
    [TryFunction]
    local procedure TryValidate(var S: Record "AT TQ Src"; No: Integer; C: Code[10])
    var F: FieldRef; R: RecordRef;
    begin
        R.GetTable(S);
        F := R.Field(No);
        F.Validate(C);
    end;
    procedure Seed()
    var R: Record "AT TQ Ref";
    begin
        R.DeleteAll();
        R.Code := 'R1'; R.Kind := R.Kind::A; R.Opt := R.Opt::A; R.Insert();
        R.Code := 'R2'; R.Kind := R.Kind::"B C"; R.Opt := R.Opt::"B C"; R.Insert();
        R.Code := 'R3'; R.Kind := R.Kind::D; R.Opt := R.Opt::D; R.Insert();
    end;
    procedure One(No: Integer; C: Code[10]): Text
    var S: Record "AT TQ Src";
    begin
        if TryValidate(S, No, C) then exit('ok,');
        exit('err,');
    end;
    procedure Probe(No: Integer): Text
    begin
        exit(One(No, 'R1') + One(No, 'R2') + One(No, 'R3'));
    end;
    procedure F(): Text
    begin
        Seed();
        exit(Probe(2) + ';' + Probe(3) + ';' + Probe(4) + ';' + Probe(6) + ';' + Probe(7));
    end;
    procedure G(): Text
    begin
        Seed();
        exit(Probe(8));
    end;
}
"#;

/// record/lt-table-relation-quotes/TableRelationQuotes.al:41-46 (the relations), :126-207 (R1 = A, R2 = "B C", R3 = D):
/// `A | "B C"` on an enum and an option field, `"B C"` alone, const("B C") and `<> "B C"`.
#[test]
fn a_quoted_member_in_a_table_relation_filter_is_pinned() {
    let (got, hits) = run(&REL_SRC
        .replace("procedure F()", "procedure F_()")
        .replace("procedure G()", "procedure F()"));
    assert!(
        hits.contains(&"table-relation-quotes"),
        "a range with a quoted member is not pinned: {hits:?} {got}"
    );
    let (got, hits) = run(REL_SRC);
    assert_eq!(got, "ok,ok,err,;ok,ok,err,;err,ok,err,;err,ok,err,;ok,err,ok,");
    assert!(!hits.contains(&"table-relation-quotes"), "{hits:?}");
}

const NL_OBJECTS: &str = r#"
enum 69870 "AT NL Type" { Extensible = false; value(0; A) { } value(1; B) { } }
table 69871 "AT NL Header" { fields { field(1; "No."; Code[10]) { } field(2; Type; Enum "AT NL Type") { } } keys { key(PK; "No.") { } } }
table 69872 "AT NL Line"
{
    fields
    {
        field(1; "Document No."; Code[10])
        {
            TableRelation = "AT NL Header"."No." where (Type = field(Type));
            trigger OnValidate() begin Seen := Format(Type); end;
        }
        field(2; "Line No."; Integer) { }
        field(3; Type; Enum "AT NL Type") { }
        field(4; Seen; Text[20]) { }
        field(5; Descr; Text[30]) { }
    }
    keys { key(PK; KEYFIELDS) { } }
}
page 69873 "AT NL Lines B"
{
    PageType = ListPart; SourceTable = "AT NL Line"; SourceTableView = where(Type = const(B)); AutoSplitKey = true;
    layout { area(Content) { repeater(Rows) {
        field("Document No."; Rec."Document No.") { } field(Type; Rec.Type) { } field(Seen; Rec.Seen) { } field(Descr; Rec.Descr) { } } } }
}
page 69874 "AT NL Host"
{
    PageType = Card; SourceTable = "AT NL Header";
    layout { area(Content) {
        field("No."; Rec."No.") { } field(Type; Rec.Type) { }
        part(LinesB; "AT NL Lines B") { SubPageLink = "Document No." = field("No."); } } }
}
codeunit 69875 T
{
    procedure F(): Text
    var H: Record "AT NL Header"; L: Record "AT NL Line"; Host: TestPage "AT NL Host"; R: Text;
    begin
        L.DeleteAll(); H.DeleteAll();
        H."No." := 'HA'; H.Type := H.Type::A; H.Insert();
        Host.OpenEdit();
        Host.LinesB.New();
        R := Host.LinesB."Document No.".Value() + '|' + Host.LinesB.Type.Value() + '|' + Host.LinesB.Seen.Value();
        Host.LinesB.Descr.SetValue('x');
        Host.Close();
        L.FindFirst();
        exit(R + ';' + L."Document No." + '|' + Format(L.Type) + '|' + L.Seen + '|' + L.Descr);
    end;
}
"#;

/// pages/lt-new-line-stamps-filters/NewLineStampsFilters.al:214-237 PartB_ExistingHeaderTypeA_NewLine: the part's
/// SourceTableView `Type = const(B)` is not stamped on the blank line, the link field's OnValidate sees A.
#[test]
fn a_new_part_line_keeps_a_view_field_outside_the_key_unstamped() {
    let (got, hits) = run(&NL_OBJECTS.replace("KEYFIELDS", "\"Document No.\", \"Line No.\""));
    assert_eq!(got, "HA|A|A;HA|A|A|x");
    assert!(!hits.contains(&"new-line-stamps-all-before-validating"), "{hits:?}");
}

/// Two key fields with a single filter value (Type from the view, Document No. from the link), validated on the new
/// line: whether the second is on the record when the first validates is not pinned (NewLineStampsFilters.al stamps
/// one key field only). The run keeps stamping both first and marks the assumption.
#[test]
fn a_new_part_line_with_two_stamped_key_fields_runs_under_the_assumption() {
    let src = NL_OBJECTS.replace("KEYFIELDS", "Type, \"Document No.\", \"Line No.\"");
    al_builtins::assumptions::clear_hits();
    let _ = al_testkit::run_in(&src, "T", "F");
    let hits: Vec<_> = al_builtins::assumptions::take_hits()
        .iter()
        .map(|a| a.info().id)
        .collect();
    assert!(hits.contains(&"new-line-stamps-all-before-validating"), "{hits:?}");
}

const DATE_SRC: &str = r#"
table 69876 "AT P147 Dates" { fields { field(1; Id; Integer) { } field(2; D; Date) { } field(3; Txt; Text[30]) { } field(4; Cd; Code[20]) { } } keys { key(PK; Id) { } } }
codeunit 69877 T
{
    local procedure TakeText(Value: Text): Text begin exit(Value); end;
    local procedure DateVariant(): Variant
    var Rec: Record "AT P147 Dates"; RecRef: RecordRef; FldRef: FieldRef;
    begin
        Rec.DeleteAll();
        Rec.Id := 1; Rec.D := DMY2Date(5, 3, 2024); Rec.Insert();
        RecRef.GetTable(Rec);
        FldRef := RecRef.Field(2);
        exit(FldRef.Value);
    end;
    [TryFunction]
    local procedure TryParam(V: Variant; var R: Text) begin R := TakeText(V); end;
    [TryFunction]
    local procedure TryText(V: Variant; var R: Text) var T: Text; begin T := V; R := T; end;
    [TryFunction]
    local procedure TryCode(V: Variant; var R: Text) var C: Code[20]; begin C := V; R := C; end;
    [TryFunction]
    local procedure TryTextField(V: Variant; var R: Text) var Rec: Record "AT P147 Dates"; begin Rec.Txt := V; R := Rec.Txt; end;
    [TryFunction]
    local procedure TryCodeField(V: Variant; var R: Text) var Rec: Record "AT P147 Dates"; begin Rec.Cd := V; R := Rec.Cd; end;
    local procedure Res(Ok: Boolean; R: Text): Text
    begin
        if Ok then exit('[' + R + ']');
        exit(GetLastErrorCode() + ':' + GetLastErrorText());
    end;
    procedure F(): Text
    var V: Variant; D: Date; R: Text; S: Text;
    begin
        V := DateVariant();
        S += Res(TryParam(V, R), R) + ';';
        S += Res(TryText(V, R), R) + ';';
        S += Res(TryCode(V, R), R) + ';';
        S += Res(TryTextField(V, R), R) + ';';
        S += Res(TryCodeField(V, R), R) + ';';
        D := DMY2Date(5, 3, 2024);
        V := D;
        S += Res(TryCode(V, R), R);
        exit(S);
    end;
}
"#;

/// report/lt-requestpage-savevalues-date-text-setvalue/P147DateToText.al:89-105: a Variant holding a Date (from
/// FieldRef.Value) into a Text parameter raises TypeConversion; into a Text variable too (types/lt-variant/VaTCompare.al).
/// A Code variable and a Text or Code field raise the same error with the Code text where the target is a Code
/// (types/lt-variant-to-text-fields/VariantNumberToField.al pins the NavCode text).
#[test]
fn a_variant_holding_a_date_into_a_text_or_code_raises() {
    let (got, hits) = run(DATE_SRC);
    let t = "TypeConversion:The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavText value.";
    let c = "TypeConversion:The expression Microsoft.Dynamics.Nav.Runtime.NavIndirectValue cannot be type-converted to a Microsoft.Dynamics.Nav.Runtime.NavCode value.";
    assert_eq!(got, format!("{t};{t};{c};{t};{c};{c}"));
    assert!(!hits.contains(&"date-to-text-is-format"), "{hits:?}");
}

/// types/lt-implicit-conversions/P79BigOption.al:284-302 Field_ValidateBigToOption: Validate of an Option field with a
/// BigInteger beyond Integer raises Overflow, not the ConversionCast of an assignment.
#[test]
fn validate_of_an_option_field_with_a_biginteger_beyond_integer_raises_overflow() {
    let src = r#"
table 69878 "AT VB" { fields { field(1; Id; Integer) { } field(2; Opt; Option) { OptionMembers = A,B,C; } } keys { key(PK; Id) { } } }
codeunit 69879 T
{
    [TryFunction]
    local procedure TryValidate(Big: BigInteger) var R: Record "AT VB"; begin R.Validate(Opt, Big); end;
    procedure F(): Text
    var Big: BigInteger;
    begin
        Evaluate(Big, '2147483648');
        if TryValidate(Big) then exit('ok');
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;
    let (got, _) = run(src);
    assert_eq!(got, "Arithmetic operation resulted in an overflow.|AL:Overflow");
}

/// A quoted blank member (`filter(" " | A)`, Base Application's Warehouse Activity Line) is not among the pinned forms.
#[test]
fn a_quoted_blank_member_in_a_table_relation_filter_keeps_the_mark() {
    let src = r#"
table 69880 "AT TB Ref" { fields { field(1; "Code"; Code[10]) { } field(2; Opt; Option) { OptionMembers = " ",A,"B C"; } } keys { key(PK; "Code") { } } }
table 69881 "AT TB Src" { fields { field(1; "No."; Code[10]) { } field(2; "By Blank"; Code[10]) { TableRelation = "AT TB Ref"."Code" where (Opt = filter(" " | A)); } } keys { key(PK; "No.") { } } }
codeunit 69882 T
{
    procedure F(): Text
    var R: Record "AT TB Ref"; S: Record "AT TB Src";
    begin
        R.Code := 'R1'; R.Opt := R.Opt::A; R.Insert();
        S.Validate("By Blank", 'R1');
        exit(S."By Blank");
    end;
}
"#;
    let (got, hits) = run(src);
    assert_eq!(got, "R1");
    assert!(hits.contains(&"table-relation-quotes"), "{hits:?}");
}
