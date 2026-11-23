//! The import of a linked table element whose SourceTableView holds one `const` filter on a field the document does not
//! carry (corpus PR 1550, tests/al-language/xmlports/shapes-properties/XpAutoUpdateAssign.al:629-641 element `SuiteEval`,
//! cell Suite_Fresh_SuiteEvaluator at :1047-1057: `where(LineNo = const(0))` on an Integer field whose initial value is 0,
//! the row is stored with 0, `rows` `EA/0/e0`). The shape of the AI Test Toolkit's suite-level Evaluator element (xmlport
//! 149031, al-runner issue 8877). Assumption `xmlport-linked-view-const` until the probe is pinned.
//!
//! What the cell measures is the stored row of the first such element under its parent when the field holds the filter's
//! value at the save. Whether code sees the filter on the record is not measured: no routine of the port may name the
//! element's record. Every other linked element with a filtering view keeps main's refusal.

use super::*;

const LINKED_VIEW_FLOW: &str = "XmlPort import of a linked table element whose SourceTableView has a const filter, skipped by its trigger (not pinned)";
const LINKED_VIEW_OTHER_VALUE: &str = "XmlPort import of a linked table element whose SourceTableView const filter's field holds another value at the save (not pinned)";

/// The field position and value of the one `const` filter of the SourceTableView of `node`, when the element is in the
/// measured shape: right below a table element (`Suite`, :556) that stands right below the root text element of a Format =
/// Xml port and has AutoSave without AutoUpdate, AutoReplace or UseTemporary; linked to it by LinkTable with one LinkFields
/// pair (:631-632); MinOccurs = Zero (:633); AutoSave without AutoUpdate, AutoReplace or UseTemporary; only field attributes
/// of its own without triggers below it (:637), none of them nor the link field on the filtered field; the view one
/// `where(<Integer field> = const(..))` (:635); an OnAfterInitRecord as its only trigger, which calls no procedure of the
/// port and uses the port only to call `Skip` (:639-641); and no routine of the port naming its record.
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
        first_of_node: bool,
    ) -> Result<Option<LinkedViewConst>, Signal> {
        let n = schema.node(node);
        let filtering_link = n.link_node.is_some()
            && n.source_view
                .as_deref()
                .is_some_and(|v| crate::views::view_clauses(v).filters);
        if !filtering_link {
            return Ok(None);
        }
        // The cell holds one row of the element under its parent: a later row is not measured.
        if first_of_node && let Some(c) = self.xp_linked_view_const(inst, schema, node)? {
            return Ok(Some(c));
        }
        unsupported(format!(
            "XmlPort import: sourcetableview with a filter on the linked tableelement {} is not pinned",
            n.name
        ))
    }

    /// Refuses when the measured element's OnAfterInitRecord asked for Skip, Break or BreakUnbound. Checked right after
    /// that trigger: the site of the first field attribute would take the request.
    pub(super) fn xp_linked_view_after_init(inst: &Inst, view_const: &Option<LinkedViewConst>) -> Result<(), Signal> {
        if view_const.is_some() && Self::xp_flow(inst).is_some() {
            return unsupported(LINKED_VIEW_FLOW);
        }
        Ok(())
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
        self.xp_holds_view_const(inst, schema, node, c)?;
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::XmlportLinkedViewConst);
        self.xp_save_record(inst, schema, node, assigned, Save::Once)?;
        self.xp_holds_view_const(inst, schema, node, c)
    }

    fn xp_linked_view_const(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
    ) -> Result<Option<LinkedViewConst>, Signal> {
        let n = schema.node(node);
        let Some(view) = n.source_view.as_deref() else {
            return Ok(None);
        };
        let Some(slot) = n.slot else {
            return Ok(None);
        };
        let clauses = crate::views::view_clauses(view);
        // The parent `Suite` (:556): a table element right below the root, AutoSave without AutoUpdate, AutoReplace or
        // UseTemporary.
        let parent_measured = n.parent.is_some_and(|p| {
            let pn = schema.node(p);
            pn.kind == ir::XmlNodeKind::Table && below_root(schema, p) && plain_autosave(pn)
        });
        let field_attrs_only = schema.children(node).iter().all(|&c| {
            let cn = schema.node(c);
            cn.kind == ir::XmlNodeKind::FieldAttr && cn.table_node == Some(node) && cn.triggers.is_empty()
        });
        let shape = schema.xp().format == ir::XmlFormat::Xml
            && clauses.filters
            && !clauses.sorting
            && !clauses.order
            && parent_measured
            && n.link_node == n.parent
            && n.link_fields.len() == 1
            && n.min_zero
            && plain_autosave(n)
            && field_attrs_only
            && n.triggers
                .iter()
                .all(|(name, r)| name == "onafterinitrecord" && trigger_only_skips(schema, *r))
            && !port_names_global(schema, slot);
        if !shape {
            return Ok(None);
        }
        // The view's filters as a record holds them, read from a group of their own and taken off again.
        let rec = self.xp_record(inst, n)?;
        let t = self.table(&rec)?;
        let kept = rec.borrow().clone();
        let view = crate::views::property_view(view);
        self.set_view_in(&rec, &t, &view, Some(XP_VIEW_GROUP), false, false, false)?;
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
        // `where(LineNo = const(0))` on an Integer field: a `filter(0)` reads as the same filter and is not measured.
        let const_form = crate::views::single_where_const(&t, &view) == Some(only.field);
        let integer = t.fields[only.field].ty == RtType::Integer;
        let carried = schema
            .children(node)
            .iter()
            .any(|&c| schema.node(c).field == Some(only.field));
        let linked = n.link_fields.iter().any(|&(own, _)| own == only.field);
        if !const_form || !integer || carried || linked {
            return Ok(None);
        }
        Ok(Some(LinkedViewConst {
            field: only.field,
            value: value.clone(),
        }))
    }

    /// Refuses unless the record of `node` holds the filter's value in the filtered field: the measured row, before and
    /// after its save.
    fn xp_holds_view_const(
        &mut self,
        inst: &Inst,
        schema: &Schema,
        node: usize,
        c: &LinkedViewConst,
    ) -> Result<(), Signal> {
        let rec = self.xp_record(inst, schema.node(node))?;
        let r = rec.borrow();
        if ops::compare(&r.fields[c.field], &c.value)? == std::cmp::Ordering::Equal {
            Ok(())
        } else {
            unsupported(LINKED_VIEW_OTHER_VALUE)
        }
    }
}

