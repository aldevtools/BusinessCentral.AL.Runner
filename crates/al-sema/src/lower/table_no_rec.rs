//! Names in a codeunit with TableNo that the implicit Rec of OnRun can take: a field of the TableNo table named like a
//! system function, and a global variable named `Rec`. OnRun has the implicit Rec; a procedure has none (corpus
//! must-not-compile 3140 and 3141, corpus PR 1386, merged, not pinned yet).
//!
//! What BC runs there is measured by corpus PR 1393 (open, not pinned), tests/al-language/codeunits/
//! TableNoBareSystemFunction.al, on each of the three supported majors, for codeunits without a Subtype only. Until it
//! is pinned the procedure shapes run under assumptions; a codeunit with a Subtype (Test, Install, TestRunner: compiled
//! but not run there, :20-23) keeps main's result.

use super::*;
use al_builtins::assumptions::Assumption;
use al_syntax::ast::NodeText;

/// The codeunit has no Subtype property: the only kind TableNoBareSystemFunction.al runs (its codeunits 83003, 83004).
fn codeunit_without_subtype(obj: &ast::Object) -> bool {
    obj.properties().all(|p| !p.name().eq_ignore_ascii_case("Subtype"))
}

/// A global procedure written as `procedure Name(..)`: no attribute and nothing before the keyword (no access modifier).
/// The kind TableNoBareSystemFunction.al:459-482 reads a global `Rec` in (SetGlobalRec, ProcRead of codeunit 83004).
fn plain_global_procedure(r: &ast::Routine) -> bool {
    r.syntax().kind() == SyntaxKind::Procedure
        && r.attributes().next().is_none()
        && al_syntax::ast::significant_green_tokens(r.syntax())
            .next()
            .is_some_and(|t| t.text.eq_ignore_ascii_case("procedure"))
}

fn in_procedure(node: &SyntaxNode) -> bool {
    node.ancestors()
        .find_map(ast::Routine::cast)
        .is_some_and(|r| r.syntax().kind() == SyntaxKind::Procedure)
}

/// A global `Rec` that a codeunit with TableNo `table` declares. Only main's shape runs: a record of the TableNo table,
/// declared without `temporary`, whose run-time type is the implicit slot's. Any other declared type refuses every routine
/// of the codeunit (a record of another table read `TableName()` as the TableNo table's; review of PR 7980).
///
/// Main's shape in a codeunit without Subtype records the declared variable's slot (`declared_rec_slot`): a procedure
/// reads and writes it, OnRun keeps the implicit Rec (`declared_rec_place`). In a codeunit with a Subtype the declared
/// variable runs as the implicit slot, as on main. Called before the implicit slot takes the name `Rec`.
pub(super) fn declared_rec_refusal(
    lw: &mut Lowerer<'_>,
    kind: ObjKind,
    obj: &ast::Object,
    table: usize,
) -> Option<String> {
    if kind != ObjKind::Codeunit {
        return None;
    }
    let decl = obj
        .var_decls()
        .find(|v| v.names().iter().any(|n| n.eq_ignore_ascii_case("Rec")))?;
    let ty = decl.type_ref().map(|t| Ty::from_type_ref(&t)).unwrap_or(Ty::Unknown);
    let main_shape = matches!(ty, Ty::Record { temporary: false, .. })
        && lw.rt(&ty) == Ok(RtType::Record(table as u32, lw.is_temporary_table(table)));
    if main_shape && codeunit_without_subtype(obj) {
        lw.declared_rec_slot = lw.globals.get("rec").copied();
    }
    (!main_shape).then(|| {
        format!(
            "a global `Rec` declared as `{}` in a codeunit with TableNo: BC's behavior is not measured (only a record of the \
             TableNo table, not temporary, runs, as the codeunit's Rec)",
            decl.type_ref().map(|t| t.syntax().text_string().trim().to_string()).unwrap_or_default()
        )
    })
}

impl Lowerer<'_> {
    /// `Rec` in a codeunit without Subtype that declares main's global `Rec` (`declared_rec_refusal`). In a plain global
    /// procedure it is the declared variable, under `TableNoCodeunitGlobalRecOwnVariable`; in OnRun it is the record given
    /// to Codeunit.Run (the implicit slot, main's result). TableNoBareSystemFunction.al:459-482: `Proc.RecUserId` = the
    /// global, `OnRun.RecUserId` = the run record. Every other routine refuses: main's one slot reads the run record there
    /// after Codeunit.Run, and BC's value is not measured. A codeunit with a Subtype records no declared slot and keeps
    /// main's one slot.
    pub(super) fn declared_rec_place(&mut self, n: &SyntaxNode) -> L<ir::Place> {
        let implicit = *self
            .globals
            .get("rec")
            .ok_or("implicit variable `rec` is not lowered yet")?;
        let Some(slot) = self.declared_rec_slot else {
            return Ok(ir::Place::Global(implicit));
        };
        let routine = n.ancestors().find_map(ast::Routine::cast);
        if routine.as_ref().is_some_and(plain_global_procedure) {
            self.assume(Assumption::TableNoCodeunitGlobalRecOwnVariable);
            return Ok(ir::Place::Global(slot));
        }
        if routine.is_some_and(|r| r.is_trigger() && r.name().eq_ignore_ascii_case("OnRun")) {
            return Ok(ir::Place::Global(implicit));
        }
        Err("`Rec` names the global `Rec` of a codeunit with TableNo outside OnRun and a global procedure without attribute \
             or access modifier: BC's value there is not measured"
            .into())
    }

    /// A call of a system function (`UserId`, `Today`, with or without parentheses, quoted or not) with no receiver,
    /// outside the OnRun trigger of a codeunit with TableNo whose table has a field of that name. There the name is the
    /// system function. In a procedure of a codeunit without Subtype it runs under `TableNoProcedureBareSystemFunction`
    /// (TableNoBareSystemFunction.al:337-417, :421-431, :435-456, :459-482). Every other shape keeps main's refusal: a
    /// trigger other than OnRun and every routine of a codeunit with a Subtype.
    pub(super) fn system_function_named_like_a_rec_field(
        &mut self,
        node: &SyntaxNode,
        receiver: Option<&SyntaxNode>,
    ) -> L<()> {
        let Some(table) = self.table_of[self.oi].filter(|_| self.ws.objects[self.oi].kind == ObjKind::Codeunit) else {
            return Ok(());
        };
        let callee = match node.kind() {
            SyntaxKind::CallExpr => node.children().next(),
            _ => Some(node.clone()),
        };
        let Some(callee) = callee.filter(|c| receiver.is_none() && c.kind() == SyntaxKind::NameRef) else {
            return Ok(());
        };
        let name = callee.text_string().trim().trim_matches('"').to_string();
        let in_on_run = node
            .ancestors()
            .find_map(ast::Routine::cast)
            .is_some_and(|r| r.is_trigger() && r.name().eq_ignore_ascii_case("OnRun"));
        if in_on_run || self.members[table].field(&name).is_none() {
            return Ok(());
        }
        let assumed_codeunit = node
            .ancestors()
            .find_map(ast::Object::cast)
            .is_some_and(|o| codeunit_without_subtype(&o));
        if in_procedure(node) && assumed_codeunit {
            self.assume(Assumption::TableNoProcedureBareSystemFunction);
            return Ok(());
        }
        Err(format!(
            "the system function `{name}` called bare outside OnRun of a codeunit with TableNo whose table has a field `{name}`: \
             its value there is not measured"
        ))
    }
}
