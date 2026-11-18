//! The xRec of table code. What a Validate hands its triggers (`RecordOp::Validate`): nested in a Validate of the same record,
//! in a plain table procedure call, in a page's OnModifyRecord, else the record before the call. What a table procedure called
//! on a record value (`Rec.Proc()`) runs with (`table_proc_xrec`, by `call_site` in `interp.rs`). Where a Validate was called
//! from (`ValidateSite`, `validate_at`). What a table procedure called on a record through other code sees while a call on the
//! same record runs (`running_call_xrec`, al-runner issue 8189).

use crate::records::Rec;
use crate::{Frame, Session};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, Value};
use al_ir::ObjKind;
use std::rc::Rc;

/// The `RecordState::unpinned_buffer` reason of the blank xRec of a table procedure called on a record through other code while a
/// trigger or procedure of the same record runs with an xRec that is not blank (`Session::running_call_xrec`): BC hands that
/// call's xRec on in the shape al-runner issue 8189 measured; here a read of it refuses.
pub(crate) const XREC_OF_RUNNING_CALL: &str = "xRec in a table procedure called through other code on a record whose trigger or procedure runs: real BC hands it the running call's xRec in the shape al-runner issue 8189 measured, this shape is not pinned";

/// The before-image a Validate hands its triggers as xRec: the field values, and the `RecordState::unpinned_buffer` reason
/// the xRec gets (`Some(XREC_OF_RUNNING_CALL)` when the values stand for a running call's xRec, `validate_before_image`).
pub(crate) type BeforeImage = (Vec<Value>, Option<&'static str>);

/// The xRec a table procedure called on a record value runs with ([`Session::table_proc_xrec`]).
pub(crate) enum ProcXrec {
    /// The caller's xRec handle, shared.
    Shared(Rec),
    /// The blank xRec of a plain call (`enter_record`).
    Plain,
    /// The blank xRec of a plain call, marked [`XREC_OF_RUNNING_CALL`].
    PlainUnpinned,
}

/// The call site of a Validate of a record field. Each caller names its own (`Session::validate_at`); a Validate that names none
/// has none. Only `AlRecordValidate` is a site the corpus measures for `Session::table_proc_xrec`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValidateSite {
    /// `Record.Validate(Field)` or `Record.Validate(Field, Value)` that AL code calls
    /// (record/validate/XRecTableProcCalls.al:160, :162, :177, :179).
    AlRecordValidate,
    /// `Record.Validate(Field, Value)` with the value written as a Variant.
    AlRecordValidateVariant,
    /// `FieldRef.Validate`.
    FieldRef,
    /// The write of a TestPage control (SetValue and the like). `new_line_of_trapped_static_card`: the control is on the line
    /// `New()` started on a Card that a static `Page.Run(Id, Rec)` opened and `TestPage.Trap()` caught
    /// (`Session::new_line_of_trapped_static_card`).
    TestPageControl { new_line_of_trapped_static_card: bool },
    /// The field element or attribute of an XmlPort import.
    XmlPortField,
    /// A part's link value stamped on its new line.
    PartLink,
    /// The filter value stamped on the blank Rec an action left.
    ActionStamp,
}

impl Session {
    /// The xRec handle a Validate hands its triggers and validate events: `xrec_handle` of the before-image's values, with the
    /// image's `unpinned_buffer` reason when it has one (so a read of a running call's xRec refuses).
    pub(crate) fn validate_xrec_handle(rec: &Rec, before: &BeforeImage) -> Rec {
        let xrec = Self::xrec_handle(rec, before.0.clone());
        if before.1.is_some() {
            xrec.borrow_mut().unpinned_buffer = before.1;
        }
        xrec
    }

    /// Validates a field of `rec` (`args`: the field's position, then the value if one is given) from the call site `site`.
    /// The site is handed to the Validate this call starts and to no other (`take_validate_site`).
    pub(crate) fn validate_at(&mut self, site: ValidateSite, rec: &Rec, args: &mut [Value]) -> Result<Value, Signal> {
        self.validate_site = Some((Rc::as_ptr(rec) as usize, site));
        let r = match site {
            ValidateSite::AlRecordValidate | ValidateSite::AlRecordValidateVariant => {
                self.record_op_from_al(RecordOp::Validate, rec, args)
            }
            _ => self.record_op(RecordOp::Validate, rec, args),
        };
        self.validate_site = None;
        r
    }