/// Whether trigger routine `r` of the port calls no procedure of the port and uses the port itself (`currXMLport`) only as
/// the receiver of `Skip`, as the measured OnAfterInitRecord (XpAutoUpdateAssign.al:637-641, element SuiteEval) does.
fn trigger_only_skips(schema: &Schema, r: usize) -> bool {
    let routine = &schema.engine.program.objects[schema.object].routines[r];
    routine.body.read(|stmts| {
        let mut port_calls = 0usize;
        let mut this_uses = 0usize;
        let mut skips = 0usize;
        ir::types::for_each_expr_post(stmts, &mut |e| match e {
            ir::Expr::ThisInstance => this_uses += 1,
            ir::Expr::Call { object, .. } if *object == schema.object => port_calls += 1,
            ir::Expr::Builtin {
                id,
                receiver: Some(recv),
                ..
            } if *id == ids::XMLPORT_SKIP_1 && matches!(recv.as_ref(), ir::Arg::Value(ir::Expr::ThisInstance)) => {
                skips += 1
            }
            _ => {}
        });
        port_calls == 0 && this_uses == skips
    })
}

/// Whether a routine of the port (a trigger of any node, of the port, or a procedure) names global `slot`: reads, writes or
/// passes it. The runner does not put the view's filter on the record, which the cell does not measure, so no code of the
/// port may look at the record. (`ClearAll` in an xmlport is refused before the run.)
fn port_names_global(schema: &Schema, slot: usize) -> bool {
    schema.engine.program.objects[schema.object]
        .routines
        .iter()
        .any(|routine| routine.body.read(|stmts| stmts_name_global(stmts, slot)))
}

fn stmts_name_global(stmts: &[ir::Stmt], slot: usize) -> bool {
    let mut named = stmts.iter().any(|s| stmt_place_names(s, slot));
    ir::types::for_each_expr_post(stmts, &mut |e| named |= expr_names_global(e, slot));
    named
}

/// Whether a place a statement of `s` writes (an assignment or a loop variable), or one nested in it, starts at `slot`.
fn stmt_place_names(s: &ir::Stmt, slot: usize) -> bool {
    use ir::Stmt;
    let any = |b: &[Stmt]| b.iter().any(|s| stmt_place_names(s, slot));
    match s {
        Stmt::Assign(p, _) => root_global(p) == Some(slot),
        Stmt::For { var, body, .. } | Stmt::Foreach { var, body, .. } => root_global(var) == Some(slot) || any(body),
        Stmt::If(_, a, b) => any(a) || any(b),
        Stmt::While(_, b) | Stmt::Repeat(b, _) | Stmt::AssertError(b) => any(b),
        Stmt::Case {
            branches, otherwise, ..
        } => branches.iter().any(|(_, b)| any(b)) || any(otherwise),
        Stmt::Expr(_)
        | Stmt::Exit(_)
        | Stmt::Break
        | Stmt::Pos(_)
        | Stmt::Assume(_)
        | Stmt::TddMark(_)
        | Stmt::Continue => false,
    }
}

/// Whether expression node `e` itself (not the nodes inside it) names `slot`: a load, a `var` argument or a receiver.
fn expr_names_global(e: &ir::Expr, slot: usize) -> bool {
    use ir::Expr;
    let arg = |a: &ir::Arg| matches!(a, ir::Arg::Ref(p) if root_global(p) == Some(slot));
    match e {
        Expr::Load(p) => root_global(p) == Some(slot),
        Expr::Call { args, .. } | Expr::InterfaceCall { args, .. } | Expr::DotNet { args, .. } => args.iter().any(arg),
        Expr::Builtin { receiver, args, .. } => receiver.as_deref().is_some_and(arg) || args.iter().any(arg),
        Expr::Const(_)
        | Expr::Unary(..)
        | Expr::Binary(..)
        | Expr::In(..)
        | Expr::Ternary(..)
        | Expr::Convert(..)
        | Expr::BaseInstance
        | Expr::ThisInstance
        | Expr::Refused(_)
        | Expr::Fail { .. }
        | Expr::ControlAddInCall { .. }
        | Expr::InterfaceTest { .. }
        | Expr::EnumImpl { .. }
        | Expr::Control { .. }
        | Expr::FieldOf(..)
        | Expr::QueryColumn { .. }
        | Expr::CharAt(..)
        | Expr::DotNetCoerce(..)
        | Expr::DotNetOperand(..)
        | Expr::TddStub { .. }
        | Expr::TddFit { .. }
        | Expr::FitOrRefuse { .. }
        | Expr::VarSectionValue(_) => false,
    }
}

/// The global slot a place starts from.
fn root_global(p: &ir::Place) -> Option<usize> {
    match p {
        ir::Place::Global(g) => Some(*g),
        ir::Place::Index(b, _) | ir::Place::Field(b, _) | ir::Place::RecGlobal(b, ..) => root_global(b),
        ir::Place::Local(_) | ir::Place::BaseGlobal(_) | ir::Place::ExtGlobal(..) => None,
    }
}
