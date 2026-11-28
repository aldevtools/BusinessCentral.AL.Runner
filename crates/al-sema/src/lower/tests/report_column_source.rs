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
    for (column, want) in [
        ("column(X; \"A. B\".Qty) { IncludeCaption = true; }", Some(2)),
        ("column(Y; \"A. B\".Qty.G) { }", None),
    ] {
        let (ws, l) = lower_src(&format!(
            "{TABLE}\nreport 50100 R {{ dataset {{ dataitem(\"A. B\"; T) {{ {column} }} }} }}"
        ));
        let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
        let c = &l.program.objects[oi].report.as_ref().unwrap().items[0].columns[0];
        assert_eq!(
            (c.item_field, c.caption_field, c.places_field),
            (want, want, want),
            "{column}"
        );
    }
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
/// Code, Integer, Text, Date and Enum fields with IncludeCaption; Decimal AutoFormatType 0 with DecimalPlaces 0:5 or
/// alone, AutoFormatType 2 and 1 with AutoFormatExpression ''; a column without property over AutoFormatType 0 + DecimalPlaces
/// 0:5 and over AutoFormatType 1 + AutoFormatExpression '', and over an Enum and a Date field (:395, :398, values :600, :614).
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
    column(LBinCode; \"Dq. Line\".Bin) { IncludeCaption = true; }
    column(LSeq; \"Dq. Line\".Seq) { IncludeCaption = true; }
    column(LZeroFmt; \"Dq. Line\".Zero) { IncludeCaption = true; }
    column(LQuantityNoProp; \"Dq. Line\".Q5) { }
    column(LCostAmountNoProp; \"Dq. Line\".Expr) { }
    column(DUnit; \"Dq. Line\".Unit) { IncludeCaption = true; }
    column(LStatusNoProp; \"Dq. Line\".St) { }
    column(LStartingDateNoProp; \"Dq. Line\".Due) { }
} } }",
    );
    let want = [14, 8, 0, 1, 16, 19, 20, 11, 17, 18, 21, 19, 11, 4, 14, 16];
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
    for (item, column, want) in [
        (
            "\"Dq. Line\"",
            "column(DUnit; \"Dq. Line\".Unit) { IncludeCaption = true; }",
            true,
        ),
        (
            "\"Dq. Line\"",
            "column(LUnitCost; \"Dq. Line\".UnitExpr) { IncludeCaption = true; }",
            true,
        ),
        ("\"Dq. Line\"", "column(NoProperty; \"Dq. Line\".UnitExpr) { }", false),
        (
            "\"Dq. Line\"",
            "column(Bare; UnitExpr) { IncludeCaption = true; }",
            false,
        ),
        (
            "\"Dq. Line\"",
            "column(Currency; \"Dq. Line\".UnitCur) { IncludeCaption = true; }",
            false,
        ),
        (
            "\"Dq. Line\"",
            "column(Amount; \"Dq. Line\".Expr) { IncludeCaption = true; }",
            false,
        ),
        (
            "Undotted",
            "column(ULUnitCost; Undotted.UnitExpr) { IncludeCaption = true; }",
            true,
        ),
        (
            "Undotted",
            "column(OtherCase; undotted.UnitExpr) { IncludeCaption = true; }",
            false,
        ),
        (
            "Undotted",
            "column(FieldOtherCase; Undotted.unitexpr) { IncludeCaption = true; }",
            false,
        ),
    ] {
        let (ws, l) = lower_src(&format!(
            "{TABLE}\nreport 50100 R {{ dataset {{ dataitem({item}; T) {{ {column} }} }} }}"
        ));
        let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
        let c = &l.program.objects[oi].report.as_ref().unwrap().items[0].columns[0];
        assert_eq!(c.measured_field == al_ir::MeasuredField::UnitAmount, want, "{column}");
    }
}

/// Dotted shapes the probe does not measure keep main's answer: no field, no assumption. Each column is the only one of its
/// report, so the all-or-none rule cannot hide a column the gate admits by mistake.
#[test]
fn an_unmeasured_dotted_shape_keeps_mains_answer() {
    for column in [
        "column(EnumNoCaption; \"A. B\".StNoCap) { IncludeCaption = true; }",
        "column(DateNoCaption; \"A. B\".DueNoCap) { IncludeCaption = true; }",
        "column(OptionNoCaption; \"A. B\".KindNoCap) { IncludeCaption = true; }",
        "column(OptionField; \"A. B\".Kind) { IncludeCaption = true; }",
        "column(BooleanField; \"A. B\".Flag) { IncludeCaption = true; }",
        "column(ZeroOtherPlaces; \"A. B\".Q3Zero) { IncludeCaption = true; }",
        "column(ZeroWithExpression; \"A. B\".ZeroExpr) { IncludeCaption = true; }",
        "column(UnitCurrency; \"A. B\".UnitCur) { IncludeCaption = true; }",
        "column(UnitNoCaption; \"A. B\".UnitNoCap) { IncludeCaption = true; }",
        "column(CodeNoProperty; \"A. B\".Code1) { }",
        "column(TextNoProperty; \"A. B\".Name) { }",
        "column(EnumNoCaptionNoProperty; \"A. B\".StNoCap) { }",
        "column(UnitNoProperty; \"A. B\".UnitExpr) { }",
        "column(PlacesOverZero; \"A. B\".Q5) { DecimalPlaces = 4 : 4; }",
        "column(TextNoCaption; \"A. B\".Bare) { IncludeCaption = true; }",
        "column(TextCaptionMl; \"A. B\".Ml) { IncludeCaption = true; }",
        "column(Unmodelled; \"A. B\".Unm) { IncludeCaption = true; }",
        "column(MediaField; \"A. B\".Pic) { IncludeCaption = true; }",
        "column(AmountTypeWithPlaces; \"A. B\".AmtPlaces) { IncludeCaption = true; }",
        "column(NoProperty; \"A. B\".Qty) { }",
        "column(BothProperties; \"A. B\".Qty) { IncludeCaption = true; DecimalPlaces = 4 : 4; }",
        "column(PlacesOverText; \"A. B\".Name) { DecimalPlaces = 4 : 4; }",
        "column(OtherCase; \"a. b\".Qty) { IncludeCaption = true; }",
        "column(FieldOtherCase; \"A. B\".qty) { IncludeCaption = true; }",
    ] {
        let (columns, assumes) = lowered(&format!(
            "report 50100 R {{ dataset {{ dataitem(\"A. B\"; T) {{ {column} }} }} }}"
        ));
        assert_eq!(columns[0].1, None, "{column}");
        assert!(assumes.is_empty(), "{column}: {assumes:?}");
    }
}

/// One column qualified with its own item's dotted name that the gate rejects keeps every dotted column of the report on main's
/// answer (no field, no assumption): a report serves its dotted columns only when the gate admits each of them, in any item.
#[test]
fn one_rejected_dotted_column_keeps_the_whole_report_on_mains_answer() {
    for rejected in [
        "column(NoProperty; \"A. B\".Qty) { }",
        "dataitem(\"C. D\"; T) { column(OptionField; \"C. D\".Kind) { IncludeCaption = true; } }",
        "dataitem(\"C. D\"; T) { column(OwnOtherCase; \"c. d\".Qty) { IncludeCaption = true; } }",
    ] {
        let (ws, l) = lower_src(&format!(
            "{TABLE}
report 50100 R {{ dataset {{ dataitem(\"A. B\"; T) {{
    column(Admitted; \"A. B\".Qty) {{ IncludeCaption = true; }}
    {rejected}
}} }} }}"
        ));
        let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
        let r = l.program.objects[oi].report.as_ref().unwrap();
        for c in r.items.iter().flat_map(|i| i.columns.iter()) {
            assert_eq!(c.item_field, None, "{rejected}: {}", c.name);
        }
        assert!(r.assumes.is_empty(), "{rejected}: {:?}", r.assumes);
    }
}

/// A child column that names its parent's dotted name in the shape the probe measures (part 1 at cf6884b9: :90 `CParentName`, a
/// Text field with a Caption under exactly IncludeCaption = true, labels :224, values :250) keeps main's rule for another item's
/// field (`caption_sources`) and leaves the own item's admitted dotted columns on.
#[test]
fn a_child_reading_the_dotted_parent_text_leaves_the_report_on() {
    let (ws, l) = lower_src(&format!(
        "{TABLE}
report 50100 R {{ dataset {{ dataitem(\"A. B\"; T) {{
    column(Admitted; \"A. B\".Qty) {{ IncludeCaption = true; }}
    dataitem(\"C. D\"; T) {{ DataItemLink = K = field(K); column(ParentName; \"A. B\".Name) {{ IncludeCaption = true; }} }}
}} }} }}"
    ));
    let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
    let r = l.program.objects[oi].report.as_ref().unwrap();
    assert_eq!(r.items[0].columns[0].item_field, Some(2));
    let parent = &r.items[1].columns[0];
    assert_eq!((parent.item_field, parent.caption_field), (None, Some(1)));
    assert_eq!(r.assumes, [Assumption::ReportColumnDottedItemQualifier]);
}

/// The child item of the probe's `CParentName` cell has a name that holds a dot (`"Dq. Child"`, :86) and states one
/// `Field = field(ParentField)` DataItemLink pair (:88). A child without a dot in its name, without a link, or with another
/// link turns the report's dotted columns off.
#[test]
fn a_child_outside_the_probes_shape_turns_the_dotted_columns_off() {
    for (child, on) in [
        ("dataitem(\"C. D\"; T) { DataItemLink = K = field(K);", true),
        ("dataitem(Kid; T) { DataItemLink = K = field(K);", false),
        ("dataitem(\"C. D\"; T) {", false),
        (
            "dataitem(\"C. D\"; T) { DataItemLink = K = field(K), Name = field(Name);",
            false,
        ),
        ("dataitem(\"C. D\"; T) { DataItemLink = K = const(1);", false),
    ] {
        let (ws, l) = lower_src(&format!(
            "{TABLE}
report 50100 R {{ dataset {{ dataitem(\"A. B\"; T) {{
    column(Admitted; \"A. B\".Qty) {{ IncludeCaption = true; }}
    {child} column(ParentName; \"A. B\".Name) {{ IncludeCaption = true; }} }}
}} }} }}"
        ));
        let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
        let r = l.program.objects[oi].report.as_ref().unwrap();
        assert_eq!(r.items[0].columns[0].item_field.is_some(), on, "{child}");
        assert_eq!(r.assumes.is_empty(), !on, "{child}");
    }
}

/// An own-item Decimal column that states DecimalPlaces is measured with exactly 4 : 4 (probe :82); another value turns the
/// report's dotted columns off.
#[test]
fn a_column_decimal_places_other_than_4_4_keeps_the_report_on_mains_answer() {
    for (places, on) in [("4 : 4", true), ("4:4", true), ("2 : 2", false), ("0 : 5", false)] {
        let (ws, l) = lower_src(&format!(
            "{TABLE}
report 50100 R {{ dataset {{ dataitem(\"A. B\"; T) {{
    column(Admitted; \"A. B\".Name) {{ IncludeCaption = true; }}
    column(Places; \"A. B\".Qty) {{ DecimalPlaces = {places}; }}
}} }} }}"
        ));
        let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
        let r = l.program.objects[oi].report.as_ref().unwrap();
        assert_eq!(r.items[0].columns[0].item_field.is_some(), on, "{places}");
    }
}

/// Every other column with another dotted name keeps the whole report on main's answer (no field, no assumption): a Decimal
/// field of the parent (probe :91-92, where BC writes `#,##0.###` and `#,##0.####` at :250 and main's rule for another
/// item's field writes `#,##0.00`), another field type, a field without Caption, another column property, another case, an
/// item that is not the parent, a global Record variable.
#[test]
fn another_dotted_column_outside_the_measured_shape_keeps_the_report_on_mains_answer() {
    let child = |column: &str| {
        format!(
            "dataitem(\"A. B\"; T) {{ column(Admitted; \"A. B\".Qty) {{ IncludeCaption = true; }}
            dataitem(\"C. D\"; T) {{ DataItemLink = K = field(K); {column} }} }}"
        )
    };
    for dataset in [
        child("column(ParentQty; \"A. B\".Qty) { IncludeCaption = true; }"),
        child("column(ParentAmt; \"A. B\".Amt) { IncludeCaption = true; }"),
        child("column(ParentInteger; \"A. B\".K) { IncludeCaption = true; }"),
        child("column(ParentCode; \"A. B\".Code1) { IncludeCaption = true; }"),
        child("column(ParentNoCaption; \"A. B\".Bare) { IncludeCaption = true; }"),
        child("column(ParentCaptionMl; \"A. B\".Ml) { IncludeCaption = true; }"),
        child("column(ParentNoProperty; \"A. B\".Name) { }"),
        child("column(ParentPlaces; \"A. B\".Name) { DecimalPlaces = 4 : 4; }"),
        child("column(ParentOtherCase; \"a. b\".Name) { IncludeCaption = true; }"),
        child("column(ParentFieldOtherCase; \"A. B\".name) { IncludeCaption = true; }"),
        child("dataitem(Grand; T) { column(Grandparent; \"A. B\".Name) { IncludeCaption = true; } }"),
        "dataitem(\"A. B\"; T) { column(Admitted; \"A. B\".Qty) { IncludeCaption = true; } }
        dataitem(Next; T) { column(Sibling; \"A. B\".Name) { IncludeCaption = true; } }"
            .to_string(),
        "dataitem(\"A. B\"; T) { column(Admitted; \"A. B\".Qty) { IncludeCaption = true; }
            column(FromChild; \"C. D\".Name) { IncludeCaption = true; }
            dataitem(\"C. D\"; T) { column(K; K) { } } }"
            .to_string(),
        "dataitem(\"A. B\"; T) { column(Admitted; \"A. B\".Qty) { IncludeCaption = true; }
            column(Global; \"G. V\".Name) { } }"
            .to_string(),
    ] {
        let (ws, l) = lower_src(&format!(
            "{TABLE}
report 50100 R {{ dataset {{ {dataset} }} var \"G. V\": Record T; }}"
        ));
        let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
        let r = l.program.objects[oi].report.as_ref().unwrap();
        for c in r.items.iter().flat_map(|i| i.columns.iter()) {
            assert_eq!(c.item_field, None, "{dataset}: {}", c.name);
        }
        assert!(r.assumes.is_empty(), "{dataset}: {:?}", r.assumes);
    }
}

/// A method call on a dotted own-item field leaves the report's dotted columns on only in the probe's shape (:84
/// `"Dq. Row".Name.ToUpper()`, a Text field with a Caption, a column without property; value :236); the call column itself
/// takes no field. Any other call or field keeps the report on main's answer.
#[test]
fn a_dotted_method_call_column_is_measured_only_in_the_probes_shape() {
    for (column, on) in [
        ("column(Upper; \"A. B\".Name.ToUpper()) { }", true),
        ("column(Lower; \"A. B\".Name.ToLower()) { }", false),
        ("column(UpperCode; \"A. B\".Code1.ToUpper()) { }", false),
        ("column(UpperNoCaption; \"A. B\".Bare.ToUpper()) { }", false),
        ("column(UpperCase; \"A. B\".Name.toupper()) { }", false),
    ] {
        let (columns, assumes) = lowered(&format!(
            "report 50100 R {{ dataset {{ dataitem(\"A. B\"; T) {{
    column(Admitted; \"A. B\".Qty) {{ IncludeCaption = true; }}
    {column}
}} }} }}"
        ));
        assert_eq!(columns[0].1, on.then_some(2), "{column}");
        assert_eq!(columns[1].1, None, "{column}");
        assert_eq!(!assumes.is_empty(), on, "{column}: {assumes:?}");
    }
}

/// A dotted column a reportextension adds keeps the report's dotted columns on main's answer too: an extension column never
/// takes the dotted rule.
#[test]
fn a_dotted_extension_column_keeps_the_report_on_mains_answer() {
    let (columns, assumes) = lowered(
        "report 50100 R { dataset { dataitem(\"A. B\"; T) {
    column(Admitted; \"A. B\".Qty) { IncludeCaption = true; }
} } }
reportextension 50101 X extends R { dataset { add(\"A. B\") {
    column(Added; \"A. B\".Name) { IncludeCaption = true; }
} } }",
    );
    assert_eq!(columns[0], ("Admitted".to_string(), None));
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
