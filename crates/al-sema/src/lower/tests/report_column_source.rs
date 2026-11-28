//! A report column source qualified with its own data item's name when that name holds a dot (al-runner issue 9330):
//! `"A. B".F` in data item `"A. B"` is field F of the own item in the shapes corpus PR 1652 measures (assumption
//! `report-column-dotted-item-qualifier`), `"A. B".F.G` is no field, and every other dotted shape keeps main's answer
//! (no field). A quoted qualifier of the own item: streams/P138ColCaptionQualified.al:251.
use super::*;
use al_builtins::assumptions::Assumption;

const TABLE: &str = "table 50100 T { fields {
    field(1; K; Integer) { Caption = 'K cap'; }
    field(2; Name; Text[30]) { Caption = 'Name cap'; }
    field(3; Qty; Decimal) { Caption = 'Qty cap'; DecimalPlaces = 0 : 3; }
    field(4; Amt; Decimal) { Caption = 'Amt cap'; AutoFormatType = 1; }
    field(5; Unit; Decimal) { Caption = 'Unit cap'; AutoFormatType = 2; }
    field(6; AmtPlaces; Decimal) { Caption = 'AmtPlaces cap'; AutoFormatType = 1; DecimalPlaces = 0 : 3; }
    field(7; \"Unit Amt.\"; Decimal) { DecimalPlaces = 0 : 1; }
    field(8; Plain; Decimal) { }
    field(9; Code1; Code[20]) { Caption = 'Code cap'; }
    field(10; Bare; Text[30]) { }
    field(11; Unm; Decimal) { AutoFormatType = 50733; }
    field(12; Expr; Decimal) { AutoFormatType = 1; AutoFormatExpression = ''; }
    field(13; Pic; Media) { }
    field(14; Ml; Text[30]) { CaptionML = ENU = 'Ml cap'; }
    field(15; St; Enum E) { Caption = 'St cap'; }
    field(16; Kind; Option) { Caption = 'Kind cap'; OptionMembers = \" \",Open; OptionCaption = ' ,Open cap'; }
    field(17; Due; Date) { Caption = 'Due cap'; }
    field(18; Bin; Code[10]) { }
    field(19; Seq; Integer) { }
    field(20; Q5; Decimal) { AutoFormatType = 0; Caption = 'Q5 cap'; DecimalPlaces = 0 : 5; }
    field(21; UnitExpr; Decimal) { AutoFormatType = 2; AutoFormatExpression = ''; Caption = 'UnitExpr cap'; }
    field(22; Zero; Decimal) { AutoFormatType = 0; }
    field(23; StNoCap; Enum E) { }
    field(24; DueNoCap; Date) { }
    field(25; Flag; Boolean) { Caption = 'Flag cap'; }
    field(26; Q3Zero; Decimal) { AutoFormatType = 0; DecimalPlaces = 0 : 3; }
    field(27; ZeroExpr; Decimal) { AutoFormatType = 0; AutoFormatExpression = ''; }
    field(28; UnitCur; Decimal) { AutoFormatType = 2; AutoFormatExpression = 'USD'; }
    field(29; KindNoCap; Option) { OptionMembers = A,B; }
    field(30; UnitNoCap; Decimal) { AutoFormatType = 2; }
} keys { key(PK; K) { } } }
enum 50100 E { value(0; A) { Caption = 'A cap'; } value(3; B) { Caption = 'B cap'; } }";

/// The (item field, report assumptions) of each column of the first data item of report R.
fn lowered(report: &str) -> (Vec<(String, Option<usize>)>, Vec<Assumption>) {
    let (ws, l) = lower_src(&format!("{TABLE}\n{report}"));
    let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
    let r = l.program.objects[oi].report.as_ref().unwrap();
    let columns = r.items[0]
        .columns
        .iter()
        .map(|c| (c.name.clone(), c.item_field))
        .collect();
    (columns, r.assumes.clone())
}