    /// [`Session::validate_at`] for the platform's own Validate of the field at `pos` with `value`.
    pub(crate) fn platform_validate(
        &mut self,
        site: ValidateSite,
        rec: &Rec,
        pos: usize,
        value: Value,
    ) -> Result<Value, Signal> {
        self.validate_at(site, rec, &mut [Value::Integer(pos as i32), value])
    }

    /// The record's handle and the call site of the Validate of `rec` that starts now: the site `validate_at` named for this
    /// record, `None` when the Validate did not come through it.
    pub(crate) fn take_validate_site(&mut self, rec: &Rec) -> (usize, Option<ValidateSite>) {
        let handle = Rc::as_ptr(rec) as usize;
        let site = self.validate_site.take().filter(|(h, _)| *h == handle);
        (handle, site.map(|(_, s)| s))
    }

    /// Runs `hooks` (the triggers and events of a Validate) with the Validate noted as running: its `entry` (`take_validate_site`)
    /// in `validate_sites`, and, for the outermost Validate of the record, its before-image `outermost` in `validating`.
    pub(crate) fn while_validating<T>(
        &mut self,
        entry: (usize, Option<ValidateSite>),
        outermost: Option<BeforeImage>,
        hooks: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let pushed = outermost.is_some();
        if let Some(before) = outermost {
            self.validating.push((entry.0, before));
        }
        self.validate_sites.push(entry);
        let r = hooks(self);
        self.validate_sites.pop();
        if pushed {
            self.validating.pop();
        }
        r
    }

    /// Whether the one Validate of the record at `handle` that runs now is a `Record.Validate` AL code called: the call site
    /// record/validate/XRecTableProcCalls.al measures (:160, :162, :177, :179, called from a test codeunit, no Validate of the
    /// record nested in another).
    fn sole_validate_from_al(&self, handle: usize) -> bool {
        let mut running = self.validate_sites.iter().filter(|(h, _)| *h == handle);
        matches!(
            (running.next(), running.next()),
            (Some((_, Some(ValidateSite::AlRecordValidate))), None)
        )
    }

    /// The caller's xRec that a procedure of table `object`, called on the record `rc` from the code running in frame `f`,
    /// shares: the caller's own xRec handle when the call is one of these, `None` otherwise (a plain call, whose xRec is the
    /// blank record `enter_record` makes: record/PWTableGlobals.al:196-260 XRecPlainProc_*).
    /// - Page code calls it on the page's Rec (`Rec.InitType()`): the page's xRec, as the page's triggers see it (Sales
    ///   Invoice Subform.OnNewRecord calls Rec.InitType, which copies xRec.Type; pages/xrec-in-table-procedure/
    ///   XRecTableProcedure.al `proc xRec=2/22/two`).
    /// - Measured, for the call site `ValidateSite::AlRecordValidate` alone (`sole_validate_from_al`): a field OnValidate of the
    ///   table, or a field OnValidate or OnAfterValidate of one of its table extensions, calls a procedure of the base table on
    ///   Rec (`Rec.Proc()`, `this.Proc()` or unqualified): the trigger's xRec, on a stored and on a fresh record; what the
    ///   procedure assigns to xRec shows in the trigger after the call (record/validate/XRecTableProcCalls.al:186-339, al-runner
    ///   issues 7785 and 7792; the unqualified call from the table: record/TestXRecContracts.al:52, :255).
    /// - Not measured, at every other call site of the Validate (a TestPage write, FieldRef.Validate, a Variant value, an XmlPort
    ///   field, a part's link, an action's stamp, a Validate nested in another of the record, a site that names none): the same
    ///   call from the table's own field OnValidate. Microsoft's Subscription Billing test Service Object
    ///   Test.CheckChangeServiceObjectSN needs the trigger's xRec, so it is taken on assumption
    ///   `xrec-table-proc-from-field-validate` (al-runner issue 7785), and only when that xRec holds other values than the blank
    ///   record a plain call gets (else the two read the same).
    ///
    /// Every other call from the table's or a table extension's own code keeps the blank xRec of a plain call, unprobed (OnInsert,
    /// OnModify, OnDelete, OnRename, OnBeforeValidate, another record variable, a table extension's own procedure, a table
    /// extension's trigger at a call site that is not measured). A call from other code: [`Session::running_call_xrec`].
    pub(crate) fn table_proc_xrec(&mut self, object: usize, rc: &Rec, f: &Frame) -> ProcXrec {
        match self.frame_xrec(object, rc, f) {
            Some(x) => ProcXrec::Shared(x),
            None => self.running_call_xrec(object, rc, f),
        }
    }

