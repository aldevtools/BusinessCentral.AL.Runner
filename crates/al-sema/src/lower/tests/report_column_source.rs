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
} keys { key(PK; K) { } } }";

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

/// Dotted shapes the probe does not measure keep main's answer: no field, no assumption.
#[test]
fn an_unmeasured_dotted_shape_keeps_mains_answer() {
    let (columns, assumes) = lowered(
        "report 50100 R { dataset { dataitem(\"A. B\"; T) {
    column(IntField; \"A. B\".K) { IncludeCaption = true; }
    column(CodeField; \"A. B\".Code1) { IncludeCaption = true; }
    column(TextNoCaption; \"A. B\".Bare) { IncludeCaption = true; }
    column(TextCaptionMl; \"A. B\".Ml) { IncludeCaption = true; }
    column(Unmodelled; \"A. B\".Unm) { IncludeCaption = true; }
    column(WithExpression; \"A. B\".Expr) { IncludeCaption = true; }
    column(MediaField; \"A. B\".Pic) { IncludeCaption = true; }
    column(UnitAmountType; \"A. B\".Unit) { IncludeCaption = true; }
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
