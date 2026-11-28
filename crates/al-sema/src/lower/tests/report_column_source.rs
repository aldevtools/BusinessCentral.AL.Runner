//! A report column source qualified with its own data item's name when that name holds a dot (al-runner issue 9330):
//! `"A. B".F` in data item `"A. B"` is field F of the own item, `"A. B".F.G` is no field. A quoted qualifier of the own
//! item: streams/P138ColCaptionQualified.al:251; a dot in a data item name: reports/dataset/ReportDatasetNames.al:31.
use super::*;

#[test]
fn a_source_qualified_with_a_dotted_own_item_name_is_its_field() {
    let (ws, l) = lower_src(
        "table 50100 T { fields { field(1; K; Integer) { } field(2; F; Decimal) { } } keys { key(PK; K) { } } }
report 50100 R { dataset { dataitem(\"A. B\"; T) {
    column(X; \"A. B\".F) { IncludeCaption = true; }
    column(Y; \"A. B\".F.G) { }
} } }",
    );
    let oi = ws.objects.iter().position(|o| o.name == "R").unwrap();
    let columns = &l.program.objects[oi].report.as_ref().unwrap().items[0].columns;
    let fields = |c: &al_ir::ReportColumn| (c.item_field, c.caption_field, c.places_field);
    assert_eq!(fields(&columns[0]), (Some(1), Some(1), Some(1)), "{}", columns[0].name);
    assert_eq!(fields(&columns[1]), (None, None, None), "{}", columns[1].name);
}
