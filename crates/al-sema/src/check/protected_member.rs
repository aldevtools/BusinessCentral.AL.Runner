//! `Other.<protected var>` in a table extension: a protected variable of the extended table or of another extension of it
//! (the ones the extension sees by bare name), named through another record variable of the same table (al-runner issue
//! 6984). Probe: corpus PR 1126 (branch al-runtime-rs-staging-r6983), table-objects/extensions/ProtectedVarCrossApp.al
//! (ProtVar_OtherRecordVariable_MemberForm_OtherApp, ProtVar_LocalRecordVariable_MemberForm,
//! ProtVar_OtherRecordVariable_MemberForm_SameApp, ProtVar_SalesLineStatusCheckSuspended_MemberForm): it reads and writes
//! Other's variable, not Rec's. A codeunit does not see the variable (must-not-compile 2953, AL0161).
use super::*;

impl<'a> Checker<'a> {
    /// Records [`Res::RecGlobal`] for `Other.Name` when the code is a table extension, `Other` a record of the table it
    /// extends, and `Name` a protected variable it sees by bare name that no field or procedure of the table hides.
    pub(super) fn record_protected_member(
        &mut self,
        sc: &Scope,
        node: &SyntaxNode,
        base: &Ty,
        name: &str,
    ) -> Option<E> {
        let Ty::Record { table, .. } = base else {
            return None;
        };
        if self.ws.objects[sc.oi].kind != ObjKind::TableExtension {
            return None;
        }
        let ti = self.resolve(sc, ObjKind::Table, table)?;
        if self.ws.extension_target(sc.oi) != Some(ti)
            || self.members[ti].field(name).is_some()
            || !self.members[ti].procedures_named(name).is_empty()
        {
            return None;
        }
        let key = name.to_lowercase();
        let object = if sc.base_vars.contains(&key) {
            ti
        } else {
            *sc.ext_vars.get(&key)?
        };
        let ty = sc.globals.get(&key)?.clone();
        self.record(sc, node, Res::RecGlobal { object, name: key });
        Some(E::Val(ty))
    }
}
