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
        field(8; "By Open Lower"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter(.. "B C")); }
        field(9; "Range AB"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter(A .. "B C")); }
        field(10; "Range BD"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter("B C" .. D)); }
        field(11; "Kind And"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter(<> A & <> "B C")); }
        field(12; "Opt And"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Opt = filter(<> "B C" & <> D)); }
        field(13; "Open Upper"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Kind = filter("B C" ..)); }
        field(14; "Opt Range BD"; Code[10]) { TableRelation = "AT TQ Ref"."Code" where (Opt = filter("B C" .. D)); }
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
    procedure H(): Text
    begin
        Seed();
        exit(Probe(9) + ';' + Probe(10) + ';' + Probe(11) + ';' + Probe(12) + ';' + Probe(13) + ';' + Probe(14));
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
        "an open lower range with a quoted member is not pinned: {hits:?} {got}"
    );
    assert_eq!(got, "ok,ok,err,");
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

/// types/lt-bigint-enum-option/BigIntToEnumOption.al:353-449 (corpus bump 45): a Variant holding a BigInteger beyond Integer
/// into an Option variable or field raises TypeConversion:Overflow; through FieldRef.Value or FieldRef.Validate of an Option or
/// an Enum field it raises AL:Overflow. In range, missing and negative ordinals are kept.
#[test]
fn a_variant_holding_a_biginteger_into_an_option_or_enum_is_pinned() {
    let src = r#"
enum 69890 "AT VE" { Extensible = true; value(0; Zero) { } value(1; One) { } value(7; Seven) { } }
table 69891 "AT VO" { fields { field(1; Id; Integer) { } field(2; Opt; Option) { OptionMembers = Zero,One,Two; } field(3; E; Enum "AT VE") { } } keys { key(PK; Id) { } } }
codeunit 69892 T
{
    [TryFunction]
    local procedure TryIt(Kind: Integer; V: Variant; var Res: Text)
    var R: Record "AT VO"; O: Option Zero,One,Two; RecRef: RecordRef; FR: FieldRef;
    begin
        case Kind of
            1: begin O := V; Res := Format(O, 0, 2); exit; end;
            3: R.Opt := V;
            7, 9, 10:
                begin
                    RecRef.GetTable(R);
                    if Kind = 7 then FR := RecRef.Field(2) else FR := RecRef.Field(3);
                    if Kind = 10 then FR.Validate(V) else FR.Value := V;
                    RecRef.SetTable(R);
                end;
        end;
        if Kind in [3, 7] then Res := Format(R.Opt, 0, 2) else Res := Format(R.E.AsInteger());
    end;
    local procedure Probe(Kind: Integer; V: Variant): Text
    var Res: Text;
    begin
        if TryIt(Kind, V, Res) then exit(Res);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;
    procedure F(): Text
    var Big: BigInteger; Small: BigInteger; S: Text; Kind: Integer;
    begin
        Evaluate(Big, '2147483648');
        Small := -1;
        for Kind := 1 to 10 do
            if Kind in [1, 3, 7, 9, 10] then
                S += Probe(Kind, Small) + ';' + Probe(Kind, Big) + ';';
        exit(S);
    end;
}
"#;
    let (got, _) = run(src);
    let tc = "-1;The expression Variant cannot be type-converted to a Integer value.|TypeConversion:Overflow;";
    let ov = "-1;Arithmetic operation resulted in an overflow.|AL:Overflow;";
    assert_eq!(got, format!("{tc}{tc}{ov}{ov}{ov}"));
}

/// record/lt-table-relation-quotes/TableRelationQuotes.al:279-397 (corpus bump 45, R1 = A, R2 = "B C", R3 = D): the ranges
/// `A .. "B C"`, `"B C" .. D` (enum and option field), `"B C" ..` and `<> A & <> "B C"`, `<> "B C" & <> D` run without the mark.
#[test]
fn a_quoted_member_in_a_table_relation_range_or_and_is_pinned() {
    let (got, hits) = run(&REL_SRC
        .replace("procedure F()", "procedure F_()")
        .replace("procedure H()", "procedure F()"));
    assert_eq!(
        got,
        "ok,ok,err,;err,ok,ok,;err,err,ok,;ok,err,err,;err,ok,ok,;err,ok,ok,"
    );
    assert!(!hits.contains(&"table-relation-quotes"), "{hits:?}");
}