#[test]
fn a_source_qualified_with_a_dotted_own_item_name_is_its_field() {
    let (ws, l) = lower_src(&format!(
        "{TABLE}
report 50100 R {{ dataset {{ dataitem(\"A. B\"; T) {{
    column(X; \"A. B\".Qty) {{ IncludeCaption = true; }}
    column(Y; \"A. B\".Qty.G) {{ }}
}} }} }}"
    ));
    let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
    let columns = &l.program.objects[oi].report.as_ref().unwrap().items[0].columns;
    let fields = |c: &al_ir::ReportColumn| (c.item_field, c.caption_field, c.places_field);
    assert_eq!(fields(&columns[0]), (Some(2), Some(2), Some(2)), "{}", columns[0].name);
    assert_eq!(fields(&columns[1]), (None, None, None), "{}", columns[1].name);
}

/// The measured cells of DataItemNamesDottedQualifier.al (:75-82, :145-155) resolve to their field and flag the report.
#[test]
fn the_measured_dotted_shapes_resolve_through_the_assumption() {
    for item in ["Dq. Row", "Dq.Row", "Dq. R. Two"] {
        let (columns, assumes) = lowered(&format!(
            "report 50100 R {{ dataset {{ dataitem(\"{item}\"; T) {{
    column(DName; \"{item}\".Name) {{ IncludeCaption = true; }}
    column(DQty; \"{item}\".Qty) {{ IncludeCaption = true; }}
    column(DAmt; \"{item}\".Amt) {{ IncludeCaption = true; }}
    column(DUnitAmt; \"{item}\".\"Unit Amt.\") {{ IncludeCaption = true; }}
    column(DPlain; \"{item}\".Plain) {{ IncludeCaption = true; }}
    column(DQtyOwnPlaces; \"{item}\".Qty) {{ DecimalPlaces = 4 : 4; }}
}} }} }}"
        ));
        let want = [1, 2, 3, 6, 7, 2];
        for ((name, field), pos) in columns.iter().zip(want) {
            assert_eq!(*field, Some(pos), "{item}: {name}");
        }
        assert_eq!(assumes, [Assumption::ReportColumnDottedItemQualifier], "{item}");
    }
}

/// The measured cells of part 2 (DataItemNamesDottedQualifier.al:383-400 at cf6884b9, labels :565-567, columns :596-633):
/// Code, Integer, Text, Date, Enum and Option fields with IncludeCaption; Decimal AutoFormatType 0 with DecimalPlaces 0:5 or
/// alone, AutoFormatType 2 and 1 with AutoFormatExpression ''; a column without property over AutoFormatType 0 + DecimalPlaces
/// 0:5 and over AutoFormatType 1 + AutoFormatExpression ''.
#[test]
fn the_measured_typed_dotted_shapes_resolve_through_the_assumption() {
    let (columns, assumes) = lowered(
        "report 50100 R { dataset { dataitem(\"Dq. Line\"; T) {
    column(LStatus; \"Dq. Line\".St) { IncludeCaption = true; }
    column(LOrderNo; \"Dq. Line\".Code1) { IncludeCaption = true; }
    column(LLineNo; \"Dq. Line\".K) { IncludeCaption = true; }
    column(LDescription; \"Dq. Line\".Name) { IncludeCaption = true; }
    column(LStartingDate; \"Dq. Line\".Due) { IncludeCaption = true; }
    column(LQuantity; \"Dq. Line\".Q5) { IncludeCaption = true; }
    column(LUnitCost; \"Dq. Line\".UnitExpr) { IncludeCaption = true; }
    column(LCostAmount; \"Dq. Line\".Expr) { IncludeCaption = true; }
    column(LKind; \"Dq. Line\".Kind) { IncludeCaption = true; }
    column(LBinCode; \"Dq. Line\".Bin) { IncludeCaption = true; }
    column(LSeq; \"Dq. Line\".Seq) { IncludeCaption = true; }
    column(LZeroFmt; \"Dq. Line\".Zero) { IncludeCaption = true; }
    column(LQuantityNoProp; \"Dq. Line\".Q5) { }
    column(LCostAmountNoProp; \"Dq. Line\".Expr) { }
    column(DUnit; \"Dq. Line\".Unit) { IncludeCaption = true; }
} } }",
    );
    let want = [14, 8, 0, 1, 16, 19, 20, 11, 15, 17, 18, 21, 19, 11, 4];
    for ((name, field), pos) in columns.iter().zip(want) {
        assert_eq!(*field, Some(pos), "{name}");
    }
    assert_eq!(columns.len(), want.len());
    assert_eq!(assumes, [Assumption::ReportColumnDottedItemQualifier]);
}

/// AutoFormatType 2 on the field, with no AutoFormatExpression or with '' (:34 and :354 at cf6884b9, columns :236 and :630,
/// undotted :692): the column takes the formatter of AutoFormatType 2 through assumption
/// `report-column-field-unit-amount-format`, dotted or not; every other shape keeps main's formatter.
#[test]
fn a_unit_amount_field_marks_its_measured_columns() {
    let (ws, l) = lower_src(&format!(
        "{TABLE}
report 50100 R {{ dataset {{ dataitem(\"Dq. Line\"; T) {{
    column(DUnit; \"Dq. Line\".Unit) {{ IncludeCaption = true; }}
    column(LUnitCost; \"Dq. Line\".UnitExpr) {{ IncludeCaption = true; }}
    column(NoProperty; \"Dq. Line\".UnitExpr) {{ }}
    column(Bare; UnitExpr) {{ IncludeCaption = true; }}
    column(Currency; \"Dq. Line\".UnitCur) {{ IncludeCaption = true; }}
    column(Amount; \"Dq. Line\".Expr) {{ IncludeCaption = true; }}
}}
dataitem(Undotted; T) {{
    column(ULUnitCost; Undotted.UnitExpr) {{ IncludeCaption = true; }}
    column(OtherCase; undotted.UnitExpr) {{ IncludeCaption = true; }}
    column(FieldOtherCase; Undotted.unitexpr) {{ IncludeCaption = true; }}
}} }} }}"
    ));
    let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
    let items = &l.program.objects[oi].report.as_ref().unwrap().items;
    let marks: Vec<_> = items
        .iter()
        .flat_map(|i| i.columns.iter())
        .map(|c| (c.name.as_str(), c.field_unit_amount_format))
        .collect();
    assert_eq!(
        marks,
        [
            ("DUnit", true),
            ("LUnitCost", true),
            ("NoProperty", false),
            ("Bare", false),
            ("Currency", false),
            ("Amount", false),
            ("ULUnitCost", true),
            ("OtherCase", false),
            ("FieldOtherCase", false),
        ]
    );
}

/// Dotted shapes the probe does not measure keep main's answer: no field, no assumption.
#[test]
fn an_unmeasured_dotted_shape_keeps_mains_answer() {
    let (columns, assumes) = lowered(
        "report 50100 R { dataset { dataitem(\"A. B\"; T) {
    column(EnumNoCaption; \"A. B\".StNoCap) { IncludeCaption = true; }
    column(DateNoCaption; \"A. B\".DueNoCap) { IncludeCaption = true; }
    column(OptionNoCaption; \"A. B\".KindNoCap) { IncludeCaption = true; }
    column(BooleanField; \"A. B\".Flag) { IncludeCaption = true; }
    column(ZeroOtherPlaces; \"A. B\".Q3Zero) { IncludeCaption = true; }
    column(ZeroWithExpression; \"A. B\".ZeroExpr) { IncludeCaption = true; }
    column(UnitCurrency; \"A. B\".UnitCur) { IncludeCaption = true; }
    column(UnitNoCaption; \"A. B\".UnitNoCap) { IncludeCaption = true; }
    column(CodeNoProperty; \"A. B\".Code1) { }
    column(EnumNoProperty; \"A. B\".St) { }
    column(UnitNoProperty; \"A. B\".UnitExpr) { }
    column(PlacesOverZero; \"A. B\".Q5) { DecimalPlaces = 4 : 4; }
    column(TextNoCaption; \"A. B\".Bare) { IncludeCaption = true; }
    column(TextCaptionMl; \"A. B\".Ml) { IncludeCaption = true; }
    column(Unmodelled; \"A. B\".Unm) { IncludeCaption = true; }
    column(MediaField; \"A. B\".Pic) { IncludeCaption = true; }
    column(AmountTypeWithPlaces; \"A. B\".AmtPlaces) { IncludeCaption = true; }
    column(NoProperty; \"A. B\".Qty) { }
    column(BothProperties; \"A. B\".Qty) { IncludeCaption = true; DecimalPlaces = 4 : 4; }
    column(PlacesOverText; \"A. B\".Name) { DecimalPlaces = 4 : 4; }
    column(OtherCase; \"a. b\".Qty) { IncludeCaption = true; }
    column(FieldOtherCase; \"A. B\".qty) { IncludeCaption = true; }
} } }",
    );
    for (name, field) in &columns {
        assert_eq!(*field, None, "{name}");
    }
    assert!(assumes.is_empty(), "{assumes:?}");
}

/// An undotted qualifier stays on main's rule (any field, any case) and is not flagged.
#[test]
fn an_undotted_qualifier_is_not_flagged() {
    let (columns, assumes) = lowered(
        "report 50100 R { dataset { dataitem(Row; T) {
    column(IntField; row.K) { IncludeCaption = true; }
    column(NoProperty; Row.Qty) { }
} } }",
    );
    assert_eq!(columns[0].1, Some(0));
    assert_eq!(columns[1].1, Some(2));
    assert!(assumes.is_empty(), "{assumes:?}");
}
