//! The import of a linked table element whose SourceTableView holds one `const` filter on a field the document does not
//! carry (corpus PR 1550, tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al:629-641 element `SuiteEval`,
//! cell Suite_Fresh_SuiteEvaluator at :1047-1057: `where(LineNo = const(0))` on a field whose initial value is 0, the row
//! is stored with 0, `rows` `EA/0/e0`). The shape of the AI Test Toolkit's suite-level Evaluator element (xmlport 149031,
//! al-runner issue 8877). Assumption `xmlport-linked-view-const` until the probe is pinned.
//!
//! What the cell measures is the stored row when the field holds the filter's value at the save. Whether the triggers see
//! the filter on the record is not measured: the element's triggers must not name its record or call a procedure of the
//! port. Every other linked element with a filtering view keeps main's refusal.

use super::*;

const LINKED_VIEW_OTHER_VALUE: &str = "XmlPort import of a linked table element whose SourceTableView const filter's field holds another value at the save (not pinned)";

/// The field position and value of the one `const` filter of the SourceTableView of `node`, when the element is in the
/// measured shape: right below a table element right below the root text element of a Format = Xml port, linked to it
/// (LinkTable = that element), MinOccurs = Zero (:633), AutoSave without AutoUpdate, AutoReplace or UseTemporary, only
/// field attributes (`fieldattribute`, as `Name` of `SuiteEval`) without triggers below it, none of them nor a link field
/// on the filtered field, and an OnAfterInitRecord as its only trigger, which neither names its record nor calls a
/// procedure of the port.
pub(super) struct LinkedViewConst {
    pub field: usize,
    pub value: Value,
}

impl Session {
    /// The measured `const` shape of a linked element whose SourceTableView filters; `None` for an element that is not
    /// linked or whose view does not filter. Every other linked element with a filtering view keeps main's refusal.
    pub(super) fn xp_linked_view_entry(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
    ) -> Result<Option<LinkedViewConst>, Signal> {
        let n = schema.node(node);
        let filtering_link = n.link_node.is_some()
            && n.source_view
                .as_deref()
                .is_some_and(|v| crate::views::view_clauses(v).filters);
        if !filtering_link {
            return Ok(None);
        }
        match self.xp_linked_view_const(inst, schema, node)? {
            Some(c) => Ok(Some(c)),
            None => unsupported(format!(
                "XmlPort import: sourcetableview with a filter on the linked tableelement {} is not pinned",
                n.name
            )),
        }
    }

    /// Saves the record of the measured element: the row holds the filter's value at its save, and keeps it.
    pub(super) fn xp_save_view_const(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        c: &LinkedViewConst,
        assigned: &[usize],
    ) -> Result<(), Signal> {
        if !self.xp_holds_view_const(inst, schema, node, c)? {
            return unsupported(LINKED_VIEW_OTHER_VALUE);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportLinkedViewConst);
        self.xp_save_record(inst, schema, node, assigned, false)?;
        if !self.xp_holds_view_const(inst, schema, node, c)? {
            return unsupported(LINKED_VIEW_OTHER_VALUE);
        }
        Ok(())
    }

    pub(super) fn xp_linked_view_const(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
    ) -> Result<Option<LinkedViewConst>, Signal> {
        let n = schema.node(node);
        let Some(view) = n.source_view.as_deref() else {
            return Ok(None);
        };
        let clauses = crate::views::view_clauses(view);
        let parent_below_root = n.parent.is_some_and(|p| {
            let pn = schema.node(p);
            pn.kind == ir::XmlNodeKind::Table
                && pn
                    .parent
                    .is_some_and(|r| schema.node(r).parent.is_none() && schema.node(r).kind == ir::XmlNodeKind::Text)
        });
        let field_attrs_only = schema.children(node).iter().all(|&c| {
            let cn = schema.node(c);
            cn.kind == ir::XmlNodeKind::FieldAttr && cn.table_node == Some(node) && cn.triggers.is_empty()
        });
        let shape = schema.xp().format == ir::XmlFormat::Xml
            && clauses.filters
            && !clauses.sorting
            && !clauses.order
            && parent_below_root
            && n.link_node == n.parent
            && n.min_zero
            && n.auto_save
            && !n.auto_update
            && !n.auto_replace
            && !n.use_temporary
            && field_attrs_only
            && n.triggers
                .iter()
                .all(|(name, r)| name == "onafterinitrecord" && trigger_leaves_record(schema, *r, n.slot));
        if !shape {
            return Ok(None);
        }
        // The view's filters as a record holds them, read from a group of their own and taken off again.
        let rec = self.xp_record(inst, n)?;
        let t = self.table(&rec)?;
        let kept = rec.borrow().clone();
        self.set_view_in(
            &rec,
            &t,
            &crate::views::property_view(view),
            Some(XP_VIEW_GROUP),
            false,
            false,
            false,
        )?;
        let filters: Vec<_> = rec
            .borrow()
            .filters
            .iter()
            .filter(|f| f.group == XP_VIEW_GROUP)
            .cloned()
            .collect();
        *rec.borrow_mut() = kept;
        let [only] = filters.as_slice() else {
            return Ok(None);
        };
        let al_builtins::filter::FilterExpr::Eq(value) = &only.expr else {
            return Ok(None);
        };
        let carried = schema
            .children(node)
            .iter()
            .any(|&c| schema.node(c).field == Some(only.field));
        let linked = n.link_fields.iter().any(|&(own, _)| own == only.field);
        if carried || linked {
            return Ok(None);
        }
        Ok(Some(LinkedViewConst {
            field: only.field,
            value: value.clone(),
        }))
    }

