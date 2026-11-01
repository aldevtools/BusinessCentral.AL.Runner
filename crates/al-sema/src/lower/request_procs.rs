//! A call of a procedure declared in a report's or report extension's `requestpage` section (al-runner issue 6651).

use super::*;
use crate::members::ProcSig;

impl Lowerer<'_> {
    /// The signature of the procedure at `offset` of `object`. A procedure of a request page that the calling code could also
    /// reach under another declaration of its name (a procedure of the report or report extension itself, or a non-local one
    /// of the report it extends) is refused: which one alc binds the call to is not pinned. A local procedure of the base
    /// report is no such declaration: the extension cannot reach it (AL0161), and its own request page procedure is the call.
    pub(crate) fn request_proc_refusal(&self, object: usize, offset: usize) -> L<ProcSig> {
        let m = &self.members[object];
        let sig = m.procedure_at(offset).ok_or("procedure signature")?.clone();
        if !m.request_procedures.iter().any(|p| p.offset == offset) {
            return Ok(sig);
        }
        let own = !m.procedures_named(&sig.name).is_empty();
        let base = self.ws.objects[object].kind == ObjKind::ReportExtension
            && self.ws.extension_target(object).is_some_and(|b| {
                crate::base_access::visible_procs(&self.ws.object_ast(b), &sig.name)
                    .next()
                    .is_some()
            });
        if own || base {
            return Err(format!(
                "request page procedure `{}` next to a procedure of the same name of the report{}: which one a call in the request page runs is not pinned",
                sig.name,
                if base { " it extends" } else { "" }
            ));
        }
        Ok(sig)
    }
}
