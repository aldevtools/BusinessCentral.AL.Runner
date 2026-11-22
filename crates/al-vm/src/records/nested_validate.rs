//! The xRec of table code. What a Validate hands its triggers (`RecordOp::Validate`): nested in a Validate of the same record,
//! in a plain table procedure call, in a page's OnModifyRecord, else the record before the call. What a table procedure called
//! on a record value (`Rec.Proc()`) runs with (`table_proc_xrec`, by `call_site` in `interp.rs`). Where a Validate was called
//! from (`ValidateSite`, `validate_at`). What a table procedure called on a record through other code sees while a call on the
//! same record runs: `running_call_xrec.rs` (record/validate/XRecTableProcThroughCodeunit.al, XRecTableProcThroughSubscriber.al).

use crate::records::Rec;
use crate::{Frame, Session};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;
use al_ir::ObjKind;
use std::rc::Rc;

/// The `RecordState::unpinned_buffer` reason of the blank xRec of a table procedure called on a record through other code while a
/// trigger or procedure of the same record runs with an xRec that is not blank (`Session::running_call_xrec`): BC hands that
/// call's xRec on in the shapes record/validate/XRecTableProcThroughCodeunit.al and XRecTableProcThroughSubscriber.al pin;
/// here a read of it refuses.
pub(crate) const XREC_OF_RUNNING_CALL: &str = "xRec in a table procedure called through other code on a record whose trigger or procedure runs: real BC hands it the running call's xRec in the shapes XRecTableProcThroughCodeunit.al and XRecTableProcThroughSubscriber.al pin, this shape is not run";