    /// Whether the record of `node` holds the filter's value in the filtered field: the measured row, before and after its
    /// save.
    fn xp_holds_view_const(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        c: &LinkedViewConst,
    ) -> Result<bool, Signal> {
        let rec = self.xp_record(inst, schema.node(node))?;
        let r = rec.borrow();
        Ok(ops::compare(&r.fields[c.field], &c.value)? == std::cmp::Ordering::Equal)
    }
}

/// Whether trigger routine `r` of the port leaves the element's record (global `slot`) alone: no read, write, receiver or
/// `var` argument of it, no call of a procedure of the port, and the port itself (`currXMLport`) only as the receiver of
/// `Skip`, as in the measured OnAfterInitRecord (XpAutoUpdateAssign.al, element SuiteEval).
fn trigger_leaves_record(schema: &Schema, r: usize, slot: Option<usize>) -> bool {
    let Some(slot) = slot else { return false };
    let routine = &schema.engine.program.objects[schema.object].routines[r];
    routine.body.read(|stmts| {
        let mut ok = true;
        let mut this_uses = 0usize;
        let mut skips = 0usize;
        let names = |p: &ir::Place| root_global(p) == Some(slot);
        let names_arg = |a: &ir::Arg| matches!(a, ir::Arg::Ref(p) if names(p));
        ir::types::for_each_expr_post(stmts, &mut |e| {
            let args: &[ir::Arg] = match e {
                ir::Expr::Load(p) => {
                    ok &= !names(p);
                    &[]
                }
                ir::Expr::ThisInstance => {
                    this_uses += 1;
                    &[]
                }
                ir::Expr::Call { object, args, .. } => {
                    ok &= *object != schema.object;
                    args
                }
                ir::Expr::Builtin { id, receiver, args, .. } => {
                    if let Some(recv) = receiver.as_deref() {
                        ok &= !names_arg(recv);
                        if *id == ids::XMLPORT_SKIP_1 && matches!(recv, ir::Arg::Value(ir::Expr::ThisInstance)) {
                            skips += 1;
                        }
                    }
                    args
                }
                ir::Expr::InterfaceCall { args, .. } | ir::Expr::DotNet { args, .. } => args,
                _ => &[],
            };
            ok &= !args.iter().any(names_arg);
        });
        ok && this_uses == skips && stmts.iter().all(|s| stmt_places_leave(s, &names))
    })
}

/// Whether no place a statement of `s` writes (an assignment or a loop variable) is named by `names`.
fn stmt_places_leave(s: &ir::Stmt, names: &dyn Fn(&ir::Place) -> bool) -> bool {
    use ir::Stmt;
    let all = |b: &[Stmt]| b.iter().all(|s| stmt_places_leave(s, names));
    match s {
        Stmt::Assign(p, _) => !names(p),
        Stmt::For { var, body, .. } | Stmt::Foreach { var, body, .. } => !names(var) && all(body),
        Stmt::If(_, a, b) => all(a) && all(b),
        Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => all(b),
        Stmt::Case {
            branches, otherwise, ..
        } => branches.iter().all(|(_, b)| all(b)) && all(otherwise),
        _ => true,
    }
}

/// The global slot a place starts from.
fn root_global(p: &ir::Place) -> Option<usize> {
    match p {
        ir::Place::Global(g) => Some(*g),
        ir::Place::Index(b, _) | ir::Place::Field(b, _) | ir::Place::RecGlobal(b, ..) => root_global(b),
        _ => None,
    }
}
