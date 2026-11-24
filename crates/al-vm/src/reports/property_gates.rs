//! What a `Report.SaveAs` refuses in the report's declaration before it runs: [`save_as_declaration_refusals`].
//!
//! what a `Report.SaveAs` refuses in the report's declaration before it runs (a dataset this runtime
//! cannot write, a TransactionType). Entry points: `save_as_declaration_refusals` (from
//! `report_save_core`).

use al_builtins::native::Signal;

/// What a `Report.SaveAs` refuses in the report's declaration before it runs: a dataset this runtime cannot write,
/// and a TransactionType (what SaveAs does with it is not pinned).
pub(crate) fn save_as_declaration_refusals(schema: &super::Schema) -> Result<(), Signal> {
    if let Some(why) = schema.report().dataset_unsupported.first() {
        return Err(Signal::unsupported(format!(
            "Report.SaveAs of \"{}\": {why}",
            schema.name()
        )));
    }
    if schema.report().transaction_type.is_some() {
        return Err(Signal::unsupported(format!(
            "Report.SaveAs of \"{}\", which declares a TransactionType (what SaveAs does with it is not pinned)",
            schema.name()
        )));
    }
    Ok(())
}
