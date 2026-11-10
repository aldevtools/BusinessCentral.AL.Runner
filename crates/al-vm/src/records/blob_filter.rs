//! Filters on a Blob field that the record operations check themselves (the text filters are `al_builtins::filter_blob`).

use super::*;

impl Session {
    /// `SetRange` on a Blob field with empty texts raises a conversion error: from BC 28 `The value "" can't be evaluated
    /// into type BLOB.` (Evaluate:Argument), in BC 27 `Invalid Conversion. Source: NavText, Destination: NavBLOB.` (Argument)
    /// (recordref-fieldref/fieldref/FieldRefFilterEmptyText.al:212, :234-242 `range_quotes`; al-runner issue 7426). Other
    /// values are converted as before, which refuses them.
    pub(crate) fn blob_setrange_check(&self, ty: &RtType, bounds: &[Value]) -> Result<(), Signal> {
        let empty_texts = !bounds.is_empty() && bounds.iter().all(|v| matches!(v, Value::Text(s) if s.is_empty()));
        if !al_builtins::filter_blob::is_blob(ty) || !empty_texts {
            return Ok(());
        }
        let from28 = self.has(al_version::Gate::FieldRefTextNumberEvaluateErrorFrom28);
        match al_builtins::fieldref_value_text::fieldref_value_into_blob(&bounds[0], ty, self, from28) {
            Some(r) => r.map(|_| ()),
            None => Ok(()),
        }
    }
}
