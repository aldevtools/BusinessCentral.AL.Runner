//! The read that a system table's own FlowField makes of an internal table when the session's permission sets do not grant
//! that table (ADR-062). Only "Published Application"."Tenant Visible" over "NAV App Extra", in the two shapes Microsoft's
//! Guided Experience tests reach under `Guided Exp Edit`, runs (al-runner issue 8038): shape 1 as
//! permissions/SysFlowFieldPermFilter.al pins it, shape 2 as permissions/SysFlowFieldPerm.al pins it. When the sets grant
//! nothing on Published Application, the CalcFields of shape 2's FlowFields raises the `IndirectRead` denial of Published
//! Application (`Session::published_app_calc_denied`). Every other shape keeps the refusal of
//! `Session::permission_gate_flowfield_decided`.

use std::rc::Rc;

use crate::Session;
use crate::flowfields::{Asked, CalcFrame};
use crate::permissions::{Access, Op};
use crate::records::Rec;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};
use al_ir as ir;

/// "Published Application".
const PUBLISHED_APPLICATION: u32 = 2_000_000_206;
/// "NAV App Extra", the source table of "Tenant Visible".
const NAV_APP_EXTRA: u32 = 2_000_000_157;
/// Field numbers of Published Application: ID, Name, Publisher, "Tenant Visible".
const PA_ID: u32 = 3;
const PA_NAME: u32 = 4;
const PA_PUBLISHER: u32 = 5;
const PA_TENANT_VISIBLE: u32 = 30;
/// Object ids from here on are the platform's own tables.
const SYSTEM_TABLES: u32 = 2_000_000_000;

impl Session {
    /// Runs `f`, a step of the Find method `method` on `rec` (its scan for the first row, the load of the row it found), with
    /// `find_first_read` set while it runs when it is shape 1: [`Session::is_al_find_first_of_fresh_record`].
    pub(crate) fn as_find_first<T>(&mut self, method: RecordOp, rec: &Rec, f: impl FnOnce(&mut Self) -> T) -> T {
        let shape = method == RecordOp::FindFirst && self.is_al_find_first_of_fresh_record(rec);
        let saved = std::mem::replace(&mut self.find_first_read, shape);
        let r = f(self);
        self.find_first_read = saved;
        r
    }

    /// Notes that AL code calls `method` on the Record variable `rec` (`Ctx::record`), until the returned value is put back
    /// with [`Session::end_al_record_call`]. A RecordRef method and the runtime's own record calls (pages, reports,
    /// xmlports, record links) do not note it.
    pub(crate) fn begin_al_record_call(&mut self, method: RecordOp, rec: &Rec) -> Option<(usize, RecordOp)> {
        self.al_record_call.replace((Rc::as_ptr(rec) as usize, method))
    }

    /// Ends the call [`Session::begin_al_record_call`] noted, back to what it replaced.
    pub(crate) fn end_al_record_call(&mut self, before: Option<(usize, RecordOp)>) {
        self.al_record_call = before;
    }

    /// Whether AL code called `method` on the Record variable `rec` and that call is running.
    pub(crate) fn is_al_record_call(&self, method: RecordOp, rec: &Rec) -> bool {
        self.al_record_call == Some((Rc::as_ptr(rec) as usize, method))
    }

    /// Who asks for the FlowFields of `Record.CalcFields`, `CalcSums` or `SetAutoCalcFields` on `rec`: [`Asked::CalcFields`]
    /// for the AL call `Record.CalcFields` on a Record variable, [`Asked::Other`] for every other caller of the record method.
    pub(crate) fn asked_by_calc_fields(&self, method: RecordOp, rec: &Rec) -> Asked {
        if method == RecordOp::CalcFields && self.is_al_record_call(RecordOp::CalcFields, rec) {
            Asked::CalcFields
        } else {
            Asked::Other
        }
    }

    /// `CalcFields`, `CalcSums` and `SetAutoCalcFields` on a Record method call: [`Self::flow_op_asked`] with the caller from
    /// [`Self::asked_by_calc_fields`]. Shape 2 is pinned for a CalcFields of one FlowField called by codeunit code
    /// (permissions/SysFlowFieldPerm.al:494-540, `Op` of the test codeunit): CalcFields of two or more fields, or with a page,
    /// a table or another object on the call stack, asks as [`Asked::Other`], so shape 2 does not take it.
    pub(crate) fn flow_op(&mut self, method: RecordOp, rec: &Rec, args: &[Value]) -> Result<Value, Signal> {
        let asked = match self.asked_by_calc_fields(method, rec) {
            Asked::CalcFields if args.len() == 1 && self.called_by_codeunits_only() => Asked::CalcFields,
            Asked::CalcFields => Asked::Other,
            other => other,
        };
        self.flow_op_asked(method, rec, args, asked)
    }

    /// Every routine on the call stack is a codeunit's.
    fn called_by_codeunits_only(&self) -> bool {
        let heads = self.engine.program.objects.heads();
        self.frames.iter().all(|&o| heads[o].is_kind(ir::ObjKind::Codeunit))
    }

