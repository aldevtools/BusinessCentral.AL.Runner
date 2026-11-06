//! The OnLookup trigger that `TestPage.<control>.Lookup()` starts, and what an error out of it does.

use super::*;

impl Session {
    pub(super) fn on_lookup_trigger(
        &mut self,
        st: &St,
        idx: usize,
        owner: Option<usize>,
        routine: usize,
    ) -> Result<Value, Signal> {
        // OnLookup(var Text): Boolean. The text it writes back replaces the field's value, and
        // the Boolean says whether the user picked anything (corpus
        // testpage/TestPageFieldTableLookup.al:72 Lookup_ControlWithTrigger_WinsOverTheTableFieldsOnLookup,
        // testpage/TestPageModalHandler_Tests.al:132). The text the trigger starts with is the
        // control's current text (assumed, docs/probes-needed.md).
        let object = owner.unwrap_or(st.borrow().object as usize);
        let inst = self.owner_instance(st, owner);
        let shown = self.lookup_start_text(st, idx, object, routine)?;
        let called = self.lookup_trigger_call(st, object, routine, inst, vec![Value::Text(shown.into())]);
        let called = match called {
            Err(e) => {
                st.borrow_mut().refresh = false;
                return self.lookup_error_outcome(Err(e)).unwrap_or(Ok(Value::Null));
            }
            ok => ok,
        };
        called.and_then(|(v, finals)| match (v, finals.first()) {
            (Value::Boolean(true), Some(Value::Text(t))) => {
                // Written back as the client would type it. The OnLookup of a page extension's modify block
                // leaves a Code field in the case it wrote (`Text := 'FromExt'` reads back FromExt), the page's
                // own trigger does not (FROMCTL) (pages/lookups/W2Lookups.al:
                // ModifyBlockOnLookup_ReturnsTrue, BC 27.5 and 29.0).
                st.borrow_mut().lookup_setting_value = true;
                let r = self.control_set_value(st, idx, &mut [Value::Text(t.clone())]);
                st.borrow_mut().lookup_setting_value = false;
                let binding = page_of(&self.engine(), st).controls[idx].binding.clone();
                if let ir::Binding::Field(pos) = binding {
                    let rec = self.page_rec(st);
                    let is_code = matches!(rec.borrow().fields[pos], Value::Code(_));
                    if is_code && owner.is_some() && r.is_ok() {
                        rec.borrow_mut().fields[pos] = Value::Code(t.clone());
                    }
                }
                r
            }
            (Value::Boolean(true), other) => {
                unsupported(format!("an OnLookup trigger whose Text is {}", other.describe()))
            }
            _ => Ok(Value::Null),
        })
    }

    /// The table field's OnLookup that `TestPage.<control>.Lookup()` starts on the page's record.
    pub(super) fn field_on_lookup(&mut self, st: &St, object: usize, routine: usize) -> Result<Value, Signal> {
        let rec = self.page_rec(st);
        st.borrow_mut().refresh = false;
        let called = self.call_on_record(object, routine, rec, None, Vec::new());
        if let Some(done) = self.lookup_error_outcome(called.map(|_| Value::Null)) {
            return done;
        }
        // The control now shows what the trigger wrote, as an edit of the row (assumed,
        // docs/probes-needed.md).
        st.borrow_mut().dirty = true;
        self.run_refresh(st)?;
        Ok(Value::Null)
    }

    /// What an error out of an OnLookup that `TestPage.<control>.Lookup()` started does. An error with an empty text (`Error('')`) ends
    /// the call quietly: the test goes on, the error becomes the last error (empty text, code Dialog) and the rows the trigger wrote
    /// before it stay (pages/empty-error-text/WcLookupEmptyError.al WcTableFieldLookup_CancelEmptyError_NoAsserterror,
    /// WcControlLookup_CancelEmptyError_NoAsserterror, WcControlLookup_DirectEmptyError_NoAsserterror; BC 27.5). An error with a text
    /// comes out with the code `TestWrapped:Dialog` (WcTableFieldLookup_CancelTextError_Asserterror; pinned for the table field's
    /// trigger, the control's own trigger is assumed to match, docs/probes-needed.md). Returns None for a result that is not an error.
    pub(super) fn lookup_error_outcome(&mut self, r: Result<Value, Signal>) -> Option<Result<Value, Signal>> {
        match r {
            Err(Signal::Error(mut e)) if e.message.is_empty() => {
                if let Some(own) = e.code.strip_prefix("TestWrapped:") {
                    e.code = own.to_string();
                }
                self.error_finals = None;
                self.last_error = *e;
                Some(Ok(Value::Null))
            }
            Err(Signal::Error(mut e)) if e.code == "Dialog" => {
                e.code = "TestWrapped:Dialog".into();
                Some(Err(Signal::Error(e)))
            }
            Err(other) => Some(Err(other)),
            Ok(_) => None,
        }
    }
}
