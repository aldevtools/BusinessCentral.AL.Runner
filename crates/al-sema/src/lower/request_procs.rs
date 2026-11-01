//! A call of a procedure declared in a report's or report extension's `requestpage` section (al-runner issue 6651).

use super::*;
use crate::members::ProcSig;

impl Lowerer<'_> {
    /// The signature of the procedure at `offset` of `object`. In a report extension's request page its own procedure is the
    /// call, also next to a procedure of the extension or a public one of the base report with the same name (al-runner issue 6651;
    /// corpus branch al-runtime-rs-staging-y64 85d86e7b, reportext/lt-y64-local-proc-names/Y64LocalProcNames.al:465-477: `ext rp
    /// MyProc<-ext rp open`, `ext rp Pub<-ext aftervalidate`). In a report a request page procedure next to a report procedure of
    /// the same name is not pinned: refused.
    pub(crate) fn request_proc_refusal(&self, object: usize, offset: usize) -> L<ProcSig> {
        let m = &self.members[object];
        let sig = m.procedure_at(offset).ok_or("procedure signature")?.clone();
        let in_request_page = m.request_procedures.iter().any(|p| p.offset == offset);
        if in_request_page
            && self.ws.objects[object].kind == ObjKind::Report
            && !m.procedures_named(&sig.name).is_empty()
        {
            return Err(format!(
                "request page procedure `{}` next to a procedure of the same name of the report: which one a call in the request page runs is not pinned",
                sig.name
            ));
        }
        Ok(sig)
    }
}