    /// The caller's xRec of [`Session::table_proc_xrec`] that the code running in frame `f` holds (the cases listed there).
    fn frame_xrec(&mut self, object: usize, rc: &Rec, f: &Frame) -> Option<Rec> {
        let from_al_validate = self.sole_validate_from_al(Rc::as_ptr(rc) as usize);
        let (caller_xrec, measured) = {
            let fi = f.instance.borrow();
            let objects = &self.engine.program.objects;
            let o = &objects[fi.object as usize];
            let (Value::Record(caller_rec), Value::Record(caller_xrec)) =
                (&fi.globals[o.rec_slot?], &fi.globals[o.xrec_slot?])
            else {
                return None;
            };
            if !Rc::ptr_eq(caller_rec, rc) {
                return None;
            }
            if o.kind == ObjKind::Page {
                return Some(caller_xrec.clone());
            }
            let trigger = objects[f.object].routines.get(f.routine).filter(|r| r.is_trigger)?;
            let field_on_validate = trigger.field.is_some() && trigger.name.eq_ignore_ascii_case("OnValidate");
            // The table's own field OnValidate.
            let own =
                o.kind == ObjKind::Table && f.object == object && fi.object as usize == object && field_on_validate;
            let owner = |x: usize| self.table_or_its_extension(x, object);
            let modify = trigger.container.as_ref().is_some_and(|(sec, _)| sec == "modify");
            let measured = from_al_validate
                && objects[object].kind == ObjKind::Table
                && owner(f.object)
                && owner(fi.object as usize)
                && (field_on_validate || (modify && trigger.name.eq_ignore_ascii_case("OnAfterValidate")));
            if !measured && !own {
                return None;
            }
            (caller_xrec.clone(), measured)
        };
        if measured {
            return Some(caller_xrec);
        }
        let blank = self.initial_fields(rc.borrow().table as usize);
        let differs = !Self::same_values(&caller_xrec.borrow().fields, &blank);
        (differs && assumptions::hit(Assumption::XrecTableProcFromFieldValidate)).then_some(caller_xrec)
    }

    /// Whether object `x` is table `object` itself or one of its table extensions.
    fn table_or_its_extension(&self, x: usize, object: usize) -> bool {
        let objects = &self.engine.program.objects;
        x == object || (objects[x].kind == ObjKind::TableExtension && objects[x].extends_id == objects[object].id)
    }

    /// A procedure of table `object` (or of one of its table extensions) called on `rc` by code that holds no xRec of it (frame
    /// `f`, no case of `frame_xrec`).
    /// - Called from the code of the record's table or of one of its table extensions (a trigger or procedure of any of them): a
    ///   plain call, as `table_proc_xrec` lists (record/PWTableGlobals.al:196-260; al-test-cli suite `xrec_proc_call_scope.rs`).
    /// - Called from other code while triggers or procedures on the same record run further up: the instances of the record
    ///   variable (ADR-021) of its table and of its table extensions that hold `rc` as Rec, and those calls' xRecs
    ///   (`running_xrecs`).
    /// - The shape al-runner issue 8189 measured (`through_codeunit_measured`): the xRec of the call running on the instance of
    ///   `object`, shared, on assumption `xrec-table-proc-through-codeunit` (the reporter's repro passes on real BC).
    /// - Any other shape, when one of those xRecs holds other values than the blank record a plain call gets: the blank xRec
    ///   marked [`XREC_OF_RUNNING_CALL`], so a read of it refuses.
    /// - Else, and when no call on the record runs: a plain call (record/PWTableGlobals.al:196-260 XRecPlainProc_*).
    fn running_call_xrec(&mut self, object: usize, rc: &Rec, f: &Frame) -> ProcXrec {
        let table = rc.borrow().table as usize;
        if self.table_or_its_extension(f.object, table) {
            return ProcXrec::Plain;
        }
        let blank = self.initial_fields(table);
        let not_blank = |x: &Rec| !Self::same_values(&x.borrow().fields, &blank);
        let running = self.running_xrecs(table, rc);
        if !running.iter().any(|(_, x)| not_blank(x)) {
            return ProcXrec::Plain;
        }
        let of_object = running.iter().find(|(o, _)| *o == object).map(|(_, x)| x.clone());
        if let Some(x) = of_object
            && not_blank(&x)
            && self.through_codeunit_measured(object, rc, f)
            && assumptions::hit(Assumption::XrecTableProcThroughCodeunit)
        {
            return ProcXrec::Shared(x);
        }
        ProcXrec::PlainUnpinned
    }

