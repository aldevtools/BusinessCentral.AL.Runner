//! The arguments of `Report.Execute` and `Report.SaveAs`: the parameters string and the RecordRef.

use super::*;

/// Which call hands a RecordRef next to a parameters string (`record_ref_with_parameters`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RefCall {
    /// `Report.Execute(Number, Parameters, RecordRef)`.
    StaticExecute,
    /// `ReportVariable.Execute(Parameters, RecordRef)`.
    VariableExecute,
    /// `Report.SaveAs(Number, Parameters, Format, OutStream, RecordRef)`.
    StaticSaveAs,
    /// `ReportVariable.SaveAs(Parameters, Format, OutStream, RecordRef)`.
    VariableSaveAs,
}

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
        // A RecordRef with an empty parameters string is pinned for the static form only: the report runs over the
        // RecordRef's filters (reports/W3ReportSaveAsExecute.al:446-459 Execute_WithRecordRef). With parameters both forms
        // are pinned (`record_ref_with_parameters`).
        if record_ref.is_some() && parameters.is_none() && id != ids::REPORT_EXECUTE_3 {
            return unsupported(format!(
                "{id} on a report variable with empty parameters and a RecordRef (not pinned)"
            ));
        }
        // A SubstituteReport subscriber gets an unopened RecordRef here too, as for the pinned SaveAs: which RecordRef BC hands
        // it is not pinned and no subscriber in the corpus reads it (docs/probes-needed.md, "SubstituteReport arguments").
        Ok(parameters)
    }

    /// Runs an Execute of `inst` with its parameters string and RecordRef.
    pub(super) fn report_execute_run(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        parameters: Option<String>,
        record_ref: Option<&Value>,
        call: RefCall,
    ) -> Result<(), Signal> {
        let deferred = self.report_record_ref(inst, schema, record_ref, parameters.is_some(), call)?;
        self.report.execute_parameters = parameters;
        let ran = self.report_run(inst, schema, Entry::Execute, false, deferred);
        self.report.execute_parameters = None;
        ran
    }

    /// The RecordRef of an Execute or a SaveAs. Without parameters it filters the data item over its table as a view does,
    /// before the run (reports/W3ReportSaveAsExecute.al:446-459 Execute_WithRecordRef: `pre filter=[5..6];row 5;row 6`;
    /// streams/TestReportSaveAsRecordRefFilter.al). With parameters see `record_ref_with_parameters`: the RecordRef it returns
    /// goes to the run it belongs to (`report_body`), never into session state, so a report that runs inside this one does
    /// not take it or drop it.
    pub(super) fn report_record_ref(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        record_ref: Option<&Value>,
        with_parameters: bool,
        call: RefCall,
    ) -> Result<Option<Value>, Signal> {
        match record_ref {
            None => Ok(None),
            Some(rr) if with_parameters => self.record_ref_with_parameters(schema, rr, call),
            Some(rr) => self.report_set_view(inst, schema, Some(rr)).map(|()| None),
        }
    }

    /// A RecordRef next to a parameters string: the parameters set the options and the data item views, then the RecordRef's
    /// view replaces the whole view of the data item over its table, filters, key and direction, also one the parameters or a
    /// SetTableView gave; the other items keep the parameters' views (reports/runtime/ExecuteParametersWithRecordRef.al,
    /// BC 27, 28 and 29: :294-313 parameters without a filter, :315-334 a RecordRef without a filter clears the parameters'
    /// filter, :336-376 a filter on the same and on another field, :378-401 another key descending, :403-447 over the second
    /// item's table, :535-554 after SetTableView). Checked before the run: over a table no data item reads it raises before any
    /// trigger (:449-470). A RecordRef that is not open leaves the parameters alone (:472-489). The view is applied in
    /// `apply_run_parameters`.
    ///
    /// Pinned for the static Execute in every shape above; for the static SaveAs a filter on the first item's table and the
    /// second item's table (:556-575, :598-617); for the variable's Execute and SaveAs a filter on the first item's table only
    /// (:514-554, :577-596), each with a filter, the primary key and ascending order. Refused: other shapes of those calls
    /// (another key, descending, no filter), a RecordRef not open outside the static Execute, a table two data items read
    /// and a table only a reportextension's item reads (not pinned). Returns the RecordRef whose view the run applies, or
    /// none for an unopened one.
    fn record_ref_with_parameters(
        &mut self,
        schema: &Schema,
        rr: &Value,
        call: RefCall,
    ) -> Result<Option<Value>, Signal> {
        let not_pinned =
            |what: &str| unsupported(format!("a RecordRef {what} next to a parameters string (not pinned)"));
        let Value::Library(_, h) = rr else {
            return not_pinned("that is no RecordRef");
        };
        let table = match &*h.borrow() {
            al_builtins::value::LibraryValue::RecordRef(rec) => {
                let r = rec.borrow();
                let filter_shape = r.key == 0
                    && r.adhoc.is_none()
                    && r.ascending
                    && r.field_descending == 0
                    && !r.marked_only
                    && !r.filters.is_empty();
                if call != RefCall::StaticExecute && !filter_shape {
                    return not_pinned(
                        "with a view other than a filter on the primary key in ascending order, outside a static Execute,",
                    );
                }
                r.table
            }
            al_builtins::value::LibraryValue::Empty if call == RefCall::StaticExecute => return Ok(None),
            al_builtins::value::LibraryValue::Empty => {
                return not_pinned("that is not open, outside a static Execute,");
            }
            _ => return not_pinned("that is no RecordRef"),
        };
        let items = &schema.report().items;
        let over: Vec<usize> = (0..items.len())
            .filter(|&i| items[i].table == Some(table as usize))
            .collect();
        match over.as_slice() {
            [] if schema.report().extensions.is_empty() => return Err(self.report_no_data_item(schema, table)),
            [] => {
                return not_pinned("over a table no data item of the report reads, on a report with reportextensions,");
            }
            [0] => {}
            [_] if matches!(call, RefCall::StaticExecute | RefCall::StaticSaveAs) => {}
            [_] => return not_pinned("over the table of a data item other than the first, on a report variable,"),
            _ => return not_pinned("over a table two data items read"),
        }
        Ok(Some(rr.clone()))
    }

    /// Loads the parameters string of a `SaveAs` into the data items' records, the way a request
    /// page does before the run (reports/TestReportRunRequestPage.al:TestReport_CapturedParameters_
    /// ReplayThroughSaveAs_FilterTheDataset), or of an `Execute`. Which forms of the text are understood,
    /// per call: `apply_request_parameters` and `ParamsCall`.
    pub(super) fn apply_run_parameters(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        run: &Run,
        xml: &str,
        call: ParamsCall,
        record_ref: Option<Value>,
    ) -> Result<(), Signal> {
        let engine = self.engine.clone();
        let Some(page) = engine.program.objects[schema.object].page.as_ref() else {
            return unsupported(format!(
                "parameters for report \"{}\", which has no request page",
                schema.name()
            ));
        };
        if let Some(why) = page.unsupported.first() {
            return unsupported(format!("the request page of report \"{}\": {why}", schema.name()));
        }
        let mut exts = Vec::new();
        for &e in &page.extensions {
            let ext = match run.exts.iter().find(|(_, re)| *re == e) {
                Some((i, _)) => i.clone(),
                None => self.extension_instance(e, inst),
            };
            exts.push((e, ext));
        }
        self.apply_request_parameters(inst, &exts, schema.object, xml, call)?;
        // The RecordRef's view replaces the data item's once the parameters are applied (`record_ref_with_parameters`).
        match record_ref {
            Some(rr) => self.report_set_view(inst, schema, Some(&rr)),
            None => Ok(()),
        }
    }
}
