use super::*;

/// The start of the refusal of an index with fewer positions than the array has dimensions.
const ROW_REFUSAL: &str = "a row of a ";

impl Session {
    /// An index of a place: the compiler evaluated the expression into a register (or it is a constant).
    fn index_value(e: &ir::Expr, f: &Frame) -> Result<Value, Signal> {
        match e {
            ir::Expr::Load(ir::Place::Local(i)) => Ok(f.locals[*i].clone()),
            ir::Expr::Const(c) => Ok(constant(c)),
            _ => Err(Signal::Unsupported(
                "an index expression the compiler did not evaluate ahead".into(),
            )),
        }
    }

    pub(crate) fn restore_pos(&mut self, pos: Option<u32>) {
        if let (Some(p), Some(top)) = (pos, self.call_stack.last_mut()) {
            top.pos = p;
        }
    }

    /// A builtin the VM runs itself (`Codeunit.Run`, `Xmlport.*`, `Report.*`, `FilterPageBuilder.*`, `RequestPage.Update`,
    /// `Database.IsInWriteTransaction`) on evaluated operands. Used by the bytecode.
    /// arm is the same chain kept inline.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn vm_class_call(
        &mut self,
        id: al_builtins::builtin_id::BuiltinId,
        class: al_builtins::builtin_id::Class,
        recv: Option<Value>,
        values: Vec<Value>,
        ret_used: bool,
        f: &mut Frame,
    ) -> Result<Value, Signal> {
        use al_builtins::builtin_id::{Class, ids};
        if id == ids::DATABASE_ISINWRITETRANSACTION_1 {
            return Ok(Value::Boolean(self.in_write_transaction()));
        }
        if class == Class::Xmlport {
            return self.xmlport_builtin(id, recv, &values, ret_used);
        }
        if id == ids::REQUESTPAGE_UPDATE_1 {
            return self.request_page_update(&values);
        }
        if id == ids::REQUESTPAGE_CAPTION_1 {
            return self.request_page_caption(&values, &f.instance);
        }
        if class == Class::Report {
            self.ret_used = ret_used;
            return self.report_builtin(id, recv, &values, f.instance.clone());
        }
        if class == Class::FilterPageBuilder {
            return self.filter_page_builder_op(id, recv, &values, ret_used);
        }
        self.codeunit_run(id, recv, &values, ret_used)
    }

    /// Runs the native of a builtin on evaluated operands and writes the by-reference ones back.
    #[allow(clippy::too_many_arguments)]
    #[inline(never)]
    pub(crate) fn run_native(
        &mut self,
        native: al_builtins::native::NativeFn,
        class: al_builtins::builtin_id::Class,
        receiver: Option<&ir::Arg>,
        args: &[ir::Arg],
        mut recv: Option<Value>,
        mut values: Vec<Value>,
        ret_used: bool,
        f: &mut Frame,
    ) -> Result<Value, Signal> {
        self.ret_used = ret_used;
        // A background task this call enqueued runs once the call wrote its arguments back; one that a call further out
        // enqueued waits for that call.
        let drain_pending = self.pages.bg_drain_after_call.is_some();
        let ret = native(self, recv.as_mut(), &mut values)?;
        if let (Some(ir::Arg::Ref(p)), Some(v)) = (receiver, recv) {
            self.write(p, v, f)?;
        }
        for (a, v) in args.iter().zip(values.drain(..)) {
            if let ir::Arg::Ref(p) = a {
                let v = if class == al_builtins::builtin_id::Class::InStreamRead {
                    self.fit_stream_read(p, v, f)?
                } else {
                    self.fit_ref_number(p, v, f)?
                };
                self.write(p, v, f)?;
            }
        }
        self.return_vec(values);
        if !drain_pending && self.pages.bg_drain_after_call.is_some() {
            self.bg_drain_after_call()?;
        }
        Ok(ret)
    }

    /// A number a builtin stored in a by-reference argument takes the declared number type of the variable, as an assignment
    /// does (`List of [BigInteger].Get(1, I)` leaves an Integer in `I`; an overflow raises the error of the conversion).
    #[inline]
    fn fit_ref_number(&self, p: &ir::Place, v: Value, f: &Frame) -> Result<Value, Signal> {
        // A text a builtin stores in a bounded Text or Code variable is checked against its length, as an assignment is
        // (json/JsonWriteTo.al:WriteToTextVariants `writeto_text10_overflow`).
        if let Value::Text(t) = &v {
            if let Some(ty @ (RtType::Text(Some(_)) | RtType::Code(Some(_)))) = self.place_ty(p, f) {
                if let RtType::Text(Some(m)) | RtType::Code(Some(m)) = ty {
                    if t.chars().count() > *m as usize {
                        return al_builtins::ops::convert(v, ty);
                    }
                }
            }
            return Ok(v);
        }
        if !matches!(v, Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_)) {
            return Ok(v);
        }
        let Some(ty) = self.place_ty(p, f) else {
            return Ok(v);
        };
        let same = matches!(
            (&v, ty),
            (Value::Integer(_), RtType::Integer)
                | (Value::BigInteger(_), RtType::BigInteger)
                | (Value::Decimal(_), RtType::Decimal)
        );
        if same || !matches!(ty, RtType::Integer | RtType::BigInteger | RtType::Decimal) {
            return Ok(v);
        }
        al_builtins::ops::convert(v, ty)
    }

    /// What `InStream.Read` stores in a Text or Code variable: a Code takes the characters
    /// upper-cased and cut to its length without an error (streams/ProbeJsonAndInStreamBytes.al:
    /// InStreamRead_Text_ReturnValueIsLength reads `toolongvalueforcode` into a Code[10] and gets
    /// `TOOLONGVAL`). A Text that is too short for the characters is not pinned.
    pub(crate) fn fit_stream_read(&mut self, p: &ir::Place, v: Value, f: &mut Frame) -> Result<Value, Signal> {
        match (&v, self.place_type(p, f)) {
            (Value::Code(t), RtType::Code(max)) => {
                let up = t.to_uppercase();
                Ok(Value::Code(match max {
                    Some(m) => up.chars().take(m as usize).collect(),
                    None => up.into(),
                }))
            }
            (Value::Text(t), RtType::Text(Some(m))) if t.chars().count() > m as usize => Err(Signal::Unsupported(
                "InStream.Read of more characters than the Text variable holds (not pinned)".into(),
            )),
            _ => Ok(v),
        }
    }

    /// `Rec.CurrFieldNo := n`: the field number a TestPage write is validating on this record.
    pub(crate) fn set_curr_field_no(&mut self, rec: Value, n: Value) -> Result<Value, Signal> {
        let Value::Record(rec) = rec else {
            return Err(Signal::Unsupported("CurrFieldNo of something that is no record".into()));
        };
        let Value::Integer(n) = n else {
            return Err(Signal::Unsupported("CurrFieldNo := a value that is no Integer".into()));
        };
        match self
            .pages
            .curr_field_no
            .iter_mut()
            .rev()
            .find(|(w, _)| std::ptr::eq(w.as_ptr(), Rc::as_ptr(&rec)))
        {
            Some(entry) => entry.1 = n,
            None => {
                // Entries of dropped records are dead; forget them before adding one.
                self.pages.curr_field_no.retain(|(w, _)| w.strong_count() > 0);
                self.pages.curr_field_no.push((Rc::downgrade(&rec), n));
            }
        }
        Ok(Value::Null)
    }

    /// `Rec.CurrFieldNo`.
    pub(crate) fn curr_field_no(&self, rec: Value) -> Result<Value, Signal> {
        let Value::Record(rec) = rec else {
            return Err(Signal::Unsupported("CurrFieldNo of something that is no record".into()));
        };
        Ok(Value::Integer(self.curr_field_of(&rec)))
    }

    /// `ClearAll()` resets every global variable of the running object to its default;
    /// locals are untouched (docs/probes-needed.md "ClearAll"). In a codeunit with a TableNo,
    /// a table and a page the record stays as it is and so does the page itself
    /// (session/TestClearAllTransactionTypeContracts.al:224-285, BC 27.5). Any other object
    /// kind is refused.
    pub(crate) fn clear_all(&mut self, f: &Frame) -> Result<Value, Signal> {
        let object = f.instance.borrow().object as usize;
        let o = &self.engine.program.objects[object];
        let keep: Vec<usize> = match o.kind {
            al_ir::ObjKind::Codeunit | al_ir::ObjKind::Table => o.rec_slot.into_iter().collect(),
            al_ir::ObjKind::Page => o
                .rec_slot
                .into_iter()
                .chain(o.page.as_ref().map(|p| p.curr_page_slot))
                .collect(),
            // A report resets every global: the data items' records and their filters, the request page's variables,
            // plain codeunit variables; a SingleInstance codeunit keeps its state. Called in OnInitReport, OnPreReport,
            // OnPreDataItem, OnAfterGetRecord, OnPostReport or a procedure they call (corpus
            // report/lt-clearall-report/ClearAllReport{Init,Pre,Post}Report.al and ...Procedure.al,
            // errors/lt-ms-refusals-128/P128ClearAllReport.al, errors/lt-ms-refusals-135/P135ClearAllReport.al).
            al_ir::ObjKind::Report => Vec::new(),
            _ => {
                return Err(Signal::Unsupported(format!("ClearAll in a {} (not pinned)", o.kind)));
            }
        };
        self.alias_any(f)?;
        let tys: Vec<RtType> = o.globals.iter().map(|g| g.ty.clone()).collect();
        let fresh: Vec<Value> = tys.iter().map(|t| self.default_value(t)).collect();
        let mut inst = f.instance.borrow_mut();
        for (i, v) in fresh.into_iter().enumerate() {
            if !keep.contains(&i) {
                // A global temporary record keeps its rows, as Clear leaves them
                // (types/ProbeTypeFormsAndFilters.al:Clear_TemporaryRecordWithRows_ThenCount); ClearAll does the same: real BC 29
                // keeps both rows of a codeunit's global temporary record and drops its filter, called directly and through
                // Codeunit.Run (record/lt-priority-assumptions/PriorityAssumptions.al:247-281, ClearAll_TemporaryGlobalRecord_*).
                // Pinned for a codeunit's global; the same for a report, a page or a record shared through Copy is not.
                if let (Value::Record(old), Value::Record(new)) = (&inst.globals[i], &v) {
                    let id = old.borrow().temp_id.clone();
                    if id.is_some() && old.borrow().temporary {
                        new.borrow_mut().temp_id = id;
                    }
                }
                inst.globals[i] = v;
            }
        }
        // ClearAll in a page resets the globals of its page extensions too: a flag an extension set in its OnOpenPage reads
        // false in the extension's OnAfterGetRecord once the base page's OnAfterGetRecord called ClearAll, for a base page of the
        // same app and of another (corpus pages/lt-ce-page-run/CePageExt.al:27-41 ClearAll_InOwnBasePage_ExtensionGlobal, :43-69
        // ClearAll_InBaseAppPage_ExtensionGlobal). The extension's Rec, xRec and CurrPage stay, as the page's own do.
        let engine = self.engine.clone();
        let o = &engine.program.objects[object];
        if o.kind == al_ir::ObjKind::Page {
            let curr = o.page.as_ref().map(|p| p.curr_page_slot);
            let st = curr.and_then(|c| match &inst.globals[c] {
                Value::TestPage(st) => Some(st.clone()),
                _ => None,
            });
            drop(inst);
            if let Some(st) = st {
                let exts = st.borrow().ext_instances.clone();
                for (e, ext) in exts {
                    let eo = &engine.program.objects[e];
                    let keep: Vec<usize> = eo
                        .rec_slot
                        .into_iter()
                        .chain(eo.xrec_slot)
                        .chain(eo.globals.iter().position(|g| g.name == "CurrPage"))
                        .collect();
                    let tys: Vec<RtType> = eo.globals.iter().map(|g| g.ty.clone()).collect();
                    let fresh: Vec<Value> = tys.iter().map(|t| self.default_value(t)).collect();
                    let mut ei = ext.borrow_mut();
                    for (i, v) in fresh.into_iter().enumerate() {
                        if keep.contains(&i) {
                            continue;
                        }
                        // A temporary record of an extension is not pinned (see above): refused.
                        if matches!(&ei.globals[i], Value::Record(r) if r.borrow().temporary) {
                            return Err(Signal::Unsupported(
                                "ClearAll in a page whose extension has a global temporary record (not pinned)".into(),
                            ));
                        }
                        ei.globals[i] = v;
                    }
                }
            }
        }
        Ok(Value::Null)
    }

    /// `ApplicationArea()` reads the session's areas, `ApplicationArea(Text)` sets them
    /// (corpus pageapplicationarea/TestPageApplicationAreaControlRemoval.al).
    pub(crate) fn application_area(&mut self, set: Option<Value>) -> Result<Value, Signal> {
        match set {
            None => Ok(Value::Text(self.application_areas.clone().into())),
            Some(Value::Text(t) | Value::Code(t)) => {
                // Tags are `#` and letters or digits, separated by commas (platform/lt-platform-members/PlTSession.al:
                // ApplicationArea_SetTwiceAndInvalid); whether a well-formed tag exists is not pinned.
                let well_formed = t.split(',').all(|tag| {
                    let tag = tag.trim();
                    tag.strip_prefix('#')
                        .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric()))
                });
                if !t.is_empty() && !well_formed {
                    return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                        message: format!(
                            "'{t}' is not a valid value for the APPLICATIONAREA function parameter.\\\\    The value must be a text string that contains one or more application area tags.\\    A tag must start with # and can be followed by any combination of letters (Aa-Zz) and numbers (0-9).\\    Use a comma to separate multiple tags. Example: #Basic,#Suite"
                        ),
                        code: "Argument".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                self.application_areas = t.to_string();
                Ok(Value::Null)
            }
            Some(other) => Err(Signal::Unsupported(format!("ApplicationArea({})", other.describe()))),
        }
    }

    /// What `CurrFieldNo` reads on `rec`: the field number a TestPage write is validating on this
    /// record, 0 anywhere else (handlers/TestPageCurrFieldNo_Tests.al:36-123).
    pub(crate) fn curr_field_of(&self, rec: &Rc<RefCell<RecordState>>) -> i32 {
        self.pages
            .curr_field_no
            .iter()
            .rev()
            .find(|(w, _)| std::ptr::eq(w.as_ptr(), Rc::as_ptr(rec)))
            .map_or(0, |(_, n)| *n)
    }

    /// `Text[i]`: the UTF-16 code unit at position `i`.
    #[inline]
    pub(crate) fn char_at(t: &al_builtins::text::Txt, i: i64) -> Result<Value, Signal> {
        // types/TestProbeScalarContracts.al:TextChar_AssignByIndex: position 0 and below raise, the position after
        // the last character reads as the character 0; further positions are not pinned.
        if i < 1 {
            return al_builtins::native::raise(
                format!(
                    "The value of INDEXING parameter 1 is outside of the permitted range. The current value is: {i}. The permitted range is: from 1 to 2,147,483,647."
                ),
                "OutsidePermittedRange",
            );
        }
        if t.is_ascii() {
            // No copy of a long text for each character read.
            return match t.as_bytes().get(i as usize - 1) {
                Some(u) => Ok(Value::Char(u16::from(*u))),
                None if i as usize == t.len() + 1 => Ok(Value::Char(0)),
                None => Err(Signal::Unsupported(
                    "a text read more than one position past its length (not pinned)".into(),
                )),
            };
        }
        let units: Vec<u16> = t.encode_utf16().collect();
        match units.get(i as usize - 1) {
            Some(u) => Ok(Value::Char(*u)),
            None if i as usize == units.len() + 1 => Ok(Value::Char(0)),
            None => Err(Signal::Unsupported(
                "a text read more than one position past its length (not pinned)".into(),
            )),
        }
    }

    /// The items a `foreach` runs over (a List, a JSON array or an XPath result).
    pub(crate) fn foreach_items(coll: Value) -> Result<Vec<Value>, Signal> {
        Ok(match coll {
            Value::List(l) => l.borrow().to_vec(),
            // A Text or Code visits its UTF-16 code units, one Char each, in order; the text is read once, so an assignment
            // inside the loop changes nothing about the visits (text/lt-foreach-text/ForeachText.al:Foreach_Text_Char_Order,
            // Foreach_Text_Char_Sources, Foreach_Text_ModifyDuringLoop; a Text[1] variable: Foreach_Text_Text1_Variable).
            Value::Text(t) | Value::Code(t) if !t.has_marker() => t.encode_utf16().map(Value::Char).collect(),
            // The nodes an XPath query found (corpus handlers/TestReportFlowFieldColumnAutoCalc.al).
            other => match al_builtins::json::array_items(&other).or_else(|| al_builtins::xml::node_list_items(&other))
            {
                Some(items) => items,
                None => return Err(Signal::Unsupported(format!("foreach over {}", other.describe()))),
            },
        })
    }

    /// A binary operator on two evaluated operands.
    #[inline(always)]
    pub(crate) fn binary_op(op: ir::BinOp, l: &Value, r: &Value) -> Result<Value, Signal> {
        use ir::BinOp as B;
        Ok(match op {
            B::Add => ops::arith(ops::Arith::Add, l, r)?,
            B::Sub => ops::arith(ops::Arith::Sub, l, r)?,
            B::Mul => ops::arith(ops::Arith::Mul, l, r)?,
            B::Div => ops::arith(ops::Arith::Div, l, r)?,
            B::IntDiv => ops::arith(ops::Arith::IntDiv, l, r)?,
            B::Mod => ops::arith(ops::Arith::Mod, l, r)?,
            B::And => Value::Boolean(ops::truthy(l)? & ops::truthy(r)?),
            B::Or => Value::Boolean(ops::truthy(l)? | ops::truthy(r)?),
            B::Xor => Value::Boolean(ops::truthy(l)? ^ ops::truthy(r)?),
            B::Eq => Value::Boolean(ops::compare_operator(l, r, false)? == Ordering::Equal),
            B::Ne => Value::Boolean(ops::compare_operator(l, r, false)? != Ordering::Equal),
            B::Lt => Value::Boolean(ops::compare_operator(l, r, true)? == Ordering::Less),
            B::Le => Value::Boolean(ops::compare_operator(l, r, true)? != Ordering::Greater),
            B::Gt => Value::Boolean(ops::compare_operator(l, r, true)? == Ordering::Greater),
            B::Ge => Value::Boolean(ops::compare_operator(l, r, true)? != Ordering::Less),
            B::DotNetEq => Value::Boolean(dotnet::dotnet_equal(l, r)?),
            B::DotNetNe => Value::Boolean(!dotnet::dotnet_equal(l, r)?),
            B::DotNetLt => Value::Boolean(dotnet::dotnet_order(l, r)? == Ordering::Less),
            B::DotNetLe => Value::Boolean(dotnet::dotnet_order(l, r)? != Ordering::Greater),
            B::DotNetGt => Value::Boolean(dotnet::dotnet_order(l, r)? == Ordering::Greater),
            B::DotNetGe => Value::Boolean(dotnet::dotnet_order(l, r)? != Ordering::Less),
        })
    }

    /// A unary operator on an evaluated operand.
    #[inline(always)]
    pub(crate) fn unary_op(op: ir::UnOp, v: Value) -> Result<Value, Signal> {
        match op {
            ir::UnOp::Not => Ok(Value::Boolean(!ops::truthy(&v)?)),
            ir::UnOp::Plus => Ok(v),
            ir::UnOp::Neg => ops::arith(ops::Arith::Sub, &Value::Integer(0), &v).map(|r| match (&v, r) {
                (Value::Decimal(_), r) => r,
                (_, r) => r,
            }),
        }
    }

    /// The statement at position `p` begins: the call stack, the profiler and coverage see it.
    #[inline(always)]
    pub(crate) fn at_pos(&mut self, p: u32) {
        if let Some(top) = self.call_stack.last_mut() {
            top.pos = p;
        }
        if profile::ON.load(std::sync::atomic::Ordering::Relaxed) {
            self.profile_sample();
        }
        if self.coverage.code_coverage {
            self.coverage_hit(p);
        }
        if let Some((counts, touched)) = &mut self.coverage.coverage_counts {
            let i = p as usize;
            if counts.len() <= i {
                counts.resize(self.engine.program.positions.len().max(i + 1), 0);
            }
            if counts[i] == 0 {
                touched.push(p);
            }
            counts[i] += 1;
        }
    }

    /// An Option, Enum or Boolean bound of a `for` loop as the Integer it stands for.
    #[inline(always)]
    pub(crate) fn for_bound(v: Value) -> Value {
        match v {
            Value::Option(o, _) | Value::Enum(_, o) => Value::Integer(o),
            Value::Boolean(b) => Value::Integer(i32::from(b)),
            v => v,
        }
    }

    /// The first and last counter value of a `for` loop from its evaluated bounds. A loop over a Boolean
    /// (`for Stepwise := false to true do`, 27 Microsoft codeunits) runs for false and then for true, the variable
    /// holding the Boolean each round (registry entry for-boolean). Not pinned. Option bounds of an Integer loop take
    /// its type as an assignment does: the ordinal (Microsoft's "Library - Manufacturing" loops
    /// `for Day := FromDay to ToDay` with Option parameters; pinned by the corpus Option_InCaseAndLoopBounds,
    /// types/ for_to_option).
    #[inline(always)]
    pub(crate) fn for_limits(from: Value, to: Value) -> Result<(i64, i64), Signal> {
        let (from, to) = (Self::for_bound(from), Self::for_bound(to));
        Ok((int(&from)?, int(&to)?))
    }

    /// The value the counter variable of a `for` loop holds in the round with counter `i`.
    #[inline(always)]
    pub(crate) fn for_counter(kind: ForKind, i: i64, slot_ty: &RtType) -> Result<Value, Signal> {
        if kind.is_boolean {
            Ok(Value::Boolean(i != 0))
        } else if kind.is_integer
            && let Ok(n) = i32::try_from(i)
        {
            // What the conversion of a BigInteger into an Integer gives, without the Decimal in between; a counter
            // beyond Integer takes the general path, which reports it.
            Ok(Value::Integer(n))
        } else {
            ops::convert(Value::BigInteger(i), slot_ty)
        }
    }

    /// A call of a procedure: `instance` is the value the procedure is called on (`None`: the running object). `values` are the
    /// argument values the bytecode already computed.
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn call_site(
        &mut self,
        object: usize,
        routine: usize,
        instance: Option<Value>,
        args: &[ir::Arg],
        values: Vec<Value>,
        try_call: bool,
        f: &mut Frame,
    ) -> Result<Value, Signal> {
        let inst = match instance {
            None => f.instance.clone(),
            Some(v) => match v {
                // A procedure of a reportextension called through the report variable runs on the extension's
                // instance of that variable, which keeps the extension's globals and added data items
                // (reportext/W3RpExtProcedures.al:ExtensionProcedure_*).
                Value::Codeunit(i)
                    if self.engine.program.objects[object].report_ext.is_some()
                        && i.borrow().object as usize != object =>
                {
                    self.report_extension_instance(&i, object)?
                }
                Value::Codeunit(i) => self.shared_instance(i),
                // A table procedure runs with the record as its Rec, on the record
                // variable's own globals (ADR-021).
                Value::Record(rc) => {
                    // `Rec.InitType()` in a page's code: the table procedure's xRec is the page's own xRec, as the page's triggers see it
                    // (Sales Invoice Subform.OnNewRecord calls Rec.InitType, which copies xRec.Type; pages/lt-xrec-table-procedure/
                    // XRecTableProcedure.al `proc xRec=2/22/two`).
                    let page_xrec = {
                        let fi = f.instance.borrow();
                        let o = &self.engine.program.objects[fi.object as usize];
                        match (o.kind == al_ir::ObjKind::Page, o.rec_slot, o.xrec_slot) {
                            (true, Some(r), Some(x)) => match (&fi.globals[r], &fi.globals[x]) {
                                (Value::Record(pr), Value::Record(px)) if Rc::ptr_eq(pr, &rc) => Some(px.clone()),
                                _ => None,
                            },
                            _ => None,
                        }
                    };
                    let (inst, saved) = self.enter_record(object, rc, page_xrec.as_ref())?;
                    let r = self.finish_call(object, routine, inst.clone(), args, values, try_call, f);
                    Self::leave_record(&inst, saved);
                    return r;
                }
                // A query procedure runs with the query it was called on as its own state.
                Value::Query(q) => self.query_instance(object, q),
                // The page a part shows (`CurrPage.<Part>.Page`), or a `Page` variable, which
                // is constructed at first use.
                Value::TestPage(st) => {
                    let i = self.construct_page(&st)?;
                    // A procedure a page extension declares runs on the extension's instance.
                    let ext = st
                        .borrow()
                        .ext_instances
                        .iter()
                        .find(|(e, _)| *e == object)
                        .map(|(_, i)| i.clone());
                    self.shared_instance(ext.unwrap_or(i))
                }
                other => {
                    return Err(Signal::Unsupported(format!(
                        "a procedure call on a {} value",
                        value_kind(&other)
                    )));
                }
            },
        };
        self.finish_call(object, routine, inst, args, values, try_call, f)
    }

    /// Evaluates the arguments of a resolved call and runs it on `inst`, writing back the
    /// by-reference arguments.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn finish_call(
        &mut self,
        object: usize,
        routine: usize,
        inst: Rc<RefCell<Instance>>,
        args: &[ir::Arg],
        values: Vec<Value>,
        try_call: bool,
        f: &mut Frame,
    ) -> Result<Value, Signal> {
        // A publisher's `var` argument that is a global of the caller's instance is that variable itself for its subscribers
        // (events/lt-event-var-through-sender/EventVarThroughSender.al): they alias it, see `Frame::aliases`.
        let facts = self.engine.facts[object][routine];
        if facts.publisher {
            let by_var = &self.engine.program.objects[object].routines[routine].by_var;
            self.events.var_origins = args
                .iter()
                .enumerate()
                .map(|(i, a)| match a {
                    ir::Arg::Ref(ir::Place::Global(g)) if by_var.get(i).copied().unwrap_or(false) => {
                        Some((f.instance.clone(), *g))
                    }
                    _ => None,
                })
                .collect();
        }
        // The variables the frame's event parameters stand for: the parameter's value goes to the variable before the call and
        // the variable's value comes back after it, because the callee (the sender) may read or change either.
        for (slot, inst, g) in &f.aliases {
            inst.borrow_mut().globals[*g] = f.locals[*slot].clone();
        }
        // Only a routine that reads MaxStrLen of a `var` parameter, or hands one on, gets them (known at lowering), so other
        // calls pay nothing. An event's subscribers get the lengths of the publisher's arguments (`publish`).
        let var_lens = facts.var_lens;
        if var_lens {
            self.events.next_var_lens = self.var_lens_of(object, routine, args, f);
        } else if facts.publisher {
            self.events.publisher_var_lens = self.var_lens_of(object, routine, args, f);
        }
        // A scalar global of the caller passed by `var` is watched while the call runs (`alias_access`). Not for an event
        // publisher: there the subscribers alias the variable, which is pinned (above).
        let watched = if facts.publisher {
            0
        } else {
            self.watch_var_globals(object, routine, args, &values, f)
        };
        if try_call {
            self.enter_used_try();
        }
        let result = self.call(object, routine, inst, values);
        if try_call {
            self.leave_used_try();
        }
        if watched > 0 {
            let n = self.var_alias_watch.len() - watched;
            self.var_alias_watch.truncate(n);
        }
        if var_lens {
            self.events.next_var_lens.clear();
        } else if facts.publisher {
            self.events.publisher_var_lens.clear();
        }
        for (slot, inst, g) in &f.aliases {
            f.locals[*slot] = inst.borrow().globals[*g].clone();
        }
        if let Err(Signal::Error(_)) = &result {
            if let Some(finals) = self.error_finals.take() {
                for (a, v) in args.iter().zip(finals) {
                    if let ir::Arg::Ref(p) = a {
                        self.write(p, v, f)?;
                    }
                }
            }
        }
        let (ret, finals) = match (result, try_call) {
            // A write the try was not allowed to make is not trapped by it (misc/lt-cu-odd-leads/CuTryTests.al:83-150).
            (Err(Signal::Error(e)), true) if e.code == crate::transactions::TRY_FUNCTION_WRITE => {
                return Err(Signal::Error(e));
            }
            (Err(Signal::Error(mut e)), true) => {
                self.add_top_frame(&mut e);
                self.last_error = *e;
                return Ok(Value::Boolean(false));
            }
            (Ok((_, finals)), true) => (Value::Boolean(true), finals),
            (r, _) => r?,
        };
        let mut finals = finals;
        for (a, v) in args.iter().zip(finals.drain(..)) {
            if let ir::Arg::Ref(p) = a {
                self.write_back(p, v, f)?;
            }
        }
        self.return_vec(finals);
        Ok(ret)
    }

    /// Starts watching the scalar globals of the caller's instance that `args` pass by `var`; returns how many were added.
    fn watch_var_globals(
        &mut self,
        object: usize,
        routine: usize,
        args: &[ir::Arg],
        values: &[Value],
        f: &Frame,
    ) -> usize {
        let mut added = 0;
        for (i, a) in args.iter().enumerate() {
            let ir::Arg::Ref(ir::Place::Global(g)) = a else {
                continue;
            };
            let by_var = self.engine.program.objects[object].routines[routine]
                .by_var
                .get(i)
                .copied()
                .unwrap_or(false);
            if by_var && values.get(i).is_some_and(is_alias_scalar) {
                self.var_alias_watch.push((f.instance.clone(), *g));
                added += 1;
            }
        }
        added
    }

    /// Refuses a read or a write of global `i` of the frame's instance by its own name while a procedure that got the same
    /// global by `var` runs. BC passes the argument by reference, so the parameter and the global would be one variable; the
    /// runtime copies it in and back, and the two only differ when the global is reached this way. Pinned for an event's
    /// subscribers (events/lt-event-var-through-sender/EventVarThroughSender.al), not for a plain procedure call
    /// (docs/probes-needed.md, "var parameter that is a global of the caller").
    #[inline]
    pub(crate) fn alias_access(&self, i: usize, f: &Frame) -> Result<(), Signal> {
        if self.var_alias_watch.is_empty() {
            return Ok(());
        }
        self.alias_access_slow(i, f)
    }

    #[cold]
    #[inline(never)]
    fn alias_access_slow(&self, i: usize, f: &Frame) -> Result<(), Signal> {
        if self
            .var_alias_watch
            .iter()
            .any(|(inst, g)| *g == i && Rc::ptr_eq(inst, &f.instance))
        {
            let name = &self.engine.program.objects[f.instance.borrow().object as usize].globals[i].name;
            return Err(Signal::Unsupported(format!(
                "the global variable {name} is read or changed by its name while a procedure it was passed to by var runs \
                 (whether the parameter and the global are one variable is not pinned)"
            )));
        }
        Ok(())
    }

    /// Whether any global of the frame's instance is watched (for ClearAll, which resets them all).
    pub(crate) fn alias_any(&self, f: &Frame) -> Result<(), Signal> {
        let watched = self
            .var_alias_watch
            .iter()
            .find(|(inst, _)| Rc::ptr_eq(inst, &f.instance))
            .map(|(_, g)| *g);
        match watched {
            Some(g) => self.alias_access_slow(g, f),
            None => Ok(()),
        }
    }

    /// The final value of a by-reference parameter goes back to the caller's place. An `HttpContent` the callee only worked on is the
    /// caller's own object: writing it back would store a copy (an HttpContent is a value on assignment) and cut the headers handed
    /// out before from it.
    fn write_back(&mut self, p: &ir::Place, v: Value, f: &mut Frame) -> Result<(), Signal> {
        if let Value::Library(n, cell) = &v {
            if n == "HttpContent" {
                if let Ok(Value::Library(_, current)) = self.read(p, f) {
                    if Rc::ptr_eq(&current, cell) {
                        return Ok(());
                    }
                }
            }
        }
        self.write(p, v, f)
    }

    /// The instance a call on codeunit value `i` runs on: the session's
    /// singleton for `SingleInstance = true` codeunits, `i` itself otherwise.
    pub(crate) fn shared_instance(&mut self, i: Rc<RefCell<Instance>>) -> Rc<RefCell<Instance>> {
        let object = i.borrow().object as usize;
        let single = self.engine.program.objects[object]
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("SingleInstance") && v.trim().eq_ignore_ascii_case("true"));
        if !single {
            return i;
        }
        if !self.singletons.contains_key(&object) {
            let inst = self.instance(object);
            self.singletons.insert(object, inst);
        }
        self.singletons[&object].clone()
    }

    /// The instance of the page or report that the running extension extends (ADR-023). A missing
    /// or dropped link is reported, never a panic.
    pub(crate) fn base_instance(&self, f: &Frame) -> Result<Rc<RefCell<Instance>>, Signal> {
        f.instance
            .borrow()
            .base
            .as_ref()
            .and_then(std::rc::Weak::upgrade)
            .ok_or_else(|| {
                Signal::Unsupported(
                    "internal error: extension code named a member of its base object, but the extension's instance \
                     has no live link to the base object's instance"
                        .into(),
                )
            })
    }

    /// The declared type of a local, a global, or an element of an array of those, by reference.
    pub(crate) fn place_ty<'a>(&'a self, p: &ir::Place, f: &Frame) -> Option<&'a RtType> {
        match p {
            ir::Place::Local(i) => Some(&self.engine.program.objects[f.object].routines[f.routine].locals[*i].ty),
            ir::Place::Global(i) => {
                let o = f.instance.borrow().object as usize;
                Some(&self.engine.program.objects[o].globals[*i].ty)
            }
            ir::Place::Index(base, _) => match self.place_ty(base, f)? {
                RtType::Array(_, el) => Some(el),
                _ => None,
            },
            _ => None,
        }
    }

    /// For each `var` Text or Code parameter of the callee: the declared length of the variable or field the argument names
    /// (text/lt-text-functions/MaxStrLen.Codeunit.al:91-151, LT_BU_MaxStrLen_TextParameters), `i32::MAX` for one without a
    /// length. A `var` parameter of the caller handed on keeps the length the caller got for it, through any number of hops
    /// and whatever length the parameter on the way declares (:146-170, PYC_MaxStrLen_VarParameterHops: `three hops text30`
    /// = 30, `hop through text30 param field` = 100); `None` when the caller did not get one. Empty when no such parameter
    /// is passed.
    fn var_lens_of(&self, object: usize, routine: usize, args: &[ir::Arg], f: &Frame) -> Vec<Option<i32>> {
        let r = &self.engine.program.objects[object].routines[routine];
        let mut out = Vec::new();
        for (i, a) in args.iter().enumerate().take(r.params) {
            let ir::Arg::Ref(p) = a else { continue };
            if !r.by_var.get(i).copied().unwrap_or(false)
                || !matches!(r.locals[i].ty, RtType::Text(_) | RtType::Code(_))
            {
                continue;
            }
            let caller = &self.engine.program.objects[f.object].routines[f.routine];
            let len = match p {
                ir::Place::Local(j) if *j < caller.params && caller.by_var.get(*j).copied().unwrap_or(false) => {
                    f.var_lens.get(*j).copied().flatten()
                }
                _ => match self.place_type(p, f) {
                    RtType::Text(Some(n)) | RtType::Code(Some(n)) => i32::try_from(n).ok(),
                    RtType::Text(None) | RtType::Code(None) => Some(i32::MAX),
                    _ => None,
                },
            };
            if out.len() <= i {
                out.resize(i + 1, None);
            }
            out[i] = len;
        }
        out
    }

    /// `MaxStrLen` of `var` parameter `i` of the running routine (see `var_lens_of`).
    pub(crate) fn var_param_max_len(&self, p: &ir::Place, f: &Frame) -> Result<Value, Signal> {
        match p {
            ir::Place::Local(i) => match f.var_lens.get(*i).copied().flatten() {
                Some(n) => Ok(Value::Integer(n)),
                None => Err(Signal::Unsupported(
                    "MaxStrLen of a var parameter whose argument's declared length is not known (a routine the platform calls, not an AL call: not pinned)".into(),
                )),
            },
            _ => Err(Signal::Unsupported("MaxStrLen of a var parameter that is not a local".into())),
        }
    }

    pub(crate) fn place_type(&self, p: &ir::Place, f: &Frame) -> RtType {
        if let Some(t) = self.place_ty(p, f) {
            return t.clone();
        }
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        match p {
            ir::Place::Local(i) => r.locals[*i].ty.clone(),
            ir::Place::Global(i) => {
                let o = f.instance.borrow().object as usize;
                self.engine.program.objects[o].globals[*i].ty.clone()
            }
            ir::Place::BaseGlobal(i) => match self.base_instance(f) {
                Ok(b) => {
                    let o = b.borrow().object as usize;
                    self.engine.program.objects[o].globals[*i].ty.clone()
                }
                Err(_) => RtType::Variant,
            },
            ir::Place::Index(base, _) => match self.place_type(base, f) {
                RtType::Array(_, el) => *el,
                other => other,
            },
            ir::Place::Field(base, pos) => match self.place_type(base, f) {
                RtType::Record(t, _) => self.engine.tables[t as usize]
                    .as_ref()
                    .map_or(RtType::Variant, |td| td.fields[*pos].ty.clone()),
                _ => RtType::Variant,
            },
        }
    }

    /// The append of `S += x` once the old value of `S` and `x` are known: in place when nothing else holds the text.
    #[inline(always)]
    pub(crate) fn append_finish(
        &mut self,
        p: &ir::Place,
        max: Option<u32>,
        old: Value,
        rv: Value,
        f: &mut Frame,
    ) -> Result<(), Signal> {
        let v = match (old, &rv) {
            (Value::Text(mut t), Value::Text(x) | Value::Code(x))
                if !t.has_marker()
                    && !x.has_marker()
                    && max.is_none_or(|m| t.utf16_len() + x.utf16_len() <= m as usize) =>
            {
                // Let go of the variable's hold so the append can happen in place.
                match p {
                    ir::Place::Local(i) => f.locals[*i] = Value::Null,
                    ir::Place::Global(i) => f.instance.borrow_mut().globals[*i] = Value::Null,
                    _ => unreachable!("a local or a global"),
                }
                t.push_str(x);
                ops::convert(Value::Text(t), &RtType::Text(max))?
            }
            // `S += T[i]`: a Char appends in place as a Text does. A surrogate half and the call-stack mark stay on the slow
            // path, which builds the text the long way.
            (Value::Text(mut t), Value::Char(c))
                if char::from_u32(u32::from(*c)).is_some_and(|ch| ch != al_builtins::native::CALL_STACK_MARK)
                    && !t.has_marker()
                    && max.is_none_or(|m| t.utf16_len() < m as usize) =>
            {
                let ch = char::from_u32(u32::from(*c)).expect("checked in the guard");
                match p {
                    ir::Place::Local(i) => f.locals[*i] = Value::Null,
                    ir::Place::Global(i) => f.instance.borrow_mut().globals[*i] = Value::Null,
                    _ => unreachable!("a local or a global"),
                }
                t.push(ch);
                ops::convert(Value::Text(t), &RtType::Text(max))?
            }
            (old, rv) => ops::convert(ops::arith(ops::Arith::Add, &old, rv)?, &RtType::Text(max))?,
        };
        self.write(p, v, f)
    }

    pub(crate) fn read(&mut self, p: &ir::Place, f: &mut Frame) -> Result<Value, Signal> {
        match p {
            ir::Place::Local(i) => Ok(f.locals[*i].clone()),
            ir::Place::Global(i) => {
                self.alias_access(*i, f)?;
                Ok(f.instance.borrow().globals[*i].clone())
            }
            ir::Place::BaseGlobal(i) => {
                let base = self.base_instance(f)?;
                let v = base.borrow().globals.get(*i).cloned();
                v.ok_or_else(|| base_slot_missing(*i))
            }
            ir::Place::Index(base, idx) => {
                let (arr, flat) = match self.element(base, idx, f) {
                    Ok(e) => e,
                    // A row of a two-dimensional array, as an argument.
                    Err(Signal::Unsupported(m)) if m.starts_with(ROW_REFUSAL) => {
                        return match self.row_range(base, idx, f)? {
                            Some((arr, start, len)) => Ok(Value::Array(Rc::new(RefCell::new(
                                arr.borrow()[start..start + len].to_vec(),
                            )))),
                            None => Err(Signal::Unsupported(m)),
                        };
                    }
                    Err(e) => return Err(e),
                };
                let v = arr.borrow()[flat].clone();
                Ok(v)
            }
            ir::Place::Field(base, pos) => {
                let record = self.read(base, f)?;
                self.read_record_field(record, *pos)
            }
        }
    }

    /// The value of field `pos` of the record `record` holds (the read of `Rec.Field`, also of a field of a procedure's
    /// result).
    pub(crate) fn read_record_field(&mut self, record: Value, pos: usize) -> Result<Value, Signal> {
        let pos = &pos;
        match record {
            Value::Record(rc) => {
                self.connection_check(&rc)?;
                if let Some(why) = rc.borrow().unpinned_buffer {
                    return Err(Signal::Unsupported(why.into()));
                }
                let r = rc.borrow();
                if !self.engine.unevaluable_init.is_empty() && !r.touched {
                    // The buffer of a record nothing has written is built by this read, which
                    // evaluates the InitValues (record/TestDateTimeInitValue.Codeunit.al:
                    // DateTimeInitValue_Table_UntouchedRecordRaisesOnReadNotOnInit).
                    if let Some(message) = self.engine.unevaluable_init.get(&(r.table, *pos)) {
                        return Err(Signal::Error(Box::new(AlError {
                            message: message.clone(),
                            code: String::new(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                }
                if !self.engine.unevaluable_init.is_empty() && r.touched {
                    let engine = self.engine.clone();
                    if let Some(message) = engine.unevaluable_init.get(&(r.table, *pos)) {
                        let pristine = self.initial_fields(r.table as usize);
                        if ops::compare(&r.fields[*pos], &pristine[*pos]).ok() == Some(std::cmp::Ordering::Equal) {
                            return Err(Signal::Unsupported(format!(
                                "a read of the field whose InitValue BC cannot evaluate (`{message}`), in a record where another field was written (not pinned)"
                            )));
                        }
                    }
                }
                let v = r.fields[*pos].clone();
                if let Value::Unknown(why) = &v {
                    return Err(Signal::Unsupported(why.to_string()));
                }
                if let Some(why) = self.install_row_order(r.table, r.temporary, *pos, &r.fields) {
                    return Err(Signal::Unsupported(why));
                }
                if r.load.partial {
                    drop(r);
                    rc.borrow_mut().load.touch(*pos);
                }
                Ok(v)
            }
            other => Err(Signal::Unsupported(format!("field of {}", other.describe()))),
        }
    }

    pub(crate) fn write(&mut self, p: &ir::Place, v: Value, f: &mut Frame) -> Result<(), Signal> {
        // A column a platform row leaves unknown never reaches a variable: every read of one is refused where it happens; this
        // catches a reader that missed it.
        if let Value::Unknown(why) = &v {
            return Err(Signal::Unsupported(why.to_string()));
        }
        // `:=` between FilterPageBuilders copies the control collection and shares the records
        // (corpus filterpage/TestFilterPageBuilder.al:FilterPageBuilder_Assignment_*).
        let v = match v {
            Value::Library(al_builtins::library_kind::LibraryKind::FilterPageBuilder, cell) => Value::Library(
                al_builtins::library_kind::LibraryKind::FilterPageBuilder,
                filterpage::copy_for_assignment(&cell),
            ),
            v => v,
        };
        // `:=` between records copies the fields into the target record (ADR-010).
        if let Value::Record(src) = &v {
            // A Variant that holds a record takes the new value whole, whatever table it comes from: it is no record variable
            // (Library - Variable Storage.Dequeue into one `RecVar` that held a Contact, then a Segment Header; Tests-Marketing).
            // A Variant holding a record of ANOTHER table is replaced (by the by-value copy below). A Variant holding one of the
            // same table still takes the fields, which keeps the table a Variant parameter was bound to (a temporary record's
            // rows): replacing it gave the callee an empty table (Find Record Management.GetLastEntryIntFieldValue, called from
            // "Price Asset".OnInsert; Tests-ERM Price Calculation - V16).
            let replaces_variant = matches!(self.place_type(p, f), RtType::Variant)
                && matches!(self.read(p, f), Ok(Value::Record(d)) if d.borrow().table != src.borrow().table);
            if !matches!(p, ir::Place::Field(..)) && !replaces_variant {
                if let Ok(Value::Record(dst)) = self.read(p, f) {
                    // A record variable takes no record of another table; that only happens through a Variant
                    // (types/lt-variant-record-reassign/VariantRecordReassign.al:130 Variant_Reassign_ReadBack).
                    if dst.borrow().table != src.borrow().table && !matches!(self.place_type(p, f), RtType::Variant) {
                        return Err(Signal::Error(Box::new(AlError {
                            message: "The requested operation is not supported.".into(),
                            code: "NotSupportedOperation".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    if !Rc::ptr_eq(src, &dst) {
                        // A copied record holds copies of its Blobs (value semantics). The corpus
                        // does not tell a copy from a shared Blob (record/TestFormatRecordContracts.al
                        // only formats the copy): docs/probes-needed.md.
                        let fields: Vec<Value> = src
                            .borrow()
                            .fields
                            .iter()
                            .map(|f| match f {
                                Value::Blob(b) => {
                                    let b = b.borrow();
                                    Value::Blob(std::sync::Arc::new(al_builtins::stream::BlobCell::new(
                                        al_builtins::stream::BlobBuf {
                                            bytes: b.bytes.clone(),
                                            write_pos: 0,
                                            // A copy shows the bytes (and a stream over it reads them, streams/
                                            // TestProbeStreamContracts.al:RecordAssign_CopiesBlobBytes), but CalcFields
                                            // fetches the Blob from the stored row again (types/ProbeTypeFormsAndFilters.al:
                                            // FormatRecord_AssignmentCopiesBlobBytes, BC 27.5).
                                            // A source whose content was not fetched (a Get without CalcFields) gives a copy that reads
                                            // empty (misc/lt-probes-needed-1/PnABlobAssignRename.al:Assign_Stored `copy-content-notloaded`).
                                            loaded: b.loaded,
                                            fetched: b.fetched,
                                            refetch: true,
                                            persisted: false,
                                            temp_row: b.temp_row,
                                            written: false,
                                            render_pending: b.render_pending.clone(),
                                            inserted: b.inserted,
                                        },
                                    )))
                                }
                                other => other.clone(),
                            })
                            .collect();
                        let load = src.borrow().load.clone();
                        // `:=` copies what Record.IsDirty compares with (lt-ac/ACRecordIsDirty.al:63-64 `copy_dirty`).
                        let dirty_base = src.borrow().is_dirty_base.copy();
                        let mut d = dst.borrow_mut();
                        d.fields = fields;
                        d.is_dirty_base = dirty_base;
                        // The copy holds what the source's buffer held, if that is known.
                        d.load.buffer = load.buffer;
                        d.load.partial = load.partial;
                    }
                    return Ok(());
                }
            }
        }
        if let ir::Place::Field(base, pos) = p {
            let ty = self.place_type(p, f);
            // A number a Variant holds goes into a Text or Code field as its Format text (the lowering leaves it unconverted for
            // that); the guard is one test on the common path.
            let v = if matches!(
                v,
                Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) | Value::Enum(..)
            ) {
                match ops::number_into_text_field(&v, &ty, &*self) {
                    Some(r) => r?,
                    None => ops::convert(v, &ty)?,
                }
            } else {
                ops::convert(v, &ty)?
            };
            return match self.read(base, f)? {
                Value::Record(rc) => {
                    self.connection_check(&rc)?;
                    let mut r = rc.borrow_mut();
                    r.before_field_write();
                    r.fields[*pos] = v;
                    r.touched = true;
                    r.assigned = true;
                    if r.load.partial {
                        r.load.written(*pos);
                    }
                    Ok(())
                }
                other => Err(Signal::Unsupported(format!("field of {}", other.describe()))),
            };
        }
        // A record assigned to a Variant is a by-value copy: a temporary record's copy is an empty table of its own
        // (recordref/PWRecordMisc.al:TempRecord_AssignedToVariantVariable and :TempRecord_VariantModifiedAfterAssign).
        let v = match &v {
            Value::Record(rc) if matches!(self.place_type(p, f), RtType::Variant) => self.record_by_value(rc),
            _ => v,
        };
        match p {
            ir::Place::Local(i) => f.locals[*i] = v,
            ir::Place::Global(i) => {
                self.alias_access(*i, f)?;
                f.instance.borrow_mut().globals[*i] = v
            }
            ir::Place::BaseGlobal(i) => {
                let base = self.base_instance(f)?;
                let mut b = base.borrow_mut();
                match b.globals.get_mut(*i) {
                    Some(slot) => *slot = v,
                    None => return Err(base_slot_missing(*i)),
                }
            }
            ir::Place::Index(base, idx) if matches!(self.place_type(base, f), RtType::Text(_) | RtType::Code(_)) => {
                return self.write_char(base, idx, v, f);
            }
            ir::Place::Index(base, idx) => {
                let el_ty = self.place_type(p, f);
                let (arr, flat) = match self.element(base, idx, f) {
                    Ok(e) => e,
                    // The row a callee wrote through a `var` array parameter goes back into the row.
                    Err(Signal::Unsupported(m)) if m.starts_with(ROW_REFUSAL) => {
                        let (Some((arr, start, len)), Value::Array(row)) = (self.row_range(base, idx, f)?, &v) else {
                            return Err(Signal::Unsupported(m));
                        };
                        if row.borrow().len() != len {
                            return Err(Signal::Unsupported(
                                "a row of an array written from an array of another length (not pinned)".into(),
                            ));
                        }
                        arr.borrow_mut()[start..start + len].clone_from_slice(&row.borrow());
                        return Ok(());
                    }
                    Err(e) => return Err(e),
                };
                arr.borrow_mut()[flat] = ops::convert(v, &el_ty)?;
            }
            ir::Place::Field(..) => unreachable!("handled above"),
        }
        Ok(())
    }

    /// `Text[i] := c`: sets one character of a text or code, at a position inside it or the next one within
    /// the declared length (handlers/TestPageFieldFormats_Tests.al:58-66,
    /// types/TestCharIndexAssignContracts.al:CharacterAssignedIntoAPosition); a position outside that and a value
    /// that is not one UTF-16 character are refused.
    pub(crate) fn write_char(
        &mut self,
        base: &ir::Place,
        idx: &[ir::Expr],
        v: Value,
        f: &mut Frame,
    ) -> Result<(), Signal> {
        // A Code takes the character upper-cased (types/TestCharIndexAssignContracts.al:CharacterAssignedIntoAPosition:
        // `Letter[1] := 'a'` gives `A`, `Ten[3] := 'z'` gives `ABZDE`).
        let (max, is_code) = match self.place_type(base, f) {
            RtType::Text(m) => (m, false),
            RtType::Code(m) => (m, true),
            _ => (None, false),
        };
        let [e] = idx else {
            return Err(Signal::Unsupported("a text indexed with several positions".into()));
        };
        let i = int(&Self::index_value(e, f)?)?;
        // A Byte from 0x80 up goes in as the character Format shows for it: 128 is `Ç` (199), 239 is `Ù` (217)
        // (al-language-onprem/types/ByteFormat.al:13-31 `_text1_code`, report/lt-report-dataset-header/
        // ReportDatasetHeaderRaw.al:Lines_Window1, the byte order mark read byte by byte); a byte whose character is not
        // pinned is refused.
        let unit = match &v {
            Value::Byte(b) if *b >= 0x80 => {
                let enc = al_builtins::stream::Encoding::MsDos;
                if !enc.decode_pinned(&[*b]) {
                    return Err(Signal::Unsupported(format!(
                        "assigning Byte {b} to a character of a text (its character is not pinned)"
                    )));
                }
                enc.decode(&[*b]).encode_utf16().next()
            }
            _ => match int(&v) {
                Ok(c) => u16::try_from(c).ok().filter(|u| !(0xD800..0xE000).contains(u)),
                Err(_) => None,
            },
        };
        // A Text of ASCII only gets an ASCII character in place (a loop over a long text stays linear): the variable lets
        // go of its hold first so that nothing is copied.
        if let (false, Some(unit), true) = (is_code, unit.filter(|u| *u < 128), i >= 1) {
            if let ir::Place::Local(l) = base {
                if let Value::Text(t) = &mut f.locals[*l] {
                    let at = (i - 1) as usize;
                    if t.is_ascii() && at < t.len() {
                        t.set_ascii_byte(at, unit as u8);
                        return Ok(());
                    }
                }
            }
        }
        let (Value::Text(s) | Value::Code(s)) = self.read(base, f)? else {
            return Err(Signal::Unsupported(
                "assigning a character into a value that is no text".into(),
            ));
        };
        let mut units: Vec<u16> = s.encode_utf16().collect();
        let Some(unit) = unit else {
            return Err(Signal::Unsupported(format!(
                "assigning {v} to a character of a text (only a UTF-16 character is pinned)",
                v = v.describe()
            )));
        };
        // A position outside 1 to the length plus one raises (types/TestProbeScalarContracts.al:TextChar_AssignByIndex:
        // 5, 0 and -1 on 'abc'); the position after the last one appends, and past the declared length it raises the
        // length error of an assignment.
        if i < 1 || (i as usize) > units.len() + 1 {
            return al_builtins::native::raise(
                format!(
                    "The value of INDEXING parameter 1 is outside of the permitted range. The current value is: {i}. The permitted range is: from 1 to {}.",
                    units.len()
                ),
                "OutsidePermittedRange",
            );
        }
        let i = i as usize - 1;
        if i == units.len() {
            units.push(unit);
        } else {
            units[i] = unit;
        }
        if let Some(m) = max.filter(|m| units.len() > *m as usize) {
            return al_builtins::native::raise(
                format!(
                    "The length of the string is {}, but it must be less than or equal to {m} characters. Value: {}",
                    units.len(),
                    ops::shown_in_length_error(&String::from_utf16_lossy(&units))
                ),
                "StringLengthExceeded",
            );
        }
        let text = String::from_utf16(&units)
            .map_err(|_| Signal::Unsupported("a text that ends up with a lone surrogate".into()))?;
        if is_code {
            return self.write(base, Value::Code(ops::upper_code_checked(&text)?.into()), f);
        }
        self.write(base, Value::Text(text.into()), f)
    }

    /// The cells of row `idx` of the two-dimensional array behind `base`, as (cells, first position, length): `None` for an
    /// array of another number of dimensions or an index that is not one position. A row passed as an argument is a copy for a
    /// value parameter, and for a `var` parameter it is written back into the row when the call returns (corpus
    /// errors/lt-ms-refusals-128/P128ArrayRow.al:P128_ArrayRow_ByValue, P128_ArrayRow_ByVar, P128_ArrayRow_TextRowByVar). What
    /// BC does with a row assigned or cleared is not pinned here.
    pub(crate) fn row_range(
        &mut self,
        base: &ir::Place,
        idx: &[ir::Expr],
        f: &mut Frame,
    ) -> Result<Option<(Cells, usize, usize)>, Signal> {
        let RtType::Array(dims, _) = self.place_type(base, f) else {
            return Ok(None);
        };
        // A row is what is left of the array after fixing its first indexes: one of two (`A[2]` of `array[3, 4]`), or one or two of
        // three (collections/lt-array-rows/ArrayRows.al:ThreeDim_Rows, ThreeDim_RowToShorterParameter). More dimensions are not pinned.
        if dims.len() < 2 || dims.len() > 3 || idx.is_empty() || idx.len() >= dims.len() {
            return Ok(None);
        }
        let mut start = 0usize;
        for (k, e) in idx.iter().enumerate() {
            let i = int(&Self::index_value(e, f)?)?;
            if i < 1 || i > i64::from(dims[k]) {
                return al_builtins::native::raise("Index out of bounds.", "IndexOutOfBounds");
            }
            let below: usize = dims[k + 1..].iter().map(|d| *d as usize).product();
            start += (i as usize - 1) * below;
        }
        let len: usize = dims[idx.len()..].iter().map(|d| *d as usize).product();
        let arr = match self.read(base, f)? {
            Value::Array(a) => a,
            other => return Err(Signal::Unsupported(format!("indexing {}", other.describe()))),
        };
        if start + len > arr.borrow().len() {
            return Err(Signal::Unsupported(
                "a row of an array that has fewer cells than it declares (not pinned)".into(),
            ));
        }
        Ok(Some((arr, start, len)))
    }

    /// The array behind `base` and the flat position of `idx` (1-based, row-major).
    pub(crate) fn element(
        &mut self,
        base: &ir::Place,
        idx: &[ir::Expr],
        f: &mut Frame,
    ) -> Result<(Cells, usize), Signal> {
        // The dimensions of an array variable (a local or a global, the usual case) are read where they are
        // declared; `place_type` would clone the type, and its dimension list, on every element access.
        let mut small = [0u32; 4];
        let mut n = usize::MAX;
        {
            let declared = match base {
                ir::Place::Local(i) => Some(&self.engine.program.objects[f.object].routines[f.routine].locals[*i].ty),
                ir::Place::Global(i) => {
                    let o = f.instance.borrow().object as usize;
                    Some(&self.engine.program.objects[o].globals[*i].ty)
                }
                _ => None,
            };
            if let Some(RtType::Array(d, _)) = declared {
                if d.len() <= small.len() {
                    small[..d.len()].copy_from_slice(d);
                    n = d.len();
                }
            }
        }
        let big: Vec<u32>;
        let dims: &[u32] = if n != usize::MAX {
            &small[..n]
        } else {
            let RtType::Array(d, _) = self.place_type(base, f) else {
                return Err(Signal::Unsupported("indexing a non-array".into()));
            };
            big = d;
            &big
        };
        // `A[x]` of a two-dimensional array is a row; what BC does with one (Clear, assignment, an argument) is
        // not pinned (docs/probes-needed.md). It used to be read as the cell at flat position x.
        if idx.len() != dims.len() {
            return Err(Signal::Unsupported(format!(
                "a row of a {}-dimensional array (indexed by {} of its dimensions): what BC does with it is not pinned (docs/probes-needed.md)",
                dims.len(),
                idx.len()
            )));
        }
        let arr = match self.read(base, f)? {
            Value::Array(a) => a,
            other => return Err(Signal::Unsupported(format!("indexing {}", other.describe()))),
        };
        let mut flat = 0usize;
        for (d, e) in dims.iter().zip(idx) {
            let i = int(&Self::index_value(e, f)?)?;
            // A one-dimensional array is measured by the array the variable holds: a parameter declared `array[3]` takes the caller's
            // array of any length and indexes within it (types/W7VarArray.al; Tests-Bank, Tests-Cash Flow and Tests-SCM-Manufacturing
            // index a parameter within the argument's length), so the check against the length follows below.
            if i < 1 || (dims.len() > 1 && i > i64::from(*d)) {
                // Index out of bounds.: collections/lt-array-rows/ArrayRows.al, errors/lt-error-handling/ErTRuntimeErrors.al:179-184
                // (zero, negative and too high), errors/lt-ms-refusals-129/P129ArrayIndex.al.
                return al_builtins::native::raise("Index out of bounds.", "IndexOutOfBounds");
            }
            flat = flat * (*d as usize) + (i as usize - 1);
        }
        // A parameter declared with more elements than the argument has (Tests-ERM codeunit 134831 passes
        // an `array[2]` to `var DimensionValue: array[9]`): the stored cells are the argument's. What BC does
        // is not pinned by the corpus, so it is refused rather than indexed out of range.
        // A one-dimensional array passed to a parameter declared with more elements: reading or writing past the argument's
        // elements is an index error (misc/lt-probes-needed-5/PnE01Arrays.al:SmallerArray_VarParameter, SmallerArray_ByValue).
        if flat >= arr.borrow().len() && dims.len() == 1 {
            return al_builtins::native::raise("Index out of bounds.", "IndexOutOfBounds");
        }
        if flat >= arr.borrow().len() {
            return Err(Signal::Unsupported(format!(
                "array element {} of an argument that has {} elements, passed to a parameter declared with {}; not pinned (docs/probes-needed.md)",
                flat + 1,
                arr.borrow().len(),
                dims.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", ")
            )));
        }
        Ok((arr, flat))
    }
}

impl Session {
    /// The value of a constant node. The ones that cost something to build are built once per session.
    #[inline(always)]
    pub(crate) fn constant_once(&mut self, c: &ir::Const) -> Value {
        match c {
            ir::Const::Integer(i) => Value::Integer(*i),
            ir::Const::Boolean(b) => Value::Boolean(*b),
            ir::Const::Decimal(_) | ir::Const::Text(_) | ir::Const::Option(..) => self.constant_cached(c),
            other => constant(other),
        }
    }

    #[inline(never)]
    pub(crate) fn constant_cached(&mut self, c: &ir::Const) -> Value {
        let key = c as *const ir::Const as usize;
        if let Some(v) = self.constants.get(&key) {
            return v.clone();
        }
        let v = constant(c);
        self.constants.insert(key, v.clone());
        v
    }
}

pub(crate) fn constant(c: &ir::Const) -> Value {
    match c {
        ir::Const::Boolean(b) => Value::Boolean(*b),
        ir::Const::Integer(i) => Value::Integer(*i),
        ir::Const::BigInteger(i) => Value::BigInteger(*i),
        ir::Const::Decimal(s) => {
            // A literal keeps 18 significant digits (types/lt-expression-operators-literals/ExpressionLiteralsRisky.al:
            // P195_Literal_DecimalTwentyDigits: `123456789012345.12345 * 2` is `246913578024690.246`).
            let d: rust_decimal::Decimal = s.parse().expect("decimal literal from the lowerer");
            Value::Decimal(if d.abs().mantissa() > 999_999_999_999_999_999 {
                d.round_sf_with_strategy(18, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
                    .unwrap_or(d)
            } else {
                d
            })
        }
        ir::Const::Text(s) => Value::Text(s.clone().into()),
        ir::Const::Date(d) => Value::Date(*d, false),
        ir::Const::Time(t) => Value::Time(*t),
        ir::Const::DateTime(t) => Value::DateTime(*t),
        ir::Const::Option(i, m) => Value::Option(*i, m.clone().into()),
        ir::Const::Enum(o, v) => Value::Enum(*o, *v),
        ir::Const::DateFormula(f) => Value::DateFormula(f.clone()),
        ir::Const::Guid(g) => Value::Guid(*g),
        ir::Const::FieldPos(p) => Value::Integer(*p as i32),
    }
}

/// What the type of a `for` counter variable decides, found once per loop.
#[derive(Clone, Copy)]
pub(crate) struct ForKind {
    pub(crate) is_boolean: bool,
    pub(crate) is_integer: bool,
}

impl ForKind {
    #[inline(always)]
    pub(crate) fn of(ty: &RtType) -> ForKind {
        ForKind {
            is_boolean: matches!(ty, RtType::Boolean),
            is_integer: matches!(ty, RtType::Integer),
        }
    }
}

/// The values whose `var` passing is a copy in the runtime: a record, codeunit, list and the other handle kinds are shared
/// objects, so their parameter already is the caller's variable.
fn is_alias_scalar(v: &Value) -> bool {
    matches!(
        v,
        Value::Boolean(_)
            | Value::Integer(_)
            | Value::BigInteger(_)
            | Value::Decimal(_)
            | Value::Byte(_)
            | Value::Char(_)
            | Value::Text(_)
            | Value::Code(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)
            | Value::Duration(_)
            | Value::Guid(_)
            | Value::DateFormula(_)
            | Value::Option(..)
            | Value::Enum(..)
    )
}