    /// Shape 1 starts: the AL call `Record.FindFirst` on a Record variable `rec` that is not temporary and holds the state of
    /// Guided Experience's fresh variable (`GetExtensionLogo`): default key, ascending, no SetLoadFields, no
    /// SetAutoCalcFields, default SecurityFiltering and ReadIsolation (the first option of each), filter group 0 current, MarkedOnly off.
    fn is_al_find_first_of_fresh_record(&self, rec: &Rec) -> bool {
        let r = rec.borrow();
        self.is_al_record_call(RecordOp::FindFirst, rec)
            && !r.temporary
            && r.key == 0
            && r.adhoc.is_none()
            && r.ascending
            && r.field_descending == 0
            && r.load.request.is_none()
            && r.auto_calc.is_empty()
            && r.security_filtering == 0
            && r.read_isolation == 0
            && r.filter_group == 0
            && !r.marked_only
    }

    /// Whether the read of source table `table` (object index) for the FlowField on top of `stack`, of record `rec`, is a
    /// measured read that the session's sets do not check. Every fact is required by equality:
    /// - the FlowField is "Published Application"."Tenant Visible" (field 30) over "NAV App Extra";
    /// - the session level is Read = Yes on Published Application and blank on NAV App Extra (`Guided Exp Edit`, the
    ///   issue's set);
    /// - the shape is one of the two below.
    pub(crate) fn system_flowfield_reads_unchecked(&self, rec: &Rec, table: usize, stack: &[CalcFrame]) -> bool {
        let Some(inner) = stack.last() else {
            return false;
        };
        let heads = self.engine.program.objects.heads();
        let tenant_visible = heads[inner.table].id == Some(PUBLISHED_APPLICATION)
            && self.field_id(inner.table, inner.field) == Some(PA_TENANT_VISIBLE)
            && heads[table].id == Some(NAV_APP_EXTRA);
        let Some(eff) = self.permissions.as_ref() else {
            return false;
        };
        let read = |id: u32| {
            eff.level(Access {
                kind: 0,
                id,
                op: Op::Read,
            })
        };
        let levels = read(PUBLISHED_APPLICATION) == 1 && read(NAV_APP_EXTRA) == 0;
        tenant_visible
            && levels
            && (self.find_first_on_tenant_visible(rec, stack) || self.lookup_on_tenant_visible(stack))
    }

    /// Shape 1 (Guided Experience Impl. `GetExtensionLogo`, reached by Microsoft's Guided Experience tests):
    /// `SetRange(ID, X)`, `SetRange("Tenant Visible", true)` and `FindFirst` on a Published Application Record variable. The
    /// FlowField is asked for by that record's filter while the AL call FindFirst of a fresh, stored record scans for its row
    /// and loads it (`find_first_read`, [`Session::is_al_find_first_of_fresh_record`]), alone on the stack, and the record
    /// holds exactly those two filters, in filter group 0.
    fn find_first_on_tenant_visible(&self, rec: &Rec, stack: &[CalcFrame]) -> bool {
        let [inner] = stack else {
            return false;
        };
        let r = rec.borrow();
        let one_guid = |f: &FieldFilter| matches!(&f.expr, FilterExpr::Range(Some(Value::Guid(a)), Some(Value::Guid(b))) if a == b);
        let filter_is = |f: &FieldFilter, id: u32| f.group == 0 && self.field_id(inner.table, f.field) == Some(id);
        let id_filter = |f: &FieldFilter| filter_is(f, PA_ID) && one_guid(f);
        let visible_filter = |f: &FieldFilter| {
            filter_is(f, PA_TENANT_VISIBLE)
                && matches!(
                    f.expr,
                    FilterExpr::Range(Some(Value::Boolean(true)), Some(Value::Boolean(true)))
                )
        };
        let filters = match r.filters.as_slice() {
            [a, b] => (id_filter(a) && visible_filter(b)) || (visible_filter(a) && id_filter(b)),
            _ => false,
        };
        inner.asked == Asked::RecordFilter && self.find_first_read && filters
    }

    /// Shape 2 ("Guided Experience Item"."Extension Name", reached by the same tests through the AL call `Record.CalcFields`; the
    /// issue's repro; permissions/SysFlowFieldPerm.al:94 `PARead_Found_CalcNamePublisher`, :113 `PARead_Found_CalcExistCount`): a FlowField of an app table (id below 2000000000) that `Record.CalcFields` calculates on a record that is
    /// not temporary, not negated, with exactly the conditions ID = field(F), F a stored Guid field, and "Tenant Visible" = const(true), that is a Lookup of Published Application
    /// Name or Publisher, an Exist or a Count. The stack holds that FlowField and "Tenant Visible", asked for by the condition
    /// of a source row.
    fn lookup_on_tenant_visible(&self, stack: &[CalcFrame]) -> bool {
        let [outer, inner] = stack else {
            return false;
        };
        let formula = matches!(self.published_app_formula(outer, inner.table), Some((_, true)));
        outer.asked == Asked::CalcFields && !outer.temporary && inner.asked == Asked::SourceRow && formula
    }

