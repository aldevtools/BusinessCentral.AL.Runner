//! A child column over a Decimal field of its direct parent (`Parent.Field`) takes the parent field's formatter only in the
//! layout corpus PR 1652 measures (DataItemNamesDottedQualifier.al at cf6884b9, assumption `report-column-parent-field-format`):
//! parent Qty no. 3 (DecimalPlaces 0:3) and Amt no. 4 (AutoFormatType 1) (:32-33); child no. 3 the Text field Note and no. 4
//! the Decimal Price with DecimalPlaces 0:4 (:54-55); the parent item has its own IncludeCaption column over each field
//! (:112-113). Every other layout keeps main's rule (`MeasuredField::None`).
use super::*;

const PARENT: &str = "table 50110 P { fields {
    field(1; Id; Integer) { Caption = 'Id cap'; }
    field(2; Name; Text[30]) { Caption = 'Name cap'; }
    field(3; Qty; Decimal) { Caption = 'Qty cap'; DecimalPlaces = 0 : 3; }
    field(4; Amt; Decimal) { Caption = 'Amt cap'; AutoFormatType = 1; }
} keys { key(PK; Id) { } } }";

/// The probe's child table (:46-55).
const CHILD: &str = "table 50111 C { fields {
    field(1; Id; Integer) { Caption = 'Child Id cap'; }
    field(2; RowId; Integer) { }
    field(3; Note; Text[30]) { Caption = 'Note cap'; }
    field(4; Price; Decimal) { Caption = 'Price cap'; DecimalPlaces = 0 : 4; }
} keys { key(PK; Id) { } } }";

/// The probe's undotted report (:111-131) with `parent_columns` in the parent item and child item name `child`.
fn report(parent_columns: &str, child: &str) -> String {
    format!(
        "report 50112 R {{ dataset {{ dataitem(PRow; P) {{ {parent_columns}
            dataitem({child}; C) {{ DataItemLink = RowId = field(Id);
                column(CParentQty; PRow.Qty) {{ IncludeCaption = true; }}
                column(CParentAmt; PRow.Amt) {{ IncludeCaption = true; }}
                column(CNote; {child}.Note) {{ IncludeCaption = true; }}
                column(CPrice; {child}.Price) {{ IncludeCaption = true; }}
            }} }} }} }}"
    )
}

const OWN: &str = "column(DQty; PRow.Qty) { IncludeCaption = true; } column(DAmt; PRow.Amt) { IncludeCaption = true; }";

/// Whether CParentQty and CParentAmt are marked `ParentField`.
fn marked(child_table: &str, report: &str) -> (bool, bool) {
    let (ws, l) = lower_src(&format!("{PARENT}\n{child_table}\n{report}"));
    let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
    let columns = &l.program.objects[oi].report.as_ref().unwrap().items[1].columns;
    let parent =
        |name: &str| columns.iter().find(|c| c.name == name).unwrap().measured_field == ir::MeasuredField::ParentField;
    (parent("CParentQty"), parent("CParentAmt"))
}

#[test]
fn the_probes_layout_is_marked() {
    assert_eq!(marked(CHILD, &report(OWN, "Kid")), (true, true));
}

/// P6 of the probe-cell list: Price at a number no parent field uses (here no. 5), so Amt no. 4 has no child field. Qty no. 3
/// still meets the cell's Text field Note.
#[test]
fn price_at_another_field_number_keeps_mains_rule() {
    let child = CHILD.replace("field(4; Price", "field(5; Price");
    assert_eq!(marked(&child, &report(OWN, "Kid")), (true, false));
}

/// P7: the child field with Amt's number is a Text field, Price sits at Qty's number.
#[test]
fn a_text_child_field_at_amts_number_keeps_mains_rule() {
    let child = CHILD
        .replace("field(3; Note", "field(4; Note")
        .replace("field(4; Price", "field(3; Price");
    assert_eq!(marked(&child, &report(OWN, "Kid")), (false, false));
}

/// The child's Text field at Qty's number without Caption, and Price without Caption (the probe's have one, :54-55).
#[test]
fn child_fields_without_caption_keep_mains_rule() {
    let child = CHILD.replace("Caption = 'Note cap'; ", "");
    assert_eq!(marked(&child, &report(OWN, "Kid")), (false, true));
    let child = CHILD.replace("Caption = 'Price cap'; ", "");
    assert_eq!(marked(&child, &report(OWN, "Kid")), (false, false));
}

/// P9: the parent item has no column of its own over the field, or one with another property.
#[test]
fn a_parent_without_its_own_column_keeps_mains_rule() {
    assert_eq!(marked(CHILD, &report("", "Kid")), (false, false));
    let qty_only = "column(DQty; PRow.Qty) { IncludeCaption = true; }";
    assert_eq!(marked(CHILD, &report(qty_only, "Kid")), (true, false));
    let no_property = "column(DQty; PRow.Qty) { } column(DAmt; PRow.Amt) { }";
    assert_eq!(marked(CHILD, &report(no_property, "Kid")), (false, false));
}

/// A child item whose name holds a dot, under an undotted parent: the cell has both names undotted (:111, :122) or both dotted.
#[test]
fn a_dotted_child_name_keeps_mains_rule() {
    assert_eq!(marked(CHILD, &report(OWN, "\"P. Kid\"")), (false, false));
}