    /// The xRecs of the calls running on the record variable `rc` of table `table`: of each instance (ADR-021) of the table
    /// and of its table extensions whose Rec slot holds `rc` (`enter_record` fills it while the call runs, `leave_record`
    /// empties it), with the instance's object.
    fn running_xrecs(&self, table: usize, rc: &Rec) -> Vec<(usize, Rec)> {
        let instances = rc.borrow().globals.instances();
        let objects = &self.engine.program.objects;
        let mut running = Vec::new();
        for (object, inst) in instances {
            let object = object as usize;
            if !self.table_or_its_extension(object, table) {
                continue;
            }
            let o = &objects[object];
            let (Some(rs), Some(xs)) = (o.rec_slot, o.xrec_slot) else {
                continue;
            };
            let i = inst.borrow();
            if let (Value::Record(r), Value::Record(x)) = (&i.globals[rs], &i.globals[xs])
                && Rc::ptr_eq(r, rc)
            {
                running.push((object, x.clone()));
            }
        }
        running
    }

    /// Whether the call of a procedure of table `object` on `rc` from frame `f` is the shape al-runner issue 8189 measured
    /// (ways 3 and 4 of the reporter's repro), every fact by equality:
    /// - `object` is a table (not an extension) and `f` runs a procedure of a codeunit, reached directly or through an interface,
    ///   that has `rc` as a `var` parameter;
    /// - the three innermost frames are, from the inside out: that codeunit procedure, a procedure of table `object` that is no
    ///   trigger, and the field OnValidate trigger of table `object`;
    /// - the Validate is the one `Record.Validate` AL code called on the record (`sole_validate_from_al`);
    /// - the record is stored and its buffer came from its own Insert or from a Get of the row the open transaction inserted
    ///   and wrote nothing to since (`measured_buffer`).
    fn through_codeunit_measured(&self, object: usize, rc: &Rec, f: &Frame) -> bool {
        let objects = &self.engine.program.objects;
        let n = self.frames.len();
        if n < 3 || self.call_stack.len() != n {
            return false;
        }
        let at = |k: usize| (self.frames[n - k], &self.call_stack[n - k]);
        let ((cu, cu_frame), (proc_o, proc_frame), (trig_o, trig_frame)) = (at(1), at(2), at(3));
        let callee = &objects[cu].routines[cu_frame.routine];
        let proc = &objects[proc_o].routines[proc_frame.routine];
        let trigger = &objects[trig_o].routines[trig_frame.routine];
        let var_param = (0..callee.params).any(|i| {
            callee.by_var.get(i) == Some(&true)
                && matches!(f.locals.get(i), Some(Value::Record(p)) if Rc::ptr_eq(p, rc))
        });
        objects[object].kind == ObjKind::Table
            && cu == f.object
            && cu_frame.routine == f.routine
            && objects[cu].kind == ObjKind::Codeunit
            && !(cu_frame.event || proc_frame.event || trig_frame.event)
            && var_param
            && proc_o == object
            && !proc.is_trigger
            && trig_o == object
            && trigger.is_trigger
            && trigger.field.is_some()
            && trigger.name.eq_ignore_ascii_case("OnValidate")
            && self.sole_validate_from_al(Rc::as_ptr(rc) as usize)
            && self.measured_buffer(&rc.borrow())
    }

    /// The two buffers al-runner issue 8189 measured (the reporter's `PrepareAfterInsert` and `PrepareAfterGet`): a record that is
    /// not temporary, whose buffer came from its own `Insert` (no Modify, Rename or read after it), or from a `Get` of the row
    /// that its own transaction inserted and wrote nothing to since (`inserted_unwritten_in_tx`). A Find, a GetBySystemId, a
    /// Get of a committed row, a Get of a row the open transaction modified or a TransferFields refuse.
    fn measured_buffer(&self, r: &RecordState) -> bool {
        let Some(o) = r.row_origin else {
            return false;
        };
        let own_insert = !o.read_in_write_tx && !o.by_get;
        let get_of_own_insert = o.read_in_write_tx && o.by_get && self.inserted_unwritten_in_tx(r);
        !r.temporary
            && o.own_write
            && !o.own_modified
            && !o.lock_only
            && !o.from_find
            && (own_insert || get_of_own_insert)
    }