/// report/lt-requestpage-savevalues-date-text-setvalue/P147DateToText.al:297-489 (corpus bump 45): `FieldRef.Value` of a
/// Date, Time or DateTime field handed straight to a Text, Text[30] or Code[20] parameter or assigned to a Text or Code
/// variable is its Format text (a Code drops the leading blank of a Time); a Variant holding one raises TypeConversion.
#[test]
fn a_date_time_or_datetime_into_text_is_pinned() {
    let src = r#"
table 69893 "AT DT" { fields { field(1; Id; Integer) { } field(2; D; Date) { } field(3; DT; DateTime) { } field(4; T; Time) { } } keys { key(PK; Id) { } } }
codeunit 69894 T
{
    local procedure TakeText(X: Text): Text begin exit(X); end;
    local procedure TakeText30(X: Text[30]): Text begin exit(X); end;
    local procedure TakeCode20(X: Code[20]): Text begin exit(X); end;
    [TryFunction]
    local procedure FromVariant(Kind: Integer; V: Variant; var Res: Text)
    var AsText: Text; AsCode: Code[20];
    begin
        case Kind of
            1: begin AsText := V; Res := AsText; end;
            2: Res := TakeText(V);
            3: Res := TakeText30(V);
            4: Res := TakeCode20(V);
            5: begin AsCode := V; Res := AsCode; end;
        end;
    end;
    local procedure FromFieldRef(Kind: Integer; FR: FieldRef): Text
    var AsText: Text; AsCode: Code[20];
    begin
        case Kind of
            1: begin AsText := FR.Value; exit(AsText); end;
            2: exit(TakeText(FR.Value));
            3: exit(TakeText30(FR.Value));
            4: exit(TakeCode20(FR.Value));
            5: begin AsCode := FR.Value; exit(AsCode); end;
        end;
    end;
    procedure F(): Text
    var R: Record "AT DT"; RecRef: RecordRef; V: Variant; S: Text; Res: Text; Kind: Integer; No: Integer; D: Date;
    begin
        R.Id := 1; R.D := DMY2Date(5, 3, 2024); R.T := 134500T; R.DT := CreateDateTime(DMY2Date(5, 3, 2024), 134500T); R.Insert();
        RecRef.GetTable(R);
        for No := 2 to 4 do begin
            for Kind := 1 to 5 do
                S += '[' + FromFieldRef(Kind, RecRef.Field(No)) + ']';
            S += ';';
        end;
        Evaluate(D, FromFieldRef(1, RecRef.Field(2)));
        S += Format(D, 0, 9) + ';';
        for No := 3 to 4 do begin
            if No = 3 then V := R.DT else V := R.T;
            for Kind := 1 to 5 do
                if FromVariant(Kind, V, Res) then S += 'ok,' else S += CopyStr(GetLastErrorText(), 94) + ',';
            S += ';';
        end;
        exit(S);
    end;
}
"#;
    let (got, hits) = run(src);
    let dt = "[03/05/24 01:45 PM]";
    let t = "Microsoft.Dynamics.Nav.Runtime.NavText value.,";
    let c = "Microsoft.Dynamics.Nav.Runtime.NavCode value.,";
    let raised = format!("{t}{t}{t}{c}{c};");
    assert_eq!(
        got,
        format!(
            "[03/05/24][03/05/24][03/05/24][03/05/24][03/05/24];{dt}{dt}{dt}{dt}{dt};[ 1:45:00 PM][ 1:45:00 PM][ 1:45:00 PM][1:45:00 PM][1:45:00 PM];2024-03-05;{raised}{raised}"
        )
    );
    assert!(!hits.contains(&"date-to-text-is-format"), "{hits:?}");
}
