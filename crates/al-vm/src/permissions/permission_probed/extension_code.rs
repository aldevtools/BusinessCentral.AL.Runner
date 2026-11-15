//! Which `Permissions` property covers the code of a table extension or page extension (al-runner issue 8093).
//!
//! An extension object cannot declare `Permissions` (AL0246). Corpus PR 1403 (merged, not pinned; permissions/testpermissions/
//! Y19PermExtAndCompose.al, test PC_B_ExtensionCode) measured an Insert into another table from that code on BC 27.5, 28.5
//! and 29.0 (assumption `extension-code-object-grant`):
//! - table extension code (its OnInsert trigger, a procedure of it): the base table's `Permissions` is the object grant on
//!   27 and 28 and none is on 29 (`al_version::base_table_permissions_cover_extension_code`);
//! - page extension OnOpenPage: the base page's `Permissions` is the object grant on every measured major
//!   (`al_version::base_page_permissions_cover_page_extension_open_page`), and a denial
//!   closes the page with `Page <name> has to close.`, code `FormAbort:Permission`.
//!
//! The measured shapes are allowlists of their facts here; every other access of extension code stays refused.
use crate::Session;
use crate::permission_bulk_no_match::{RecordCall, RecordCaller};
use crate::permissions::{Access, Op, SetModel};
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;

impl Session {
    /// Whether the code of extension object `caller` making `access` gets an object grant, in the shapes corpus PR 1403
    /// measured; `None` in every other shape (the caller refuses). Marks the assumption when it answers.
    pub(crate) fn extension_code_grant(&self, access: Access, caller: usize) -> Option<bool> {
        let base = self.measured_extension_base(access, caller)?;
        let engine = self.engine.clone();
        let base_grant = match &*SetModel::of(&engine).object_grants(&engine, base) {
            Ok(c) => c.level_of(access.kind, access.id, access.op as usize) != 0,
            Err(_) => return None,
        };
        let target = engine.target.as_ref();
        let covers = match engine.program.objects.heads()[caller].kind {
            // Y19PermExtAndCompose.al:317-328 (b13, b14 allowed on 27 and 28, denied on 29) and :333-342 (b17, b18
            // `IndirectInsert` on 27 and 28, `Insert` on 29); :349-370 the same for the procedure (b23, b24, b27, b28).
            al_ir::ObjKind::TableExtension => al_version::base_table_permissions_cover_extension_code(target)?,
            // :376-383: b33 allowed (the base page grants), b31, b32, b34 denied.
            _ => al_version::base_page_permissions_cover_page_extension_open_page(target)?,
        };
        let grant = covers && base_grant;
        assume(Assumption::ExtensionCodeObjectGrant);
        Some(grant)
    }

    /// The error of a denied `access` by the code of `by`, when `by` is a page extension in the measured shape: the page
    /// closes (Y19PermExtAndCompose.al:376-383, `Page LT PC Page Plain has to close.`, `FormAbort:Permission`, the base page
    /// named, with or without its Permissions).
    pub(crate) fn extension_page_denial(&self, access: Access, by: usize) -> Option<Signal> {
        let heads = self.engine.program.objects.heads();
        if heads[by].kind != al_ir::ObjKind::PageExtension {
            return None;
        }
        let base = self.measured_extension_base(access, by)?;
        Some(self.page_close_for_permission(base))
    }

    /// The base object of extension `caller` when its code makes `access` in a shape corpus PR 1403 measured: an Insert into a
    /// table of the same app that runs no trigger (`LT PC B`, Y19PermExtAndCompose.al:37), the sets giving `imd` or nothing on
    /// it (:214-226) as `Permissions Mock.Set` assigns them (:518-521), from
    /// - the OnInsert trigger of a table extension (:94-101), started by the statement `Insert(true)` of a Record variable of
    ///   the base table (:580-583), the one record method that runs, or a procedure of it without attributes (:103-109),
    ///   called by the test codeunit while no record method runs (:600-603); the base table declares no routine (it runs only
    ///   its extension's trigger), DataClassification and optionally `Permissions = tabledata "LT PC B" = imd` (:77-92);
    /// - the OnOpenPage trigger of a page extension (:187-196, :202-211), run by `TestPage.OpenView` of the base page
    ///   (:619-628), a Card that declares no routine and whose properties are PageType, SourceTable and optionally
    ///   `Permissions = tabledata "LT PC B" = imd` (:172-185).
    fn measured_extension_base(&self, access: Access, caller: usize) -> Option<usize> {
        let engine = self.engine.clone();
        let heads = engine.program.objects.heads();
        let ext = &engine.program.objects[caller];
        let base_kind = ext.kind.extended()?;
        let base = heads
            .iter()
            .position(|o| o.kind == base_kind && ext.extends_id.is_some() && o.id == ext.extends_id)?;
        let table = heads
            .iter()
            .position(|o| o.kind == al_ir::ObjKind::Table && o.id == Some(access.id))?;
        let (f, &last) = (self.call_stack.last()?, self.frames.last()?);
        let routine = ext.routines.get(f.routine)?;
        let keys = |o: usize| {
            let mut k: Vec<String> = engine.program.objects[o]
                .properties
                .iter()
                .map(|(k, _)| k.to_ascii_lowercase())
                .collect();
            k.sort();
            k
        };
        let base_grant_ok = match &*SetModel::of(&engine).object_grants(&engine, base) {
            Ok(c) => {
                c.wild == crate::permissions::Wildcards::default()
                    && (c.explicit.is_empty()
                        || matches!(c.explicit.as_slice(), [(0, id, [0, 2, 2, 2, 0])] if *id == access.id))
            }
            Err(_) => false,
        };
        let base_declares_no_routine = engine.program.objects[base].routines.is_empty();
        let code_ok = base_declares_no_routine
            && match ext.kind {
                al_ir::ObjKind::TableExtension => {
                    let start = RecordCall::statement(RecordOp::Insert, RecordCaller::Record, base, false, true);
                    let trigger = routine.is_trigger
                        && routine.field.is_none()
                        && routine.name.eq_ignore_ascii_case("OnInsert")
                        && self.record_calls.as_slice() == [start];
                    let procedure =
                        !routine.is_trigger && routine.attributes.is_empty() && self.record_calls.is_empty();
                    let props = keys(base);
                    (trigger || procedure)
                        && (props == ["dataclassification"] || props == ["dataclassification", "permissions"])
                }
                al_ir::ObjKind::PageExtension => {
                    let props = keys(base);
                    routine.is_trigger
                        && routine.name.eq_ignore_ascii_case("OnOpenPage")
                        && self.record_calls.is_empty()
                        && self.page_open_by_test_open_view(base)
                        && matches!(engine.object_facts[base].page_type, Some(al_ir::PageType::Card))
                        && (props == ["pagetype", "sourcetable"] || props == ["pagetype", "permissions", "sourcetable"])
                }
                _ => false,
            };
        let measured = access.kind == 0
            && access.op == Op::Insert
            && self.table_runs_no_trigger(table)
            && heads[table].app == heads[caller].app
            && heads[base].app == heads[caller].app
            && last == caller
            && !f.event
            && code_ok
            && base_grant_ok
            && self.levels_are(access, &[[0, 2, 2, 2], [0, 0, 0, 0]])
            && self.probed_sets(table)
            && self.only_test_codeunit_frames(caller, 1);
        measured.then_some(base)
    }
}