/// The before-image a Validate hands its triggers as xRec: the field values, and the `RecordState::unpinned_buffer` reason
/// the xRec gets (`Some(XREC_OF_RUNNING_CALL)` when the values stand for a running call's xRec, `validate_before_image`).
pub(crate) type BeforeImage = (Vec<Value>, Option<&'static str>);

/// The xRec a table procedure called on a record value runs with ([`Session::table_proc_xrec`]).
pub(crate) enum ProcXrec {
    /// The caller's xRec handle, shared.
    Shared(Rec),
    /// The running trigger's xRec handle, shared for reading: a Validate the table's own code nested runs, whose xRec here is a
    /// copy of the outer Validate's, so an assignment to it or a filter on it refuses when the call returns
    /// (`running_call_xrec.rs` `XREC_CHANGED_UNDER_NESTED_VALIDATE`).
    SharedReadOnly(Rec),
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

/// A Validate that runs (`Session::validate_sites`).
pub(crate) struct ValidateRun {
    /// The record's handle.
    pub(crate) handle: usize,
    /// The call site (`Session::take_validate_site`).
    pub(crate) site: Option<ValidateSite>,
    /// The depth of `call_stack` its trigger runs at (the frame's index from the outside in).
    pub(crate) depth: usize,
    /// The xRec it hands its trigger and events.
    pub(crate) xrec: Rec,
    /// The record's fields when its triggers start, the new value in.
    pub(crate) entered: Vec<Value>,
    /// A record method that writes the record or reads it again ran on it since (`Session::note_record_op_under_validate`).
    pub(crate) touched: bool,
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

    /// Runs `hooks` (the triggers and events of a Validate of `rec`) with the Validate noted as running: its `entry`
    /// (`take_validate_site`) in `validate_sites` with the depth of the call stack its trigger runs at, the xRec `xrec` it hands
    /// its trigger and the record's fields, and, for the outermost Validate of the record, its before-image `outermost` in
    /// `validating`.
    pub(crate) fn while_validating<T>(
        &mut self,
        entry: (usize, Option<ValidateSite>),
        rec: &Rec,
        xrec: &Rec,
        outermost: Option<BeforeImage>,
        hooks: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let pushed = outermost.is_some();
        if let Some(before) = outermost {
            self.validating.push((entry.0, before));
        }
        self.validate_sites.push(ValidateRun {
            handle: entry.0,
            site: entry.1,
            depth: self.call_stack.len(),
            xrec: xrec.clone(),
            entered: rec.borrow().fields.clone(),
            touched: false,
        });
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
    pub(super) fn sole_validate_from_al(&self, handle: usize) -> bool {
        self.sole_validate_from_al_depth(handle).is_some()
    }

    /// [`Session::sole_validate_from_al`], with the Validate's trigger running in frame `depth` (from the outside in).
    pub(super) fn sole_validate_from_al_at(&self, handle: usize, depth: usize) -> bool {
        self.sole_validate_from_al_depth(handle) == Some(depth)
    }

    /// The call site of the one Validate of the record at `handle` that runs now; `None` when none runs, when one is nested
    /// in another, or when the site is not named.
    pub(super) fn sole_validate_site(&self, handle: usize) -> Option<&ValidateSite> {
        self.sole_validate_run(handle)?.site.as_ref()
    }

    /// The one Validate of the record at `handle` that runs now; `None` when none runs or when one is nested in another.
    pub(super) fn sole_validate_run(&self, handle: usize) -> Option<&ValidateRun> {
        let mut running = self.validate_sites.iter().filter(|v| v.handle == handle);
        match (running.next(), running.next()) {
            (Some(run), None) => Some(run),
            _ => None,
        }
    }

    /// Notes that record method `method` runs on `rec`: when it writes the record (its buffer or its row) or reads it again,
    /// each running Validate of it counts as touched (`ValidateRun::touched`). A method that only reads the buffer or the view
    /// (TestField, GetFilter, FieldCaption) or sets the view (SetRange, SetCurrentKey) does not; one that changes a field
    /// value (CalcFields) shows in the fields (`ValidateRun::entered`).
    pub(crate) fn note_record_op_under_validate(&mut self, method: RecordOp, rec: &Rec) {
        use RecordOp::*;
        let writes_or_reads = matches!(
            method,
            Clear
                | Copy
                | ChangeCompany
                | Delete
                | DeleteAll
                | Find
                | FindFirst
                | FindLast
                | FindSet
                | Get
                | GetBySystemId
                | Init
                | Insert
                | LoadFields
                | Modify
                | ModifyAll
                | Next
                | Rename
                | SetPosition
                | TransferFields
                | Truncate
                | Validate
        );
        if !writes_or_reads {
            return;
        }
        let handle = Rc::as_ptr(rec) as usize;
        for v in self.validate_sites.iter_mut().filter(|v| v.handle == handle) {
            v.touched = true;
        }
    }

    /// The depth of the call stack the trigger of the one Validate of the record at `handle` runs at, when that Validate is a
    /// `Record.Validate` AL code called and no other Validate of the record runs.
    fn sole_validate_from_al_depth(&self, handle: usize) -> Option<usize> {
        match self.sole_validate_run(handle)? {
            &ValidateRun {
                site: Some(ValidateSite::AlRecordValidate),
                depth,
                ..
            } => Some(depth),
            _ => None,
        }
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
    /// extension's trigger at a call site that is not measured). A subscriber of a validate event calling it on Rec:
    /// [`Session::validate_event_xrec`]. A call from other code: [`Session::running_call_xrec`].
    pub(crate) fn table_proc_xrec(&mut self, object: usize, rc: &Rec, f: &Frame) -> ProcXrec {
        if let Some(x) = self.frame_xrec(object, rc, f) {
            return ProcXrec::Shared(x);
        }
        if let Some(x) = self.validate_event_xrec(object, rc, f) {
            return ProcXrec::Shared(x);
        }
        self.running_call_xrec(object, rc, f)
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
    pub(super) fn table_or_its_extension(&self, x: usize, object: usize) -> bool {
        let objects = &self.engine.program.objects;
        x == object || (objects[x].kind == ObjKind::TableExtension && objects[x].extends_id == objects[object].id)
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
