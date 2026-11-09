//! The arguments of `Report.Execute`: the parameters string and the RecordRef.

use super::*;

impl Session {
    /// `Execute(Parameters, ?RecordRef)`: only the empty parameter string is pinned
    /// (record/TestTxModelReportExec.al:138-211).
    pub(super) fn report_execute_args(
        &self,
        id: al_ir::BuiltinId,
        parameters: Option<&Value>,
        record_ref: Option<&Value>,
    ) -> Result<Option<String>, Signal> {
        // A parameters string with `<Options><Field name=..>` sets the request page's Text, Boolean and Integer variables
        // before OnPreReport, and a field name no control has is skipped (reports/WdReportExecuteParams.al:50-77, corpus
        // branch al-runtime-rs-staging-1030c, al-runner issue 6306). Only the form `RunRequestPage` writes is read
        // (`apply_request_parameters`).
        let parameters = match parameters {
            Some(Value::Text(s)) if s.is_empty() => None,
            // A `<DataItem name="..">VIEW</DataItem>` replaces the item's view for this run only, over a SetTableView; an
            // unknown item name is skipped and an empty view clears it (corpus master, corpus PR 886:
            // reports/request-page/ExecuteParamsTwiceAndViews.al:185-235; `apply_request_parameters`).
            Some(Value::Text(s)) => Some(s.to_string()),
            other => {
                return unsupported(format!(
                    "{id} with the parameters {} (only a text is pinned)",
                    other.describe()
                ));
            }
        };
        // A RecordRef with an empty parameters string, static form only: the report runs over the RecordRef's filters
        // (reports/W3ReportSaveAsExecute.al:446-459 Execute_WithRecordRef; `execute_record_ref_view`). With parameters, or
        // on a report variable, which filters apply is not pinned (al-runner issue 7446).
        if record_ref.is_some() && (parameters.is_some() || id != ids::REPORT_EXECUTE_3) {
            return unsupported(format!(
                "{id} with parameters or on a report variable, and a RecordRef (not pinned)"
            ));
        }
        // A SubstituteReport subscriber gets an unopened RecordRef here too, as for the pinned SaveAs: which RecordRef BC hands
        // it is not pinned and no subscriber in the corpus reads it (docs/probes-needed.md, "SubstituteReport arguments").
        Ok(parameters)
    }

    /// The RecordRef of the static `Report.Execute(Number, '', RecordRef)` filters the data item over its table as a view
    /// does (reports/W3ReportSaveAsExecute.al:446-459 Execute_WithRecordRef: `pre filter=[5..6];row 5;row 6`).
    pub(super) fn execute_record_ref_view(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        record_ref: Option<&Value>,
    ) -> Result<(), Signal> {
        match record_ref {
            Some(rr) => self.report_set_view(inst, schema, Some(rr)),
            None => Ok(()),
        }
    }
}