    /// Whether the open transaction inserted the row of the buffer `r` and wrote nothing to it since: every store write since
    /// the last Commit or rollback is noted (`writes_all_noted`), and the last of them that is to the row's key or to its whole
    /// table (a Rename, ModifyAll, DeleteAll, Truncate or other write whose rows are not told apart) is an Insert of the row.
    fn inserted_unwritten_in_tx(&self, r: &RecordState) -> bool {
        use crate::state::TxWriteKind;
        let Some(t) = self.engine.tables.get(r.table as usize).and_then(|t| t.as_deref()) else {
            return false;
        };
        let pk = t.primary_key(&r.fields);
        let last = self.tx.writes.iter().rev().find(|w| {
            w.table == t.object
                && w.company == r.company
                && (crate::records::same_key(&w.pk, &pk)
                    || !matches!(w.kind, TxWriteKind::Insert | TxWriteKind::Modify | TxWriteKind::Delete))
        });
        self.writes_all_noted()
            && last.is_some_and(|w| w.kind == TxWriteKind::Insert && crate::records::same_key(&w.pk, &pk))
    }

    /// The before-image of the innermost running Validate of the record at `handle`, when the Validate about to start is
    /// nested in it by the table's own code: it then sees that before-image as xRec (record/TestXRecContracts.al:133, a
    /// Validate that the table's OnValidate trigger nests). `None` when no Validate of the record runs, or when other code
    /// nests it (an event subscriber of the validate events, a codeunit that OnValidate calls): that Validate sees the
    /// record as it is when it starts, as an unnested one does (events/table-events/ValidateInValidateSubscriber.al:188-235
    /// AfterValidateSubscriber_* `B(rec=BLUE,x=BLUE)`, :236
    /// OnValidate_CodeunitValidatesSameFieldAgain `B(rec=INNER,x=BLUE)`; al-runner issue 6924 item 3.9).
    /// A part's link stamp is the platform's own Validate: it sees the blank line it pushed whatever code runs
    /// (pages/parts/QjPartPropagation.al:3954 QT_ValPartSplit_Empty_New_ThenType `T.Doc.V(H1 x= ln=0)`).
    pub(crate) fn outer_validate_before(&self, handle: usize, table_object: usize) -> Option<BeforeImage> {
        let (_, before) = self.validating.iter().rev().find(|(h, _)| *h == handle)?;
        let own = self
            .frames
            .last()
            .is_some_and(|&o| self.table_or_its_extension(o, table_object));
        (own || self.stamp_validating.contains(&handle)).then(|| before.clone())
    }

    /// The before-image a Validate of `rec` (at `handle`) hands its triggers as xRec, `outer` being `outer_validate_before` and
    /// `with_value` whether the Validate hands a new value.
    /// In a table procedure that no operation of the record called (a plain call, whose xRec is a blank record:
    /// `enter_record`), a Validate of the procedure's own Rec passes that xRec, not the record as it is
    /// (types/operators-expressions/UpThisField.al:175 UP_This_ValidateTestFieldCompound: `this.Validate(Qty, 5)` after
    /// Qty := 3 sees old=0, and `this.Qty := 7; this.Validate(this.Qty)` sees old=0). In the page's OnModifyRecord a Validate
    /// of the page's Rec passes the page's xRec or is refused (`pages/modify_record_xrec.rs`). Called from other code, the xRec is the record
    /// before the call (record/TestXRecContracts.al:190, record/TestTableEventSequenceContracts.al).
    /// The blank xRec of a call [`Session::running_call_xrec`] marked stands for the running call's xRec: the before-image keeps
    /// the mark, so the triggers' and validate events' read of xRec refuses, and a Validate nested in them by the table's code
    /// gets it again (`outer`).
    pub(crate) fn validate_before_image(
        &self,
        handle: usize,
        outer: Option<BeforeImage>,
        rec: &Rec,
        with_value: bool,
    ) -> Result<BeforeImage, Signal> {
        if let Some(b) = outer {
            return Ok(b);
        }
        let depth = self.call_stack.len();
        if let Some(x) = self
            .plain_proc_xrec
            .last()
            .filter(|(h, d, _)| *h == handle && *d == depth)
        {
            let x = x.2.borrow();
            let mark = (x.unpinned_buffer == Some(XREC_OF_RUNNING_CALL)).then_some(XREC_OF_RUNNING_CALL);
            return Ok((x.fields.clone(), mark));
        }
        let before = rec.borrow().fields.clone();
        let image = self
            .modify_record_validate_xrec(handle, &before, with_value)?
            .unwrap_or(before);
        Ok((image, None))
    }
}
