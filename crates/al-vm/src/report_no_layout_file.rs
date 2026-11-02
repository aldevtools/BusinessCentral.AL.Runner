//! An Execute of a report whose only layout setting is `DefaultLayout = RDLC`, with no layout declared: it fails with the
//! no-layout error before OnPreReport (report/lt-y68-selfclosing-params/Y68SelfClosingParams.al:420-444
//! Q1_SaveAs_SelfClosing_DataReport `execute text`, corpus master 0d01b0cc; a SaveAs Xml of the same report writes its
//! dataset). A Run of such a report, and other DefaultLayout values, are not pinned and stay refused where they render.

use crate::reports::Schema;
use crate::{Session, Signal};
use al_ir as ir;

/// Called by an Execute with parameters after its request page triggers ran.
pub(crate) fn execute_check(s: &Session, schema: &Schema) -> Result<(), Signal> {
    let report = schema.report();
    let only_default_rdlc = report.layout_file_missing
        && report.layout == ir::ReportLayout::Rdlc
        && !report.processing_only
        && report.items.iter().any(|i| !i.columns.is_empty())
        && report
            .output
            .iter()
            .filter(|r| r.contains("(a layout)"))
            .all(|r| r.contains("defaultlayout"));
    if !only_default_rdlc {
        return Ok(());
    }
    Err(Signal::Error(Box::new(al_builtins::native::AlError {
        message: format!(
            "Report {} does not have a valid layout. Valid layout types are: Word, RDLC, Excel, and Custom.",
            s.object_id_of(schema.object() as u32)
        ),
        code: "ReportNoLayout".into(),
        ..Default::default()
    })))
}
