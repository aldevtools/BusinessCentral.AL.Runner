//! A lower-case (indirect) letter of an `[InherentPermissions]` attribute (al-runner issue 8066).
//!
//! Upper-case letters are pinned (CpPermLeads.al:155-189, in `permissions.rs`). Corpus PR 1403 (open;
//! permissions/testpermissions/Y19InherentAttrIndirect.al, test IA_LowerCaseAttribute) measured on BC 27.5, 28.5 and 29.0 that
//! `'i'` and `'r'` give the indirect level for the call and the calls it makes, which the code's `Permissions` property turns
//! into an allowed access (ops 1, 4, 5) and which without it is denied as `Insert` (ops 11, 13) (assumption
//! `inherent-attribute-indirect-letter`). The measured shape is an allowlist of its facts here; every other indirect letter
//! stays refused.
use crate::Session;
use crate::permissions::{Access, Levels, Op, SetModel};
use al_builtins::assumptions::{Assumption, assume};

impl Session {
    /// Whether the indirect letter of attribute levels `l`, on the routine of frame `frame` (an index into `frames`), is in
    /// the shape corpus PR 1403 measured for `access` of table object `table`, made by the code of object `caller` while the
    /// session level is `u` and the caller's `Permissions` property gives `grant`. Marks the assumption when it is.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn inherent_letter_measured(
        &self,
        access: Access,
        l: Levels,
        frame: usize,
        caller: usize,
        table: Option<usize>,
        u: u8,
        grant: bool,
    ) -> bool {
        // The letter and the access it covers: `'i'` on an Insert (Y19InherentAttrIndirect.al:44-56, ops 1 and 5, allowed with
        // the Permissions property; :112-120, ops 11 and 13, denied without it) and `'r'` on a Read with the Permissions
        // property (:80-85, op 4). `'r'` without it is not measured.
        let letter = match access.op {
            Op::Insert => l == [0, 2, 0, 0, 0],
            Op::Read => l == [2, 0, 0, 0, 0] && grant,
            _ => false,
        };
        // The sets give nothing on the table (`LT IA None`, :28-32, by `Mock.Set`, :198).
        let Some(table) = table else {
            return false;
        };
        let measured = access.kind == 0
            && letter
            && u == 0
            && self.record_calls.is_empty()
            && self.probed_sets(table)
            && self.inherent_letter_frames(frame, caller, access);
        if measured {
            assume(Assumption::InherentAttributeIndirectLetter);
        }
        measured
    }

    /// Whether a denied Read of table object `table` by codeunit `by`, whose Permissions property grants it, is in the shape
    /// corpus PR 1403 measured with an attribute on the stack whose letter does not cover the read: `'m'` on the writer
    /// (`ModifyM`, Y19InherentAttrIndirect.al:59-66, op 2) or on its caller (`ModifyOnCaller`, :47-48, op 6) and `'d'` on the
    /// writer (`DeleteD`, :68-76, op 3), each denied as `IndirectRead` (:168-177); the frames as
    /// [`Session::inherent_letter_frames`]; the sets give nothing on the table. Marks the assumption when it is.
    pub(crate) fn indirect_read_measured(&self, access: Access, table: usize, by: usize) -> bool {
        let n = self.frames.len();
        if access.kind != 0 || access.op != Op::Read || n < 2 || self.call_stack.len() != n {
            return false;
        }
        let engine = self.engine.clone();
        let model = SetModel::of(&engine);
        let letter = |i: usize| -> bool {
            let g = model.inherent_of(&engine, self.frames[i], self.call_stack[i].routine);
            matches!(g.as_slice(), [Ok(Some((0, id, l)))] if *id == access.id && (*l == [0, 0, 2, 0, 0] || *l == [0, 0, 0, 2, 0]))
        };
        let frame = match (letter(n - 2), letter(n - 1)) {
            (true, false) => n - 2,
            (false, true) => n - 1,
            _ => return false,
        };
        let measured = !engine.program.objects[by].properties.is_empty()
            && self.record_calls.is_empty()
            && self.levels_are(access, &[[0, 0, 0, 0]])
            && self.probed_sets(table)
            && self.inherent_letter_frames(frame, by, access);
        if measured {
            assume(Assumption::InherentAttributeIndirectLetter);
        }
        measured
    }

    /// The measured frames: the test method, RunCase and Op of the test codeunit (:150-230), then exactly two procedures of
    /// codeunit `caller`, the attribute on one of them and none on the other: the public procedure calls the local one that
    /// writes (`InsertOnWriter` / `InsertI`, `ReadOnWriter` / `ReadR`, the attribute on the writer) or the public procedure
    /// with the attribute calls a local one without (`InsertOnCaller` / `InsertPlain`). The codeunit has no property or only
    /// `Permissions = tabledata <the table> = rimd` (:35-37, :106).
    fn inherent_letter_frames(&self, frame: usize, caller: usize, access: Access) -> bool {
        let n = self.frames.len();
        if n < 2 || self.call_stack.len() != n || (frame != n - 1 && frame != n - 2) {
            return false;
        }
        let engine = self.engine.clone();
        let o = &engine.program.objects[caller];
        let own = (n - 2..n).all(|i| {
            let f = &self.call_stack[i];
            let attrs = o.routines.get(f.routine).map(|r| (r.is_trigger, r.attributes.as_slice()));
            let attributes_ok = match attrs {
                Some((false, [a])) if i == frame => a.name.eq_ignore_ascii_case("InherentPermissions"),
                Some((false, [])) => i != frame,
                _ => false,
            };
            self.frames[i] == caller && !f.event && attributes_ok
        });
        let properties_ok = match o.properties.as_slice() {
            [] => true,
            [(k, _)] if k.eq_ignore_ascii_case("Permissions") => {
                match &*SetModel::of(&engine).object_grants(&engine, caller) {
                    Ok(c) => {
                        matches!(c.explicit.as_slice(), [(0, id, [2, 2, 2, 2, 0])] if *id == access.id)
                            && c.wild == crate::permissions::Wildcards::default()
                    }
                    Err(_) => false,
                }
            }
            _ => false,
        };
        o.kind == al_ir::ObjKind::Codeunit && own && properties_ok && self.only_test_codeunit_frames(caller, 2)
    }
}