    /// Whether the denied read of Published Application (`table`, object index) for the FlowField alone on `stack` raises the
    /// `IndirectRead` denial (permissions/SysFlowFieldPerm.al:221-340, NoPA_Found_CalcNamePublisher, NoPA_Found_CalcExistCount,
    /// NoPA_NotFound_CalcNamePublisher, NoPA_NotFound_CalcExistCount; row found or not): the AL call `Record.CalcFields` of one
    /// FlowField of an app table by codeunit code, on a record that is not temporary, the session level blank on Published
    /// Application and on NAV App Extra (the set `SFP NO PA`), and the FlowField one of the probe's
    /// ([`Self::published_app_formula`]): a Lookup of Name with or without the "Tenant Visible" condition, a Lookup of
    /// Publisher with it (`calc_publisher_visible`), an Exist or a Count with or without it.
    pub(crate) fn published_app_calc_denied(&self, table: usize, stack: &[CalcFrame]) -> bool {
        let [outer] = stack else {
            return false;
        };
        let Some(eff) = self.permissions.as_ref() else {
            return false;
        };
        let read = |id: u32| {
            eff.level(Access {
                kind: 0,
                id,
                op: Op::Read,
            })
        };
        let measured = match self.published_app_formula(outer, table) {
            Some((PaFormula::LookupPublisher, visible)) => visible,
            Some((PaFormula::LookupName | PaFormula::Exist | PaFormula::Count, _)) => true,
            None => false,
        };
        self.engine.program.objects.heads()[table].id == Some(PUBLISHED_APPLICATION)
            && read(PUBLISHED_APPLICATION) == 0
            && read(NAV_APP_EXTRA) == 0
            && outer.asked == Asked::CalcFields
            && !outer.temporary
            && measured
    }

    /// The formula of the FlowField `outer`, of an app table (id below 2000000000), over Published Application (`pa`, object
    /// index), when it has the probe's form: not negated, the condition ID = field(F) with F a stored Guid field of the outer
    /// table, and either no other condition or "Tenant Visible" = const(true) (the bool); a Lookup of Name or Publisher, an
    /// Exist or a Count (SysFlowFieldPerm.al:22-66, table "SFP Item").
    fn published_app_formula(&self, outer: &CalcFrame, pa: usize) -> Option<(PaFormula, bool)> {
        let outer_is_app_table = self.engine.program.objects.heads()[outer.table]
            .id
            .is_some_and(|id| id < SYSTEM_TABLES);
        let calc = self.engine.program.objects[outer.table]
            .table
            .as_ref()
            .and_then(|d| d.fields.get(outer.field))
            .and_then(|f| f.calc.as_ref())?;
        let cond_is = |c: &ir::CalcCond, id: u32| self.field_id(pa, c.field) == Some(id);
        // The ID condition compares with a stored (normal) Guid field of the outer table, as the probe's and the issue's table do.
        let stored_guid = |pos: usize| {
            self.engine.program.objects[outer.table]
                .table
                .as_ref()
                .and_then(|d| d.fields.get(pos))
                .is_some_and(|f| f.stored && !f.flow_filter && f.ty == RtType::Guid)
        };
        let id_cond =
            |c: &ir::CalcCond| cond_is(c, PA_ID) && matches!(c.value, ir::CalcValue::Field(pos) if stored_guid(pos));
        let visible_cond = |c: &ir::CalcCond| {
            cond_is(c, PA_TENANT_VISIBLE) && c.value == ir::CalcValue::Const(ir::Const::Boolean(true))
        };
        let visible = match calc.conds.as_slice() {
            [a] if id_cond(a) => false,
            [a, b] if (id_cond(a) && visible_cond(b)) || (visible_cond(a) && id_cond(b)) => true,
            _ => return None,
        };
        let looked_up = calc.field.and_then(|f| self.field_id(pa, f));
        let formula = match calc.method {
            ir::CalcMethod::Lookup if looked_up == Some(PA_NAME) => PaFormula::LookupName,
            ir::CalcMethod::Lookup if looked_up == Some(PA_PUBLISHER) => PaFormula::LookupPublisher,
            ir::CalcMethod::Exist if calc.field.is_none() => PaFormula::Exist,
            ir::CalcMethod::Count if calc.field.is_none() => PaFormula::Count,
            _ => return None,
        };
        (outer_is_app_table && !calc.negate && calc.table == pa).then_some((formula, visible))
    }

    /// The field number of the field at `pos` of table `t` (object index).
    fn field_id(&self, t: usize, pos: usize) -> Option<u32> {
        self.engine.program.objects[t]
            .table
            .as_ref()
            .and_then(|d| d.fields.get(pos))
            .map(|f| f.id)
    }
}

/// What a FlowField over Published Application computes, of the forms the probe measured ([`Session::published_app_formula`]).
#[derive(Clone, Copy)]
enum PaFormula {
    LookupName,
    LookupPublisher,
    Exist,
    Count,
}
