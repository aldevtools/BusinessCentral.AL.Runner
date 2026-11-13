use super::*;
use crate::blob_transfer::{BlobMove, FieldKind, Holder};

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
            return Ok(Value::Boolean(self.in_write_transaction()?));
        }
        if class == Class::Xmlport {
            return self.xmlport_builtin(id, recv, &values, ret_used);
        }
        if id == ids::REQUESTPAGE_UPDATE_1 {
            return self.request_page_update(&values);
        }
        if matches!(
            id,
            ids::REQUESTPAGE_EDITABLE_1
                | ids::REQUESTPAGE_LOOKUPMODE_1
                | ids::REQUESTPAGE_OBJECTID_1
                | ids::REQUESTPAGE_ACTIVATE_1
        ) {
            return self.request_page_member(id, &values, &f.instance);
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
        // ADR-068 item 3: a builtin that read a text with half of a surrogate pair without an audit refuses.
        al_builtins::text::check_unaudited()?;
        if let (Some(ir::Arg::Ref(p)), Some(v)) = (receiver, recv) {
            self.write_native_back(p, v, f)?;
        }
        for (a, v) in args.iter().zip(values.drain(..)) {
            if let ir::Arg::Ref(p) = a {
                let v = self.rendered_into_place(p, v, f)?;
                let v = if class == al_builtins::builtin_id::Class::InStreamRead {
                    self.fit_stream_read(p, v, f)?
                } else {
                    self.fit_ref_number(p, v, f)?
                };
                self.write_native_back(p, v, f)?;
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
            // A text longer than the variable, counted in UTF-16 units as AL counts, raises the length error of an assignment
            // (streams/instream-outstream/StreamUtf8CharsRead.al:245-247: an emoji, 2 units, read into a Text[1], with and
            // without a length).
            (Value::Text(t), RtType::Text(Some(m))) if t.utf16_len() > m as usize => {
                if t.has_lone() {
                    return Err(Signal::Unsupported(
                        "the length error of a text that holds half of a surrogate pair (not pinned)".into(),
                    ));
                }
                al_builtins::native::raise(
                    format!(
                        "The length of the string is {}, but it must be less than or equal to {m} characters. Value: {}",
                        t.utf16_len(),
                        ops::shown_in_length_error(t)
                    ),
                    "StringLengthExceeded",
                )
            }
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
    /// (session-system/TestClearAllTransactionTypeContracts.al:224-285, BC 27.5); a table extension too, with the base table
    /// (`clear_all_other_table_instances`), and a page extension with the base page (`clear_all_base_page`). Any other object
    /// kind is refused.
    pub(crate) fn clear_all(&mut self, f: &Frame) -> Result<Value, Signal> {
        let object = f.instance.borrow().object as usize;
        let engine = self.engine.clone();
        let o = &engine.program.objects[object];
        let keep: Vec<usize> = match o.kind {
            al_ir::ObjKind::Codeunit | al_ir::ObjKind::Table => o.rec_slot.into_iter().collect(),
            // A table extension keeps its record, fields and filters, as a table does (pages/extensions/CyClearAllExtensions.al:
            // 137-162, record/ProbeClearAllSetKeyHyperlink.al:358-385); the base table's globals are reset below.
            al_ir::ObjKind::TableExtension => o.rec_slot.into_iter().chain(o.xrec_slot).collect(),
            // A page extension keeps Rec, xRec and CurrPage, as a page does; the base page's globals are reset below
            // (`clear_all_base_page`, pages/extensions/CyClearAllExtensions.al:166-193).
            al_ir::ObjKind::PageExtension => o
                .rec_slot
                .into_iter()
                .chain(o.xrec_slot)
                .chain(o.globals.iter().position(|g| g.name == "CurrPage"))
                .collect(),
            al_ir::ObjKind::Page => o
                .rec_slot
                .into_iter()
                .chain(o.page.as_ref().map(|p| p.curr_page_slot))
                .collect(),
            // A report resets every global: the data items' records and their filters, the request page's variables,
            // plain codeunit variables; a SingleInstance codeunit keeps its state. Called in OnInitReport, OnPreReport,
            // OnPreDataItem, OnAfterGetRecord, OnPostReport or a procedure they call (corpus
            // report/lt-clearall-report/ClearAllReport{Init,Pre,Post}Report.al and ...Procedure.al,
            // reports/runtime/P128ClearAllReport.al, reports/runtime/P135ClearAllReport.al).
            al_ir::ObjKind::Report => Vec::new(),
            _ => {
                return Err(Signal::unsupported(format!("ClearAll in a {} (not pinned)", o.kind)));
            }
        };
        let table_like = matches!(o.kind, al_ir::ObjKind::Table | al_ir::ObjKind::TableExtension);
        let page_ext = o.kind == al_ir::ObjKind::PageExtension;
        self.alias_any(f)?;
        let globals = f.instance.borrow().globals.clone();
        let bound = self.clear_all_bound_globals(&globals, &keep, true)?;
        let keep: Vec<usize> = keep.into_iter().chain(bound).collect();
        let fresh = self.slot_defaults(&o.globals);
        let mut inst = f.instance.borrow_mut();
        for (i, v) in fresh.into_iter().enumerate() {
            if !keep.contains(&i) {
                // A global temporary record keeps its rows, as Clear leaves them
                // (types/ProbeTypeFormsAndFilters.al:Clear_TemporaryRecordWithRows_ThenCount); ClearAll does the same: real BC 29
                // keeps both rows of a codeunit's global temporary record and drops its filter, called directly and through
                // Codeunit.Run (record/PriorityAssumptions.al:247-281, ClearAll_TemporaryGlobalRecord_*).
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
        if table_like {
            drop(inst);
            return self.clear_all_other_table_instances(f);
        }
        if page_ext {
            drop(inst);
            return self.clear_all_base_page(f);
        }
        // ClearAll in a page resets the globals of its page extensions too: a flag an extension set in its OnOpenPage reads
        // false in the extension's OnAfterGetRecord once the base page's OnAfterGetRecord called ClearAll, for a base page of the
        // same app and of another (corpus pages/run-modal/CePageExt.al:27-41 ClearAll_InOwnBasePage_ExtensionGlobal, :43-69
        // ClearAll_InBaseAppPage_ExtensionGlobal). The extension's Rec, xRec and CurrPage stay, as the page's own do.
        let engine = self.engine.clone();
        let o = &engine.program.objects[object];
        if o.kind == al_ir::ObjKind::Page {
            let curr = o.page.as_ref().map(|p| p.curr_page_slot);
            let st = curr.and_then(|c| match &inst.globals[c] {
                Value::TestPage(st) => Some(crate::pages::st_of(st).clone()),
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
                    let ext_globals = ext.borrow().globals.clone();
                    self.clear_all_bound_globals(&ext_globals, &keep, false)?;
                    let fresh = self.slot_defaults(&eo.globals);
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

    /// The globals (not in `keep`) that `ClearAll()` leaves pointing at their instance because the instance is bound by
    /// `BindSubscription`. Real BC keeps the bound instance in the variable and its binding, and resets the instance's own
    /// globals (events/binding-senders/BindSubscriptionClearAll.al:7-10, ClearAll_HolderWithBoundGlobal_Binding :76-107 and
    /// ClearAll_TestCodeunitWithBoundGlobal_Binding :148-171; al-runner issue 6974).
    /// Pinned only for a codeunit's global (`in_place`) whose bound instance has no TableNo record and only Text globals
    /// (the probe's subscriber "LT BSK Manual" has one global, `Tag: Text`, BindSubscriptionKinds.Objects.al:44-49).
    /// Everything else is refused: an array of codeunits, a page extension's global, a bound instance with a TableNo
    /// record, or with a global of any other kind (records, temporary records, codeunits, arrays, numbers, ...).
    fn clear_all_bound_globals(
        &mut self,
        globals: &[Value],
        keep: &[usize],
        in_place: bool,
    ) -> Result<Vec<usize>, Signal> {
        let refuse = || {
            Err(Signal::Unsupported(
                "ClearAll of this global codeunit variable whose instance is bound by BindSubscription (not pinned)"
                    .into(),
            ))
        };
        let mut kept = Vec::new();
        for (i, v) in globals.iter().enumerate() {
            if keep.contains(&i) || !self.holds_bound_codeunit(v) {
                continue;
            }
            let Value::Codeunit(rc) = v else { return refuse() };
            if !in_place {
                return refuse();
            }
            let object = rc.borrow().object as usize;
            let engine = self.engine.clone();
            let o = &engine.program.objects[object];
            if o.rec_slot.is_some() {
                return Err(Signal::Unsupported(
                    "ClearAll of a global codeunit variable whose bound instance has a TableNo record (not pinned)"
                        .into(),
                ));
            }
            if o.globals.iter().any(|g| !matches!(g.ty, RtType::Text(_))) {
                return Err(Signal::Unsupported(
                    "ClearAll of a global codeunit variable whose bound instance has a global that is not Text (not pinned)"
                        .into(),
                ));
            }
            let fresh = self.slot_defaults(&o.globals);
            let mut b = rc.borrow_mut();
            for (k, nv) in fresh.into_iter().enumerate() {
                if k < b.globals.len() {
                    b.globals[k] = nv;
                }
            }
            kept.push(i);
        }
        Ok(kept)
    }

    /// The rest of `ClearAll()` in a table or a table extension: the globals of the other instances of the record
    /// variable are reset as well, except their `Rec` and `xRec`. In a table extension the base table's globals go
    /// (pages/extensions/CyClearAllExtensions.al:137-153 `in ext: base glob` 0); in the base table the extension's
    /// globals go (CyClearAllExtensions.al:154-162 `in base: ext glob` 0, record/ProbeClearAllSetKeyHyperlink.al:373-384).
    /// Only one extension with globals is pinned: a second one, or a global temporary record there, is refused.
    fn clear_all_other_table_instances(&mut self, f: &Frame) -> Result<Value, Signal> {
        let own = f.instance.borrow().object as usize;
        let engine = self.engine.clone();
        let objects = &engine.program.objects;
        let Some(Value::Record(rec)) = objects[own]
            .rec_slot
            .and_then(|s| f.instance.borrow().globals.get(s).cloned())
        else {
            return Ok(Value::Null);
        };
        let slots = |o: usize| -> Vec<usize> { objects[o].rec_slot.into_iter().chain(objects[o].xrec_slot).collect() };
        let others: Vec<(usize, Rc<RefCell<Instance>>)> = rec
            .borrow()
            .globals
            .instances()
            .into_iter()
            .filter(|(_, i)| !Rc::ptr_eq(i, &f.instance))
            .map(|(o, i)| (o as usize, i))
            .filter(|(o, _)| (0..objects[*o].globals.len()).any(|g| !slots(*o).contains(&g)))
            .collect();
        let exts = others
            .iter()
            .filter(|(o, _)| objects[*o].kind == al_ir::ObjKind::TableExtension)
            .count();
        let own_ext = objects[own].kind == al_ir::ObjKind::TableExtension;
        if exts > usize::from(!own_ext) {
            return Err(Signal::Unsupported(
                "ClearAll in a table or table extension while a second extension of the table holds globals (not pinned)".into(),
            ));
        }
        for (o, inst) in others {
            let keep = slots(o);
            let fresh = self.slot_defaults(&objects[o].globals);
            let mut i = inst.borrow_mut();
            for (g, v) in fresh.into_iter().enumerate() {
                if keep.contains(&g) {
                    continue;
                }
                if matches!(&i.globals[g], Value::Record(r) if r.borrow().temporary) {
                    return Err(Signal::Unsupported(
                        "ClearAll in a table or table extension whose other part has a global temporary record (not pinned)".into(),
                    ));
                }
                i.globals[g] = v;
            }
        }
        Ok(Value::Null)
    }

    /// `ApplicationArea()` reads the session's areas, `ApplicationArea(Text)` sets them
    /// (corpus pages/application-area/TestPageApplicationAreaControlRemoval.al).
    pub(crate) fn application_area(&mut self, set: Option<Value>) -> Result<Value, Signal> {
        match set {
            None => Ok(Value::Text(self.application_areas.clone().into())),
            Some(Value::Text(t) | Value::Code(t)) => {
                // Tags are `#` and letters or digits, separated by commas (metadata/platform-tables/PlTSession.al:
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
            Some(other) => Err(Signal::unsupported(format!("ApplicationArea({})", other.describe()))),
        }
    }

    /// What `CurrFieldNo` reads on `rec`: the field number a TestPage write is validating on this
    /// record, 0 anywhere else (testpage/TestPageCurrFieldNo_Tests.al:36-123).
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
        // types/TestProbeScalarContracts.al:TextChar_AssignByIndex: position 0 and below raise, a position after
        // the last character reads as the character 0, however far past it is (text/text-methods/Index1.Codeunit.al
        // Index_Read_Variable: `T[7]`, `T[100]`, `E[1]`).
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
                None => Ok(Value::Char(0)),
            };
        }
        // A lone half reads as itself (ADR-068; text/CrTextLeads.al:425-442 through the corpus's `Codes`,
        // text/text-methods/Index1.Codeunit.al:359-373).
        let units: Vec<u16> = t.units();
        match units.get(i as usize - 1) {
            Some(u) => Ok(Value::Char(*u)),
            None => Ok(Value::Char(0)),
        }
    }

    /// The items a `foreach` runs over (a List, a JSON array or an XPath result).
    pub(crate) fn foreach_items(coll: Value) -> Result<Vec<Value>, Signal> {
        Ok(match coll {
            Value::List(l) => l.borrow().to_vec(),
            // A Text or Code visits its UTF-16 code units, one Char each, in order; the text is read once, so an assignment
            // inside the loop changes nothing about the visits (text/text-methods/ForeachText.al:Foreach_Text_Char_Order,
            // Foreach_Text_Char_Sources, Foreach_Text_ModifyDuringLoop; a Text[1] variable: Foreach_Text_Text1_Variable).
            // A lone half is visited as itself (ADR-068; ForeachText.al:Foreach_Text_SurrogatePair).
            Value::Text(t) | Value::Code(t) if !t.has_marker() => t.units().into_iter().map(Value::Char).collect(),
            // The nodes an XPath query found (corpus xml/TestReportFlowFieldColumnAutoCalc.al).
            other => match al_builtins::json::array_items(&other).or_else(|| al_builtins::xml::node_list_items(&other))
            {
                Some(items) => items,
                None => return Err(Signal::unsupported(format!("foreach over {}", other.describe()))),
            },
        })
    }

    /// A binary operator on two evaluated operands.
    #[inline(always)]
    pub(crate) fn binary_op(op: ir::BinOp, l: &Value, r: &Value) -> Result<Value, Signal> {
        use ir::BinOp as B;
        if matches!(l, Value::SqlCount(_)) || matches!(r, Value::SqlCount(_)) {
            return Self::sql_count_op(op, l, r);
        }
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
            B::Eq => Value::Boolean(ops::equal_operator(l, r)?),
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

    /// An operator with a SQL counter value on either side (`al_builtins::sqlcount`).
    #[cold]
    #[inline(never)]
    fn sql_count_op(op: ir::BinOp, l: &Value, r: &Value) -> Result<Value, Signal> {
        use al_builtins::sqlcount::{Cmp, compare};
        use ir::BinOp as B;
        let cmp = match op {
            B::Eq => Cmp::Eq,
            B::Ne => Cmp::Ne,
            B::Lt => Cmp::Lt,
            B::Le => Cmp::Le,
            B::Gt => Cmp::Gt,
            B::Ge => Cmp::Ge,
            B::Add | B::Sub | B::Mul | B::Div | B::IntDiv | B::Mod => {
                let a = match op {
                    B::Add => ops::Arith::Add,
                    B::Sub => ops::Arith::Sub,
                    B::Mul => ops::Arith::Mul,
                    B::Div => ops::Arith::Div,
                    B::IntDiv => ops::Arith::IntDiv,
                    _ => ops::Arith::Mod,
                };
                return al_builtins::sqlcount::arith(a, l, r);
            }
            _ => return Err(al_builtins::sqlcount::refusal("an operator")),
        };
        compare(cmp, l, r).map(Value::Boolean)
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

    /// [`Self::at_pos`] after the check of ADR-068 item 3: a read of a text with half of a surrogate pair by a reader that is
    /// not audited refuses before the next statement runs.
    #[inline]
    pub(crate) fn at_pos_checked(&mut self, p: u32) -> Result<(), Signal> {
        al_builtins::text::check_unaudited()?;
        self.at_pos(p);
        Ok(())
    }

    /// The statement at position `p` begins: the call stack, the profiler and coverage see it.
    #[inline(always)]
    pub(crate) fn at_pos(&mut self, p: u32) {
        self.profile_statement(p);
        if let Some(top) = self.call_stack.last_mut() {
            top.pos = p;
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

    /// An Option, Enum or Boolean bound of a `for` loop as the Integer it stands for. A loop over a Boolean
    /// (`for Stepwise := false to true do`, 27 Microsoft codeunits) runs for false and then for true, the variable
    /// holding the Boolean each round (registry entry for-boolean). Not pinned. Option bounds of an Integer loop take
    /// its type as an assignment does: the ordinal (Microsoft's "Library - Manufacturing" loops
    /// `for Day := FromDay to ToDay` with Option parameters; pinned by the corpus Option_InCaseAndLoopBounds,
    /// types/ for_to_option).
    #[inline(always)]
    pub(crate) fn for_bound(v: Value) -> Value {
        match v {
            Value::Option(o, _) | Value::Enum(_, o) => Value::Integer(o),
            Value::Boolean(b) => Value::Integer(i32::from(b)),
            v => v,
        }
    }

    /// The value the counter variable of a `for` loop holds in the round with counter `i`.
    #[inline(always)]
    pub(crate) fn for_counter(kind: ForKind, i: i64, slot_ty: &RtType) -> Result<Value, Signal> {
        if matches!(slot_ty, RtType::Date) {
            return Ok(date_of_key(i));
        }
        if kind.is_boolean {
            Ok(Value::Boolean(i != 0))
        } else if kind.is_integer
            && let Ok(n) = i32::try_from(i)
        {
            // What the conversion of a BigInteger into an Integer gives, without the Decimal in between; a counter
            // beyond Integer takes the general path, which reports it.
            Ok(Value::Integer(n))
        } else if let (Ok(o), RtType::Option(_) | RtType::Enum(_)) = (i32::try_from(i), slot_ty) {
            // An Option or Enum counter holds the ordinal of each round, a member the type lacks included (an enum with a gap
            // shows Red,Green,2,3,4,Blue), as variable or field, up or downto (types/date-time/
            // ForLoopOptionVariable.al:31-54, :221-237; ForOptionEnum_risky.al:39-104).
            Ok(match slot_ty {
                RtType::Enum(id) => Value::Enum(*id, o),
                _ => ops::convert(Value::Integer(o), slot_ty)?,
            })
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
        let mut lagged = None;
        let inst = match instance {
            None => f.instance.clone(),
            Some(v) => match v {
                // A procedure of a reportextension called through the report variable runs on the extension's
                // instance of that variable, which keeps the extension's globals and added data items
                // (reports/extensions/W3RpExtProcedures.al:ExtensionProcedure_*).
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
                    // The caller's xRec the procedure shares (`table_proc_xrec`: the page's or the field trigger's), else none.
                    let caller_xrec = self.table_proc_xrec(object, &rc, f);
                    let handle = Rc::as_ptr(&rc) as usize;
                    let (inst, saved) = self.enter_record(object, rc, caller_xrec.as_ref())?;
                    // A plain call: the procedure's xRec is the blank record `enter_record` made (`plain_proc_xrec`).
                    let plain = caller_xrec.is_none().then(|| {
                        let slot = self.engine.program.objects[object].xrec_slot?;
                        match &inst.borrow().globals[slot] {
                            Value::Record(x) => Some(x.clone()),
                            _ => None,
                        }
                    });
                    let pushed = if let Some(Some(x)) = plain {
                        self.plain_proc_xrec.push((handle, self.call_stack.len() + 1, x));
                        true
                    } else {
                        false
                    };
                    let r = self.finish_call(object, routine, inst.clone(), args, values, try_call, f);
                    if pushed {
                        self.plain_proc_xrec.pop();
                    }
                    Self::leave_record(&inst, saved);
                    return r;
                }
                // A query procedure runs with the query it was called on as its own state.
                Value::Query(q) => self.query_instance(object, q),
                // The page a part shows (`CurrPage.<Part>.Page`), or a `Page` variable, which
                // is constructed at first use.
                Value::TestPage(st) => {
                    let st = crate::pages::into_st(st);
                    let i = self.construct_page(&st)?;
                    self.note_outside_page_call(&st, &f.instance);
                    lagged = self.enter_server_row(&st);
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
                    return Err(Signal::unsupported(format!(
                        "a procedure call on a {} value",
                        value_kind(&other)
                    )));
                }
            },
        };
        let result = self.finish_call(object, routine, inst, args, values, try_call, f);
        if let Some((rec, now)) = lagged {
            rec.borrow_mut().fields = now;
        }
        result
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
        // (events/binding-senders/EventVarThroughSender.al): they alias it, see `Frame::aliases`.
        let facts = self.engine.routine_facts(object, routine);
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
            // The global a `var` parameter of this frame stands for gets the parameter's value: a write (ADR-066
            // amendment 3, R1).
            self.single_wrote(inst, *g);
            inst.borrow_mut().globals[*g] = f.locals[*slot].clone();
            self.single_written(inst, *g);
        }
        // While the call runs, the global is the current value of a parameter of this frame that aliases it.
        let own_aliases = self.set_alias_live(f, false);
        // Only a routine that reads MaxStrLen of a `var` parameter, or hands one on, gets them (known at lowering), so other
        // calls pay nothing. An event's subscribers get the lengths of the publisher's arguments (`publish`).
        let var_lens = facts.var_lens;
        if var_lens {
            self.events.next_var_lens = self.var_lens_of(object, routine, args, f);
        } else if facts.publisher {
            self.events.publisher_var_lens = self.var_lens_of(object, routine, args, f);
        }
        // A scalar global of the caller passed by `var` is watched while the call runs (`alias_access`); to a procedure of
        // the same instance the parameter aliases it. Not for an event publisher: there the subscribers alias the variable,
        // which is pinned (above).
        let watched = if facts.publisher {
            0
        } else {
            self.watch_var_globals(object, routine, &inst, args, &values, f)
        };
        if try_call {
            self.enter_used_try();
        }
        let result = self.call_with_facts(object, routine, inst, values, facts);
        if try_call {
            self.leave_used_try();
        }
        if watched > 0 {
            let n = self.var_alias_watch.len() - watched;
            self.var_alias_watch.truncate(n);
            // A call refused before its frame was made leaves them behind.
            self.events.next_aliases.clear();
        }
        if var_lens {
            self.events.next_var_lens.clear();
        } else if facts.publisher {
            self.events.publisher_var_lens.clear();
        }
        if facts.publisher {
            // A publisher call refused before `publish` took them leaves them behind; the next event must not alias this
            // caller's globals (al-runner issue 7615).
            self.events.var_origins.clear();
        }
        for (slot, inst, g) in &f.aliases {
            f.locals[*slot] = inst.borrow().globals[*g].clone();
        }
        if own_aliases {
            self.set_alias_live(f, true);
        }
        if let Err(Signal::Error(_)) = &result {
            if let Some(finals) = self.error_finals.take() {
                for (a, v) in args.iter().zip(finals) {
                    if let ir::Arg::Ref(p) = a {
                        self.write_back(p, v, f)?;
                    }
                }
            }
        }
        let (ret, finals) = match (result, try_call) {
            // A write the try was not allowed to make is not trapped by it (errors/CuTryTests.al:83-150).
            (Err(Signal::Error(e)), true) if e.code == crate::transactions::TRY_FUNCTION_WRITE => {
                return Err(Signal::Error(e));
            }
            (Err(Signal::Error(mut e)), true) => {
                self.add_top_frame(&mut e);
                self.catch_error(*e);
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
    /// To a procedure of the caller's own instance the parameter and the global are one variable
    /// (codeunits/procedures/VarGlobalAlias.al:12-118, tests :120-170): the callee's frame aliases the global (`Frame::aliases`),
    /// also when the argument is a parameter of the caller that aliases it. To another instance only the copy is made, and
    /// reaching the global by its name while the call runs is refused (not pinned).
    fn watch_var_globals(
        &mut self,
        object: usize,
        routine: usize,
        callee: &Rc<RefCell<Instance>>,
        args: &[ir::Arg],
        values: &[Value],
        f: &Frame,
    ) -> usize {
        let mut added = 0;
        let same = Rc::ptr_eq(callee, &f.instance);
        for (i, a) in args.iter().enumerate() {
            // Only an argument that names a variable can be watched; a value needs no look at the parameter.
            if matches!(a, ir::Arg::Value(_)) {
                continue;
            }
            let by_var = self.engine.program.objects[object].routines[routine]
                .by_var
                .get(i)
                .copied()
                .unwrap_or(false);
            // A subscriber's parameter that is a global of the publisher's instance (`dispatch`), handed on by `var`: to a
            // procedure of the subscriber's own instance it stays that global, at any depth (events/binding-senders/
            // EventVarThroughSender.al:284-306, modes 2-5 and 7; :308-330). To another instance it is not measured: reaching the
            // global by its name while the call runs is refused.
            if let ir::Arg::Ref(ir::Place::Local(slot)) = a {
                if let Some((_, owner, g)) = f
                    .aliases
                    .iter()
                    .find(|(s, inst, _)| s == slot && !Rc::ptr_eq(inst, &f.instance))
                {
                    if by_var && values.get(i).is_some_and(is_alias_scalar) {
                        self.var_alias_watch.push(crate::AliasWatch {
                            instance: owner.clone(),
                            global: *g,
                            live: if same { Some(false) } else { None },
                        });
                        if same {
                            self.events.next_aliases.push((i, owner.clone(), *g));
                        }
                        added += 1;
                    }
                    continue;
                }
            }
            let g = match a {
                ir::Arg::Ref(ir::Place::Global(g)) => *g,
                ir::Arg::Ref(ir::Place::Local(slot)) => match own_alias_of_slot(f, *slot) {
                    Some(g) => g,
                    None => continue,
                },
                _ => continue,
            };
            if by_var && values.get(i).is_some_and(is_alias_scalar) {
                if same {
                    self.var_alias_watch.push(crate::AliasWatch {
                        instance: f.instance.clone(),
                        global: g,
                        live: Some(true),
                    });
                    self.events.next_aliases.push((i, f.instance.clone(), g));
                } else {
                    self.var_alias_watch.push(crate::AliasWatch {
                        instance: f.instance.clone(),
                        global: g,
                        live: None,
                    });
                }
                added += 1;
            }
        }
        added
    }

    /// Marks the watch entries of the frame's own aliased globals live (the frame's parameter holds the value) or not (a call
    /// the frame made runs, and the global holds it). Returns whether the frame has any.
    fn set_alias_live(&mut self, f: &Frame, live: bool) -> bool {
        if self.var_alias_watch.is_empty() {
            return false;
        }
        let mut any = false;
        for (_, inst, g) in &f.aliases {
            if !Rc::ptr_eq(inst, &f.instance) {
                continue;
            }
            if let Some(w) = self
                .var_alias_watch
                .iter_mut()
                .rev()
                .find(|w| w.global == *g && w.live.is_some() && Rc::ptr_eq(&w.instance, inst))
            {
                w.live = Some(live);
                any = true;
            }
        }
        any
    }

    /// A read of global `i` of the frame's instance: the alias watch, then the single-instance notes.
    #[inline]
    pub(crate) fn global_read(&mut self, i: usize, f: &Frame) -> Result<(), Signal> {
        self.alias_access(i, f)?;
        self.single_read(&f.instance, i);
        Ok(())
    }

    /// Before a read or a write of global `i` of the frame's instance by its own name while a procedure that got the same
    /// global by `var` runs. In the frame of that procedure, when it is of the same instance, the parameter and the global
    /// are one variable (codeunits/procedures/VarGlobalAlias.al:96-110, :160-169): the parameter's value goes to the
    /// global first. Refused: a procedure of another instance got it (not pinned), or code that the procedure did not call
    /// (a trigger a builtin runs) reaches it while the parameter holds the value.
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
        for w in &self.var_alias_watch {
            if w.global != i || !Rc::ptr_eq(&w.instance, &f.instance) {
                continue;
            }
            match w.live {
                Some(false) => {}
                Some(true) => match own_alias_of_global(f, i) {
                    Some(slot) => {
                        f.instance.borrow_mut().globals[i] = f.locals[slot].clone();
                    }
                    None => return Err(self.alias_refusal(i, f, "from code the procedure did not call")),
                },
                None => {
                    return Err(self.alias_refusal(
                        i,
                        f,
                        "(whether the parameter and the global are one variable is not pinned)",
                    ));
                }
            }
        }
        Ok(())
    }

    /// After a write of global `i` by its own name: in the frame of a procedure whose parameter aliases it, the parameter
    /// takes the value (VarGlobalAlias.al:96-102).
    #[inline]
    pub(crate) fn alias_wrote(&self, i: usize, f: &mut Frame) {
        if self.var_alias_watch.is_empty() {
            return;
        }
        if let Some(slot) = own_alias_of_global(f, i) {
            let v = f.instance.borrow().globals[i].clone();
            f.locals[slot] = v;
        }
    }

    fn alias_refusal(&self, i: usize, f: &Frame, why: &str) -> Signal {
        let name = &self.engine.program.objects[f.instance.borrow().object as usize].globals[i].name;
        Signal::unsupported(format!(
            "the global variable {name} is read or changed by its name while a procedure it was passed to by var runs {why}"
        ))
    }

    /// Whether any global of the frame's instance is watched (for ClearAll, which resets them all: not pinned for a
    /// global a running procedure got by var).
    pub(crate) fn alias_any(&self, f: &Frame) -> Result<(), Signal> {
        match self
            .var_alias_watch
            .iter()
            .find(|w| Rc::ptr_eq(&w.instance, &f.instance))
        {
            Some(w) => Err(self.alias_refusal(w.global, f, "(ClearAll of it is not pinned)")),
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
        // A `var Text` parameter can stand for a variable of a declared length.
        let v = self.rendered_into_place(p, v, f)?;
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
            self.single_made(object, &inst);
            self.singletons.insert(object, inst);
        } else if !self.singles_unclaimed.is_empty() {
            self.claim_carried_single(object);
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

    /// The instance of page extension `ext` on the page whose extension runs frame `f`: reached through the page's instance and
    /// its `CurrPage` (the open page holds every extension's instance).
    ///
    /// In a table extension: the instance of table extension `ext` for the record the frame runs on, created when that
    /// extension has not run on the record yet (al-runner issue 6485; corpus writer Y20, Y20TableExtSiblingGlobals.al: one
    /// variable per record variable, shared by both extensions).
    pub(crate) fn sibling_instance(&mut self, f: &Frame, ext: usize) -> Result<Rc<RefCell<Instance>>, Signal> {
        let own = f.instance.borrow().object as usize;
        if self.engine.program.objects[own].kind == al_ir::ObjKind::TableExtension {
            let rec = self.engine.program.objects[own]
                .rec_slot
                .and_then(|slot| match f.instance.borrow().globals.get(slot) {
                    Some(Value::Record(r)) => Some(r.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    Signal::Unsupported(
                        "internal error: table extension code named a variable of another extension of its table, but \
                         its instance holds no record"
                            .into(),
                    )
                })?;
            let existing = rec.borrow().globals.instance(ext as u32);
            return Ok(match existing {
                Some(i) => i,
                None => {
                    let i = self.table_instance(ext);
                    rec.borrow().globals.set_instance(ext as u32, i.clone());
                    i
                }
            });
        }
        let base = self.base_instance(f)?;
        let object = base.borrow().object as usize;
        let page = self.engine.program.objects[object].page.as_ref();
        let st = page.and_then(|p| match base.borrow().globals.get(p.curr_page_slot) {
            Some(Value::TestPage(st)) => Some(crate::pages::st_of(st).clone()),
            _ => None,
        });
        st.and_then(|st| {
            st.borrow()
                .ext_instances
                .iter()
                .find(|(o, _)| *o == ext)
                .map(|(_, i)| i.clone())
        })
        .ok_or_else(|| {
            Signal::Unsupported(
                "internal error: extension code named a variable of another extension of its page, but the page has no live \
                 instance of that extension"
                    .into(),
            )
        })
    }

    /// The instance of table or table extension `object` on the record in place `base` (`Other.<protected var>` in a table
    /// extension, al-runner issue 6984): the one the record's calls run on, made and kept in the record when it has none yet
    /// (ADR-021 per-record instances; corpus PR 1126, table-objects/extensions/ProtectedVarCrossApp.al).
    fn record_instance(
        &mut self,
        base: &ir::Place,
        object: usize,
        f: &mut Frame,
    ) -> Result<Rc<RefCell<Instance>>, Signal> {
        let Value::Record(rec) = self.read(base, f)? else {
            return Err(Signal::Unsupported(
                "internal error: a protected variable named through a record variable that holds no record".into(),
            ));
        };
        let existing = rec.borrow().globals.instance(object as u32);
        Ok(match existing {
            Some(i) => i,
            None => {
                let i = self.table_instance(object);
                rec.borrow().globals.set_instance(object as u32, i.clone());
                i
            }
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
    /// (text/text-methods/MaxStrLen.Codeunit.al:91-151, LT_BU_MaxStrLen_TextParameters), `i32::MAX` for one without a
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
            ir::Place::ExtGlobal(e, i) => self.engine.program.objects[*e].globals[*i].ty.clone(),
            ir::Place::RecGlobal(_, o, i) => self.engine.program.objects[*o].globals[*i].ty.clone(),
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
                // Halves that meet merge (ADR-068; text/text-methods/ForeachText.al:137-148).
                t.push_txt(x);
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
        self.read_place(p, false, f)
    }

    /// The value of the place `p`. `value` is true only where an expression reads it as a value (`Op::ReadPlace`); it lets the
    /// index of an array element convert as a value read does (`element`). An element taken by reference (a `var` argument, the
    /// record whose field is written) reads with `value` false.
    pub(crate) fn read_place(&mut self, p: &ir::Place, value: bool, f: &mut Frame) -> Result<Value, Signal> {
        match p {
            ir::Place::Local(i) => Ok(f.locals[*i].clone()),
            ir::Place::Global(i) => {
                self.alias_access(*i, f)?;
                self.single_read(&f.instance, *i);
                Ok(f.instance.borrow().globals[*i].clone())
            }
            ir::Place::BaseGlobal(i) => {
                let base = self.base_instance(f)?;
                let v = base.borrow().globals.get(*i).cloned();
                v.ok_or_else(|| base_slot_missing(*i))
            }
            ir::Place::ExtGlobal(e, i) => {
                let ext = self.sibling_instance(f, *e)?;
                let v = ext.borrow().globals.get(*i).cloned();
                v.ok_or_else(|| base_slot_missing(*i))
            }
            ir::Place::RecGlobal(base, o, i) => {
                let inst = self.record_instance(base, *o, f)?;
                let v = inst.borrow().globals.get(*i).cloned();
                v.ok_or_else(|| base_slot_missing(*i))
            }
            ir::Place::Index(base, idx) => {
                let (arr, flat) = match self.element(base, idx, value, f) {
                    Ok(e) => e,
                    // A row of a two-dimensional array, as an argument.
                    Err(Signal::Unsupported(m)) if m.starts_with(ROW_REFUSAL) => {
                        return match self.row_range(base, idx, f)? {
                            Some((arr, start, len)) => Ok(Value::Array(Rc::new(RefCell::new(
                                arr.borrow()[start..start + len].to_vec(),
                            )))),
                            None => Err(Signal::unsupported(m)),
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
                if let Some(why) = rc
                    .borrow()
                    .unpinned_buffer
                    .filter(|w| !crate::records::run_page_link_fields_read(w) && *w != crate::records::HELD_PART_REC)
                {
                    if why == crate::records::RUN_PAGE_LINK_OPEN {
                        return Err(Signal::unsupported(format!("a read of a field of Rec {why}")));
                    }
                    return Err(Signal::Unsupported(why.into()));
                }
                // The buffer of a table element record after an export under SourceTableView is the last row the export read
                // (QrXpContainerOccurs.al TS_Again_SameData on branch al-runtime-rs-staging-pub-ts).
                // Pinned by xmlports/shapes-properties/QrXpContainerOccurs.al TS_Again_SameData, TS_Again_AssignBetween, TS_Again_TransferFieldsBetween, TS_Again_InitBetween, TS_Again_KeyBetween, TS_Again_LineFilter, TS_View_SecondExport, TS_Again_TrBooking, TS_Again_TrMember (was assumption `xmlport-export-again`).
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
                            return Err(Signal::unsupported(format!(
                                "a read of the field whose InitValue BC cannot evaluate (`{message}`), in a record where another field was written (not pinned)"
                            )));
                        }
                    }
                }
                let v = r.fields[*pos].clone();
                if let Value::Unknown(why) = &v {
                    return Err(Signal::unsupported(why.to_string()));
                }
                if let Some(why) = self.install_row_order(r.table, r.temporary, *pos, &r.fields) {
                    return Err(Signal::unsupported(why));
                }
                if r.load.partial {
                    // A Blob counts as loaded (partial.rs `AreFieldsLoaded`), and a FlowField is no column of the row: their
                    // reads load nothing else (P81LoadFields.al:664-689 CalcFields_OnNotLoadedFlowAndBlob `loaded_qty` No).
                    let stored = self.engine.program.objects[r.table as usize]
                        .table
                        .as_ref()
                        .and_then(|s| s.fields.get(*pos))
                        .is_some_and(|f| f.stored);
                    let missing = stored
                        && r.load.buffer.as_ref().is_some_and(|b| !b.have.contains(*pos))
                        && !matches!(r.fields[*pos], Value::Blob(_));
                    drop(r);
                    if missing && !rc.borrow().temporary {
                        // A read of a field that is not loaded loads every missing field from the row (partial.rs).
                        self.load_missing_fields(&rc, Some(*pos))?;
                        let v = rc.borrow().fields[*pos].clone();
                        if let Value::Unknown(why) = &v {
                            return Err(Signal::unsupported(why.to_string()));
                        }
                        return Ok(v);
                    }
                    rc.borrow_mut().load.touch(*pos);
                }
                Ok(v)
            }
            other => Err(Signal::unsupported(format!("field of {}", other.describe()))),
        }
    }

    pub(crate) fn write(&mut self, p: &ir::Place, v: Value, f: &mut Frame) -> Result<(), Signal> {
        self.write_as(p, v, f, true)
    }

    /// The write-back of a builtin's by-reference receiver or argument (`run_native`): a Blob it hands back is stored as the
    /// buffer it is, never copied (a stream `CreateOutStream` created on it writes there; `blob_transfer::moved`).
    fn write_native_back(&mut self, p: &ir::Place, v: Value, f: &mut Frame) -> Result<(), Signal> {
        self.write_as(p, v, f, false)
    }

    /// [`Self::write`]; `assign` is false for a builtin's write-back.
    fn write_as(&mut self, p: &ir::Place, v: Value, f: &mut Frame, assign: bool) -> Result<(), Signal> {
        // A column a platform row leaves unknown never reaches a variable: every read of one is refused where it happens; this
        // catches a reader that missed it.
        if let Value::Unknown(why) = &v {
            return Err(Signal::unsupported(why.to_string()));
        }
        // A counter value in a field would reach the database and every reader of the row (`al_builtins::sqlcount`).
        if let (Value::SqlCount(_), ir::Place::Field(..)) = (&v, p) {
            return Err(al_builtins::sqlcount::refusal("storing in a record field"));
        }
        // A placeholder's size in a field would be stored as a number (`al_builtins::placeholder_output::size`).
        if let (Value::PlaceholderSize(m), ir::Place::Field(..)) = (&v, p) {
            return Err(al_builtins::placeholder_output::size_refusal(
                m,
                "storing in a record field",
            ));
        }
        if self.sql.active && matches!(p, ir::Place::Field(..)) && al_builtins::sqlcount::formatted(&v) {
            return Err(al_builtins::sqlcount::refusal(
                "storing the formatted value in a record field",
            ));
        }
        // `:=` between FilterPageBuilders copies the control collection and shares the records
        // (corpus pages/filterpagebuilder/TestFilterPageBuilder.al:FilterPageBuilder_Assignment_*).
        let v = match v {
            Value::Library(al_builtins::library_kind::LibraryKind::FilterPageBuilder, cell) => Value::Library(
                al_builtins::library_kind::LibraryKind::FilterPageBuilder,
                filterpage::copy_for_assignment(&cell),
            ),
            v => v,
        };
        // `:=` between records copies the fields into the target record (ADR-010); the target keeps its own SecurityFiltering
        // (al-language/reports/runtime/ACAttributes.al:379-384).
        if let Value::Record(src) = &v {
            // A Variant that holds a record takes the new value whole, whatever table it comes from: it is no record variable
            // (Library - Variable Storage.Dequeue into one `RecVar` that held a Contact, then a Segment Header; Tests-Marketing).
            // A Variant holding another record is replaced (by the by-value copy below), so it holds the new record whole, as a
            // Variant that held nothing would: a RecordRef.GetTable of it takes the new filters, also with a temporary record on
            // either side (al-runner issue 6503; recordref-fieldref/recordref/Y23VariantRecordView.al, Y23_SecondRecordTemporary
            // :289-303). A Variant given the very record it holds keeps it, and with it the table a Variant parameter was bound
            // to (a temporary record's rows): replacing that gave the callee an empty table (Find Record
            // Management.GetLastEntryIntFieldValue, called from "Price Asset".OnInsert; Tests-ERM Price Calculation - V16).
            let replaces_variant = matches!(self.place_type(p, f), RtType::Variant)
                && match self.read(p, f) {
                    Ok(Value::Record(d)) => !Rc::ptr_eq(&d, src),
                    _ => false,
                };
            if !matches!(p, ir::Place::Field(..)) && !replaces_variant {
                // `:=` into a global record is a write in place: it replaces what an earlier test codeunit left in a
                // SingleInstance global, it reads nothing of it (ADR-066 amendment 3, R1).
                let dst = match p {
                    ir::Place::Global(i) => {
                        self.alias_access(*i, f)?;
                        Ok(f.instance.borrow().globals[*i].clone())
                    }
                    _ => self.read(p, f),
                };
                if let Ok(Value::Record(dst)) = dst {
                    // A record variable takes no record of another table; that only happens through a Variant
                    // (types/variant/VariantRecordReassign.al:130 Variant_Reassign_ReadBack).
                    if dst.borrow().table != src.borrow().table && !matches!(self.place_type(p, f), RtType::Variant) {
                        return Err(Signal::Error(Box::new(AlError {
                            message: "The requested operation is not supported.".into(),
                            code: "NotSupportedOperation".into(),
                            call_stack: String::new(),
                            ..Default::default()
                        })));
                    }
                    if !Rc::ptr_eq(src, &dst) {
                        if let ir::Place::Global(i) = p {
                            self.single_replaced(&f.instance, *i);
                        }
                        // The source's buffer after an export under SourceTableView is not pinned (records.rs
                        // `XP_VIEW_AFTER_EXPORT`).
                        if let Some(why) = crate::records::xp_export_marked(src) {
                            return Err(Signal::unsupported(format!("an assignment from a record: {why}")));
                        }
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
                                            // empty (record/PnABlobAssignRename.al:Assign_Stored `copy-content-notloaded`).
                                            loaded: b.loaded,
                                            fetched: b.fetched,
                                            refetch: true,
                                            persisted: false,
                                            temp_row: b.temp_row,
                                            uncompressed: b.uncompressed,
                                            // An unsaved stream write comes over as unsaved: CalcFields keeps it on a stored row
                                            // (`Session::load_blobs`, al-runner issue 7360).
                                            written: b.written,
                                            written_base: b.written_base.clone(),
                                            render_pending: b.render_pending.clone(),
                                            inserted: b.inserted,
                                            temp_inserted: false,
                                            // A shorter write into a copy of an unloaded Blob is not pinned (`UNLOADED_WRITE`).
                                            db_row: false,
                                            first_write_truncates: false,
                                            file: false,
                                            dotnet_view: 0,
                                            unread_content: false,
                                            assigned_unsaved: false,
                                            utf16_binary: b.utf16_binary,
                                            unicode_read: None,
                                        },
                                    )))
                                }
                                other => other.clone(),
                            })
                            .collect();
                        let load = src.borrow().load.clone();
                        // `:=` copies what Record.IsDirty compares with (record/ACRecordIsDirty.al:63-64 `copy_dirty`).
                        let dirty_base = src.borrow().is_dirty_base.copy();
                        // The row version comes over with the fields, and where it came from (record/crud/
                        // Y76StaleBuffers.al Y76_Part1_FindSet_Assign, `assign/*`).
                        let row_origin = src.borrow().row_origin;
                        let mut d = dst.borrow_mut();
                        d.fields = fields;
                        d.row_origin = row_origin;
                        d.own_read = Default::default();
                        d.probed_source = crate::blob_transfer::assigned(&d, &src.borrow(), self.commits);
                        d.is_dirty_base = dirty_base;
                        // The copy holds what the source's buffer held, if that is known.
                        d.load.buffer = load.buffer;
                        d.load.partial = load.partial;
                    }
                    if let ir::Place::Global(i) = p {
                        self.single_written(&f.instance, *i);
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
                    if let Some(why) = r.unpinned_buffer {
                        crate::records::held_part_field_write(why)?;
                    }
                    let how = if assign { BlobMove::Assign } else { BlobMove::WriteBack };
                    let v =
                        crate::blob_transfer::moved(Some(&r.fields[*pos]), v, how, Holder::Other, || FieldKind::Plain)?;
                    r.before_field_write();
                    r.fields[*pos] = v;
                    r.touched = true;
                    r.assigned = true;
                    if r.load.partial {
                        r.load.written(*pos);
                    }
                    Ok(())
                }
                other => Err(Signal::unsupported(format!("field of {}", other.describe()))),
            };
        }
        // A record assigned to a Variant is a by-value copy: a temporary record's copy is an empty table of its own
        // (recordref-fieldref/recordref/PWRecordMisc.al:TempRecord_AssignedToVariantVariable and :TempRecord_VariantModifiedAfterAssign).
        let v = match &v {
            Value::Record(rc) if matches!(self.place_type(p, f), RtType::Variant) => {
                let copy = self.record_by_value_unmeasured(rc);
                // An assignment to a Variant the probes measured moves the copy's Blobs as `Session::blobs_by_value` says.
                if let (true, Some(taker)) = (assign, crate::blob_transfer::Taker::variant_at(p)) {
                    let (temporary, holder) = {
                        let r = rc.borrow();
                        (r.temporary, self.holder(&r))
                    };
                    self.blobs_by_value(copy, temporary, holder, taker, crate::blob_transfer::Call::Plain)?
                } else {
                    copy
                }
            }
            // A RecordRef assigned to a Variant or to another RecordRef: a RecordRef of its own on the same buffer
            // (`recordref_variant.rs`). A builtin's write-back of the receiver keeps its handle.
            Value::Library(al_builtins::library_kind::LibraryKind::RecordRef, h)
                if match self.place_type(p, f) {
                    RtType::Variant => true,
                    RtType::Library(al_builtins::library_kind::LibraryKind::RecordRef) => assign,
                    _ => false,
                } =>
            {
                if matches!(self.place_type(p, f), RtType::Variant) {
                    self.variant_refs.variant_copy(h)
                } else {
                    self.variant_refs.copy_typed_alias(h)
                }
            }
            // A Blob assigned to a local Variant: the Variant holds a copy (`blob_transfer::moved`), whatever field or holder
            // the Blob came from. A field read stored in another Variant is moved before it comes here
            // (`blob_transfer::FieldRead`, `Op::BlobFieldRead`).
            Value::Blob(_)
                if assign && matches!(p, ir::Place::Local(_)) && matches!(self.place_type(p, f), RtType::Variant) =>
            {
                let how = BlobMove::Variant;
                crate::blob_transfer::moved(None, v, how, Holder::Other, || FieldKind::Plain)?
            }
            _ => v,
        };
        match p {
            ir::Place::Local(i) => f.locals[*i] = v,
            ir::Place::Global(i) => {
                self.alias_access(*i, f)?;
                // A builtin's write-back of the value the global holds is no write (`D.ContainsKey(K)` on a global).
                let noted = if assign {
                    self.single_wrote(&f.instance, *i);
                    true
                } else {
                    self.single_wrote_back(&f.instance, *i, &v)
                };
                f.instance.borrow_mut().globals[*i] = v;
                if noted {
                    self.single_written(&f.instance, *i);
                }
                self.alias_wrote(*i, f);
            }
            ir::Place::BaseGlobal(i) => {
                let base = self.base_instance(f)?;
                let mut b = base.borrow_mut();
                match b.globals.get_mut(*i) {
                    Some(slot) => *slot = v,
                    None => return Err(base_slot_missing(*i)),
                }
            }
            ir::Place::ExtGlobal(e, i) => {
                let ext = self.sibling_instance(f, *e)?;
                let mut b = ext.borrow_mut();
                match b.globals.get_mut(*i) {
                    Some(slot) => *slot = v,
                    None => return Err(base_slot_missing(*i)),
                }
            }
            ir::Place::RecGlobal(base, o, i) => {
                let inst = self.record_instance(base, *o, f)?;
                let mut b = inst.borrow_mut();
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
                let (arr, flat) = match self.element(base, idx, false, f) {
                    Ok(e) => e,
                    // The row a callee wrote through a `var` array parameter goes back into the row.
                    Err(Signal::Unsupported(m)) if m.starts_with(ROW_REFUSAL) => {
                        let (Some((arr, start, len)), Value::Array(row)) = (self.row_range(base, idx, f)?, &v) else {
                            return Err(Signal::unsupported(m));
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
    /// the declared length (testpage/TestPageFieldFormats_Tests.al:58-66,
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
        // (al-language-onprem/types/ByteFormat.al:13-31 `_text1_code`, reports/dataset/
        // ReportDatasetHeaderRaw.al:Lines_Window1, the byte order mark read byte by byte); a byte whose character is not
        // pinned is refused.
        let unit = match &v {
            Value::Byte(b) if *b >= 0x80 => {
                let enc = al_builtins::stream::Encoding::MsDos;
                if !enc.decode_pinned(&[*b]) {
                    return Err(Signal::unsupported(format!(
                        "assigning Byte {b} to a character of a text (its character is not pinned)"
                    )));
                }
                enc.decode(&[*b]).encode_utf16().next()
            }
            // Half of a surrogate pair is stored as it is (ADR-068; types/text-code-char/TextCharIndexAssignEdges.al:100-118
            // `c55296_highSurrogate`, `c56320_lowSurrogate`, `c57343`).
            _ => match int(&v) {
                Ok(c) => u16::try_from(c).ok(),
                Err(_) => None,
            },
        };
        // A Text of ASCII only gets an ASCII character in place (a loop over a long text stays linear): the variable lets
        // go of its hold first so that nothing is copied.
        if let (false, Some(unit), true) = (is_code, unit.filter(|u| (1..128).contains(u)), i >= 1) {
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
        if s.has_marker() {
            Self::char_write_into_marked(s.audited_str(), i)?;
        }
        let mut units: Vec<u16> = s.units();
        let Some(unit) = unit else {
            return Err(Signal::unsupported(format!(
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
        if unit == 0 {
            // The character 0 ends the text there: `T[2] := 0` on 'abc' gives 'a' of length 1
            // (types/text-code-char/TextCharIndexAssignEdges.al:92,108 `c0`). A Code is not pinned.
            if is_code {
                return Err(Signal::Unsupported(
                    "assigning the character 0 to a character of a Code (not pinned)".into(),
                ));
            }
            units.truncate(i);
        } else if i == units.len() {
            units.push(unit);
        } else {
            units[i] = unit;
        }
        if let Some(m) = max.filter(|m| units.len() > *m as usize) {
            if String::from_utf16(&units).is_err() {
                return Err(Signal::Unsupported(
                    "the length error of a text that holds half of a surrogate pair (not pinned)".into(),
                ));
            }
            return al_builtins::native::raise(
                format!(
                    "The length of the string is {}, but it must be less than or equal to {m} characters. Value: {}",
                    units.len(),
                    ops::shown_in_length_error(&String::from_utf16_lossy(&units))
                ),
                "StringLengthExceeded",
            );
        }
        let text = al_builtins::text::Txt::from_units(&units);
        if is_code {
            if text.has_lone() {
                return Err(Signal::Unsupported(
                    "a Code that holds half of a surrogate pair (not pinned)".into(),
                ));
            }
            let text = text.audited_str();
            // A blank written at the last position is cut off like any trailing blank of a Code value: `C[3] := ' '` on 'abc'
            // gives `AB` of length 2 (text/text-methods/Index1.Codeunit.al LT_BU_Index_Write_Code `code blank last`).
            let text = text.trim_end_matches(' ');
            return self.write(base, Value::Code(ops::upper_code_checked(text)?.into()), f);
        }
        self.write(base, Value::Text(text), f)
    }

    /// `T[i] := c` on the text `t` that holds a marker (`al_builtins::native::TEXT_MARKS`), `i` counted from 1. Rendered content
    /// refuses at every position (ADR-040 amendment 2026-11-12, A3b `T[i]` write). Another marker stands for characters that
    /// are not known, so only a position inside the known characters before the first marker is a known position: that write
    /// goes on as for a known text. From the marker on the position is not known, and below 1 the range error would name a
    /// length that is not known: both refuse.
    fn char_write_into_marked(t: &str, i: i64) -> Result<(), Signal> {
        if al_builtins::rendered_text::holds(t) {
            return Err(al_builtins::rendered_text::refusal(
                t,
                "assigning a character of the text (T[i] := c)",
            ));
        }
        let known: usize = t
            .chars()
            .take_while(|c| !al_builtins::native::TEXT_MARKS.contains(c))
            .map(char::len_utf16)
            .sum();
        if usize::try_from(i).is_ok_and(|i| (1..=known).contains(&i)) {
            return Ok(());
        }
        Err(Signal::unsupported(
            "assigning a character of a text at or after characters this runtime does not know, or outside the text \
             (the position and the length are not known)"
                .to_string(),
        ))
    }

    /// A text that holds rendered content, on its way into the variable `p` outside an assignment (the write-back of a
    /// `var` argument of a builtin or a procedure, the loop variable of a `foreach`): it takes the conversion an assignment takes, which lets
    /// it into an unbounded Text and a Variant only (`ops::convert`; ADR-040 amendment 2026-11-12, A5, loss mode L3). An array
    /// (the destination of `CopyArray`, a `var` array parameter) gives each element that holds rendered content the conversion
    /// to its element type (loss mode L23). Any other value is given back as it is.
    pub(crate) fn rendered_into_place(&mut self, p: &ir::Place, v: Value, f: &mut Frame) -> Result<Value, Signal> {
        let rendered = |v: &Value| matches!(v, Value::Text(t) | Value::Code(t) if t.has_marker() && al_builtins::rendered_text::holds(t.audited_str()));
        match &v {
            v2 if rendered(v2) => {
                let ty = self.place_type(p, f);
                ops::convert(v, &ty)
            }
            Value::Array(cells) if cells.borrow().iter().any(rendered) => {
                let RtType::Array(_, el) = self.place_type(p, f) else {
                    return Err(Signal::unsupported(
                        "an array that holds rendered content written into a place that is no array".to_string(),
                    ));
                };
                for cell in cells.borrow_mut().iter_mut().filter(|c| rendered(c)) {
                    *cell = ops::convert(cell.clone(), &el)?;
                }
                Ok(v)
            }
            _ => Ok(v),
        }
    }

    /// The cells of row `idx` of the two-dimensional array behind `base`, as (cells, first position, length): `None` for an
    /// array of another number of dimensions or an index that is not one position. A row passed as an argument is a copy for a
    /// value parameter, and for a `var` parameter it is written back into the row when the call returns (corpus
    /// permissions/P128ArrayRow.al:P128_ArrayRow_ByValue, P128_ArrayRow_ByVar, P128_ArrayRow_TextRowByVar). What
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
        // three (collections/multi-dimension-array-rows/ArrayRows.al:ThreeDim_Rows, ThreeDim_RowToShorterParameter). More dimensions are not pinned.
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
            other => return Err(Signal::unsupported(format!("indexing {}", other.describe()))),
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
        value_read: bool,
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
            return Err(Signal::unsupported(format!(
                "a row of a {}-dimensional array (indexed by {} of its dimensions): what BC does with it is not pinned (docs/probes-needed.md)",
                dims.len(),
                idx.len()
            )));
        }
        let arr = match self.read(base, f)? {
            Value::Array(a) => a,
            other => return Err(Signal::unsupported(format!("indexing {}", other.describe()))),
        };
        let mut flat = 0usize;
        for (d, e) in dims.iter().zip(idx) {
            let at = Self::index_value(e, f)?;
            // An element of a one-dimensional array read as a value (`R := A[D]`, collections/ListNonIntegerIndex.al:196-217 with
            // :433-452) converts a Decimal or BigInteger index as a checked cast to System.Int32: a fraction or a value past the
            // Int32 range raises ConversionCast. A write, an element taken by reference and an array of more dimensions keep `int`.
            let i = match &at {
                Value::Decimal(_) | Value::BigInteger(_) if value_read && dims.len() == 1 => {
                    match al_builtins::conv::convert_cell(&at, &RtType::Integer) {
                        Some(conv) => int(&conv?)?,
                        None => int(&at)?,
                    }
                }
                _ => int(&at)?,
            };
            // A one-dimensional array is measured by the array the variable holds: a parameter declared `array[3]` takes the caller's
            // array of any length and indexes within it (types/W7VarArray.al; Tests-Bank, Tests-Cash Flow and Tests-SCM-Manufacturing
            // index a parameter within the argument's length), so the check against the length follows below.
            if i < 1 || (dims.len() > 1 && i > i64::from(*d)) {
                // Index out of bounds.: collections/multi-dimension-array-rows/ArrayRows.al, errors/error-texts/ErTRuntimeErrors.al:179-184
                // (zero, negative and too high), errors/P129ArrayIndex.al.
                return al_builtins::native::raise("Index out of bounds.", "IndexOutOfBounds");
            }
            flat = flat * (*d as usize) + (i as usize - 1);
        }
        // A parameter declared with more elements than the argument has (Tests-ERM codeunit 134831 passes
        // an `array[2]` to `var DimensionValue: array[9]`): the stored cells are the argument's. What BC does
        // is not pinned by the corpus, so it is refused rather than indexed out of range.
        // A one-dimensional array passed to a parameter declared with more elements: reading or writing past the argument's
        // elements is an index error (types/arrays/PnE01Arrays.al:SmallerArray_VarParameter, SmallerArray_ByValue).
        if flat >= arr.borrow().len() && dims.len() == 1 {
            // GetLastErrorText(true) leaves the text of a write past the end (errors/last-error/LastErrorFlagRuntimeCodes.al:
            // 188-189, :194) and of a read (IndexSites_ExcludeFlag `arrayread_true`; corpus pull request 1327).
            return al_builtins::native::raise_same_text("Index out of bounds.", "IndexOutOfBounds");
        }
        if flat >= arr.borrow().len() {
            return Err(Signal::unsupported(format!(
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
            // A literal keeps 18 significant digits (types/operators-expressions/ExpressionLiteralsRisky.al:
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

/// The global of the frame's own instance that parameter `slot` aliases.
fn own_alias_of_slot(f: &Frame, slot: usize) -> Option<usize> {
    f.aliases
        .iter()
        .find(|(s, inst, _)| *s == slot && Rc::ptr_eq(inst, &f.instance))
        .map(|(_, _, g)| *g)
}

/// The parameter of the frame that aliases global `g` of the frame's own instance.
fn own_alias_of_global(f: &Frame, g: usize) -> Option<usize> {
    f.aliases
        .iter()
        .find(|(_, inst, x)| *x == g && Rc::ptr_eq(inst, &f.instance))
        .map(|(s, _, _)| *s)
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

/// The counter of a `for` loop over Dates (language/ControlFlow.al:607, QX_ForLoop_DateControlVariable): the
/// day number twice, plus one for a closing date, so that a closing date sorts after its day and before the next one
/// (`to ClosingDate(31.12.2023)` visits 30.12 and 31.12).
pub(crate) fn date_key(v: &Value) -> i64 {
    match v {
        Value::Date(d, closing) => i64::from(*d) * 2 + i64::from(*closing),
        _ => 0,
    }
}

/// The Date a [`date_key`] stands for.
pub(crate) fn date_of_key(k: i64) -> Value {
    Value::Date((k.div_euclid(2)) as i32, k.rem_euclid(2) == 1)
}
