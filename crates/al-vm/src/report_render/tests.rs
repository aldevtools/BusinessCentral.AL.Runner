//! The gate of the measured Html cells: every fact by equality, a neighbor that differs in one fact keeps Part A.

use super::cells::HtmlCell;
use super::{HtmlInputs, bound_part, measured_cell};
use crate::report_save::Renders;

const CELL: HtmlCell = HtmlCell {
    majors: &[27, 28],
    layout_sha256: "aa",
    data_sha256: "dd",
    title: "RX Word Default",
};

fn inputs() -> HtmlInputs {
    HtmlInputs {
        layout: Vec::new(),
        title: "RX Word Default".into(),
        layout_sha256: "aa".into(),
        data_sha256: "dd".into(),
    }
}

#[test]
fn a_save_whose_every_fact_equals_a_cell_renders() {
    let t = [CELL];
    assert!(measured_cell(&t, true, Renders::Default, Some(27), &inputs()).is_some());
    assert!(measured_cell(&t, true, Renders::Default, Some(28), &inputs()).is_some());
}

#[test]
fn a_neighbor_that_differs_in_one_fact_keeps_part_a() {
    let t = [CELL];
    // Another save form, target or selection.
    assert!(measured_cell(&t, false, Renders::Default, Some(27), &inputs()).is_none());
    // The user-defined layout a temporary selection named.
    assert!(measured_cell(&t, true, Renders::Selected, Some(27), &inputs()).is_none());
    // A major the cell did not measure, or none known.
    assert!(measured_cell(&t, true, Renders::Default, Some(29), &inputs()).is_none());
    assert!(measured_cell(&t, true, Renders::Default, None, &inputs()).is_none());
    // Another layout, dataset or caption.
    for change in [
        |i: &mut HtmlInputs| i.layout_sha256 = "ab".into(),
        |i: &mut HtmlInputs| i.data_sha256 = "de".into(),
        |i: &mut HtmlInputs| i.title = "RX Word Alpha".into(),
    ] {
        let mut i = inputs();
        change(&mut i);
        assert!(measured_cell(&t, true, Renders::Default, Some(27), &i).is_none());
    }
}

#[test]
fn the_bound_part_starts_after_the_information_block_and_drops_the_row_system_ids() {
    let ds =
        b"<?xml?>\r\n<ReportDataSet>\r\n  <BCReportInformation>\r\n    <X>12:00</X>\r\n  </BCReportInformation>\r\n  \
               <DataItems>\r\n    <DataItem name=\"Row\" tableId=\"1\" systemId=\"55a4b0f3-1ea0\">x</DataItem>\r\n";
    assert_eq!(
        bound_part(ds).as_deref(),
        Some(&b"  <DataItems>\r\n    <DataItem name=\"Row\" tableId=\"1\">x</DataItem>\r\n"[..])
    );
    assert_eq!(bound_part(b"<ReportDataSet/>"), None);
    // An attribute that does not close is no key.
    assert_eq!(bound_part(b"</BCReportInformation>\r\n<D systemId=\"x"), None);
}
